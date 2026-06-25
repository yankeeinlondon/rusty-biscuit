---
agent: open_code/zai-coding-plan/glm-5.2
phases: 7
created: 2026-06-24
start_phase: 1
yolo: "true"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/lifecycle.rs
    - claudine/lib/src/composition/error.rs
    - claudine/lib/src/composition/loop_engine.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/lib/src/composition/lifecycle_context.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_touched_during_phase_4:
    - claudine
source_files_during_phase_5:
    - claudine/lib/src/composition/lifecycle.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_touched_during_phase_5:
    - claudine
source_files_during_phase_6:
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/select.rs
    - claudine/lib/src/composition/lifecycle.rs
    - claudine/cli/src/commands/wrap/composition/dry_run.rs
docs_updated_during_phase_6:
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/features/2026-05-12-lifecycle/spec.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
packages_touched_during_phase_6:
    - claudine
source_files_during_phase_7:
    - claudine/lib/src/composition/schema_validation.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_touched_during_phase_7:
    - claudine
source_code:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/tests.rs
    - darkmatter/lib/src/markdown/compose/subtree.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/lifecycle.rs
    - claudine/lib/src/composition/error.rs
    - claudine/lib/src/composition/loop_engine.rs
    - claudine/lib/src/composition/lifecycle_context.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/select.rs
    - claudine/lib/src/composition/schema_validation.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/composition/dry_run.rs
documentation:
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/features/2026-05-12-lifecycle/spec.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
packages:
    - darkmatter
    - claudine
---

# Execution Plan — Event-Time Interpolation for Lifecycle Properties (Late Binding)

Source spec: [`spec.md`](spec.md). All tasks below trace to a spec section
(DM1, DM1a, DM1b, DM2, C1–C7) and to the named files in the spec's
*Implementation Notes*. File paths are absolute-relative to the repo root.

## Dependency graph at a glance

```
Phase 1 (DM1/DM1a/DM1b) ──┬──> Phase 3 (C1 raw subtree + C3 shell) ──┬──> Phase 5 (C4 guards) ──┐
                          │                                          │                          ├─> Phase 7 (integration)
Phase 2 (DM2) ────────────┴──> Phase 4 (C2 event-time + retire) ─────┘                          │
                                                                          Phase 6 (C5/C7 docs) ────┘
```

- **Phase 2 is parallelizable with Phase 1** once DM1's exclude-set plumbing
  lands (DM2 reuses the same interpolation core but is otherwise independent).
- **Phase 6 docs can be drafted in parallel** with Phases 3–5 (the author-facing
  behavior is fixed by the spec); the final accuracy review must follow
  implementation.

---

## Phase 1 — Darkmatter: exclude lifecycle keys from main compose (DM1, DM1a, DM1b)

Goal: give callers an opt-in way to defer named top-level frontmatter keys
from every compose-time resolution pass, and forbid composed keys from
reading deferred subtrees. Default empty → no behavior change for any
existing caller.

- [x] **1.1 (DM1) Add the exclude-keys option to `ComposeOptions`.** New
  `pub(crate) exclude_keys: HashSet<String>` field + `with_exclude_keys(...)`
  builder in `darkmatter/lib/src/markdown/compose/context/options.rs`. Default
  empty. Add to the `Debug` impl. No behavior change when empty.
- [x] **1.2 (DM1) Honor excluded keys across all four resolution passes.** In
  `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
  (`interpolate_frontmatter_impl`), skip any key in the exclude set during
  seed/templated classification so its `{{ }}` / structure survives intact.
  Extend the skip to whole-value expansion, `$(...)` shell expansion
  (`frontmatter_shell_expansion.rs`), and schema value interpolation
  (`schema_validation.rs`) — each pass must treat an excluded key as
  passthrough. Preserve the excluded value's JSON type/shape (only resolution
  is skipped, not parsing). Verify both interpolation call sites in
  `compose/pipeline/mod.rs:156` and `:250` and the best-effort call in
  `compose/preflight/collect.rs:525` honor the set.
- [x] **1.3 (DM1) Surface deferred-key metadata in `ComposeReport`.** Extend
  the report (or a typed field on it) with the set of intentionally-deferred
  keys so callers can distinguish "raw because deferred" from "raw because
  composition failed". Claudine consumes this for C5 dry-run labeling and
  diagnostics.
- [x] **1.4 (DM1a) Reject composed keys that read deferred keys.** Add a
  dependency-analysis pass (in `frontmatter_interpolation.rs` or a sibling)
  that walks each *non-deferred* templated key's references and fails with a
  typed `MarkdownError` naming both the composed key and the deferred key when
  a reference is `{{ failure }}`, `{{ failure.message }}`, or
  `{{ doc.failure.message }}`. Decision: reject, not warn (a warning still
  produces a usable value with raw lifecycle syntax — the bug class this fix
  removes).
- [x] **1.5 (DM1b) Exclude deferred keys from user `$schema` value validation.**
  In `schema_validation.rs`, ordinary non-deferred frontmatter validates
  exactly as today; deferred lifecycle keys are skipped from user schema value
  validation (Claudine owns their subtree validation). Keep the existing
  `defer_shell_pending_schema_problems` behavior orthogonal.
- [x] **1.6 (Validation checkpoint — Darkmatter DM1/DM1a/DM1b tests).** Add
  unit tests in the darkmatter compose test modules covering each spec bullet:
  excluded key left raw across `{{ }}`/whole-value/`$()`/schema-interpolation;
  non-excluded key unaffected; excluded value types preserved; composed-key
  references deferred key (bare root and `doc.<key>`) fails naming both keys;
  ordinary schema validation still runs while a deferred key containing
  `{{err.msg}}` is not rejected by user schema. Run `just test` in
  `darkmatter/`.

---

## Phase 2 — Darkmatter: subtree compose with an injectable lookup (DM2)

Goal: expose a public entry point that interpolates a frontmatter *subtree*
(JSON value) through the same interpolation core as main compose, driven by a
caller-supplied layered lookup. This is the primitive Claudine uses for both
event-time resolution (C2) and early-binding shell resolution (C3).

- [x] **2.1 Define the injected-globals lookup contract.** A lookup type that
  layers caller-injected named globals (eager values + lazy closures) over
  Darkmatter's own seed state (`ctx`/`env`/`doc` + read-side functions via
  `ResolutionContext`). The API takes arbitrary named globals — not three
  hardcoded names — so future late-binding globals need no further Darkmatter
  change. Lazy globals evaluate on first access exactly as `ctx` is at compose
  time.
- [x] **2.2 Expose the public subtree-compose entry.** In
  `darkmatter/lib/src/markdown/compose/` (likely a new `subtree.rs` or a public
  fn on the compose facade) reusing `Evaluator`/`interpolate_value` so
  whole-value typing and mixed-string *resolution* rules match main compose
  byte-for-byte. The subtree compose applies identical resolution semantics;
  strictness is an **orthogonal** mode flag.
- [x] **2.3 Add the strictness mode flag.** Strict mode returns typed errors
  for malformed spans, unknown functions, and unknown roots instead of a
  degraded/lenient string. Lenient behavior stays available for body and
  mixed-string use cases. Claudine uses strict mode for lifecycle
  communication/action text.
- [x] **2.4 Wire laziness verification.** Confirm (with a test) that a lazy
  global's closure is invoked at most once and only when its name is
  referenced — never eagerly at subtree-compose entry.
- [x] **2.5 (Validation checkpoint — Darkmatter DM2 tests).** Add unit tests
  matching the spec's DM2 bullets: subtree resolves injected eager (`err`) and
  lazy (`current`) globals; layered seed state (`ctx`/`env`/`doc`/functions)
  still resolves; whole-value typing and mixed-string behavior match main
  compose for the same string + same data; lazy global evaluated only when
  referenced; strict mode returns typed errors for malformed spans, unknown
  functions, unknown roots. Run `just test` in `darkmatter/`.

> **Parallelizable:** 2.1–2.5 can proceed in parallel with 1.4–1.6 once 1.1
> lands (DM2 reuses the interpolation core but does not depend on DM1a/DM1b).

---

## Phase 3 — Claudine: parse lifecycle from the raw subtree + early-binding shell pre-flight (C1, C3)

Goal: with DM1 deferring the seven lifecycle keys, `parse_lifecycle_config`
reads them raw; shell commands inside that subtree are resolved at pre-flight
through DM2 with an early-binding-only lookup and stamped back so approval
equals execution.

- [x] **3.1 (Edge-case spike) Decide the `loop:` deferral strategy.** Before
  implementing C1, verify whether `loop:` iteration controls (`while`/`until`/
  `actions`/`max`/`fail_fast`) depend on compose-time `{{ }}` interpolation.
  If they do not (expected — they are evaluated by the loop engine), defer the
  whole `loop:` key (option (a)). If any control depends on compose-time
  interpolation, extend DM1 to accept sub-paths (`loop.say`, `loop.stack`, …)
  — option (b). Record the decision in the task notes.
  - **Decision: option (a) — defer the whole `loop:` key.** Verified:
    `resolve_loop_config` (`loop_config.rs`) parses `while`/`until` as
    whole-expression *condition* strings (`LoopCondition::While|Until(String)`)
    evaluated by the loop engine against live runtime state, and `action(s)`
    are parsed as action expressions evaluated per iteration — none rely on
    compose-time `{{ }}` interpolation. Deferring `loop:` is therefore a no-op
    for the iteration controls and correctly defers the loop's lifecycle
    concern keys (`say`/`message`/`stack`/…) to event-time. `loop` is included
    in `LIFECYCLE_EVENT_KEYS`.
- [x] **3.2 (C1) Pass the seven lifecycle keys to the exclude set.** In
  `claudine/lib/src/composition/prepare.rs`, both `prepare_direct` and
  `prepare_inline` build `ComposeOptions` — add the lifecycle event keys
  (`initialize`/`start`/`success`/`blocked`/`failure`/`finalize`/`loop`, or the
  `loop:` sub-path set per 3.1) via `with_exclude_keys`. Non-lifecycle keys
  compose as today, so variable *values* (`phase`, `pass_icon`, …) are
  composed before launch and may still be mutated by lifecycle/loop side
  effects during the run.
  - Done via `with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied())` in both
    `prepare_direct` and `prepare_inline`.
- [x] **3.3 (C1) Confirm `parse_lifecycle_config` reads the raw subtree.**
  `parse_lifecycle_config` already reads from `effective_frontmatter`
  (`prepare.rs:195`, `:322`) — with 3.2 in place the lifecycle strings keep
  their `{{ }}` spans. Verify the existing leak/undefined/err guards still
  parse against raw spans (they already read raw frontmatter separately where
  needed). Adjust any call site that assumed resolved lifecycle strings.
  - **Adjustment made:** the prepare-time `validate_no_interpolation_leaks` and
    `validate_no_undefined_lifecycle_variables` calls *assumed resolved*
    lifecycle strings — with deferral they would flag every authored
    `{{ }}` span as a bug. Both calls were removed from `prepare_direct` /
    `prepare_inline`; their responsibilities move to event-time strict DM2
    resolution (Phase 4 / C2) and the post-DM2 dispatch-time leak guard
    (Phase 5 / C4). `validate_no_err_in_no_error_events` is **kept**: a bare
    `err` in a no-error event is invalid regardless of binding time, and it
    operates on parsed expression surfaces, not deferred literal spans. The
    seven existing prepare tests that asserted compose-time resolution / a
    prepare-time leak were updated to assert the deferred (raw-span) behavior.
- [x] **3.4 (C3) Pre-flight shell resolution via DM2 (early-binding only).** In
  `claudine/lib/src/composition/preflight.rs`, for each shell command
  discovered by `collect_lifecycle_shell_commands` (both short-form
  `shell(...)` and long-form `shell` actions with `command:`), resolve via DM2
  with a lookup containing **only** early-binding surfaces (`doc.*`, `ctx.*`,
  `env.*`, read-side functions) — no `err`/`timing`/`current`. Reject any
  late-binding reference in a shell command with a typed `CompositionError`
  (new variant or reuse an existing one with a clear message naming the
  property path). Stamp the resolved bytes back so the approved command equals
  the executed command.
  - Implemented `resolve_lifecycle_shell_commands` in `preflight.rs`: builds an
    early-binding `EffectiveState` (frontmatter + the composed `ComposeContext`
    for `ctx.*`/`env.*`) plus a `ResolutionContext` rooted at the prompt's
    parent dir for read-side functions, then runs each shell `command`/`on_error`
    string-literal span through DM2 `SubtreeCompose::strict()` (no injected
    globals) and stamps the resolved literal back. Late-binding references are
    pre-scanned (`LATE_BINDING_ROOTS`) and rejected with the new typed
    `CompositionError::LifecycleShellResolution { source_path, property, raw,
    message }`, naming the dotted property path. Non-literal command exprs and
    literals without a `{{ }}` span are left untouched. The signature takes the
    composed `&ComposeContext` so the lookup sees the same `ctx`/`env` main
    compose saw.
- [x] **3.5 (Validation checkpoint — C1/C3 tests).** Add claudine tests:
  effective_frontmatter retains raw `{{err.msg}}` after `prepare_direct`;
  `parse_lifecycle_config` sees the spans; `shell(git fetch {{branch}})`
  resolves at pre-flight and approved == executed; `shell(rm {{err.msg}})` and
  long-form `command: "rm {{err.msg}}"` are rejected at prepare time with the
  property path. Run `just test` in `claudine/`.
  - Added `lifecycle_err_span_survives_raw_in_effective_frontmatter`,
    `shell_command_early_binding_resolves_at_preflight`,
    `shell_command_late_binding_reference_rejected_at_prepare`, and
    `shell_long_form_command_late_binding_rejected_at_prepare` in `prepare.rs`.
    `just test` (claudine + claudine-contract + claudine-cli) and `just lint`
    pass.

> Depends on: Phase 1 (DM1) and Phase 2 (DM2).

---

## Phase 4 — Claudine: event-time interpolation via DM2 + retire the bespoke interpolator (C2)

Goal: when an event fires, interpolate each property/action string through
DM2 just-in-time against the live effective document state plus the
late-binding globals; remove claudine's bespoke `interpolate`/`LifecycleLookup`
runtime path.

- [x] **4.1 (C2) Add a Claudine-side injected-globals builder.** In
  `claudine/lib/src/composition/lifecycle_context.rs`, reduce `LifecycleLookup`
  to the injected-globals layer handed to DM2: eager `err` and `timing`, lazy
  `current` (evaluated on first access exactly as `ctx` is at compose time).
  Keep the `LifecycleErrorInfo`/`LifecycleTiming`/`LifecycleCurrent` data
  types; what changes is how they reach the evaluator (injected into DM2's
  layered lookup over the current document state, not a bespoke
  `EvaluationLookup` impl).
  - Removed the `LifecycleLookup` `EvaluationLookup` struct + `walk_json`.
    Added `lifecycle_injected_globals(err, timing, current) ->
    HashMap<String, InjectedGlobal>`: `err`/`timing` eager (`to_value`),
    `current` lazy (the cloned snapshot materializes its JSON only when a
    lifecycle string references `current`). `mod.rs` now re-exports
    `lifecycle_injected_globals` instead of `LifecycleLookup`.
- [x] **4.2 (C2) Route every lifecycle property/action through DM2 at
  event-time.** In `claudine/lib/src/composition/lifecycle_executor.rs`,
  replace the body of `render_message`, `run_shell_action`,
  `dispatch_side_effect`, `resolve_control`, `invoke_expression_function`, and
  the top-level emission path so each string is resolved via DM2 immediately
  before it is used — against the *current* effective document state at that
  instant (not a snapshot taken when the event fired). This is required so a
  `set_frontmatter` side-effect run by stack action #1 is visible to action #2
  in the same event's stack, and so `current`/`timing` read the state at the
  point of use.
  - `StackExecutionContext::lookup()` is replaced by `build_state` (DM2
    `EffectiveState` over the current document `fm` with demand-driven `ctx.*`
    capture via `ComposeContext::capture_for_content` — sniff-free unless the
    string references `ctx.*`), `injected_globals`, `eval_expr`
    (`LayeredLookup` + `evaluate` for expression surfaces: `when`/control
    args/side-effect args/expression-functions), and `resolve_string_value`
    (`SubtreeCompose` for `{{ … }}` interpolation, preserving whole-value
    typing). `emit_top_level` now interpolates the deferred top-level fields at
    event-time (the core bug fix). The stack threads an evolving in-memory
    `working` frontmatter; a frontmatter-verb side effect that targets the
    document mirrors its mutation onto `working` (`mirror_frontmatter_mutation`
    + `targets_document`/`lexical_normalize`), so action #2 sees action #1's
    `set_frontmatter`. Side-effect string args now interpolate `{{ … }}` too.
    Lenient mode is used (matching the old interpolator's leniency); strict
    fail-closed enforcement is Phase 5's scope.
- [x] **4.3 (C2) Delete the bespoke interpolator.** Remove the module-level
  `interpolate` fn at `lifecycle_executor.rs:668` and any remaining
  `LifecycleLookup`-as-`EvaluationLookup` usage that bypasses DM2. Confirm no
  other call site imports the removed symbol.
  - `interpolate` deleted; `LifecycleLookup` deleted. The one external user
    (the `attached_globals_resolve_through_lookup` CLI test) was rewritten to
    use `lifecycle_injected_globals` + `LayeredLookup`. `ExpressionFinder`/
    `parse` imports dropped from the executor.
- [x] **4.4 (C2) Keep the raw deferred subtree as the stored lifecycle
  definition.** Dispatch uses the resolved event subtree only for the event
  currently firing; the raw deferred subtree stays the stored definition so
  later events and later loop iterations re-resolve against their own
  event-time state.
  - The parsed `LifecycleConfig` is never mutated by the executor — every
    resolution reads the raw `Expr`/string and resolves a fresh copy per event
    via DM2, so later events and loop iterations re-resolve against their own
    event-time `fm`/globals.
- [x] **4.5 (Validation checkpoint — C2 tests).** Add claudine tests matching
  the spec's C2 + acceptance bullets: top-level `failure.message:
  "{{err.msg}}"` renders the real error end-to-end; `failure.stack` with
  `message(❌️ {{err.msg}})` renders end-to-end through composition (extend
  `emit_preflight_blocked_and_finalize_propagates_err_msg_into_blocked_stack`
  to go through composition, not raw JSON); mixed body `message(phase {{phase}}
  failed: {{err.msg}})` resolves both spans at event-time with live `phase`;
  loop currentness (a lifecycle message's `{{phase}}` reflects the current
  iteration); just-in-time resolution (stack action #1 `set_frontmatter`, action
  #2 references that key — action #2 sees the mutated value); parity
  (event-time rendering of a representative string matches what compose
  produces for the same string with the same data, and the bespoke
  interpolator is gone). Run `just test` in `claudine/`.
  - Added to `lifecycle_executor.rs` tests:
    `top_level_message_interpolates_err_at_event_time` (the original bug),
    `stack_message_interpolates_err_at_event_time`,
    `mixed_body_resolves_both_spans_at_event_time`,
    `message_reflects_current_frontmatter_per_event` (currentness),
    `stack_action_sees_prior_set_frontmatter` (just-in-time), and
    `event_time_rendering_matches_compose` (parity vs `compose_subtree`).
    `lifecycle_context.rs` gained DM2-routed tests for the injected globals.
    The bespoke interpolator is gone (compile-time guarantee — the `interpolate`
    fn and `LifecycleLookup` no longer exist).

> Depends on: Phase 2 (DM2) and Phase 3 (C1 raw subtree).

---

## Phase 5 — Claudine: guard rework (C4)

Goal: with lifecycle strings raw through prepare, the static scans operate on
authored `{{ }}` spans; the leak guard splits into prepare-time deferred-span
handling and post-DM2 dispatch-time enforcement; event-time resolution errors
fail closed.

- [x] **5.1 (C4) Introduce `LATE_BINDING_ROOTS`.** Add a constant in
  `claudine/lib/src/composition/lifecycle.rs` naming `err`/`timing`/`current`
  so every guard and the undefined-variable scan share one authority.
  - Already landed in Phase 3 (`lifecycle.rs:85`) for the pre-flight shell
    resolution. Phase 5 now also consumes it in
    `resolves_outside_frontmatter` (the undefined-variable scan's shared
    known-root authority).
- [x] **5.2 (C4) Rework the err-availability scan.** Update
  `validate_no_err_in_no_error_events` (`lifecycle.rs:2882`) to walk the `{{ }}`
  spans inside communication/action strings **plus** the whole `when:`
  expression; reject `err` in a no-error event
  (`initialize`/`start`/`success`/`loop`). `timing`/`current` are allowed
  everywhere; `doc.err` remains the escape hatch.
  - The scan now walks top-level communication fields
    (`notification_comm_fields`) and stack surfaces. Communication/action
    strings are span-scanned (`literal_spans_reference_err` parses each
    `{{ … }}` span and checks for bare `err`); expression surfaces (`when:`,
    multi-arg args, control operands) keep the whole-expression
    `references_bare_err` check. Both are unified in `surface_references_err`.
- [x] **5.3 (C4) Rework the undefined-variable scan.** Update
  `validate_no_undefined_lifecycle_variables` (`lifecycle.rs:2672`) to treat
  `err`/`timing`/`current` and `ctx`/`env`/`doc` as known roots and flag only
  genuinely-unknown roots (typos). Preserve the existing fallback/ternary
  tolerance for intentionally optional values **only inside the operands**
  where that tolerance is already documented.
  - Added `resolves_outside_frontmatter` (uses `LATE_BINDING_ROOTS`);
    `undefined_bare_variable` and `undefined_stack_variable` now share it, so
    top-level and stack scans use one known-root contract. The
    prepare-time scan stays **removed** for lifecycle (Phase 3 decision); the
    authoritative typo enforcement is event-time DM2 strict mode (5.5/5.6).
    The function/helpers are updated to the new contract and an existing test
    (`bare_err_in_top_level_field_is_not_exempt`) was rewritten to
    `late_binding_global_in_top_level_field_is_a_known_root` per the new rule.
- [x] **5.4 (C4) Split the interpolation-leak guard.** Stop treating authored
  lifecycle strings as prepare-time leaks — they are deferred by design. Keep
  `validate_no_interpolation_leaks` (`lifecycle.rs:2318`) for non-lifecycle
  surfaces. Add a **post-DM2 dispatch-time** enforcement: after DM2 resolves
  an event subtree, run the leak guard on the resolved side-effect strings
  immediately before dispatch; a surviving `{{ }}` is a typed error and the
  side effect is not sent.
  - `validate_no_interpolation_leaks` is unchanged and no longer runs over
    deferred lifecycle strings (Phase 3). The post-DM2 enforcement is
    `reject_surviving_spans` in `lifecycle_executor.rs`, called inside
    `resolve_string_value` after every event-time DM2 resolution: a resolved
    string still carrying a recognized `{{ … }}` span (e.g. a frontmatter value
    that is itself raw template text) becomes an action error before dispatch.
- [x] **5.5 (C4) Fail closed on event-time resolution errors.** Malformed
  expressions, unknown functions, unknown roots (typos / genuinely undefined
  variables), and late-binding globals used outside their legal event fail the
  event with a typed `CompositionError`. Lifecycle side effects must not
  silently render empty operational text for these cases.
  - `resolve_string_value` now uses DM2 `.strict()`. Stack-action failures flow
    through the existing `LifecycleErrorInfo` action-error channel (stack stops,
    routes to `failure` at setup events). Top-level emission was made
    fail-closed: `emit_top_level`/`resolve_emit` return
    `Result<_, LifecycleErrorInfo>` and `execute_event` converts a top-level
    failure into `LifecycleEventOutcome.action_error` (no field dispatched).
- [x] **5.6 (C4) Strict mode does not error on known-but-empty.** A reference
  whose root is a *known* surface — a declared frontmatter key,
  `ctx`/`env`/`doc`, or an in-scope late-binding global — that resolves to
  `null`/empty renders empty, as today. Strictness targets *unknown* roots and
  malformed/illegal expressions only. (Guards existing prompts like
  `implement-plan.md`'s `{{total_phases}}` / `{{spec_file}}` that legitimately
  resolve to empty.) Migration note: an author wanting an *unknown* optional
  name tolerated must opt in with explicit fallback syntax (`{{ maybe || '' }}`).
  - Delivered by DM2 strict mode's existing `is_known_variable_root` /
    `validate_strict_roots` (a known root resolving to `null`/empty renders
    empty; only unknown roots / malformed spans error). Covered by
    `known_but_empty_reference_renders_empty` vs `unknown_root_typo_fails_closed`.
- [x] **5.7 (C4) Deferred effect validation.** For `effect:
  "{{effect_name}}"` and `effect({{effect_name}})` (which cannot be validated
  against the sound catalog at prepare time): validate the raw literal at
  prepare time when no interpolation is present; otherwise validate the
  resolved effect name immediately before dispatch and report
  `LifecycleUnknownEffect` with the event property path. Adjust the existing
  effect validation at `lifecycle.rs:1216` accordingly.
  - Prepare-time validation in `parse_event_block` now skips when the `effect`
    field contains `{{` (deferred); a literal unknown name is still rejected.
    Event-time validation is `validate_effect_name` in the executor (used by the
    top-level audio Effect phase and the `effect(...)` Communication channel):
    an invalid resolved name produces a `LifecycleErrorInfo` built from
    `CompositionError::LifecycleUnknownEffect` (carried as the action error's
    variant), failing closed before dispatch.
- [x] **5.8 (Validation checkpoint — C4 tests).** Add claudine tests matching
  the spec's C4 bullets: no-error events still reject `err` (`start.message:
  "{{err.msg}}"` halts at parse time) while `timing`/`current` are allowed;
  no leak false-positive (a deferred `{{err.msg}}` does not trip the
  prepare-time leak guard); known-but-empty renders empty (`{{spec_file}}`
  resolves to empty and does not error) while a typo (`{{spec_fil}}`) fails
  closed; post-DM2 leak guard (a malformed/nested event-time result still
  containing `{{...}}` fails before any messenger/TTS/sound/stderr/stdout/notify
  side effect is dispatched); deferred effect validation reports
  `LifecycleUnknownEffect` for an invalid resolved name. Run `just test` in
  `claudine/`.
  - Added to `lifecycle.rs` tests:
    `err_interpolation_span_in_top_level_field_rejected_in_no_error_event`,
    `err_interpolation_span_in_stack_message_rejected_in_no_error_event`,
    `timing_and_current_interpolation_allowed_in_no_error_events`,
    `err_interpolation_span_allowed_in_error_carrying_event`,
    `effect_field_with_interpolation_skips_prepare_validation`,
    `effect_field_literal_unknown_name_still_rejected_at_prepare`. Added to
    `lifecycle_executor.rs` tests: `known_but_empty_reference_renders_empty`,
    `unknown_root_typo_fails_closed`, `top_level_unknown_root_fails_event_closed`,
    `post_dm2_surviving_span_fails_before_dispatch`,
    `deferred_effect_invalid_resolved_name_reports_unknown_effect`,
    `deferred_effect_valid_resolved_name_dispatches`. The "no leak
    false-positive" case is covered by Phase 3's
    `lifecycle_err_span_survives_raw_in_effective_frontmatter`. `just test`,
    full `claudine` lib nextest, and `just lint` all pass.

> Depends on: Phase 3 (C1 raw spans) and Phase 4 (C2 post-DM2 enforcement).

---

## Phase 6 — Claudine: dry-run visibility + documentation (C5, C7)

Goal: make the new binding-time behavior legible — raw deferred keys label
themselves as event-time in dry-run output, and every authoritative doc names
the new rule.

- [x] **6.1 (C5) Label excluded keys in dry-run output.** In the dry-run /
  `effective_frontmatter` rendering path, consume the deferred-key metadata
  from DM1 (task 1.3) and label deferred lifecycle keys as **interpolated at
  event-time** so a raw `{{err.msg}}` span reads as intentional rather than as
  the bug that started this thread.
  - `PreparedComposition` gained `deferred_lifecycle_keys: Vec<String>`,
    populated in both `prepare_direct`/`prepare_inline` from the DM1 compose
    report's `deferred_frontmatter_keys` (sorted, present-only). `DryRunRender`
    carries it through `from_request`; `render_metadata_table` adds a
    conditional **Deferred** row reading `interpolated at event-time: <keys>`,
    shown only when at least one lifecycle key was deferred. Covered by
    `table_omits_deferred_row_when_no_keys` and
    `table_shows_deferred_keys_labeled_event_time`.
- [x] **6.2 (C7) `claudine/docs/topics/lifecycle.md`.** Add **"Binding time:
  early vs late"** and **"When lifecycle properties interpolate"** sections
  covering the event-time rule, the early/late split, and the `shell`
  exception (C3).
  - Added the two sections after the lifecycle-properties table, plus a
    `### The shell exception` subsection. Reworked the `LifecycleInterpolationLeak`
    (now post-DM2 dispatch-time), `LifecycleUndefinedVariable` (event-time strict,
    known-but-empty renders empty), and `LifecycleErrNotAvailable` (walks `{{ }}`
    spans + `when:`) validation sections to match the implemented behavior.
- [x] **6.3 (C7) `claudine/docs/topics/composition.md`.** Update the five-stage
  compose pipeline and lifecycle guard text so `effective_frontmatter` explains
  deferred lifecycle keys and the event-time second pass.
  - Added a "Deferred lifecycle keys" callout after Direct Composition,
    extended the Lifecycle Integration section with the event-time second-pass
    rule, and added the **Deferred** row to the Dry Run metadata-table
    description. `hash:` frontmatter regenerated with `md hash`.
- [x] **6.4 (C7) `claudine/docs/topics/frontmatter-properties.md`.** Update
  lifecycle property descriptions to mention event-time interpolation and the
  `shell` exception.
  - Added an "Event-time interpolation" callout under Lifecycle Notifications
    and reworked the `effect` sub-property row (literal → parse-time;
    interpolated → event-time `LifecycleUnknownEffect`). `hash:` regenerated.
- [x] **6.5 (C7) `claudine/features/2026-05-12-lifecycle/spec.md`.** Clarify
  line 67 (lifecycle property interpolation is event-time) and lines 96–101
  (communication/action strings are literal-with-`{{ }}`; the `err` scan walks
  their interpolation spans; only `when:` is a whole expression).
  - Clarified the lifecycle-only-globals note (non-lifecycle frontmatter/body
    are compose-time; lifecycle event properties interpolate at event-time) and
    rewrote the `err`-scan surface list to distinguish whole-expression `when:`
    from span-scanned literal communication/action strings. No `hash:` property.
- [x] **6.6 (C7) `.claude/skills/claudine/`.** Update `SKILL.md`
  (stack-globals bullet — event-time interpolation through DM2, early/late
  binding split, shell exception) and `timeline.md` (new entry for the
  late-binding fix, referencing this fix directory).
  - Rewrote the `SKILL.md` stack-globals bullet into a "Lifecycle event-time
    interpolation (late binding)" bullet (DM1/DM2, early/late split, fail-closed,
    `shell` exception, bespoke interpolator removed); reworked the leak-guard
    bullet to post-DM2 + a kept-prepare-scan bullet (drift fix — the old bullets
    described the now-removed prepare-time lifecycle scans). Added the
    `2026-06-25 — late-binding` timeline entry. `hash:` regenerated on both via
    `md hash --save`; `SKILL.md` `last_updated` bumped to 2026-06-25.
- [x] **6.7 (C6) Verify the single-parameter argument grammar is unchanged.**
  Confirm `message(phase 6 is too large)` is literal, `effect(crowd-applause)`
  is the literal name, and `message(failed: {{err.msg}})` is literal +
  interpolation. This is a verification task — no behavior change expected
  beyond what Phases 3–5 deliver.
  - Verified: `parse_short_form_action` routes single-parameter verbs through
    `is_single_text_arg_verb` (every `CommunicationChannel` verb — including
    `effect` — plus `shell`/`error`/`proxy`/`resume`/`defer`), producing a single
    `Expr::StringLiteral` from the trimmed body with `{{ … }}` spans intact;
    multi-arg verbs keep the comma-separated expression grammar. Unchanged by
    Phases 3–5. Fixed a drifted comment that claimed interpolation was already
    applied at parse time (it is now event-time via DM2).
- [x] **6.8 (Validation checkpoint — docs review).** Walk every acceptance
  criterion (1–13 in the spec) against the implemented behavior and the
  updated docs; confirm each doc section names the new rule consistently with
  the implementation.
  - Walked all 13 criteria: (1) top-level + stack `err` → lifecycle.md *Binding
    Time*; (2) event-time `timing`/`current`/loop `phase` → *When Lifecycle
    Properties Interpolate*; (3) no-error `err` halts → `LifecycleErrNotAvailable`
    section; (4) shell byte-identical + late-binding rejected → *shell exception*
    in lifecycle.md / composition.md / frontmatter-properties.md; (5) literal
    args + whole-expression `when:`/loop → *Action Forms* + *Binding Time*;
    (6) body/ordinary-fm timing unchanged → *Binding Time*; (7) DM2 + bespoke
    removed → SKILL.md / timeline.md; (8) opt-in DM1/DM2 → SKILL.md; (11) fails
    closed, known-but-empty empty → `LifecycleUndefinedVariable`/`When…` sections.
    All consistent. Caught and fixed two doc drifts: the SKILL.md leak/undefined
    bullets (still described removed prepare-time scans) and a `lifecycle.rs`
    comment claiming parse-time interpolation. `just test` (1694 pass) and
    `just lint` clean.

> Docs (6.2–6.6) can be drafted in parallel with Phases 3–5; the final review
> (6.8) must follow implementation.

---

## Phase 7 — Integration validation, parity, and regression

Goal: prove the whole change end-to-end against the spec's acceptance criteria
and ensure no regression for existing claudine + darkmatter callers (DM1/DM2
are opt-in; default compose behavior for every other caller is unchanged).

- [x] **7.1 Reproduction fixture.** `prompts/implement-plan.md`'s `failure`
  reporting `{{err.msg}}` renders the actual error on a real/simulated failure
  (the original bug from the spec's *Problem* section).
  - Added `reproduction_failure_block_renders_real_error_at_event_time`
    (`lifecycle_executor.rs`): a top-level `failure` block shaped like
    `implement-plan.md` (`say` + `message`, mixing early-binding `{{phase}}` with
    late-binding `{{err.msg}}`) renders the real values
    (`"❌️ phase 6 failed: disk full"` + `"Phase 6 ran into problems!"`) when the
    failure event fires — the original bug that collapsed `{{err.msg}}` to empty
    at compose time. The schema-bearing reproduction precondition is covered by
    7.3's `schema_validates_while_lifecycle_err_span_is_deferred`.
- [x] **7.2 Cross-platform smoke.** Run the claudine + darkmatter suites on
  macOS (primary host). Reason explicitly about Windows/Linux path and shell
  behavior for the pre-flight stamping (C3) and the system shell runner — no
  platform-specific interpolation behavior is introduced, but the
  approved == executed byte equality must hold across shells.
  - macOS suites green: claudine lib 2882 / claudine-contract 47 /
    claudine-cli 1694 pass; darkmatter (workspace-delegated `just test`) passes
    for darkmatter + every darkmatter consumer (only pre-existing flaky
    `sniff` timeout / `visualizer` failures, both outside this blast radius).
  - **Cross-platform reasoning (C3 stamping):** the pre-flight resolution is a
    pure-string transform — `resolve_lifecycle_shell_commands` runs each shell
    literal span through DM2 `SubtreeCompose::strict()` and stamps the resolved
    UTF-8 bytes straight back into the `Expr::StringLiteral`. No `Path`
    separator handling, shell-quoting, or OS-conditional branch sits between
    resolution and stamping, so the *resolved bytes* are identical on
    macOS/Linux/Windows for the same frontmatter + `ctx`/`env`. The system shell
    runner later executes those exact stamped bytes, so approved == executed
    byte-equality is a property of the data path, not of the host shell — the
    only platform difference is *which* shell interprets the (already-identical)
    bytes, which is unchanged by this fix. Read-side functions that touch paths
    (`file_exists`, `dirname`, …) were already cross-platform via
    `ResolutionContext` and are unchanged here.
- [x] **7.3 Schema compatibility.** A prompt with `$schema` and
  `failure.message: "{{err.msg}}"` still validates ordinary schema inputs and
  reaches lifecycle parsing (DM1b).
  - Added `schema_validates_while_lifecycle_err_span_is_deferred`
    (`schema_validation.rs`): drives the real CLI entry
    `prepare_direct_with_schema` over a doc with `$schema` (`phase`/`total_phases`
    `number(required)`) + `failure.message: "❌️ phase {{phase}} failed:
    {{err.msg}}"`. The ordinary schema inputs validate and are present
    (`phase=1`, `total_phases=3`); `failure` is reported in
    `deferred_lifecycle_keys`; the late-binding span survives raw into
    `lifecycle.failure.message` — i.e. DM1b excludes the deferred key from user
    schema value validation while ordinary validation still runs.
- [x] **7.4 Default-caller parity.** With no `exclude_keys` set, darkmatter
  compose output is byte-identical to `main` for a representative set of
  non-lifecycle prompts (acceptance criterion 8).
  - DM1/DM2 are strictly opt-in: `ComposeOptions::exclude_keys` defaults to an
    empty set and the subtree-compose entry is a *new* public fn that no
    existing caller invokes. Parity is proven by (a) the Phase 1 unit tests
    `empty_exclude_set_is_no_op` / `non_excluded_key_still_resolves_normally`,
    and (b) the full workspace `just test` passing unchanged for darkmatter and
    **every** downstream darkmatter consumer (renderable, biscuit-terminal,
    biscuit-icon, reaper, …) — none of which set `exclude_keys`, so their
    compose output is unaffected by this change.
- [x] **7.5 Acceptance criteria walk-through.** Verify all 13 acceptance
  criteria from the spec pass: late-bound `err` in top-level + stack;
  event-time `timing`/`current`/live loop `phase`; `err` still halts in
  no-error events; shell pre-flight byte-identical and late-binding-in-shell
  rejected; literal-default args with `{{ }}`; `when:`/loop conditions whole
  expressions; body + ordinary frontmatter timing unchanged; event-time
  through DM2 + bespoke interpolator removed; DM1/DM2 opt-in; composed keys
  cannot consume deferred subtrees; user schema unchanged for ordinary inputs;
  event-time failures fail closed (known-but-empty still renders empty); docs
  updated; all existing tests pass.
  - Each criterion maps to a green test (module abbreviated: `le` =
    `lifecycle_executor.rs`, `lc` = `lifecycle.rs`, `pp` = `prepare.rs`,
    `sv` = `schema_validation.rs`, `dm` = darkmatter `compose`):
    1. top-level + stack `err` → `le::top_level_message_interpolates_err_at_event_time`,
       `le::stack_message_interpolates_err_at_event_time`,
       `le::reproduction_failure_block_renders_real_error_at_event_time`.
    2. event-time `timing`/`current`/live loop `phase` →
       `le::message_reflects_current_frontmatter_per_event` (+ the
       `lifecycle_context.rs` injected-globals tests).
    3. `err` halts in no-error events at parse time →
       `lc::err_interpolation_span_in_top_level_field_rejected_in_no_error_event`,
       `lc::timing_and_current_interpolation_allowed_in_no_error_events`.
    4. shell pre-flight byte-identical + late-binding rejected →
       `pp::shell_command_early_binding_resolves_at_preflight`,
       `pp::shell_command_late_binding_reference_rejected_at_prepare`,
       `pp::shell_long_form_command_late_binding_rejected_at_prepare`.
    5. literal-default args with `{{ }}`; `when:`/loop whole expressions →
       6.7 verification + `le::mixed_body_resolves_both_spans_at_event_time`.
    6. body + ordinary frontmatter timing unchanged →
       `dm::empty_exclude_set_is_no_op`, `dm::non_excluded_key_still_resolves_normally`.
    7. event-time via DM2 + bespoke interpolator removed →
       `le::event_time_rendering_matches_compose` (compile-time: `interpolate` /
       `LifecycleLookup` no longer exist).
    8. DM1/DM2 opt-in → see 7.4 (`dm::empty_exclude*` + workspace parity).
    9. composed keys cannot consume deferred subtrees →
       `dm::composed_key_referencing_deferred_bare_root_rejected`,
       `dm::composed_key_referencing_deferred_doc_namespace_rejected`,
       `dm::composed_key_not_referencing_deferred_key_unaffected`.
    10. user schema unchanged + deferred not rejected →
       `sv::schema_validates_while_lifecycle_err_span_is_deferred`,
       `dm::ordinary_schema_validation_runs_alongside_deferred_key`,
       `dm::deferred_key_with_lifecycle_syntax_not_rejected_by_user_schema`.
    11. fail closed; known-but-empty renders empty →
       `le::unknown_root_typo_fails_closed`,
       `le::top_level_unknown_root_fails_event_closed`,
       `le::post_dm2_surviving_span_fails_before_dispatch`,
       `le::deferred_effect_invalid_resolved_name_reports_unknown_effect`,
       `le::known_but_empty_reference_renders_empty`.
    12. docs updated → Phase 6 (lifecycle.md / composition.md /
       frontmatter-properties.md / spec / SKILL.md / timeline.md).
    13. all existing tests pass → 7.6 sweep below.
- [x] **7.6 Full test + lint sweep.** `just test` and `just lint` in both
  `darkmatter/` and `claudine/`, plus `just doctest` where the compose public
  API docs reference examples. Confirm `cargo fmt --check` (read-only) reports
  no drift introduced by this change (per repo policy, never run write-mode
  `cargo fmt`).
  - `just test` (claudine): lib 2882 / contract 47 / cli 1694 — all pass.
    `just test` (darkmatter): workspace-delegated; darkmatter + all consumers
    pass (only pre-existing flaky `sniff` timeout / `visualizer`, outside blast
    radius and untouched by this change).
  - `just lint` (claudine): claudine / claudine-contract / claudine-cli clean.
    `just lint` (darkmatter): darkmatter / darkmatter-cli clean.
  - `just doctest`: claudine 11 pass, claudine-contract 2 pass, darkmatter 172
    pass.
  - `cargo fmt --check -p claudine` (read-only): the two files this phase
    touched (`lifecycle_executor.rs`, `schema_validation.rs`) show **no diff
    hunk within the added test regions** — the only diffs reported are the
    pre-existing repo-wide rustfmt-version drift in untouched code (closure.rs,
    benches, earlier-phase edits). Per repo policy, write-mode `cargo fmt` was
    **not** run.

---

## Notes for the implementer

- **Never run `cargo fmt` write-mode** — match surrounding style by hand
  (repo policy in `AGENTS.md`).
- **Rustdoc convention:** no `# H1` inside `///`; use `## H2` sections in the
  order summary → `## Examples` → `## Returns` → `## Errors` → `## Panics` →
  `## Safety` → `## Notes`.
- **Comment quality:** any edit that changes a symbol's behavior must include
  a pass over its `///`/`//!` docs and inline `//` comments; fix or delete
  drifted ones in the same change. When drift is detected, assume the code is
  correct and the comment is wrong.
- **Non-interactive session:** export `GIT_TERMINAL_PROMPT=0` before any git
  command; never run credential prompts, `gpg`, `ssh-add`, `sudo`, or
  background `&` shells. If a shell command does not complete within ~60s,
  abandon it.
- **Testing:** use `just test` (unit) / `just test-l2` (integration) inside a
  package area; `just test {pkg}` from the repo root. Nextest is the runner.
