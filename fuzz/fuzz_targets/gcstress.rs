// GC-stress target: run allocation-heavy generated programs while forcing the
// incremental collector to run as often and as hard as possible, so a collection
// lands at hostile points (mid-allocation, between metamethod re-entries). A
// faithful GC port's worst class of bug — a missed root / premature free — only
// manifests when collection interleaves with mutation like this, and it stays
// silent under the default lazy cadence the other targets run at.
//
// Two sound levers drive the aggression, both from the fuzzer's config bytes:
//   * `gc_inc(goal, step_multiplier, step_size)` — a low goal + high multiplier +
//     small step makes the VM's OWN allocation-triggered incremental GC fire
//     frequently and finish quickly during execution (the intended tuning
//     surface — no reentrancy concern).
//   * an optional `gc_step()` from inside the interrupt callback (gated on a
//     fuzzer bit), which forces a step at VM safepoints. This is the more
//     aggressive lever; the gate lets the standalone smoke run confirm it's a
//     supported operation in this port rather than a false-positive source.
// A final `gc_collect()` after the run surfaces anything left dangling.
//
// Oracle: never panic/abort/hang — only Ok or a structured Err. Interrupt
// step-limited so a generated loop can't hang the fuzzer. The whole run is on a
// large native stack (`run_on_big_stack`): spliced real programs can recurse
// deeply and luaur recurses natively per Lua call, so the default stack would
// abort on legal deep recursion.

use std::cell::Cell;
use std::rc::Rc;

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, Result, VmState};

fn exercise_input(data: &[u8]) {
    // Run on a large native stack: spliced real programs can recurse deeply and
    // luaur recurses natively per Lua call, so the default stack would abort on
    // legal deep recursion (see `run_on_big_stack`). The owned copy is cheap
    // (inputs are small) and lets the VM state live entirely inside the thread.
    let owned = data.to_vec();
    luaur_fuzz::run_on_big_stack(move || exercise_owned(&owned));
}

fn exercise_owned(data: &[u8]) {
    // Reserve the last 5 bytes as GC configuration; the rest drives the program.
    let split = data.len().saturating_sub(5);
    let (prog_bytes, cfg) = data.split_at(split);
    let cfg_byte = |i: usize| cfg.get(i).copied().unwrap_or(0);

    // Alloc-heavy program: the metadata-driven stdlib generator (buffers, tables,
    // strings, closures, nested values) on even config, a real spliced script on
    // odd — both allocate far more than a pure-arithmetic program, giving the GC
    // real work.
    let src = if cfg_byte(0) & 1 == 0 {
        luaur_fuzz::generate_api_call(prog_bytes)
    } else {
        luaur_fuzz::generate_spliced(prog_bytes)
    };

    let lua = Lua::new();

    // Aggressive incremental-GC tuning. `pause`->goal: 100..=199 (default 200) so
    // the collector starts early; a high multiplier + small step keeps each
    // increment short and frequent.
    let goal = 100 + (cfg_byte(1) as i32 % 100);
    let step_mul = 100 + (cfg_byte(2) as i32 * 4 % 900);
    let step_size = 1 + (cfg_byte(3) as i32 % 16);
    lua.gc_inc(goal, step_mul, step_size);

    // Optionally also force a GC step from the interrupt (the harder lever),
    // gated on a fuzzer bit and rate-limited so it doesn't dominate throughput.
    let gc_in_interrupt = cfg_byte(4) & 1 == 1;
    let gc_every = 1 + (cfg_byte(4) as u64 >> 1) % 32;

    let steps = Rc::new(Cell::new(0u64));
    let counter = steps.clone();
    let step_limit = luaur_fuzz::vm_step_limit();
    lua.set_interrupt(move |l| -> Result<VmState> {
        let c = counter.get() + 1;
        counter.set(c);
        if c > step_limit {
            return Err(luaur_rt::Error::runtime("fuzz: step limit"));
        }
        if gc_in_interrupt && c % gc_every == 0 {
            // A collector step at a VM safepoint — the aggressive interleaving.
            let _ = l.gc_step();
        }
        Ok(VmState::Continue)
    });

    if let Ok(f) = lua.load(&src).set_name("fuzz").into_function() {
        let _ = f.call::<()>(());
    }

    // Final full collection: with the run's roots gone, a mis-rooted object that
    // survived the incremental steps gets freed here — and any use-after-free it
    // causes is caught by the assertion/ASan build.
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
