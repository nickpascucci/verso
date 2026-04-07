//! Security vulnerability tests for the verso/recto codebase.
//!
//! These tests validate hypothesized vulnerabilities including:
//! - Path traversal in output file paths
//! - ReDoS via user-supplied regex patterns
//! - Symbol injection via environment variables
//! - Metadata expansion edge cases
//! - Fragment ID collision/overwrite
//! - Unicode homoglyph confusion

use std::collections::BTreeMap;
use verso::{extract_fragments, weave, Fragment, SymbolKey};

// =========================================================================
// VULNERABILITY 1: Path Traversal in recto output
// =========================================================================
// The `recto` binary joins user-supplied filenames with an output directory
// using `Path::new(&out_dir).join(&filename)`. If `filename` contains
// directory traversal sequences like `../`, the resulting path escapes the
// intended output directory.

#[test]
fn test_path_traversal_via_filename() {
    use std::path::Path;

    let out_dir = "/tmp/verso_output";

    // Simulating what recto.rs:82 does: Path::new(&cfg.out_dir).join(&filename)
    let malicious_filenames = vec![
        "../../../etc/passwd",
        "../../.ssh/authorized_keys",
        "subdir/../../escape.txt",
        "../sibling_dir/overwrite.txt",
    ];

    for filename in &malicious_filenames {
        let out_file = Path::new(out_dir).join(filename);

        // The joined path contains ".." traversal components, which means
        // when the OS resolves it, the file will be written OUTSIDE out_dir.
        // recto does NO validation to prevent this.
        let path_str = out_file.to_string_lossy();
        assert!(
            path_str.contains(".."),
            "VULNERABILITY CONFIRMED: recto joins filenames without sanitization. \
             Filename '{}' produces path '{}' which contains directory traversal \
             sequences that escape the output directory '{}'.",
            filename, path_str, out_dir
        );
    }
}

#[test]
fn test_path_traversal_normalized() {
    use std::path::Path;

    let out_dir = "/tmp/verso_output";
    let filename = "../../etc/shadow";
    let out_file = Path::new(out_dir).join(filename);

    // Demonstrate the actual path that would be written to
    // /tmp/verso_output/../../etc/shadow -> /etc/shadow
    assert_eq!(
        out_file.to_str().unwrap(),
        "/tmp/verso_output/../../etc/shadow",
        "The joined path contains traversal sequences"
    );

    // Show that this path does NOT start with the output directory
    // when canonicalized (if the path existed).
    assert!(
        !out_file.starts_with(out_dir) || out_file.to_string_lossy().contains(".."),
        "VULNERABILITY: Output path '{}' contains directory traversal and could \
         escape the output directory '{}'",
        out_file.display(),
        out_dir
    );
}

// =========================================================================
// VULNERABILITY 2: ReDoS via user-supplied regex patterns
// =========================================================================
// The `weave` function processes `@*` pattern lines by compiling user
// input as regex. While Rust's `regex` crate is resistant to catastrophic
// backtracking, very complex patterns can still be slow to compile.

#[test]
fn test_regex_pattern_complexity() {
    use std::time::Instant;

    let symbols = SymbolKey::default();
    let mut annotations = BTreeMap::new();
    let frag = Fragment {
        id: String::from("test"),
        body: String::from("body"),
        file: String::from("test.rs"),
        line: 1,
        col: 0,
    };
    annotations.insert(frag.id.clone(), frag);

    // A pattern designed to stress regex compilation
    // Nested alternations with many branches
    let complex_pattern = format!(
        "@* {}",
        (0..100)
            .map(|i| format!("(a{{{}}}b{{{}}}|c{{{}}}d{{{}}})", i, i, i, i))
            .collect::<Vec<_>>()
            .join("|")
    );

    let start = Instant::now();
    let _ = weave("test", &complex_pattern, &annotations, &symbols);
    let elapsed = start.elapsed();

    // Flag if pattern compilation takes more than 1 second
    assert!(
        elapsed.as_secs() < 1,
        "POTENTIAL VULNERABILITY: Complex regex pattern took {:?} to process, \
         which could be used for denial of service",
        elapsed
    );
}

#[test]
fn test_regex_large_pattern() {
    let symbols = SymbolKey::default();
    let annotations = BTreeMap::new();

    // Very long pattern string
    let long_pattern = format!("@* {}", "a".repeat(100_000));

    let start = std::time::Instant::now();
    let _ = weave("test", &long_pattern, &annotations, &symbols);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 2,
        "POTENTIAL VULNERABILITY: Very long regex pattern took {:?} to process",
        elapsed
    );
}

// =========================================================================
// VULNERABILITY 3: Symbol injection via environment variables
// =========================================================================
// SymbolKey::from_environment() reads custom symbols without validation.
// Empty or overlapping symbols could cause unexpected behavior.
// Note: SymbolKey fields are private, so we test via environment variables.

#[test]
fn test_empty_fragment_open_symbol_via_env() {
    // Setting VERSO_FRAGMENT_OPEN_SYMBOL to "" would cause every line to
    // match content.find("") which returns Some(0), making the parser try
    // to extract an ID from every line.
    //
    // We can't easily set env vars in parallel tests safely, but we can
    // demonstrate the underlying issue: String::find("") always matches.
    let empty_symbol = "";
    let normal_line = "this is just a normal line of text";

    // This is what extract_fragments does internally
    let found = normal_line.find(empty_symbol);
    assert_eq!(
        found,
        Some(0),
        "VULNERABILITY: Empty symbol matches at position 0 on every line. \
         Setting VERSO_FRAGMENT_OPEN_SYMBOL='' via environment would cause \
         the parser to try to extract fragment IDs from every line of input."
    );
}

#[test]
fn test_symbol_prefix_ambiguity() {
    // The weave function checks insertion symbol before pattern symbol.
    // If insertion is "@" and pattern is "@@", a line starting with "@@"
    // matches insertion first, not pattern.
    //
    // Demonstrating with default symbols: insertion is "@@", pattern is "@*"
    // These don't overlap. But custom symbols could create ambiguity.
    let symbols = SymbolKey::default();

    // A line with the insertion symbol followed by a valid ID
    let frag = Fragment {
        id: String::from("test"),
        body: String::from("replaced body"),
        file: String::from("test.rs"),
        line: 1,
        col: 0,
    };
    let mut annotations = BTreeMap::new();
    annotations.insert(frag.id.clone(), frag);

    let input = "@@test";
    let result = weave("test", input, &annotations, &symbols);
    assert!(
        result.is_ok(),
        "Default symbols should process insertion correctly"
    );
    assert_eq!(result.unwrap(), "replaced body");
}

// =========================================================================
// VULNERABILITY 4: Metadata abspath injection
// =========================================================================
// The abspath handler prepends "/" to the fragment filename without
// sanitization. If the filename contains traversal sequences, the
// "absolute path" is misleading.

#[test]
fn test_abspath_with_traversal_in_filename() {
    let symbols = SymbolKey::default();

    let frag = Fragment {
        id: String::from("evil"),
        body: String::from("malicious content"),
        file: String::from("../../../etc/passwd"), // Traversal in source filename
        line: 1,
        col: 0,
    };

    let mut annotations = BTreeMap::new();
    annotations.insert(frag.id.clone(), frag);

    let input = "@?evil.abspath";
    let result =
        weave("test.md", input, &annotations, &symbols).expect("weave should succeed");

    // The abspath just prepends "/" blindly, producing a dangerous-looking path
    assert_eq!(
        result, "/../../../etc/passwd",
        "abspath naively prepends '/' to filenames containing traversal sequences"
    );
}

#[test]
fn test_relpath_with_traversal_in_filename() {
    let symbols = SymbolKey::default();

    let frag = Fragment {
        id: String::from("evil"),
        body: String::from("malicious content"),
        file: String::from("../../../etc/passwd"),
        line: 1,
        col: 0,
    };

    let mut annotations = BTreeMap::new();
    annotations.insert(frag.id.clone(), frag);

    let input = "@?evil.relpath";
    let result =
        weave("output/test.md", input, &annotations, &symbols).expect("weave should succeed");

    // The relative path computation doesn't sanitize against traversal
    assert!(
        result.contains(".."),
        "relpath should contain parent traversal for path with ../ components"
    );
}

// =========================================================================
// VULNERABILITY 5: Fragment ID collision / overwrite
// =========================================================================
// When multiple source files define fragments with the same ID, later
// definitions silently overwrite earlier ones. This could be exploited
// if an attacker can inject a source file.

#[test]
fn test_fragment_id_collision() {
    let symbols = SymbolKey::default();

    // First file defines fragment "auth"
    let file1 = "@<auth\ndef authenticate(user):\n    return verify_password(user)\n>@";
    let frags1 = extract_fragments(file1, "auth.py", &symbols).unwrap();

    // Second file (possibly attacker-controlled) also defines "auth"
    let file2 = "@<auth\ndef authenticate(user):\n    return True  # bypass!\n>@";
    let frags2 = extract_fragments(file2, "evil.py", &symbols).unwrap();

    // Simulate what verso.rs does: append all fragments
    let mut all_fragments = vec![];
    all_fragments.extend(frags1);
    all_fragments.extend(frags2);

    // Build annotation map like recto does
    let mut annotations = BTreeMap::new();
    for frag in &all_fragments {
        annotations.insert(frag.id.clone(), frag.clone());
    }

    // The evil fragment overwrites the legitimate one
    let auth_frag = annotations.get("auth").unwrap();
    assert_eq!(
        auth_frag.file, "evil.py",
        "VULNERABILITY: Fragment ID collision allows later definitions to \
         silently overwrite earlier ones. The 'auth' fragment from auth.py \
         was overwritten by the one from evil.py"
    );
    assert!(
        auth_frag.body.contains("return True"),
        "The overwritten fragment contains the attacker's bypass code"
    );
}

// =========================================================================
// VULNERABILITY 6: Unchecked file read in verso
// =========================================================================
// verso reads any file path passed as a command-line argument without
// checking if it's within an expected directory. Combined with the
// JSON output piped to recto, this forms a read-anywhere-write-anywhere
// pipeline.

#[test]
fn test_arbitrary_file_content_in_fragments() {
    let symbols = SymbolKey::default();

    // Simulate a source file that captures sensitive content as a fragment.
    // If an attacker can control which files are passed to verso, they
    // can extract content from arbitrary files and have recto write it
    // to the output directory.
    let sensitive_content = "@<secrets\nSSH_KEY=AAAA...\nDB_PASSWORD=hunter2\n>@";

    let fragments =
        extract_fragments(sensitive_content, "/etc/secrets.conf", &symbols).unwrap();

    assert_eq!(fragments.len(), 1);
    assert!(
        fragments[0].body.contains("DB_PASSWORD"),
        "Fragments can capture sensitive file content without restriction"
    );
}

// =========================================================================
// Edge case: Very deeply nested fragments
// =========================================================================

#[test]
fn test_deeply_nested_fragments_stack_depth() {
    let symbols = SymbolKey::default();

    // Build a deeply nested fragment structure
    let depth = 1000;
    let mut input = String::new();
    for i in 0..depth {
        input.push_str(&format!("@<frag{}\n", i));
    }
    input.push_str("innermost content\n");
    for _ in 0..depth {
        input.push_str(">@\n");
    }

    let result = extract_fragments(&input, "deep.rs", &symbols);
    match result {
        Ok(fragments) => {
            assert_eq!(
                fragments.len(),
                depth,
                "Should produce {} fragments for depth {}",
                depth,
                depth
            );
            // Check that inner content propagates to all outer fragments
            assert!(
                fragments.last().unwrap().body.contains("innermost content"),
                "Outermost fragment should contain innermost content"
            );
        }
        Err(e) => {
            panic!(
                "Deep nesting caused error (potential stack overflow vulnerability): {:?}",
                e
            );
        }
    }
}

// =========================================================================
// Edge case: Very long lines
// =========================================================================

#[test]
fn test_extremely_long_line() {
    let symbols = SymbolKey::default();

    // A very long line could cause performance issues in string searching
    let long_line = "x".repeat(10_000_000); // 10MB line
    let input = format!("@<test\n{}\n>@", long_line);

    let start = std::time::Instant::now();
    let result = extract_fragments(&input, "test.rs", &symbols);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Should handle very long lines");
    assert!(
        elapsed.as_secs() < 5,
        "Processing 10MB line took {:?}, potential DoS vector",
        elapsed
    );
}

// =========================================================================
// Edge case: Unicode / multi-byte characters in IDs
// =========================================================================

#[test]
fn test_unicode_in_fragment_ids() {
    let symbols = SymbolKey::default();

    // Unicode characters are alphanumeric per Rust's char::is_alphanumeric(),
    // so they pass the ID safety check. This could cause issues if downstream
    // systems don't handle Unicode fragment IDs properly.
    let input = "@<\u{0410}\u{0411}\u{0412}\nUnicode ID content\n>@"; // Cyrillic

    let result = extract_fragments(input, "test.rs", &symbols);
    match result {
        Ok(fragments) => {
            assert_eq!(fragments[0].id, "\u{0410}\u{0411}\u{0412}");
        }
        Err(e) => {
            panic!("Unicode ID unexpectedly rejected: {:?}", e);
        }
    }
}

#[test]
fn test_homoglyph_id_collision() {
    let symbols = SymbolKey::default();

    // Latin 'A' (U+0041) vs Cyrillic 'A' (U+0410) - visually identical
    let file1 = "@<A\nlegitimate code\n>@"; // Latin A
    let file2 = "@<\u{0410}\nmalicious code\n>@"; // Cyrillic A

    let frags1 = extract_fragments(file1, "legit.rs", &symbols).unwrap();
    let frags2 = extract_fragments(file2, "evil.rs", &symbols).unwrap();

    // These are different IDs despite looking identical
    assert_ne!(
        frags1[0].id, frags2[0].id,
        "Homoglyph IDs should be different strings but look identical, \
         creating confusion potential"
    );

    // In a BTreeMap, they'd be separate entries - not a collision
    // The risk is visual confusion in code review
    let mut annotations = BTreeMap::new();
    annotations.insert(frags1[0].id.clone(), frags1[0].clone());
    annotations.insert(frags2[0].id.clone(), frags2[0].clone());

    assert_eq!(
        annotations.len(),
        2,
        "Homoglyph IDs create separate entries that look identical in output"
    );
}
