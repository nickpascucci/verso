use std::collections::HashMap;

use verso::{Fragment};

use std::io;
use std::fs;
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
    pub out_dir: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("Expected at least two arguments");
        }

        let out_dir = args[1].to_owned();
        let filenames = args[2..].iter().cloned().collect();

        Ok(Config { out_dir, filenames })
    }
}

pub fn run(cfg: Config) -> Result<(), Box<dyn Error>> {

    // Read annotations from stdin, and index by ID.
    let mut annotations = HashMap::new();
    { // Read the annotations into the map in a block to reduce pressure.
        let raw_annotations: Vec<Fragment> = serde_json::from_reader(io::stdin())?;

        for ann in raw_annotations {
            annotations.insert(ann.id.to_owned(), ann.to_owned());
            println!("Read annotation {}", ann.id);
        }
    }

    for filename in cfg.filenames {
        // TODO Improve error messages.
        let contents = fs::read_to_string(filename.clone())?;
        // Add annotations into the text body and emit to out directory
    }


    Ok(())
}
