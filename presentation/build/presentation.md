<!-- The next comment contains a verso fragment identifier. There is a matching comment at the end
of the file. Everything between the two comments will be extracted by verso and woven into the
presentation by recto. -->

 <!-- @<1 -->

# Rust, by example

-----

## Example 1

This is the script that builds this presentation:

```
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
```

-----

## Slide 2

Here's another slide.

<!-- >@1 -->