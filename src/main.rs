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

use std::cell::Cell;
use std::ops::Not;
use std::thread;
use std::time::Duration;

use libc::{SIGHUP, SIGINT, SIGTERM};

use crate::args::soft_args::SoftArgs;
use crate::binary::args_builder::load_context;
use crate::config::args;
use crate::config::project_conf::load_info;
use crate::config::prop::RestartPolicy::{FAIL, NONE};
use crate::log::{debug_str, error_str, info_str};
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
    info_str("??????????????????.");
    let args = SoftArgs::parse(); // ????????????
    let mut soft_config = load_info(&args.config_path, &args.variable)?; // ??????????????????
    soft_config.log.console.level = args.log_level;
    log_init(&soft_config);
    let data = load_context(&soft_config)?; // ??????????????????????????????
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
    // ??????????????????
    let stable_worker = StableWorker::new(
        soft_config.project.binary.to_owned(),
        data.args.clone(),
        data.envs.clone(),
        &soft_config.project.signals,
        HookScripts {
            script_worker: soft_config.project.script_worker.clone(),
            before_script: Some(soft_config.project.before_script).filter(|e| e.is_empty().not()),
            after_script: Some(soft_config.project.after_script).filter(|e| e.is_empty().not()),
        },
    ); // ?????????????????????

    let health_check = Some(&soft_config.project.check_health.script)
        .filter(|_| soft_config.project.check_health.interval != 0)
        .filter(|e| e.is_empty().not())
        .map(|e| {
            ScriptWorker::new(
                "??????????????????",
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
                "????????????????????????",
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
    // ?????????????????????
    let started_success: Cell<i32> = Cell::new(0);
    let health_fail: Cell<i32> = Cell::new(0);
    let enable_check = || -> () {
        debug_str("????????????");
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
                error_str("??????????????????????????????????????????");
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
                info_str("???????????????????????????");
                let status = execute_script(
                    "????????????????????????",
                    &soft_config.project.script_worker,
                    &soft_config.project.check_started.started_script,
                    &data.envs,
                );
                if let Ok(code) = status {
                    if code == 0 {
                        info_str("?????????????????????????????????")
                    } else {
                        error_str("?????????????????????????????????")
                    }
                } else {
                    error_str("?????????????????????????????????")
                }
                started_success.set(-1);
            }
        }

        let signals = signal_hook.signals().to_vec();
        if signals.contains(&SIGINT) || signals.contains(&SIGTERM) {
            // ?????????????????????????????????
            debug_str("?????? SIGINT");
            break;
        } else if signals.contains(&SIGHUP) {
            debug_str("?????? SIGHUP");
            stable_worker.restart();
            enable_check();
        }
        if let Ok(nonblock) = stable_worker.status.try_lock() {
            if let EXITED(exit_code) = *nonblock {
                if (exit_code == 0 && soft_config.project.restart_policy == FAIL)
                    || soft_config.project.restart_policy == NONE
                {
                    debug_str("?????????????????????????????????,???????????????.");
                    stable_worker.exit();
                    break;
                } else if exit_code != 0 && soft_config.project.restart_policy == FAIL {
                    debug_str("????????????????????????????????????,???????????????.");
                    stable_worker.restart();
                    enable_check();
                } else {
                    debug_str("?????????????????????????????????,???????????????.");
                    stable_worker.restart();
                    enable_check();
                }
            };
        }

        thread::sleep(Duration::from_millis(500));
    }
    stable_worker.exit();
    debug_str("???????????????????????????.");
    signal_hook.close();
    for x in health_check {
        debug_str("??????????????????????????????...");
        x.close();
        x.wait_closed();
    }
    for x in started_check {
        debug_str("??????????????????????????????...");
        x.close();
        x.wait_closed();
    }
    stable_worker.wait_exited();
    Ok(())
}
