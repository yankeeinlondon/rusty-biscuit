---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T06:05:56-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: true
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-1.md
previous: /
next: 2026-07-11-sequence-plus/review-2.md
---

# Review 1: Sequence Plus

## Verdict

The feature is **not ready for production**. Most of the normalization,
preflight, JIT state, output-accumulation, task, and group machinery is present,
but two user-facing parts of the specification are functionally incomplete:
parallel task output is not connected to the attributed stream renderer, and
formal sequence documents do not have the same semantics when invoked directly
and when referenced. The rendering and Ctrl+C requirements also lack the
verification levels required by this review.

## Findings

### 1. High — Task body output never enters the attributed task stream

The concurrency UX at `spec.md:650-661` requires every task status/output line
to render through a task-colored bar, with serial work using the same geometry
through an invisible bar. The implementation constructs a `TaskStream` for each
group member, but calls only `open()` and `close()`
(`lib/src/composition/sequence/task/group.rs:84-103,143-172,258-285`). There is
no production call to `TaskStream::append` from the sequence or task execution
paths.

This leaves each task's actual data outside the promised framing:

- `run_step_task` discards `TaskOutcome.stdout` when translating the task into a
  `StepOutcome` (`cli/src/commands/wrap/sequence/task_run.rs:127-150`). Shell and
  side-effect output reaches the reserved `outputs` accumulator but is not
  emitted as task data on stdout.
- Prompt-task output continues through the ordinary wrapper stream, bypassing
  the member's `TaskStream`; concurrent provider lines therefore have no
  task-colored bar or stable per-line task attribution.
- The process-wide output coordinator may prevent byte tearing, but it cannot
  supply the task identity that was never attached to the frame.

The integration test at `cli/tests/sequence_groups.rs:651-700` checks only each
task's header/footer and explicitly does not assert that `fetched` or `rendered`
appears as framed task data. The `TaskStream::append` unit tests exercise a
renderer that the runtime does not use.

Required change: introduce a task-scoped live-output sink and route prompt,
shell, side-effect, lifecycle/status, and partial failure output through that
sink as complete frames. Keep captured undecorated stdout separate for
`outputs`, and add an end-to-end assertion that body lines, not only headers and
footers, carry the correct task attribution.

### 2. High — Formal sequence document semantics depend on how the file is entered

The specification requires the same formal `sequence:` shape for direct and
referenced invocation, permits `template` values of any type, accepts scalar
steps as shorthand, and applies `$schema` to normalized step state
(`spec.md:189-223,352`). The implementation has three incompatible behaviors:

- A directly invoked YAML document takes the inline-list branch and calls only
  `normalize_plan` (`lib/src/composition/sequence/mod.rs:123-129`). It never
  applies the formal document's `template` and never runs per-step state schema
  validation.
- A referenced formal document alone takes the loader path that applies
  `template` and validates each state
  (`lib/src/composition/sequence/source.rs:126-153`). The integration test at
  `cli/tests/sequence_sources_cli.rs:388-409` explicitly pins the direct-entry
  schema behavior as a known deviation from the specification.
- The referenced-document template implementation rejects every non-string
  default and requires every raw item to be an object
  (`lib/src/composition/sequence/source.rs:259-302`). This rejects valid
  `template: {rank: 42}` data despite the specified `{string: any}` shape, and
  rejects a valid scalar step such as `sequence: [blue]` whenever a template is
  present. The test at `lib/src/composition/sequence/tests.rs:711-729` currently
  enshrines the non-string rejection rather than the specified behavior.

Required change: route direct and referenced formal documents through one
normalization pipeline. Normalize scalar shorthand before applying template
defaults, preserve typed template values while evaluating interpolated strings,
insert generated fields, and then validate the same normalized state schema in
both entry modes. Replace the deviation tests with parity tests that run one
fixture both ways.

### 3. High — Visible rendering requirements have no Level-2 real-terminal evidence

The strongest rendering tests construct a `Terminal` value in process
(`lib/src/render/task_stream/tests.rs:13-30`) and assert on returned strings.
The CLI group tests capture child-process pipes. Both are Level 1 under this
review's taxonomy: neither passes the output through a real terminal emulator
or multiplexer and captures its rendered pane.

The feature's validation matrix acknowledges that real-terminal task-stream
coverage was deliberately omitted
(`features/2026-07-11-sequence-plus/validation-matrix.md:159-166`). That is not
compatible with the review rubric. The required colored bars, `▶`/ASCII glyph
fallback, visible widths, wrapping, invisible-bar alignment, no-color
attribution, and absence of torn ANSI sequences are observable terminal
behavior and require Level 2. A constructed capability object can retain
precise unit coverage, but it cannot replace a pane capture.

The file named `level2_sequence_overlay_pty.rs` is an `expectrl` pseudo-TTY
suite (`cli/tests/level2_sequence_overlay_pty.rs:1-14`), which is Level 1 by the
taxonomy in this review. It covers interactive schema collection, not task
stream rendering.

Required change: add focused tmux and/or WezTerm Level-2 tests that run a real
serial and parallel group, capture pane text, and verify body-line attribution,
wrapping at a narrow width, geometry parity, Unicode/ASCII fallback, and
no-color labeling. Keep the existing Level-1 tests for exact SGR/frame
integrity.

### 4. High — Sequence Ctrl+C and parallel fan-out are not verified at Level 3

The specification asserts that a user's Ctrl+C stops between steps with exit
`130` and fans out to all children of a parallel group
(`spec.md:393-398,644-648`). The sequence test manufactures `SIGINT` from a fake
provider (`cli/tests/sequence_jit.rs:538-580`), and the group unit test sets the
shared atomic interrupt flag directly. Those are useful Level-1 tests, but they
do not exercise a terminal emulator's input encoder.

The repository has genuine Level-3 Ctrl+C coverage, but it launches
`claudine compose --opencode`, not `claudine sequence`
(`cli/tests/level3_wrap_ctrl_c.rs:1-12,264-313`). It proves the common wrapper
can receive an OS keyboard chord; it does not prove sequence boundary handling
or all-child group fan-out.

Required change: add an env-gated Level-3 test that launches a sequence with a
blocking parallel group in a real terminal, injects OS Ctrl+C, and proves every
child terminates, later steps do not launch, the shell regains control, and the
sequence exits `130`.

### 5. High — Required release verification is incomplete and the recorded Level-2 gate is red

Acceptance Criteria 8 and 11 require compilation across macOS, Windows, and
Linux plus clean `just test`, `just test-l2`, and `just lint` verification
(`spec.md:692-695`). The feature's own validation matrix records three failing
Level-2 tests and says Windows execution/compilation was skipped
(`validation-matrix.md:159-165`). Source audit is useful, but it does not satisfy
the explicit compiled-platform requirement.

During this review, the focused sequence integration command did not reach test
execution within the non-interactive 60-second ceiling because this worktree
had to rebuild the dependency graph. It was stopped cleanly with exit `130`.
No green gate result is claimed here.

Required change: after the functional and test-level findings are fixed, record
clean full L1, L2, and lint runs and obtain native or CI compile/test evidence
for Linux and Windows. Pre-existing gate failures must be fixed or formally
removed from the required gate before production readiness can be asserted.

## Requirement Verification Levels

| User-observable requirement | Strongest verification observed | Assessment |
|---|---|---|
| Scalar/object state normalization, generated ids, neighbors, and `state` name coercion | Level 1 unit and CLI integration | Appropriate for typed composition state. |
| Dynamic expressions, shell/file sources, list classification, offsets, and operators | Level 1 unit and CLI integration | Appropriate for data parsing and resolution. |
| Formal templates and step-state schemas behave identically for direct and referenced invocation | Level 1 tests | **Gap:** tests pin divergent behavior; scalar and typed template defaults are rejected. |
| Typed preflight errors, recursive loading, cycles, exclusivity, and approved-byte parity | Level 1 unit and CLI integration | Appropriate for graph validation and process arguments. |
| JIT live-disk rereads, runtime `set`, reserved precedence, and serial `outputs` chaining | Level 1 CLI integration | Appropriate; no terminal encoder behavior is involved. |
| Standalone compose/inline-compose lifecycle visibility of `outputs` | Level 1 CLI integration | Appropriate. |
| Serial/parallel scheduling, caps, snapshot isolation, deterministic merges, and failure policy | Level 1 concurrency and CLI integration | Appropriate for scheduler state, subject to the incomplete output channel in Finding 1. |
| Interactive missing-property collection is deduplicated before provider launch | Level 1 pseudo-TTY | Appropriate for prompt/application logic; the current `level2_pty` name overstates its taxonomy. |
| Dynamic empty source prints a styled zero-step notice | Level 1 pipe capture | **Gap:** styling is terminal-visible and needs Level 2 capture. |
| Parallel headers, colored bars, body-line attribution, footers, wrapping, glyph fallback, and no-color behavior | Level 1 constructed-terminal and pipe tests | **Gap:** implementation frames only headers/footers, and terminal rendering needs Level 2. |
| Task/provider data stays on stdout while status framing stays on stderr | Level 1 pipe tests | **Gap:** shell/side-effect stdout is dropped from the user channel and prompt body data bypasses task attribution. |
| Concurrent frames do not tear ANSI sequences in a real terminal | Level 1 renderer/sink tests | **Gap:** keep Level 1 exact-byte checks and add Level 2 pane evidence. |
| User Ctrl+C stops between steps and fans out to all parallel children with exit `130` | Level 1 manufactured signal/flag; Level 3 exists only for standalone compose | **Gap:** sequence-specific Level 3 is required. |
| Cross-platform spawning, paths, newlines, and interruption | Level 1/source audit on macOS; Windows run skipped | **Gap:** the specified Windows/Linux compile and runtime evidence is incomplete. |

## Verification Performed

- Read the complete feature specification, Claudine sequence architecture/topic
  documentation, implementation, focused tests, and acceptance validation
  matrix.
- Queried the GitNexus index for the sequence execution flow and inspected the
  `execute_sequence` call context before tracing the concrete implementation
  seams.
- Started the focused `claudine-cli` sequence integration targets with nextest.
  Compilation did not finish inside the non-interactive command ceiling, so the
  command was interrupted with exit `130` and produced no test verdict.
- Did not run Level 2 or lint after the compilation ceiling. Their prior status
  is reported only where the checked-in validation matrix records it.

## Production Readiness Closure

Production readiness requires all five findings to close. In particular, a
follow-up review should see live task body output routed through the attributed
stream, one formal-document pipeline with parity tests, real-terminal captures
for the visible renderer, a sequence-specific Level-3 Ctrl+C test, and clean
cross-platform/release gates.
