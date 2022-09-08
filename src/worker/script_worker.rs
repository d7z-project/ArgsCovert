use std::collections::HashMap;
use std::path::PathBuf;

pub struct ScriptWorker {}

pub enum ScriptStatus {
    INIT,
    START,
    EXIT(i32),
    DESTROY,
}

/**
脚本任务脚本加载器
 **/
impl ScriptWorker {
    fn new(
        work_path: &PathBuf,
        script: String,
        parser: String,
        env: HashMap<String, String>,
        is_loop: bool,
        delay:usize,
        interval:usize,
    ) -> Self {
        todo!()
    }
}
