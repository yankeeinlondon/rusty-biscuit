---
ready: true
agent: codex
model: ""
---

# Review

## Findings

### High: The mandatory performance and cross-platform release gates remain incomplete

The specification makes same-host Criterion comparison a hard production gate.
No gix-versus-git2 comparison results are recorded. The execution plan still
defers every comparison to Phase 8, and `sniff/lib/baselines/git2.md` explicitly
says its reduced-sampling baseline must be recaptured before a real
no-regression decision. There is likewise no evidence that the final
implementation builds and passes parity tests on macOS, Linux, and Windows, or
that the 12-key system-config behavior was validated on macOS and Windows.

This blocks production readiness regardless of the implementation's functional
state. Run the complete specified benchmark filter against a same-host baseline,
record each Criterion decision and approved high-variance exception, then
provide three-platform build/L1 results and macOS/Windows config-layer parity.

### High: Production repository opens still bypass or suppress trust failures

The migration contract requires every production open to reject untrusted
repositories and distinguish open failures from repository absence. Several
paths violate that contract:

- `blast_radius.rs:77-79` reopens the already trusted repository with
  `gix::open()` and converts every failure into an empty last-commit path list.
- `worktree.rs:135-140` suppresses failure to open the main repository and then
  continues using the linked-worktree handle.
- `worktree.rs:190-194` uses raw `gix::open()` for linked worktrees and turns
  every failure into `(branch: None, detached: false)`.
- `remote_refresh.rs:601` uses raw `gix::open(...).ok()?`, silently omitting a
  worktree on trust, permission, corruption, or I/O failure.
- `discovery.rs:192-197` suppresses failure to open the main repository while
  resolving the base branch.

The public worktree APIs are fallible, so they should propagate these failures.
The infallible `GitRepo::worktrees()` surface needs an explicit, documented
error policy rather than silently violating the trusted-open invariant.

### High: Remote-containment pruning is incorrect when commit timestamps are skewed

`remote_refresh.rs:447-452` stops an ancestry walk at the first commit older than
the oldest requested commit. Commit timestamps are not monotonic along ancestry:
an old-dated child can have a newer-dated parent. In that case the `break`
prevents the parent from being visited and incorrectly omits the remote from a
requested commit's `remotes`.

The same migration already corrected this assumption in
`recent_commits.rs:393-403`, where out-of-range commits use `continue` because
gix's commit-time walk is a lazy frontier, not a globally monotonic sequence.
Containment needs the same treatment, or an ID-based stopping strategy that
cannot discard unseen requested commits. Add a skewed-timestamp ancestry test.

### Medium: Required parity coverage is still incomplete

The strongest applicable verification level for all git and CLI requirements is
Level 1; no terminal encoder or rendering behavior is involved, so Level 2 and
Level 3 are not required. The current Level 1 coverage still misses specified
contracts:

- The config test checks only `user.name` and `user.email`, not all 12 keys,
  source precedence, or the macOS/Windows extra system config.
- Ref parity covers ordinary UTF-8 local branches and a lightweight tag, but not
  remote refs, symbolic refs, annotated tags, or the required non-UTF-8 ref
  policy.
- Worktree tests do not exercise trusted-open failures or prove that a failed
  linked-worktree open is surfaced rather than omitted/misreported.
- There is no skewed-timestamp remote-containment test for the incorrect pruning
  above.

These are Level 1 gaps. There are no user-observable terminal requirements in
this feature that need Level 2 or Level 3 verification.

## Verification

- Reviewed the specification, execution plan, prior review, staged production
  changes, parity tests, benchmark harness, and baseline record.
- `git diff --cached --check` passes.
- Production `git2` use is removed; remaining source references are test-only.
- CLI production source contains no `git2` or `gix` imports.
- Tests, Clippy, doctests, and metadata could not be rerun because rustup has no
  configured default toolchain in this session.

## Decision

Not ready for production. The backend migration is substantially more complete
than iteration 1, but trust/error semantics, remote-containment correctness, and
the specification's mandatory performance and cross-platform gates remain open.
