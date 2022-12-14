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

use std::collections::HashMap;
use std::ops::Not;
use std::os::unix::prelude::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};

use libc::SIGTERM;
use nonblock::NonBlockingReader;

use crate::config::prop::SoftSignals;
use crate::log::{debug, debug_str, error, error_str, info, trace_str, warn};
use crate::worker::binary_worker::CallbackAction::{CREATED, DESTROYED, EXITED, STARTED};
use crate::worker::binary_worker::ChildThreadAction::{EXIT, KILL, RESTART, START};

pub struct StableWorker {
    pub master_rx: SyncSender<ChildThreadAction>,
    pub status: Arc<Mutex<CallbackAction>>,
}

#[derive(PartialEq)]
pub enum ChildThreadAction {
    START,
    #[allow(dead_code)]
    KILL(i32),
    EXIT,
    RESTART,
}

#[derive(PartialEq, Debug)]
pub enum CallbackAction {
    CREATED,
    STARTED,
    EXITED(i32),
    DESTROYED,
}

impl StableWorker {
    pub fn wait_exited(&self) {
        loop {
            if let Ok(data) = self.status.try_lock() {
                if DESTROYED == *data {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.master_rx.send(KILL(SIGTERM)).unwrap();
    }
    pub fn restart(&self) {
        self.master_rx.send(RESTART).unwrap();
    }
    pub fn exit(&self) {
        self.master_rx.send(EXIT).unwrap();
    }
    pub fn start(&self) {
        self.master_rx.send(START).unwrap();
    }

    fn thread_fun(
        nio_rx: Receiver<ChildThreadAction>,
        binary: String,
        args: Vec<String>,
        envs: HashMap<String, String>,
        callback_action: Arc<Mutex<CallbackAction>>,
        signals: SoftSignals,
        hooks: HookScripts,
    ) {
        let system_time = SystemTime::now();
        let duration = system_time.duration_since(UNIX_EPOCH).unwrap();
        let before_script_path = Path::new(env::temp_dir().as_path())
            .join(format!("args-before-script-{:?}.sh", duration));
        let after_script_path = Path::new(env::temp_dir().as_path())
            .join(format!("args-after-script-{:?}.sh", duration));
        debug(format!(
            "???????????????????????? {:?} ??? {:?}.",
            before_script_path, after_script_path
        ));
        if let Some(data) = &hooks.before_script {
            fs::write(&before_script_path, data).expect("?????????????????????????????????");
        }
        if let Some(data) = &hooks.after_script {
            fs::write(&after_script_path, data).expect("?????????????????????????????????");
        }
        let mut restart = false;
        debug_str("?????????????????????.");
        let before_hook = || -> bool {
            if let Some(_) = &hooks.before_script {
                debug_str("????????????????????????????????????????????????.");
                let before_path = before_script_path.to_str().unwrap().to_string();
                if let Ok(data) = Command::new(&hooks.script_worker)
                    .envs(&envs)
                    .arg(&before_path)
                    .output()
                {
                    info(format!(
                        "???????????????????????? - {} => \n {}",
                        &before_path,
                        String::from_utf8_lossy(&data.stdout).to_string()
                    ));
                    for x in Some(String::from_utf8_lossy(&data.stderr).to_string())
                        .filter(|e| e.trim().is_empty().not())
                    {
                        warn(format!("???????????????????????? - {} => \n {}", &before_path, x));
                    }

                    if data.status.code().unwrap_or(-1) != 0 {
                        {
                            if let Ok(mut lock) = callback_action.lock() {
                                *lock = EXITED(1);
                            }
                        }
                        error_str("?????????????????????????????????????????? 0");
                        return false;
                    } else {
                        debug_str("???????????????????????????");
                    }
                } else {
                    {
                        if let Ok(mut lock) = callback_action.lock() {
                            *lock = EXITED(1);
                        }
                    }
                    error(format!("????????????????????????,????????????????????????"));
                    return false;
                }
            }
            true
        };
        let destroy_hook = || -> () {
            if let Some(_) = &hooks.after_script {
                debug_str("???????????????????????????????????????.");
                let after_path = after_script_path.to_str().unwrap().to_string();
                if let Ok(data) = Command::new(&hooks.script_worker)
                    .envs(&envs)
                    .arg(&after_path)
                    .output()
                {
                    for x in Some(String::from_utf8_lossy(&data.stdout).to_string())
                        .filter(|e| e.trim().is_empty().not())
                    {
                        info(format!("???????????????????????? - {} => \n {}", &after_path, x));
                    }
                    for x in Some(String::from_utf8_lossy(&data.stderr).to_string())
                        .filter(|e| e.trim().is_empty().not())
                    {
                        warn(format!("???????????????????????? - {} => \n {}", &after_path, x));
                    }
                }
            }
        };
        'e: loop {
            {
                if let Ok(mut lock) = callback_action.lock() {
                    if let EXITED(_) = *lock {
                    } else {
                        debug_str("??????????????????");
                        *lock = EXITED(0);
                    }
                }
            };
            if before_hook().not() {
                continue;
            }
            if !restart {
                debug_str("????????????????????????");
                let action = nio_rx.recv().unwrap();
                match action {
                    // ??????????????????
                    START => {}
                    RESTART => {}
                    EXIT => break,
                    _ => continue,
                }
                debug_str("?????????????????????????????????");
            } else {
                debug_str("???????????????????????????????????????.");
            }
            restart = false;
            {
                if let Ok(mut lock) = callback_action.lock() {
                    *lock = CREATED;
                }
            };
            let mut child_process = Command::new(&binary);
            debug(format!("????????????: {} ", &binary));
            debug(format!("????????????: {:?} ", &args));
            let child_process = child_process
                .current_dir(PathBuf::from(&binary).parent().unwrap())
                .args(&args)
                .envs(&envs)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            unsafe {
                child_process.pre_exec(move || {
                    let pid = libc::getpid();
                    libc::setpgid(pid, pid);
                    libc::signal(SIGTERM, libc::SIG_DFL);
                    Ok(())
                });
            }
            let child_process = child_process.spawn();
            if let Err(e) = child_process {
                {
                    if let Ok(mut lock) = callback_action.lock() {
                        *lock = EXITED(1);
                    }
                }
                error(format!("?????????????????????{}", e.to_string()));
                continue;
            }

            let mut child_process = child_process.unwrap();
            {
                if let Ok(mut lock) = callback_action.lock() {
                    *lock = STARTED;
                }
            }
            debug_str("????????????????????????????????????.");
            let mut stdout =
                NonBlockingReader::from_fd(child_process.stdout.take().unwrap()).unwrap();
            let mut stderr =
                NonBlockingReader::from_fd(child_process.stderr.take().unwrap()).unwrap();
            let mut buffer = vec![];
            'l: loop {
                if let Ok(Some(code)) = child_process.try_wait() {
                    let i = code.code().unwrap_or_else(|| 1);
                    {
                        if let Ok(mut lock) = callback_action.lock() {
                            *lock = EXITED(i);
                        }
                    }
                    if i != 0 {
                        debug_str("??????????????????");
                    } else {
                        debug_str("??????????????????");
                    }
                    destroy_hook();
                    break;
                }
                trace_str("????????????????????????.");
                thread::sleep(Duration::from_secs(1));
                buffer.clear();
                let stdout_size = stdout.read_available(&mut buffer).unwrap();
                trace_str("??????????????????.");
                let s_out = String::from_utf8_lossy(&buffer[..stdout_size])
                    .trim()
                    .to_string();
                if s_out.is_empty().not() {
                    info(format!("?????????????????????:\n{}", s_out));
                }
                buffer.clear();
                trace_str("??????????????????.");
                let stderr_size = stderr.read_available(&mut buffer).unwrap();
                let s_err = String::from_utf8_lossy(&buffer[..stderr_size])
                    .trim()
                    .to_string();
                if s_err.is_empty().not() {
                    warn(format!("?????????????????????:\n{}", s_err));
                }
                buffer.clear();
                trace_str("??????????????????.");
                fn wait_then_kill(child: &mut Child, timeout: i32) {
                    for index in 0..(timeout * 2) {
                        if let Ok(Some(code)) = child.try_wait() {
                            debug(format!(
                                "?????????????????? {} ??????????????????,??????????????? {}.",
                                index / 2,
                                code.code().unwrap_or(-1)
                            ));
                            return;
                        }
                        thread::sleep(Duration::from_millis(500));
                    }
                    warn(format!(
                        "????????????????????????({}s)?????????????????????????????????.",
                        timeout
                    ));
                    child.kill().unwrap_or(());
                }
                if let Ok(e) = nio_rx.try_recv() {
                    match e {
                        KILL(i) => {
                            unsafe {
                                libc::kill(child_process.id() as i32, i);
                            }
                            debug(format!(
                                "????????????????????????????????????????????????????????????,?????? {} ?????????.",
                                i
                            ));
                            destroy_hook();
                            wait_then_kill(&mut child_process, 90);
                            debug(format!("???????????? {} ??????.", child_process.id()));
                            break 'l;
                        }
                        RESTART => {
                            unsafe {
                                libc::kill(child_process.id() as i32, signals.exit);
                            }
                            destroy_hook();
                            debug(format!(
                                "????????????????????????????????????????????????,?????? {} ?????????.",
                                signals.exit
                            ));
                            wait_then_kill(&mut child_process, 90);
                            restart = true;
                            break 'l;
                        }
                        EXIT => {
                            unsafe {
                                libc::kill(child_process.id() as i32, signals.exit);
                            }
                            destroy_hook();
                            debug(format!(
                                "???????????????????????????????????????,?????? {} ?????????.",
                                signals.exit
                            ));
                            wait_then_kill(&mut child_process, 90);
                            break 'e;
                        }
                        _ => {}
                    }
                }
            }
        }
        {
            if let Ok(mut lock) = callback_action.lock() {
                *lock = DESTROYED;
            }
            debug_str("????????????????????????????????????????????????");
        }
    }
    pub fn new(
        binary: String,
        args: Vec<String>,
        envs: HashMap<String, String>,
        signals: &SoftSignals,
        hooks: HookScripts,
    ) -> Self {
        let arc = Arc::new(Mutex::new(CREATED));
        let to_master: (SyncSender<ChildThreadAction>, Receiver<ChildThreadAction>) =
            mpsc::sync_channel(255);
        let callback_action = Arc::clone(&arc);
        let config_signals = signals.clone();
        thread::spawn(move || {
            Self::thread_fun(
                to_master.1,
                binary,
                args,
                envs,
                callback_action,
                config_signals,
                hooks,
            );
        });
        let worker = StableWorker {
            master_rx: to_master.0,
            status: arc,
        };
        return worker;
    }
}

pub struct HookScripts {
    pub script_worker: String,
    pub before_script: Option<String>,
    pub after_script: Option<String>,
}
