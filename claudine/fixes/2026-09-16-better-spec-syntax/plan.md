---
total_phases: 5
created: 2026-09-15
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
---

# Plan: Mapping-Only Claudine Lifecycle `set` Syntax

Implements [spec.md](spec.md) — make a literal key-to-value mapping the only
authored Claudine lifecycle `set` payload, with snapshot evaluation and
all-or-nothing runtime commits, typed diagnostics, and migration of every
active artifact off the removed forms.

## Work Summary

The grammar change touches four layers of one shared pipeline:

1. **Parse layer** (`claudine/lib/src/composition/lifecycle/`): today a
   single-key action object `{set: <value>}` routes through
   `parse_positional_action` (`action_shape.rs:1`) whose
   `classify_positional_value` rejects object payloads with
   `LifecycleObjectDataThroughInterpolationPositional` — the exact
   `object value not allowed here` error from the bug report
   (`parse.rs:734` `parse_stack_item_action_object`, `parse.rs:870`
   `parse_long_form_action_object` for the `{action: set, key:, value:}`
   form). `set` becomes a deliberate grammar exception: the verb accepts
   **only** a mapping of literal string keys to typed values, represented as
   a recursive typed action-value tree analogous to
   `ProxyWithValue` (`actions.rs:217`). Positional, long-form, and any
   call-spelling variants become typed migration-guidance errors.
2. **Executor layer** (`executor.rs:1272` `dispatch_side_effect`,
   `executor.rs:1367` `apply_runtime_set`): the mapping is one typed action
   and one runtime-state transaction — all values resolve against the
   pre-action working state, all keys validate, then the complete mutation
   map commits. The single-key `verb == "set"` arm is replaced by the batch
   path; no compatibility branch remains.
3. **Runtime layer** (`runtime_state.rs:120` `RuntimeState::set`): add a
   batch operation at Claudine's boundary that validates/resolves outside
   the lock and commits under one mutex acquisition, selecting prior values
   by **key presence** rather than null-sentinel matching (fixing the
   existing ambiguity at `runtime_state.rs:134-137` where a runtime null
   incorrectly falls back to a non-null document value). Darkmatter's
   `EffectEngine::set` (`darkmatter/lib/src/effects/verbs.rs:73`), its
   `EFFECT_DESCRIPTORS` row (`darkmatter/lib/src/effects/catalog.rs:64`),
   and the loop-control `set(prop, value)` DSL
   (`claudine/lib/src/composition/looping/`) are explicitly out of scope and
   must keep compiling and passing unchanged.
4. **Consumers**: sequence `side_effect` tasks
   (`sequence/task/mod.rs:651` `run_side_effect`, `:957`
   `is_side_effect_action`, `executor.rs:533` `dispatch_task_side_effect`)
   and task `setup:`/`teardown:` stacks (`parse.rs:294`
   `parse_task_action_stack`) must accept the mapping through the same
   shared parser and serialize the prior-value result object through the
   existing output path.

Downstream of the code change: every active artifact authored in the old
forms must migrate (shipped `prompts/_implement/implement-plan.md`,
in-repo lib/CLI tests, docs, the active sequence-plus feature spec), and the
 Claudine skill/docs must label the two surviving `set` surfaces
(`set(key, value)` capability/loop API vs `set: {property: value}` lifecycle
YAML).

### What Successful Completion Looks Like

- The reported mappings from `prompts/_implement/implement-plan.md`
  (`set: {epilog: "{{message_to_agent}}", message_to_agent: null}` and
  `set: {epilog: null}`) parse and execute through the normal lifecycle
  path, including under a false `when`.
- The mapping shape works across event stacks, nested action lists, task
  `setup`/`teardown`, and sequence `side_effect` tasks through the shared
  parser and executor.
- The swap example yields `left: B, right: A` in either key order; a bad key
  or failed expression leaves the shared runtime cell and working map
  untouched; an explicitly null prior runtime value no longer falls back to
  a non-null document value.
- Removed forms fail with typed, actionable diagnostics naming the source
  document and a stable semantic action path, offering only the canonical
  `set: {property: value}` rewrite, with identical selection across
  terminal, `err.*`, and machine projections.
- A hermetic CLI regression exercises the real implementation prompt's
  mapping shape via the normal invocation path (isolated from the separate
  initialize-after-proxy bug), and a passive corpus check covers all active
  shipped artifacts.
- `just test`, `just test-l2`, and `just lint` pass in the claudine package
  area; loop-control `set(...)` and Darkmatter positional `set` coverage
  still passes untouched; no compatibility branch for removed forms remains
  in production parsing or execution.

## Phase 1 — Rulings, Spikes, and Baseline

No production code changes. Close every open design question so Phase 2 is
pure execution, and record the baseline the later phases are measured
against.

### Necessary Rules

The following rulings are required before Phase 2 begins. Each records the
recommended resolution; the author must confirm or override.

- [ ] **Ruling 1 — Where reserved-key validation happens for the mapping.**
  Recommendation: structural malformation (non-mapping payload, non-string /
  empty / interpolation-bearing keys, `set: "{{mapping}}"` whole-span form)
  fails at **parse time**, including under a false `when` (R1's "statically
  well-formed"). Reserved-root-key refusal (`state`, `previous`, `next`,
  `outputs`, `sequence_id`) stays an **execution-time** dispatch failure, as
  it is today for `set: [outputs, …]`, so `no_error` suppression and
  `when`-guard laziness keep their existing lifecycle contract (R1: `no_error`
  suppresses "an evaluation or dispatch failure"). Acceptance criterion 10's
  "static malformed mappings fail even under a false `when`" reads on
  structural shape only.
- [ ] **Ruling 2 — Duplicate destination keys.** R1 requires duplicates to
  "fail during YAML parsing rather than silently selecting one value".
  Spike 1 determines whether Darkmatter's frontmatter YAML parsing already
  rejects duplicate mapping keys. If it silently last-wins, the ruling must
  decide the enforcement point (frontmatter parser behavior is shared with
  Darkmatter and may be out of bounds; the fallback is a typed parse error
  at the `set` mapping boundary, which is only reachable when Claudine's
  serde path has already collapsed duplicates — in that case the spike's
  finding decides whether a Claudine-side guard is possible at all, or
  whether the spec requirement is met by the YAML layer).
- [ ] **Ruling 3 — Action representation.** Recommendation: a dedicated
  `LifecycleActionKind` variant (e.g. `RuntimeSet`) holding an
  order-preserving `IndexMap<String, …>` of a recursive typed value tree —
  a `ProxyWithValue`-shaped enum (`Scalar(Expr)` / `Null` / `Array` /
  `Object`). This is the spec's "single mapping-based action representation"
  (never desugared into single-key actions). `is_side_effect_action` and the
  side-effect-task dispatch must accept the new kind so sequence
  `side_effect: {set: {…}}` remains executable work. Decide whether to
  generalize `ProxyWithValue` into one shared authored-value type or add a
  sibling type; prefer whichever leaves one definition (Rule 2), but do not
  rename `ProxyWith`'s public surface in this change.
- [ ] **Ruling 4 — `no_error` as a sibling of the single-key object.**
  `{set: {…}, no_error: true}` currently falls into the multi-key ambiguity
  error in `parse_stack_item_action_object` (`parse.rs:751-783`).
  Recommendation: extend that disambiguation to treat `no_error` as the one
  universal modifier key permitted beside a single verb key, parsed with the
  existing boolean check, so the spec's sibling-modifier example parses
  (`{proxy, with}` already established the precedent of a verb-scoped
  exception; this one is verb-universal per R1).
- [ ] **Ruling 5 — Diagnostics strategy.** Which `CompositionError` variants
  are new versus reused: recommendation — new variants for (a) removed
  positional form, (b) removed `action: set` long form, (c) non-mapping
  payload, each carrying source path + semantic action path + canonical
  rewrite; reuse the `proxy.with` diagnostic family's path-building pattern
  (`parse.rs:988`, `action[N].set[.key]`) for value-path errors inside the
  mapping. Per R5, reuse an existing catalog code when remedy and
  disposition are unchanged; any new code/detail field must update the
  catalog and its guard tests in the same change
  (`composition/error/tests.rs`).

### Spikes

- [ ] **Spike 1 — Duplicate-key behavior of the YAML frontmatter parser.**
  Write a throwaway test against Darkmatter frontmatter parsing with a
  duplicate-key mapping and record error-vs-last-wins. Feeds Ruling 2.
  Small (≤ half day).
- [ ] **Spike 2 — Result-value plumbing audit.** Trace
  `dispatch_task_side_effect` (`executor.rs:533`) and stack execution to
  confirm where an action's `Ok(Value)` surfaces: side-effect task
  serialization (`run_side_effect`'s `Ok(other) => other.to_string()`),
  event-stack discard, and `err.*`/machine projection seams. Confirms R4's
  "where the side-effect protocol exposes an action's result" inventory and
  the exact seams Phase 3 must touch. Small.
- [ ] **Spike 3 — Migration inventory validation.** Re-run the corpus scans
  (`set: [`, `action: set`, `"set":`, `action: "set"`) over `prompts/`,
  `claudine/` excluding `_completed` trees, and record the authoritative
  file list. Known starting inventory from planning:
  `prompts/_implement/implement-plan.md:43,59-60`;
  `claudine/docs/topics/flow-control/sequences.md:378,381,468`;
  `claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs`;
  `claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs`;
  `claudine/lib/src/composition/sequence/preflight/tests.rs:1353`;
  `claudine/cli/tests/{sequence_jit.rs,level2_sequence_task_stream_capture.rs,compose_caller_file_provenance.rs,inline_completion_lifecycle.rs,composition_outputs.rs,sequence_groups.rs}`;
  `claudine/features/2026-07-11-sequence-plus/{spec.md,plan.md,validation-matrix.md}`;
  comment drift at `executor.rs:1364`. Explicit non-targets: loop-control
  `set(...)`, Darkmatter `set(key, value)` descriptor examples, jq
  expressions in `prompts/dependency-upgrade.md` (`requirement_set:` is an
  unrelated property), `fixes/_completed/**`.

### Tasks

- [ ] **Graph impact refresh**
  - Re-run `just gitnexus`, then upstream impact for
    `parse_lifecycle_config`, `parse_positional_action`,
    `dispatch_side_effect`, `RuntimeState::set`, `dispatch_task_side_effect`,
    and `is_known_side_effect` (`signatures.rs:221`); record callers,
    processes, and risk in the implementation log.
  - Planning-run findings for reference: `dispatch_side_effect` upstream is
    LOW risk (2 direct callers, Lifecycle module only);
    `parse_lifecycle_config` upstream is LOW risk (3 direct callers,
    Preflight module); `RuntimeState::set` callers are the executor plus its
    own tests.
- [ ] **Baseline capture**
  - Run `just test` and `just test-l2` in the claudine package area and
    record the pre-change pass/fail baseline (any pre-existing failures
    must be distinguishable from regressions introduced by this change).
  - Run `just lint` and record a clean baseline.

Validation checkpoint: rulings recorded in the implementation log with the
author's confirmation; Spike 1-3 findings written up; baseline outputs
captured. No source files modified.

## Phase 2 — Parse Layer: Mapping-Only Grammar and Typed Diagnostics

Replace the authored grammar at the parse layer and migrate every lib-level
test that authors the old forms, so the library compiles and its unit tests
pass at the end of this phase.

### Tasks

- [ ] **Typed value tree** (prerequisite for all other Phase 2 tasks)
  - Introduce the recursive authored-value tree per Ruling 3
    (`Scalar(Expr)` / `Null` / `Array` / `Object`), order-preserving for
    mappings, following the `ProxyWithValue` typing rules
    (`actions.rs:209-303`): string values keep whole-value `{{ … }}` spans
    typed, YAML scalars/nulls/arrays/objects keep their authored types,
    object values assign as complete values (no implicit merge).
  - Constructor rejects interpolation-bearing destination keys (analogous to
    `ProxyWithError::DynamicKey`).
- [ ] **Mapping-only `set` parsing**
  - In `parse_stack_item_action_object` / `parse_positional_action`
    (`parse.rs:734`, `action_shape.rs:1`): special-case verb `set` — the
    payload must be a mapping (empty allowed); build the new action kind
    from the typed tree; keys must be literal, nonempty strings (dotted and
    empty key shapes surface through the existing runtime key-shape rule in
    Phase 3; dynamic keys fail here at parse).
  - Reject `set: "{{mapping}}"` whole-span form and any call spelling with
    a typed non-goal error (mirror `LifecycleProxyWithWholeMapping`
    `parse.rs:1007`).
  - Implement Ruling 4: `{set: {…}, no_error: true}` parses; `no_error` is
    never a destination key at the mapping's top level but remains writable
    nested under `set` (i.e. `set: {no_error: x}` is a normal assignment).
  - Structural validation runs regardless of a false `when` (parse always
    happens; no eager value evaluation).
  - Apply Ruling 2's outcome for duplicate destination keys.
- [ ] **Removed-form diagnostics**
  - `set: [key, value]` positional, `{action: set, key: …, value: …}`
    long-form, and any formerly accepted lifecycle call spelling of `set`
    fail with typed errors identifying the source document and a stable
    semantic action path (e.g. `initialize.stack[0].action[1].set.epilog`),
    showing the accepted shape `set: {property: value}`; extend the
    stack-path annotator (`{event}.stack[{i}]` prefix) to root `set`
    diagnostics like `proxy.with`'s `action[N].with` pattern.
  - Wire the variants through the full `CompositionError` surface: display,
    render (`error/render/lifecycle.rs`), frontmatter excerpt selection
    (`error/mod.rs` `frontmatter_block_spec`), facets/`err.*`/machine
    projections — per Ruling 5 and the catalog guard tests
    (`composition/error/tests.rs`) updated in the same change.
  - Do not reuse the current "avoid object values" instruction
    (`LifecycleObjectDataThroughInterpolationPositional`'s hint) for `set`.
- [ ] **Lib test migration and parse coverage**
  - Migrate `executor/tests/runtime_set.rs` and
    `lifecycle/tests/action_shape_control.rs` (plus any other lib test the
    Spike 3 inventory lists) to the mapping form; keep every existing
    behavioral assertion (reserved keys, dotted keys, typed values, runtime
    visibility, no-file-write) intact — only the authored grammar changes.
  - Add parse tests: one key, multiple keys, empty mapping, native vs
    quoted scalars, null, arrays, nested objects, `no_error` as modifier and
    as literal destination key, duplicate keys (per Ruling 2), removed forms
    failing with guidance, statically malformed mapping failing under a
    false `when` while a valid guarded mapping does not evaluate or mutate.

### Work-Group A (concurrent after the typed value tree lands)

- [ ] Removed-form + payload diagnostics and their render/facets/catalog
  guard tests (independent of executor work; needs only the new error
  variants and parse seam).
- [ ] Runtime batch API groundwork on `RuntimeState` (validate-then-commit
  under one lock; key-presence prior selection) — compiles independently of
  the parser; its executor wiring lands in Phase 3.

Validation checkpoint: `just test-library` green in the claudine package
area; new parse tests cover the R1 matrix; a `rg 'set: \['` over
`claudine/lib` returns no production or test authoring of the removed
lifecycle form (comments and prose updated, not left drifting).

## Phase 3 — Execution Semantics: Snapshot Evaluation and Atomic Batch

Implement the executor and runtime semantics (R2-R4) and their test matrix.

### Tasks

- [ ] **Executor batch dispatch**
  - Dispatch the new action kind: evaluate every value against the
    pre-action working state (recursive resolution through arrays/objects;
    whole-value spans keep types; mixed strings keep interpolation
    semantics), validate every destination key, then commit the complete
    mutation map — validate/resolve **outside** the lock, commit under one
    mutex acquisition (R3).
  - Without a shared runtime cell, perform the same validation and snapshot
    evaluation before changing the working map; reserved-key and key-shape
    refusals leave both layers unchanged by this action.
  - Update the executor's working map only after the commit succeeds;
    successful updates are visible to later actions in the same stack, later
    events, loop iterations, and serial tasks per the existing contract.
  - Remove the single-key `verb == "set"` arm in `dispatch_side_effect`
    (`executor.rs:1317-1327`) and `apply_runtime_set`; no compatibility
    branch remains in production execution.
- [ ] **Runtime batch semantics**
  - Batch operation on `RuntimeState` (from Work-Group A): prior values
    selected by key presence — an explicitly assigned null is present state;
    this must also correct the single-key ambiguity at
    `runtime_state.rs:134-137` (runtime null falling back to a non-null
    document value). Keep `RuntimeState::set`'s single-key public behavior
    consistent with the presence rule or fold it into the batch path —
    without broadening or breaking Darkmatter's `EffectEngine::set`.
- [ ] **Result semantics**
  - The action's result is one object mapping each assigned key to its
    prior effective value (null for absent; empty update → empty object);
    `run_side_effect` serializes it through the existing textual output path
    (`Ok(other) => other.to_string()`) and appends one `outputs` entry
    (Spike 2's audit confirms the seams).
- [ ] **Execution test matrix** (R3/acceptance 4-5, 7)
  - Swap example succeeds in either key order; consecutive actions observe
    each other's updates.
  - Failure following an otherwise valid entry produces no partial update,
    with and without a shared runtime cell; reserved-key and failed-
    expression cases assert both `runtime.snapshot().mutations` and the
    working/live map are clean.
  - Explicitly null prior runtime value over a non-null document value is
    reported as null and does not resurrect the document value.
  - `no_error` suppression never exposes a partial write; authored
    `state`/`previous`/`next` views stay immutable; no source file is
    written.
  - Mutation-visibility tests (cross-event, cross-iteration) still pass on
    the mapping grammar.

Validation checkpoint: `just test-library` green; the R2-R4 matrix above has
one named test per behavior; a targeted regression exists for the reported
`implement-plan.md` mapping shape at the library level (event stack with
`when:` guards).

## Phase 4 — Sequence Surface, CLI Migration, and Hermetic Regression

Extend the green state to the whole package area: sequence consumers, all
remaining in-repo tests, the hermetic CLI regression, and the passive
corpus check.

### Tasks

- [ ] **Sequence consumer verification**
  - `side_effect: {set: {…}}` parses via `parse_single_action`
    (`parse.rs:330`), classifies as executable side-effect work
    (`is_side_effect_action`, `task/mod.rs:957`), executes through
    `dispatch_task_side_effect`, and serializes the prior-value object as
    its `outputs` entry; `setup:`/`teardown:` stacks accept the mapping via
    `parse_task_action_stack`.
  - Serial/parallel group isolation and visibility boundaries are unchanged
    by this syntax (assert a parallel member's writes still land only in its
    private cell until the post-group merge).
- [ ] **CLI test migration** (work-group: one task per file, concurrent)
  - Migrate `claudine/cli/tests/sequence_jit.rs`,
    `level2_sequence_task_stream_capture.rs`,
    `compose_caller_file_provenance.rs`, `inline_completion_lifecycle.rs`,
    `composition_outputs.rs`, `sequence_groups.rs`, and
    `claudine/lib/src/composition/sequence/preflight/tests.rs` to the
    mapping grammar, preserving each test's behavioral assertions.
- [ ] **Hermetic CLI regression** (acceptance 8)
  - A `CliProcessFixture`-backed, fake-provider, isolated-state,
    suppressed-audio CLI test composes/executes the **real**
    `prompts/_implement/implement-plan.md` mapping shape through the normal
    invocation path (the shipped-prompt contract machinery copies the real
    tree), isolating this syntax change from the separately specified
    initialize-after-proxy bug; terminal/browser windows must not gain
    focus.
- [ ] **Passive corpus coverage** (acceptance 8)
  - A corpus test walks all active shipped artifacts (prompts, fixtures,
    gen inputs) and asserts none authors a removed `set` form and every
    `set` occurrence parses under the new grammar; `claudine/schemas/` and
    the schema-trigger catalog are checked for any descriptor advertising
    the old YAML forms (planning found none — the check makes it stay true).

Validation checkpoint: `just test` and `just test-l2` green in the claudine
package area; the hermetic regression demonstrably fails if the mapping form
is reverted (verify once by stashing the parse change).

## Phase 5 — Shipped Artifacts, Docs, Skill, and Final Validation

Migrate user-facing artifacts and close out with the full validation sweep
and acceptance audit.

### Tasks

- [ ] **Prompt migration**
  - Rewrite `prompts/_implement/implement-plan.md:43,59-60` to the canonical
    mapping form (`set: {epilog: "{{message_to_agent}}", message_to_agent:
    null}` and the null reset), removing the temporary two-positional-action
    workaround; review `prompts/depencency-upgrade.md` (non-target per Spike
    3) stays untouched.
  - Review `claudine/cli/tests/fixtures/shipped_implement_route/` for
    byte-copy drift against the migrated shipped prompt and sync if the
    fixture mirrors it.
- [ ] **Documentation updates** (work-group: docs, skill, and spec migration
  are independent files, concurrent)
  - `claudine/docs/topics/flow-control/sequences.md` — rewrite "Mutating
    state with `set`" (line 370+) and the `setup:` example (line 468) to
    the mapping-only grammar; add the two-`set`-surfaces labeling rule
    (`set(key, value)` = capability/loop API, `set: {property: value}` =
    Claudine lifecycle YAML) wherever both appear, including
    `docs/topics/composition.md` and the `claudine context --side-effects`
    presentation if it implies the lifecycle YAML form.
  - Claudine skill (`.claude/skills/claudine/` and any `.opencode/skill/
    claudine/` mirror): update lifecycle/sequence guidance that shows `set`
    authoring.
  - Active feature spec `claudine/features/2026-07-11-sequence-plus/`
    (`spec.md:113,416-417,570,600`, `plan.md`, `validation-matrix.md`):
    migrate old-form examples; leave `fixes/_completed/**` intact.
- [ ] **Comment-drift review**
  - Behavior-changing symbols get a doc/comment pass in the same change:
    `dispatch_side_effect`, `apply_runtime_set` (deleted or rewritten),
    `RuntimeState::set`/batch, `parse_positional_action`,
    `classify_positional_value`, and `executor.rs:1364`'s now-drifted
    comment; module docs in `actions.rs`, `runtime_state.rs` ("what the
    `set` side effect writes"), and `parse.rs` disambiguation docs.
- [ ] **Final validation sweep**
  - `just test`, `just test-l2`, `just lint` in the claudine package area;
    record evidence per repository policy (reuse qualifying passing
    evidence; macOS host evidence recorded; flag Linux/native-Windows/WSL2
    coverage needs for CI without adding speculative matrix cells).
  - Run GitNexus `detect_changes` (scope `all`) and review the affected-
    processes/risk report before hand-off; `partial: true` or
    `truncated: true` is not a clean check — re-run it.
  - Produce the acceptance-criteria → test mapping (all 10 criteria) with
    the exact targeted tests, broader gates run, and every skipped or
    pre-existing failure listed.
  - Verify Darkmatter's `EffectEngine::set`, `EFFECT_DESCRIPTORS` row,
    `claudine context --side-effects` signature, and loop-control
    `set(...)` tests pass unchanged (acceptance 9).

Validation checkpoint: corpus scan clean repo-wide (excluding `_completed`
and documented non-targets); all 10 acceptance criteria mapped to passing
tests; validation evidence recorded in the implementation log; plan ready
for review — do not move the fix to `_completed` (author's action).
