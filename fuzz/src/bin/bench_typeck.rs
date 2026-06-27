//! Time `Checker::check` over the REAL evolved typeck corpus (the AFL queue) to
//! see whether typeck's low throughput is a few large/pathological inputs or
//! uniform cost. Reports size + per-input time and the slowest inputs.
//!   cargo run --release --no-default-features --bin bench_typeck -- artifacts/afl/typeck/default/queue
use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "artifacts/afl/typeck/default/queue".into());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("cannot read {dir}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && !p.to_string_lossy().contains("README"))
        .collect();
    files.sort();

    // warm the checker
    CHECKER.with(|c| {
        let _ = c.borrow_mut().check("local x = 1");
    });

    let mut rows: Vec<(f64, usize, String)> = Vec::new();
    let mut total = 0.0f64;
    let mut bytes_total = 0usize;
    for p in &files {
        let data = std::fs::read(p).unwrap_or_default();
        bytes_total += data.len();
        let Ok(src) = std::str::from_utf8(&data) else {
            continue;
        };
        let t = Instant::now();
        CHECKER.with(|c| {
            let _ = c.borrow_mut().check(src);
        });
        let ms = t.elapsed().as_secs_f64() * 1e3;
        total += ms;
        rows.push((
            ms,
            data.len(),
            p.file_name().unwrap().to_string_lossy().into_owned(),
        ));
    }

    let n = rows.len();
    eprintln!(
        "== {} inputs, {} KB total, mean {} B/input ==",
        n,
        bytes_total / 1024,
        bytes_total / n.max(1)
    );
    eprintln!(
        "  throughput  {:>8.0}/s   (mean {:.2} ms/check)",
        n as f64 / (total / 1e3),
        total / n as f64
    );

    // What if AFL capped input size? Report throughput keeping only inputs <= cap.
    for cap in [65536usize, 16384, 8192, 4096] {
        let kept: Vec<&(f64, usize, String)> = rows.iter().filter(|r| r.1 <= cap).collect();
        let t: f64 = kept.iter().map(|r| r.0).sum();
        eprintln!(
            "  cap {:>6} B: keep {:>4}/{} inputs, {:>8.0}/s  (mean {:.3} ms)",
            cap,
            kept.len(),
            n,
            kept.len() as f64 / (t / 1e3),
            t / kept.len().max(1) as f64
        );
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let slow: f64 = rows.iter().take(10).map(|r| r.0).sum();
    eprintln!(
        "  slowest 10 inputs = {:.0}% of all check time:",
        100.0 * slow / total
    );
    for (ms, sz, name) in rows.iter().take(10) {
        eprintln!("    {:>8.2} ms  {:>7} B   {}", ms, sz, name);
    }
}
