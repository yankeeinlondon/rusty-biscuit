---
ready: false
agent: codex
model: ""
---

# Review: Eliminate Redundant Repo-Root Detection in Child Env Build

## Findings

### Medium: The `None` regression test does not prove repo-root resolution occurs

`build_repo_home_env_fallback_to_cwd_when_no_effective_root` passes the repository directory itself as `cwd`, does not initialize it as a Git repository, and places the prompt fixture directly beneath that directory (`repo_home.rs:699-712`). The test would still pass if the fallback were incorrectly changed from `resolve_repo_root(cwd)` to `cwd.to_path_buf()`. It therefore verifies off-repo CWD fallback behavior, but not the acceptance criterion that `None` preserves repository-root resolution.

Use a real temporary Git repository, put `.claude/commands/review.md` at its root, pass a nested subdirectory as `cwd`, and assert that the root-level prompt is materialized. That L1 test fails if `resolve_repo_root` is bypassed and directly proves the documented fallback contract.

## Test Rigor

This feature has no terminal rendering or input-encoder behavior, so Level 2 and Level 3 tests are not required. Level 1 is appropriate for all requirements.

- Supplied effective root controls Codex shadow-home detection: Level 1, appropriate.
- Supplied effective root controls prompt materialization instead of `cwd`: Level 1, appropriate.
- Source metadata root remains distinct from launch `child_cwd` through real environment wiring: Level 1, appropriate.
- `None` invokes repository-root resolution: Level 1 test present, but the fixture cannot distinguish resolution from direct CWD reuse.
- Production call sites reuse `child_cwd`, including MCP late materialization: verified by static call-site inspection.
- Performance collapse: the captured acceptance artifact reports `repo root detect` at `0us` and `child env build` at `3.5ms` with the existing tree shape.

## Notes

The implementation otherwise matches the specification: both APIs accept `Option<&Path>`, normal and MCP-late paths pass the launch-child root, comments describe known-root reuse, and `git diff --check` passes. I could not independently rerun Cargo tests because this session has no installed Rust toolchain; `acceptance.md` records the prior successful targeted tests, compile check, and perf smoke.
