use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args[1].as_str();
    match command {
        "init" => {
            init().unwrap();
        }
        _ => {}
    };
}

fn init() -> std::io::Result<()> {
    fs::create_dir_all(".rgit/objects")?;
    Ok(())
}
