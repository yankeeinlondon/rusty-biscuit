---
total_phases: 9
created: 2026-11-08
phase: 9
agent: claude/sonnet
yolo: true
packages:
- darkmatter-lsp
- darkmatter
- simplified-schema
- sniff
- darkmatter-cli
- claudine-cli
- claudine
source_files_during_phase_1:
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/providers/frontmatter/enum_item_tests.rs
- darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs
- darkmatter/lib/tests/l1/dynamic_slots_fixture_repos.rs
- darkmatter/lib/tests/l1/main.rs
- simplified-schema/lib/tests/fixtures/dynamic_slots/value-positions.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/values/revision.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/values/minimum.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/values/maximum.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/values/default-retries.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/values/slug-pattern.yaml
docs_updated_during_phase_1:
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/dmls/docs/autocomplete.md
- simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/darkmatter/dmls.md
source_files_during_phase_2:
- simplified-schema/lib/src/completion.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/expression/scan.rs
- simplified-schema/lib/src/format.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/recursive.rs
- simplified-schema/lib/src/schema/error.rs
- simplified-schema/lib/src/schema/export.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/schema/layering.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/schema/tests.rs
- simplified-schema/lib/src/schema/view.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/grammar/slot_tests.rs
- simplified-schema/lib/src/simplified/lint.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/narrowing.rs
- simplified-schema/lib/src/simplified/query.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/subtype.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/tests/l1/declaration_grammar.rs
- simplified-schema/lib/tests/l1/expression_scanner.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/schema_object.rs
- simplified-schema/lib/tests/l1/slot_spans.rs
- simplified-schema/lib/tests/l1/slot_templates.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
docs_updated_during_phase_2:
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/language-core.md
- simplified-schema/docs/topics/language/index.md
- simplified-schema/docs/topics/simplified-schema-struct.md
docs_created_during_phase_2:
- simplified-schema/docs/topics/language/slots.md
skills_files_updated_during_phase_2:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/buffers.rs
- simplified-schema/lib/src/resolve/recursive.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/simplified/definitions.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/host.rs
- simplified-schema/lib/src/slots/pending.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
docs_updated_during_phase_3:
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/host-capabilities.md
- simplified-schema/docs/topics/language/index.md
- simplified-schema/docs/topics/language/slots.md
- simplified-schema/docs/topics/language/types.md
- simplified-schema/docs/topics/simplified-schema-struct.md
- simplified-schema/docs/topics/validation.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/context/capture/clock.rs
- darkmatter/lib/src/markdown/compose/context/capture/datetime.rs
- darkmatter/lib/src/markdown/compose/context/capture/mod.rs
- darkmatter/lib/src/markdown/compose/context/capture/observations.rs
- darkmatter/lib/src/markdown/compose/context/runtime.rs
- darkmatter/lib/src/markdown/compose/expression/ctx.rs
- darkmatter/lib/src/markdown/compose/expression/data/load.rs
- darkmatter/lib/src/markdown/compose/expression/data/mod.rs
- darkmatter/lib/src/markdown/compose/expression/data/observation_tests.rs
- darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli.rs
- darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
- darkmatter/lib/src/markdown/compose/expression/functions/composition.rs
- darkmatter/lib/src/markdown/compose/expression/functions/data.rs
- darkmatter/lib/src/markdown/compose/expression/functions/dates.rs
- darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
- darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
- darkmatter/lib/src/markdown/compose/expression/functions/git.rs
- darkmatter/lib/src/markdown/compose/expression/functions/markdown_docs.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/expression/functions/network.rs
- darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
- darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
- darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
- darkmatter/lib/src/markdown/compose/expression/functions/shell.rs
- darkmatter/lib/src/markdown/compose/expression/functions/skills.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/read_gate.rs
- darkmatter/lib/src/markdown/compose/expression/read_gate_tests.rs
- darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
- darkmatter/lib/src/markdown/compose/remote.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/admission.rs
- darkmatter/lib/src/markdown/schemas/slots/codes.rs
- darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
- darkmatter/lib/src/markdown/schemas/slots/host.rs
- darkmatter/lib/src/markdown/schemas/slots/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/tests.rs
- darkmatter/lib/tests/l1/load_data_runtime.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/resolve/buffers.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
docs_updated_during_phase_4:
- darkmatter/docs/cli/data-contracts.md
- darkmatter/docs/topics/compose-requests.md
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/docs/topics/loading-data.md
- darkmatter/docs/topics/schemas/index.md
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/language/slots.md
docs_created_during_phase_4:
- darkmatter/docs/topics/schemas/schema-slots.md
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/requests.md
- .claude/skills/darkmatter/schema.md
source_files_during_phase_5:
- darkmatter/cli/src/approval.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/expression/read_gate.rs
- darkmatter/lib/src/markdown/compose/expression/read_gate_tests.rs
- darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
- darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
- darkmatter/lib/src/markdown/compose/remote_fetch.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/approval_tests.rs
- darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/auto.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/matching.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/request.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/store.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/tests.rs
- darkmatter/lib/src/markdown/schemas/slots/host.rs
- darkmatter/lib/src/markdown/schemas/slots/mod.rs
- sniff/lib/src/filesystem/git/mod.rs
- sniff/lib/src/filesystem/git/tracked_state.rs
- sniff/lib/src/filesystem/git/tracked_state_tests.rs
- sniff/lib/src/filesystem/git/types.rs
docs_updated_during_phase_5:
- darkmatter/docs/topics/schemas/schema-slots.md
- sniff/docs/sniff-library-architecture.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/schema.md
- .claude/skills/sniff/SKILL.md
- .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_6:
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/tests/l1/yaml_schema_envelope.rs
- darkmatter/lib/src/markdown/schemas/slots/cache.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/src/markdown/schemas/slots/session_tests.rs
- darkmatter/lib/src/markdown/schemas/slots/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/host.rs
- darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
- darkmatter/lib/src/markdown/schemas/slots/codes.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
- darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/select.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/semantic.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
- darkmatter/lib/src/markdown/compose/expression/data/contract.rs
- darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
- darkmatter/lib/src/markdown/output/command/data.rs
- darkmatter/lib/src/work_counters.rs
- darkmatter/lib/tests/l1/dynamic_schema_slots.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/global_catalog.rs
- darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
- darkmatter/lib/tests/l1/yaml_kind_registry.rs
- darkmatter/cli/src/commands/schema/triggers.rs
- darkmatter/cli/schemas/output/schema-triggers.yaml
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/yaml_document.rs
- darkmatter/dmls/tests/l1/yaml_document_editor.rs
- claudine/cli/src/completion/schema_completion/mod.rs
- claudine/cli/src/completion/schema_completion/keys.rs
- claudine/cli/src/completion/schema_completion/tests.rs
- claudine/cli/src/completion/engine/mod.rs
- claudine/cli/src/completion/engine/tests.rs
docs_updated_during_phase_6:
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/schema-triggers.md
- darkmatter/docs/cli/schema.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
- .claude/skills/darkmatter/schema.md
source_files_during_phase_7:
- darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/schemas/clean.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/tests/l1/diagnostic_projection.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/report_diagnostics.rs
- darkmatter/lib/tests/l1/shared_diagnostics.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
- darkmatter/cli/src/commands/clean.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/commands/compose/lifecycle.rs
- darkmatter/cli/src/commands/graph.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/src/commands/validate.rs
- darkmatter/cli/src/output/mod.rs
- darkmatter/cli/tests/common/fixture.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/output_commands.rs
- darkmatter/cli/tests/l1/dynamic_schema_slots.rs
- darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
- darkmatter/cli/schemas/output/clean.yaml
- darkmatter/cli/schemas/output/code-block.yaml
- darkmatter/cli/schemas/output/compose.yaml
- darkmatter/cli/schemas/output/delta.yaml
- darkmatter/cli/schemas/output/edit.yaml
- darkmatter/cli/schemas/output/get.yaml
- darkmatter/cli/schemas/output/graph.yaml
- darkmatter/cli/schemas/output/hash.yaml
- darkmatter/cli/schemas/output/list-themes.yaml
- darkmatter/cli/schemas/output/render.yaml
- darkmatter/cli/schemas/output/rm.yaml
- darkmatter/cli/schemas/output/schema-about.yaml
- darkmatter/cli/schemas/output/schema-detect.yaml
- darkmatter/cli/schemas/output/schema-triggers.yaml
- darkmatter/cli/schemas/output/schema-validate.yaml
- darkmatter/cli/schemas/output/set.yaml
- darkmatter/cli/schemas/output/toc.yaml
- darkmatter/cli/schemas/output/validate-refs.yaml
- claudine/docs/research/sandboxing/providers/sandboxing.yaml
docs_updated_during_phase_7:
- darkmatter/docs/cli/clean.md
- darkmatter/docs/cli/compose.md
- darkmatter/docs/cli/data-contracts.md
- darkmatter/docs/cli/graph.md
- darkmatter/docs/cli/schema.md
- darkmatter/docs/cli/validate.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/schema-slots.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
- .claude/skills/darkmatter/schema.md
source_files_during_phase_8:
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/cache.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/grants/store.rs
- darkmatter/lib/src/markdown/schemas/slots/host.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/src/markdown/schemas/slots/session_tests.rs
- darkmatter/dmls/src/capabilities.rs
- darkmatter/dmls/src/context.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/mod.rs
- darkmatter/dmls/src/diagnostics/slots.rs
- darkmatter/dmls/src/lib.rs
- darkmatter/dmls/src/overlay/analysis.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/overlay/yaml_document.rs
- darkmatter/dmls/src/providers/code_actions.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/router.rs
- darkmatter/dmls/src/slot_jobs.rs
- darkmatter/dmls/src/workspace/watch.rs
- darkmatter/dmls/tests/common/mod.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/no_side_effects.rs
- darkmatter/dmls/tests/l1/schema_slots_editor.rs
docs_updated_during_phase_8:
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
- .claude/skills/darkmatter/dmls.md
---

# Plan: Dynamic Schema Types

Source: the `2026-11-05-dynamic-schema-types` spec (Rulings Q1–Q20 are in force;
a later ruling governs where it corrects an earlier one). The spec's
"Investigation outcomes" record the S1–S8 results; the full reports are in
`investigations/` beside this plan (`s4-function-classes.md` holds the
per-function table N20 uses). Every remaining implementation choice is settled
in **Phase 1 → Necessary Rules** (rule IDs `N1`…`N42`, plus `N6a`, `N12a`, `N12b`). Later phases cite rules
by ID and never reopen them.

**Reconciled 2026-11-08.** This plan was first written before the
investigations reported and before Rulings Q8–Q20. It was revised to match
them; the "Reconciliation" section at the end lists every rule that changed.

Execution conventions for every phase:

- Run package-area recipes only: `just test` and `just lint` inside
  `simplified-schema/`, `sniff/`, `darkmatter/` (never workspace-wide gates).
  Tests run under nextest through those recipes.
- Never run `cargo fmt`, never commit, never move the spec to `_completed`.
  The terminal state is "implementation complete, ready for review".
- Each task that changes a symbol's behavior also fixes that symbol's `///`,
  `//!` and inline comments in the same change (repo rule on drift).
- Windows/WSL2 are proven after merge; still write every path comparison with
  `biscuit_file::canonicalize_simplified` and `Path::starts_with`, never string
  prefixes, and load the `os` skill before touching path-comparison code.
- No test in this plan opens a terminal or browser window.

## Summary and Definition of Done

### What is being built

A schema author can write a `{{ … }}` expression as a complete list element
inside `enum(...)` and `suggest(...)`, and as the whole argument of
`literal(...)`, `default(...)`, `min(...)`/`max(...)`, and `pattern(...)`
(Q16):

```yaml
types:
    os: enum({{ load_data("../../../os.yaml", "string[]") }}; required)
```

Schema handling stays **passive** until an explicit, approved, cached
**resolution** step evaluates the slots. The work splits into five layers:

1. **Language (`simplified-schema`)**: grammar, AST (`Constraint::Dynamic`),
   passive export/query, typed "unresolved" errors, and a host-neutral
   resolution hook (`SlotHost`, `PendingSchema`) that produces an ordinary
   concrete `ResolvedSchema` / `SimplifiedSchema`.
2. **Evaluation (`darkmatter` lib)**: slot admission (binding rules),
   a slot evaluator on the existing expression engine, input fingerprints,
   function determinism classes.
3. **Approval (`sniff`, `darkmatter` lib, `darkmatter-cli`)**: the Q5 per-resource
   model: automatic grant for clean same-repo files, a user-level grant store,
   batched prompts beside shell approvals, typed `approval-required`.
4. **Sessions and cache (`darkmatter` lib)**: a process-wide, input-fingerprinted
   resolution cache; per-command pinned snapshots; trigger payloads kept pending
   (Q6); stale-snapshot re-resolution after mutations (Q7).
5. **Surfaces**: `md compose`, `md schema validate`, `md clean`, reference
   checks, the shipped `sandboxing.yaml`, DMLS (worker, watches, stale serving,
   grant commands), docs and skills.

### Definition of Done

All of these are observable and each has a test named in the phase that
delivers it:

- [ ] M1/M2: `enum(...)` and `suggest(...)` accept slots mixed with literals and
  the `; required` / `default(...)` tail, on every type where `suggest` is
  allowed today.
- [ ] Q16: `literal(...)`, `default(...)`, `min`/`max`, and `pattern(...)` accept a
  whole-argument slot with the result shapes in N6; a data-supplied pattern is
  capped (N6a).
- [ ] Q20: DMLS completion inside a static or resolved `enum(...)` list (the
  sandboxing `os[]` case) offers the members.
- [ ] Q18: every input-reading function records its dependencies in the read set
  (including `load_data`'s not-found probe and missing contract imports), and
  pre-fetch URL discovery folds case and underscores like evaluation does.
- [ ] M3: slot syntax errors, forbidden bindings, wrong result shapes, empty
  results, approval failures, and `load_data` failures are positioned schema
  errors in `md schema validate` and DMLS; a failing slot keeps its `dm.data.*`
  code and `data_location`; positions are `null` unless proven.
- [x] M4: `claudine/docs/research/sandboxing/providers/sandboxing.yaml` uses the
  schema-relative `load_data` spelling, is admitted passively, validates its
  documents after explicit resolution, and the corpus carve-out
  (`awaiting_enum_slots` in `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`)
  is gone.
- [ ] M5: each guarantee of "The tension to resolve" is kept or replaced and the
  replacement is documented (N36).
- [ ] Parsing, query, YAML export, and trigger/kind discovery evaluate nothing and
  read nothing (proved by counters and by a read-nothing fixture).
- [ ] `to_json_schema`, validator preparation, and layering on an unresolved
  schema return a typed unresolved error; a type query reports pending
  information; no placeholder schema exists anywhere.
- [ ] A warm cache hit performs zero evaluator runs and zero validator compiles,
  and replays the recorded reads into the command's read set.
- [ ] The Q5 approval matrix passes (clean grant; refusals for dirty, untracked,
  ignored, symlink-escaping, other-repo, no-repo; every grant scope; deny;
  batched prompt; computed targets; typed `approval-required` on data outputs
  and for library callers; DMLS commands write the user-level store; repository
  content cannot grant).
- [ ] An unselected trigger payload with a template reads nothing; a write to a
  snapshot dependency forces re-resolution (Q6, Q7).
- [ ] DMLS requests perform no I/O or evaluation; stale worker results and
  unsaved edits cannot overwrite newer snapshots.
- [ ] `just test` and `just lint` pass in `simplified-schema/`, `sniff/`, and
  `darkmatter/` (including `simplified-schema`'s minimal-feature checks), and
  `cargo check --all-targets -p claudine -p claudine-cli -p messenger` compiles;
  Claudine's CLI completion reports an unresolved template instead of silently
  offering nothing (N41).
- [ ] Topic docs and the `darkmatter`, `simplified-schema`, `biscuit-file`-adjacent
  skills are updated; whole-type slots are documented as **planned** and a
  follow-up spec for them exists (N6).
- [ ] The Phase 9 timing sample is recorded against the S1 budgets (N1).

## Phase 1 — Decisions, baseline, and fixtures

Goal: record every decision, prove the starting state is green, and create the
fixtures and log every later phase uses.

### Necessary Rules

**Spike dispositions**

- **N1. The spikes have run; this plan applies their outcomes.** See the spec's
  "Investigation outcomes" and `investigations/`.
  - S1 set the budgets: warm overhead at most 5% per document; a cold slot at
    most 2× the lowering and compile of the schema it produces and at most 1% of
    the surface's cold time; DMLS requests do no I/O and gain at most 0.1 ms.
    Every position passed on the simulated prototype. Phase 9 re-measures the
    real implementation once on one host and records it against these budgets;
    a miss is reported to the author in the log, not fixed speculatively.
  - S4 is applied by N20; S5 by N24–N30; S7 by N8–N13; S8 by N6.
  - S6: lazy/partial resolution is **not** built (it saves 1.7%); a schema's
    effective slots resolve together, once per equivalent context.
- **N2. Cache scope is process-wide and in-memory only (S2).** No on-disk
  resolution cache: it would save about 37 µs per process and adds
  secret-handling and staleness hazards. Within a command the pinned snapshot
  is trusted (Q7/N35 govern writes); across commands a hit is re-checked by
  `stat` (size + mtime), rehashing only the inputs whose `stat` changed (Q19).
  The same-size-within-mtime-resolution miss is documented as a limitation.

**Language representation (simplified-schema)**

- **N3. Representation.** Add to `simplified-schema/lib/src/simplified/types.rs`:
  ```rust
  pub struct Slot { pub expression: String,            // trimmed text between {{ and }}
                    pub span: SourceSpan,              // whole `{{ … }}`
                    pub expression_span: SourceSpan,   // the trimmed expression text
                    pub origin: Option<PathBuf> }      // None = the schema's own file/document
  pub enum DynamicKind { Members, Suggest }
  pub enum DynamicElement { Member(String), Suggestion(SuggestionCandidate), Slot(Slot) }
  pub struct DynamicList { pub kind: DynamicKind, pub target: SimplifiedType, pub elements: Vec<DynamicElement> }
  Constraint::Dynamic(DynamicList)
  ```
  A list becomes `Constraint::Dynamic` **only when it contains at least one
  slot**; fully static lists stay `Constraint::Members` / `Constraint::Suggest`
  (static-schema parity and the fast path are untouched). `target` is `Enum` for
  members and the suggest target type (`String`, `Number`, `Alphanumeric`, or
  `String` for a deferred reference, exactly as `parse_one_constraint` picks it
  at `grammar.rs` near line 2761). `Constraint::keyword()` for `Dynamic` is the
  first element's authored text (`"{{"` for a leading slot), which is how
  option lists are located today (`types.rs` near line 1125).
  Whole-argument positions (Q16) add
  `pub enum ValueSlotKind { Literal, Default, Min, Max, Pattern }` and
  `Constraint::DynamicValue { kind: ValueSlotKind, target: SimplifiedType, slot: Slot }`.
  Concrete `Members`, `Suggest`, `Literal`, `Default`, `Min`, `Max`, and `Pattern`
  constraints stay concrete-only, so their existing readers (Claudine included)
  never see a slot.
- **N4. Where slots are admitted.** (a) **List positions:** `enum(...)` members
  and `suggest(...)` arguments, as a complete element. A slot is recognised when,
  after whitespace, an element begins with `{{`; it ends at the end of the
  scanner's expression span and must then be followed (after whitespace) by
  `,`, `;` or `)`; anything else is the positioned grammar error "a slot must be
  a complete list element". (b) **Value positions:** `literal(...)`,
  `default(...)`, `min(...)`, `max(...)`, `pattern(...)`, where the slot must be
  the whole argument (no mixing with literal text); anything else is the
  positioned error "a slot must be the whole argument". A `{{` inside a quoted
  element, or anywhere else, keeps today's behavior. Evidence: spec "Slot
  boundaries", M1/M2, and Q16.
- **N5. One closing-delimiter rule.** Add `pub fn scan_expression_at(content:
  &str, start: usize) -> Option<ExpressionLocation>` to
  `simplified-schema/lib/src/expression/scan.rs`, a thin public wrapper over the
  private `scan_expression`. The grammar, the YAML span scanners
  (`simplified/source.rs`: `split_top_level`, `scan_constraint_groups`,
  `constraint_call_arguments`, `inline_object_end`, `find_top_level_*`) and DMLS
  all call it; none implements its own closing rule. `None` (unclosed) is the
  positioned error "unterminated `{{ … }}` slot". A slot whose text does not
  parse is the positioned error from `parse_spanned` with the byte offset mapped
  through `expression_span`.
- **N6. Breadth (S8, Q16).** This plan delivers every position below except whole
  types. Whole types are documented as **planned** in
  `simplified-schema/docs/topics/language/types.md` and get a follow-up spec
  (Phase 9); per Q16 that spec resolves imports in a returned type against the
  slot's defining file.

  | Position | Result shape | Admission | Diagnostics |
  | --- | --- | --- | --- |
  | `enum(…)` / `suggest(…)` | arrays, per N9 | complete list element (N4a) | per N9 |
  | `literal(…)` | one scalar (string, number, boolean), type preserved | whole argument (N4b) | wrong shape at slot |
  | `default(…)` | any JSON value valid for the type | whole argument | value judged against the type after resolution, positioned at slot |
  | `min` / `max` | one finite native number (through `simplified_schema::numeric`) | whole argument | non-number or non-finite at slot; `min > max` after resolution at the later slot |
  | `pattern(…)` | one string, capped per N6a | whole argument | invalid or over-cap regex positioned at slot |
  | whole type (**planned**) | one string parsed by the existing type-definition parser | whole property definition; imports follow existing rules | parse errors positioned at slot |

- **N6a. Pattern cap.** Validation compiles patterns with the linear `regex`
  engine and switches a schema to backtracking `fancy-regex` only for
  lookaround (`validate.rs::schema_uses_lookaround`). A slot-supplied pattern
  must not move a schema onto the backtracking engine: it is rejected at the
  slot if it contains a lookaround or backreference, if it exceeds 4 KiB of
  source, or if `regex::RegexBuilder` fails with the default `size_limit`.
  Authored (static) patterns keep today's behavior.
- **N7. Duplicates.** In a `Dynamic` list, after candidate interpretation, keep
  the first occurrence of each interpreted value (literal–literal, literal–slot,
  slot–slot). Static lists keep their current behavior (`suggest` duplicate is a
  grammar error; `enum` unchanged). Evidence: spec "Slot boundaries" last bullet.

**Resolution hook (S7)**

- **N8. Mechanism: two-pass replay under a scoped substitution, like source
  buffers.** `simplified-schema` already scopes ambient inputs with
  `resolve::buffers::with(..)`. Add `simplified-schema/lib/src/slots.rs`:
  - `SlotKey { origin: Option<PathBuf>, start: usize, expression: String }`
    (dedupes the same slot inlined at several use sites);
  - `SlotValues` (map key → `serde_json::Value`) and
    `with_values(&SlotValues, f)` / `active_values()` (thread-local scope);
  - `collect_slots(&SchemaRoot, &DefinitionTable) -> Vec<Slot>` (unique, traversal
    order);
  - `substitute(&mut SchemaRoot, &mut DefinitionTable, &SlotValues) ->
    Result<(), SchemaError>` which turns every `Constraint::Dynamic` into the
    static `Members` / `Suggest` constraint (N9) and positions every error at the
    slot span;
  - `has_dynamic(&SchemaRoot, &DefinitionTable) -> bool`.
- **N9. Substitution semantics (the result-shape validator).**
  Default: every invalid shape is a located error
  (`SchemaError::SlotResult { span, kind }`), publishes nothing, and aborts
  the whole resolution. Exceptions:
  - slot result must be an array; each array is flattened exactly one level in
    declaration order; `[]` contributes nothing; the final list must be
    non-empty (`kind = Empty`, positioned at the first slot of the list);
  - **enum**: every element must be a JSON string (no conversion of number,
    boolean, null); anything else is `WrongShape`;
  - **suggest**: elements may be string, number, boolean, null; strings go
    through `interpret_suggestion_parts` unchanged; a number becomes its
    canonical numeric text via `simplified_schema::numeric` (the same
    formatter `format_number` uses in `grammar.rs`), `true`/`false`/`null` become
    their literal text; the text is then interpreted exactly like a literal
    candidate for `target` (so an invalid numeric suggestion is kept for lint,
    never a schema-load error); mapping or nested array element is `WrongShape`;
  - candidate span for a slot-produced suggestion is the slot's `span`;
  - data containing `,` `;` `)` `{{` stays one literal value because values are
    inserted into typed constraints, never re-parsed from text;
  - **value positions** (N4b) take the shapes in N6's table; `null` is always a
    `WrongShape` (never "absent"), and a `DynamicValue` becomes the matching
    concrete constraint.
- **N10. Pending schema and entry points.** Add `SchemaError::UnresolvedSlots(Box<UnresolvedSlots>)`
  where `UnresolvedSlots { slots: Vec<Slot>, pending: Option<PendingSchema> }`.
  The lowering functions (`to_json_schema_with_definitions` in
  `simplified/convert.rs`) return it (with `pending: None`) when they meet a
  `Constraint::Dynamic`, exactly the way a function-typed property already makes
  lowering fail (`SchemaError::Convert`). So **every existing caller that does
  not know about slots fails closed with a typed error**; no caller needed
  changing for safety. `PendingSchema { slots, template:
  Option<(SchemaRoot, Arc<DefinitionTable>)>, replay: Arc<dyn Fn() ->
  Result<ResolvedSchema, SchemaError> + Send + Sync> }` is attached by these
  public entry points in `resolve.rs`, which clone their inputs into a replay
  closure: `resolve_schema`, `resolve_schema_with_roots`, `resolve_yaml_schema`,
  `resolve_yaml_schema_with_roots`, `resolve_yaml_schema_owned_by`,
  `prepare_standalone_yaml_schema`. Entry points that stay slot-free
  (`resolve_catalog_declaration`, `prepare_embedded_*`, `expand_named_types*` used
  by `contract.rs` outside a slot session) keep returning
  `UnresolvedSlots` with `pending: None` ("not resolvable here").
  `PendingSchema::resolve(&self, host: &dyn SlotHost, request: &SlotRunRequest)
  -> Result<ResolvedSchema, SlotResolutionError>` evaluates each unique slot via
  the host, builds `SlotValues`, and runs `with_values(&values, || (self.replay)())`
  — the same resolution code, with substitution applied immediately after
  import expansion and before both lowering and storing `simplified`.
  Insert `slots::apply_active(&mut schema, &mut definitions)?` after
  `expand_imports` / `expand_document_imports` at the lowering sites in
  `resolve.rs` (`resolve_inline_mapping` ~l.317, `resolve_root_union` ~l.594,
  the standalone single-root site ~l.1243, `resolve_standalone_root_union`
  ~l.1634, and the embedded site ~l.1554, which instead **rejects** any slot:
  "an embedded schema cannot contain slots", mirroring its `example(...)` rule).
- **N11. Host trait.**
  ```rust
  pub trait SlotHost {
      /// Passive. No I/O, no evaluation. Binding rules (N14).
      fn admit(&self, slot: &Slot) -> Result<(), SlotAdmissionError>;
      /// The only effectful step.
      fn resolve(&self, request: &SlotRequest<'_>) -> Result<serde_json::Value, SlotFailure>;
      /// Checked before each slot and before publishing; `true` aborts with
      /// `SlotResolutionError::Canceled` and publishes nothing (DMLS supersedes a
      /// job when a newer edit arrives).
      fn is_canceled(&self) -> bool { false }
  }
  pub struct SlotRequest<'a> { pub slot: &'a Slot, pub defining_file: Option<&'a Path>, pub context: &'a FileResolutionContext }
  pub struct SlotRunRequest { pub context: FileResolutionContext }
  ```
  `SlotFailure` is an opaque typed carrier: `{ source: Arc<dyn Error + Send + Sync>, code: Option<&'static str>, data_location: Option<…> }` so Darkmatter's `DataError` (code + `data_location`) survives unwrapped (M3). `SlotResolutionError { failures: Vec<(Slot, SlotFailure)> | Schema(SchemaError) }`:
  all independent slots are attempted; every failure is reported.
  `SlotRequest.defining_file` is `slot.origin` or the entry schema's file
  (`None` for an in-memory/document-inline slot, whose directory is the
  context's source directory). No Darkmatter type appears in
  `simplified-schema`; `simplified-schema/lib/tests/l1/dependency_direction_guard.rs`
  and `schema_language_ownership_guard`-style guards must still pass.
- **N12. Origin stamping.** In `resolve.rs::expand_imports` where a `Name@file`
  (or `@this`) definition body from `DefNamespace::File(path)` is lifted, stamp
  `Slot.origin = Some(path)` on every slot in the lifted body that has `None`.
  Slots authored inline keep `None`. Evidence: spec "Imported definitions retain
  their defining file's context"; `DefNamespace::File(PathBuf)` carries the
  canonical path (`simplified/definitions.rs` l.60–74).
- **N12b. Schema-file bytes for the key.** Resolution records the exact text of
  every schema file it read (entry and transitive imports) as it reads it
  (`PendingSchema::read_sources() -> &[(PathBuf, Arc<str>)]`); the host hashes
  that text with biscuit-hash for N21. No file is reopened after evaluation.
- **N13. `SimplifiedSchema` object.** `schema/layer.rs::resolve_authored` keeps the
  typed (unsubstituted) root on the layer and stores
  `lowered = Err(Arc<SchemaError::UnresolvedSlots>)`. Add to the public object:
  `is_resolved()`, `pending_slots() -> &[Slot]`, and
  `resolve_slots(&self, host: &dyn SlotHost) -> Result<SimplifiedSchema,
  SlotResolutionError>` which replays `resolve_authored` under
  `with_values` and returns a **new** concrete schema (the template is never
  mutated; re-resolving starts from the template). `layer_under`, `layer_over`,
  `to_json_schema`, `validate_*` on a schema with any unresolved layer return
  `SimplifiedSchemaError::Unresolved(Arc<SchemaError>)`.
  `to_simplified_yaml()` exports the template with `{{ … }}` preserved
  (`serialize.rs`); `query()` marks the dynamic constraint pending
  (`TypeDefinition::pending_slots()`), listing literals but never presenting a
  partial list as complete.
- **N12a. Layer order.** Resolution happens per layer, before Darkmatter's
  `merge_baseline`. `EffectiveSchema`'s merge of already-lowered schemas **stays
  as it is** (S7 answer): pending layers never reach it because a template layer
  is resolved to a `ResolvedSchema` first, and `effective_schema_layering.rs`
  continues to pin the equivalence.

**Darkmatter evaluation**

- **N14. Binding admission (passive, every branch).** `DarkmatterSlotHost::admit`
  parses the slot with `parse_spanned` and walks it with
  `expression::absence::static_variable_reads_in_scope(expr, &[])` plus callback
  patterns. Allowed free roots: `ctx` (except the members below), `env`, and the
  literals `null`, `true`, `false`; `current.*` and `local.*` only inside a
  callback body that binds them (Q12). Everything else is rejected at the read's
  span with `dm.schema.slot_forbidden_binding`: `fm` (Q3); any bare identifier
  (it falls through to document properties); host globals (including lifecycle
  state); `ctx.cwd`; document identity `ctx.self`, `ctx.last_updated`,
  `ctx.hash`; per-run nonces `ctx.id`, `ctx.sid` (Q12); and `current`/`local`
  outside a callback. Rejection happens whether or not the branch would run.
  Evidence: `catalog/roots.rs` reserved roots are exactly
  `fm, ctx, env, current, local`; `absence.rs` already walks both ternary
  branches and every `||` operand; `investigations/s4-function-classes.md`
  ("Functions to reject in a slot").
- **N15. Evaluator.** New module `darkmatter/lib/src/markdown/schemas/slots/evaluator.rs`.
  It parses, evaluates with `expression::evaluate` over
  `ShortcutLookup::in_context(&Value::Null, &resolution)` (modeled on
  `conditions.rs::evaluate_condition_in`), where `resolution` is a
  `ResolutionContext::new(slot_context)` whose `data` is the **command's**
  `DataRuntime` and whose remote runtime is the slot policy of N27. The slot
  context is `template_context.for_source(defining_file)`
  (`biscuit_file::FileResolutionContext::for_source`), never a recapture of
  process state. Result is a `serde_json::Value`.
- **N16. Work limit.** Each slot evaluation gets a fresh
  `IterationBudget` built from the default `IterationLimits`
  (100 000 visits, 16 MiB, `expression/iteration.rs`); exhaustion is the
  existing `dm.iteration.visit_limit` / `dm.iteration.expansion_limit`
  diagnostic positioned at the slot. Evidence: an expression with no iteration is
  linear in its source text, so the budget is the only unbounded dimension and
  the codebase already owns it; no new constant is needed.
- **N17. Fingerprints.** `DataRuntime` (in `compose/expression/data/runtime.rs`)
  gains an observation log: `Observation { entry: String, kind:
  ReadKind::{File, Url, MissingProbe}, hash: Option<[u8;32]> }`, recorded
  where bytes are read (`data/load.rs` line ~103 for local, the fetched body for
  URL) hashed with `biscuit_hash::blake3_hash_bytes`, and stored **inside the
  memo entry** (`LoadedSource`) so a memo hit re-records the same observation.
  **Every** input-reading function in `investigations/s4-function-classes.md`
  records the same way, not only `REMOTE_READ_FUNCTIONS` (Q18): content reads
  record `File` with the hash of the bytes; existence and path-shape probes
  record `MissingProbe` or an existence flag; directory listings (`find_files`,
  `find_first_index`, `find_last_index`, `has_skill`) record a `Listing` kind
  with the hash of the sorted entry names. `load_data` records its not-found
  probe (today it returns before recording, `data/load.rs` ~l.64–73) and every
  missing contract import. A failed read keeps its dependency (`hash: None`).
  These recordings also improve the document read set, which is the
  loading-data contract. New
  `DataRuntime::observations_since(mark)` and `read_mark()` give a slot its own
  slice. Context inputs: after evaluation, fingerprint each group in
  `CtxLookup::captured_groups()` by BLAKE3 of the group's canonical JSON, and
  each `env.NAME` read found by the N14 walk by BLAKE3 of the value (`None` if
  unset); a non-static env access (`env[expr]`) fingerprints the whole sorted
  environment map. Digests only, in memory only; no value text is ever a key.
- **N18. Local reads honor open buffers.** `load.rs` reads local bytes through a new
  `simplified_schema::resolve::buffers::read(path) -> io::Result<Vec<u8>>`
  (buffer text bytes when `buffers::with` is active and the path is buffered,
  else `std::fs::read`), so DMLS refreshes unsaved data files through the
  existing mechanism. Outside a `with` scope behavior is byte-identical to
  today.
- **N19. Typed causes survive.** A failed `load_data` inside a slot yields
  `SlotFailure` wrapping the `ExpressionError::Data(Arc<DataError>)`;
  Darkmatter's diagnostic projection (`report_diagnostics.rs`) reads
  `code()` and builds `data_location` from it unchanged, with the primary
  `source` being the slot's authored position in its defining file (including an
  imported file) and `data_location` secondary.
- **N20. Determinism classes (S4).** The class is a **required fact on each
  function's registration** (`FunctionBinding`, `compose/expression/functions/mod.rs`
  ~l.86; host functions through `HostFunctionsBuilder` and the facts artifact),
  not a name table that can drift: `SlotClass::{Pure, InputReading, Captured,
  Volatile, Rejected}` plus a `reads` fact. Values come from the per-function
  table in `investigations/s4-function-classes.md`. A passive
  `slot_class(&SpannedExpr)` walk (`visit_call_nodes`, every branch and callback)
  reports the class, passive targets, computed targets, and rejections.
  - **Rejected** (Q10), a positioned `dm.schema.slot_forbidden_function`:
    `as_markdown`; the network and provider functions `pr`, `pr_list`, `cicd`,
    `cicd_list`, `ping`, `ping_under`; the login-shell probes `has_alias`,
    `has_builtin_function`, `has_user_function`, `can_execute`.
  - **Captured** (S4 step 1): the clock, the host timezone, and `PATH` are read
    from the captured request context, not live. Fix the 14 date functions that
    are registered `pure` but read the live clock or timezone (`functions/dates.rs`
    ~l.85, the `is_today` family ~l.119), and make `has_command`/`has_binary`
    read the captured `PATH` (Q11; today `functions/fs.rs` ~l.269–293 reads the
    process `PATH` through `which`). A captured value is fingerprinted like any
    other context value (N17).
  - **Input-reading**: classified **per invocation**; a URL argument is a remote
    read, gated by N27.
  - **Volatile** (git state, remaining provider and ICMP-free network reads such
    as `ipv4`/`ipv6`): allowed, resolved **once per command snapshot**, and never
    reused across commands; no TTL. The validator cache is keyed on the resolved
    content, so a volatile slot that yields the same values skips recompiling.
  - `REMOTE_READ_FUNCTIONS` (`compose/remote.rs`) is derived from the
    registration facts instead of the hand-kept list, and its pre-fetch matching
    folds case and underscores exactly as evaluation does (today `remote.rs`
    ~l.429 matches exactly and ~l.455 folds), so `loaddata("https://…")` is
    found up front (Q18).
- **N21. Cache key.** `ResolutionKey` = BLAKE3 of canonical JSON of:
  template identity (source revision hash of the defining schema file bytes, the
  hashes of every transitive import file, the entry schema path, and the slot keys),
  the derived context identity (canonical request root, repository root,
  magic-root bindings, `home` presence), `SLOT_RESOLVER_VERSION` (a `u32` const
  starting at 1), and the BLAKE3 of the embedded function catalog source
  (`compose/expression/catalog/mod.rs::CATALOG_SOURCE`). An entry stores
  `{ snapshot: Arc<ResolvedSchema>, observations, context_digests, reads:
  Vec<ReadRecord>, volatile: bool }`; each file observation also stores the
  `stat` (size, mtime) seen at read time. Lookup: key match → `stat` every file
  observation and rehash only those whose size or mtime changed (Q19; reads
  happen outside DMLS request paths) → check the **requesting** session's grants
  cover every `ReadRecord` → hit. A volatile entry is never served to another
  command.
  Failures are cached with their observations too (so creating a missing file or
  repairing a bad one invalidates them). The cache is a
  `LazyLock<Mutex<HashMap<…>>>` cleared by `reset_process_caches_for_bench`.
- **N22. Read-set replay.** A hit calls the new
  `DataRuntime::record_entry(&str)` for each recorded read so the command's
  read set equals what a cold run would report (loading-data contract).

**Approval model (Q5)**

- **N23. Grant vocabulary.** `GrantTarget::{File(PathBuf), Dir(PathBuf),
  Url(Url) /*exact, query included*/, UrlAnyQuery(Url) /*query stripped*/,
  Site{scheme,host,port}}`, `GrantScope::{Once, Persisted}`, decision `Allow` or
  `Deny`, each with either scope: "deny for this run" (`Deny` + `Once`) is also
  the decision on cancel or end of input (Q18). Matching: `File` = canonical equality;
  `Dir` = canonical `starts_with` (recursive); `Url` = exact string after `Url`
  normalization; `UrlAnyQuery` = same scheme/host/port/path; `Site` = scheme +
  host + port equality, no subdomains. Deny beats allow at equal specificity and
  anywhere in the book (a persisted deny beats a once-grant).
- **N24. Automatic grant rule.** A local **content read** is granted
  automatically iff all hold: (a) the slot's **defining** file (`Slot.origin`,
  else the entry schema file) lies in the request's captured repository
  (`RepositoryObservation.root`, never a new discovery); a schema outside it
  gets no automatic grant for any read (Q13); (b) after `canonicalize_simplified`
  of the target (nonexistent target: of its parent plus file name) it lies under
  that same root; (c) the Git blob hash of the **bytes actually read** equals
  the index entry and the `HEAD` entry (S5): a quick per-path status pre-check
  (N25) avoids reading obviously dirty files, and a hash mismatch discards the
  bytes and treats the file as dirty; (d) no open editor buffer for it differs
  from disk (`resolve::buffers`). Untracked, ignored, dirty, conflicted, LFS
  pointer, other-repository, and no-repository targets get none.
  Probes and listings (Q11): `file_exists` and other existence probes are
  automatic when (a) and (b) hold, tracked or not; directory listings
  (`find_files` and the other `Listing` readers) are **never** automatic and
  need a directory grant. Pure expressions need no grant.
  **Absent targets (Q21).** A content read (for example `load_data`) whose
  target does not exist is judged as an existence probe: automatic when (a) and
  (b) hold, reporting the ordinary typed not-found failure (recoverable by
  `try`); outside the defining repository it still needs a grant. If the file
  appears later in the run, the read is a content read again and (c) applies.
  Windows: under `core.autocrlf` the worktree bytes differ from the blob; apply
  gix's worktree-to-index conversion before hashing (load the `os` skill).
- **N25. Repository cleanliness comes from Sniff.** Sniff has no per-file status
  query (its `GitRepo` exposes `repo_status`/`changed_paths`; the private
  `status.rs::collect_path_status` holds the byte-path sets). Add
  `GitRepo::tracked_state(&self, paths: &[PathBuf]) -> Result<Vec<TrackedState>>`
  to `sniff/lib/src/filesystem/git/`, where `TrackedState::{Clean { index_blob,
  head_blob }, Dirty, Untracked, Ignored, Conflicted, Outside}`: `Clean` = in the
  gix index and in none of the `staged`/`unstaged`/`conflicted` sets, carrying
  the index and `HEAD` blob ids N24(c) compares against; not in the index and in
  `untracked` = `Untracked`; not in the index and in none = `Ignored`. The query
  is per path and must not walk the whole worktree (S1 measured about 3–5 ms per
  file against 109 ms for a full status walk); batched per call. The Darkmatter side keeps the
  observation's root as the repository identity and opens the repo at that root
  once per session (`SlotSession::tracked_states` memoizes by path for the
  session). Evidence: `RepositoryObservation` is `pub(crate)` with `root` and
  `info` but no status; opening at the observed root is not a recapture.
- **N26. Grant store (Q8).** The user-level file
  `<home>/.config/darkmatter/schema-read-policy`, where `<home>` is
  `FileResolutionContext::home_dir()` (read through `biscuit_file::home_dir()`
  at the one production site, so the existing rule holds), on every OS. It is
  **not** under the repository or document directory, no workspace setting
  (`.dmls.toml`, editor settings) can move it, and it is separate from the shell
  whitelist/blacklist, whose behavior is unchanged (rule sets are not merged).
  Repository content, schemas, and workspace config never read it as input and
  never write it. With no home, `Persisted` is not offered and the file is
  treated as absent. The parent directory is created on first persisted write.
  Format, one grant per line, UTF-8, `#` full-line comments, blank lines ignored,
  CRLF tolerated, optional UTF-8 BOM at the start tolerated:
  `allow file <abs-path>`, `allow dir <abs-path>`, `allow url <url>`,
  `allow url-any-query <url>`, `allow site <scheme://host[:port]>` and the same
  five with `deny`. The argument is the rest of the line, trimmed.
  Writes append through write-temp-then-rename in the same directory.
- **N27. Remote reads for slots (Q9).** Slot evaluation builds its **own** remote
  runtime; a slot grant never enters the request's shared `FetchPolicy`, so it
  cannot widen document reads. Its `FetchPolicy` allows exactly the hosts of
  (granted sites ∪ granted URLs ∪ the command's `--allow host=` hosts),
  `HostPattern::Exact`; scheme, port, query, and exact-URL grants are checked by
  the slot gate before the fetch, because `FetchPolicy` matches host only.
  `--allow host=HOST` keeps its meaning for document reads (any scheme or port)
  and is also a one-run site grant for slots. A document's own `load_data` and
  `::file` reads keep their current rules and never consult the gate.
- **N28. The gate.** `ResolutionContext` gains an optional `slot_gate:
  Option<Arc<dyn ReadGate>>` (set only by the slot evaluator).
  `ReadGate::check(&ReadTarget) -> Result<(), GateRefusal>`; `GateRefusal` is
  `Denied`, or `ApprovalRequired { needed: Vec<GrantChoice> }`. Every
  local/remote read in `compose/expression/functions/` that records into the read
  set (the `REMOTE_READ_FUNCTIONS` list, via `DataRuntime::record` call sites and
  `fs::read`/`read_dir` sites in `functions/`) calls `ctx.authorize_slot_read(..)`
  before touching the target, and **before the shared `load_data` memo lookup**
  (`data/load.rs` ~l.60–112), so a document's own ungated read cannot hand a
  slot the file from the memo (S5); with `slot_gate = None` it is a no-op, so
  document behavior is byte-identical. This is also how **computed targets** are
  checked at evaluation time.
- **N29. Pre-flight collection (passive).** `slots/targets.rs::collect_targets`
  walks a template's slots with `visit_call_nodes`, counting both branches of
  `?:` and every `||` operand as the shell pre-flight does; for each read-side
  call whose target argument is a literal, or an expression built only from
  literals and pure functions (folded passively, no reads), it yields a
  `ReadTarget` (path resolved against the slot's derived context; URL parsed);
  anything else is `Computed` and is checked at evaluation (N28).
  `compose_preflight_approvals_with` (in `compose/preflight/lifecycle.rs`)
  additionally asks `DarkmatterSchemas` for the pending templates of each
  visited document (new passive
  `DarkmatterSchemas::pending_templates_for(&Markdown) -> Vec<PendingSchema>`:
  document layer plus **matching** trigger payload layers; transcluded children
  are visited by the walk the function already performs) and raises one batch:
  slot requests are judged before the shell decisions, then both are asked
  together. **Only the effective schema's slots count** (Q14): a slot under a
  property that a higher layer redeclares is never collected, read, or asked
  for. Targets already covered by an automatic or persisted grant, or a
  persisted deny, are not asked. Pre-flight resolves nothing.
- **N30. Handler extension.** Add a **provided** method to `ShellApprovalHandler`
  (`shell_expansion/types.rs`):
  `fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) ->
  Result<Vec<SlotGrantDecision>, ShellExpansionError> { Ok(vec![Unanswered; n]) }`.
  `SlotGrantDecision::{Allow{target, scope}, Deny{target, scope}, Unanswered}`.
  `Unanswered` (or no handler) yields typed `SlotError::ApprovalRequired`. The
  CLI handler (`cli/src/approval.rs::CliShellApprovalHandler`) overrides it with a
  numbered prompt in the same batch. Choices for a file: this file, this file's
  directory; for a URL: this exact URL; this URL whatever its query (offered only
  when the URL has a query); this site; each with once / persisted; plus deny
  persisted and deny for this run (the decision on cancel or end of input, Q18).
  Computed targets reached during evaluation use the same handler through the
  gate when one exists and the run is interactive; otherwise
  `ApprovalRequired`. Library callers pass the handler to
  `SlotSession::new`; none ⇒ `ApprovalRequired`.

**Sessions, surfaces, mutation**

- **N31. Session.** `SlotSession` (new, `slots/session.rs`, `Arc`) bundles the base
  `FileResolutionContext`, the command's `DataRuntime`, the remote-policy inputs,
  the `GrantBook` (persisted file + once-grants + `--allow host=`), the
  `tracked_state` memo, an optional handler, a `SessionMode::{Full,
  ReferenceCheck, Passive}`, the **pinned snapshots** map (`TemplateKey` →
  `Arc<Snapshot>`), and the **active resolution stack** (`Vec<TemplateKey>`).
  `DarkmatterSchemas` gains `with_slot_session(Arc<SlotSession>)`; without one, a
  template layer makes `effective_for` return `SchemaError::UnresolvedSlots`.
  `ComposeOptions::with_slot_session` carries it and
  `compose/schema_validation.rs::effective_schema_for` hands it to the schemas.
  Modes: `Full` resolves with automatic, persisted, once grants, asks via the
  handler, else `ApprovalRequired`; `ReferenceCheck` uses the pinned snapshot or
  resolves with automatic + persisted grants only, never prompts, and an
  unapproved slot defers that property's judgment (as `for_reference_check`
  defers other values today); `Passive` never resolves.
- **N32. Pinning and permissions.** A command pins one snapshot per
  `(TemplateKey, equivalent context)`. A cache entry produced under broader
  permissions is only served after the requesting session's grants cover every
  recorded `ReadRecord` (N21); an entry that needed a once-grant is not served
  to another session. Remote or shell-permission grants never transfer.
- **N33. Cycles.** Nested resolution (a `load_data` contract naming a dynamic
  schema, via `compose/expression/data/contract.rs::prepare_contract` →
  `expand_named_types`) resolves the nested template through the same session
  and pushes its `TemplateKey` on the stack; a key already on the stack is the
  typed `SlotError::Cycle` (`dm.schema.slot_cycle`) positioned at the
  outer slot, and a cache entry in progress is never waited on. Outside a slot
  session, a contract that names a dynamic schema fails with
  `UnresolvedSlots` (no implicit resolution).
- **N34. Trigger payloads (Q6).** `triggers/assemble.rs` (the call at l.310 to
  `resolve_yaml_schema_owned_by`) turns `UnresolvedSlots(pending: Some)` into a
  new `PayloadState::Pending(Arc<PendingSchema>)`; the registry never reports it
  broken, skipped, or empty. Matching stays passive; `resolve_trigger_layers`
  resolves only matched pending payloads through the session at the command's
  authorized boundary; a slot-dependent compilation error is reported at resolve
  time positioned at the slot in the trigger file. Genuinely invalid triggers
  keep their handling. Trigger **match conditions** reject slots (only payloads
  may hold them). Kind definitions (`yaml_documents/registry.rs` ~l.482), the
  baseline (`with_baseline*`, `load_baseline_file`, `catalog.rs`), and the DMLS
  extension layer (`SchemaConfig::extensions`, Q15) reject slots through the same
  `UnresolvedSlots` result, judged after import expansion: a kind claim becomes a
  broken claim `dm.schema.slot_not_allowed_in_kind`; the baseline and an
  extension become `SchemaError::Baseline` with
  `dm.schema.slot_not_allowed_in_baseline`. An ordinary `kind: schema` document
  named by `$schema` is unaffected.
- **N35. Mutation (Q7).** `DataRuntime::invalidate_memo` records no paths, so add a
  `memo_generation()` counter incremented in it. A pinned snapshot stores the
  generation it was pinned at; on each use, if the generation moved, the session
  re-verifies the snapshot's observations (cheap re-hash); any difference marks
  the snapshot stale and the next use re-resolves from the template under current
  grants. A failed re-resolution is a typed error keeping its `dm.data.*` code;
  nothing validates against a stale snapshot. Unrelated writes keep the pin.
  Record in the log that the stricter generation + fingerprint check replaces
  per-path recording, which does not exist.
  **Grants are decided once per run (Q17).** The session memoizes each target's
  grant decision (automatic, once, persisted, or denied) for the whole run, and
  re-resolution reuses it: a file that the run's own approved step made dirty
  keeps the automatic grant it had earlier in the run.
- **N36. M5 replacements (documented).** Passive schemas → "parse, query, export,
  matching, completion, hover, and `md schema validate <schema>` evaluate nothing;
  opening a folder may evaluate pure slots and read clean same-repo files only";
  validator caching → resolution cache + `SHARED_VALIDATOR_CACHE` keyed on the
  concrete lowered schema (static schemas unchanged); DMLS latency →
  snapshots served from memory, resolution only in the worker; extraction
  boundary → `SlotHost` in `simplified-schema`, evaluator in Darkmatter.
- **N37. Diagnostics codes.** New family in `darkmatter/lib/src/markdown/schemas/slots/codes.rs`
  (listed in `SLOT_DIAGNOSTIC_CODES`), spelled as three dot-separated segments
  like the repo's other codes, with `dm.schema.slot_approval_required` fixed by
  Q18: `dm.schema.slot_syntax`, `slot_forbidden_binding`,
  `slot_forbidden_function`, `slot_wrong_shape`, `slot_empty`,
  `slot_pattern_rejected`, `slot_approval_required`, `slot_denied`, `slot_cycle`,
  `slot_pending`, `slot_failed`, `slot_canceled`, `slot_not_allowed_in_kind`,
  `slot_not_allowed_in_baseline`, `slot_grant_store_invalid` (all under
  `dm.schema.`). Data failures keep `dm.data.*`; work limits keep
  `dm.iteration.*`. `SchemaError::UnresolvedSlots` on a validate-document run
  maps to `dm.schema.slot_pending`. Positions follow the
  diagnostic position rule (line/column only when proven, else null).
- **N38. `md schema validate`.** Output schema `file-record.error` gains the value
  `slots`; the record has `valid: false`, `problems: []`, `message`, the
  slot diagnostics in `diagnostics`, and a new optional `grants_needed:
  grant[]`. `outcome: failure`, exit 1. `md schema validate <schema file>` runs
  the passive syntax/admission check only and reports a template as valid-syntax
  with a `slots` count field, never resolving. `md clean` with an unapproved
  slot reports the constraint unavailable (a diagnostic) and does not report the
  document clean. Data outputs print `approval-required` diagnostics (never
  prompt).
- **N39. DMLS design.**
  - Slot resolution runs on its **own** worker, **not** on the registry job lane:
    requests wait on that lane, so a slow `load_data` there would stall
    completion (S5). Results install through a `SlotSnapshots` store with the
    same `seq`/epoch stale rejection `TriggerRegistries::install` uses; a newer
    edit cancels an in-flight job through `SlotHost::is_canceled`; unsaved
    buffers flow through `TriggerRegistries::source_buffers()` and
    `buffers::with` around the job. Per-slot evaluation in DMLS is also bounded
    by about 25 ms of work (S1 measured roughly 40× headroom), tripping
    `dm.iteration.*` at the slot.
  - Dependencies: `Dependencies` gains `slot_reads` (every observation path,
    including missing probes, listings, imported schemas, and `.git/index` and
    `HEAD` for files holding an automatic grant); the watch registration adds
    one absolute-path `FileSystemWatcher` per dependency (replacing registration
    through `register_workspace_watchers(true)`); the `ServerRescan` fallback
    re-checks them by `stat` (Q19). Watches are by path, regardless of extension.
  - Serving: completion/hover/diagnostics read only the in-memory snapshot
    store. While a refresh is running they use the last-good snapshot marked
    stale (hover/completion annotate "stale"; diagnostics report
    `dm.schema.slot_pending` and never a current "valid"). On first load or
    failure: syntax diagnostics + `.pending`/`.failed`/`.approval_required` on the
    slot; never an unconstrained schema.
  - Grants: add `workspace/executeCommand` (`executeCommandProvider` with
    `dmls.grantSlotRead` and `dmls.refreshSchemaSlots`); an `approval_required`
    diagnostic's quick-fix code actions are `Command`s (one per choice in N30).
    `once` in DMLS means in-memory for the server process; `persisted` writes the
    N26 file; both then enqueue a refresh. `dmls.refreshSchemaSlots` also reruns
    after `workspace/didChangeConfiguration` (environment and context changes need
    an explicit refresh or a new captured request; file events cannot see them).
    Evidence: `code_actions.rs` offers only `WorkspaceEdit` quick fixes and no
    `executeCommand` handler exists; a user-level file outside the workspace
    cannot be written by a `WorkspaceEdit` reliably.
  - Pure slots and clean same-repo files may be evaluated on folder open by the
    worker; the request path never does.
- **N40. Shipped schema and corpus.** Change `sandboxing.yaml` line
  `os: enum({{ as_csv(claudine/docs/os.yaml) }}; required) -> …` to
  `os: enum({{ load_data("../../../os.yaml", "string[]") }}; required) -> …`
  (the path from `claudine/docs/research/sandboxing/providers/` to
  `claudine/docs/os.yaml`; `os.yaml` is the list `macOS, Linux, Windows`). The
  shipped-schema corpus (`shipped_schema_corpus.rs`) stays passive: delete
  the `awaiting_enum_slots` closure and its use; the template is admitted
  without resolution. Resolved validation is proved by a separate compose test.
- **N41. Consumers outside Darkmatter.** `effective_for` and the resolve entry
  points can now return `UnresolvedSlots`, which reaches Claudine (15 files) and
  Messenger (3) (`investigations/s7-s8-boundary-breadth.md`). Claudine's CLI
  completion swallows it with `.ok()?`, so a template would silently offer
  nothing: make it report the unresolved state (a pending marker or a typed
  error), and sweep both packages for other swallowed results. Both must compile
  with `cargo check --all-targets`.
- **N42. DMLS enum completion (Q20).** Completion inside the sandbox schema's
  `os[]` enum list returns no items at any position today (S3 side finding).
  Reproduce it first with a failing L1 test on a **static** enum (independent of
  slots), find the cause in `providers/frontmatter.rs` (`enum_members` ~l.2951)
  or its callers, and fix it; the slot work then reuses the same path.

### Input Robustness Matrix (applies to two loaders)

1. **Slot result validator** (N9). Default: any invalid result shape is a located
   `SlotResult` error, runs nothing further, publishes nothing. Exceptions only:
   `[]` contributes nothing (final emptiness is its own error); suggestion
   scalars of every JSON scalar type are accepted; `null` is an error, never
   conflated with `[]`. Control row: the unedited fixture resolves to
   `[macOS, Linux, Windows]`. Cells (one edit each, in-process through
   `PendingSchema::resolve` with a stub host in `simplified-schema`): null,
   scalar string, number, mapping, nested array, `[]`, array with one number
   (enum), `[1]` (enum, every element), `[ "a", {} ]` (suggest), `["a","a"]`
   (dedupe). Darkmatter then needs, per slot kind, the control row and one
   invalid shape through `DarkmatterSlotHost` to prove wiring; the CLI spawn
   test covers the control row and one invalid shape once.
2. **Grant store loader** (N26). Default: any invalid line is a located
   `grant_store_invalid` error (path, 1-based line), the whole file grants
   nothing and the command fails (a lost deny would be unsafe). Exceptions: a
   missing file and an empty file are both an empty store; duplicate identical
   lines are idempotent; allow and deny for the same target resolve to deny;
   comments, blank lines, CRLF, and a leading BOM are accepted. Field shapes
   (each verified once at the loader): absent target (`allow file`), empty
   target after trim, relative path for `file`/`dir`, URL with a fragment,
   unparseable URL, `site` with a path component, unknown verb, unknown kind.
   Control row: a fixture with one line of each kind. One CLI test proves wiring.

Loader code-smell grep before closing the phase's loaders: `rg
'serde\(default\)|unwrap_or_default\(\)|filter_map\(.*as_str|\.ok\(\)'` over the new
`slots/` files; every hit on a load-bearing parse is fixed.

### Wave 1 (parallel, disjoint files)

- [x] **Baseline run**
  - In `simplified-schema/`, `sniff/`, `darkmatter/` run `just test` then
    `just lint`; save pass/fail counts and any pre-existing failures in
    `implementation-log.md` under "Baseline" so later red tests are attributable.
  - Prerequisite: working tree already holds uncommitted render-surface work;
    do not touch it.
- [x] **Implementation log skeleton**
  - Write `implementation-log.md` headings: Baseline, Departures (seed it with
    N26 location, N35 mechanism, N6 staging), Decisions applied, Measurements.
- [x] **Fixtures** (partly written by the first run: review the existing files
  in both `dynamic_slots/` folders against this list and N4/N6 rather than
  recreating them; add value-position cases for Q16)
  - `simplified-schema/lib/tests/fixtures/dynamic_slots/`: `schema-with-slot.yaml`
    (kind: schema, one enum slot mixing a literal), `suggest-slot.yaml`,
    `imported-types.yaml` (a slot in an imported type), `os.yaml`.
  - `darkmatter/lib/tests/fixtures/dynamic_slots/`: a miniature repo builder helper
    (`git init` via the existing `CliProcessFixture` conventions, files tracked
    and committed, plus untracked/ignored/dirty variants) and a copy of the
    sandboxing schema shape with its `os.yaml`.
  - Real-tool fixture rule: each matrix cell edits one thing in the control
    fixture.
- [x] **DMLS enum completion (N42, Q20)**
  - Independent of slots and of every later phase (disjoint files under
    `darkmatter/dmls/`): failing L1 test on the static sandbox-shaped enum, then
    the fix.
- [x] **Validation checkpoint**
  - Baseline recorded; fixtures compile into the test crates (`just test` in both areas
    unchanged-green).

## Phase 2 — Slot syntax and the passive template (simplified-schema)

Goal: `enum(...)` / `suggest(...)` parse slots; templates round-trip, export,
and query without evaluating anything; lowering fails with a typed error.

### Wave 2 (serial base)

- [x] **AST types**
  - Add `Slot`, `DynamicKind`, `DynamicElement`, `DynamicList`,
    `Constraint::Dynamic`, `ValueSlotKind`, `Constraint::DynamicValue` per N3 in `simplified/types.rs`; update `keyword()`;
    re-export from `simplified/mod.rs` and `lib.rs`.
  - Fix every exhaustive `match` the compiler reports (known: `types.rs`
    `keyword()`, `narrowing.rs` ~l.173/194, `darkmatter` `triggers/grammar.rs`
    ~l.599).
- [x] **Scanner API**
  - Add `scan_expression_at` (N5) with doctest and tests in
    `simplified-schema/lib/tests/l1/expression_scanner.rs` (quoted `{{`, nested
    braces, unclosed, escaped opener).

### Wave 3 (parallel after Wave 2; one task per file group)

- [x] **Grammar**
  - In `simplified/grammar.rs`: `parse_enum_members` and the `suggest` arm of
    `parse_one_constraint` recognise slots (N4). Build `Constraint::Dynamic`
    only when a slot is present; literals before/after keep their normal tokens;
    the `; required` / `default(...)` tail still parses. Keep the exact-type and
    one-`suggest`-per-definition rules (`second_suggestion_span`); apply N7 to
    dynamic lists only.
  - Value positions (N4b): `literal`, `default`, `min`, `max`, `pattern` accept a
    whole-argument slot and build `Constraint::DynamicValue`; partial mixing is
    the positioned "whole argument" error.
  - Reinterpretation for deferred references: extend
    `reinterpret_suggestion_candidates` to retarget `DynamicList`.
  - Tests: unit tests in the file plus `declaration_grammar.rs`: mixed order,
    multiple slots, quoted `'{{ x }}'` stays literal, `abc{{x}}` keeps today's
    behavior, incomplete element error, unterminated slot, parse-error offset.
- [x] **Serialize / export**
  - `simplified/serialize.rs` (`Constraint::Dynamic` arm near l.491/509): export
    literals with their quoting and slots as authored `{{ expr }}`; round-trip
    export → reparse equals the template. Test with the three fixtures.
- [x] **Source spans**
  - `simplified/source.rs`: make `split_top_level`, `scan_constraint_groups`,
    `constraint_call_arguments`, `inline_object_end` and the span projection
    (`project_suggestion_spans`, `project_atom_suggestions`,
    `project_located_*_suggestions`) brace/quote aware through `scan_expression_at`
    and project `Slot.span` / `expression_span` (and slot-produced candidate
    spans) through plain, quoted, and block-scalar YAML carriers and Markdown
    frontmatter offsets, without normalizing line endings.
  - Tests: spans point at the authored `{{` in plain, `'…'`, `"…"`, `|`, `>`
    carriers, and at a slot in an imported file.
- [x] **Lowering error and query**
  - `simplified/convert.rs`: `Constraint::Dynamic` makes lowering return
    `SchemaError::UnresolvedSlots` (N10) for `Dynamic` and `DynamicValue` at the `Constraint` match sites
    (~l.1934, 2013, 2117, 2452); add the variant to `errors.rs` with a
    `Display`/located-position mapping like `Convert`.
  - `simplified/query.rs` and `lint.rs`: expose pending information
    (`pending_slots`), lint only literal suggestion candidates, never report a
    partial list as complete.

### Wave 4 (parallel)

- [x] **Consumer sweep**
  - `resolve.rs` (~l.2353, 3204–3320) and `resolve/recursive.rs` (l.87, 691, 719,
    987): treat `Dynamic` suggest like `Suggest` for the one-list-per-property and
    marker checks; `completion.rs`, `coercion/engine.rs`, `format.rs`,
    `subtype.rs`, `narrowing.rs`: operate on concrete schemas only; where a
    template can reach them, return the typed unresolved result (enum subtype /
    coercion of an unresolved enum is `UnresolvedSlots`, never "no members").
    Grep list to clear: `rg 'Constraint::(Members|Suggest|Literal|Default|Min|Max|Pattern)' simplified-schema/lib/src`.
  - Completion on a template shows the literals plus a pending marker.
- [x] **`SimplifiedSchema` passive behavior**
  - `schema/layer.rs`, `export.rs`, `view.rs`, `mod.rs`: N13 passive half —
    factories accept valid templates, `lowered = Err(UnresolvedSlots)`,
    `is_resolved`, `pending_slots`, typed errors from `to_json_schema`,
    `validate_*`, `layer_*`; YAML export preserved. Tests in
    `tests/l1/schema_object.rs`.

### Validation checkpoint (end of Phase 2)

- [x] `just test` and `just lint` in `simplified-schema/` pass, including the
  minimal-feature (no `file`) build and `work-counters`.
- [x] Static-schema parity: the existing shipped corpus and `schema_object` suites
  pass unchanged; a test proves a static `enum`/`suggest` still lowers to
  `Constraint::Members` / `Suggest` (never `Dynamic`).
- [x] A counter/read-nothing test proves parse, query, export evaluate nothing and
  read nothing.

## Phase 3 — Resolution hook and concrete snapshot (simplified-schema)

Goal: `PendingSchema::resolve` and `SimplifiedSchema::resolve_slots` produce
immutable concrete schemas through a host-neutral `SlotHost`.

### Wave 5 (parallel)

- [x] **`slots` module**
  - `simplified-schema/lib/src/slots.rs` per N8–N9 and N11: keys, values scope,
    `collect_slots`, `has_dynamic`, `substitute` (flatten, dedupe, enum/suggest
    shapes, value-position shapes, numeric text, positions), `SlotHost` with
    `is_canceled`, `SlotRequest`, `SlotFailure`, `SlotResolutionError`
    (including `Canceled`), `SchemaError::SlotResult`, the pattern cap (N6a), and
    `read_sources` (N12b).
  - Unit tests for every cell of Input Matrix 1 with a stub host, one valid and
    one invalid shape per value position, `min > max`, each pattern-cap
    refusal (lookaround, backreference, over 4 KiB, over `size_limit`), and
    cancellation publishing nothing.
- [x] **Buffer bytes API**
  - `resolve/buffers.rs`: `pub fn read(path) -> io::Result<Vec<u8>>` (N18) with
    tests inside and outside `with`.

### Wave 6 (parallel)

- [x] **`resolve.rs` integration**
  - Origin stamping (N12), `apply_active` at the lowering sites, embedded
    rejection, `PendingSchema` and replay closures at the six entry points (N10),
    `PendingSchema::resolve`. Imports/examples/searched lists are identical
    between a cold concrete resolve and a template-resolve (test).
  - Recursive named types and slots in imported + recursive types: tests.
- [x] **`SimplifiedSchema::resolve_slots`**
  - N13 resolve half: replay `resolve_authored` under `with_values`; new concrete
    schema, template unchanged, repeated resolves independent; failed resolution
    publishes nothing. Provenance includes the same import/searched edges.

### Wave 7 (parallel)

- [x] **Library tests**
  - `tests/l1/dynamic_slots.rs`: passive checks, resolve success, each
    Matrix 1 cell, mixed literals + slots, `; required` tail, duplicate
    normalization, imported + recursive, layering after resolution equals layering
    of equivalent static schemas, whole-property precedence preserved.
  - Guards: `dependency_direction_guard.rs` and ownership guards still pass; no
    `darkmatter` mention in the new module.
- [x] **Library docs**
  - `simplified-schema/docs/topics/language/types.md` (slot syntax, result rules,
    every N6 position with its result shape, the pattern cap, and whole types marked **planned**), `simplified-schema-struct.md` (object API),
    `host-capabilities.md` (the `SlotHost` hook), `validation.md`/`diagnostics.md`
    (unresolved errors); README stays an overview.

### Validation checkpoint (end of Phase 3)

- [x] `just test` and `just lint` in `simplified-schema/` (all feature builds).
- [x] `cargo check --all-targets -p darkmatter` compiles (new `SchemaError` /
  `Constraint` variants handled in Darkmatter's exhaustive matches).

## Phase 4 — Admission, evaluator, fingerprints (darkmatter lib)

Goal: a Darkmatter host that admits slots passively and evaluates them on the
existing engine, with fingerprints and typed causes. No approval or caching yet:
the evaluator takes a `ReadGate` that defaults to deny-all for tests that read.

### Wave 8 (parallel, disjoint files)

- [x] **Admission and classes**
  - `schemas/slots/admission.rs` (N14), the `SlotClass` registration fact and
    `slot_class` walk (N20), and codes (N37). Walk every branch and every
    callback body. Tests: `fm.x`, `fm['x']`, bare `title`, a host global,
    `ctx.cwd`, `ctx.self`, `ctx.id`, `current.x` outside a callback, each Q10
    function, each forbidden in a ternary's untaken branch and in a `||` operand;
    allowed `ctx.today`, `env.HOME`, `current.*`/`local.*` inside a callback.
- [x] **Captured clock, timezone, `PATH` (N20, Q11)**
  - Move the 14 live-clock date functions and `has_command`/`has_binary` onto
    the captured request context; registration facts corrected. Tests: two
    evaluations in one request see one clock; `has_command` ignores a changed
    process `PATH`. Document behavior unchanged otherwise.
- [x] **Remote-read discovery (N20, Q18)**
  - Derive `REMOTE_READ_FUNCTIONS` from the facts; fold case and underscores in
    pre-fetch matching. Test: `loaddata("https://…")` is discovered up front.
- [x] **Data runtime observations**
  - N17, N18, N22, N35-generation: `Observation` log, hash at read, stored in
    `LoadedSource`, `read_mark`/`observations_since`, `record_entry`,
    `memo_generation`, `load.rs` through `buffers::read`.
  - Every input-reading function records (N17, Q18), including listings,
    `load_data`'s not-found probe, and missing contract imports.
  - Tests: second `load_data` in one run (memo hit) re-records; failed read keeps
    its dependency; missing probe recorded; one test per previously unrecorded
    function family; generation increments on `invalidate_memo`; document
    `load_data` results unchanged (its read set now gains the missing probes).
- [x] **Gate hook**
  - `ReadGate` trait, `ResolutionContext::slot_gate`, `authorize_slot_read` at
    the read sites, before the memo lookup (N28). Tests with a recording gate:
    every read-side function asks before reading; a file the document already
    loaded is still gated for the slot; `None` gate changes nothing.

### Wave 9 (after Wave 8)

- [x] **Evaluator and host**
  - `evaluator.rs` (N15, N16), `DarkmatterSlotHost` implementing
    `admit`/`resolve`, typed-cause preservation (N19), context-digest capture
    (N17).
  - Tests: evaluating `load_data("os.yaml","string[]")` returns the array; a
    missing file keeps `dm.data.*` and `data_location`; a malformed file keeps
    `dm.data.malformed` with line/column; work limit trips
    `dm.iteration.visit_limit`; relative paths resolve against the defining
    file, including a slot in an imported schema in another directory.
- [x] **Validation checkpoint**
  - `just test`, `just lint` in `darkmatter/`; `rg` loader smell grep (see Phase 1
    Matrix note) clean for `slots/`.

## Phase 5 — Approval model (sniff, darkmatter lib, darkmatter-cli)

Goal: the Q5 model works end to end for library and CLI callers.

### Wave 10 (parallel)

- [x] **Sniff tracked state (N25)**
  - Add `TrackedState` and `GitRepo::tracked_state` in `sniff/lib/src/filesystem/git/`,
    plus docs in the `sniff` topic page and skill note. Tests build real repos
    (tracked-clean, staged-modified, worktree-modified, untracked, ignored,
    conflicted, outside, symlink inside pointing out is handled by the caller's
    canonicalization).
  - Run `just test` and `just lint` in `sniff/`.
- [x] **Grant store (N23, N26)**
  - `slots/grants/store.rs` (parser, writer, `GrantBook`) at
    `<home>/.config/darkmatter/schema-read-policy` (N26), `matching.rs`
    (specificity, deny-wins). Tests: Matrix 2 (all cells), control row, atomic
    append creating the parent directory, persisted + once allow and deny,
    no-home.
- [x] **Auto grant**
  - `slots/grants/auto.rs` (N24) taking the observation root, `canonicalize_simplified`,
    `tracked_state`, the blob-hash comparison on bytes read, and buffers. Tests
    per refusal: dirty, untracked, ignored, LFS pointer, a file changed between
    status and read (hash mismatch), symlink-escaping (Unix gate `#[cfg(unix)]`,
    plus a junction-free Windows variant skipped with an explicit `cfg`), other
    repo, no repo, defining schema outside the request's repository (Q13),
    unsaved buffer differing from disk. Probes and listings (Q11): `file_exists`
    automatic for an untracked in-repo path; `find_files` never automatic.
    Absent targets (Q21): `load_data` of a missing in-repo file needs no grant
    and fails with the typed not-found error; a missing file outside the
    repository needs a grant.

### Wave 11 (after Wave 10)

- [x] **Gate implementation and remote policy**
  - `grants/gate.rs`: `ReadGate` over `GrantBook` + auto + handler; remote runtime
    per N27 (own `FetchPolicy`, scheme/port/query checks).
- [x] **Targets and pre-flight**
  - `grants/targets.rs` (N29); `DarkmatterSchemas::pending_templates_for`;
    `compose_preflight_approvals_with` raises the combined batch; handler
    extension (N30). Tests: a document with `$schema` template + a shell command
    produces one batch holding both; targets already granted are not asked;
    a pending-but-unselected trigger payload yields no request; a slot under a
    property redeclared by a higher layer yields no request (Q14); a target
    built from literals and pure functions is collected up front.

### Wave 12 (after Wave 11)

- [x] **CLI prompt**
  - `cli/src/approval.rs`: numbered slot prompt in the same batch, including
    "deny for this run"; end of input or cancel ⇒ deny for this run; no TTY ⇒ no
    prompt, data output never prompts. Test the prompt with `approve_with_io`
    (no window, scripted IO).
- [x] **Validation checkpoint**
  - `just test`/`just lint` in `sniff/` and `darkmatter/`.
  - Approval matrix tests green (spec "Verification": clean grant, each refusal,
    every scope, deny, batched prompting beside shell, computed targets,
    `approval-required` on data outputs and for library callers, repository
    content cannot grant: a committed `.config/darkmatter/schema-read-policy` in
    the repository root and document directory is ignored, and no `.dmls.toml`
    or editor setting can relocate the store).

## Phase 6 — Sessions, cache, schema integration, triggers, staleness

Goal: resolution is cached, pinned, wired into `DarkmatterSchemas`, and honors
Q4/Q6/Q7.

### Wave 13 (parallel)

- [x] **Cache and snapshot types**
  - `slots/cache.rs` (N21), `slots/session.rs` (N31–N33, N35): key building,
    observation re-verification, permission check against the requesting session,
    failure caching, volatile bypass, read-set replay (N22), stack for cycles,
    pinned map, `reset_process_caches_for_bench` registration, work counter
    `SlotResolution` (via `note_work!`) counting evaluator runs and validator
    compiles.
  - Tests: warm hit = zero evaluator runs and zero compiles; edit data file ⇒
    miss; edit imported schema ⇒ miss; create a missing probed file ⇒ retry;
    context (`ctx`/`env`) change ⇒ miss; permission narrowing ⇒ miss; failure
    cached then repaired ⇒ recovered; read set equal between cold and warm; volatile slot
    re-evaluates in a new command, not within one; a cross-command hit `stat`s
    and does not rehash unchanged inputs, and a changed `stat` rehashes (Q19).
- [x] **`kind`/baseline prohibitions (N34)**
  - `yaml_documents/registry.rs`, `yaml_documents/semantic.rs`, baseline loaders,
    `catalog.rs`, the DMLS extension layer (Q15), and trigger match conditions:
    map `UnresolvedSlots` to the codes in N34, including slots through imports.
    Tests per site.

### Wave 14 (after Wave 13)

- [x] **`DarkmatterSchemas` integration**
  - `with_slot_session`; in `effective_for_with_override` (mod.rs ~l.700) a
    template document layer resolves through the session before
    `merge_trigger/merge_baseline`; no session ⇒ `UnresolvedSlots`; `validate_yaml_document`
    follows the same rule. `ComposeOptions::with_slot_session`,
    `compose/schema_validation.rs::effective_schema_for` passes it down.
    `EffectiveSchema` is built only from concrete snapshots (assert in tests that
    no `Constraint::Dynamic` reaches it).
  - Contract recursion (N33): `contract.rs::prepare_contract` resolves nested
    dynamic schemas through the session; cycle test with two schemas that load
    each other.
- [x] **Trigger payloads pending (N34)**
  - `PayloadState::Pending`; `resolve_trigger_layers` resolves matched pending
    payloads; `TriggerRegistry` listing/`md schema triggers` shows "pending".
- [x] **Mutation staleness (N35)**
  - Session generation check; test with a lifecycle/shell effect that rewrites the
    data file: next validation re-resolves with the grant decided earlier in the
    run (Q17), even though the file is now dirty; unrelated write keeps the pin;
    failed re-resolve keeps `dm.data.*`.

### Wave 15 (parallel)

- [x] **Integration tests**
  - `darkmatter/lib/tests/l1/dynamic_schema_slots.rs`: end to end through
    `Markdown::compose` with a real repo fixture and the sandboxing-shaped schema:
    valid and invalid document values, passive default, pinned snapshot,
    trigger selected vs unselected (unselected reads nothing — assert read set
    empty), `effective_schema_layering.rs` equality unchanged.
  - Consumers (N41): Claudine's CLI completion reports an unresolved template
    instead of `.ok()?`-swallowing it; sweep Claudine and Messenger for other
    swallowed results; `cargo check --all-targets -p claudine -p claudine-cli -p messenger`.
- [x] **Validation checkpoint**
  - `just test`, `just lint` in `darkmatter/`; the existing `SHARED_VALIDATOR_CACHE`
    / `reset_process_caches` tests still pass.

## Phase 7 — CLI surfaces and the shipped schema

### Wave 16 (parallel, disjoint files)

- [x] **`md compose`**
  - `cli/src/commands/compose.rs`: build the `SlotSession` from the command's
    `DataRuntime`, request context, `--allow host=` hosts, the grant book, and the
    handler (only when interactive and not a data output); run slot collection in
    the pre-flight even when no shell feature is enabled; read set equals what a
    cold run reports; data output prints `dm.schema.slot_approval_required`
    diagnostics with the grants needed and stays silent on stderr.
- [x] **`md schema validate`**
  - `cli/src/commands/schema/validate.rs` per N38: this command does not use the
    compose pre-flight walk, so it gets its own small collect-and-approve step
    (N29's collector + the same handler) before resolving; document runs then
    resolve under the same gate (`error: slots`, `valid: false`, exit 1 when
    unavailable); a schema file argument is the passive check only.
- [x] **`md clean`, `md graph`, `md validate refs`**
  - `md clean`: the same collect-and-approve step as `md schema validate`;
    unapproved slot ⇒ constraint unavailable diagnostic, document not reported
    clean. `graph`/`validate refs` use `SessionMode::ReferenceCheck`,
    never prompt, defer the property (as `ComposeOptions::for_reference_check`).

### Wave 17 (after Wave 16)

- [x] **Output schemas and conformance**
  - Add the new codes, `slots` error value and `grants_needed` to every output
    schema file that carries `diagnostic` (all 17 in `cli/schemas/output/`; the
    shared-diagnostic test requires identical types) and to
    `schema-validate.yaml`; conformance tests in `cli/tests/l1/output_commands.rs`.
- [x] **Shipped schema and corpus (N40)**
  - Edit `claudine/docs/research/sandboxing/providers/sandboxing.yaml`; delete
    `awaiting_enum_slots` from `shipped_schema_corpus.rs`; add the passive-admission
    assertion for the file.
- [x] **End-to-end tests**
  - A normal `md compose` run (spawned `md`, isolated temp repo copy of the real
    sandboxing schema and `os.yaml`, `MD_DRY_RUN` set, no window): control row
    valid; document with `os_support: [BeOS]` fails with the enum error naming the
    resolved members; data output without grant on a dirty `os.yaml` returns
    `approval-required` and exit 1; clean file resolves silently; read set includes
    `os.yaml`. `md schema validate` of the schema file is passive. One spawn for
    Matrix 2 (corrupt store ⇒ `grant_store_invalid`).
- [x] **Validation checkpoint**
  - `just test` and `just lint` in `darkmatter/` (the spawned-`md` tests are L1 in
    `cli/tests/l1/`; no L2 recipe is needed).

## Phase 8 — DMLS

### Wave 18 (parallel)

- [x] **Worker lane and snapshot store**
  - N39 first bullet: a dedicated slot worker (not the registry job lane),
    `SlotSnapshots` with seq-based rejection, cancellation of superseded jobs,
    the DMLS work bound, buffer scoping; job enqueued on schema open/change and
    on refresh/grant; last-good retention; atomic publish.
- [x] **Watches**
  - N39 second bullet: `Dependencies.slot_reads`, per-path watcher registration and
    `ServerRescan` `stat` re-check, `.git/index`/`HEAD` watches for automatic
    grants, event → invalidation → refresh.

### Wave 19 (after Wave 18)

- [x] **Serving**
  - Diagnostics (`diagnostics/frontmatter.rs`, standalone-schema path): syntax,
    `.pending`, `.failed`, `.approval_required`, `.forbidden_binding` positioned at
    the slot; completion/hover (`providers/frontmatter.rs` `enum_members`
    l.2951, `suggestion_completions` l.1419, `overlay/suggestions.rs`) read the
    snapshot or literals + pending marker; stale marker while refreshing.
- [x] **Grant commands**
  - `executeCommandProvider`, `dmls.grantSlotRead`, `dmls.refreshSchemaSlots`,
    quick-fix `Command` code actions per diagnostic choice (N30), user-level store
    write, in-memory once-grants; refresh on `didChangeConfiguration`.

### Wave 20 (parallel)

- [x] **DMLS tests**
  - L1 using `LspWorkspace`/`LspFixture` and the registry gates
    (`start_with_registry_gate`): request-path no-I/O/no-eval (extend
    `no_side_effects.rs`), stale worker result rejected, unsaved data buffer
    refreshes the snapshot, saved dependency change refreshes, missing file
    creation refreshes, last-good served stale, first-load failure retains syntax
    diagnostics (never unconstrained), code action ⇒ command ⇒ store written ⇒
    refresh, once-grant not persisted, no prompt or block, a slow slot job does
    not delay a completion request, a newer edit cancels the in-flight job,
    staging a change to a granted file (`.git/index` event) refreshes it.
- [x] **Validation checkpoint**
  - `just test`, `just lint` in `darkmatter/` (DMLS crate included).

## Phase 9 — Docs, skills, measurement, closure

### Wave 21 (parallel)

- [ ] **Darkmatter docs**
  - Update `darkmatter/docs/topics/schemas/` (`schema-constraints.md`,
    `authoring-schemas.md`, `schema-validation.md`, `schema-triggers.md`,
    `dmls-schema-support.md`), `darkmatter/docs/cli/schema.md`, `compose.md`,
    `policy.md`, `darkmatter/docs/topics/loading-data.md`, `darkmatter/docs/lsp/features.md`
    and `architecture.md`, `docs/topics/caching.md`. Audience: a developer new to
    the repo; lead with the task ("make an enum from a data file"), one example per
    rule, Mermaid for the passive→resolve→compile flow and the approval flow.
    Document the grant store location and format, every slot position and its
    result shape (N6), the pattern cap, the `stat` re-check limitation (Q19), and
    whole-type slots as **planned**. Docs never
    name the feature/spec directory. No crate is added, so `docs/dependencies.md` files are not touched.
- [ ] **Skills**
  - `.claude/skills/darkmatter/schema.md`, `compose.md`, `dmls.md`, `cli.md`,
    `requests.md`; `.claude/skills/simplified-schema/SKILL.md`; the `sniff` skill
    note for `tracked_state`. Add the slot boundaries to `darkmatter/SKILL.md`'s
    non-negotiable list (passive default, explicit resolution, home-only grants).
- [ ] **Doc drift pass**
  - Re-read every changed symbol's `///`/`//!` and inline comments; fix drift; list
    any in the implementation log. No narration comments.

### Wave 22 (serial)

- [ ] **Follow-up spec for whole-type slots (Q16)**
  - Create `darkmatter/features/2026-11-08-dynamic-whole-type-slots/spec.md`
    (`kind: feature`, `status: draft-spec`, `related:
    2026-11-05-dynamic-schema-types`), stating the S8 findings for whole types
    and Q16's import rule. Not under `_unscheduled`.
- [ ] **Timing sample against the S1 budgets**
  - One host, load below 4 (record it), static and slot fixtures interleaved:
    cold and warm `md schema validate`/`md compose`, DMLS completion and hover
    latency; compare with N1's budgets and with the simulated numbers in
    `investigations/s1-s2-s3-s6-performance.md`. Record the numbers in
    `implementation-log.md` under Measurements. A missed budget is reported to the
    author there; make no speculative code change.
- [ ] **Final verification**
  - `just test` and `just lint` in `simplified-schema/`, `sniff/`, `darkmatter/`;
    `cargo check --all-targets -p claudine -p claudine-cli -p messenger`; run the grep audit
    from Phase 1 over `slots/`; confirm Definition of Done items one by one and tick
    them; fill the implementation log (Decisions applied, Departures). Do not commit,
    do not move the spec; state "implementation complete, ready for review".

## Reconciliation (2026-11-08)

The first version of this plan was written before the investigations reported
and before Rulings Q8–Q20. These rules changed; everything else stands.

| Rule | Was | Now | Source |
| --- | --- | --- | --- |
| N1 | no spikes; timing is explanation only | spikes ran; Phase 9 measures against the S1 budgets | Investigation outcomes |
| N2 | in-memory cache | unchanged, plus `stat` re-check across commands | S2, Q19 |
| N3, N4, N6, N9 | enum/suggest only; other positions **planned** | `literal`, `default`, `min`/`max`, `pattern` delivered; whole types planned with a follow-up spec | S8, Q16 |
| N6a | — | pattern cap: linear engine only, size limits | S8, Q16 |
| N11 | no cancellation | `SlotHost::is_canceled` | S7 |
| N12b | — | schema-file text recorded at read for the key | S7 |
| N14 | `current` always rejected; `local` always allowed | `current`/`local` only inside a callback; host globals, `ctx.cwd`, identity and nonce `ctx` values rejected | Q12 |
| N17 | only `REMOTE_READ_FUNCTIONS` record | every input-reading function records, including listings, `load_data`'s not-found probe, missing contract imports | S4, Q18 |
| N20 | name table; "no function is banned"; volatile resolves on every use | registration fact; Q10 functions rejected; clock/timezone/`PATH` captured; volatile once per command; `REMOTE_READ_FUNCTIONS` derived and alias-folded | S4, Q10, Q11, Q18 |
| N21 | re-hash every observation on lookup | `stat` first, rehash on change | S2, Q19 |
| N23, N30 | deny always persisted | deny once or persisted; deny-for-run on cancel or end of input | Q18 |
| N24, N25 | status sets only | blob hash of bytes read vs index and `HEAD`; Q13 defining-schema rule; Q11 probes and listings; per-path status query | S5, Q11, Q13 |
| N26 | `~/.darkmatter-slot-grants` | `<home>/.config/darkmatter/schema-read-policy`, not relocatable | Q8 |
| N27 | shared hosts list | separate slot fetch policy stated per Q9 | Q9 |
| N28 | gate at read sites | gate before the shared memo lookup | S5 |
| N29 | string-literal targets; all layers | literal-or-pure targets; only the effective schema's slots; slot decisions before shell decisions | S5, Q14 |
| N34 | kind and baseline prohibitions | plus the DMLS extension layer and trigger match conditions | Q15, S7 |
| N35 | re-resolve under current grants | grants decided once per run and reused | Q17 |
| N37 | `dm.schema.slot.*` | `dm.schema.slot_*`, including `slot_approval_required` | Q18 |
| N39 | registry job lane | dedicated worker, cancellation, DMLS work bound, `.git` watches, `stat` rescan | S3, S5, Q19 |
| N41 | Claudine compile check only | Claudine completion fix; Messenger check | S7 |
| N42 | — | DMLS enum completion fix | Q20 |