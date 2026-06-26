#!/usr/bin/env bash
# Local fuzz runner for luaur.
#
# Real fuzzing is a long soak with an accumulating corpus — run this for minutes
# or hours, not seconds. The corpus persists in fuzz/corpus/<target> and grows
# across runs, so each session builds on the last. (CI does NOT gate on fuzzing;
# the nightly soak lives in .github/workflows/fuzz.yml.)
#
# Usage:
#   scripts/fuzz.sh [target] [seconds]
#   scripts/fuzz.sh                 # default: typeck for 60s
#   scripts/fuzz.sh run 300         # run target for 5 minutes
#   scripts/fuzz.sh all 600         # every target, 10 minutes each
#   scripts/fuzz.sh list            # list available targets
#
# Needs a nightly toolchain and cargo-fuzz:  cargo +nightly install cargo-fuzz
set -euo pipefail

TARGETS="compile run typeck typeck_defs number structured determinism"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

target="${1:-typeck}"
secs="${2:-60}"

if [ "$target" = "list" ]; then
  echo "fuzz targets: $TARGETS"
  exit 0
fi

# cargo-fuzz needs an explicit host triple so the ASan build links a dynamic
# libc (its own default triple can be a static-musl one ASan rejects).
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)   triple="aarch64-apple-darwin" ;;
  Darwin-x86_64)  triple="x86_64-apple-darwin" ;;
  Linux-x86_64)   triple="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64)  triple="aarch64-unknown-linux-gnu" ;;
  *)              triple="" ;;
esac

if ! command -v cargo-fuzz >/dev/null 2>&1 && ! cargo +nightly fuzz --version >/dev/null 2>&1; then
  echo "cargo-fuzz not found. Install it with:  cargo +nightly install cargo-fuzz" >&2
  exit 1
fi

run_one() {
  local t="$1"
  echo ">>> fuzzing '$t' for ${secs}s  (triple: ${triple:-default}, corpus: fuzz/corpus/$t)"
  local args=()
  [ -n "$triple" ] && args=(--target "$triple")
  ( cd "$REPO_ROOT/fuzz" && \
    cargo +nightly fuzz run "$t" "${args[@]}" -- \
      -max_total_time="$secs" -rss_limit_mb=4096 -print_final_stats=1 )
}

if [ "$target" = "all" ]; then
  for t in $TARGETS; do run_one "$t"; done
else
  case " $TARGETS " in
    *" $target "*) run_one "$target" ;;
    *) echo "unknown target '$target' (try: scripts/fuzz.sh list)" >&2; exit 1 ;;
  esac
fi
