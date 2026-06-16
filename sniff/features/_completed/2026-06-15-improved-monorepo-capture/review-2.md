---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: Full repo detection can panic for a nested-only workspace

`detect_repo_inner_with_shared` skips building a `ManifestIndex` when the supplied root has no workspace marker (`!has_workspace_marker(root)`), but the new nested marker walk can still discover workspace outcomes below that root. Once any nested outcome exists, full detection reaches:

- `sniff/lib/src/filesystem/repo/detection.rs:176`
- `sniff/lib/src/filesystem/repo/detection.rs:303`
- `sniff/lib/src/filesystem/repo/detection.rs:339`

At `detection.rs:343`, full mode calls:

```rust
let index = manifest_index.expect("full repo detection requires a manifest index");
```

That assumption is no longer valid. A directory with no root marker but with `web/pnpm-workspace.yaml` below it now produces outcomes from `discover_nested_workspace_outcomes`, then panics in `detect_repo` / full filesystem detection because `manifest_index` is `None`.

This is a production blocker because the spec explicitly models topology as a forest, and nested workspace roots are now part of the detection surface. The fix should either build the manifest index after nested outcomes are discovered when full mode needs it, or avoid the full nested package scan when there is no index. Add an L1 regression test that calls full `detect_repo`, not only `detect_repo_structure`, on a root whose only workspace is nested.

### Medium: Detection confidence is computed per standard, not per root/outcome

`build_detected_standards` maps each `DetectorOutcome` to a `DetectedStandard`, but `standard_confidence` only checks whether any layer with the same standard resolves non-degenerately:

- `sniff/lib/src/filesystem/repo/topology.rs:146`
- `sniff/lib/src/filesystem/repo/topology.rs:176`

This means a degenerate `pnpm-workspaces` layer can be marked `marker-confirmed` if a separate `pnpm-workspaces` layer elsewhere in the same forest is non-degenerate. The spec defines confidence for the detected standard/root: `MarkerConfirmed` means a strong marker matched and membership resolved non-degenerately. The current implementation can overstate confidence in multi-root topologies with the same standard.

Pass the outcome root into the confidence calculation and match both `authority` and `root` when deciding whether that specific detected standard is confirmed. Add an L1 fixture with two same-standard roots, one non-degenerate and one degenerate, and assert the degenerate `DetectedStandard` remains `inferred`.

## Test Rigor

The requirements in this feature are filesystem detection, JSON shape, CLI text snapshots, and no-subprocess binary resolution. L1 unit/integration/snapshot tests are the appropriate verification level; I did not identify any requirement that needs L2 real-terminal capture or L3 OS keyboard injection.

Existing coverage is broad for descriptor metadata, positive/degenerate fixtures, kebab-case JSON, CLI snapshots, root package degeneracy, lockfile parity, and no-subprocess binary resolution. The missing L1 coverage is specifically for full detection after nested-only discovery and per-root confidence in same-standard forests.

## Verification

Ran:

```bash
cargo test -p sniff --test integration -- --nocapture test_nested_pnpm_under_cargo_is_discovered_as_its_own_layer test_virtual_cargo_single_member_is_not_a_monorepo test_uv_lockfile_parity_upgrades_provenance
```

Result: passed, 3 tests.

## Production Readiness

Not ready. The nested-only full-detection panic is a user-facing correctness/stability issue and should be fixed before production.
