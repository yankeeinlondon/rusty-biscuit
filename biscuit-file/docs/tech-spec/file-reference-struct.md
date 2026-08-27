# FileReference Technical Specification

## Context

This document translates the functional behavior described in [file-resolution.md](../file-resolution.md) into an implementable design for the `biscuit-file` library.

The goal is to define a `FileReference` type that can capture a string-based file descriptor now and resolve it later against the runtime environment that exists at the time of resolution.

## Goals

- Provide a stable `FileReference` API centered on lazy file resolution.
- Support plain relative paths, absolute paths, magic references, package-root references, vault references, recursive references, and environment interpolation.
- Keep resolution deterministic and testable.
- Preserve the functional requirement that the current working directory at construction time must not influence later resolution.
- Fit repository conventions:
  - library APIs return `Result` for fallible operations
  - production code avoids `unwrap()` / `expect()`
  - public behavior is documented and covered by tests

## Non-goals

- Globbing support (`*.md`, `**/*.rs`).
- Remote URI resolution (`http:`, `https:`, `s3:`).
- Obsidian alias indexing or full wikilink graph resolution in v1.
- Symlink-aware canonicalization beyond normal filesystem existence checks.
- Cross-language monorepo detection outside Cargo workspace conventions.

## Functional Clarifications

The functional spec is intentionally broad. The implementation should lock down the following details:

- `resolve()` and `resolve_relative()` should return `Result<Option<PathBuf>, FileReferenceError>`, not bare `Option<PathBuf>`.
  - `Ok(None)` means the reference was well-formed but no file matched.
  - `Err(...)` means the reference could not be evaluated correctly.
- `vault:` is the canonical vault prefix.
  - For compatibility with the functional example, `vault::...` should also be accepted and normalized internally.
- Vault resolution in v1 should resolve vault-relative filesystem paths.
  - Full Obsidian alias lookup is deferred.
- Package-root resolution should be Cargo-workspace aware.
  - If workspace/package-area discovery fails, fall back to the git repo root only when the repo behaves like a single-package repository.

## Public API (Proposed)

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReference {
    raw: String,
    parsed: ParsedReference,
    magic_paths: MagicPathList,
    vault_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPosition {
    Start,
    End,
}

impl FileReference {
    pub fn new<S: Into<String>>(reference: S) -> Self;

    pub fn raw(&self) -> &str;

    pub fn add_magic_path<P: Into<PathBuf>>(self, position: PathPosition, root: P) -> Self;

    pub fn add_vault<P: Into<PathBuf>>(self, root: P) -> Self;

    pub fn resolve(&self) -> Result<Option<PathBuf>, FileReferenceError>;

    pub fn resolve_relative(
        &self,
        base: Option<&Path>,
    ) -> Result<Option<PathBuf>, FileReferenceError>;
}
```

## API Notes

- `new()` is infallible because it only stores and parses syntactic structure.
- Builder methods consume and return `Self` so they chain naturally.
- `resolve()` returns an absolute path when successful.
- `resolve_relative()` resolves first, then converts the result to a path relative to `base` or the current working directory when `base` is `None`.

## Internal Data Model

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedReference {
    recursive: bool,
    kind: ReferenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReferenceKind {
    Relative(PathTemplate),
    Absolute(PathTemplate),
    Magic(PathTemplate),
    Package(PathTemplate),
    Vault(PathTemplate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathTemplate {
    segments: Vec<TemplateSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TemplateSegment {
    Literal(String),
    EnvVar(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MagicPathList {
    prepend: Vec<PathBuf>,
    append: Vec<PathBuf>,
}
```

## Parsing Rules

Parsing is purely syntactic and must not inspect the filesystem.

Resolution prefix parsing order:

1. Strip a leading `%` and mark the reference as recursive.
2. Detect the base kind using this precedence:
   - `vault:`
   - `vault::`
   - `@`
   - `!`
   - absolute filesystem path
   - default relative path

Examples:

- `./foo.md` => `recursive = false`, `Relative`
- `%./foo.md` => `recursive = true`, `Relative`
- `@docs/spec.md` => `recursive = false`, `Magic`
- `%@README.md` => `recursive = true`, `Magic`
- `!lib/src/lib.rs` => `recursive = false`, `Package`
- `vault:notes/today.md` => `recursive = false`, `Vault`

## Interpolation Rules

Interpolation happens at resolution time, never at construction time.

Supported syntax:

- `{{NAME}}`

Rules:

- Environment variable names match `[A-Z0-9_]+`.
- Unknown variables produce `FileReferenceError::MissingEnvironmentVariable`.
- Empty placeholders such as `{{}}` are invalid syntax.
- Interpolated values are treated as raw path text and may contain separators.
- No escaping syntax is provided in v1.

## Resolution Context

`resolve()` must evaluate against live runtime state:

- current working directory
- git repository membership
- Cargo workspace/package metadata
- current environment variables
- configured builder paths

For testability, implementation should use an internal context type:

```rust
struct ResolutionContext {
    cwd: PathBuf,
    home_dir: Option<PathBuf>,
    env: HashMap<String, String>,
}
```

The public API continues to use ambient process state. Tests can inject a synthetic context through private or `pub(crate)` helpers.

## Resolution Pipeline

1. Load the resolution context.
2. Expand interpolation tokens into a resolved `PathBuf` or relative path string.
3. Build an ordered list of candidate roots based on `ReferenceKind`.
4. For non-recursive references, join each candidate root with the interpolated path and return the first existing file.
5. For recursive references, recursively search each root and return the first deterministic match.
6. Normalize the output to an absolute path without canonicalizing symlinks.

The resolver checks `is_file()` and ignores directories.

## Scope-Specific Resolution

### Relative References

Base root:

- current working directory at the time of resolution

Examples:

- `foo.md` => `{cwd}/foo.md`
- `../foo.md` => `{cwd}/../foo.md`

### Absolute References

Rules:

- After interpolation, the path must still be absolute.
- The resolver checks that exact path and returns it if it is a file.
- No alternate roots are consulted.

### Magic References (`@`)

Ordered search roots:

1. builder-provided `PathPosition::Start` roots, in insertion order
2. current git repository root, if one can be discovered from the current working directory
3. current user home directory, if available
4. builder-provided `PathPosition::End` roots, in insertion order

Magic references strip the leading `@` and treat the remaining path as relative beneath each root.

Example:

```rust
let reference = FileReference::new("@docs/spec.md")
    .add_magic_path(PathPosition::Start, "/opt/shared")
    .add_magic_path(PathPosition::End, "/Users/alice/.claude");
```

Search order:

1. `/opt/shared/docs/spec.md`
2. `{repo_root}/docs/spec.md`
3. `{home}/docs/spec.md`
4. `/Users/alice/.claude/docs/spec.md`

### Package References (`!`)

Package references resolve relative to the current package area, not the crate directory.

Detection algorithm:

1. Discover the current git repository root from the current working directory.
2. If no repo exists, return `Ok(None)`.
3. Attempt to load Cargo workspace metadata rooted at the repository.
4. Infer the current package area from the current working directory:
   - find the workspace member whose manifest directory is an ancestor of `cwd`
   - compute the path from workspace root to that member directory
   - use the first path component beneath the workspace root as the package-area root
5. If package-area inference succeeds, resolve `!` relative to `{workspace_root}/{first_component}`.
6. If workspace metadata does not indicate a monorepo-style package area, fall back to the git repo root.

Examples in this repository:

- inside `/repo/biscuit-file/lib/src`, `!docs/file-resolution.md` resolves from `/repo/biscuit-file`
- inside `/repo/homelab/server/src`, `!README.md` resolves from `/repo/homelab`
- inside `/repo/tabby/src`, `!Cargo.toml` resolves from `/repo/tabby`

If `cwd` is inside the repo but not inside any discovered package area, `!` returns `Ok(None)`.

### Vault References (`vault:`)

Ordered vault roots:

1. vaults added with `add_vault(...)`, in insertion order
2. paths from the `VAULT` environment variable, parsed with `std::env::split_paths`

Rules:

- The prefix `vault:` or `vault::` is stripped.
- The remaining path is treated as vault-relative.
- Each vault root is checked in order.
- If no vault roots are configured, return `Err(FileReferenceError::VaultNotConfigured)`.

Examples:

- `vault:daily/2026-03-12.md`
- `vault::projects/biscuit.md`

## Recursive Resolution (`%`)

Recursive mode changes how the already-selected base scope is searched.

Examples:

- `%foo.md`
- `%./docs/spec.md`
- `%@README.md`
- `%!Cargo.toml`

Rules:

- Recursive search only applies to the final base roots chosen by the non-recursive scope logic.
- If the interpolated target has directory components:
  - parent components define the recursive starting directory
  - the final file name becomes the match needle
- If the target is only a file name:
  - the base root becomes the recursive starting directory
  - the file name becomes the match needle
- Only files whose basename exactly matches the needle are considered matches.
- Search must be deterministic:
  - collect matching paths
  - sort lexicographically by full path
  - return the first result
- Directory traversal must not follow symlinks in v1.

Examples:

- `%foo.md` under `/repo` searches `/repo/**/foo.md`
- `%./docs/spec.md` under `/repo` searches `/repo/docs/**/spec.md`
- `%!README.md` in `biscuit-file/lib` searches `/repo/biscuit-file/**/README.md`

## Relative Output (`resolve_relative`)

`resolve_relative(base)` works as follows:

1. Call `resolve()`.
2. If the result is `Ok(None)`, return `Ok(None)`.
3. Compute a relative path from:
   - `base`, when provided
   - otherwise the current working directory at resolution time
4. Return the relative path, even if it contains `..` segments.

Implementation should use a small internal path-diff helper rather than adding a dependency solely for this operation.

## Error Model

```rust
#[derive(Debug, thiserror::Error)]
pub enum FileReferenceError {
    #[error("file reference syntax is invalid: {0}")]
    InvalidSyntax(String),

    #[error("environment variable `{name}` is not set")]
    MissingEnvironmentVariable { name: String },

    #[error("could not determine the current working directory: {0}")]
    CurrentDirectory(#[source] std::io::Error),

    #[error("could not inspect git repository state: {0}")]
    Git(String),

    #[error("could not inspect Cargo workspace state: {0}")]
    Workspace(String),

    #[error("vault reference used without any configured vault roots")]
    VaultNotConfigured,

    #[error("could not produce a relative path from `{from}` to `{to}`")]
    RelativePath {
        from: PathBuf,
        to: PathBuf,
    },

    #[error("filesystem error while resolving `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
```

## Error Semantics

- Missing file => `Ok(None)`
- Missing repo for `!` => `Ok(None)`
- Missing repo for `@` => continue to HOME and configured magic paths
- Missing environment variable in interpolation => `Err(MissingEnvironmentVariable)`
- No configured vaults for `vault:` => `Err(VaultNotConfigured)`
- Unreadable directories during recursive search:
  - skip the unreadable subtree when possible
  - only return an error for a root-level failure that prevents the requested search strategy entirely

## Recommended Module Layout

```txt
lib/src/
├── file_reference/
│   ├── mod.rs
│   ├── error.rs
│   ├── parse.rs
│   ├── resolve.rs
│   └── context.rs
└── lib.rs
```

Recommended responsibilities:

- `mod.rs`: public `FileReference` API, re-exports
- `error.rs`: `FileReferenceError`
- `parse.rs`: prefix parsing and interpolation tokenization
- `resolve.rs`: root discovery, recursive search, path joining
- `context.rs`: ambient resolution context and test helpers

## Dependencies

Recommended additions:

- `git2`
  - robust repo-root discovery without spawning `git`
- a caller-supplied repository scope catalog
  - pure-data package and package-area scope selection without filesystem discovery
- `walkdir`
  - deterministic recursive traversal

These dependencies should be added to `biscuit-file/lib/Cargo.toml` when implementation begins and reflected in `docs/dependencies.md` if that document is introduced for this package area.

## Test Strategy

### Unit Tests

- parse relative, absolute, magic, package, vault, and recursive prefixes
- parse `vault:` and `vault::` equivalently
- parse interpolation tokens and reject invalid placeholders
- verify ordered magic-path insertion behavior

### Integration Tests

Use `tempfile` for isolated directory trees and `serial_test` for environment-variable and current-directory mutation.

Cover:

- `resolve()` does not depend on construction-time CWD
- relative path lookup from runtime CWD
- absolute path pass-through
- `@` search order: start roots, repo root, HOME, end roots
- `!` package-area lookup inside single-crate and multi-package workspaces
- `VAULT` parsing with one and multiple roots
- recursive search deterministic ordering
- recursive search ignores symlink loops
- interpolation with present and missing environment variables
- `resolve_relative()` across sibling directories and parent traversals

### Fixture Expectations

Tests for `!` should include at least:

- a single-package repo
- a workspace with package areas like `biscuit-file/lib` and `biscuit-file/cli`
- a workspace with both nested package areas (`homelab/server`) and top-level members (`tabby`)

## Documentation Updates Required During Implementation

When the code is added, the same change should also update:

- `biscuit-file/lib/README.md`
- `biscuit-file/docs/file-resolution.md`
- rustdoc examples on `FileReference`

The functional spec currently contains a few typos and ambiguities; those should be normalized to the behavior defined here so the public documentation matches the implementation exactly.

## Deferred Enhancements

The following are intentionally left for later iterations:

- Obsidian wikilink parsing such as `vault:[[Note Name]]`
- frontmatter alias resolution within vaults
- explicit `resolve_in(context)` public API for advanced callers
- configurable recursive match policies beyond "first deterministic match"
- optional canonicalization mode

## Summary

`FileReference` should be implemented as a small parsed reference plus runtime resolution engine. Parsing happens once, environment-dependent resolution happens later, and every scope resolves through a deterministic ordered search strategy. The resulting design keeps the user-facing API simple while making the complex parts, especially `@`, `!`, `%`, and interpolation, precise enough to implement and test confidently.
