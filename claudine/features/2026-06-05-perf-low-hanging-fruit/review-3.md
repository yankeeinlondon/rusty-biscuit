---
ready: true
agent: codex
model: ""
---

# Review: Eliminate Redundant Repo-Root Detection in Child Env Build

## Findings

No blocking findings.

The implementation satisfies the specification's root-reuse contract:

- `needs_shadow_home` and `build_repo_home_env` accept the optional effective root and use lazy fallback resolution, so `resolve_repo_root` is not called when a root is supplied.
- Normal environment construction passes `launch_ctx.child_cwd`, and MCP late materialization passes `env_plan.child_cwd`.
- The source-metadata-root versus launch-child-root split remains intact.
- The shadow-HOME behavior and perf tree shape are otherwise unchanged.

The iteration #1 and #2 findings are closed. The environment-wiring regression now exercises `repo_root != child_cwd`, and the `None` fallback test starts from a nested directory in a real Git repository, so it fails if repository-root resolution is replaced with direct CWD reuse.

## Test Rigor

All requirements are appropriately verified at Level 1. This feature changes filesystem root selection and performance instrumentation; it has no terminal rendering, terminal input encoding, keyboard, mouse, paste, or IME behavior requiring Level 2 or Level 3 coverage.

- Supplied-root selection in `needs_shadow_home`: Level 1, appropriate.
- Supplied-root Codex prompt materialization: Level 1, appropriate.
- `None` fallback repository-root resolution: Level 1 with a real local Git fixture, appropriate.
- Source metadata root versus launch-child root through environment wiring: Level 1, appropriate.
- Production call sites, including MCP late materialization: verified by static call-site inspection.
- Performance acceptance: the captured smoke result reports `repo root detect` at `0us` and `child env build` at `3.5ms`, preserving the existing tree shape.

## Verification Notes

Scoped diff inspection and call-site searches found no missing callers. The feature files pass whitespace checks; repository-wide `git diff --check` reports only unrelated pre-existing whitespace in other changed files.

I could not independently rerun Cargo tests because this session has no active Rust toolchain. `acceptance.md` records successful targeted `claudine-cli` tests, a compile check for both Claudine crates, and the perf smoke result. The implementation and those captured results are internally consistent.
