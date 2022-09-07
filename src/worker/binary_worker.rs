use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::time::Duration;
use nonblock::NonBlockingReader;
use crate::log::{error, info, info_str, trace, trace_str};
use crate::worker::binary_worker::CallbackAction::EXITED;
use crate::worker::binary_worker::ChildThreadAction::START;

pub struct StableWorker {
    pub master_tx: Receiver<CallbackAction>,
    pub master_rx: SyncSender<ChildThreadAction>,
}


#[derive(PartialEq, Debug)]
pub enum ChildThreadAction {
    START,
    STOP,
    EXIT,
    RESTART,
}

#[derive(PartialEq, Debug)]
pub enum CallbackAction {
    INITED,
    STARTED,
    FailExited(i32),
    EXITED,
}

impl StableWorker {
    pub fn start(&self) {
        self.master_rx.send(START).unwrap();
    }
    pub fn new_status(&self) -> Option<CallbackAction> {
        let mut last = None;
        loop {
            if let Ok( result) = self.master_tx.try_recv(){
                if let Some(msg) = last {
                    trace(format!("丢弃消息 {:?}.",msg));
                }
                last = Some(result);
            }else {
                break last;
            }
        }
    }
    fn thread_fun(nio_tx: SyncSender<CallbackAction>,
                  nio_rx: Receiver<ChildThreadAction>,
                  binary: String,
                  args: Vec<String>,
                  envs: HashMap<String, String>) {
        let mut restart = false;
        'e: loop {
            if !restart {
                let action = nio_rx.recv().unwrap();
                match action {// 等待启动指令
                    START => {}
                    ChildThreadAction::EXIT => break,
                    _ => continue
                }
            }
            restart = false;
            info_str("子进程开始启动.");
            nio_tx.send(CallbackAction::INITED).unwrap();
            let child_process = Command::new(&binary)
                .current_dir(PathBuf::from(&binary).parent().unwrap())
                .args(&args).envs(&envs).stdout(Stdio::piped())
                .stderr(Stdio::piped()).spawn();
            if let Err(e) = child_process {
                nio_tx.send(CallbackAction::FailExited(1)).unwrap();
                error(format!("项目启动错误！{}", e.to_string()));
                continue;
            }
            let mut child_process = child_process.unwrap();
            nio_tx.send(CallbackAction::STARTED).unwrap();
            info_str("开始抓取进程标准输出信息.");
            let mut stdout = NonBlockingReader::from_fd(child_process.stdout.take().unwrap()).unwrap();
            let mut stderr = NonBlockingReader::from_fd(child_process.stderr.take().unwrap()).unwrap();
            let mut buffer = vec![];
            'l: loop {
                if let Ok(Some(code)) = child_process.try_wait() {
                    let i = code.code().unwrap();
                    if i != 0 {
                        info_str("程序异常退出");
                        nio_tx.send(CallbackAction::FailExited(i)).unwrap();
                    } else {
                        info_str("程序正常退出");
                        nio_tx.send(EXITED).unwrap();
                    }
                    break;
                }
                trace_str("开始搜集响应日志.");
                thread::sleep(Duration::from_secs(1));
                buffer.clear();
                let stdout_size = stdout.read_available(&mut buffer).unwrap();
                trace_str("抓取标准日志.");
                info(String::from_utf8_lossy(&buffer[..stdout_size]).to_string());
                buffer.clear();
                trace_str("抓取错误日志.");
                let stderr_size = stderr.read_available(&mut buffer).unwrap();
                error(String::from_utf8_lossy(&buffer[..stderr_size]).to_string());
                buffer.clear();
                trace_str("抓取日志完成.");
                if let Ok(e) = nio_rx.try_recv() {
                    match e {
                        ChildThreadAction::STOP => {
                            child_process.kill().unwrap();
                            break 'l;
                        }
                        ChildThreadAction::RESTART => {
                            child_process.kill().unwrap();
                            restart = true;
                            break 'l;
                        }
                        ChildThreadAction::EXIT => {
                            child_process.kill().unwrap();
                            break 'e;
                        }
                        _ => {}
                    }
                }
            }
        }
        nio_tx.send(EXITED).unwrap();
    }
    pub fn new(
        binary: String,
        args: Vec<String>,
        envs: HashMap<String, String>,
    ) -> Self {
        let to_worker: (SyncSender<CallbackAction>, Receiver<CallbackAction>) = mpsc::sync_channel(255);
        let to_master: (SyncSender<ChildThreadAction>, Receiver<ChildThreadAction>) = mpsc::sync_channel(255);
        thread::spawn(|| Self::thread_fun(
            to_worker.0, to_master.1,
            binary, args, envs,
        ));
        StableWorker {
            master_tx: to_worker.1,
            master_rx: to_master.0,
        }
    }
}
