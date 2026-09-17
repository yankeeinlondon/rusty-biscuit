---
spec: "claudine/fixes/2026-09-16-better-spec-syntax/spec.md"
plan: "claudine/fixes/2026-09-16-better-spec-syntax/plan.md"
implemented_by: "codex/gpt-5.6-sol"
started_phase: "5"
implemented: true
source_code:
    - claudine/cli/tests/compose_caller_file_provenance.rs
    - claudine/cli/tests/composition_outputs.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/inline_completion_lifecycle.rs
    - claudine/cli/tests/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/sequence_groups.rs
    - claudine/cli/tests/sequence_jit.rs
    - claudine/cli/tests/shipped_prompt_contract.rs
    - claudine/cli/tests/shipped_prompts.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - darkmatter/cli/tests/get_set_rm.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - prompts/_implement/implement-plan.md
    - claudine/cli/src/commands/compose/loop_run.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/tests/loop_initialize_state.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/seed.rs
    - darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md
documentation:
    - claudine/fixes/2026-09-16-better-spec-syntax/spec.md
    - claudine/fixes/2026-09-16-better-spec-syntax/plan.md
    - claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
    - claudine/README.md
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-07-11-sequence-plus/plan.md
    - claudine/features/2026-07-11-sequence-plus/spec.md
    - claudine/features/2026-07-11-sequence-plus/validation-matrix.md
    - darkmatter/README.md
    - darkmatter/docs/structs/Markdown.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/lifecycle.md
    - .claude/skills/darkmatter/frontmatter.md
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-16-better-spec-syntax/spec.md
    - claudine/fixes/2026-09-16-better-spec-syntax/plan.md
    - claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/cli/tests/compose_caller_file_provenance.rs
    - claudine/cli/tests/composition_outputs.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/inline_completion_lifecycle.rs
    - claudine/cli/tests/level2_sequence_task_stream_capture.rs
    - claudine/cli/tests/sequence_groups.rs
    - claudine/cli/tests/sequence_jit.rs
    - claudine/cli/tests/shipped_prompt_contract.rs
    - claudine/cli/tests/shipped_prompts.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - darkmatter/cli/tests/get_set_rm.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - prompts/_implement/implement-plan.md
docs_updated_during_phase_2:
    - claudine/fixes/2026-09-16-better-spec-syntax/spec.md
    - claudine/fixes/2026-09-16-better-spec-syntax/plan.md
    - claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_3:
    - claudine/cli/src/commands/compose/loop_run.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/loop_initialize_state.rs
    - claudine/cli/tests/shipped_prompt_contract.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/seed.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/schema/tests.rs
    - prompts/_implement/implement-plan.md
docs_updated_during_phase_3:
    - claudine/README.md
    - claudine/docs/topics/lifecycle.md
    - claudine/fixes/2026-09-16-better-spec-syntax/spec.md
    - claudine/fixes/2026-09-16-better-spec-syntax/plan.md
    - claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/lifecycle.md
source_files_during_phase_4:
    - claudine/cli/tests/shipped_prompts.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-09-16-better-spec-syntax/plan.md
    - claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/cli/tests/shipped_prompts.rs
    - darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md
docs_updated_during_phase_5:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/lifecycle.md
    - claudine/features/2026-07-11-sequence-plus/plan.md
    - claudine/features/2026-07-11-sequence-plus/spec.md
    - claudine/features/2026-07-11-sequence-plus/validation-matrix.md
    - claudine/fixes/2026-09-16-better-spec-syntax/plan.md
    - claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
    - darkmatter/README.md
    - darkmatter/docs/structs/Markdown.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/lifecycle.md
    - .claude/skills/darkmatter/frontmatter.md
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - dmls
completed_phase: "5"
human_review: true
human_review_items:
    - >-
        Diagnose the unrelated terminal proxy-route L2 capture, which reaches
        provider launch and then never renders the expected
        failure.stack[*].proxy diagnostic before the harness deadline.
message_to_agent: >-
    Phase 5 is implementation-complete and ready for review. Targeted tests,
    full Claudine L1, both package-area lint gates, full Darkmatter L1, and the
    DMLS fixture regression pass. The full Claudine L2 run has one reproducible
    pre-existing proxy-route terminal-capture failure recorded in
    human_review_items; two other initial capture failures passed alone.
---

# Implementation Log for 2026-09-16-better-spec-syntax (5 phases)

## Phase 1

### Recovery record — 2026-09-17

These findings were recovered from the prior agent's handoff supplied by the
author. They are reported results, not independently rerun during this
update. The original agent attribution is retained.

The attempt stopped with ENOSPC on `/private/tmp` and `/Volumes/coding`,
including failed log writes and cleanup. The author has restored storage.
No source changes were reported. The plan's startup `implemented: true`
was not completion evidence and is corrected to `false`; Phase 1 remains
incomplete and no `completed_phase` is set. Empty phase file lists describe
implementation deliverables, excluding plan/log bookkeeping. The log's
`spec` field incorrectly pointed to the plan; it now points to the spec.

### Spike 1 — Duplicate keys

Tested with the workspace's exact `serde_yaml_ng` version and `md get`:

- Darkmatter's frontmatter map silently keeps the last duplicate at both
  top-level and nested mappings. `set: {epilog: first, epilog: second}`
  retains `second`. The same occurs inside `{{ … }}` values.
- Parsing into `serde_yaml_ng::Value` instead rejects duplicates with line
  and column.
- `parse_lifecycle_config` receives already-collapsed JSON; Claudine cannot
  detect duplicates at the `set` boundary.

At recovery, Ruling 2 awaited the author's decision: (a) reject duplicates in shared
Darkmatter `parse_yaml_with_fallbacks` (recommended; affects all frontmatter
readers and requires expanded scope/validation); (b) reparse raw frontmatter
in Claudine (requires amending the no-second-parser constraint); or (c) drop
the duplicate-key requirement. Option (a) was subsequently approved; see
the decision record below.

### Spike 2 — Execution and result plumbing

- Reported locations: `dispatch_side_effect` at `executor.rs:1293`, its
  `set` arm at 1342, `apply_runtime_set` at 1439, and outdated comment at
  1436. Line numbers are navigation hints to recheck before editing.
- Only sequence side-effect tasks expose results through `run_side_effect`
  and `Ok(other) => other.to_string()`. Event stacks and setup/teardown
  discard values (`executor.rs:1172-1182`).
- `dispatch_task_side_effect` writes its working map back to live state even
  when the action fails. Validate and resolve everything before mutation;
  test this outer write-back path for partial updates.
- `sequence/task/group.rs:406` also calls `RuntimeState::set` to merge
  parallel-group results; the original caller inventory missed it.
- Lifecycle `set('k', v)` already fails with `LifecycleShortFormRemoved`,
  but suggests the positional form being removed. Phase 2 needs a
  mapping-specific suggestion for `set`.

### Spike 3 — Migration inventory corrections

- `claudine/lib/src/composition/sequence/task/tests.rs`: about 17 sites.
- `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md`:
  executable fixture migration touches Darkmatter.
- `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`:
  already drifted from the real prompt.
- `action_shape_control.rs`: no `set` usage; not a migration target.
- Real prompt: lines 40, 56, and 57. `sequences.md`: 389, 392, and 479.
- `composition/error/tests.rs:2174,2181`: `set` is only a label; review
  these assertions when diagnostics change.
- No hits in `claudine/schemas/` or Claudine skill files.

Preserve the plan's non-targets: loop-control `set(...)`, Darkmatter
capability descriptors, unrelated jq expressions, and historical completed
specs that are not executable inputs.

### Rulings and remaining Phase 1 work

Rulings 1, 3, 4, and 5 retain the plan's recommendations, without recorded
author confirmation. Ruling 3 additionally requires explicit matching in
`is_side_effect_action` and task dispatch. Ruling 5 includes the call-spelling
fix above. Ruling 2 is now approved as recorded below.

1. Review scratch cleanup: `/tmp/bss-spike` holds the small test crate and
   build output; `/tmp/bss-baseline` holds partial logs. Previous deletion
   failed. If further build-space reclamation is necessary, follow the
   storage-strategy skill and run `just sweep` before manual deletion.
2. Rerun `just test`, `just test-l2`, and `just lint` in `claudine/`, saving
   outputs and exit statuses. The prior L1 attempt started a from-scratch
   build and exited nonzero, likely but not conclusively due to disk
   exhaustion. Neither L2 nor lint left a log. No baseline is established.
3. Run `just gitnexus`, then upstream impact for `parse_lifecycle_config`,
   `parse_positional_action`, `dispatch_side_effect`, `RuntimeState::set`,
   `dispatch_task_side_effect`, and `is_known_side_effect`, plus
   Darkmatter's `parse_yaml_with_fallbacks` for approved Ruling 2. Record callers,
   processes, and risk. The prior refresh failed once; later attempts hit
   ENOSPC. No impact results were recorded.
4. Obtain confirmation of Rulings 1, 3, 4, and 5. Ruling 2 is approved
   and incorporated into the spec, scope, and validation plan. Capture
   Darkmatter `just test` and `just lint` baselines for its shared parser.
5. Record actual baseline and graph results before closing Phase 1. Keep
   `human_review: true` while author decisions remain pending.

This recovery update changes only the plan and implementation log. Tests,
graph refresh, cleanup, and Phase 2 implementation were not run.

### Ruling 2 closed — author approval, 2026-09-17

The author approved the recommendation to reject duplicate keys in
Darkmatter's shared frontmatter parser (option a). This is a confirmed
design decision, not a completed implementation task.

The approved contract rejects duplicates within any YAML frontmatter
mapping, including nested mappings, consistently across direct,
indentation-normalized, and expression-protected parsing. Preserve typed
errors and source-location information. Fallbacks must not hide duplicates;
Claudine must not reparse frontmatter. Duplicate-looking text in expression
strings remains expression content. Explicit merging between separate
documents is unchanged.

The behavior change applies to all Darkmatter frontmatter consumers;
last-wins documents must be corrected. The spec and plan now include
Darkmatter scope, parser impact analysis and baselines, fallback and nested
mapping regressions, valid expression coverage, passive shipped-artifact
validation, normal CLI invocation, and documentation updates.

At that recovery checkpoint, Ruling 2 was checked off and removed from
`human_review_items` in both plan and log. Rulings 1, 3, 4, and 5 still awaited
confirmation, so Phase 1 remained open. The completion record below supersedes
that checkpoint status.

### Phase 1 completion — 2026-09-17

The instruction to complete this non-interactive phase ratifies the plan's
recommendations for Rulings 1, 3, 4, and 5. Together with the separately
approved Ruling 2, all five rulings are binding:

1. Structural mapping shape errors are parse-time failures, including below a
   false `when`; reserved-root-key refusals remain execution-time failures.
2. Darkmatter rejects duplicate YAML mapping keys through the shared parser.
3. `set` is one dedicated action kind containing an order-preserving recursive
   typed value tree; it is not desugared into single-key actions.
4. `no_error` is the universal sibling modifier beside the one verb key;
   `set: {no_error: value}` still assigns a property named `no_error`.
5. Removed and malformed `set` forms receive typed, path-bearing migration
   diagnostics that recommend only `set: {property: value}` and preserve the
   existing diagnostic-selection seam.

#### Graph impact refresh

Repository binding: `better-static-analysis` at
`/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis`, worktree on
`feat/better-static-analysis`, index and HEAD both
`fd0da67a0478a74e2489a5d535fd403fcf05d8d1`. `just gitnexus` reported all
6,975 covered files current.

- `is_known_side_effect`: LOW, one direct caller
  (`is_known_lifecycle_verb`), Lifecycle module, no indexed processes.
- `parse_yaml_with_fallbacks`: LOW, one direct caller (`parse_frontmatter`),
  23 total upstream impacts through depth 3, Markdown and Components modules,
  no indexed processes.
- `parse_lifecycle_config`: UNKNOWN in GitNexus (zero resolved callers). Text
  search confirms two production calls: canonical preparation in
  `composition/prepare.rs` and sequence preflight in
  `composition/sequence/preflight/mod.rs`, plus extensive unit-test use.
- `parse_positional_action`: UNKNOWN in GitNexus (zero resolved callers). Text
  search confirms its direct parser call in `lifecycle/parse.rs`.
- `dispatch_side_effect`, `dispatch_task_side_effect`, and
  `RuntimeState::set`: UNKNOWN because the current Rust index does not expose
  those impl methods as addressable symbols. Text search confirms four
  internal `dispatch_side_effect` call sites, the sequence task runner as the
  production caller of `dispatch_task_side_effect`, and production
  `RuntimeState::set` calls from lifecycle dispatch and parallel-group merge.
- Concept/process search returned no relevant indexed lifecycle execution
  process. UNKNOWN results remain unresolved graph limitations, not low-risk
  evidence; the text-search inventory is the Phase 2 scope floor.

No refreshed result was HIGH or CRITICAL. The shared Darkmatter parser has the
widest known transitive surface and requires its area gates plus shipped
artifact and CLI coverage in the implementation phases.

#### Baseline capture

The shared `target/` initially failed before test execution because cached
`.rmeta` files were mode `0444`. Fresh runs used
`CARGO_TARGET_DIR=/tmp/bss-phase1-target.PYTJhj`; no shared artifacts were
deleted or permissions changed.

- Claudine `just test`: red after 1,273 passes because fail-fast stopped on
  `composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`.
  The current adjacent edit to `prompts/_implement/implement-plan.md` omits two
  `gitnexus analyze --force` commands still expected by the test. This is a
  pre-existing dirty-worktree mismatch, not a Phase 1 regression; 5,986 tests
  were not run after fail-fast and nine were tier-skipped.
- Claudine `just test-l2`: pass. `claudine-cli` ran 231/231 with 2,747
  non-L2 tests skipped; `claudine-gen` ran 3/3 with 174 non-L2 tests skipped.
- Claudine `just lint`: pass for all five area packages. The linker emitted the
  existing macOS compact-unwind-size warning while building `claudine-cli`.
- Darkmatter `just test`: pass, 7,937/7,937 with seven tier-skipped tests.
- Darkmatter `just lint`: pass for `darkmatter`, `darkmatter-cli`, `dmls`,
  `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check.

#### Test-design mapping

Phase 1 changes no production behavior, parser, schema, template, prompt, or
configuration artifact, so no targeted regression, corpus, end-to-end, or
round-trip test was added in this phase. The tests required for each planned
behavior are already enumerated in Phases 2–5 and acceptance criteria 1–10:
parser shape/representation variants in Phase 2, snapshot and atomic state in
Phase 3, shipped-artifact corpus plus real CLI paths in Phase 4, and
cross-surface diagnostics and broad gates in Phase 5. The baseline above is
the comparison point for those tests.

#### Recovery cleanup decision

Reviewed `/tmp/bss-spike` (56 MiB: duplicate-key fixture/crate/build) and
`/tmp/bss-baseline` (240 KiB: partial prior log). Storage now has about 462 GiB
available. Both were preserved as useful recovery evidence; no sweep or manual
deletion was necessary. The isolated Phase 1 target was also retained for the
next phase's warm build and can be removed after later gates complete.

Phase 1 modified no source code. The plan, spec, and this log are the only
Phase 1 documentation updates. No skill change was warranted because this
phase closed design and baseline questions without changing Claudine behavior
or workflow.

## Phase 2

### Test-design map — before implementation

- Mapping-only lifecycle grammar and recursive typed values: extend the
  lifecycle parser tests with the reported `epilog` / `message_to_agent`
  mapping verbatim, plus one-key, multi-key, empty, native and quoted scalars,
  null, arrays, nested objects, whole-value expressions, recursive
  interpolation, `no_error` as both modifier and destination, and valid
  false-guarded input. Assertions inspect the public typed action tree and
  confirm no eager evaluation occurs.
- Structural and removed-form failures: targeted parser tests retain the old
  positional, long-form, and call-spelling inputs exactly, plus scalar,
  whole-mapping interpolation, empty, non-string, and interpolation-bearing
  destination keys under both ordinary and false-guarded stacks. Assertions
  cover the typed variant, source document, semantic action path, canonical
  rewrite, rendered block, frontmatter highlight, and diagnostic detail.
- Duplicate YAML keys: Darkmatter frontmatter tests cover top-level, nested,
  array-nested, tab-normalized, and expression-protected paths; valid
  expression-bearing values and duplicate-looking text inside expressions
  remain accepted. The existing explicit document-merge tests remain the
  regression boundary for unchanged merge semantics.
- Shipped artifacts and normal invocation: extend the existing Claudine
  shipped-prompt corpus parser test to cover every active shipped prompt after
  migration, and exercise the real shipped implementation prompt through the
  hermetic `CliProcessFixture` normal invocation path. Darkmatter receives a
  hermetic CLI regression that parses a real repository fixture through `md`.
- Runtime batch groundwork: unit tests call the public `RuntimeState` batch API
  with multiple values, verify the prior-value object, explicit-null presence,
  empty updates, all-or-nothing invalid-key behavior, and read/write/read
  snapshots. Executor wiring and snapshot expression evaluation remain Phase 3.
- Existing lib-authored lifecycle inputs are migrated without weakening their
  downstream assertions for reserved/dotted keys, typed values, runtime
  visibility, task output, and no-file-write behavior.

All new coverage is L1: it needs only in-process parsing or hermetic
filesystem/subprocess fixtures, not a terminal, browser, device, or external
provider.

### Pre-edit impact refresh

The worktree is bound to `rusty-biscuit` at
`/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis`, branch
`feat/better-static-analysis`, indexed/current commit `d5edb8b`; `just
gitnexus` reports all 6,975 covered files current. The CLI registry still
reports its older alias snapshot as 11 commits behind for impact output, so
UNKNOWN results below are resolved with the Phase 1 caller inventory and fresh
text search rather than treated as safe:

- `ProxyWith`/typed-value surface: the struct candidate reports MEDIUM (10
  direct, 15 total); impl candidates remain UNKNOWN. Text search identifies
  parser construction plus lifecycle validation/execution/tests as consumers.
- `CompositionError`: UNKNOWN in the graph; exhaustive Rust matches in the
  renderer, diagnostic projection, frontmatter highlighting, and tests are the
  required edit surface.
- `RuntimeState`: UNKNOWN in the graph; text search confirms lifecycle
  dispatch, parallel-group merge, preparation, and unit-test consumers.
- `parse_yaml_with_fallbacks`: LOW, one direct caller and 23 total upstream
  symbols across the Markdown and Components modules, with no indexed process.

No HIGH or CRITICAL result was returned. The shared Darkmatter parser remains
the broadest surface and therefore receives its package-area test and lint
gates plus corpus and CLI coverage.

### Implementation progress

- Added one `LifecycleActionKind::RuntimeSet` action carrying an
  `IndexMap<String, ProxyWithValue>`. The existing recursive typed value tree
  is shared rather than duplicated. Mapping construction rejects empty and
  interpolation-bearing destination keys while preserving native scalars,
  quoted scalars, nulls, arrays, nested objects, and whole-value expressions.
- The lifecycle parser now accepts only `set: {property: value}` and permits
  the universal sibling modifier `{set: {...}, no_error: true}`. A nested
  destination literally named `no_error` remains ordinary data. Structural
  validation occurs while parsing, including below false guards.
- Positional arrays, `action: set` long form, scalar/whole-mapping values, and
  call spellings now select typed, path-bearing diagnostics that render the
  canonical mapping rewrite through terminal, detail, and machine facets.
- Darkmatter's shared YAML fallback parser now parses through
  `serde_yaml_ng::Value` before converting to its JSON-backed map. This rejects
  duplicate keys recursively before conversion can collapse them, including
  indentation-normalized and expression-protected paths, while retaining
  valid expression text and separate-document merge behavior.
- Added `RuntimeState::set_batch`: all keys validate before locking, the full
  update is prepared on a clone and published once, and prior values use key
  presence so an explicit runtime null does not fall back to document state.
  The single-key API delegates to this boundary. The mapping action dispatch
  was wired far enough to keep all migrated library and CLI behavior green;
  Phase 3 retains the dedicated execution-semantics audit and matrix.
- Migrated the shipped implementation prompt, its executable fixture, all
  library-authored lifecycle `set` cases, and the L1/L2 CLI test inputs needed
  by the requested package gate. A passive shipped-prompt assertion pins the
  mapping artifact, and the existing hermetic normal `compose` invocation now
  explicitly confirms it exercises that real shipped source.

### Targeted regression coverage

- `runtime_set_parses_the_reported_mappings_as_one_typed_action` includes the
  reported `epilog: "{{message_to_agent}}"`, `message_to_agent: null`, and
  `epilog: null` inputs verbatim and asserts the single typed action tree.
- `runtime_set_preserves_recursive_authored_types_and_empty_mapping` covers
  empty/one-or-more destinations plus native and quoted scalars, null, arrays,
  nested objects, and expression leaves.
- `runtime_set_accepts_no_error_as_modifier_and_as_destination`,
  `runtime_set_invalid_keys_fail_even_below_a_false_guard`, and
  `runtime_set_reports_the_full_nested_value_path` cover modifier ambiguity,
  negative key cases, false-guard structural validation, parse-only laziness,
  and stable nested paths.
- `runtime_set_removed_and_non_mapping_forms_have_canonical_guidance` and
  `runtime_set_shape_diagnostics_share_render_highlight_and_machine_identity`
  cover every removed shape, canonical guidance, source highlighting, render,
  detail, and diagnostic identity.
- Runtime-state tests cover multi-key atomic commit, prior-value objects,
  explicit null presence, empty batches, late invalid dotted keys with no
  partial write, and repeated read/write/read snapshots.
- Darkmatter parser tests cover direct top-level/nested/array duplicates,
  indentation and expression fallbacks, valid expression values, and
  duplicate-looking expression/shell text. The `md get` process regression
  covers the normal CLI failure path. `shipped_prompt_corpus_parses_frontmatter`
  passively parses every shipped prompt.

The changed values are runtime-memory state rather than persisted storage, so
there is no disk persistence round trip. The repeated runtime snapshot test is
the applicable read/write/read boundary. No terminal or browser window is
opened by this L1 coverage.

### Phase 2 verification and handoff

The requirement-to-test mapping above was exercised at the public parser,
diagnostic, runtime-state, shipped-corpus, and normal CLI invocation
boundaries. In particular, the exact reported mappings are pinned by
`runtime_set_parses_the_reported_mappings_as_one_typed_action`; the native,
quoted, null, collection, empty, and malformed variants are covered by the
remaining targeted parser tests; dependent runtime results and atomic state
are covered by the batch tests; duplicate YAML failures are exercised both
in-process and through `md get`; and the shipped artifact is covered both by
the passive corpus test and the hermetic `compose` route.

Verification results:

- Targeted Claudine parser, runtime-state, diagnostic, and shipped-artifact
  regressions passed. The full-path test initially exposed a split nested
  diagnostic path; prefixing the typed value error with its complete action
  path made the targeted regression pass.
- `cargo nextest run -p claudine --lib` passed 4,179/4,179 tests before the
  final nested-path regression was added; that regression was then run and
  passed directly. The later package gate includes the final library state.
- `just test` in `darkmatter/` passed 7,941/7,941 tests with 7 tier-filtered
  tests skipped. `just lint` in `darkmatter/` passed.
- The first Claudine package run exposed two stale shipped-prompt expectations:
  the fixture hash and an assertion for a command no longer present in the
  shipped prompt. Both expectations were updated to the current shipped
  artifact. A second run inherited `/Users/ken/.claudine` as `HOME`, causing
  seven spawn-fixture failures because those tests intentionally reject a
  provider overlay as a home directory; all nine affected spawn tests passed
  with `HOME` absent.
- The final `just test` in `claudine/`, run with `HOME` absent and explicit
  Cargo/Rustup homes, passed 7,269/7,269 tests with 9 tier-filtered tests
  skipped. `just lint` under the same environment passed every Claudine-area
  package.

No L2-only, terminal, browser, or cross-OS gate was necessary: the phase
changes parser, diagnostic, in-memory state, and hermetic filesystem/process
behavior without an OS-specific branch. The package runs emitted a pre-existing
macOS compact-unwind linker warning and a non-failing kache cross-volume copy
advisory. No unresolved test or lint failure remains. `cargo fmt` was not run.

The final GitNexus all-scope change analysis completed without truncation. It
reported 53 dirty-worktree files, 30 changed symbols, no affected indexed
process, and LOW risk; the larger file count includes the author's unrelated
pre-existing worktree changes. The Phase 2 scoped diff contains 36 files.

## Phase 3 resume blocker repair

The migrated handoff mapping exposed two initialization defects before Phase 3
could launch: `set.epilog` rejected an absent `message_to_agent`, and the loop's
failure handler then lacked the derived `plan` field because it received only
the loop-control seed. Initialization also had no shared RuntimeState, so its
successful `set` writes could disappear before body preparation.

The repair keeps full bootstrap frontmatter separate from the loop-control
seed, shares initialization's live state with its catch handlers, and carries
initialization writes through the invocation RuntimeState. A mapping's absent
destination keys are known null bindings in its pre-write snapshot; existing
values retain snapshot semantics. Whole-value null stays typed null and renders
empty when embedded in text. Unrelated unknown names still fail validation.

Regression coverage includes absent/null/present handoff values, a fake-provider
CLI run through phases 3 and 4, a deliberate initialization error whose failure
handler reads `plan` and an earlier mutation, and the shipped implementation
route with a populated phase-2 handoff log. Phase 3's remaining dedicated audit
and legacy-helper removal are still pending.

Validation on macOS: `just test` passed 7,271 tests (9 tier-filtered skips),
including the router/handoff regression; `just lint` passed all area packages.
The first concurrent build/test runs timed out under compilation load; both
checks passed when rerun after the release build. The shipped fixture and its
Darkmatter hash pin were synchronized with the current prompt, and the stale
schema-test expectation for the updated staging command was corrected.
The optimized binary was installed at `/Users/ken/.cargo/bin/claudine`; an
isolated smoke check against that installed executable verified phases 3 and 4,
null interpolation, persisted initialization writes, and failure-context
preservation without launching a real provider. No formatting or commits ran.

## Phase 3

### Test-design map — before implementation

- Snapshot evaluation: executor tests will run the exact `left`/`right` swap in
  both mapping orders and assert the runtime mutations, live working map, and
  returned prior-value object. A separate two-action case will prove that a
  later action observes the earlier action's committed values.
- Atomic expression failure: one mapping will resolve an otherwise valid first
  entry and then fail on an unknown expression. Tests will exercise both a
  shared `RuntimeState` and the no-runtime-cell path, asserting that neither
  runtime mutations nor the live/working map publishes the valid prefix.
- Atomic destination validation: one mapping will contain a valid first entry
  followed by a reserved or dotted destination. Tests will assert the same two
  state layers remain unchanged, including the task-side-effect outer
  write-back path that copies its caller-owned working map after dispatch.
- Suppression and immutable views: a `no_error: true` dispatch failure will
  continue to the next action without exposing any write, while attempts to
  assign `state`, `previous`, or `next` will leave those authored views and the
  runtime layer unchanged.
- Result semantics: executor and sequence-task tests will assert the complete
  prior-value object for multiple keys, explicit runtime null over a non-null
  document value, and an empty mapping; event/setup/teardown result discard
  remains covered by their observable state and output behavior.
- Visibility and persistence: existing cross-event, cross-iteration,
  serial/parallel task, and no-file-write tests remain the broader regression
  boundary. The exact reported implementation-prompt mapping is already pinned
  by the library parser/executor regression, while the passive shipped-prompt
  corpus and hermetic normal CLI route added in Phase 2 remain the required
  artifact-level coverage.
- Removed execution compatibility: production-source audit will require the
  legacy positional `verb == "set"` dispatch and `apply_runtime_set` helper to
  be absent. Observable removed-form failures remain covered at the public
  parser/diagnostic boundary rather than by testing executor internals.

All new tests are L1: they use in-process lifecycle/sequence fixtures and no
terminal, browser, network service, or real provider.

### Pre-edit impact and implementation

GitNexus could not resolve `dispatch_side_effect`, `set_batch`,
`dispatch_task_side_effect`, or `is_side_effect_action` as indexed symbols and
returned `risk: UNKNOWN` for each. Text search therefore supplied the required
caller confirmation: mapping actions enter both event-stack and task
side-effect dispatch, sequence classification explicitly accepts
`RuntimeSet`, and `RuntimeState::set_batch` is shared with the single-key Rust
API while parallel-group merge continues through `RuntimeState::set`. No HIGH
or CRITICAL result was returned.

The executor now routes lifecycle mappings only through
`dispatch_runtime_set`. The remaining positional `verb == "set"` branch and
`apply_runtime_set` helper were deleted, leaving no execution compatibility
path for removed lifecycle syntax. `RuntimeState::set_batch` validates the
whole update before locking, then publishes cloned values with infallible map
insertion under one mutex acquisition. Working/live state is updated only
after that commit succeeds.

### Targeted execution coverage

- `mapping_set_swaps_values_in_either_destination_order` proves both authored
  destination orders resolve from one pre-action snapshot and asserts the
  prior-value object, runtime mutations, and live working map.
- `consecutive_set_actions_observe_each_others_updates` proves a second action
  sees the first action's commit while retaining its own snapshot boundary.
- `failed_expression_publishes_no_part_of_the_mapping_with_or_without_runtime`
  covers a valid prefix followed by an unknown-expression failure in
  both runtime-cell configurations.
- `late_invalid_destination_publishes_no_part_of_the_task_side_effect` covers
  reserved and dotted destinations after a valid entry, with and without a
  shared runtime cell, including the task dispatcher’s outer write-back seam.
- `no_error_suppresses_a_batch_refusal_without_exposing_a_partial_write`
  proves suppressed dispatch continues against unchanged state.
- `mapping_result_reports_all_priors_and_preserves_explicit_runtime_null`
  covers multiple keys, absent prior values, an explicit runtime null over a
  non-null document value, and the empty-object result.
- `set_refuses_every_reserved_root_key` now seeds every authored reserved view
  and proves `state`, `previous`, `next`, `outputs`, and `sequence_id` remain
  byte-for-value unchanged after refusal.

The focused mapping/runtime/sequence selection passed 29/29 tests, including
the reported implementation mapping, mapping parser variants, runtime batch
read/write/read coverage, serialized side-effect results, parallel-group
isolation, and no-file-write behavior. A source audit found no remaining
`verb == "set"` or `apply_runtime_set` production site; the parser's two `set`
comparisons are the required mapping-only routing and removed-form diagnostic
boundaries.

### Phase 3 verification

- `just test` in `claudine/`: pass, 7,276/7,276 tests with 9 tier-filtered
  tests skipped. This includes `shipped_prompt_corpus_parses_frontmatter` and
  `shipped_implement_plan_launches_without_a_sibling_spec`, so the passive
  shipped-artifact corpus and hermetic normal-invocation route remain green.
- `just lint` in `claudine/`: pass for `claudine-catalog-types`, `claudine`,
  `claudine-contract`, `claudine-cli`, and `claudine-gen`; the error-guard and
  lifecycle-doc-facets prerequisite checks also passed.
- The L1 build emitted the existing macOS compact-unwind-size linker warning;
  it did not fail the gate. No targeted, package, or lint failure remains.
- No L2, browser, real-provider, or cross-OS run was needed: Phase 3 changes
  in-memory map evaluation/publication and has no platform-conditional, path,
  process, terminal, or shell behavior. Linux, native Windows, and WSL2 remain
  covered by the ordinary CI matrix.

The runtime cell is in-memory rather than persisted storage, so the existing
repeated snapshot read/write/read test is the applicable round trip. No
terminal or browser window opened, no formatting command ran, and no file was
staged or committed.

GitNexus `detect-changes --scope all` completed without partial or truncated
output: 20 changed tracked files, 16 changed symbols, zero affected indexed
processes, and LOW risk. The changed-file count excludes the new untracked CLI
regression until it is added by the author’s later staging workflow.

## Phase 4

### Test-design map — before implementation

- Sequence `side_effect:` parsing, classification, dispatch, prior-value
  serialization, runtime publication, and `outputs` accumulation map to
  `side_effect_tasks::a_mapping_set_appends_its_prior_value_object`.
- Task-stack mapping syntax and downstream visibility map to
  `side_effect_tasks::the_mutation_delta_reports_the_keys_the_task_wrote`,
  which runs `setup:` before the primary mapping and asserts both the returned
  prior-value object and the final mutation delta.
- Parallel task isolation and post-group merge map to
  `parallel_groups::a_sibling_mutation_is_invisible_until_the_group_completes`.
  Its writer uses a mapping in `setup:`, its reader uses a mapping in
  `teardown:`, and a gate proves the read occurs after the sibling write while
  still observing the group-start snapshot.
- The six named CLI files already author only mapping payloads on entry to
  Phase 4. Targeted test-binary runs will preserve their existing public CLI,
  filesystem, output, and group-order assertions; a source scan will prove no
  removed form remains in those files.
- The artifact regression will invoke a copied, byte-identical real
  `prompts/_implement/implement-plan.md` with `CliProcessFixture`, a fake
  provider, fixture-owned repository/home, dry-run audio policy, and a
  populated prior-phase log. Reaching the provider without a lifecycle parse
  or evaluation error proves the reported multi-key mapping executed on the
  normal non-dry-run route; the test will also assert the source and log remain
  unchanged.
- Passive corpus coverage will parse lifecycle frontmatter for shipped prompt
  and executable Markdown fixture trees, scan generated-input fixtures plus
  Claudine schema catalogs for removed lifecycle `set` spellings, and retain
  the real implementation prompt as a positive mapping witness. The corpus
  test is L1 and performs no provider, terminal, browser, or network work.

All Phase 4 additions are L1 except the already-existing group-stream CLI
binary whose canonical test is L2. The L2 recipe is required for that migrated
file and must remain non-focusing under the package harness.

### Pre-edit impact refresh

Repository: `rusty-biscuit` at
`/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis`; worktree and
index were refreshed at commit `b3a4e6d00`. `is_side_effect_action` is LOW risk
with one direct caller (`TaskExecution::run_side_effect`) and three upstream
symbols in the Task module. GitNexus returned UNKNOWN for
`dispatch_task_side_effect`, `parse_single_action`, and
`parse_task_action_stack`; text search resolved their production seams:
`TaskExecution::run_side_effect` calls the single-action parser and task
dispatcher, while `TaskExecution::parse_stacks` calls the task-stack parser for
both `setup` and `teardown`. No HIGH or CRITICAL result was returned.

### Implementation and targeted verification

- `shipped_lifecycle_artifacts_use_mapping_only_set` walks shipped prompts,
  executable CLI fixtures, generator fixtures, and both Claudine schema
  catalogs. It rejects exact `set` keys whose payload is not a mapping,
  rejects the explicit `action: set` form, parses every Markdown artifact that
  authors `set` through `parse_lifecycle_config`, and requires a positive
  mapping witness. This remains a passive L1 test.
- `shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`
  copies the real prompt corpus, invokes the byte-identical shipped
  `implement-plan.md` through normal non-dry-run `compose`, and uses a failing
  fake Goose provider so the prompt's unrelated success-shell branch cannot
  run. The provider is reached after initialization; the reported
  `epilog`/`message_to_agent` mapping raises no parse/evaluation diagnostic;
  the prior-phase handoff is rendered; the source prompt and existing log are
  byte-unchanged; and no audio is published.
- The three focused sequence tests passed 3/3. They prove task side-effect
  classification/dispatch/result serialization, setup-to-primary visibility,
  teardown mapping acceptance, private parallel state, and declaration-order
  post-group merge.
- The five named L1 CLI binaries passed 88/88 tests. The mapping-relevant test
  in `level2_sequence_task_stream_capture` passed alone through the canonical
  L2 recipe in self-spawn mode. A source scan found no removed `set` spelling
  in any of the six migration files.
- The final shipped-prompts binary passed 4/4 after the Clippy-only rewrite,
  proving both new tests are reachable in ordinary L1 (the end-to-end test was
  renamed so the reserved `real_` tier prefix could not exclude it).

### Broader gates and outstanding failures

- `just test` in `claudine/`: pass, 7,278/7,278 tests with 9 tier-filtered
  tests skipped. The macOS linker emitted the pre-existing compact-unwind-size
  warning; it did not fail the gate.
- `just lint` in `claudine/`: pass for all five area packages after replacing
  one nested test-only `if` with the Clippy-required let-chain. The error guard
  and lifecycle-doc-facets prerequisites also passed.
- `BISCUIT_L2_THREADS=4 just test-l2 --no-fail-fast`: 229/231 passed with
  2,750 non-L2 tests filtered out. The Phase 4 sequence test passed. Two
  reproducible failures are outside this phase's syntax behavior:
  `level2_dry_run_approval_prompt_matches_normal_mode_in_tmux` expects the old
  approval prompt even though the documented dry-run contract stops before
  shell approval, and `level2_proxy_routes_share_identity_across_routes_in_tmux`
  reaches provider launch on its terminal route but does not render the
  expected terminal proxy diagnostic before the harness deadline. Each also
  failed alone. They are recorded in `human_review_items`; no unrelated
  terminal behavior or test contract was changed here.
- The complete L2 run used self-spawn mode and did not create shared terminal
  windows. Two earlier focused attempts used the recipe's serial broker before
  this was isolated; the broker reported spawning WezTerm, tmux, and Apple
  Terminal resources. No browser test ran.
- Phase 4 adds no platform-conditional production behavior. Linux, native
  Windows, and WSL2 execution remain CI evidence; no cross-OS run was needed
  for the passive source scan and hermetic fake-provider test.

No persisted lifecycle state was introduced, so no new persistence round trip
applies. The existing prompt and log are read before invocation and read again
afterward to prove this runtime-only mutation leaves both unchanged. No
formatting command ran, and no file was staged or committed.

Post-edit GitNexus `detect-changes --scope all` completed without partial or
truncated output. It saw four dirty files, including the unrelated pre-existing
`messenger/features/2026-09-17-research-metadata-pipeline/spec.md`, and reported
that no indexed symbol overlapped the changed hunks. The three Claudine files
listed in the Phase 4 metadata are the complete authorized change set.

## Phase 5

### Test-design map — before implementation

- The shipped implementation prompt and its intentionally side-effect-reduced
  CLI mirror map to
  `shipped_lifecycle_artifacts_use_mapping_only_set` and
  `shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`.
  These tests observe parseability, provider reachability, the reported
  multi-key mapping, source/log immutability, and suppressed audio through the
  normal CLI path.
- The remaining DMLS `sequence_descent/implement-plan.md` migration maps to
  `providers::frontmatter::sequence_tests::expression_pass_materializes_each_distinct_ancestor_shape_once`
  and to the expanded shipped-artifact corpus scan. The provider test consumes
  the real fixture; the corpus scan rejects every removed lifecycle `set` form.
- Lifecycle documentation, active Sequence Plus specification examples, and
  skill mirrors map to an active-artifact source scan for `set: [...]` and
  `action: set`, excluding completed historical specifications and the
  deliberately unchanged Darkmatter/capability `set(key, value)` API.
- Darkmatter duplicate-key documentation maps to
  `duplicate_frontmatter_keys_are_rejected_at_every_nesting_level`,
  `duplicate_keys_are_not_hidden_by_indentation_or_expression_fallbacks`,
  `duplicate_looking_expression_text_remains_scalar_content`, and the existing
  `md` frontmatter read/write/read CLI regression from Phase 2. Those tests
  cover direct and fallback parsing, nested mappings and arrays, valid
  expression content, typed errors, and unchanged explicit merge behavior.
- The behavior-changing source comments for lifecycle parsing, runtime batch
  commit, and executor dispatch map to a direct contract-drift review. No
  production edit is planned unless a comment contradicts the implemented
  mapping-only, snapshot, atomicity, or presence-based prior-value behavior.

GitNexus was current at commit `3a43d5f`. The shared Darkmatter parser is
CRITICAL risk with 46 upstream impacts, four indexed processes, and eight
modules; Phase 5 does not edit that parser. `parse_lifecycle_config` is LOW
risk with seven upstream test/preflight impacts. The corpus test symbol is not
indexed (`UNKNOWN`); text search confirms it is an isolated test entry point
with no production callers.

### Artifact, documentation, and comment migration

- The real `prompts/_implement/implement-plan.md` was already canonical from
  Phase 2. Its CLI fixture intentionally omits audio and commit shell actions,
  but its lifecycle `set` payloads match the real prompt. The remaining DMLS
  sequence-descent fixture now carries the reported null reset and two-key
  mapping exactly.
- The passive corpus test now includes the DMLS fixture directory. The
  `shipped_prompts` binary passed 4/4, including the corpus test and normal-path
  fake-provider regression. The consuming DMLS test
  `expression_pass_materializes_each_distinct_ancestor_shape_once` passed 1/1;
  7,947 unrelated tests were filtered. An earlier attempt to pass a raw
  parenthesized Nextest expression through the package recipe failed in the
  generated shell before compilation or test execution; the supported name
  filter rerun is the recorded evidence.
- Sequence and lifecycle docs now teach only `set: {property: value}`, snapshot
  evaluation, and atomic commit. Composition docs explicitly distinguish that
  YAML shape from the retained capability/loop `set(key, value)` API. The
  active Sequence Plus spec, plan, and validation matrix use the same labels.
- Darkmatter's README, Markdown API docs, and linked skill mirrors now document
  duplicate-key rejection at every YAML mapping depth, the migration away from
  last-value-wins parsing, and unchanged explicit document-merge strategies.
- The source-comment review covered `parse_positional_action`,
  `classify_positional_value`, `dispatch_side_effect`,
  `dispatch_runtime_set`, `RuntimeState::set`, `RuntimeState::set_batch`, and
  their module docs. They already describe the mapping-only parser exception,
  positional-effect separation, pre-write snapshot, atomic batch, and
  presence-based prior values. No stale comment or production edit was found.
- An active-artifact scan found no removed lifecycle `set: [...]` or
  `action: set` spelling in shipped prompts, active Claudine docs, the active
  Sequence Plus feature, the migrated DMLS fixture, or either Claudine skill
  mirror. Completed historical specifications and intentional negative tests
  remain untouched.

### Acceptance criteria to test mapping

1. The two reported prompt mappings and a valid false-guarded mapping are
   covered by `runtime_set_parses_the_reported_mappings_as_one_typed_action`,
   `runtime_set_invalid_keys_fail_even_below_a_false_guard`, and
   `shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`.
2. Event, setup/teardown, and sequence-task surfaces are covered by
   `a_mutation_in_start_is_visible_to_a_later_event`,
   `a_mapping_set_appends_its_prior_value_object`,
   `the_mutation_delta_reports_the_keys_the_task_wrote`, and
   `a_sibling_mutation_is_invisible_until_the_group_completes`.
3. The representation matrix is covered by
   `runtime_set_parses_the_reported_mappings_as_one_typed_action`,
   `runtime_set_preserves_recursive_authored_types_and_empty_mapping`,
   `runtime_set_accepts_no_error_as_modifier_and_as_destination`,
   `a_one_property_mapping_writes_the_runtime_layer`, and
   `a_whole_value_span_keeps_its_type`.
4. Snapshot ordering and later-action visibility are covered by
   `mapping_set_swaps_values_in_either_destination_order` and
   `consecutive_set_actions_observe_each_others_updates`.
5. Atomic refusal and null-presence semantics are covered by
   `failed_expression_publishes_no_part_of_the_mapping_with_or_without_runtime`,
   `late_invalid_destination_publishes_no_part_of_the_task_side_effect`,
   `no_error_suppresses_a_batch_refusal_without_exposing_a_partial_write`,
   `mapping_result_reports_all_priors_and_preserves_explicit_runtime_null`,
   `set_refuses_every_reserved_root_key`, and `set_refuses_a_dotted_key`.
6. Every removed or nonmapping spelling is covered by
   `runtime_set_removed_and_non_mapping_forms_have_canonical_guidance`.
7. Prior-value output, empty/absent results, downstream visibility, and source
   immutability are covered by `set_can_read_an_absent_destination_as_null`,
   `mapping_result_reports_all_priors_and_preserves_explicit_runtime_null`,
   `a_mapping_set_appends_its_prior_value_object`,
   `a_mutation_in_start_is_visible_to_a_later_event`, `set_writes_no_file`,
   and the hermetic shipped-prompt regression.
8. `shipped_lifecycle_artifacts_use_mapping_only_set` passively scans the
   shipped corpus, while
   `shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`
   executes the real prompt through the normal isolated CLI path.
9. Runtime isolation and retained APIs are covered by
   `a_sibling_mutation_is_invisible_until_the_group_completes`,
   `set_rejects_every_reserved_root_key`, the
   `context_side_effects_*` CLI tests, loop-control
   `set_with_lookup_stores_typed_number`,
   `set_without_lookup_stores_raw_template_string`,
   `set_rejects_reserved_properties`, `set_assigns_new_value`, and
   Darkmatter's `set_returns_prior_value_and_mutates_in_memory`,
   `set_preserves_whole_value_types`, `set_performs_no_filesystem_write`,
   `set_rejects_empty_and_dotted_keys`, and
   `verb_signature_set_equals_descriptor_signature_set`. A production source
   audit found only the deliberate mapping parser branch and typed dispatch;
   there is no compatibility executor for removed lifecycle forms.
10. Full paths, canonical guidance, typed projections, and guarded behavior are
    covered by `runtime_set_reports_the_full_nested_value_path`,
    `runtime_set_shape_diagnostics_share_render_highlight_and_machine_identity`,
    `runtime_set_invalid_keys_fail_even_below_a_false_guard`, and
    `runtime_set_parses_the_reported_mappings_as_one_typed_action`.

### Final validation

- `just test-cli --test shipped_prompts` in `claudine/`: 4/4 passed.
- The focused DMLS fixture test
  `expression_pass_materializes_each_distinct_ancestor_shape_once`: 1/1
  passed; 7,947 unrelated tests were filtered.
- `just test` in `claudine/`: 7,278/7,278 passed, with nine tests skipped by
  the configured tier/filter policy. The linker emitted the known non-failing
  macOS compact-unwind warning.
- `just lint` in `claudine/`: passed for all five packages, error guards, and
  lifecycle documentation facets.
- `just test` in `darkmatter/`: 7,941/7,941 passed, with seven tests skipped by
  the configured tier/filter policy. This includes the duplicate-key unit
  regressions, the normal `md get` CLI rejection path, explicit merge/hash
  round trips, and the retained `EffectEngine::set` tests.
- `just lint` in `darkmatter/`: passed for `darkmatter`, `darkmatter-cli`,
  `dmls`, `zed-dmls-cli`, and the `wasm32-wasip2` Zed extension. A non-failing
  kache cross-device-copy advisory was emitted.
- `BISCUIT_L2_THREADS=4 just test-l2 --no-fail-fast` in `claudine/` initially
  passed 228/231 with 2,750 tests skipped by the L2 filter. Two capture-only
  failures, `level2_explicit_sequence_source_miss_uses_typed_diagnostic_without_picker`
  and `level2_lifecycle_loop_gate_error_fails_and_exits`, passed when rerun
  alone through the canonical L2 recipe. The pre-existing
  `level2_proxy_routes_share_identity_across_routes_in_tmux` remains
  reproducibly red: provider launch is reached, but the expected
  `failure.stack[*].proxy` diagnostic does not arrive before the harness
  deadline. It is unrelated to this documentation/artifact phase and is the
  sole human-review item.
- Phase 5 changes contain no OS-conditional production logic: they migrate
  platform-neutral YAML, documentation, skills, and corpus coverage. macOS
  evidence is recorded above; existing CI remains responsible for Linux,
  native Windows, and WSL2 without adding speculative matrix work.
- Final GitNexus `detect-changes --scope all` completed without a partial or
  truncated result: 18 changed files and 45 symbols, zero affected indexed
  processes, and LOW aggregate risk. The worktree also contains unrelated
  concurrent prompt and Messenger edits, which were not modified or included
  in the Phase 5 inventory.
- No formatting command was run.
