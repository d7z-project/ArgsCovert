use crate::lib::SoftError;
use crate::log::error;
use crate::utils::string::not_blank_then;
use crate::{log, new_temp_path};
use std::collections::HashMap;
use std::fs;
use std::ops::Not;
use std::process::Command;

pub fn execute_script(
    name: &str,
    worker: &str,
    script: &str,
    envs: &HashMap<String, String>,
) -> Result<i32, SoftError> {
    let buf = new_temp_path("temp_script");
    fs::write(&buf, script)?;
    let output = Command::new(worker.clone())
        .arg(&buf.to_str().unwrap().to_string())
        .envs(envs)
        .output()?;
    for x in
        Some(String::from_utf8_lossy(&output.stdout).to_string()).filter(|e| e.is_empty().not())
    {
        log::info(format!("任务 {} 标准输出 - {:?} => \n {}", name, &buf, x,));
    }
    for x in
        Some(String::from_utf8_lossy(&output.stderr).to_string()).filter(|e| e.is_empty().not())
    {
        log::info(format!("任务 {} 错误输出 - {:?} => \n {}", name, &buf, x,));
    }

    output
        .status
        .code()
        .ok_or(SoftError::AppError("其他错误".to_string()))
}
