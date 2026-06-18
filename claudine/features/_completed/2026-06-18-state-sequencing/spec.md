---
created: 2026-06-18
reviewed: false
status: draft
area:
  - claudine
  - claudine-cli
---

# Loop State Sequencing — Resolve Frontmatter Before the Loop Mutates It

## Problem

Running a `loop:` composition whose control variables are defined as
frontmatter **expressions** fails the instant the loop tries to mutate them.

Repro (the command that triggered this spec):

```sh
claudine compose prompts/implement-plan.md \
  plan=features/2026-06-09-improved-descriptions/plan.md \
  -y --opencode --model kimi-for-coding/k2p7
```

The first iteration composes and executes correctly: the prompt body renders
`phase = 1` (the `|| 1` default) and `total_phases = 6` (read from the plan's
frontmatter). Then the loop's post-iteration action fires and dies:

```
CompositionError: composition failed

  invalid increment at iteration 1, action 1 of 1: property `phase` has type string
```

The prompt's relevant frontmatter:

```yaml
phase: "{{ frontmatter(plan, 'start_phase') || 1 }}"
total_phases: "{{ frontmatter(plan, 'total_phases') || frontmatter(plan, 'phases') }}"
loop:
    until: "phase > total_phases"
    action: "increment(phase)"
```

The error message is genuinely unhelpful — "has type string" gives no hint that
the *actual* value of `phase` at action time is the literal, unevaluated
template text `"{{ frontmatter(plan, 'start_phase') || 1 }}"`. But the message
is a symptom. The defect is architectural: **the loop engine mutates and tests
raw, unresolved frontmatter, not the resolved state the rest of composition
sees.**

## Root cause (verified against current code)

The loop maintains its own mutable frontmatter map. It is seeded from the **raw
document frontmatter** — template strings and all — and never from the resolved
(composed) frontmatter.

1. **Seed is raw.** `run_loop_with_overrides`
   (`claudine/cli/src/commands/compose.rs:1359`) and the library's
   `execute_loop` (`claudine/lib/src/composition/loop_engine.rs:232`) both build
   `initial_frontmatter` directly from `source.markdown.frontmatter().as_map()`.
   CLI `key=value` setters are merged on top, but those are raw user strings too.
   So the loop's `phase` starts life as the string
   `"{{ frontmatter(plan, 'start_phase') || 1 }}"`.

2. **The condition reads raw values, not resolved ones.**
   `LoopExpressionLookup::get` (`loop_expression.rs:91`) returns frontmatter
   values **verbatim** — it does not re-render `{{ ... }}`. So
   `until: "phase > total_phases"` compares two unevaluated template strings.
   Comparing two such strings with `>` happens to be falsy, so `until` decides
   "keep going" and iteration 1 runs. The loop looked healthy purely by
   accident — the condition was never actually evaluated against `1` and `6`.

3. **The action mutates raw values and blows up.** After iteration 1,
   `apply_actions` → `apply_increment_with_context`
   (`loop_actions.rs:251`) reads `phase`, finds the non-numeric template string,
   and `increment_value` (`loop_actions.rs:293`) returns `None` because the
   string does not parse as a number → `InvalidIncrementType { found: "string" }`.

4. **The resolved state already exists — it is just thrown away.** Each
   iteration calls `prepare_direct_with_schema`, which composes the document and
   produces `PreparedComposition.effective_frontmatter`
   (`prepare.rs:141`) — the fully resolved frontmatter where `phase = 1` and
   `total_phases = 6` as typed JSON. The body renders from this. But the loop
   engine never reads it back; the loop's own state stays raw forever.

5. **Even if the increment had not crashed, the body would be wrong.**
   Darkmatter's `set_overrides` *unconditionally overwrite* frontmatter keys
   before interpolation (`darkmatter/.../compose/util.rs:203`). The per-iteration
   overrides are `ctx.as_set_overrides()` — i.e. the loop's *raw* state. So each
   iteration overwrites the document's `phase` with the template string and
   re-resolves it back to `1`. A working `increment` would bump the loop's
   internal counter while the rendered body stayed pinned at `phase 1` every
   pass. The raw-state design is doubly broken; the crash just surfaces first.

This matches the user's pointer: *when the loop mutates state it must operate on
the fully resolved state.* It does not today.

## Goals

- The loop's mutable state holds **resolved, typed** values for the variables it
  reads (in `until`/`while`) and writes (`increment`/`decrement`/`set`/…). A
  numeric expression like `phase: "{{ ... || 1 }}"` is `1` (a JSON number) by the
  time the condition or an action touches it.
- `increment(phase)` succeeds on the repro above and the loop advances
  `phase` 1 → 2 → … → 7, stopping when `phase > total_phases`.
- The rendered body of iteration *N* reflects the mutated control value
  (`phase = N`), not a value re-pinned to the seed.
- Frontmatter that is *derived per iteration* (e.g.
  `pass_icon: "{{ _loop_is_last ? '✅' : '🧑‍💻' }}"`, lifecycle `message` /
  `say` strings) keeps re-resolving each iteration against current state and
  ambients — it must **not** be frozen at its seed-iteration value.
- When a control variable still cannot be coerced (a genuinely non-numeric
  value), the error names the offending value and explains the resolution stage,
  rather than the bare "has type string".

## Non-Goals

- Redesigning the expression language, the ambient `_loop_*` namespace, or the
  `until`/`while` semantics.
- Changing `set_overrides` precedence in Darkmatter compose.
- Touching `inline-compose` or the non-loop `compose`/`sequence` paths beyond
  what shared seeding requires.
- Rate-limit, timeout, or interrupt behavior.

## The design tension (must be resolved, not glossed)

Two classes of frontmatter coexist in a looping document and they want opposite
treatment:

| Class | Examples | Desired lifetime |
|-------|----------|------------------|
| **Control / state** | `phase`, `total_phases` | Resolved **once** at seed; thereafter **owned and mutated** by the loop; supplied as overrides so the body sees the live value. |
| **Derived / presentation** | `pass_icon`, `start.message`, `success.say` | Re-resolved **every** iteration from the document against current state + ambients. Never frozen. |

If we naively resolve *all* frontmatter once and feed the whole resolved map
back as per-iteration overrides, presentation variables freeze (e.g. `pass_icon`
sticks on `🧑‍💻` and never flips to `✅` on the last pass). If we resolve
*nothing* (today's behavior), control variables are raw template strings and the
loop crashes. The fix must treat the two classes differently.

### Proposed model: the loop owns only its control variables

1. **Identify control variables.** The set is the union of:
   - every property named as a `loop.action` target (`increment(phase)` →
     `phase`), and
   - every bare identifier referenced in the `loop.until` / `loop.while`
     expression (`phase`, `total_phases`).

   This is statically derivable from the already-parsed `LoopConfig` plus a
   light identifier walk of the condition expression (the expression parser
   already exists in Darkmatter; reuse it rather than regexing).

2. **Resolve the seed once.** Run a single compose pass at loop start (with the
   CLI setters and iteration-0 ambients) and read the resolved values of the
   control variables out of `effective_frontmatter`. These typed values become
   the loop's initial state. `phase → 1`, `total_phases → 6`.

3. **Each iteration:**
   - Build per-iteration overrides = CLI setters **+ the loop's current control
     state** (typed) **+ ambient `_loop_*`**. Derived variables are *not* in this
     set, so the document's own templates for them re-resolve normally.
   - Compose + execute (body sees `phase = N`).
   - Evaluate the condition against the loop's typed control state.
   - Apply actions to the typed control state (`increment(phase)` → `N+1`).

This keeps presentation variables live while giving the loop a numeric `phase`
it can actually increment, and propagating the mutated value into the body via
the override.

### Alternative considered (and why not, yet)

*Re-sync the entire loop state from `effective_frontmatter` each iteration, then
re-overlay mutated keys.* Simpler to wire (the resolved map is right there) but
it inverts the precedence problem: derived keys would be correct, but every
control key would be re-pinned to its seed expression unless explicitly
re-overlaid — i.e. it collapses into "track which keys the loop owns" anyway.
The control-variable model above makes that ownership explicit instead of
implicit. Flagged as an open question below in case implementation reveals the
re-sync form is cheaper.

## Section 1 — Resolve the loop seed

- Add a seed-resolution step shared by `execute_loop` (lib) and
  `run_loop_with_overrides` (CLI) so both entrypoints behave identically. The
  library is the source of truth; the CLI wrapper should not re-implement
  seeding.
- The seed compose pass must use the same `PrepareOptions` (env overrides,
  shell working directory, source repo root, CLI setters) as iteration 1, so the
  resolved seed matches what iteration 1 would compute.
- Only the **control variables** are lifted from the resolved seed into loop
  state. All other keys remain whatever the per-iteration compose produces.

## Section 2 — Make the condition and actions operate on resolved state

- The loop's working frontmatter passed to `LoopExpressionLookup` and
  `ActionStaging` must be the resolved control state, not the raw map.
- No change to `LoopExpressionLookup::get`'s verbatim-read contract is required
  *if* the values it reads are already resolved. (Re-rendering templates inside
  the lookup is explicitly rejected: it would re-pin mutated control values to
  their seed expression every iteration — the same bug in a new place.)

## Section 3 — Propagate mutated control values into the body

- Per-iteration overrides must carry the loop's current typed control values so
  Darkmatter's unconditional `set_overrides` overwrite renders the body with
  `phase = N`.
- Confirm with a test that iteration *N*'s composed body contains the mutated
  value (`Implement Phase N of 6`) and that `pass_icon` still flips to the
  last-iteration glyph on the final pass.

## Section 4 — Honest error when coercion genuinely fails

Even after resolution, a user can target a non-numeric control variable (e.g.
`increment(area)` where `area` is a string). The error must be diagnosable:

- When an increment/decrement target holds a value that still looks like an
  unresolved template (matches `{{ ... }}`), say so explicitly — name the raw
  text and state that the loop seed failed to resolve it. This is a
  defense-in-depth message; reaching it after this fix indicates a seed-resolution
  gap, not user error.
- When the target is a resolved-but-non-numeric value, keep the type name but
  add the offending value (truncated) so the author can see *what* `phase` was.
- These messages flow through the existing claudine composition-error formatting
  contract (styled, deduplicated, no raw chains) — do not bypass it.

## Phasing

1. **Phase 1 — Library seed + control-variable identification.** Add control
   variable extraction from `LoopConfig` + condition expression; add the
   seed-resolution step to the library loop engine; switch loop state to typed
   resolved values. Unit tests at the engine level using a fake executor that
   returns resolved `effective_frontmatter`.
2. **Phase 2 — CLI wiring.** Route `run_loop_with_overrides` through the shared
   seeding; ensure per-iteration overrides carry the typed control state;
   preserve the launch-CWD restoration already in place.
3. **Phase 3 — Error messages.** Upgrade `InvalidIncrementType` /
   `InvalidDecrementType` (and the unresolved-template detection) per Section 4.
4. **Phase 4 — Integration test.** A `compose` loop fixture mirroring
   `implement-plan.md` (expression-defined `phase`/`total_phases`, `increment`
   action, `until` condition) that runs to completion with a stub executor and
   asserts: increment advances, body reflects `phase = N`, derived `pass_icon`
   stays live, loop stops at the right iteration.

## Open questions

1. **Control-variable extraction precision.** Is a bare-identifier walk of the
   condition expression sufficient, or do we also need identifiers that appear
   only inside action *values* (e.g. `set(next, "{{ phase + 1 }}")`)? Leaning:
   include identifiers referenced by action value templates too, so any variable
   the loop reads is resolved.
2. **Nested / dotted control variables.** Can a control variable be a dotted
   path (`state.phase`)? `increment` currently targets top-level keys only.
   Decide whether to support dotted control variables now or explicitly reject
   them with a clear error.
3. **Seed cost.** Seeding adds one compose pass before iteration 1. Is that pass
   redundant with iteration 1's own prepare, and can the resolved seed be reused
   as iteration 1's composition to avoid double work?
4. **Re-sync vs. owned-keys (see Alternative).** If implementation shows the
   owned-keys bookkeeping is heavier than re-syncing from `effective_frontmatter`
   with an explicit mutated-key overlay, revisit.
5. **`total_phases` as a constant.** It is read but never mutated. Treating it as
   a control variable (resolved once, overridden each iteration) is harmless, but
   confirm there is no document where a "control" variable is *intended* to
   re-derive each iteration.

## Success criteria

- The repro command runs the loop to completion: `phase` advances 1 → 7 and the
  loop halts on `phase > total_phases`, with no `InvalidIncrementType`.
- Iteration *N*'s composed body shows `Implement Phase N of 6`.
- `pass_icon` resolves to the working glyph on iterations 1..6 and the
  last-iteration glyph on iteration 6 (the final pass) — proving derived
  variables stay live.
- A control variable that genuinely cannot be coerced produces an error that
  names the value and the resolution stage, not the bare "has type string".
- Library and CLI loop entrypoints share one seeding path; no behavioral drift
  between `execute_loop` and `run_loop_with_overrides`.
- All existing `loop_engine` / `loop_actions` / `loop_expression` tests still
  pass; new tests cover seed resolution and the derived-vs-control split.
