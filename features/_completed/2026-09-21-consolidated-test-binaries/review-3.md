---
$schema: feature-review.yaml
ready: false
findings:
  - title: Macro spacing still lets dormant modules pass the layout gate
    priority: high
human_review: true
human_review_items:
  - |-
    Confirm whether the four machine-readable migration manifests should remain the sole authoritative mapping of old test programs to their consolidated targets.

    - **A — Keep the manifests as the sole authority (recommended).** Every automated comparison reads the same mapping, avoiding a second record that can drift.
    - **B — Require an additional hand-reviewed mapping.** This adds an independent review of more than 3,800 identities, but creates a second source that must be maintained.

    Select A or B. No implementation change is needed for A.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-22T13:31:05-07:00"
spec: 2026-09-21-consolidated-test-binaries/spec.md
implemented: true
implemented_by: claude/opus
log: features/2026-09-21-consolidated-test-binaries/implementation-log.md
description: "A **fix** review of `2026-09-21-consolidated-test-binaries/spec.md`"
fix: 2026-09-21-consolidated-test-binaries/review-3.md
previous: 2026-09-21-consolidated-test-binaries/review-2.md
next: 2026-09-21-consolidated-test-binaries/review-4.md
---

# Review 3

**Not production-ready.** Review 2's raw C string finding is implemented with
both parser-level and whole-layout regressions, and its focused L1 gates pass.
However, the same acceptance-12 guard still treats valid tokens inside some
dormant macros as compiled module declarations. That can let a new test source
remain silently uncompiled while every live package guard passes.

Review 2 had no formal `Unblocked Findings` or `Blocked Findings` sections. Its
one implementation finding is closed. The migration-manifest authority decision
remains open and is carried forward above; it does not determine implementation
readiness. Native Windows and WSL2 verification had already been unblocked and
closed before the review-2 implementation.

## Findings

### High — Macro spacing still lets dormant modules pass the layout gate

`blank_macro_token_trees` recognizes a macro only when `!` is immediately
preceded by an identifier byte (`tools/test-toolkit/src/test_layout.rs:285-289`).
Rust also accepts whitespace between the name and `!`. This valid source
therefore leaves both the macro definition and invocation bodies visible to
`module_declarations`:

```rust
macro_rules ! discard { ($($token:tt)*) => {} }
discard ! { mod orphan; }
mod guard;
```

The source compiles with the repository toolchain, but a temporary regression
in `a_module_token_inside_a_macro_declares_nothing` failed with
`left: ["orphan", "guard"]` and `right: ["guard"]`. At layout level, an
existing `orphan.rs` would consequently be marked reachable even though the
discarding macro never expands to that declaration.

The definition-name scan has a related hole: it consumes only identifier bytes
after `macro_rules!` (`tools/test-toolkit/src/test_layout.rs:290-298`). A valid
raw-identifier definition such as
`macro_rules! r#type { () => { mod orphan; }; }` stops at `#`, leaves its body
unblanked, and produces the same incorrect declaration list. This probe also
compiled and failed the temporary parser regression with the same values.

This is a high-priority readiness defect because acceptance 12 is the safeguard
against Cargo silently omitting test files after `autotests = false`. Extend the
macro parser to accept Rust's legal trivia and raw identifiers, and retain both
forms as parser-level and whole-layout regressions. Since three consecutive
iterations have exposed tokenization omissions, a token-aware implementation is
preferable if it can preserve byte offsets and the guard's lightweight build
contract without material cost.

## Previous-review disposition

- **Raw C string implementation finding:** implemented. `raw_string_open`
  recognizes `cr` as well as `r` and `br`; the exact hashed raw C case and a
  zero-hash escape-sensitive case are retained, and the whole-layout regression
  proves the orphan remains a violation.
- **Migration-manifest authority item:** still awaiting a human decision. The
  four manifests remain the sole input to the automated comparisons.
- **Ordinary CI producer observations:** still pending by design. Acceptance
  criterion 10 forbids triggering a run solely to collect them and does not make
  those observations a readiness gate.

## Requirement-to-verification map

This feature changes compilation and test discovery. It adds no keyboard,
mouse, paste, IME, styling, hotkey, or other user-facing terminal behavior, so
there is no new requirement whose correct boundary is L2 or L3. Existing
L2/L3/browser tests remain responsible for the behavior inside the moved test
modules.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC3 — explicit targets, identities, tiers, and feature contracts | L1 metadata check and exact macOS/Linux Nextest-list comparisons | Appropriate and passing. All 139/74/53/38 former targets map into 4/5/2/2 declared targets. |
| AC4 — platform conditions and reachability | L1 attribute audits, macOS/Linux identity comparisons, native-Windows suite runs, and WSL2 archive runs | Appropriate and complete under the author's narrowed Windows criterion. |
| AC5–AC7 — structural edits, snapshots, and path guards | L1 mechanical reports plus the prior review's direct inspection | Appropriate. No new migrated-test-body or snapshot change was introduced by the review-2 fix. |
| AC8 — canonical package suites and resource contracts | L1/L2/browser migration evidence; unchanged L3 and real-provider gates | Appropriate. Consolidation changes reachability, not terminal rendering or input encoding. |
| AC9 — pilot performance guardrail | Matched clean-build and edit-loop measurements | Appropriate and within the specified split threshold. |
| AC10 — producer observations | No qualifying ordinary CI run yet | Pending by design; not a readiness gate. |
| AC11 — selector and process-isolation documentation | L1 active-document sweep and direct inspection | Appropriate and passing. |
| AC12 — reject stray roots and undeclared modules | L1 shared-walker unit tests and live guards in all four packages | **Gap:** valid whitespace and raw-identifier macro syntax can expose dormant `mod` tokens and hide an orphaned test source. |

## Verification performed

- Read the specification, review 2, implementation log, acceptance record,
  shared layout walker, its tests, and the live package guard call sites.
- Bound GitNexus to the current `rusty-biscuit` worktree index at `eaf73d54e`.
  `raw_string_open` has low, exact upstream impact through `sanitize`,
  `module_declarations`, and `layout_violations`. The public walker's graph
  impact was unknown, so text search confirmed the Claudine, Darkmatter,
  Darkmatter CLI, and Biscuit Terminal consumers.
- Compiled the whitespace-before-`!` and raw-identifier macro probes with the
  repository toolchain. Temporary parser regressions failed with the incorrect
  `orphan` declaration in both cases; those temporary edits were removed.
- `just test` in `tools/`: 342 passed, 2 expected tier-filter skips.
- `just lint` in `tools/`: passed.
- `just test-cli test_placement::` in `claudine/`: 12 passed.
- `just test test_layout::` in `darkmatter/`: 2 passed.
- `just test test_layout::` in `biscuit-terminal/`: 1 passed.
- `acceptance/metadata-check.py`: passed for all four packages.
- `selfproof/mutation-check.py`: all eight mutants were detected.

No L2, L3, browser, or GUI-backed test was launched in this iteration. The only
implementation change under review is the L1 structural parser, and this review
does not alter the moved terminal/browser behavior.

## Production readiness

Not ready for production. Close the remaining macro-token false negatives and
retain regressions proving an orphan named only inside each dormant macro form
is rejected. The human decision about manifest authority and later CI producer
observations remain external follow-ups rather than causes of this verdict.
