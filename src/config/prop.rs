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
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use libc::{SIGHUP, SIGKILL, SIGTERM};
use serde::{Deserialize, Serialize};

use args_tools::SoftError;

use crate::config::prop::RestartPolicy::ALWAYS;
use crate::config::prop::SourceKeyMode::ARG;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    #[serde(default = "default_args_vec")]
    pub args: Vec<ProjectArgs>,
    #[serde(default = "default_vec")]
    pub path: Vec<String>,
    #[serde(default = "def_log")]
    pub log: ProjectLog,
    #[serde(default = "default_map")]
    pub attach: HashMap<String, String>,
    #[serde(default = "default_alias")]
    pub config_alias: Vec<ProjectConfigAlias>,
}

fn default_alias() -> Vec<ProjectConfigAlias> {
    vec![]
}

fn def_log() -> ProjectLog {
    serde_yaml::from_str("").unwrap()
}

fn default_args_vec() -> Vec<ProjectArgs> {
    vec![]
}

fn default_vec() -> Vec<String> {
    vec![]
}

fn default_map() -> HashMap<String, String> {
    HashMap::new()
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ProjectConfigAlias {
    pub key: String,
    pub expr: Vec<String>,
    #[serde(default = "def_over")]
    pub over: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ProjectLog {
    #[serde(default = "def_console")]
    pub console: ConsoleLog,
    #[serde(default = "def_file")]
    pub file: FileLog,
}

fn def_console() -> ConsoleLog {
    serde_yaml::from_str("").unwrap()
}

fn def_file() -> FileLog {
    serde_yaml::from_str("").unwrap()
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ConsoleLog {
    #[serde(default = "console_log_level")]
    pub level: LoggerLevel,
}

fn console_log_level() -> LoggerLevel {
    LoggerLevel::NONE
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct FileLog {
    #[serde(default = "file_log_level")]
    pub level: LoggerLevel,
    #[serde(default = "file_log_path")]
    pub path: String,
    #[serde(default = "file_log_path")]
    pub error_path: String,
    #[serde(default = "bool_enable")]
    pub append: bool,
}

fn bool_enable() -> bool {
    true
}

fn file_log_path() -> String {
    "/tmp/test.log".to_string()
}

fn file_log_level() -> LoggerLevel {
    LoggerLevel::NONE
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub enum LoggerLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
    NONE,
}

pub trait LogLevelId {
    fn id(&self) -> u8;
}

impl Display for LoggerLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            LoggerLevel::TRACE => "TRACE",
            LoggerLevel::DEBUG => "DEBUG",
            LoggerLevel::INFO => "INFO",
            LoggerLevel::WARN => "WARN",
            LoggerLevel::ERROR => "ERROR",
            LoggerLevel::NONE => "NONE",
        };
        f.write_str(x)
    }
}

impl FromStr for LoggerLevel {
    type Err = SoftError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TRACE" => Ok(LoggerLevel::TRACE),
            "DEBUG" => Ok(LoggerLevel::DEBUG),
            "INFO" => Ok(LoggerLevel::INFO),
            "WARN" => Ok(LoggerLevel::WARN),
            "ERROR" => Ok(LoggerLevel::ERROR),
            "NONE" => Ok(LoggerLevel::NONE),
            _ => Err(SoftError::AppError("unknown logger level".to_string())),
        }
    }
}

impl LogLevelId for LoggerLevel {
    fn id(&self) -> u8 {
        match self {
            LoggerLevel::TRACE => 0,
            LoggerLevel::DEBUG => 1,
            LoggerLevel::INFO => 2,
            LoggerLevel::WARN => 3,
            LoggerLevel::ERROR => 4,
            LoggerLevel::NONE => 5,
        }
    }
}

fn def_over() -> bool {
    true
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ProjectArgs {
    pub key: String,
    pub expr: Vec<String>,
    #[serde(default = "def_mode")]
    pub mode: SourceKeyMode,
    #[serde(default = "def_false")]
    pub must: bool,
    #[serde(default = "empty_str")]
    pub valid_regex: String,
    #[serde(default = "error_message")]
    pub valid_message: String,
}

fn error_message() -> String {
    "参数 '{{key}}' 的值 '{{value}}'校验失败。".to_string()
}

fn def_mode() -> SourceKeyMode {
    ARG
}

fn def_false() -> bool {
    true
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub struct SoftSignals {
    #[serde(default = "i32_data_1")]
    pub reload: i32,
    #[serde(default = "i32_data_15")]
    pub exit: i32,
    #[serde(default = "i32_data_9")]
    pub kill: i32,
}

fn i32_data_1() -> i32 {
    SIGHUP
}

fn i32_data_15() -> i32 {
    SIGTERM
}

fn i32_data_9() -> i32 {
    SIGKILL
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub enum SourceKeyMode {
    ARG,
    ENV,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ProjectInfo {
    pub name: String,
    pub binary: String,
    #[serde(default = "empty_str")]
    pub before_script: String,
    #[serde(default = "empty_str")]
    pub after_script: String,
    #[serde(default = "def_check_health")]
    pub check_health: HealthCheck,
    #[serde(default = "def_check_started")]
    pub check_started: StartedCheck,
    #[serde(default = "def_signals")]
    pub signals: SoftSignals,
    #[serde(default = "def_restart_policy")]
    pub restart_policy: RestartPolicy,
    #[serde(default = "bash_str")]
    pub script_worker: String,
}

fn def_signals() -> SoftSignals {
    serde_yaml::from_str("").unwrap()
}

fn def_check_health() -> HealthCheck {
    serde_yaml::from_str("").unwrap()
}

fn def_check_started() -> StartedCheck {
    serde_yaml::from_str("").unwrap()
}

fn def_restart_policy() -> RestartPolicy {
    ALWAYS
}

fn bash_str() -> String {
    "bash".to_string()
}

fn empty_str() -> String {
    "".to_string()
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct HealthCheck {
    #[serde(default = "def_script")]
    pub script: String,
    #[serde(default = "usize_zero")]
    pub delay: usize,
    #[serde(default = "usize_zero")]
    pub interval: usize,
    #[serde(default = "u16_zero")]
    pub failures: u16,
}

fn def_script() -> String {
    "".to_string()
}

fn u16_zero() -> u16 {
    0
}

fn usize_zero() -> usize {
    0
}

///重启策略
#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub enum RestartPolicy {
    /// 不重启
    NONE,
    /// 总是重启
    ALWAYS,
    /// 失败重启
    FAIL,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct StartedCheck {
    #[serde(default = "def_script")]
    pub script: String,
    #[serde(default = "usize_zero")]
    pub interval: usize,
    #[serde(default = "usize_zero")]
    pub success: usize,
    #[serde(default = "def_script")]
    pub started_script: String,
}
