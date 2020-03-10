# Traditional Literate Programming

Most LP tools, such as CWEB, noweb, and org-babel, take the following approach. As the programmer
writes prose describing their thought process and the theory behind their program, they can
introduce code within named "blocks". These blocks can reference other blocks, providing a form of
macro abstraction over the primary programming language. Crucially, blocks are not required to be
introduced in the order that the compiler might require them; rather, they are introduced in
whatever order makes the underlying logic the clearest. To satisfy the compiler the blocks can be
rearranged through the liberal use of cross references and embeddings, generally driven through a
top-level block which is creates the compilation target.

Can we simulate this with `verso|recto`? I think so. Let's try.

_Note that this file is intended to simulate traditional LP, so this file is actually a source file._

We begin at the end: with `main`.

```rust
// @<main
fn main() {
  @@vars
  @@printables
  @@post
}
// >@
```

As you can see, `main` has a few interesting syntactic choices. It has a pretty standard Rust
function declaration, but it also includes both fragment markers and insertion markers. We'll see in
a little bit how these will interact. The next step in our program will be to define some variables
that we'll need for our program.

```rust
// @<vars
  let x = 1;
  let y = "Hi, I'm Y!";
  let z = (0..10).map(|a| a + 1).fold(0, |a, b| a + b);
// >@
```

After we're done we'll go ahead and clean up some stuff:

```rust
// @<post
  panic!("The program is over?! ABORT!");
// >@
```

But before that happens, we need to do some work.

```rust
// @<printables
  println!("x = {}", x);
  println!("z ({}) + x ({}) = {}", z, x, z + x);
  println!("Y says: {}", y);
// >@
```

Whew! That was a lot of code. How will we pull it all together? The key is this script:

```bash
# @<build
  set -ex
  verso traditional.md > /tmp/trad-fragments.json
  cat /tmp/trad-fragments.json | recto phase1 traditional.md
  cd phase1
  echo "@@main" > main.rs
  verso traditional.md | recto phase2 main.rs
  cd phase2
  rustc main.rs
  ./main
# >@
```

(You might also want to take a look at `build.sh`, in this directory.)

To kick things off, run this command:

```bash
  verso traditional.md | recto phase0 build.sh && bash phase0/build.sh
```

------------

Pretty cool, huh? I think this conclusively demonstrates that we can do literate programming in the
traditional style using `verso|recto`.

Of course, `verso|recto` is more than just an LP tool; it's a generalized snippet extraction and
insertion tool. It can obviously be used for literate programming, but if you wanted to go crazy one
could imagine all sorts of interesting applications for copying and pasting text between multiple
files.
