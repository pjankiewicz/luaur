# Contributing rules for this repo (luaur)

## Git / PR workflow

- **Never push code changes directly to `main`.** Every code change goes through a pull request:
  branch → open PR → CI must be **green** → only then merge.
- Use `gh pr create` to open the PR and `gh pr checks <n> --watch` to wait for CI. Do **not** merge
  until all checks pass.
- Squash-merge PRs (`gh pr merge <n> --squash --delete-branch`) to keep `main` history clean.

## CI

- CI lives in `.github/workflows/ci.yml` and runs on every `pull_request` (and pushes to `main`):
  the full workspace test suite via `cargo nextest run --workspace --locked` on Linux/macOS/Windows,
  the `luaur-rt` feature matrix (`async,serde,macros,typecheck` / `send,typecheck` /
  `send,async,serde,macros,typecheck`), doctests, `cargo fmt --all --check`, and a `wasm32` build.
- Builds use `--locked`, so keep `Cargo.lock` in sync (run `cargo metadata --locked` to verify).
- When adding a feature/capability, extend the CI feature matrix to cover it.

## Version bumps & releases

- **A version bump may skip CI**: commit it straight to `main` with a `[skip ci]` suffix in the
  message (matching the existing mechanical-commit pattern, e.g. `release: bump workspace to X [skip ci]`).
  This is the one exception to the no-direct-push rule.
- The workspace shares one version (`[workspace.package] version`, inherited via
  `version.workspace = true`); a release bumps **all** crates. Bump the workspace version and every
  internal `luaur-*` path-dep requirement together (they are all `version = "X"` strings in the
  `Cargo.toml`s), then refresh `Cargo.lock`.
- **Publishing to crates.io is irreversible — confirm the version with the user before publishing.**
  Publish in dependency order; `cargo publish --workspace` handles ordering + index waiting. The 4
  `publish = false` crates (`luaur-cli-test`, `luaur-unit-test`, `luaur-conformance`, `luaur-e2e`) and
  the `examples/*` are skipped automatically; 20 crates publish.

## Commit / PR hygiene

- Do **not** add Claude/Anthropic attribution to commits or PRs (no `Co-Authored-By: Claude …`
  trailers, no "Generated with Claude" footers).
