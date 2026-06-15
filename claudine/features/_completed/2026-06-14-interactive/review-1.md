---
ready: false
agent: codex
model: ""
---

# Review 1 — Frontmatter-Driven Interactive Sessions

## Findings

### High — Resolved interactive mode does not reject `--step-timeout`

The spec requires timeout conflict validation to run against the resolved session mode, with the diagnostic explaining whether interactivity came from `--interactive`, `--no-interactive`, frontmatter, or default. The implementation only enforces the resolved-mode guard for `--timeout` in `claudine/cli/src/commands/wrap/composition/mod.rs:1532`; it does not check `request.step_timeout`.

The early command guards in `claudine/cli/src/commands/compose.rs:394` and `claudine/cli/src/commands/compose.rs:786` only check `shared.interactive`, so `interactive: true` frontmatter plus `--step-timeout 30s` passes those guards. The executor then only checks `request.timeout.is_some() && request.session_interactive`, so the run can proceed as an interactive provider session with a step timeout even though `--step-timeout` is documented as only valid in non-interactive mode.

This also misses the spec's "last point where both the resolved session mode and resolved timeout plan are known" requirement for timeout values that come from effective frontmatter or environment defaults. Harness frontmatter timeouts are parsed later in `claudine/cli/src/commands/wrap/composition/mod.rs:1694`, after the current guard, and the non-harness timeout plan is resolved later at `claudine/cli/src/commands/wrap/composition/mod.rs:2018`.

Required fix: validate both wall-clock `timeout` and explicit/effective `step_timeout` against `request.session_interactive` after timeout resolution has considered CLI, effective frontmatter, and env sources. Add command-level regression tests for `interactive: true` + `--step-timeout`, `interactive: true` + effective frontmatter timeout, and `--no-interactive` overriding `interactive: true` with timeouts allowed.

### High — Inline schema collection invariant lacks the required verification level

The spec makes schema-required collection independent of session mode for both `compose` and `inline-compose`, and requires collection to complete before any provider session starts. The new Level 2 PTY tests in `claudine/cli/tests/level2_schema_prompt_pty.rs:600` cover `compose -i`, `compose` with `interactive: true`, and `compose --no-interactive` overriding frontmatter. They do not cover `inline-compose`.

`inline-compose` has a distinct preparation path (`prepare_inline_with_schema`) and an additional interactive closure rejection path for providers that cannot capture the final assistant message. That makes the ordering requirement user-observable and command-specific: missing schema values must be collected before the inline interactive unsupported error or provider launch path is reached.

Verification level: strongest present coverage for inline is Level 1/unit and non-PTY CLI coverage around schema and unsupported interactive behavior. The required behavior is a real TTY prompt ordering guarantee, so this needs Level 2 PTY coverage, matching the compose tests. Until that exists, this requirement is not production-ready under the review rubric.

Required fix: add Level 2 PTY tests for `inline-compose -i` and `inline-compose` with `interactive: true` frontmatter where a required schema value is missing. The test should assert the prompt appears, the value can be submitted, and no provider launch or inline unsupported diagnostic occurs before collection completes.

### Medium — Sequence rejection is only unit-tested, not verified through the CLI

The spec requires `sequence` to hard-reject authored `interactive: true` frontmatter and to explain that dialog-shaped prompts should use `compose` or `inline-compose`. The implementation adds `reject_sequence_interactive` and unit tests in `claudine/cli/src/commands/sequence.rs:185`, but I do not see an integration test that runs `claudine sequence <file>` and verifies the rendered CLI error surface and that no provider step launches.

Verification level: Level 1 CLI integration is sufficient here because this is a non-TTY command error, but the current coverage is below that.

Required fix: add a CLI integration test with a sequence document containing `interactive: true`, a provider stub that records launches, and assertions on the typed error text plus no stub invocation.

## Readiness

Not ready for production. The `--step-timeout` conflict bug violates the resolved precedence contract, and the schema collection invariant is not verified for `inline-compose` at the required level.

