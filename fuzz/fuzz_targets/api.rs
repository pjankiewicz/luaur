// Metadata-driven stdlib target: fuzz the WHOLE standard-library surface. The
// other generator targets drive the parser/compiler/type-checker/VM control
// flow, but they essentially never CALL a builtin — `SAFE_GLOBALS` is just
// `type`/`tostring`/... — so the integer/size-arithmetic and format/pattern
// bugs that live in the builtins (bit32 / os / string.pack / table / buffer /
// utf8 / vector) were unreachable. This target closes that gap: a metadata table
// of ~140 builtins (call path + per-argument KIND) crossed with boundary-value
// pools (INT_MIN/MAX, 2^53, NaN, inf, huge strings, out-of-range offsets), plus
// fuzzer-driven string/number leaves so the string mini-language parsers
// (`format`/`pack`/patterns) are fuzzed through any string-accepting builtin.
// Programs are stateful (locals reused across calls → read-after-write / aliasing)
// and may be loop-wrapped (GC pressure). See `luaur_fuzz::api_gen`.
//
// Oracle: never panic/abort/hang — only Ok or a structured Err. A Lua error from
// a bad argument is caught by the VM call boundary; only a panic/abort (overflow,
// assert, OOB) surfaces as a crash. Interrupt-step-limited. The metadata bounds
// COUNT/SIZE/RANGE args (K::Idx) so no call OOM-aborts or hangs the fuzzer.

use std::cell::Cell;
use std::rc::Rc;

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, Result, VmState};

fn exercise_input(data: &[u8]) {
    // No big-stack wrapper: the metadata generator is structurally SHALLOW —
    // recursive value construction is depth-bounded (MAX_DEPTH) and no builtin
    // it emits recurses natively, so it can't reach the deep-recursion stack
    // limit that `gcstress` (spliced real programs) needs `run_on_big_stack` for.
    let src = luaur_fuzz::generate_api_call(data);

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
