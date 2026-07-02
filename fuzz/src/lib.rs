//! Structured Luau generator for the `structured` fuzz target — the Rust analog
//! of Luau's `luau.proto` + `protoprint` (proto.cpp). Instead of a protobuf AST
//! mutated by libprotobuf-mutator, we walk a grammar and pull every choice from
//! libFuzzer's byte stream via [`arbitrary::Unstructured`], so the fuzzer's
//! coverage feedback *steers* the generated program toward new code paths while
//! the output stays syntactically valid Luau.

use arbitrary::Unstructured;

/// Metadata-driven stdlib generator for the `api` target — a table of every
/// covered builtin (call path + per-argument KIND) crossed with boundary-value
/// pools, so the fuzzer systematically reaches the whole library surface with
/// hostile arguments. See [`api_gen`].
pub mod api_gen;
pub use api_gen::generate_api_call;

/// VM interrupt step budget for the run/splice/structured targets, read ONCE from
/// `LUAUR_FUZZ_STEPS` (default 100_000). Measured: ~93% of those targets' wall
/// time is the VM loop, and it's dominated by the ~2% of inputs that are infinite
/// loops (`while true do end`) running to the limit — they add no new coverage
/// after the first iteration, so capping them lower is a near-pure throughput win
/// (1M→100k ≈ 6×, →20k ≈ 11×, with ~zero coverage loss). Lower it for breadth
/// campaigns (`LUAUR_FUZZ_STEPS=20000`), raise it to exercise deep finite loops
/// (`LUAUR_FUZZ_STEPS=1000000`). Cached so there's no getenv per input.
pub fn vm_step_limit() -> u64 {
    use std::sync::OnceLock;
    static LIMIT: OnceLock<u64> = OnceLock::new();
    *LIMIT.get_or_init(|| {
        std::env::var("LUAUR_FUZZ_STEPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&n| n > 0)
            .unwrap_or(100_000)
    })
}

/// Native stack size (bytes) for [`run_on_big_stack`]. Overridable via
/// `LUAUR_FUZZ_STACK_MB` (default 256 MiB). See that function for why.
fn big_stack_bytes() -> usize {
    use std::sync::OnceLock;
    static MB: OnceLock<usize> = OnceLock::new();
    *MB.get_or_init(|| {
        std::env::var("LUAUR_FUZZ_STACK_MB")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(256)
            * 1024
            * 1024
    })
}

/// Run `f` on a thread with a large native stack and wait for it.
///
/// luaur executes Lua-to-Lua calls via **native recursion**, so a legal but deep
/// Lua recursion (e.g. the ~20 000-deep recursion in Luau's own `pcall.luau`
/// conformance test) exhausts the default ~8 MiB thread stack and *aborts the
/// process*. That abort is an uncatchable FALSE POSITIVE — it is not a
/// memory-safety bug and not an upstream-Luau bug: the same program runs fine
/// with adequate stack, which is exactly what the C++ conformance harness gives
/// it. Any target that runs arbitrary or spliced programs wraps its VM work in
/// this so only genuine crashes surface. A truly UNBOUNDED native recursion
/// still overflows even the large stack (just deeper), so real infinite-recursion
/// bugs remain caught — this raises the false-positive floor without lowering the
/// true-positive ceiling.
///
/// `f` is `FnOnce + Send`: it must CREATE its `Lua`/`Rc` state inside the closure
/// (those types are `!Send`) so nothing non-`Send` crosses the thread boundary —
/// only the owned input bytes are captured.
pub fn run_on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    // A panic inside `f` is already reported+aborted by the installed AFL panic
    // hook (process-global); join just reaps a normal return. If the OS can't
    // give us the thread (effectively never), the input is dropped — `spawn`
    // consumes `f`, so there's no inline fallback, and losing a rare input beats
    // aborting.
    if let Ok(handle) = std::thread::Builder::new()
        .stack_size(big_stack_bytes())
        .spawn(f)
    {
        let _ = handle.join();
    }
}

/// Panic hook for the AFL compile-running targets. The compiler emulates C++
/// `throw CompileError` with `panic_any(CompileError)`, which `compile()`'s
/// `catch_unwind` catches and returns as `Err` — a normal "this program doesn't
/// compile" outcome (verified identical to C++ Luau), NOT a crash. But AFL's
/// default `fuzz!` hook calls `process::abort()` on ANY panic, firing at the panic
/// point BEFORE that catch runs — so a legitimate compile error becomes a false
/// crash AND a coverage wall (AFL stops exploring past it). This hook lets ONLY
/// that exact payload unwind normally; everything else still aborts so AFL records
/// genuine bugs. Use with `afl::fuzz_nohook!` (not `fuzz!`, which would re-install
/// the abort-all hook). Idempotent enough to call once per process at startup.
pub fn install_afl_panic_hook() {
    use std::panic;
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if info
            .payload()
            .is::<luaur_compiler::records::compile_error::CompileError>()
        {
            return; // intentional, caught compiler throw — not a crash
        }
        prev(info); // real/unexpected panic: print like normal ...
        std::process::abort(); // ... and abort so AFL flags it as a crash
    }));
}

const NAMES: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h", "x", "y", "z"];
const SAFE_GLOBALS: &[&str] = &["type", "tostring", "tonumber", "select", "rawequal"];
/// Type-alias names the typed generator declares up front and then references.
const TYPE_NAMES: &[&str] = &["T", "U", "V", "Pt", "BoxT", "Pair"];

pub struct Gen<'a> {
    u: Unstructured<'a>,
    out: String,
    budget: i32,
    loop_depth: u32,
}

impl<'a> Gen<'a> {
    pub fn new(data: &'a [u8]) -> Gen<'a> {
        Gen {
            u: Unstructured::new(data),
            out: String::with_capacity(256),
            budget: 200,
            loop_depth: 0,
        }
    }

    /// Generate one program. Bounded by the size budget and by the available
    /// fuzzer bytes (when they run out, `below` returns 0 → only leaves emit).
    pub fn program(mut self) -> String {
        self.block(6);
        self.out
    }

    fn below(&mut self, n: u32) -> u32 {
        if n <= 1 {
            return 0;
        }
        self.u.int_in_range(0..=n - 1).unwrap_or(0)
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn name(&mut self) {
        let i = self.below(NAMES.len() as u32) as usize;
        self.out.push_str(NAMES[i]);
    }

    fn dry(&self) -> bool {
        self.budget <= 0
    }

    fn expr(&mut self) {
        self.budget -= 1;
        if self.dry() {
            return self.atom();
        }
        match self.below(10) {
            0 | 1 => self.atom(),
            2 => self.name(),
            3 => {
                self.expr();
                let ops = [
                    " + ", " - ", " * ", " / ", " % ", " ^ ", " .. ", " == ", " ~= ", " < ",
                    " <= ", " > ", " >= ", " and ", " or ",
                ];
                let op = ops[self.below(ops.len() as u32) as usize];
                self.push(op);
                self.expr();
            }
            4 => {
                let ops = ["-", "not ", "#"];
                let op = ops[self.below(ops.len() as u32) as usize];
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
                self.name();
                if self.below(2) == 0 {
                    self.push("[");
                    self.expr();
                    self.push("]");
                } else {
                    self.push(".");
                    self.name();
                }
            }
            8 => {
                match self.below(3) {
                    0 => {
                        let g = SAFE_GLOBALS[self.below(SAFE_GLOBALS.len() as u32) as usize];
                        self.push(g);
                    }
                    1 => self.name(),
                    _ => {
                        self.name();
                        self.push(":");
                        self.name();
                    }
                }
                self.args();
            }
            _ => self.func_expr(),
        }
    }

    fn atom(&mut self) {
        match self.below(6) {
            0 => {
                let n = self.below(1000);
                self.out.push_str(&n.to_string());
            }
            1 => {
                let n = self.below(1000);
                let f = self.below(100);
                self.out.push_str(&n.to_string());
                self.push(".");
                self.out.push_str(&f.to_string());
            }
            2 => self.push("true"),
            3 => self.push("false"),
            4 => self.push("nil"),
            _ => {
                self.push("\"");
                let len = self.below(6);
                for _ in 0..len {
                    let cs = b"abcdeABCDE01234";
                    let c = cs[self.below(cs.len() as u32) as usize];
                    self.out.push(c as char);
                }
                self.push("\"");
            }
        }
    }

    fn table(&mut self) {
        self.push("{");
        let n = self.below(4);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            match self.below(3) {
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

    fn args(&mut self) {
        self.push("(");
        let n = self.below(3);
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
        let n = self.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.name();
        }
        self.push(") ");
        let saved = self.loop_depth;
        self.loop_depth = 0;
        self.block(2);
        self.loop_depth = saved;
        self.push(" end");
    }

    fn block(&mut self, max_stmts: u32) {
        let n = self.below(max_stmts + 1);
        for _ in 0..n {
            if self.dry() {
                break;
            }
            self.stmt();
            self.push("\n");
        }
        if self.below(4) == 0 {
            self.push("return");
            if self.below(2) == 0 {
                self.push(" ");
                self.expr();
            }
            self.push("\n");
        }
    }

    fn stmt(&mut self) {
        self.budget -= 1;
        if self.dry() {
            self.push("local ");
            self.name();
            self.push(" = ");
            return self.atom();
        }
        match self.below(12) {
            0 => {
                self.push("local ");
                self.name();
                self.push(" = ");
                self.expr();
            }
            1 => {
                self.name();
                let ops = [" = ", " += ", " -= ", " *= ", " /= ", " %= ", " ..= "];
                let op = ops[self.below(ops.len() as u32) as usize];
                self.push(op);
                self.expr();
            }
            2 => {
                self.push("if ");
                self.expr();
                self.push(" then\n");
                self.block(2);
                if self.below(2) == 0 {
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
                if self.below(2) == 0 {
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
                match self.below(2) {
                    0 => {
                        let g = SAFE_GLOBALS[self.below(SAFE_GLOBALS.len() as u32) as usize];
                        self.push(g);
                    }
                    _ => self.name(),
                }
                self.args();
            }
            10 if self.loop_depth > 0 => self.push("break"),
            11 if self.loop_depth > 0 => self.push("continue"),
            _ => {
                self.name();
                self.push(" = ");
                self.expr();
            }
        }
    }

    fn func_tail(&mut self) {
        self.push("(");
        let n = self.below(3);
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

    // ---- typed Luau ------------------------------------------------------
    // The untyped grammar above drives the parser + VM but barely touches the
    // type checker (luaur's unique surface). These methods emit the type system:
    // annotations, aliases, generics, unions/intersections/optionals, function
    // and table types, type assertions — plus deliberately PARTIAL annotation so
    // the inference<->annotation interaction is exercised. Programs are mostly
    // well-formed (to reach deep checker logic) but a fraction are ill-typed or
    // cyclic (to fuzz the diagnostic / cycle-detection paths). The oracle is
    // unchanged: never panic/abort/hang, only Ok or a structured error.

    fn type_name(&mut self) {
        let i = self.below(TYPE_NAMES.len() as u32) as usize;
        self.push(TYPE_NAMES[i]);
    }

    /// Emit a type expression.
    fn ty(&mut self) {
        self.budget -= 1;
        if self.dry() {
            self.push("any");
            return;
        }
        match self.below(13) {
            0 => self.push("number"),
            1 => self.push("string"),
            2 => self.push("boolean"),
            3 => self.push("nil"),
            4 => self.push("any"),
            5 => self.push("unknown"),
            6 => self.push("never"),
            7 => self.type_name(),
            8 => {
                self.ty();
                self.push("?");
            }
            9 => {
                self.ty();
                self.push(" | ");
                self.ty();
            }
            10 => {
                self.ty();
                self.push(" & ");
                self.ty();
            }
            11 => self.table_ty(),
            _ => self.fn_ty(),
        }
    }

    fn table_ty(&mut self) {
        if self.below(2) == 0 {
            self.push("{");
            self.ty();
            self.push("}"); // array type {T}
        } else {
            self.push("{ ");
            let n = 1 + self.below(3);
            for i in 0..n {
                if i > 0 {
                    self.push(", ");
                }
                self.name();
                self.push(": ");
                self.ty();
            }
            self.push(" }");
        }
    }

    fn fn_ty(&mut self) {
        self.push("(");
        let n = self.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.ty();
        }
        self.push(") -> ");
        self.ty();
    }

    fn typed_func_tail(&mut self) {
        // sometimes generic
        if self.below(3) == 0 {
            self.push("<T>");
        }
        self.push("(");
        let n = self.below(4);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.name();
            // partial annotation: ~2/3 of params are typed
            if self.below(3) != 0 {
                self.push(": ");
                self.ty();
            }
        }
        self.push(")");
        if self.below(2) == 0 {
            self.push(": ");
            self.ty(); // declared return type
        }
        self.push("\n");
        let saved = self.loop_depth;
        self.loop_depth = 0;
        self.block(3); // reuse the value-level body grammar
        self.loop_depth = saved;
        self.push("end");
    }

    fn typed_stmt(&mut self) {
        self.budget -= 1;
        if self.dry() {
            self.push("local x: any = nil");
            return;
        }
        match self.below(9) {
            0 => {
                self.push("type ");
                self.type_name();
                if self.below(3) == 0 {
                    self.push("<T>");
                }
                self.push(" = ");
                self.ty();
            }
            1 => {
                self.push("local ");
                self.name();
                self.push(": ");
                self.ty();
                self.push(" = ");
                self.expr();
            }
            2 => {
                // partially typed: no annotation, rely on inference
                self.push("local ");
                self.name();
                self.push(" = ");
                self.expr();
            }
            3 => {
                self.push("local function ");
                self.name();
                self.typed_func_tail();
            }
            4 => {
                self.push("function ");
                self.name();
                self.typed_func_tail();
            }
            5 => {
                // type assertion
                self.push("local ");
                self.name();
                self.push(" = (");
                self.expr();
                self.push(") :: ");
                self.ty();
            }
            6 => {
                // typed local without initializer (forces inference / error)
                self.push("local ");
                self.name();
                self.push(": ");
                self.ty();
            }
            7 => {
                self.name();
                self.args(); // a call
            }
            _ => {
                self.push("return ");
                self.expr();
            }
        }
    }

    fn typed_block(&mut self, max_stmts: u32) {
        let n = self.below(max_stmts + 1);
        for _ in 0..n {
            if self.dry() {
                break;
            }
            // interleave typed and untyped statements
            if self.below(2) == 0 {
                self.typed_stmt();
            } else {
                self.stmt();
            }
            self.push("\n");
        }
    }

    /// Generate one type-rich program: a few alias declarations up front (so
    /// references resolve), then a mix of typed and untyped statements.
    pub fn typed_program(mut self) -> String {
        let aliases = 1 + self.below(TYPE_NAMES.len() as u32);
        for i in 0..aliases {
            self.push("type ");
            // declare aliases in order so most references resolve
            self.push(TYPE_NAMES[i as usize % TYPE_NAMES.len()]);
            self.push(" = ");
            self.ty();
            self.push("\n");
        }
        self.typed_block(8);
        self.out
    }
}

/// Generate a syntactically-valid Luau program from raw fuzzer bytes.
pub fn generate(data: &[u8]) -> String {
    Gen::new(data).program()
}

/// Generate a **type-rich** Luau program (annotations, aliases, generics,
/// unions/intersections/optionals, function & table types, type assertions, and
/// partially-typed code) from raw fuzzer bytes — drives the type checker far
/// deeper than the untyped [`generate`].
pub fn generate_typed(data: &[u8]) -> String {
    Gen::new(data).typed_program()
}

impl<'a> Gen<'a> {
    /// Generate a Luau **definition file** (`declare ...` blocks) — the host-type
    /// surface fed to `check_with_definitions`.
    pub fn definitions(mut self) -> String {
        let n = self.below(8) + 1;
        for _ in 0..n {
            self.decl();
            self.push("\n");
        }
        self.out
    }

    fn decl(&mut self) {
        match self.below(3) {
            0 => {
                self.push("declare function ");
                self.name();
                self.push("(");
                let p = self.below(3);
                for i in 0..p {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.name();
                    self.push(": ");
                    self.ty();
                }
                self.push("): ");
                self.ty();
            }
            1 => {
                self.push("declare ");
                self.name();
                self.push(": ");
                self.ty();
            }
            _ => {
                self.push("declare class ");
                self.name();
                self.push("\n");
                let m = self.below(4);
                for _ in 0..m {
                    self.name();
                    self.push(": ");
                    self.ty();
                    self.push("\n");
                }
                self.push("end");
            }
        }
    }
}

/// Generate a Luau definition file (`declare ...`) from raw fuzzer bytes.
pub fn generate_definitions(data: &[u8]) -> String {
    Gen::new(data).definitions()
}

// ---------------------------------------------------------------------------
// AST splicing — byte-driven MUTATION of real seed scripts.
//
// The other generators build programs from a grammar; this one starts from a
// real, hand-written Luau program (the vendored conformance suite, embedded by
// build.rs into `SPLICE_CORPUS`) and applies a fuzzer-byte-driven sequence of
// edits to its TOP-LEVEL STATEMENTS. The leading bytes pick the seed; the tail
// bytes are the "mutation program" — so AFL mutating the tail explores nearby
// mutations of the SAME seed (mutation locality), reaching language-feature
// combinations a from-scratch grammar never invents while staying close to valid.
// ---------------------------------------------------------------------------

include!(concat!(env!("OUT_DIR"), "/splice_corpus.rs"));

/// Split a Luau source into the source-text of its TOP-LEVEL statements, using
/// the parser + each statement's location span. Returns the whole source as a
/// single element if it doesn't parse cleanly (so the caller always has at least
/// one usable chunk).
fn split_top_level_statements(src: &str) -> Vec<String> {
    use luaur_ast::records::allocator::Allocator;
    use luaur_ast::records::ast_name_table::AstNameTable;
    use luaur_ast::records::parse_options::ParseOptions;
    use luaur_ast::records::parser::Parser;

    let mut allocator = Allocator::allocator();
    let mut names = AstNameTable::new(&mut allocator);
    let parse_result = Parser::parse(
        src,
        src.len(),
        &mut names,
        &mut allocator,
        ParseOptions::default(),
    );
    if !parse_result.errors.is_empty() || parse_result.root.is_null() {
        return alloc_one(src);
    }

    // Byte offset of the start of each (0-based) line.
    let mut line_starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let off = |line: u32, col: u32| -> usize {
        let l = line as usize;
        if l >= line_starts.len() {
            return src.len();
        }
        (line_starts[l] + col as usize).min(src.len())
    };

    let body = unsafe { (*parse_result.root).body.as_slice() };
    let mut out = Vec::new();
    for &stat in body {
        if stat.is_null() {
            continue;
        }
        let loc = unsafe { (*stat).base.location };
        let s = off(loc.begin.line, loc.begin.column);
        let e = off(loc.end.line, loc.end.column);
        // `get` returns None on non-char-boundary / out-of-range — skip those.
        if let Some(slice) = src.get(s..e) {
            let t = slice.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    if out.is_empty() {
        alloc_one(src)
    } else {
        out
    }
}

fn alloc_one(src: &str) -> Vec<String> {
    let t = src.trim();
    if t.is_empty() {
        Vec::new()
    } else {
        vec![t.to_string()]
    }
}

// ---------------------------------------------------------------------------
// Observable execution — the shared substrate for the differential / metamorphic
// oracles. Runs `src` and returns a DETERMINISTIC observation of its behavior:
// captured `print` output (with table/function values reduced to a type tag so
// allocation addresses don't leak in) plus the final ok/err status.
//
// Returns `None` ("inconclusive") when the program doesn't compile, or when the
// run hits the step limit — because optimization changes VM step counts, a
// step-limited program can legitimately diverge between opt levels, which would
// be a false positive. Oracles only compare when every run returns `Some`.
// ---------------------------------------------------------------------------

fn value_repr(v: &luaur_rt::Value) -> String {
    use luaur_rt::Value;
    match v {
        Value::Nil => "nil".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => format!("{n}"),
        Value::String(s) => format!("{:?}", s.to_string_lossy()),
        Value::Table(_) => "<table>".to_string(),
        Value::Function(_) => "<function>".to_string(),
        Value::Vector(_) => "<vector>".to_string(),
        Value::Thread(_) => "<thread>".to_string(),
        Value::Buffer(_) => "<buffer>".to_string(),
        _ => "<other>".to_string(),
    }
}

/// Compiler configuration for [`run_observed_cfg`]. The optimization level is the
/// classic differential dimension; `debug`, `coverage`, and `type_info` levels
/// are all supposed to be **behavior-preserving** (they add metadata /
/// instrumentation, not semantics), so varying them must not change the observed
/// output — that's what the flag-matrix oracle checks.
#[derive(Clone, Copy)]
pub struct ObserveCfg {
    pub opt_level: u8,
    pub debug_level: u8,
    pub coverage_level: u8,
    pub type_info_level: u8,
}

impl ObserveCfg {
    /// The baseline used by the plain opt-level differential: opt `n`, debug 1,
    /// no coverage, type-info 1 (a middle-of-the-road, always-valid combination).
    pub fn opt(opt_level: u8) -> ObserveCfg {
        ObserveCfg {
            opt_level,
            debug_level: 1,
            coverage_level: 0,
            type_info_level: 1,
        }
    }
}

/// Run `src` at the given compiler optimization level and return a deterministic
/// observation, or `None` if inconclusive (didn't compile / hit the step limit).
pub fn run_observed(src: &str, opt_level: u8) -> Option<String> {
    run_observed_cfg(src, ObserveCfg::opt(opt_level))
}

/// Like [`run_observed`] but with full control over the compiler flag matrix
/// (opt / debug / coverage / type-info levels) — the substrate for the flag
/// invariance oracle in the `optdiff` target.
pub fn run_observed_cfg(src: &str, cfg: ObserveCfg) -> Option<String> {
    use luaur_rt::{Compiler, Error, Lua, Result, Value, Variadic, VmState};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    let lua = Lua::new();

    // Capture `print` deterministically instead of writing to stdout.
    let buf: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    {
        let b = buf.clone();
        if let Ok(print_fn) = lua.create_function(move |_, args: Variadic<Value>| -> Result<()> {
            let mut out = b.borrow_mut();
            for (i, v) in args.iter().enumerate() {
                if i > 0 {
                    out.push('\t');
                }
                out.push_str(&value_repr(v));
            }
            out.push('\n');
            Ok(())
        }) {
            let _ = lua.globals().set("print", print_fn);
        }
    }

    // Step limit (generous — most generated programs finish in a few thousand).
    let hit_limit = Rc::new(Cell::new(false));
    {
        let steps = Cell::new(0u64);
        let hl = hit_limit.clone();
        lua.set_interrupt(move |_| -> Result<VmState> {
            let n = steps.get() + 1;
            steps.set(n);
            if n > 5_000_000 {
                hl.set(true);
                Err(Error::runtime("fuzz: step limit"))
            } else {
                Ok(VmState::Continue)
            }
        });
    }

    let compiler = Compiler::new()
        .set_optimization_level(cfg.opt_level)
        .set_debug_level(cfg.debug_level)
        .set_coverage_level(cfg.coverage_level)
        .set_type_info_level(cfg.type_info_level);
    let func = lua
        .load(src)
        .set_name("fuzz")
        .set_compiler(compiler)
        .into_function()
        .ok()?; // didn't compile — nothing to compare

    match func.call::<Value>(()) {
        Ok(v) => Some(format!("ok:{}|{}", value_repr(&v), buf.borrow())),
        Err(_) if hit_limit.get() => None, // step limit — inconclusive
        Err(_) => Some(format!("err|{}", buf.borrow())),
    }
}

// ---------------------------------------------------------------------------
// Computational generator — DETERMINISTIC, OBSERVABLE programs. Pure arithmetic /
// boolean / string expressions over in-scope variables, with `print`s, and NO
// nondeterministic builtins (no os/io/random/time/pointer identity). This is the
// substrate the differential (opt-level) and metamorphic oracles need: the
// grammar generators don't print or return, so they have nothing to compare, and
// real scripts may be nondeterministic. (Foundation for known-value oracles too.)
// ---------------------------------------------------------------------------

fn comp_atom(u: &mut Unstructured, nvars: usize) -> String {
    match u.int_in_range(0u8..=4).unwrap_or(0) {
        0 if nvars > 0 => format!("v{}", u.int_in_range(0..=nvars - 1).unwrap_or(0)),
        1 => format!("{}", u.int_in_range(0i64..=1000).unwrap_or(0)),
        2 => {
            let n = u.int_in_range(0i64..=100000).unwrap_or(0);
            format!("{}.{}", n / 1000, n % 1000)
        }
        3 => if u.arbitrary().unwrap_or(false) {
            "true"
        } else {
            "false"
        }
        .to_string(),
        _ => format!("\"s{}\"", u.int_in_range(0u8..=9).unwrap_or(0)),
    }
}

fn comp_expr(u: &mut Unstructured, nvars: usize, depth: u32) -> String {
    if depth == 0 || u.is_empty() {
        return comp_atom(u, nvars);
    }
    match u.int_in_range(0u8..=7).unwrap_or(0) {
        0 | 1 => comp_atom(u, nvars),
        2 => {
            // numeric arithmetic (Luau numbers are doubles: x%0 / x//0 give
            // inf/nan deterministically, so no zero guard is needed).
            const OPS: &[&str] = &["+", "-", "*", "/", "//", "%", "^"];
            let op = OPS[u.int_in_range(0..=OPS.len() - 1).unwrap_or(0)];
            format!(
                "({} {} {})",
                comp_expr(u, nvars, depth - 1),
                op,
                comp_expr(u, nvars, depth - 1)
            )
        }
        3 => {
            const OPS: &[&str] = &["<", "<=", ">", ">=", "==", "~="];
            let op = OPS[u.int_in_range(0..=OPS.len() - 1).unwrap_or(0)];
            format!(
                "({} {} {})",
                comp_expr(u, nvars, depth - 1),
                op,
                comp_expr(u, nvars, depth - 1)
            )
        }
        4 => {
            let op = if u.arbitrary().unwrap_or(false) {
                "and"
            } else {
                "or"
            };
            format!(
                "({} {} {})",
                comp_expr(u, nvars, depth - 1),
                op,
                comp_expr(u, nvars, depth - 1)
            )
        }
        5 => format!(
            "(tostring({}) .. tostring({}))",
            comp_expr(u, nvars, depth - 1),
            comp_expr(u, nvars, depth - 1)
        ),
        6 => {
            const FNS: &[&str] = &["math.floor", "math.abs", "math.ceil", "-", "#tostring"];
            let f = FNS[u.int_in_range(0..=FNS.len() - 1).unwrap_or(0)];
            let inner = comp_expr(u, nvars, depth - 1);
            match f {
                "-" => format!("(-{inner})"),
                "#tostring" => format!("(#tostring({inner}))"),
                _ => format!("{f}({inner})"),
            }
        }
        _ => format!("(not {})", comp_expr(u, nvars, depth - 1)),
    }
}

/// Generate a DETERMINISTIC program of pure arithmetic/logic with `print`s — see
/// the module comment. Output is fully reproducible run-to-run, so two runs that
/// disagree (different opt level, or a behavior-preserving transform) reveal a
/// real bug.
pub fn generate_computational(data: &[u8]) -> String {
    let mut u = Unstructured::new(data);
    let mut out = String::new();
    let mut nvars = 0usize;
    let n_stmts = u.int_in_range(1..=30u32).unwrap_or(5);
    for _ in 0..n_stmts {
        if u.is_empty() {
            break;
        }
        // Statement kind. The extra kinds beyond "print"/"local = expr" give the
        // OPTIMIZER something to actually transform: a bounded numeric `for` loop
        // (loop-unroll / const-fold / LOOP fastpaths) and a constant-key table
        // read/write (GETTABLEKS/SETTABLEKS + array-part paths). Both stay fully
        // DETERMINISTIC and finite (fixed small bounds, integer-key tables, no
        // time/random) so the differential (optdiff) and metamorphic oracles keep
        // a valid same-in-same-out invariant. Iteration bounds are tiny so a loop
        // never approaches the run_observed step limit (which would go
        // inconclusive and waste the input).
        match u.int_in_range(0u8..=4).unwrap_or(0) {
            0 if nvars > 0 => {
                let i = u.int_in_range(0..=nvars - 1).unwrap_or(0);
                out.push_str(&format!("print(v{i})\n"));
            }
            // A bounded loop that folds an expression into a fresh accumulator.
            2 if nvars > 0 => {
                let n = u.int_in_range(1..=8i64).unwrap_or(3);
                let seed = comp_expr(&mut u, nvars, 2);
                let step = comp_expr(&mut u, nvars, 2);
                out.push_str(&format!(
                    "local v{nvars} = {seed}\nfor _i = 1, {n} do v{nvars} = (v{nvars} + {step}) end\n"
                ));
                nvars += 1;
            }
            // A small constant-key table: build it, then read one key back into a
            // fresh var so the value is observable.
            3 => {
                let a = comp_expr(&mut u, nvars, 2);
                let b = comp_expr(&mut u, nvars, 2);
                let c = comp_expr(&mut u, nvars, 2);
                let k = u.int_in_range(1..=3i64).unwrap_or(1);
                out.push_str(&format!(
                    "local _t = {{ {a}, {b}, {c} }}\nlocal v{nvars} = _t[{k}]\n"
                ));
                nvars += 1;
            }
            _ => {
                let e = comp_expr(&mut u, nvars, 4);
                out.push_str(&format!("local v{nvars} = {e}\n"));
                nvars += 1;
            }
        }
    }
    // Print every variable at the end so the whole computed state is observable.
    for i in 0..nvars {
        out.push_str(&format!("print(v{i})\n"));
    }
    out
}

/// Apply a behavior-preserving transform to `src`: sprinkle no-op statements
/// (`do end`, `;`, fresh unused locals) BEFORE top-level statements (never after,
/// so a trailing `return` stays last). The observable behavior must be identical
/// — that's the metamorphic invariant. Insertions are byte-driven by `tail`.
pub fn metamorphic_noop(src: &str, tail: &[u8]) -> String {
    let stmts = split_top_level_statements(src);
    let mut u = Unstructured::new(tail);
    const NOOPS: &[&str] = &["do end", ";", "local _meta = nil", "do local _ = 1 end"];
    let mut out = String::new();
    for s in &stmts {
        if u.ratio(1u8, 2u8).unwrap_or(false) {
            let n = NOOPS[u.int_in_range(0..=NOOPS.len() - 1).unwrap_or(0)];
            out.push_str(n);
            out.push('\n');
        }
        out.push_str(s);
        out.push('\n');
    }
    out
}

/// Apply a DEAD-CODE metamorphic transform to `src`: before each top-level
/// statement, sometimes inject a statement wrapped in a never-taken branch
/// (`if false then <stmt> end`, `while false do <stmt> end`). The injected body
/// is drawn from a REAL seed script (via [`generate_spliced`]-style slicing), so
/// it's arbitrary-but-parseable code the optimizer must prove unreachable and
/// eliminate. Because the branch condition is a literal `false`, the transform
/// provably preserves observable behavior — so if the transformed program's
/// captured output diverges, that's a dead-code-elimination / const-fold /
/// jump-threading bug (the classic Equivalence-Modulo-Inputs signal). This
/// complements [`metamorphic_noop`], which only inserts semantic no-ops: here
/// the eliminated code is real, so it stresses the optimizer's reachability
/// analysis rather than just statement interleaving.
pub fn metamorphic_dead_branch(src: &str, tail: &[u8]) -> String {
    let stmts = split_top_level_statements(src);
    let mut u = Unstructured::new(tail);

    // A pool of real, parseable statements to bury in dead branches. Falls back
    // to a couple of literals if the embedded corpus is empty.
    let pool: Vec<String> = if SPLICE_CORPUS.is_empty() {
        vec!["error('dead')".to_string(), "return 42".to_string()]
    } else {
        let i = u.int_in_range(0..=SPLICE_CORPUS.len() - 1).unwrap_or(0);
        let sliced = split_top_level_statements(SPLICE_CORPUS[i]);
        if sliced.is_empty() {
            vec!["error('dead')".to_string()]
        } else {
            sliced
        }
    };

    let mut out = String::new();
    for s in &stmts {
        if u.ratio(1u8, 2u8).unwrap_or(false) {
            let dead = &pool[u.int_in_range(0..=pool.len() - 1).unwrap_or(0)];
            // Only a literal-`false` guard is guaranteed dead regardless of the
            // buried statement's effects — keep it that way (never a variable).
            match u.int_in_range(0u8..=1).unwrap_or(0) {
                0 => out.push_str(&format!("if false then\n{dead}\nend\n")),
                _ => out.push_str(&format!("while false do\n{dead}\nend\n")),
            }
        }
        out.push_str(s);
        out.push('\n');
    }
    out
}

/// Pick a real seed program and apply a byte-driven sequence of statement-level
/// mutations to it. See the module comment above.
pub fn generate_spliced(data: &[u8]) -> String {
    if SPLICE_CORPUS.is_empty() {
        return generate(data); // no embedded corpus — fall back to the grammar
    }
    let mut u = Unstructured::new(data);

    // The seed scripts are a fixed `&'static` set, so slicing one into top-level
    // statements is a pure function of the seed index — memoize it. Re-parsing a
    // (often hundreds-of-lines) conformance script on EVERY input was the splice
    // target's main per-exec cost; the cache turns it into a one-time parse per
    // distinct seed, then a cheap clone. Byte consumption (and thus output) is
    // identical to the old `split_top_level_statements(pick_seed(u))`.
    thread_local! {
        static SLICE_CACHE: core::cell::RefCell<
            std::collections::HashMap<usize, std::rc::Rc<Vec<String>>>,
        > = core::cell::RefCell::new(std::collections::HashMap::new());
    }
    let pick_sliced = |u: &mut Unstructured| -> std::rc::Rc<Vec<String>> {
        let i = u.int_in_range(0..=SPLICE_CORPUS.len() - 1).unwrap_or(0);
        SLICE_CACHE.with(|c| {
            c.borrow_mut()
                .entry(i)
                .or_insert_with(|| std::rc::Rc::new(split_top_level_statements(SPLICE_CORPUS[i])))
                .clone()
        })
    };

    let mut stmts: Vec<String> = (*pick_sliced(&mut u)).clone();
    if stmts.is_empty() {
        return generate(data);
    }

    // Pick an index into a list of length `len` (>= 1).
    fn idx(u: &mut Unstructured, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            u.int_in_range(0..=len - 1).unwrap_or(0)
        }
    }

    let n_ops = u.int_in_range(0..=24u32).unwrap_or(0);
    for _ in 0..n_ops {
        if u.is_empty() || stmts.is_empty() {
            break;
        }
        match u.int_in_range(0u8..=5).unwrap_or(0) {
            0 => {
                // duplicate a statement
                let i = idx(&mut u, stmts.len());
                let s = stmts[i].clone();
                let at = (i + 1).min(stmts.len());
                stmts.insert(at, s);
            }
            1 => {
                // delete a statement
                let i = idx(&mut u, stmts.len());
                stmts.remove(i);
            }
            2 => {
                // swap two statements
                let i = idx(&mut u, stmts.len());
                let j = idx(&mut u, stmts.len());
                stmts.swap(i, j);
            }
            3 => {
                // move a statement to a new position
                let i = idx(&mut u, stmts.len());
                let s = stmts.remove(i);
                let at = idx(&mut u, stmts.len() + 1).min(stmts.len());
                stmts.insert(at, s);
            }
            4 => {
                // splice in a statement taken from ANOTHER real seed
                let other = pick_sliced(&mut u);
                if !other.is_empty() {
                    let k = idx(&mut u, other.len());
                    let at = idx(&mut u, stmts.len() + 1).min(stmts.len());
                    stmts.insert(at, other[k].clone());
                }
            }
            _ => {
                // wrap a statement in a control structure
                let i = idx(&mut u, stmts.len());
                stmts[i] = match u.int_in_range(0u8..=2).unwrap_or(0) {
                    0 => format!("do\n{}\nend", stmts[i]),
                    1 => format!("if true then\n{}\nend", stmts[i]),
                    _ => format!("for _spl=1,1 do\n{}\nend", stmts[i]),
                };
            }
        }
    }

    if stmts.len() > 80 {
        stmts.truncate(80);
    }
    stmts.join("\n")
}
