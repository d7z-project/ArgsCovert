use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn new_temp_path(name: &str) -> PathBuf {
    let system_time = SystemTime::now();
    let duration = system_time.duration_since(UNIX_EPOCH).unwrap();
    Path::new(env::temp_dir().as_path()).join(format!("{}-{:?}.sh", name, duration))
}
