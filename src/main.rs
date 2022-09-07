use crate::config::project_conf::load_info;
use crate::config::soft_args::{SoftArgs, SoftStaticArgs};
use clap::Parser;

mod config;
mod lib;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: SoftStaticArgs = SoftStaticArgs::parse();
    let args1 = SoftArgs::parse();
    let soft_config = load_info(&args.config_path, &args1.variable)?;
    println!("{:?}", soft_config);
    Ok(())
}
