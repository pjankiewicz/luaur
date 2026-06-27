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
OUT_DIR="${OUT_DIR:-artifacts/afl/${TARGET}}"

# Prune empty resume-marker archives. On every AFL_AUTORESUME, afl-fuzz rotates the
# crashes/ and hangs/ dirs into `crashes.<timestamp>/` / `hangs.<timestamp>/` — even
# when they hold nothing but AFL's own README.txt. Across a long fuzz-loop these
# README-only dirs pile up and masquerade as "new crash archives" in triage scans.
# Drop the ones with no actual crash/hang files; keep any with real findings.
for _arch in "${OUT_DIR}"/*/crashes.* "${OUT_DIR}"/crashes.* "${OUT_DIR}"/*/hangs.* "${OUT_DIR}"/hangs.*; do
  [[ -d "${_arch}" ]] || continue
  if [[ -z "$(find "${_arch}" -type f ! -name 'README*' 2>/dev/null | head -1)" ]]; then
    rm -rf "${_arch}"
  fi
done

# Prefer the real-program corpus (fuzz/scripts/fetch_corpus.sh) when it's been
# populated — it's a far richer starting set than the tiny curated seeds. Fall
# back to the committed curated seeds otherwise.
if [[ -z "${IN_DIR:-}" ]]; then
  if [[ -d "corpus/${TARGET}" && -n "$(ls -A "corpus/${TARGET}" 2>/dev/null)" ]]; then
    IN_DIR="corpus/${TARGET}"
  else
    IN_DIR="seeds/${TARGET}"
  fi
fi

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

# Cap input size (-G) for the DIRECT-SOURCE targets (compile/run/typeck — the raw
# bytes ARE the program). Their corpora carry a few large real scripts (up to
# 460KB) and AFL grows ~14KB mutation clusters that each take 6-10ms to
# parse+check, dominating throughput — while the MEDIAN input is <500B and
# coverage has plateaued, so the giants add ~no coverage. Measured: typeck
# check throughput 1680/s -> 6200/s at an 8KB cap (3.7x), ~zero coverage loss.
# Generator targets are NOT capped — their bytes drive a bounded grammar, not raw
# source, so input length maps to program complexity, not parse cost. Override
# FUZZ_MAXLEN=0 to disable, or =N for a different byte cap.
MAXLEN_ARGS=()
case "${TARGET}" in
  compile|run|typeck)
    _maxlen="${FUZZ_MAXLEN:-8192}"
    [[ "${_maxlen}" != "0" ]] && MAXLEN_ARGS=(-G "${_maxlen}")
    ;;
esac

# Optimized build with the assertion layer on (see fuzz/Cargo.toml
# [profile.release]); much faster execs than debug, still LUAU_ASSERT-checked.
cargo afl build --release --bin "${TARGET}"

# Minimize the seed corpus once (cached in corpus-cmin/<target>) before fuzzing.
# A rich corpus (fetch_corpus + gen_corpus) has many seeds that add no new
# coverage; AFL floods the dry run with "No new instrumentation output, test case
# may be useless" warnings on each. `cmin` keeps only the coverage-distinct
# inputs, so the dry run is clean (and the fuzzer starts from a leaner set).
#
# cmin runs SINGLE-WORKER with the AFL forkserver DISABLED — both are mandatory on
# macOS:
#   * AFL_NO_FORKSRV=1: with the forkserver on, afl-showmap's batch execs hang on
#     macOS — every seed times out (~5s) and records an EMPTY coverage map, so cmin
#     "narrows down to 0 files" and nukes the corpus. Disabling the forkserver runs
#     each seed fresh (like cmin's own reference exec, which always worked) — it's
#     both correct AND ~70x faster (120 seeds: 100s+/garbage -> ~4s/118 kept).
#   * single-worker: parallel mode (-T) stays broken even with the forkserver off
#     (it hangs / never returns on macOS). No -T.
# Cost is (#seeds x exec_time) at ~0.03-0.07s/seed: seconds for the 250-seed
# targets, ~2min for compile (4162). ONE-TIME and cached in corpus-cmin/<target> —
# every later launch reuses it and starts instantly. Progress streams to the
# terminal. -t (default 5000ms) is just a safety cap now that execs are fast. Skip
# with AFL_NO_CMIN=1; delete corpus-cmin/<target> to refresh; tune with CMIN_T=<ms>.
if [[ "${IN_DIR}" == corpus/* && "${AFL_NO_CMIN:-0}" != "1" ]]; then
  CMIN_DIR="corpus-cmin/${TARGET}"
  if [[ ! -d "${CMIN_DIR}" || -z "$(ls -A "${CMIN_DIR}" 2>/dev/null)" ]]; then
    SEED_COUNT="$(ls -A "${IN_DIR}" 2>/dev/null | wc -l | tr -d ' ')"
    echo "[run_afl] minimizing ${SEED_COUNT}-seed corpus ${IN_DIR} -> ${CMIN_DIR} (one-time, cached) ..." >&2
    mkdir -p "${CMIN_DIR}"
    AFL_NO_FORKSRV=1 AFL_NO_UI=1 cargo afl cmin -t "${CMIN_T:-5000}" \
      -i "${IN_DIR}" -o "${CMIN_DIR}" -- "target/release/${TARGET}" || true
    # If cmin still came back empty, drop the empty dir so we fall back to the raw
    # corpus AND don't waste time re-running cmin on the next launch.
    [[ -z "$(ls -A "${CMIN_DIR}" 2>/dev/null)" ]] && { rmdir "${CMIN_DIR}" 2>/dev/null || true; echo "[run_afl] cmin produced 0 files; using raw corpus ${IN_DIR}" >&2; }
  fi
  [[ -d "${CMIN_DIR}" && -n "$(ls -A "${CMIN_DIR}" 2>/dev/null)" ]] && IN_DIR="${CMIN_DIR}"
fi

cargo afl fuzz -i "${IN_DIR}" -o "${OUT_DIR}" "${TIME_ARGS[@]}" "${MAXLEN_ARGS[@]}" "$@" "target/release/${TARGET}"
