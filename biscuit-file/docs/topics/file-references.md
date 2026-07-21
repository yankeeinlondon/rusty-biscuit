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

Construction (`FileReference::new()`) is purely syntactic -- it does not read
the filesystem, environment, or process working directory. State is captured
separately when a `FileResolutionContext` is created or when an ambient
compatibility resolver is called.

## Quick Reference

| Prefix                | Kind                  | Resolves against                                         | Example                      |
|-----------------------|-----------------------|----------------------------------------------------------|------------------------------|
| `./` or `../`         | **Explicit Relative** | Current working directory (or `base`); no fallback       | `./src/main.rs`, `../a.md`   |
| _(none)_              | **Implicit Relative** | Git repository root, then CWD (or `base`)                | `README.md`, `docs/spec.md`  |
| `/`, drive, or UNC    | **Absolute**          | Used verbatim                                            | `/etc/config.toml`, `C:\\cfg.toml` |
| `~` or `~/`           | **Home**              | The user's home directory only (`~user` unsupported)     | `~/.config/app.toml`         |
| `@`                   | **Magic**             | Configurable search roots (git root, HOME, custom paths) | `@docs/spec.md`              |
| `!`                   | **Package**           | Cargo workspace package area (or git root fallback)      | `!README.md`                 |
| `vault:` or `vault::` | **Vault**             | Configured vault root directories                        | `vault:notes/today.md`       |
| `http://`, `https://` | **Remote URL**        | A typed remote target; never a local candidate           | `https://example.com/a.md`   |

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

## Absolute References

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
2. **Git repository root** -- discovered through `gix` on ambient paths, or
   supplied by an explicit context
3. **Home directory** -- from the cross-platform home provider
4. **Appended paths** -- added via `.add_magic_path(path, PathPosition::End)`

The first candidate that fallible metadata probing confirms as a regular file
wins. Missing and non-file candidates advance the search; other I/O failures
stop it with a typed error.

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

### Ambient Package Area Detection

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

- On the ambient compatibility path, if no git repository is found, no
  candidates are generated and resolution returns `Ok(None)`.

- With an explicit `FileResolutionContext`, the supplied package area is
  authoritative. If it is absent, the supplied repository root is the fallback;
  if both are absent, resolution reports `MissingPackageContext` without doing
  live discovery.

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

Recursive references use the same post-interpolation effective anchoring and
root order as direct references. Their diagnostics record each traversal root
with `ProbeDisposition::SearchRoot`. Directory traversal does not follow
symlinks; direct exact-path metadata probing does follow a final symlink to its
regular-file target.

## Environment Variable Interpolation

Any path segment can include `{{VAR_NAME}}` placeholders. Variable names must
match `[A-Z0-9_]+`. Ambient compatibility methods read a live environment
snapshot when called; explicit-context methods use the environment captured in
`FileResolutionContext`. If a variable is absent from that snapshot, resolution
fails with `MissingEnvironmentVariable`. Remote-target interpolation retains
unresolved placeholders verbatim; local-path resolution is the strict path
described here.

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
**after** one interpolation pass. This applies equally to direct and `%`
recursive references. An implicit `{{PROJECT_ROOT}}/docs/spec.md` whose
`PROJECT_ROOT` expands to an absolute path therefore resolves as an absolute
reference rather than silently joining the expanded value onto a search root.
The detailed resolver exposes both the authored kind (`class().kind`) and the
effective anchoring (`effective_kind()`) so the behavior is observable.

Interpolation may **not** inject a grammar sigil: a local reference whose
interpolated payload begins with `@`, `!`, `%`, `vault:`, or a case-insensitive
HTTP(S) URL scheme is rejected with `InvalidSyntax` rather than honored as that
kind. The rule also applies under recursive `%` resolution. Grammar sigils
remain author-controlled. Authored magic (`@`), package (`!`), vault, and URL
references keep their classification and interpolate within that grammar.

## API

### Choosing an entry point

Document-backed and request-scoped code should use an explicit
`FileResolutionContext`. The context is authoritative: candidate construction
does not reread CWD, HOME, environment variables, repository state, package
metadata, or configured roots.

| Entry point | State model | Outcome shape | Intended use |
|-------------|-------------|---------------|--------------|
| `resolve_in_context(&ctx)` | Explicit, authoritative context | `Result<Option<PathBuf>, FileReferenceError>` | Convenience projection for request-scoped execution |
| `resolve_detailed(&ctx)` | Explicit, authoritative context | `DetailedResolution` | Diagnostics that must retain candidates, dispositions, provenance, and failures |
| `candidate_plan(&ctx)` | Explicit, authoritative context | `Result<Vec<ResolutionCandidate>, _>` | Inspect the complete ordered plan without probing |
| `complete_partial_in_context(token, &ctx)` | Explicit, authoritative context | `Result<Option<PartialCompletion>, _>` | Completion that must agree with execution |
| `resolve()` | Live ambient state | `Result<Option<PathBuf>, FileReferenceError>` | Compatibility and simple top-level calls |
| `resolve_from(base)` | Explicit base plus other live ambient state | `Result<Option<PathBuf>, FileReferenceError>` | Compatibility for document-relative callers not yet carrying a request context |
| `complete_partial(token, base)` | Explicit base plus live discovery | `Result<Option<PartialCompletion>, _>` | Compatibility completion |

`resolve_relative()` and the `url`-gated `resolve_target()` are also ambient
compatibility operations. The explicit methods use magic and vault roots stored
on the context; roots added to a `FileReference` with `add_magic_path()` or
`add_vault()` apply only to the ambient `resolve()`/`resolve_from()` path.

### Capturing and deriving `FileResolutionContext`

`FileResolutionContext::new(base_dir)` captures the process environment and
cross-platform home directory once. The caller supplies the request's trusted
repository root, package area, and configured magic/vault roots. The base is a
directory and should already be absolute; `biscuit-file` deliberately leaves
trusted repository and package discovery to the caller.

After capture, use `for_source()` whenever an in-repository nested file becomes
the author of more references. It changes `source_path` and `base_dir` (to the
source's parent) while preserving the captured snapshot. Use `for_base()` for
an in-memory document with a new in-repository authoring directory. Both normal
derivations enforce containment for the request boundary and their new base.

A document already accepted through a configured external home, magic, or
vault root crosses a different trust boundary. Derive it with
`for_trusted_external_source()` or `for_trusted_external_base()`. These methods
allow the new authoring base outside the repository but still validate the
original request boundary. Neither normal nor trusted-external derivation reads
ambient state or performs discovery.

```rust,no_run
use std::collections::HashMap;
use std::path::PathBuf;
use biscuit_file::{FileReference, FileResolutionContext, PathPosition};

let launch_dir = PathBuf::from("/work/repo");
let repo_root = launch_dir.clone();
let package_area = repo_root.join("biscuit-file");
let env = HashMap::from([("DOCS_DIR".to_string(), "docs".to_string())]);

// Capture request-wide inputs once.
let request = FileResolutionContext::new(&launch_dir)
    .with_repository_root(&repo_root)
    .with_package_area(&package_area)
    .with_env(env)
    .add_magic_path(repo_root.join("prompts"), PathPosition::Start)
    .add_vault(repo_root.join("notes"));

// A file-backed document becomes the authoring source.
let document = request.for_source(repo_root.join("docs/guide.md"));
let resolved = FileReference::new("./images/diagram.png")?
    .resolve_in_context(&document)?;

// A nested document gets its own base without recapturing request state.
let nested = document.for_source(repo_root.join("includes/chapter.md"));
assert_eq!(nested.base_dir(), repo_root.join("includes"));
# let _ = resolved;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

`with_source_path()` records source provenance but does not change `base_dir`;
use a derivation method when both must move together. `validate()` checks that
the caller-supplied repository root lexically contains the initial request base
and every normal derived base, without canonicalizing symlinks. A trusted-
external derivation exempts only its current authoring base.

Important context methods are:

| Method | Purpose |
|--------|---------|
| `new(base_dir)` | Capture environment/home and establish the request's initial base |
| `for_source(source_path)` | Derive a child whose base is `source_path.parent()` |
| `for_base(base_dir)` | Derive a child base with no source path |
| `for_trusted_external_source(source_path)` | Derive a file-backed child across an explicitly accepted external trust root |
| `for_trusted_external_base(base_dir)` | Derive an in-memory child across an explicitly accepted external trust root |
| `with_repository_root(root)` | Supply the trusted worktree root |
| `with_package_area(area)` | Supply the authoritative package-area root |
| `with_home_dir(home)` / `without_home_dir()` | Override or explicitly clear captured home context |
| `with_env(env)` | Replace the captured interpolation/`VAULT` environment |
| `add_magic_path(path, position)` | Add an authoritative context magic root |
| `add_vault(path)` | Add an authoritative context vault root |
| `source_path()`, `base_dir()`, `repository_root()`, `package_area()`, `home_dir()`, `env()` | Inspect captured inputs |
| `validate()` | Validate request and authoring-base repository containment |

### `FileReference` methods

| Method | Description |
|--------|-------------|
| `new(raw)` | Parse a reference without reading ambient state |
| `raw()` | Return the authored string |
| `class()` | Return `FileReferenceClass { kind, recursive }` without re-parsing prefixes |
| `add_magic_path(path, position)` | Add an ambient-path magic root |
| `with_package_area_magic_path()` | Ambiently discover and prepend the current package-area magic root |
| `add_vault(path)` | Add an ambient-path vault root |
| `resolve()` / `resolve_from(base)` | Resolve through the ambient compatibility APIs |
| `resolve_in_context(ctx)` | Resolve through the explicit API, mapping no-match to `Ok(None)` |
| `resolve_detailed(ctx)` | Preserve the detailed success or failure record |
| `candidate_plan(ctx)` | Build the complete ordered plan without filesystem probes |
| `complete_partial(token, base)` | Expand an ambient completion token |
| `complete_partial_in_context(token, ctx)` | Expand a completion token from the same explicit roots as execution |
| `resolve_relative(base)` | Resolve ambiently and return a lexical relative path |
| `resolve_target()` | With `url`, distinguish `Resolved::Local` from `Resolved::Remote` |

All builder methods consume and return `self`, enabling chained use.
`PathPosition::Start` inserts a magic root before default roots;
`PathPosition::End` inserts one after them.

### Detailed resolution model

`resolve_detailed()` never returns `Err`. It returns a `DetailedResolution`
whose `outcome()` is either `DetailedOutcome::Matched(path)` or
`DetailedOutcome::Failed(failure)`. Its accessors retain:

- `raw()` and `class()` for authored intent;
- `effective_kind()` for post-interpolation anchoring;
- `base_dir()`, optional `source_path()`, and optional `repository_root()`;
- `candidates()` for the ordered candidates actually probed before resolution
  stopped, each as a `ProbedCandidate`;
- `error()` for the underlying `FileReferenceError` (present for failures other
  than `NoMatch`, and absent for successful matches); and
- `matched_path()` for the successful path.

`candidate_plan()` is distinct from `candidates()`: it returns the full ordered,
unprobed plan. A detailed result contains attempted candidates only, so later
planned candidates are absent after the first match or a terminal I/O failure.
`into_convenience()` performs the legacy projection: match becomes
`Ok(Some(path))`, `NoMatch` becomes `Ok(None)`, and every other failure becomes
`Err(error)`.

`ResolutionFailure` is the stable diagnostic classification and must not be
inferred from the reference kind:

| Variant | Meaning |
|---------|---------|
| `InvalidReference` | Syntax or effective-anchoring invariant is invalid |
| `MissingContext` | A required environment, home, vault, repository, or package input is unavailable |
| `NoMatch` | The complete applicable search found no regular file |
| `Io` | CWD access or a candidate metadata probe failed |
| `UnsupportedRemote` | A remote reference was sent through local-path resolution |

Every `ResolutionCandidate` exposes `path()` and `provenance()`. The
`RootProvenance` vocabulary is `Repository`, `Source`, `Package`, `Home`,
`Magic`, `Vault`, and `Absolute`. Every attempted `ProbedCandidate` adds one
`ProbeDisposition`:

| Disposition | Meaning |
|-------------|---------|
| `Missing` | Metadata returned `NotFound`; continue |
| `NonFile` | The path exists but is not a regular file; continue |
| `Matched` | The path is a regular file, or a symlink whose target is one; stop successfully |
| `Io(ErrorKind)` | Metadata returned another I/O error; stop with a typed source |
| `SearchRoot` | A recursive traversal root, not a direct candidate probe |

### Completion and execution parity

`complete_partial_in_context()` supports magic (`@`) and implicit-relative
tokens. Other entry forms return `Ok(None)` rather than being reinterpreted. A
`PartialCompletion` exposes `entry_form()`, ordered `roots()`, the
`active_segment()`, and the `rendered_prefix()` a completion consumer uses to
construct an emitted token.

With one shared `FileResolutionContext`, completion and execution consume the
same captured roots and precedence: implicit roots are repository then base;
magic roots are configured prepends, repository, home, then configured appends.
Duplicates are removed in first-seen order. A consumer that enumerates those
roots in order can pass its emitted value unchanged to `FileReference::new()`
and `resolve_in_context()` and get the file it displayed. The ambient
`complete_partial()` counterpart cannot see request-configured magic roots, so
request-scoped completion should always use `complete_partial_in_context()`.

### `FileReferenceError`

The complete current error vocabulary is:

| Variant | Trigger |
|---------|---------|
| `InvalidSyntax(message)` | Empty/malformed syntax, invalid interpolation, or an injected grammar sigil |
| `MissingEnvironmentVariable { name }` | `{{NAME}}` is absent from the selected environment snapshot |
| `CurrentDirectory(source)` | An ambient compatibility operation could not read CWD |
| `Git(source)` | Ambient repository discovery failed for a reason other than “not a repository” |
| `BareRepository` | Repository discovery found no working directory |
| `Workspace(source)` | Ambient Cargo workspace/package-area inspection failed |
| `VaultNotConfigured` | A vault reference has no explicit or captured `$VAULT` roots |
| `UnsupportedUserHome(raw)` | A non-portable `~user` reference was authored |
| `MissingHomeContext` | A home reference has no home directory in the explicit context |
| `MissingPackageContext` | A package reference has neither package-area nor repository anchor |
| `RepositoryRootNotContainingSource { repository_root, source_path }` | The request base or a normal derived authoring base fails lexical root containment |
| `RelativePath { from, to }` | `resolve_relative()` cannot produce the requested lexical relative path |
| `Io { path, source }` | A direct candidate metadata probe failed and records the candidate path |
| `RemoteNotLocal(raw)` | A remote URL was passed to local-path resolution |
| `InvalidUrl(message)` | With `url`, a remote target is malformed or has an unsupported scheme |

`FileReferenceError` is separate from fetch-policy/network `FetchError`; the
latter belongs to the optional fetching API rather than local resolution.

## Resolution Algorithm

### Phase 1: Parse once

`FileReference::new()` records the `%` modifier, authored kind, and literal or
environment-variable template segments. Detection order is HTTP(S) URL
(ASCII-case-insensitive) → `vault::` → `vault:` → `@` → `!` → `~` → absolute
(POSIX, Windows drive, or UNC) → explicit relative → implicit relative. No
filesystem, environment, or CWD access occurs.

### Phase 2: Select captured context

Explicit APIs consume the supplied `FileResolutionContext` as authoritative
data. Ambient compatibility APIs capture or discover the needed process state
when called. Repository discovery is performed at most once for a resolution
and only for kinds that use it; explicit, absolute, home, and vault references
do not trigger unnecessary repository discovery.

### Phase 3: Interpolate and determine effective anchoring

Template variables are expanded from the selected environment snapshot. For
the local anchoring family, the payload is then classified as absolute,
explicit-relative, or implicit-relative. This happens before both direct and
recursive candidate/root construction, and injected grammar sigils are rejected.

### Phase 4: Build an ordered plan

| Effective/authored kind | Direct candidate or recursive root order |
|-------------------------|------------------------------------------|
| Explicit relative | Source/base only |
| Implicit relative | Repository root, then source/base |
| Absolute | Authored absolute path only |
| Home | Home directory only |
| Magic | Configured prepends, repository, home, configured appends |
| Package | Package area, or repository fallback |
| Vault | Configured roots, then captured `VAULT` paths |
| Remote URL | No local candidates |

Paths are lexically deduplicated without changing first-seen order, and every
plan entry retains root provenance.

### Phase 5: Probe or traverse

Direct candidates are checked with fallible `std::fs::metadata`, not
`Path::is_file()`. `NotFound` records `Missing` and advances; existing
non-regular paths record `NonFile` and advance. Any other metadata error records
`Io(error.kind())`, stores `FileReferenceError::Io { path, source }`, and stops
immediately without probing later candidates. A regular file records `Matched`
and wins. Because `metadata` follows symlinks, a direct symlink to a regular
file can match.

Recursive resolution traverses the shared ordered roots without following
directory symlinks, applies the filename and optional parent-suffix filters,
sorts all matches lexically across roots, and selects the first. Its detailed
candidates are traversal roots marked `SearchRoot`, not direct file probes.

### Phase 6: Normalize the result

Resolved local paths are made absolute and `.`/`..` components are normalized
lexically without canonicalizing through symlinks.

## Relative Path Computation

`resolve_relative()` uses ambient resolution, then lexically normalizes the
resolved target and selected base, removes their common prefix, adds `..` for
remaining base components, and appends remaining target components. It reports
`RelativePath` when it cannot produce a relative path.

## Feature Flag

File reference support is gated behind the default `file-reference` feature.
It enables repository discovery, Cargo metadata, recursive traversal,
cross-platform home discovery, and URL classification.

```toml
[dependencies]
biscuit-file = { version = "0.1", default-features = false, features = ["file-reference"] }
```
