use std::{env, fs};
use std::fs::{File, OpenOptions};
use crate::config::project_conf::{LoggerLevel, LogLevelId, ProjectConfig};
use crate::config::project_conf::LoggerLevel::{DEBUG, ERROR, INFO, NONE, TRACE, WARN};
use std::io::Write;
use std::ops::Not;
use std::path::{Path, PathBuf};

struct LoggerInfo {
    pub console_level: LoggerLevel,
    pub file_level: LoggerLevel,
    pub file_path: File,
    error_file_path: File,
}

static mut LOG_INFO: Option<LoggerInfo> = None;

pub fn debug(data: String) {
    _output(DEBUG, &data);
}

pub fn trace_str(data: &str) {
    _output(TRACE, data);

}
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
        if let Some(data) = &LOG_INFO {
            if data.console_level.id() <= level.id() {
                if data.console_level.id() > WARN.id() {
                    eprintln!("{:?} - {}", level, message);
                } else {
                    println!("{:?} - {}", level, message);
                }
            }
            if data.file_level.id() <= level.id() {
                if level.id() >= WARN.id() {
                    if let Err(_) = writeln!(&data.error_file_path, "{:?} - {}", level, message) {}
                } else {
                    if let Err(_) = writeln!(&data.file_path, "{:?} - {}", level, message) {}
                }
            }
        }
    }
}

pub fn log_default() {
    unsafe {
        let mut buf = env::temp_dir();
        buf.push("test.log");
        LOG_INFO = Some(LoggerInfo {
            file_level: INFO,
            console_level: NONE,
            file_path: File::create(&buf).unwrap(),
            error_file_path: File::create(&buf).unwrap(),
        });
    }
}

pub fn log_init(
    soft_config: &ProjectConfig,
) {
    let file_path = &PathBuf::from(&soft_config.log.file.path);
    let error_file_path = &PathBuf::from(&soft_config.log.file.error_path);
    if (soft_config.log.file.append).not() {
        if file_path.is_file() {
            fs::remove_file(file_path).expect("日志文件无法写入！");
        }
        if error_file_path.is_file() {
            fs::remove_file(error_file_path).expect("日志文件无法写入！");
        }
    }
    let path = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true) // This is needed to append to file
        .open(file_path)
        .unwrap();
    let error_path = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true) // This is needed to append to file
        .open(error_file_path)
        .unwrap();
    unsafe {
        LOG_INFO = Some(LoggerInfo {
            file_level: soft_config.log.file.level.clone(),
            console_level: soft_config.log.console.level.clone(),
            file_path: path,
            error_file_path: error_path,
        });
    }
}
