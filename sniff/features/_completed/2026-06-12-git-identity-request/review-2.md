---
ready: true
agent: codex
model: ""
---

# Review: Git Identity Request, Iteration 2

## Findings

No blocking findings.

The iteration addresses the prior review's production blockers:

- Status-bearing `GitInfo` payloads keep `head_id` absent, preserving existing `sniff repo git-status --json` shape while identity-only payloads populate `head_id` and omit `status`.
- The status-walk proof is now path-scoped instead of a reset global counter, so it passes under the standard Rust test runner even with concurrent tests.
- CLI render and selection helpers tolerate `GitInfo { status: None }` without panicking or claiming the tree is clean; status-derived answers become indeterminate or skip the status line.

## Requirement Coverage

- `GitRequest::identity()` exists, is not folded into `is_minimal()`, and returns before any status path: Level 1 unit coverage at `sniff/lib/src/request.rs` and `sniff/lib/src/filesystem/git/types.rs:830`.
- Identity output includes repo root, branch, `head_id`, worktree flag, base root, and cheap org/repo while leaving status-bearing collections empty: Level 1 unit coverage in `identity_request_on_branch_has_expected_fields` and related HEAD/worktree tests.
- Identity mode does not invoke a working-tree status walk: Level 1 proof in `identity_request_does_not_walk_status`, with path-scoped instrumentation around the status entry points.
- Plan-level "git identity only + repo structure only" is expressible and avoids status walks end-to-end: Level 1 proof in `identity_plan_does_not_walk_status_end_to_end`.
- `GitInfo.status` is optional and identity JSON omits `status`; existing presets still include `status`: Level 1 serialization coverage in `identity_request_serializes_without_status_field` and CLI JSON coverage for `repo git-status --json`.
- Main worktree, linked worktree, detached HEAD, and unborn HEAD behavior are covered at Level 1. Existing malformed HEAD behavior is preserved through the shared fallible `try_current_branch()?` path before the identity early return.
- Docs and the sniff skill now describe `identity()` as the status-free floor, and the plan records the before/after measurement for the motivating claudine path.

## Verification Levels

Level 1 is the appropriate verification level for this feature. The user-observable behavior is Rust API behavior, JSON shape, CLI command data shape, and absence of a library status walk. There are no terminal-emulator rendering, keyboard input, paste/IME, mouse, or scroll requirements that would require Level 2 or Level 3 coverage.

## Verification Performed

- `cargo test --color=never -p sniff identity_request --lib`
- `cargo test --color=never -p sniff identity_plan_does_not_walk_status_end_to_end --lib`
- `cargo test --color=never -p sniff git_request_ --lib`
- `cargo test --color=never -p sniff-cli identity_only_git_info_yields_indeterminate_not_panic --lib`
- `cargo test --color=never -p sniff-cli build_git_status_returns_git_info_shape --lib`
- `cargo test --color=never -p sniff-cli test_git_status_subcommand_json_output --test cli`
- `cargo check --color=never -p sniff-cli --tests`
