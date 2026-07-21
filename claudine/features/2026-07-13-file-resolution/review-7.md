---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T15:11:50-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-7.md
previous: 2026-07-13-file-resolution/review-6.md
---

# Review 7: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Both Review 6 findings are closed:
Claudine's lifecycle-shell and nested-sequence preflight paths now retain the
request-scoped resolution snapshot, and Darkmatter transclusion now routes
mixed-case HTTP(S) through `FileReference` classification. Their new Level 1
regression tests passed, as did the package, lint, and real-terminal gates.

One high-severity shared-grammar defect remains. A magic reference with two or
more separators after `@` can discard every configured magic root and resolve an
absolute filesystem path. Claudine sequence loading also retains a private
`@/` rewrite that makes the same authored value resolve differently there. The
existing Level 1 tests cover only the single-separator spelling and therefore do
not detect either violation.

## Findings

### 1. High — Repeated separators escape magic roots and expose cross-surface grammar drift

The shared parser removes at most one slash after `@`
(`biscuit-file/lib/src/file_reference/parse.rs:89-90`). Candidate construction
then joins the remaining payload to each configured root
(`biscuit-file/lib/src/file_reference/resolve.rs:781-785`). When the payload is
still absolute, `PathBuf::join` discards the root. On this macOS review host, the
current source produced these results:

```text
@/etc/hosts   | exit=1 | no match under the configured magic roots
@//etc/hosts  | exit=0 | /etc/hosts
@///etc/hosts | exit=0 | /etc/hosts
```

Thus a value still classified and reported as `Magic` can bypass magic search
order and root provenance entirely. Equivalent rooted-payload cases can exist
on Windows through drive-qualified or UNC spellings, so a POSIX-only fix would
not satisfy the cross-platform contract.

Claudine's two sequence entry points separately strip `@/` and rebuild the
reference before calling `FileReference::new`
(`claudine/lib/src/composition/sequence/source.rs:36-48` and `68-82`). For
`@//etc/hosts`, that private rewrite removes the first slash and the shared
parser removes the second, producing a root-relative magic payload in sequence
loading while direct `FileReference` and Darkmatter surfaces resolve the
absolute file. The adjacent comment says this is not string surgery, but the
implementation is string surgery and is now stale after the shared parser
learned the normal `@/` spelling.

This violates D1, D3, D7 and Acceptance Criteria 1, 6, 7, and 9. It is high
severity because root selection is part of the public resolution contract and
the defect can select an arbitrary existing absolute file while retaining
misleading magic provenance. The correct verification level is Level 1; no
terminal encoder or renderer participates in parsing or candidate construction.

**Required change:** make `FileReference` reject any magic payload that remains
rooted after consuming the supported sigil spelling, using portable handling for
POSIX roots, Windows drive-qualified paths, and UNC paths. Do not silently trim
an arbitrary number of separators because the documented equivalent spellings
are only `@x` and `@/x`. Remove both sequence-local `@/` rewrites and pass the
authored string directly to `FileReference::new`. Add Level 1 parser, candidate-
plan, resolution, and completion cases for repeated POSIX separators and native
Windows rooted spellings, including recursive magic references. Add a
cross-surface fixture proving direct, sequence, and transclusion entry points
either produce the same candidates or reject the same authored value.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
|---|---|---|
| Shared parsing; explicit/implicit precedence; interpolation kind; candidate provenance; fallible probing; home, package, vault, and recursive behavior | Level 1 `biscuit-file` unit/integration tests | Existing cases are green, but repeated-separator magic references have no discriminating test and violate root ordering (Finding 1). |
| Every Claudine production surface delegates syntax to `FileReference` | Level 1 cross-surface tests and source inspection | **Gap:** both sequence resolvers retain a private `@/` grammar rewrite (Finding 1). |
| Immutable HOME, environment, repository, package, magic, and source context survives lifecycle-shell and nested sequence preflight | Level 1 adapter tests with ambient-state mutation | Appropriate and green. Review 6 Finding 1 is closed. |
| Mixed/uppercase HTTP(S) transclusions follow shared classification and remote policy in preflight and execution | Level 1 parser, policy, and mock-server tests for `::file` and `::code` | Appropriate and green. Review 6 Finding 2 is closed. |
| Top-level compose, inline-compose, sequence, schema/file values, expression functions, and completion share candidate behavior | Level 1 unit/subprocess integration | Green for covered spellings; **gap** for repeated-separator magic parity (Finding 1). |
| Bare motivating reference succeeds; explicit source-relative reference fails; no-match renders repository then source candidates | Level 2 tmux captures | Appropriate and green. |
| Proxy routes share typed identity and proxy cycles surface to the user | Level 1 identity tests plus Level 2 tmux lifecycle captures | Appropriate and green. |
| Typed error blocks, candidate ordering, styling, widths, and hyperlinks render through a real terminal | Level 2 tmux/WezTerm captures | Appropriate and green. |
| OS-native keyboard, mouse, paste, IME, or hotkey behavior | Level 3 | Not applicable; the feature makes no input-encoder claim. |

## Verification Performed

- Read the full specification, Review 6, current implementation changes, public
  file-reference guidance, and the affected Claudine, Darkmatter, and
  `biscuit-file` tests.
- Used GitNexus against the current worktree to trace lifecycle-shell, nested
  sequence, and transclusion callers and confirm the prior findings' repaired
  execution paths.
- Reproduced the remaining defect through the current `biscuit-file-cli` source:
  `@//etc/hosts` and `@///etc/hosts` both resolved to `/etc/hosts`, while
  `@/etc/hosts` did not.
- `biscuit-file/just test` passed: 378 library/integration tests selected with 4
  configured skips; 61 CLI tests passed.
- `darkmatter/just test` passed: 5,659 library tests, 555 CLI tests, and 566 DMLS
  tests completed green with their configured skips. One library test passed on
  retry after a leaked-handle timeout.
- `claudine/just test` passed across catalog-types, library (3,842/3,842),
  contract (47/47), CLI (2,169/2,169), and generator (152/152) crates. One
  interpolation-conformance test passed on its third retry.
- `biscuit-file/just lint`, `darkmatter/just lint`, and `claudine/just lint`
  passed. Claudine's 18 error-guard tests also passed.
- `claudine/just test-l2` passed: 154/154 real-terminal tests, including the
  dedicated repository-first, explicit-relative, candidate-order, proxy, and
  proxy-cycle captures.
- `git diff --check` passed after review metadata was written. No formatting or
  Git commit was performed; unrelated worktree changes were preserved.

## Production Readiness Closure

Production readiness requires closing Finding 1 in the shared parser, deleting
the two sequence-local grammar branches, and adding discriminating Level 1
cross-platform and cross-surface coverage. Existing Level 2 evidence is
appropriate and green; no Level 3 work is required.
