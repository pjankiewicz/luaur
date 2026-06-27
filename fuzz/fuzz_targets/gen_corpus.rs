// Seed-corpus generator (a plain tool, not an AFL target). Distills two kinds of
// AFL seed from the in-tree generators so `run_afl.sh` (which prefers
// `corpus/<target>/`) starts from deep inputs instead of cold:
//
//   * SOURCE seeds — valid generated Luau program text (computational + typed +
//     untyped) for the DIRECT-SOURCE targets (compile/run/typeck), which take a
//     program as raw bytes. Complements the real-script corpus from fetch_corpus.
//
//   * distilled BYTE seeds — for the GENERATOR targets, the byte-inputs whose
//     decoded program reaches a deep state: for optdiff/metamorphic, inputs whose
//     computational program RUNS TO COMPLETION (full VM pipeline); for
//     typeck_typed, inputs whose typed program actually type-checks with output.
//
// Usage: `make gen-corpus` (or `cd fuzz && cargo run --no-default-features
// --bin gen_corpus`). Idempotent; safe to re-run.

use std::fs;
use std::path::Path;

fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn rand_bytes(s: &mut u64, len: usize) -> Vec<u8> {
    (0..len).map(|_| (rng(s) & 0xff) as u8).collect()
}

fn write_seed(dir: &str, name: &str, data: &[u8]) {
    let _ = fs::create_dir_all(dir);
    let _ = fs::write(Path::new(dir).join(name), data);
}

fn main() {
    // Fixed seed → reproducible corpus.
    let mut s: u64 = 0x5EED_C0FFEE_1234_u64 | 1;

    // ---- A. SOURCE seeds for the direct-source targets -------------------
    let src_targets = ["compile", "run", "typeck"];
    let mut src_n = 0usize;
    let mut emit_source = |prog: &str, tag: &str, n: usize| {
        for t in src_targets {
            write_seed(
                &format!("corpus/{t}"),
                &format!("gen-{tag}-{n}.luau"),
                prog.as_bytes(),
            );
        }
    };
    for _ in 0..400 {
        let len = 40 + (rng(&mut s) % 400) as usize;
        let b = rand_bytes(&mut s, len);
        emit_source(&luaur_fuzz::generate_computational(&b), "comp", src_n);
        src_n += 1;
    }
    for _ in 0..300 {
        let len = 40 + (rng(&mut s) % 400) as usize;
        let b = rand_bytes(&mut s, len);
        emit_source(&luaur_fuzz::generate_typed(&b), "typed", src_n);
        src_n += 1;
    }
    for _ in 0..200 {
        let len = 40 + (rng(&mut s) % 400) as usize;
        let b = rand_bytes(&mut s, len);
        emit_source(&luaur_fuzz::generate(&b), "untyped", src_n);
        src_n += 1;
    }

    // ---- B. distilled BYTE seeds for EVERY generator target --------------
    // The seed is the bytes we fed. For each target, keep inputs that DECODE to a
    // deep program (one that reaches the target's interesting path), clearing any
    // stale seeds first (e.g. the old mis-wired real-script seeds). We can't
    // invert an arbitrary program to bytes, but we can keep the bytes of the
    // programs we generate that land somewhere useful.

    // Does `src` parse + compile? (proxy for "reaches the compiler/VM".)
    fn compiles(src: &str) -> bool {
        let lua = luaur_rt::Lua::new();
        lua.load(src).set_name("s").into_function().is_ok()
    }

    // Distill `want` byte seeds into each of `dirs` (cleared first), keeping a
    // byte-input when `accept(bytes)` holds. Returns how many were kept.
    let mut distill = |s: &mut u64,
                       dirs: &[&str],
                       want: usize,
                       max_tries: usize,
                       accept: &dyn Fn(&[u8]) -> bool|
     -> usize {
        for d in dirs {
            let _ = fs::remove_dir_all(format!("corpus/{d}"));
        }
        let (mut kept, mut tries) = (0usize, 0usize);
        while kept < want && tries < max_tries {
            tries += 1;
            let len = 40 + (rng(s) % 400) as usize;
            let b = rand_bytes(s, len);
            if accept(&b) {
                for d in dirs {
                    write_seed(&format!("corpus/{d}"), &format!("seed-{kept}.bin"), &b);
                }
                kept += 1;
            }
        }
        kept
    };

    // optdiff / metamorphic: computational program RUNS TO COMPLETION (full VM).
    let n_opt = distill(&mut s, &["optdiff", "metamorphic"], 250, 100_000, &|b| {
        luaur_fuzz::run_observed(&luaur_fuzz::generate_computational(b), 1)
            .map(|o| o.starts_with("ok:"))
            .unwrap_or(false)
    });
    // typeck_typed: typed program type-checks with diagnostics.
    let n_tc = distill(&mut s, &["typeck_typed"], 250, 100_000, &|b| {
        luaur_rt::check(&luaur_fuzz::generate_typed(b)).is_err()
    });
    // structured: program compiles AND runs to completion (reaches the VM).
    let n_struct = distill(&mut s, &["structured"], 250, 80_000, &|b| {
        luaur_fuzz::run_observed(&luaur_fuzz::generate(b), 1).is_some()
    });
    // splice: spliced real-script program compiles + runs (selective — recombined
    // statements often reference undefined locals, so this filters to live ones).
    let n_splice = distill(&mut s, &["splice"], 250, 80_000, &|b| {
        luaur_fuzz::run_observed(&luaur_fuzz::generate_spliced(b), 1).is_some()
    });
    // determinism / roundtrip: untyped program parses + compiles.
    let n_det = distill(&mut s, &["determinism"], 250, 60_000, &|b| {
        compiles(&luaur_fuzz::generate(b))
    });
    let n_rt = distill(&mut s, &["roundtrip"], 250, 60_000, &|b| {
        compiles(&luaur_fuzz::generate(b))
    });
    // typeck_defs: the SCRIPT half (second half of the bytes) compiles.
    let n_defs = distill(&mut s, &["typeck_defs"], 250, 60_000, &|b| {
        let mid = b.len() / 2;
        compiles(&luaur_fuzz::generate(&b[mid..]))
    });

    println!(
        "gen_corpus:\n  {src_n} source seeds -> corpus/{{compile,run,typeck}}\n  \
         byte seeds (bytes that decode to a deep program):\n    \
         optdiff+metamorphic={n_opt}  typeck_typed={n_tc}  structured={n_struct}  \
         splice={n_splice}  determinism={n_det}  roundtrip={n_rt}  typeck_defs={n_defs}"
    );
}
