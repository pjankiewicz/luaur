// Host-embedding target: fuzz the Rust<->Lua boundary — the mlua-compat surface
// (`create_function`, `UserData`, the registry) that downstream Rust embedders
// depend on and that NO other target touches. Host functions appear elsewhere
// only as fixed harness plumbing (the `print` capture, the interrupt); here they
// are the fuzzed object.
//
// The input drives: (1) a set of host callbacks `h0..hN`, each with a
// fuzzer-chosen behavior (echo args, raise an error value, call an argument back,
// force a GC, build a table, re-enter the compiler via `lua.load`, round-trip a
// registry ref); (2) a userdata value `ud` with generated methods + metamethods;
// (3) a generated Luau driver that calls those callbacks and the userdata with
// generated args — including passing callbacks to each other, so control bounces
// Rust->Lua->Rust repeatedly.
//
// Reentrancy is DEPTH-BOUNDED by a shared counter: an unbounded callback->arg->
// callback chain would overflow the native stack (an uncatchable abort — a false
// positive), so beyond a fixed depth the "call back" behaviors return instead of
// recursing. That still exercises deep reentrancy without a spurious crash.
//
// Oracle: never panic/abort/hang — only Ok or a structured Err. Interrupt
// step-limited.

use std::cell::Cell;
use std::rc::Rc;

use arbitrary::Unstructured;

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, Result, UserData, UserDataMethods, Value, Variadic, VmState};

/// Maximum Rust-reentrancy depth for callback-calls-its-argument behaviors,
/// beyond which they stop recursing (native-stack-overflow guard).
const MAX_DEPTH: u32 = 16;

/// A tiny fuzzable userdata: interior-mutable integer state plus methods and
/// metamethods, so `ud:get()`, `ud:set(n)`, `#ud`, `tostring(ud)`, `ud + x`
/// exercise the userdata dispatch / metatable paths.
struct FuzzUd {
    v: Cell<i64>,
}

impl UserData for FuzzUd {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |_, this, ()| Ok(this.v.get()));
        // Cell interior mutability lets a `&self` method mutate — no need for
        // add_method_mut (which would also exercise the RefCell reentrancy guard,
        // but &self keeps re-entrant `ud:set()` from a metamethod legal).
        methods.add_method("set", |_, this, n: i64| {
            this.v.set(n);
            Ok(())
        });
        methods.add_meta_method("__len", |_, this, ()| Ok(this.v.get().unsigned_abs()));
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("ud({})", this.v.get()))
        });
        methods.add_meta_method("__add", |_, this, other: i64| {
            Ok(this.v.get().wrapping_add(other))
        });
    }
}

fn install_host_fns(lua: &Lua, u: &mut Unstructured, depth: &Rc<Cell<u32>>) -> usize {
    let n = u.int_in_range(1..=4usize).unwrap_or(2);
    for i in 0..n {
        let behavior = u.int_in_range(0u8..=7).unwrap_or(0);
        let depth = depth.clone();
        let f = lua.create_function(
            move |lua, args: Variadic<Value>| -> Result<Variadic<Value>> {
                match behavior {
                    0 => Ok(Variadic::new()), // return nothing
                    1 => Ok(args),            // echo the arguments back
                    2 => Err(luaur_rt::Error::runtime("host: intentional error")),
                    3 => {
                        // Call the first argument if it's callable — depth-bounded.
                        if depth.get() < MAX_DEPTH {
                            depth.set(depth.get() + 1);
                            if let Some(Value::Function(callee)) = args.first() {
                                let _ = callee.call::<Value>(());
                            }
                            depth.set(depth.get() - 1);
                        }
                        Ok(Variadic::new())
                    }
                    4 => {
                        // Force a collection mid-call, then hand back a fresh table.
                        let _ = lua.gc_collect();
                        let t = lua.create_table();
                        Ok(Variadic::from(vec![Value::Table(t)]))
                    }
                    5 => {
                        // Build a small nested table.
                        let t = lua.create_table();
                        let inner = lua.create_table();
                        let _ = inner.set(1, 1);
                        let _ = t.set("inner", inner);
                        Ok(Variadic::from(vec![Value::Table(t)]))
                    }
                    6 => {
                        // Re-enter the compiler from within a host call.
                        if depth.get() < MAX_DEPTH {
                            depth.set(depth.get() + 1);
                            if let Ok(g) = lua.load("return 1 + 1").set_name("host").into_function()
                            {
                                let _ = g.call::<Value>(());
                            }
                            depth.set(depth.get() - 1);
                        }
                        Ok(Variadic::new())
                    }
                    _ => {
                        // Round-trip a value through the registry.
                        let val = args.first().cloned().unwrap_or(Value::Nil);
                        if let Ok(key) = lua.create_registry_value(val) {
                            let back = lua.registry_value::<Value>(&key).unwrap_or(Value::Nil);
                            return Ok(Variadic::from(vec![back]));
                        }
                        Ok(Variadic::new())
                    }
                }
            },
        );
        if let Ok(f) = f {
            let _ = lua.globals().set(format!("h{i}"), f);
        }
    }
    n
}

/// Build a Luau driver that calls `h0..h{nfns-1}` and `ud` with generated args.
fn build_driver(u: &mut Unstructured, nfns: usize) -> String {
    let mut out = String::new();
    let arg = |u: &mut Unstructured, nfns: usize| -> String {
        match u.int_in_range(0u8..=4).unwrap_or(0) {
            0 => format!("{}", u.int_in_range(-100i64..=100).unwrap_or(0)),
            1 => format!("\"s{}\"", u.int_in_range(0u8..=9).unwrap_or(0)),
            2 => format!("h{}", u.int_in_range(0..=nfns - 1).unwrap_or(0)),
            3 => "ud".to_string(),
            // A closure that itself calls a host fn — deepens Rust<->Lua bounce.
            _ => format!(
                "function() return h{}() end",
                u.int_in_range(0..=nfns - 1).unwrap_or(0)
            ),
        }
    };

    let n_stmts = u.int_in_range(1..=14u32).unwrap_or(4);
    for _ in 0..n_stmts {
        if u.is_empty() {
            break;
        }
        match u.int_in_range(0u8..=4).unwrap_or(0) {
            // ud method / metamethod exercises
            0 => out.push_str("pcall(function() return ud:get() end)\n"),
            1 => out.push_str(&format!(
                "pcall(function() ud:set({}) end)\n",
                u.int_in_range(-100i64..=100).unwrap_or(0)
            )),
            2 => out.push_str("pcall(function() return #ud + tostring(ud):len() end)\n"),
            // a host call with generated args
            _ => {
                let i = u.int_in_range(0..=nfns - 1).unwrap_or(0);
                let nargs = u.int_in_range(0u8..=3).unwrap_or(1);
                let mut args = Vec::new();
                for _ in 0..nargs {
                    args.push(arg(u, nfns));
                }
                out.push_str(&format!("pcall(h{}, {})\n", i, args.join(", ")));
            }
        }
    }
    out
}

fn exercise_input(data: &[u8]) {
    // No big-stack wrapper: callback-calls-its-argument reentrancy is DEPTH-
    // BOUNDED to MAX_DEPTH, so this target can't reach the deep-recursion native-
    // stack limit that `gcstress` (spliced real programs) needs `run_on_big_stack`
    // for. Keeping it inline avoids the per-input thread-spawn cost.
    let mut u = Unstructured::new(data);
    let lua = Lua::new();
    let depth = Rc::new(Cell::new(0u32));

    let nfns = install_host_fns(&lua, &mut u, &depth);

    // Expose the userdata as `ud`.
    if let Ok(ud) = lua.create_userdata(FuzzUd { v: Cell::new(0) }) {
        let _ = lua.globals().set("ud", ud);
    }

    // Step-limit the driver.
    let steps = Rc::new(Cell::new(0u64));
    let counter = steps.clone();
    let step_limit = luaur_fuzz::vm_step_limit();
    lua.set_interrupt(move |_| -> Result<VmState> {
        let c = counter.get() + 1;
        counter.set(c);
        if c > step_limit {
            Err(luaur_rt::Error::runtime("fuzz: step limit"))
        } else {
            Ok(VmState::Continue)
        }
    });

    let src = build_driver(&mut u, nfns);
    if let Ok(f) = lua.load(&src).set_name("fuzz").into_function() {
        let _ = f.call::<()>(());
    }

    let _ = lua.gc_collect();
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
