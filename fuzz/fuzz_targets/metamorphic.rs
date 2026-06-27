// Metamorphic oracle: a behavior-preserving source transform must NOT change
// observable behavior. We take a deterministic program and sprinkle no-op
// statements (`do end`, `;`, fresh unused locals) before its top-level
// statements — a transform that provably preserves semantics — then check that
// the captured `print` output + ok/err status is identical to the original.
//
// A divergence means a parser/scoping/lowering/codegen bug that the cosmetically
// different (but equivalent) program exposes — the classic Equivalence-Modulo-
// Inputs / metamorphic compiler-testing idea. Inconclusive runs are skipped.

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    // First half drives the program; second half drives the no-op insertions, so
    // AFL can vary the transform independently of the program.
    let mid = data.len() / 2;
    let src = luaur_fuzz::generate_computational(&data[..mid]);
    let transformed = luaur_fuzz::metamorphic_noop(&src, &data[mid..]);

    // Same opt level for both — we're isolating the transform, not the optimizer.
    let a = luaur_fuzz::run_observed(&src, 1);
    let b = luaur_fuzz::run_observed(&transformed, 1);

    if let (Some(a), Some(b)) = (a, b) {
        assert!(
            a == b,
            "behavior-preserving transform changed behavior:\n  original  = {a}\n  transformed = {b}\n--- original ---\n{src}\n--- transformed ---\n{transformed}"
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
