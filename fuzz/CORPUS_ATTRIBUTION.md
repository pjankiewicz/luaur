# Fuzz corpus attribution

The fuzz corpus (`fuzz/corpus/<target>/`, gitignored) is seeded with **real Luau
programs** harvested by `fuzz/scripts/fetch_corpus.sh`. Those programs are NOT
committed to this repository; the script downloads / copies them on demand. They
are used only as fuzzing seed inputs. Their copyright and licenses remain with
their respective authors, reproduced/attributed here:

## Luau language test suite — `luau-lang/luau`
- Source: https://github.com/luau-lang/luau (`tests/`, `bench/`)
- License: **MIT** — Copyright © Roblox Corporation and the Luau contributors.
- Also vendored in this repo under `crates/luaur-conformance/conformance/` (the
  ported conformance suite), which `fetch_corpus.sh` likewise stages.

The MIT license permits use, copying, and modification with attribution; using
these scripts as fuzzing inputs is well within it. If you add more corpus
sources to `fetch_corpus.sh`, list them here with their source URL and license
(only **MIT / Apache-2.0 / BSD / public-domain** sources, with attribution).
