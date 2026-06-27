#!/usr/bin/env bash
# Replay every saved crash/hang for a target against the current binary and
# bucket by the distinct abort/panic message. Tells us how many still reproduce
# (live bugs) vs. are dead (already fixed) and how many distinct root causes.
#   TARGET=typeck_typed ./scripts/replay_crashes.sh
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${TARGET:-run}"
BIN="target/release/${TARGET}"
WD="${WD:-6}"   # per-input watchdog seconds (type-check blowups can hang)

[ -x "$BIN" ] || { echo "missing $BIN — build it first"; exit 1; }

tmp_err="$(mktemp)"
declare -a files=()
while IFS= read -r f; do files+=("$f"); done < <(
  find "artifacts/afl/${TARGET}" -type f \
    -path '*crashes*' -o -path '*hangs*' 2>/dev/null | grep -v 'README'
)

total=0; repro=0
# bucket: message -> count (use a temp file as the assoc store for portability)
buckets="$(mktemp)"
for f in "${files[@]}"; do
  [ -f "$f" ] || continue
  total=$((total+1))
  ( "$BIN" < "$f" >/dev/null 2>"$tmp_err" ) & pid=$!
  ( sleep "$WD"; kill -9 $pid 2>/dev/null ) & wd=$!
  wait $pid 2>/dev/null; rc=$?
  kill $wd 2>/dev/null; wait $wd 2>/dev/null
  if [ "$rc" -ne 0 ]; then
    repro=$((repro+1))
    msg=$(grep -iE "assert|panic|overflow|out of bounds|unreachable|left ==|right ==|getMutable|follow|index out|None\b" "$tmp_err" 2>/dev/null \
          | sed -E 's/0x[0-9a-f]+/0xADDR/g; s/:[0-9]+:[0-9]+/:L:C/g' | head -1)
    [ -z "$msg" ] && msg="<rc=$rc, no recognizable message>"
    echo "$msg" >> "$buckets"
  fi
done

echo "=== TARGET=$TARGET : replayed=$total  still-crashing=$repro ==="
echo "--- distinct root-cause buckets (count x message) ---"
sort "$buckets" | uniq -c | sort -rn
rm -f "$tmp_err" "$buckets"
