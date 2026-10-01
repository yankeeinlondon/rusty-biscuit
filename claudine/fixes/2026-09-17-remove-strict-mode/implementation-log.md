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
packages: []
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
