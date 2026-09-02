---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-02T13:15:28+01:00
spec: 2026-09-02-proxy-file-param-provenance/spec.md
log: claudine/fixes/2026-09-02-proxy-file-param-provenance/log.md
implemented: true
implemented_by: codex/default
description: A **fix** review of `2026-09-02-proxy-file-param-provenance/spec.md`
fix: 2026-09-02-proxy-file-param-provenance/review-3.md
previous: 2026-09-02-proxy-file-param-provenance/review-2.md
next: 2026-09-02-proxy-file-param-provenance/review-4.md
---

# Review 3: Proxy File Parameter Provenance

## Verdict

The fix is **not ready for production**. Review 2's five findings have targeted
implementations and the scoped Level 1 and lint gates pass. However, the implementation
also removes the committed pending-review routing branch from the shipped
`prompts/implement.md` and deletes its regression test. In addition, later-read
diagnostic provenance for caller file arrays is recovered only when the first
filesystem-function argument uses a literal numeric index; a normal dynamic
index loses the raw caller spelling and origin/candidate evidence required by
the specification.

## Findings

### 1. High: the fix removes the shipped pending-review routing behavior and its test

Commit `a47d36a2e` added a first-priority `pending_review` branch to
`prompts/implement.md`: when `review-<review_iterations>.md` exists beside an
unimplemented spec and is itself unimplemented, the router sends that review to
`implement-suggestions.md` before falling back to the original implementation
plan. Implementation commit `8925676b4` reverses that entire behavior
(`prompts/implement.md:10-25`) and removes
`shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan`
from `compose_caller_file_provenance.rs`.

This is not required by the provenance specification or listed as a non-goal.
It is also not a harmless fixture adjustment: users invoking the shipped
implementation router with an outstanding review will once again be routed by
the spec's `implemented` flag, potentially re-running the original plan instead
of implementing the review findings. The full Level 1 suite remains green
because the only assertion for the committed routing contract is deleted in the
same implementation series.

**Required change:** restore the `pending_review` property, its first-priority
initialize branch, and the dedicated Level 1 process test. Make the provenance
fixture coexist with the shipped routing contract—for example, choose fixture
frontmatter/files that deliberately select the intended branch—instead of
changing production routing to fit the fixture.

### 2. High: dynamic array indexing drops required caller diagnostic provenance

The collision repair correctly keys provenance by JSON-pointer-like occurrence,
including `/files/0` and `/files/1`. At filesystem-function dispatch,
`caller_file_occurrence` reconstructs that key from the source expression
(`expression/mod.rs:777-796`), but it accepts an array index only when the AST
node is a non-negative integer `NumberLiteral`. Thus
`frontmatter(files[0], 'value')` selects `/files/0`, while the semantically
equivalent and ordinary `frontmatter(files[index], 'value')` returns no
occurrence and installs no `active_caller_file_provenance`.

The file read still targets the projected absolute path, but a failure is then
reported as if that absolute path were document-authored: it loses the raw
array element spelling, caller property, captured origin/base, and selected
candidate evidence. This violates D6, D7, and acceptance criteria 9, 11, and
12. The new duplicate-array test covers only literal indices, so it cannot
detect the gap. Direct/proxy equivalence may still compare equal while both
routes omit the required evidence.

This is not the non-goal concerning arbitrary derived strings. Indexing a
schema-selected caller file array directly is part of the typed input value and
is explicitly within the array materialization contract.

**Required change:** carry the selected occurrence through expression
evaluation (or otherwise resolve a dynamic integral index against the original
array expression) so direct array-element access retains its caller record.
Add Level 1 cases for a variable-selected element, including two aliased raw
spellings that materialize to the same path, and assert raw reference,
property, origin/base, candidate, and direct/proxy diagnostic equality.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Exact shipped router accepts an area-relative `spec`, proxies to the lazy target, and reaches the provider | Level 1 fake-provider process test | Appropriate and present for the implemented-spec branch. |
| Existing unimplemented review beside a spec retains precedence in the shipped router | No retained test; the prior Level 1 test is deleted | **Regression:** production behavior and its only test are removed (Finding 1). |
| Router, direct target, and proxied target read the same specification | Level 1 fake-provider process tests | Appropriate and present. |
| Target derives `review`, `log`, and present/absent optional `design` beside `spec` | Level 1 captured-provider-prompt assertions | Appropriate and present; Review 2 Finding 5 is resolved. |
| Lazy local files use `FileReference`-owned candidate ordering | Level 1 candidate-order collision matrix | Appropriate and present. |
| Scalar, array, property-union, and root-union schemas select exactly one applicable file arm | Level 1 scalar/array/union tests, including mixed origins and document-owned siblings | Appropriate for materialization; Review 2 Finding 2 is resolved. |
| Caller origin survives proxy, retry, resume, loop, inline-compose, and sequence/task routes | Level 1 fake-provider process tests for each route | Appropriate and present. |
| Task, CLI, runtime-mutation, and reserved-overlay values retain precedence and ownership | Level 1 process tests for independent winners | Appropriate and present; Review 2 Finding 3 is resolved. |
| Missing/malformed direct and proxy failures retain equal typed identity and provenance evidence | Level 1 structured process matrix for scalar arguments; literal-index Darkmatter array tests | **Gap:** dynamic array-element access loses provenance before diagnostic construction (Finding 2). |
| Equal semantic paths retain per-occurrence identity and distinct request identity | Level 1 property and literal-array collision tests | Appropriate for literal occurrences; dynamic array selection remains uncovered (Finding 2). |
| Candidate disposition reflects available evidence | Level 1 permission-denied structured-detail test | Appropriate and present; Review 2 Finding 4 is resolved. |
| Native semantic and portable presentation paths preserve identity on macOS, Linux, and Windows | Level 1 host tests plus an enabled `#[cfg(windows)]` test | Appropriate; macOS was exercised here and the Windows row is intended for Windows CI. |

Levels 2 and 3 are not applicable. The fix changes filesystem semantics,
schema selection, routing, and structured errors; it does not claim real-terminal
rendering or physical-key behavior.

## Verification Performed

- `just test biscuit-file darkmatter claudine`: **15,347 passed; 63 higher-tier
  tests skipped** across 13 selected packages.
- `biscuit-file/just lint`: **passed** for `biscuit-file` and
  `biscuit-file-cli`.
- `darkmatter/just lint`: **passed** for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- `claudine/just lint`: **passed** for the 18 diagnostic guards and all five
  package-area crates.
- `git diff --check`: **passed**, including the review-file closure edits.
- GitNexus upstream impact: **high risk** for canonical Claudine preparation
  (two direct callers across compose and sequence) and **high risk** for
  Darkmatter caller-file projection (22 upstream symbols and affected compose
  flows).

The green gates establish that the covered behavior is stable on this macOS
host. They do not make the deleted router contract or the untested dynamic
array diagnostic path correct.

## Closure Criteria

Restore the pending-review router branch and its Level 1 process test, preserve
caller provenance for dynamically indexed file-array elements, and rerun the
Biscuit File, Darkmatter, and Claudine Level 1 and lint gates. Production
readiness requires both findings to be resolved without weakening the shipped
router's existing behavior.
