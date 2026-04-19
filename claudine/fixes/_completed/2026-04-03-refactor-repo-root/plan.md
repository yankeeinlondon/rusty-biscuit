# Repo Root Launch Refactor Plan

## Context

Claudine wrappers are supposed to preserve two distinct pieces of launch context:

1. the user's original launch location inside the monorepo, which determines `PACKAGE` and `PACKAGE_AREA` for logging/context
2. the child process working directory, which must be the monorepo root so the wrapped agent's default workspace and permission model are anchored correctly

Those concerns have drifted together in the wrapper environment pipeline. Today the wrapper often does the right thing, but the logic is fragile because:

- repo-root resolution is folded into the same path that derives monorepo package metadata
- child working directory is recomputed in multiple entrypoints instead of being a first-class resolved output
- package-context failures or future refactors can silently affect whether the child launches at repo root
- direct-wrap and composition paths each have slightly different repo-root handling rules

The net effect is a recurring regression pattern: fixing launch-at-root in one path does not make it structurally hard to break again later.

This plan fixes the current issue and refactors the wrapper so repo-root launch behavior becomes explicit, centralized, and test-locked.

## Problem Statement

### Required invariant

When Claudine is launched from a nested monorepo package directory:

- `PACKAGE_AREA` and `PACKAGE` must reflect the original launch directory
- the wrapped child process must start in the monorepo root

This must hold for:

- direct wrapper runs such as `claudine opencode ...`
- composition runs such as `claudine compose --opencode ...`
- harness retries/resumes that relaunch the provider

### Current design weakness

In `claudine/cli/src/commands/wrap/env.rs`, `build_child_env()` currently does all of the following together:

- sanitize and build the child environment
- derive package context from the caller cwd
- derive repo root from either a hint or monorepo-context resolution
- indirectly influence the child working directory chosen later

Then downstream callers independently compute child cwd:

- direct wrapper: `env_plan.repo_root.as_deref().unwrap_or(&cwd)`
- composition path: `effective_repo_root.unwrap_or(&cwd)`

That means "where the child starts" is not a single resolved fact. It is an emergent property of several loosely related values and fallback rules.

## Goals

1. Make the child launch directory an explicit resolved value, not a downstream reconstruction.
2. Decouple repo-root detection from package-context detection.
3. Preserve original package/package-area context even when the child launches from repo root.
4. Use the same launch-root resolution model in direct-wrap and composition paths.
5. Add integration tests that fail if a wrapped provider starts anywhere other than monorepo root.

## Non-Goals

- Redesigning all environment detection in `claudine::events`
- Changing how `PACKAGE` / `PACKAGE_AREA` are consumed elsewhere
- Refactoring provider prompt delivery further in this change
- Reworking shadow-HOME behavior beyond making it follow the resolved launch root consistently

## Root Cause Analysis

### Current behavior split

`build_child_env()` in `claudine/cli/src/commands/wrap/env.rs` documents the intended distinction:

- package context should come from the caller cwd
- repo root should come from either a hint or repo detection

That intent is correct, but the implementation still makes repo-root selection part of the same function and same state object as package-context selection. This creates two risks:

1. a future edit can accidentally make repo-root dependent on monorepo detection succeeding
2. downstream callers can re-derive or override launch cwd differently

### Why this keeps regressing

The code currently has no single type whose job is:

- "this is where the user launched Claudine from"
- "this is the repo root the child must run from"
- "this is the monorepo package context derived from the launch cwd"

Instead these ideas are spread across:

- `cwd` locals in wrapper/compose entrypoints
- `repo_root_hint`
- `env_plan.repo_root`
- composition-specific `source_repo_root`
- local `child_cwd` calculations

That fragmentation is the design bug.

## Refactor Strategy

Introduce an explicit wrapper launch-context model that resolves once and is then consumed everywhere else.

### New central concept

Add a dedicated internal type in `claudine/cli/src/commands/wrap/env.rs`:

```rust
pub(crate) struct LaunchWorkspaceContext {
    pub(crate) launch_cwd: PathBuf,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) child_cwd: PathBuf,
    pub(crate) package_context: Option<PackageContext>,
    pub(crate) warnings: Vec<String>,
}
```

This becomes the authoritative answer to:

- where Claudine was launched
- which repo root was resolved
- where the child will start
- which package/package-area the user was in

### Resolution rules

`resolve_launch_workspace_context(launch_cwd, repo_root_hint)` should:

1. resolve repo root independently:
   - use `repo_root_hint` when provided
   - otherwise detect git root via Sniff
   - otherwise detect repo root via Sniff repo structure
   - otherwise fall back to `launch_cwd`
2. derive `child_cwd` from resolved repo root or fallback cwd
3. derive package context from the original launch cwd only
4. never let package-context failure change the resolved child cwd

This is the key invariant:

> package-context resolution may fail and emit warnings, but child launch root must remain stable.

## Detailed Implementation Plan

## Phase 1: Introduce Explicit Launch Workspace Resolution

**Goal:** Resolve launch cwd, repo root, child cwd, and package context in one dedicated step.

### File

`claudine/cli/src/commands/wrap/env.rs`

### Changes

1. Add:

```rust
pub(crate) struct LaunchWorkspaceContext {
    pub(crate) launch_cwd: PathBuf,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) child_cwd: PathBuf,
    pub(crate) package_context: Option<PackageContext>,
    pub(crate) warnings: Vec<String>,
}
```

2. Add a small helper dedicated to repo-root detection:

```rust
fn detect_repo_root(cwd: &Path) -> Option<PathBuf>
```

Implementation order:

- `sniff::filesystem::git::detect_git(cwd, false, 1)`
- fallback to `sniff::filesystem::repo::detect_repo(cwd)`

3. Add:

```rust
fn resolve_launch_workspace_context(
    launch_cwd: &Path,
    repo_root_hint: Option<&Path>,
) -> LaunchWorkspaceContext
```

Rules:

- `repo_root = repo_root_hint.or_else(detect_repo_root)`
- `child_cwd = repo_root.unwrap_or(launch_cwd)`
- `package_context` still comes from `resolve_monorepo_package_context(launch_cwd)`
- warnings from package-context resolution are preserved
- package-context resolution must not mutate `repo_root` or `child_cwd`

4. Update `EnvPlan`:

```rust
pub(crate) struct EnvPlan {
    ...
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) child_cwd: PathBuf,
    ...
}
```

5. Update `build_child_env()` to:

- call `resolve_launch_workspace_context(...)` first
- use `launch_ctx.child_cwd` for shadow-HOME/repo-root-dependent setup
- populate `PACKAGE_AREA` / `PACKAGE` from `launch_ctx.package_context`
- populate `repo_root` and `child_cwd` directly from `launch_ctx`

### Important design constraint

Do not keep recomputing repo root inside `build_child_env()`. Once `LaunchWorkspaceContext` exists, `build_child_env()` should consume it or reconstruct it in one call, not merge partial repo-root state later.

## Phase 2: Make Callers Use `EnvPlan.child_cwd`

**Goal:** Remove duplicated child-cwd reconstruction from wrapper callers.

### Files

- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/composition.rs`

### Changes

1. Direct wrapper path:

Replace:

```rust
let child_cwd = env_plan.repo_root.as_deref().unwrap_or(&cwd);
```

with:

```rust
let child_cwd = env_plan.child_cwd.as_path();
```

2. Composition path:

Replace local fallback logic that recomputes child cwd from `effective_repo_root` and `cwd`.

Use `env_plan.child_cwd.as_path()` as the only child launch directory.

3. Keep `effective_repo_root` for:

- config loading
- environment detection
- harness context
- policy probes

But do not use it to recompute launch cwd anymore.

### Rationale

The launch directory for the child should be a resolved output from one place, not something every caller recomputes from related data.

## Phase 3: Align Shadow-HOME and Related Repo-Scoped Features

**Goal:** Ensure all repo-scoped setup follows the resolved launch root.

### Files

- `claudine/cli/src/commands/wrap/env.rs`
- `claudine/cli/src/commands/wrap/repo_home.rs`
- any direct callsites using repo-root hints for shadow-home setup

### Changes

Use resolved child cwd / repo root consistently for:

- `repo_home::needs_shadow_home(...)`
- `repo_home::build_repo_home_env(...)`
- MCP runtime shadow-home setup

### Constraint

Package context must continue to come from the original launch cwd, not from `child_cwd`.

## Phase 4: Add Regression Tests for Launch-at-Root

**Goal:** Make the bug hard to reintroduce.

### Primary test location

`claudine/cli/tests/wrap_commands.rs`

### New tests

#### 1. Direct wrapper: OpenCode launches at monorepo root

Create a temp monorepo like:

```txt
repo/
  .git/
  Cargo.toml   # workspace
  claudine/cli/Cargo.toml
  claudine/lib/Cargo.toml
  bin/opencode
```

Run from `repo/claudine/cli`:

```bash
claudine opencode summarize
```

Fake `opencode` script should write:

- `pwd` to a file
- `PACKAGE`
- `PACKAGE_AREA`
- args

Assert:

- child `pwd == repo root`
- `PACKAGE=claudine-cli`
- `PACKAGE_AREA=claudine`

#### 2. Compose path: OpenCode launches at source repo root

Run:

```bash
claudine compose --opencode <prompt-file>
```

from a nested package dir inside the same temp monorepo.

Assert:

- child `pwd == repo root`
- prompt still arrives correctly

#### 3. Package-context failure does not break repo-root launch

Add a unit test around `resolve_launch_workspace_context()` or `build_child_env()` where:

- repo root is detectable
- package-context resolution fails or yields no package match

Assert:

- `child_cwd` still resolves to repo root
- warnings are emitted
- package context may be `None`

### Optional additional test

Codex or Kimi direct wrapper launched from nested package dir also starts at repo root. This broadens confidence that the fix is architectural, not OpenCode-only.

## Phase 5: Tighten Internal Types and Naming

**Goal:** Make future misuse more obvious.

### Changes

1. Rename local `cwd` variables in wrapper entrypoints where appropriate:

- `launch_cwd` for original process cwd
- `child_cwd` only for the resolved child working dir

2. Add rustdoc/comments on `EnvPlan.child_cwd`:

- must be the actual process cwd for the child
- must not be recomputed from `repo_root` by callers

3. Add a code comment near `PACKAGE` / `PACKAGE_AREA` setup:

- they are derived from original launch cwd, not child cwd

## Acceptance Criteria

The fix is complete when all of the following are true:

1. Running a wrapped agent from a nested monorepo package starts the child at the monorepo root.
2. `PACKAGE` and `PACKAGE_AREA` still reflect the original nested launch location.
3. Direct-wrap and composition paths both use the same resolved child cwd source.
4. Package-context failures cannot silently cause child launch cwd to fall back to the nested package dir.
5. OpenCode regression tests prove the current issue is fixed.

## Suggested Commit Sequence

### Commit 1

`refactor(claudine): centralize wrapper launch workspace resolution`

- add `LaunchWorkspaceContext`
- add `EnvPlan.child_cwd`
- decouple repo-root detection from package-context resolution

### Commit 2

`fix(claudine): launch wrapped agents from monorepo root`

- switch wrapper/composition callers to `env_plan.child_cwd`
- align shadow-home/repo-root-dependent setup

### Commit 3

`test(claudine): lock wrapper launch cwd to repo root`

- add OpenCode integration regression tests
- add package-context failure coverage

## Verification Commands

Use the repo’s `just` conventions where applicable. Suggested checks:

```bash
just test
```

Or for faster focused work during implementation:

```bash
cargo test -p claudine-cli opencode_non_interactive_injects_default_model -- --nocapture
cargo test -p claudine-cli compose_opencode_non_interactive_passes_prompt_as_positional_arg -- --nocapture
```

And new tests from this plan once implemented:

```bash
cargo test -p claudine-cli opencode_launches_child_from_repo_root -- --nocapture
cargo test -p claudine-cli compose_opencode_launches_child_from_repo_root -- --nocapture
```

## Open Questions

1. Should `build_child_env()` keep taking raw `cwd` + `repo_root_hint`, or should callers resolve and pass a `LaunchWorkspaceContext` directly?

Recommended answer:
- eventually pass a pre-resolved context
- for the first implementation, resolving inside `build_child_env()` is acceptable if it remains centralized and single-purpose

2. Should `EnvPlan.repo_root` remain optional if `child_cwd` is always populated?

Recommended answer:
- yes
- `repo_root` still communicates whether a real repo root was detected
- `child_cwd` communicates where the child actually launches

3. Should the same launch-context abstraction also be reused by `claudine::system_prompt::LaunchContext`?

Recommended answer:
- not in this fix
- the concepts are related but serve different consumers
- keep this refactor scoped to wrapper launch correctness first
