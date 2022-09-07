use crate::binary::args_builder::load_context;
use crate::config::project_conf::load_info;
use crate::config::soft_args::{SoftArgs};

mod config;
mod lib;
mod utils;
mod binary;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args1 = SoftArgs::parse(); // 拉取参数
    let soft_config = load_info(&args1.config_path, &args1.variable)?; // 加载系统配置
    let vec = load_context(&soft_config);
    Ok(())
}
