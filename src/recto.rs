use std::collections::BTreeMap;

use verso::{weave, Fragment};

use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        eprintln!(
            "
Hint: The 'verso' and 'recto' tools are meant to be used together, like this:

    verso main.rs lib.rs | recto build chap1.tex chap2.tex blog/home.md
    #     ^       ^              ^     ^         ^         ^
    #     +-------+              |     +---------+---------+
    #     |                      |                         |
    #     +--- Source files      +--- Output directory     +--- Prose files
"
        );
        process::exit(1);
    });

    if let Err(e) = run(config) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub filenames: Vec<String>,
    pub out_dir: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("Expected at least two arguments");
        }

        let out_dir = String::from(&args[1]);
        let filenames = args[2..].to_vec();

        Ok(Config { out_dir, filenames })
    }
}

pub fn run(cfg: Config) -> Result<(), Box<dyn Error>> {
    // Read annotations from stdin, and index by ID.
    let mut annotations = BTreeMap::new();
    {
        // Read the annotations into the map in a block to reduce memory pressure.
        let raw_annotations: Vec<Fragment> = serde_json::from_reader(io::stdin())?;

        for ann in raw_annotations {
            annotations.insert(ann.id.to_owned(), ann.to_owned());
            eprintln!("Read annotation {}", ann.id);
        }
    }

    eprintln!("Creating results in directory '{}'...", &cfg.out_dir);
    fs::create_dir_all(&cfg.out_dir)?;

    for filename in cfg.filenames {
        eprintln!("Expanding annotations in '{}'...", &filename);

        // TODO Improve error messages.
        let contents = fs::read_to_string(&filename)?;

        // Add annotations into the text body and emit to out directory
        let woven_body = weave(&filename, &contents, &annotations)?;
        let out_file = Path::new(&cfg.out_dir).join(&filename);

        // Create subdirectories if needed.
        if let Some(out_subdir) = out_file.parent() {
            fs::create_dir_all(&out_subdir)?
        }

        eprintln!("Writing result to {:?}...", out_file);
        fs::write(out_file, woven_body)?;
    }

    Ok(())
}
