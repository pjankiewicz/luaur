// AST-splicing target: start from a REAL Luau program (the embedded conformance
// suite) and apply a fuzzer-byte-driven sequence of statement-level mutations to
// it (see `luaur_fuzz::generate_spliced`). The leading bytes pick the seed; the
// tail bytes are the mutation program, so AFL mutating the tail explores nearby
// mutations of the same real script — reaching language-feature combinations a
// from-scratch grammar never produces. Type-check, compile, and (if it compiles)
// run the result under a step limit; must never crash.

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, Result, VmState};

thread_local! {
    // Builtins registered once (the expensive step); see the `typeck` target.
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate_spliced(data);

    CHECKER.with(|c| {
        let _ = c.borrow_mut().check(&src);
    });

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
