---
ready: false
agent: codex
model: ""
---

# Review: Eliminate Redundant Repo-Root Detection in Child Env Build

## Findings

### High: Missing composition-path regression for the source-repo vs launch-repo split

The spec requires a regression test where a composed source document lives outside the launch repo and Codex shadow-HOME prompt materialization still uses the launch-child root, not the source metadata root. The implementation adds useful low-level `repo_home` tests in `claudine/cli/src/commands/wrap/repo_home.rs`, especially `build_repo_home_env_uses_supplied_effective_root_not_cwd`, but that test calls `build_repo_home_env` directly with a hand-picked `Some(&launch_repo)`.

That does not exercise the actual composition/environment wiring that can regress here: `execute_composition_request_inner` -> `EnvPlan` -> late MCP materialization, or `build_child_env_with_launch` with a `LaunchWorkspaceContext` whose `repo_root` differs from `child_cwd`. A future implementation could accidentally pass `env_plan.repo_root` / source metadata to the shadow-HOME call and still keep the low-level `repo_home` tests green.

Relevant code:

- `claudine/cli/src/commands/wrap/composition/mod.rs:1216` passes `Some(env_plan.child_cwd.as_path())` in the MCP late materialization path.
- `claudine/cli/src/commands/wrap/env.rs:178` passes `Some(launch_ctx.child_cwd.as_path())` in the normal shadow-HOME path.
- `claudine/cli/src/commands/wrap/repo_home.rs:648` covers only the low-level helper contract.
- `claudine/features/2026-06-05-perf-low-hanging-fruit/plan.md:82` and `:90` mark the composition/integration regression as complete, but I do not see a corresponding integration or higher-level env test in the diff.

Add an L1 test that constructs a `LaunchWorkspaceContext` or composition request with `repo_root != child_cwd`, uses Codex shadow-HOME materialization, and asserts the shadow prompts come from `child_cwd`. L1 is the right level here; no terminal emulator behavior is involved.

### Medium: Acceptance/perf verification is not reproducible from this change

The spec’s acceptance criteria require proving the hot-path repo-root detection collapsed in a `--perf --dry-run --repo` run and that targeted tests/checks pass. The plan marks those as done at `claudine/features/2026-06-05-perf-low-hanging-fruit/plan.md:89` through `:95`, but there is no captured command output or artifact in the feature directory, and I could not run them in this session because rustup reports that no toolchains are installed.

The code likely removes the redundant shadow-HOME `resolve_repo_root` call on the updated call sites, but production readiness should wait until the author records or reruns:

- `cargo test -p claudine-cli repo_home --lib --color=never`
- the composition/source-vs-launch regression test
- `cargo check -p claudine -p claudine-cli --color=never`
- a `claudine compose --perf --dry-run --repo ...` smoke showing `child env build -> shadow home sync -> repo root detect` is microsecond-scale

### Low: A nearby perf comment still describes the old dominant cost as unconditional

`claudine/cli/src/commands/wrap/env.rs:94` through `:97` still says the perf breakdown points at "the sniff git walk inside the shadow sync" as the real cost. The code below now passes a known effective root on the production path, so this should be softened the same way the later rustdoc was updated. This is comment drift only; the implementation below it is the source of truth.

## Test Rigor

The feature is environment/root-selection and perf-instrumentation work. The appropriate verification level is Level 1: unit/integration tests with temporary directories and process environment control. No Level 2 or Level 3 coverage is required because there are no terminal rendering, terminal input encoder, keyboard, mouse, paste, or modifier-key requirements.

Current strongest observed coverage:

- Supplied effective root affects Codex prompt detection: Level 1 unit coverage.
- `build_repo_home_env(..., Some(root))` uses the supplied root instead of `cwd`: Level 1 unit coverage.
- `build_repo_home_env(..., None)` fallback behavior: Level 1 unit coverage.
- Actual composition/source-repo vs launch-repo shadow-HOME wiring: missing or not visible in this diff.
- Manual `--perf --dry-run --repo` acceptance: not reproducible in this session.

## Notes

The implementation approach is otherwise conservative and matches the spec: `needs_shadow_home` and `build_repo_home_env` accept `Option<&Path>`, normal env construction passes `launch_ctx.child_cwd`, and MCP late materialization passes `env_plan.child_cwd`. I would not add a cache here.

I attempted to run `cargo test -p claudine-cli repo_home --lib --color=never`, but the environment has no configured Rust toolchain:

```text
error: rustup could not choose a version of cargo to run, because one wasn't specified explicitly, and no default is configured.
```

