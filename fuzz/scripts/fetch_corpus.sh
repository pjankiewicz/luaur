#!/usr/bin/env bash
# Populate the fuzz corpus with REAL Luau programs — they exercise language
# features and combinations no generator invents, so AFL starts from deep-valid
# inputs and mutates outward (the rustc-fuzzing "splice real files" idea).
#
# Sources (all permissively licensed — see fuzz/CORPUS_ATTRIBUTION.md):
#   * this repo's vendored Luau conformance suite (crates/luaur-conformance/…)
#   * the upstream Luau language repo's test scripts (luau-lang/luau, MIT)
#
# The corpus lives in fuzz/corpus/<target>/ (gitignored — not committed, no repo
# bloat); re-run this any time to refresh it. run_afl.sh prefers it over the
# small curated fuzz/seeds/ set when present.
set -euo pipefail

FUZZ_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="$(cd "${FUZZ_DIR}/.." && pwd)"
cd "${FUZZ_DIR}"

# Only the DIRECT-SOURCE targets take a Luau program as raw bytes (`from_utf8`),
# so real programs are genuine seeds for them. The GENERATOR targets
# (typeck_typed, typeck_defs, structured, determinism, roundtrip) drive a grammar
# from the bytes via `generate*()` — a .luau file is just random driver bytes
# there, NOT a meaningful seed, so staging the corpus for them does nothing. To
# leverage real scripts for those code paths, use the AST splicer (`splice`
# target / `luaur_fuzz::generate_spliced`), which parses real scripts and
# recombines their statements into new valid-ish programs.
SRC_TARGETS="compile run typeck"

stage() { # copy a .lua/.luau file into every source target's corpus, hashed name
  local f="$1"
  [ -s "$f" ] || return 0
  local h
  h="$(shasum "$f" 2>/dev/null | cut -c1-16)" || h="$(basename "$f")"
  for t in $SRC_TARGETS; do
    mkdir -p "corpus/$t"
    cp "$f" "corpus/$t/seed-$h.luau" 2>/dev/null || true
  done
}

echo ">> harvesting vendored Luau conformance suite"
count=0
while IFS= read -r f; do stage "$f"; count=$((count + 1)); done < <(
  find "${REPO_DIR}/crates/luaur-conformance" -name '*.luau' -o -name '*.lua' 2>/dev/null
)
echo "   staged ${count} vendored programs"

echo ">> downloading upstream Luau test scripts (luau-lang/luau, MIT)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
if git clone --depth 1 --filter=blob:none --sparse https://github.com/luau-lang/luau "$tmp/luau" >/dev/null 2>&1; then
  ( cd "$tmp/luau" && git sparse-checkout set tests bench >/dev/null 2>&1 || true )
  dl=0
  while IFS= read -r f; do stage "$f"; dl=$((dl + 1)); done < <(
    find "$tmp/luau" \( -name '*.luau' -o -name '*.lua' \) 2>/dev/null
  )
  echo "   staged ${dl} upstream programs"
else
  echo "   (skipped: no network / clone failed — vendored corpus still staged)"
fi

echo ">> corpus sizes:"
for t in $SRC_TARGETS; do
  printf '   %-14s %s files\n' "$t" "$(ls "corpus/$t" 2>/dev/null | wc -l | tr -d ' ')"
done
echo "Done. See fuzz/CORPUS_ATTRIBUTION.md for sources + licenses."
