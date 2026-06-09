---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: A packed checked-out branch is reported as detached or unborn

`GitRepo::current_branch()` parses symbolic `HEAD`, but then requires
`.git/refs/heads/<name>` to exist as a loose file
(`lib/src/filesystem/git/types.rs:622-637`). That assumption is false after
`git pack-refs --all --prune`: the checked-out branch may exist only in
`packed-refs` while `HEAD` still contains `ref: refs/heads/main`.

For that valid repository shape, sniff returns `None` for the current branch.
`detect_with_request()` consequently emits no `current_branch`,
`try_branches()` does not mark the checked-out branch current, and
`try_tracking_status()` returns an empty list because it receives no branch name
(`types.rs:729-750,782-783`). This breaks the spec's branch-name and tracking
parity requirements and affects `sniff repo git-status`, including its
`--branch` defaulting behavior.

Use gix's HEAD-name query and distinguish detached/unborn from real errors in a
fallible helper. Add an L1 fixture that packs and prunes the checked-out branch,
then assert `current_branch`, current-branch flags, tracking status, full
detection, and CLI output remain unchanged.

### High: Corrupt remote-tracking branch revisions still become empty success

`get_commits_for_branch_fallible()` supports remote-tracking names such as
`origin/main`, but its fallback calls `resolve_single_opt()`
(`lib/src/filesystem/git/discovery.rs:672-689`). When `rev_parse_single()` fails
for any non-hex input, that helper unconditionally returns `Ok(None)`
(`discovery.rs:47-59`). A malformed `refs/remotes/origin/main`, a ref targeting a
missing object, or an I/O failure while resolving that ref is therefore reported
as a valid empty history.

This violates the spec's rule that fallible public APIs suppress only genuine
not-found/optional cases. The new tests cover malformed and missing-object
**local** refs (`git_parity.rs:827-870`), so they do not exercise the fallback
path that remains broken.

Resolve named revisions with structured ref lookup and peel errors, reserving
the object-prefix probe for SHA input. Add L1 malformed and missing-object
fixtures for `refs/remotes/origin/main`, plus a CLI `repo git-status --branch
origin/main` failure assertion.

### High: Fallible branch and tracking APIs still suppress malformed HEAD

`try_branches()` and `try_tracking_status()` advertise propagation of
permission, I/O, and corruption failures, but both obtain the branch through
the infallible `current_branch()` (`lib/src/filesystem/git/types.rs:725-750`).
That method converts every HEAD read/parse failure to `None`. In addition,
`get_local_branches_fallible()` converts every `head_id()` failure to `None`
(`lib/src/filesystem/git/remote_refresh.rs:157-165`). A missing or malformed
HEAD can therefore produce successful branch metadata with zeroed
ahead/behind values, while tracking returns a successful empty list.

The existing corrupt-`packed-refs` tests validate iterator failures only
(`types.rs:1343-1389`); they do not corrupt HEAD. Introduce one shared fallible
HEAD identity query and use it from these Result APIs. Add L1 tests for missing
and malformed HEAD through `try_branches()`, `try_tracking_status()`, and a
metadata-producing detection request.

## Verification Levels

All user-observable requirements here concern repository and CLI data behavior,
so Level 1 is the appropriate tier. No requirement needs Level 2 terminal
rendering or Level 3 OS keyboard injection.

| Requirement | Required | Strongest evidence | Result |
|---|---|---|---|
| Discovery, status, diff, history, refs | L1 on macOS/Linux/Windows | Cross-platform workflow and parity fixtures | Gaps above |
| Checked-out branch and tracking parity | L1 packed-ref fixture | Loose-ref fixtures only | Gap |
| Fallible revision/HEAD error policy | L1 corrupt-ref/HEAD fixtures | Local-ref and history corruption fixtures | Gap |
| Performance | Same-host Criterion comparison | All 16 recorded IDs improved | Pass |

## Verification

- Reviewed the specification, review 7, iteration-8 working-tree changes,
  production Git paths, L1 parity tests, CLI tests, benchmark records, and the
  macOS/Linux/Windows workflow.
- `git diff --check` passes.
- Rust tests, rustfmt, Clippy, and doctests could not run because no Rust
  toolchain is installed in this session (`rustup toolchain list` reports none).

## Decision

Not ready for production. Iteration 8 fixes the previously reported history
collection failures, but valid packed refs and remaining fallible HEAD/revision
paths can still return incorrect or valid-looking empty metadata.
