//! Micro-bench for `generate_spliced`: run it over many pseudo-random byte
//! inputs and report throughput. Used to measure the seed-slice memoization.
//!   cargo run --release --no-default-features --bin bench_spliced -- 50000
use std::time::Instant;

fn main() {
    let iters: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(50_000);

    // Deterministic LCG so before/after runs see identical inputs.
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut buf = [0u8; 64];
    let mut sink: usize = 0; // keep the result observable so it isn't optimized away

    let t0 = Instant::now();
    for _ in 0..iters {
        for b in buf.iter_mut() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (state >> 33) as u8;
        }
        let s = luaur_fuzz::generate_spliced(&buf);
        sink = sink.wrapping_add(s.len());
    }
    let dt = t0.elapsed();
    eprintln!(
        "generate_spliced: {} iters in {:.3}s = {:.0} calls/s  (checksum {})",
        iters,
        dt.as_secs_f64(),
        iters as f64 / dt.as_secs_f64(),
        sink
    );
}
