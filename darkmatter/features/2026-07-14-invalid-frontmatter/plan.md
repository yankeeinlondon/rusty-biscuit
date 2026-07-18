---
agent: codex/
total_phases: 8
created: 2026-07-14
phase: 2
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/benches/clean_hot_paths.rs
  - darkmatter/lib/Cargo.toml
docs_updated_during_phase_1:
  - darkmatter/features/2026-07-14-invalid-frontmatter/plan.md
docs_created_during_phase_1:
  - darkmatter/features/2026-07-14-invalid-frontmatter/decisions.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/impact-report.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/baselines.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/acceptance-matrix.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/baselines/no-fm.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/baselines/clean-fm.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/baselines/invalid-reserved.md
  - darkmatter/features/2026-07-14-invalid-frontmatter/baselines/coercible.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/src/span.rs
  - biscuit-file/lib/src/yaml/mod.rs
  - biscuit-file/lib/src/yaml/types.rs
  - biscuit-file/lib/src/yaml/location.rs
  - biscuit-file/lib/src/yaml/analyze/mod.rs
  - biscuit-file/lib/src/yaml/analyze/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/analysis.rs
  - biscuit-file/lib/src/yaml/analyze/edit_set.rs
  - biscuit-file/lib/src/yaml/tests/mod.rs
  - biscuit-file/lib/src/yaml/tests/location.rs
  - biscuit-file/lib/src/yaml/tests/retained_source.rs
  - biscuit-file/lib/src/yaml/analyze/tests/mod.rs
  - biscuit-file/lib/src/yaml/analyze/tests/analysis.rs
  - biscuit-file/lib/src/yaml/analyze/tests/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/tests/edit_set.rs
  - biscuit-file/lib/tests/span_compat.rs
  - darkmatter/lib/src/markdown/span.rs
  - darkmatter/lib/tests/span_compat.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-07-14-invalid-frontmatter/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - biscuit-file
  - darkmatter
---

# Invalid Frontmatter — Execution Plan

This plan derives v1 scope from [`spec.md`](./spec.md) and uses
[`research.md`](./research.md) only as an implementation opportunity catalog.
It proceeds source-first: build a reusable schema-agnostic YAML engine in
`biscuit-file`, add Darkmatter's schema-aware layer, and then integrate both
into `md clean` without inspecting YAML fences in the Markdown body.

## Success Criteria

- [ ] `md clean` examines only a non-empty leading frontmatter block; fenced
  YAML and all other body content remain outside the YAML diagnostic engine.
- [ ] Source normalization, parse-equivalent whitespace cleanup, the ratified
  no-schema reserved-indicator case, and schema-proven scalar quoting are the
  only repairs eligible for automatic application in v1.
- [ ] Every `deterministic-find-non-deterministic-solution` and
  `non-deterministic-find` result is report-only, never changes source, and
  never changes the existing `md clean` exit-code contract.
- [ ] `biscuit-file` owns the source-first schema-agnostic analyzer, shared
  byte-span/diagnostic/repair types, edit application, and value-equivalence
  checks; Darkmatter owns effective-schema resolution and schema-aware
  diagnostics.
- [ ] File, stdin, ordinary stdout, `--save`, STDERR suggestions, and `--json`
  behavior follow the Phase 1 contract, including byte preservation for every
  range outside an accepted edit or the existing Markdown body cleanup.
- [ ] No-frontmatter documents bypass YAML analysis, schema resolution, and
  trigger discovery; already-clean frontmatter is parsed once and incurs no
  candidate reparse.
- [ ] Focused tests, corpus/mutation tests, cross-platform line-ending tests,
  package suites, lints, doctests, and before/after benchmarks satisfy the
  ratified definition of done.

## Current-Code Constraints

- `darkmatter/cli/src/commands/clean.rs` currently calls `load_markdown` before
  cleanup, so malformed frontmatter fails before any repair can run.
- `Markdown::as_string()` serializes the full frontmatter map with
  `serde_yaml_ng`; the new clean path cannot use it to reassemble an untouched
  frontmatter block because comments, quoting, and presentation would drift.
- `extract_frontmatter_block` already locates raw YAML and document spans
  without parsing YAML and is the correct frontmatter boundary authority.
- `YamlError::Parse` already owns `serde_yaml_ng::Error`, so the groundwork is
  a stable structured-location accessor/model rather than replacing a lost
  error object.
- `Yaml` currently guarantees successful parsing. A source-first API is needed
  for the flagship invalid-YAML case; `Yaml::diagnose()` alone cannot analyze
  input that cannot construct a `Yaml` value.
- `EffectiveSchema::validate_with_positions` coerces values before validation.
  Schema-proven quoting needs an explicit raw/non-coercing type-mismatch path
  so `1.20` is not silently accepted as a schema string before repair analysis.
- `md compose` currently has `--baseline-schema`, `--no-baseline-schema`, and
  `--no-trigger-schemas`; `md schema validate` separately has `--schema`.
  `md clean --schema` semantics must be pinned rather than described as an
  already-existing compose flag.

## Phase 1: Contract Freeze, Impact Analysis, and Baselines

- [x] Refresh the GitNexus index and read the repository context before source
  edits; if refresh cannot complete within the non-interactive command limit,
  record the stale-index state and verify every graph result against current
  source.
- [x] Run upstream GitNexus impact analysis for every existing symbol expected
  to change, including `Yaml`, `YamlSource`, `YamlError`, Darkmatter's
  `SourceSpan` re-export, `EffectiveSchema` validation entry points,
  `extract_frontmatter_block`, `run_clean`, `apply_cleanup`, the clean CLI
  variant, and top-level `--save` dispatch. Record direct callers, affected
  processes, and risk; stop and warn before editing any HIGH or CRITICAL target.
- [x] Capture pre-change functional baselines for no-frontmatter input,
  clean-frontmatter input, malformed `title: @daily-report`, schema-coercible
  `release: 1.20`, stdin, `--save`, verbose delta reporting, and current exit
  codes/channels.
- [x] Capture repeatable performance baselines for the two required hot paths:
  a representative document with no frontmatter and a representative document
  with already-clean frontmatter.
- [x] Create `decisions.md` and ratify a source-first public API that can return
  diagnostics for both parseable and unparseable YAML while retaining
  `Yaml::diagnose()` as a convenience for successfully parsed values.
- [x] Ratify a per-repair safety matrix instead of applying one contradictory
  gate to every class: define the proof required for parse-equivalent edits,
  schema-proven invalid-to-valid quoting, and invalid-YAML parse-recovery
  quoting. Explicitly resolve how the flagship case is proven when there is no
  original parsed value to compare.
- [x] Ratify the exact bounded grammar for the no-schema reserved-indicator
  repair, including which indicators and mapping/sequence contexts qualify,
  how the lexeme boundary is found, and which ambiguous cases remain
  report-only.
- [x] Ratify the clean pipeline order. Recommended order: raw input and
  frontmatter extraction; schema-agnostic diagnostics/accepted repairs;
  Markdown parse; effective-schema diagnostics/accepted repairs; existing body
  cleanup; raw-preserving document assembly.
- [x] Ratify raw-source ownership for `YamlSource::Path`. Recommended decision:
  retain the bytes/text read at construction so diagnostics and repairs cannot
  observe a second, TOCTOU-raced file version.
- [x] Ratify CLI behavior: “default auto-apply” versus existing `--save`
  semantics, stdout versus in-place writes, `--json` output channel/envelope,
  coexistence of cleaned Markdown and JSON diagnostics, and verbose delta
  behavior when the original frontmatter cannot be parsed.
- [x] Ratify clean schema-option semantics and conflicts for
  `--baseline-schema`, `--no-baseline-schema`, `--schema`, and
  `--no-trigger-schemas`, including whether `--schema` replaces or layers over
  the document's own `$schema` and whether schema flags are meaningful for
  stdin.
- [x] Ratify unconstrained-key behavior, exact/stable JSON field names and enum
  spellings, line/column indexing rules, idempotency expectations, BOM scope,
  line-ending scope, and the corpus licensing/pinning strategy; do not promote
  any undecided item from the spec's Open Questions without this sign-off.
- [x] Convert the ratified decisions into an acceptance matrix mapping every
  in-scope opportunity, safety invariant, CLI mode, OS newline form, and
  performance case to a named test or benchmark location.

### Validation Checkpoint

- [x] Confirm `decisions.md`, the impact report, behavior baselines,
  performance baselines, and acceptance matrix are reviewed and complete
  before any production symbol is edited.

## Phase 2: `biscuit-file` Source and Diagnostic Foundations

- [x] Add one shared byte-offset `SourceSpan` vocabulary to `biscuit-file` and
  preserve Darkmatter's existing public import path through a compatible
  re-export or lossless conversion; add compile-time/public-API tests that
  prevent the two crates from silently diverging.
- [x] Add serde-capable `YamlDiagnostic`, `YamlRepair`, stable diagnostic-code
  and certainty-classification enums, and any top-level analysis/repair result
  required by the Phase 1 JSON contract; re-export them from the
  `biscuit_file` root.
- [x] Add a structured parse-location projection/accessor around the existing
  `serde_yaml_ng::Error::location()` data while preserving current
  `YamlError` display text and conversion behavior.
- [x] Retain the original source read by path-backed `Yaml` values according to
  the Phase 1 ownership decision; define behavior for text, bytes, and
  `Yaml::from_value`, where no authored source may exist.
- [x] Add a source edit-set utility that validates UTF-8 boundaries, rejects
  out-of-range or overlapping repairs, applies accepted edits from the end of
  the source toward the beginning, and returns an audit record of applied and
  rejected candidates.
- [x] Add focused unit tests for multibyte spans, CRLF byte offsets, parser
  byte/line/column projection, retained path content, `from_value` behavior,
  diagnostic JSON serialization, and edit overlap/boundary rejection.
- [x] Parallelizable: diagnostic/serde contract tests and raw-source/location
  tests may be drafted independently, then merged before the shared edit-set
  utility is finalized.

### Validation Checkpoint

- [x] From `biscuit-file`, run focused nextest cases, `just test`, and
  `just lint`; verify pre-change `YamlError` display snapshots remain
  byte-identical and all new public types are reachable from the crate root.

## Phase 3: Schema-Agnostic Analyzer and Deterministic Repairs

- [ ] Implement the ratified source-first analyzer entry point so invalid YAML
  returns structured diagnostics instead of failing construction; make
  `Yaml::diagnose()` and repair-candidate convenience methods delegate to the
  same engine for parseable, source-backed `Yaml` values.
- [ ] Build a context-aware lexical/source map that records mappings,
  sequences, flow collections, scalar styles, comments, block scalars,
  anchors/aliases, document markers, and exact UTF-8 byte spans without
  reserializing the document.
- [ ] Parse at most once on the clean-input path, retain the structured parse
  outcome, and generate bounded/local candidates only for a matching
  diagnostic; do not reparse documents that have no candidate edits.
- [ ] Implement source normalization for the ratified YAML source scope: BOM,
  CRLF/CR to LF, trailing whitespace outside scalar content, and final-newline
  handling, with each candidate evaluated under the Phase 1 safety matrix.
- [ ] Implement parse-equivalent whitespace candidates around flow delimiters
  and commas, mapping colons, and sequence markers; accept only candidates
  whose reparsed `serde_yaml_ng::Value` exactly equals the original value.
- [ ] Implement the ratified no-schema reserved-indicator quoting algorithm,
  quoting the exact authored lexeme and accepting only candidates that satisfy
  the dedicated parse-recovery proof. Keep indentation, delimiter, comment,
  quote, escape, and multi-edit ambiguity outside auto-apply.
- [ ] Apply multiple deterministic repairs only after checking their combined
  non-overlap and final safety result; return diagnostics and candidate/applied
  repair records in stable source order.
- [ ] Add table-driven tests for every accepted normalization/whitespace/
  reserved-indicator case and every refusal boundary, including
  `host:localhost`, negative numbers, comments, URLs, Windows paths, block
  scalars, flow nesting, anchors, multibyte text, BOM, LF, CRLF, and lone CR.
- [ ] Add source-preservation assertions that reconstruct the expected output
  solely from accepted spans and prove every untouched byte is unchanged.

### Validation Checkpoint

- [ ] From `biscuit-file`, run the focused analyzer tests, `just test`, and
  `just lint`; confirm clean input parses once, candidate-free input reparses
  zero times, and all auto-applied edits satisfy the ratified safety proof.

## Phase 4: Schema-Agnostic Report-Only Diagnostics

- [ ] Detect duplicate mapping keys at every nesting level with source spans
  for the conflicting entries even though `serde_yaml_ng` rejects the final
  document; offer only the candidate information ratified in Phase 1 and never
  select a repair automatically.
- [ ] Detect undeclared, forward, misspelled, duplicate, and unused
  anchor/alias conditions that the v1 contract retains; classify every result
  as report-only and preserve graph-sensitive source.
- [ ] Detect multiple YAML documents for single-document analysis and report
  the incompatibility without selecting, splitting, or rewriting a document.
- [ ] Implement the schema-free `non-deterministic-find` lints named by the
  spec: ambiguous scalars, suspicious empty values, block-scalar smells,
  comment-truncation/indicator smells, style/indentation inconsistency, and
  similar/misplaced keys.
- [ ] Add suppression and confidence boundaries that keep common intentional
  YAML quiet; record the reason for every heuristic threshold in tests rather
  than promoting a smell to an error.
- [ ] Add a single auto-apply filter keyed on classification and prove with
  exhaustive enum tests that neither report-only classification can reach edit
  application, even if a diagnostic carries candidate repairs.
- [ ] Add positive, negative, nested, comment-preservation, and ordering tests
  for each detector, plus a mixed document proving deterministic repairs can
  coexist with report-only findings without applying the latter.
- [ ] Parallelizable after Phase 3: Phase 4 can proceed independently of the
  Darkmatter schema-aware work in Phase 5 because both consume the frozen
  shared diagnostic and repair contracts.

### Validation Checkpoint

- [ ] From `biscuit-file`, run focused detector tests, `just test`, and
  `just lint`; assert report-only diagnostics never change source and remain
  deterministically ordered across runs.

## Phase 5: Darkmatter Schema-Aware Repair Layer

- [ ] Introduce a library-owned clean schema configuration/resolution surface
  that can express the Phase 1 baseline, explicit schema, trigger-discovery,
  document path, and opt-out decisions without depending on clap types.
- [ ] Reuse or extract current compose/schema-validation baseline loading and
  trigger discovery so clean and compose share schema semantics; preserve
  existing compose behavior and use the existing `sniff`-backed repository
  discovery path rather than adding filesystem/OS discovery logic.
- [ ] Build at most one `DarkmatterSchemas`/validator-cache context per clean
  run. Resolve it only after a non-empty frontmatter block exists and
  schema-agnostic repair has made the document parseable; retain full document
  source/path context for relative `$schema` files and trigger matching.
- [ ] Add or expose a raw, non-coercing schema-validation query for repair
  analysis while leaving ordinary compose validation/coercion unchanged; it
  must identify whether exactly one plain-scalar node fails solely because a
  string was required.
- [ ] Map schema instance pointers and validation problems back to the shared
  authored YAML spans, including nested mappings/sequences, quoted/plain
  scalars, CRLF, comments, and multibyte values.
- [ ] Implement schema-proven scalar quoting: quote only the exact plain-scalar
  lexeme identified by the raw type mismatch, reparse, require the candidate to
  pass the complete effective schema, require no unrelated validation
  regression, and apply no other source edit.
- [ ] Implement schema-guided key-correction and shape/type repair diagnostics
  as report-only results using the same shared shape; do not insert JSON Schema
  `default` annotations or guess enum/type/parent changes.
- [ ] Apply the ratified schema-side safety checks to schema-independent
  parse-equivalent candidates when an effective schema exists, without
  blocking the explicitly invalid-to-valid schema-quoting transition.
- [ ] Add tests for default Darkmatter baseline keys, inline `$schema`,
  referenced schemas, root unions, custom baseline, disabled baseline,
  matching/nonmatching triggers, disabled triggers, unconstrained keys,
  coercion-sensitive strings, and report-only schema suggestions.
- [ ] Parallelizable within Phase 5 after the raw validation/span contract is
  frozen: schema configuration/cache tests and report-only suggestion tests may
  proceed independently from scalar-quoting candidate tests.

### Validation Checkpoint

- [ ] From `darkmatter`, run focused schema-aware tests, `just test`, and
  `just lint`; compare compose/schema-validation baselines from Phase 1 and
  confirm no behavior changed outside the new clean analysis surface.

## Phase 6: `md clean` Raw-Source Integration and UX

- [ ] Add a raw Markdown input loader for clean that resolves file references
  once, reads file/stdin source once, and carries the optional resolved path;
  keep existing parsed `load_markdown` behavior unchanged for other commands.
- [ ] Use `extract_frontmatter_block` as the only boundary detector and skip
  YAML analysis, schema resolution, and trigger discovery for absent or empty
  frontmatter.
- [ ] Implement the Phase 1 pipeline order and re-run extraction after accepted
  frontmatter edits so all subsequent spans refer to the current source.
- [ ] Run existing Markdown cleanup only after frontmatter is parseable, then
  assemble the final document from the repaired raw frontmatter block and the
  cleaned body; do not route this assembly through `Markdown::as_string()`.
- [ ] Preserve current file/stdin and `--save` behavior according to the Phase 1
  ruling. For save mode, retain meaningful delta reporting for both originally
  parseable and originally invalid frontmatter without pretending an invalid
  source can construct a baseline `Markdown` value.
- [ ] Add clean CLI args and clap conflicts for the ratified schema options and
  `--json`; thread defaults through both the `clean` subcommand and top-level
  `INPUT --save` shorthand without duplicating schema-resolution policy.
- [ ] Render human suggestions to STDERR using `TerminalRenderable`
  components such as `Prose`, `UnorderedList`, and an appropriate status/code
  component; keep presentation logic out of the analyzer and never hand-roll
  ANSI sequences.
- [ ] Serialize `--json` from the shared diagnostic values and exact Phase 1
  envelope/channel contract; add golden fixtures that pin all fields, enum
  spellings, spans, empty repair arrays, and multiple-diagnostic ordering.
- [ ] Keep every report-only finding at the existing success exit status and
  preserve existing failures for missing input, unreadable files, invalid
  flags, and `--save` with stdin; do not add the deferred `--strict` mode.
- [ ] Add L1 CLI tests for stdout mode, stdin, `--save`, verbose save, default
  repair, no-frontmatter bypass, empty frontmatter, all schema flags, inline and
  referenced schemas, trigger discovery/opt-out, STDERR suggestions, JSON,
  unchanged exit codes, and repeated execution.
- [ ] Add a sentinel fixture containing intentionally broken YAML inside a
  fenced body block and prove its bytes are unaffected by the YAML engine while
  ordinary Markdown body cleanup retains its established behavior.

### Validation Checkpoint

- [ ] From `darkmatter`, run focused `darkmatter-cli` clean tests, `just test`,
  and `just lint`; manually inspect one human-rendered diagnostic and one JSON
  fixture, and verify no real-terminal L2 coverage is required for this
  in-process CLI contract.

## Phase 7: Corpus, Cross-Platform, Performance, and Regression Gate

- [ ] Vendor or otherwise pin the ratified YAML Test Suite subset with license
  and upstream case IDs; cover valid, expected-failure, duplicate-key,
  anchor/alias, flow, scalar, BOM, and multi-document cases without making test
  execution depend on the network.
- [ ] Build mutation fixtures from real monorepo frontmatter and inject every
  v1 repair/finding class; verify exact spans, deterministic ordering,
  classification, candidate edits, accepted output, and untouched-byte
  preservation.
- [ ] Add suite-wide invariant/property tests proving report-only findings
  never mutate, accepted parse-equivalent edits preserve values, schema-aware
  edits satisfy their dedicated proof, edit sets never overlap, and any
  ratified idempotency guarantee holds.
- [ ] Exercise LF, CRLF, lone CR, UTF-8 BOM, non-ASCII keys/values, Windows
  paths, Unix paths, and final-newline variants in platform-independent tests;
  run or confirm the existing macOS/Windows/Linux CI matrix compiles and tests
  both affected packages.
- [ ] Add instrumentation or benchmark counters proving no-frontmatter input
  performs zero YAML/schema/trigger work, clean frontmatter parses once,
  candidate-free input reparses zero times, and schema/trigger state is reused
  within one clean invocation.
- [ ] Re-run the Phase 1 benchmarks under the same profile and corpus, record
  Criterion/statistical comparisons in a feature artifact, and investigate any
  measurable regression in the two common cases without inventing a hard
  millisecond threshold.
- [ ] From `biscuit-file`, run `just test`, `just lint`, and `just doctest`.
- [ ] From `darkmatter`, run `just test`, `just lint`, and `just doctest`; run
  `just test-l2` only if implementation review identifies a real-terminal
  behavior not covered by the specified in-process rendering contract.
- [ ] Run `detect_changes({scope: "compare", base_ref: "main"})` and review
  every changed symbol and affected execution flow before requesting review.

### Validation Checkpoint

- [ ] Confirm the ratified acceptance matrix is entirely green, performance
  evidence shows no measurable regression in the required common cases, CI is
  green on macOS/Windows/Linux, and GitNexus reports only expected change scope.

## Phase 8: Documentation, Hashes, and Lifecycle Closure

- [ ] Update `biscuit-file` public API/README documentation for source-first
  YAML diagnosis, retained source, structured locations, diagnostics, repairs,
  safety classifications, and the absence of automatic schema-aware behavior.
- [ ] Update Darkmatter's `md clean` CLI documentation and README with default
  behavior, frontmatter-only scope, schema precedence/options, stdout/save
  semantics, STDERR suggestions, JSON examples, exit-code stability, and the
  explicit absence of `--strict` in v1.
- [ ] Update affected architecture/testing documentation and
  `.claude/skills/biscuit-file` / `.claude/skills/darkmatter` so future agents
  use the shared engine and raw-source clean path rather than reserializing
  frontmatter or duplicating schema resolution.
- [ ] Update root and per-area `docs/dependencies.md` only if implementation
  adds or removes crates; avoid a new parser dependency unless Phase 1 evidence
  proves the existing scanner/parser combination cannot meet the contract.
- [ ] Revisit every changed symbol's rustdoc and nearby comments for behavioral
  drift, deleting HOW-narration and updating only comments whose contract
  changed.
- [ ] Recompute any edited Markdown `hash:` fields with Darkmatter's
  `md hash <file>` workflow, then rerun doctests and the focused documentation
  examples.
- [ ] Move the feature directory to `darkmatter/features/_completed/` only
  after all validation gates pass and update links that depend on the active
  path.
- [ ] Run a final `detect_changes({scope: "compare", base_ref: "main"})` after
  documentation/lifecycle edits and confirm no unexpected source or execution
  flow entered the change set.

### Validation Checkpoint

- [ ] Confirm docs and skills describe shipped behavior, all required hashes
  are current, the feature is in `_completed`, and the final test/lint/doctest
  and change-scope evidence is attached to the implementation handoff.

## Dependency and Parallelization Summary

```text
Phase 1: contracts, impact, baselines
  └─ Phase 2: shared source/diagnostic foundations
       └─ Phase 3: analyzer + deterministic repairs
            ├─ Phase 4: schema-free report-only diagnostics ─┐
            └─ Phase 5: Darkmatter schema-aware layer ──────┤ parallel
                                                            └─ Phase 6: md clean integration
                                                                 └─ Phase 7: full validation/performance
                                                                      └─ Phase 8: docs/closure
```

- [ ] Parallel work is limited to the explicitly flagged tasks after their
  shared contracts land; coordinate files with exhaustive enum matches to
  avoid conflicting edits.
- [ ] Phase 6 does not start until Phases 4 and 5 both pass their checkpoints;
  Phase 8 does not start until the complete Phase 7 gate is green.
