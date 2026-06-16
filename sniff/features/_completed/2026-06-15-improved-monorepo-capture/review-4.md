---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### High: Nested root-manifest packages leak layer-relative paths into top-level `RepoInfo.packages`

Nested root-manifest detectors are dispatched at the nested workspace root, and their `Package` values are folded directly into the top-level repo package list:

- `sniff/lib/src/filesystem/repo/nested.rs:320`
- `sniff/lib/src/filesystem/repo/nested.rs:435`
- `sniff/lib/src/filesystem/repo/detection.rs:1337`
- `sniff/lib/src/filesystem/repo/types.rs:124`

`create_package` computes `Package.relative` from the detector root it is given. For a repo rooted at `/repo` with a nested pnpm workspace at `/repo/web`, the nested pnpm detector creates packages such as `packages/app`, not `web/packages/app`. That is correct for the `MonorepoLayer` contract, but the same `Package` is then pushed into the top-level `RepoInfo.packages`, where `Package.relative` is documented and consumed as repo-root-relative.

This breaks user-facing package output and filters for nested workspaces. `sniff repo structure --json`, `sniff repo packages`, package-area grouping, dirty/staged package matching, and `--package-area web` style filtering all rely on `repo.packages[*].relative` being relative to the repo root. A nested workspace package can appear under the wrong area (`root` / `packages`) or fail to match repo-root-relative changed paths.

Suggested fix: keep two path frames explicit. Preserve layer-root-relative paths for `MonorepoLayer.packages`, but rebase cloned packages before adding them to the top-level flat `packages` list, including `relative`, `package_area`, and any package-local file paths that are expected to be repo-root-relative. Add Level 1 fixture coverage that detects a nested pnpm/Cargo workspace and asserts `repo.packages[*].relative` includes the nested root prefix while `monorepo_layers[*].packages[*].relative` remains layer-relative.

### High: Nested Bazel layer packages are repo-root-relative, violating the layer contract

The leaf-marker detector has the opposite path-frame problem:

- `sniff/lib/src/filesystem/repo/polyglot.rs:103`
- `sniff/lib/src/filesystem/repo/polyglot.rs:144`
- `sniff/lib/src/filesystem/repo/topology.rs:72`
- `sniff/lib/src/filesystem/repo/topology.rs:77`

`walk_leaf_workspaces` intentionally calls `create_package(&dir, root, ...)` with the outer repo root so flat package reporting stays repo-root-relative. But `build_monorepo_layers` then copies `pkg.relative` directly into `LayerPackage.relative`. For a nested Bazel workspace rooted at `/repo/nested`, the layer package for `/repo/nested/BUILD` becomes `nested`; a package at `/repo/nested/sub/BUILD` becomes `nested/sub`. The spec says `MonorepoLayer.packages` are relative to the layer root, so those should be `.` / empty-root semantics and `sub`, respectively.

The current tests lock in the wrong behavior: `test_bazel_workspace_segments_nested_workspace_into_its_own_layer` expects the nested layer to contain `nested`, and the polyglot unit test expects `nested` plus `nested/sub`. That makes consumers unable to treat layer package paths uniformly across pnpm/Cargo/uv versus Bazel forests.

Suggested fix: have `DetectorOutcome` carry package paths in a single well-defined frame plus enough root context to derive the other frame, or rebase inside `build_monorepo_layers` from `Package.path` against `outcome.root`. Update the Bazel tests to assert nested-layer-relative paths and keep a separate assertion that top-level `RepoInfo.packages` remains repo-root-relative.

## Test Rigor

This feature is filesystem detection plus CLI text/JSON reporting. It does not define terminal input, real-terminal rendering, paste, mouse, or styling behavior that requires Level 2 or Level 3 verification. Level 1 unit, integration, and CLI snapshot tests are the appropriate floor.

The remaining gaps are Level 1 contract gaps: nested workspace package paths must be asserted in both public frames, `RepoInfo.packages` and `MonorepoLayer.packages`. Current tests cover discovery but do not verify the top-level nested package paths, and the Bazel tests assert the wrong layer-relative value.

## Verification

Ran:

```bash
cargo test -p sniff --test integration -- --nocapture test_nested_pnpm_under_cargo_is_discovered_as_its_own_layer test_nested_cargo_under_pnpm_is_discovered_as_its_own_layer test_bazel_workspace_segments_nested_workspace_into_its_own_layer
```

Result: passed, 3 tests.

## Production Readiness

Not ready. The topology forest is now discovered, but nested package path frames are inconsistent across APIs and leak into user-facing CLI/JSON behavior. That should be fixed before production.
