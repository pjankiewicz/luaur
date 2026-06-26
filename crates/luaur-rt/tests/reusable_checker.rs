//! `Checker` (the reusable type checker that caches the builtin global env) must
//! produce the SAME result as the one-shot `check`, and must not bleed state
//! across reused checks. It's ~100x faster than `check` per call (it skips
//! re-registering the `@luau` builtins) — these tests guard that the speed didn't
//! cost correctness.
#![cfg(feature = "typecheck")]

use luaur_rt::Checker;

#[test]
fn checker_matches_one_shot_check() {
    let cases = [
        "return 1 + 2",                                                        // clean
        "local x: number = \"s\"\nreturn x",                                   // type error
        "local function f(a: string): number return #a end\nreturn f(\"hi\")", // clean
        "@#$ not lua",                                                         // parse error
        "return",                                                              // clean
        "local t: {x: number} = {x = true}\nreturn t",                         // type error
    ];
    let mut c = Checker::new();
    for src in cases {
        let reused = c.check(src);
        let one_shot = luaur_rt::check(src);
        assert_eq!(
            reused.is_ok(),
            one_shot.is_ok(),
            "ok/err mismatch vs one-shot check for: {src:?}"
        );
        let reused_n = reused.err().map(|d| d.len()).unwrap_or(0);
        let one_shot_n = one_shot.err().map(|d| d.len()).unwrap_or(0);
        assert_eq!(
            reused_n, one_shot_n,
            "diagnostic count mismatch vs one-shot check for: {src:?}"
        );
    }
}

#[test]
fn checker_does_not_bleed_state_across_reuse() {
    // Re-checking the same source must give the same verdict no matter what was
    // checked in between (the global env is cached, but per-module state must be
    // reset by mark_dirty).
    let mut c = Checker::new();
    let probe = "local x: number = 1\nreturn x + \"s\""; // a type error
    let expected = c.check(probe).is_ok();
    assert!(!expected, "probe should be a type error");
    for _ in 0..100 {
        let _ = c.check("return 1");
        let _ = c.check("local t = {} return t.missing");
        let _ = c.check("@bad");
        assert_eq!(
            c.check(probe).is_ok(),
            expected,
            "verdict drifted across reuse"
        );
    }
}
