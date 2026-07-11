---
phase: 7
status: complete
date: 2026-07-11
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine
  - claudine-cli
---

# Phase 7 Closeout

Phase 7 removed the final stale references to Rust-authored expression-function
descriptors and verified the authored catalog across Darkmatter, DMLS, and
Claudine consumers.

## Cleanup

- Confirmed the shared `P_*` and `R_*` constants, `FunctionRegistration`,
  `catalog_order`, registration flattening accessor, and descriptor-bearing
  registration paths are absent.
- Updated the remaining DMLS comment that named a removed return constant.
- Updated Claudine's expression-engine and drift-control documentation to
  describe the authored YAML catalog and runtime-only binding boundary.
- Confirmed the Darkmatter skill and expression authoring guide already
  document the catalog parser, accessor, and canonical-name join.
- No `AGENTS.md` update was required because workspace layout and conventions
  did not change.

## Validation

| Command | Result |
| --- | --- |
| `cargo check -p darkmatter -p darkmatter-cli -p dmls` | Passed |
| `just test` in `darkmatter/` | Passed: Darkmatter 5,455; CLI 552; DMLS 412 |
| `just test-l2` in `darkmatter/` | Passed: Darkmatter 19; CLI 69; DMLS 0 selected |
| `just lint` in `darkmatter/` | Passed |
| `just doctest` in `darkmatter/` | Passed: 181 passed, 10 ignored |
| `just check` in `darkmatter/` | Native check passed; Zed check skipped because `wasm32-wasip1` is not installed |
| `just test` in `claudine/` | Passed for all area packages |
| Five-package cross-consumer `cargo check` | Passed |

The acceptance suite covers bidirectional catalog/binding parity, alias
collisions, representative scalar/array/optional/variadic/overloaded/fallible
projections, parser invariant failures, no malformed-fixture promotion,
example evaluation, generated narrative parity, and DMLS completion/hover.
`as_csv` projects as `as_csv(list: any[]) -> string | error`. The generated
expression documentation has no diff from the catalog-backed generator.
