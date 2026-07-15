---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T07:47:41-07:00
spec: 2026-07-11-provider-errors-as-data/spec.md
implemented: true
description: A **feature** review of `2026-07-11-provider-errors-as-data/spec.md`
feature: 2026-07-11-provider-errors-as-data/review-3.md
---

# Review 3: Provider Errors as Data

## Verdict

The feature is **not ready for production**. The research-backed generation
cutover is deterministic, the exact Codex capacity incident now classifies
through the real parser as `ApiRemote`, and the gate now persists explicit
`clean` / `findings` / `gate_error` outcomes. However, the prior review's
highest-risk lifecycle finding is not actually closed: exhausted remediation
still returns the provider's successful exit code because an `error` control
raised by `finalize` is converted to fallthrough by the real harness dispatcher.

## Findings

### High: the exhausted-remediation finalize guard still returns success

The new `finalize.stack` correctly produces `StackControl::Error` when the
durable outcome is not `clean`, but the production harness does not treat that
control as a failed run:

1. `drive_terminal_recovery` invokes `run_finalize_with_recovery` after the
   exhausted `resume` falls through.
2. `run_finalize_with_recovery` sends the finalize outcome through
   `dispatch_terminal_control`.
3. `decide_control` maps `StackControl::Error` to `ControlDispatch::Stop`.
4. `dispatch_terminal_control` maps `Stop` to
   `TerminalControlAction::Fallthrough`.
5. The success path then returns the original provider `outcome.exit_code`,
   which is zero.

The test named `exhausted_remediation_fails_finalize_and_preserves_findings`
does not exercise that path. It executes the lifecycle stack directly and
asserts only that the raw finalize outcome contains `StackControl::Error`; it
never calls `run_finalize_with_recovery`, `drive_terminal_recovery`, or the
harness loop that decides the command result. The test therefore passes while
the user-visible fleet result remains successful.

Make an `Error` surfaced by `finalize` abort the run at the production routing
boundary, without changing the intended semantics of `Stop`. Add a Level-1
process or harness-loop integration test that keeps the document invalid for
the initial run and both resumes, then asserts a non-zero command result and an
unchanged machine-readable findings report. This is the same D10 terminal
condition identified in review 2 and remains a production blocker.

### Medium: authoritative research and closeout records still contain stale or unsupported claims

The main spec, plan, and fleet review now describe graduation, but several
records still contradict the shipped state:

- `_pilot-codex.md` labels the B2 human checkpoint **PENDING** and says later
  phases must not begin, even though the feature declares Phase C complete.
- `codex.md` says its accepted additions are merely proposed and “do not change
  runtime behavior by themselves,” although that frontmatter is now the sole
  executable source and both additions are present in generated runtime code.
- `_delta-report.md` says “The single delta (Δ1)” in its R3 discussion even
  though the report now accepts two deltas.
- The Codex research and delta report say an OpenAI collaborator confirmed the
  capacity interpretation in issue #17014. The linked issue records the exact
  phrase and the reporter's capacity/admission inference, but exposes no
  collaborator confirmation. The issue is still valid `issue_tracker` evidence
  for the observed string; the stronger attribution is unsupported and should
  be removed or replaced with a source that contains it.

Because these documents are the provenance and adjudication record for the
generated data, this is more than cosmetic drift. Reconcile them with the final
source-of-truth state and distinguish observed issue evidence from official
provider confirmation.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Research frontmatter is the sole executable vocabulary source and generated output is deterministic | Level 1: generator integration tests and `claudine-gen check` | Meets the required level. |
| Seed branch, kind, bucket, item, and value identity survive graduation | Level 1: archived-seed gate tests over the full roster | Meets the required level. |
| Gate executions durably distinguish `clean`, `findings`, and `gate_error`, including report-write failure | Level 1: checker process tests | Meets the required level. |
| Kilo selects its own vocabulary while sharing OpenCode wire parsing | Level 1: parser/vocabulary tests | Meets the required level. |
| The exact Codex `Selected model is at capacity` event classifies as `ApiRemote` without matching ordinary capacity prose | Level 1: parser-level positive and collision fixtures | Meets the required level. |
| Findings that survive both resume attempts make the fleet command fail | Level 1 component test stops at raw `StackControl::Error`; no process/harness-loop result assertion | **Gap.** The tested seam bypasses the dispatcher that converts the error to success. |
| Research provenance and checkpoint records describe the graduated behavior accurately | Document and linked-source inspection | **Gap.** Stale phase claims and one unsupported attribution remain. |

No requirement in this feature concerns terminal rendering, terminal input
encoding, keyboard, paste, IME, mouse, or scrolling. Level 2 and Level 3 tests
are therefore not applicable; Level 1 process coverage is the correct rigor for
the lifecycle behavior above.

## Verification Performed

- Focused `claudine-gen` outcome and vocabulary integration suites passed: 13
  tests.
- Focused fleet lifecycle tests passed: 2 tests. Inspection showed that the
  exhaustion test does not reach production control dispatch.
- Exact Codex capacity positive and collision parser fixtures passed: 2 tests.
- `cargo run --quiet -p claudine-gen -- check` reported the generated stream
  vocabulary and all other generated artifacts clean.
- `just lint` passed for all five Claudine packages and repository guards.
- `git diff --check` passed.
- The linked Codex issue and OpenAI API error-code documentation were inspected
  for the two accepted additions.

GitNexus was used for initial flow orientation, but its registered Claudine
index was 112 commits stale. The documented analyzer fallback exceeded the
non-interactive 60-second ceiling and was stopped; all findings above were
therefore verified directly against the current worktree and executable tests.
