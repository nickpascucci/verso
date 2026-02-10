# Test Coverage Analysis

## Current State

The codebase has **21 unit tests** in `src/lib.rs` and **0 tests** in the two binary
crates (`src/verso.rs`, `src/recto.rs`). There is a shell-based integration test
(`examples/check-examples.sh`) that is **not run in CI** — the GitHub Actions workflow
only runs `cargo test`.

## What's Covered

| Function | Tests | Covered Scenarios |
|---|---|---|
| `extract_id` | 6 | Empty input, valid ID, whitespace termination, safe separators, reserved chars, column offset |
| `extract_pattern` | 3 | Valid regex, whitespace-only input, invalid regex |
| `extract_fragments` | 6 | Basic extraction, nested fragments, close-before-open, halt, halt-while-open, missing ID |
| `weave` | 5 | Happy path (insertion + pattern + metadata), pattern ordering, missing fragment, bad metadata type, bad metadata ID |
| `find_relative_path` | 1 | Three path scenarios (sibling, ancestor, diverged) |

## Gaps and Recommendations

### 1. `extract_fragments` — UnclosedFragment error path (High)

**Location:** `src/lib.rs:248-256`

The `UnclosedFragment` error branch is never tested. This triggers when the input ends
with a fragment still open (no matching close symbol).

### 2. `extract_fragments` — ReservedCharacterUsed in fragment ID (High)

**Location:** `src/lib.rs:186-198`

The `IdExtractError::ReservedCharacterUsed` branch within `extract_fragments` is never
tested. While `extract_id` tests reserved characters in isolation, the error propagation
path inside `extract_fragments` is untested.

### 3. Binary crate `Config` and `run` functions — Zero coverage (High)

**Location:** `src/verso.rs:35-62`, `src/recto.rs:37-94`

Both binaries have `pub` functions `Config::new()` and `run()` with zero test coverage:

- `verso::Config::new` accepts any args without validation
- `recto::Config::new` has argument count validation that is untested
- `recto::run` contains substantial logic for JSON parsing, directory creation, and file
  writing

### 4. `weave` — Missing error paths (Medium)

**Location:** `src/lib.rs:289-388`

Several `weave` error branches lack tests:

- `WeaveError::IdExtractError` — reserved character in an insertion ID
- `WeaveError::PatternExtractError` (no pattern) — bare pattern symbol with nothing after
- `WeaveError::PatternExtractError` (bad regex) — invalid regex in pattern insertion

### 5. `expand_metadata_refs` — Edge cases (Medium)

**Location:** `src/lib.rs:398-493`

- `MetadataParseError` in `ReadingMetaStart` state — partial metadata symbol match
- `MetadataParseError` in `ReadingId` state — invalid character after metadata symbol
- Mixed valid/invalid metadata refs on one line

### 6. `expand_metadata` — Individual property types (Medium)

**Location:** `src/lib.rs:496-549`

- `abspath` property — never tested
- `relpath` property — never tested through `weave`
- `BadMetadata` — wrong number of `.`-separated parts (e.g., `@?a.b.c`)

### 7. `SymbolKey::from_environment` — Custom symbols (Medium)

**Location:** `src/lib.rs:53-67`

All tests use `SymbolKey::default()`. The environment-variable override path is entirely
untested.

### 8. `weave` — Trailing newline handling (Low)

**Location:** `src/lib.rs:380-383`

The logic for preserving a trailing newline in weave output is never explicitly tested.

### 9. `find_relative_path` — Edge cases (Low)

**Location:** `src/lib.rs:552-589`

Missing edge cases: identical paths, one path is a prefix of the other, root paths.

### 10. Integration test script not in CI (Infrastructure)

**Location:** `.github/workflows/rust.yml`

`examples/check-examples.sh` validates the end-to-end pipeline but is not run in CI.

## Summary

| Area | Current Tests | Gap |
|---|---|---|
| `extract_id` | 6 | Adequate |
| `extract_pattern` | 3 | Adequate |
| `extract_fragments` | 6 | Missing: UnclosedFragment, ReservedChar propagation |
| `weave` | 5 | Missing: 3 error paths |
| `expand_metadata_refs` | Indirect | Missing: MetadataParseError paths, abspath/relpath |
| `find_relative_path` | 1 (3 cases) | Missing: identical paths, prefix paths |
| `SymbolKey::from_environment` | 0 | Entirely untested |
| `verso::Config` / `verso::run` | 0 | Entirely untested |
| `recto::Config` / `recto::run` | 0 | Entirely untested |
| CI integration test | Not run | `check-examples.sh` excluded from CI |

### Highest-Impact Improvements

1. Add missing error-path unit tests for `extract_fragments` and `weave`
2. Add tests for the binary crate logic (`Config::new`, `run`)
3. Add `check-examples.sh` to CI or convert to Rust integration tests
4. Test `SymbolKey::from_environment` with env var overrides
