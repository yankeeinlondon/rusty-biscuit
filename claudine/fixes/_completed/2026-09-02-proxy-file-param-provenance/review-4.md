---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-02T13:59:24+01:00
spec: 2026-09-02-proxy-file-param-provenance/spec.md
implemented: false
description: A **fix** review of `2026-09-02-proxy-file-param-provenance/spec.md`
fix: 2026-09-02-proxy-file-param-provenance/review-4.md
previous: 2026-09-02-proxy-file-param-provenance/review-3.md
---

# Review 4: Proxy File Parameter Provenance

## Verdict

The fix is **ready for production**. Review 3's two high-severity findings are
resolved without weakening the broader provenance contract. The shipped
implementation router again gives an existing unimplemented review precedence,
and dynamically indexed caller file arrays retain the selected occurrence's raw
reference, origin, candidate, and typed diagnostic across direct and proxied
routes.

No blocking functionality, correctness, ergonomics, performance, or test-rigor
gaps were found in this iteration.

## Findings

No findings.

## Review 3 Closure

### Shipped pending-review routing

The `pending_review` derivation and first-priority lifecycle branch are restored
in `prompts/implement.md`. A dedicated Level 1 process test supplies an
unimplemented spec, completed plan, and unimplemented review, then proves the
shipped router selects `implement-suggestions.md` instead of resuming the plan.
The shipped-prompt hash fixture and drift guards were updated consistently.

### Dynamic array diagnostic provenance

Filesystem functions now evaluate their first argument through an occurrence-
aware path. Dynamic array indices are evaluated once, use the same numeric,
negative-index, and bounds semantics as ordinary expression indexing, and append
the resolved element index to the caller occurrence key. Invalid or derived
arguments continue without caller provenance rather than guessing ownership.

Darkmatter Level 1 coverage selects both aliased spellings (`missing.md` and
`./missing.md`) through a variable index and asserts the raw reference,
property, caller source/base/repository origin, selected candidate, and root
provenance. Claudine's Level 1 process test asserts the same structured details
and exact diagnostic equality across direct and proxied routes.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Exact shipped router accepts an area-relative `spec`, proxies to the lazy target, and reaches the provider | Level 1 fake-provider process test | Appropriate and present. |
| Existing unimplemented review beside a spec takes precedence over a completed original plan | Level 1 fake-provider process test plus shipped-prompt drift guards | Appropriate and present; Review 3 Finding 1 is resolved. |
| Router, direct target, and proxied target read the same specification | Level 1 fake-provider process tests | Appropriate and present. |
| Target derives `review`, `log`, and present/absent optional `design` beside `spec` | Level 1 captured-provider-prompt assertions | Appropriate and present. |
| Lazy local files use `FileReference`-owned candidate ordering | Level 1 candidate-order collision matrix | Appropriate and present. |
| Scalar, array, property-union, and root-union schemas select exactly one applicable file arm | Level 1 scalar, array, and union tests, including mixed origins and document-owned siblings | Appropriate and present. |
| Caller origin survives proxy, retry, resume, loop, inline-compose, and sequence/task routes | Level 1 fake-provider process tests for every route | Appropriate and present. |
| Task, CLI, runtime-mutation, and reserved-overlay values retain precedence and ownership | Level 1 process tests for independent winning layers | Appropriate and present. |
| Missing or malformed direct and proxy failures retain equal typed identity and provenance evidence | Level 1 structured process matrix plus Darkmatter scalar, literal-index, and dynamic-index tests | Appropriate and present; Review 3 Finding 2 is resolved. |
| Equal semantic paths retain distinct property and array-occurrence identity | Level 1 property, literal-array, and dynamic-array collision tests | Appropriate and present. |
| Candidate disposition reports only available evidence | Level 1 structured-detail permission-failure test | Appropriate and present. |
| Raw, semantic, and presentation projections remain distinct through re-entry and cache identity | Level 1 projection, request-identity, re-entry, and approval-cache tests | Appropriate and present. |
| Native semantic and portable presentation paths preserve identity on macOS, Linux, and Windows | Level 1 host tests plus an enabled `#[cfg(windows)]` test | Appropriate; macOS ran in this review and Windows coverage remains assigned to Windows CI. |

Levels 2 and 3 are not applicable. This fix changes schema-selected filesystem
semantics, routing, and structured diagnostics. It does not claim real-terminal
rendering, terminal input encoding, keybindings, paste/IME behavior, mouse
behavior, glyph layout, styling, or scrolling.

## Verification Performed

- `just test darkmatter claudine`: **14,536 passed; 63 higher-tier tests
  skipped** across 11 selected packages.
- `just ci-local darkmatter claudine`: **all 22 scoped lint and Level 1 gates
  passed** across the same 11 packages.
- `git diff --check`: **passed** after the review closure edits.
- GitNexus upstream impact for `evaluate_function`: **critical radius**, with
  210 upstream symbols across 13 modules; the complete scoped gates passed.
- GitNexus change detection before closure: **low risk** for the six indexed
  implementation files, with 11 changed symbols and no affected indexed
  execution flows.

The executed tests establish macOS-host confidence. Platform-native behavior is
also covered by ordinary cross-platform Level 1 tests, including a Windows-only
path-identity row intended to execute on Windows CI.

## Production Readiness

All acceptance criteria have implementation and appropriately leveled
verification. The fix preserves caller-owned file identity through every
required preparation route, keeps document-owned resolution and layer
precedence unchanged, and retains provenance-complete diagnostics. No further
changes are required for production readiness.
