---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-21T17:09:38-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: false
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-9.md
previous: 2026-07-13-file-resolution/review-8.md
---

# Review 9: Unified File-Reference Resolution

## Verdict

The feature is **ready for production**. Review 8's high-severity blocker is
closed: frontmatter `prologue` and `epilogue` values now distinguish inline
content, a parsed `FileReference`, and a shared-parser failure. Preflight and
execution propagate the same typed parser error, while valid inline content
retains its established behavior.

No new functional, correctness, ergonomics, performance, or test-rigor finding
was identified. The shared parser remains the syntax authority, the parsed value
is passed into resolution without reparsing at the frontmatter boundary, and the
full affected-area Level 1 and lint gates are green.

## Findings

No findings.

## Review 8 Closure

`classify_frontmatter_reference` now returns a three-way typed classification
instead of collapsing every `FileReference::new` error into `false`
(`darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:241-282`). Both
the static preflight collector and execution engine propagate `ParseError` and
pass `Parsed(FileReference)` to `resolve_parsed_target`
(`preflight/collect.rs:393-409`; `transclusion/engine.rs:653-689`). This closes
the prior D1/D5 and Acceptance Criteria 1/6 divergence.

The new Level 1 regression covers both `prologue` and `epilogue`, direct and
recursive rooted magic, unsupported `~user`, preflight/execution diagnostic
parity, and valid inline content
(`darkmatter/lib/src/markdown/compose/tests/transclusion.rs:198-278`). The test
also enables `ignore_invalid_references`, proving malformed shared grammar is
not silently downgraded to skipped or inline content.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
|---|---|---|
| Shared parsing; explicit/implicit precedence; interpolation kind; candidate provenance; fallible probing; home, package, vault, recursive, symlink, and cross-platform absolute-path behavior | Level 1 `biscuit-file` unit and process-integration tests | Appropriate and green. These are deterministic parser/filesystem semantics. |
| Rooted magic cannot escape a configured root through direct parsing, recursive parsing, resolution, or completion | Level 1 parser, detailed-resolution, grammar, and completion round-trip tests | Appropriate and green for POSIX and portable Windows spellings. |
| Frontmatter prologue/epilogue preserve shared parser errors and parse once before resolution while retaining valid inline content | Level 1 Darkmatter preflight/execution integration tests | Appropriate and green. Review 8's gap is closed. |
| Case-insensitive HTTP(S) targets retain remote policy, discovery, preflight, and execution behavior | Level 1 Darkmatter unit and wiremock integration tests | Appropriate and green; no terminal behavior participates. |
| Immutable HOME, environment, repository, package, magic, and source context survives lifecycle, loop, sequence, and nested-document paths | Level 1 adapter and process-integration tests with ambient-state mutation | Appropriate and green. |
| Top-level compose, inline-compose, sequence, schema/file values, expression functions, transclusion, local links, completion, and proxy routes share candidate behavior and typed identity | Level 1 cross-surface and subprocess integration tests | Appropriate and green. |
| Bare motivating reference succeeds; explicit source-relative reference remains pinned; no-match displays repository then source candidates | Level 2 tmux captures in `level2_file_resolution_capture.rs` | Appropriate. The feature's existing real-terminal evidence remains present and was green in Review 8; the Review 9 changes do not alter this renderer. |
| Typed proxy/transclusion failures, candidate ordering, styling, widths, and hyperlinks render through a real terminal | Level 2 tmux/WezTerm captures in the typed-error and invalid-file-reference suites | Appropriate. Existing 154-test Claudine L2 evidence was green in Review 8; no rendering code changed afterward. |
| OS-native keyboard, mouse, paste, IME, or hotkey behavior | Level 3 | Not applicable; the feature makes no terminal input-encoder claim. |

## Verification Performed

- Read the full specification, Review 8, current implementation diff,
  file-reference documentation, Claudine test-placement guidance, and the
  affected tests.
- Traced frontmatter preflight/execution through GitNexus and inspected the key
  classification and resolution symbols. The final dirty-worktree audit
  reported 56 changed symbols across 15 files, low risk, and no affected indexed
  execution process.
- Ran the focused Level 1 regressions for frontmatter parser-error parity,
  inline-content parity, and rooted-magic parser/resolution/completion behavior;
  all passed.
- `biscuit-file/just test` passed all 383 selected library/integration tests
  with 4 configured skips; its 61 CLI tests also passed. `just lint` passed.
- `darkmatter/just test` passed 5,661 library tests, 555 CLI tests, and 566 DMLS
  tests with configured skips. `just lint` passed.
- `claudine/just test` passed 21 catalog tests, 3,843 library tests, 47 contract
  tests, 2,169 CLI tests, and 152 generator tests with configured skips. The
  library reported one configured flaky retry and the CLI reported three; all
  ultimately passed and none concerns file resolution. `just lint` passed,
  including all 18 error guards.
- Reviewed the Level 2 suite inventory and Review 8's green execution evidence.
  No Review 9 change touches terminal rendering, and no Level 3 behavior is in
  scope.
- `md schema validate` could not validate the review because the repository's
  existing `schemas/feature-review.yaml` is not accepted as a standalone
  SimplifiedSchema: it combines tagged-schema `kind`/`types` with unsupported
  `$schema` and `description` keys. The failure occurs while parsing the schema,
  before validation of this review's frontmatter.
- No formatting or Git commit was performed; existing implementation changes
  were preserved.

## Production Readiness Closure

All fourteen acceptance criteria have implementation and appropriately leveled
verification. The only open finding from Review 8 is fixed, the affected package
gates pass, and there is no remaining requirement at the wrong verification
level. The feature is production-ready.
