---
ready: false
implemented: true
agent: codex/default
created: 2026-06-19T22:29:00
---

# Review 2: Runaway-Output Guards + Ctrl+C Hardening

## Findings

### High: Repo/frontmatter guard config still fails open

The spec requires unknown `scope` agents and invalid regexes to fail at config-load, not silently no-op (`spec.md:157`, `spec.md:206`, `spec.md:438`). The implementation still documents and implements best-effort fallback in the wrapper resolver:

- `claudine/cli/src/commands/wrap/runaway_guard.rs:12` says resolution is best-effort and unreadable config does not abort.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:56` defaults any user-config load error to built-ins.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:59` drops repo override load errors with `.ok().flatten()`.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:68` drops malformed frontmatter `exit_expressions`.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:95` catches compile failures and disables the whole compiled set.
- `claudine/cli/src/commands/wrap/runaway_guard.rs:150` explicitly drops malformed frontmatter `guard_settings`.

That means a repo or prompt can declare a safety rule with a typo and Claudine proceeds with weaker guards, while the user believes the rule is active. This is the same contract gap as review 1.

Verification level present: Level 1 validation coverage exists in `claudine/lib/src/runaway/config.rs`, but it does not cover the production resolver path that suppresses those errors. Required level: Level 1 is sufficient for config-load behavior, but it must assert the resolver/compose path fails closed for invalid repo and frontmatter declarations.

### High: Agent/model-scoped exit expressions miss provider-reported models

The spec says the CLI wiring point supplies the active provider and model for scope selection (`spec.md:96`) and exact `{agent/model}` scope matching is a supported v1 requirement (`spec.md:143`, `spec.md:438`). The implementation resolves and compiles the detector before streaming, using only CLI/env/frontmatter hints:

- Direct wrappers pass only `args.model` into guard resolution (`claudine/cli/src/commands/wrap/wrapper_exec.rs:60`).
- Composition attempts use only `MODEL` env overrides or the frontmatter `model` key (`claudine/cli/src/commands/wrap/harness_orch/attempt.rs:82`).
- `SessionStart` later updates the sink's cached model (`claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs:19`), but the already-compiled detector is not re-scoped.
- If no launch-time model hint exists, `resolve_runaway_guards` treats the model as `""`, so exact agent/model scopes never match (`claudine/cli/src/commands/wrap/runaway_guard.rs:82`).

A user rule scoped to the actual model emitted by the provider can therefore be inactive unless the same model was also provided in frontmatter or on the CLI. That violates the "correct scope" success criterion (`spec.md:671`).

Verification level present: Level 1 tests cover `scope_matches` and detector behavior once a pattern is compiled, but there is no test where `SessionStart { model }` activates an agent/model-scoped rule. Required level: Level 1 is enough for this wiring contract; add a sink/resolver test or process-level fake-provider test proving provider-reported model scope works.

### High: Ctrl+C user-key behavior is still below the required verification level

The spec requires Ctrl+C to terminate every spawn/wait path on Unix and Windows, including when a wall-clock timeout is configured (`spec.md:623`, `spec.md:684`). Under the review rubric, behavior of the form "when the user presses key X" requires Level 3 OS keyboard injection.

Current coverage is still process-signal or unit-level:

- `claudine/cli/tests/wrap_sigint.rs:89` sends SIGINT with `libc::kill(pid, SIGINT)`, not an OS keyboard event through a terminal emulator.
- I found no `level3_` tests or `RUN_LEVEL3`-gated claudine tests for Ctrl+C.
- The visible feedback strings exist in `claudine/cli/src/commands/wrap/exec/termination.rs:49`, but I found no Level 2 terminal capture asserting they render during a real terminal run.

Verification level present: Level 1/process-signal coverage on Unix and unit tests for the escalation ladder. Required level: Level 3 for actual Ctrl+C key behavior; Level 2 is appropriate for visible feedback rendering. Windows parity also needs Windows-host CI/manual validation evidence or a documented verification record.

## Notes

The detector seam is stronger than in review 1. Level 1 tests now cover OutputText/Reasoning scanning, tool-payload exclusion, repetition thresholding, per-turn volume reset, output suppression after a trip, and handler payload propagation.

Targeted checks run:

- `cargo nextest run --color=never -p claudine --test runaway_handler_payload` — passed 7/7.
- `cargo nextest run --color=never -p claudine-cli content_guard` — passed 8/8.
- `cargo nextest run --color=never -p claudine-cli 'test(/runaway|sigint|timeout_volume|capture_volume|wrap_sigint/)'` — no matching tests, nextest exited with "no tests to run".

## Recommendation

Do not mark this feature production-ready. Fix the fail-open resolver behavior, make agent/model scoping honor provider-reported models, and add the required L3/L2 verification for Ctrl+C behavior before closing the feature.
