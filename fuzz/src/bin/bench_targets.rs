//! Quantify the per-input fixed costs shared across fuzz targets, to see which
//! slow targets have *accidental* (cacheable) overhead vs *inherent* work.
//!   cargo run --release --no-default-features --bin bench_targets
use std::time::Instant;

fn bench<F: FnMut()>(name: &str, iters: u64, mut f: F) {
    // warm
    for _ in 0..(iters / 20).max(1) {
        f();
    }
    let t0 = Instant::now();
    for _ in 0..iters {
        f();
    }
    let dt = t0.elapsed();
    eprintln!(
        "  {:<26} {:>9.0}/s   ({:.1} us/call)",
        name,
        iters as f64 / dt.as_secs_f64(),
        dt.as_secs_f64() * 1e6 / iters as f64
    );
}

fn rng(seed: &mut u64) -> [u8; 64] {
    let mut b = [0u8; 64];
    for x in b.iter_mut() {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *x = (*seed >> 33) as u8;
    }
    b
}

fn main() {
    eprintln!("== fixed per-input costs (the registration / setup steps) ==");
    bench("Lua::new()", 20_000, || {
        let _l = luaur_rt::Lua::new();
    });
    bench("Checker::new()", 2_000, || {
        let _c = luaur_rt::Checker::new();
    });

    eprintln!("== generators (from-scratch; should have no parse cost) ==");
    let mut s = 0xfeed_face_dead_beefu64;
    bench("generate", 50_000, || {
        let b = rng(&mut s);
        let _ = luaur_fuzz::generate(&b);
    });
    bench("generate_typed", 50_000, || {
        let b = rng(&mut s);
        let _ = luaur_fuzz::generate_typed(&b);
    });
    bench("generate_computational", 50_000, || {
        let b = rng(&mut s);
        let _ = luaur_fuzz::generate_computational(&b);
    });
    bench("generate_definitions", 50_000, || {
        let b = rng(&mut s);
        let _ = luaur_fuzz::generate_definitions(&b);
    });
    bench("generate_spliced (memoized)", 50_000, || {
        let b = rng(&mut s);
        let _ = luaur_fuzz::generate_spliced(&b);
    });
}
