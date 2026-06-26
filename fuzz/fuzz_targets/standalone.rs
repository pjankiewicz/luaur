// Standalone fuzz driver used when the `afl-runtime` feature is OFF, so the
// targets build and run WITHOUT the AFL toolchain. `include!`d into each target.
//
// Input sources (priority order):
//   * file path args            -> run each file (corpus replay / crash repro)
//   * LUAUR_FUZZ_STDIN=1        -> read a single input from stdin
//   * otherwise                 -> generate LUAUR_FUZZ_ITERS (default 50000)
//                                  pseudo-random inputs seeded by LUAUR_FUZZ_SEED
//
// A panic in `exercise` is caught, reported with a reproducible hex dump of the
// offending input, and the process exits non-zero (so it fails like a test).

#[allow(dead_code)]
fn sa_env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[allow(dead_code)]
fn sa_hex(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for b in data.iter().take(512) {
        s.push_str(&format!("{b:02x}"));
    }
    if data.len() > 512 {
        s.push_str("...");
    }
    s
}

#[allow(dead_code)]
fn standalone_main(exercise: fn(&[u8])) {
    use std::io::Read;

    // Fully-qualified so this driver compiles even when the including target has
    // a `use luaur_rt::Result` in scope (e.g. `run`, `structured`).
    let run = |data: &[u8]| -> std::result::Result<(), String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| exercise(data))).map_err(|p| {
            p.downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic>".to_string())
        })
    };
    let fail = |data: &[u8], msg: &str| -> ! {
        eprintln!("FUZZ PANIC: {msg}");
        eprintln!("input ({} bytes): {}", data.len(), sa_hex(data));
        std::process::exit(1);
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        for path in &args {
            let data = std::fs::read(path).unwrap_or_default();
            if let Err(msg) = run(&data) {
                fail(&data, &msg);
            }
        }
        eprintln!("standalone fuzz OK: replayed {} file(s), no panic", args.len());
        return;
    }

    if std::env::var("LUAUR_FUZZ_STDIN").is_ok() {
        let mut data = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut data);
        if let Err(msg) = run(&data) {
            fail(&data, &msg);
        }
        eprintln!("standalone fuzz OK: stdin input, no panic");
        return;
    }

    let iters = sa_env_u64("LUAUR_FUZZ_ITERS", 50_000);
    let mut seed = sa_env_u64("LUAUR_FUZZ_SEED", 0x1234_5678_9ABC_DEF0).max(1);
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..iters {
        let len = (next() % 768) as usize;
        let mut buf = vec![0u8; len];
        for b in buf.iter_mut() {
            *b = (next() & 0xff) as u8;
        }
        if let Err(msg) = run(&buf) {
            fail(&buf, &msg);
        }
    }
    eprintln!("standalone fuzz OK: {iters} random inputs, no panic");
}
