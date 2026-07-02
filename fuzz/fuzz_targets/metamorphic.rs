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
    // First half drives the program; second half drives the transforms, so AFL
    // can vary the transform independently of the program.
    let mid = data.len() / 2;
    let src = luaur_fuzz::generate_computational(&data[..mid]);
    let tail = &data[mid..];

    // Two behavior-preserving transforms, each a distinct oracle:
    //   * no-op insertion — statement interleaving / scoping / lowering.
    //   * dead-branch insertion — buries REAL (but unreachable) code in
    //     `if false` / `while false`, which the optimizer must prove dead and
    //     eliminate. A divergence here is a dead-code-elimination / const-fold /
    //     jump-threading bug (the classic Equivalence-Modulo-Inputs signal).
    let noop = luaur_fuzz::metamorphic_noop(&src, tail);
    let dead = luaur_fuzz::metamorphic_dead_branch(&src, tail);

    // Same opt level for all — we're isolating the transform, not the optimizer.
    let a = luaur_fuzz::run_observed(&src, 1);
    let b = luaur_fuzz::run_observed(&noop, 1);
    let c = luaur_fuzz::run_observed(&dead, 1);

    if let (Some(a), Some(b)) = (a.clone(), b) {
        assert!(
            a == b,
            "no-op transform changed behavior:\n  original  = {a}\n  transformed = {b}\n--- original ---\n{src}\n--- transformed ---\n{noop}"
        );
    }
    if let (Some(a), Some(c)) = (a, c) {
        assert!(
            a == c,
            "dead-branch transform changed behavior:\n  original  = {a}\n  transformed = {c}\n--- original ---\n{src}\n--- transformed ---\n{dead}"
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
