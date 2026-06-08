---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: Minimal and summary detection still suppress malformed HEAD

`GitRepo::detect_with_request()` obtains `current_branch` through the infallible
`current_branch()` accessor (`sniff/lib/src/filesystem/git/types.rs:794`). The
new fallible HEAD query is used later only when `wants_repo_metadata()` enables
branch and tracking collection (`types.rs:876-884`). `GitRequest::minimal()` and
`summary()` deliberately return false from that gate, even though branch name is
part of both presets' contract (`sniff/lib/src/request.rs:302-335,413-415`).

Consequently, a missing or malformed HEAD still produces `Ok(GitInfo)` with
`current_branch: None` for these public fallible detection requests. That
violates the specification's absent-repository/error policy and makes corruption
indistinguishable from a detached or unborn checkout for the cheapest, most
commonly embedded request levels. The new test covers only `GitRequest::full()`
(`sniff/lib/tests/git_parity.rs:1020-1030`), whose metadata path happens to catch
the failure later.

Call `try_current_branch()` from `detect_with_request()` and reuse the result for
branch/tracking collection. Add L1 tests for both `minimal()` and `summary()`
with malformed HEAD, plus a missing-HEAD case created after discovery so the
detection call itself exercises the failure.

### High: Missing hex-looking branch names are reported as errors

When neither a local nor remote-tracking branch exists,
`resolve_remote_or_sha()` sends the branch name to `resolve_single_opt()`
(`sniff/lib/src/filesystem/git/discovery.rs:78-90`). If the name consists only
of hexadecimal characters but is too short or too long for a gix object prefix,
that helper returns an error (`discovery.rs:44-65`). Thus an absent branch such
as `dead` or `add` fails `commits_for_branch_at()` and `sniff repo git-status
--branch dead`, while an absent non-hex branch correctly returns empty history.

This contradicts the documented contract that an unresolved branch is
`Ok(empty)` (`discovery.rs:685-689`) and the specification's requirement to
separate genuine absence from operational failure. The corruption fix should
not infer whether user input is a branch or SHA solely from its characters.

Keep branch/ref absence as `Ok(None)` and probe a SHA only when the input meets
the supported object-ID/prefix shape, or make the requested revision kind
explicit. Add L1 library and CLI parity tests for absent short-hex, valid-length
hex, and ordinary branch names.

## Verification Levels

All user-observable requirements in this migration are repository and CLI data
behavior, so Level 1 is the appropriate tier. No requirement needs Level 2
terminal rendering or Level 3 OS keyboard injection.

| Requirement | Required | Strongest evidence | Result |
|---|---|---|---|
| Packed checked-out branch and tracking parity | L1 | Library and CLI packed-ref fixtures | Pass |
| Malformed remote-tracking refs surface | L1 | Library and CLI corrupt-ref fixtures | Pass |
| Fallible HEAD error policy for every detection preset | L1 | Full-request fixture only | Gap |
| Unresolved branch names return empty history | L1 | No short-hex absence fixture | Gap |
| Performance | Same-host Criterion comparison | Recorded final comparison for all specified IDs | Pass |

## Verification

- Reviewed the specification, review 8, iteration-9 staged changes, affected git
  APIs, request presets, L1 parity tests, CLI tests, benchmark records, and the
  macOS/Linux/Windows workflow.
- `git diff --cached --check` passes.
- Rust tests, rustfmt, Clippy, and doctests could not run because this session
  has no installed Rust toolchain (`rustup toolchain list` reports none).

## Decision

Not ready for production. Iteration 9 fixes the packed-ref, remote-tracking-ref,
and metadata-level HEAD failures from review 8, but two public fallible paths
still misclassify invalid repository state or genuine branch absence.
