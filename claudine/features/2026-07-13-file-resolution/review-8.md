---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T16:28:04-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-8.md
previous: 2026-07-13-file-resolution/review-7.md
---

# Review 8: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Review 7's absolute-path escape is
closed in the shared parser: direct parsing, candidate planning, resolution,
completion, sequence loading, and `::file` transclusion now reject rooted magic
payloads with typed `InvalidSyntax` errors, including portable Windows forms.

One high-severity cross-surface defect remains. Frontmatter `prologue` and
`epilogue` values convert every `FileReference` parse error into the decision
that the value is inline content. The same malformed magic or unsupported home
reference is therefore rejected by direct and sequence surfaces but rendered
verbatim by frontmatter transclusion. The new cross-surface test exercises only
`::file`, so it cannot detect this divergence.

## Findings

### 1. High — Frontmatter transclusion swallows shared parser errors as inline content

`is_file_like_reference` calls `FileReference::new`, but maps every error to
`false` (`darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:241-243`).
Both frontmatter preflight and execution interpret `false` as inline content:
preflight skips the value
(`darkmatter/lib/src/markdown/compose/preflight/collect.rs:393-395`), and the
engine emits it as a fixed section
(`darkmatter/lib/src/markdown/compose/transclusion/engine.rs:662-670`). A valid
reference is also parsed a second time by `resolve_target`, so this path does
not satisfy D1's parse-once contract even when it succeeds.

The current binary reproduced the behavior from stdin:

```text
prologue: "@//escape.md"       | exit=0 | output contains @//escape.md
prologue: "~alice/secret.md"   | exit=0 | output contains ~alice/secret.md
```

Direct `FileReference`, both sequence resolvers, and `::file` transclusion
return their typed `InvalidSyntax` or `UnsupportedUserHome` errors for the same
authored values. Frontmatter transclusion silently changes author intent and
diagnostic identity instead. This violates D1, D5, Acceptance Criteria 1 and 6,
and the testing strategy's requirement that prologue/epilogue targets share the
unified contract. It is high severity because an explicitly reference-shaped
configuration is accepted as successful content rather than failing with the
shared typed diagnostic. The correct verification level is Level 1; no terminal
encoder or renderer participates in classification.

**Required change:** replace the boolean helper at this boundary with a typed
classification result that can distinguish inline content, a successfully
parsed `FileReference`, and a parser failure. Pass the parsed value into
resolution rather than reparsing it. Parser failures for single-line
reference-shaped frontmatter values must propagate through both static
preflight and execution. Add Level 1 prologue and epilogue cases for rooted
magic (direct and recursive), unsupported `~user`, and at least one valid inline
string, proving preflight/execution parity and guarding the intentional inline
content behavior.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
|---|---|---|
| Shared parsing; explicit/implicit precedence; interpolation kind; candidate provenance; fallible probing; home, package, vault, recursive, and rooted-magic behavior | Level 1 `biscuit-file` unit/integration tests | Appropriate and green. Review 7's shared-parser escape is closed for POSIX and portable Windows spellings. |
| Completion rejects rooted magic and emitted values execute through the same candidate context | Level 1 completion/parser integration | Appropriate and green. |
| Sequence and direct transclusion use the shared grammar without private `@/` rewriting | Level 1 cross-surface test and source inspection | Green for direct, both sequence resolvers, and `::file`; **gap** for frontmatter prologue/epilogue (Finding 1). |
| Frontmatter prologue/epilogue references share parser errors and resolution semantics with other transclusion surfaces | Level 1 tests for valid local/remote values only | **Gap:** invalid references are rendered as inline content, and no discriminating test covers the behavior (Finding 1). |
| Immutable HOME, environment, repository, package, magic, and source context survives lifecycle, loop, and nested-sequence paths | Level 1 adapter tests with ambient-state mutation | Appropriate and green. |
| Top-level compose, inline-compose, sequence, schema/file values, expression functions, transclusion, and completion share candidate behavior | Level 1 unit/subprocess integration | Green except for malformed prologue/epilogue references (Finding 1). |
| Bare motivating reference succeeds; explicit source-relative reference fails; no-match renders repository then source candidates | Level 2 tmux captures | Appropriate and green. |
| Proxy routes share typed identity and proxy cycles surface to the user | Level 1 identity tests plus Level 2 tmux lifecycle captures | Appropriate and green. |
| Typed error blocks, candidate ordering, styling, widths, and hyperlinks render through a real terminal | Level 2 tmux/WezTerm captures | Claudine's relevant 154-test L2 suite is green. |
| OS-native keyboard, mouse, paste, IME, or hotkey behavior | Level 3 | Not applicable; the feature makes no input-encoder claim. |

## Verification Performed

- Read the full specification, Review 7, current implementation changes,
  file-reference guidance, and affected Claudine, Darkmatter, and
  `biscuit-file` tests.
- Used GitNexus to inspect the file-resolution and proxy/sequence execution
  seams and to analyze the current worktree changes. Its index reported low
  graph risk and no affected indexed execution process for the dirty diff.
- Reproduced malformed frontmatter handling through the current `md compose`
  binary: rooted magic and unsupported `~user` prologues both exited zero and
  rendered verbatim.
- `biscuit-file/just test` passed: 383 library/integration tests selected with
  4 configured skips; 61 CLI tests passed. `just lint` passed.
- `darkmatter/just test` passed: 5,659 library tests, 555 CLI tests, and 566
  DMLS tests completed green with configured skips. `just lint` passed.
- `claudine/just test` did not complete: after 3,339 passing tests, the unrelated
  `loop_and_lifecycle_agree_on_shared_syntax` test timed out on all four suite
  attempts, so fail-fast skipped 503 library tests and all later crate phases.
  The timed-out test passed alone in 9.1 seconds. Feature-related malformed-
  magic and resolution tests passed before the suite stopped.
- `claudine/just lint` passed, including all 18 error guards; one guard passed
  on retry after a leaked-handle report.
- `claudine/just test-l2` passed 154/154 real-terminal tests, including the
  repository-first, explicit-relative, candidate-order, proxy-identity, and
  typed transclusion captures.
- `darkmatter/just test-l2` passed 19/19 library real-terminal tests, then its
  CLI phase stopped after an unrelated code-block pixel-luma assertion failed
  four attempts; 66 of 69 CLI L2 tests were skipped by fail-fast.
- No formatting or Git commit was performed; existing implementation changes
  were preserved.

## Production Readiness Closure

Production readiness requires preserving `FileReference` parser failures at
the prologue/epilogue boundary, parsing each value once, and adding the missing
Level 1 frontmatter parity cases. The direct rooted-magic escape from Review 7
is fixed, existing feature-relevant Level 2 evidence is green, and no Level 3
work is required.
