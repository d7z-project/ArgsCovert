use std::collections::HashMap;
use std::ops::Not;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::time::Duration;
use libc::SIGTERM;
use nonblock::NonBlockingReader;
use crate::config::project_conf::SoftSignals;
use crate::log::{debug, debug_str, error, info, trace_str};
use crate::worker::binary_worker::CallbackAction::{DESTROYED, EXITED, CREATED, STARTED};
use crate::worker::binary_worker::ChildThreadAction::{EXIT, KILL, RESTART, START};

pub struct StableWorker {
    pub master_rx: SyncSender<ChildThreadAction>,
    pub status: Arc<Mutex<CallbackAction>>,
}


#[derive(PartialEq)]
pub enum ChildThreadAction {
    START,
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
    pub(crate) fn stop(&self) {
        self.master_rx.send(KILL(SIGTERM)).unwrap();
    }
    pub(crate) fn restart(&self) {
        self.master_rx.send(RESTART).unwrap();
    }
    pub(crate) fn exit(&self) {
        self.master_rx.send(EXIT).unwrap();
    }
    pub fn start(&self) {
        self.master_rx.send(START).unwrap();
    }

    fn thread_fun(nio_rx: Receiver<ChildThreadAction>,
                  binary: String,
                  args: Vec<String>,
                  envs: HashMap<String, String>,
                  arc: Arc<Mutex<CallbackAction>>,
                  signals: SoftSignals) {
        let mut restart = false;
        debug_str("子进程开始启动.");
        'e: loop {
            {
                if let Ok(mut lock) = arc.lock() {
                    if let EXITED(_) = *lock {} else {
                        debug_str("确认程序退出");
                        *lock = EXITED(0);
                    }
                }
            };
            if !restart {
                debug_str("等待启动命令唤醒");
                let action = nio_rx.recv().unwrap();
                match action {// 等待启动指令
                    START => {}
                    RESTART => {}
                    EXIT => break,
                    _ => continue
                }
                debug_str("启动唤醒成功，开始执行");
            } else {
                debug_str("当前为重启模式，项目重启中.");
            }
            restart = false;
            {
                if let Ok(mut lock) = arc.lock() {
                    *lock = CREATED;
                }
            };
            let child_process = Command::new(&binary)
                .current_dir(PathBuf::from(&binary).parent().unwrap())
                .args(&args).envs(&envs).stdout(Stdio::piped())
                .stderr(Stdio::piped()).spawn();
            if let Err(e) = child_process {
                {
                    if let Ok(mut lock) = arc.lock() {
                        *lock = EXITED(1);
                    }
                }
                error(format!("项目启动错误！{}", e.to_string()));
                continue;
            }
            let mut child_process = child_process.unwrap();
            {
                if let Ok(mut lock) = arc.lock() {
                    *lock = STARTED;
                }
            }
            debug_str("开始抓取进程标准输出信息.");
            let mut stdout = NonBlockingReader::from_fd(child_process.stdout.take().unwrap()).unwrap();
            let mut stderr = NonBlockingReader::from_fd(child_process.stderr.take().unwrap()).unwrap();
            let mut buffer = vec![];
            'l: loop {
                if let Ok(Some(code)) = child_process.try_wait() {
                    let i = code.code().unwrap_or_else(|| 1);
                    {
                        if let Ok(mut lock) = arc.lock() {
                            *lock = EXITED(i);
                        }
                    }
                    if i != 0 {
                        debug_str("程序异常退出");
                    } else {
                        debug_str("程序正常退出");
                    }
                    break;
                }
                trace_str("开始搜集响应日志.");
                thread::sleep(Duration::from_secs(1));
                buffer.clear();
                let stdout_size = stdout.read_available(&mut buffer).unwrap();
                trace_str("抓取标准日志.");
                let s_out = String::from_utf8_lossy(&buffer[..stdout_size]).trim().to_string();
                if s_out.is_empty().not() {
                    info(format!("Child Process STDOUT:\n{}", s_out));
                }
                buffer.clear();
                trace_str("抓取错误日志.");
                let stderr_size = stderr.read_available(&mut buffer).unwrap();
                let s_err = String::from_utf8_lossy(&buffer[..stderr_size]).trim().to_string();
                if s_err.is_empty().not() {
                    error(format!("Child Process STDERR:\n{}", s_err));
                }
                buffer.clear();
                trace_str("抓取日志完成.");
                if let Ok(e) = nio_rx.try_recv() {
                    match e {
                        KILL(i) => {
                            unsafe {
                                libc::kill(child_process.id() as i32, i);
                            }
                            debug(format!("程序收到停止指令，停止程序并等待下次唤醒,使用 {} 信号量.", i));

                            break 'l;
                        }
                        RESTART => {
                            unsafe {
                                libc::kill(child_process.id() as i32, signals.exit);
                            }
                            debug(format!("程序收到重启指令，退出程序并重启,使用 {} 信号量.", signals.exit));
                            restart = true;
                            break 'l;
                        }
                        EXIT => {
                            unsafe {
                                libc::kill(child_process.id() as i32, signals.exit);
                            }
                            debug(format!("程序收到退出指令，退出程序,使用 {} 信号量.", signals.exit));
                            break 'e;
                        }
                        _ => {}
                    }
                }
            }
        }
        {
            if let Ok(mut lock) = arc.lock() {
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
    ) -> Self {
        let arc = Arc::new(Mutex::new(CREATED));
        let to_master: (SyncSender<ChildThreadAction>, Receiver<ChildThreadAction>) = mpsc::sync_channel(255);
        let arc_clone = Arc::clone(&arc);
        let to_signals = signals.clone();

        thread::spawn(move || {
            Self::thread_fun(
                to_master.1,
                binary,
                args,
                envs,
                arc_clone,
                to_signals,
            )
        });
        let worker = StableWorker {
            master_rx: to_master.0,
            status: arc,
        };
        return worker;
    }
}
