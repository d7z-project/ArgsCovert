pub mod args;
pub mod prop;

pub mod project_conf {
    use crate::config::prop::ProjectConfig;
    use crate::lib::SoftError;
    use crate::utils;
    use is_executable::IsExecutable;
    use libc::{SIGHUP, SIGKILL, SIGTERM};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fmt::{Display, Formatter};
    use std::fs::canonicalize;
    use std::io::{Error as IOError, ErrorKind};
    use std::ops::Not;
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use std::{env, fs};

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
        let path = path.as_path();
        if path.is_file().not() {
            return Err(SoftError::AppError(
                format!("配置文件 {} 不存在.", config_path).to_string(),
            ));
        };
        let mut data = fs::read_to_string(path)?;
        let _static_var = String::from("{{item}}");
        utils::string::replace_all_str(
            &mut data,
            &attrs
                .iter()
                .map(|e| (_static_var.replace("item", e.0), e.1.to_string()))
                .collect(),
        );

        let mut result: ProjectConfig = serde_yaml::from_str(&data)
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
                canonicalize(path.parent().unwrap())?.to_str().unwrap(),
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
        data = serde_yaml::to_string(&result).unwrap();
        utils::string::replace_all_str(
            &mut data,
            &result
                .attach
                .iter()
                .map(|e| (_static_var.replace("item", e.0), e.1.to_string()))
                .collect(),
        );
        let result: ProjectConfig = serde_yaml::from_str(&data)
            .map_err(|e| IOError::new(ErrorKind::Other, e.to_string()))?;
        Ok(result)
    }
}
