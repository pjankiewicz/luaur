//! Shared helpers for the luaur end-to-end integration tests.
//!
//! Included via `mod common;` from each `tests/*.rs` file. Not all helpers are
//! used by every test file, so individual unused items are tolerated.
#![allow(dead_code)]

use assert_cmd::Command;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Locate one of the six shipping CLI binaries and wrap it in an
/// `assert_cmd::Command`.
///
/// `assert_cmd::Command::cargo_bin` only works for binaries belonging to the
/// *current* crate (it reads `CARGO_BIN_EXE_<name>`, which cargo sets only for
/// the package under test). Our binaries live in sibling workspace crates, so
/// we resolve their path from `CARGO_MANIFEST_DIR` (the `luaur-e2e` crate dir)
/// up to the workspace root, then into `target/<profile>/<name>`. This is also
/// robust to the project's `build-dir` relocation (final binaries stay under
/// `./target`, only intermediates move).
pub fn bin(name: &str) -> Command {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Honor the platform executable suffix: the bins are `luaur-analyze` on Unix
    // but `luaur-analyze.exe` on Windows, so a bare `name` join would never
    // `.exists()` there and every spawn test would fail to locate the binary.
    let file_name = format!("{name}{}", std::env::consts::EXE_SUFFIX);

    // Most robust source: this test executable's own location. cargo / nextest
    // run integration tests from `<target>/<profile>/deps/<test-exe>`, so two
    // parents up is the profile dir that also holds the workspace `[[bin]]`
    // outputs. Deriving from `current_exe` automatically honors a relocated
    // `CARGO_TARGET_DIR` / `build-dir` and the active profile (debug/release),
    // which the previous hard-coded `target/debug` lookup did not — that mismatch
    // failed every e2e test when the workspace built into a non-default target dir.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            candidates.push(profile_dir.join(&file_name));
        }
    }

    // Fallbacks: an explicit `CARGO_TARGET_DIR`, then `<workspace root>/target`.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/luaur-e2e -> crates -> <workspace root>
    let workspace_target = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("target"));
    let target_roots: Vec<PathBuf> = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .into_iter()
        .chain(workspace_target)
        .collect();
    for target in &target_roots {
        candidates.push(target.join("debug").join(&file_name));
        candidates.push(target.join("release").join(&file_name));
    }

    let path = candidates.iter().find(|p| p.exists()).unwrap_or_else(|| {
        panic!(
            "could not locate binary {name}; looked in {:?}. \
             Build the workspace bins first (cargo build --workspace --bins).",
            candidates
        )
    });
    Command::new(path)
}

/// Create a fresh temp dir and write `source` into `<dir>/<name>`, returning the
/// dir (which must be kept alive for the file to persist) and the full path.
pub fn write_script(name: &str, source: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).expect("create script file");
    f.write_all(source.as_bytes()).expect("write script");
    f.flush().expect("flush script");
    (dir, path)
}

/// A path inside a brand-new temp dir that does not exist on disk.
pub fn missing_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("does-not-exist.luau");
    (dir, path)
}
