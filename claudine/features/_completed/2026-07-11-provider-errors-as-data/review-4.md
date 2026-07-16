---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T09:45:01-07:00
spec: 2026-07-11-provider-errors-as-data/spec.md
implemented: true
description: A **feature** review of `2026-07-11-provider-errors-as-data/spec.md`
feature: 2026-07-11-provider-errors-as-data/review-4.md
---

# Review 4: Provider Errors as Data

## Verdict

The feature is **not ready for production**. The prior runtime blocker is
closed: exhausted remediation now fails the real `claudine compose` process,
preserves the durable findings report, and retains the authored finalize
reason. The provenance records identified in review 3 are also reconciled.
However, the authoritative specification still states twice that exhaustion
fails open and completes successfully, directly contradicting both its adjacent
fail-closed requirement and the implementation.

## Findings

### Medium: D10 still specifies the retired fail-open exhaustion behavior

The implementation and its process test now enforce the intended terminal
condition, but the spec still describes the behavior that review 3 rejected:

- `spec.md:467-470` says exhausted remediation completes and “fails open to
  human judgment.”
- `spec.md:600-603` repeats that P5 exhaustion fails open to C1 review.
- `spec.md:480-488`, by contrast, requires a non-successful fleet result when
  findings survive exhaustion.

The production dispatcher now distinguishes `StackControl::Error` from an
ordinary `Stop` and returns `TerminalControlAction::Abort`
(`control_dispatch.rs:96-106`). The real-binary regression proves that the
initial run plus two resumes end non-zero while leaving the findings report
unchanged (`provider_error_finalize.rs:37-101`). The spec should describe that
sequence explicitly: `Exhausted` falls through only to `finalize`; the
non-clean finalize guard then raises `error`, which aborts the command. Update
both stale passages and P5's disposition so the design contract matches the
shipped behavior.

This is the remaining portion of review 3's documentation finding. The Codex
research, pilot checkpoint, and delta report now correctly describe the two
graduated additions and distinguish issue-reporter interpretation from official
provider confirmation.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Research frontmatter is the sole executable vocabulary source and generated output is deterministic | Level 1: generator vocabulary tests plus `claudine-gen check` | Meets the required level. |
| Archived seed branch, kind, bucket, item, and value identity survive graduation | Level 1: full-roster archived-seed gate tests | Meets the required level. |
| Gate executions durably distinguish `clean`, `findings`, and `gate_error`, including report-write failure | Level 1: checker process tests | Meets the required level. |
| Kilo selects its own vocabulary while sharing OpenCode wire parsing and rejects invalid identities | Level 1: discriminating injected-vocabulary parser test plus typed-constructor-error test | Meets the required level. |
| The exact Codex `Selected model is at capacity` event classifies as `ApiRemote` without matching ordinary capacity prose | Level 1: parser-level positive and collision fixtures | Meets the required level. |
| Findings that survive both resume attempts make the fleet command fail | Level 1: real `claudine compose` process test with a fake resumable provider; asserts three attempts, non-zero exit, preserved report, and authored reason | Meets the required level. |
| Research provenance and checkpoint records describe the graduated behavior accurately | Document inspection | Meets the requirement outside the stale D10 exhaustion passages identified above. |

No requirement in this feature concerns terminal rendering, terminal input
encoding, keyboard, paste, IME, mouse, or scrolling. Level 2 and Level 3 tests
are therefore not applicable; Level 1 is the appropriate verification level.

## Verification Performed

- Focused Level-1 CLI regression tests passed: the real-binary exhausted-resume
  test and the dispatcher `Error`/`Stop` discrimination test.
- Fleet lifecycle tests passed: 2 tests.
- Generator gate outcome tests passed: 9 tests.
- Vocabulary projection tests passed: 4 tests; signal replay passed all 83
  records with 17 declared exclusions.
- `cargo run --quiet -p claudine-gen -- check` reported all generated artifacts,
  including `stream vocabulary.rs`, clean.
- `just lint` passed for all five Claudine packages and repository guards.
- The package-wide `just test` run reached the CLI suite with all completed
  provider-error tests green, then failed an unrelated existing
  `context_reports_preserve_all_columns_at_minimum_supported_width` test. In
  this dirty review worktree, the dynamic `File Changes` value makes its table
  require 82 content columns at `COLUMNS=53`. No context-command source or test
  is changed by this feature; the failure is recorded as a test-isolation
  caveat rather than attributed to provider-error behavior.

GitNexus reports the changed terminal dispatcher as HIGH blast radius (13
direct callers, 17 impacted symbols, and two composition execution flows). Its
index points at the nearby Claudine checkout rather than this exact worktree, so
the graph was used for topology and the current worktree's process/unit tests
were used for behavioral verification.
