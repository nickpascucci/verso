use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::error::Error;
use std::fmt;

const BLOCK_OPEN_SYMBOL: &'static str = concat!("@", "<");
const BLOCK_CLOSE_SYMBOL: &'static str = concat!(">", "@");
const INSERTION_SYMBOL: &'static str = "@@";
const REFERENCE_SYMBOL: &'static str = "@?";
const HALT_SYMBOL: &'static str = "@!halt";

const FILENAME_REF: &'static str = "file";
const LINE_NO_REF: &'static str = "line";
const COL_NO_REF: &'static str = "col";
const LOC_REF: &'static str = "loc";

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
    HaltWhileOpen,
}

// @<errors
#[derive(Debug, PartialEq, Clone)]
pub enum WeaveError {
    MissingFragment(String),
    MissingId,
    BadReference(String),
    ReferenceParseError,
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

                if let Some(_) = content.find(HALT_SYMBOL) {
                    break;
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

                if let Some(col) = content.find(HALT_SYMBOL) {
                    return Err(FileError {
                        err_type: ParseError::HaltWhileOpen,
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

    for (line_no, line) in contents.lines().enumerate().map(|(l, c)| (l + 1, c)) {
        if line.trim_start().starts_with(INSERTION_SYMBOL) {
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
        } else if line.contains(REFERENCE_SYMBOL) {
            let expanded = expand_references(&line, &filename, line_no, &annotations)?;
            substrings.push(expanded);
        } else {
            substrings.push(line.to_owned());
        }
    }

    return Ok(substrings.join("\n"));
}

#[derive(Debug, PartialEq, Clone)]
enum ScannerState {
    SearchingForRefStart,
    ReadingRefStart,
    ReadingId,
    ReadingRefType,
}

fn expand_references(
    line: &str,
    filename: &str,
    line_no: usize,
    annotations: &HashMap<String, Fragment>,
) -> Result<String, FileError<WeaveError>> {
    let mut pieces: Vec<String> = vec![];

    let mut state = ScannerState::SearchingForRefStart;
    let mut start_col: usize = 0;

    for (col, c) in line.chars().enumerate() {
        match &state {
            ScannerState::SearchingForRefStart => {
                if line[col..].starts_with(REFERENCE_SYMBOL) {
                    if col > start_col {
                        pieces.push(line[start_col..col].to_owned());
                    }
                    start_col = col;
                    state = ScannerState::ReadingRefStart;
                }
            }
            ScannerState::ReadingRefStart => {
                let chars_read = col - start_col;
                if chars_read >= REFERENCE_SYMBOL.len() {
                    state = ScannerState::ReadingId;
                } else if c != REFERENCE_SYMBOL.chars().nth(chars_read).expect("") {
                    return Err(FileError {
                        err_type: WeaveError::ReferenceParseError,
                        filename: filename.to_owned(),
                        line: line_no,
                        col,
                    });
                }
            }
            ScannerState::ReadingId => {
                if !c.is_alphanumeric() {
                    if c == '.' {
                        state = ScannerState::ReadingRefType;
                    } else {
                        return Err(FileError {
                            err_type: WeaveError::ReferenceParseError,
                            filename: filename.to_owned(),
                            line: line_no,
                            col,
                        });
                    };
                }
            }
            ScannerState::ReadingRefType => {
                // TODO Clean up this code a little, to reduce duplication.
                if !c.is_alphanumeric() {
                    state = ScannerState::SearchingForRefStart;
                    let expansion = expand_reference(
                        &line[start_col..col],
                        &filename,
                        line_no,
                        start_col,
                        annotations,
                    )?;
                    pieces.push(expansion);
                    start_col = col;
                } else if col == line.len() - 1 {
                    state = ScannerState::SearchingForRefStart;
                    let col = col + 1; // NOTE This differs from the block above.
                    let expansion = expand_reference(
                        &line[start_col..col],
                        &filename,
                        line_no,
                        start_col,
                        annotations,
                    )?;
                    pieces.push(expansion);
                    start_col = col;
                };
            }
        }
    }

    // Pick up any remaining unparsed data.
    if state != ScannerState::ReadingRefType {
        pieces.push(line[start_col..].to_owned());
    }

    Ok(pieces.join(""))
}

fn expand_reference(
    word: &str,
    filename: &str,
    line: usize,
    col: usize,
    annotations: &HashMap<String, Fragment>,
) -> Result<String, FileError<WeaveError>> {
    let word = word.trim_start_matches(REFERENCE_SYMBOL);
    let pieces: Vec<&str> = word.split('.').collect();
    if pieces.len() == 2 {
        let frag_id = pieces[0];
        let prop = pieces[1];
        let frag = annotations.get(frag_id);
        match frag {
            Some(f) => match prop.to_ascii_lowercase().as_str() {
                FILENAME_REF => Ok(f.file.to_owned()),
                LINE_NO_REF => Ok(f.line.to_string()),
                COL_NO_REF => Ok(f.col.to_string()),
                LOC_REF => Ok(format!("{} ({}:{})", f.file, f.line, f.col)),
                _ => Err(FileError {
                    err_type: WeaveError::BadReference(prop.to_owned()),
                    filename: filename.to_owned(),
                    line,
                    col,
                }),
            },
            None => Err(FileError {
                err_type: WeaveError::MissingFragment(frag_id.to_owned()),
                filename: filename.to_owned(),
                line,
                col,
            }),
        }
    } else {
        // TODO Make these errors more granular.
        Err(FileError {
            err_type: WeaveError::BadReference(word.to_owned()),
            filename: filename.to_owned(),
            line,
            col,
        })
    }
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

    // @!halt

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

    #[test]
    fn test_extract_fragments() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example
import sys

# @<foobarbaz The fragment starts and its ID is defined on this line; it is foobarbaz.
def main():
    do_stuff()
    make_awesome()
    # >@ This line ends the fragment.
    sys.exit(1) # oops",
            "test.py",
        );

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
            fragments[0].id == String::from("foobarbaz"),
            "Unexpected ID {:?}",
            fragments[0].id
        );
    }

    #[test]
    fn test_extract_fragments_close_before_open() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example.

# >@ This is an error on line 3.
012345 The error is at column 2.
# @< This begins the fragment.
# >@ This line ends the fragment.",
            "test.py",
        );

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            FileError {
                err_type: ParseError::CloseBeforeOpen,
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::CloseBeforeOpen, got {:?}", fragments),
        }
    }

    #[test]
    fn test_extract_fragments_halt() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example.

# @<1
Fragment 1
# >@

# @!halt This line causes the reader to stop looking for fragments in this file.
# Mostly this is useful for keeping Verso out of uninteresting areas, like these tests.

# >@ This would cause an error.",
            "test.py",
        );

        let fragments = fragments.expect("Expected a clean read");
        assert!(
            fragments.len() == 1,
            "Expected one fragment, found {}",
            fragments.len()
        );
    }

    #[test]
    fn test_extract_fragments_double_open() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example.
# @<foobarbaz This begins the fragment.
# @< This is an error on line 3.
012345 The error is at column 2.
# >@ This line ends the fragment.",
            "test.py",
        );

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            FileError {
                err_type: ParseError::DoubleOpen,
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::DoubleOpen, got {:?}", fragments),
        }
    }

    #[test]
    fn test_extract_fragments_missing_id() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example.

# @< This is an error on line 3: No ID.
012345 The error is at column 2.
# >@ This line ends the fragment.",
            "test.py",
        );

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            FileError {
                err_type: ParseError::MissingId,
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::MissingId, got {:?}", fragments),
        }
    }

    #[test]
    fn test_weave_good() {
        let text = "This is the first line!

@@1
@?1.file (@?1.line:@?1.col)
@?1.loc

Another line.";

        let frag = Fragment {
            id: String::from("1"),
            body: String::from("{Example Code}"),
            file: String::from("example.code"),
            line: 1,
            col: 0,
        };

        let mut annotations = HashMap::new();
        annotations.insert(frag.id.to_owned(), frag.to_owned());
        let result = weave("test", &text, &annotations).expect("Expected weave to return Ok");

        assert_eq!(
            result,
            String::from(
                "This is the first line!

{Example Code}
example.code (1:0)
example.code (1:0)

Another line."
            )
        );
    }

    #[test]
    fn test_weave_missing_fragment() {
        let text = "This is the first line!

@@1

Another line.";

        let annotations = HashMap::new();
        weave("test", &text, &annotations).expect_err("Expected weave to return an error");
    }

    #[test]
    fn test_weave_bad_reference_type() {
        let text = "This is the first line!

@?1.foo

Another line.";

        let frag = Fragment {
            id: String::from("1"),
            body: String::from("{Example Code}"),
            file: String::from("example.code"),
            line: 1,
            col: 0,
        };

        let mut annotations = HashMap::new();
        annotations.insert(frag.id.to_owned(), frag.to_owned());

        let err = weave("test", &text, &annotations).expect_err("Expected weave to return an error");
        match err {
            FileError {
                err_type: WeaveError::BadReference(s),
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 0, "Expected error on col 0, found col {:?}", col);
                assert_eq!(s, "foo", "Expected error message to be \"foo\", got {:?}", s);
            }
            _ => panic!("Expected WeaveError::BadReference, got {:?}", err),
        }
    }

    #[test]
    fn test_weave_bad_reference_id() {
        let text = "This is the first line!

@?1.foo

Another line.";

        let annotations = HashMap::new();

        let err = weave("test", &text, &annotations).expect_err("Expected weave to return an error");
        match err {
            FileError {
                err_type: WeaveError::MissingFragment(s),
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 0, "Expected error on col 0, found col {:?}", col);
                assert_eq!(s, "1", "Expected error message to be \"1\", got {:?}", s);
            }
            _ => panic!("Expected WeaveError::MissingFragment, got {:?}", err),
        }
    }
}
