---
created: 2026-06-27
status: draft
reviewed: false
review_iterations: 6
area: claudine
packages:
    - claudine
related_specs:
    - "@darkmatter/features/2026-06-27-file-property-rewrite/spec.md"
    - "@claudine/fixes/2026-06-27-path-resolution/plan.md"
---

# Late-Binding Lifecycle Evaluation Errors Must Surface and Halt

A lifecycle event's `when:` guard and action strings are bound **late** — they
are evaluated when the event *fires*, not at prepare time. When that late-time
evaluation **raises an error** (a `frontmatter()` that throws on a missing path,
an unknown root/typo under DM2 strict mode, a malformed `{{ … }}` span), a
**terminal-phase** event (`success`, `failure`, `finalize`, `loop`) **swallows
it silently**: nothing reaches the user, the pipeline does not stop, and the run
still reports clean success.

This is the second defect behind the original report. The `success` stack's
first guard

```yaml
when: "frontmatter(review_file,'ready') == true"
```

threw at event time (the read path did not resolve), so the whole `success`
event aborted before the not-ready branch could run — **and the abort itself was
invisible**. The run exited `0`, no error printed, no message sent. A user has
no way to tell a deliberately-skipped branch from a crashed one.

> **Scope note.** This is distinct from
> `@darkmatter/features/2026-06-27-file-property-rewrite/spec.md`. That feature
> removes *this* error's trigger (the read will resolve). This spec makes *any*
> late-binding evaluation error — present or future — loud and halting, so a
> swallowed lifecycle error can never again look like success.

## Current Behavior (verified)

- `when_matches` returns `Err` on an unresolvable guard with **no emit**
  (`claudine/lib/src/composition/lifecycle_executor.rs:640`). `execute_event`
  packages it as `LifecycleEventOutcome { action_error: Some(_) }`
  (`:589`).
- `routes_action_error_to_failure` is `true` **only** for
  `Initialize | Start | Blocked` (`claudine/lib/src/composition/lifecycle.rs:953`).
  Terminal-phase events return `false`, so the pipeline's `routes_to_failure`
  branch never fires for them and the `action_error` is dropped.
- The only logging for an errored *action* is a `tracing::warn!` in `run_action`
  (`lifecycle_executor.rs:666`), which is invisible without `RUST_LOG`/`--debug`
  per the repo's verbose-vs-debug split — and the `when:`-guard error path does
  not even reach it.

Net: a late-binding evaluation error on a terminal-phase event is neither
surfaced nor halting.

## Goals & Non-Goals

**Goals**

- A late-binding **evaluation/binding error** in *any* lifecycle event emits its
  message to **stderr immediately**, as styled user-facing output (not a
  `tracing` line gated behind `--debug`).
- Such an error **stops the pipeline**: the run does not report clean success.
  On a terminal-phase event (where the provider already ran) "stop" means the
  process exits non-zero and the error is the run's recorded outcome.
- **`finalize` can catch it**: `finalize` fires with the evaluation error exposed
  as the `err` late-binding global, so an author can react (notify, clean up).
- Apply uniformly to the surfaces that bind late: `when:` guards, action-string
  interpolation, and any expression evaluated at event time.

**Non-Goals (this fix)**

- **No change to action-dispatch tolerance.** An action that *evaluated* fine
  but whose *side effect* failed (a Discord/Slack send, a TTS call, a `shell`
  non-zero) keeps today's behavior, including the `no_error: true` escape hatch.
  This fix is about the **expression layer raising**, not a channel failing.
- **No change to setup-phase routing.** `initialize`/`start`/`blocked` already
  route to `failure`; this fix only adds the missing surfacing/halt for
  terminal-phase events and unifies the emit path.
- **No new authoring syntax.** This is runtime error surfacing, not a DSL change.
- **No change to what counts as a valid expression.** DM2 strict-mode semantics
  (unknown root fails closed, known-but-null renders empty) are unchanged; this
  only governs what happens to an error *once raised*.

## Key Decisions (proposed — for review)

- **Decision #1 — Distinguish evaluation errors from dispatch failures.** The
  halt/surface applies to errors originating in the expression layer
  (`when_matches`, `resolve_string_value`, `eval_expr`) — a guard or string that
  *threw*. A side-effect dispatch failure (channel/TTS/shell) remains a normal
  `action_error` subject to `no_error` and the existing per-phase policy. These
  are already separable: the guard error path is distinct from `run_action`.

- **Decision #2 — Emit to stderr at the point of error, once.** When an event's
  stack raises an evaluation error, render the message to stderr immediately
  through the same styled error surface the CLI uses for composition errors
  (red `Error:` treatment), before any further events fire. No double-emit if a
  later stage also reports the same error.

- **Decision #3 — Terminal-phase evaluation errors halt without firing
  `failure`.** The provider already ran (and may have genuinely succeeded), so a
  `success`-time guard error does **not** retroactively fire the `failure`
  event. Instead: surface → fire `finalize` (with `err`) → exit non-zero. This
  matches "the `finalize` lifecycle event can catch this" and avoids
  misreporting a provider success as a provider failure. *(Open question: should
  `failure` also fire? See below.)*

- **Decision #4 — `finalize` receives the error as `err`.** The evaluation error
  is wrapped as a `LifecycleErrorInfo` and threaded into the `finalize` event
  context (`with_error`), so `finalize.when`/actions can branch on
  `err.kind`/`err.msg`. `finalize` is the universal always-runs event and the
  natural catch point.

- **Decision #5 — Setup-phase behavior is preserved, surfacing unified.**
  `initialize`/`start`/`blocked` continue to route to `failure`, but their
  evaluation errors flow through the same stderr-surfacing helper so the user
  sees one consistent error presentation regardless of phase.

- **Decision #6 — Exit code reflects the halt.** A run that ends on a
  late-binding evaluation error returns a non-zero exit code, so non-interactive
  callers (CI, `sequence`, chained composition) observe the failure instead of a
  false `0`.

## Behavior Matrix (proposed)

| Event phase | Evaluation error in `when:`/action string | Side-effect dispatch failure |
|---|---|---|
| `initialize` / `start` / `blocked` | stderr + route to `failure` → `finalize` → non-zero (today, with surfacing added) | route to `failure` (today) |
| `success` / `failure` / `finalize` / `loop` | **stderr + `finalize` (with `err`) + non-zero** (new) | log + continue; `no_error` honored (today) |

## Testing Strategy (L1)

- A `success` stack whose first `when:` raises (e.g. `frontmatter()` on a missing
  path) emits the error to stderr, fires `finalize` with `err` populated, and
  yields a non-zero run outcome — not a silent success.
- A `success` stack whose `when:` evaluates cleanly to `false` still just skips
  the item (no error, no halt) — the falsy/skip path is unchanged.
- A terminal-phase **action-dispatch** failure (mock channel error) without
  `no_error` keeps today's log-and-continue outcome; with `no_error: true` it is
  suppressed — neither is escalated by this change.
- An evaluation error in `finalize` itself surfaces to stderr and sets a non-zero
  outcome without infinite re-entry (it does not re-fire `finalize`).
- Setup-phase evaluation error still routes to `failure` → `finalize`, now with
  the unified stderr surfacing asserted.
- Exit-code assertion: a late-binding evaluation error produces a non-zero
  process exit in a non-interactive run.

## Risks

- **Revises a deliberate prior decision.** The current "terminal-phase events log
  but leave the outcome unchanged" was intentional so a flaky *dispatch* on
  success wouldn't fail an otherwise-good run. Decision #1 preserves that for
  dispatch failures and only escalates genuine *evaluation* errors — the
  narrowing must be precise so a flaky Discord send is never treated as a halt.
- **`finalize` error re-entry.** A late-binding error raised *inside* `finalize`
  must not recursively fire `finalize`. The handler needs a guard against
  re-entry (Decision per the `finalize` test above).
- **Exit-code churn for existing prompts.** Prompts that today "succeed" while
  silently swallowing a terminal-phase evaluation error will start exiting
  non-zero. That is the intended correction, but it is a behavior change for any
  prompt currently relying (unknowingly) on the silent swallow.

## Open Questions

- Should a terminal-phase evaluation error also fire the **`failure`** event, or
  only `finalize`? Leaning `finalize`-only (Decision #3) because the provider
  itself did not fail; `failure` semantics ("the run failed to complete") would
  misrepresent a clean provider run. Worth confirming against how authors expect
  to catch these.
- Should the surfaced stderr message include the originating event name and the
  offending expression text (e.g. ``success.when: frontmatter(review_file,
  'ready') == true``) to make the diagnosis self-explanatory? Leaning yes — it
  turns a silent failure into an actionable one.
