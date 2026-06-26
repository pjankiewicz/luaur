// `check_with_definitions` with generated host definition files, called
// **repeatedly in-process** — the exact shape of issue #6's SIGSEGV (repeated
// calls / larger definitions). The `typeck` target only fuzzed `check` (no
// defs), which is how that crash slipped through. AFL already re-runs the target
// across inputs; the inner loop additionally hammers the repeated-call path
// within a single input.

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    // Split the fuzzer bytes: first half drives the definition file, second the
    // script that type-checks against it.
    let mid = data.len() / 2;
    let defs = luaur_fuzz::generate_definitions(&data[..mid]);
    let src = luaur_fuzz::generate(&data[mid..]);

    for _ in 0..3 {
        let _ = luaur_rt::check_with_definitions(&src, &defs);
    }
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
