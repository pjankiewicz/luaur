// Port of Luau's `fuzz/proto.cpp`: structured generation. Rather than feeding
// raw bytes at the parser, the bytes drive a grammar that emits syntactically
// valid Luau (see `luaur_fuzz::generate`, which builds an `Unstructured` over the
// bytes), so the input reaches deep into the compiler + VM while AFL's coverage
// feedback steers generation. Compile and (if it compiles) run under an interrupt
// step-limit; must never crash.

use std::cell::Cell;
use std::rc::Rc;

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, Result, VmState};

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate(data);

    let lua = Lua::new();
    let steps = Rc::new(Cell::new(0u64));
    let counter = steps.clone();
    lua.set_interrupt(move |_| -> Result<VmState> {
        let c = counter.get() + 1;
        counter.set(c);
        if c > 1_000_000 {
            Err(luaur_rt::Error::runtime("fuzz: step limit"))
        } else {
            Ok(VmState::Continue)
        }
    });

    // Also type-check it (valid programs exercise the analysis layer).
    let _ = luaur_rt::check(&src);

    if let Ok(f) = lua.load(&src).set_name("fuzz").into_function() {
        let _ = f.call::<()>(());
    }
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
