# Rust, by example

_Illustrated along the way by `verso|recto`_

-----

I'm going to use my side project, `verso|recto`, to illustrate some basic Rust
ideas. You can find the source at:

https://github.com/nickpascucci/verso

-----

_Rust_ is a systems programming language, like C.

That means it's intended for use in writing things that need to be fast, close to the metal, or
efficient in their memory use.

Things like operating systems, embedded device firmware, web browsers, and so on.

vvvvv

So what makes Rust special?

1. Static memory safety
2. Expressive, high-level language constructs
3. Direct control of memory when needed
4. Excellent tools, such as Cargo

vvvvv

![](examples/comparison.png)

-----

## Memory Safety

Unlike C, Rust is _memory safe_: it is not possible in Rust to inadvertently do bad things with your
memory.

Safety is enforced both statically by the compiler, and with a few runtime checks where needed.

These are the same checks needed for safe C, but done by the machine rather than you.

vvvvv

In Rust, every piece of data has an _owner_. When a variable's owner goes out of scope, it is deallocated.

This helps avoid bugs like:

```c
@@ex1c
```

vvvvv

Rust, you simply can't write a double free:

```rust
@@ex1rs
```

![](examples/ex1.png)

-----

## High Level Language

In C we use return codes for errors, but it's easy to ignore them.

```rust
@@ex3rs
```

![](examples/ex3.png)

vvvvv

We can use algebraic enumerations and structures to create rich data models:

```rust
@@errors
```

-----

## Direct Control

The `unsafe` keyword tells the compiler to trust you.

You can use raw pointers and other low-level tools to implement features that it can't prove are safe.

-----

## Tooling

Unit testing support is built into the language.

```rust
@@tests
```

vvvvv

Cargo includes:
 - Compiler
 - Package manager
 - Source formatter
 - From-source installation support
 - Custom build scripts, written in Rust
 - Cross compilation
 - Extensible via Rust libraries
 - etc.

-----

`verso|recto` is a literate programming tool that lets you include source code
in prose documents.

Documents like this slide show.

vvvvv

## Example 1

To illustrate, this is the script that builds this presentation:

```bash
@@buildsh
```
