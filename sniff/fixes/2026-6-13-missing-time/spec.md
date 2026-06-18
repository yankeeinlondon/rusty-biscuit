# Fix: `sniff repo git-status` omits commit time for merge commits

## Problem

In the recent-commits list rendered by `sniff repo git-status`, merge commits (and
any other non–conventional-commit message) are printed **without the time** of the
commit, while conventional commits show it.

Observed output:

```
- [1389882] feat(darkmatter) at 10:05am on 2026-06-05: shell spans and capture timings in perf report
- [caee9dd] docs(claudine) at 10:51am on 2026-06-05: add better-perf-metrics plan, inventory, and reviews
- [3e2ca7f] Merge branch 'claudine' on 2026-06-05                          <-- no "at 10:51am"
- [c54e496] chore at 10:52am Today: stop tracking .opencode/package-lock.json
```

The merge commit line `[3e2ca7f] Merge branch 'claudine' on 2026-06-05` is missing the
`at <time>` segment that every conventional commit displays. This happens for merge
commits both on prior days (`on <date>`) and on the current day (`Today`/`Yesterday`).

## Root Cause

`format_commit_line()` in
`sniff/cli/src/output/filesystem/mod.rs` (lines ~288–342) branches on whether the
message parses as a conventional commit:

- **Conventional branch** (`cc.operation` is `Some`) — format string includes
  `<i>at</i> <blue><b>{time_str}</b></blue>` (line ~318).
- **Non-conventional branch** (`cc.operation` is `None`, line ~329) — format string
  is:
  ```rust
  "[{}] <dim>{}</dim> {}<blue><b>{}</b></blue>{}{}"
  // sha_display, truncated, date_prefix, date_str, refs_part, user_part
  ```
  It builds `time_str` via `format_commit_datetime()` but **never uses it**.

Merge commit subjects (e.g. `Merge branch 'claudine'`) do not match the conventional
commit regex (`^([a-zA-Z0-9-]+)(?:\(([^)]*)\))?: (.+)$` in
`sniff/lib/src/filesystem/git/types.rs`), so they always fall into the
non-conventional branch and lose their time.

This is presentation-only logic; it correctly lives in the CLI. No library change is
required. The timestamp is already available on `CommitInfo.timestamp`, and
`format_commit_datetime()` already computes the correct `time_str`.

## Desired Behavior

Non-conventional commits (including merges) must show the time in the same position
and style as conventional commits.

Expected output for the merge line:

```
- [3e2ca7f] Merge branch 'claudine' at 10:51am on 2026-06-05
```

And for a same-day non-conventional commit:

```
- [abc1234] Merge branch 'x' at 10:52am Today
```

### Formatting rules (must match the conventional branch)

- Insert `<i>at</i> <blue><b>{time_str}</b></blue> ` immediately before the existing
  `{date_prefix}` segment.
- Keep the existing `date_prefix` logic: `<i>on</i> ` when `use_on` is true (older
  dates), empty for `Today`/`Yesterday`.
- Preserve all other parts unchanged: SHA display (incl. OSC8 hyperlink when present),
  the dimmed/truncated message, ref decorations, and the optional `--verbose` author
  suffix.
- The message remains dimmed (`<dim>{truncated}</dim>`); only the time segment is
  added.

## Scope / Non-Goals

- **In scope:** the non-conventional branch of `format_commit_line()` in
  `sniff/cli/src/output/filesystem/mod.rs`.
- **Out of scope:** `commit_blocks.rs` `render_commit_block()` already emits
  `<i>at</i> <b>{time_label}</b>` for every commit regardless of type — it is **not**
  affected and must not be changed.
- No library (`sniff/lib`) changes.
- No change to `format_commit_datetime()` behavior.

## Acceptance Criteria

1. `sniff repo git-status` shows `at <time>` for merge commits, matching the position
   and styling of conventional commits.
2. Same-day merge commits show `at <time> Today` (no `on`); older merge commits show
   `at <time> on <date>`.
3. Conventional commit lines are unchanged.
4. A unit test in `sniff/cli/src/output/filesystem/mod.rs` asserts that a
   non-conventional (merge) commit line includes the time segment. Existing tests in
   `mod git_status_rendering` continue to pass.
5. `just test` and `just lint` pass for the sniff area.
