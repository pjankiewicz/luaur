//! Diagnostic helper: read fuzzer bytes on stdin, print the Luau source that
//! `generate_typed` decodes them into. Lets us turn a `typeck_typed` crash
//! reproducer (raw driver bytes) back into source to feed C++ luau-analyze.
//!   cargo run --release --no-default-features --bin dump_typed < repro.bin
use std::io::Read;

fn main() {
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data).unwrap();
    print!("{}", luaur_fuzz::generate_typed(&data));
}
