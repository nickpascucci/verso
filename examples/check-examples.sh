#! /bin/bash

set -e

DIFF=$(which colordiff || echo "diff")

SOURCE_FILES="example.py test/example-2.py test/nested.rs"
PROSE_FILES="empty.md example.md test/example-2.md test/level-2/l2.md"
OUTPUT_DIRECTORY=out

cd "$(dirname "$0")/.."

cargo build

cd examples

rm -rf $OUTPUT_DIRECTORY || true

../target/debug/verso $SOURCE_FILES \
    | tee /tmp/verso.json \
    | ../target/debug/recto $OUTPUT_DIRECTORY $PROSE_FILES

GENFILES=$(find $OUTPUT_DIRECTORY -type f | sed "s|$OUTPUT_DIRECTORY/||")

function md5() {
    md5sum $1 | awk '{ print $1 }'
}

function same_checksum() {
    [ $(md5 $1) = $(md5 $2) ]
}

DIFFS_FOUND=0

for GENFILE in $GENFILES; do
    RESULT="$OUTPUT_DIRECTORY/$GENFILE"
    REFERENCE="reference/$GENFILE"
    echo
    echo "Checking weave result $RESULT against $REFERENCE"
    if ! $DIFF $RESULT $REFERENCE || ! same_checksum $RESULT $REFERENCE; then
        echo "BAD: Found differences against reference file."
        DIFFS_FOUND=1
    else
        echo "GOOD: Result matches reference."
    fi 
done

echo

if [ $DIFFS_FOUND -gt 0 ]; then
    echo "Differences found. Please examine diffs above."
    exit 1
else
    echo "All of the woven examples match their references."
fi
