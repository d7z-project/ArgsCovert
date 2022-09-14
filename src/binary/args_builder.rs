/*
 * Copyright (c) 2022, Dragon's Zone Project. All rights reserved.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::collections::HashMap;
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use regex::Regex;

use crate::config::prop::{ProjectArgs, ProjectConfig, SourceKeyMode};
use crate::lib::SoftError;
use crate::lib::SoftError::AppError;
use crate::utils::log::warn;
use crate::utils::string;

#[derive(Debug)]
pub struct BinaryContext {
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
    pub before_script_path: PathBuf,
    pub after_script_path: PathBuf,
    pub started_check_script_path: PathBuf,
    pub health_check_script_path: PathBuf,
    pub script_vars: HashMap<String, String>,
}

#[derive(Debug)]
pub struct BinaryArg {
    pub key: String,
    pub value: String,
    pub mode: SourceKeyMode,
}

/**
加载配置文件当前上下文和环境变量的数据

默认情况下，环境变量配置优于配置文件下配置，

 **/
pub fn load_context(config: &ProjectConfig) -> Result<BinaryContext, SoftError> {
    let mut args_container: HashMap<String, String> = HashMap::new(); // 变量容器
    let var_replace = String::from("{{item}}");
    let attrs: HashMap<String, String> = config
        .attach
        .iter()
        .map(|e| (var_replace.replace("item", e.0), e.1.to_owned()))
        .collect(); // 项目所有的变量
    for conf in &config.path {
        let res = if conf.starts_with("file://") {
            // 加载本地文件
            load_form_local(&mut args_container, &conf, false)
        } else if conf.starts_with("http://") || conf.starts_with("https://") {
            // 加载网络配置
            load_form_remote(&mut args_container, &conf, false)
        } else {
            // 默认加载本地配置
            load_form_local(&mut args_container, &format!("file://{}", conf), false)
        };
        if let Err(e) = res {
            warn(format!("无法从 \"{}\" 位置加载配置，因为{}.", &conf, e))
        }
    }
    for env_item in env::vars() {
        args_container.insert(env_item.0, env_item.1);
    } // 装入环境变量，覆盖从文件读取的信息

    for args_item in args_container.iter_mut() {
        string::replace_all_str_from_map(args_item.1, &attrs);
    } // 遍历替换参数内容变量
    let mut args: Vec<BinaryArg> = vec![];
    for arg in &config.args {
        if let Some(arg) = get_then_check_arg(&arg, &args_container)? {
            args.push(arg);
        }
    } // 装入变量并检查合法性
    let system_time = SystemTime::now();
    let duration = system_time.duration_since(UNIX_EPOCH).unwrap();
    let before_script_path = Path::new(env::temp_dir().as_path()).join(format!(
        "{}-script-before-{:?}.sh",
        config.project.name, duration
    ));
    let after_script_path = Path::new(env::temp_dir().as_path()).join(format!(
        "{}-script-after-{:?}.sh",
        config.project.name, duration
    ));

    let started_check_script_path = Path::new(env::temp_dir().as_path()).join(format!(
        "{}-script-started-{:?}.sh",
        config.project.name, duration
    ));

    let health_check_script_path = Path::new(env::temp_dir().as_path()).join(format!(
        "{}-script-health-{:?}.sh",
        config.project.name, duration
    ));
    let mut out_envs: HashMap<String, String> = env::vars().collect();
    let mut out_args: Vec<String> = vec![];
    let mut script_vars: HashMap<String, String> = attrs.clone();
    for x in args {
        script_vars.insert(
            var_replace.replace("item", &x.key).to_string(),
            x.value.to_string(),
        );
        match x.mode {
            SourceKeyMode::ARG => {
                out_args.push(x.key);
                out_args.push(x.value);
            }
            SourceKeyMode::ENV => {
                out_envs.insert(x.key, x.value);
            }
        }
    }
    Ok(BinaryContext {
        args: out_args,
        before_script_path,
        after_script_path,
        envs: out_envs,
        script_vars,
        started_check_script_path,
        health_check_script_path,
    })
}

fn get_then_check_arg(
    args: &ProjectArgs,
    vars: &HashMap<String, String>,
) -> Result<Option<BinaryArg>, SoftError> {
    let regex_str = args.valid_regex.trim();
    let alias_vars: HashMap<String, String> = args
        .source_alias
        .iter()
        .map(|e| (&e.target, e.over, vars.get(&e.source)))
        .filter(|e| e.2.is_some() && (vars.contains_key(e.0).not() || e.1))
        .map(|e| (e.0.to_string(), e.2.unwrap().to_string()))
        .collect();
    let dist_value_regex = Regex::new(regex_str).map_err(|e| AppError(e.to_string()))?;
    let variable_regex = Regex::new("\\{\\{\\w.*?}}").unwrap();
    let get_envs_value = |key: &str| -> Option<&String> {
        let key = key.replace("{{", "").replace("}}", "");
        vars.get(&key).or(alias_vars.get(&key))
    };
    for arg_format in &args.from {
        // 获取单个判断
        let variables: HashMap<String, Option<String>> = variable_regex
            .find_iter(arg_format)
            .map(|e| arg_format[e.start()..e.end()].to_string())
            .map(|e| (e.to_string(), get_envs_value(&e).map(|v| v.to_string())))
            .collect();
        let not_found_var: Vec<String> = variables
            .iter()
            .filter(|e| e.1.is_none())
            .map(|e| e.0.to_string())
            .collect();
        if not_found_var.is_empty().not() {
            warn(format!(
                "参数 '{}' 有 '{:?}' 变量未找到,跳过此配置",
                arg_format, not_found_var
            ));
            continue;
        }
        let variables: HashMap<String, String> = variables
            .iter()
            .filter(|e| e.1.is_some())
            .map(|e| (e.0.to_string(), e.1.to_owned().unwrap()))
            .collect();
        let mut filled_arg_format = arg_format.clone();
        string::replace_all_str_from_map(&mut filled_arg_format, &variables);
        if dist_value_regex.is_match(&filled_arg_format).not() {
            let message = args
                .valid_message
                .replace("{{message.value}}", &filled_arg_format)
                .replace("{{message.key}}", arg_format);
            warn(message);
            continue;
        }
        return Ok(Some(BinaryArg {
            key: args.key.to_string(),
            value: filled_arg_format.to_string(),
            mode: args.mode,
        }));
    }
    if args.must {
        return Err(AppError(
            format!("未找到配置参数 '{}' 的值，项目无法启动.", args.key).to_string(),
        ));
    }
    return Ok(None);
}

fn load_form_local(
    container: &mut HashMap<String, String>,
    config_path: &String,
    cover: bool,
) -> Result<(), SoftError> {
    let path = config_path.replace("file://", "").trim().to_string();
    let buf = PathBuf::from(&path);
    if buf.is_file().not() {
        return Err(AppError(format!("文件不存在").to_string()));
    }
    let config_str = fs::read_to_string(buf).unwrap();
    if path.ends_with(".yaml") || *&path.ends_with(".yml") {
        load_yaml(container, config_str, cover)?
    } else if path.ends_with(".properties") || path.ends_with(".env") {
        load_properties(container, config_str, cover)?
    } else {
        Err(AppError(format!("未知文件类型").to_string()))?
    }
    Ok(())
}

fn load_yaml(
    _container: &mut HashMap<String, String>,
    _data: String,
    _cover: bool,
) -> Result<(), SoftError> {
    Err(AppError("暂未实现加载 YAML".to_string()))
}

fn load_form_remote(
    _container: &mut HashMap<String, String>,
    _config_path: &String,
    _cover: bool,
) -> Result<(), SoftError> {
    Err(AppError("暂未实现加载网络配置".to_string()))
}

fn load_properties(
    container: &mut HashMap<String, String>,
    data: String,
    cover: bool,
) -> Result<(), SoftError> {
    data.lines()
        .filter(|e| e.starts_with("#").not())
        .map(|e| -> Vec<&str> { e.splitn(2, "=").collect() })
        .filter(|e| e.len() == 2)
        .map(|e| (e[0].to_string(), e[1].to_string()))
        .for_each(|e| {
            if cover {
                container.insert(e.0, e.1);
            } else {
                container.entry(e.0).or_insert(e.1);
            }
        });
    Ok(())
}
