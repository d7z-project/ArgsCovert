use clap::Command;
use std::cell::{Cell, RefCell};
use std::ops::Not;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use std::{env, fs, thread};

use libc::{SIGCONT, SIGHUP, SIGINT, SIGTERM, SIGTSTP, SIGUSR1, SIGWINCH};

use crate::binary::args_builder::load_context;
use crate::config::project_conf::load_info;
use crate::config::project_conf::RestartPolicy::{FAIL, NONE};
use crate::config::soft_args::SoftArgs;
use crate::log::debug_str;
use crate::utils::command::execute_script;
use crate::utils::file::new_temp_path;
use crate::utils::log;
use crate::utils::log::{log_default, log_init};
use crate::utils::signal_hook::UnixSignalHook;
use crate::utils::string::replace_all_str_from_map;
use crate::worker::binary_worker::CallbackAction::EXITED;
use crate::worker::binary_worker::{HookScripts, StableWorker};
use crate::worker::script_worker::ScriptWorker;

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
    replace_all_str_from_map(
        &mut soft_config.project.check_health.script,
        &data.script_vars,
    );
    replace_all_str_from_map(
        &mut soft_config.project.check_started.script,
        &data.script_vars,
    );
    // 脚本内容替换
    let stable_worker = StableWorker::new(
        soft_config.project.binary.to_owned(),
        data.args.clone(),
        data.envs.clone(),
        &soft_config.project.signals,
        HookScripts {
            script_worker: soft_config.project.script_worker.clone(),
            before_script: Some(soft_config.project.before_script),
            after_script: Some(soft_config.project.after_script),
        },
    ); // 主要进程工作区

    let health_check = Some(&soft_config.project.check_health.script)
        .filter(|_| soft_config.project.check_health.interval != 0)
        .filter(|e| e.is_empty().not())
        .map(|e| {
            ScriptWorker::new(
                &soft_config.project.script_worker,
                e,
                &data.envs,
                soft_config.project.check_health.delay,
                soft_config.project.check_health.interval,
            )
        })
        .filter(|e| e.is_ok())
        .map(|e| e.unwrap());
    let started_check = Some(&soft_config.project.check_started.script)
        .filter(|_| soft_config.project.check_started.interval != 0)
        .filter(|e| e.is_empty().not())
        .map(|e| {
            ScriptWorker::new(
                &soft_config.project.script_worker,
                e,
                &data.envs,
                0,
                soft_config.project.check_started.interval,
            )
        })
        .filter(|e| e.is_ok())
        .map(|e| e.unwrap());
    stable_worker.start();
    // 项目启动完成。
    let started_success: Cell<i32> = Cell::new(0);
    let health_fail: Cell<i32> = Cell::new(0);
    let enable_check = || -> () {
        debug_str("开始重置");
        started_success.set(0);
        health_fail.set(0);
        if let Some(started_check) = &started_check {
            started_check.start()
        }
        if let Some(health_check) = &health_check {
            health_check.start()
        }
    };
    loop {
        if let Some(health_check) = &health_check {
            for x in health_check.get_status() {
                if x == 0 {
                    health_fail.set(0);
                } else {
                    health_fail.set(health_fail.get() + 1);
                }
            }
            if health_fail.get() >= soft_config.project.check_health.failures as i32 {
                debug_str("健康检查失败！");
                health_check.stop();
                stable_worker.restart();
                enable_check();
            }
        }
        if let Some(started_check) = &started_check {
            for x in started_check.get_status() {
                if x == 0 {
                    started_success.set(started_success.get() + 1);
                } else {
                    started_success.set(0);
                }
            }
            if started_success.get() >= soft_config.project.check_started.success as i32
                && started_success.get() != -1
            {
                started_check.stop();
                debug_str("启动成功！回调脚本");
                let status = execute_script(
                    &soft_config.project.script_worker,
                    &soft_config.project.check_started.started_script,
                    &data.envs,
                );
                if let Ok(code) = status {
                    if code == 0 {
                        debug_str("启动检测回调执行完成。")
                    } else {
                        debug_str("启动检测回调执行失败。")
                    }
                } else {
                    debug_str("启动检测回调执行失败。")
                }
                started_success.set(-1);
            }
        }

        let signals = signal_hook.signals().to_vec();
        if signals.contains(&SIGINT) || signals.contains(&SIGTERM) {
            // 收到停止命令，开始停止
            debug_str("发现 SIGINT");
            break;
        } else if signals.contains(&SIGHUP) {
            debug_str("发现 SIGHUP");
            stable_worker.restart();
            enable_check();
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
                    enable_check();
                } else {
                    debug_str("主进程已退出，根据策略,项目将重启.");
                    stable_worker.restart();
                    enable_check();
                }
            };
        }

        thread::sleep(Duration::from_millis(500));
    }
    stable_worker.exit();
    debug_str("等待主工作线程退出.");
    signal_hook.close();
    for x in health_check {
        debug_str("等待健康检测脚本停止...");
        x.close();
        x.wait_closed();
    }
    for x in started_check {
        debug_str("等待启动检测脚本停止...");
        x.close();
        x.wait_closed();
    }
    stable_worker.wait_exited();
    Ok(())
}
