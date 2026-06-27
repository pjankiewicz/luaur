// `check_with_definitions` with generated host definition files — the exact
// shape of issue #6's SIGSEGV (repeated calls / larger definitions). The `typeck`
// target only fuzzed `check` (no defs), which is how that crash slipped through.
//
// Throughput note: unlike `typeck`/`typeck_typed`, this target can't use the
// reusable `luaur_rt::Checker`. Host definitions are loaded by *mutating* the
// shared global scope (`load_definition_file`: unfreeze -> load -> freeze), and
// each fuzz input generates *different* defs — reusing one cached environment
// would accumulate/conflict declarations across inputs. Caching the builtins here
// would require snapshotting and restoring the global type scope around every
// input; that's deferred (the global scope is a complex arena-backed structure
// and a faithful snapshot/restore is its own change). So each input pays the
// builtin-registration cost via the one-shot `check_with_definitions`.
//
// The old 3x inner loop (added to reproduce #6's repeated-call crash) is dropped:
// #6 is fixed, and AFL's persistent mode already re-runs the target many times
// per input, so the loop only tripled cost without adding coverage.

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

    let _ = luaur_rt::check_with_definitions(&src, &defs);
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
