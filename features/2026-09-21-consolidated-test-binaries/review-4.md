---
$schema: feature-review.yaml
ready: false
findings:
  - title: Unicode macro identifiers still let dormant modules pass the layout gate
    priority: high
human_review: true
human_review_items:
  - |-
    Confirm whether the four machine-readable migration manifests should remain the sole authoritative mapping of old test programs to their consolidated targets.

    - **A — Keep the manifests as the sole authority (recommended).** Every automated comparison reads the same mapping, avoiding a second record that can drift.
    - **B — Require an additional hand-reviewed mapping.** This adds an independent review of more than 3,800 identities, but creates a second source that must be maintained.

    Select A or B. No implementation change is needed for A.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-22T13:44:39-07:00"
spec: 2026-09-21-consolidated-test-binaries/spec.md
implemented: true
implemented_by: claude/opus
log: features/2026-09-21-consolidated-test-binaries/implementation-log.md
description: "A **fix** review of `2026-09-21-consolidated-test-binaries/spec.md`"
fix: 2026-09-21-consolidated-test-binaries/review-4.md
previous: 2026-09-21-consolidated-test-binaries/review-3.md
next: 2026-09-21-consolidated-test-binaries/review-5.md
---

# Review 4

**Not production-ready.** Review 3's whitespace, comment-trivia, qualified-path,
raw-identifier, and non-macro-bang cases are implemented and its focused L1
gates pass. The replacement scanner still treats Rust identifiers as ASCII,
however, so a valid Unicode macro definition or invocation exposes dormant
`mod` tokens to the module walker. That can let a new test source remain
silently uncompiled while every live package guard passes.

Review 3 had no formal `Unblocked Findings` or `Blocked Findings` sections. Its
one implementation finding is closed for the cases it named. No blocked
finding became actionable before the implementation: migration-manifest
authority remains a human decision, and ordinary CI producer observations
remain pending by design rather than blocked implementation work.

## Findings

### High — Unicode macro identifiers still let dormant modules pass the layout gate

Rust accepts Unicode identifiers, including macro names such as `café`. The
layout scanner's shared `is_ident` helper accepts only ASCII alphanumerics and
underscore (`tools/test-toolkit/src/test_layout.rs:230-232`). Both sides of the
new macro recognizer inherit that restriction:

- For `macro_rules! café { () => { mod orphan; }; }`, `identifier_end` consumes
  only the ASCII prefix `caf`, does not reach the opening delimiter, and leaves
  the definition body visible to `module_declarations`
  (`tools/test-toolkit/src/test_layout.rs:297-311,369-376`).
- For `café! { mod orphan; }`, `macro_path_before` stops immediately on the
  final non-ASCII UTF-8 byte and returns `None`, leaving the invocation body
  visible (`tools/test-toolkit/src/test_layout.rs:342-366`).

The repository toolchain compiled the Unicode `macro_rules! café` definition.
Temporary cases added to `a_module_token_inside_a_macro_declares_nothing`
failed independently for both the definition and invocation with
`left: ["orphan", "guard"]` and `right: ["guard"]`. At whole-layout level, an
existing `orphan.rs` is therefore marked reachable even though the dormant
macro does not compile it. The temporary probes were removed.

This is a high-priority readiness defect because acceptance 12 is the safeguard
against Cargo silently omitting test files after `autotests = false`. Replace
the ASCII approximation with Rust-compatible identifier tokenization (or a
token-aware implementation) and retain parser-level and whole-layout
regressions for both a Unicode macro definition name and invocation path.

## Previous-review disposition

- **Review 3 implementation finding:** implemented for whitespace around `!`,
  comment/newline trivia, raw definition names, qualified paths, and negative
  unary/operator cases. The new parser and layout regressions pass.
- **Migration-manifest authority item:** still awaiting a human decision. The
  four manifests remain the sole input to the automated comparisons.
- **Ordinary CI producer observations:** still pending by design. Acceptance
  criterion 10 forbids triggering a run solely to collect them and does not
  make those observations a readiness gate.

## Requirement-to-verification map

This feature changes compilation and test discovery. It adds no keyboard,
mouse, paste, IME, styling, hotkey, or other user-facing terminal behavior, so
there is no new requirement whose correct verification boundary is L2 or L3.
Existing L2/L3/browser tests remain responsible for behavior inside the moved
test modules.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC3 — explicit targets, identities, tiers, and feature contracts | L1 metadata check and exact macOS/Linux Nextest-list comparisons | Appropriate and passing. All 139/74/53/38 former targets map into 4/5/2/2 declared targets. |
| AC4 — platform conditions and reachability | L1 attribute audits, macOS/Linux identity comparisons, native-Windows suite runs, and WSL2 archive runs | Appropriate and complete under the author's narrowed Windows criterion. |
| AC5–AC7 — structural edits, snapshots, and path guards | L1 mechanical reports plus prior direct review | Appropriate. Review 3 changed only the shared structural walker and its tests. |
| AC8 — canonical package suites and resource contracts | L1/L2/browser migration evidence; unchanged L3 and real-provider gates | Appropriate. Consolidation changes reachability, not terminal rendering or input encoding. |
| AC9 — pilot performance guardrail | Matched clean-build and edit-loop measurements | Appropriate and within the specified split threshold. |
| AC10 — producer observations | No qualifying ordinary CI run yet | Pending by design; not a readiness gate. |
| AC11 — selector and process-isolation documentation | L1 active-document sweep and direct inspection | Appropriate and passing. |
| AC12 — reject stray roots and undeclared modules | L1 shared-walker unit tests and live guards in all four packages | **Gap:** valid Unicode macro names expose dormant `mod` tokens and can hide an orphaned test source. |

## Verification performed

- Read the specification, review 3, implementation log, acceptance record,
  shared layout walker, its tests, and all four live package guard call sites.
- Bound GitNexus to the current `rusty-biscuit` worktree and refreshed the
  stale index with `just gitnexus`; it then covered all 7,406 files. Upstream
  impact for the private macro pass and public walker remained `UNKNOWN`, so a
  text search confirmed the Claudine, Darkmatter, Darkmatter CLI, and Biscuit
  Terminal consumers.
- Compiled a Unicode macro-name probe with the repository Rust toolchain.
  Temporary definition and invocation regressions each failed with the
  incorrect `orphan` declaration; those edits were removed.
- `just test` in `tools/`: 344 passed, 2 expected tier-filter skips.
- `just lint` in `tools/`: passed.
- `just test-cli test_placement::` in `claudine/`: 12 passed.
- `just test test_layout::` in `darkmatter/`: 2 passed.
- `just test test_layout::` in `biscuit-terminal/`: 1 passed.
- `acceptance/metadata-check.py`: passed for all four packages.
- `selfproof/mutation-check.py`: all eight mutants were detected.

No L2, L3, browser, or GUI-backed test was launched in this iteration. The only
implementation change under review is the L1 structural parser, and this review
does not alter or make new claims about moved terminal/browser behavior.

## Production readiness

Not ready for production. Make the layout guard recognize the full Rust
identifier grammar for macro definitions and invocations, and retain
parser-level plus whole-layout Unicode regressions. The human decision about
manifest authority and later CI producer observations remain external
follow-ups rather than causes of this verdict.
