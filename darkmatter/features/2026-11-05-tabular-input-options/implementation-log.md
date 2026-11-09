---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-tabular-input-options/spec.md"
plan: "darkmatter/features/2026-11-05-tabular-input-options/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
---

# Implementation Log for 2026-11-05-tabular-input-options (6 phases)

## Phase 1

### Rebase check

`2026-11-05-dynamic-schema-types` is closed and committed (`cc54ccea0`, moved
to completed); `git status` shows no uncommitted change under the data module,
`functions/`, `compose/remote.rs`, or `schemas/slots/`. (The working tree
holds unrelated uncommitted edits from another session under `claudine/`,
`dmls/docs/editors/README.md`, `docs/topics/schemas/dmls-schema-support.md`,
and `features/2026-11-05-yaml-locator-hardening/`; this phase does not touch
them.)

- **A1 (N17): holds, with a shape difference (revised R9, R10).** The log is
  `DataRuntime::observations` (`Observation { entry, kind, hash }`,
  `data/runtime.rs:112`), `read_mark()` / `observations_since()`
  (`runtime.rs:587,592`). What the memo entry stores is not an `Observation`
  but `LoadedSource::hash: [u8; 32]`; `load.rs` re-records it on every call
  with `ctx.data.observe_read(&identity, hash)` after `load_source`
  (`hash: None` for a memoized failure), and observes `None` through the
  `unread` closure for a failure before the read (unsupported format, row
  shape, fetch refusal). R9/R10 now say "the hash" and `observe_read`.
- **A2 (N17): holds.** `record_probed_candidates(&file_ref, ctx)` runs before
  the not-found error (`load.rs:71`); `DataRuntime::prepare_contract`
  observes missing imports through `contract_observations` (`runtime.rs:649`).
- **A3 (N18): holds.** `simplified_schema::resolve::buffers::read(path)` in
  the source loader (`load.rs:115`).
- **A4 (N28): holds, earlier than the plan assumed (revised R14).**
  `ctx.authorize_slot_read(FUNCTION, ReadKind::Content, …)` runs right after
  resolution and **before** `ctx.data.record(&identity)`, format choice, the
  fetch, and the memo (`load.rs:77-83`). It precedes steps 9–12 as required;
  R14 now lists it between resolution and recording.
- **A5 (N35): holds.** `DataRuntime::memo_generation()` returns
  `DataMemo::generation()`, the `Generation::number` replaced by
  `DataMemo::invalidate` under the one lock (`runtime.rs:315-325,604`).
- **A6 (N20): holds.** `REMOTE_READ_FUNCTIONS` no longer exists; discovery
  uses `remote::fetches_url(name)` derived from the registration's
  `read_fact` (`remote.rs:436`), `load_data` registers
  `.reads_or_fetches(ReadKind::Content)` which sets
  `SlotClass::InputReading` (`functions/data.rs:5`, `functions/mod.rs:230`),
  and `collect_expression_urls` reads `args.first()` at any arity
  (`remote.rs:410`).
- **A7 (N29): holds.** The collector is
  `schemas/slots/grants/targets.rs::collect_targets` (path differs from the
  plan's `slots/targets.rs`); `reads_of` reads `args.first()` at any arity
  (`targets.rs:85`).
- **A8 (memo shape): holds.** `DataMemo<S, B>`, `single_flight`,
  `Generation { number, sources, bindings, contracts }`, and
  `DataRuntimeStats { reads, parses, binds, source_hits, bind_hits,
  contract_preparations, contract_hits, invalidations }` (`runtime.rs:173-311`).
- **A9 (catalog and call check): holds.** `schemas/partials/functions.yaml:792`
  keeps `parameters(file: file | null, as: type-definition(container))`;
  `call_check.rs::load_data_problem` (`:1096`), `load_data_type` (`:236`),
  `data_problem` (`:1145`), `is_tabular_path` (`:1155`) keep their shapes.
- **A10 (DMLS): holds.** N42 landed (`dmls/src/providers/frontmatter/enum_item_tests.rs`);
  `type_definition_argument` wiring is at `dsl.rs:57,60` and
  `frontmatter.rs:75`.

Plan revisions made for this check: R9 and R10 (hash instead of an
`Observation` value), R14 (gate position). See "Decisions applied".

### Baseline

(pending)

### Departures

- **R25, execution location of computed-option failures.** The spec asks for
  `dm.data.invalid_options` to be located at the third argument. Passive
  findings are (`DataProblem.argument = 2`), but at execution the evaluator
  carries no per-argument spans, so a failure of a *computed* options object
  is located at the call expression, with the key named in the message. A
  literal mistake is always caught passively first. The docs will say so.

### Decisions applied

- R9/R10/R14 revised by the rebase check (above).
