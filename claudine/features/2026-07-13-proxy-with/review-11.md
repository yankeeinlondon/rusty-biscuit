---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T10:00:34-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-11.md
previous: 2026-07-13-proxy-with/review-10.md
next: 2026-07-13-proxy-with/review-12.md
---

# Review 11: Proxy With

## Verdict

The feature is **not ready for production**. The review-10 implementation closes
the credential allow-list, interactivity-marker, switched-attempt identity, temp-file
replay, and hermetic-fixture findings it set out to address. The new child-visible
credential and interactivity rows are appropriately Level 2, and the provider-switch
system-prompt row now proves that a replay-created file can be read by the child.

That work exposed a broader lifetime split: the initial composition command still
drops its own file-backed system-prompt artifacts before the first child starts.
Three additional high-severity divergences remain in the same re-entry boundary:
retry/resume selection admits providers the invocation snapshot says are not
runnable, MCP tags are re-read from raw disk instead of the canonical composed
document, and provider capability warnings are discarded on replay.

## Findings

### 1. Critical: the initial composition launch drops file-backed system prompts before spawn

R3, R6, and AC10 require the prepared launch to retain the resources named by its
argv/environment through child exit (`spec.md:382-410`, `spec.md:492-505`,
`spec.md:934-937`). `construct_argv_and_system_prompt` stores
`application.artifacts` in the local `sp_artifacts` vector
(`cli/src/commands/wrap/composition/pipeline.rs:862-912`), but `CommandPhase`
does not carry that vector (`pipeline.rs:1128-1134`). The vector is therefore
dropped when command construction returns, before `provider_run_handoff` begins
the lifecycle/provider path (`pipeline.rs:1446-1452`). For a `NamedTempFile`,
that drop unlinks the file while the recorded argv or environment still contains
only its path.

The first attempt does not repair the loss. Its rebuilt facets equal the recorded
invocation, so `build_launch_plan` takes the verbatim shortcut and returns an empty
`system_prompt_artifacts` vector (`cli/src/commands/wrap/launch_plan.rs:438-458`).
Consequently a direct `claudine compose` using Gemini's `GEMINI_SYSTEM_MD`,
Claude's file flag, or Codex's file-backed replacement can start successfully
while silently receiving no system-prompt content.

Review 10 added the right lifetime owner for *replay-created* artifacts and an L2
Goose-to-Gemini retry row, but the direct/proxy equivalence row uses Goose's inline
`--system` delivery (`cli/tests/level2_lifecycle_control.rs:6466-6498`). No test
makes a first-attempt file-backed provider read the referenced file. Carry the
initial artifacts in command/attempt state until the child exits, and add an L1
lifetime seam plus an L2 direct-compose row whose fake Gemini and Codex/Claude
children read the file content, not merely the path.

### 2. High: retry/resume selection launches providers the immutable snapshot says are unavailable

R3/R6/R10 require the refreshed document to pass through the same provider
selection and typed-error behavior as a direct invocation
(`spec.md:382-410`, `spec.md:492-505`, `spec.md:575-586`). The direct selector
classifies a scalar unavailable provider as `SingleNotInstalled` and a list with
no runnable members as `ZeroInstalledList`
(`lib/src/composition/select.rs:80-130`), and the direct launch then requires a
resolved binary.

The retry/resume rebuild implements a different rule. `provider_from_hints`
accepts every scalar without consulting `InstalledProviderSnapshot`; for a list
with no runnable member it deliberately falls back to the first unavailable
entry (`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:451-468`).
`resolve_binary_for` then suppresses `resolve_binary_path_direct`'s actionable
missing-binary error and substitutes the bare executable name
(`target_launch.rs:471-488`). A refreshed document can therefore pass canonical
preparation, run `start`, and fail later at process spawn where invoking the same
document directly fails selection with the established typed diagnostic. On a
resume, the unavailable provider may instead appear as an incompatibility that
recommends retry, only for that retry to fail at spawn.

The L1 `frontmatter_agent_moves_the_provider_and_its_binary` row uses no installed
snapshot and asserts only that the path changed; every L2 provider-switch fixture
puts both fake binaries on `PATH`. There is no unavailable-provider direct-versus-
retry row. Route refreshed selection through the canonical non-TTY selector against
the invocation snapshot, preserve its concrete `CompositionError`, and add L1
scalar/list cases plus an L2 diagnostic-equivalence case proving no child starts.

### 3. High: MCP replay is derived from a second raw file read, not the prepared document

R3, R5, R8, AC8, AC10, and AC15 require one coherent prepared snapshot to own
body composition and the launch/MCP plan (`spec.md:382-410`, `spec.md:477-490`,
`spec.md:518-574`, `spec.md:930-956`). `materialize_attempt_prompt_phase` first
canonically reads and composes the document, then calls `rebuild_launch_identity`
with the source path (`cli/src/commands/wrap/harness_orch/loop_control.rs:1230-1265`).
That rebuild opens the file a second time and silently converts any read failure
to an empty body (`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:305-318`).

This is not only a filesystem race. The second read sees raw authored Markdown,
so it bypasses `proxy.with`, caller overrides, frontmatter interpolation, and the
canonical composed body. A body such as `#{{ mcp_server }}` can select an MCP
server on direct invocation, where the initial pipeline lexes
`request.prepared.prompt` (`pipeline.rs:1016-1023`), but lose that server on
retry/resume because the rebuild lexes the unresolved source text. The child
prompt later has its tag removed, so the requested server can disappear without
either reaching the model as text or reaching the launch as MCP configuration.

The current L2 retry row appends a literal `#proxyprobeserver` to disk before
both reads, and the L1 test explicitly blesses source-path lexing; neither covers
composed or overlaid tags. Compute and retain the sorted tag set from the
canonically composed prompt before a resume follow-up replaces provider input,
then pass that prepared value into the launch builder. Add Level 2 rows for an
interpolated/overlay-supplied tag across retry and resume, plus L1 coverage that
no second file read can erase the prepared set.

### 4. High: re-entry silently discards provider capability warnings

The equivalence contract includes warnings, and R3 requires canonical preparation
to own them (`spec.md:48-64`, `spec.md:382-410`). The direct command renders
system-prompt and sandbox warnings under the command's `silent`/`quiet` policy
(`cli/src/commands/wrap/composition/pipeline.rs:905-921`). Replay instead passes
an empty callback to model validation (`cli/src/commands/wrap/launch_plan.rs:531-545`),
discards the result of `apply_output_format` (`launch_plan.rs:550-559`), ignores
the warning returned by `apply_sandbox`, and consumes only args/env/artifacts from
`apply_system_prompt` while dropping `application.warnings`
(`launch_plan.rs:561-610`). The comment that a retry has no operator does not make
stderr warnings meaningless; non-interactive direct composition emits them too.

A provider switch can thus silently lose an unsupported model/output/sandbox or
system-prompt warning that invoking the refreshed document directly would show.
No L2 row compares warning output under a switch; the current system-prompt rows
only assert delivered bytes for supported mechanisms. Carry structured warnings
in the rebuilt attempt bundle and render them through the existing
`TerminalRenderable` logging path with the same output policy as direct launch.
Add L2 pane assertions for at least unsupported system-prompt delivery and an
unsupported output/sandbox request.

## Verification-level audit

| User-observable requirement | Strongest verification present | Required level | Result |
|---|---|---|---|
| Credential admission/removal follows the switched provider | Level 2 child-env rows in both directions | Level 2 | Present |
| Refreshed interactivity markers reach the child | Level 2 child-env rows in both directions | Level 2 | Present |
| Replay-created file-backed system prompt survives to child start | Level 2 Goose-to-Gemini retry child read | Level 2 | Present |
| Initial/direct file-backed system prompt survives to child start | L1 dispatcher lifetime tests; L2 direct row uses inline Goose delivery | Level 2 child read | **Gap — finding 1** |
| Unavailable provider selection matches direct execution after refresh | No direct-versus-retry unavailable-provider row | Level 2 typed diagnostic/no-spawn comparison | **Gap — finding 2** |
| Composed/overlay-derived MCP tags survive retry and resume | Level 2 covers only a literal tag appended to disk | Level 2 child launch observation | **Gap — finding 3** |
| Provider-switch capability warnings match direct execution | No re-entry warning row | Level 2 pane capture | **Gap — finding 4** |
| Recovery after a switched attempt uses the executed identity | Level 2 chain coverage; fix-discriminating admission remains L1/latent because all shipped profiles support resume | Level 2 where reachable, L1 seam otherwise | Present with the documented latent limitation |

No requirement in this feature depends on the terminal emulator's keyboard
encoder, so Level 3 OS keyboard injection is not applicable.

## Validation performed

- Source inspection traced initial command construction through first-attempt
  launch, and retry/resume materialization through provider selection, MCP
  planning, warning handling, and spawn.
- GitNexus confirmed `materialize_attempt_prompt_phase` is the production caller
  of `rebuild_launch_identity` and that the initial command builder feeds the
  later provider-run phase without an artifact owner in `CommandPhase`.
- `git diff --check 1d25ca37f..HEAD -- claudine/cli/src/commands/wrap
  claudine/cli/tests` passed.
- Focused `just test-cli` runs were started, but this clean worktree required a
  cold dependency build that exceeded the session's approximately 60-second
  non-interactive command limit. They were stopped with exit 130 before any test
  executed; this review does not present them as passing evidence.
- `cargo fmt --check --manifest-path claudine/cli/Cargo.toml` could not run
  because the pinned stable toolchain lacks the `rustfmt` component. No formatting
  write command was run.
- The acceptance map records that all 103 `level2_lifecycle_*` rows passed on its
  2026-07-20 macOS run. Those rows do not exercise the four cases above; the same
  recorded run still had one unrelated inventory failure and a WezTerm
  reachability failure outside the lifecycle matrix.

## Production readiness

`ready: false`. The first-attempt temp-file lifetime bug can silently omit a
system prompt, and each remaining high-severity item breaks the specification's
single canonical preparation/equivalence contract on a reachable re-entry path.
Production readiness requires fixing all four findings and adding the Level 2
child/pane evidence identified above.
