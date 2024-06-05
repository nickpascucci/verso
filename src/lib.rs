use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use std::error::Error;
use std::fmt;

// These are built using compile-time macros so that verso does not see them as starting a fragment
// in this file.
const FRAGMENT_OPEN_SYMBOL: &str = concat!("@", "<");
const FRAGMENT_CLOSE_SYMBOL: &str = concat!(">", "@");

const ID_SAFE_CHARS: &[char] = &['/', '_', '-'];

const HALT_SYMBOL: &str = "@!halt";
const INSERTION_SYMBOL: &str = "@@";
const PATTERN_SYMBOL: &str = "@*";
const METADATA_SYMBOL: &str = "@?";
const METADATA_SEPARATOR: char = '.';

const FILENAME_REF: &str = "file";
const LINE_NO_REF: &str = "line";
const COL_NO_REF: &str = "col";
const LOC_REF: &str = "loc";
const ABS_PATH_REF: &str = "abspath";
const REL_PATH_REF: &str = "relpath";

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SymbolKey {
    fragment_open: String,
    fragment_close: String,

    halt: String,
    insertion: String,
    pattern: String,
    metadata: String,
}

impl Default for SymbolKey {
    fn default() -> Self {
        Self {
            fragment_open: FRAGMENT_OPEN_SYMBOL.to_string(),
            fragment_close: FRAGMENT_CLOSE_SYMBOL.to_string(),
            halt: HALT_SYMBOL.to_string(),
            insertion: INSERTION_SYMBOL.to_string(),
            pattern: PATTERN_SYMBOL.to_string(),
            metadata: METADATA_SYMBOL.to_string(),
        }
    }
}

impl SymbolKey {
    pub fn from_environment() -> Self {
        use std::env::var;

        let defaults = Self::default();

        Self {
            fragment_open: var("VERSO_FRAGMENT_OPEN_SYMBOL").unwrap_or(defaults.fragment_open),
            fragment_close: var("VERSO_FRAGMENT_CLOSE_SYMBOL").unwrap_or(defaults.fragment_close),
            halt: var("VERSO_HALT_SYMBOL").unwrap_or(defaults.halt),
            insertion: var("RECTO_INSERTION_SYMBOL").unwrap_or(defaults.insertion),
            pattern: var("RECTO_PATTERN_SYMBOL").unwrap_or(defaults.pattern),
            metadata: var("RECTO_METADATA_SYMBOL").unwrap_or(defaults.metadata),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub body: String,
    pub id: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IdExtractError {
    NoIdFound,
    ReservedCharacterUsed(char),
}

#[derive(Debug, PartialEq, Clone)]
pub enum PatternExtractError {
    NoPatternFound,
    RegexConstruction(regex::Error),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParseError {
    UnclosedFragment,
    CloseBeforeOpen,
    MissingId,
    IdExtractError,
    HaltWhileOpen,
}

// @<errors
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WeaveError {
    MissingFragment(String),
    MissingId,
    IdExtractError,
    PatternExtractError,
    MetadataParseError,
    BadMetadata(String),
    UnknownProperty(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FileError<T: fmt::Debug> {
    err_type: T,
    filename: String,
    line: usize,
    col: usize,
    message: Option<String>,
}
// >@errors

impl<T: fmt::Debug> Error for FileError<T> {}

impl<T: fmt::Debug> fmt::Display for FileError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error: ({}:{}:{}) {:?} {}",
            self.filename,
            self.line,
            self.col,
            self.err_type,
            self.message.to_owned().unwrap_or_default()
        )
    }
}

trait IdSafe {
    fn is_safe_for_ids(&self) -> bool;
    fn is_safe_for_refs(&self) -> bool;
}

impl IdSafe for char {
    fn is_safe_for_ids(&self) -> bool {
        self.is_alphanumeric() || ID_SAFE_CHARS.contains(self)
    }

    fn is_safe_for_refs(&self) -> bool {
        self.is_alphanumeric()
    }
}

pub fn extract_fragments(
    contents: &str,
    filename: &str,
    symbols: &SymbolKey,
) -> Result<Vec<Fragment>, FileError<ParseError>> {
    let mut fragments: Vec<Fragment> = vec![];
    let mut fragment_stack: Vec<Fragment> = vec![];

    for (line, content) in contents.split('\n').enumerate().map(|(l, c)| (l + 1, c)) {
        if let Some(col) = content.find(&symbols.fragment_open) {
            match extract_id(content, col + symbols.fragment_open.len()) {
                Ok(id) => {
                    // Push a new Fragment onto the stack.
                    fragment_stack.push(Fragment {
                        body: String::new(),
                        id,
                        file: filename.to_owned(),
                        // The Fragment starts on the line after the opening symbol.
                        line: line + 1,
                        col: 0,
                    });
                }
                Err(IdExtractError::NoIdFound) => {
                    return Err(FileError {
                        err_type: ParseError::MissingId,
                        filename: filename.to_owned(),
                        line,
                        col,
                        message: Some(format!(
                            "no fragment identifier found in fragment open symbol: {}",
                            line
                        )),
                    });
                }
                Err(IdExtractError::ReservedCharacterUsed(c)) => {
                    return Err(FileError {
                        err_type: ParseError::IdExtractError,
                        filename: filename.to_owned(),
                        line,
                        col,
                        message: Some(format!(
                            "error parsing fragment identifier in fragment open symbol: {}
                                     (used reserved character {})",
                            line, c
                        )),
                    });
                }
            }
        } else if let Some(col) = content.find(&symbols.fragment_close) {
            if let Some(closed_fragment) = fragment_stack.pop() {
                let trimmed_body = closed_fragment.body.trim_end_matches('\n').to_string();
                if let Some(parent_fragment) = fragment_stack.last_mut() {
                    // Special handling of "empty" fragments.
                    if !trimmed_body.is_empty() {
                        // Add the child fragments body to the parent fragment.
                        parent_fragment.body.push_str(&trimmed_body);
                        parent_fragment.body.push('\n');
                    }
                }
                // Add the closed fragment to the results list
                fragments.push(Fragment {
                    body: trimmed_body,
                    ..closed_fragment
                });
            } else {
                return Err(FileError {
                    err_type: ParseError::CloseBeforeOpen,
                    filename: filename.to_owned(),
                    line,
                    col,
                    message: Some("fragment close symbol found without an open symbol".to_string()),
                });
            }
        } else if let Some(col) = content.find(&symbols.halt) {
            // If the Fragment stack is not empty, we have an error as there is at least 1 open
            // Fragment.
            if !fragment_stack.is_empty() {
                return Err(FileError {
                    err_type: ParseError::HaltWhileOpen,
                    filename: filename.to_owned(),
                    line,
                    col,
                    message: Some(format!(
                        "halt symbol found while a fragment was open: {}",
                        line
                    )),
                });
            }
            // Otherwise stop processing and break out.
            break;
        } else if let Some(fragment) = fragment_stack.last_mut() {
            fragment.body.push_str(content);
            fragment.body.push('\n');
        }
    }

    if !fragment_stack.is_empty() {
        return Err(FileError {
            err_type: ParseError::UnclosedFragment,
            filename: filename.to_owned(),
            line: contents.lines().count(),
            col: 0,
            message: Some("not all fragments were closed".to_string()),
        });
    }

    Ok(fragments)
}

// @<extractid
fn extract_id(content: &str, col: usize) -> Result<String, IdExtractError> {
    let it = content.chars().skip(col);
    let id: String = it.take_while(|c| !c.is_whitespace()).collect();
    if id.is_empty() {
        Err(IdExtractError::NoIdFound)
    } else if let Some(idx) = id.find(|c: char| !c.is_safe_for_ids()) {
        Err(IdExtractError::ReservedCharacterUsed(
            id.chars().nth(idx).unwrap(),
        ))
    } else {
        Ok(id)
    }
}
// >@extractid

fn extract_pattern(content: &str, col: usize) -> Result<Regex, PatternExtractError> {
    // Remove leading characters to get just the pattern
    let pat = &content[col..];
    // Remove leading and trailing whitespace; patterns should use ^/$ to include it
    let pat = pat.trim_start().trim_end();
    if pat.is_empty() {
        Err(PatternExtractError::NoPatternFound)
    } else {
        Regex::new(pat).map_err(PatternExtractError::RegexConstruction)
    }
}

pub fn weave(
    filename: &str,
    contents: &str,
    annotations: &BTreeMap<String, Fragment>,
    symbols: &SymbolKey,
) -> Result<String, FileError<WeaveError>> {
    let mut substrings: Vec<String> = vec![];

    for (line_no, line) in contents.lines().enumerate().map(|(l, c)| (l + 1, c)) {
        if line.trim_start().starts_with(&symbols.insertion) {
            let id = extract_id(line.trim_start(), symbols.insertion.len());
            match id {
                Ok(id) => {
                    let fragment = annotations.get(&id);
                    match fragment {
                        // TODO Add indexing information.
                        Some(f) => substrings.push(f.body.to_owned()),
                        None => {
                            return Err(FileError {
                                err_type: WeaveError::MissingFragment(id.to_owned()),
                                filename: filename.to_owned(),
                                line: line_no,
                                col: symbols.insertion.len(),
                                message: Some(format!("no fragment found with identifier {}", id)),
                            })
                        }
                    }
                }
                Err(IdExtractError::NoIdFound) => {
                    return Err(FileError {
                        err_type: WeaveError::MissingId,
                        filename: filename.to_owned(),
                        line: line_no,
                        col: 0,
                        message: Some(format!("no fragment identifier found in line: {}", line)),
                    })
                }
                Err(IdExtractError::ReservedCharacterUsed(c)) => {
                    return Err(FileError {
                        err_type: WeaveError::IdExtractError,
                        filename: filename.to_owned(),
                        line: line_no,
                        col: 0,
                        message: Some(format!(
                            "error parsing identifier in fragment open symbol: {}
                             (used reserved character {})",
                            line, c
                        )),
                    })
                }
            }
        } else if line.trim_start().starts_with(&symbols.pattern) {
            let re = extract_pattern(line.trim_start(), symbols.pattern.len());
            match re {
                Ok(re) => {
                    annotations
                        .iter()
                        .filter(|(k, _)| re.is_match(k))
                        .for_each(|(_, v)| substrings.push(v.body.to_owned()));
                }
                Err(PatternExtractError::NoPatternFound) => {
                    return Err(FileError {
                        err_type: WeaveError::PatternExtractError,
                        filename: filename.to_owned(),
                        line: line_no,
                        col: 0,
                        message: Some(format!("no fragment pattern found in line: {}", line)),
                    })
                }
                Err(PatternExtractError::RegexConstruction(e)) => {
                    return Err(FileError {
                        err_type: WeaveError::PatternExtractError,
                        filename: filename.to_owned(),
                        line: line_no,
                        col: 0,
                        message: Some(format!(
                            "error parsing pattern at insertion symbol: {}
                             (regex construction failed with {})",
                            line, e
                        )),
                    })
                }
            }
        } else if line.contains(&symbols.metadata) {
            let expanded = expand_metadata_refs(line, filename, line_no, annotations, symbols)?;
            substrings.push(expanded);
        } else {
            substrings.push(line.to_owned());
        }
    }

    // Account for final newline, which str.lines() may drop.
    if contents.ends_with('\n') && substrings.last().map_or(true, |c| !c.contains('\n')) {
        substrings.push("".to_owned());
    }

    let document = substrings.join("\n");

    Ok(document)
}

#[derive(Debug, PartialEq, Clone)]
enum ScannerState {
    SearchingForRefStart,
    ReadingMetaStart,
    ReadingId,
    ReadingRefType,
}

fn expand_metadata_refs(
    line: &str,
    filename: &str,
    line_no: usize,
    annotations: &BTreeMap<String, Fragment>,
    symbols: &SymbolKey,
) -> Result<String, FileError<WeaveError>> {
    let mut pieces: Vec<String> = vec![];

    let mut state = ScannerState::SearchingForRefStart;
    let mut start_col: usize = 0;

    for (col, c) in line.chars().enumerate() {
        match &state {
            ScannerState::SearchingForRefStart => {
                if line[col..].starts_with(&symbols.metadata) {
                    if col > start_col {
                        pieces.push(line[start_col..col].to_owned());
                    }
                    start_col = col;
                    state = ScannerState::ReadingMetaStart;
                }
            }
            ScannerState::ReadingMetaStart => {
                let chars_read = col - start_col;
                if chars_read >= symbols.metadata.len() {
                    state = ScannerState::ReadingId;
                } else if c != symbols.metadata.chars().nth(chars_read).unwrap() {
                    return Err(FileError {
                        err_type: WeaveError::MetadataParseError,
                        filename: filename.to_owned(),
                        line: line_no,
                        col,
                        message: Some(format!(
                            "unexpected character '{}' while reading metadata symbol in line {}",
                            c, line
                        )),
                    });
                }
            }
            ScannerState::ReadingId => {
                if !c.is_safe_for_ids() {
                    if c == METADATA_SEPARATOR {
                        state = ScannerState::ReadingRefType;
                    } else {
                        return Err(FileError {
                            err_type: WeaveError::MetadataParseError,
                            filename: filename.to_owned(),
                            line: line_no,
                            col,
                            message: Some(format!(
                                "expected '{}', got '{}' while reading metadata symbol in line {}",
                                METADATA_SEPARATOR, c, line
                            )),
                        });
                    };
                }
            }
            ScannerState::ReadingRefType => {
                // TODO Clean up this code a little, to reduce duplication.
                if !c.is_safe_for_refs() {
                    state = ScannerState::SearchingForRefStart;
                    let expansion = expand_metadata(
                        &line[start_col..col],
                        filename,
                        line_no,
                        start_col,
                        annotations,
                        symbols,
                    )?;
                    pieces.push(expansion);
                    start_col = col;
                } else if col == line.len() - 1 {
                    state = ScannerState::SearchingForRefStart;
                    let col = col + 1; // NOTE This differs from the fragment above.
                    let expansion = expand_metadata(
                        &line[start_col..col],
                        filename,
                        line_no,
                        start_col,
                        annotations,
                        symbols,
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

fn expand_metadata(
    word: &str,
    filename: &str,
    line: usize,
    col: usize,
    annotations: &BTreeMap<String, Fragment>,
    symbols: &SymbolKey,
) -> Result<String, FileError<WeaveError>> {
    let word = word.trim_start_matches(&symbols.metadata);
    let col = col + symbols.metadata.len(); // Offset column to account for the symbol we removed.
    let pieces: Vec<&str> = word.split(METADATA_SEPARATOR).collect();
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
                ABS_PATH_REF => Ok(format!("/{}", f.file)),
                REL_PATH_REF => {
                    let from_path = std::path::Path::new(filename);
                    let to_path = std::path::Path::new(&f.file);
                    let rel_path = find_relative_path(from_path, to_path);
                    Ok(rel_path.to_string_lossy().to_string())
                }
                _ => Err(FileError {
                    err_type: WeaveError::UnknownProperty(prop.to_owned()),
                    filename: filename.to_owned(),
                    line,
                    col: col + frag_id.len() + 1,
                    message: Some(format!("unknown metadata type '{}'", prop)),
                }),
            },
            None => Err(FileError {
                err_type: WeaveError::MissingFragment(frag_id.to_owned()),
                filename: filename.to_owned(),
                line,
                col,
                message: Some(format!("unknown fragment '{}'", frag_id)),
            }),
        }
    } else {
        // TODO Make these errors more granular.
        Err(FileError {
            err_type: WeaveError::BadMetadata(word.to_owned()),
            filename: filename.to_owned(),
            line,
            col,
            message: Some(format!("malformed property lookup '{}'", word)),
        })
    }
}

fn find_relative_path(a: &std::path::Path, b: &std::path::Path) -> std::path::PathBuf {
    let apcs = a.components();
    let mut bpcs = b.components();
    let mut ups = std::path::PathBuf::new();
    let mut downs = std::path::PathBuf::new();

    // Cut off shared prefix.
    let mut prefix_read = false;
    for apc in apcs {
        if !prefix_read {
            match bpcs.next() {
                Some(_x) if _x.eq(&apc) => {
                    // Path components match; ignore them.
                    continue;
                }
                Some(x) => {
                    // Mismatch! End of shared prefix.
                    prefix_read = true;
                    downs.push(x);
                }
                None => {
                    // Ran out of path components, all of which were shared.
                    downs.push(".");
                }
            }
        } else {
            // Add enough "parent dir" tokens to get to shared directory path from A
            ups.push("..");
        }
    }

    // Add all of the remaining path components in B's path
    for bpc in bpcs {
        downs.push(bpc);
    }

    ups.join(downs)
}

// @<tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_id_missing() {
        let id = extract_id("", 0);
        assert_eq!(
            id,
            Err(IdExtractError::NoIdFound),
            "Expected NoIdFound, got {:?}",
            id
        );
    }
    // ... snip ...
    // >@tests

    // @!halt

    #[test]
    fn test_extract_id_good() {
        let id = extract_id("foobarbaz", 0);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("foobarbaz")
        );
    }

    #[test]
    fn test_extract_id_whitespace() {
        let id = extract_id("foo bar baz quuz", 0);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("foo")
        );
    }

    #[test]
    fn test_extract_id_acceptable_separators() {
        let id_elements = ["foo", "bar", "baz", "quuz"];
        for sep in ID_SAFE_CHARS {
            let input = &id_elements.join(&sep.to_string());
            let id = extract_id(input, 0);
            assert_eq!(&id.expect("Expected successful ID extraction"), input);
        }
    }

    #[test]
    fn test_extract_id_reserved_chars() {
        let id_elements = ["foo", "bar", "baz", "quuz"];
        let id_reserved_chars = &[METADATA_SEPARATOR];
        for sep in id_reserved_chars {
            let input = &id_elements.join(&sep.to_string());
            let id = extract_id(input, 0);
            assert_eq!(
                id.expect_err(&format!(
                    "Expected ID extraction to fail due to reserved char '{}'",
                    sep
                )),
                IdExtractError::ReservedCharacterUsed(*sep)
            );
        }
    }

    #[test]
    fn test_extract_id_offset() {
        let id = extract_id("foo bar baz quuz", 4);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("bar")
        );
    }

    #[test]
    fn test_extract_pattern_good() {
        let pattern = extract_pattern("[a-z0-9]+", 0);
        assert!(pattern
            .expect("Expected successful pattern extraction")
            .is_match("abc123"));
    }

    #[test]
    fn test_extract_pattern_missing() {
        let pattern =
            extract_pattern("   ", 0).expect_err("Expected error extracting empty pattern");
        assert_eq!(
            pattern,
            PatternExtractError::NoPatternFound,
            "Expected NoPatternFound, got {:?}",
            pattern
        );
    }

    #[test]
    fn test_extract_pattern_invalid() {
        let pattern =
            extract_pattern("{[}]", 0).expect_err("Expected error extracting invalid pattern");
        assert!(
            matches!(pattern, PatternExtractError::RegexConstruction(_)),
            "Expected RegexConstruction, got {:?}",
            pattern
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
            &SymbolKey::default(),
        );

        let fragments = fragments.expect("Expected no parse errors");
        assert!(
            fragments.len() == 1,
            "Expected one fragment, found {}",
            fragments.len()
        );
        assert!(
            fragments[0].body
                == *"def main():
    do_stuff()
    make_awesome()",
            "Unexpected code fragment {:?}",
            fragments[0].body
        );
        assert!(
            fragments[0].id == *"foobarbaz",
            "Unexpected ID {:?}",
            fragments[0].id
        );
    }

    #[test]
    fn test_extract_fragments_nested() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example.
    # @<foobarbaz This begins the fragment.
    Start of outer
    # @<qux
        Start of inner
            # @<quux
            # >@
        End of inner
    # >@
    End of outer
    # >@",
            "test.py",
            &SymbolKey::default(),
        );

        let fragments = fragments.expect("Expected no parse errors");
        assert!(
            fragments.len() == 3, // Two fragments in addition to the full file
            "Expected two fragments, found {}",
            fragments.len()
        );
        // Innermost, empty fragment
        assert_eq!(fragments[0].body, "", "Expected fragment to be empty");
        assert!(
            fragments[0].id == *"quux",
            "Unexpected ID {:?}",
            fragments[0].id
        );
        // Middle fragment, that has content, including all child fragments with tags removed.
        assert_eq!(
            fragments[1].body,
            "        Start of inner
        End of inner",
            "Expected nested fragment markers to be removed"
        );
        assert!(
            fragments[1].id == *"qux",
            "Unexpected ID {:?}",
            fragments[1].id
        );
        // Outer fragment, that has content, including all child fragments with tags removed.
        assert_eq!(
            fragments[2].body,
            "    Start of outer
        Start of inner
        End of inner
    End of outer",
            "Expected nested fragment markers to be removed"
        );
        assert!(
            fragments[2].id == *"foobarbaz",
            "Unexpected ID {:?}",
            fragments[2].id
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
            &SymbolKey::default(),
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
            &SymbolKey::default(),
        );

        let fragments = fragments.expect("Expected a clean read");
        assert!(
            fragments.len() == 1,
            "Expected one fragment, found {}",
            fragments.len()
        );
    }

    #[test]
    fn test_extract_fragments_halt_while_open() {
        let fragments: Result<Vec<Fragment>, FileError<ParseError>> = extract_fragments(
            "# This is an example.
# @<1
Fragment 1
# @!halt This line causes an error as we have an open Fragment.
# >@",
            "test.py",
            &SymbolKey::default(),
        );

        let fragments = fragments.expect_err("Expected a parsing error");
        match fragments {
            FileError {
                err_type: ParseError::HaltWhileOpen,
                line,
                col,
                ..
            } => {
                assert_eq!(line, 4, "Expected error on line 4, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
            }
            _ => panic!("Expected ParseError::HaltWhileOpen, got {:?}", fragments),
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
            &SymbolKey::default(),
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
  @@1
@* [0-9]
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

        let mut annotations = BTreeMap::new();
        annotations.insert(frag.id.to_owned(), frag);
        let result = weave("test", text, &annotations, &SymbolKey::default())
            .expect("Expected weave to return Ok");

        assert_eq!(
            result,
            String::from(
                "This is the first line!

{Example Code}
{Example Code}
{Example Code}
example.code (1:0)
example.code (1:0)

Another line."
            )
        );
    }

    #[test]
    fn test_weave_pattern_order() {
        let text = "@* [0-9]";

        let frag1 = Fragment {
            id: String::from("1"),
            body: String::from("{Example Code 1}"),
            file: String::from("example.code"),
            line: 1,
            col: 0,
        };

        let frag2 = Fragment {
            id: String::from("2"),
            body: String::from("{Example Code 2}"),
            file: String::from("example.code"),
            line: 2,
            col: 0,
        };

        let mut annotations = BTreeMap::new();
        annotations.insert(frag2.id.to_owned(), frag2);
        annotations.insert(frag1.id.to_owned(), frag1);
        let result = weave("test", text, &annotations, &SymbolKey::default())
            .expect("Expected weave to return Ok");

        assert_eq!(
            result,
            String::from(
                "{Example Code 1}
{Example Code 2}"
            )
        );
    }

    #[test]
    fn test_weave_missing_fragment() {
        let text = "This is the first line!

@@1

Another line.";

        let annotations = BTreeMap::new();
        weave("test", text, &annotations, &SymbolKey::default())
            .expect_err("Expected weave to return an error");
    }

    #[test]
    fn test_weave_bad_metadata_type() {
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

        let mut annotations = BTreeMap::new();
        annotations.insert(frag.id.to_owned(), frag);

        let err = weave("test", text, &annotations, &SymbolKey::default())
            .expect_err("Expected weave to return an error");
        match err {
            FileError {
                err_type: WeaveError::UnknownProperty(s),
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 4, "Expected error on col 4, found col {:?}", col);
                assert_eq!(
                    s, "foo",
                    "Expected error message to be \"foo\", got {:?}",
                    s
                );
            }
            _ => panic!("Expected WeaveError::UnknownProperty, got {:?}", err),
        }
    }

    #[test]
    fn test_weave_bad_metadata_id() {
        let text = "This is the first line!

@?1.loc

Another line.";

        let annotations = BTreeMap::new();

        let err = weave("test", text, &annotations, &SymbolKey::default())
            .expect_err("Expected weave to return an error");
        match err {
            FileError {
                err_type: WeaveError::MissingFragment(s),
                line,
                col,
                ..
            } => {
                assert_eq!(line, 3, "Expected error on line 3, found line {:?}", line);
                assert_eq!(col, 2, "Expected error on col 2, found col {:?}", col);
                assert_eq!(s, "1", "Expected error message to be \"1\", got {:?}", s);
            }
            _ => panic!("Expected WeaveError::MissingFragment, got {:?}", err),
        }
    }

    #[test]
    fn test_find_relative_path() {
        {
            let a = std::path::PathBuf::from("/a/b/c/d.foo");
            let b = std::path::PathBuf::from("/a/b/e/f/g.bar");
            let rel_path = find_relative_path(&a, &b);
            assert_eq!(rel_path, std::path::PathBuf::from("../e/f/g.bar"));
        }

        {
            let a = std::path::PathBuf::from("/a/b/c/d.foo");
            let b = std::path::PathBuf::from("/a/g.bar");
            let rel_path = find_relative_path(&a, &b);
            assert_eq!(rel_path, std::path::PathBuf::from("../../g.bar"));
        }

        {
            let a = std::path::PathBuf::from("/a/b/c/d.foo");
            let b = std::path::PathBuf::from("/e/f/g.bar");
            let rel_path = find_relative_path(&a, &b);
            assert_eq!(rel_path, std::path::PathBuf::from("../../../e/f/g.bar"));
        }
    }
}
