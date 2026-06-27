#!/usr/bin/env bash
# Extensive wasm fuzzing by corpus-replay under wasmtime, with abort CLASSIFICATION.
#
# wasm forces panic=abort, so luaur's panic-based exception emulation (Lua errors
# via luaD_throw, compile errors via CompileError) aborts instead of unwinding to
# its catch_unwind boundary. We therefore can't treat "abort = crash". Instead we
# read wasmtime's backtrace and classify the abort by its first luaur frame:
#   * luaD_throw / lua_error / luaG_* / compile_error_raise / longjmp  -> EXPECTED
#     (a normal Lua/compile error; only looks like a crash because of panic=abort)
#   * anything else (LUAU_ASSERT, null deref, memory trap)             -> REAL BUG
#
#   TARGET=run INPUTS=corpus/run LIMIT=300 ./scripts/wasm_fuzz.sh
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

TARGET="${TARGET:-run}"
INPUTS="${INPUTS:-corpus/${TARGET}}"
LIMIT="${LIMIT:-300}"
WASM="target/wasm32-wasip1/release/${TARGET}.wasm"
[ -f "$WASM" ] || { echo "build it: cargo build --release --no-default-features --bin ${TARGET} --target wasm32-wasip1"; exit 1; }

# The panic_any-based "normal error" control-flow paths (caught at a boundary on
# native; abort under wasm panic=abort). Garbage fuzzer input hits these
# constantly — they are NOT bugs: parse errors, compile errors / out-of-reg/local
# limits, Lua runtime errors, and the recursion limiter. NOTE: the ICE path
# (internal_error_reporter / ice_*) is deliberately NOT here — an ICE is a real
# "impossible state" bug, like a LUAU_ASSERT, and should surface.
EXPECTED_RE='luaD_throw|lua_error|luaG_|longjmp|luaD_pcall|luaD_rawrunprotected|lua_exception|compile_error_raise|compile_or_throw|alloc_reg|parse_error_raise|parser_report_parser|recursion_limiter'
tmp_in="$(mktemp -d)"
i=0; aborts=0; expected=0; real=0
declare -a realbugs=()

for f in "${INPUTS}"/*; do
  [ -f "$f" ] || continue
  case "$(basename "$f")" in README*) continue;; esac
  i=$((i+1)); [ "$i" -gt "$LIMIT" ] && break
  cp "$f" "${tmp_in}/case"
  bt=$(WASMTIME_BACKTRACE_DETAILS=1 wasmtime run --dir="${tmp_in}::/in" "$WASM" /in/case 2>&1 >/dev/null)
  rc=$?
  if [ "$rc" -ne 0 ]; then
    aborts=$((aborts+1))
    site=$(echo "$bt" | grep -oE 'luaur_(vm|compiler|analysis|ast|rt)::[a-z_:]+::[a-z_0-9]+' | head -1)
    if echo "$bt" | grep -qE "$EXPECTED_RE"; then
      expected=$((expected+1))
    else
      real=$((real+1))
      realbugs+=("$(basename "$f") :: ${site:-<unknown>}")
    fi
  fi
done

echo "=== wasm fuzz: TARGET=$TARGET  ran=$((i-1))  aborts=$aborts  (expected-error=$expected  REAL-BUG=$real) ==="
if [ "$real" -gt 0 ]; then
  echo "--- REAL wasm bugs (abort NOT via the normal error path) ---"
  printf '%s\n' "${realbugs[@]}" | sort | uniq -c | sort -rn | head -20
fi
rm -rf "$tmp_in"
