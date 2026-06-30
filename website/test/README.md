# Playground smoke test

A headless-browser smoke test for the in-browser playground (`website/`). It
boots the real static site in Chromium and asserts the **typed** run/error
behavior end-to-end — the wasm engine, the runtime-error panic→JS bridge, and
`app.js`'s classification — so a regression in any of those layers fails loudly.

## What it checks

- `run()` returns a typed `{ output, error }`; iterating `_G` (which contains a
  global literally named `error`) is classified **OK** from the `error` field,
  never by scanning the output text.
- A compile error populates the `error` field (no trap).
- A *runtime* error traps as a `WebAssembly.RuntimeError` and its message is
  bridged to JS via `globalThis.__luaurOnRuntimeError`.
- In the real UI: the `globals` example runs **out-ok** (the original bug
  report — it used to paint red); a runtime error shows **out-err** with the
  real message; the engine recovers after the trap so the next run works.

## Run it

```sh
cd website/test
npm install                      # installs playwright
npx playwright install chromium  # one-time browser download
npm test
```

The test serves `website/` itself (tiny built-in static server, no config), so
it needs **a fresh `website/pkg/`** — rebuild the wasm first if the engine
changed:

```sh
# from the repo root
cargo build -p luaur-web --target wasm32-unknown-unknown --features wasm --release
wasm-bindgen --target web --out-dir website/pkg \
  target/wasm32-unknown-unknown/release/luaur_web.wasm
```

This is a **local** smoke test (it needs a wasm build + a browser), not a CI
gate — run it before deploying the playground after a release.
