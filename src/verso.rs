use serde_json;

use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::process;

use verso::{extract_fragments, Fragment};

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
    let mut annotations: Vec<Fragment> = vec![];

    // Do the read and print in separate passes to enable clean error messages.
    for filename in cfg.filenames {
        let contents = fs::read_to_string(filename.clone())?;
        let mut fragments = extract_fragments(contents, filename)?;
        annotations.append(&mut fragments);
    }

    serde_json::to_writer(io::stdout(), &annotations)?;

    Ok(())
}
