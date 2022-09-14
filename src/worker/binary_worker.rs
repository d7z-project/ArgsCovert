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
            "脚本钩子文件位于 {:?} 和 {:?}.",
            before_script_path, after_script_path
        ));
        if let Some(data) = &hooks.before_script {
            fs::write(&before_script_path, data).expect("前置脚本钩子无法写入！");
        }
        if let Some(data) = &hooks.after_script {
            fs::write(&after_script_path, data).expect("后置脚本钩子无法写入！");
        }
        let mut restart = false;
        debug_str("子进程开始启动.");
        let before_hook = || -> bool {
            if let Some(_) = &hooks.before_script {
                debug_str("发现启动前钩子，开始执行脚本钩子.");
                let before_path = before_script_path.to_str().unwrap().to_string();
                if let Ok(data) = Command::new(&hooks.script_worker)
                    .envs(&envs)
                    .arg(&before_path)
                    .output()
                {
                    info(format!(
                        "前置钩子标准输出 - {} => \n {}",
                        &before_path,
                        String::from_utf8_lossy(&data.stdout).to_string()
                    ));
                    for x in Some(String::from_utf8_lossy(&data.stderr).to_string())
                        .filter(|e| e.trim().is_empty().not())
                    {
                        warn(format!("前置钩子错误输出 - {} => \n {}", &before_path, x));
                    }

                    if data.status.code().unwrap_or(-1) != 0 {
                        {
                            if let Ok(mut lock) = callback_action.lock() {
                                *lock = EXITED(1);
                            }
                        }
                        error_str("前置钩子执行失败，返回码不为 0");
                        return false;
                    } else {
                        debug_str("前置钩子执行完成。");
                    }
                } else {
                    {
                        if let Ok(mut lock) = callback_action.lock() {
                            *lock = EXITED(1);
                        }
                    }
                    error(format!("前置钩子执行失败,内部流程出现问题"));
                    return false;
                }
            }
            true
        };
        let destroy_hook = || -> () {
            if let Some(_) = &hooks.after_script {
                debug_str("发现销毁钩子，开始执行脚本.");
                let after_path = after_script_path.to_str().unwrap().to_string();
                if let Ok(data) = Command::new(&hooks.script_worker)
                    .envs(&envs)
                    .arg(&after_path)
                    .output()
                {
                    for x in Some(String::from_utf8_lossy(&data.stdout).to_string())
                        .filter(|e| e.trim().is_empty().not())
                    {
                        info(format!("销毁钩子标准输出 - {} => \n {}", &after_path, x));
                    }
                    for x in Some(String::from_utf8_lossy(&data.stderr).to_string())
                        .filter(|e| e.trim().is_empty().not())
                    {
                        warn(format!("销毁钩子错误输出 - {} => \n {}", &after_path, x));
                    }
                }
            }
        };
        'e: loop {
            {
                if let Ok(mut lock) = callback_action.lock() {
                    if let EXITED(_) = *lock {
                    } else {
                        debug_str("确认程序退出");
                        *lock = EXITED(0);
                    }
                }
            };
            if before_hook().not() {
                continue;
            }
            if !restart {
                debug_str("等待启动命令唤醒");
                let action = nio_rx.recv().unwrap();
                match action {
                    // 等待启动指令
                    START => {}
                    RESTART => {}
                    EXIT => break,
                    _ => continue,
                }
                debug_str("启动唤醒成功，开始执行");
            } else {
                debug_str("当前为重启模式，项目重启中.");
            }
            restart = false;
            {
                if let Ok(mut lock) = callback_action.lock() {
                    *lock = CREATED;
                }
            };
            let mut child_process = Command::new(&binary);
            debug(format!("启动命令: {} ", &binary));
            debug(format!("启动参数: {:?} ", &args));
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
                error(format!("项目启动错误！{}", e.to_string()));
                continue;
            }

            let mut child_process = child_process.unwrap();
            {
                if let Ok(mut lock) = callback_action.lock() {
                    *lock = STARTED;
                }
            }
            debug_str("开始抓取进程标准输出信息.");
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
                        debug_str("程序异常退出");
                    } else {
                        debug_str("程序正常退出");
                    }
                    destroy_hook();
                    break;
                }
                trace_str("开始搜集响应日志.");
                thread::sleep(Duration::from_secs(1));
                buffer.clear();
                let stdout_size = stdout.read_available(&mut buffer).unwrap();
                trace_str("抓取标准日志.");
                let s_out = String::from_utf8_lossy(&buffer[..stdout_size])
                    .trim()
                    .to_string();
                if s_out.is_empty().not() {
                    info(format!("子进程标准输出:\n{}", s_out));
                }
                buffer.clear();
                trace_str("抓取错误日志.");
                let stderr_size = stderr.read_available(&mut buffer).unwrap();
                let s_err = String::from_utf8_lossy(&buffer[..stderr_size])
                    .trim()
                    .to_string();
                if s_err.is_empty().not() {
                    warn(format!("子进程错误输出:\n{}", s_err));
                }
                buffer.clear();
                trace_str("抓取日志完成.");
                fn wait_then_kill(child: &mut Child, timeout: i32) {
                    for index in 0..(timeout * 2) {
                        if let Ok(Some(code)) = child.try_wait() {
                            debug(format!(
                                "程序等待大约 {} 秒后程序停止,停止状态为 {}.",
                                index / 2,
                                code.code().unwrap_or(-1)
                            ));
                            return;
                        }
                        thread::sleep(Duration::from_millis(500));
                    }
                    warn(format!(
                        "程序在规定时间内({}s)未停止，将强制杀死进程.",
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
                                "程序收到停止指令，停止程序并等待下次唤醒,使用 {} 信号量.",
                                i
                            ));
                            destroy_hook();
                            wait_then_kill(&mut child_process, 90);
                            debug(format!("等待进程 {} 结束.", child_process.id()));
                            break 'l;
                        }
                        RESTART => {
                            unsafe {
                                libc::kill(child_process.id() as i32, signals.exit);
                            }
                            destroy_hook();
                            debug(format!(
                                "程序收到重启指令，退出程序并重启,使用 {} 信号量.",
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
                                "程序收到退出指令，退出程序,使用 {} 信号量.",
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
            debug_str("执行器已被销毁，无法执行新的程序");
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
