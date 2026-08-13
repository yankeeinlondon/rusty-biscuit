---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-13T21:50:31+01:00
spec: 2026-08-13-finalize/spec.md
implemented: false
description: A **fix** review of `2026-08-13-finalize/spec.md`
fix: 2026-08-13-finalize/review-1.md
---

# Review 1 — Finalize

## Verdict

This fix is **not ready for production**. The effective-context, prompt-pin, path-projection, command-normalization, and latency work is broadly present and the local Level-1 suites pass. However, literal interpolation is not safe for all values required by the specification, its Claudine regression test does not reproduce the motivating Windows path shape, and the required native-Windows, repeated-Ubuntu, and CI-baseline closure evidence is absent.

## Findings

### Critical — The ratified CI baseline and identity-diff closure is not implemented

F8 and success criteria 5–6 require the ratified baseline policy to be encoded in `.github/ci/ci-baseline.toml`, followed by a full `ci-verdict` run and an identity-aware `ci-diff`. There is no baseline-file change, and `plan.md` still records the entire Phase 5 workflow as pending. Consequently, the merge gate still has the blocking cells this fix is intended to disposition, and there is no proof that only the intended cells changed.

Implement the Option A baseline changes only after collecting the required platform evidence, then run the full verdict and identity-aware diff and record every changed cell and its justification.

### High — Literal interpolation is not safe in the Markdown event model

`darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs` escapes ASCII punctuation only for `InterpolationContext::Prose`; inline-code replacements are inserted unchanged. A value containing a backtick can therefore terminate or split its surrounding code span. Prose values containing structural whitespace, such as blank lines followed by four-space indentation, can likewise create paragraphs or code blocks. Both outcomes violate F3's requirement that ordinary scalar interpolation preserve literal text and that only `raw_markdown(...)` intentionally create Markdown structure.

The current punctuation matrix covers single-line prose punctuation and the inline-code test uses a value without a backtick, so neither failure mode is exercised. Serialize values according to their Markdown context—or insert them directly into the event model—and add Level-1 tests for backtick runs in inline code plus multiline, blank-line, and indentation-bearing prose values.

### High — The Claudine end-to-end test does not cross the motivating `\\.` boundary

`ctx_launch_anchor_baseline.rs` creates the launch repository as `workspace_root/launch-repo`, rather than beneath a hidden segment such as `.tmpZZZ`. It therefore does not reproduce the Windows input that triggered the regression: a context path containing `\\.`. The test also compares captured Markdown source directly instead of parsing it and asserting the resulting text event. That conflates serialized source spelling with parsed literal semantics, which F3 explicitly requires the tests to distinguish.

Exercise the normal Claudine invocation with a Windows-shaped path containing a hidden segment, parse the captured Markdown, and assert the exact text/event value separately from source-level escaping. Retain the assertion that the projected path contains no verbatim `\\?\\` prefix.

### High — Required native-Windows Level-1 verification is missing

F3–F6 and success criterion 3 require the native Windows fixture and the ordinary Claudine path to pass on Windows. The implementation contains Windows-targeted tests and cross-platform helpers, but `plan.md` records the native Windows run as pending. Cross-compilation and macOS Level-1 results do not exercise Windows path prefixes, canonicalization behavior, or native process invocation.

Run the specified Level-1 suite on a native Windows host and retain the results as acceptance evidence. This is the appropriate verification level; Level 2 and Level 3 terminal testing are not required for these non-interactive behaviors.

### High — The required two consecutive Ubuntu latency runs are missing

F7 requires two consecutive standard two-core Ubuntu Level-1 runs demonstrating that both split subprocess tests remain below the budget. `latency.md` and `plan.md` explicitly record this evidence as outstanding. A local macOS pass cannot establish the acceptance criterion on the environment where the regression occurred.

Run the prescribed Ubuntu suite twice consecutively, record both elapsed times and per-phase diagnostics, and confirm that neither run relies on a retry or a relaxed threshold.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| F1 — effective context view | Level 1 unit/integration tests; passed locally on macOS | Appropriate and passing locally. |
| F2 — prompt hash pin | Level 1 drift/hash checks; pinned hash independently matches `md hash` | Appropriate and passing locally. |
| F3 — literal text interpolation | Level 1 unit and Claudine process tests | Insufficient: event-model edge cases are broken, the regression shape is absent, and native Windows has not run. |
| F4 — projected Windows paths | Level 1 unit/process tests | Appropriate level, but required native-Windows execution is missing. |
| F5 — native Windows fixture | Level 1 process fixture exists | Required native-Windows execution is missing. |
| F6 — normalized platform command | Level 1 unit/process tests | Appropriate level, but native-Windows execution is missing. |
| F7 — latency regression | Level 1 subprocess tests; passed locally on macOS | Required two consecutive Ubuntu runs are missing. |
| F8 — CI baseline policy | No completed baseline/verdict/identity-diff evidence | Not implemented. This is CI policy verification rather than a Level 1/2/3 terminal interaction. |

No requirement in this fix depends on terminal rendering or terminal input encoding, so Level 2 real-terminal capture and Level 3 OS keyboard injection would not add relevant assurance. The gaps are missing or incomplete Level-1 platform/process coverage and missing CI-policy closure.

## Verification Performed

- `darkmatter/just test`: passed all package gates locally on macOS.
- `claudine/just test`: passed all package gates locally on macOS; one linker warning was non-fatal.
- `md hash prompts/_implement/implement-plan.md`: produced `62d70fb16a02592c-652e691e678f8b5a`, matching the updated pin.
- Static review of the specification, implementation diff, tests, `plan.md`, `latency.md`, and CI baseline.

## Code Quality and Performance

The effective-context abstraction, typed projected-path handling, shared normalized command, and split latency tests are directionally sound and reduce duplicated platform logic. No additional performance blocker was found beyond completing the Ubuntu acceptance runs. Correct event-model-safe interpolation should be addressed before further ergonomic refinement.
