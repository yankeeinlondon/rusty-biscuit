---
ready: false
agent: codex
model: ""
---

# Review 3

## Findings

### High: Nested Cargo workspace roots are never discovered under non-Cargo parents

The new nested marker walk is intended to make root-manifest standards a real
forest, but `NESTED_MARKERS` does not include Cargo's marker:

- `sniff/lib/src/filesystem/repo/nested.rs:65`
- `sniff/lib/src/filesystem/repo/nested.rs:116`
- `sniff/lib/src/filesystem/repo/detection.rs:290`
- `sniff/lib/src/filesystem/repo/standard.rs:645`
- `sniff/lib/src/filesystem/repo/standard.rs:680`

`CargoWorkspace` has a strong `Cargo.toml` marker and a
`NestingPolicy::ForbidsNested` descriptor. The implementation correctly models
that `ForbidsNested` blocks same-standard nesting only: the comment in
`discover_nested_workspace_outcomes` explicitly says a nested Cargo workspace
under a root Cargo workspace is invalid, while a nested workspace under a
different standard can be valid. However, because `Cargo.toml` is absent from
the candidate marker table, `dispatch_detector_at(... CargoWorkspace ...)` is
dead for nested discovery. A repo such as a pnpm workspace containing
`crates/Cargo.toml` with `[workspace].members` will report only the pnpm layer;
the nested Cargo standard, layer packages, matched marker, and binary metadata
are silently missing. A root with no marker and only a nested Cargo workspace is
also missed entirely.

This leaves the topology forest incomplete for one of the core membership
authorities, contrary to the spec's forest model and the descriptor table's
single-source-of-truth intent.

Suggested fix: add `Cargo.toml -> CargoWorkspace` to the nested marker mapping
and let the existing same-standard `ForbidsNested` check suppress only nested
Cargo under an ancestor Cargo workspace. Add Level 1 fixture coverage for:

- pnpm root with a nested Cargo workspace, expecting two layers.
- bare root with only a nested Cargo workspace, expecting the nested Cargo layer.
- existing Cargo-under-Cargo forbidden behavior, to keep the guard intact.

## Test Rigor

This feature is filesystem detection plus CLI text/JSON reporting. It does not
define terminal encoder/decoder behavior, key input, paste, mouse, or styling
requirements that need Level 2 or Level 3 verification. Level 1 unit,
integration, and snapshot tests are the appropriate floor.

Current Level 1 coverage is strong for the issues raised in review 1 and review
2: no-subprocess binary resolution, virtual Cargo single-member degeneracy,
nested pnpm/uv discovery, nested-only full detection, per-root confidence, and
lockfile superset drift. The remaining gap above is also a Level 1 filesystem
case: the candidate marker table and detector dispatch need fixture coverage for
nested Cargo under a different standard.

## Verification

Ran:

```bash
cargo test -p sniff --test integration -- --nocapture test_nested_pnpm_under_cargo_is_discovered_as_its_own_layer test_nested_uv_under_pnpm_is_discovered_as_its_own_layer test_nested_only_workspace_full_detection_does_not_panic test_per_root_confidence_in_same_standard_forest
```

Result: passed, 4 tests.

## Production Readiness

Not ready. The implementation is much closer than the previous iteration, but
the topology forest still omits nested Cargo workspaces whenever the ancestor is
not also Cargo. That is a designed membership authority and should be fixed
before production.
