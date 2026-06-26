#!/usr/bin/env bash
# Drive an AFL (cargo-afl) fuzzing session against one target, with its live TUI.
#
#   make fuzz-typeck                  # or: TARGET=typeck ./scripts/run_afl.sh
#   TARGET=run ./scripts/run_afl.sh -V 120   # extra args pass through to afl-fuzz
#   LUAUR_FUZZ_ASAN=1 make fuzz-typeck       # AddressSanitizer build (finds leaks/UAF)
#
# Seeds come from a small CURATED set in `seeds/<target>/` (good starting inputs
# for the type checker / compiler), NOT a giant imported corpus — a foreign or
# random corpus floods AFL's calibration with "no new instrumentation" /
# "instability" warnings and buries the TUI. AFL evolves its own corpus in the
# output dir from these seeds.
set -euo pipefail

export RUSTC_WRAPPER=""

if ! command -v cargo-afl >/dev/null 2>&1; then
  echo "cargo-afl is not installed. Run: make setup-afl  (cargo install cargo-afl)" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FUZZ_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${FUZZ_DIR}"

TARGET="${TARGET:-typeck}"
IN_DIR="${IN_DIR:-seeds/${TARGET}}"
OUT_DIR="${OUT_DIR:-artifacts/afl/${TARGET}}"

# Seed the curated input dir if empty. The Lua-source targets share a handful of
# small, type-checker-exercising snippets; `number` gets numeric literals.
mkdir -p "${IN_DIR}" "${OUT_DIR}"
if [[ -z "$(ls -A "${IN_DIR}" 2>/dev/null)" ]]; then
  case "${TARGET}" in
    number)
      printf '123'        > "${IN_DIR}/dec"
      printf '0x1fap2'    > "${IN_DIR}/hex"
      printf '0b1011'     > "${IN_DIR}/bin"
      printf '3.14e10'    > "${IN_DIR}/float"
      ;;
    *)
      printf 'return 1 + 2'                                  > "${IN_DIR}/expr"
      printf 'local x: number = 1\nreturn x'                 > "${IN_DIR}/annot"
      printf 'function f(a: string): number return #a end'   > "${IN_DIR}/func"
      printf 'type T = {x: number}\nlocal v: T = {x=1}'      > "${IN_DIR}/alias"
      printf 'local t = {a=1, b="s"}\nreturn t.a'            > "${IN_DIR}/table"
      printf 'for i=1,3 do end\nwhile true do break end'     > "${IN_DIR}/control"
      ;;
  esac
fi

# Optional ASan build: set LUAUR_FUZZ_ASAN=1 to instrument with AddressSanitizer
# (finds the use-after-free / leak classes this faithful port cares about, e.g.
# the per-unification shared_seen leak). Slower, but catches memory errors a bare
# crash check would miss.
if [[ "${LUAUR_FUZZ_ASAN:-0}" == "1" ]]; then
  echo "[run_afl] ASan instrumentation enabled (AFL_USE_ASAN=1)" >&2
  export AFL_USE_ASAN=1
fi

# The type checker keys hash maps on raw pointer addresses (ASLR-randomized), so
# the same input takes slightly different code paths across runs — AFL calls this
# "instability". It's structural to the pointer-based port, not a corruption, so
# tell AFL to proceed instead of stalling calibration on it. (Output determinism
# is checked separately by the `determinism` target.)
export AFL_IGNORE_PROBLEMS="${AFL_IGNORE_PROBLEMS:-1}"
export AFL_IGNORE_SEED_PROBLEMS="${AFL_IGNORE_SEED_PROBLEMS:-1}"
# Resume an existing session in OUT_DIR instead of erroring if it's non-empty.
export AFL_AUTORESUME="${AFL_AUTORESUME:-1}"

# Per-target time limit. By default afl-fuzz runs until you Ctrl+C; set
# FUZZ_SECS=N (or `make fuzz-typeck SECS=N`) to stop after N seconds — used by the
# `fuzz-all` cycle to give each target a fair slice. Explicit `-V` in "$@" wins.
TIME_ARGS=()
if [[ -n "${FUZZ_SECS:-}" ]]; then
  TIME_ARGS=(-V "${FUZZ_SECS}")
fi

# Optimized build with the assertion layer on (see fuzz/Cargo.toml
# [profile.release]); much faster execs than debug, still LUAU_ASSERT-checked.
cargo afl build --release --bin "${TARGET}"
cargo afl fuzz -i "${IN_DIR}" -o "${OUT_DIR}" "${TIME_ARGS[@]}" "$@" "target/release/${TARGET}"
