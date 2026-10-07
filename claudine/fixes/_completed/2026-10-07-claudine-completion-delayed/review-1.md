---
$schema: feature-review.yaml
ready: false
findings:
    - high: "The registered diagnostic and populated lifecycle transport are unimplemented (R3/R4)"
    - high: "Shipped-CLI caller controls for failure shapes are missing (R1)"
    - high: "The author recovery and continuation regression suite is missing (R5)"
    - high: "Real-run installation and observation evidence is absent (R6)"
observations:
    - "Post-freeze settlement observations are recorded where no reader can observe them"
human_review: true
human_review_items:
    - |-
        Decide whether the new "completion was delayed" diagnostic should include a
        CPU-information field. The diagnostic is a published contract for authors:
        once a field is added, it cannot be renamed or removed without breaking
        author configurations. The host-discovery library this repository uses
        (`sniff`) can report only static CPU facts (brand, architecture, core
        count), not CPU load, so the field would carry no measurement today. The
        specification reserves this decision for you and the implementing plan
        cannot lock the diagnostic's field list until it is made.

        - Omit CPU information (recommended by the specification): the
          diagnostic stays small and honest; a CPU field can be added later
          without breaking anyone. If a delay recurs before measurement support
          exists, the record carries no host-load context.
        - Keep an always-unavailable CPU field: reserves a place in the contract
          now, but commits to a value shape nobody has measured, and authors
          see a field that never carries data.
        - Add CPU measurement: provides real data, but widens this fix with a
          sampling subsystem and cross-platform verification the specification
          deliberately excluded; the plan would need a scope revision first.
has_blocked_findings: true
blocked: false
reviewed_by: opencode/zai-coding-plan/glm-5.3
recurrence: false
created: "2026-10-07T11:10:06-07:00"
spec: 2026-10-07-claudine-completion-delayed/spec.md
implemented: false
description: "A **fix** review of `2026-10-07-claudine-completion-delayed/spec.md`"
fix: 2026-10-07-claudine-completion-delayed/review-1.md
---

# Review: claudine completion delayed, iteration 1

The fix is **not production ready**. The work that landed is well built and
well tested — settlement precedence, bounded retention, and the shipped-CLI
success chains are implemented and passing — but four acceptance areas of the
specification remain open, and the implementation log, plan, and documentation
all record them honestly as open. This review confirms those gaps against the
code, verifies the landed behavior, and adds one observation about the
observation machinery for the next cycle.

Context for a reader new to this repository: Claudine is a wrapper around
agentic CLIs (the `claudine` package in this monorepo). When a provider such
as Claude or Codex finishes, Claudine's reader threads must still drain and
present the provider's output; if that cleanup stalls, the old code converted
a successful run into a failure. This fix's goal is that a confirmed success
stays success, and that an *unconfirmed* completion (provider exited 0 but
Claudine never saw the provider's own completion record) is reported as a
distinct, diagnosable condition with the retained answer attached.

## What landed and was verified by this review

- **Settlement precedence** in [settle_parser](../../cli/src/commands/wrap/exec/reader_join.rs):
  a panicked reader stays `parse_failure`; a native nonzero exit without a
  result is `exit_failure` (or `interrupted` for exit 130) rather than a
  reader timeout; native exit 0 with no published verdict selects the new
  internal kind `claudine_completion_delayed`; `stream_reader_timeout`
  survives only as a subordinate warning, on stderr too. A repository-wide
  search found no remaining site that uses it as a primary summary identity.
  The nonzero-exit change the spec's reader's note requires is implemented,
  and the timeout topic page's outcome table and the reader-join matrix were
  updated in the same change.
- **Bounded retention** in [retention.rs](../../cli/src/commands/wrap/run_scope/retention.rs):
  separate 256 KiB answer and raw-stdout prefixes captured before decoding,
  honest complete/truncated flags, UTF-8-boundary clipping, base64-tagged
  invalid UTF-8, a bounded read of the Codex last-message file as answer-only
  data, and an idempotent freeze that moves buffers under the publication
  lock so a late or abandoned reader can neither mutate the snapshot nor
  dispatch into a settled run.
- **Answer-before-callback publication** in [run_scope.rs](../../cli/src/commands/wrap/run_scope.rs):
  the parser's finalized summary (answer, session, usage, verdict) is
  published before the line's rendering, logging, and hook callbacks run, and
  original answer text reaches the sink through a new non-blocking
  `on_response_text` observation that every sink wrapper forwards. This
  closes the result-line answer-loss gap the specification called out.
- **Operation observations** in [observation.rs](../../cli/src/commands/wrap/run_scope/observation.rs)
  and the output worker: lock-free coherent operation/time lanes per reader
  plus a settlement lane, saturating counters, and queued-versus-active
  terminal delivery recorded separately, all matching the R2 design
  constraint (no allocation or formatting per record).
- **Caller-exit projection** in [wrapper_exec.rs](../../cli/src/commands/wrap/wrapper_exec.rs):
  the direct structured wrapper returns native-exit evidence and a separate
  caller exit, so a settled semantic failure with native exit 0 exits 1 while
  the session evidence keeps the native 0 — the distinguishability R1 asks
  for.
- **Shipped-CLI regressions** in [completion_delayed.rs](../../cli/tests/l1/completion_delayed.rs):
  hermetic compiled fake Claude and Codex providers, POSIX and native Windows
  shell chains capturing the Claudine exit separately from a marker command,
  three controlled stall points (completion callback, earlier answer callback,
  unfinished terminal delivery) under injected short budgets with
  synchronization handshakes, and assertions on the frozen observation. The
  missing-verdict caller control is covered: the answer-callback scenario
  asserts caller exit 1 and no marker.
- **Budgets and scope invariants** (R6): the 120 s drain limit, 5 s pipe cap,
  and 250 ms settle requirement are unchanged; short budgets exist only
  behind the `test-fixtures` feature; no new public settings, samplers,
  writers, or deadline resets were introduced.

Documentation (`docs/topics/timeouts.md`, `flow-control/lifecycle.md`,
`composition.md`, `non-interactive-sessions.md`, the README, and the claudine
skill) correctly distinguishes implemented behavior from the planned
diagnostic transport, and no topic page names this fix directory.

## Findings

### The registered diagnostic and populated lifecycle transport are unimplemented (R3/R4) — priority high

**Authority.** R3: "Register the diagnostic through the existing
`Diagnostic`/`BlockError` architecture so terminal reports, lifecycle
`err.*`, and machine output describe the same selected condition" and
"`code_for_error_kind` gains the mapping `claudine_completion_delayed` →
`timeout.claudine_completion_delayed`." R4: "**The detail must be carried,
not reconstructed from a label.** … The attempt must carry a populated
diagnostic snapshot (or equivalent typed detail) from reader settlement
through the session summary to the lifecycle context." R2: "An exhausted
cleanup deadline must publish the observation through the existing
run/session diagnostic storage where available … A blocked terminal cannot be
the only destination." Acceptance: "a lifecycle handler reads populated
`err.detail` values (not all-null) for `timeout.claudine_completion_delayed`"
and "terminal blockage cannot prevent diagnostic snapshot publication through
the existing storage route."

**Defect class.** The spec's author-facing contract for the new condition —
a stable diagnostic code with populated typed detail — exists today only as
an internal error-kind string inside claudine-cli. Every surface an author or
machine consumer would read is absent. This is the same gap for every site in
the class; the sweep:

| Surface the spec requires | Site | State |
|---|---|---|
| Registry row `timeout.claudine_completion_delayed` (timeout/transient/internal) | [registry.rs](../../lib/src/diagnostics/registry.rs) | Absent; only `timeout.step_silence` and `timeout.wall_clock` exist |
| `code_for_error_kind` mapping | [error_kind.rs](../../lib/src/diagnostics/error_kind.rs) | Absent; a lifecycle handler sees `err.code == null` for the new kind |
| Populated `err.detail` to lifecycle handlers | [context.rs](../../lib/src/composition/lifecycle/context.rs) `LifecycleErrorInfo` | Label-only seam unchanged; detail seeds all-null |
| Production storage publication of the frozen observation | [semantic.rs](../../cli/src/commands/wrap/exec/spawn/semantic.rs) | Observation carried in `ProcessResult.completion_observation`; only the test-fixtures publisher ([completion_fixture.rs](../../cli/src/commands/wrap/exec/completion_fixture.rs)) serializes it; no session/attempt consumer persists it |
| `claudine errors` catalog coverage | CLI introspection | No row |
| Attempt status / session record / enclosing composition errors | CLI summary surfaces | Internal kind string only; no populated snapshot |

Because the mapping is absent, the spec's own R5 example handler
(`when: "err.code == 'timeout.claudine_completion_delayed' …"`) cannot match
today, and the R4 fields (`provider`, `exit_code`, `elapsed_ms`, `limit_ms`,
`verdict_received`, `response_text`, completeness flags, `observed_operation`)
exist only inside the frozen CLI observation, unreachable by authors.

**Blocked portion.** Locking the registry field list is blocked by the
author's CPU-field ruling (specification Open Question 1, carried in this
review's `human_review_items`); the implementing plan explicitly waits for it
before freezing registry fields. The populated-detail transport and the
production storage publication site are engineering work that is *not*
blocked and can proceed once the ruling lands (with omission recommended, the
transport needs no CPU handling at all). This finding is therefore partially
blocked: its registry portion waits on the human decision; its transport
portion does not.

### Shipped-CLI caller controls for failure shapes are missing (R1) — priority high

**Authority.** R1: "Controls must show that a genuine provider error, nonzero
native exit, real reader panic, interruption, and provider timeout retain
their outcomes and prevent the success chain where appropriate" and "A
visible answer alone must not establish success." Acceptance: "actual
failures and missing-verdict controls remain failures." The plan's Wave 6
additionally names the text-only and Codex last-message-only honest-failure
controls.

**Defect class.** The compiled fake provider
([fake_completion/main.rs](../../cli/tests/bin/fake_completion/main.rs)) can
emit only the success shape (hardcoded success records, always exit 0), so
the shipped-CLI suite proves the success-preserving half of the caller
contract but none of its failure-preserving controls. Sweep of every control
R1 names, with the strongest existing verification for each:

| Control (shape at the caller boundary) | Shipped-CLI chain test | Strongest existing coverage |
|---|---|---|
| Confirmed success, delayed callback/delivery | Yes — exit 0, marker written | `completion_delayed.rs`, both providers, three stall points |
| Missing verdict, native exit 0 (unconfirmed completion) | Yes — exit 1, no marker | `answer-callback` scenario; matrix `claudine_completion_delayed` cells |
| Genuine provider error | Missing | Helper level: `provider_streams` keeps the provider error with the real exit code |
| Nonzero native exit, no result | Missing | Helper level: matrix `exit_failure` cells; unit `a_published_successful_summary_does_not_turn_a_nonzero_exit_into_success` |
| Real reader panic | Missing | Helper level: unit payload test; `reader_join/diagnostics.rs` real-panic-hook test |
| Interruption (exit 130) | Missing | Helper level: matrix `interrupted` cells |
| Provider timeout | Missing | Helper level: matrix early-termination override test |
| Text-only answer, no verdict (Claude) | Missing | Helper level: `an_answer_callback_before_the_verdict_retains_partial_text_without_success` |
| Codex last-message file without `turn.completed` | Missing | Unit: `last_message` bounded read; no run-level control (the fake always writes the file *and* emits the verdict) |

The settlement-level halves are genuinely covered; what is missing is the
caller-boundary assertion the spec asks for — that each failure shape keeps
the Claudine exit nonzero and prevents the marker. The existing fixture
machinery (scenario handshake directories, chain helper) extends to these
shapes cheaply, so the fix cost is low next to the protection: the caller
exit is the one behavior a shell user directly consumes.

### The author recovery and continuation regression suite is missing (R5) — priority high

**Authority.** R5: "Document and test a complete author example: a sequence
with `fail_fast: false` whose step's `failure` handler reads the retained
response. Verify that the later step runs and that the original step keeps
its honest failed outcome. Also test default failure behavior (`fail_fast`
defaults to `true`, so the sequence halts), an explicit successful recovery,
and exactly-once `finalize`." Acceptance: "author recovery and continuation
examples behave as documented."

No such tests exist, and the documentation correctly withholds the runnable
example because the fields a handler would read are not yet reachable
(`err.detail.response_text` is null through the label-only seam). This
finding is blocked by the first finding, not by human review: once the
populated transport lands, the sequence/loop policy suite is straightforward
against the existing composition test fixtures. The documentation already
describes `fail_fast` semantics and the retry-side-effects warning; only the
tests and the promoted example are owed.

### Real-run installation and observation evidence is absent (R6) — priority high

**Authority.** R6: "After local validation, install the current implementation
through the normal Claudine recipe and observe representative real Claude and
Codex runs. Record the tested revision/build, provider version, native exit,
caller exit, and available completion observations."

No installation or live-provider observation has occurred; the implementing
plan sequences this strictly after the earlier waves, so it is blocked by the
three findings above rather than by human review. It is listed so the cycle's
terminal state stays honest: this evidence, plus the recorded cross-OS runs,
is what closes the specification's verification section.

## Observations

### Post-freeze settlement observations are recorded where no reader can observe them

The settlement lane in [spawn/semantic.rs](../../cli/src/commands/wrap/exec/spawn/semantic.rs)
records `SummaryConstruction` and `Settlement` (and the vocabulary defines
`StoragePublication`) *after* `freeze_with_output` has already frozen the
`CompletionObservation`; the frozen snapshot is immutable, so those tags can
never appear in any snapshot a consumer reads, and `StoragePublication` has
no production recording site at all. This is harmless in the current partial
state — the frozen snapshot correctly represents "last observed operation at
cutoff" — but when the populated transport lands, its designer should decide
deliberately whether the diagnostic's `observed_operation` freezes at cutoff
(spec wording supports this) or whether settlement completion should be
observable, and either remove the dead recordings or give them a reader. No
action is required from this review cycle.

## Verification and requirement coverage

No file-format or configuration reader was added or changed; the plan's rule
9 confirmed no persisted-summary deserializer change, so the input robustness
matrix does not apply. The new shipped-CLI tests are declared in
`cli/tests/l1/main.rs` behind the `test-fixtures` feature, which the
package's CI features list enables; `just check-tier-coverage claudine`
reports zero stranded tests. The spec classifies these caller checks as
Level 1; no requirement in this spec needs a real terminal emulator, so no
verification-level mismatch exists.

| Spec requirement | Status |
|---|---|
| R1 success chains continue under delayed cleanup | Implemented and tested (both providers, both stall classes) |
| R1 failure controls at the caller boundary | Gap — finding 2 |
| R2 bounded operation observations | Implemented and tested; production publication gap is part of finding 1 |
| R3 outcome selection and label demotion | Implemented and tested |
| R3/R4 registered code, populated detail, storage publication | Gap — finding 1 (partially blocked on the CPU ruling) |
| R4 retention, honest flags, result-line answer retention | Implemented and tested |
| R5 author policy suite and runnable example | Gap — finding 3 |
| R6 budget invariants | Preserved; no new settings |
| R6 real-run observation | Gap — finding 4 |

Results from this review (focused reruns of the landed suites; the
implementer's logs additionally record the full area `just test` at 8,507
passed and `just lint` clean):

- `just test-cli completion_delayed::` — 3 passed.
- `just test-cli run_scope:: reader_join:: output_worker::` — 63 passed.
- `just check-tier-coverage claudine` — no stranded tests.

Cross-OS evidence (native Windows, WSL2) remains CI's responsibility and does
not affect this review's readiness decision; the shipped chain test carries a
native Windows branch, so CI will exercise it.
