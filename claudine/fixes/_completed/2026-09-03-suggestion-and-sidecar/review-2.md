---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-03T21:30:20+01:00
spec: suggestion-and-sidecar/spec.md
implemented: false
description: A **fix** review of `suggestion-and-sidecar/spec.md`
fix: suggestion-and-sidecar/review-2.md
previous: suggestion-and-sidecar/review-1.md
---

# Review 2: Suggestion and Sidecar

## Verdict

The fix is **ready for production**. The only finding from Review 1 is resolved:
the production repository walk now counts its depth-zero root, yielded errors,
and ordinary entries against one 20,000-item bound, and the regression test
fails if the collector polls item 20,001. No new correctness, ergonomics,
performance, or verification-level gaps were found.

## Findings

None.

## Review 1 Closure

The walker maps every raw iterator result into `SuggestionWalkItem`, including
an explicit `Root` item, before the collector applies
`.take(SUGGESTION_ENTRY_BUDGET)`. The instrumented boundary test includes a
root item, an error, and ordinary entries; retains a match at visit 20,000; and
panics if a 20,001st item is requested. This closes Review 1's production-seam
budget mismatch without changing suggestion selection or the primary
file-resolution diagnostic.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: an earlier dangling symlink does not suppress sorted, capped suggestions in rendered and structured diagnostics | Level 1 Unix real-filesystem fixture plus in-process render/detail assertions | Appropriate and present. This does not depend on terminal-emulator rendering or input encoding. |
| AC2: first/middle entry errors are skipped and budgeted; visit 20,001 is not consumed; in-budget matches survive; unusable roots return no suggestions | Level 1 synthetic raw-walk iterator with exact polling instrumentation plus real-filesystem root tests | Appropriate and present. The corrected test now exercises the depth-zero-root accounting omitted in Review 1. |
| AC3: directory symlinks remain pruned | Level 1 OS-specific filesystem tests on Unix and Windows | Appropriate and present. OS-specific tests remain L1 because they require no real terminal or OS input injection. |
| AC4: a simplified-looking bare sidecar remains raw JSON Schema and emits exactly one advisory across one or two validation passes | Level 1 resolver, compose-report, and composed-document tests | Appropriate and present. Tests also prove the document remains unconstrained and unchanged. |
| AC5: raw JSON Schema keywords, reserved/custom vocabularies, supported envelopes, scalar references, and mixed/invalid maps do not warn | Level 1 resolver and validation-entry-point fixture matrices | Appropriate and present. |
| AC6: one typed advisory reaches Darkmatter validation and composition, Claudine stderr, `md schema validate` pretty/JSON output, and the consuming DMLS `$schema` range without changing validity or exit status | Level 1 library, CLI subprocess, and LSP protocol tests | Appropriate and present. The CLI tests cover channel placement and structured identity; the LSP test covers range, source, code, severity, deduplication, and dependency-cache invalidation. |
| AC6: Claudine `--silent` and `md schema validate --quiet` suppress successful advisory output | Level 1 CLI subprocess tests | Appropriate and present. |
| AC7: macOS, Windows, and Linux compatibility | Level 1 package suites on macOS plus the implementation cycle's native Windows and Linux `claudine-cli` cross-checks | Appropriate and present for the platform-independent collector and platform-specific symlink containment paths. |

Levels 2 and 3 are not applicable. The requirements concern deterministic
filesystem traversal, typed diagnostic data, captured text channels, and LSP
messages. They do not depend on terminal glyph layout, emulator input encoding,
or physical keyboard/mouse events.

## Additional Review Notes

The schema classifier remains conservative: it reuses the passive
SimplifiedSchema parser, rejects recognized Draft 2020-12 keywords and reserved
`$`/`x-` vocabularies, performs no additional I/O, and leaves the input
interpreted as raw JSON Schema. Advisories retain one typed semantic identity,
are sorted and deduplicated through schema assembly, and are projected by each
consumer without reparsing. Documentation and the Claudine/Darkmatter skill
snapshots describe the implemented envelope and skipped-entry contracts.

## Verification Performed

- `claudine/ just test`: **6,716 passed; 11 configured tests skipped**.
- `claudine/ just lint`: **passed** for all five Claudine packages and the
  diagnostic/documentation guards.
- `darkmatter/ just test`: **7,633 passed; 50 configured tests skipped**.
- `darkmatter/ just lint`: **passed** for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- Implementation-cycle native Windows `claudine-cli` Level 1 cross-check:
  **2,069 passed; 8 skipped**.
- Implementation-cycle native Linux `claudine-cli` Level 1 cross-check:
  **2,431 passed; 9 skipped**.

## Production Readiness

AC1–AC7 are satisfied with the appropriate verification level. The Review 1
budget defect is closed at the production collector seam, so this fix is ready
for production.
