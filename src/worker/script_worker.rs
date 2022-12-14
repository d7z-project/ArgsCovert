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
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::{fs, thread};

use crate::lib::SoftError;
use crate::log::{debug, error};
use crate::utils::file::new_temp_path;
use crate::worker::script_worker::WorkerAction::{EXIT, START, STOP};
use crate::{debug_str, log};

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
        debug(format!("??????????????????."));
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
    pub name: String,
}

/**
???????????????????????????
 **/
impl ScriptWorker {
    fn thread_fun(info: ScriptThreadInfo) {
        'a: loop {
            if let Ok(i) = info.receiver.try_recv() {
                if i == EXIT {
                    break;
                } else if i == STOP {
                    debug_str("???????????????????????????????????????");
                    'b: loop {
                        if let Ok(data) = info.receiver.try_recv() {
                            match data {
                                START => {
                                    debug_str("??????????????????");
                                    break 'b;
                                }
                                STOP => continue,
                                EXIT => break 'a,
                            };
                        } else {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                    debug_str("??????????????????");
                    thread::sleep(Duration::from_secs(info.delay as u64));
                } else if i == START {
                    debug_str("??????????????????");
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
                        "{} ?????????????????? - {} => \n {}",
                        &info.name, &info.script_path, x
                    ));
                }
                for x in Some(String::from_utf8_lossy(&data.stderr).to_string())
                    .filter(|e| e.is_empty().not())
                {
                    error(format!(
                        "{} ?????????????????? - {} => \n {}",
                        &info.name, &info.script_path, x
                    ));
                }

                if data.status.code().unwrap_or(-1) != 0 {
                    debug(format!("{}???????????????????????????????????????", info.name));
                    info.sender.send(1).unwrap();
                } else {
                    debug(format!("{}????????????????????????????????????", info.name));
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
