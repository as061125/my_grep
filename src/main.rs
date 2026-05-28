use std::env;
use grep_redo::{Config, search};

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("参数错误: {err}");
        std::process::exit(1);
    });

    if let Err(e) = search::run(config) {
        eprintln!("读取错误: {e}");
        std::process::exit(1);
    }
}
