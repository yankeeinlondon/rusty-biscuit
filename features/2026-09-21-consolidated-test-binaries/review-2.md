---
$schema: feature-review.yaml
ready: false
findings:
  - title: Raw C strings reopen the dormant-macro false negative
    priority: high
human_review: true
human_review_items:
  - |-
    Confirm whether the four machine-readable migration manifests should remain the sole authoritative mapping of old test programs to their consolidated targets.

    - **A — Keep the manifests as the sole authority (recommended).** Every automated comparison reads the same mapping, avoiding a second record that can drift.
    - **B — Require an additional hand-reviewed mapping.** This adds an independent review of more than 3,800 identities, but creates a second source that must be maintained.

    Select A or B. No implementation change is needed for A.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-22T13:10:37-07:00"
spec: 2026-09-21-consolidated-test-binaries/spec.md
implemented: true
implemented_by: claude/opus
log: features/2026-09-21-consolidated-test-binaries/implementation-log.md
description: "A **fix** review of `2026-09-21-consolidated-test-binaries/spec.md`"
fix: 2026-09-21-consolidated-test-binaries/review-2.md
previous: 2026-09-21-consolidated-test-binaries/review-1.md
next: 2026-09-21-consolidated-test-binaries/review-3.md
---

# Review 2

**Not production-ready.** Review 1's consolidation and shared-walker changes
are present, and the previously blocked native-Windows and WSL2 verification is
now complete. However, the high-priority macro-token finding is not fully
closed: another stable Rust literal form can desynchronize the textual lexer
and make an uncompiled module look reachable.

The previous review did not contain formal `Unblocked Findings` or `Blocked
Findings` sections. It contained one implementation finding and two human
review items. The Windows/WSL2 item was unblocked before this implementation
and is now evidenced for all four packages. The migration-manifest authority
decision remains open and is carried forward above; it does not determine the
implementation-readiness verdict.

## Findings

### High — Raw C strings reopen the dormant-macro false negative

The review-1 fix correctly centralizes all four package guards on
`test_toolkit::test_layout`, blanks ordinary macro token trees, and adds parser-
and layout-level regressions. The follow-up `\xNN` character-literal fix also
closes the additional case recorded in the implementation log. But
`raw_string_open` recognizes `r#"…"#` and `br#"…"#`, not Rust's stable raw C
string form `cr#"…"#` (`tools/test-toolkit/src/test_layout.rs:392-409`).

A valid dormant invocation such as this still defeats the gate:

```rust
macro_rules! discard { ($($token:tt)*) => {}; }
discard! { cr#""}""#; mod orphan; }
mod guard;
```

The raw C string contains a quote, closing brace, and quote. Because the
sanitizer treats its quotes as ordinary strings, it leaves the embedded `}`
visible. `blank_macro_token_trees` therefore ends the invocation at that brace,
and `module_declarations` returns both `orphan` and `guard`. If `orphan.rs`
exists, `layout_violations` marks it reachable even though the macro discards
the tokens and rustc never compiles the file. This is the same acceptance-12
false negative as review 1, reached through a literal prefix that the new tests
do not cover.

The probe was compiled successfully with the repository toolchain, then added
temporarily to `a_module_token_inside_a_macro_declares_nothing`. The test
failed with `left: ["orphan", "guard"]` and `right: ["guard"]`; the probe was
removed after diagnosis. Extend the sanitizer to recognize every supported raw
literal prefix, including `cr`, and retain this exact case as both a parser
regression and a whole-layout regression. Given that review 1 and its first
implementation each found a different lexer omission, using a Rust lexer for
literal/comment boundaries would be the more durable fix if it can be added
without materially increasing the guard's dependency or compile cost.

## Previous-review disposition

- **Implementation finding:** partially implemented. The shared walker and the
  ordinary dormant-macro regressions are correct, but the raw C case above
  preserves the original false-negative behavior.
- **Native Windows and WSL2 item:** unblocked and closed before this
  implementation. `acceptance.md` records native Windows and WSL2 runs for all
  four packages, with the one Darkmatter failure identified as pre-existing on
  the unmigrated base.
- **Migration-manifest authority item:** still awaiting a human decision. The
  four manifests remain the single input to the automated checks.
- **Ordinary CI producer observations:** still pending by design. Acceptance
  criterion 10 forbids triggering a run solely to collect them, and the spec
  defines them as observations rather than a pass/fail gate.

## Requirement-to-verification map

This feature changes test compilation and discovery, not terminal interaction.
It adds no keyboard, mouse, paste, IME, styling, or hotkey behavior. L1 is the
appropriate level for the migration metadata, identity, and layout contracts;
the existing L2/L3/browser tests retain responsibility for the user-facing
behaviors inside the moved modules.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC3 — explicit targets, identity, tiers, and feature contracts | L1 metadata check and exact macOS/Linux Nextest-list comparisons | Appropriate and passing. Metadata maps all 139/74/53/38 former targets into 4/5/2/2 declared targets. |
| AC4 — platform conditions and reachability | L1 attribute audit, macOS/Linux identity comparisons, native-Windows suite runs, and WSL2 archive runs | Appropriate and now complete under the author's narrowed Windows criterion. |
| AC5–AC7 — structural edits, snapshots, and path guards | L1 mechanical reports plus reviewer inspection | Appropriate. No new migration-body or snapshot issue was found in this iteration. |
| AC8 — canonical suites and resource contracts | L1/L2/browser evidence retained from the migration; L3 and real-provider gates unchanged | Appropriate. Consolidation changes reachability, not the terminal encoder or rendering behavior exercised by those tests. |
| AC9 — pilot performance guardrail | Matched clean-build and edit-loop measurements | Appropriate and within the specified split threshold. |
| AC10 — producer observations | No qualifying ordinary CI run yet | Pending by design and not a readiness gate. |
| AC11 — selector and process-isolation documentation | L1 active-document sweep and direct inspection | Appropriate and passing. |
| AC12 — reject stray roots and undeclared modules | L1 shared-walker unit tests and live guards in all four packages | **Gap:** a stable raw C string can expose a dormant macro's `mod` token and hide an orphaned source file. |

## Verification performed

- Read the specification, review 1, implementation log, acceptance record,
  migration metadata, shared walker, its unit tests, and every live package
  guard.
- Refreshed the `rusty-biscuit` GitNexus index at `eaf73d54e`. Named context
  shows `module_declarations` feeding `layout_violations`; upstream impact is
  low for the private parser. GitNexus could not resolve external callers of
  the public walker, so text search confirmed the Claudine, Darkmatter,
  Darkmatter CLI, and Biscuit Terminal consumers instead of treating the
  unknown result as absence.
- `acceptance/metadata-check.py`: passed for all four packages.
- `selfproof/mutation-check.py`: all eight mutants were detected.
- `just test` and `just lint` in `tools/`: 341 passed, 2 expected tier-filter
  skips; lint passed.
- `just test-cli test_placement::` in `claudine/`: 12 passed.
- `just test test_layout::` in `darkmatter/`: 2 passed.
- `just test test_layout::` in `biscuit-terminal/`: 1 passed.
- Compiled the raw C probe successfully, then confirmed the temporary
  parser-level regression fails as described above.
- `git diff --check`: passed before writing this review.

No L2, L3, browser, or GUI-backed test was launched during this iteration. The
changed code is an L1 structural guard; existing cross-platform and
real-terminal evidence was inspected for the unchanged higher-level behavior.

## Production readiness

Not ready for production. Close the raw C literal hole and retain a regression
that proves an orphan named only inside that dormant macro remains a violation.
The remaining human decision about manifest authority and the eventual CI
producer observations are external follow-ups, not causes of this readiness
verdict.
