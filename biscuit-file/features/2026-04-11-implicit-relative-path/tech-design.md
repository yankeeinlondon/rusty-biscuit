# Technical Design: Implicit Relative Path for FileReference

This document outlines the technical design for implementing "implicit relative path" resolution in the `FileReference` struct.

## Goals

- Support file references that are neither absolute nor explicitly relative (starting with `./` or `../`).
- Resolve these implicit relative paths by searching both the current working directory (CWD) and the git repository root.
- Maintain existing resolution logic for magic (`@`), package (`!`), vault, absolute, and explicitly relative paths.

## Proposed Changes

### 1. Update `ReferenceKind` Enum

In `biscuit-file/lib/src/file_reference/mod.rs`, add `ImplicitRelative` to the `ReferenceKind` enum.

```rust
pub(crate) enum ReferenceKind {
    Relative(PathTemplate),         // Explicitly relative: ./foo.md, ../foo.md
    ImplicitRelative(PathTemplate), // Implicitly relative: foo.md, sub/foo.md
    Absolute(PathTemplate),
    Magic(PathTemplate),
    Package(PathTemplate),
    Vault(PathTemplate),
}
```

### 2. Update Path Parsing Logic

In `biscuit-file/lib/src/file_reference/parse.rs`, update `detect_kind` to distinguish between explicitly relative and implicitly relative paths.

```rust
fn detect_kind(s: &str) -> (DetectedKind, &str) {
    // ... (existing prefixes)
    if s.starts_with('/') {
        return (DetectedKind::Absolute, s);
    }
    if s.starts_with("./") || s.starts_with("../") {
        return (DetectedKind::Relative, s);
    }
    (DetectedKind::ImplicitRelative, s)
}
```

An internal `DetectedKind` enum will also need to be updated to include `ImplicitRelative`.

### 3. Update Resolution Logic

In `biscuit-file/lib/src/file_reference/resolve.rs`, update `collect_roots` to handle `ReferenceKind::ImplicitRelative`.

```rust
ReferenceKind::Relative(_) => Ok(vec![ctx.cwd.clone()]),
ReferenceKind::ImplicitRelative(_) => {
    let mut roots = vec![ctx.cwd.clone()];
    if let Some(git_root) = find_git_root(&ctx.cwd)? {
        // Only add git_root if it's different from CWD to avoid redundant checks
        if git_root != ctx.cwd {
            roots.push(git_root);
        }
    }
    Ok(roots)
}
```

### 4. Recursive Resolution

The `resolve_recursive` function in `resolve.rs` uses `build_search_roots`, which in turn calls `collect_roots`. This means recursive resolution (starting with `%`) will also benefit from the new search roots for implicit relative paths.

For example, `%foo.md` will search for `foo.md` recursively starting from both the CWD and the git root.

## Verification Plan

### Automated Tests

1.  **Parsing Tests (`parse.rs`):**
    - Verify `foo.md` is parsed as `ImplicitRelative`.
    - Verify `./foo.md` and `../foo.md` are still parsed as `Relative`.
    - Verify subdirectories like `docs/spec.md` are parsed as `ImplicitRelative`.

2.  **Resolution Tests (`resolve.rs`):**
    - Mock a directory structure where a file exists only in the git root, not the CWD.
    - Verify `FileReference::new("file_in_root.md")?.resolve()` finds the file.
    - Verify `FileReference::new("./file_in_root.md")?.resolve()` does *not* find the file (explicitly relative to CWD).
    - Verify `FileReference::new("file_in_cwd.md")?.resolve()` finds the file in CWD.

3.  **Recursive Tests:**
    - Verify `%file_deep_in_repo.md` finds the file if it's anywhere under the git root.

### Manual Verification

- Use the `bf` (biscuit-file CLI) to resolve various paths in the `rusty-biscuit` monorepo to ensure it behaves as expected.
