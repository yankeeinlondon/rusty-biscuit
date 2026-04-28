# Implicit Relative Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the current catch-all "relative" reference kind into an `ImplicitRelative` kind (bare paths like `foo.md`, `docs/spec.md`) that searches both the CWD and the git repository root, while keeping explicit relative paths (`./foo.md`, `../foo.md`) CWD-only.

**Architecture:** The change is contained to `biscuit-file/lib/src/file_reference/`. Three surgical edits:

1. `parse.rs` — detect `./` / `../` as `Relative`; anything else lacking a recognized prefix becomes `ImplicitRelative`.
2. `mod.rs` — add an `ImplicitRelative(PathTemplate)` variant to `ReferenceKind` (internal enum) and extend the `template()` accessor.
3. `resolve.rs` — add a branch in `collect_roots` that returns `[cwd, git_root]` for `ImplicitRelative`, de-duplicating when they coincide.

Recursive search (`%` prefix) works unchanged because `build_search_roots` delegates to `collect_roots`. `Magic` / `Package` / `Vault` / `Absolute` kinds are untouched.

**Tech Stack:** Rust 2024 edition, `thiserror`, `walkdir`, `git2`, `cargo_metadata`, `tempfile` (tests), `serial_test` (tests that mutate CWD).

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `biscuit-file/lib/src/file_reference/mod.rs` | Modify | Add `ImplicitRelative` variant + `template()` arm |
| `biscuit-file/lib/src/file_reference/parse.rs` | Modify | Distinguish `./`/`../` from bare paths; extend `DetectedKind`; adjust inline unit tests |
| `biscuit-file/lib/src/file_reference/resolve.rs` | Modify | Handle `ImplicitRelative` in `collect_roots` |
| `biscuit-file/lib/tests/implicit_relative.rs` | Create | Integration test: tempdir + `git init` + `resolve_from` |
| `biscuit-file/docs/topics/file-references.md` | Modify | Document the new behavior in the "Relative References" section |
| `biscuit-file/lib/Cargo.toml` | Verify | Confirm `tempfile` and `serial_test` are present under `[dev-dependencies]`; add if missing |

---

## Task 1: Add `ImplicitRelative` variant to `ReferenceKind`

**Files:**
- Modify: `biscuit-file/lib/src/file_reference/mod.rs:236-254`

- [ ] **Step 1: Edit `ReferenceKind` enum and `template()` method**

Open `biscuit-file/lib/src/file_reference/mod.rs` and replace the block at lines 235-254 with:

```rust
#[derive(Debug, Clone)]
pub(crate) enum ReferenceKind {
    Relative(PathTemplate),
    ImplicitRelative(PathTemplate),
    Absolute(PathTemplate),
    Magic(PathTemplate),
    Package(PathTemplate),
    Vault(PathTemplate),
}

impl ReferenceKind {
    pub(crate) fn template(&self) -> &PathTemplate {
        match self {
            Self::Relative(t)
            | Self::ImplicitRelative(t)
            | Self::Absolute(t)
            | Self::Magic(t)
            | Self::Package(t)
            | Self::Vault(t) => t,
        }
    }
}
```

- [ ] **Step 2: Verify the crate still compiles (it will fail — that is expected)**

Run: `cargo check -p biscuit-file`
Expected: FAIL with non-exhaustive match errors in `parse.rs` (`match kind { ... }` at the top of `parse()`) and `resolve.rs` (`match kind { ... }` in `collect_roots`). This confirms the two follow-up sites.

## Task 2: Parse bare paths as `ImplicitRelative`

**Files:**
- Modify: `biscuit-file/lib/src/file_reference/parse.rs:22-73`
- Modify (tests): `biscuit-file/lib/src/file_reference/parse.rs:275-283` (the `bare_filename` test)

- [ ] **Step 1: Rewrite the `bare_filename` test to expect `ImplicitRelative`**

In `biscuit-file/lib/src/file_reference/parse.rs`, replace the existing `bare_filename` test (at the bottom of the file) with:

```rust
#[test]
fn bare_filename_is_implicit_relative() {
    let parsed = parse("foo.md").unwrap();
    assert!(!parsed.recursive);
    assert!(matches!(parsed.kind, ReferenceKind::ImplicitRelative(_)));
}

#[test]
fn bare_subdir_path_is_implicit_relative() {
    let parsed = parse("docs/spec.md").unwrap();
    assert!(!parsed.recursive);
    assert!(matches!(parsed.kind, ReferenceKind::ImplicitRelative(_)));
    let template = parsed.kind.template();
    assert_eq!(
        template.segments[0],
        TemplateSegment::Literal("docs/spec.md".to_string())
    );
}

#[test]
fn explicit_dot_slash_is_relative() {
    let parsed = parse("./foo.md").unwrap();
    assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
}

#[test]
fn explicit_dotdot_slash_is_relative() {
    let parsed = parse("../foo.md").unwrap();
    assert!(matches!(parsed.kind, ReferenceKind::Relative(_)));
}
```

- [ ] **Step 2: Run the new tests to confirm they fail**

Run: `cargo test -p biscuit-file --lib file_reference::parse`
Expected: FAIL — compilation error (the `ImplicitRelative` variant is matched but the parser never produces it) AND the `bare_filename_is_implicit_relative` assertion would fail if it compiled.

- [ ] **Step 3: Extend `DetectedKind` and `detect_kind`**

Still in `biscuit-file/lib/src/file_reference/parse.rs`, replace the `DetectedKind` enum (around line 46-52) and the `detect_kind` function (around line 54-73) with:

```rust
enum DetectedKind {
    Relative,
    ImplicitRelative,
    Absolute,
    Magic,
    Package,
    Vault,
}

/// Detect the reference kind from its prefix and return the remaining path string.
fn detect_kind(s: &str) -> (DetectedKind, &str) {
    // vault:: (double colon) before vault: (single colon)
    if let Some(rest) = s.strip_prefix("vault::") {
        return (DetectedKind::Vault, rest);
    }
    if let Some(rest) = s.strip_prefix("vault:") {
        return (DetectedKind::Vault, rest);
    }
    if let Some(rest) = s.strip_prefix('@') {
        return (DetectedKind::Magic, rest);
    }
    if let Some(rest) = s.strip_prefix('!') {
        return (DetectedKind::Package, rest);
    }
    if s.starts_with('/') {
        return (DetectedKind::Absolute, s);
    }
    if s.starts_with("./") || s.starts_with("../") || s == "." || s == ".." {
        return (DetectedKind::Relative, s);
    }
    (DetectedKind::ImplicitRelative, s)
}
```

Note: `s == "."` and `s == ".."` are included so a lone `.` or `..` still maps to `Relative` rather than `ImplicitRelative`; the user clearly meant CWD.

- [ ] **Step 4: Extend the mapping in `parse()`**

In the same file, locate the `match kind { ... }` block inside `parse()` (around lines 24-30) and replace it with:

```rust
        kind: match kind {
            DetectedKind::Relative => ReferenceKind::Relative(template),
            DetectedKind::ImplicitRelative => ReferenceKind::ImplicitRelative(template),
            DetectedKind::Absolute => ReferenceKind::Absolute(template),
            DetectedKind::Magic => ReferenceKind::Magic(template),
            DetectedKind::Package => ReferenceKind::Package(template),
            DetectedKind::Vault => ReferenceKind::Vault(template),
        },
```

- [ ] **Step 5: Also update the existing `interpolation_single_var` test**

That test currently asserts `matches!(parsed.kind, ReferenceKind::Relative(_))` for the input `"{{DIR}}/foo.md"`. Since this input has no `./` prefix it is now `ImplicitRelative`. Find the test in `parse.rs` (around line 196-210) and change the kind assertion from `ReferenceKind::Relative(_)` to `ReferenceKind::ImplicitRelative(_)`.

- [ ] **Step 6: Run parse tests to confirm they all pass**

Run: `cargo test -p biscuit-file --lib file_reference::parse`
Expected: PASS for all parse tests (including the four new ones and the adjusted `interpolation_single_var`).

- [ ] **Step 7: Commit**

```bash
git add biscuit-file/lib/src/file_reference/mod.rs biscuit-file/lib/src/file_reference/parse.rs
git commit -m "feat(biscuit-file): parse bare paths as ImplicitRelative kind"
```

## Task 3: Resolve `ImplicitRelative` against CWD and git root

**Files:**
- Modify: `biscuit-file/lib/src/file_reference/resolve.rs:125-165` (the `collect_roots` function)

- [ ] **Step 1: Write a failing unit test for the new resolver behavior**

Append the following tests to the existing `#[cfg(test)] mod tests { ... }` block at the bottom of `biscuit-file/lib/src/file_reference/resolve.rs`:

```rust
    #[test]
    fn implicit_relative_uses_cwd_then_git_root() {
        use crate::file_reference::{MagicPathList, ParsedReference, ReferenceKind};

        // We can't call collect_roots for ImplicitRelative without a real git
        // context, so exercise the *direct* path by constructing a
        // ParsedReference with no git root. In that case the only root
        // should be CWD, matching the "git lookup returned None" branch.
        let parsed = ParsedReference {
            recursive: false,
            kind: ReferenceKind::ImplicitRelative(PathTemplate {
                segments: vec![TemplateSegment::Literal("nope.md".to_string())],
            }),
        };
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env: std::collections::HashMap::new(),
        };
        let roots =
            collect_roots(&parsed.kind, &MagicPathList::default(), &[], &ctx).unwrap();
        // /tmp has no git repo, so only CWD is returned.
        assert_eq!(roots, vec![PathBuf::from("/tmp")]);
    }
```

- [ ] **Step 2: Run the new test to confirm it fails**

Run: `cargo test -p biscuit-file --lib file_reference::resolve`
Expected: FAIL — compile error (`collect_roots` does not cover `ImplicitRelative` yet, and the `match` is exhaustive).

- [ ] **Step 3: Add the `ImplicitRelative` arm to `collect_roots`**

In `biscuit-file/lib/src/file_reference/resolve.rs`, locate the `match kind { ... }` block inside `collect_roots` (around line 131). Immediately after the `ReferenceKind::Relative(_) => Ok(vec![ctx.cwd.clone()]),` line, insert:

```rust
        ReferenceKind::ImplicitRelative(_) => {
            let mut roots = vec![ctx.cwd.clone()];
            if let Some(git_root) = find_git_root(&ctx.cwd)?
                && git_root != ctx.cwd
            {
                roots.push(git_root);
            }
            Ok(roots)
        }
```

Rationale for the `git_root != ctx.cwd` guard: when CWD *is* the git root, the two roots are identical and the second filesystem probe would be wasted work.

- [ ] **Step 4: Run the resolver tests to confirm they pass**

Run: `cargo test -p biscuit-file --lib file_reference::resolve`
Expected: PASS. All existing tests (diff_paths, normalize_*, interpolate_*) and the new `implicit_relative_uses_cwd_then_git_root` test pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-file/lib/src/file_reference/resolve.rs
git commit -m "feat(biscuit-file): resolve ImplicitRelative against CWD then git root"
```

## Task 4: Add an end-to-end integration test

**Files:**
- Verify: `biscuit-file/lib/Cargo.toml` — confirm `tempfile` and `serial_test` are in `[dev-dependencies]`
- Create: `biscuit-file/lib/tests/implicit_relative.rs`

- [ ] **Step 1: Check dev-dependencies**

Run: `grep -E '^(tempfile|serial_test)' biscuit-file/lib/Cargo.toml`
Expected: at minimum a line like `tempfile = ...` should exist. If `serial_test` is missing, add it under `[dev-dependencies]`:

```toml
serial_test = { workspace = true }
```

(Confirm `serial_test` exists in the root `Cargo.toml`'s `[workspace.dependencies]` first — per the repo CLAUDE.md, `serial_test` is already used across the repo. If it is not in the workspace table, add it there too as `serial_test = "3"`.)

If `tempfile` is missing, add `tempfile = { workspace = true }` similarly.

- [ ] **Step 2: Create the integration test file**

Create `biscuit-file/lib/tests/implicit_relative.rs` with the following contents:

```rust
//! Integration tests for the `ImplicitRelative` reference kind.
//!
//! These tests build a real git repository in a temp directory so the
//! `find_git_root` logic in `biscuit-file` can discover it the same way it
//! would in production.

use std::fs;
use std::path::Path;

use biscuit_file::FileReference;
use tempfile::TempDir;

/// Initialise a fresh git repository at `path`.
fn git_init(path: &Path) {
    // Use git2 directly so we don't depend on a system git binary.
    git2::Repository::init(path).expect("git init failed");
}

#[test]
fn resolves_file_in_git_root_when_absent_from_cwd() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Create `file_in_root.md` at the repo root.
    fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();

    // Create a nested subdir that will act as the "CWD".
    let subdir = repo_root.join("sub/dir");
    fs::create_dir_all(&subdir).unwrap();

    // Canonicalize to match what the resolver does internally.
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("file_in_root.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("file_in_root.md").as_path()),
        "implicit relative path should fall back to git root"
    );
}

#[test]
fn prefers_cwd_over_git_root_on_name_collision() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Same filename exists in both git root and a subdirectory.
    fs::write(repo_root.join("notes.md"), b"root").unwrap();
    let subdir = repo_root.join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("notes.md"), b"subdir").unwrap();

    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("notes.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(subdir.join("notes.md").as_path()),
        "CWD should take priority over git root for implicit relative refs"
    );
}

#[test]
fn explicit_relative_does_not_fall_back_to_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();
    let subdir = repo_root.join("sub");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("./file_in_root.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert!(
        resolved.is_none(),
        "./ prefix should pin lookup to CWD only; got {resolved:?}"
    );
}

#[test]
fn subdir_path_resolves_against_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::create_dir_all(repo_root.join("foo/bar")).unwrap();
    fs::write(repo_root.join("foo/bar/doc.md"), b"nested").unwrap();

    // CWD is the repo root's sibling subdir with no `foo/bar/doc.md`.
    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("foo/bar/doc.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("foo/bar/doc.md").as_path()),
    );
}

#[test]
fn returns_none_when_neither_cwd_nor_git_root_has_file() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    let subdir = repo_root.join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("does_not_exist.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert!(resolved.is_none());
}

#[test]
fn recursive_implicit_relative_finds_file_under_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Deeply nested file.
    fs::create_dir_all(repo_root.join("a/b/c")).unwrap();
    fs::write(repo_root.join("a/b/c/deep.md"), b"deep").unwrap();

    // CWD is a sibling that won't see `deep.md` via its own walk.
    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("%deep.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("a/b/c/deep.md").as_path()),
        "recursive search should include git root as a traversal start"
    );
}
```

Why `resolve_from` instead of mutating the process CWD: `resolve_from` sets the resolution context's `cwd` directly, so `find_git_root` and the candidate join both operate on our tempdir. No `std::env::set_current_dir` and therefore no cross-test races — no `serial_test` attribute needed.

- [ ] **Step 3: Run the new integration tests**

Run: `cargo test -p biscuit-file --test implicit_relative`
Expected: PASS — all six tests green.

- [ ] **Step 4: Run the whole crate test suite to catch regressions**

Run: `just test` from `biscuit-file/` (or `cargo test -p biscuit-file` from the repo root).
Expected: PASS — no regressions in existing parse, resolve, round-trip, or CLI tests.

- [ ] **Step 5: Commit**

```bash
git add biscuit-file/lib/tests/implicit_relative.rs biscuit-file/lib/Cargo.toml
git commit -m "test(biscuit-file): integration tests for implicit relative paths"
```

## Task 5: Update the file-references topic doc

**Files:**
- Modify: `biscuit-file/docs/topics/file-references.md`

- [ ] **Step 1: Rewrite the "Relative References" section**

Open `biscuit-file/docs/topics/file-references.md`. Replace the current "Relative References (no prefix)" section (roughly lines 34-53) with:

```markdown
## Relative References

There are two kinds of relative reference, distinguished by whether the path
*explicitly* starts with `./` or `../`:

### Explicit Relative (`./`, `../`)

A leading `./` or `../` pins the lookup to the current working directory.
No fallback search is performed.

```text
./README.md         → <CWD>/README.md
../sibling/foo.md   → <CWD>/../sibling/foo.md   (normalized)
```

### Implicit Relative (bare path, no prefix)

A bare path with no recognized prefix is treated as *implicitly* relative.
It is first checked against the CWD and, if not found there, against the
root of the enclosing git repository (when one is present).

```text
foo.md              → <CWD>/foo.md, then <git_root>/foo.md
docs/spec.md        → <CWD>/docs/spec.md, then <git_root>/docs/spec.md
```

If the reference is not found in either location, `resolve()` returns
`Ok(None)`. If no git repository is discoverable, only the CWD is searched.

```rust,no_run
use biscuit_file::FileReference;

// From <repo>/biscuit-file/lib/src, resolves to <repo>/README.md
let file_ref = FileReference::new("README.md")?;
let path = file_ref.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```
```

- [ ] **Step 2: Update the "Quick Reference" table**

Near the top of the same file (around lines 21-30), replace the single "Relative" row with two rows:

```markdown
| Prefix                | Kind                  | Resolves against                                         | Example                      |
|-----------------------|-----------------------|----------------------------------------------------------|------------------------------|
| `./` or `../`         | **Relative**          | Current working directory                                | `./src/main.rs`, `../a.md`   |
| _(none)_              | **Implicit Relative** | CWD, then git repository root                            | `README.md`, `docs/spec.md`  |
| `/`                   | **Absolute**          | Used verbatim                                            | `/etc/config.toml`           |
| `@`                   | **Magic**             | Configurable search roots (git root, HOME, custom paths) | `@docs/spec.md`              |
| `!`                   | **Package**           | Cargo workspace package area (or git root fallback)     | `!README.md`                 |
| `vault:` or `vault::` | **Vault**             | Configured vault root directories                        | `vault:notes/today.md`       |
```

- [ ] **Step 3: Update the Phase-4 "Search Roots" table**

Near the bottom of the same file (the table immediately under "Phase 4: Candidate Building", around lines 386-392), replace its body with:

```markdown
| Kind              | Search Roots                                                       |
|-------------------|--------------------------------------------------------------------|
| Relative          | `[CWD]`                                                            |
| Implicit Relative | `[CWD, git_root]` (git_root omitted when equal to CWD or absent)   |
| Absolute          | `[interpolated path directly]`                                     |
| Magic             | `magic_paths.prepend` → `git_root` → `HOME` → `magic_paths.append` |
| Package           | `[package_area or git_root]`                                       |
| Vault             | `vault_roots` → `$VAULT` env var split paths                       |
```

- [ ] **Step 4: Run docs-free sanity build**

Run: `cargo check -p biscuit-file && cargo test -p biscuit-file --doc`
Expected: PASS — doctests still compile against the updated prose.

- [ ] **Step 5: Commit**

```bash
git add biscuit-file/docs/topics/file-references.md
git commit -m "docs(biscuit-file): document implicit relative path resolution"
```

## Task 6: Full verification

- [ ] **Step 1: Lint**

Run: `just lint` from `biscuit-file/` (or `cargo clippy -p biscuit-file --all-targets -- -D warnings`).
Expected: PASS — no new clippy warnings.

- [ ] **Step 2: Format**

Run: `cargo fmt --package biscuit-file`
Expected: no diff. If there is a diff, stage and amend the last commit — do NOT create a new "format" commit.

- [ ] **Step 3: Full area test sweep**

Run: `just test` from `biscuit-file/`.
Expected: PASS — library unit tests, `implicit_relative` integration tests, `round_trip` integration tests, and CLI tests all green.

- [ ] **Step 4: Manual smoke test via `bf`**

From anywhere inside the rusty-biscuit monorepo, run:

```bash
cargo run -p biscuit-file-cli -- ref resolve README.md
```

(Use whatever the CLI's resolve subcommand is called — check `bf --help` if unsure; no changes to the CLI are needed for this feature.)

Expected: prints the absolute path to the monorepo's top-level `README.md`, demonstrating that the implicit relative fell back to the git root.

If the CLI does not expose a resolve-only subcommand, skip this step — the integration tests already exercise the public API.

---

## Self-Review Checklist (performed while writing this plan)

1. **Spec coverage** — all spec bullets covered:
    - Detect and classify implicit relative paths ✅ Task 2
    - Search CWD first, then git root ✅ Task 3
    - Git-root fallback only when git repo exists ✅ Task 3 (guarded by `find_git_root` returning `None`)
    - Multi-segment paths behave the same ✅ Task 4 `subdir_path_resolves_against_git_root`
    - Invalid path returns `Ok(None)` ✅ Task 4 `returns_none_when_neither_cwd_nor_git_root_has_file`
    - Recursive (`%`) inherits new roots ✅ Task 4 `recursive_implicit_relative_finds_file_under_git_root`

2. **Type consistency** — `ReferenceKind::ImplicitRelative` is used identically in `mod.rs`, `parse.rs`, and `resolve.rs`. `DetectedKind::ImplicitRelative` never escapes `parse.rs`.

3. **Placeholder scan** — no TBD/TODO/"add appropriate handling" text; every code step shows full code.

4. **Edge cases covered** — CWD == git root is guarded (`git_root != ctx.cwd`); bare `.` and `..` are still `Relative`; existing parse tests adjusted where their assertions would otherwise drift.
