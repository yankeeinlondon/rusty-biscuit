---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-17-remove-strict-mode/spec.md
plan: claudine/fixes/2026-09-17-remove-strict-mode/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - claudine/docs/rollout-strategy.md
    - claudine/fixes/2026-09-17-remove-strict-mode/design.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/tests/l1/binding_contract.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/tests/l1/absent_property_contract.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/cli/tests/l1/compose_unknown_identifiers.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/interpolation_conformance.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/strict_mode_provenance_spike.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/authored_text_rendering.rs
    - prompts/_reviews/performance-review.md
    - prompts/brainstorm.md
docs_updated_during_phase_3:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/inline/fm-interpolation.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
---

# Implementation Log for 2026-09-17-remove-strict-mode (7 phases)

## Phase 1

Phase 1 is planning-only: rulings, a test baseline on the unmodified tree, a
refresh of the lookup inventory, a seam impact check, and a prompt exposure
audit. No source file is changed in this phase.

### Rulings

Recorded 2026-10-01 by claude/opus. The plan's Phase 1 allows each ruling to
be recorded "by the owner or by accepting the stated recommendation", and the
plan runs with `yolo: "true"`; every ruling below accepts the plan's stated
recommendation. **Decided by:** claude/opus on behalf of the owner (Ken
Snyder), under that mandate. NR-3 carries an explicit "owner sign-off
required" note, so it is also raised in the spec's `human_review_items`; the
owner may overrule any ruling before Phase 2.

| # | Ruling | Outcome |
|---|---|---|
| NR-1 | Design approval gate (rollout ruling 2) | **Accepted.** The plan review covers the in-scope design artifacts. Recorded in `claudine/docs/rollout-strategy.md` (ruling 2 row, the "Next" paragraph, step 3's "Needs first", and the Evidence status row) and in `design.md`'s status line |
| NR-2 | Plain `serde_json::Value` instead of `ValueEnvelope` | **Accepted.** `ResolvedBinding`, `RuntimeBinding::Eager`, lazy providers, and `evaluate_prepared` carry `Value`; lazy providers keep the `Fn() -> Value` shape; no provenance/stage/policy metadata |
| NR-3 | Reserved names come from `reserved_root_descriptors()` (`doc`, `ctx`, `env`, `current`, `current_env`) | **Accepted, pending owner sign-off** (see below). `current` leaves C3's Claudine catalog |
| NR-4 | `timing`, not `tracking` | **Accepted.** The catalog declares `err`, `timing`, `group`; no rename |
| NR-5 | One resolution channel: evolve `get_checked` into `resolve` | **Accepted.** `resolve(&self, path) -> Result<ResolvedBinding, ExpressionError>` with a default wrapping `get` as `Document`; add `binding_view()` (default `None`) and `format_resolved`; `ContextNotCaptured`/`ContextProjectionInvariant` move onto it unchanged |
| NR-6 | Prepared representation scope | **Accepted.** Ship `prepare_value`, `validate_prepared`, `evaluate_prepared`; prepared identity is the `BindingView` identity only; new error families only for binding configuration and unavailable bindings |
| NR-7 | `dm.expression.unknown_identifier` → `dm.expression.undeclared_property` | **Accepted.** One-change rename with no alias across library, DMLS, `md` CLI tests, and docs; message "`{root}` is an undeclared document property (unknown type; `null` unless supplied at runtime)"; drop context-descriptor names from `is_statically_known_root`; keep fallback/ternary suppression |
| NR-8 | Loop `_loop_*` names | **Accepted.** Ordinary data through the default `resolve`; only the `ctx` fallback is removed; no loop catalog |
| NR-9 | `ApprovedCommand` for setup/teardown | **Accepted.** Claudine-private `ApprovedCommand { site, bytes: String }`, no envelope |
| NR-10 | Delete `strict_mode_provenance_spike.rs` | **Accepted.** Deleted when `.strict()` is removed (Phase 3, Wave 7); findings stay in `spike-results.md` |
| NR-11 | Owner-visible behavior changes (bare `err` in `finalize`/teardown → explicit `null`; bare `group` outside a group → typed unavailable error; no bare-name `ctx` fallback) | **Accepted.** Reach measured by the prompt exposure audit below |

**Widened acceptance criterion 2 (NR-3).** The spec's criterion 2 and its
Verification items say registrations named exactly `doc`, `ctx`, or `env`
fail. Under NR-3 registration rejects every root in
`reserved_root_descriptors()`: `doc`, `ctx`, `env`, `current`, and
`current_env`. The spec is a snapshot and is not edited; Phase 7's acceptance
map records this as a departure.

### Lookup inventory refresh (Phase 3 migration checklist)

Re-verified against source on 2026-10-01. **23 compiled `EvaluationLookup`
implementations plus 2 `SimpleLookup` rustdoc examples** (12 production, 11
test-only), confirming the plan's count. The inventory's 21 rows all still
exist. `darkmatter/features/2026-09-21-schema-enhancements/spikes/coercion-baseline/`
has no `Cargo.toml` and is not compiled (GitNexus does index it, so its
`main` shows up in `EvaluationLookup` impact results; ignore it). A further
uncompiled example lives in `darkmatter/docs/topics/darkmatter-expressions.md:1009`
and is a Phase 7 doc concern only.

DM = `darkmatter/lib/src/markdown/compose/`, CL = `claudine/lib/src/`. IKVR =
`is_known_variable_root` (trait default `true`, `expression/mod.rs:301`).

| # | Type | File:line | Role | IKVR | Bare-name `ctx` fallback | Test-only |
|---|---|---|---|---|---|---|
| 1 | `EffectiveState` | DM `context/effective_state.rs:414` | Overrides `get_checked` | yes | **yes**: inherent `get` `.or_else(get_context_value(path))` (:247); `get_checked` via `into_checked_bare_name` (:277) | no |
| 2 | `ResolvingLookup` | DM `context/effective_state.rs:493` | Wraps #1 plus resolution context | forwards | inherited from #1 | no |
| 3 | `FrontmatterSeedState` | DM `frontmatter_interpolation.rs:124` | Seed state; overrides `get_checked` | yes (accepts bare ctx variable names) | `get`: none; IKVR still treats bare ctx names as known | no |
| 4 | `ShortcutLookup` | DM `conditions.rs:364` | Condition shortcut | no | **yes**, explicit: `get` → `ctx.{path}` (:388), `get_checked` (:400) | no |
| 5 | `CtxLookup` | DM `expression/ctx.rs:109` | Context-only | no | none | no |
| 6 | `LayeredLookup` | DM `subtree.rs:229` | Globals over #1 | yes | inherited from #1 | no |
| 7 | `FsLookup` | DM `expression/catalog/mod.rs:642` | Fixture | no | none | yes |
| 8 | `FixtureLookup` | DM `expression/catalog/mod.rs:973` | Fixture | no | none | yes |
| 9 | `MapLookup` | DM `expression/catalog/mod.rs:1166` | Fixture | no | none | yes |
| 10 | `FixtureLookup` | DM `expression/semantics.rs:810` | Fixture | no | none | yes |
| 11 | `TestLookup` | DM `expression/mod.rs:935` | Fixture | no | none | yes |
| 12 | `Nothing` **(new)** | DM `expression/absence.rs:398` | All-missing test lookup | yes (`false`) | none | yes |
| 13 | `DeferrableLookup` **(new)** | DM `inline/interpolation.rs:163` | Forwards every method to #2; `get_checked` maps `ContextNotCaptured` → `Ok(None)` when deferring | forwards | inherited from #1 | no |
| 14 | `Lookup` | `darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs:11` **(moved)** | Fixture | no | none | yes |
| 15 | `Lookup` | `darkmatter/lib/tests/l1/predict_conflicts.rs:147` **(moved)** | Fixture | no | none | yes |
| 16 | `SizedLookup` | CL `composition/looping/actions.rs:249` | Wrapper; forwards **only** `get`/`get_string` | no | none | no |
| 17 | `LoopExpressionLookup` | CL `composition/looping/expression.rs:139` | Loop, ambient, frontmatter | no | none (`ctx.` prefix only) | no |
| 18 | `MapLookup` | CL `composition/looping/actions/tests.rs:12` | Fixture | no | none | yes |
| 19 | `SourceExpressionLookup` | CL `composition/sequence/expr.rs:75` | Item → env → frontmatter → `resolve_ctx` | no | none (`resolve_ctx` needs the `ctx.` prefix) | no |
| 20 | `EventMetaExpressionLookup` | CL `dispatch/expression.rs:85` | Event-meta projection | no | none | no |
| 21 | `EventMetaConditionLookup` | CL `dispatch/expression.rs:169` | Wrapper; `ctx*` → `CtxLookup`, else inner; forwards only `get`/`resolution_context` | no | none | no |
| 22 | `MapLookup` | CL `composition/lifecycle/tests/action_shape_control.rs:1410` | Fixture | no | none | yes |
| 23 | `EmptyLookup` | CL `composition/lifecycle/tests/action_shape_control.rs:1418` | Fixture | no | none | yes |
| R1 | `SimpleLookup` | DM `expression/mod.rs:180` | Rustdoc example | no | none | doc |
| R2 | `SimpleLookup` | DM `expression/mod.rs:413` | Rustdoc example | no | none | doc |

Deltas against `migration-inventory.md`: 2 new (#12, #13), 2 moved (#14,
#15), none removed, 11 line drifts in unchanged files (rows 1–9, 11, 16). The
inventory's prose count "21 actual lookup implementations" is now 23.

Findings that refine later phases:

- **The bare-name `ctx` fallback lives in only two places:** `EffectiveState`
  (#1) and `ShortcutLookup` (#4). `ResolvingLookup`, `LayeredLookup`, and
  `DeferrableLookup` inherit it from #1. `FrontmatterSeedState`'s `get` no
  longer falls back, but its IKVR still admits bare ctx names.
- **`LoopExpressionLookup` and `SourceExpressionLookup` have no `ctx`
  fallback today**, contrary to NR-8's and Wave 9's "remove the `ctx`
  fallback" wording. For them Phase 4 only needs the forwarding/session work;
  there is no fallback to delete.
- **Two Claudine wrappers forward only part of the trait:** `SizedLookup`
  forwards only `get`/`get_string`, and `EventMetaConditionLookup` only
  `get`/`resolution_context`. Neither forwards `get_checked` today, so they
  silently drop the checked channel. Phase 3/4 must make both forward
  `resolve`, `binding_view`, and `format_resolved`.
- **`is_known_variable_root` call sites (3):** `expression/mod.rs:461`
  (`observe_missing`, runtime unknown-root observer), `subtree.rs:539` (strict
  root rejection), `unknown_identifiers.rs:44` (warning candidate filter).
  Overrides (6): `effective_state.rs:451`, `effective_state.rs:528`,
  `frontmatter_interpolation.rs:229`, `subtree.rs:280`,
  `inline/interpolation.rs:195`, `expression/absence.rs:402`.

### Seam impact check

GitNexus (index one commit behind HEAD; the trailing commit touched only
planning files) resolved `EvaluationLookup` as CRITICAL: 20 direct, 79 total
upstream, 4 affected processes (`run_stage`, `run_compose_pipeline_node`,
`build_node`, plus the uncompiled spike `main`). `SubtreeCompose`,
`InjectedGlobal`, `LayeredLookup`, and `CompositionError` came back
ambiguous/`UNKNOWN` (no resolved callers), and `lifecycle_injected_globals`
was not found in the index, so all five were verified by text search.

Callers the plan and inventory do not name (each must compile after Phase 3
and be reviewed in Phase 4):

| Symbol | Unlisted users |
|---|---|
| `SubtreeCompose` | `claudine/cli/tests/l1/composition_seams.rs`, `darkmatter/cli/tests/l1/compose_schema.rs`, `darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs` |
| `InjectedGlobal` | `darkmatter/lib/src/markdown/compose/tests/frontmatter.rs`, `.../tests/lazy_roots.rs` |
| `LayeredLookup` | `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs`, `claudine/lib/src/composition/lifecycle/context/tests.rs` |
| `lifecycle_injected_globals` | `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs`, `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs`, re-export in `claudine/lib/src/composition/mod.rs` |
| `SubtreeStrictness` | `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs`, `darkmatter/lib/src/markdown/compose/expression/mod.rs`, `darkmatter/lib/src/markdown/compose/tests/frontmatter.rs` |
| `CompositionError::LifecycleUndefinedVariable` | Renderer branches are in the **library**, not `claudine/cli` as Wave 10 says: `claudine/lib/src/composition/error/render/mod.rs` (:45, :413, :822), `render/lifecycle.rs:116`, and the frontmatter-highlight arm in `error/mod.rs:3242`. Tests: `lifecycle/tests/validation.rs` (9 tests) and `lifecycle/tests/diagnostics.rs:392` |

`CompositionError` as a whole is used in 146 files across `claudine/lib` and
`claudine/cli`; Wave 10 only deletes one variant and adds a typed cause, so
only the sites above are affected. Note also the neighboring
`CompositionError::LifecycleErrNotAvailable`, which overlaps the `err`
availability that the Wave 8 catalog now owns; Phase 4 should decide whether
it becomes the catalog's unavailable error or stays a parse-time check.

### Prompt exposure audit

Searched `prompts/`, Claudine prompts/fixtures/docs, and Darkmatter fixtures
(excluding `_completed`, `features/`, `fixes/`, `target/`). The `ctx`
fallback was confirmed empirically with `md compose` in a scratch repository
on branch `feat/xyz`: an undeclared bare `{{branch}}` rendered `feat/xyz`
(through `effective_state.rs:243-247`), while a `branch` declared in
`$schema` but unsupplied rendered empty. So **only names declared nowhere in
the document relied on the fallback**; schema-declared optional properties
never did.

**A. Fallback guards** — Phase 6 work list.

| File:line | Snippet | Class | Phase 6 action |
|---|---|---|---|
| `prompts/implement.md:53-55` | comment "`\|\| false` is the guarded-optional form…" | teaches workaround | delete |
| `prompts/implement.md:56` | `when: "review \|\| false"` | legality guard | → `when: "review"` |
| `prompts/implement.md:64-67` | `(spec \|\| false) ? '- a specification file…' : ''`, same for `plan`/`review`, and `!(spec\|\|false) && …` | legality guard | plain ternaries |
| `prompts/implement.md:75-77` | `(spec \|\| false) ? '✔' : '<red>⤫</red>'` (+ plan, review) | legality guard | plain |
| `prompts/implement.md:80, 82` | `(spec \|\| false) ? … : (review \|\| false) ? …` | legality guard | plain |
| `prompts/implement.md:81` | `(frontmatter(spec, "implemented") \|\| false)` | **real default** | keep (renders `false` on purpose) |
| `prompts/implement.md:23` | `frontmatter(spec,'review_iterations') \|\| 1` | real default | keep |
| `prompts/review.md:31-35` | workaround comment | teaches workaround | delete |
| `prompts/review.md:36, 42, 45` | `when: "spec \|\| false"` / `"plan \|\| false"` / `"review \|\| false"` | legality guard | plain |
| `prompts/_prompt.md:89` | "Guard it with a fallback: `{{{ title \|\| '' }}}` … `review \|\| false` in a `when:`" | teaches workaround | rewrite: absent is `null`; a fallback chooses a default |
| `prompts/pr.md:52-54, 62-64`; `prompts/_pr/dirty.md:39-41`; `prompts/_pr/push.md:49-50` | `branch: "{{ branch \|\| '' }}"`, same for `title`/`about`, in proxy `with:` | **real default** (reclassified) | keep: unguarded they would pass `null`, not `''`, into a callee whose property is typed `string`. Removing them is a behavior change and out of scope for R8 |
| `prompts/_pr/dirty.md:30` | `about \|\| 'These are the uncommitted changes…'` | real default | keep |
| `prompts/pr.md:32` | `remote_vendor() \|\| 'an unrecognized provider'` | real default | keep |
| `prompts/code-comment-quality.md:15`, `prompts/merge-conflicts.md:3`, `prompts/_reviews/performance-review.md:2,4` | `ctx.*` roots with `\|\| null` / `\|\| ''` | not affected | leave |
| `claudine/docs/topics/flow-control/lifecycle.md:92, 835-839, 859` | "opt in with explicit fallback syntax `{{ maybe \|\| '' }}`"; "fails … closed via Darkmatter's strict mode" | teaches workaround / stale | Phase 7 rewrite |
| `claudine/docs/topics/flow-control/flow-control-reference.md:127` | "an unknown root" as an evaluation failure | stale | Phase 7 |

Side observation: `prompts/implement.md:29` (`when: "spec && pending_review && …"`)
and `prompts/review.md:30` (`when: "spec && (…)"`) already read `spec`
unguarded, contradicting their own comments. Moot after the fix.

**B. Bare `err` in `finalize`/teardown.** No prompt or fixture uses it. Doc
examples `flow-control.md:147`, `lifecycle.md:681-683` (`finalize: … when:
"err"`) and `docs/research/reasoning-level/_fleet.md:101,106` (`err &&
err.category == 'cap'`) stay correct: an explicit `null` `err` makes the gate
false, which is the intended result.

**C. Bare `group` outside a group.** No hits in prompts, fixtures, or doc
examples (only prose in `looping.md:318`, `flow-control.md:111`).

**D. Bare names that relied on the `ctx` fallback** — these regress silently
(render empty) once R3 lands, so **Phase 6 must fix them alongside R8**:

| File:line | Snippet | Fix |
|---|---|---|
| `prompts/_reviews/performance-review.md:342` | `{{today}} at {{time}}` (`today` is frontmatter; `time` is not) | `ctx.time` |
| `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md:342` | fixture copy of the above | keep in sync |
| `prompts/brainstorm.md:20` | `the {{area}} package area` (`area` undeclared) | `ctx.area` |
| `darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md:20` | fixture copy | keep in sync |
| `claudine/docs/topics/flow-control/lifecycle.md:611-612` | `running {{agent}}`, `git fetch origin {{branch}}` (undeclared) | Phase 7: `ctx.agent` / `ctx.branch` |
| `claudine/cli/tests/fixtures/nested_span_regression/commit.md:31` | `{{repo.name}}` (`ctx.repo` is a string, so likely already empty) | re-check expected output in Phase 3/4 |
| Darkmatter docs: `inline/interpolation.md` 223-406, `shell-expansion.md` 63-69, `topics/darkmatter-expressions.md` 119/203, `topics/frontmatter-recursion.md` 18/26 (`{{ area }}`); `dmls/docs/autocomplete.md:111`, `hover.md:70`, `design/interpolation.md` 747/808/846 (`{{today}}`/`{{year}}`) | check each in Phase 7; any snippet whose frontmatter leaves the name undeclared documents the fallback and should say `ctx.<name>` |

Not affected: `branch` in `prompts/_pr/{diagnose,fix,triage}.md` and `area`
in `prompts/_implement/implement-plan.md` and `_reviews/*` have real
frontmatter defaults; `branch` in `pr.md`, `dirty.md`, `push.md`, `open.md`
and `agent`/`model` in `commit.md` are `$schema`-declared. The
`os.name`/`cwd`/`timestamp` hits in `claudine/lib/README.md`,
`unified-events.md`, and `configuring-actions.md` are hook-event payload
templates, not Darkmatter lookups.

### Test baseline

Run on 2026-10-01 (macOS host) at HEAD `13df7698f` plus this phase's
planning-document edits only; no source file changed.

| Area | Recipe | Result |
|---|---|---|
| `darkmatter/` | `just test` | 8744 passed, 12 skipped (4 slow), exit 0 |
| `darkmatter/` | `just lint` | clean, exit 0 |
| `claudine/` | `just test` | 8072 passed, 9 skipped (9 slow), exit 0 |
| `claudine/` | `just lint` | clean (including its 9-test lint suite), exit 0 |

**No pre-existing failures.** Any later failure in these four gates is caused
by this fix.

### Phase 1 outcome

Checkpoint 1 is met: rulings recorded, baseline known, migration checklist and
prompt-exposure list written above. No tests were added: Phase 1 changes no
behavior, so the requirement-to-test mapping is empty; the four baseline gates
are the verification. Source files changed: none. Planning/docs changed:
`claudine/docs/rollout-strategy.md` (NR-1), this fix's `design.md` status line
(NR-1), `plan.md` checkboxes and frontmatter, and this log.

## Phase 2

Phase 2 adds Darkmatter's binding model and the passive prepared-expression
validator. It is additive: strict mode, `is_known_variable_root`, and the
bare-name `ctx` fallback are all untouched, every crate compiles, and no
existing caller changes behavior. All Phase 2 code is in the `darkmatter`
library; Claudine needed no source change.

### What was built

| Plan task | Where | Notes |
|---|---|---|
| Binding types | `darkmatter/lib/src/markdown/compose/expression/binding.rs` (new) | `ResolvedBinding { Document, Namespace, Global }` carrying `Option<Value>` / `Value` (NR-2); `BindingError` (configuration arms `ReservedName`, `InvalidRoot`, `Duplicate`, `UnknownGlobal`, `OmittedDeclaredGlobal`, `ContradictsDeclaration`, `InvalidReasonCode`, plus `Unavailable(Box<UnavailableBinding>)` with `root`, `path`, `scope`, `reason`, `span`); `UnavailabilityReason` (namespaced `owner.reason` code + structured `parameters`) |
| Declarations and sessions | same file | `ScopeId` (opaque), `Availability { Available, Unavailable(reason), ExecutionDependent }`, immutable provider-free `BindingView` built by `BindingView::builder(scope).declare(..).build()`, `RootClass` + `BindingView::classify_root`, `RuntimeBinding { Eager, Lazy(Arc<dyn Fn() -> Value>), Unavailable }`, and `EvaluationSession::associate(view, document, runtime)` |
| Trait channel (NR-5) | `expression/mod.rs` | `get_checked` → `resolve(&self, path) -> Result<ResolvedBinding, ExpressionError>` (default wraps `get` as `Document`); new `binding_view()` (default `None`) and `format_resolved(path, value)` (default = the old `get_string` rendering); `get_string`'s default now composes `get` + `format_resolved`. `ExpressionError::Binding(Box<BindingError>)` added, authoring-fatal |
| Evaluator routing | `expression/mod.rs` (`evaluate_expr`), `interpolation/evaluator.rs` (`Evaluator::eval` fast path) | Both read through `resolve`; the fast path formats through `format_resolved` instead of a second `get_string` lookup |
| Prepared validator | `expression/prepared.rs` (new) | `AuthoredMode { Expression(ParseMode), InterpolatedValue, Subtree }`, `prepare_value`, `validate_prepared`, `evaluate_prepared`, `PreparedValue`/`PreparedExpression`, `PreparationError`, `ValidationDiagnostic`; reuses `ExpressionFinder::scan_plain`, `parse_spanned`/`parse_condition_spanned`, and `static_variable_reads` (the existing all-branches walk) |
| Binding/passive tests | `darkmatter/lib/tests/l1/binding_contract.rs` (new, declared in `tests/l1/main.rs`) | 16 tests, see the mapping below |

The six lookups that overrode `get_checked` now override `resolve`, each
classifying its result with `ResolvedBinding::classify` (reserved root →
`Namespace`, otherwise `Document`): `EffectiveState`, `ResolvingLookup`,
`FrontmatterSeedState`, `ShortcutLookup`, `CtxLookup`, `LayeredLookup`
(injected globals → `Global`), and the forwarder `DeferrableLookup` (still maps
`ContextNotCaptured` to an absent value when deferring, now as a classified
`None`). Each keeps exactly its old values and errors.

### Decisions and departures

- **`EvaluationSession::associate`, not `BindingEnvironment::associate`.**
  C1 and the plan name a `BindingEnvironment` type whose only member would be
  `associate`. An empty struct that hosts one function adds nothing in Rust, so
  the constructor lives on the type it returns. Same arguments, same checks.
- **No `PreparationContext` parameter.** NR-6 removed the schema, policy, and
  feature-restriction inputs C2 put in it, which leaves it with nothing to
  carry. `prepare_value(input, mode)` takes two arguments, and the parse mode
  of a bare expression lives in `AuthoredMode::Expression(ParseMode)` because a
  `when:` condition (`||` = OR) and an interpolation expression (`||` =
  fallback) parse differently.
- **No prepared identity check.** NR-6 makes the `BindingView` identity the
  only prepared identity. A `PreparedValue` stores no view-derived data (the
  view is an explicit argument to `validate_prepared`, and the session supplies
  it at evaluation), so there is nothing a stale view could corrupt and no
  `PreparedContextMismatch` to raise. Add it if C5 later caches view data.
- **Error sizes.** Clippy's `result_large_err` rejected the first shape, so
  `ExpressionError::Binding` holds a `Box<BindingError>`, the unavailable read
  is a boxed `UnavailableBinding` struct, and `PreparationError::cause` is a
  `Box<ExpressionError>`, matching `MarkdownError::Interpolation`'s boxed cause.
- **Unknown-function check.** `validate_prepared` decides "unknown function"
  with a new `functions::is_dispatchable`, which matches names exactly as
  `evaluate_function` does (lowercased, canonical name or alias, lazy
  operators included). It builds the diagnostic with the evaluator's own
  `unknown_function_error`, so validation and runtime raise the same typed
  error (now `pub(crate)`).
- **Reserved roots (NR-3)** come from `reserved_root_descriptors()` through
  `binding::is_reserved_namespace`: `doc`, `ctx`, `env`, `current`,
  `current_env`. If the owner chooses option B, that function is the one place
  to narrow.
- **Lazy cache.** One `Mutex<HashMap<root, Value>>` per session. The lock is
  dropped before a provider runs and re-taken to insert (first value wins if a
  re-entrant provider filled the slot). A poisoned lock is recovered with
  `PoisonError::into_inner`, never read as missing data. Members are projected
  from the cached root, so `err` and `err.message` share one provider call.
- **A global's missing member is `Global { null }`** in the session, which
  differs from `LayeredLookup` (where it was `None` and then `null` in the
  evaluator). The rendered output is the same; only the classification is new.
- **`LayeredLookup` keeps the default `format_resolved`.** It never applied the
  base state's name coercion when rendering (its old `get_string` override did
  not), so overriding the hook would have been a behavior change.
  `EffectiveState`, `ResolvingLookup`, `FrontmatterSeedState`, and
  `DeferrableLookup` override or forward it, preserving coercion. Coercion only
  ever touches objects (`coerce_named_object`), so routing arrays and scalars
  through the hook renders the same bytes as the old fast path.
- **Claudine forwarders were left alone.** `SizedLookup` is only used with the
  generic `evaluate` (never the interpolation fast path), so the new
  `format_resolved` hook cannot change its output. It and
  `EventMetaConditionLookup` gain `resolve`/`binding_view`/`format_resolved`
  forwarding in Phase 4 (Wave 9), as planned; forwarding `resolve` now would
  surface errors they drop today, which is a behavior change.

### Pre-existing defect fixed

The rustdoc example on `expression::evaluate` (`expression/mod.rs`, the second
`SimpleLookup` example the plan names) did not compile at HEAD: it imported
`evaluate` but called the private `evaluate_expr`. `just test` does not run
doctests, which is why Phase 1's baseline missed it. It now calls `evaluate`.
`just doctest` in `darkmatter/` passes (192 passed, 10 ignored).

### Requirement-to-test mapping

All tests are in `darkmatter/lib/tests/l1/binding_contract.rs`, compiled by the
declared `l1` binary (`tests/l1/main.rs`) and selected by L1 (no tier marker in
any path segment). They read no repository file.

| Requirement (plan Wave 5) | Test |
|---|---|
| available-null vs unavailable vs absent document property | `resolution::available_null_unavailable_and_absent_property_are_distinct` (whole value, mixed string, ternary, `\|\|` fallback does not hide an unavailable global, `doc.group` still reads the document) |
| an injected global shadows a same-named document property | `resolution::a_global_shadows_a_document_property_of_the_same_name` (missing member stays `Global { null }`) |
| each reserved root's registration fails before any lazy provider runs | `registration::every_reserved_root_is_rejected_before_any_provider_runs` (declaration and association, with a panicking provider; asserts the five-name table) |
| an omitted declared global fails association even when only referenced in an inactive branch | `registration::an_omitted_declared_global_fails_association` (each of the three globals omitted in turn; control row evaluates) |
| configuration errors (duplicate, unknown, dotted, contradicting, reason codes) | `registration::association_rejects_every_inconsistent_registration`, `registration::a_view_rejects_invalid_duplicate_and_unnamespaced_declarations` |
| `doc.doc`, `doc.ctx`, `doc.env` read document data | `resolution::doc_prefixed_reserved_names_read_document_data` |
| one lazy evaluation per session, including a `null` result | `resolution::a_lazy_global_runs_once_per_session_including_a_null_result` (0 calls before reference, 1 across four reads, 2 after a new session) |
| a get-only lookup stays source-compatible | `resolution::a_get_only_lookup_resolves_as_document_data` |
| an unavailable root in an inactive ternary branch fails `validate_prepared` | `passive_validation::an_unavailable_root_in_an_inactive_branch_fails_validation` (runtime takes the other branch and succeeds) |
| an `execution-dependent` root is deferred | `passive_validation::validation_walks_every_position_and_defers_execution_dependent_roots` |
| a counting provider records zero calls during preparation and validation | `passive_validation::preparation_and_validation_invoke_no_provider` |
| unknown functions in every branch; aliases and letter case accepted | `passive_validation::unknown_functions_are_reported_in_every_branch` |
| mode semantics (condition `\|\|`, typed whole value, subtree) | `passive_validation::each_authored_mode_evaluates_with_its_own_rules` |
| malformed input located; literals not prepared | `passive_validation::a_malformed_expression_fails_preparation_with_its_location`, `passive_validation::literals_are_not_prepared` |

Unchanged behavior for existing callers is shown by the existing suites, which
pass unmodified apart from the mechanical `get_checked` → `resolve` call-site
edits in `context/checked.rs` and `expression/ctx.rs` unit tests.

Input Robustness Matrix: not applicable (no file-format or configuration
reader changed; plan § Input Robustness Matrix).

### Gates

| Area | Recipe | Result |
|---|---|---|
| workspace | `cargo check --workspace --all-targets` | clean |
| `darkmatter/` | `just test` | 8760 passed (baseline 8744 + 16 new), 12 skipped |
| `darkmatter/` | `just lint` | exit 0 |
| `darkmatter/` | `just doctest` | 192 passed, 10 ignored (includes the repaired `evaluate` example and the new `prepared` module example) |
| `claudine/` | `just test` | 8072 passed, 9 skipped (identical to baseline) |
| `claudine/` | `just lint` | exit 0 |

No `just test-l2` was run: Phase 2 changes no L2-covered boundary, and the
checkpoint names only `just test`/`just lint`. Cross-OS: the change is pure Rust
with no `#[cfg]`, path, or process code, so no `just cross-check` was needed;
CI covers the other environments.

### Docs

`darkmatter/docs/topics/darkmatter-expressions.md` § `EvaluationLookup` Trait
named the removed `get_checked`; it now describes `resolve`/`format_resolved`,
and a new "Host Bindings" subsection documents the resolution order, the three
availability states, association checks, lazy caching, and
prepare/validate/evaluate with a compact example. Phase 7 still owns the full
documentation pass (the Mermaid resolution diagram and the remaining pages).

### Phase 2 outcome

Checkpoint 2 is met: `cargo check --workspace` is clean, `darkmatter` and
`claudine` `just test`/`just lint` pass, and no existing caller changed
behavior. Evidence toward acceptance criteria 6 and 8 (partial): an unavailable
global is a typed error that never reads the document (6), and the passive
validator reports definitely unavailable roots in every branch without running
a provider (8).

## Phase 3

Phase 3 removes strict mode and the bare-name `ctx` fallback from Darkmatter,
rebuilds subtree compose on the Phase 2 binding session, renames the
undeclared-property advisory (NR-7), and makes Claudine compile against the
result. Claudine's own walkers (`first_undefined_stack_variable`,
`validate_no_undefined_lifecycle_variables`, the `err` placement scan) are
untouched; Phase 4 owns them.

### What was built

| Plan task | Where | Notes |
|---|---|---|
| Subtree API removal | `darkmatter/lib/src/markdown/compose/subtree.rs` | `SubtreeStrictness`, `.strict()`, `with_strictness`, the `compose_subtree` strictness argument, `validate_strict_roots`, and the subtree-local `collect_variable_roots` are deleted. Subtree compose always runs `ExpressionFailurePolicy::Strict` (ordinary fail-fast propagation). `InjectedGlobal` is now `pub type InjectedGlobal = RuntimeBinding<'static>`, so a caller can also pass `InjectedGlobal::unavailable(reason)`. `SubtreeCompose` gains `with_binding_view(Arc<BindingView>)` |
| `LayeredLookup` rebuilt on the session | same file, `expression/binding.rs` | `LayeredLookup` is deleted; `subtree::layered_session(base, globals, view, resolution_context) -> Result<EvaluationSession, BindingError>` replaces it. Without a view it synthesizes one (scope `darkmatter.subtree`) declaring each supplied global as supplied (available, or unavailable with its reason). `EvaluationSession::with_resolution_context` was added so the session can carry the read-side context that `LayeredLookup` used to hold. An association failure surfaces from `SubtreeCompose::compose` as `MarkdownError::Interpolation { cause: ExpressionError::Binding(..) }`, keyed by the new `BindingError::root()` |
| Namespace fallback removal | `context/effective_state.rs`, `frontmatter_interpolation.rs`, `conditions.rs`, `context/checked.rs`, `context/runtime.rs` | `EffectiveState::get`/`get_checked` and `ShortcutLookup` no longer read `ctx.<name>` for a missing bare name; `CtxLookupOutcome::into_checked_bare_name` is deleted. Exact `ctx` and `env` roots are resolved as namespaces before any document key (`ComposeContext::env_value` added for bare `env`), so a frontmatter `env` key is reachable only as `doc.env` |
| Trait + remaining lookups | `expression/mod.rs` and every override | `EvaluationLookup::is_known_variable_root` is removed from the trait and from `EffectiveState`, `ResolvingLookup`, `FrontmatterSeedState`, `DeferrableLookup`, `EvaluationSession`, and the test-only `Nothing` |
| Lookup parity inventory (R3) | `darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs` (new) | See the test mapping below |
| Undeclared-property advisory (NR-7) | `unknown_identifiers.rs`, `expression/mod.rs` (`observe_missing`), `interpolation/evaluator.rs`, `context/report.rs`, `expression/absence.rs` | The observer now takes the `ResolvedBinding`: only `Document { value: None }` is a candidate (never a namespace or global; reserved roots and `null` skipped). `reconcile` filters by final-state keys (`state.data()`), caller input records, and the effective schema. `ComposeWarning::UNDECLARED_PROPERTY_CODE = "dm.expression.undeclared_property"` and `ComposeWarning::undeclared_property` replace the old constant/constructor, with no alias. Message: `` `{root}`{location} is an undeclared document property (unknown type; `null` unless supplied at runtime) ``. `is_statically_known_root` is now exactly the reserved roots (context-descriptor names dropped) |
| Claudine compile bridge | `lifecycle/executor.rs`, `preflight.rs`, `sequence/preflight/mod.rs`, `sequence/task/mod.rs`, `lifecycle/context/tests.rs`, `interpolation_conformance.rs`, `event_time_interpolation.rs`, `cli/.../loop_control/tests/mod.rs` | `.strict()` deleted at the five sites; `eval_expr` and the two tests use `layered_session`. `strict_mode_provenance_spike.rs` deleted and unregistered from `tests/l1/main.rs` (NR-10). `signals/version.rs`, `darkmatter/cli/src/commands/compose.rs`, and `dmls/src/diagnostics/frontmatter.rs` needed no change |

### Decisions and departures

- **`LayeredLookup` is deleted, not wrapped.** A struct owning both a
  `ResolvingLookup` and a session that borrows it is self-referential, and a
  forwarding wrapper is the partial-forwarding hazard the inventory exists to
  prevent. A constructor function that returns the `EvaluationSession` keeps one
  lookup type. Callers change from `LayeredLookup::new(&s, &g, ctx)` to
  `layered_session(&s, g, None, ctx)?`.
- **A reserved-root global now fails association instead of being silently
  ignored.** `LayeredLookup` quietly skipped a global named `current`; the
  session rejects it (`BindingError::ReservedName`) before any provider runs.
  `compose/tests/lazy_roots.rs::an_injected_global_cannot_shadow_a_reserved_root`
  now asserts the typed refusal, with no provider call. No production caller
  registers a reserved name.
- **Subtree formatting now matches main compose.** The session forwards
  `format_resolved` to the document lookup, so `EffectiveState`'s configured
  name coercion now applies in subtree compose. `LayeredLookup` used the trait
  default. This brings subtree compose to its documented "byte-for-byte with
  main compose" contract. No test depended on the old rendering.
- **`group` outside a group is already an unavailable global (bridge toward
  Phase 4).** Without strict mode, a later sequence step's `{{ group.label }}`
  stopped failing (`group_variables_do_not_leak_to_a_later_step` and the CLI
  `sequence_groups::group_variables_reach_members_and_do_not_leak_to_the_next_step`
  turned red): sequence members carry `group` as *document data*, so outside a
  group it was an absent property. Rather than weaken two safety tests,
  `lifecycle::context::outside_group_global()` registers `group` as
  `InjectedGlobal::unavailable("claudine.outside-group")` (the code Wave 8
  names) in `TaskExecution::resolve_value` when the task's overlay carries no
  `group` scope, and in the lifecycle executor's `injected_globals` when
  `self.group` is `None`. This is NR-11's ruled behavior ("bare `group` outside
  a group raises a typed unavailable error; `doc.group` still reads the
  document"). Phase 4's catalog should replace the helper and both call sites.
- **Shipped-prompt fixes moved forward from Phase 6 (group D of the Phase 1
  audit).** Removing the fallback made `prompts/_reviews/performance-review.md`
  (`{{time}}`) and `prompts/brainstorm.md` (`{{area}}`) render those names
  empty, and DMLS's mapping-only corpus test reported the new advisory on the
  fixture copy. Both prompts and both fixture copies now say `ctx.time` /
  `ctx.area`. Neither prompt carries a `hash:` pin. The remaining group D rows
  are documentation (Phase 7) and `claudine/cli/tests/fixtures/nested_span_regression/commit.md:31`
  (`{{repo.name}}`), whose expected output was already empty and whose tests pass.
- **Minimal DMLS edits.** DMLS calls the library's `is_statically_known_root`,
  so dropping context names changed one DMLS unit test and one LSP session
  expectation: bare `repo` is now reported. DMLS still emits its own
  `dm.expression.unknown_identifier` code and wording; Phase 5 renames it. So
  between Phase 3 and Phase 5 the library and DMLS spell the code differently.
- **Old-contract Claudine tests updated, not left red.** Checkpoint 3 allowed
  old-contract failures to be listed for Phase 4, but this phase's completion
  bar is a green `just test`. The four tests that asserted "an unknown root
  fails" now assert the ruled contract (R1: absent is `null`):
  `event_time_interpolation::top_level_absent_property_renders_empty` (emits
  `""`), `::an_authored_span_with_an_absent_property_renders_empty` (emits
  `"done: "`), `cli agent_text_is_data::an_absent_property_in_an_authored_lifecycle_span_is_not_an_error`,
  and `cli authored_text_rendering::diagnostic_quotes_an_underscored_root_exactly`,
  which keeps its subject (an underscored root quoted verbatim) by sourcing it
  from the body advisory, since a lifecycle field no longer produces a root
  diagnostic. `interpolation_conformance.rs` only lost its strictness parameter.
  Its former "unknown-root divergence" is now `an_absent_root_is_empty_in_both_engines`,
  and the full matrix rewrite remains Phase 4.
- **Doc drift fixed in this phase.** Pages that described the removed fallback,
  `SubtreeStrictness`, or the old code were corrected (list in the frontmatter).
  Claudine's lifecycle docs now describe the interim state:
  `LifecycleUndefinedVariable` still guards `stack` entries, and its removal is
  marked **planned**. DMLS's docs (`darkmatter/docs/lsp/features.md`,
  `topics/schemas/dmls-schema-support.md`, `dmls/docs/*`) still describe
  DMLS's unchanged code and are Phase 5's.

### Requirement-to-test mapping

| Requirement | Test(s) | Level / target |
|---|---|---|
| Strict mode removed; subtree compose fails on real failures only (malformed, unknown function, rejected argument, failed file read) in whole values and mixed strings; absent → `null`/empty | `lib/tests/l1/compose_expression_failure_contract.rs::subtree_compose_fails_on_real_expression_failures_only`; `subtree.rs` unit tests `an_absent_property_is_null_whole_value_and_empty_in_a_mixed_string`, `malformed_spans_and_unknown_functions_fail_in_any_position`; `tests/frontmatter.rs::dm2_subtree_absent_property_is_null_not_an_error`, `dm2_subtree_rejects_malformed_span`, `dm2_subtree_rejects_unknown_function`, `dm2_subtree_absent_function_argument_is_null` | Darkmatter L1 (`l1` binary) + lib unit |
| Unavailable global is a typed error and never reads the document; `doc.group` still does | `subtree.rs::an_unavailable_global_fails_and_never_reads_the_document` | lib unit |
| Reserved roots cannot be registered; no provider runs (AC 2, widened by NR-3) | `subtree.rs::a_reserved_root_cannot_be_registered_and_no_provider_runs` (all five roots, panicking lazy provider); `tests/lazy_roots.rs::an_injected_global_cannot_shadow_a_reserved_root` | lib unit |
| A declared view is enforced at association | `subtree.rs::a_declared_view_is_enforced_at_association` | lib unit |
| No bare-name `ctx` fallback in any lookup (R3, AC 4) | `compose/tests/lookup_parity.rs::the_inventory_lists_every_lookup_implementation` (source scan of `CARGO_MANIFEST_DIR/src`, 13 implementations), `::every_production_lookup_keeps_bare_names_out_of_ctx` (7 production lookups probed), `::the_probe_name_is_a_captured_context_key` (control row). Mutation-checked: restoring the fallback in `EffectiveState` turns the probe red | lib unit |
| Exact `ctx`/`env` roots never read a document key; `doc.env` does | `lookup_parity` probe; `conditions.rs::shortcut_bare_name_never_reads_ctx`; `absent_property_contract::an_exact_namespace_root_is_never_a_document_property` | lib unit + L1 |
| Missing bare property does not resolve from `ctx` (body, frontmatter, `when=`) | `lib/tests/l1/absent_property_contract.rs::a_missing_bare_property_does_not_resolve_from_ctx`; `context/checked.rs::a_bare_name_never_reads_the_ctx_namespace`; `subtree.rs::a_bare_name_never_reads_ctx` | L1 + lib unit |
| Absent bare and `doc.<p>` are `null` whole values; empty in mixed strings; ternary falsy branch; `\|\|` value semantics (AC 1, 3) | `absent_property_contract::an_absent_property_is_a_null_whole_value`, `::an_absent_property_is_empty_in_a_mixed_string`, `::an_absent_property_takes_the_falsy_branch_and_the_fallback` | L1 |
| Schema-declared-unset and undeclared both valid; required violation still blocks | `absent_property_contract::a_schema_declared_unset_property_and_an_undeclared_one_are_both_valid`, `::a_required_property_violation_still_blocks`; `cli/tests/l1/compose_schema.rs::declared_unset_and_undeclared_properties_are_both_null_in_subtree_compose` | L1 (lib + `md` CLI) |
| Malformed syntax, unknown function, rejected arguments, file failure still raise | `absent_property_contract::expression_failures_still_stop_composition` (body + frontmatter mixed text); the existing body/frontmatter matrix in `compose_expression_failure_contract.rs` | L1 |
| All three escape forms inert; whole-value and mixed; no extra evaluation pass | `absent_property_contract::every_escape_form_is_inert` (asserts no advisory, i.e. nothing evaluated) | L1 |
| NR-7 code, wording, `ctx`-name drop, suppression kept | `unknown_identifier_warning.rs` (all rows updated; new `a_bare_context_name_is_an_undeclared_property_not_ctx`), `unknown_identifiers.rs` message unit tests, `absence.rs::statically_known_roots_are_exactly_the_reserved_roots`, `feature_review_incident.rs`, `cli/tests/l1/compose_unknown_identifiers.rs` (end-to-end `md compose`) | L1 + lib unit + `md` CLI L1 |
| Group variables do not leak (bridge) | existing `sequence/task/tests.rs::group_variables_do_not_leak_to_a_later_step`, `cli/tests/l1/sequence_groups.rs::group_variables_reach_members_and_do_not_leak_to_the_next_step` (unchanged, green again) | Claudine lib unit + CLI L1 |
| Shipped prompt corpus | `dmls/tests/l1/mapping_only_corpus.rs` (passive corpus over the fixture copies; baseline unchanged after the prompt fix) | DMLS L1 |

Tier and placement: `absent_property_contract.rs` is declared in
`darkmatter/lib/tests/l1/main.rs`, and `lookup_parity.rs` in
`compose/tests/mod.rs`. One new name, `real_expression_failures_still_stop_composition`,
was first stranded by its `real_` prefix (seen as a 13th skip) and renamed to
`expression_failures_still_stop_composition`; the final run shows the baseline
12 skips. The source scan joins a literal onto `CARGO_MANIFEST_DIR`, as the
test-inputs rule asks. Input Robustness Matrix: not applicable (no file-format
or configuration reader changed).

### Gates

| Area | Recipe | Result |
|---|---|---|
| workspace | `cargo check --workspace --all-targets` | clean |
| `darkmatter/` | `just test` | 8768 passed, 12 skipped (baseline skips) |
| `darkmatter/` | `just lint` | exit 0 |
| `darkmatter/` | `just doctest` | 192 passed, 10 ignored |
| `claudine/` | `just test` | 8068 passed, 9 skipped (baseline 8072 minus the 4 deleted spike tests) |
| `claudine/` | `just lint` | exit 0 |
| `claudine/` | `just test-l2` | 277 + 3 passed |
| `darkmatter/` | `just test-l2` | 3 + 18 + 69 passed |

Pre-existing or unrelated: none failed. Two specs outside this fix
(`claudine/fixes/2026-07-13-cli-switches/spec.md`,
`claudine/fixes/2026-07-22-setters/spec.md`) were rewritten by another session
during this phase; they are not part of this change. One `just test` run went
to the main checkout by mistake (`cd claudine` from inside `claudine/`
resolved through zsh `CDPATH`). It changed nothing; every gate above was run
with an absolute worktree path. Cross-OS: the change is pure Rust plus
Markdown, with no `#[cfg]`, process, or path-comparison code. The new source
scan normalizes `\` to `/`, so no `just cross-check` was run; CI covers the
other environments.

### Phase 3 outcome

Checkpoint 3 is met, and exceeded: `cargo check --workspace` is clean,
`darkmatter` `just test`/`just lint` pass, and Claudine's `just test` is fully
green rather than limited to listed old-contract failures. Acceptance criteria
1 (absent → `null`), 3 (no extra evaluation of escapes/inserted data), and 4
(no bare-name `ctx` fallback, with the parity inventory) hold in Darkmatter. For
criterion 2, the reserved-root refusal is now enforced in subtree compose too.
