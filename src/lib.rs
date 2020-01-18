use serde::{Deserialize, Serialize};

use std::error::Error;
use std::fmt;

// Blocks are opened by using the form @!<id>. For example: @!202001171309.
// The ID can be any set of characters terminated by whitespace.
const BLOCK_OPEN_SYMBOL: &'static str = "@!";
const BLOCK_CLOSE_SYMBOL: &'static str = "!@";

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Fragment {
    body: String,
    id: String,
    file: String,
    line: usize,
    col: usize,
}

#[derive(Debug, PartialEq, Clone)]
enum ParseErrorType {
    DoubleOpen,
    CloseBeforeOpen,
    MissingId,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParseError {
    err_type: ParseErrorType,
    line: usize,
    col: usize,
}

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}: line {}, column {}",
            self.err_type, self.line, self.col
        )
    }
}

pub fn extract_fragments(contents: &str, filename: &str) -> Result<Vec<Fragment>, ParseError> {
    let mut fragments: Vec<Fragment> = vec![];

    let mut fragment: Option<Fragment> = None;

    for (line, content) in contents.split("\n").enumerate().map(|(l, c)| (l + 1, c)) {
        match &fragment {
            None => {
                if let Some(col) = content.find(BLOCK_CLOSE_SYMBOL) {
                    return Err(ParseError {
                        err_type: ParseErrorType::CloseBeforeOpen,
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
                        return Err(ParseError {
                            err_type: ParseErrorType::MissingId,
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
                    continue;
                }

                if let Some(col) = content.find(BLOCK_OPEN_SYMBOL) {
                    return Err(ParseError {
                        err_type: ParseErrorType::DoubleOpen,
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

fn extract_id(content: &str, col: usize) -> Option<String> {
    let it = content.chars().skip(col);
    let id: String = it.take_while(|c| !c.is_whitespace()).collect();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_id_good() {
        let id = extract_id(&String::from("foobarbaz"), 0);
        assert_eq!(id.expect("Expected successful ID extraction"), String::from("foobarbaz"));
    }

    #[test]
    fn test_extract_id_missing() {
        let id = extract_id(&String::from(""), 0);
        assert_eq!(id, None, "Expected None, got {:?}", id);
    }

    #[test]
    fn test_extract_id_nonalphanumeric() {
        let id = extract_id(&String::from("foo-bar-baz"), 0);
        assert_eq!(id.expect("Expected successful ID extraction"), String::from("foo-bar-baz"));
    }

    #[test]
    fn test_extract_id_whitespace() {
        let id = extract_id(&String::from("foo-bar-baz quuz"), 0);
        assert_eq!(id.expect("Expected successful ID extraction"), String::from("foo-bar-baz"));
    }

    #[test]
    fn test_extract_id_offset() {
        let id = extract_id(&String::from("foo-bar-baz quuz"), 4);
        assert_eq!(id.expect("Expected successful ID extraction"), String::from("bar-baz"));
    }

    #[test]
    fn test_extract_fragments() {
        let fragments: Result<Vec<Fragment>, ParseError> = extract_fragments(
            "# This is an example
import sys

# @!foo-bar-baz The fragment starts and its ID is defined on this line; it is foo-bar-baz.
def main():
    do_stuff()
    make_awesome()
    # !@ This line ends the fragment.
    sys.exit(1) # oops", "test.py");

        let fragments = fragments.expect("Expected no parse errors");
        assert!(
            fragments.len() == 1,
            "Expected one fragment, found {}",
            fragments.len()
        );
        assert!(
            fragments[0].body
                == String::from(
                    "def main():
    do_stuff()
    make_awesome()"
                ),
            "Unexpected code fragment {:?}",
            fragments[0].body
        );
        assert!(
            fragments[0].id == String::from("foo-bar-baz"),
            "Unexpected ID {:?}",
            fragments[0].id
        );
    }

    #[test]
    fn test_extract_fragments_close_before_open() {
        let fragments: Result<Vec<Fragment>, ParseError> = extract_fragments(
            "# This is an example.

# !@ This is an error on line 3.
012345 The error is at column 2.
# @! This begins the fragment.
# !@ This line ends the fragment.", "test.py");

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            ParseError { err_type: ParseErrorType::CloseBeforeOpen, line, col, .. } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::CloseBeforeOpen, got {:?}", fragments),
        }
    }

    #[test]
    fn test_extract_fragments_double_open() {
        let fragments: Result<Vec<Fragment>, ParseError> = extract_fragments(
            "# This is an example.
# @!foo-bar-baz This begins the fragment.
# @! This is an error on line 3.
012345 The error is at column 2.
# !@ This line ends the fragment.", "test.py");

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            ParseError { err_type: ParseErrorType::DoubleOpen, line, col, .. } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::DoubleOpen, got {:?}", fragments),
        }
    }


    #[test]
    fn test_extract_fragments_missing_id() {
        let fragments: Result<Vec<Fragment>, ParseError> = extract_fragments(
            "# This is an example.

# @! This is an error on line 3: No ID.
012345 The error is at column 2.
# !@ This line ends the fragment.", "test.py");

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            ParseError { err_type: ParseErrorType::MissingId, line, col, .. } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::MissingId, got {:?}", fragments),
        }
    }
}
