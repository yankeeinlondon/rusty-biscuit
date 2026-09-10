# Magic Paths

Magic paths let you customize where darkmatter searches when resolving `@`-prefixed file references. By default, `@ref` resolves by searching the intrinsic roots — package, package area, git repository root, then `HOME`. Magic paths inject additional search roots before or after those intrinsic roots.

## Default Search Order

When darkmatter encounters an `@`-prefixed reference like `@config/settings.toml`, `biscuit_file::FileReference` searches these intrinsic roots in order:

1. **Package root** containing the reference base (when known)
2. **Package-area root** containing the reference base (when known)
3. **Git repository root** (discovered from the source document's location, not the ambient CWD)
4. **HOME directory** (`$HOME`)

The first directory containing a match wins. Each intrinsic root is searched exactly once; registering one again as a magic path does not add a second probe.

## Adding Custom Search Roots

Use `ComposeOptions::with_magic_path()` to insert custom directories into the search order:

```rust
use darkmatter::markdown::compose::{ComposeOptions, PathPosition};

let options = ComposeOptions::new()
    .with_source_file("docs/root.md")
    // Searched BEFORE the intrinsic roots
    .with_magic_path("/project/.claudine", PathPosition::Start)
    .with_magic_path("/home/user/.claudine", PathPosition::Start)
    // Searched AFTER the intrinsic roots
    .with_magic_path("/etc/defaults", PathPosition::End);
```

### `PathPosition::Start`

Paths added with `Start` are searched **before** the intrinsic roots. Multiple `Start` entries are searched in the order they were added.

### `PathPosition::End`

Paths added with `End` are searched **after** the intrinsic roots. These act as fallback locations.

### Resulting Search Order

With the configuration above, a reference like `@skills/SKILL.md` would search:

1. `/project/.claudine/skills/SKILL.md`
2. `/home/user/.claudine/skills/SKILL.md`
3. `<package>/skills/SKILL.md`
4. `<package-area>/skills/SKILL.md`
5. `<git-root>/skills/SKILL.md`
6. `$HOME/skills/SKILL.md`
7. `/etc/defaults/skills/SKILL.md`

## Where Magic Paths Apply

Magic paths are threaded through all four darkmatter subsystems that construct `FileReference`:

| Subsystem | File | Function |
|-----------|------|----------|
| Compose transclusion | `compose/transclusion/resolver.rs` | `resolve_path()` |
| Reference graph | `reference/graph.rs` | `resolve_local_target()` |
| Reference validation | `reference/validate.rs` | `validate_local_path()` |
| Cross-doc fragment validation | `reference/validate.rs` | `validate_cross_doc_fragment()` |

This ensures consistent `@` resolution regardless of whether you are composing documents, building dependency graphs, or validating references.

## Graph and Validation APIs

The graph and validation APIs receive magic paths through `ReferenceGraphOptions`, which wraps `ComposeOptions`:

```rust
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, PathPosition};
use darkmatter::markdown::reference::types::ReferenceGraphOptions;

let compose = ComposeOptions::new()
    .with_source_file("docs/root.md")
    .with_magic_path("/project/.claudine", PathPosition::Start);

let graph_options = ReferenceGraphOptions { compose };

let md = Markdown::new("# Doc\n\n[link](@/shared.md)");
let graph = md.reference_graph(graph_options)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Cache Behavior

Magic paths are included in the compose cache hash. Different magic path configurations produce different cache keys, so cached results from one configuration are never served to another.

## Use Case: Claudine

The primary motivation for magic paths is [claudine](../../claudine/), which needs `@` references to search claudine-specific directories before falling back to the git root:

```rust
use darkmatter::markdown::compose::{ComposeOptions, PathPosition};

let options = ComposeOptions::new()
    .with_source_file(&source.resolved_path)
    .with_magic_path(&project_claudine_dir, PathPosition::Start)  // .claudine/ in project
    .with_magic_path(&global_claudine_dir, PathPosition::Start);  // ~/.claudine/
```

This lets claudine skill files use `@skills/topic/SKILL.md` and have it resolve from the project's `.claudine/` directory first, then the user's global `~/.claudine/`, and finally fall back to the git root and HOME.
