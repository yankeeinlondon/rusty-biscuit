---
status: ready for planning and implementation
reviewed: true
---

# Specification: A Status-Free Git Identity Request Level

**Review note:** This inline review keeps the feature narrow, but makes four
decisions explicit:

- `GitRequest::identity()` is the new request floor below `minimal()` and
  `summary()`.
- `GitInfo.status` becomes `Option<RepoStatus>` so identity results can honestly
  represent "status was not computed."
- Existing presets and existing CLI commands must keep their current status
  behavior and JSON shapes; only identity-only results may omit `status`.
- CLI surface is not added by default. If a command later exposes this request,
  it must render identity fields through a dedicated path rather than the
  status-oriented `git-status` renderer.

## Problem Statement

Every `GitRequest` preset forces a working-tree status walk, even the cheapest
ones. A caller who needs only the repository **root, branch, and worktree flag**
— with no notion of whether the tree is dirty — has no request level that
expresses that, so it pays for a status walk it never reads.

This is the single high-value finding (L1 + L2 + L9) carved out of the
`2026-06-06-api-surface` current-state audit. The broader API-surface redesign
in that document (preset-naming consistency, caller-directed parallelism,
generalizing the `GitRepo` handle, folding `programs`/`services` into the plan)
is **explicitly out of scope** here.

### Motivating consumer

claudine's compose-prep needs the repo root plus package structure before it
renders its execution header. The cheapest shared scan it can ask for today is
`GitRequest::summary()`, which still runs a working-tree status walk it discards.
The audit measured this at ~40 ms on the rusty-biscuit tree.

> Planning must confirm the exact claudine call site and re-measure the cost
> before vs. after, rather than trusting the audit's figure. claudine lives in a
> separate package area; this spec changes only `sniff/lib` (and, if pursued, the
> `sniff` CLI), never claudine itself.

## Current State (verified against `sniff/lib`)

All citations checked against the working tree on 2026-06-12. The original audit
was written on 2026-06-06 and has already drifted (see **Drift corrections**).

### Every preset walks the tree

`GitRepo::detect_with_request` (`git/types.rs:798`) is the funnel for all four
presets. Its status block has three branches (`git/types.rs:824-856`):

- `include_file_changes` → `get_repo_status_with_changes` (full walk).
- `is_minimal()` (true for `minimal()` and `summary()`) →
  `is_repo_dirty(&self.gix.borrow())?` — a working-tree walk that short-circuits
  on the first change but still **opens and walks the worktree**.
- otherwise → `get_repo_status_counts_detailed` (full walk).

There is no branch that produces a `GitInfo` without touching the working tree.

### The cheap capability already exists at Tier 3

`GitRepo` exposes zero-status getters that never call `statuses()`:

- `repo_root()` (`git/types.rs:587`)
- `current_branch()` / `try_current_branch()` (`git/types.rs:620,638`)
- `in_worktree()` (`git/types.rs:653`), `base_repo_root()` (`git/types.rs:658`)
- `head_id()` (`git/types.rs:603`)

These are reachable only by a caller who imports `GitRepo` directly. They are
invisible from `request.rs`, and they receive none of the plan's scheduling.

### The plan already discovers the handle once

The filesystem stage discovers the repository a single time up front and threads
that handle into the git stage (`filesystem/mod.rs:101-129`):

```rust
let discovered_git = match request.git.as_ref() {
    Some(_) => GitRepo::discover(root)?,   // one parent-walk + open
    None => None,
};
// ...later, on a scoped thread:
let git = match discovered {
    Some(repo) => repo.detect_with_request(git_request).map(Some),
    None => Ok(None),
};
```

So the discovered handle — which already knows `repo_root()`, `current_branch()`,
and `in_worktree()` for free — is in hand at the exact point the status walk is
triggered. A status-free level needs only to *return early* from
`detect_with_request` instead of falling into the status block.

### Drift corrections to the 2026-06-06 audit

The narrow scope changes nothing about these, but the planning phase must not
trust the stale line counts:

1. **L6 / field count.** `GitRequest` now has **9 fields**, not 8
   (`full_worktree_details` was added) plus a `wants_repo_metadata()` method
   (`request.rs:279-303,427`). Two fields still lack builder setters
   (`include_remote_branch_details`, `include_commit_remote_containment`).
2. **L9 / "duplicate repo-root derivation."** The audit says `GitRepo::discover`
   and `detect_repo_identity` "each re-open the repo via raw `git2`
   independently." That is now inaccurate. Both route through the **same** pure
   gix entry point, `open::trusted_discover` (`git/api.rs:38`, `git/types.rs:565`;
   `detect_repo_identity` → `filesystem::repo_root` → `trusted_discover`,
   `repo/identity.rs:74` → `git/api.rs:36-39`). The remaining waste is two
   *discovery calls* (two parent-walks + opens) where the plan could share one
   handle — a smaller, real dedup, not two competing implementations.

## Target Behavior

Add a git request level that returns repository **identity only** — root,
branch, HEAD id, worktree flag — and provably **does not walk the working tree**.
Expose it through the plan so the motivating consumer can request it via
`DetectionPlan → FilesystemRequest → GitRequest` without dropping to Tier 3.

### 1. A status-free request level

Introduce an identity level on `GitRequest`. Shape:

- A `GitRequest::identity()` preset, the new floor below `minimal()`.
- A predicate `is_identity_only(&self) -> bool` that `detect_with_request`
  checks **before** the status block, returning a `GitInfo` populated purely from
  the discovered handle's zero-status getters.
- `GitRequest::is_minimal()` remains true only for the existing dirty-flag
  presets. Do not silently fold `identity()` into `is_minimal()`, because that
  would preserve the current status walk.
- `GitRequest::wants_repo_metadata()` remains false for `identity()`.

The detection branch in `detect_with_request` gains a first arm:

```rust
let current_branch = self.try_current_branch()?;   // already the first line today

if request.is_identity_only() {
    return Ok(self.identity_only_info(current_branch));  // no is_repo_dirty, no walk
}
```

`identity_only_info` fills `repo_root`, `current_branch`, `in_worktree`,
`base_repo_root`, `org`/`repo` (cheap preferred-remote URL parse, no fetch),
`head_id`, and leaves all collections empty.

Identity mode must keep the existing HEAD error policy: legitimate detached or
unborn HEAD states return `current_branch: None`; malformed, unreadable, or
otherwise corrupt HEAD state still returns an error. `head_id` is optional and is
`None` for an unborn HEAD.

Remote handling is intentionally narrow. Identity mode may read configured
remote URLs to populate the cheap `org`/`repo` convenience fields, but it must
not populate `remotes`, branch metadata, tracking status, or anything that
requires network access, ref graph walks, or status traversal.

### 2. `GitInfo.status` shape

`GitInfo.status` is a **non-optional** `RepoStatus` (`git/types.rs:987`).
"Identity, status not computed" cannot be expressed today.

**Decision: make `GitInfo.status` an `Option<RepoStatus>`.** Identity mode yields
`None`; every existing preset yields `Some`. Add
`#[serde(skip_serializing_if = "Option::is_none")]` so identity JSON omits the
field instead of serializing `"status": null`.

The reviewed alternatives were:

| Option | Change | Cost |
| --- | --- | --- |
| **A. `status: Option<RepoStatus>`** (selected) | Make the field optional; `identity()` yields `None`; every other preset yields `Some`. | Touches all `GitInfo.status` readers in lib + CLI. Cleanest semantics: absence is honest and JSON can omit the field. |
| B. Sentinel `RepoStatus` | Keep the type; add `computed: bool` (default true). | Less reader churn, but a `RepoStatus` whose counts are meaningless is a footgun the next reader must remember. |
| C. Sanction Tier 3 only | Don't change `GitInfo`; document `GitRepo::discover().repo_root()` as the supported answer and make it discoverable from `request.rs` docs. | Zero type change, but does **not** give the consumer plan scheduling; fails the stated goal. |

Implementation impact:

- Update every Rust reader of `git.status` in `sniff/lib` and `sniff/cli` to
  handle `Option<RepoStatus>` explicitly. Status-oriented paths may produce a
  clear internal error if called with identity-only data; they must not
  silently treat `None` as clean.
- Existing presets (`minimal`, `summary`, `full`, `deep`) must still serialize a
  top-level `status` object. This preserves `sniff repo git-status --json`,
  package-scoped git-status JSON, and existing library behavior for callers that
  do not opt into `identity()`.
- Tests that construct `GitInfo` fixtures must set `status: Some(...)` unless
  they are specifically testing identity-only behavior.
- Documentation examples that read `info.status.is_dirty` must unwrap or match
  the optional field and explain that `None` means status was not requested, not
  that the repository is clean.

### 3. Share the discovered handle (the real L9 dedup) — secondary

Where the plan has already discovered a handle, `detect_repo_identity` should be
able to reuse it instead of issuing a second `trusted_discover`. Scope this as a
**follow-on** within the same spec only if it stays additive (e.g. a
`detect_repo_identity_with_repo(&GitRepo)` variant the plan calls, leaving the
public `detect_repo_identity(&Path)` intact). If it forces signature churn on
public callers, defer it to its own change.

### 4. CLI surface — optional, default off

No new subcommand is required to satisfy the library consumer. If a CLI affordance
is wanted, the natural home is making an existing identity-style command (e.g.
`sniff repo name`) route through the new level. Decide explicitly rather than
adding surface by default (Simplicity First).

Do **not** route `sniff repo git-status` through `GitRequest::identity()`. That
command's contract is status-oriented, and its JSON tests currently assert a
top-level `status` object. If a future CLI command uses identity mode, it must
return focused identity JSON and render with a small identity-specific output
path using `biscuit-terminal` components where styled terminal output is needed.

## Success Criteria

1. A new test proves the identity level produces correct `repo_root` +
   `current_branch` + `in_worktree` + `head_id` **and never invokes a status
   walk**. Preferred proof is a test-only counter around the status entry points
   (`is_repo_dirty`, `get_repo_status_with_changes`, and
   `get_repo_status_counts_detailed`) or an equivalent test-only instrumentation
   point. A dirty-fixture test that only observes `status: None` is useful, but
   is not sufficient by itself because it does not prove the walk was skipped.
2. `DetectionPlan` can express "git identity only, repo structure only, nothing
   else," end-to-end through `detect_with_plan`, returning a `SniffResult` with no
   status walk performed.
3. The motivating claudine path can be expressed against the new level; cost
   re-measured before/after on the rusty-biscuit tree and recorded.
4. All four existing presets (`minimal`/`summary`/`full`/`deep`) are unchanged in
   behavior; existing `GitInfo` consumers compile and pass after handling
   `Option<RepoStatus>`.
5. Docs updated in the same change: `request.rs` module docs, the
   `sniff-library-architecture.md` cost model, and the sniff skill cheat sheet
   (which currently states "**Every** preset … runs a working-tree status walk" —
   that sentence becomes false and must be corrected).
6. Existing `sniff repo git-status --json` and package-scoped git-status JSON
   still include a top-level `status` object. New identity-only JSON omits
   `status`.
7. Identity mode works in main worktrees, linked worktrees, detached HEAD, and
   unborn HEAD repositories on macOS, Linux, and Windows-compatible paths. The
   tests may use fixtures and host-independent path assertions; no network is
   required.

## Out of Scope

Inherited verbatim from the `2026-06-06-api-surface` audit's "Out of scope":

- L7 preset-naming unification (`summary` vs `interfaces_only` vs `structure`).
- L10 / L1 caller-directed parallel runner for arbitrary metric subsets.
- Generalizing the `GitRepo` handle pattern to every domain (audit Q2).
- Folding `programs` / `services` / `package` into the plan vocabulary (L8, Q1).
- Adding the missing `GitRequest` setters (L6) and `RepoRequest` builders — touch
  only if a planned change already edits those lines.

## Open Questions

1. **L9 dedup (§3)** — include the handle-sharing follow-on in this spec, or
   split it out?

   - Include it when additive: fewer duplicate opens in the plan, and the same
     implementation work already has the discovered handle in scope. Risk is
     small if the public `detect_repo_identity(&Path)` API remains intact.
   - Split it out: keeps this feature focused on the `GitRequest` contract and
     avoids mixing two performance changes. Cost is leaving one known duplicate
     discovery in place after this work.

   **Recommendation:** include it only if it is an additive
   `detect_repo_identity_with_repo(&GitRepo)`-style helper used internally by the
   plan. Split it out if public signatures need to change.

2. **CLI affordance (§4)** — route an existing identity command through the new
   level, or leave this library-only?

   - Library-only: smallest implementation; directly serves claudine's
     compose-prep path; avoids introducing another user-visible JSON shape.
   - Reuse an existing identity-style command: proves the path through the CLI
     and may reduce latency for commands like `sniff repo name`, but expands the
     test matrix and requires dedicated rendering/JSON handling.

   **Recommendation:** leave this library-only for the first implementation.
   Add CLI use after the library behavior and compatibility migration are
   stable.
