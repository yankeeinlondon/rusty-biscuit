# Feature: Expose Magic Path Configuration Through ComposeOptions

## Problem

Darkmatter's compose pipeline delegates `@`-prefixed path resolution to `biscuit_file::FileReference`, which supports custom search roots via `add_magic_path(path, PathPosition)`. However, darkmatter never surfaces this capability — the `FileReference` is constructed bare (`FileReference::new(...)`) in every call site without forwarding any caller-configured paths.

This means consumers like **claudine** cannot customize where `@` references resolve. Claudine needs `@` references to search claudine-specific directories (e.g., `~/.claudine/`, a project's `.claudine/` folder) *before* falling back to the git root and HOME defaults.

## Scope

There are **four call sites** in darkmatter that construct `FileReference::new(...)` without custom magic paths:

| # | File | Function | Context |
|---|------|----------|---------|
| 1 | `compose/transclusion/resolver.rs:101` | `resolve_path()` | Compose transclusion (`::file`, `::code`) |
| 2 | `reference/graph.rs:762` | `resolve_local_target()` | Dependency graph building |
| 3 | `reference/validate.rs:313` | `validate_local_path()` | Reference validation |
| 4 | `reference/mod.rs:38` | `resolve_transclusion_target()` | Transclusion target resolution |

All four must be updated to thread through custom magic paths for consistent behavior.

## Design

### Approach: Add `magic_paths` field to `ComposeOptions`

The cleanest approach is to add a `Vec<(PathBuf, PathPosition)>` to `ComposeOptions` with a builder method, then thread it through to every `FileReference` construction. This follows the existing pattern where `ComposeOptions` already mirrors `FileReference` capabilities (e.g., `resolve_repo_root`).

### Why not re-export `biscuit_file::PathPosition`?

We **should** re-export `PathPosition` from darkmatter's compose module. Consumers already depend on `darkmatter` — requiring them to also add `biscuit_file` as a direct dependency just for an enum would be poor ergonomics. The re-export keeps the API self-contained.

### Data flow

```
ComposeOptions::new()
    .with_magic_path("/path", PathPosition::Start)
    │
    ├─► ComposeOptions.magic_paths: Vec<(PathBuf, PathPosition)>
    │
    ├─► transclusion_options() ──► TransclusionOptions.magic_paths
    │   └─► resolver.rs: resolve_path() ──► FileReference::new().add_magic_path(...)
    │
    ├─► graph.rs: resolve_local_target() ──► FileReference::new().add_magic_path(...)
    │   (receives magic_paths via new parameter or shared config)
    │
    ├─► validate.rs: validate_local_path() ──► FileReference::new().add_magic_path(...)
    │
    └─► reference/mod.rs: resolve_transclusion_target() ──► FileReference::new().add_magic_path(...)
```

## Implementation Plan

### Phase 1: Core plumbing in darkmatter types

**File: `darkmatter/lib/src/markdown/compose/types.rs`**

1. Add re-export of `biscuit_file::PathPosition` from the compose module
2. Add field to `ComposeOptions`:
   ```rust
   /// Custom search roots for `@`-prefixed (magic) file references.
   ///
   /// Each entry is a `(path, position)` pair where `position` controls
   /// whether the path is searched before (`Start`) or after (`End`) the
   /// default roots (git repo root, HOME).
   pub magic_paths: Vec<(PathBuf, PathPosition)>,
   ```
3. Add builder method to `ComposeOptions`:
   ```rust
   /// Adds a custom search root for `@`-prefixed file references.
   ///
   /// Paths added with `PathPosition::Start` are searched before the
   /// git repository root; paths with `PathPosition::End` are searched
   /// after HOME.
   #[must_use]
   pub fn with_magic_path(mut self, path: impl Into<PathBuf>, position: PathPosition) -> Self {
       self.magic_paths.push((path.into(), position));
       self
   }
   ```
4. Initialize `magic_paths: Vec::new()` in `ComposeOptions::new()`
5. Add `magic_paths` to the `Debug` impl
6. Add `magic_paths: Vec<(PathBuf, PathPosition)>` to `TransclusionOptions`
7. Thread `magic_paths` through `transclusion_options()`:
   ```rust
   pub(crate) fn transclusion_options(&self) -> TransclusionOptions {
       TransclusionOptions {
           // ...existing fields...
           magic_paths: self.magic_paths.clone(),
       }
   }
   ```

**File: `darkmatter/lib/src/markdown/compose/mod.rs`**

8. Add `PathPosition` to the public re-exports:
   ```rust
   pub use types::{
       ComposeContext, ComposeOperation, ComposeOperationSet, ComposeOptions,
       ComposePhase, ComposeReport, ComposeSource, ComposeWarning,
   };
   // New:
   pub use biscuit_file::PathPosition;
   ```

### Phase 2: Transclusion resolver

**File: `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs`**

9. Update `resolve_path()` to accept and apply magic paths from `TransclusionOptions`:
   ```rust
   let mut file_ref = FileReference::new(ref_input)?;
   for (path, position) in &options.magic_paths {
       file_ref = file_ref.add_magic_path(path, *position);
   }
   let resolved = file_ref.resolve()?;
   ```

### Phase 3: Reference module (graph, validate, transclusion resolution)

The three call sites in `reference/` all construct `FileReference::new()` independently of `ComposeOptions`. They need access to the magic paths. The most ergonomic approach:

**Option A (chosen): Pass magic paths as a parameter**

These functions already receive `&ComposeSource`. We add a `magic_paths: &[(PathBuf, PathPosition)]` parameter alongside it. This avoids coupling the reference module to `ComposeOptions` while still being flexible.

**File: `darkmatter/lib/src/markdown/reference/graph.rs`**

10. Update `resolve_local_target()` signature to accept magic paths:
    ```rust
    fn resolve_local_target(
        raw_target: &str,
        source: &ComposeSource,
        magic_paths: &[(PathBuf, PathPosition)],
    ) -> Option<std::path::PathBuf>
    ```
11. Apply magic paths to `FileReference`:
    ```rust
    let mut file_ref = biscuit_file::FileReference::new(raw_target).ok()?;
    for (path, position) in magic_paths {
        file_ref = file_ref.add_magic_path(path, *position);
    }
    if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) { ... }
    ```
12. Update all callers of `resolve_local_target()` within `graph.rs` to pass magic paths through. This likely means the graph-building entry points need to accept magic paths too — check the public `build_reference_graph()` or equivalent API and thread the parameter down.

**File: `darkmatter/lib/src/markdown/reference/validate.rs`**

13. Update `validate_local_path()` to accept and apply magic paths (same pattern as graph.rs)
14. Update callers within the validation pipeline

**File: `darkmatter/lib/src/markdown/reference/mod.rs`**

15. Update `resolve_transclusion_target()` to accept and apply magic paths
16. Update callers

### Phase 4: Public API surface for reference/graph/validate

The graph and validation modules have their own public entry points (e.g., `ReferenceGraph::build()`, `validate_references()`). These need to accept magic paths from callers.

17. Audit the public API of `darkmatter/lib/src/markdown/reference/graph.rs` for graph construction entry points and add magic_paths parameter
18. Audit the public API of `darkmatter/lib/src/markdown/reference/validate.rs` for validation entry points and add magic_paths parameter
19. If the graph/validate APIs already take `ComposeOptions` or `ComposeSource`, prefer extending the existing parameter. If they only take `ComposeSource`, add a separate `magic_paths` slice parameter.

### Phase 5: Cache invalidation

20. Update `cache/hashing.rs::options_hash()` to include `magic_paths` in the hash computation so that different magic path configurations don't return stale cached results.

### Phase 6: Tests

21. **Unit test in `resolver.rs`**: Compose with magic paths prepended — verify `@ref` resolves from custom root before git root
22. **Unit test in `resolver.rs`**: Compose with magic paths appended — verify git root is preferred, custom root is fallback
23. **Unit test in `types.rs`**: Builder test — verify `with_magic_path()` accumulates entries and they appear in `transclusion_options()`
24. **Unit test in `graph.rs`**: Build a reference graph where an `@` reference resolves via a custom magic path
25. **Unit test in `validate.rs`**: Validate references where `@` paths use custom magic paths

### Phase 7: Documentation

26. Update `ComposeOptions` struct doc comment to mention magic path support
27. Add doc examples on `with_magic_path()` showing the claudine use case
28. Update darkmatter README/skill if they mention `@` path resolution

## Claudine integration (out-of-scope but noted)

Once this feature lands, claudine's `prepare.rs` changes from:

```rust
let options = ComposeOptions::new().with_source_file(&source.resolved_path);
```

to something like:

```rust
let options = ComposeOptions::new()
    .with_source_file(&source.resolved_path)
    .with_magic_path(&project_claudine_dir, PathPosition::Start)
    .with_magic_path(&global_claudine_dir, PathPosition::Start);
```

This is a separate PR/feature once the darkmatter plumbing lands.

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Breaking change to internal `TransclusionOptions` | None | It's `pub(crate)`, no external consumers |
| Breaking change to reference module internal fns | None | They're private; only callers within darkmatter |
| Public API breakage on graph/validate entry points | **Medium** | If public APIs already take `&ComposeSource`, adding a parameter is breaking. May need to accept `&ComposeOptions` instead, or provide a parallel method. Check CLI callers first. |
| Cache staleness | Low | Phase 5 addresses by including magic_paths in hash |
| Performance | Negligible | Just cloning a small Vec of paths |

## File Change Summary

| File | Change Type |
|------|-------------|
| `compose/types.rs` | Add field, builder, thread to `TransclusionOptions` |
| `compose/mod.rs` | Add `PathPosition` re-export |
| `compose/transclusion/resolver.rs` | Apply magic paths to `FileReference` |
| `reference/graph.rs` | Thread magic paths through resolution |
| `reference/validate.rs` | Thread magic paths through validation |
| `reference/mod.rs` | Thread magic paths through transclusion resolution |
| `compose/cache/hashing.rs` | Include magic_paths in options hash |
| Tests across all above | New test cases |
