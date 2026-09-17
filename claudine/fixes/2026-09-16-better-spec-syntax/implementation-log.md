---
spec: "claudine/fixes/2026-09-16-better-spec-syntax/spec.md"
plan: "claudine/fixes/2026-09-16-better-spec-syntax/plan.md"
implemented_by: "codex/gpt-5.6-sol"
started_phase: "2"
implemented: false
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
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
completed_phase: "2"
human_review: false
human_review_items: []
message_to_agent: >-
    Phase 2 is complete; begin Phase 3. Mapping-only parsing, typed diagnostics,
    shared duplicate-key rejection, and RuntimeState batch groundwork are in
    place. Executor wiring was added to keep the migrated corpus green, but
    Phase 3 must still perform its dedicated snapshot/atomicity/result audit
    and remove the remaining legacy single-key executor helper and dispatch arm.
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
