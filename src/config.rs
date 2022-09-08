pub mod project_conf {
    use std::{env, fs};
    use std::collections::HashMap;
    use std::fmt::{Display, Formatter};
    use std::fs::{canonicalize};
    use std::io::Error as IOError;
    use std::io::ErrorKind;
    use std::ops::{Not};
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use serde::{Serialize, Deserialize};
    use crate::lib::SoftError;
    use crate::utils;
    use is_executable::IsExecutable;

    /**
    加载配置文件
     */
    pub fn load_info(config_path: &str, attrs: &HashMap<String, String>) -> Result<ProjectConfig, SoftError> {
        let mut attrs = attrs.clone();
        let path = canonicalize(Path::new(config_path)).map_err(
            |_| SoftError::AppError(format!("配置文件 {} 不存在.", config_path).to_string())
        )?;
        let path = path.as_path();
        if path.is_file().not() {
            return Err(SoftError::AppError(format!("配置文件 {} 不存在.", config_path).to_string()));
        };
        let mut data = fs::read_to_string(path)?;
        let _static_var = String::from("{{item}}");
        utils::string::replace_all_str(
            &mut data,
            &attrs.iter()
                .map(|e| {
                    (_static_var.replace("item", e.0), e.1.to_string())
                })
                .collect(),
        );

        let mut result: ProjectConfig = serde_yaml::from_str(&data)
            .map_err(|e| IOError::new(ErrorKind::Other, e.to_string()))?;
        result.attach.iter().for_each(|it| {
            (&mut attrs).entry(it.0.to_owned()).or_insert(it.1.to_owned());
        });

        let binary_paths = vec![
            PathBuf::from_str(&result.project.binary)?,
            PathBuf::from(
                format!("{}{}",
                        canonicalize(path.parent()
                            .unwrap())?.to_str().unwrap(), &result.project.binary)),
            PathBuf::from(
                format!("{}{}",
                        canonicalize(env::current_dir()
                            .unwrap())?.to_str().unwrap(), &result.project.binary)),
        ];
        for binary_path in binary_paths {
            if binary_path.is_file() { // 如果文件存在
                let binary_path = canonicalize(&binary_path).unwrap();
                let binary_path = binary_path.to_str().unwrap();
                attrs.insert("binary.location".to_string(), binary_path.to_string());
                result.project.binary = binary_path.to_string();
                break;
            }
        }
        if PathBuf::from(&result.project.binary).is_file().not() {
            return Err(SoftError::AppError(format!("可执行文件 {} 不存在.", &result.project.binary).to_string()));
        }
        if Path::new(&result.project.binary).is_executable().not() {
            return Err(SoftError::AppError(format!("可执行文件 {} 无运行权限.", &result.project.binary).to_string()));
        }
        result.attach = attrs.clone();
        data = serde_yaml::to_string(&result).unwrap();
        utils::string::replace_all_str(
            &mut data,
            &result.attach.iter()
                .map(|e| {
                    (_static_var.replace("item", e.0), e.1.to_string())
                })
                .collect(),
        );
        let result: ProjectConfig = serde_yaml::from_str(&data)
            .map_err(|e| IOError::new(ErrorKind::Other, e.to_string()))?;
        Ok(result)
    }


    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct ProjectConfig {
        pub project: ProjectInfo,
        pub args: Vec<ProjectArgs>,
        pub path: Vec<String>,
        pub log: ProjectLog,
        pub attach: HashMap<String, String>,
    }


    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct ProjectLog {
        pub console: ConsoleLog,
        pub file: FileLog,

    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct ConsoleLog {
        pub level: LoggerLevel,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct FileLog {
        pub level: LoggerLevel,
        pub path: String,
        pub error_path: String,
        pub append: bool,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
    pub enum LoggerLevel {
        TRACE,
        DEBUG,
        INFO,
        WARN,
        ERROR,
        NONE,
    }

    pub trait LogLevelId {
        fn id(&self) -> u8;
    }

    impl Display for LoggerLevel {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let x = match self {
                LoggerLevel::TRACE => "TRACE",
                LoggerLevel::DEBUG => "DEBUG",
                LoggerLevel::INFO => "INFO",
                LoggerLevel::WARN => "WARN",
                LoggerLevel::ERROR => "ERROR",
                LoggerLevel::NONE => "NONE",
            };
            f.write_str(x)
        }
    }

    impl FromStr for LoggerLevel {
        type Err = SoftError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "TRACE" => Ok(LoggerLevel::TRACE),
                "DEBUG" => Ok(LoggerLevel::DEBUG),
                "INFO" => Ok(LoggerLevel::INFO),
                "WARN" => Ok(LoggerLevel::WARN),
                "ERROR" => Ok(LoggerLevel::ERROR),
                "NONE" => Ok(LoggerLevel::NONE),
                _ => Err(SoftError::AppError("unknown logger level".to_string()))
            }
        }
    }

    impl LogLevelId for LoggerLevel {
        fn id(&self) -> u8 {
            match self {
                LoggerLevel::TRACE => 0,
                LoggerLevel::DEBUG => 1,
                LoggerLevel::INFO => 2,
                LoggerLevel::WARN => 3,
                LoggerLevel::ERROR => 4,
                LoggerLevel::NONE => 5,
            }
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct ProjectArgs {
        pub name: String,
        pub key: String,
        pub from: Vec<String>,
        pub mode: SourceKeyMode,
        pub must: bool,
        pub valid_regex: String,
        pub valid_message: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
    pub struct SoftSignals {
        pub reload: i32,
        pub exit: i32,
        pub kill: i32,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
    pub enum SourceKeyMode {
        ARG,
        ENV,
    }


    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct ProjectInfo {
        pub name: String,
        pub binary: String,
        pub before_script: String,
        pub after_script: String,
        pub check_health: HealthCheck,
        pub check_started: StartedCheck,
        pub signals: SoftSignals,
        pub restart_policy: RestartPolicy,

    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct HealthCheck {
        pub script: String,
        pub delay: String,
        pub interval: String,
        pub failures: u16,
    }


    ///重启策略
    #[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
    pub enum RestartPolicy {
        /// 不重启
        NONE,
        /// 总是重启
        ALWAYS,
        /// 失败重启
        FAIL,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct StartedCheck {
        pub script: String,
        pub interval: String,
        pub started_script: String,
    }
}

pub mod soft_args {
    use std::collections::HashMap;
    use std::env;
    use std::path::{PathBuf};
    use clap::Parser;
    use crate::config::project_conf::LoggerLevel;
    use crate::lib::SoftError;

    /// Easy Application Args covert.
    #[derive(Parser, Debug)]
    #[clap(author, version, about, long_about = None)]
    pub struct SoftStaticArgs {
        /// config path
        #[clap(short, long = "--config", default_value_t = String::from("application.yaml"))]
        pub config_path: String,
        /// add variable
        #[clap(short = 'a', long = "--attach")]
        pub variable: Option<Vec<String>>,
        /// logger level
        #[clap(short = 'l', long = "--level", default_value_t = LoggerLevel::INFO)]
        pub console_log_level: LoggerLevel,
    }


    #[derive(Debug)]
    pub struct SoftArgs {
        pub config_path: String,
        pub log_level: LoggerLevel,
        pub variable: HashMap<String, String>,
    }

    impl SoftArgs {
        pub fn parse() -> Self {
            let args: SoftStaticArgs = SoftStaticArgs::parse();
            let mut attach: HashMap<String, String> = args.variable.
                unwrap_or(vec![]).iter()
                .map(|e| -> Vec<&str>  { e.splitn(2, "=").collect() })
                .filter(|e| e.len() == 2)
                .map(|e| (e.get(0).unwrap().to_string(), e.get(1).unwrap().to_string()))
                .collect();
            let user_dir = env::current_dir().unwrap_or(PathBuf::new());
            #[allow(deprecated)]
                let user_home = env::home_dir().unwrap();
            attach.insert("user.dir".to_string(), user_dir.to_str().unwrap_or("").to_string());
            attach.insert("user.home".to_string(), user_home.to_str().unwrap().to_string());
            attach.insert("app.dir".to_string(), env::current_exe().unwrap().parent().unwrap().to_str().unwrap().to_string());
            SoftArgs {
                log_level: args.console_log_level,
                config_path: args.config_path,
                variable: attach,
            }
        }
    }
}
