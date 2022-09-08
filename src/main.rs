use std::thread;
use std::time::Duration;

use libc::{
    c_void, sighandler_t, signal, SIGCONT, SIGHUP, SIGINT, SIGTERM, SIGTSTP, SIGUSR1, SIGWINCH,
};
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::SignalsInfo;

use crate::binary::args_builder::load_context;
use crate::config::project_conf::load_info;
use crate::config::project_conf::RestartPolicy::{FAIL, NONE};
use crate::config::soft_args::SoftArgs;
use crate::log::debug_str;
use crate::utils::log;
use crate::utils::log::{log_default, log_init};
use crate::utils::signal_hook::UnixSignalHook;
use crate::utils::string::{replace_all_str, replace_all_str_from_map};
use crate::worker::binary_worker::CallbackAction::EXITED;
use crate::worker::binary_worker::{HookScripts, StableWorker};

mod binary;
mod config;
mod lib;
mod utils;
mod worker;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::info_str("项目已经启动.");
    let args = SoftArgs::parse(); // 拉取参数
    let mut soft_config = load_info(&args.config_path, &args.variable)?; // 加载系统配置
    soft_config.log.console.level = args.log_level;
    log_init(&soft_config);
    let data = load_context(&soft_config)?; // 载入并校验可用的参数
    let signal_hook = UnixSignalHook::new(vec![SIGINT, SIGTERM, SIGHUP]);
    replace_all_str_from_map(&mut soft_config.project.before_script, &data.script_vars);
    replace_all_str_from_map(&mut soft_config.project.after_script, &data.script_vars);
    let stable_worker = StableWorker::new(
        soft_config.project.binary.to_owned(),
        data.args,
        data.envs,
        &soft_config.project.signals,
        HookScripts {
            script_worker: soft_config.project.script_worker,
            before_script: Some(soft_config.project.before_script),
            after_script: Some(soft_config.project.after_script),
        },
    ); // 主要进程工作区
    stable_worker.start();
    // 项目启动完成。

    loop {
        let signals = signal_hook.signals().to_vec();
        if signals.contains(&SIGINT) || signals.contains(&SIGTERM) {
            // 收到停止命令，开始停止
            debug_str("发现 SIGINT");
            break;
        } else if signals.contains(&SIGHUP) {
            debug_str("发现 SIGHUP");
            stable_worker.restart();
        }

        if let Ok(nonblock) = stable_worker.status.try_lock() {
            if let EXITED(exit_code) = *nonblock {
                if (exit_code == 0 && soft_config.project.restart_policy == FAIL)
                    || soft_config.project.restart_policy == NONE
                {
                    debug_str("主进程已结束，根据策略,项目已结束.");
                    stable_worker.exit();
                    break;
                } else if exit_code != 0 && soft_config.project.restart_policy == FAIL {
                    debug_str("主进程异常退出，根据策略,项目将重启.");
                    stable_worker.restart();
                } else {
                    debug_str("主进程已退出，根据策略,项目将重启.");
                    stable_worker.restart();
                }
            };
        }

        thread::sleep(Duration::from_millis(500));
    }
    stable_worker.exit();
    debug_str("等待主工作线程退出..");
    signal_hook.close();
    stable_worker.wait_exited();
    Ok(())
}
