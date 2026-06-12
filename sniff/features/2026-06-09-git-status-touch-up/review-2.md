---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### Medium: Worktree hyperlinks do not use the spec's relative path labels

The spec's target format requires worktree links to use an absolute href with a
relative visible label (`{relative-path}`) for the main and current worktree
paths (`spec.md:18-25`, `spec.md:32-37`). The implementation now renders a
home-abbreviated absolute label instead:

- `worktree_path_link()` sets the href to the absolute path and the label to
  `home_abbreviated(path)` (`sniff/cli/src/output/filesystem/mod.rs:548-558`).
- `home_abbreviated()` returns `~/...` when the path is under home, otherwise
  the full absolute path (`sniff/cli/src/output/filesystem/mod.rs:563-574`).
- The renderer uses that helper for the Case A `main:` line, the Case A current
  worktree line, and the Case B current worktree line
  (`sniff/cli/src/output/filesystem/mod.rs:640-688`).

That means common layouts outside `$HOME` show labels like
`/tmp/demo/project`, not a relative label such as `..` or `../project`.
The tests pin the new home-abbreviated behavior rather than the spec contract
(`sniff/cli/src/output/filesystem/mod.rs:1601-1649`), so this regression is now
protected by tests.

Either change the renderer/tests to compute the visible label relative to the
current worktree/repository context, or update the specification to explicitly
accept home-abbreviated absolute labels. As written, the implementation does
not satisfy the documented output format.

## Verification Levels

| Requirement | Required level | Strongest evidence | Result |
|---|---:|---|---|
| Case A: linked worktree shows main location, current details, ahead/behind, and count-only others | Level 1 | CLI integration test with real temporary linked worktrees (`test_git_status_from_linked_worktree_renders_case_a`) | Covered, except path label mismatch above |
| Case B: main worktree shows current details and always renders other-worktree count, including zero | Level 1 | Renderer/unit coverage for zero count plus CLI integration coverage for linked count | Covered |
| Case selection by physical location, not branch spelling | Level 1 | Renderer regression tests for non-main main worktree, `master`, and detached linked worktree | Covered |
| Default JSON lazy-loads non-current worktree ahead/behind; full detail restores it | Level 1 | CLI integration test `test_git_status_json_worktree_ahead_is_lazy_by_default` plus library `get_worktrees_*` tests | Covered |
| Text and JSON agree on current worktree | Level 1 | CLI integration test `test_git_status_text_and_json_agree_on_current_worktree` | Covered |
| Section headers use double underline with graceful terminal degradation | Level 2 | `just test-l2 level2_git_status --color=never` passed `level2_git_status_headers_and_links_render_styled_in_tmux` | Covered |
| Worktree hyperlinks render or degrade through a real terminal | Level 2 | Same Level 2 tmux capture test | Covered |
| Exact blank-row layout around sections | Level 1 plus Level 2 smoke capture | Unit test checks exact one-row separators; Level 2 tmux test checks the Worktrees separator through a real terminal | Covered enough for this feature |
| Default text and JSON performance under 500 ms without remote refresh | Benchmark/manual timing | Existing release binary measured `0.16s` text and `0.17s` JSON in this checkout; plan records ~70-85 ms on the large monorepo | Covered |

Level 3 is not applicable: this feature has no keyboard, mouse, paste, IME, or
terminal input encoder behavior.

## Verification

- Reviewed the spec, plan, review-1, library request model, gix worktree
  detection path, CLI rendering, CLI command plan construction, docs, and
  relevant tests.
- Ran `cargo test -p sniff --lib request:: --color=never` — passed.
- Ran `cargo test -p sniff remote_refresh::tests::get_worktrees --color=never`
  — passed.
- Ran `cargo test -p sniff-cli test_git_status_ --color=never` — passed
  12 git-status CLI tests.
- Ran `just test-l2 level2_git_status --color=never` from `sniff/` — passed
  the Level 2 tmux styling test.
- Timed the existing release binary:
  - `target/release/sniff repo git-status --plain`: `real 0.16`
  - `target/release/sniff repo git-status --json`: `real 0.17`

## Decision

Not ready for production as specified. The review-1 high-severity performance,
Case A/B, JSON, and Level 2 styling gaps appear addressed, but the visible
worktree path labels still do not match the spec's relative-label requirement.
