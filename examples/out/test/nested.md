# Out

## See the `main fn`

```rust
// test/nested.rs

fn main() {
    let stdout = stdout();
    let message = String::from("Hello fellow Rustaceans!");
    let width = message.chars().count();

    let mut writer = BufWriter::new(stdout.lock());
    say(&message, width, &mut writer).unwrap();
}
```

## Take note of this special line

```rust
    let message = String::from("Hello fellow Rustaceans!");
```
