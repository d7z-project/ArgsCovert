use std::collections::HashMap;
use std::fs;
use std::ops::Not;
use regex::Regex;
use std::path::{PathBuf};
use crate::config::project_conf::{ProjectArgs, ProjectConfig, SourceKeyMode};
use crate::lib::SoftError;
use crate::lib::SoftError::AppError;
use crate::utils::string;

#[derive(Debug)]
pub struct BinaryContext {
    pub args: HashMap<String, BinaryArg>,
    pub envs: HashMap<String, String>,
    pub before_script_path: PathBuf,
    pub after_script_path: PathBuf,
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
    let attrs: HashMap<String, String> = config.attach.iter().map(|e| {
        (var_replace.replace("item", e.0), e.1.to_owned())
    }).collect(); // 项目所有的变量
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
            eprintln!("config load failed form \"{}\",{}.", &conf, e)
        }
    }
    for env_item in std::env::vars() {
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
    }// 装入变量并检查合法性
    println!("{:?}", args_container);
    println!("{:?}", args);
    Err(AppError("暂未实现".to_string()))
}

fn get_then_check_arg(args: &ProjectArgs, vars: &HashMap<String, String>) -> Result<Option<BinaryArg>, SoftError> {
    let regex_str = args.valid_regex.trim();
    let regex = Regex::new(regex_str).map_err(|e| AppError(e.to_string()))?;
    for x in &args.from {
        return if let Some(data) = vars.get(x) {
            if regex_str.is_empty().not() {
                // 带正则判断
                if regex.is_match(data).not() {
                    return Err(AppError(format!("参数 {} 正则校验失败.", data).to_string()));
                }
            }
            Ok(Some(
                BinaryArg {
                    key: args.key.to_string(),
                    value: data.to_string(),
                    mode: args.mode,
                }
            ))
        } else {
            if args.must {
                return Err(AppError(format!("未参数 {} 的值，项目无法启动.", args.key).to_string()));
            }
            Ok(None)
        }
    }
    return Ok(None);
}

fn load_form_local(container: &mut HashMap<String, String>, config_path: &String, cover: bool) -> Result<(), SoftError> {
    let path = config_path.replace("file://", "").trim().to_string();
    let buf = PathBuf::from(&path);
    if buf.is_file().not() {
        return Err(AppError(format!("file not exists").to_string()));
    }
    let config_str = fs::read_to_string(buf).unwrap();
    if path.ends_with(".yaml") || *&path.ends_with(".yml") {
        load_yaml(container, config_str, cover)?
    } else if path.ends_with(".properties") || path.ends_with(".env") {
        load_properties(container, config_str, cover)?
    } else {
        Err(AppError(format!("unknown file type").to_string()))?
    }
    Ok(())
}

fn load_yaml(container: &mut HashMap<String, String>, data: String, cover: bool) -> Result<(), SoftError> {
    Err(AppError("暂未实现".to_string()))
}

fn load_form_remote(container: &mut HashMap<String, String>, config_path: &String, cover: bool) -> Result<(), SoftError> {
    Err(AppError("暂未实现".to_string()))
}

fn load_properties(container: &mut HashMap<String, String>, data: String, cover: bool) -> Result<(), SoftError> {
    data.lines().filter(|e|
        e.starts_with("#").not()
    ).map(|e| -> Vec<&str>{ e.splitn(2, "=").collect() })
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
