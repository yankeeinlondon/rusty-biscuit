# Plan: add commit time to merge / non-conventional commit lines

Derived from [`spec.md`](./spec.md). Single-file presentation fix plus a unit test.

## Target

`sniff/cli/src/output/filesystem/mod.rs` — `format_commit_line()`, the `else`
(non-conventional) branch at ~lines 329–341.

## Step 1 — Add the time segment to the non-conventional format string

Current code (~lines 337–340):

```rust
format!(
    "[{}] <dim>{}</dim> {}<blue><b>{}</b></blue>{}{}",
    sha_display, truncated, date_prefix, date_str, refs_part, user_part,
)
```

Change to insert the time segment (mirroring the conventional branch's
`<i>at</i> <blue><b>{time_str}</b></blue> ` immediately before `date_prefix`):

```rust
format!(
    "[{}] <dim>{}</dim> <i>at</i> <blue><b>{}</b></blue> {}<blue><b>{}</b></blue>{}{}",
    sha_display, truncated, time_str, date_prefix, date_str, refs_part, user_part,
)
```

Notes:
- `time_str` is already in scope (computed at line ~294 via
  `format_commit_datetime`). No new bindings needed; this removes the existing
  "computed but unused for this branch" gap.
- Keep the message dimmed and all other parts (refs, author) intact.
- Conventional branch is untouched.

## Step 2 — Add a unit test

In `sniff/cli/src/output/filesystem/mod.rs`, inside the existing
`mod git_status_rendering` test module (or a sibling unit-test module near
`format_commit_line`), add a test that:

1. Builds a `CommitInfo` whose `message` is a merge subject (e.g.
   `"Merge branch 'claudine'"`) with a fixed `timestamp`.
2. Calls `format_commit_line(&commit, 0, None)`.
3. Asserts the returned markup contains the `at` + time segment (e.g. contains
   `<i>at</i>` and the expected `time_str` produced for that timestamp), and still
   contains the merge subject and SHA.

Guidance for a deterministic assertion:
- `format_commit_datetime` converts to **local** time and uses relative-day labels
  (`Today`/`Yesterday`/`on <date>`), which are not stable across machines/dates.
  Make the test robust by asserting on the **presence of the `at` segment markup**
  (`"<i>at</i>"`) and that the merge subject is present, rather than hard-coding an
  absolute clock string. If a precise time assertion is desired, derive the expected
  `time_str` by calling `format_commit_datetime(&commit.timestamp)` in the test and
  assert the output contains that value — this avoids timezone flakiness.
- Add a companion assertion that a conventional commit still renders `<i>at</i>`
  (guards against regressions) if not already covered.

Check existing tests in the file for the established pattern of constructing
`CommitInfo` fixtures and reuse it (same field set: `sha`, `message`, `author`,
`timestamp`, `remotes`, `refs`).

## Step 3 — Verify

From the sniff area:

- `just test` (sniff) — new test passes; `mod git_status_rendering` tests still pass.
- `just lint` (sniff) — clippy/fmt clean.

## Out of Scope (do not touch)

- `sniff/cli/src/output/commit_blocks.rs` — already includes the time for all commit
  types.
- `sniff/lib/**` — no library changes.
- `format_commit_datetime()` logic.

## Acceptance

All criteria in `spec.md` satisfied: merge commits show `at <time>` in the correct
position/style; conventional lines unchanged; tests + lint green.
