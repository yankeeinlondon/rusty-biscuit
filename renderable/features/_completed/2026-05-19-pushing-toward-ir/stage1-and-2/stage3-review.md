# Stage 3 Spec Review

> Status: historical review of the first Stage 3 spec draft. The current
> `stage3-spec.md` has absorbed these findings: the baseline table now calls
> out `BlockQuote`, `StatusBlock`, and `FileSystem` as missing
> `render_tree_node` overrides; `FileSystem::render` is an explicit decision;
> retained `render_bespoke` hooks stay `#[doc(hidden)] pub` for integration
> tests; and the Compose coverage is expressed as a fixture table rather than
> "all twelve components as inner parts."

## Findings

### 1. The S3-1 baseline overstates the current Stage 2 completion state [`HIGH`]

The spec says Stage 2 flipped all twelve components to canonical render-tree terminal rendering and that S3-1 is just a mechanical `render_tree_node` override for all twelve (`stage3-spec.md:5-8`, `stage3-spec.md:67-102`). The current source does not match that baseline:

- `StatusBlock` implements `TreeRenderable` but does not override `TerminalRenderable::render_tree_node`, so nested `StatusBlock` still falls through to plain-text projection.
- `FileSystem` implements `TreeRenderable` but does not override `render_tree_node`, and its `TerminalRenderable::render` path still uses the bespoke renderer. Its own docs say the bespoke terminal path remains production until parity proves the tree renderer (`biscuit-terminal/lib/src/components/filesystem/mod.rs:1505`, `biscuit-terminal/lib/src/components/filesystem/mod.rs:1610`, `biscuit-terminal/lib/src/components/filesystem/mod.rs:2792`).
- Most other components already have the override, so “apply to all twelve” will create churn in files that only need verification.

**Suggestion:** Split S3-1 into:

1. Add missing overrides for `StatusBlock` and `FileSystem`.
2. Audit the other ten with a no-op checklist.
3. Decide explicitly whether `FileSystem::render` must flip to `render_via_tree` in Stage 3 or remains an exception with a named acceptance criterion.

### 2. Retained `render_bespoke()` hooks cannot be both `pub(crate)` and tested from `biscuit-terminal/lib/tests` [`HIGH`]

S3-4 says the four escape-hatch components should demote `render_bespoke` to `pub(crate) + #[cfg(test)]` while keeping their `*_parity.rs` files (`stage3-spec.md:192-199`). Those parity files live under `biscuit-terminal/lib/tests`, which are integration tests compiled as an external crate. They cannot call `pub(crate)` methods from `biscuit_terminal`.

This will break the exact tests the spec says to keep.

**Suggestion:** Pick one access pattern:

- Move the retained escape-hatch parity tests into unit-test modules inside the component source files, where `pub(crate)` / private helpers are accessible.
- Or keep a public test-only surface under a feature or `#[cfg(any(test, feature = "test-support"))]` module designed for integration tests.
- Or keep `#[doc(hidden)] pub` for the four sanctioned escape hatches and document that they remain public only because integration parity tests need them.

### 3. S3-3 requires a concrete type name, but the current fallback only has a `Debug`-derived label [`MEDIUM`]

The spec requires the warning to include the component `type_name` (`stage3-spec.md:144-150`). The current fallback builds a label from `format!("{:?}", component)` and takes the first whitespace-delimited token (`biscuit-terminal/lib/src/render_tree/projection.rs:326-335`). That is not a stable Rust type name and can be unhelpful or misleading for custom `Debug` implementations.

**Suggestion:** Add an object-safe type-name helper to `TerminalRenderable`, then use it in both the diagnostic and `tracing::warn!`. For example, add a default method that returns a stable `&'static str` for the concrete implementer, then have `RenderableTerminalContent::to_tree_nodes` use that instead of parsing `Debug`.

Also specify whether the warning is emitted once per type. The risks table mentions `OnceLock` or a feature flag (`stage3-spec.md:348`), but S3-3’s task list does not require either. If log volume matters, make “warn once per concrete type per process” part of the acceptance criteria.

### 4. S3-4’s “retire wholesale” table assumes every listed component has a public `render_bespoke()` [`MEDIUM`]

The spec says every migrated component carries `#[doc(hidden)] pub fn render_bespoke()` (`stage3-spec.md:159-164`), then assigns `Compose` and `FileSystem` to the wholesale-retire tier (`stage3-spec.md:174-175`). Current code and prior reviews indicate those two do not follow that pattern consistently. `FileSystem` has a bespoke production path but not a clean `render_bespoke` parity hook; `Compose` has already collapsed around the tree path.

**Suggestion:** Change S3-4 from a uniform removal step to a per-component inventory:

- “Remove existing public hidden hook” for components that actually expose one.
- “No action; already retired” for components without one.
- “Extract or leave bespoke implementation?” for `FileSystem`, depending on the Stage 3 decision in finding 1.

This avoids implementation churn where there is no public surface to retire.

### 5. The layout-matrix `via_render` vs `via_tree_direct` contract needs explicit escape-hatch handling [`MEDIUM`]

S3-6 says the matrix should compare `component.render(&term)` against directly rendering `TreeRenderable::render_tree(component)` (`stage3-spec.md:244-263`). That is sound for fully tree-backed components, but it is not automatically sound for sanctioned terminal-only escape hatches:

- `BlockQuote::with_border(arbitrary)`
- `StatusBlock::border(arbitrary)`
- `Table::prefer_cursor_alignment`
- `TwoColumn` image overlay
- potentially `FileSystem` if its terminal render remains bespoke

For these cases, `via_render` may intentionally choose a bespoke path that `via_tree_direct` cannot represent. Treating any drift as a component regression (`stage3-spec.md:350`) would be wrong for cases the same spec declares out of scope for the tree (`stage3-spec.md:337-339`).

**Suggestion:** Add a matrix policy:

- Default component cases should avoid escape-hatch knobs and must match.
- Escape-hatch cases, if included, should be excluded from `via_tree_direct` parity or tracked in a separate “terminal-only behavior” suite.
- Any expected drift from sanctioned escape hatches must be recorded separately from ordinary render-tree regressions.

### 6. The test expansion says “all twelve as inner parts,” but not all components are practical inline/container children [`LOW`]

S3-2 asks `compose.rs` to broaden nested-component coverage to all twelve components as inner parts (`stage3-spec.md:125-126`). Some components require setup or external state (`FileSystem` needs a built tree), and some produce root/block-level structures that may not be semantically valid as inline `Compose` parts.

**Suggestion:** Replace “all twelve” with a table of valid nested-component fixtures. For each component, name the minimal deterministic fixture and the expected `NodeKind`. If any component is intentionally skipped as an inner `Compose` part, document why and cover it through a block-container test instead.

## Summary

The Stage 3 direction is good: per-component structural projection is the right way to close the nested-component flattening gap. The main fixes needed before implementation are to make the baseline accurate, decide the `FileSystem` terminal path explicitly, and reconcile retained parity hooks with integration-test visibility.
