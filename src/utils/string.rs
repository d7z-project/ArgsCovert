#[warn(dead_code)]
pub fn replace_range(src: &mut String, old: &str, new: &str) {
    'l: loop {
        if let Some(index) = src.find(old) {
            src.replace_range(index..(index + old.len()), new);
        } else {
            break 'l;
        }
    }
}

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
