//! Generative fuzzing of the compile + run pipeline.
//!
//! ## What this is
//!
//! A language fuzzer does not type random *bytes* at the parser — most would be
//! rejected at the first character and never reach the interesting code. Instead
//! it *generates programs*: it walks a grammar to emit syntactically-valid Luau
//! (so the input reaches deep into the compiler and VM), and also emits a
//! fraction of deliberately-**corrupted** programs (to exercise the error
//! paths). Every program is driven by a `u64` seed, so any failure is a
//! one-line, deterministic repro.
//!
//! ## The oracle (what counts as a bug)
//!
//! The implementation must **never panic, abort, hang, or exhibit UB** on *any*
//! input. It must always either succeed or return a structured `Err`
//! (`SyntaxError`, `RuntimeError`, ...). So:
//!
//! * compiling any program returns `Ok(function)` **or** `Err(SyntaxError)` —
//!   never a panic;
//! * running any compiled program returns `Ok`/`Err` — never a panic, and never
//!   runs forever (a step-limit interrupt bounds it).
//!
//! A panic escaping `catch_unwind` is the bug signal. This is the same class of
//! defect as issue #3 (a compiler error-path that over-read / failed to build) —
//! "it didn't crash" fuzzing is the cheapest, highest-yield oracle for an
//! interpreter. (Differential testing against reference Lua — comparing *output*
//! — is a stronger oracle and a natural next step, but is out of scope here.)
//!
//! ## Running it
//!
//! ```text
//! cargo nextest run -p luaur-rt --test fuzz_generated      # CI default (fixed seeds)
//! FUZZ_ITERS=200000 cargo nextest run -p luaur-rt --test fuzz_generated   # soak locally
//! FUZZ_SEED=12345   cargo nextest run -p luaur-rt --test fuzz_generated   # a chosen run
//! ```
//!
//! Seeds are fixed by default, so a given commit either always passes or always
//! *fails the same seed* — the inputs are reproducible. The default iteration
//! counts are kept low so this is a fast bounded smoke that finishes well inside
//! nextest's 30s cap even on a loaded CI runner (an over-long run once *timed
//! out* on macOS — a wall-clock flake, not a seed flake). Real fuzzing is AFL
//! (`fuzz/`, `make fuzz-<target>`); crank `FUZZ_ITERS` here for a deeper local soak.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use luaur_rt::{Lua, Result, VmState};

// ---------------------------------------------------------------------------
// A tiny dependency-free PRNG (SplitMix64) — deterministic and seedable.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
    /// True with probability `num/den`.
    fn chance(&mut self, num: u32, den: u32) -> bool {
        self.below(den) < num
    }
}

// ---------------------------------------------------------------------------
// The generator: emits syntactically-valid Luau into a String.
// ---------------------------------------------------------------------------

// A small identifier pool. Short and reused so generated code interacts
// (assign/read the same names). None are reserved words. Referencing an
// undefined name is valid Luau (it reads the global, which is `nil`), so the
// generator needs no scope tracking to stay *syntactically* valid.
const NAMES: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h", "x", "y", "z"];

// Globals that are safe to call (no stdout spam, total functions).
const SAFE_GLOBALS: &[&str] = &["type", "tostring", "tonumber", "select", "rawequal"];

struct Gen {
    rng: Rng,
    out: String,
    /// Remaining "size budget"; when it runs out the generator emits only
    /// leaves, guaranteeing termination and bounded output.
    budget: i32,
    /// `break`/`continue` are only legal inside a loop.
    loop_depth: u32,
}

impl Gen {
    fn new(seed: u64) -> Gen {
        Gen {
            rng: Rng::new(seed),
            out: String::with_capacity(512),
            budget: 220,
            loop_depth: 0,
        }
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn name(&mut self) {
        let n = *self.rng.pick(NAMES);
        self.out.push_str(n);
    }

    fn out_of_budget(&self) -> bool {
        self.budget <= 0
    }

    // --- expressions -------------------------------------------------------

    fn expr(&mut self) {
        self.budget -= 1;
        if self.out_of_budget() {
            self.atom();
            return;
        }
        match self.rng.below(10) {
            0 | 1 => self.atom(),
            2 => self.name(),
            3 => {
                // binary op
                self.expr();
                let op = *self.rng.pick(&[
                    " + ", " - ", " * ", " / ", " % ", " ^ ", " .. ", " == ", " ~= ", " < ",
                    " <= ", " > ", " >= ", " and ", " or ",
                ]);
                self.push(op);
                self.expr();
            }
            4 => {
                let op = *self.rng.pick(&["-", "not ", "#"]);
                self.push(op);
                self.expr();
            }
            5 => {
                self.push("(");
                self.expr();
                self.push(")");
            }
            6 => self.table(),
            7 => {
                // index
                self.name();
                if self.rng.chance(1, 2) {
                    self.push("[");
                    self.expr();
                    self.push("]");
                } else {
                    self.push(".");
                    self.name();
                }
            }
            8 => {
                // call: a safe global, a (possibly nil) local, or a method call
                // `a:b(..)` (exercises the NAMECALL opcode + its inline cache).
                // Runtime errors from calling nil are caught — compile is fine.
                match self.rng.below(3) {
                    0 => {
                        let g = *self.rng.pick(SAFE_GLOBALS);
                        self.push(g);
                    }
                    1 => self.name(),
                    _ => {
                        self.name();
                        self.push(":");
                        self.name();
                    }
                }
                self.call_args();
            }
            _ => self.func_expr(),
        }
    }

    fn atom(&mut self) {
        match self.rng.below(6) {
            0 => {
                let n = self.rng.below(1000);
                self.out.push_str(&n.to_string());
            }
            1 => {
                let n = self.rng.below(1000);
                let f = self.rng.below(100);
                self.out.push_str(&format!("{n}.{f}"));
            }
            2 => self.push("true"),
            3 => self.push("false"),
            4 => self.push("nil"),
            _ => {
                // a short alphanumeric string literal (no escapes to keep valid)
                self.push("\"");
                let len = self.rng.below(6);
                for _ in 0..len {
                    let c = b"abcdeABCDE01234"[self.rng.below(15) as usize];
                    self.out.push(c as char);
                }
                self.push("\"");
            }
        }
    }

    fn table(&mut self) {
        self.push("{");
        let n = self.rng.below(4);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            match self.rng.below(3) {
                0 => self.expr(),
                1 => {
                    self.name();
                    self.push(" = ");
                    self.expr();
                }
                _ => {
                    self.push("[");
                    self.expr();
                    self.push("] = ");
                    self.expr();
                }
            }
        }
        self.push("}");
    }

    fn call_args(&mut self) {
        self.push("(");
        let n = self.rng.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.expr();
        }
        self.push(")");
    }

    fn func_expr(&mut self) {
        self.push("function(");
        let n = self.rng.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.name();
        }
        self.push(") ");
        // A function body is its own loop scope for break/continue.
        let saved = self.loop_depth;
        self.loop_depth = 0;
        self.block(2);
        self.loop_depth = saved;
        self.push(" end");
    }

    // --- statements --------------------------------------------------------

    fn block(&mut self, max_stmts: u32) {
        let n = self.rng.below(max_stmts + 1);
        for _ in 0..n {
            if self.out_of_budget() {
                break;
            }
            self.stmt();
            self.push("\n");
        }
        // Optional trailing `return` (must be the final statement of a block).
        if self.rng.chance(1, 4) {
            self.push("return");
            if self.rng.chance(1, 2) {
                self.push(" ");
                self.expr();
            }
            self.push("\n");
        }
    }

    fn stmt(&mut self) {
        self.budget -= 1;
        if self.out_of_budget() {
            // cheapest valid statement
            self.push("local ");
            self.name();
            self.push(" = ");
            self.atom();
            return;
        }
        match self.rng.below(12) {
            0 => {
                self.push("local ");
                self.name();
                self.push(" = ");
                self.expr();
            }
            1 => {
                // plain or compound assignment (`a = e`, `a += e`, `a ..= e`, …)
                self.name();
                let op = *self
                    .rng
                    .pick(&[" = ", " += ", " -= ", " *= ", " /= ", " %= ", " ..= "]);
                self.push(op);
                self.expr();
            }
            2 => {
                self.push("if ");
                self.expr();
                self.push(" then\n");
                self.block(2);
                if self.rng.chance(1, 2) {
                    self.push("else\n");
                    self.block(2);
                }
                self.push("end");
            }
            3 => {
                self.push("while ");
                self.expr();
                self.push(" do\n");
                self.loop_depth += 1;
                self.block(2);
                self.loop_depth -= 1;
                self.push("end");
            }
            4 => {
                self.push("for ");
                self.name();
                self.push(" = ");
                self.expr();
                self.push(", ");
                self.expr();
                if self.rng.chance(1, 2) {
                    self.push(", ");
                    self.expr();
                }
                self.push(" do\n");
                self.loop_depth += 1;
                self.block(2);
                self.loop_depth -= 1;
                self.push("end");
            }
            5 => {
                self.push("repeat\n");
                self.loop_depth += 1;
                self.block(2);
                self.loop_depth -= 1;
                self.push("until ");
                self.expr();
            }
            6 => {
                self.push("do\n");
                self.block(2);
                self.push("end");
            }
            7 => {
                self.push("local function ");
                self.name();
                self.func_tail();
            }
            8 => {
                self.push("function ");
                self.name();
                self.func_tail();
            }
            9 => {
                // call statement
                if self.rng.chance(1, 2) {
                    let g = *self.rng.pick(SAFE_GLOBALS);
                    self.push(g);
                } else {
                    self.name();
                }
                self.call_args();
            }
            10 if self.loop_depth > 0 => self.push("break"),
            11 if self.loop_depth > 0 => self.push("continue"),
            _ => {
                // fallback when break/continue aren't legal here
                self.name();
                self.push(" = ");
                self.expr();
            }
        }
    }

    fn func_tail(&mut self) {
        self.push("(");
        let n = self.rng.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.name();
        }
        self.push(")\n");
        let saved = self.loop_depth;
        self.loop_depth = 0;
        self.block(3);
        self.loop_depth = saved;
        self.push("end");
    }
}

/// Generate a syntactically-valid Luau program for `seed`.
fn gen_program(seed: u64) -> String {
    let mut g = Gen::new(seed);
    g.block(6);
    g.out
}

/// Corrupt `src` a little (truncate, delete, or inject a stray token) so the
/// parser/compiler error path is fuzzed too. The result is usually invalid Luau.
fn corrupt(src: &str, seed: u64) -> String {
    let mut rng = Rng::new(seed ^ 0x5151_5151_5151_5151);
    let bytes = src.as_bytes();
    if bytes.is_empty() {
        return String::from("end end");
    }
    match rng.below(4) {
        0 => {
            // truncate at a random point (unterminated block/expr)
            let at = rng.below(bytes.len() as u32) as usize;
            String::from_utf8_lossy(&bytes[..at]).into_owned()
        }
        1 => {
            // inject a stray closing/opening token
            let at = rng.below(bytes.len() as u32) as usize;
            let tok = *rng.pick(&["end", ")", "(", "}", "then", "]", "::"]);
            let mut s = String::from_utf8_lossy(&bytes[..at]).into_owned();
            s.push_str(tok);
            s.push_str(&String::from_utf8_lossy(&bytes[at..]));
            s
        }
        2 => {
            // delete a random byte (may produce invalid UTF-8-safe token soup)
            let at = rng.below(bytes.len() as u32) as usize;
            let mut s = String::from_utf8_lossy(&bytes[..at]).into_owned();
            s.push_str(&String::from_utf8_lossy(
                &bytes[(at + 1).min(bytes.len())..],
            ));
            s
        }
        _ => {
            // duplicate a slice
            let at = rng.below(bytes.len() as u32) as usize;
            let mut s = src.to_string();
            s.push_str(&String::from_utf8_lossy(&bytes[..at]));
            s
        }
    }
}

// ---------------------------------------------------------------------------
// Harness config
// ---------------------------------------------------------------------------

fn iters(default: u64) -> u64 {
    std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn base_seed() -> u64 {
    std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xA5F0_1234_C0DE_0001)
}

/// Silence the default panic hook for the duration of a fuzz test. luaur's
/// compiler raises syntax errors via `panic_any` and catches them below the
/// `Lua` boundary (compile returns `Err`), but the *hook* still fires per error
/// and would flood stderr across thousands of corrupted inputs. We detect real
/// bugs via `catch_unwind` + our own repro print, so the hook output is pure
/// noise here. (nextest runs each test in its own process, so this is local.)
fn quiet_panics() {
    std::panic::set_hook(Box::new(|_| {}));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Compiling any generated (valid or corrupted) program must never panic — only
/// `Ok` or `Err(SyntaxError)`.
#[test]
fn fuzz_compile_never_panics() {
    quiet_panics();
    // A fast, bounded smoke — NOT a soak. The real fuzzing is AFL (see fuzz/ +
    // `make fuzz-compile`); this just guards the common path on every test run, so
    // keep the default low enough to finish well inside nextest's 30s cap on a
    // loaded CI runner. Crank `FUZZ_ITERS` locally for depth.
    let n = iters(500);
    for i in 0..n {
        let seed = base_seed().wrapping_add(i);
        // ~1 in 4 inputs is corrupted to fuzz the error path.
        let valid = gen_program(seed);
        let src = if i % 4 == 3 {
            corrupt(&valid, seed)
        } else {
            valid
        };

        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let lua = Lua::new();
            // Ok (compiled) and Err (syntax error) are both acceptable.
            let _ = lua.load(&src).set_name("fuzz").into_function();
        }));

        assert!(
            outcome.is_ok(),
            "COMPILE PANICKED on seed {seed}\n---- source ----\n{src}\n----------------"
        );
    }
}

/// Compiling **and running** any generated program must never panic or hang —
/// execution is bounded by a step-limit interrupt, and `Ok`/`Err` are both fine.
#[test]
fn fuzz_run_never_panics() {
    quiet_panics();
    // Compile + run is the slowest path; 800 iters timed out (>30s) on a loaded
    // macOS CI runner. Keep the default a fast smoke; deep fuzzing is AFL's job
    // (`make fuzz-run`). Crank `FUZZ_ITERS` locally for depth.
    let n = iters(200);
    for i in 0..n {
        let seed = base_seed() ^ 0x0000_BEEF_0000_0001 ^ i;
        let src = gen_program(seed);

        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let lua = Lua::new();
            // Bound execution: abort after a budget of interrupt safepoints so a
            // generated infinite loop can't hang the test.
            // `Arc<AtomicU64>` (not `Rc<Cell>`) so the interrupt closure is `Send`
            // — `set_interrupt` requires `MaybeSend`, which is `Send` under the
            // `send` feature (the feature-combo CI build compiles this test).
            let steps = Arc::new(AtomicU64::new(0));
            let counter = steps.clone();
            lua.set_interrupt(move |_| -> Result<VmState> {
                if counter.fetch_add(1, Ordering::Relaxed) + 1 > 500_000 {
                    Err(luaur_rt::Error::runtime("fuzz: step limit reached"))
                } else {
                    Ok(VmState::Continue)
                }
            });
            if let Ok(f) = lua.load(&src).set_name("fuzz").into_function() {
                let _ = f.call::<()>(()); // Ok or Err both fine; just must not panic
            }
        }));

        assert!(
            outcome.is_ok(),
            "RUN PANICKED on seed {seed}\n---- source ----\n{src}\n----------------"
        );
    }
}
