#!/bin/bash
#
# Demonstrates the --annotations-file flag for verso.
#
# This script shows how to merge annotations from a stored JSON file with
# annotations extracted from source code. When both sources define the same
# annotation ID, the extracted (source code) version wins.
#
# Usage: ./run.sh

set -euo pipefail

cd "$(dirname "$0")/../.."

echo "=== Building verso and recto ==="
cargo build --quiet

VERSO=./target/debug/verso
RECTO=./target/debug/recto

DIR=examples/annotations-file
OUT_DIR="$DIR/out"
rm -rf "$OUT_DIR"

echo
echo "=== Stored annotations (from file) ==="
echo "The file stored-annotations.json contains three annotations:"
echo "  - intro/description  (unique to file)"
echo "  - demo/greeting      (CONFLICT: also in source.py)"
echo "  - demo/handwritten-note (unique to file)"
echo
echo "=== Source annotations (extracted from source.py) ==="
echo "The file source.py contains two annotations:"
echo "  - demo/greeting      (CONFLICT: also in stored file)"
echo "  - demo/source-only   (unique to source)"

echo
echo "=== Running verso with --annotations-file ==="
$VERSO --annotations-file "$DIR/stored-annotations.json" "$DIR/source.py" \
    | $RECTO "$OUT_DIR" "$DIR/document.md"

echo
echo
echo "=== Woven output ==="
cat "$OUT_DIR/$DIR/document.md"

echo
echo "=== Verifying conflict resolution ==="
if grep -q "NEW version extracted from source" "$OUT_DIR/$DIR/document.md"; then
    echo "PASS: Extracted annotation won the conflict for demo/greeting."
else
    echo "FAIL: Stored annotation was not overridden."
    exit 1
fi

if grep -q "only exists in the stored file" "$OUT_DIR/$DIR/document.md"; then
    echo "PASS: Stored-only annotation (demo/handwritten-note) is present."
else
    echo "FAIL: Stored-only annotation is missing."
    exit 1
fi

if grep -q "Goodbye!" "$OUT_DIR/$DIR/document.md"; then
    echo "PASS: Source-only annotation (demo/source-only) is present."
else
    echo "FAIL: Source-only annotation is missing."
    exit 1
fi

echo
echo "All checks passed."
