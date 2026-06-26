// Port of Luau's `kFuzzVM` path: compile arbitrary source and, if it compiles,
// run it on the VM. Execution is bounded by an interrupt step-limit so a
// generated infinite loop can't hang the fuzzer. The VM must never panic/crash
// — only return `Ok`/`Err`.
#![no_main]

use std::cell::Cell;
use std::rc::Rc;

use libfuzzer_sys::fuzz_target;
use luaur_rt::{Lua, Result, VmState};

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else {
        return;
    };
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
    if let Ok(f) = lua.load(src).set_name("fuzz").into_function() {
        let _ = f.call::<()>(());
    }
});
