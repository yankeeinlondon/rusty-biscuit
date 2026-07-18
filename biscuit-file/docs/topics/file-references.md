---
blast_radius:
- biscuit-file/lib/src/file_reference/mod.rs
- biscuit-file/lib/src/file_reference/parse.rs
- biscuit-file/lib/src/file_reference/resolve.rs
- biscuit-file/lib/src/file_reference/context.rs
- biscuit-file/lib/src/file_reference/error.rs
- biscuit-file/lib/src/lib.rs
- biscuit-file/lib/Cargo.toml
---
# File References in `biscuit-file`

File references are compact string descriptors that resolve lazily to filesystem
paths. They let callers express _where_ a file lives relative to project
structure, environment, or vault configuration without committing to an absolute
path at authoring time.

Construction (`FileReference::new()`) is purely syntactic -- no filesystem
access occurs until `resolve()` is called.

## Quick Reference

| Prefix                | Kind                  | Resolves against                                         | Example                      |
|-----------------------|-----------------------|----------------------------------------------------------|------------------------------|
| `./` or `../`         | **Explicit Relative** | Current working directory (or `base`); no fallback       | `./src/main.rs`, `../a.md`   |
| _(none)_              | **Implicit Relative** | Git repository root, then CWD (or `base`)                | `README.md`, `docs/spec.md`  |
| `/`                   | **Absolute**          | Used verbatim                                            | `/etc/config.toml`           |
| `~` or `~/`           | **Home**              | The user's home directory only (`~user` unsupported)     | `~/.config/app.toml`         |
| `@`                   | **Magic**             | Configurable search roots (git root, HOME, custom paths) | `@docs/spec.md`              |
| `!`                   | **Package**           | Cargo workspace package area (or git root fallback)      | `!README.md`                 |
| `vault:` or `vault::` | **Vault**             | Configured vault root directories                        | `vault:notes/today.md`       |

Any reference can be prefixed with `%` to enable recursive directory search,
and any path segment can contain `{{VAR}}` environment variable interpolation.

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

A bare path with no recognized prefix is treated as *implicitly* relative. It is
first checked against the root of the enclosing git repository (when one is
present) and, if not found there, against the CWD (or the `base` passed to
`resolve_from`). Repository-shaped bare paths are the primary authoring form, so
the repository candidate takes precedence over the source-local one.

```text
foo.md              → <git_root>/foo.md, then <CWD>/foo.md
docs/spec.md        → <git_root>/docs/spec.md, then <CWD>/docs/spec.md
```

If the reference is not found in either location, `resolve()` returns
`Ok(None)`. If no git repository is discoverable, only the CWD is searched. When
the CWD *is* the git root, the two candidates collapse to a single one.

```rust,no_run
use biscuit_file::FileReference;

// From <repo>/biscuit-file/lib/src, resolves to <repo>/README.md
let file_ref = FileReference::new("README.md")?;
let path = file_ref.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

## Absolute References (`/`)

The path is used exactly as written with no search logic:

```text
/etc/config.toml    → /etc/config.toml
/tmp/output.json    → /tmp/output.json
```

```rust,no_run
use biscuit_file::FileReference;

let file_ref = FileReference::new("/etc/hosts")?;
let path = file_ref.resolve()?;        // checks /etc/hosts directly
# Ok::<(), biscuit_file::FileReferenceError>(())
```

## Home References (`~`)

`~` and `~/...` (plus the Windows `~\...` spelling) pin resolution to the
current user's home directory only -- there is no repository or search-root
fallback. Unlike a shell, `~user` expansion is **not** portable and is rejected
at parse time with `FileReferenceError::UnsupportedUserHome`.

```text
~                   → <home>
~/.config/app.toml  → <home>/.config/app.toml
```

```rust,no_run
use biscuit_file::FileReference;

let file_ref = FileReference::new("~/.bashrc")?;
let path = file_ref.resolve()?;        // checks <home>/.bashrc directly
# Ok::<(), biscuit_file::FileReferenceError>(())
```

The home directory is supplied through the resolution context; missing home
context is a typed missing-context failure rather than a silent no-match. Magic
(`@`) references also include HOME in their ordered search, but `~` is distinct:
it is home-pinned with no other candidate.

## Magic References (`@`)

Magic references search a prioritized list of root directories. This is the
most flexible kind -- useful for finding files that could live at the project
root, in your home directory, or in custom search paths.

### Default Search Order

1. **Prepended paths** -- added via `.add_magic_path(path, PathPosition::Start)`
2. **Git repository root** -- discovered via `git2::Repository::discover()`
3. **Home directory** -- from `$HOME` environment variable
4. **Appended paths** -- added via `.add_magic_path(path, PathPosition::End)`

The first path that exists as a file wins.

### Examples

```text
@docs/spec.md       → searches <git_root>/docs/spec.md, then ~/docs/spec.md
@.bashrc            → searches <git_root>/.bashrc, then ~/.bashrc
@config.toml        → searches <git_root>/config.toml, then ~/config.toml
```

```rust,no_run
use biscuit_file::{FileReference, PathPosition};

// Basic magic lookup: git root → HOME
let file_ref = FileReference::new("@docs/spec.md")?;
let path = file_ref.resolve()?;

// Custom search paths
let file_ref = FileReference::new("@config.toml")?
    .add_magic_path("/opt/configs", PathPosition::Start)   // searched first
    .add_magic_path("/etc/defaults", PathPosition::End);   // searched last
let path = file_ref.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

### Monorepo-Aware Magic with `with_package_area_magic_path()`

In a monorepo, you often want `@` to search your current package area _before_
the workspace root. The `with_package_area_magic_path()` builder method
automatically detects the Cargo workspace package area and prepends it to the
search order:

```rust,no_run
use biscuit_file::FileReference;

// If CWD is /repo/biscuit-file/lib/src/, this prepends /repo/biscuit-file/
let file_ref = FileReference::new("@prompts/commit.md")?
    .with_package_area_magic_path();
// Search order: <package_area> → <git_root> → HOME
let path = file_ref.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

This is a no-op if the current directory is not inside a Cargo workspace.

## Package References (`!`)

Package references resolve relative to the current Cargo workspace "package
area" -- the first path component of the workspace member containing the
working directory.

### Package Area Detection

1. Find the git repository root from CWD
2. Load `Cargo.toml` workspace metadata from that root
3. For each workspace member, extract its first path component (the "area")
4. Find which area contains the CWD
5. Resolve the reference relative to that area directory

For example, in the `rusty-biscuit` monorepo with CWD at
`/repo/biscuit-file/lib/src/`, the package area is `/repo/biscuit-file/`.

### Fallback Behavior

- If no workspace member matches (e.g., a single-crate repo), the **git root**
    is used instead.

- If no git repository is found, no candidates are generated and resolution
    returns `Ok(None)`.

### Examples

```text
!README.md          → <package_area>/README.md
!docs/spec.md       → <package_area>/docs/spec.md
!Cargo.toml         → <package_area>/Cargo.toml
```

```rust,no_run
use biscuit_file::FileReference;

// From within /repo/biscuit-file/lib/src:
let file_ref = FileReference::new("!README.md")?;
let path = file_ref.resolve()?;        // checks /repo/biscuit-file/README.md
# Ok::<(), biscuit_file::FileReferenceError>(())
```

This is particularly useful in monorepos where you want to reference files
belonging to the current package regardless of where you are within it.

## Vault References (`vault:` / `vault::`)

Vault references search configured vault root directories. This is designed for
personal knowledge bases, notes systems, or any collection of files stored in
well-known locations outside the project.

### Vault Root Sources

Vault roots are checked in this order:

1. **Explicitly configured** via `.add_vault()` (in order added)
2. **`$VAULT` environment variable** -- split using the platform path separator

Both `vault:` (single colon) and `vault::` (double colon) are accepted and
behave identically. The double-colon form exists for compatibility with systems
that use `scheme::path` syntax.

### Examples

```text
vault:notes/today.md     → <vault_root_1>/notes/today.md, then <vault_root_2>/...
vault::projects/plan.md  → same behavior as single-colon
```

```rust,no_run
use biscuit_file::FileReference;

let file_ref = FileReference::new("vault:notes/today.md")?
    .add_vault("/personal/vault")
    .add_vault("/shared/vault");
let path = file_ref.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

### Error: `VaultNotConfigured`

If neither `.add_vault()` nor `$VAULT` provides any roots, resolution fails
with `FileReferenceError::VaultNotConfigured`.

## Recursive Search (`%` prefix)

An optional leading `%` on any reference kind switches from exact-path checking
to recursive directory traversal. Instead of testing whether a specific path
exists, it walks directory trees from each search root looking for matching
files.

### How It Works

1. The same search roots as the underlying kind are used as traversal starting
   points (not join targets)

2. Every file under each root is checked against the **filename** (last path
   component)

3. If the reference includes subdirectory components (e.g., `%docs/spec.md`),
   the match is further filtered: the entry's parent path must end with those
   components

4. All matches are sorted lexicographically; the first is returned

### Examples

```text
%@README.md         → recursively search git root, HOME for any "README.md"
%./config.toml      → recursively search under CWD for "config.toml"
%vault:notes.md     → recursively search all vault roots for "notes.md"
%@docs/spec.md      → find "spec.md" where the parent path ends with "docs"
```

```rust,no_run
use biscuit_file::FileReference;

let file_ref = FileReference::new("%@README.md")?;
let path = file_ref.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

Recursive search does not follow symlinks.

## Environment Variable Interpolation

Any path segment can include `{{VAR_NAME}}` placeholders. Variable names must
match `[A-Z0-9_]+`. At resolution time the value is read from the process
environment; if the variable is unset, resolution fails with
`MissingEnvironmentVariable`.

```text
{{PROJECT_ROOT}}/docs/spec.md         → relative ref with interpolation
vault:{{VAULT_NAME}}/notes.md         → vault ref with interpolation
@configs/{{APP}}/settings.toml        → magic ref with interpolation
%vault:{{VAULT_NAME}}/notes.md        → recursive + vault + interpolation
```

Multiple interpolations in a single reference are supported and expanded
left-to-right. Empty variable names (`{{}}`) and invalid names
(`{{invalid-name}}`) are rejected at parse time with `InvalidSyntax`.

Interpolation happens during resolution, not parsing.

### Interpolation and filesystem anchoring

For the local anchoring family (explicit-relative, implicit-relative, and
absolute references), the *effective* anchoring is re-derived from the payload
**after** one interpolation pass. An implicit `{{PROJECT_ROOT}}/docs/spec.md`
whose `PROJECT_ROOT` expands to an absolute path therefore resolves as an
absolute reference rather than silently joining the expanded value onto a search
root. The detailed resolver exposes both the authored kind (`class().kind`) and
the effective anchoring (`effective_kind()`) so the behavior is observable.

Interpolation may **not** inject a grammar sigil: an environment value that
begins with `@`, `!`, `%`, `vault:`, or a URL scheme is rejected with
`InvalidSyntax` rather than honored as that kind. Grammar sigils remain
author-controlled. Magic (`@`), package (`!`), vault, and URL references keep
their authored classification and interpolate within their own root search.

## API

### `FileReference`

The primary public type. Construction parses the reference string
syntactically -- no filesystem access occurs until `resolve()` is called.

```rust,no_run
use biscuit_file::{FileReference, PathPosition};

let file_ref = FileReference::new("@docs/spec.md")?;
let resolved = file_ref.resolve()?;
if let Some(path) = resolved {
    println!("Found: {}", path.display());
}
# Ok::<(), biscuit_file::FileReferenceError>(())
```

#### Methods

| Method                           | Returns                                       | Description                                                          |
|----------------------------------|-----------------------------------------------|----------------------------------------------------------------------|
| `new(raw: &str)`                 | `Result<FileReference, FileReferenceError>`   | Parse a reference string                                             |
| `raw()`                          | `&str`                                        | The original reference string                                        |
| `add_magic_path(path, position)` | `Self`                                        | Add a search root for `@` references (builder pattern)               |
| `with_package_area_magic_path()` | `Self`                                        | Prepend Cargo package area to `@` search roots (builder pattern)     |
| `add_vault(path)`                | `Self`                                        | Add a vault root for `vault:` references (builder pattern)           |
| `resolve()`                      | `Result<Option<PathBuf>, FileReferenceError>` | Resolve to an absolute path using ambient CWD                        |
| `resolve_from(base)`             | `Result<Option<PathBuf>, FileReferenceError>` | Resolve using `base` as the working directory instead of ambient CWD |
| `resolve_relative(base)`         | `Result<Option<PathBuf>, FileReferenceError>` | Resolve and return a path relative to `base` (or CWD if `None`)      |

All builder methods consume and return `self`, enabling chained usage.

### `resolve_from()` -- Document-Relative Resolution

When a file reference appears inside a document, it should usually resolve
relative to _that document's location_, not wherever the process happens to be
running. `resolve_from(base)` treats `base` as the working directory for
explicit-relative, implicit-relative, `@`, and `!` lookups. `base` is a
directory: for a file-backed source, pass the source file's parent. Implicit
references still search the enclosing git repository root of `base` first, then
`base`; explicit `./`/`../` references use `base` only.

```rust,no_run
use std::path::Path;
use biscuit_file::FileReference;

// Reference found inside /repo/docs/guide.md
let file_ref = FileReference::new("./images/diagram.png")?;
let path = file_ref.resolve_from(Path::new("/repo/docs"))?;
// Checks /repo/docs/images/diagram.png (not <CWD>/images/diagram.png)
# Ok::<(), biscuit_file::FileReferenceError>(())
```

HOME and environment variables are still read from the live process state.

### `PathPosition`

Controls where custom magic paths are inserted relative to the default search
roots.

| Variant | Behavior                                        |
|---------|-------------------------------------------------|
| `Start` | Prepend before default roots (highest priority) |
| `End`   | Append after default roots (lowest priority)    |

### `FileReferenceError`

All errors produced by the file reference subsystem.

| Variant                               | Trigger                                                             |
|---------------------------------------|---------------------------------------------------------------------|
| `InvalidSyntax(msg)`                  | Malformed reference string (empty, unclosed `{{`, invalid var name) |
| `MissingEnvironmentVariable { name }` | An interpolated env var is not set                                  |
| `CurrentDirectory(source)`            | Could not determine CWD                                             |
| `Git(msg)`                            | git2 error while discovering repository root                        |
| `Workspace(msg)`                      | cargo_metadata error while inspecting workspace                     |
| `VaultNotConfigured`                  | `vault:` reference used with no vault roots configured              |
| `RelativePath { from, to }`           | Cannot compute a relative path between two locations                |
| `Io { path, source }`                 | Filesystem error during resolution                                  |

## Resolution Algorithm

### Phase 1: Parsing

Purely syntactic. The raw string is decomposed into:

1. **Recursive flag** -- stripped if leading `%`
2. **Kind prefix** -- determines the reference kind
3. **Path template** -- a sequence of `Literal` and `EnvVar` segments

Detection order: URL scheme (`http(s)://`) > `vault::` > `vault:` > `@` > `!` >
`~` > absolute (`/`, Windows drive/UNC) > explicit relative (`./`, `../`) >
implicit relative (default).

No filesystem or environment access occurs during parsing.

### Phase 2: Context Gathering

When `resolve()` is called, a `ResolutionContext` is built from live process
state:

- **CWD** -- `std::env::current_dir()` (or `base` if using `resolve_from()`)
- **Home directory** -- `$HOME` environment variable
- **Environment** -- all process environment variables (for interpolation)

### Phase 3: Interpolation

Template segments are joined. `Literal` segments are appended verbatim;
`EnvVar` segments are replaced by the value of the corresponding environment
variable. If any variable is missing, resolution fails immediately.

### Phase 4: Candidate Building

For **non-recursive** references, a list of candidate absolute paths is
constructed by joining each search root with the interpolated path:

| Kind              | Search Roots                                                       |
|-------------------|--------------------------------------------------------------------|
| Explicit Relative | `[CWD]` (no fallback)                                              |
| Implicit Relative | `[git_root, CWD]` (CWD omitted when equal to git_root; git_root omitted when absent) |
| Absolute          | `[interpolated path directly]`                                     |
| Home              | `[home_dir]` (no fallback; `~user` rejected at parse time)         |
| Magic             | `magic_paths.prepend` → `git_root` → `HOME` → `magic_paths.append` |
| Package           | `[package_area or git_root]`                                       |
| Vault             | `vault_roots` → `$VAULT` env var split paths                       |

For **recursive** references, the same search roots are used as traversal
starting points rather than join targets.

### Phase 5: Matching

**Non-recursive**: Each candidate path is checked with `is_file()`. The first
match is returned, normalized to an absolute path.

**Recursive**: Each root directory is walked (non-following symlinks). Files
whose name matches the final component of the interpolated path are collected.
If subdirectory components exist in the reference, the entry's relative path
must end with those components. Matches are sorted lexicographically and the
first is returned.

### Phase 6: Normalization

Resolved paths are normalized by resolving `.` and `..` components
lexicographically (without touching the filesystem). Relative paths are joined
with CWD before normalization.

## Relative Path Computation

`resolve_relative()` first performs a full resolution, then computes a relative
path from the given base (or CWD) to the resolved target. This uses a
pure-lexical algorithm that:

1. Normalizes both paths
2. Strips the common prefix
3. Adds `..` for each remaining base component
4. Appends remaining target components

If the paths share no common ancestor, an error is returned.

## Feature Flag

File reference support is gated behind the `file-reference` feature, which is
enabled by default. It adds dependencies on `git2`, `cargo_metadata`, and
`walkdir`.

```toml
[dependencies]
biscuit-file = { version = "0.1", default-features = false, features = ["file-reference"] }
```
