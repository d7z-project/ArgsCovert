pub mod soft_args {
    use std::collections::HashMap;
    use std::env;
    use std::path::PathBuf;

    use clap::Parser;

    use crate::config::prop::LoggerLevel;
    use crate::log_default;

    #[derive(Parser, Debug)]
    #[clap(author, version, about = Some(about()))]
    pub struct SoftStaticArgs {
        /// 指定配置文件位置
        #[clap(short, long = "--config", default_value_t = String::from("application.yaml"))]
        pub config_path: String,
        /// 添加内部替换的变量
        #[clap(short = 'a', long = "--attach")]
        pub variable: Option<Vec<String>>,
        /// 配置控制台输出的日志级别
        #[clap(short = 'l', long = "--level", default_value_t = LoggerLevel::INFO)]
        pub console_log_level: LoggerLevel,
    }

    fn about() -> &'static str {
        include_str!("about.txt")
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
            let mut attach: HashMap<String, String> = args
                .variable
                .unwrap_or(vec![])
                .iter()
                .map(|e| -> Vec<&str> { e.splitn(2, "=").collect() })
                .filter(|e| e.len() == 2)
                .map(|e| (e.get(0).unwrap().to_string(), e.get(1).unwrap().to_string()))
                .collect();
            let user_dir = env::current_dir().unwrap_or(PathBuf::new());
            #[allow(deprecated)]
            let user_home = env::home_dir().unwrap();
            attach.insert(
                "user.dir".to_string(),
                user_dir.to_str().unwrap_or("").to_string(),
            );
            attach.insert(
                "user.home".to_string(),
                user_home.to_str().unwrap().to_string(),
            );
            attach.insert(
                "app.dir".to_string(),
                env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
            log_default(args.console_log_level);
            SoftArgs {
                log_level: args.console_log_level,
                config_path: args.config_path,
                variable: attach,
            }
        }
    }
}
