---
ready: true
agent: codex
model: ""
---

# Review

## Findings

### Medium: Worktree detail rows are rendered as double-bulleted list items

The spec's target output makes `Current Worktree:` and `Other Worktrees:` group
labels with their details on indented child rows (`spec.md:26-32`,
`spec.md:39-43`). The renderer instead adds each already-prefixed child string
to the same `UnorderedList` as a top-level item:

- `Current Worktree:` is added as one list item, then the detail rows are added
  as separate strings beginning with `"  - "` (`sniff/cli/src/output/filesystem/mod.rs:690-708`,
  `sniff/cli/src/output/filesystem/mod.rs:718-720`).
- `Other Worktrees:` is combined with the count in a single top-level list item
  rather than rendering the heading and count-only summary as separate rows
  (`sniff/cli/src/output/filesystem/mod.rs:710-713`,
  `sniff/cli/src/output/filesystem/mod.rs:723-725`).

In actual `--plain` output this produces rows like:

```text
- Current Worktree:
-   - you are in the sniff worktree located at [.](file:///...)
-   - this worktree is on the sniff branch and is 50 behind main
- Other Worktrees: there are 15 other active worktrees in this repo
```

That is not the requested target shape and reads as a literal nested bullet
inside another bullet rather than a structured worktree subsection. The current
tests assert only substring presence/counts, so they do not catch this layout
regression. I would render `Current Worktree:` and `Other Worktrees:` with a
proper nested `UnorderedList` or remove the embedded `"  - "` prefixes and
update the expected output contract accordingly.

## Verification Levels

| Requirement | Required level | Strongest evidence | Result |
|---|---:|---|---|
| Case A: linked worktree shows main location, current details, ahead/behind, and count-only others | Level 1 | CLI integration test with real temporary linked worktrees (`test_git_status_from_linked_worktree_renders_case_a`) | Covered, except output hierarchy mismatch above |
| Case B: main worktree shows current details and always renders other-worktree count, including zero | Level 1 | Renderer/unit coverage for zero count plus CLI integration coverage for linked count | Covered, except output hierarchy mismatch above |
| Case selection by physical location, not branch spelling | Level 1 | Renderer regression tests for non-main main worktree, `master`, and detached linked worktree | Covered |
| Default JSON lazy-loads non-current worktree ahead/behind; full detail restores it | Level 1 | CLI integration test `test_git_status_json_worktree_ahead_is_lazy_by_default` plus library `get_worktrees_*` tests | Covered |
| Text and JSON agree on current worktree | Level 1 | CLI integration test `test_git_status_text_and_json_agree_on_current_worktree` | Covered |
| Section headers use double underline with graceful terminal degradation | Level 2 | `just test-l2 level2_git_status --color=never` from `sniff/` passed `level2_git_status_headers_and_links_render_styled_in_tmux` | Covered |
| Worktree hyperlinks render or degrade through a real terminal | Level 2 | Same Level 2 tmux capture test | Covered |
| Exact blank-row layout around sections | Level 1 plus Level 2 smoke capture | Unit test checks exact one-row separators; Level 2 tmux test checks the Worktrees separator through a real terminal | Covered |
| Default text and JSON performance under 500 ms without remote refresh | Release-binary timing | Rebuilt `target/release/sniff`; plain `real 0.24`, JSON `real 0.19` in this checkout | Covered |

Level 3 is not applicable: this feature has no keyboard, mouse, paste, IME, or
terminal input encoder behavior.

## Verification

- Reviewed the spec, previous review, library request model, gix worktree
  detection path, CLI rendering, CLI command plan construction, docs, and
  relevant tests.
- Ran `cargo test -p sniff --lib request:: --color=never` — passed 12 tests.
- Ran `cargo test -p sniff remote_refresh::tests::get_worktrees --color=never`
  — passed 4 tests.
- Ran `cargo test -p sniff-cli test_git_status_ --color=never` — passed 12
  git-status CLI tests.
- Ran `just test-l2 level2_git_status --color=never` from `sniff/` — passed the
  Level 2 tmux styling test. The same command from the monorepo root failed
  because the root justfile has no `test-l2` recipe.
- Rebuilt the current release binary with
  `cargo build --release -p sniff-cli --bin sniff --color=never`.
- Timed the rebuilt release binary:
  - `target/release/sniff repo git-status --plain`: `real 0.24`
  - `target/release/sniff repo git-status --json`: `real 0.19`

## Decision

Ready for production. The Worktrees section now uses proper nested `UnorderedList`
components for `Current Worktree:` and `Other Worktrees:` details, matching the
documented output hierarchy. The renderer/unit tests and Level 2 terminal-rendering
test continue to pass.
