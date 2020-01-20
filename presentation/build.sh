#! /bin/bash
# @<buildsh
set -ex

if [ -d build ]; then
    rm -rf build/*
else
    mkdir build
fi
    
verso ../src/lib.rs \
      ../src/verso.rs \
      ../src/recto.rs build.sh  \
    | recto build presentation.md
# >@buildsh

echo "Run a local HTTP server to view the presentation."
echo "python -m SimpleHTTPServer 5000 > http://localhost:5000"
