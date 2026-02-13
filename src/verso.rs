use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::process;

use verso::SymbolKey;
use verso::{extract_fragments, Fragment};

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

Options:
    --annotations-file <path>   Read additional annotations from a JSON file.
                                Extracted annotations take precedence on ID conflict.
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
    pub annotations_file: Option<String>,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        let mut filenames = Vec::new();
        let mut annotations_file = None;
        let mut i = 1;

        while i < args.len() {
            if args[i] == "--annotations-file" {
                i += 1;
                if i >= args.len() {
                    return Err("--annotations-file requires a path argument");
                }
                annotations_file = Some(args[i].clone());
            } else {
                filenames.push(args[i].clone());
            }
            i += 1;
        }

        Ok(Config {
            filenames,
            annotations_file,
        })
    }
}

pub fn run(cfg: Config) -> Result<(), Box<dyn Error>> {
    // If an annotations file is provided, load it as the base set.
    let mut annotations_map: BTreeMap<String, Fragment> = BTreeMap::new();

    if let Some(ref ann_path) = cfg.annotations_file {
        let ann_contents = fs::read_to_string(ann_path)?;
        let file_annotations: Vec<Fragment> = serde_json::from_str(&ann_contents)?;
        for ann in file_annotations {
            annotations_map.insert(ann.id.clone(), ann);
        }
    }

    // Extract annotations from source files. These overwrite file-loaded ones on ID conflict.
    let symbols = SymbolKey::from_environment();
    for filename in cfg.filenames {
        let contents = fs::read_to_string(&filename)?;
        let fragments = extract_fragments(&contents, &filename, &symbols)?;
        for frag in fragments {
            annotations_map.insert(frag.id.clone(), frag);
        }
    }

    let annotations: Vec<Fragment> = annotations_map.into_values().collect();
    serde_json::to_writer(io::stdout(), &annotations)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_config_no_flags() {
        let cfg = Config::new(&args(&["verso", "a.rs", "b.rs"])).unwrap();
        assert_eq!(cfg.filenames, vec!["a.rs", "b.rs"]);
        assert_eq!(cfg.annotations_file, None);
    }

    #[test]
    fn test_config_with_annotations_file() {
        let cfg =
            Config::new(&args(&["verso", "--annotations-file", "ann.json", "a.rs"])).unwrap();
        assert_eq!(cfg.filenames, vec!["a.rs"]);
        assert_eq!(cfg.annotations_file, Some("ann.json".to_string()));
    }

    #[test]
    fn test_config_annotations_file_at_end() {
        let cfg =
            Config::new(&args(&["verso", "a.rs", "--annotations-file", "ann.json"])).unwrap();
        assert_eq!(cfg.filenames, vec!["a.rs"]);
        assert_eq!(cfg.annotations_file, Some("ann.json".to_string()));
    }

    #[test]
    fn test_config_annotations_file_missing_path() {
        let result = Config::new(&args(&["verso", "--annotations-file"]));
        assert!(result.is_err());
    }

    #[test]
    fn test_run_with_annotations_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary annotations JSON file.
        let file_annotations = vec![
            Fragment {
                body: "from file".to_string(),
                id: "file-only".to_string(),
                file: "stored.rs".to_string(),
                line: 1,
                col: 0,
            },
            Fragment {
                body: "old body".to_string(),
                id: "shared".to_string(),
                file: "stored.rs".to_string(),
                line: 10,
                col: 0,
            },
        ];
        let mut ann_file = NamedTempFile::new().unwrap();
        serde_json::to_writer(&mut ann_file, &file_annotations).unwrap();
        ann_file.flush().unwrap();

        // Create a temporary source file that extracts an annotation with conflicting ID.
        let mut src_file = NamedTempFile::with_suffix(".py").unwrap();
        writeln!(src_file, "# @<shared").unwrap();
        writeln!(src_file, "new body").unwrap();
        writeln!(src_file, "# >@").unwrap();
        src_file.flush().unwrap();

        let cfg = Config {
            filenames: vec![src_file.path().to_str().unwrap().to_string()],
            annotations_file: Some(ann_file.path().to_str().unwrap().to_string()),
        };

        // We can't easily capture stdout from run(), so replicate the merge logic here.
        let mut annotations_map: BTreeMap<String, Fragment> = BTreeMap::new();

        let ann_contents = fs::read_to_string(cfg.annotations_file.as_ref().unwrap()).unwrap();
        let loaded: Vec<Fragment> = serde_json::from_str(&ann_contents).unwrap();
        for ann in loaded {
            annotations_map.insert(ann.id.clone(), ann);
        }

        let symbols = SymbolKey::from_environment();
        for filename in &cfg.filenames {
            let contents = fs::read_to_string(filename).unwrap();
            let fragments = extract_fragments(&contents, filename, &symbols).unwrap();
            for frag in fragments {
                annotations_map.insert(frag.id.clone(), frag);
            }
        }

        let annotations: Vec<Fragment> = annotations_map.into_values().collect();

        // file-only should be present from the annotations file.
        assert!(
            annotations.iter().any(|a| a.id == "file-only"),
            "Expected file-only annotation to be present"
        );

        // shared should have the extracted body, not the file body.
        let shared = annotations.iter().find(|a| a.id == "shared").unwrap();
        assert_eq!(shared.body, "new body", "Extracted annotation should win on conflict");
    }
}
