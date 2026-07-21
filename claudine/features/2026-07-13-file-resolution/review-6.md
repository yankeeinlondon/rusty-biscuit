---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T11:18:58-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-6.md
previous: 2026-07-13-file-resolution/review-5.md
---

# Review 6: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Review 5's Darkmatter reference-
analysis divergence, schema-root private resolver, stale contract comments, and
incomplete Claudine Level 1 gate are closed. The accepted-external provenance
fixture now keeps composition, enumeration, graph construction, and validation
aligned, and every required package, lint, and real-terminal gate completed
successfully in this review.

Two high-severity contract gaps remain. Claudine still discards the immutable
file-resolution snapshot in lifecycle-shell preflight and nested sequence task
reference loading. Darkmatter transclusion also routes HTTP(S) through a private,
case-sensitive prefix helper, so uppercase schemes do not receive the same
surface fetch policy as lowercase schemes even though `FileReference` classifies
them correctly. Both are deterministic Level 1 behaviors and lack discriminating
tests.

## Findings

### 1. High — Two Claudine preflight paths still recapture file-resolution state

`resolve_lifecycle_shell_commands` constructs a fresh Darkmatter
`ResolutionContext::new` from the source directory
(`claudine/lib/src/composition/preflight.rs:297-305`). Both preparation callers
already own `PrepareOptions::file_resolution_context`, but pass only the legacy
fallback directory (`composition/prepare.rs:289-295` and `462-468`). A read-side
function inside a lifecycle shell interpolation can therefore recapture HOME and
the process environment and rediscover repository/package roots instead of using
the request snapshot that body composition and event-time lifecycle evaluation
share.

Sequence graph preflight has the same defect for nested task, group, and catalog
references. `Loader` stores `file_resolution_context`, but
`Loader::resolve_reference` calls the ambient
`resolve_sequence_reference(reference, origin)` compatibility entry point
(`composition/sequence/preflight/mod.rs:779-785`) instead of
`resolve_sequence_reference_in_context`. This can make static preflight approve,
reject, or load a different file after ambient HOME/environment/CWD or repository
state changes.

The existing lifecycle-shell tests prove document anchoring and rejection of the
launch-area fallback, while sequence tests prove the top-level external sequence
source reuses its snapshot. Neither test reaches these two adapters with mutated
HOME/environment or configured magic/package roots. This violates D2, D5, D10,
D12 and Acceptance Criteria 6, 12, and 14.

**Required change:** pass the captured `FileResolutionContext` into lifecycle
shell preflight and build its expression context through
`document_expression_resolution_context`. Change `Loader::resolve_reference` to
use its stored snapshot and the context-aware sequence resolver. Add Level 1
fixtures that mutate ambient HOME/environment after capture and discriminate
home, environment-expanded, magic, and package references in both paths. Include
a nested external task/group fixture so the authoring document remains the base.

### 2. High — Transclusion's private URL classifier is case-sensitive and bypasses the shared grammar

`biscuit_file::FileReference` correctly recognizes HTTP(S) schemes
case-insensitively, but Darkmatter's `is_url_like` checks only lowercase literal
prefixes (`darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:231-233`).
That helper chooses the URL versus local-file branch for `::file` and `::code`
in both preflight and execution (`compose/preflight/collect.rs:395-400` and
`compose/transclusion/engine.rs:662-676`). `resolve_target` repeats the same
private branch at `transclusion/resolver.rs:26-38`.

Consequently, `::file HTTP://host/page.md` or an equivalent `HTTPS://` target is
sent through local resolution and fails as an unsupported remote reference,
instead of following the same configured remote-fetch policy as the lowercase
spelling. The transclusion resolver also retains related prefix grammar for URL,
magic enablement, and `@/` normalization rather than parsing once and branching
on `FileReferenceClass`.

The shared parser has Level 1 uppercase-scheme coverage, and transclusion has
lowercase remote coverage, but no cross-surface test joins those two facts. This
violates D1, the ratified case-insensitive URL contract, and Acceptance Criteria
1, 6, and 9. The correct verification level is Level 1: a local mock HTTP server
can prove routing and policy without a terminal.

**Required change:** parse the authored target once through `FileReference` and
select local versus remote handling from `FileReferenceKind::Url`; remove the
case-sensitive prefix routing. Preserve the existing allowlist/fetch policy.
Add Level 1 `::file` and `::code` fixtures for mixed/uppercase HTTP and HTTPS,
covering both preflight discovery and execution, plus a denial-policy case.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
|---|---|---|
| Shared parsing, explicit/implicit precedence, effective interpolation kind, candidate provenance, fallible probing, home, package, recursive behavior, and cross-platform path classification | Level 1 `biscuit-file` unit/integration tests | Appropriate and green. |
| Top-level compose, inline-compose, sequence, and completion values use shared repository-first candidates | Level 1 subprocess integration | Appropriate and green. |
| Lifecycle, loop, sequence expressions, and reference-graph `when=` reuse immutable request HOME/environment/magic/package state | Level 1 adapter tests | Green for event-time lifecycle, loop, source expressions, task-value interpolation, and reference graph; **gap** for lifecycle-shell preflight and nested sequence preflight references (Finding 1). |
| Nested accepted external documents keep their authoring source across composition, enumeration, graph construction, and validation | Level 1 shared Darkmatter fixture | Appropriate and green. |
| HTTP(S) schemes are case-insensitive on every Claudine-executed file-reference surface | Level 1 shared-parser tests; lowercase-only transclusion tests | **Gap:** `::file`/`::code` routing remains case-sensitive and has no discriminating Level 1 test (Finding 2). |
| Bare motivating reference succeeds; explicit source-relative reference fails; no-match renders repository then source candidates | Level 2 tmux captures | Appropriate and green: all three dedicated captures passed. |
| Proxy routes share typed identity and proxy cycles surface to the user | Level 1 identity tests plus Level 2 tmux lifecycle captures | Appropriate and green. |
| Typed error blocks, candidate ordering, styling, widths, and hyperlinks render through a real terminal | Level 2 tmux/WezTerm captures | Appropriate and green. |
| Required package gates | Level 1 area tests, lint, and Claudine Level 2 | Satisfied: all required gates completed successfully. |

Level 3 is not applicable. The feature claims no OS keyboard/mouse event,
paste, IME, hotkey, or terminal input-encoder behavior.

## Verification Performed

- Read the full specification, Review 5, implementation inventory, current
  Darkmatter provenance/schema patch, Claudine context adapters, transclusion
  routing, sequence preflight, and relevant tests.
- Used GitNexus to locate the file-resolution execution surfaces and cross-check
  the current reference-analysis definitions.
- `biscuit-file/just test` passed: 377 library/integration tests selected with
  4 configured skips; 61 CLI tests passed.
- `darkmatter/just test` passed: 5,653 library tests, 555 CLI tests, and 566 DMLS
  tests completed green with their configured skips.
- `claudine/just test` passed across catalog-types, library, contract, CLI, and
  generator crates. The formerly timing-out wrapper coverage completed.
- `biscuit-file/just lint`, `darkmatter/just lint`, and `claudine/just lint`
  passed. Claudine's 18 error-guard tests also passed.
- `claudine/just test-l2` passed: 148/148 real-terminal tests, including all
  dedicated file-resolution captures, proxy identity, and proxy-cycle coverage.
- `git diff --check` passed after review metadata was written. No formatting or
  Git commit was performed; unrelated worktree changes were preserved.

## Production Readiness Closure

Production readiness requires both remaining adapters in Finding 1 to consume
the immutable request snapshot with discriminating Level 1 fixtures, and the
transclusion URL routing in Finding 2 to use `FileReference` classification with
case-insensitive cross-surface tests. Existing Level 2 evidence is appropriate
and green; no Level 3 work is required.
