# Magic Paths

Magic paths let you customize where darkmatter searches when resolving `@`-prefixed file references. By default, `@ref` searches the local tree — package, package area, then the git repository root (or, outside a repository, the request directory) — before `HOME`. Magic paths inject additional search roots into that order.

## Default Search Order

When darkmatter encounters an `@`-prefixed reference like `@config/settings.toml`, `biscuit_file::FileReference` searches these intrinsic roots in order:

1. **Package root** of the request directory (when known)
2. **Package-area root** of the request directory (when known)
3. **Local root** — the git repository root, or the request directory itself when there is no repository
4. **HOME directory** (`$HOME`)

The first directory containing a match wins. Each root is searched exactly once; registering an intrinsic root again as a magic path does not add a second probe.

These anchors come from the request's **launch `@` scope**, not from the document being composed. Transcluded and nested documents keep the scope of the request, so an `@` reference inside `includes/chapter.md` searches the same tree as one in the root document, while `./`, bare, `&`, and `^` references stay relative to the document that wrote them. For `md compose` the request directory is the directory you ran `md` in. If the input document lies outside that directory's repository, `md` builds the context from the document's own repository instead. A library caller supplies the scope with `ComposeOptions::with_file_resolution_context()`; without one, compose captures it at the root document's directory. See [the launch `@` scope](../../../biscuit-file/docs/topics/file-references.md#the-launch--scope) for the full contract.

## Adding Custom Search Roots

Use `ComposeOptions::with_magic_path()` to insert custom directories into the search order:

```rust
use darkmatter::markdown::compose::{ComposeOptions, PathPosition};

// Launched from /project (a git repository)
let options = ComposeOptions::new()
    .with_source_file("docs/root.md")
    // Inside the local root: local tier, before the intrinsic local roots
    .with_magic_path("/project/.claudine", PathPosition::Start)
    // Outside the local root: user tier, after every local root, before HOME
    .with_magic_path("/home/user/.claudine", PathPosition::Start)
    // Outside the local root: user tier, after HOME
    .with_magic_path("/etc/defaults", PathPosition::End);
```

### Tiers

Every root belongs to a tier, and the **local tier is always searched before the user tier**. A magic path that lies inside the local root (the repository root, or the request directory when there is none) is local; every other path — including one under `$HOME` and one such as `/etc/defaults` — is user tier. No position can move a user-tier path ahead of a local file.

`with_magic_path` always infers the tier from the path. When the local root *is* `$HOME`, inference cannot tell a user convention from a local one; a caller that needs the explicit `MagicPathTier::User` override registers the root with `add_magic_path_with_tier` on the `FileResolutionContext` it passes to `with_file_resolution_context()`.

### `PathPosition::Start`

Paths added with `Start` are searched **before** their tier's intrinsic roots (package, package area, and local root for the local tier; `HOME` for the user tier). Multiple `Start` entries are searched in the order they were added.

### `PathPosition::End`

Paths added with `End` are searched **after** their tier's intrinsic roots. These act as fallback locations within the tier.

### Resulting Search Order

With the configuration above, a reference like `@skills/SKILL.md` would search:

1. `/project/.claudine/skills/SKILL.md`
2. `<package>/skills/SKILL.md`
3. `<package-area>/skills/SKILL.md`
4. `/project/skills/SKILL.md`
5. `/home/user/.claudine/skills/SKILL.md`
6. `/home/user/skills/SKILL.md` (`$HOME`)
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

Magic paths are included in the compose cache hash, together with the launch `@` scope and each supplied context's tier-aware magic-root registrations. Any of these can change which file an `@` reference picks, so different configurations produce different cache keys and cached results from one are never served to another.

## Use Case: Claudine

The primary motivation for magic paths is [claudine](../../claudine/), which needs `@` references to reach Claudine-specific prompt directories as well as the repository. Claudine registers its conventions on the `FileResolutionContext` it hands to darkmatter, not through `with_magic_path`, because its `~/.claudine` roots need the explicit user-tier override:

```rust
use biscuit_file::{FileResolutionContext, MagicPathTier, PathPosition};
use darkmatter::markdown::compose::ComposeOptions;

let context = FileResolutionContext::new(&launch_dir)
    .with_repository_root(&repo_root)
    .add_magic_path(repo_root.join(".claudine/prompts"), PathPosition::Start)
    .add_magic_path_with_tier(home.join(".claudine/prompts"), PathPosition::Start, MagicPathTier::User);
let options = ComposeOptions::new()
    .with_source_file(&source.resolved_path)
    .with_file_resolution_context(context);
```

So `@review.md` resolves from the project's `.claudine/prompts/` and the rest of the local tree first, and only then from the user's `~/.claudine/prompts/` and `HOME`. Claudine's full registration and order are documented in [shell completions](../../../claudine/docs/topics/completions/shell-completions.md#scopes).
