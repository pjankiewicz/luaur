# Publishing to crates.io

luaur is published as independent crates so consumers can depend on exactly the layer they
need (`luaur-vm` alone, `luaur-analysis` alone, etc.). crates.io requires that every
dependency already exist on the registry at publish time, so crates **must be published in
dependency order** (a topological sort of the workspace graph).

## Prerequisites

- Each crate's path dependencies carry an explicit `version` (crates.io ignores `path` but
  requires `version`). This is set via `[workspace.package]` + `version.workspace = true`.
- Test/harness crates are marked `publish = false` and are **not** released:
  `luaur-unit-test`, `luaur-cli-test`, `luaur-conformance`.
- The `luaur-*` names are free on crates.io and must be owned by the publishing account.
- `cargo publish` runs a verification build per crate; budget time for ~17 builds.

## Publish order

Publish top-to-bottom; each layer only depends on layers above it.

```
# Layer 0 — foundations
1.  luaur-common

# Layer 1 — depend only on common
2.  luaur-ast
3.  luaur-bytecode
4.  luaur-vm

# Layer 2
5.  luaur-compiler      (ast, bytecode, common)
6.  luaur-code-gen      (common, vm)

# Layer 3
7.  luaur-config        (ast, bytecode, compiler, vm, common)

# Layer 4
8.  luaur-analysis      (ast, bytecode, compiler, config, vm, common)
9.  luaur-require       (+ config)
10. luaur-cli-lib       (+ config)
11. luaur-rt            (vm, compiler, common — the mlua-style API; also luaur-config + luaur-analysis, optional, only under the `typecheck` feature — both already published above at 7 and 8)

# Layer 5 — umbrella + leaves (wasm + CLIs)
12. luaur               (umbrella: re-exports every lib + luaur-rt — publish after them all)
13. luaur-web           (analysis, ...)
14. luaur-ast-cli
15. luaur-analyze-cli
16. luaur-bytecode-cli
17. luaur-compile-cli
18. luaur-reduce-cli
19. luaur-repl-cli
```

## Recommended dry run

```sh
# Verify every publishable crate packages cleanly, in order, without uploading:
for c in luaur-common luaur-ast luaur-bytecode luaur-vm luaur-compiler luaur-code-gen \
         luaur-config luaur-analysis luaur-require luaur-cli-lib luaur-rt luaur luaur-web \
         luaur-ast-cli luaur-analyze-cli luaur-bytecode-cli luaur-compile-cli \
         luaur-reduce-cli luaur-repl-cli; do
  cargo publish -p "$c" --dry-run || { echo "FAILED: $c"; break; }
done
```

Then drop `--dry-run` and run the same loop to release. If a later crate fails after
earlier ones uploaded, fix and resume from the failed crate (already-published versions are
immutable — bump the version if a re-publish is needed).
