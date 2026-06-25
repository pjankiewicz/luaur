# luaur

**A faithful, line-for-line translation of [Luau](https://github.com/luau-lang/luau) — Roblox's typed Lua — from C++17 to Rust.**

Not bindings. Not a reimplementation. The actual Luau compiler, virtual machine, and
type checker, ported to safe-by-default Rust and validated against Luau's **own** test
suite: **5,347 ported unit tests pass, and all 293 upstream conformance scripts run
byte-identically** on the Rust VM (against Luau commit [`8f33df9`](https://github.com/luau-lang/luau)).

```rust
// A safe, mlua-style API over a pure-Rust Luau VM — no C, no FFI, no emscripten.
use luaur::Lua;

let lua = Lua::new();

// Expose a Rust function to Luau, then run a script that calls it:
lua.globals().set("add", lua.create_function(|_, (a, b): (i64, i64)| Ok(a + b))?)?;
let sum: i64 = lua.load("return add(2, 3)").eval()?;
assert_eq!(sum, 5);
```

## Why this exists

Automated C++→Rust translation is an open problem. The published state of the art
(RustMap, EvoC2Rust, DARPA TRACTOR) tops out around **~13k lines of C at ~87%
equivalence with human patching** — their atomization breaks down before real scale.

luaur is **~205k lines of production C++17** (lexer, parser, bytecode compiler, register
VM, a full bidirectional type checker, native code generation, CLIs) translated to
**~420k lines of Rust**, with equivalence proven by two independent oracles instead of
spot checks:

1. **The maintainers' own test suite** — 5,347 unit tests ported and passing.
2. **A byte-exact bytecode differential** — programs compiled by C++ Luau and executed
   on the Rust VM produce identical results.

The interesting part isn't the graph-and-topo-sort skeleton (that's in the literature).
It's the **atomization and per-node context engineering** that let the obvious approach
survive to production scale as a *convergent* system — see
[`docs/TRANSLATION.md`](docs/TRANSLATION.md) for how it was actually built, the timeline,
the model economics, and the war stories.

## A faithful engine — *plus* an mlua-style API

luaur is two things. The translated engine above is a *faithful* port of Luau's C++.
On top of it sits **`luaur-rt`**, a safe, ergonomic Rust API whose interface deliberately
mirrors [`mlua`](https://github.com/mlua-rs/mlua), so embedders are immediately at home —
`Lua`, `Value`, `Table`, `Function`, `FromLua`/`IntoLua`, `create_function`, `UserData`.
*(This high-level layer is a Rust-native addition; it has no C++ counterpart and is **not**
part of the "faithful translation" claim — it's the value-add for using luaur as a library.)*

Expose Rust types — with methods and metamethods — to Luau:

```rust
use luaur::{Lua, UserData, UserDataMethods};

struct Vec2 { x: f64, y: f64 }
impl UserData for Vec2 {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("magnitude", |_, v, ()| Ok((v.x * v.x + v.y * v.y).sqrt()));
        // metamethods too: methods.add_meta_method("__add", ...)
    }
}

let lua = Lua::new();
lua.globals().set("v", lua.create_userdata(Vec2 { x: 3.0, y: 4.0 })?)?;
let m: f64 = lua.load("return v:magnitude()").eval()?;   // 5.0
```

A Rust `Err` (or even a `panic!`) returned from a callback surfaces as a catchable Lua
error, not a crash. Unlike `mlua` — which FFI-binds the C/C++ Luau and needs a C toolchain —
this is **pure Rust**, so it runs anywhere Rust does, including `wasm32-unknown-unknown`.
The lower-level `luaur::{compile, eval, check}` helpers and the raw C-style VM API
(`luaur::vm`) are there when you want them.

### How compatible with mlua, really?

The honest way to answer "is the interface mlua-compatible" is to take mlua's **own**
test suite and run it against luaur-rt with nothing changed but the import path. We did:
**184 of 187 ported mlua tests pass unmodified (98%)** — import-swap only, no test rewrites.
The other **3** are pinned to *document* a genuine Lua-vs-Luau deviation rather than hide it
(`typeof(err)` is `"string"` not `"error"` because Luau has no tagged error value; no heap
object enumeration by type; the `{:#?}` table-dump format). A handful of mlua behaviors are
intentionally not ported because Luau is Lua-5.x-incompatible by design — the Lua-5.x debug
hooks (only the VM *interrupt* exists), native `i64`, and `collectgarbage`/`loadstring` in the
base library — several of which **mlua itself disables** for its own `luau` feature.

The same opt-in feature flags as mlua are mirrored, so existing mlua code feels at home:

```toml
luaur = { version = "0.1", features = ["serde", "async", "macros"] }   # or "send"
```

`serde` (Rust↔Lua `Serialize`/`Deserialize`), `async` (Rust futures ↔ Luau coroutines),
`send` (`Send`/`Sync` handles, mutually exclusive with `async`, as in mlua), and `macros`
(`#[derive(UserData)]` / `#[derive(FromLua)]`).

### A type checker mlua can't have

Because luaur ships Luau's **type checker**, not just its VM, you can type-check a script
against the host surface *before* running it — register Luau `declare` definitions for the
Rust functions and userdata you expose, and the static checker holds the script to them.
Lua has no static types, so this is something an mlua-style API fundamentally cannot offer:

```rust
// `add` is a host function; declaring its type lets the script type-check against it.
luaur::check("local n: number = add(1, 2)").unwrap_err();            // unknown global
luaur::check_with_definitions(
    "local n: number = add(1, 2)",
    "declare function add(a: number, b: number): number",
)
.unwrap();                                                            // checks clean
```

## How idiomatic is it?

Body-to-body (imports, comments and blanks stripped), the port is **1.96×** the size of
the C++ — expansion you'd expect from making implicit C++ ownership explicit in Rust, not
from transliteration bloat. Pointers became `*mut T` where Luau's arena/GC model requires
it (faithfully), and ordinary value code became ordinary Rust.

```cpp
// C++ (Luau, VM/src/lvmexecute.cpp)
LuaTable* h = hvalue(ra);
const TValue* res = luaH_get(h, kv);
```
```rust
// Rust (luaur-vm)
let h = hvalue(ra);
let res = luaH_get(h, kv);
```

## The crates

luaur is published as independent crates so you can depend on exactly the layer you need:

| Crate | What it is |
|---|---|
| [`luaur`](crates/luaur) | **Start here.** Umbrella: the mlua-style API + `compile`/`eval`/`check` helpers, re-exporting every layer |
| [`luaur-rt`](crates/luaur-rt) | The safe, ergonomic **mlua-style API** (`Lua`, `create_function`, `UserData`, `FromLua`/`IntoLua`) |
| [`luaur-common`](crates/luaur-common) | Foundations: `SmallVector`, `DenseHashMap`, `Variant`, FastFlags |
| [`luaur-ast`](crates/luaur-ast) | Lexer, parser, AST |
| [`luaur-bytecode`](crates/luaur-bytecode) | Bytecode format + builder |
| [`luaur-compiler`](crates/luaur-compiler) | Luau source → bytecode compiler |
| [`luaur-code-gen`](crates/luaur-code-gen) | Native code generation (A64 / X64) |
| [`luaur-vm`](crates/luaur-vm) | The register VM + standard library |
| [`luaur-analysis`](crates/luaur-analysis) | Type checker / type inference |
| [`luaur-config`](crates/luaur-config) | `.luaurc` configuration |
| [`luaur-require`](crates/luaur-require) | Require-by-string module resolution |
| [`luaur-repl-cli`](crates/luaur-repl-cli) | Interactive REPL |
| [`luaur-analyze-cli`](crates/luaur-analyze-cli) | Standalone type-checker CLI |
| [`luaur-web`](crates/luaur-web) | `wasm32` bindings — run/type-check Luau in the browser |

(Plus `luaur-ast-cli`, `luaur-compile-cli`, `luaur-bytecode-cli`, `luaur-reduce-cli`, and `luaur-cli-lib`.)

## WebAssembly

The compiler, VM and type checker build for `wasm32-unknown-unknown` — the entire toolchain
runs client-side with no server. `luaur-web` exposes `run`/`check` entry points for an
in-browser playground:

```toml
luaur-web = { version = "0.1", features = ["wasm"] }
```

## Conformance scope

What is and isn't covered (and against which upstream commit) is stated precisely — no
blanket "perfect port" claims — in [`docs/CONFORMANCE.md`](docs/CONFORMANCE.md).

## License

MIT. luaur is a derivative translation of Luau (© Roblox Corporation) which derives from
Lua (© Lua.org, PUC-Rio); both upstream copyrights are preserved. See [`LICENSE`](LICENSE).
