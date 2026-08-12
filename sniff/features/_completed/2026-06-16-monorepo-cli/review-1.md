---
ready: false
agent: codex
model: ""
---

# Review: Monorepo CLI Wiring Cleanup

## Findings

### High: CLI label rendering drops all but the first orchestrator

- Location: `sniff/cli/src/output/filesystem/repo.rs:147`
- Requirement: D5 says the focused `repo is-monorepo` leaf and the shared CLI helper derive the label from the primary authority and its orchestrators. The JSON contract explicitly preserves all orchestrators as an array, and the topology model allows multiple orchestrators on one layer.
- Issue: `format_monorepo_label` calls `orchestrators.first()` and ignores the rest:

```rust
if let Some(&orchestrator) = orchestrators.first() {
    format!("{} (using {})", orchestrator.spec().label, authority_label)
}
```

For a layer with `orchestrators = [Nx, Lerna]`, text surfaces render only `Nx (using pnpm workspaces)`, while JSON correctly emits both `["nx", "lerna"]`. That affects `sniff repo is-monorepo`, the single-layer repo summary, and the multi-layer listing because they all use this helper.

Recommended fix: make the helper join every orchestrator label in a deterministic order already carried by the layer, for example `Nx + Lerna (using pnpm workspaces)`, and add an L1 helper test covering zero, one, and two orchestrators. Add an integration/CLI test for the focused leaf or structure renderer if there is an existing Nx+Lerna fixture.

Verification level: this is plain text/JSON CLI behavior. L1 unit/integration coverage is appropriate; no L2/L3 terminal verification is needed because the requirement is not about terminal encoder behavior, styling fidelity, or keyboard input.

### Medium: `primary_layer()` fallback rule is under-tested

- Location: `sniff/lib/src/filesystem/repo/types.rs:182`
- Requirement: D1 documents a three-part selection rule: repo-root layer first, otherwise shallowest root, then enum-order tie-break.
- Issue: the tests cover no layers, one root layer, shared-root enum tie, and a root-vs-nested case. The test named `primary_layer_selects_shallowest_root` includes a repo-root layer, so it exercises rule 1, not the "otherwise shallowest root" fallback. There is also no direct test for enum-order tie-breaking when no layer is rooted at the repo root.

Recommended fix: add L1 unit tests with only nested layers, such as `/repo/apps` vs `/repo/tools/nested`, and a same-depth nested tie ordered opposite the expected enum order.

Verification level: L1 is appropriate; this is deterministic library selection logic.

## Coverage Notes

- D5 focused `repo is-monorepo` behavior has L1 integration coverage for monorepo text, non-monorepo text, `--no-error`, JSON success, JSON predicate failure with valid STDOUT, and genuine failure with STDERR.
- JSON aggregate compatibility has L1 coverage showing the aggregate keeps the legacy unwrapped `"is-monorepo"` bool.
- `MonorepoLayer.packages` is now `Vec<String>` and existing integration tests assert layer package entries resolve against the canonical package catalog.
- No L2/L3 tests are required for this feature as specified: there are no requirements about real terminal rendering fidelity, terminal input encoding, key handling, paste/IME, mouse, or scrolling.

## Production Readiness

Not ready for production. The multiple-orchestrator text path loses user-observable topology information that the spec says should be carried through the unified label helper. Fix that and add the missing L1 coverage before marking ready.
