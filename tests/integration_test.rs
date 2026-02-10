//! Integration tests that replicate the end-to-end pipeline tested by
//! `examples/check-examples.sh`, exercising verso's `extract_fragments` and `weave`
//! functions against real example files and comparing the output to reference files.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use verso::{extract_fragments, weave, Fragment, SymbolKey};

/// Root of the examples directory, relative to the project root (where Cargo.toml lives).
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// Extract fragments from a list of source files (paths relative to examples/).
/// Returns a BTreeMap keyed by fragment ID, mirroring what the `recto` binary does.
fn extract_all_fragments(source_files: &[&str]) -> BTreeMap<String, Fragment> {
    let symbols = SymbolKey::default();
    let mut annotations = BTreeMap::new();

    for source_file in source_files {
        let path = examples_dir().join(source_file);
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read source file {:?}: {}", path, e));

        let fragments = extract_fragments(&contents, source_file, &symbols)
            .unwrap_or_else(|e| panic!("Failed to extract fragments from {:?}: {}", path, e));

        for frag in fragments {
            annotations.insert(frag.id.clone(), frag);
        }
    }

    annotations
}

/// Weave a single prose file and compare the result against its reference output.
fn weave_and_compare(
    prose_file: &str,
    annotations: &BTreeMap<String, Fragment>,
    symbols: &SymbolKey,
) {
    let prose_path = examples_dir().join(prose_file);
    let reference_path = examples_dir().join("reference").join(prose_file);

    let prose_contents = fs::read_to_string(&prose_path)
        .unwrap_or_else(|e| panic!("Failed to read prose file {:?}: {}", prose_path, e));

    let reference_contents = fs::read_to_string(&reference_path)
        .unwrap_or_else(|e| panic!("Failed to read reference file {:?}: {}", reference_path, e));

    let woven = weave(prose_file, &prose_contents, annotations, symbols)
        .unwrap_or_else(|e| panic!("Weave failed for {:?}: {}", prose_file, e));

    assert_eq!(
        woven, reference_contents,
        "\n\nMismatch for prose file: {}\n\n--- expected (reference) ---\n{}\n--- actual (woven) ---\n{}\n",
        prose_file, reference_contents, woven
    );
}

/// Full pipeline test: extract fragments from all source files, weave all prose files,
/// and compare each output against its reference. This is the Rust equivalent of
/// `examples/check-examples.sh`.
#[test]
fn test_full_pipeline() {
    let source_files = &["example.py", "test/example-2.py", "test/nested.rs"];

    let prose_files = &[
        "empty.md",
        "example.md",
        "test/example-2.md",
        "test/level-2/l2.md",
        "test/nested.md",
    ];

    let annotations = extract_all_fragments(source_files);
    let symbols = SymbolKey::default();

    for prose_file in prose_files {
        weave_and_compare(prose_file, &annotations, &symbols);
    }
}

/// Test that fragment extraction produces the expected fragments from example.py.
#[test]
fn test_extract_example_py() {
    let symbols = SymbolKey::default();
    let path = examples_dir().join("example.py");
    let contents = fs::read_to_string(&path).unwrap();

    let fragments = extract_fragments(&contents, "example.py", &symbols).unwrap();

    assert_eq!(fragments.len(), 2, "Expected 2 fragments from example.py");

    let ids: Vec<&str> = fragments.iter().map(|f| f.id.as_str()).collect();
    assert!(ids.contains(&"examples/1"), "Missing fragment examples/1");
    assert!(ids.contains(&"examples/2"), "Missing fragment examples/2");

    let frag1 = fragments.iter().find(|f| f.id == "examples/1").unwrap();
    assert_eq!(
        frag1.body,
        "    print(\"Hello, World!\")\n    sys.exit(1)",
        "Fragment examples/1 body mismatch"
    );

    let frag2 = fragments.iter().find(|f| f.id == "examples/2").unwrap();
    assert_eq!(
        frag2.body,
        "if __name__ == \"__main__\":\n    main()",
        "Fragment examples/2 body mismatch"
    );
}

/// Test that nested fragment extraction works correctly with the nested.rs example.
#[test]
fn test_extract_nested_rs() {
    let symbols = SymbolKey::default();
    let path = examples_dir().join("test/nested.rs");
    let contents = fs::read_to_string(&path).unwrap();

    let fragments = extract_fragments(&contents, "test/nested.rs", &symbols).unwrap();

    assert_eq!(
        fragments.len(),
        2,
        "Expected 2 fragments from nested.rs (mainfnmessage + mainfn)"
    );

    let ids: Vec<&str> = fragments.iter().map(|f| f.id.as_str()).collect();
    assert!(ids.contains(&"mainfn"), "Missing fragment mainfn");
    assert!(
        ids.contains(&"mainfnmessage"),
        "Missing fragment mainfnmessage"
    );

    // The inner fragment should have just the message line.
    let inner = fragments.iter().find(|f| f.id == "mainfnmessage").unwrap();
    assert_eq!(
        inner.body,
        "    let message = String::from(\"Hello fellow Rustaceans!\");",
        "Inner fragment body mismatch"
    );

    // The outer fragment should include the inner fragment's body (tags removed).
    let outer = fragments.iter().find(|f| f.id == "mainfn").unwrap();
    assert!(
        outer.body.contains("let message = String::from"),
        "Outer fragment should contain inner fragment body"
    );
    assert!(
        !outer.body.contains("@<"),
        "Outer fragment should not contain fragment open markers"
    );
    assert!(
        !outer.body.contains(">@"),
        "Outer fragment should not contain fragment close markers"
    );
}

/// Test weaving the empty.md file (should pass through unchanged).
#[test]
fn test_weave_empty_file() {
    let annotations = BTreeMap::new();
    let symbols = SymbolKey::default();

    let path = examples_dir().join("empty.md");
    let contents = fs::read_to_string(&path).unwrap();

    let reference_path = examples_dir().join("reference/empty.md");
    let reference = fs::read_to_string(&reference_path).unwrap();

    let woven = weave("empty.md", &contents, &annotations, &symbols).unwrap();
    assert_eq!(woven, reference, "Empty file should weave to itself");
}

/// Test that metadata references (file, line, relpath, abspath) are expanded correctly.
#[test]
fn test_weave_metadata_expansion() {
    let source_files = &["example.py", "test/example-2.py", "test/nested.rs"];
    let annotations = extract_all_fragments(source_files);
    let symbols = SymbolKey::default();

    // example.md uses @?examples/1.relpath and @?examples/1.abspath
    let path = examples_dir().join("example.md");
    let contents = fs::read_to_string(&path).unwrap();
    let woven = weave("example.md", &contents, &annotations, &symbols).unwrap();

    assert!(
        woven.contains("\nexample.py\n"),
        "Expected relative path 'example.py' in woven output of example.md"
    );
    assert!(
        woven.contains("\n/example.py\n"),
        "Expected absolute path '/example.py' in woven output of example.md"
    );
}

/// Test that relative paths are computed correctly when prose file is in a subdirectory.
#[test]
fn test_weave_relative_path_from_subdirectory() {
    let source_files = &["example.py", "test/example-2.py", "test/nested.rs"];
    let annotations = extract_all_fragments(source_files);
    let symbols = SymbolKey::default();

    // test/example-2.md uses @?examples/1.relpath (should be ../example.py)
    // and @?examples/3.abspath (should be /test/example-2.py)
    let path = examples_dir().join("test/example-2.md");
    let contents = fs::read_to_string(&path).unwrap();
    let woven = weave("test/example-2.md", &contents, &annotations, &symbols).unwrap();

    assert!(
        woven.contains("../example.py"),
        "Expected relative path '../example.py' in woven output of test/example-2.md"
    );
    assert!(
        woven.contains("/test/example-2.py"),
        "Expected absolute path '/test/example-2.py' in woven output of test/example-2.md"
    );
}

/// Test that the file metadata property is expanded correctly.
#[test]
fn test_weave_file_metadata() {
    let source_files = &["example.py", "test/example-2.py", "test/nested.rs"];
    let annotations = extract_all_fragments(source_files);
    let symbols = SymbolKey::default();

    // test/nested.md uses @?mainfn.file (should be test/nested.rs)
    let path = examples_dir().join("test/nested.md");
    let contents = fs::read_to_string(&path).unwrap();
    let woven = weave("test/nested.md", &contents, &annotations, &symbols).unwrap();

    assert!(
        woven.contains("test/nested.rs"),
        "Expected file metadata 'test/nested.rs' in woven output of test/nested.md"
    );
}

/// Test each prose file individually to get granular failure messages.
#[test]
fn test_weave_example_md() {
    let annotations = extract_all_fragments(&["example.py", "test/example-2.py", "test/nested.rs"]);
    weave_and_compare("example.md", &annotations, &SymbolKey::default());
}

#[test]
fn test_weave_example_2_md() {
    let annotations = extract_all_fragments(&["example.py", "test/example-2.py", "test/nested.rs"]);
    weave_and_compare("test/example-2.md", &annotations, &SymbolKey::default());
}

#[test]
fn test_weave_level_2_md() {
    let annotations = extract_all_fragments(&["example.py", "test/example-2.py", "test/nested.rs"]);
    weave_and_compare("test/level-2/l2.md", &annotations, &SymbolKey::default());
}

#[test]
fn test_weave_nested_md() {
    let annotations = extract_all_fragments(&["example.py", "test/example-2.py", "test/nested.rs"]);
    weave_and_compare("test/nested.md", &annotations, &SymbolKey::default());
}
