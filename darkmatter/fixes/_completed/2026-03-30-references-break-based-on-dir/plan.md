# Fix Plan: Magic Path Validation Fails When CWD ≠ Document Directory

## Summary

Three locations use `resolve_relative()` then check the returned relative path with CWD-dependent operations (`.exists()`, `.canonicalize()`). The fix eliminates or corrects these CWD-dependent checks.

## Steps

### Step 1: Fix `validate_local_path()` in validate.rs:346-359

**Strategy**: Option A — trust `resolve_relative`'s implicit existence guarantee.

`resolve_relative()` internally calls `resolve()`, which only returns `Some` after confirming `candidate.is_file()`. The `.exists()` re-check is redundant and broken (resolves against CWD).

**Change**: Remove the `if resolved.exists()` branch; if `resolve_relative` returns `Ok(Some(_))`, count as valid.

```rust
// Before
if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
    if resolved.exists() {
        report.references_valid += 1;
    } else {
        report.issues.push(/* MissingLocalTarget */);
    }
    return;
}

// After
if let Ok(Some(_resolved)) = file_ref.resolve_relative(base_dir) {
    report.references_valid += 1;
    return;
}
```

### Step 2: Fix `validate_cross_doc_fragment()` in validate.rs:493-504

**Strategy**: Option B — reconstitute absolute path before `.exists()` check.

Unlike `validate_local_path`, this function needs the resolved path to read the target file for heading extraction. Join with `base_dir` to make it CWD-independent.

**Change**: After `resolve_relative` returns, join with `base_dir` before calling `.exists()`.

```rust
// Before
let target_path = if let Ok(file_ref) = ... {
    file_ref.resolve_relative(base_dir).ok().flatten()
} else { None }
.unwrap_or_else(|| base_dir.map(|d| d.join(path)).unwrap_or(...));

if !target_path.exists() { return; }

// After — when resolve_relative returns a relative path, join with base_dir
let target_path = if let Ok(file_ref) = ... {
    file_ref.resolve_relative(base_dir).ok().flatten().map(|rel| {
        match base_dir {
            Some(bd) => bd.join(&rel),
            None => rel,
        }
    })
} else { None }
.unwrap_or_else(|| base_dir.map(|d| d.join(path)).unwrap_or(...));
```

### Step 3: Fix `resolve_local_target()` in graph.rs:780-781

**Strategy**: Option B — join with `base_dir` before `.canonicalize()`.

`.canonicalize()` resolves relative paths against CWD. Join with base_dir first.

```rust
// Before
if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
    return Some(resolved.canonicalize().unwrap_or(resolved));
}

// After
if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
    let abs = base_dir.map(|bd| bd.join(&resolved)).unwrap_or(resolved);
    return Some(abs.canonicalize().unwrap_or(abs));
}
```

### Step 4: Add regression test

Add a test that creates a temp git repo, places a magic-path reference in a nested document, and validates from a different CWD. This is the exact scenario that was broken.

Test structure:
1. Create temp dir with `git init`
2. Create `darkmatter/docs/inline/text-replacement.md` inside it
3. Create a source document with `[@darkmatter/docs/inline/text-replacement.md]` link
4. Set magic path pointing `@darkmatter` → `<repo>/darkmatter`
5. Run `validate()` and assert the reference is valid
