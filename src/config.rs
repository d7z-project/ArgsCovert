pub mod project_conf {
    use std::{env, fs};
    use std::env::args;
    use std::io::Error as IOError;
    use std::io::ErrorKind;
    use std::ops::Not;
    use std::path::Path;
    use serde::{Serialize, Deserialize};
    use crate::lib::SoftError;

    pub fn load_info() -> Result<ProjectConfig, SoftError> {
        let execute_path = env::current_exe()?;
        let execute_path = Path::new(execute_path.to_str()
            .ok_or(IOError::new(ErrorKind::Other, ""))?);
        let config_file_path = args().last().filter(|e|
            e.eq(execute_path.to_str().unwrap()).not()
        ).unwrap_or_else(|| {
            format!("{}/{}.yaml",
                    execute_path.parent().unwrap().to_str().unwrap(),
                    execute_path.file_name().unwrap().to_str().unwrap()).to_string()
        });
        let path = Path::new(&config_file_path);

        if path.is_file().not() {
            Err(IOError::new(ErrorKind::Other, format!("配置文件 {} 不存在.", &config_file_path).to_string()))?;
        };
        let data = fs::read_to_string(path)?;
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
        pub append: bool,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub enum LoggerLevel {
        TRACE,
        DEBUG,
        INFO,
        WARN,
        ERROR,
        NONE,
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

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub enum SourceKeyMode {
        ARG,
        ENV,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub enum ProjectSource {
        ENV,
        FILE,
        NETWORK,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct ProjectInfo {
        pub name: String,
        pub exec: String,
        pub before_script: String,
        pub after_script: String,
        pub check_health: HealthCheck,
        pub check_started: StartedCheck,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct HealthCheck {
        pub script: String,
        pub delay: String,
        pub interval: String,
        pub failures: u16,
        pub fail_step: CheckFailStep,
    }


    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub enum CheckFailStep {
        WAIT,
        RESTART,
        EXIT,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    pub struct StartedCheck {
        pub script: String,
        pub interval: String,
    }
}
