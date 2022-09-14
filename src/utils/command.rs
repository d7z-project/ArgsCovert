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
use std::fs;
use std::ops::Not;
use std::process::Command;

use crate::lib::SoftError;
use crate::log::error;
use crate::utils::string::not_blank_then;
use crate::{log, new_temp_path};

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
