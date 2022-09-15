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

use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::ops::Not;
use std::path::PathBuf;

use chrono::Local;

use crate::config::prop::LoggerLevel::{DEBUG, ERROR, INFO, NONE, TRACE, WARN};
use crate::config::prop::{LogLevelId, LoggerLevel, ProjectConfig};

struct LoggerInfo {
    pub console_level: LoggerLevel,
    pub file_level: LoggerLevel,
    pub file_path: Option<File>,
    error_file_path: Option<File>,
}

static mut LOG_INFO: Option<LoggerInfo> = None;

pub fn debug_str(data: &str) {
    _output(DEBUG, data);
}

pub fn debug(data: String) {
    _output(DEBUG, &data);
}

pub fn trace_str(data: &str) {
    _output(TRACE, data);
}

#[allow(dead_code)]
pub fn trace(data: String) {
    _output(TRACE, &data);
}

pub fn info_str(data: &str) {
    _output(INFO, data);
}

pub fn info(data: String) {
    _output(INFO, &data);
}

pub fn warn(data: String) {
    _output(WARN, &data);
}

pub fn error_str(data: &str) {
    _output(ERROR, data);
}

pub fn error(data: String) {
    _output(ERROR, &data);
}

fn _output(level: LoggerLevel, message: &str) {
    let message = message.trim();
    unsafe {
        let date = Local::now();
        let time = date.format("%Y/%m/%d %H:%M:%S%.3f").to_string();
        if let Some(data) = &LOG_INFO {
            if data.console_level.id() <= level.id() {
                if data.console_level.id() > WARN.id() {
                    eprintln!("{} - {:?} - {}", time, level, message);
                } else {
                    println!("{} - {:?} - {}", time, level, message);
                }
            }
            if data.file_level.id() <= level.id() {
                if level.id() >= WARN.id() {
                    for mut x in &data.error_file_path {
                        if let Err(_) = writeln!(x, "{} - {:?} - {}", time, level, message) {}
                    }
                } else {
                    for mut x in &data.file_path {
                        if let Err(_) = writeln!(x, "{} - {:?} - {}", time, level, message) {}
                    }
                }
            }
        }
    }
}

pub fn log_default(level: LoggerLevel) {
    unsafe {
        LOG_INFO = Some(LoggerInfo {
            file_level: level,
            console_level: NONE,
            file_path: None,
            error_file_path: None,
        });
    }
}

pub fn log_init(soft_config: &ProjectConfig) {
    let file_path = Some(&soft_config.log.file.path)
        .filter(|e| e.is_empty().not())
        .map(|e| PathBuf::from(e));
    let error_file_path = Some(&soft_config.log.file.error_path)
        .filter(|e| e.is_empty().not())
        .map(|e| PathBuf::from(e));

    for x in &file_path {
        if soft_config.log.file.append.not() && x.is_file() {
            fs::remove_file(x).ok();
        }
    }

    for x in &error_file_path {
        if soft_config.log.file.append.not() && x.is_file() {
            fs::remove_file(x).ok();
        }
    }

    let file_path = file_path
        .filter(|_| soft_config.log.file.level != NONE)
        .map(|e| {
            OpenOptions::new()
                .create(true)
                .append(true) // This is needed to append to file
                .write(true)
                .open(e)
                .ok()
        })
        .filter(|e| e.is_some())
        .map(|e| e.unwrap());
    let error_path = error_file_path
        .filter(|_| soft_config.log.file.level.id() < WARN.id())
        .map(|e| {
            OpenOptions::new()
                .create(true)
                .write(true)
                .append(true) // This is needed to append to file
                .open(e)
                .ok()
        })
        .filter(|e| e.is_some())
        .map(|e| e.unwrap());
    if file_path.is_none() || file_path.is_none() {
        eprintln!("无法写入标准/错误日志，请配置日志写入位置，如已配置，请检查日志文件权限.")
    }

    unsafe {
        LOG_INFO = Some(LoggerInfo {
            file_level: soft_config.log.file.level.clone(),
            console_level: soft_config.log.console.level.clone(),
            file_path,
            error_file_path: error_path,
        });
    }
}
