---
$schema: feature-review.yaml
ready: false
findings:
  - title: The layout guard can mistake dormant macro tokens for compiled modules
    priority: high
human_review: true
human_review_items:
  - |-
    After the implementation finding is fixed, choose how to handle the missing native-Windows and WSL2 verification before merging:

    - **A — Free at least 50 GiB on the Windows build drive and run the recorded Windows and WSL2 checks (recommended).** This proves that every test remains present on native Windows and that the Linux archive runs from a different path under WSL2.
    - **B — Merge with compile-only Windows evidence.** The normal post-merge CI run will execute Windows tests, but it will not perform the before-and-after identity comparison that detects a silently missing Windows-only test.
    - **C — Record native Windows and WSL2 as accepted gaps.** This closes the review honestly but weakens the specification's original acceptance criterion.

    The reviewer should select one option. For option A, confirm that the resulting comparison reports no lost, gained, reclassified, or newly ignored tests on native Windows and that the WSL2 archive check passes.
  - |-
    Confirm whether the four machine-readable migration manifests should remain the sole authoritative mapping of old test programs to their consolidated targets.

    - **A — Keep the manifests as the sole authority (recommended).** Every automated comparison reads the same mapping, avoiding a second record that can drift.
    - **B — Require an additional hand-reviewed mapping.** This adds an independent review of more than 3,800 identities, but creates a second source that must be maintained.

    The reviewer should choose A or B; no code change is needed for A.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-22T01:39:51-07:00"
spec: 2026-09-21-consolidated-test-binaries/spec.md
implemented: true
implemented_by: claude/opus
log: features/2026-09-21-consolidated-test-binaries/implementation-log.md
description: "A **fix** review of `2026-09-21-consolidated-test-binaries/spec.md`"
fix: 2026-09-21-consolidated-test-binaries/review-1.md
next: 2026-09-21-consolidated-test-binaries/review-2.md
---

# Review 1

**Not production-ready.** The migration itself is well evidenced on macOS and
Linux: target metadata, exact normalized identities, tier classification,
platform absence, feature boundaries, snapshots, and structural source changes
all have checked records. The canonical owner suites and each live layout guard
also pass. However, the new guard required to prevent future silent test loss
has a false-negative path: it recognizes textual `mod name;` tokens inside
macro token trees as compiled module declarations.

The pending native-Windows listing, WSL2 archive run, and ordinary CI producer
measurements do not determine this readiness verdict. They remain explicit
evidence gaps and human-review items, as requested by the closure rules.

## Findings

### High — The layout guard can mistake dormant macro tokens for compiled modules

The shared parser scans every sanitized byte offset for `mod name;` and does
not track whether the token is an actual Rust item or merely part of a macro
definition or invocation (`tools/test-toolkit/src/test_layout.rs:138-179`).
`layout_violations` then follows every returned declaration and marks the
matching file reachable (`tools/test-toolkit/src/test_layout.rs:97-109`).
Claudine has the same textual parser and graph walk in
`claudine/cli/tests/l1/test_placement.rs`.

For example, a declared root can contain a macro that never expands its body:

```rust
macro_rules! dormant {
    () => { mod orphan; };
}
```

Rust does not compile `orphan.rs` unless the macro expands at that location,
but both layout walkers report the `mod orphan;` token as a declaration and
therefore accept the file. A token-discarding invocation has the same problem.
This defeats acceptance criterion 12's purpose: with `autotests = false`, an
orphaned test file can be silently omitted while the guard stays green.

The tests prove that comments, strings, ordinary missing declarations, stray
roots, and nested helper directories are handled, but no fixture puts a module
token inside a dormant macro (`tools/test-toolkit/src/test_layout/tests.rs:47-63,
93-127`). Fix the walker so reachability follows Rust items rather than arbitrary
token text, or otherwise exclude unexpanded macro token trees. Add a regression
whose root contains a dormant macro naming a real orphan file and assert that
the orphan remains a violation. Apply the same proof to Claudine's local guard,
or move Claudine onto the corrected shared implementation so the two parsers
cannot drift.

## Requirement-to-verification map

This feature changes test compilation and discovery, not terminal UX. It adds
no keyboard, mouse, paste, IME, rendering, or hotkey behavior, so no new L2 or
L3 behavioral test is required. Existing user-facing terminal tests must remain
at their original levels and remain reachable; exact listing comparisons and
unchanged test bodies verify that packaging property, while the existing L2/L3
tests continue to own the underlying behavior.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1 — explicit targets and complete migration manifests | L1 Cargo metadata check plus each package's in-binary layout guard | Metadata and current layout pass; the guard has the macro-token false negative described above. |
| AC2 — exact identity and tier parity | L1 before/after Nextest JSON comparisons on macOS and Linux, with mutation-tested identity normalization | Appropriate and passing on the evidenced hosts; native Windows remains pending. |
| AC3 — exact required-feature contracts | L1 Cargo metadata-to-manifest comparison | Appropriate and passing, including Darkmatter's separate terminal and browser L3 targets. |
| AC4 — platform conditions and OS reachability | L1 attribute audit plus macOS/Linux compile-and-list comparisons; Windows cross-compile | Conditions are checked; native-Windows listing and WSL2 archive execution remain pending external evidence. |
| AC5 — structural-only source edits | L1 mechanical body diff plus reviewer inspection of every non-structural classification | Appropriate; inspected differences are path repairs, guard paths, selector instructions, comments/messages, or the required layout gate. |
| AC6 — snapshots preserved | L1 checked snapshot mapping, byte comparison, and `INSTA_UPDATE=no` suite evidence | Appropriate and passing; no `.snap.new` file is present. |
| AC7 — path guards retain coverage | L1 before/after enumerated path reports | Appropriate and passing for the migration. |
| AC8 — canonical suites and tier/resource contracts | L1 canonical suites, L2 real-terminal runs, browser-tier runs, and unchanged L3/real gates | Appropriate levels. The feature changes reachability rather than the user-facing behaviors inside those tests; no lower-tier substitution was found. |
| AC9 — pilot performance guardrail | Matched build/edit measurements with target count, size, time, and peak memory | Appropriate and within the specified split threshold. |
| AC10 — ordinary producer observations | No ordinary CI producer has selected the branch | Explicitly pending and not a readiness gate. |
| AC11 — active selector documentation and Nextest isolation contract | L1 active-document sweep plus direct inspection of recipes and the Rust testing skill | Appropriate and passing. |
| AC12 — reject stray roots and undeclared modules | L1 temporary-layout unit tests and live package guards | **Gap:** macro token trees can make an uncompiled module appear reachable. |

## Verification performed

- Read the specification, acceptance record, implementation log, migration
  manifests, package manifests and roots, source-difference report, measurement
  report, comparison reports, consolidation tool, shared layout walker, and
  Claudine's local walker.
- Refreshed the `rusty-biscuit` GitNexus index at `d3bbafbc5`. Broad process
  extraction was truncated, so absence of flows was not treated as evidence.
  Named context confirms `layout_violations` calls `module_declarations`;
  upstream risk is `UNKNOWN`, and text search confirms all four package guards.
- `python3 acceptance/metadata-check.py`: passed for 4/5/2/2 declared targets
  and all 139/74/53/38 old targets.
- `python3 acceptance/body-diff.py`: regenerated the checked source-change
  classification; results match the committed acceptance record.
- `python3 selfproof/mutation-check.py`: all eight mutants were detected.
- `just test repo-deps`: 449 passed, 1 expected tier-filter skip.
- `just test test-toolkit`: 339 passed, 2 expected tier-filter skips.
- `just test-cli test_placement::` in `claudine/`: 13 passed.
- `just test test_layout::` in `darkmatter/`: 2 passed.
- `just test test_layout::` in `biscuit-terminal/`: 1 passed.
- `git diff --check`: passed before writing this review.

No L2, L3, or browser window was launched during this review. Existing
committed L2/browser evidence was inspected, and the review did not seek new L3
evidence because the feature does not alter the underlying input behavior.

## Production readiness

Not ready for production. Correct the module reachability guard and add a
macro-token regression at every implementation boundary. After that, repeat
the review; the missing Windows/WSL2 and ordinary-CI observations remain
explicit human-review decisions rather than implementation-readiness blockers.
