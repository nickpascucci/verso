#! /bin/bash

set -e

cargo build

cd examples

rm -rf out || true

../target/debug/verso example.py test/example-2.py \
    | ../target/debug/recto out example.md test/example-2.md test/level-2/l2.md

GENFILES=$(find out -type f | sed 's|out/||')

for GENFILE in $GENFILES; do
    if ! diff "out/$GENFILE" "reference/$GENFILE"; then
        echo "Found differences in $GENFILE"
        exit 1
    fi
done

echo "All of the woven examples match their references."
