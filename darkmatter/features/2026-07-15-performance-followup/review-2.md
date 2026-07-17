---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T16:30:01-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: true
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-2.md
previous: 2026-07-15-performance-followup/review-1.md
next: 2026-07-15-performance-followup/review-3.md
---

# Review 2 — Performance Follow-up

## Verdict

This feature is **not ready for production**. Review 1's immediate red tests and
checkout-hostile fixtures were repaired, and the current macOS Level-1 and
Level-2 gates are green. The remaining gaps are nevertheless release blockers:
the integrated compose regression has no owner disposition, required Windows
behavior is absent, the terminal proof does not meet its cross-platform and
theme-independence contract, several benchmark claims remain unreproducible,
and two remediations violate the explicit no-new-public-API invariant.

GitNexus reports a **HIGH** aggregate change risk for the feature range: 283
changed symbols, 68 files, and six affected execution processes. The directly
reviewed hash symbols are individually low-risk, while the compose changes feed
the `run_stage` process family through `execute_directive_detailed`.

## Findings

### High — The integrated compose regression gate still fails

The retained bracketed run in `results.md` confirms regressions of **+34.0%**
for `compose_trivial`, **+27.4%** for schema/transclusion, **+14.4%** for heavy
transclusion, and **+11.0%** for heavy interpolation. Bracket drift is only
0.4–5.9%, the bootstrap confidence intervals do not overlap, and the render
control remains flat. Attribution to the Opaque Reference Graph feature does
not satisfy this feature's predeclared integrated-head gate.

The results correctly offer three closure paths—fix the setup cost, record an
owner-approved re-threshold, or keep this feature blocked—but no owner decision
is recorded. Acceptance criteria 5 and 6 therefore remain unmet.

### High — Required Windows behavioral evidence is absent

F17's child wait/timeout behavior and F22's directory traversal/hash membership
are OS-divergent paths for which the specification explicitly rejects a
cross-compile as behavioral evidence. macOS and real-Linux runs exist, and the
Windows target compiles, but there is no Windows-host execution. The same F17
test binary also retains unrelated Unix-command tests, so the documented
Windows command cannot yet be expected to run green as a complete binary.

This is a verification gap at the appropriate Level 1, not a request for Level
2 or Level 3. It blocks compatibility invariant 8 and acceptance criterion 6.

### High — The remediations add public API despite the compatibility invariant

Compatibility invariant 2 permits no new public Rust API shape. The F35.5 fix
adds public inherent methods `Markdown::diff_hash` and
`Markdown::plan_hash_save_explained` in
`darkmatter/lib/src/markdown/hash/{explain,save}.rs`. The F2 proof also adds the
public Cargo feature `osc-query-counter` and publicly re-exports
`discovery::osc_queries::actual_query_count`; feature-gating a public item does
not make it non-public. The source comment claiming that it is not released API
is therefore incorrect.

No compatibility exception is recorded for either change. Keep the
instrumentation and shared-artifact operation crate-private/test-only through a
non-public harness seam, or obtain and document an explicit owner ruling before
release.

### High — Approval-handler and policy-write failures leak shell reservations

`prepare_directive` reserves every unapproved command before calling the
approval handler. At `shell_expansion/mod.rs:323`, `handler.approve(request)?`
can return early without completing any reservation. The persistence arms have
the same problem: `append_whitelist_exact`, `append_whitelist_prefix`, or
`append_blacklist_exact` can fail after only part of a chain has been released,
leaving later commands in `pending_allow_once`.

A later composition of the same command can then wait for the full 30-second
`RESERVATION_WAIT_TIMEOUT` or receive a spurious approval conflict. No test
injects a handler error or a policy-store write failure and asserts that every
reservation is released and waiters are notified. Use an RAII reservation
guard or a single cleanup path, and add same-command waiter tests for each error
class.

### High — F2's Level-2 proof is not theme-independent and Linux is still Level 1

The new WezTerm test is genuinely Level 2: a real emulator answers OSC 10, and
the current gate passes. However, its only proof that the answer came from the
terminal is `reported_rgb != (229, 229, 229)`, the library's WezTerm fallback.
A legitimate user theme may use exactly that foreground, causing a false
failure even when the terminal answered. The test neither sets a known pane
foreground nor independently captures the terminal's configured value, so it
does not meet the specification's “without depending on a user's shell theme”
requirement.

Moreover, Linux evidence runs the manufactured-response PTY test, which is
Level 1. `results.md` acknowledges that real-terminal Level 2 on Linux remains
open, while the spec's audit table still claims macOS and real-Linux Level 2.
This requirement-level mismatch is a production blocker. Establish a
deterministic foreground in a supported Linux and macOS emulator, verify the
wire-derived value, restore it afterward, and retain both runs.

### High — Several benchmark dispositions still lack retained raw samples

The new vectors for F13, F14, F33, and F35.2 recompute successfully. The
repository still has no raw observations for F23, F25, F35.3, F35.5, F35.6, or
F35.7; their temporary harnesses were deleted and the retained text contains
only derived statistics or prose. `results.md` correctly calls these
observations unrecoverable, but review 1's implementation-status preface calls
the finding closed.

The evidence contract requires identical source/fixture/harness bytes and
retained raw samples for every checkpoint, including no-win dispositions. In
particular, the newly changed F35.5 implementation has no reproducible
measurement against its current code. Rebuild pinned harnesses, repeat the
target/control measurements, and retain the raw vectors and recomputation
command.

### Medium — Closeout documentation contradicts the current implementation

The final audit is not an honest single source of truth as required by
acceptance criterion 7:

- the spec table claims Linux Level-2 F2 evidence, while `results.md` correctly
  says Linux has Level 1 only;
- `results.md` still lists F35.5 as computing the diff artifact twice, although
  the current implementation adds shared-artifact methods to remove that work;
- review 1's status section marks the raw-sample finding closed while
  `results.md` lists six unreproducible checkpoints; and
- the Windows runbook names the retired
  `fast_command_completion_is_not_delayed_by_a_poll_interval` test.

Reconcile the spec, review status, results, and runbook after resolving the code
and evidence findings.

## Requirement-to-verification assessment

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| F2: repeated terminal construction reuses one OSC 10 result | Level 1 manufactured PTY on macOS/Linux; Level 2 real WezTerm on macOS | **Gap.** Correct level exists only on macOS and its oracle depends on the user's foreground theme. Linux remains at the wrong level. |
| F3: one `md compose` invocation performs one terminal detection | Level 1 spawned CLI with debug-event count | Appropriate for the process-local `OnceCell`; no terminal encoder or rendering claim requires Level 2/3. |
| F21: redirected compose does not spawn macOS appearance discovery | Level 1 spawned CLI with a PATH sentinel; F2 supplies the interactive Level-2 counterpart | Appropriate for the no-spawn assertion. |
| F17: fast completion, saturation, timeout, kill/reap, and failure selection | Level 1 real child processes on macOS and Linux | Appropriate level, but required Windows execution is missing. |
| F22: directory membership, aggregate hash, and exit status | Level 1 library and spawned-CLI tests on macOS and Linux | Appropriate level, but required Windows execution is missing. |
| F23: theme remains dynamic between renders and output remains stable | Unit/snapshot tests, headless-browser tests, and the area Level-2 terminal suite | Appropriate rendering levels; the performance/no-win disposition still lacks raw samples. |
| F32: approval behavior remains compatible under concurrent composition | Level 1 unit/concurrency tests | Appropriate level, but handler/store error cleanup is uncovered and defective. |
| F35.5: hash explanation, persisted hash, and CLI exit behavior remain compatible | Level 1 library and spawned-CLI tests | Appropriate behavioral level; public-API compatibility and reproducible performance evidence fail. |

No requirement in this feature depends on terminal keyboard encoding, so no
Level-3 test is required.

## Verification performed for this review

- `darkmatter/just test`: 5,761 library, 559 CLI, and 566 DMLS tests passed;
- `biscuit-terminal/just test-l2`: 2 library and 76 CLI Level-2 tests passed;
- strict `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2`: the two F2 WezTerm tests
  passed, then the broader CLI tier stopped because Kitty remote control was not
  provisioned; the canonical non-strict area gate subsequently passed;
- retained raw vectors recomputed cleanly for F13, F14, F33, and F35.2; and
- GitNexus impact/change analysis and Sniff package/dependency discovery were
  used to establish the affected Darkmatter and Biscuit Terminal scope.
