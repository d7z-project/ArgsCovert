use crate::lib::SoftError;
use crate::log::error;
use crate::{log, new_temp_path};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

pub fn execute_script(
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
    log::info(format!(
        "脚本标准输出 - {:?} => \n {}",
        &buf,
        String::from_utf8_lossy(&output.stdout).to_string()
    ));
    error(format!(
        "脚本错误输出 - {:?} => \n {}",
        &buf,
        String::from_utf8_lossy(&output.stderr).to_string()
    ));
    output
        .status
        .code()
        .ok_or(SoftError::AppError("其他错误".to_string()))
}
