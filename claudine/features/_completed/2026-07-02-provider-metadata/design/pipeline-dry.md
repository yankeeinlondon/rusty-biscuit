# Supplemental Design: Execution-Pipeline DRY & the CLI Drift Guard

> **Status:** draft for Ken's review. Refines spec.md "DRY / Module Cogency
> Workstreams" items 3–4 and adds the wrapper↔composition unification the spec omits.
> Ratified input: F5 (pipeline unification as workstream 0).

## Workstream 0 — shared prep stages (F5, ratified)

Verified duplication between the wrapper pipeline
(`wrap/mod.rs` → `wrapper_stages.rs` → `wrapper_exec.rs`/`harness_orch`) and the
composition pipeline (`composition/mod.rs`, ~2050 lines with its own prep):

| Duplicated concern | Sites today |
| --- | --- |
| OpenCode model resolution | `wrapper_stages.rs:96` AND `composition/mod.rs:974` |
| Shadow-home env construction | wrapper env plan AND re-derived at `composition/mod.rs:857` |
| Codex structured-output prep/rendering | `wrapper_stages.rs:363`, composition, AND `harness_orch/attempt.rs` (×3) |

Ruling: extract these into provider-neutral **shared stage functions** in a
`cli/src/commands/exec_prep/` module (name bikesheddable), consumed by both pipelines.
Composition already cherry-picks one wrapper stage
(`apply_opencode_yolo_config_overlay`) — that becomes the norm, not the exception.
The Codex output case additionally becomes the `FinalMessage` renderable component
(design/render-components.md); the stage function's job shrinks to invoking it.

This lands **before** WrapperProfile migration: every catalog migration and guard
allow-list entry otherwise pays double.

## The mechanical dispatch inventory (deliverable, not prose)

The topic doc's decentralized-dispatch tables are already stale (dead line refs into
the pre-refactor `wrap/mod.rs`; "296+" vs 515 actual `Provider::` refs). Prose
inventories rot; the replacement is a **regenerable inventory**: a script/test target
that scans the CLI crate for the full pattern set (below) and emits a machine-readable
site list (path, line, pattern form, provider(s) named). The committed inventory file
is diff-reviewed like codegen; the topic doc's tables are replaced by a pointer to it.
This inventory is a prerequisite for both the guard's allow-list seeding and the
WrapperProfile disposition table.

## CLI drift guard (workstream 4)

- **Pattern set (extended):** the lib guard's three forms PLUS `matches!(provider, …)`
  and `provider == Provider::…` / `!=` — the dominant CLI forms the current guard
  cannot see. Back-port the extended set to the lib guard in the same change.
- **Blanket exemptions:** `commands/wrap/profile/*.rs` (the 7 per-provider impl files —
  legitimate dispatch by design), the clap Provider mapping in `main.rs`, and test
  paths.
- **Seeding policy: grandfather-with-burn-down.** The initial allow-list is the
  mechanical inventory at guard-landing time, but every grandfathered entry carries a
  workstream tag (`ws0-prep`, `ws3-profile`, `render`, `keep`). The guard fails on any
  site not in the list — new decentralized dispatch cannot regrow — and a companion
  report renders the burn-down by tag. `keep` entries require a `reason:`.

## WrapperProfile end-state (workstream 3)

Reality check: ~32 trait methods with defaults, **57** override fns across 7 impl
files (codex 10, gemini 13, qwen 10, opencode 9, kimi 6, claude 5, goose 4) — not the
spec's "~14". RooCode has no profile (`profile/mod.rs:611` → `None`), which is the
roster-ahead-of-code norm, not a bug.

- **Disposition table:** generated from the mechanical inventory + the data/behavior
  litmus test (design/catalog-generation.md, OQ5 ruling): each of the 57 overrides is
  classified `catalog-data` (migrates to a generated field; override deleted) or
  `behavior` (stays). The spec's table A and the topic doc's method table are both
  superseded by this one generated table — resolving the two-tables tension.
- **Success criterion:** static-fact overrides reach **zero**; the trait survives as a
  CLI-side behavior shim whose remaining impls are genuinely behavioral (prompt
  delivery mechanics, wire-RPC quirks). Defaults derive from catalog fields.
- RooCode/`None`: unchanged — profiles exist only for code-supported providers.

## Sequencing

```
mechanical inventory → workstream 0 (shared stages) → profile disposition table
       → per-method migration (with catalog fields landing, Phase 2)
       → CLI guard lands LAST on the shrunken inventory (locks in the wins — spec Phase 4 agrees)
```
