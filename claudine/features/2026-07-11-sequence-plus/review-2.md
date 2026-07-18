---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T07:16:58-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-2.md
previous: 2026-07-11-sequence-plus/review-1.md
---

# Review 2: Sequence Plus

## Verdict

The feature is **not ready for production**. The follow-up work partially closes
Review 1's task-body framing defect: shell and side-effect output now reaches a
task-scoped data channel, and ordinary prompt assistant chunks receive the
task's gutter. That migration currently leaves the Claudine library test target
uncompilable, however, and several prompt-output paths still bypass task
attribution.

Review 1's formal-document parity, Level-2 rendering, sequence-specific Level-3
Ctrl+C, and cross-platform/release-gate findings remain open. The current source
and tests explicitly preserve or document several of those deviations.

## Findings

### 1. High — The task-stream API migration leaves the Level-1 test target uncompilable

`TaskLiveOutput` now owns an `Arc<dyn TaskStreamSink>` and has no lifetime
parameter (`lib/src/render/task_stream.rs:397-415`), but its tests still construct
`TaskLiveOutput<'a>` from `&RecordingSink`
(`lib/src/render/task_stream/tests.rs:326-327,410-417`). The task fixture likewise
still supplies `Option<&dyn TaskStreamSink>` where `TaskExecution` now requires
`Option<&Arc<dyn TaskStreamSink>>`
(`lib/src/composition/sequence/task/tests.rs:194`).

A focused `cargo check --manifest-path lib/Cargo.toml --tests --color=never`
fails with one `E0107` and three `E0308` errors at those sites. Consequently the
package's canonical `just test` gate cannot reach test execution, and none of
the newly added renderer assertions currently run in the repository's normal
test build.

Required change: finish the ownership migration in all fixtures by using shared
`Arc` sinks and the new `TaskExecution::stream` type, then run the complete
Claudine Level-1 suite. Add a compile-oriented regression only if it protects a
public ownership contract; ordinary unit compilation should otherwise be
sufficient.

### 2. High — Formal sequence semantics still depend on the entry path

The specification requires direct and referenced formal sequence documents to
accept the same shape, normalize scalar shorthand, preserve typed `template`
values, and validate `$schema` against normalized step state. The implementation
still has the three divergences identified in Review 1:

- A directly invoked YAML document reaches the inline source branch and only
  calls `normalize_plan` (`lib/src/composition/sequence/mod.rs:123-129`). It does
  not pass through the formal-document template and per-step schema pipeline.
- Only the referenced-file loader applies `template` and calls
  `validate_state_schema`
  (`lib/src/composition/sequence/source.rs:126-153`).
- `apply_template` rejects every non-string value and requires every raw item to
  be an object before scalar normalization
  (`lib/src/composition/sequence/source.rs:267-302`). This conflicts with the
  specified `{ "<string>": any }` template shape and scalar-step shorthand.

The tests do not merely omit parity: they lock the divergence.
`external_template_non_string_value_fails` expects `template: {rank: 42}` to
fail (`lib/src/composition/sequence/tests.rs:712-730`), while
`a_directly_invoked_documents_schema_validates_the_document_not_the_state`
labels direct behavior a known specification deviation
(`cli/tests/sequence_sources_cli.rs:388-409`). The public sequence topic repeats
the known asymmetry (`docs/topics/flow-control/sequences.md:261-268`).

Required change: route direct and referenced formal documents through one
normalization pipeline. Normalize scalar shorthand before applying defaults,
evaluate interpolated strings without rejecting already typed values, generate
state fields, and apply the formal step-state schema identically in both entry
modes. Replace the deviation tests and documentation with one fixture exercised
through both paths.

### 3. High — Task attribution still has live-output bypass paths

The new `task_frame_writer` correctly reaches the structured provider stdout
reader for ordinary assistant chunks
(`cli/src/commands/wrap/sequence/task_run.rs:342-347` and
`cli/src/commands/wrap/exec/spawn/semantic.rs:156-162,237-269`). Shell and
side-effect payloads also call the task's `TaskLiveOutput`. That closes the
common short-output path, but not the full rendering contract:

- The 30-second idle-flush ticker receives `StreamOutput` and `AssistantStream`,
  but not the task frame writer
  (`cli/src/commands/wrap/exec/spawn/semantic.rs:166-175`). It writes flushed
  assistant frames directly through `stdout_writer`
  (`cli/src/commands/wrap/exec/watchdog/spawn.rs:61-68`), so a long-running
  task can visibly lose its bar at the first idle flush.
- After a fatal semantic-parser error, raw fallback lines use `writeln!(out, …)`
  directly (`cli/src/commands/wrap/exec/spawn/semantic.rs:330-351`). Those lines
  are task data but bypass the task gutter.
- The task decorator is threaded only through assistant stdout. Task-local
  status/reasoning events continue through the ordinary stderr/status emitters,
  even though the specification requires every subsequent status/output line
  to retain task attribution.

The new Level-1 CLI tests exercise immediate shell payloads. They do not hold a
partial Markdown block through an idle flush, force semantic fallback, or prove
task attribution on provider status/reasoning output. In addition,
`task_run.rs:14-32` still says prompt text is not framed, which is stale after
the ordinary assistant path was wired; per repository convention the code is
authoritative and the comment must be corrected.

Required change: make every task-owned data and status emission route through a
task-scoped decorator, including idle flush, semantic fallback, and provider
status/reasoning. Preserve the stdout/stderr channel contract and captured
undecorated `outputs`. Add Level-1 tests for each bypass before adding the
Level-2 evidence in Finding 4.

### 4. High — Terminal-visible sequence rendering still has no Level-2 evidence

The strongest task-stream tests remain in-process renderer assertions and
child-process pipe captures. `FORCE_COLOR=1` in `sequence_groups.rs` constructs
the desired capability mode but does not run output through a real terminal.
Under this review's taxonomy those are Level 1.

There is still no Level-2 sequence task-stream test. The only sequence-named
file, `cli/tests/level2_sequence_overlay_pty.rs`, uses an `expectrl` pseudo-TTY;
it is Level 1 by the stated rubric and covers schema collection rather than task
stream rendering. No `level2_*` test references `TaskStream` or parallel-group
body framing. The checked-in validation matrix explicitly says Level 2 was
deliberately omitted (`validation-matrix.md:159-166`), contrary to this review's
required test rigor.

This leaves the colored bars, cross-channel color association, narrow-width
wrapping, invisible-bar alignment, Unicode/ASCII glyph behavior, no-color
attribution, styled zero-step notice, and real-pane frame integrity at the
wrong verification level.

Required change: add focused tmux and/or WezTerm Level-2 tests that run serial
and parallel sequence tasks and capture the pane. Verify visible body
attribution, wrapping and geometry, textual attribution without color, glyph
fallback where the harness can control capabilities, and the zero-step notice.
Keep Level-1 exact-byte/SGR tests for frame atomicity; Level 2 complements rather
than replaces them.

### 5. High — Sequence Ctrl+C and parallel fan-out still lack Level-3 verification

The specification makes user-keyboard claims: Ctrl+C must stop at sequence
boundaries, fan out to every running parallel child, prevent later work from
launching, and return exit `130`. The sequence tests still manufacture a signal
or set the cooperative interrupt flag at Level 1. The existing Level-3 Ctrl+C
test launches standalone `claudine compose --opencode`, not a sequence or
parallel group (`cli/tests/level3_wrap_ctrl_c.rs:264-313`).

That standalone test proves the shared wrapper can receive one OS keyboard
chord; it does not prove sequence scheduling or group fan-out. No
sequence-specific `level3_*` test exists.

Required change: add an env-gated Level-3 test that launches a sequence with a
blocking parallel group in a real terminal, injects OS Ctrl+C, and verifies all
children terminate, no later step starts, the shell regains control, and the
sequence exits `130`.

### 6. High — The required release and cross-platform gates are not green

Acceptance Criteria 8 and 11 require macOS, Windows, and Linux compilation and
clean `just test`, `just test-l2`, and `just lint` results. Current evidence does
not satisfy those gates:

- The Claudine library test target fails to compile as described in Finding 1.
- No new feature-specific Level-2 suite exists, and the validation matrix records
  three failures in the overall Level-2 gate.
- The validation matrix records Windows runtime/compilation as skipped and
  substitutes source audit (`validation-matrix.md:153-165`). No Linux execution
  evidence is recorded for this feature either.

Cross-platform source design is useful evidence, but it cannot satisfy an
acceptance criterion that explicitly requires compilation on all three target
families. A red or unexecuted required gate also prevents a production-ready
verdict independently of the functional findings above.

Required change: after closing the functional and test-level defects, record
clean Level-1, Level-2, and lint runs and obtain native or CI compile/test
evidence for Windows and Linux as well as macOS.

## Requirement Verification Levels

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Scalar/object state normalization, generated ids, neighbors, and `state` name coercion | Level 1 unit and CLI tests | Appropriate level, but the current library test target does not compile. |
| Dynamic expression/shell/file sources, list classification, offsets, operators, and typed source errors | Level 1 unit and CLI tests | Appropriate level for parsing/resolution; current gate is blocked by Finding 1. |
| Formal templates and step-state schemas behave identically through direct and referenced entry | Level 1 tests | **Gap:** tests explicitly pin divergent behavior and reject specified typed/scalar inputs. |
| Static preflight, cycles, approved-byte parity, JIT rereads, runtime `set`, reserved precedence, and `outputs` chaining | Level 1 unit and CLI tests | Appropriate level for state/process contracts; current gate is blocked by Finding 1. |
| Serial/parallel scheduling, caps, snapshot isolation, merge order, and failure policy | Level 1 concurrency and CLI tests | Appropriate for scheduler semantics, subject to the signal and rendering gaps below. |
| Interactive missing-property collection | Level 1 pseudo-TTY | Appropriate for application prompt logic; the current `level2_…_pty` filename overstates its tier. |
| Task/provider data stays on stdout while task status stays on stderr | Level 1 in-process and pipe capture | **Gap:** ordinary paths are covered, but idle flush, parser fallback, and task status attribution bypass the decorator. |
| Colored bars, body attribution, footers, wrapping, glyph/no-color behavior, invisible-bar geometry, and styled zero-step notice | Level 1 constructed-terminal and pipe tests | **Gap:** terminal-visible behavior requires Level 2. |
| Concurrent frames remain intact after rendering through a real terminal | Level 1 synchronized-sink tests | **Gap:** exact bytes belong at Level 1, but the real-pane result also needs Level 2. |
| User Ctrl+C stops the sequence and fans out to every parallel child with exit `130` | Level 1 manufactured signal/flag; unrelated standalone compose has Level 3 | **Gap:** the sequence-specific keyboard path requires Level 3. |
| Cross-platform spawning, paths, newlines, and interruption | macOS source audit/partial build; Windows skipped; no feature Linux run recorded | **Gap:** the explicit three-platform compile requirement is unmet. |

## Verification Performed

- Read the complete specification, Review 1, Claudine sequence architecture and
  user documentation, current sequence/task/render implementation, integration
  tests, and acceptance validation matrix.
- Used the GitNexus sequence execution flow and symbol contexts to trace
  `execute_sequence`, `resolve_sequence_plan`, and parallel task execution
  before inspecting the concrete source paths.
- Ran `cargo check --manifest-path lib/Cargo.toml --tests --color=never`; it
  failed with four type errors caused by the unfinished task-stream ownership
  migration.
- Started the canonical Claudine `just test` recipe. The catalog-types package
  passed 21 tests, but the cold dependency build did not reach Claudine within
  the non-interactive command ceiling and was stopped cleanly with exit `130`.
  The subsequent focused compile check supplies the actual Claudine verdict.
- Did not run Level 2 or lint after the Level-1 compile blocker. No passing
  result is claimed for an unrun or interrupted gate.

## Production Readiness Closure

Production readiness requires all six findings to close. A follow-up review
should see a compiling Level-1 suite, one formal-document pipeline with direct
and referenced parity, task attribution on every live-output path, real-terminal
Level-2 captures, sequence-specific Level-3 Ctrl+C fan-out, and clean
cross-platform/release gates.
