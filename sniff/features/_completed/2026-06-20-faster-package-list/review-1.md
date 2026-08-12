---
ready: false
agent: codex/default
created: 2026-06-20T20:23:50
implemented: true
---

# Review 1

## Findings

### High: Gitignored-marker regression test does not exercise the intentional behavior change

The spec requires an L1 test proving that a gitignored nested marker is no longer detected. The added fixture ignores `nested/package.json` and writes `{"name":"ignored"}` with no `workspaces` field ([fixtures.rs:1006](../../lib/tests/fixtures.rs#L1006), [fixtures.rs:1008](../../lib/tests/fixtures.rs#L1008)). Even the old `Path::exists()` implementation would have registered `nested/` as a candidate and then produced no layer, because `detect_npm_workspace` returns `None` when `workspaces` is empty. The assertion in [integration.rs:641](../../lib/tests/integration.rs#L641) would therefore pass before and after the behavior change.

Use an ignored marker that would produce a layer under the old probe-based implementation, such as an ignored `pnpm-workspace.yaml` with a real member package, or an ignored `package.json` containing `workspaces` and a matching member. Then assert that the pnpm/npm layer is absent. Current strongest verification: Level 1 in-process, but it is non-discriminating for the requirement.

### Medium: Prune-guard test would pass if `node_modules` pruning stopped working

The spec requires a prune guard proving that `filter_entry` still prevents `node_modules/` from being scanned. The fixture places `app/node_modules/lodash/package.json` with no workspace declaration ([fixtures.rs:983](../../lib/tests/fixtures.rs#L983), [fixtures.rs:985](../../lib/tests/fixtures.rs#L985)), and the test only checks that no layer path contains `node_modules` ([integration.rs:614](../../lib/tests/integration.rs#L614)). If pruning regressed, the walker would register `app/node_modules/lodash` as a candidate, but the npm/yarn/bun detectors would return `None`; the test would still pass.

Make the pruned subtree contain a marker that would create a real layer if visited, for example `node_modules/workspace/pnpm-workspace.yaml` plus `packages/app/package.json`, and assert that no such layer appears. Current strongest verification: Level 1 in-process, but not sensitive to the specified prune behavior.

### Medium: Root-marker skip test does not prove root markers are excluded from nested candidates

The spec requires root-level markers to be ignored by nested discovery. The fixture uses a root `package.json` without workspaces ([fixtures.rs:961](../../lib/tests/fixtures.rs#L961)), and the test allows either `None` or a repo with no layers ([integration.rs:588](../../lib/tests/integration.rs#L588)). If the `parent == root` check regressed for root files, the nested dispatcher would still produce no layer for this fixture because the JS workspace detectors self-filter non-workspace package manifests.

This is best covered with a unit test inside `nested.rs` that directly asserts `walk_for_nested_markers(root)` returns no candidate for a root marker, or with a fixture whose root marker would produce a detectable nested outcome if it were incorrectly registered. Current strongest verification: Level 1 in-process, but not discriminating for the non-root-only nested discovery contract.

## Notes

The library implementation is otherwise appropriately scoped: `walk_for_nested_markers` now inspects walker entries in memory, keeps the existing ignore/prune settings, preserves sorted candidate output, and does not add CLI-side detection logic. I did not find a stdout/stderr contract change.

I ran focused L1 checks:

```text
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_nested_
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_root_marker_is_not_registered_as_nested_candidate
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_node_modules_package_json_is_pruned
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_gitignored_nested_marker_is_not_detected
cargo test --color=never --manifest-path sniff/lib/Cargo.toml marker_name_matches_is_exact_on_unix_and_case_insensitive_on_windows
```

All focused checks passed, but the three findings above mean required behavior is not yet strongly verified. Production readiness: **not ready**.
