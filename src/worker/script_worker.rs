use crate::lib::SoftError;
use crate::log::{debug, error};
use crate::utils::file::new_temp_path;
use crate::worker::script_worker::WorkerAction::{EXIT, START, STOP};
use crate::{debug_str, log};
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Not;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, SyncSender};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::{env, fs, thread};

pub struct ScriptWorker {
    receiver: Receiver<usize>,
    sender: SyncSender<WorkerAction>,
    exited: Arc<AtomicBool>,
}

impl ScriptWorker {
    pub(crate) fn stop(&self) {
        self.sender.send(STOP).unwrap();
    }
    pub(crate) fn start(&self) {
        self.sender.send(START).unwrap();
    }
}

impl ScriptWorker {
    pub(crate) fn get_status(&self) -> Vec<usize> {
        let mut vec = vec![];
        loop {
            if let Ok(item) = self.receiver.try_recv() {
                vec.push(item);
            } else {
                return vec;
            }
        }
    }
}

impl ScriptWorker {
    pub fn close(&self) {
        debug(format!("退出脚本任务."));
        self.sender.send(EXIT).unwrap();
    }
    pub fn wait_closed(&self) {
        while self.exited.load(Ordering::Relaxed).not() {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum WorkerAction {
    START,
    STOP,
    EXIT,
}

struct ScriptThreadInfo {
    pub interpreter: String,
    pub envs: HashMap<String, String>,
    pub delay: usize,
    pub interval: usize,
    pub sender: SyncSender<usize>,
    pub script_path: String,
    pub exited: Arc<AtomicBool>,
    receiver: Receiver<WorkerAction>,
    name: String,
}

/**
脚本任务脚本加载器
 **/
impl ScriptWorker {
    fn thread_fun(info: ScriptThreadInfo) {
        'a: loop {
            if let Ok(i) = info.receiver.try_recv() {
                if i == EXIT {
                    break;
                } else if i == STOP {
                    debug_str("检测任务暂停，等待下次唤醒");
                    'b: loop {
                        if let Ok(data) = info.receiver.try_recv() {
                            match data {
                                START => {
                                    debug_str("收到开始指令");
                                    break 'b;
                                }
                                STOP => continue,
                                EXIT => break 'a,
                            };
                        } else {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                    debug_str("任务已被唤醒");
                    thread::sleep(Duration::from_secs(info.delay as u64));
                } else if i == START {
                    debug_str("任务已被唤醒");
                    thread::sleep(Duration::from_secs(info.delay as u64));
                }
            }
            if let Ok(data) = Command::new(&info.interpreter)
                .arg(&info.script_path)
                .envs(&info.envs)
                .output()
            {
                for x in Some(String::from_utf8_lossy(&data.stdout).to_string())
                    .filter(|e| e.is_empty().not())
                {
                    log::info(format!(
                        "{} 任务标准输出 - {} => \n {}",
                        &info.name, &info.script_path, x
                    ));
                }
                for x in Some(String::from_utf8_lossy(&data.stderr).to_string())
                    .filter(|e| e.is_empty().not())
                {
                    error(format!(
                        "{} 任务错误输出 - {} => \n {}",
                        &info.name, &info.script_path, x
                    ));
                }

                if data.status.code().unwrap_or(-1) != 0 {
                    debug_str("执行完成，但状态异常。");
                    info.sender.send(1).unwrap();
                } else {
                    debug_str("脚本执行完成，退出正常。");
                    info.sender.send(0).unwrap();
                }
            }

            thread::sleep(Duration::from_secs(info.interval as u64));
        }
        info.exited.swap(true, Ordering::Release);
    }
    pub fn new(
        name: &str,
        interpreter: &String,
        script: &String,
        envs: &HashMap<String, String>,
        delay: usize,
        interval: usize,
    ) -> Result<Self, SoftError> {
        let name = name.to_string();
        let to_master: (SyncSender<usize>, Receiver<usize>) = mpsc::sync_channel(255);
        let to_thread: (SyncSender<WorkerAction>, Receiver<WorkerAction>) = mpsc::sync_channel(255);
        let worker_script_path = new_temp_path("args-worker-script");
        fs::write(&worker_script_path, script)?;
        let to_master_sender = to_master.0;
        let to_master_receiver = to_master.1;
        let to_thread_sender = to_thread.0;
        let to_thread_receiver = to_thread.1;
        let script_path = worker_script_path.as_path().to_str().unwrap().to_string();
        let arc = Arc::new(AtomicBool::new(false));
        let arc1 = Arc::clone(&arc);
        let worker = ScriptThreadInfo {
            name,
            exited: arc,
            interpreter: interpreter.clone(),
            envs: envs.clone(),
            delay,
            interval,
            sender: to_master_sender,
            receiver: to_thread_receiver,
            script_path,
        };
        thread::spawn(|| -> () { Self::thread_fun(worker) });
        Ok(ScriptWorker {
            receiver: to_master_receiver,
            sender: to_thread_sender,
            exited: arc1,
        })
    }
}
