// Differential oracle (the doc's "probably your strongest strategy"):
// optimization must PRESERVE observable behavior. The same deterministic program
// is compiled + run at -O0, -O1, and -O2; the observation (captured `print`
// output + final ok/err status) must be identical across all three. A divergence
// is a miscompilation / optimizer bug.
//
// NB: this is OUR OWN compiler's optimization levels — not a differential against
// a C++ reference (explicitly out of scope). Inconclusive runs (didn't compile,
// or hit the step limit — optimization changes step counts) are skipped, so the
// oracle only fires on a genuine behavioral divergence.

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate_computational(data);

    let o0 = luaur_fuzz::run_observed(&src, 0);
    let o1 = luaur_fuzz::run_observed(&src, 1);
    let o2 = luaur_fuzz::run_observed(&src, 2);

    // Only compare when every level produced a conclusive observation.
    if let (Some(a), Some(b), Some(c)) = (o0, o1, o2) {
        assert!(
            a == b && b == c,
            "optimization changed observable behavior:\n  O0 = {a}\n  O1 = {b}\n  O2 = {c}\n--- program ---\n{src}"
        );
    }
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    {
        luaur_fuzz::install_afl_panic_hook();
        fuzz_nohook!(|data: &[u8]| {
            exercise_input(data);
        });
    }
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
