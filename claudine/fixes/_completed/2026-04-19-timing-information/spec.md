---
fixed: 2026-04-19
agent: claude
---

# Timing Information

## Problem

The following output that claudine produces has issues:

```sh
 ← Task(successful, Commit prompts docs)
 ← Task(successful, Commit messenger refactor)
 180s · 10 done
 210s · 10 done
 240s · 10 done
󰀨 no provider activity in 2m — provider may be hung; press Ctrl+C to abort
 270s · 10 done
 300s · 10 done
```

- first off is that we're reporting how many of _something_ are done
    - i happen to know that this how many tool calls came back
    - but the question is ... who cares?
    - it's not really a meaningful metric
- then we need to talk about the `󰀨 no provider activity in 2m — provider may be hung; press Ctrl+C to abort` message:
    - 2 minutes is not enough time to make any assumptions
    - that is particularly true in project with compiled languages that have a build step that can take a good amount of time
- the last big issue is that we're providing how many seconds "OF WHAT?"
    - i'm still not sure but I think it must be a wall clock measurement?
    - this is not helpful to report like this

## Informational

A user can leverage the `timeout` and `step_timeout` frontmatter properties which:

- `timeout` - returns a timeout error based on wall clock time of the prompt
- `step_timeout` - returns a timeout error based on the amount of time

There isn't currently any way to "warn" a user after a certain amount of time but we'll add that in the next section.

## Solution

We will report timing information under these circumstances:

1. Periodic prompt-scoped timing header:

    - **Cadence / anchor:** Ticks are anchored on the prompt's start time and emitted at monotonic offsets from `t=0` (i.e. t=0, t=10m, t=20m, t=30m, …). Ticks are NOT aligned to wall-clock `:00 :10 :20` boundaries.
    - **`t=0` emission (header, no duration):** Emitted once when the prompt begins. Same format as subsequent ticks but with the duration segment (and the trailing "for") omitted so the line is grammatical without a duration phrase:
        - `⏱️ {hh}:{mm}{tz} <i><dim>running the <blue>{prompt}</blue> prompt</dim></i>`
    - **Subsequent ticks (`t=10m`, `t=20m`, `t=30m`, …):** Full line with duration:
        - `⏱️ {hh}:{mm}{tz} <i><dim>running the <blue>{prompt}</blue> prompt for</dim></i> {duration}`
    - **Duration formatting:**
        - measured in minutes up to 59 minutes, then
        - measured in hours and minutes
    - **Duration rendering rules.** These rules apply uniformly to every user-visible `{duration}` introduced by this feature — the periodic header's trailing `{duration}`, AND every `{duration}` occurrence inside `timeout_warn` and `step_timeout_warn` messages (including both occurrences in the "with timeout also set" variants):
        - Under 60 seconds: `Ns` (e.g. `45s`).
        - 1–59 minutes: `Nm` (e.g. `12m`).
        - 60 minutes or more: `Hh Mm` with a single space between (e.g. `1h 30m`, `2h 5m`). Do not pad to two digits; do not use a colon-separated form.
    - **Timezone:** `{hh}:{mm}{tz}` uses local time with a `{tz}` suffix (e.g. `14:13 PDT`). Local time is used everywhere `{hh}:{mm}` or `{HH}:{MM}` appears in any user-visible timing string (header ticks and both `*_warn` messages) so all absolute times agree.

2. We will introduce two more Frontmatter properties with special meaning:
    - `timeout_warn` — prompt-scoped; fires when the prompt has been running for `timeout_warn` total wall-clock time.
    - `step_timeout_warn` — step-scoped; fires when the provider has produced no events for `step_timeout_warn` of silence. This reuses the existing inactivity clock behind `step_timeout` / `detect_step_timeout` on `LiveMetrics`; no new timer is introduced.

    These two new properties behave similarly to `timeout` and `step_timeout` but rather than returning an error they report a warning when the threshold is first crossed.

    **Frontmatter value type.** `timeout_warn` and `step_timeout_warn` use the **identical** frontmatter value type, parser, and coercion rules as the existing `timeout` / `step_timeout` properties (defined in `claudine/lib/src/composition/types.rs`). Whatever surface those accept today — humantime strings, integer seconds, or whatever the `Deserialize` impl yields — the two new `*_warn` properties accept the same. Users should not have to learn two notations.

    **Fire-once semantics:** Each `*_warn` fires exactly **once** when its threshold is first crossed during a given prompt/step. It does NOT re-emit on subsequent 10-minute periodic ticks, and the step-scoped warning does NOT re-arm within the same stall episode. This matches the "one warning per stall episode" discipline of the prior stall detector.

    **Ordering when both cross in the same cycle:** If `step_timeout_warn` and `timeout_warn` both cross in the same scheduling cycle, emit the step-scoped warning first, then the prompt-scoped warning, each as a separate `Status` WARN line.

    **Messages:**

    - `timeout_warn` emits via `Status` in WARN state:
        - if `timeout` is also set:
            - `the {prompt} has been running for {duration}, this is longer than we'd expect it to take but we won't timeout this prompt until we reach {HH}:{MM} in {duration}.`
        - if `timeout` is not set:
            - `the {prompt} has been running for {duration}, this is longer than we'd expect it to take. Press CTRL+C to terminate this prompt if you're convinced that the prompt has hung.`
    - `step_timeout_warn` emits via `Status` in WARN state. The duration reported is silence since the last provider event (the same "step" semantic as `step_timeout`):
        - if `step_timeout` is also set:
            - `the {prompt} has not produced output for {duration}, this is longer than we'd expect, but we won't abort this step until we reach {HH}:{MM} in {duration}.`
        - if `step_timeout` is not set:
            - `the {prompt} has not produced output for {duration}, this is longer than we'd expect. Press CTRL+C to terminate this prompt if you're convinced that the prompt has hung.`

    All references to `{prompt}` in every message above (including both `step_timeout_warn` variants) are rendered as an OSC8 link to the prompt file being used:
    - the displayed text is blue and is a relative path to the prompt (relative from repo root, or CWD, or the user's HOME directory if not in a repo)
    - the OSC8 link target is an absolute path to the file (as is always the case in OSC8 links)

    **Two `{duration}` occurrences in the "with timeout also set" variants.** Both `timeout_warn` and `step_timeout_warn` "with timeout also set" messages contain `{duration}` twice. The meanings are:
    - First `{duration}` — elapsed wall-clock time since prompt start (for `timeout_warn`) or silence duration since the last provider event (for `step_timeout_warn`).
    - Second `{duration}` — **remaining time** from now until the hard timeout deadline (`{HH}:{MM}`); that is, the difference between the hard-timeout deadline and the current time.

    Both occurrences are formatted using the same duration rendering rules defined in Solution §1 (`Ns` / `Nm` / `Hh Mm`).

3. **Preflight validation (hard errors):**

    Validated at the same preflight stage that already rejects the `--timeout` / `--interactive` conflict in `claudine/cli/src/commands/wrap/composition.rs`. Each failure must reject the run with a clear error message that names both offending values.

    - `timeout_warn >= timeout` → reject.
    - `step_timeout_warn >= step_timeout` → reject.
    - Any `*_warn` value `<= 0` → reject.
    - `timeout_warn` set with no `timeout` → legal (the "no `timeout`" message variant applies).
    - `step_timeout_warn` set with no `step_timeout` → legal (the "no `step_timeout`" message variant applies).

    **Error surface.** These preflight rejections use the same error channel, formatting style, and exit code as the existing `--timeout` / `--interactive` conflict rejection in `claudine/cli/src/commands/wrap/composition.rs`. The implementer should follow the local precedent at that site (matching its `Status` ERROR / `color_eyre` / `bail!` idiom, whichever it uses today); exact wording is not prescribed here.

4. **Removed behavior:**

    The new timing surface fully replaces the existing heartbeat and stall-threshold stderr warning. The following are deleted as part of this work:

    - `emit_progress_heartbeat` — the `240s · 10 done` style lines. Deleted.
    - The stall-threshold stderr warning `󰀨 no provider activity in 2m — provider may be hung; press Ctrl+C to abort`. Deleted.
    - `stall_threshold_from_env` and any associated env-var wiring. Deleted.

    **Preserved:**

    - `flush_if_idle` is kept for correctness (it prevents dangling assistant paragraphs from being masked). Its current tuning is retained unchanged:
        - Driven by a dedicated, independent ticker — separate from both the new 10-minute cadence loop and the deleted heartbeat loop.
        - The ticker fires every **30 seconds** and calls `flush_if_idle(Duration::from_secs(30))` on `StreamTextRenderer` (30-second silence window).
        - No new env-var tuning knob is introduced at this time. Only the driver changes (it is no longer the deleted heartbeat thread); cadence and silence window are preserved.
    - The stall detector's underlying observation signal MAY still feed tracing / `--debug` output, but MUST NOT emit to stderr.

    After this change, **all** user-visible stderr timing output comes from exactly two producers:

    1. The periodic prompt-scoped header (t=0 and every 10 minutes thereafter).
    2. The two `*_warn` Status WARN lines (`step_timeout_warn` and `timeout_warn`), each fire-once.

5. **Tracing coverage:**

    Every new user-visible timing emission also emits a matching structured tracing event for downstream `--debug` capture:

    - The t=0 header emission → `tracing::info!`.
    - Each subsequent 10-minute periodic tick → `tracing::info!`.
    - The `timeout_warn` Status WARN emission → `tracing::info!`.
    - The `step_timeout_warn` Status WARN emission → `tracing::info!`.
    - Each preflight rejection (from Solution §3) → `tracing::error!`, emitted adjacent to the user-facing error.

    Structured fields on these events include at minimum: absolute prompt path, elapsed duration, remaining duration (where applicable — i.e. the "time until hard timeout" used by the `*_warn` "with timeout also set" variants), and the threshold value that triggered the event. Follow the same tracing idiom used by adjacent emissions in `exec.rs` and `composition.rs` today.
