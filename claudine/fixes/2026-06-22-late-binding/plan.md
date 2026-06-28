---
agent: open_code/zai-coding-plan/glm-5.2
phases: 7
created: 2026-06-27
start_phase: 1
yolo: "true"
---

# Execution Plan — Event-Time Interpolation for Lifecycle Properties (Late Binding)

Converts `spec.md` into a dependency-ordered, observable execution plan. The
spec's design principle is **one interpolation engine**: route the event-time
pass back through Darkmatter (DM1/DM2) and retire claudine's bespoke
`interpolate`/`LifecycleLookup`.

Dependency graph (high level):

```
DM1 (exclude-keys) ─┬─► C1 (parse raw subtree) ──► C2 (event-time DM2) ──► C4 (guards)
                    │                              ►
DM2 (subtree compose)┘                              C3 (shell early-binding)

C5 (dry-run) ◄── DM1        Edge cases ◄── C2/C3/C4        Docs (C7) ◄── all
```

Legend:

- `[P]` — parallelizable with sibling tasks in the same phase.
- **Validation** blocks are checkpoints; a phase is not done until its
  checkpoint is green.

## Phase 1 — Darkmatter: exclude-keys compose option (DM1, DM1a, DM1b)

Goal: a compose option that defers named top-level frontmatter keys from every
compose-time resolution pass, so they survive raw in `effective_frontmatter`.
Default empty ⇒ no behavior change for any other caller.

- [ ] **DM1 — add the exclude-keys option.** Add `exclude_keys: HashSet<String>`
  to `ComposeOptions` (`darkmatter/lib/src/markdown/compose/context/options.rs`)
  with a `with_exclude_keys<I: IntoIterator<Item = S>>` builder and an
  `exclude_keys()` reader. Default is empty.
- [ ] **DM1 — thread exclusion through every compose-time pass.** Skip excluded
  keys in: `{{ }}` frontmatter interpolation, whole-value expansion,
  `$(...)` shell expansion (`frontmatter_shell_expansion.rs`), and schema value
  interpolation. Touch points:
  `frontmatter_interpolation.rs`, `frontmatter_shell_expansion.rs`,
  `schema_validation.rs`, `pipeline/mod.rs`, `preflight/collect.rs`.
- [ ] **DM1 — preserve types and shape of deferred values.** A deferred key
  keeps its authored structure (string/object/array) with `{{ }}` spans intact;
  only *resolution* is skipped, not parsing/typing.
- [ ] **DM1 — surface deferred keys in the compose report.** Make deferred-key
  status explicit in the report (`context/report.rs`) so callers can distinguish
  "raw because intentionally deferred" from "raw because composition failed".
  This metadata drives claudine's dry-run labeling (C5) and diagnostics.
- [ ] **DM1a — forbid composed keys from reading deferred keys.** During
  dependency analysis in `frontmatter_interpolation.rs`, reject a non-deferred
  key that references a deferred key via bare root (`{{ failure }}`),
  `{{ failure.message }}`, or `{{ doc.failure.message }}`. Emit a typed error
  naming both the composed key and the deferred key. Reject (not warn) — a
  warning would still inject raw lifecycle syntax.
- [ ] **DM1b — schema validation ordering.** Ordinary non-deferred frontmatter
  is validated by user `$schema` exactly as today. Deferred lifecycle keys are
  excluded from user schema *value* validation (validated later by claudine's
  lifecycle parser/guards). If a non-claudine caller opts into deferred keys,
  that caller owns the equivalent subtree validation.
- [ ] **Tests — DM1.** A listed key is left raw across all passes; a
  non-excluded key is unaffected; types of excluded values are preserved.
- [ ] **Tests — DM1a.** A composed key referencing a deferred key (bare or
  `doc.<key>`) fails with a diagnostic naming both keys.
- [ ] **Tests — DM1b.** Ordinary schema validation still runs for non-deferred
  keys while a deferred lifecycle key containing `{{err.msg}}` is *not*
  rejected by user schema value validation.

**Validation checkpoint (Phase 1):** `just test darkmatter` is green for all
`exclude_keys`/deferred-reference/schema-ordering cases; default compose output
for an empty `exclude_keys` is byte-identical to pre-change.

## Phase 2 — Darkmatter: subtree compose with injectable lookup (DM2)

Goal: a public entry point that interpolates a frontmatter *subtree* through the
same interpolation core as main compose, driven by a caller-supplied layered
lookup. **Largely parallelizable with Phase 1** (it depends only on the existing
interpolation core, not on DM1).

- [ ] `[P]` **DM2 — add the subtree module.** Create
  `darkmatter/lib/src/markdown/compose/subtree.rs` and re-export it from
  `compose/mod.rs`. Reuse `Evaluator`/`interpolate_value` so a lifecycle string
  resolved at event-time produces the same typed/substituted result as the same
  string with the same data at compose-time.
- [ ] `[P]` **DM2 — `LayeredLookup` + `InjectedGlobal`.** Caller-injected named
  globals layered over Darkmatter seed state (`ctx`/`env`/`doc` + read-side
  functions). Support **eager** (`InjectedGlobal::Eager(Value)`) and **lazy**
  (`InjectedGlobal::Lazy(closure)`, evaluated on first access and memoized per
  subtree compose so it sees state at the point of first reference). API takes
  arbitrary named globals — not three hardcoded names.
- [ ] `[P]` **DM2 — strictness mode flag.** `SubtreeStrictness` (Lenient/Strict)
  is **orthogonal** to resolution typing: it changes what happens on *failure*
  (typed error vs lenient empty), not how a successful resolution is typed or
  substituted. Claudine uses Strict for lifecycle communication/action text.
- [ ] `[P]` **DM2 — known-root gating.** Extend `EvaluationLookup` with an
  `is_known_variable_root` trait method (default `true`) used by strict subtree
  compose to flag unknown roots (typos) while tolerating known-but-empty.
- [ ] `[P]` **Tests — DM2.** Subtree composed with injected eager (`err`) and
  lazy (`current`) globals resolves them; layered seed state still resolves;
  whole-value typing and mixed-string behavior match main compose.
- [ ] `[P]` **Tests — DM2 laziness.** A lazy global is evaluated only when
  referenced (never eagerly at subtree-compose entry).
- [ ] `[P]` **Tests — DM2 strictness.** Strict subtree compose returns typed
  errors for malformed spans, unknown functions, and unknown roots instead of
  returning a degraded string.

**Validation checkpoint (Phase 2):** `just test darkmatter` green for all
subtree cases; a representative string yields identical bytes whether resolved
via main compose or subtree compose with matching data (parity).

## Phase 3 — Claudine: prepare-time wiring (C1, C3)

Goal: claudine hands the seven lifecycle event keys to DM1's exclude set, and
pre-flight resolves shell commands via DM2 with an early-binding-only lookup.
Depends on Phase 1 (DM1) and Phase 2 (DM2).

- [ ] **C1 — pass lifecycle keys to the exclude set.** In
  `claudine/lib/src/composition/prepare.rs`, define `LIFECYCLE_EVENT_KEYS`
  (`initialize`/`start`/`success`/`blocked`/`failure`/`finalize`/`loop`) and
  pass them via `ComposeOptions::with_exclude_keys(...)`.
- [ ] **C1 — confirm `loop:` deferral is safe for iteration controls.** Per spec
  edge case "Top-level `loop` is mixed-purpose": verify whether deferring all of
  `loop:` affects `while`/`until`/`actions`/`max`/`fail_fast`. If any iteration
  control depends on compose-time interpolation, extend DM1 to accept sub-paths
  (`loop.say`, `loop.stack`, …). **Verify before assuming option (a).**
- [ ] **C1 — parse lifecycle config from the raw subtree.** `parse_lifecycle_config`
  reads from `effective_frontmatter`, so every lifecycle string keeps its
  `{{ }}` spans. Non-lifecycle keys compose as today.
- [ ] **C3 — pre-flight shell resolution via DM2.** In `preflight.rs`, resolve
  each shell command (short-form `shell(...)` and long-form `shell` with
  `command:`) via DM2 with an **early-binding-only lookup** (no
  `err`/`timing`/`current`).
- [ ] **C3 — stamp resolved bytes.** The approved command must equal the executed
  command; stamp the resolved bytes back so approval and execution are
  byte-identical.
- [ ] **C3 — reject late-binding references in shell.** Any `err`/`timing`/`current`
  reference in a shell command fails at prepare time with a typed error
  (`LifecycleShellResolution`).
- [ ] **Tests — C1.** Lifecycle keys survive raw in `effective_frontmatter`;
  non-lifecycle keys compose normally; variable values are available to later
  event-time resolution.
- [ ] **Tests — C3.** `shell(git fetch {{branch}})` resolves at pre-flight with
  approved == executed; `shell(rm {{err.msg}})` and long-form
  `command: "rm {{err.msg}}"` are rejected at prepare time.

**Validation checkpoint (Phase 3):** `just test claudine` green; `--dry-run`
shows lifecycle keys raw; shell approval/execution byte-equality holds.

## Phase 4 — Claudine: event-time interpolation via DM2 (C2)

Goal: when an event fires, interpolate each lifecycle property subtree via DM2
with a lookup layering `err`/`timing`/`current` over the composed document
state. Retire the bespoke interpolator. Depends on Phase 2 (DM2) and Phase 3 (C1).

- [ ] **C2 — replace bespoke interpolation with DM2.** Remove claudine's
  `interpolate` + `LifecycleLookup` runtime path
  (`lifecycle_executor.rs`, `lifecycle_context.rs`). Replace with
  `lifecycle_injected_globals` → Darkmatter `LayeredLookup`.
- [ ] **C2 — reduce lifecycle context to the injected-globals layer.**
  `lifecycle_context.rs` shrinks to building the `err`/`timing`/`current`
  injected globals (plus current document state) handed to DM2.
- [ ] **C2 — just-in-time resolution granularity.** Resolve each lifecycle
  property/action string via DM2 immediately before it is used, against the
  *live* effective document state at that instant — not once when the event
  fires. Required so a `set_frontmatter` side-effect from stack action #1 is
  visible to action #2, and so `current`/`timing` read state at the point of use.
- [ ] **C2 — uniform top-level and stack handling.** The top-level/stack
  distinction disappears: `failure.message: "{{err.msg}}"` and a stack
  `message(… {{err.msg}})` both resolve at event-time through the same path.
- [ ] **C2 — keep raw subtree as the stored definition.** Dispatch uses the
  resolved event subtree only for the event currently firing; the raw deferred
  subtree stays the stored lifecycle definition so later events and later loop
  iterations re-resolve against their own event-time state.
- [ ] **Tests — top-level late binding.** `failure: message: "{{err.msg}}"`
  through the real pipeline, executed with an injected `err`, emits the real
  error.
- [ ] **Tests — stack late binding.** `failure: stack: - action: message(❌️ {{err.msg}})`
  end-to-end (extends
  `emit_preflight_blocked_and_finalize_propagates_err_msg_into_blocked_stack` to
  go through composition, not raw JSON).
- [ ] **Tests — mixed body.** `message(phase {{phase}} failed: {{err.msg}})` —
  both spans resolve at event-time; `phase` is the live value.
- [ ] **Tests — just-in-time resolution.** A stack whose action #1 runs
  `set_frontmatter` and whose action #2 references that key sees the mutated
  value in action #2.
- [ ] **Tests — parity.** The bespoke claudine interpolator is gone; event-time
  rendering of a representative string matches what compose produces for the
  same string with the same data.

**Validation checkpoint (Phase 4):** `just test claudine` green; the bespoke
interpolator is fully removed (no `fn interpolate` in
`lifecycle_executor.rs`); real error surfaces in `failure` messages.

## Phase 5 — Claudine: guard rework (C4)

Goal: static scans operate on authored `{{ }}` spans; leak enforcement moves to
dispatch-time (post-DM2). Depends on Phase 4 (C2).

- [ ] **C4 — `err`-availability scan.** Walk `{{ }}` spans inside
  communication/action strings plus the whole `when:` expression; reject `err`
  in a no-error event (`initialize`/`start`/`success`/`loop`) as
  `LifecycleErrNotAvailable`. `timing`/`current` are allowed everywhere;
  `doc.err` remains the escape hatch. Add `LATE_BINDING_ROOTS` constant in
  `lifecycle.rs`.
- [ ] **C4 — undefined-variable scan.** Treat `err`/`timing`/`current` and
  `ctx`/`env`/`doc` as known roots; flag only genuinely-unknown roots (typos).
  Preserve existing fallback/ternary tolerance for intentionally optional
  values, only inside the operands where that tolerance is documented.
- [ ] **C4 — post-DM2 leak guard (dispatch-time).** Stop treating authored
  lifecycle strings as prepare-time leaks (they are deferred by design). After
  DM2 resolves an event subtree, run the leak guard on the resolved side-effect
  strings immediately before dispatch. A surviving `{{ }}` is a typed error and
  the side effect is not sent.
- [ ] **C4 — event-time resolution errors fail closed.** Malformed expressions,
  unknown functions, unknown roots (typos / genuinely undefined variables), and
  late-binding globals used outside their legal event fail the event with a typed
  `CompositionError`. Lifecycle side effects must not silently render empty
  operational text for these cases.
- [ ] **C4 — strict mode tolerates known-but-empty.** A reference whose root is
  a *known* surface (declared frontmatter key, `ctx`/`env`/`doc`, or an in-scope
  late-binding global) that resolves to `null`/empty renders empty, as today.
  Strictness targets *unknown* roots and malformed/illegal expressions, not
  legitimately-absent values. (Protects existing prompts that reference optional
  keys like `{{total_phases}}` or `{{spec_file}}`.)
- [ ] **Tests — no-error events reject `err`.** `start: message: "{{err.msg}}"`
  halts at parse time; `timing`/`current` are allowed.
- [ ] **Tests — no leak false-positive.** A deferred `{{err.msg}}` does not trip
  the prepare-time leak guard.
- [ ] **Tests — known-but-empty renders empty.** A lifecycle message referencing
  a declared frontmatter key that resolves to empty (e.g. `{{spec_file}}`)
  renders empty and does not error; a typo (`{{spec_fil}}`) fails closed.
- [ ] **Tests — post-DM2 leak guard.** A malformed or nested event-time result
  that still contains `{{...}}` fails before any
  messenger/TTS/sound/stderr/stdout/notify side effect is dispatched.

**Validation checkpoint (Phase 5):** `just test claudine` green; `err` scan and
post-dispatch leak guard behave per spec; existing optional-key prompts
(`implement-plan.md`) do not regress.

## Phase 6 — Claudine: visibility & edge cases (C5 + edge cases)

Goal: dry-run labels deferred keys, and the remaining edge cases (effect
validation, empty normalization, loop currentness) are handled. Depends on
Phases 3–5.

- [ ] **C5 — dry-run labeling.** Excluded keys render raw in
  `effective_frontmatter` / `--dry-run`; label them as **interpolated at
  event-time** so a raw span reads as intentional rather than as the original
  bug. Touch the dry-run renderer (`claudine/cli/src/commands/wrap/composition/dry_run.rs`).
- [ ] **Edge — deferred effect validation.** `effect: "{{effect_name}}"` and
  `effect({{effect_name}})` cannot be validated against the sound catalog at
  prepare time. Validate the raw literal at prepare time when no interpolation
  is present; otherwise validate the resolved effect name immediately before
  dispatch and report `LifecycleUnknownEffect` with the event property path.
- [ ] **Edge — empty resolved communication fields.** Preserve existing
  empty-string normalization after event-time interpolation: a field that
  resolves to empty is treated as absent, but only after *successful*
  interpolation; errors do not normalize away.
- [ ] **Edge — loop currentness.** Re-composing the event subtree each iteration
  means `{{phase}}` reflects the live iteration (a correctness improvement, not
  a regression). Verify cost is negligible (per-event subtree compose is tiny).
- [ ] **Edge — `doc.<excluded-key>`.** Document that it resolves to the raw
  subtree (rare).
- [ ] **Tests — deferred effect validation.** `effect: "{{effect_name}}"` validates
  the resolved sound name at event-time and reports `LifecycleUnknownEffect` for
  an invalid result.
- [ ] **Tests — loop currentness.** A lifecycle message's `{{phase}}` reflects the
  current iteration.
- [ ] **Tests — schema compatibility.** A prompt with `$schema` and
  `failure.message: "{{err.msg}}"` still validates ordinary schema inputs and
  reaches lifecycle parsing.

**Validation checkpoint (Phase 6):** dry-run output clearly labels deferred
keys; all edge-case tests green.

## Phase 7 — Documentation, parity & acceptance (C6, C7, acceptance)

Goal: docs reflect the new event-time rule; grammar is confirmed unchanged; the
full suite is green and every acceptance criterion is demonstrably met.
Depends on all prior phases.

- [ ] **C6 — verify single-parameter grammar is unchanged.** Confirm
  `message(phase 6 is too large)` is literal, `effect(crowd-applause)` is the
  literal name, and `message(failed: {{err.msg}})` is literal + interpolation.
  This fix only supplies the event-time interpolation that pairs with
  `literal-short-form-args`; no grammar change.
- [ ] **C7 — `docs/topics/lifecycle.md`.** Add **"Binding time: early vs late"**
  and **"When lifecycle properties interpolate"** (event-time rule, early/late
  split, the `shell` exception).
- [ ] **C7 — `docs/topics/composition.md`.** Update the five-stage compose
  pipeline and lifecycle guard text: `effective_frontmatter` explains deferred
  lifecycle keys and the event-time second pass.
- [ ] **C7 — `docs/topics/frontmatter-properties.md`.** Update lifecycle property
  descriptions to mention event-time interpolation and the `shell` exception.
- [ ] **C7 — `features/2026-05-12-lifecycle/spec.md`.** Clarify line 67 (lifecycle
  property interpolation is event-time) and lines 96–101 (communication/action
  strings are literal-with-`{{ }}`; the `err` scan walks their interpolation
  spans; only `when:` is a whole expression).
- [ ] **C7 — `.claude/skills/claudine/`.** Update the timeline and the
  `SKILL.md` stack-globals bullet.
- [ ] **Reproduction fixture.** `prompts/implement-plan.md`'s `failure` reporting
  `{{err.msg}}` renders the actual error on a real/simulated failure.
- [ ] **Acceptance walkthrough.** Verify each of the 13 acceptance criteria in
  `spec.md` (top-level + stack `err.msg` render; event-time state reflected;
  `err` rejected in no-error events; shell approval == execution; literal-by-default
  args; body/ordinary frontmatter timing unchanged; DM2 is the engine; DM1/DM2
  opt-in; no accidental consumption of deferred subtrees; user schema
  unchanged; event-time failures fail closed while known-but-empty renders
  empty; docs updated; all existing tests pass).

**Validation checkpoint (Phase 7):** `just test claudine` and
`just test darkmatter` both green; `just lint` clean for both packages; doctests
current; acceptance criteria checklist fully satisfied.

## Cross-cutting notes

- **Rule 3 (Surgical Changes).** Touch only the files in the spec's touch list
  plus the docs/skill files. Do not refactor adjacent code or run `cargo fmt`
  write-mode (match surrounding style by hand; `main` is the formatting
  authority).
- **One engine.** The bespoke claudine interpolator must be *removed*, not left
  as dead code, by the end of Phase 4.
- **Fail closed.** Event-time interpolation failures never silently render empty
  operational text and never dispatch raw `{{ }}`; only legitimately-absent
  *known* values render empty.
- **Cross-platform.** All changes must compile and behave identically on macOS,
  Windows, and Linux — no platform-specific interpolation or shell behavior.
