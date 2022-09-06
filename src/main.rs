use crate::config::project_conf::load_info;

mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = load_info()?;
    println!("{:?}", result);
    Ok(())
}
