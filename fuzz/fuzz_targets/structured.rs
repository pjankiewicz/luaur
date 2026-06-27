// Port of Luau's `fuzz/proto.cpp`: structured generation. Rather than feeding
// raw bytes at the parser, the bytes drive a grammar that emits syntactically
// valid Luau (see `luaur_fuzz::generate`, which builds an `Unstructured` over the
// bytes), so the input reaches deep into the compiler + VM while AFL's coverage
// feedback steers generation. Compile and (if it compiles) run under an interrupt
// step-limit; must never crash.

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, Result, VmState};

thread_local! {
    // The type-check here used the one-shot `luaur_rt::check`, which rebuilds the
    // frontend and re-checks the whole @luau definition file every input — that
    // alone capped the target near ~400 exec/s (observed ~77/s combined with the
    // compile+run below). Reusing a Checker registers the builtins once, so the
    // per-input analysis cost drops to just parsing+checking the program. A faster
    // target explores more inputs per wall-clock second — i.e. finds more bugs.
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate(data);

    let lua = Lua::new();
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

    // Also type-check it (valid programs exercise the analysis layer).
    CHECKER.with(|c| {
        let _ = c.borrow_mut().check(&src);
    });

    if let Ok(f) = lua.load(&src).set_name("fuzz").into_function() {
        let _ = f.call::<()>(());
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
