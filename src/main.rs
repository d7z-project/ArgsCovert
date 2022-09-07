use std::io::{Read};
use crate::binary::args_builder::load_context;
use crate::config::project_conf::{load_info, SourceKeyMode};
use crate::config::soft_args::{SoftArgs};
use std::process::{Command, Stdio};
use std::{fs, thread};

use std::time::Duration;
use crate::utils::log;
use crate::utils::log::{log_default, log_init};

mod config;
mod lib;
mod utils;
mod binary;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log_default();
    log::info_str("项目已经启动.");
    let args1 = SoftArgs::parse(); // 拉取参数
    let soft_config = load_info(&args1.config_path, &args1.variable)?; // 加载系统配置
    log_init(&soft_config);
    let data = load_context(&soft_config)?; // 载入并校验可用的参数
    fs::write(data.started_check_script_path, soft_config.project.check_started.script)?;
    fs::write(data.health_check_script_path, soft_config.project.check_health.script)?;
    fs::write(data.before_script_path, soft_config.project.before_script)?;
    fs::write(data.after_script_path, soft_config.project.after_script)?;
    let mut command = Command::new(soft_config.project.binary);
    command.envs(data.envs);
    for x in data.args {
        match x.mode {
            SourceKeyMode::ARG => { command.arg(x.key).arg(x.value); }
            SourceKeyMode::ENV => { command.env(x.key, x.value); }
        }
    };
    command.stdout(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    loop {
        let mut x1 = [0; 1024];
        log::info(format!("{:?}", stdout.read(&mut x1)));
        log::info(format!("{}", String::from_utf8_lossy(&x1)));
        thread::sleep(Duration::from_secs(1));
        log::info(format!("{:?}", &child.try_wait().unwrap()));
    }
}
