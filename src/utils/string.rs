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
