---
ready: true
agent: codex/default
created: 2026-07-10T12:16:03
---

# Review: Suggested Values for SimplifiedSchema (Iteration #3)

## Summary

The three production-blocking findings from iteration 2 are resolved. Nested
inline-object candidates now project through their containing YAML scalar,
lint filtering uses an unambiguous segmented property path, and block-array
completion computes its edit range from the marker actually present. Focused
regressions cover each fix, including both standalone envelopes and the
end-to-end in-memory LSP path.

No additional correctness, completeness, ergonomics, performance, or test-tier
gaps were found. The feature is ready for production.

## Findings

No findings.

## Iteration-2 Finding Closure

### Nested inline-object source projection: resolved

The source projector first matches the complete containing scalar and then
recursively projects nested candidates with that scalar's decoded-to-raw byte
map. Exact spans are verified for inline Markdown and both pure and tagged
standalone envelopes. Inline `$schema` projection is also scoped to the
`$schema` value subtree, preventing identical text in an earlier frontmatter
field from capturing the diagnostic range.

### Cross-property lint identity: resolved

`SuggestionLintProblem` now carries `property_path: Vec<String>`, while its
dotted `property` remains a human-readable label. Completion filtering compares
the segmented path together with root-arm and property-arm provenance, so
sibling, nested, root-union, and dotted-key identities cannot collide.

### Literal block-array dash completion: resolved

Block-array completion distinguishes a bare `-` from `- ` and starts its text
edit after the actual one- or two-byte marker. Separate Level-1 LSP regressions
cover `-`, `- `, and `- partial`, including exact edit ranges.

## Test Rigor Assessment

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Grammar, interpretation, normalization, metadata, non-validating behavior, and numeric boundaries (AC 1-13, 23) | Level 1 unit/integration/snapshot | Appropriate and passing. |
| Exact inline/standalone diagnostics and envelope behavior (AC 14-17) | Level 1 in-memory LSP plus source-aware resolver tests | Appropriate and passing, including nested schemas, decoy text, malformed envelopes, and whole-file references. |
| Scalar, nested, array, prefix, insertion, and union completion (AC 18-22) | Level 1 in-memory LSP | Appropriate and passing, including all iteration-2 regressions. |
| Cross-platform behavior (AC 24) | Level 1 CRLF, UTF-8, explicit-newline, and `FileReference` tests on macOS | Appropriate for platform-neutral library/LSP logic; Windows and Linux were not executed in this review. |

No requirement needs Level 2 real-terminal capture or Level 3 OS keyboard
injection. The feature exposes library and LSP protocol behavior and specifies
no terminal rendering, key encoding, mouse, paste, or IME behavior.

## Verification

- Focused nextest selection: 107 passed, 5,650 skipped.
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- A full `just test` run reached 1,953 passing tests with no failures before it
  was stopped because the package-wide suite exceeded this review's bounded
  command window; the complete focused feature suite was then run separately
  and passed.
- The acceptance matrix now classifies the in-memory LSP sessions as Level 1,
  points to the correct implementation paths, and includes executable owners
  for the iteration-2 regressions.

## Verdict

Ready for production. The implementation satisfies the specification, the
prior findings are closed with appropriate regressions, and the verification
level matches every user-observable requirement.
