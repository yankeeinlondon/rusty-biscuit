# Magic Path Validation Fails When CWD ≠ Document Directory

## Symptom

```sh
# From darkmatter/darkmatter/ (package area root)
md compose example-docs/composition/test.md
# → FAILS with:
#   Invalid Hyperlink(s)
#   - the preparation.md reference to @darkmatter/docs/inline/text-replacement.md is not valid
#   - the preparation.md reference to @darkmatter/docs/inline/interpolation.md is not valid

# From darkmatter/darkmatter/example-docs/composition/
md compose test.md
# → WORKS correctly
```

The compose operation itself is not broken — the **reference validation** step (which runs before compose) rejects valid magic-path hyperlinks when CWD differs from the referenced document's directory.

## Root Cause

The bug is in `validate_local_path()` at `darkmatter/lib/src/markdown/reference/validate.rs:329-402`.

### The problematic code path (validate.rs:341-361)

```rust
if let Ok(file_ref) = biscuit_file::FileReference::new(raw) {
    // ... add magic_paths ...
    if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
        if resolved.exists() {          // ← BUG: resolves against CWD, not base_dir
            report.references_valid += 1;
        } else {
            report.issues.push(/* MissingLocalTarget */);
        }
        return;
    }
}
```

### What happens step by step

1. `preparation.md` contains `[Text Replacement](@darkmatter/docs/inline/text-replacement.md)`
2. During validation, the reference's `source` is `ComposeSource::File(.../example-docs/composition/preparation.md)`
3. `base_dir` = `.../example-docs/composition/` (parent of preparation.md)
4. `FileReference::new("@darkmatter/docs/inline/text-replacement.md")` creates a **Magic** reference
5. `file_ref.resolve_relative(base_dir)` internally:
   - Calls `resolve()` which uses `ResolutionContext::from_ambient()` → `ctx.cwd = std::env::current_dir()`
   - `find_git_root(ctx.cwd)` finds the repo root (e.g., `/repo-root/`)
   - Builds candidate: `/repo-root/darkmatter/docs/inline/text-replacement.md` → **exists** → returns absolute path
   - Computes `diff_paths(absolute_path, base_dir)` → `../../docs/inline/text-replacement.md` (relative to base_dir)
   - Returns `Ok(Some("../../docs/inline/text-replacement.md"))`
6. `resolved.exists()` calls `std::fs::metadata()` which resolves relative paths against **CWD**, not `base_dir`
7. When CWD = `.../darkmatter/darkmatter/`:
   - `../../docs/inline/text-replacement.md` resolves to `/repo-root/docs/inline/text-replacement.md`
   - This path does **NOT** exist (the file is at `darkmatter/docs/inline/...`, not `docs/inline/...`)
   - Validation reports an error
8. When CWD = `.../example-docs/composition/`:
   - `../../docs/inline/text-replacement.md` resolves to `.../darkmatter/docs/inline/text-replacement.md`
   - This **does** exist
   - Validation passes

### Why compose itself works

The compose transclusion resolver (`transclusion/resolver.rs:109`) correctly uses `file_ref.resolve()` (which returns an **absolute** path) instead of `resolve_relative()`:

```rust
let resolved = file_ref.resolve()?;   // ← absolute, CWD-independent for magic paths
```

The validation code is the only place that uses `resolve_relative()` for existence checking.

## Affected Code Locations

### Primary (causes the reported bug)

- **`validate_local_path()`** in `darkmatter/lib/src/markdown/reference/validate.rs:341-361`

### Secondary (same pattern, latent bug)

- **`validate_cross_doc_fragment()`** in `validate.rs:488-493` — uses `resolve_relative` then `.exists()` on the result; a wrong CWD would cause fragment validation to silently skip valid cross-doc targets
- **`resolve_local_target()`** in `graph.rs:776-782` — uses `resolve_relative` then `.canonicalize()` which also resolves against CWD; the `unwrap_or(resolved)` fallback returns a relative path where an absolute one is expected

## Fix

### Option A: Trust `resolve_relative`'s implicit existence guarantee (minimal, recommended)

`resolve_relative()` internally calls `resolve()`, which only returns `Some` after confirming `candidate.is_file()`. If `resolve_relative` returns `Ok(Some(_))`, the file existed at resolution time. The `.exists()` re-check is redundant and broken.

```rust
// validate.rs – validate_local_path
if let Ok(Some(_resolved)) = file_ref.resolve_relative(base_dir) {
    report.references_valid += 1;
    return;
}
```

**Pros**: One-line change, zero performance impact, no new allocations.
**Cons**: Relies on `resolve()`'s internal existence check; a TOCTOU race is theoretically possible but irrelevant in practice.

### Option B: Reconstitute the absolute path before checking existence

If we want an explicit existence check, join the relative result back with its base:

```rust
// validate.rs – validate_local_path
if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
    let check_path = match base_dir {
        Some(bd) => bd.join(&resolved),
        None => resolved,
    };
    if check_path.exists() {
        report.references_valid += 1;
    } else {
        report.issues.push(/* MissingLocalTarget */);
    }
    return;
}
```

**Pros**: Explicit existence check, easy to understand.
**Cons**: Allocates a new PathBuf; reconstituting an absolute path from a relative path just to check existence is wasteful when `resolve()` already did the check.

### Option C: Use `resolve()` directly instead of `resolve_relative()`

Since validation only needs a boolean "exists or not", not a path:

```rust
// validate.rs – validate_local_path
if let Ok(file_ref) = biscuit_file::FileReference::new(raw) {
    // ... add magic_paths ...
    match file_ref.resolve() {
        Ok(Some(_)) => {
            report.references_valid += 1;
            return;
        }
        Ok(None) => { /* fall through to fallback */ }
        Err(_) => { /* fall through to fallback */ }
    }
}
```

**Pros**: Returns absolute path, no CWD dependency for magic paths.
**Cons**: `resolve()` uses CWD for **relative** references (e.g., `./file.md`), which would be wrong for child documents. The fallback handles relative paths correctly, but we'd be running FileReference resolution for no benefit on relative paths.

### Recommendation

**Use Option A for `validate_local_path`** — it's the smallest, safest change with zero performance impact. The `resolve()` contract already guarantees the file exists.

For the secondary locations:
- `validate_cross_doc_fragment`: Apply the same Option B pattern (join relative result with base_dir)
- `resolve_local_target` in graph.rs: Consider switching to `resolve()` for magic paths and keeping the fallback for relative paths — but this is lower priority since it's a latent bug not currently triggered by example documents

## Testing Gap

The current test suite has **zero tests** for validating magic-path (`@`) references. All validation tests in `validate.rs` use:
- In-memory markdown (no source context) → tests `MissingSourceContext` warning
- `tempfile` directories with relative paths (`./file.md`) → tests simple local path validation
- Remote URLs and fragments

Missing test coverage:
1. **Magic path in a child document validated from a different CWD** — this is the exact scenario that broke
2. **Magic path validation where CWD is inside vs outside the repo**
3. **Cross-doc fragment validation for magic-path targets**
4. **`resolve_local_target` in graph.rs with magic paths and non-matching CWD**

A comprehensive regression test should:
- Create a temp git repo
- Create a nested source document with a magic-path hyperlink
- Set CWD to the repo root (not the document's directory)
- Run validation and assert the reference is reported as valid
