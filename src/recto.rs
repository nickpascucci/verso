use std::env;
use std::error::Error;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(e) = run(config) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub filenames: Vec<String>,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        let filenames = args[1..].iter().cloned().collect();

        Ok(Config { filenames })
    }
}

pub fn run(cfg: Config) -> Result<(), Box<dyn Error>> {
    Ok(())
}
