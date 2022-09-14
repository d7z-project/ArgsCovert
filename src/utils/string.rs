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

use regex::Regex;

pub fn _replace_range(src: &mut String, old: &str, new: &str) {
    'l: loop {
        if let Some(index) = src.find(old) {
            src.replace_range(index..(index + old.len()), new);
        } else {
            break 'l;
        }
    }
}

//noinspection ALL,DuplicatedCode
pub fn replace_all_str_from_map(src: &mut String, data: &HashMap<String, String>) {
    for (key, value) in data.iter() {
        'l: loop {
            if let Some(index) = src.find(key) {
                src.replace_range(index..(index + key.len()), value.as_str());
            } else {
                break 'l;
            }
        }
    }
}

//noinspection ALL,DuplicatedCode
pub fn replace_all_str(src: &mut String, data: &Vec<(String, String)>) {
    for (key, value) in data {
        'l: loop {
            if let Some(index) = src.find(key) {
                src.replace_range(index..(index + key.len()), value.as_str());
            } else {
                break 'l;
            }
        }
    }
}

#[test]
fn get_value_from_exp_test() {
    let map: HashMap<String, String> = vec![
        ("key1", "value1"),
        ("key2", "value2"),
        ("key3", "value3"),
        ("key4", "value4"),
        ("key5", "value5"),
        ("key6", "value6"),
    ]
    .iter()
    .map(|e| (e.0.to_string(), e.1.to_string()))
    .collect();
    assert_eq!(
        get_value_from_exp("{{key1}}", &map),
        Some("value1".to_string())
    );
    assert_eq!(get_value_from_exp("{{no_key1}}", &map), None);
    assert_eq!(
        get_value_from_exp("{{key10 ? key1}}", &map),
        Some("value1".to_string())
    );
}

/// 替换内部变量，如果失败则返回空
pub fn get_value_from_exp(exp: &str, vars: &HashMap<String, String>) -> Option<String> {
    let variable_regex = Regex::new("\\{\\{\\w.*?}}").unwrap();
    let get_envs_value = |key: &str| -> Option<String> {
        for item in key.split("?") {
            let item = item.trim();
            if item.is_empty() {
                return Some("".to_string());
            } else if let Some(item) = vars.get(item) {
                return Some(item.to_string());
            }
        }
        None
    };
    let find_vars: HashMap<String, Option<String>> = variable_regex
        .find_iter(exp)
        .map(|e| {
            (
                exp[e.start()..e.end()].trim().to_string(),
                Some(exp[e.start() + 2..e.end() - 2].trim())
                    .filter(|v| v.is_empty().not())
                    .map(|v| get_envs_value(v))
                    .unwrap_or(None),
            )
        })
        .filter(|e| e.0.is_empty().not())
        .collect();
    for (_, v) in &find_vars {
        if v.is_none() {
            return None;
        }
    }
    let find_vars: HashMap<String, String> = find_vars
        .iter()
        .map(|(first, second)| (first.to_string(), second.to_owned().unwrap()))
        .collect();
    let mut exp = exp.to_string();
    replace_all_str_from_map(&mut exp, &find_vars);
    Some(exp)
}
