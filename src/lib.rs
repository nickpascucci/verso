use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::error::Error;
use std::fmt;

const BLOCK_OPEN_SYMBOL: &'static str = concat!("@", "<");
const BLOCK_CLOSE_SYMBOL: &'static str = concat!(">", "@");
const INSERTION_SYMBOL: &'static str = "@@";

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub body: String,
    pub id: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParseError {
    DoubleOpen,
    CloseBeforeOpen,
    MissingId,
}

// @<errors
#[derive(Debug, PartialEq, Clone)]
pub enum WeaveError {
    MissingFragment(String),
    MissingId,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FileError<T: fmt::Debug> {
    err_type: T,
    filename: String,
    line: usize,
    col: usize,
}
// >@errors

impl<T: fmt::Debug> Error for FileError<T> {}

impl<T: fmt::Debug> fmt::Display for FileError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?} in {}: line {}, column {}",
            self.err_type, self.filename, self.line, self.col
        )
    }
}

pub fn extract_fragments(
    contents: &str,
    filename: &str,
) -> Result<Vec<Fragment>, FileError<ParseError>> {
    let mut fragments: Vec<Fragment> = vec![];

    let mut fragment: Option<Fragment> = None;

    for (line, content) in contents.split("\n").enumerate().map(|(l, c)| (l + 1, c)) {
        match &fragment {
            None => {
                if let Some(col) = content.find(BLOCK_CLOSE_SYMBOL) {
                    return Err(FileError {
                        err_type: ParseError::CloseBeforeOpen,
                        filename: filename.to_owned(),
                        line,
                        col,
                    });
                }

                if let Some(col) = content.find(BLOCK_OPEN_SYMBOL) {
                    // If the line contains a start marker, begin a fragment file.
                    if let Some(id) = extract_id(content, col + BLOCK_OPEN_SYMBOL.len()) {
                        fragment = Some(Fragment {
                            body: String::new(),
                            id,
                            // The fragment to extract starts at the beginning of the next line.
                            line: line + 1,
                            col: 0,
                            file: filename.to_owned(),
                        });
                    } else {
                        return Err(FileError {
                            err_type: ParseError::MissingId,
                            filename: filename.to_owned(),
                            line,
                            col,
                        });
                    }
                }
            }

            Some(f) => {
                // If the line contains an end marker, end the fragment if one exists.
                if let Some(_) = content.find(BLOCK_CLOSE_SYMBOL) {
                    fragments.push(f.to_owned());
                    fragment = None;
                    continue;
                }

                if let Some(col) = content.find(BLOCK_OPEN_SYMBOL) {
                    return Err(FileError {
                        err_type: ParseError::DoubleOpen,
                        filename: filename.to_owned(),
                        line,
                        col,
                    });
                }

                // If there no markers, append the line to the existing fragment.
                fragment = fragment.map(|x| Fragment {
                    body: if x.body.is_empty() {
                        content.to_string()
                    } else {
                        x.body + "\n" + content
                    },
                    ..x
                });
            }
        }
    }

    Ok(fragments)
}

// @<extractid
fn extract_id(content: &str, col: usize) -> Option<String> {
    let it = content.chars().skip(col);
    let id: String = it.take_while(|c| c.is_alphanumeric()).collect();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}
// >@extractid

pub fn weave(
    filename: &str,
    contents: &str,
    annotations: &HashMap<String, Fragment>,
) -> Result<String, FileError<WeaveError>> {
    let mut substrings: Vec<String> = vec![];

    for (line_no, line) in contents.lines().enumerate() {
        if line.trim().starts_with(INSERTION_SYMBOL) {
            let id = extract_id(line, INSERTION_SYMBOL.len());
            match id {
                Some(id) => {
                    let fragment = annotations.get(&id);
                    match fragment {
                        // TODO Add indexing information.
                        Some(f) => substrings.push(f.body.to_owned()),
                        None => {
                            return Err(FileError {
                                err_type: WeaveError::MissingFragment(id.to_owned()),
                                filename: filename.to_owned(),
                                line: line_no,
                                col: INSERTION_SYMBOL.len(),
                            })
                        }
                    }
                }
                None => {
                    return Err(FileError {
                        err_type: WeaveError::MissingId,
                        filename: filename.to_owned(),
                        line: line_no,
                        col: 0,
                    })
                }
            }
        } else {
            substrings.push(line.to_owned());
        }
    }

    return Ok(substrings.join("\n"));
}

// @<tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_id_missing() {
        let id = extract_id(&String::from(""), 0);
        assert_eq!(id, None, "Expected None, got {:?}", id);
    }
    // ... snip ...
    // >@tests


    #[test]
    fn test_extract_id_good() {
        let id = extract_id(&String::from("foobarbaz"), 0);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("foobarbaz")
        );
    }

    #[test]
    fn test_extract_id_nonalphanumeric() {
        let id = extract_id(&String::from("foo-bar-baz"), 0);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("foo")
        );
    }

    #[test]
    fn test_extract_id_whitespace() {
        let id = extract_id(&String::from("foo bar baz quuz"), 0);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("foo")
        );
    }

    #[test]
    fn test_extract_id_offset() {
        let id = extract_id(&String::from("foo-bar-baz quuz"), 4);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("bar")
        );
    }
}
