use ferris_says::say;
use std::io::{stdout, BufWriter};

// @<mainfn
fn main() {
    let stdout = stdout();
    // @<mainfnmessage
    let message = String::from("Hello fellow Rustaceans!");
    // >@
    let width = message.chars().count();

    let mut writer = BufWriter::new(stdout.lock());
    say(&message, width, &mut writer).unwrap();
}
// >@
