//! Metadata-driven stdlib fuzzer.
//!
//! The from-scratch grammar generators emit only small numbers (0-999) and never
//! name the library globals (`SAFE_GLOBALS` is `type`/`tostring`/...), so they
//! structurally cannot reach the integer/size-arithmetic bugs that live in the
//! builtins (bit32 / os.time / string.rep / string.pack / table.create /
//! buffer.* / utf8.*) — that whole class fell to the static audit, not the fuzzer.
//!
//! This generator closes the gap: a metadata table of builtins (call path + the
//! KIND of each argument) crossed with per-kind BOUNDARY-VALUE pools. Each input
//! picks a function and hostile arguments, so every covered function gets called
//! with extreme values (INT_MIN/MAX, 2^53, NaN, inf, huge strings, huge counts,
//! out-of-range buffer offsets, …). A Lua error from bad args is caught by the VM
//! call boundary; only a panic/abort (overflow, assert, OOB) surfaces as a crash.

use arbitrary::Unstructured;

#[derive(Clone, Copy)]
enum K {
    Num,
    Str,
    Tbl,
    Fn,
    Bool,
    Buf,
    Vec,
    Any,
    VarNum,
    VarAny,
    /// A bounded COUNT / SIZE / RANGE index — like `Num` but drawn from a small
    /// pool (magnitude <= ~2000), never `math.huge` / ±2^31 / 2^53. Used for the
    /// positions that drive an ALLOCATION or an internal COPY LOOP: `table.create`
    /// / `buffer.create` size, `string.rep` count, `table.move` f/e/t and
    /// `table.concat` i/j range, `table.insert` position. An edge value there
    /// doesn't test bounds ARITHMETIC (that's the offset/index positions, still
    /// `Num`) — it just makes the builtin allocate gigabytes or loop ~2^31 times,
    /// which OOM-aborts or hangs the fuzzer (a false positive; those overflow bugs
    /// are already fixed + regression-tested in `stdlib_overflow.rs`, and the
    /// `table.move` huge-range loop matches C++ Luau, so it's expected). Bounding
    /// keeps every such builtin reachable, and still hits their error/edge paths
    /// (0, negative, empty), without the allocation/loop blowup.
    Idx,
}

/// `(call path, argument kinds)`. Library functions are called as `lib.f(args)`
/// (equivalent to method form). Concentrated on the size/offset/field arithmetic
/// surface where the overflow class lives.
#[rustfmt::skip]
const API: &[(&str, &[K])] = &[
    // ---- bit32: field/shift/count integer arithmetic (overflow-prone) ----
    ("bit32.band", &[K::VarNum]), ("bit32.bor", &[K::VarNum]), ("bit32.bxor", &[K::VarNum]),
    ("bit32.btest", &[K::VarNum]), ("bit32.bnot", &[K::Num]),
    ("bit32.extract", &[K::Num, K::Num, K::Num]), ("bit32.replace", &[K::Num, K::Num, K::Num, K::Num]),
    ("bit32.lshift", &[K::Num, K::Num]), ("bit32.rshift", &[K::Num, K::Num]),
    ("bit32.arshift", &[K::Num, K::Num]), ("bit32.lrotate", &[K::Num, K::Num]),
    ("bit32.rrotate", &[K::Num, K::Num]), ("bit32.countlz", &[K::Num]),
    ("bit32.countrz", &[K::Num]), ("bit32.byteswap", &[K::Num]),
    // ---- os: date-field arithmetic ----
    ("os.time", &[K::Tbl]), ("os.date", &[K::Str, K::Num]),
    ("os.difftime", &[K::Num, K::Num]), ("os.clock", &[]),
    // ---- string: size/offset arithmetic ----
    ("string.byte", &[K::Str, K::Num, K::Num]), ("string.char", &[K::VarNum]),
    ("string.find", &[K::Str, K::Str, K::Num, K::Bool]), ("string.format", &[K::Str, K::VarAny]),
    ("string.gsub", &[K::Str, K::Str, K::Str, K::Num]), ("string.len", &[K::Str]),
    ("string.lower", &[K::Str]), ("string.match", &[K::Str, K::Str, K::Num]),
    ("string.rep", &[K::Str, K::Idx, K::Str]), ("string.reverse", &[K::Str]), // count: Idx (alloc)
    ("string.sub", &[K::Str, K::Num, K::Num]), ("string.upper", &[K::Str]),
    ("string.pack", &[K::Str, K::VarAny]), ("string.unpack", &[K::Str, K::Str, K::Num]),
    ("string.packsize", &[K::Str]), ("string.split", &[K::Str, K::Str]),
    // ---- math: integer/float arg conversion ----
    ("math.abs", &[K::Num]), ("math.ceil", &[K::Num]), ("math.floor", &[K::Num]),
    ("math.sqrt", &[K::Num]), ("math.max", &[K::VarNum]), ("math.min", &[K::VarNum]),
    ("math.random", &[K::Num, K::Num]), ("math.randomseed", &[K::Num]),
    ("math.fmod", &[K::Num, K::Num]), ("math.modf", &[K::Num]), ("math.frexp", &[K::Num]),
    ("math.ldexp", &[K::Num, K::Num]), ("math.log", &[K::Num, K::Num]),
    ("math.clamp", &[K::Num, K::Num, K::Num]), ("math.round", &[K::Num]),
    ("math.sign", &[K::Num]), ("math.pow", &[K::Num, K::Num]), ("math.noise", &[K::Num, K::Num, K::Num]),
    // ---- table: size/index arithmetic ----
    // Idx on the alloc/range positions (pos, i/j, count, f/e/t) — see K::Idx.
    ("table.insert", &[K::Tbl, K::Idx, K::Any]), ("table.remove", &[K::Tbl, K::Num]),
    ("table.concat", &[K::Tbl, K::Str, K::Idx, K::Idx]), ("table.create", &[K::Idx, K::Any]),
    ("table.move", &[K::Tbl, K::Idx, K::Idx, K::Idx, K::Tbl]), ("table.find", &[K::Tbl, K::Any, K::Num]),
    ("table.unpack", &[K::Tbl, K::Num, K::Num]), ("table.maxn", &[K::Tbl]),
    ("table.sort", &[K::Tbl, K::Fn]), // hostile/non-transitive comparator
    ("table.clear", &[K::Tbl]), ("table.freeze", &[K::Tbl]), ("table.clone", &[K::Tbl]),
    // ---- buffer: offset/size arithmetic (very crash-prone) ----
    ("buffer.create", &[K::Idx]), ("buffer.len", &[K::Buf]), ("buffer.tostring", &[K::Buf]), // size: Idx (alloc)
    ("buffer.readi8", &[K::Buf, K::Num]), ("buffer.readu8", &[K::Buf, K::Num]),
    ("buffer.readi16", &[K::Buf, K::Num]), ("buffer.readu16", &[K::Buf, K::Num]),
    ("buffer.readi32", &[K::Buf, K::Num]), ("buffer.readu32", &[K::Buf, K::Num]),
    ("buffer.readf32", &[K::Buf, K::Num]), ("buffer.readf64", &[K::Buf, K::Num]),
    ("buffer.writei8", &[K::Buf, K::Num, K::Num]), ("buffer.writeu8", &[K::Buf, K::Num, K::Num]),
    ("buffer.writei32", &[K::Buf, K::Num, K::Num]), ("buffer.writef64", &[K::Buf, K::Num, K::Num]),
    ("buffer.readstring", &[K::Buf, K::Num, K::Num]), ("buffer.writestring", &[K::Buf, K::Num, K::Str]),
    ("buffer.fill", &[K::Buf, K::Num, K::Num, K::Num]),
    ("buffer.copy", &[K::Buf, K::Num, K::Buf, K::Num, K::Num]),
    // ---- buffer: bit-level offset/count arithmetic (very crash-prone) ----
    ("buffer.readbits", &[K::Buf, K::Num, K::Num]),
    ("buffer.writebits", &[K::Buf, K::Num, K::Num, K::Num]),
    ("buffer.writeu16", &[K::Buf, K::Num, K::Num]), ("buffer.writei16", &[K::Buf, K::Num, K::Num]),
    ("buffer.writeu32", &[K::Buf, K::Num, K::Num]), ("buffer.writef32", &[K::Buf, K::Num, K::Num]),
    // ---- utf8: codepoint/offset arithmetic ----
    ("utf8.char", &[K::VarNum]), ("utf8.codepoint", &[K::Str, K::Num, K::Num]),
    ("utf8.len", &[K::Str, K::Num, K::Num]), ("utf8.offset", &[K::Str, K::Num, K::Num]),
    // ---- math: breadth (transcendental, conversion) ----
    ("math.sin", &[K::Num]), ("math.cos", &[K::Num]), ("math.tan", &[K::Num]),
    ("math.asin", &[K::Num]), ("math.acos", &[K::Num]), ("math.atan", &[K::Num]),
    ("math.atan2", &[K::Num, K::Num]), ("math.exp", &[K::Num]), ("math.log10", &[K::Num]),
    ("math.sinh", &[K::Num]), ("math.cosh", &[K::Num]), ("math.tanh", &[K::Num]),
    ("math.deg", &[K::Num]), ("math.rad", &[K::Num]), ("math.lerp", &[K::Num, K::Num, K::Num]),
    ("math.map", &[K::Num, K::Num, K::Num, K::Num, K::Num]),
    // ---- string: breadth ----
    ("string.gmatch", &[K::Str, K::Str]),
    // ---- base: value-sensitive + reflection ----
    ("tonumber", &[K::Str, K::Num]), ("tostring", &[K::Any]), ("select", &[K::Num, K::VarAny]),
    ("rawequal", &[K::Any, K::Any]), ("setmetatable", &[K::Tbl, K::Tbl]),
    ("rawget", &[K::Tbl, K::Any]), ("rawset", &[K::Tbl, K::Any, K::Any]), ("rawlen", &[K::Tbl]),
    ("getmetatable", &[K::Any]), ("next", &[K::Tbl, K::Any]), ("assert", &[K::Any, K::Str]),
    ("type", &[K::Any]), ("typeof", &[K::Any]),
    // ---- coroutine: hostile callbacks ----
    ("coroutine.create", &[K::Fn]), ("coroutine.wrap", &[K::Fn]),
    // ---- vector: float component math (NaN/inf/huge) ----
    ("vector.create", &[K::Num, K::Num, K::Num]), ("vector.magnitude", &[K::Vec]),
    ("vector.normalize", &[K::Vec]), ("vector.cross", &[K::Vec, K::Vec]),
    ("vector.dot", &[K::Vec, K::Vec]), ("vector.angle", &[K::Vec, K::Vec, K::Vec]),
    ("vector.floor", &[K::Vec]), ("vector.ceil", &[K::Vec]), ("vector.abs", &[K::Vec]),
    ("vector.clamp", &[K::Vec, K::Vec, K::Vec]), ("vector.lerp", &[K::Vec, K::Vec, K::Num]),
    // ---- debug: stack-level arithmetic ----
    ("debug.info", &[K::Num, K::Str]), ("debug.traceback", &[K::Str, K::Num]),
];

const NUM: &[&str] = &[
    "0",
    "-1",
    "1",
    "2",
    "8",
    "255",
    "65536",
    "2147483647",
    "-2147483648",
    "4294967295",
    "9007199254740993",
    "2^53",
    "2^63",
    "1/0",
    "-1/0",
    "0/0",
    "1e308",
    "-1e308",
    "0.5",
    "math.huge",
];
/// Bounded count/size/range pool for [`K::Idx`] — magnitude <= 2000, so an
/// allocation (`table.create`/`buffer.create`/`string.rep`) or a range copy
/// (`table.move`/`table.concat`) stays cheap and finite. Includes 0 / negatives
/// so error and empty-range paths are still exercised.
const IDX: &[&str] = &[
    "0", "1", "-1", "2", "8", "64", "255", "1000", "2000", "-64", "-1000",
];
const STR: &[&str] = &[
    "\"\"",
    "\"a\"",
    "\"abc\"",
    "\"%d%s%q%z%c%x %-5d %999d\"",
    "\"\\0\\1\\255\"",
    "(\"ab\"):rep(40000)",
    "\"<>=!hHiIlLjJTfdnsz\"",
    "\"hello world\"",
];
const TBL: &[&str] = &[
    "{}",
    "{1,2,3}",
    "{a=1,b=2}",
    "setmetatable({},{__index=function() return 1 end})",
    "{[1000000]=1}",
    // A non-matching element at INT_MAX: makes index-walk loops (table.find,
    // table.unpack) run up to INT_MAX and overflow on the next increment.
    "{[2147483647]=false}",
    "table.create(8,0)",
];
const FN: &[&str] = &[
    "function() end",
    "function(...) return ... end",
    "function() error(\"x\") end",
    "print",
    "function(a,b) return a<b end",
];
const BOOL: &[&str] = &["true", "false"];
const BUF: &[&str] = &[
    "buffer.create(0)",
    "buffer.create(8)",
    "buffer.create(64)",
    "buffer.fromstring(\"abcd\")",
];
const ANY: &[&str] = &[
    "nil",
    "0",
    "\"\"",
    "{}",
    "true",
    "-2147483648",
    "1/0",
    "function() end",
];
const VEC: &[&str] = &[
    "vector.create(0, 0, 0)",
    "vector.create(1, 2, 3)",
    "vector.create(1/0, 0/0, -1e308)",
    "vector.create(math.huge, -1e308, 2^53)",
];

fn pick(u: &mut Unstructured, pool: &[&str]) -> String {
    let i = u.int_in_range(0..=pool.len() - 1).unwrap_or(0);
    pool[i].to_string()
}

/// Bytes a fuzzer-generated string leaf is biased toward: the format / pack-code /
/// pattern metacharacters of the string mini-languages, so that whenever a
/// generated string flows into `string.format`/`pack`/`unpack`/`find`/`match`/
/// `gsub` (or any string-accepting builtin) those hand-ported parsers get fuzzed
/// automatically — the string ARGUMENT is itself a fuzz target, not a fixed pool.
const STR_META: &[u8] = b"%diouxXeEfgGqsc0123456789.-+ #*lhLI<>=!bBhHjJTnzZ[]()^$?&aA{}/:";

/// A string-typed leaf: with even odds a hostile edge value from [`STR`] (empty,
/// huge, non-UTF-8, format/pattern-laden) OR a FUZZER-DRIVEN string built from the
/// byte stream (metacharacter-biased), escaped for a Lua `"..."` literal. The
/// second arm is what makes `string.format`/`pack`/pattern parsers reachable
/// through the api target — the leaf is the fuzz surface.
fn gen_string(u: &mut Unstructured) -> String {
    if matches!(u.int_in_range(0..=1u8), Ok(0)) {
        return pick(u, STR);
    }
    let n = u.int_in_range(0..=24u8).unwrap_or(0);
    let mut s = String::from("\"");
    for _ in 0..n {
        let c = if matches!(u.int_in_range(0..=3u8), Ok(0)) {
            u.int_in_range(32..=126u8).unwrap_or(b'a')
        } else {
            *u.choose(STR_META).unwrap_or(&b'%')
        } as char;
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            _ => s.push(c),
        }
    }
    s.push('"');
    s
}

/// A number-typed leaf: with even odds a hostile edge value from [`NUM`]
/// (INT_MIN/MAX, 2^53, NaN, inf, ...) OR a fuzzer-driven arbitrary integer, so the
/// number argument is a fuzz target too rather than only the curated corners.
fn gen_number(u: &mut Unstructured) -> String {
    if matches!(u.int_in_range(0..=1u8), Ok(0)) {
        return pick(u, NUM);
    }
    let v: i64 = u.arbitrary().unwrap_or(0);
    v.to_string()
}

/// Max nesting depth for the recursive value generator.
const MAX_DEPTH: u8 = 4;

/// The scalar+compound kinds a recursively-generated value can take.
const VALUE_KINDS: [K; 7] = [K::Num, K::Str, K::Bool, K::Tbl, K::Fn, K::Buf, K::Vec];

/// One value of `kind`, recursively generated and fuzzer-steered. Compound kinds
/// (table/function/buffer/any) are built from the SAME generator, so tables nest
/// and carry hostile leaf values, functions return hostile values, and `any` can
/// be anything — all driven by the fuzzer's byte stream (`Unstructured`), so
/// coverage feedback steers the *shape* of the data, not just scalar leaves.
fn gen_value(u: &mut Unstructured, kind: K, depth: u8) -> String {
    match kind {
        K::Num | K::VarNum => gen_number(u),
        // Bounded count/size/range: half from the IDX pool, half a fuzzer-driven
        // small int — leaf-driven but magnitude-capped (see K::Idx).
        K::Idx => {
            if matches!(u.int_in_range(0..=1u8), Ok(0)) {
                pick(u, IDX)
            } else {
                u.int_in_range(-2000i64..=2000).unwrap_or(0).to_string()
            }
        }
        K::Str => gen_string(u),
        K::Bool => pick(u, BOOL),
        K::Fn => gen_function(u, depth),
        K::Buf => gen_buffer(u),
        K::Vec => pick(u, VEC),
        K::Tbl => gen_table(u, depth),
        K::Any | K::VarAny => {
            if matches!(u.int_in_range(0..=5u8), Ok(0)) {
                return "nil".to_string();
            }
            let k = *u.choose(&VALUE_KINDS).unwrap_or(&K::Num);
            gen_value(u, k, depth)
        }
    }
}

/// A hostile scalar table key.
fn gen_key(u: &mut Unstructured) -> String {
    match u.int_in_range(0..=2u8).unwrap_or(0) {
        1 => pick(u, STR),
        2 => pick(u, BOOL),
        _ => pick(u, NUM),
    }
}

/// A table literal whose fields are themselves recursively generated values
/// (array + keyed), optionally wrapped in a throwing / redirecting metatable.
fn gen_table(u: &mut Unstructured, depth: u8) -> String {
    if depth == 0 {
        return pick(u, &["{}", "{1, 2, 3}", "{[1000000] = 1}"]);
    }
    let n = u.int_in_range(0..=4u8).unwrap_or(0);
    let mut fields = Vec::new();
    for _ in 0..n {
        let vk = *u.choose(&VALUE_KINDS).unwrap_or(&K::Num);
        if matches!(u.int_in_range(0..=2u8), Ok(0)) {
            fields.push(format!(
                "[{}] = {}",
                gen_key(u),
                gen_value(u, vk, depth - 1)
            ));
        } else {
            fields.push(gen_value(u, vk, depth - 1));
        }
    }
    let base = format!("{{{}}}", fields.join(", "));
    if matches!(u.int_in_range(0..=4u8), Ok(0)) {
        let mt = match u.int_in_range(0..=2u8).unwrap_or(0) {
            1 => format!(
                "{{__index = function() return {} end}}",
                gen_value(u, K::Num, 0)
            ),
            2 => format!("{{__len = function() return {} end}}", pick(u, NUM)),
            _ => "{__index = function() error(\"mt\") end}".to_string(),
        };
        format!("setmetatable({base}, {mt})")
    } else {
        base
    }
}

/// A function value: a comparator, a thrower, or a closure returning a hostile
/// value (so callbacks like `table.sort`/`gsub`/metamethods get adversarial returns).
fn gen_function(u: &mut Unstructured, depth: u8) -> String {
    match u.int_in_range(0..=4u8).unwrap_or(0) {
        1 => {
            let k = *u.choose(&VALUE_KINDS).unwrap_or(&K::Num);
            format!(
                "function() return {} end",
                gen_value(u, k, depth.saturating_sub(1))
            )
        }
        2 => format!(
            "function(a, b) return {} end",
            pick(u, &["a < b", "b < a", "true", "false", "a == b"])
        ),
        3 => "function(...) return ... end".to_string(),
        4 => "function() end".to_string(),
        _ => "function() error(\"x\") end".to_string(),
    }
}

/// A buffer value built from a hostile size or a hostile source string.
fn gen_buffer(u: &mut Unstructured) -> String {
    match u.int_in_range(0..=2u8).unwrap_or(0) {
        1 => format!("buffer.fromstring({})", pick(u, STR)),
        2 => format!("buffer.create({})", pick(u, &["0", "1", "8", "64", "256"])),
        _ => format!("buffer.create({})", pick(u, &["0", "8", "64"])),
    }
}

/// Whether a local of kind `have` can satisfy an argument slot of kind `want`.
fn kind_compatible(want: K, have: K) -> bool {
    match want {
        K::Any | K::VarAny => true,
        K::Num | K::VarNum => matches!(have, K::Num | K::VarNum),
        // An Idx slot must NOT reuse a `Num` local — that local could be
        // `math.huge` and reintroduce the allocation/loop blowup. No local is ever
        // declared as `Idx` (VALUE_KINDS excludes it), so this always generates a
        // fresh bounded value.
        K::Idx => matches!(have, K::Idx),
        K::Str => matches!(have, K::Str),
        K::Bool => matches!(have, K::Bool),
        K::Tbl => matches!(have, K::Tbl),
        K::Fn => matches!(have, K::Fn),
        K::Buf => matches!(have, K::Buf),
        K::Vec => matches!(have, K::Vec),
    }
}

/// Fill one argument slot of `kind`: with decent probability reuse a previously
/// bound local of a compatible kind — so stateful read-after-write sequences and
/// data aliasing (the same buffer/table written then read, or shared across two
/// calls) become reachable — otherwise generate a fresh recursive value.
fn fill_arg(u: &mut Unstructured, kind: K, vars: &[(String, K)]) -> String {
    let compatible: Vec<&str> = vars
        .iter()
        .filter(|(_, vk)| kind_compatible(kind, *vk))
        .map(|(n, _)| n.as_str())
        .collect();
    if !compatible.is_empty() && matches!(u.int_in_range(0..=1u8), Ok(0)) {
        if let Ok(v) = u.choose(&compatible) {
            return (*v).to_string();
        }
    }
    gen_value(u, kind, MAX_DEPTH)
}

/// Append one builtin call statement, drawing each argument from `fill_arg` (so it
/// may reuse the in-scope locals).
fn build_call(
    u: &mut Unstructured,
    name: &str,
    kinds: &[K],
    vars: &[(String, K)],
    prog: &mut String,
) {
    let mut args = Vec::new();
    for &k in kinds {
        match k {
            K::VarNum => {
                for _ in 0..u.int_in_range(0..=4u8).unwrap_or(0) {
                    args.push(fill_arg(u, K::Num, vars));
                }
            }
            K::VarAny => {
                for _ in 0..u.int_in_range(0..=4u8).unwrap_or(0) {
                    args.push(fill_arg(u, K::Any, vars));
                }
            }
            _ => args.push(fill_arg(u, k, vars)),
        }
    }
    prog.push_str(&format!("{}({})\n", name, args.join(", ")));
}

/// Loop iteration counts the call body may be wrapped in (0 = no loop). Bounded so
/// repeated execution exercises GC pressure / persistent-object mutation without
/// hanging (the step-limit interrupt only fires between bytecode ops).
const LOOP_COUNTS: [u32; 4] = [0, 4, 16, 64];

/// Build a short stateful program: zero to three locals (recursively-generated
/// hostile values), then one to three builtin call statements that may reuse those
/// locals — optionally wrapped in a bounded loop. Three combined strategies:
///   * metadata enumeration — every covered builtin is reachable with type-correct
///     argument kinds;
///   * recursive + edge-value arguments — nested tables/closures/buffers/vectors
///     with INT_MIN/MAX, NaN, inf, huge sizes;
///   * stateful sequences + loops — locals declared once and reused across calls
///     (read-after-write / aliasing) and re-run under a loop (`for` over a hostile
///     count) so repeated allocation/free and persistent mutation are exercised.
/// All choices are steered by the fuzzer byte stream (deterministic per input).
pub fn generate_api_call(data: &[u8]) -> String {
    let mut u = Unstructured::new(data);
    let mut prog = String::new();
    let mut vars: Vec<(String, K)> = Vec::new();

    // Locals live OUTSIDE the loop, so a wrapped body mutates persistent state.
    let nlocals = u.int_in_range(0..=3u8).unwrap_or(0);
    for i in 0..nlocals {
        let k = *u.choose(&VALUE_KINDS).unwrap_or(&K::Num);
        let name = format!("v{i}");
        prog.push_str(&format!(
            "local {} = {}\n",
            name,
            gen_value(&mut u, k, MAX_DEPTH)
        ));
        vars.push((name, k));
    }

    let mut body = String::new();
    let nstmts = u.int_in_range(1..=3u8).unwrap_or(1);
    for _ in 0..nstmts {
        let idx = u.int_in_range(0..=API.len() - 1).unwrap_or(0);
        let (name, kinds) = API[idx];
        build_call(&mut u, name, kinds, &vars, &mut body);
    }

    let loop_n = *u.choose(&LOOP_COUNTS).unwrap_or(&0);
    if loop_n == 0 {
        prog.push_str(&body);
    } else {
        // `_i` is also offered to fill_arg-free callers via a wrapped index; the
        // body already chose its args, so the loop just repeats them.
        prog.push_str(&format!("for _i = 1, {loop_n} do\n{body}end\n"));
    }
    prog
}

/// Every covered call path, for a coverage/smoke pass (one call per function).
pub fn api_function_count() -> usize {
    API.len()
}

/// The call path at `index` (for the standalone coverage smoke).
pub fn api_function_name(index: usize) -> &'static str {
    API[index % API.len()].0
}

/// A benign argument for kind `k` (used to fill the non-probed positions during
/// a single-argument hostile sweep).
fn safe(k: K) -> &'static str {
    match k {
        K::Num | K::VarNum => "1",
        K::Idx => "1",
        K::Str => "\"x\"",
        K::Tbl => "{1, 2, 3}",
        K::Fn => "function(a, b) return a < b end",
        K::Bool => "true",
        K::Buf => "buffer.create(8)",
        K::Vec => "vector.create(1, 2, 3)",
        K::Any | K::VarAny => "1",
    }
}

/// The boundary-value pool for kind `k`.
fn pool(k: K) -> &'static [&'static str] {
    match k {
        K::Num | K::VarNum => NUM,
        K::Idx => IDX,
        K::Str => STR,
        K::Tbl => TBL,
        K::Fn => FN,
        K::Bool => BOOL,
        K::Buf => BUF,
        K::Vec => VEC,
        K::Any | K::VarAny => ANY,
    }
}

/// A small "most dangerous" subset of kind `k`, for the bounded pairwise sweep.
fn danger(k: K) -> &'static [&'static str] {
    match k {
        K::Num | K::VarNum => &["-2147483648", "2147483647", "2^53", "-1", "0"],
        K::Idx => &["2000", "-1000", "-1", "0"],
        K::Str => &["\"\"", "(\"ab\"):rep(40000)"],
        K::Tbl => &["{}", "{1, 2, 3}", "{[2147483647] = false}"],
        K::Buf => &["buffer.create(0)", "buffer.create(8)"],
        K::Fn => &["function() error(\"x\") end"],
        K::Bool => &["true", "false"],
        K::Vec => &["vector.create(0, 0, 0)", "vector.create(1/0, 0/0, -1e308)"],
        K::Any | K::VarAny => &["nil", "-2147483648"],
    }
}

/// Deterministic hostile sweep for function `index`, used by the `api_triage`
/// crash-matrix tool (which runs each function in its own process). Two passes:
///   1. single-arg: every boundary value in each position, others benign — finds
///      the one-extreme-arg bugs (most of the overflow/assert class);
///   2. pairwise: two positions hostile at once (from the smaller `danger` set),
///      others benign — catches multi-arg interactions (e.g. an index/range pair
///      that overruns the stack) that a single-arg sweep can't reach.
/// Bounded (linear + quadratic-in-positions), never cartesian, so no blowup.
pub fn api_probe_calls(index: usize) -> Vec<String> {
    let (name, kinds) = API[index % API.len()];
    if kinds.is_empty() {
        return vec![format!("return {}()", name)];
    }
    let build = |overrides: &[(usize, &str)]| -> String {
        let args: Vec<String> = kinds
            .iter()
            .enumerate()
            .map(|(j, &k)| {
                overrides
                    .iter()
                    .find(|&&(p, _)| p == j)
                    .map(|&(_, v)| v.to_string())
                    .unwrap_or_else(|| safe(k).to_string())
            })
            .collect();
        format!("return {}({})", name, args.join(", "))
    };
    let mut out = Vec::new();
    for pos in 0..kinds.len() {
        for &val in pool(kinds[pos]) {
            out.push(build(&[(pos, val)]));
        }
    }
    for a in 0..kinds.len() {
        for b in (a + 1)..kinds.len() {
            for &va in danger(kinds[a]) {
                for &vb in danger(kinds[b]) {
                    out.push(build(&[(a, va), (b, vb)]));
                }
            }
        }
    }
    out
}
