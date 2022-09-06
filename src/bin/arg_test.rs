use std::env;
use std::env::{args};

fn main() {
    println!("传入的参数有：");
    args().for_each(|e| println!("{}", e));
    println!("传入的环境变量有：");
    env::vars().for_each(|e| println!("{}={}", e.0, e.1));
    println!("当前的工作目录为 :{}", env::current_dir().unwrap().to_str().unwrap());
    println!("程序位于 :{}", env::current_exe().unwrap().to_str().unwrap());
}
