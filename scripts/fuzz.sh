#!/usr/bin/env bash
# Standalone (toolchain-free) fuzz runner for luaur — for quick local smoke
# checks. Builds a target with --no-default-features (NO AFL / nightly / ASan)
# and runs it over LUAUR_FUZZ_ITERS pseudo-random inputs. A panic prints a
# reproducible hex dump and exits non-zero.
#
# For real, coverage-guided fuzzing with AFL's live TUI use the Makefile:
#   make setup-afl && make fuzz-typeck      (see fuzz/README.md)
#
# Usage:
#   scripts/fuzz.sh [target] [iters]
#   scripts/fuzz.sh                 # default: typeck, 20000 iterations
#   scripts/fuzz.sh compile 50000   # 50k random inputs at the compiler
#   scripts/fuzz.sh all 5000        # every target, 5k inputs each
#   scripts/fuzz.sh list            # list available targets
#
# Env: LUAUR_FUZZ_SEED overrides the PRNG seed (default 305419896).
set -euo pipefail

TARGETS="compile run typeck typeck_defs number structured determinism api gcstress host serde_roundtrip"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

target="${1:-typeck}"
iters="${2:-20000}"
seed="${LUAUR_FUZZ_SEED:-305419896}"

if [ "$target" = "list" ]; then
  echo "fuzz targets: $TARGETS"
  exit 0
fi

run_one() {
  local t="$1"
  echo ">>> standalone fuzzing '$t' for ${iters} inputs (seed: ${seed})"
  ( cd "$REPO_ROOT/fuzz" && \
    cargo build --no-default-features --bin "$t" && \
    LUAUR_FUZZ_ITERS="$iters" LUAUR_FUZZ_SEED="$seed" "./target/debug/$t" )
}

if [ "$target" = "all" ]; then
  for t in $TARGETS; do run_one "$t"; done
else
  case " $TARGETS " in
    *" $target "*) run_one "$target" ;;
    *) echo "unknown target '$target' (try: scripts/fuzz.sh list)" >&2; exit 1 ;;
  esac
fi
