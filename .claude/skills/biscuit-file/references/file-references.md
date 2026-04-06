## File Reference Resolution

Resolves compact descriptors to absolute paths lazily, with configurable search
roots and vault roots. For the full treatment, see
[the topic doc](../docs/topics/file-references.md).

### Reference Kinds

| Prefix                | Kind         | Resolves against                                         |
|-----------------------|--------------|----------------------------------------------------------|
| _(none)_ or `./`     | **Relative** | Current working directory                                |
| `/`                   | **Absolute** | Used verbatim                                            |
| `@`                   | **Magic**    | Prepended paths → git root → HOME → appended paths      |
| `!`                   | **Package**  | Cargo workspace package area (git root fallback)         |
| `vault:` or `vault::` | **Vault**   | Configured vault roots + `$VAULT` env var                |

Any kind can be prefixed with `%` for recursive directory search, and any
segment can contain `{{VAR_NAME}}` environment variable interpolation.

### Syntax Examples

```text
README.md                   # Relative: <CWD>/README.md
./src/main.rs               # Relative: <CWD>/src/main.rs
/etc/config.toml            # Absolute: used as-is
@docs/spec.md               # Magic: search git root, HOME, custom paths
@.bashrc                    # Magic: finds in HOME if not in repo
!README.md                  # Package: <package_area>/README.md
!docs/spec.md               # Package: <package_area>/docs/spec.md
vault:notes/today.md        # Vault: search configured vault roots
%@README.md                 # Recursive: walk git root + HOME for "README.md"
%./config.toml              # Recursive: walk CWD tree for "config.toml"
{{PROJECT}}/docs/spec.md    # Env var interpolation
```

### Magic Search Order (`@`)

1. Prepended paths (`.add_magic_path(path, PathPosition::Start)`)
2. Git repository root (via `git2`)
3. `$HOME` directory
4. Appended paths (`.add_magic_path(path, PathPosition::End)`)

Use `.with_package_area_magic_path()` in monorepos to prepend the current Cargo
package area before git root. No-op outside a workspace.

### Package Area Detection (`!`)

Finds the first path component of the Cargo workspace member containing CWD.
Example: CWD `/repo/biscuit-file/lib/src/` → package area `/repo/biscuit-file/`.
Falls back to git root if no workspace member matches. Returns `Ok(None)` if no
git repo found.

### Recursive Search (`%`)

Walks directory trees from each search root. Matches on filename; if subdirs
present in reference (e.g., `%docs/spec.md`), parent path must end with them.
Matches sorted lexicographically, first returned. Does not follow symlinks.

### API

```rust
use biscuit_file::{FileReference, PathPosition};

// Relative
let f = FileReference::new("README.md")?;

// Absolute
let f = FileReference::new("/etc/config.toml")?;

// Magic with custom paths
let f = FileReference::new("@docs/spec.md")?
    .add_magic_path("/opt/configs", PathPosition::Start)
    .add_magic_path("/etc/defaults", PathPosition::End);

// Package-area-aware magic (monorepo)
let f = FileReference::new("@prompts/commit.md")?
    .with_package_area_magic_path();

// Vault
let f = FileReference::new("vault:notes/today.md")?
    .add_vault("/personal/vault")
    .add_vault("/shared/vault");

// Resolution
let path = f.resolve()?;                           // absolute, uses ambient CWD
let path = f.resolve_from(Path::new("/repo/docs"))?;  // override CWD (document-relative)
let path = f.resolve_relative(None)?;              // relative to CWD
```

### `resolve_from()` -- Document-Relative Resolution

Overrides ambient CWD for relative, `@`, and `!` lookups. Use when a reference
appears inside a document and should resolve relative to that document's
location. HOME and env vars still read from live process state.

### Error Handling

```rust
// FileReferenceError variants:
// - InvalidSyntax          -- malformed reference string
// - MissingEnvironmentVariable -- unset {{VAR}}
// - CurrentDirectory       -- cannot determine CWD
// - Git                    -- git2 error during repo discovery
// - Workspace              -- cargo_metadata error
// - VaultNotConfigured     -- vault: ref with no roots
// - RelativePath           -- cannot compute relative path
// - Io                     -- filesystem error
```
