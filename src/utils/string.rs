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

pub fn not_blank_then(data: String, func: &fn(String)) {
    if data.trim().is_empty().not() {
        func(data)
    }
}
