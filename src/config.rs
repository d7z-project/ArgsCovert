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

pub mod args;
pub mod prop;

pub mod project_conf {
    use std::collections::HashMap;
    use std::fmt::{Display, Formatter};
    use std::fs::canonicalize;
    use std::io::{Error as IOError, ErrorKind};
    use std::ops::Not;
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use std::{env, fs};

    use is_executable::IsExecutable;
    use libc::{SIGHUP, SIGKILL, SIGTERM};
    use serde::{Deserialize, Serialize};

    use crate::config::prop::ProjectConfig;
    use crate::lib::SoftError;
    use crate::utils;

    /**
    加载配置文件
     */
    pub fn load_info(
        config_path: &str,
        attrs: &HashMap<String, String>,
    ) -> Result<ProjectConfig, SoftError> {
        let mut attrs = attrs.clone();
        let path = canonicalize(Path::new(config_path)).map_err(|_| {
            SoftError::AppError(format!("配置文件 {} 不存在.", config_path).to_string())
        })?;
        let _config_path = path.as_path();
        if _config_path.is_file().not() {
            return Err(SoftError::AppError(
                format!("配置文件 {} 不存在.", config_path).to_string(),
            ));
        };
        let mut config_data_str = fs::read_to_string(_config_path)?;
        let _static_var = String::from("{{item}}");
        utils::string::replace_all_str(
            &mut config_data_str,
            &attrs
                .iter()
                .map(|e| (_static_var.replace("item", e.0), e.1.to_string()))
                .collect(),
        );

        let mut result: ProjectConfig = serde_yaml::from_str(&config_data_str)
            .map_err(|e| IOError::new(ErrorKind::Other, e.to_string()))?;
        result.attach.iter().for_each(|it| {
            (&mut attrs)
                .entry(it.0.to_owned())
                .or_insert(it.1.to_owned());
        });

        let binary_paths = vec![
            PathBuf::from_str(&result.project.binary)?,
            PathBuf::from(format!(
                "{}{}",
                canonicalize(_config_path.parent().unwrap())?
                    .to_str()
                    .unwrap(),
                &result.project.binary
            )),
            PathBuf::from(format!(
                "{}{}",
                canonicalize(env::current_dir().unwrap())?.to_str().unwrap(),
                &result.project.binary
            )),
        ];
        for binary_path in binary_paths {
            if binary_path.is_file() {
                // 如果文件存在
                let binary_path = canonicalize(&binary_path).unwrap();
                let binary_path = binary_path.to_str().unwrap();
                attrs.insert("binary.location".to_string(), binary_path.to_string());
                result.project.binary = binary_path.to_string();
                break;
            }
        }
        if PathBuf::from(&result.project.binary).is_file().not() {
            return Err(SoftError::AppError(
                format!("可执行文件 {} 不存在.", &result.project.binary).to_string(),
            ));
        }
        if Path::new(&result.project.binary).is_executable().not() {
            return Err(SoftError::AppError(
                format!("可执行文件 {} 无运行权限.", &result.project.binary).to_string(),
            ));
        }
        result.attach = attrs.clone();
        config_data_str = serde_yaml::to_string(&result).unwrap();
        utils::string::replace_all_str(
            &mut config_data_str,
            &result
                .attach
                .iter()
                .map(|e| (_static_var.replace("item", e.0), e.1.to_string()))
                .collect(),
        );
        let result: ProjectConfig = serde_yaml::from_str(&config_data_str)
            .map_err(|e| IOError::new(ErrorKind::Other, e.to_string()))?;
        Ok(result)
    }
}
