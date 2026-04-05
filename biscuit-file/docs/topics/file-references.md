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
paths. They allow callers to express _where_ a file lives relative to
project structure, environment, or vault configuration without committing to an
absolute path at authoring time.

## Syntax

A file reference has the general form:

```text
[%]<prefix><path-with-optional-interpolation>
```

### Recursive Flag (`%`)

An optional leading `%` marks the reference as **recursive**. Instead of
checking exact file paths, a recursive search walks directory trees from each
search root, looking for a file whose name matches the final path component.
Subdirectory components in the reference act as a suffix filter on the
ancestor path.

```text
%@README.md       → search every directory under each root for "README.md"
%./config.toml    → search under CWD for "config.toml"
%vault:notes.md   → search all vault roots for "notes.md"
```

### Reference Kinds

| Prefix                | Kind         | Description                                                         |
|-----------------------|--------------|---------------------------------------------------------------------|
| _(none)_              | **Relative** | Resolved relative to the current working directory                  |
| `/`                   | **Absolute** | Used verbatim as an absolute path                                   |
| `@`                   | **Magic**    | Searches a configurable set of roots (git root, HOME, custom paths) |
| `!`                   | **Package**  | Resolved relative to the current Cargo package area or git root     |
| `vault:` or `vault::` | **Vault**    | Resolved against configured vault root directories                  |

### Environment Variable Interpolation

Any path segment can include `{{VAR_NAME}}` placeholders. Variable names must
match `[A-Z0-9_]+`. At resolution time the value is read from the process
environment; if the variable is unset, resolution fails with
`MissingEnvironmentVariable`.

```text
{{PROJECT_ROOT}}/docs/spec.md
vault:{{VAULT_NAME}}/notes.md
@configs/{{APP}}/settings.toml
```

## API

### `FileReference`

The primary public type. Construction parses the reference string
syntactically—no filesystem access occurs until `resolve()` is called.

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

| Method                           | Returns                                       | Description                                                     |
|----------------------------------|-----------------------------------------------|-----------------------------------------------------------------|
| `new(raw: &str)`                 | `Result<FileReference, FileReferenceError>`   | Parse a reference string                                        |
| `raw()`                          | `&str`                                        | The original reference string                                   |
| `add_magic_path(path, position)` | `Self`                                        | Add a search root for `@` references (builder pattern)          |
| `add_vault(path)`                | `Self`                                        | Add a vault root for `vault:` references (builder pattern)      |
| `resolve()`                      | `Result<Option<PathBuf>, FileReferenceError>` | Resolve to an absolute path                                     |
| `resolve_relative(base)`         | `Result<Option<PathBuf>, FileReferenceError>` | Resolve and return a path relative to `base` (or CWD if `None`) |

Both `add_magic_path` and `add_vault` consume and return `self`, enabling
chained builder usage.

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

1. **Recursive flag** — stripped if leading `%`
2. **Kind prefix** — determines the reference kind
3. **Path template** — a sequence of `Literal` and `EnvVar` segments

No filesystem or environment access occurs during parsing.

### Phase 2: Context Gathering

When `resolve()` is called, a `ResolutionContext` is built from live process
state:

- **CWD** — `std::env::current_dir()`
- **Home directory** — `$HOME` environment variable
- **Environment** — all process environment variables (for interpolation)

### Phase 3: Interpolation

Template segments are joined. `Literal` segments are appended verbatim;
`EnvVar` segments are replaced by the value of the corresponding environment
variable. If any variable is missing, resolution fails immediately.

### Phase 4: Candidate Building

For **non-recursive** references, a list of candidate absolute paths is
constructed by joining each search root with the interpolated path:

| Kind     | Search Roots                                                       |
|----------|--------------------------------------------------------------------|
| Relative | `[CWD]`                                                            |
| Absolute | `[interpolated path directly]`                                     |
| Magic    | `magic_paths.prepend` → `git_root` → `HOME` → `magic_paths.append` |
| Package  | `[package_area or git_root]`                                       |
| Vault    | `vault_roots` → `$VAULT` env var split paths                       |

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

## Feature Flag

File reference support is gated behind the `file-reference` feature, which is
enabled by default. It adds dependencies on `git2`, `cargo_metadata`, and
`walkdir`.

```toml
[dependencies]
biscuit-file = { version = "0.1", default-features = false, features = ["file-reference"] }
```

## Package Area Detection

The `!` (package) reference kind uses Cargo workspace metadata to find the
nearest workspace member containing the current working directory. The "package
area" is the first path component of that member relative to the workspace root.

For example, in the rusty-biscuit monorepo, if CWD is
`/repo/biscuit-file/lib/src/`, the package area resolves to
`/repo/biscuit-file/`.

If no workspace member matches (e.g., a single-crate repo), the git repository
root is used as a fallback. If no git repository is found, no candidates are
generated and resolution returns `Ok(None)`.

## Vault Resolution

Vault roots come from two sources:

1. **Explicitly configured** via `FileReference::add_vault()`
2. **Environment variable** `$VAULT` — split using the platform path separator

If neither source provides any roots and a `vault:` reference is used,
resolution fails with `VaultNotConfigured`.

Both `vault:` (single colon) and `vault::` (double colon) are accepted and
behave identically. The double-colon form exists for compatibility with other
systems that use `scheme::path` syntax.

## Relative Path Computation

`resolve_relative()` first performs a full resolution, then computes a relative
path from the given base (or CWD) to the resolved target. This uses a
pure-lexical algorithm that:

1. Normalizes both paths
2. Strips the common prefix
3. Adds `..` for each remaining base component
4. Appends remaining target components

If the paths share no common ancestor, an error is returned.
