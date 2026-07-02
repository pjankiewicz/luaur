#!/usr/bin/env bash
# Deploy the in-browser Luau playground (website/) to the GitHub Pages repo.
#
# The playground is served from a SEPARATE repo — pjankiewicz.github.io — under
# its `luaur/` directory (GitHub Pages auto-deploys on push). The wasm engine
# there goes stale after every luaur release, so this rebuilds it and syncs the
# code assets.
#
#   scripts/deploy-website.sh              # build wasm, smoke-test, sync, commit, push
#   scripts/deploy-website.sh --no-push    # do everything except `git push`
#   scripts/deploy-website.sh --skip-smoke # skip the browser smoke test (not advised)
#   DEPLOY_DIR=/path/to/pages/luaur scripts/deploy-website.sh   # override target
#
# NOTE on index.html: the DEPLOYED index.html is intentionally AHEAD of the repo
# source (it carries the analytics tag, content-hash cache-bust `?v=` query
# strings, and prose edits made directly on the live copy). This script therefore
# NEVER overwrites index.html — it syncs only the code assets (the wasm engine,
# app.js, the CodeMirror bundle, env.js, style.css, examples/). Edit index.html
# in the Pages repo directly, or reconcile it into website/ first.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOY_DIR="${DEPLOY_DIR:-$HOME/Projects/pjankiewicz.github.io/luaur}"
PUSH=1
SMOKE=1
for arg in "$@"; do
  case "$arg" in
    --no-push) PUSH=0 ;;
    --skip-smoke) SMOKE=0 ;;
    *) echo "unknown flag: $arg" >&2; exit 2 ;;
  esac
done

if [[ ! -d "$DEPLOY_DIR" ]]; then
  echo "deploy target not found: $DEPLOY_DIR" >&2
  echo "set DEPLOY_DIR=/path/to/pjankiewicz.github.io/luaur" >&2
  exit 1
fi

cd "$REPO_ROOT"

echo "==> building the wasm engine (release)"
cargo build -p luaur-web --target wasm32-unknown-unknown --features wasm --release
wasm-bindgen --target web --out-dir website/pkg \
  target/wasm32-unknown-unknown/release/luaur_web.wasm

if [[ "$SMOKE" == "1" ]]; then
  echo "==> smoke-testing the playground (typed run/error behavior)"
  ( cd website/test && npm test )
fi

echo "==> syncing code assets -> $DEPLOY_DIR (index.html preserved)"
# The engine is the release delta; app.js / codemirror / env.js / style.css /
# examples follow for good measure. index.html is deliberately excluded.
rsync -a --delete website/pkg/ "$DEPLOY_DIR/pkg/"
for f in app.js codemirror.bundle.js env.js style.css; do
  [[ -f "website/$f" ]] && cp "website/$f" "$DEPLOY_DIR/$f"
done
rsync -a --delete website/examples/ "$DEPLOY_DIR/examples/"

echo "==> committing + pushing the Pages repo"
version="$(grep -m1 '^version' "$REPO_ROOT/Cargo.toml" | sed -E 's/.*"(.*)".*/\1/')"
# Stage ONLY the paths this script syncs — never a blanket `git add luaur`, which
# would sweep in unrelated pre-existing edits (e.g. a hand-edited index.html
# sitting uncommitted in the Pages working tree).
git -C "$DEPLOY_DIR/.." add \
  luaur/pkg luaur/app.js luaur/codemirror.bundle.js luaur/env.js \
  luaur/style.css luaur/examples 2>/dev/null || true
if git -C "$DEPLOY_DIR/.." diff --cached --quiet; then
  echo "no changes to deploy (assets already current)"
  exit 0
fi
git -C "$DEPLOY_DIR/.." commit -q -m "luaur playground: engine ${version} (wasm rebuild after release)"
if [[ "$PUSH" == "1" ]]; then
  git -C "$DEPLOY_DIR/.." push
  echo "==> deployed. GitHub Pages will publish shortly."
else
  echo "==> committed (not pushed; --no-push). Push manually to publish."
fi
