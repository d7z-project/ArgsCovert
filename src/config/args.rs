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

pub mod soft_args {
    use std::collections::HashMap;
    use std::env;
    use std::ops::Not;
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
        pub variable: Vec<String>,
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
            let mut attach: HashMap<String, String> = Some(args.variable)
                .filter(|e| e.is_empty().not())
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
