## File Reference Resolution

 resolves compact descriptors (`@`, `!`, `vault:`, `%`, `{{ENV}}`) to absolute paths lazily

 with configurable search roots and and vault roots.

 The topic doc at `biscuit-file/docs/topics/file-references.md` has the authoritative reference.

 For a deeper treatment, see [the detailed topic doc](../docs/topics/file-references.md).

.

### Syntax

```
@path/to/file.md            # Magic: search repo root, HOME, custom paths
!path/to/file.md            # Package: Cargo workspace area or git root
vault:notes/today.md           # Vault: configured vault roots
%path/to/file.md              # Recursive: walk directories
{{ENV_VAR}}/rest/of/file.md   # Interpolate environment variable
/path/to/file.md              # Absolute: used verbatim
./foo.md                 # Relative to CWD
```

The `%` prefix makes a search **recursive** — walks directories instead of checking exact paths.

 Each `%` prefix can be used with any reference kind.

 Supports environment variable interpolation via `{{VAR_NAME}}` in any path segment.

 Variable names must match `[A-Z0-9_]+`.
 Multiple interpolations are expanded left-to right at right.

 Adjacent `Literal` and `EnvVar` segments.

 Empty strings and invalid variable names are rejected.

 The `@`, `!`, `vault:`, `%` all support interpolation.

 Interpolation happens during resolution, not parsing.

 `{{}}` (empty) or `{{invalid-name}}` (invalid chars) are rejected.

 Parses `%` from `./docs/spec.md` → `!@docs/spec.md`:
 `%vault:notes.md` → `vault:notes.md`
 etc See `biscuit-file/docs/topics/file-references.md` for the full topic document.

### API
```rust
use biscuit_file::{FileReference, PathPosition};

let file_ref = FileReference::new("@docs/spec.md")?;          // parse only
let path = file_ref.resolve()?;                   // resolve (absolute path)
let path = file_ref.resolve_relative(None)?;    // relative to CWD

let file_ref = FileReference::new("vault:notes/today.md")?
    .add_vault("/path/to/vault")
    .add_magic_path("/extra", PathPosition::Start);
```
### Error Handling
```rust
// FileReferenceError variants:
// - InvalidSyntax -- malformed reference string
// - MissingEnvironmentVariable -- unset env var
// - CurrentDirectory -- cannot determine CWD
// - Git -- git2 error during repo discovery
// - Workspace -- cargo_metadata error
// - VaultNotConfigured -- vault: ref with no roots
// - RelativePath -- cannot compute relative path
// - Io -- filesystem error
