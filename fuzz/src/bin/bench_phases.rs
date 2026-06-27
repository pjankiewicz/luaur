//! Where do the milliseconds actually go per input? Replicates the `structured`
//! harness (generate -> check -> compile -> run under the 1M step limit) but
//! times each phase separately and records how many VM steps each input burns —
//! to see whether the step limit (not check/setup) is the throughput killer.
//!   cargo run --release --no-default-features --bin bench_phases -- 4000
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use luaur_rt::{Lua, Result, VmState};

thread_local! {
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn rng(seed: &mut u64) -> [u8; 96] {
    let mut b = [0u8; 96];
    for x in b.iter_mut() {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *x = (*seed >> 33) as u8;
    }
    b
}

fn main() {
    let iters: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    let limit: u64 = std::env::var("STEP_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);

    let (mut t_gen, mut t_chk, mut t_comp, mut t_run) = (
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
    );
    let mut hit_limit = 0u64;
    let mut ran = 0u64;
    let mut total_steps = 0u128;
    let mut seed = 0xabcd_1234_dead_0001u64;

    let t_all = Instant::now();
    for _ in 0..iters {
        let data = rng(&mut seed);

        let t = Instant::now();
        let src = luaur_fuzz::generate(&data);
        t_gen += t.elapsed();

        let t = Instant::now();
        CHECKER.with(|c| {
            let _ = c.borrow_mut().check(&src);
        });
        t_chk += t.elapsed();

        let t = Instant::now();
        let lua = Lua::new();
        let steps = Rc::new(Cell::new(0u64));
        let counter = steps.clone();
        lua.set_interrupt(move |_| -> Result<VmState> {
            let c = counter.get() + 1;
            counter.set(c);
            if c > limit {
                Err(luaur_rt::Error::runtime("limit"))
            } else {
                Ok(VmState::Continue)
            }
        });
        let f = lua.load(&src).set_name("fuzz").into_function();
        t_comp += t.elapsed();

        if let Ok(f) = f {
            let t = Instant::now();
            let _ = f.call::<()>(());
            t_run += t.elapsed();
            ran += 1;
            let used = steps.get();
            total_steps += used as u128;
            if used >= limit {
                hit_limit += 1;
            }
        }
    }
    let wall = t_all.elapsed();

    eprintln!("== {} inputs, step limit {} ==", iters, limit);
    eprintln!(
        "  throughput      {:>8.0}/s",
        iters as f64 / wall.as_secs_f64()
    );
    let pct = |d: Duration| 100.0 * d.as_secs_f64() / wall.as_secs_f64();
    eprintln!(
        "  generate        {:>6.1}%  ({:.1} us/in)",
        pct(t_gen),
        t_gen.as_secs_f64() * 1e6 / iters as f64
    );
    eprintln!(
        "  check           {:>6.1}%  ({:.1} us/in)",
        pct(t_chk),
        t_chk.as_secs_f64() * 1e6 / iters as f64
    );
    eprintln!(
        "  compile+setup   {:>6.1}%  ({:.1} us/in)",
        pct(t_comp),
        t_comp.as_secs_f64() * 1e6 / iters as f64
    );
    eprintln!(
        "  VM run          {:>6.1}%  ({:.1} us/in)",
        pct(t_run),
        t_run.as_secs_f64() * 1e6 / iters as f64
    );
    eprintln!(
        "  ran {}/{} compiled; {} hit step limit ({:.1}%); avg {} steps/run",
        ran,
        iters,
        hit_limit,
        100.0 * hit_limit as f64 / ran.max(1) as f64,
        (total_steps / ran.max(1) as u128)
    );
}
