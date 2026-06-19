---
title: Eliminate redundant repo-root detection in child env build
date: 2026-06-05
status: ready for planning and implementation
reviewed: true
area: claudine
scope: claudine-cli wrapper environment performance
depends-on: claudine/features/2026-06-05-better-perf-metrics/spec.md
---

# Eliminate Redundant Repo-Root Detection in Child Env Build

## Background

The `claudine` CLI `--perf` flag attributes the overwhelming majority of a
`compose` / `inline-compose` / `sequence` launch to a single leaf,
`environment setup -> child env build`. On a large worktree this leaf measured
~2.11s (73% of total wall-clock).

The child sub-metrics added under `child env build` localize the cost precisely:

```
child env build
├─ env sanitize        ~236µs
└─ shadow home sync    ~826ms
   └─ repo root detect ~822ms   <- the entire cost
```

`repo root detect` is a single call to `resolve_repo_root`
(`claudine/lib/src/linking/paths.rs`), which runs `sniff::filesystem::detect_git`.
On this repository that git walk costs ~660ms–2s. The shadow-HOME file
copy/symlink work everyone would suspect is only ~2.7ms.

The cost is **redundant**: the repo root is already resolved earlier in the same
launch and is in hand at the call site.

## Problem

Two code paths re-derive a repo root that the caller already holds:

### Opportunity 1 — `build_repo_home_env` (primary, ~all of the cost)

`build_child_env_with_launch` (`claudine/cli/src/commands/wrap/env.rs`) receives
a `LaunchWorkspaceContext` that already carries the resolved child working
directory in `child_cwd: PathBuf`. When the launch CWD is inside a repo,
`child_cwd` is already the launch repo root; when no repo is detected,
`child_cwd` is the launch CWD fallback. It passes only that path into
`repo_home::build_repo_home_env` (`claudine/cli/src/commands/wrap/repo_home.rs`),
which then calls `resolve_repo_root(cwd)` again from scratch.

Inside `build_repo_home_env`, the resolved `repo_root` is consumed **only** by
`materialize_repo_scoped_resources`, which is a no-op for every provider except
Codex (it locates Codex repo-local prompt directories). For every non-Codex
`--repo` launch — the common case — the git walk produces a value that flows
straight into a no-op.

Reader note: do not confuse the launch-child root with
`LaunchWorkspaceContext::repo_root` / `EnvPlan::repo_root`. In composition flows,
`repo_root` is metadata and may intentionally follow the composed source
document's repo, while `child_cwd` follows the user's launch repo so the wrapped
provider does not jump into an unrelated checkout. This feature must preserve
that split. Shadow-HOME materialization currently resolves from `child_cwd`, so
the optimized path must reuse the already-selected `child_cwd`-root value, not
blindly substitute the metadata repo root.

The shadow-HOME branch runs only when `needs_shadow_home` is true, i.e. under
`--repo` (`repo_only`) or a Codex prompt overlay. Without `--repo`, the branch
is skipped and `child env build` is ~300µs.

### Opportunity 2 — `needs_shadow_home` Codex branch (secondary)

`repo_home::needs_shadow_home` calls `resolve_repo_root(cwd)` on its Codex
branch. For plain `--repo` launches the `repo_only` term short-circuits before
that call, so it does not fire; but a **Codex non-`--repo`** launch resolves the
repo root here and then again inside `build_repo_home_env`, walking twice.

### Opportunity 3 — MCP shadow-HOME late materialization

`execute_composition_request_inner` can call `repo_home::build_repo_home_env`
directly when MCP injection discovers that a provider needs a shadow HOME and
the earlier environment plan did not create one. That late path currently passes
`env_plan.child_cwd` and would continue to re-run `resolve_repo_root` unless the
new API is applied to every call site, not only `env.rs`.

## Functional Requirements

- Once the launch workspace context exists, the shadow-HOME pipeline must reuse
  its already-selected launch-child root instead of adding another repo-root
  detection pass.
- `build_repo_home_env` must accept an optional already-resolved effective root
  for repo-scoped shadow-HOME resources and use it instead of recomputing via
  `resolve_repo_root`.
- At the `build_child_env_with_launch` call site, the supplied effective root is
  `launch_ctx.child_cwd.as_path()`. This preserves current behavior because
  `child_cwd` is the launch repo root when one was found and the launch CWD
  fallback otherwise.
- At the MCP late-materialization call site, the supplied effective root is
  `env_plan.child_cwd.as_path()` for the same reason.
- When the caller supplies an effective root, `build_repo_home_env` must not
  invoke `sniff::filesystem::detect_git` directly or indirectly through
  `resolve_repo_root`.
- When the caller supplies no effective root, `build_repo_home_env` must fall
  back to the existing `resolve_repo_root(cwd)` behavior. This fallback remains
  available for legacy/test callers and for any future caller that genuinely has
  only a CWD.
- The Codex branch of `needs_shadow_home` must accept the same optional
  effective root and reuse it rather than calling `resolve_repo_root`
  independently.
- `needs_shadow_home` and `build_repo_home_env` must agree on the root used for
  Codex repo-local prompt detection/materialization during a single launch.
- Any comments or rustdoc around `RepoHomeTimings`, `build_repo_home_env`,
  `needs_shadow_home`, and `LaunchWorkspaceContext` usage must be updated so they
  describe reuse of a known root instead of claiming the hot path always performs
  a sniff git walk.

## Design Decision

Use an additive parameter, not a new cache:

```rust
pub fn needs_shadow_home(
    provider: Provider,
    cwd: &Path,
    repo_only: bool,
    effective_root: Option<&Path>,
) -> bool

pub fn build_repo_home_env(
    provider: Provider,
    cwd: &Path,
    repo_only: bool,
    perf: bool,
    effective_root: Option<&Path>,
) -> Result<(..., Option<RepoHomeTimings>)>
```

Inside each function, resolve the root with:

```rust
let repo_root = effective_root
    .map(Path::to_path_buf)
    .unwrap_or_else(|| resolve_repo_root(cwd));
```

This is intentionally simple and local. It removes the duplicate hot-path walk
without adding global memoization, without changing `resolve_repo_root`, and
without changing the source-repo metadata contract used by composition,
guardrails, MCP defaults, and harness path resolution.

## Behavior Preservation

- Shadow-HOME isolation behavior is unchanged: the same HOME override, symlink
  vs. copy decisions, and volatile-state-file skipping continue to apply.
- Codex repo-local prompt overlay behavior is unchanged: repo-scoped prompts and
  commands continue to resolve from the launch-child root that
  `resolve_repo_root(child_cwd)` would have produced before this feature.
- Composition flows that use a source document from another repo keep the
  existing split: metadata-sensitive subsystems use the source repo root, while
  shadow-HOME repo-scoped resources follow the launch-child root.
- The `--perf` tree shape is unchanged; only the measured value of
  `child env build -> shadow home sync -> repo root detect` is expected to drop
  to zero or microsecond scale when the root is reused.
- Off-repo launches (no resolvable repo root) behave exactly as before.

## Non-Requirements

- No change to the shadow-HOME directory walk, symlink strategy, or
  `is_volatile_state_file` handling.
- No caching layer or memoization inside `resolve_repo_root` itself is required;
  threading the known value through the call chain is sufficient.
- No broad rewrite of context capture or the sniff scan that originally resolves
  the launch repo root.
- No change to the perf instrumentation, metric names, or reconciliation model.
- No new CLI flags or environment variables.

## Acceptance Criteria

- A `--repo` `compose` (or `inline-compose` / `sequence`) launch performs no
  additional repo-root detection inside the shadow-HOME pipeline after the launch
  workspace context exists; `child env build -> repo root detect` reports a
  microsecond-scale value rather than the previous ~660ms–2s.
- The non-Codex `--repo` path performs zero `detect_git` walks inside
  `build_repo_home_env`.
- A Codex non-`--repo` launch performs at most one repo-root resolution across
  `needs_shadow_home` and `build_repo_home_env`.
- MCP late shadow-HOME materialization does not re-run repo-root detection when
  an `EnvPlan` already exists.
- Off-repo launches (no resolvable repo root) produce the same HOME, shadow-HOME,
  and prompt-resolution outcomes as before the change.
- Existing `env`, `repo_home`, and composition tests pass after updating call
  sites for the new parameter.
- Tests cover both branches of the new API: supplied effective root avoids
  fallback resolution, and `None` preserves the `resolve_repo_root(cwd)`
  fallback.
- A regression test covers the source-repo-vs-launch-repo split: when a composed
  source document lives outside the launch repo, shadow-HOME Codex prompt
  materialization still uses the launch-child root, not the source metadata root.
- A before/after `--perf --dry-run --repo` run shows `child env build` collapse
  while every other node in the tree is unchanged.

## Affected Code

- `claudine/cli/src/commands/wrap/repo_home.rs` — `build_repo_home_env`
  signature and the `resolve_repo_root` call site; `needs_shadow_home` Codex
  branch; comments for `RepoHomeTimings`.
- `claudine/cli/src/commands/wrap/env.rs` — `build_child_env_with_launch`
  call sites for `needs_shadow_home` and `build_repo_home_env`, passing
  `launch_ctx.child_cwd.as_path()` as the effective root.
- `claudine/cli/src/commands/wrap/composition/mod.rs` — MCP late
  shadow-HOME materialization call site, passing `env_plan.child_cwd.as_path()`
  as the effective root.
- `claudine/lib/src/linking/paths.rs` — `resolve_repo_root` remains the
  off-repo fallback only (no behavior change).

## Open Questions

None. The design decision above keeps the change local, preserves the existing
composition root split, and avoids a cache whose invalidation rules would be
larger than the problem.
