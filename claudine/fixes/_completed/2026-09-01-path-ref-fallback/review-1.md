---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-02T00:10:48+01:00
spec: 2026-09-01-path-ref-fallback/spec.md
implemented: false
description: "A **fix** review of `2026-09-01-path-ref-fallback/spec.md`"
fix: 2026-09-01-path-ref-fallback/review-1.md
---

# Review 1: Path Reference Fallback

## Verdict

Ready for production.

## Findings

No production-blocking findings.

The implementation follows the specification's intended boundaries:

- `FileReference` remains the grammar and candidate-resolution authority.
- Only clean no-matches for bare, non-recursive, single-component implicit
  references reach the existing picker.
- Explicit misses retain the authored reference, resolver classifications,
  launch directory, repository root, and ordered probe record under the
  established `composition.invalid_file_reference` identity.
- Repository basename suggestions are exact-case, repository-relative,
  ignore-aware, deterministically ordered, deduplicated, capped at five, and
  abandoned without replacing the primary diagnostic on walk failure or the
  20,000-entry budget.
- Compose, inline-compose, Markdown sequence, and YAML sequence use the same
  recovery decision. The shared `path_matches_query` behavior is unchanged.
- The terminal report uses `Prose` and `UnorderedList` and renders paths through
  `biscuit_file::to_portable_string`.
- The authoritative completion topic and the portable Claudine-skill snapshot
  contain the same changed three-outcome passage.

## Requirement-to-Verification Map

| Requirement | Strongest verification | Assessment |
|---|---|---|
| AC1: explicit references produce the typed no-match for compose, inline-compose, and sequence | L2 tmux for inline-compose and sequence; L1 routing/projection for compose and both sequence formats | Appropriate and present |
| AC2: reference forms route intentionally and non-no-match failures retain their typed identity | L1 cross-platform classification table plus typed resolver-error tests | Appropriate and present |
| AC3: bare names retain no-match, over-cap, cancellation, confirmation, and chooser behavior | L2 tmux/WezTerm picker tests, supported by L1 failure-path tests | Appropriate and present |
| AC4: human and machine diagnostics preserve candidate and suggestion order and the complete catalog shape | L1 diagnostic projection/render tests; L2 real-terminal capture of candidate rendering | Appropriate and present |
| AC5: basename suggestions are exact, bounded, deterministic, filtered, and symlink-safe | L1 walker and pure collector tests | Appropriate and present |
| AC6: explicit failure is TTY-independent and never waits for picker input | L2 self-isolating tmux test paired with a non-PTY process run | Appropriate and present |
| AC7: all four operation modes share the policy | L1 direct/inline/Markdown-sequence/YAML-sequence tests; L2 inline-compose and sequence tests | Appropriate and present |
| AC8: Windows spellings classify on every host and displayed paths are portable | L1 host-independent classification and portable-render tests | Appropriate and present |
| AC9: shared query matching remains unchanged | L1 existing completion/schema matcher suite plus diff inspection | Appropriate and present |
| AC10: documentation describes omitted, bare, and explicit outcomes consistently | L1 byte comparison of the changed passage plus direct inspection | Appropriate and present |

Level 3 is not required. This fix does not assert what bytes a terminal emits
for a physical key event; picker interaction is already exercised through real
terminal emulators at Level 2, while the changed explicit-miss path must not
accept input at all.

## Verification

Run from `claudine/`:

```text
just test
just test-l2
just lint
```

Results:

- Level 1: 6,664 passed; 11 skipped.
- Level 2 `claudine-cli`: 232 passed; 2,414 skipped.
- Level 2 `claudine-gen`: 3 passed; 155 skipped.
- Lint and diagnostic guards: passed for every Claudine package.

## Production Readiness

The fix satisfies the acceptance criteria at the verification levels required
for each user-observable behavior and is ready for production.
