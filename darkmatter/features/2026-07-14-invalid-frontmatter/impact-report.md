---
phase: 1
created: 2026-07-18
artifact: impact-report
---

# Phase 1 Impact Report — GitNexus Upstream Analysis

Index state: refreshed 2026-07-18 against the working tree (`--force` full
re-index, 134,670 nodes / 267,464 edges / 300 flows). An earlier incremental
attempt failed with `Failed calling LOWER: Invalid UTF-8`; the full rebuild
succeeded. Every graph result below was additionally verified against current
source with `rg` because the graph undercounts Rust enum-variant and
re-export usage.

Risk policy from the plan: **stop and warn before editing any HIGH or CRITICAL
target**. Phase 1 edits no production symbols. The HIGH/CRITICAL findings are
recorded here so Phases 2–6 treat those symbols as constrained surfaces.

## Symbol Results

| Symbol | Location | Graph risk | Direct (graph) | Verified consumers (source) |
|--------|----------|-----------|----------------|------------------------------|
| `Yaml` (struct) | `biscuit-file/lib/src/yaml/types.rs:209` | HIGH (9 impacted) | 5 (all `claudine/gen`) | `biscuit-file` cli, `claudine` cli/lib/gen, `biscuit-terminal` discovery, `research/lib`, `schematic/gen`, `darkmatter` (`schemas/format.rs`, `compose/schema_validation.rs`), fuzz + benches |
| `YamlSource` (enum) | `biscuit-file/lib/src/yaml/types.rs:9` | LOW (0 upstream) | 0 | Re-exported crate-root; matched in `biscuit-file` tests and consumer pattern matches |
| `YamlError` (enum) | `biscuit-file/lib/src/yaml/types.rs:20` | LOW (0 upstream) | 0 | `claudine/lib/src/composition/error/mod.rs` embeds it twice via `#[from]` — **Display text is a load-bearing contract**; `biscuit-file` cli |
| `SourceSpan` (darkmatter alias) | `darkmatter/lib/src/markdown/span.rs:9` | LOW (0 upstream) | 0 | 25 files in the darkmatter package area; re-exported from `darkmatter` crate root (`lib.rs:95`) |
| `SourceSpan` (renderable struct) | `renderable/src/tree/source.rs:138` | LOW (22 impacted) | 4 | Distinct type from darkmatter's alias — the two must not be conflated by the shared-vocabulary work |
| `EffectiveSchema` | `darkmatter/lib/src/markdown/schemas/mod.rs:621` | MEDIUM (85 impacted) | 1 | compose schema validation, `md schema validate`, DMLS overlay |
| `validate_with_positions` | `darkmatter/lib/src/markdown/schemas/mod.rs:700` | HIGH (47 impacted) | 3 | `validate`, `validate_with_options`, compose pipeline stage |
| `extract_frontmatter_block` | `darkmatter/lib/src/markdown/frontmatter.rs:379` | **CRITICAL (90 impacted)** | 11 | darkmatter lib facade + 6 DMLS files (`overlay/*`, `graph/substrate.rs`, `providers/{folding,dsl}.rs`, `source_map/region.rs`) |
| `run_clean` | `darkmatter/cli/src/commands/clean.rs:28` | LOW (3 impacted) | 2 | `run_subcommand` (`commands/mod.rs:77`), top-level `--save` dispatch (`main.rs:149`) |
| `apply_cleanup` (cli) | `darkmatter/cli/src/commands/clean.rs:83` | LOW (4 impacted) | 1 | `run_clean` only (the same-named `toc_linking::apply_cleanup` is unrelated) |
| Clean CLI variant | `darkmatter/cli/src/args/command.rs:40` | LOW | — | Constructed by clap; destructured in `commands/mod.rs:67` |
| Top-level `--save` dispatch | `darkmatter/cli/src/main.rs:149` | LOW | — | Calls `run_clean` with fixed defaults (no indent, Normal spacing, no fixed-width) |

## Constraints Imposed on Later Phases

1. **`extract_frontmatter_block` is CRITICAL and is consumed read-only.** Its
   signature and span semantics do not change; the clean pipeline uses it as
   the sole frontmatter boundary authority (plan constraint ratified).
2. **`validate_with_positions` is HIGH and its coercion behavior is frozen.**
   The schema-proven-quoting work adds a *new* raw/non-coercing query beside
   it rather than altering the existing entry points (plan constraint).
3. **`Yaml` is HIGH.** Changes are purely additive: new private retained-source
   field (populated by existing constructors), new methods. No existing
   constructor, method signature, or `YamlSource`/`YamlError` variant changes.
4. **`YamlError` Display text is frozen.** `claudine` wraps it with `#[from]`
   in two public error enums; the structured-location accessor (Phase 2) is
   additive and must not alter rendered messages (Phase 2 checkpoint already
   requires byte-identical display snapshots).
5. **Two different `SourceSpan` types exist** (darkmatter alias
   `Range<usize>` vs renderable struct). The Phase 2 shared vocabulary lives
   in `biscuit-file`; darkmatter keeps its public import path via re-export or
   lossless conversion. The renderable struct is out of scope.
6. **`run_clean` / `apply_cleanup` / CLI variant / `--save` dispatch are LOW**
   — the clean-command integration surface is narrow (two call sites), so the
   Phase 6 rewiring is contained.
