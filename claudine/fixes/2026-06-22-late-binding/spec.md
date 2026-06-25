---
status: ready for planning and implementation
created: 2026-06-22
area: claudine
review_iterations: 3
reviewed: true
packages:
    - darkmatter
    - claudine
---

# Event-Time Interpolation for Lifecycle Properties (Late Binding)

## Problem

A composed prompt cannot surface the runtime error in a `failure` message.
Both obvious authoring forms fail:

```yaml
# top-level field — rejected at prepare time
failure:
    message: "❌️  {{err.msg}}"        # "references undefined variable `err`"

# stack action — parses, but renders empty
failure:
    stack:
        - action: message(❌️  {{err.msg}})   # "❌️  " — err is gone
```

There is no working way to put `err`, `timing`, or `current` into a lifecycle
message. These globals exist to let a lifecycle event **react to and report the
state at the moment it fires**, and that is exactly what is unreachable.

Cause: lifecycle property strings are interpolated during the initial *compose*,
before any event fires, so `{{err.msg}}` resolves against a context where `err`
does not exist and collapses to empty. A contributing factor — the
`literal-short-form-args` change (2026-06-24) made single-parameter action
arguments literal text (correct; kept) but left no late-binding-aware
interpolation to pair with it.

## The principle: lifecycle properties interpolate at event-time

> **Every frontmatter property of a lifecycle event is interpolated when that
> event is triggered — not during the initial compose.** This lets the
> interpolation use both the compose-time variables (`ctx`, `env`, `doc`) *and*
> the dynamic late-binding variables (`err`, `timing`, `current`) that report
> the state *at the moment the event fires* — rather than the state captured
> right before we composed.

This is also the **consistent** rule. Everywhere else, a value is literal text
and `{{ }}` is how you opt into the expression engine: ordinary frontmatter is
literal unless wrapped in `{{ }}`; `$(ls dir)` treats `dir` as the literal word
and needs `$(ls '{{dir}}')` to inject `doc.dir`. Lifecycle communication
properties (`message`, `say`, `warn`, `notify`, `effect`, side-effect args)
follow the same rule: literal prose with `{{ }}` interpolation. The only
whole-expression surfaces are the **conditionals** — stack `when:` and loop
`while`/`until` — which must produce a boolean.

The single timing carve-out is `shell(...)` commands (C3): approved at
pre-flight, so early-binding only.

Reader's note: this is an intended change to the lifecycle interpolation
standard, not a relaxation of the lifecycle safety guards. The old compose-time
rule made `err`, `timing`, and `current` unusable in operational messages. The
new rule defers lifecycle strings by design, but event-time interpolation still
fails closed before dispatch when a lifecycle string is malformed or references
a genuinely unknown value.

### Early vs late binding (the contract)

- **Early-binding (resolvable before the run):** `doc.*` (frontmatter), `ctx.*`,
  `env.*`, read-side functions (`parent_dir`, `dirname`, `frontmatter`,
  `file_exists`, …).
- **Late-binding (only exists at event-time):** `err` (in `blocked`/`failure`/
  optional-error `finalize`), `timing`, `current`.

A lifecycle property's `{{ }}` interpolation resolves against the union of both,
**at event-time**. Bare frontmatter references (`{{phase}}`, `{{artifact.path}}`)
read the current effective document state at the moment the event fires, not a
copy captured at initial compose. This is required for loop actions and lifecycle
side-effect actions that mutate frontmatter between iterations. Document body and
ordinary (non-lifecycle) frontmatter keys remain compose-time.

## Design principle: do it in Darkmatter, not claudine

Claudine must not grow a second interpolation engine. It already carries a
bespoke runtime interpolator (`lifecycle_executor.rs::interpolate` +
`LifecycleLookup`) that can drift from Darkmatter's compose semantics
(whole-value typing, mixed-string rules, function resolution, null propagation).
This fix routes the event-time pass **back through Darkmatter** and **retires**
claudine's bespoke path. Two new Darkmatter primitives make that possible.

## Proposed Design

### DM1 — Darkmatter: exclude keys from main compose

Add a compose option that names top-level frontmatter keys to defer from the
compose-time value-resolution passes (`{{ }}` interpolation, whole-value
expansion, `$(...)` shell expansion, and schema value interpolation). Deferred
keys survive in `effective_frontmatter` with their authored `{{ }}` / structure
intact; their *types and shape* are preserved (only resolution is skipped).
Default empty -> no behavior change for any other caller.

Claudine passes the seven lifecycle event keys
(`initialize`/`start`/`success`/`blocked`/`failure`/`finalize`/`loop`).

Darkmatter must keep deferred-key behavior explicit in the compose report so
callers can distinguish "raw because intentionally deferred" from "raw because
composition failed". Claudine uses that metadata for dry-run labeling (C5) and
for diagnostics that point at lifecycle binding time.

### DM1a — Darkmatter: forbid composed keys from reading deferred keys

A compose-time key must not read a deferred lifecycle key with `{{ failure }}`,
`{{ failure.message }}`, or `{{ doc.failure.message }}`. That would inject a raw
lifecycle subtree into an early-bound value and make the result depend on a
binding-time accident. Darkmatter should reject this during dependency analysis
with a typed error naming both the composed key and the deferred key.

Decision: reject rather than warn. A warning would still produce a usable value
with raw lifecycle syntax, which is the class of bug this fix is removing.

### DM1b — Schema validation ordering

User `$schema` validation remains a compose/prepare-time contract for ordinary
frontmatter. Lifecycle event keys are Claudine control keys, not user data, and
they need event-time interpolation. Therefore:

- Darkmatter still validates ordinary non-deferred frontmatter exactly as today.
- Deferred lifecycle keys are excluded from user schema value validation and are
  validated by Claudine's lifecycle parser/guards instead.
- If a caller other than Claudine opts into deferred keys, that caller owns the
  equivalent subtree validation for those keys.

This avoids requiring user schemas to accept raw `{{err.msg}}` lifecycle strings
while preserving the existing schema contract for prompt inputs.

### DM2 — Darkmatter: subtree compose with an injectable lookup

Expose a public entry point that interpolates a **frontmatter subtree** (a JSON
value) using the *same* interpolation core as main compose, driven by a
caller-supplied lookup:

- The lookup layers caller-injected globals over Darkmatter's own seed state
  (`ctx`/`env`/`doc` + read-side functions). Darkmatter owns parsing and
  interpolation; the caller owns only the extra variable *data*.
- Injected globals may be **eager** (`err`, `timing`) or **lazy** (`current`,
  evaluated on first access exactly as `ctx` is at compose time). The API takes
  arbitrary named globals, not three hardcoded names, so future late-binding
  globals need no further Darkmatter change.
- The subtree compose applies identical whole-value-typing and mixed-string
  *resolution* rules, so a lifecycle string interpolated at event-time produces
  the same typed/substituted result as the same string with the same data at
  compose-time. Strictness (below) is an **orthogonal** mode flag: it changes
  what happens on *failure* (typed error vs lenient empty), not how a successful
  resolution is typed or substituted.
- The API has an explicit strictness mode. Claudine uses strict mode for
  lifecycle communication/action text so parse failures, fatal evaluation
  failures, and unresolved roots become typed errors before any side effect is
  dispatched. Lenient behavior remains available for Darkmatter's existing body
  and mixed-string use cases.

### C1 — Claudine: parse the lifecycle config from the excluded (raw) subtree

With DM1 excluding the lifecycle keys, `effective_frontmatter` holds them raw.
`parse_lifecycle_config` reads from there, so every lifecycle string keeps its
`{{ }}` spans. Non-lifecycle keys are composed as today, so variable *values*
(`phase`, `pass_icon`, ...) are composed before launch, then may still be updated
by lifecycle/loop side effects during the run. Event-time lookup always reads
from the current effective document state.

### C2 — Claudine: interpolate each lifecycle property at event-time via DM2

When an event fires, claudine calls DM2 on that event's property subtree with a
lookup layering `err`/`timing`/`current` over the composed document state. This
covers **top-level communication fields and stack action bodies uniformly** —
the top-level/stack distinction disappears, and `failure.message: "{{err.msg}}"`
resolves at failure-time. Claudine's bespoke `interpolate`/`LifecycleLookup`
runtime interpolation path is removed; the lifecycle context data shrinks to the
injected-globals layer handed to DM2 plus the current document state.

Dispatch uses the resolved event subtree only for the event currently firing.
The raw deferred subtree remains the stored lifecycle definition so later events
and later loop iterations re-resolve against their own event-time state.

**Resolution granularity is just-in-time, not a single snapshot.** Each lifecycle
property/action string is resolved via DM2 immediately before it is used, against
the live effective document state at that instant — not once when the event
fires. This is required so a `set_frontmatter` side-effect run by stack action #1
is visible to action #2 in the same event's stack, and so `current`/`timing` read
the state at the point of use. (Resolving the whole event subtree up front would
miss mid-stack mutations.)

### C3 — `shell(...)` is the early-binding exception

Shell commands are inside the excluded subtree, so they are unresolved after main
compose. Pre-flight resolves each one via **DM2 with an early-binding-only
lookup** (no `err`/`timing`/`current`), stamps the resolved bytes back so the
approved command equals the executed command, and **rejects** any late-binding
reference in a shell command with a typed error. This applies to both short-form
`shell(...)` and long-form `shell` actions with `command:`. One mechanism (DM2),
two lookups: early-binding for shell/pre-flight, full for events.

### C4 — Guard rework

Because lifecycle strings stay raw through prepare, the static scans operate on
the authored `{{ }}` spans, which is what we want:

- **`err`-availability scan** (`LifecycleErrNotAvailable`): walk the `{{ }}`
  spans inside communication/action strings plus the whole `when:` expression;
  reject `err` in a no-error event (`initialize`/`start`/`success`/`loop`).
  `timing`/`current` are allowed everywhere; `doc.err` remains the escape hatch.
- **Undefined-variable scan:** treat `err`/`timing`/`current` and
  `ctx`/`env`/`doc` as known roots; flag only genuinely-unknown roots (typos).
  Preserve the existing fallback/ternary tolerance for intentionally optional
  values, but only inside the operands where that tolerance is already
  documented.
- **Interpolation-leak guard:** stop treating authored lifecycle strings as
  prepare-time leaks. They are deferred by design. After DM2 resolves an event
  subtree, run the leak guard on the resolved side-effect strings immediately
  before dispatch. A surviving `{{ }}` at that point is a typed error and the
  side effect is not sent.
- **Event-time resolution errors:** malformed expressions, unknown functions,
  unknown roots (typos / genuinely undefined variables), and late-binding globals
  used outside their legal event fail the event with a typed `CompositionError`.
  Lifecycle side effects must not silently render empty operational text for these
  cases.
- **Strict mode does not error on known-but-empty.** A reference whose root is a
  *known* surface — a declared frontmatter key, `ctx`/`env`/`doc`, or an
  in-scope late-binding global — that resolves to `null`/empty renders empty, as
  today. Strictness targets *unknown* roots and malformed/illegal expressions,
  not legitimately-absent values. Without this line, existing prompts that
  reference optional keys (e.g. `{{total_phases}}` or `{{spec_file}}` in
  `implement-plan.md`, both of which legitimately resolve to empty) would newly
  fail. **Migration note:** an author who wants an *unknown* optional name to be
  tolerated must opt in with explicit fallback syntax (`{{ maybe || '' }}`).

### C5 — Dry-run visibility

Excluded keys render raw (`{{err.msg}}` literally) in `effective_frontmatter` /
`--dry-run`. Label them as **interpolated at event-time** in dry-run output so a
raw span reads as intentional rather than as the bug that started this thread.

### C6 — Single-parameter argument grammar is unchanged

`literal-short-form-args` stays: `message(phase 6 is too large)` is literal,
`effect(crowd-applause)` is the literal name (no expression ambiguity),
`message(failed: {{err.msg}})` is literal + interpolation. This fix only supplies
the event-time, late-binding-aware interpolation that pairs with it.

### C7 — Documentation

- `claudine/docs/topics/lifecycle.md`: add **"Binding time: early vs late"** and
  **"When lifecycle properties interpolate"** (event-time rule, early/late split,
  the `shell` exception).
- `claudine/docs/topics/composition.md`: update the five-stage compose pipeline
  and lifecycle guard text so `effective_frontmatter` explains deferred lifecycle
  keys and the event-time second pass.
- `claudine/docs/topics/frontmatter-properties.md`: update lifecycle property
  descriptions to mention event-time interpolation and the `shell` exception.
- `claudine/features/2026-05-12-lifecycle/spec.md`: clarify line 67 (lifecycle
  property interpolation is event-time) and lines 96–101 (communication/action
  strings are literal-with-`{{ }}`; the `err` scan walks their interpolation
  spans; only `when:` is a whole expression).
- `.claude/skills/claudine/` timeline + `SKILL.md` stack-globals bullet.

## Edge Cases

- **Cross-reference into an excluded key.** A non-lifecycle key
  `summary: "{{ failure.message }}"` is rejected during compose as described in
  DM1a. Use a normal non-lifecycle key for shared prose, then reference that key
  from the lifecycle event.
- **`doc.<excluded-key>`.** Resolves to the raw subtree (rare); documented.
- **Loop currentness.** Re-composing the event subtree each iteration means
  `{{phase}}` reflects the live iteration — a correctness improvement, not a
  regression. Per-event subtree compose is tiny; cost is negligible.
- **Unresolvable span at event-time.** Fails closed for lifecycle side-effect
  surfaces. It never renders empty and never dispatches raw `{{ }}` text.
- **Top-level `loop` is mixed-purpose.** Only lifecycle concern keys inside
  `loop:` (`say`, `message`, `stack`, etc.) are deferred. Iteration controls
  (`while`, `until`, `actions`, `max`, `fail_fast`) keep their existing parsing
  and binding rules unless they contain lifecycle communication/action text.
  DM1 defers whole top-level keys, so the planning step must pick one of: (a)
  defer all of `loop:` and confirm the iteration controls are unaffected (likely
  safe — `while`/`until`/`actions` are evaluated by the loop engine, not by
  compose-time `{{ }}` interpolation, so deferring them is a no-op for resolution),
  or (b) extend DM1 to accept sub-paths (`loop.say`, `loop.stack`, …) if any
  iteration control turns out to depend on compose-time interpolation. Verify
  before implementing rather than assuming (a).
- **Effect names.** `effect: "{{effect_name}}"` and `effect({{effect_name}})`
  cannot be validated against the sound catalog at prepare time. Validate the
  raw literal at prepare time when no interpolation is present; otherwise
  validate the resolved effect name immediately before dispatch and report
  `LifecycleUnknownEffect` with the event property path.
- **Empty resolved communication fields.** Preserve the existing empty-string
  normalization after event-time interpolation. A field that resolves to empty
  is treated as absent, but only after successful interpolation; errors do not
  normalize away.

## Tests

### Darkmatter

- DM1: a key listed in the exclude set is left raw across all compose passes
  (`{{ }}`, whole-value, `$()`, schema-interpolation); a non-excluded key is
  unaffected; types of excluded values are preserved.
- DM1a: a composed key that references a deferred key through either a bare root
  or `doc.<key>` fails with a diagnostic naming both keys.
- DM1b: ordinary schema validation still runs for non-deferred keys while a
  deferred lifecycle key containing `{{err.msg}}` is not rejected by user schema
  value validation.
- DM2: a subtree composed with injected eager (`err`) and lazy (`current`)
  globals resolves them; layered seed state (`ctx`/`env`/`doc`/functions) still
  resolves; whole-value typing and mixed-string behavior match main compose.
- DM2 laziness: a lazy global is only evaluated when referenced.
- DM2 strictness: strict subtree compose returns typed errors for malformed
  spans, unknown functions, and unknown roots instead of returning a degraded
  string.

### Claudine

- **Top-level late binding:** `failure: message: "{{err.msg}}"` through the real
  pipeline, executed with an injected `err`, emits the real error.
- **Stack late binding:** `failure: stack: - action: message(❌️ {{err.msg}})`
  end-to-end (extends `emit_preflight_blocked_and_finalize_propagates_err_msg_into_blocked_stack`
  to go through composition, not raw JSON).
- **Mixed body:** `message(phase {{phase}} failed: {{err.msg}})` — both spans
  resolve at event-time; `phase` is the live value.
- **Loop currentness:** a lifecycle message's `{{phase}}` reflects the current
  iteration.
- **No-error events still reject `err`:** `start: message: "{{err.msg}}"` halts
  at parse time; `timing`/`current` are allowed.
- **Shell early-binding:** `shell(git fetch {{branch}})` resolves at pre-flight,
  approved == executed; `shell(rm {{err.msg}})` and long-form
  `command: "rm {{err.msg}}"` are rejected at prepare time.
- **No leak false-positive:** a deferred `{{err.msg}}` does not trip the
  prepare-time leak guard.
- **Known-but-empty renders empty:** a lifecycle message referencing a declared
  frontmatter key that resolves to empty (e.g. `{{spec_file}}`) renders empty and
  does **not** error, while a typo (`{{spec_fil}}`) fails closed.
- **Just-in-time resolution:** a stack whose action #1 runs `set_frontmatter` and
  whose action #2 references that key sees the mutated value in action #2.
- **Post-DM2 leak guard:** a malformed or nested event-time result that still
  contains `{{...}}` fails before any messenger/TTS/sound/stderr/stdout/notify
  side effect is dispatched.
- **Deferred effect validation:** `effect: "{{effect_name}}"` validates the
  resolved sound name at event-time and reports `LifecycleUnknownEffect` for an
  invalid result.
- **Schema compatibility:** a prompt with `$schema` and `failure.message:
  "{{err.msg}}"` still validates ordinary schema inputs and reaches lifecycle
  parsing.
- **Parity:** the bespoke claudine interpolator is gone; event-time rendering of
  a representative string matches what compose produces for the same string with
  the same data.

### Reproduction fixture

`prompts/implement-plan.md`'s `failure` reporting `{{err.msg}}` renders the
actual error on a real/simulated failure.

## Acceptance Criteria

1. `failure.message: "{{err.msg}}"` and a `failure` stack `message(… {{err.msg}})`
   both render the real error at runtime.
2. Lifecycle messages reflect event-time state (`timing`, `current`, live loop
   `phase`).
3. Referencing `err` in a no-error event still halts at parse time.
4. Shell pre-flight approval is byte-identical to execution; late-binding refs in
   shell commands are rejected.
5. Communication arguments remain literal-by-default with `{{ }}`; `when:`/loop
   conditions remain whole expressions.
6. Document body and ordinary frontmatter interpolation timing is unchanged.
7. Event-time interpolation goes through Darkmatter (DM2); claudine's bespoke
   runtime interpolator is removed.
8. DM1/DM2 are opt-in; default compose behavior for every other caller is
   unchanged.
9. Compose-time keys cannot accidentally consume deferred lifecycle subtrees.
10. User schema validation remains unchanged for ordinary prompt inputs and does
    not reject deferred lifecycle interpolation.
11. Event-time interpolation *failures* (unknown roots, malformed/illegal
    expressions, late-binding misuse) fail closed before lifecycle side effects
    dispatch; they do not silently render empty. A *known* surface that resolves
    to null/empty still renders empty, as today.
12. Docs updated (`lifecycle.md`, `composition.md`, `frontmatter-properties.md`,
    lifecycle spec lines 67 & 96–101, skill).
13. All existing claudine + darkmatter tests pass.

## Implementation Notes

Touch list (anticipated):

- `darkmatter/lib/src/markdown/compose/` — exclude-keys compose option (DM1);
  public subtree-compose entry with an injectable layered lookup supporting eager
  and lazy injected globals (DM2). Likely centered on
  `frontmatter_interpolation.rs` and the pipeline entry, plus `ComposeOptions`.
- `claudine/lib/src/composition/prepare.rs` — pass lifecycle keys to the exclude
  set; drive pre-flight shell resolution via DM2.
- `claudine/lib/src/composition/lifecycle_executor.rs` — interpolate every event
  property via DM2 at event-time (top-level + stack); delete the bespoke
  `interpolate`.
- `claudine/lib/src/composition/lifecycle_context.rs` — reduce `LifecycleLookup`
  to the injected-globals layer handed to DM2.
- `claudine/lib/src/composition/preflight.rs` — early-binding shell resolution +
  late-binding rejection + stamp resolved bytes (C3).
- `claudine/lib/src/composition/lifecycle.rs` — `err`/undefined scans over
  `{{ }}` spans; split the lifecycle leak guard into prepare-time deferred-span
  handling and post-DM2 dispatch-time enforcement; `LATE_BINDING_ROOTS` constant.
- Dry-run output — label excluded keys as event-time interpolated (C5).
- Docs — `docs/topics/lifecycle.md`, `docs/topics/composition.md`,
  `docs/topics/frontmatter-properties.md`,
  `features/2026-05-12-lifecycle/spec.md`,
  `.claude/skills/claudine/{SKILL.md,timeline.md}`.

**Relationship to prior work.** `literal-short-form-args` made single-parameter
arguments literal; this fix supplies the event-time, late-binding-aware
interpolation that pairs with it — and does so through Darkmatter so claudine
keeps a single, authoritative parsing/interpolation engine.
