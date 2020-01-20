#[cfg(test)]
mod tests {
    use verso::*;

    #[test]
    fn test_extract_id_good() {
        let id = extract_id(&String::from("foobarbaz"), 0);
        assert_eq!(
            id.expect("Expected successful ID extraction"),
            String::from("foobarbaz")
        );
    }

    #[test]
    fn test_extract_id_missing() {
        let id = extract_id(&String::from(""), 0);
        assert_eq!(id, None, "Expected None, got {:?}", id);
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
}
