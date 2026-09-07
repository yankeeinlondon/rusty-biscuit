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

A **file reference** is a compact string — `README.md`, `@docs/spec.md`,
`&Cargo.toml`, `~/.config/app.toml` — that describes *where a file lives
relative to project structure* instead of committing to an absolute path at
authoring time. You parse the string once, then resolve it against captured
state to get a real path:

```rust,no_run
use biscuit_file::FileReference;

// "@" means: search the well-known roots (git repo root, then HOME, plus
// any roots you configure). Parsing reads nothing from the environment.
let spec = FileReference::new("@docs/spec.md")?;

// Resolution probes the filesystem. Some(path) = found; None = clean miss.
if let Some(path) = spec.resolve()? {
    println!("found: {}", path.display());
}
# Ok::<(), biscuit_file::FileReferenceError>(())
```

This document is the primary reference for the feature. It assumes you know
Rust but nothing about this library.

## The Five Ideas That Explain Everything Else

1. **Parsing is pure.** `FileReference::new()` only classifies the string. It
   never reads the filesystem, environment variables, or the process working
   directory. A `FileReference` is just "what the author wrote," typed.

2. **Resolution needs state, and you choose how to supply it.** There are two
   API families:
   - **Explicit context** (recommended for anything non-trivial): you build a
     [`FileResolutionContext`](#capturing-state-fileresolutioncontext) once,
     capturing the environment, home directory, repository root, and any
     configured search roots. Every later resolution reads *only* that
     snapshot — nothing ambient, ever. Deterministic and immune to
     `set_current_dir` surprises.
   - **Ambient convenience** (`resolve()`, `resolve_from(base)`): the library
     reads live process state (CWD, `$HOME`, git discovery) at call time.
     Fine for simple CLI tools and one-off lookups.

3. **Each reference kind has a fixed, closed candidate list.** The sigil
   (`@`, `&`, `^`, `~`, `./`, none, …) selects an ordered list of base directories.
   Resolution joins the path onto each base in order and takes the **first
   existing regular file**. There is no cross-kind fallback: a missed `./foo`
   is never retried as a magic path, a missed `@foo` never falls back to a
   bare-path search.

4. **Three outcomes, cleanly separated.**
   - `Ok(Some(path))` — a candidate matched.
   - `Ok(None)` — the reference is well-formed but nothing matched. Whether a
     miss is an error is *your* policy decision, not the library's.
   - `Err(FileReferenceError)` — the reference is malformed, or resolution
     required state that is genuinely unavailable (missing env var, no vault
     configured, no home directory, …).

5. **Anchors are supplied, not guessed.** In explicit-context mode the caller
   supplies a repository scope catalog;
   `biscuit-file` deliberately performs no trusted discovery of its own.
   ambient mode discovers only the repository root live as a compatibility convenience.

## Syntax Quick Reference

| Prefix                | Kind                  | Resolves against                                                | Example                            |
|-----------------------|-----------------------|-----------------------------------------------------------------|------------------------------------|
| `./` or `../`         | **Explicit relative** | The base directory only; no fallback                            | `./src/main.rs`, `../a.md`         |
| _(none)_              | **Implicit relative** | The base directory, then the git repository root                | `README.md`, `docs/spec.md`        |
| `/`, drive, or UNC    | **Absolute**          | Used verbatim                                                   | `/etc/config.toml`, `C:\\cfg.toml` |
| `~` or `~/`           | **Home**              | The user's home directory only (`~user` unsupported)            | `~/.config/app.toml`               |
| `@` or `@/`           | **Magic**             | Custom, package, package-area, repository, and HOME roots       | `@docs/spec.md`                    |
| `&` or `&/`           | **Repository root**   | The repository root only; repository-contained                  | `&README.md`                       |
| `^` or `^/`           | **Repository scoped** | Package, package area, then repository; repository-contained    | `^README.md`                       |
| `vault:` or `vault::` | **Vault**             | Configured vault root directories                               | `vault:notes/today.md`             |
| `http://`, `https://` | **Remote URL**        | A typed remote target; never a local candidate                  | `https://example.com/a.md`         |

Two modifiers compose with the kinds above:

- a leading `%` switches to [recursive directory search](#recursive-search-);
- any segment may contain [`{{VAR}}` environment interpolation](#environment-variable-interpolation).

**"Base directory"** above means: the process CWD in ambient mode, or the
context's `base_dir` in explicit-context mode (typically the directory of the
document that authored the reference).

The grammar reserves recognized introducers. A value beginning with `@`, `&`,
`^`, `%`, `~`, `vault:`, `http://`, or `https://` must satisfy that form's
grammar; malformed input is not reinterpreted as an implicit path. The removed
`!` sigil is also reserved and returns `InvalidSyntax` with a suggestion to use
`^`. Environment interpolation cannot inject a sigil into an otherwise
implicit reference.

### Which sigil should an author use?

| The file's identity is…                              | Write        |
|------------------------------------------------------|--------------|
| "belongs to this document" (moves with it)           | `./` / `../` |
| "try beside this document, then at the repository"   | bare path    |
| "belongs exactly at a repository path"               | `&`          |
| "belongs to the nearest repository sub-project"      | `^`          |
| "find it in the usual places"                        | `@`          |
| "belongs to this user"                               | `~`          |
| "lives in my notes/knowledge-base vault"             | `vault:`     |
| "is exactly this path"                               | absolute     |

Bare paths are base-first, so a document-local copy shadows the repository
copy. Use `&README.md` when the repository-root identity is required, or
`^README.md` for package → package-area → repository fallback.

## Reference Kinds

### Explicit Relative (`./`, `../`)

A leading `./` or `../` (or the `.\`/`..\` Windows spellings) pins the lookup
to the base directory. There is exactly one candidate and no fallback.

```text
./README.md         → <base>/README.md
../sibling/foo.md   → <base>/../sibling/foo.md   (normalized)
```

**Use it when** the file belongs to the authoring document and should move
with it — an image next to a markdown file, a fragment included by a template.

### Implicit Relative (bare path)

A bare path with no recognized prefix gets two candidates, in this order:

1. the **base directory**, then
2. the **root of the enclosing git repository** (when one is known).

```text
docs/spec.md        → <base>/docs/spec.md, then <git_root>/docs/spec.md
```

If no repository is known, the
base directory is the only candidate. When the base *is* the repository root,
the two candidates collapse into one. A miss returns `Ok(None)`.

```rust,no_run
use biscuit_file::FileReference;

// From <repo>/some/deep/dir, this tries the deep directory before the repo root.
let path = FileReference::new("README.md")?.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

**Use it when** the file has a stable repository-level address (`CLAUDE.md`,
`docs/architecture.md`, a root `justfile`).

### Absolute

The path is used exactly as written — one candidate, no search. POSIX
(`/etc/hosts`), Windows drive (`C:\cfg.toml`), and UNC paths are all
recognized.

```rust,no_run
use biscuit_file::FileReference;

let path = FileReference::new("/etc/hosts")?.resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

### Home (`~`)

`~` and `~/...` (plus the Windows `~\...` spelling) pin resolution to the
current user's home directory — one candidate, no repository or search-root
fallback. Unlike a shell, `~user` expansion is **not** portable and is
rejected at parse time with `FileReferenceError::UnsupportedUserHome`.

```text
~                   → <home>
~/.config/app.toml  → <home>/.config/app.toml
```

The home directory comes from the resolution context (or the cross-platform
home provider in ambient mode). If the explicit context has no home
directory, resolution fails with the typed `MissingHomeContext` — not a
silent miss.

Note the difference from magic references: `@` *includes* HOME in its search
list, but `~` is home-*pinned* with no other candidate.

### Magic (`@`)

Magic references search a prioritized list of root directories — the most
flexible kind, for files that could live at the project root, in your home
directory, or in application-defined convention directories.

`@docs/spec.md` and `@/docs/spec.md` are equivalent spellings: the grammar
consumes exactly one optional `/`, and the remaining payload must be
relative. Repeated POSIX separators and Windows drive-qualified, rooted, or
UNC payloads are rejected with `InvalidSyntax` rather than being allowed to
replace a configured magic root.

The search order is:

1. **Prepended roots** — added via `add_magic_path(path, PathPosition::Start)`,
   in registration order;
2. **the package root** selected for the reference base (when known);
3. **the package-area root** selected for the reference base (when known);
4. **the git repository root** (when known);
5. **the home directory** (when known);
6. **Appended roots** — added via `add_magic_path(path, PathPosition::End)`.

The first candidate confirmed to be a regular file wins. Missing and
non-file candidates advance the search; any other I/O failure stops it with a
typed error.

```text
@docs/spec.md       → <package>/docs/spec.md, <area>/docs/spec.md, <repo>/docs/spec.md, ~/docs/spec.md
@.bashrc            → <package>/.bashrc, <area>/.bashrc, <repo>/.bashrc, ~/.bashrc
```

```rust,no_run
use biscuit_file::{FileReference, PathPosition};

// Out of the box: package → package area → repository → HOME.
let path = FileReference::new("@docs/spec.md")?.resolve()?;

// With application convention directories:
let path = FileReference::new("@config.toml")?
    .add_magic_path("/opt/configs", PathPosition::Start)   // searched first
    .add_magic_path("/etc/defaults", PathPosition::End)    // searched last
    .resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

**Use it when** you want convention-over-configuration lookup — "find
`plan.md` wherever this application usually keeps prompts." Applications
embedding this library typically prepend their own convention roots so that
`@name.md` checks the nearest, most specific location first.

### Repository Root (`&`) and Repository Scoped (`^`)

`&path` has exactly one candidate: `<repository_root>/path`. `^path` searches
the package root containing the reference base, then its package-area root,
then the repository root. Missing levels and duplicate roots are skipped.
Both forms require a repository and reject lexical `..` escapes plus existing
symlink, junction, or reparse targets that resolve outside it.

Containment is checked twice: lexically after component normalization, then
canonically against an existing target or its deepest existing ancestor. This
closes ordinary symlink, junction, and reparse-point escapes at resolution
time, but it is still subject to filesystem time-of-check/time-of-use races. A
different process can replace a checked path before a later open. `biscuit-file`
is a resolver, not a filesystem sandbox; consumers needing adversarial
confinement must use operating-system sandboxing or handle-relative secure-open
primitives.

In a shell, `&` is a control operator. Quote repository-root references passed
as arguments or setter values, for example `spec='&docs/plan.md'`.

### Vault (`vault:` / `vault::`)

Vault references search configured vault roots — designed for personal
knowledge bases, notes systems, or any file collection kept in well-known
locations outside the project. Both spellings behave identically; the
double-colon form exists for compatibility with `scheme::path` syntaxes.

Roots are checked in this order:

1. explicitly configured roots, via `add_vault()`, in the order added;
2. the **`$VAULT` environment variable**, split on the platform path
   separator (`:` on Unix, `;` on Windows).

```rust,no_run
use biscuit_file::FileReference;

let path = FileReference::new("vault:notes/today.md")?
    .add_vault("/personal/vault")
    .add_vault("/shared/vault")
    .resolve()?;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

If neither `add_vault()` nor `$VAULT` provides any roots, resolution fails
with `FileReferenceError::VaultNotConfigured` — a configuration problem, not
a miss.

### Remote URLs (`http://`, `https://`)

A URL parses into a typed remote reference. It never becomes a local
candidate: sending it through local-path resolution fails with
`RemoteNotLocal`. With the `url` feature, `resolve_target()` distinguishes
`Resolved::Local` from `Resolved::Remote` so callers can route remote
references to the separate fetching API (whose failures are `FetchError`, a
different type from `FileReferenceError`).

## Modifiers

### Recursive Search (`%`)

A leading `%` on any reference kind switches from exact-path checking to
recursive directory traversal. The same roots the kind would normally *join*
against become traversal *starting points*:

1. every file under each root is checked against the reference's **filename**
   (last path component);
2. if the reference has directory components (`%docs/spec.md`), a match's
   parent path must additionally *end with* those components;
3. all matches across all roots are sorted lexicographically and the first is
   returned.

```text
%@README.md         → search git root, then HOME, recursively for "README.md"
%./config.toml      → search under the base directory for "config.toml"
%@docs/spec.md      → find any "spec.md" whose parent path ends with "docs"
%vault:notes.md     → search all vault roots for "notes.md"
```

Recursive references use the same post-interpolation anchoring and root order
as direct references. Traversal does **not** follow directory symlinks
(direct exact-path probing does follow a final symlink to its regular-file
target). In [detailed diagnostics](#diagnostics-resolve_detailed), each
traversal root is recorded with `ProbeDisposition::SearchRoot` rather than as
a direct candidate probe.

### Environment Variable Interpolation

Any path segment can include `{{VAR_NAME}}` placeholders. Names must match
`[A-Z0-9_]+`; empty (`{{}}`) or invalid (`{{invalid-name}}`) names are
rejected at **parse** time with `InvalidSyntax`. Expansion itself happens at
**resolution** time:

- explicit-context methods expand from the environment captured in the
  `FileResolutionContext`;
- ambient methods read a live environment snapshot when called;
- a variable absent from the selected snapshot fails with
  `MissingEnvironmentVariable`.

```text
{{PROJECT_ROOT}}/docs/spec.md
@configs/{{APP}}/settings.toml
%vault:{{VAULT_NAME}}/notes.md
```

Multiple placeholders expand left-to-right. Remote-target interpolation
retains unresolved placeholders verbatim; local-path resolution is the strict
form described here.

**Interpolation can change the anchoring — but never the grammar.** For the
local family (explicit-relative, implicit-relative, absolute), the *effective*
anchoring is re-derived from the payload after one interpolation pass, for
both direct and `%` recursive references. So an implicit
`{{PROJECT_ROOT}}/docs/spec.md` whose variable expands to an absolute path
resolves as an absolute reference instead of silently joining the expansion
onto a search root. The detailed resolver exposes both the authored kind
(`class().kind`) and the effective anchoring (`effective_kind()`), so this is
observable. What interpolation may **not** do is inject a sigil: a local
reference whose expanded payload begins with `@`, `&`, `^`, `%`, `vault:`, or a
case-insensitive HTTP(S) scheme is rejected with `InvalidSyntax` rather than
reinterpreted. Sigils remain author-controlled; authored `@`/`&`/`^`/vault/URL
references keep their classification and interpolate within it.

## Capturing State: `FileResolutionContext`

The explicit API's promise: **capture once, resolve many times, read nothing
ambient in between.**

```rust,no_run
use std::collections::HashMap;
use std::path::PathBuf;
use biscuit_file::{
    FileReference, FileResolutionContext, PackageAreaFallback, PathPosition,
    RepositoryScopeCatalog,
};

let launch_dir = PathBuf::from("/work/repo");
let repo_root = launch_dir.clone();
let package_area = repo_root.join("biscuit-file");
let package_root = package_area.join("lib");
let scopes = RepositoryScopeCatalog::new(
    repo_root.clone(),
    vec![package_area],
    vec![package_root],
    PackageAreaFallback::FirstComponent,
)?;
let env = HashMap::from([("DOCS_DIR".to_string(), "docs".to_string())]);

// Capture request-wide inputs once, at the start of the request.
let request = FileResolutionContext::new(&launch_dir)
    .with_repository_scope_catalog(scopes)
    .with_env(env)
    .add_magic_path(repo_root.join("prompts"), PathPosition::Start)
    .add_vault(repo_root.join("notes"));

// A file-backed document becomes the authoring source: its references
// resolve with base_dir = the document's parent directory, while every
// process-state inputs carry over while repository scopes are selected again
// from the catalog for the document's base.
let document = request.for_source(repo_root.join("docs/guide.md"));
let resolved = FileReference::new("./images/diagram.png")?
    .resolve_in_context(&document)?;

// Nested documents derive again — still no rediscovery.
let nested = document.for_source(repo_root.join("includes/chapter.md"));
assert_eq!(nested.base_dir(), repo_root.join("includes"));
# let _ = resolved;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

`FileResolutionContext::new(base_dir)` captures the process environment and
the cross-platform home directory once. Everything trusted — the repository
scope catalog plus magic and vault roots — is *supplied by you*; `biscuit-file`
deliberately leaves that discovery to the caller so the context is a pure
data snapshot. The base should be an absolute directory.

### Deriving contexts for documents

When a document inside the request becomes the author of further references,
derive a child context instead of building a new one:

- `for_source(source_path)` — records the source and sets `base_dir` to the
  source's parent. Use it for file-backed documents.
- `for_base(base_dir)` — new base, no source path. Use it for in-memory
  documents.
- `with_source_path(path)` records provenance *without* moving `base_dir`;
  use a derivation method when both must move together.

Derivations clone the captured snapshot and recompute repository, package-area,
and package anchors from the catalog for the new base. A trusted external base
outside the catalog clears those anchors. No derivation re-reads process state
or performs discovery.

### Trust boundaries and containment

When a repository root is supplied, `validate()` enforces that the original
request base and every normally-derived base lie **lexically inside** that
root (component-aware, after `.`/`..` normalization and Windows
verbatim-prefix reduction; symlinks are not canonicalized, so worktree
identity is preserved). Violation is the typed
`RepositoryRootNotContainingSource`. This is a trust check on the
caller-provided root, not a filesystem sandbox.

Some documents legitimately live *outside* the repository — one found through
a configured home, magic, or vault root (say, `~/.myapp/prompts/example.md`).
Derive those with `for_trusted_external_source()` / `for_trusted_external_base()`:
the new authoring base is exempt from containment, but the original request
boundary is still checked, so derivation can never launder an invalid request
snapshot into a valid one.

### Context method summary

| Method | Purpose |
|--------|---------|
| `new(base_dir)` | Capture environment/home once; establish the request's initial base |
| `for_source(source_path)` | Derive a child whose base is `source_path.parent()` |
| `for_base(base_dir)` | Derive a child base with no source path |
| `for_trusted_external_source(source_path)` | File-backed child across an accepted external trust root |
| `for_trusted_external_base(base_dir)` | In-memory child across an accepted external trust root |
| `with_repository_root(root)` | Supply the trusted worktree root |
| `with_repository_scope_catalog(catalog)` | Supply topology and select repository/package scopes for this base |
| `with_package_root(package)` | Supply a package root for compatibility callers without a catalog |
| `with_package_area(area)` | Supply the authoritative package-area root |
| `with_home_dir(home)` / `without_home_dir()` | Override or explicitly clear captured home |
| `with_env(env)` | Replace the captured interpolation/`VAULT` environment |
| `add_magic_path(path, position)` | Add an authoritative magic root |
| `add_vault(path)` | Add an authoritative vault root |
| `source_path()`, `base_dir()`, `repository_root()`, `package_root()`, `package_area()`, `home_dir()`, `env()` | Inspect captured inputs |
| `validate()` | Check repository containment of request and derived bases |

## Choosing an Entry Point

| Entry point | State model | Outcome shape | Intended use |
|-------------|-------------|---------------|--------------|
| `resolve_in_context(&ctx)` | Explicit, authoritative context | `Result<Option<PathBuf>, FileReferenceError>` | The normal request-scoped call |
| `resolve_detailed(&ctx)` | Explicit, authoritative context | `DetailedResolution` | Diagnostics needing candidates, dispositions, provenance |
| `candidate_plan(&ctx)` | Explicit, authoritative context | `Result<Vec<ResolutionCandidate>, _>` | Inspect the full ordered plan without probing |
| `complete_partial_in_context(token, &ctx)` | Explicit, authoritative context | `Result<Option<PartialCompletion>, _>` | Completion that must agree with execution |
| `resolve()` | Live ambient state | `Result<Option<PathBuf>, FileReferenceError>` | Simple top-level calls; compatibility |
| `resolve_from(base)` | Explicit base + live ambient state | `Result<Option<PathBuf>, FileReferenceError>` | Document-relative callers not yet carrying a context |
| `complete_partial(token, base)` | Explicit base + live discovery | `Result<Option<PartialCompletion>, _>` | Compatibility completion |

`resolve_relative()` and the `url`-gated `resolve_target()` are also ambient
compatibility operations.

**One easy-to-miss rule:** the explicit methods use the magic and vault roots
stored on the *context*. Roots added directly to a `FileReference` via its
own `add_magic_path()` / `add_vault()` builders apply **only** to the ambient
`resolve()` / `resolve_from()` path.

### `FileReference` method summary

| Method | Description |
|--------|-------------|
| `new(raw)` | Parse without reading ambient state |
| `raw()` | The authored string |
| `class()` | `FileReferenceClass { kind, recursive }` — branch on typed kind, not prefixes |
| `add_magic_path(path, position)` | Add an ambient-path magic root |
| `add_vault(path)` | Add an ambient-path vault root |
| `resolve()` / `resolve_from(base)` | Resolve through the ambient compatibility APIs |
| `resolve_in_context(ctx)` | Resolve through the explicit API; no-match maps to `Ok(None)` |
| `resolve_detailed(ctx)` | Keep the detailed success/failure record |
| `candidate_plan(ctx)` | Build the complete ordered plan, no filesystem probes |
| `candidate_plan_with_order(ctx, order)` | Build the unprobed plan using an explicit `CandidatePlanOrder` |
| `validate_repository_candidate(candidate, repository_root)` | Apply the shared `&`/`^` containment check |
| `complete_partial(token, base)` | Expand an ambient completion token |
| `complete_partial_in_context(token, ctx)` | Expand a completion token from the same roots as execution |
| `resolve_relative(base)` | Resolve ambiently, return a lexical relative path |
| `resolve_target()` | With `url`: distinguish `Resolved::Local` from `Resolved::Remote` |

All builder methods consume and return `self` for chaining.
`PathPosition::Start` inserts a magic root before the default roots;
`PathPosition::End` inserts one after them.

`CandidatePlanOrder::Resolution` preserves the reference kind's normal order.
`CandidatePlanOrder::AuthoringBaseFirst` stably moves `Source` candidates first
for a consumer binding a lazy identity to the context that authored it. The
policy does not change parsing, candidate construction, or filesystem probing;
the consumer still selects from the plan explicitly.

## Diagnostics: `resolve_detailed()`

When "it didn't resolve" needs a real answer — which paths were tried, in
what order, and why the search stopped — use `resolve_detailed()`. It never
returns `Err`; it returns a `DetailedResolution` whose `outcome()` is either
`DetailedOutcome::Matched(path)` or `DetailedOutcome::Failed(failure)`, and
retains:

- `raw()` and `class()` — authored intent;
- `effective_kind()` — post-interpolation anchoring;
- `base_dir()`, optional `source_path()`, optional `repository_root()`;
- `candidates()` — the ordered candidates actually probed before the search
  stopped, each a `ProbedCandidate`;
- `error()` — the underlying `FileReferenceError` (present for failures other
  than `NoMatch`; absent on success);
- `matched_path()` — the winning path.

Note that `candidate_plan()` and `candidates()` differ: the plan is the full
ordered, *unprobed* list; a detailed result contains only the candidates
attempted, so entries after the first match or a terminal I/O failure are
absent. `into_convenience()` performs the legacy projection: match →
`Ok(Some(path))`, `NoMatch` → `Ok(None)`, anything else → `Err(error)`.

`ResolutionFailure` is the stable diagnostic classification. Consume it as
data — never re-derive it from the reference kind:

| Variant | Meaning |
|---------|---------|
| `InvalidReference` | Syntax or an effective-anchoring invariant is invalid |
| `MissingContext` | A required environment, home, vault, or repository input is unavailable |
| `NoMatch` | The complete applicable search found no regular file |
| `Io` | CWD access or a candidate metadata probe failed |
| `UnsupportedRemote` | A remote reference was sent through local-path resolution |

Every `ResolutionCandidate` exposes `path()` and `provenance()`; the
`RootProvenance` vocabulary is `Repository`, `Source`, `PackageRoot`,
`PackageArea`, `Home`, `Magic`, `Vault`, and `Absolute`. Every attempted
`ProbedCandidate` adds a
`ProbeDisposition`:

| Disposition | Meaning |
|-------------|---------|
| `Missing` | Metadata returned `NotFound`; continue |
| `NonFile` | The path exists but is not a regular file; continue |
| `Matched` | A regular file (or a symlink to one); stop successfully |
| `Io(ErrorKind)` | Another I/O error; stop with a typed source |
| `SearchRoot` | A recursive traversal root, not a direct candidate probe |

## Completion

`complete_partial_in_context()` supports magic (`@`), repository-root (`&`),
repository-scoped (`^`), and implicit-relative tokens. Recursive forms return
`Ok(None)` rather than being reinterpreted.
A `PartialCompletion` exposes `entry_form()`, the ordered `roots()`, the
`active_segment()`, and the `rendered_prefix()` a completion consumer uses to
construct the emitted token. A rooted magic token is invalid grammar and
returns `InvalidSyntax` — including when a `%` prefix makes the otherwise
unsupported token recursive.

The parity guarantee: with one shared `FileResolutionContext`, completion and
execution consume the same captured roots in the same precedence (implicit:
base, then repository; magic: configured prepends, package, package area,
repository, home, configured appends; duplicates removed in first-seen order).
A consumer that
enumerates those roots in order can pass its emitted value unchanged to
`FileReference::new()` + `resolve_in_context()` and get the file it
displayed. The ambient `complete_partial()` cannot see request-configured
magic roots, so request-scoped completion should always use the
`_in_context` form.

## Error Reference

The complete `FileReferenceError` vocabulary:

| Variant | Trigger |
|---------|---------|
| `InvalidSyntax(message)` | Empty/malformed syntax, a rooted magic payload, invalid interpolation, or an injected grammar sigil |
| `MissingEnvironmentVariable { name }` | `{{NAME}}` is absent from the selected environment snapshot |
| `CurrentDirectory(source)` | An ambient operation could not read the CWD |
| `Git(source)` | Ambient repository discovery failed for a reason other than "not a repository" |
| `BareRepository` | Repository discovery found no working directory |
| `VaultNotConfigured` | A vault reference has no explicit or captured `$VAULT` roots |
| `UnsupportedUserHome(raw)` | A non-portable `~user` reference was authored |
| `MissingHomeContext` | A home reference has no home directory in the explicit context |
| `OutsideRepository { sigil, reference_cwd }` | `&` or `^` was used without a repository containing the reference base |
| `RepositoryEscape { .. }` | A repository sigil's lexical or resolved target escapes the repository |
| `RepositoryRootNotContainingSource { repository_root, source_path }` | The request base or a normal derived base fails lexical root containment |
| `RelativePath { from, to }` | `resolve_relative()` cannot produce the requested lexical relative path |
| `Io { path, source }` | A direct candidate metadata probe failed; records the candidate path |
| `RemoteNotLocal(raw)` | A remote URL was passed to local-path resolution |
| `InvalidUrl(message)` | With `url`: a remote target is malformed or has an unsupported scheme |

`FileReferenceError` is separate from the fetch-policy/network `FetchError`,
which belongs to the optional fetching API rather than local resolution.

## How Resolution Works, End to End

1. **Parse once.** `FileReference::new()` records the `%` modifier, the
   authored kind, and literal or `{{VAR}}` template segments. Detection order
   is HTTP(S) URL (ASCII-case-insensitive) → `vault::` → `vault:` → `@` → `&` → `^`
   → `~` → absolute (POSIX, Windows drive, or UNC) → explicit relative →
   implicit relative. No filesystem, environment, or CWD access.

2. **Select the state snapshot.** Explicit APIs consume the supplied
   `FileResolutionContext` as authoritative data. Ambient APIs capture or
   discover the needed process state at call time. Repository discovery runs
   at most once per resolution and only for the kinds that use it — explicit
   relative, absolute, home, and vault references never trigger it.

3. **Interpolate, then re-derive anchoring.** Template variables expand from
   the selected environment. For the local family, the payload is then
   re-classified as absolute / explicit-relative / implicit-relative — before
   both direct and recursive candidate construction — and injected grammar
   sigils are rejected.

4. **Build the ordered plan.**

   | Effective/authored kind | Direct candidate or recursive-root order |
   |-------------------------|------------------------------------------|
   | Explicit relative | Base only |
   | Implicit relative | Base, then repository root |
   | Absolute | The authored path only |
   | Home | Home directory only |
   | Magic | Configured prepends, package, package area, repository, home, configured appends |
   | Repository root | Repository only |
   | Repository scoped | Package, package area, repository |
   | Vault | Configured roots, then captured `$VAULT` paths |
   | Remote URL | No local candidates |

   Plans are lexically deduplicated preserving first-seen order, and every
   entry retains its root provenance.

5. **Probe (or traverse).** Direct candidates are checked with fallible
   `std::fs::metadata`, not `Path::is_file()` — so permission problems are
   distinguishable from absence. `NotFound` records `Missing` and advances;
   an existing non-regular path records `NonFile` and advances; any other
   metadata error records `Io`, stores `FileReferenceError::Io { path, source }`,
   and stops immediately. A regular file records `Matched` and wins (metadata
   follows symlinks, so a direct symlink to a regular file matches).
   Recursive resolution traverses the same ordered roots without following
   directory symlinks, applies the filename and parent-suffix filters, sorts
   matches lexically across roots, and takes the first.

6. **Normalize.** Resolved local paths are made absolute with `.`/`..`
   normalized lexically — symlinks are not canonicalized.

## Relative Path Computation

`resolve_relative()` resolves ambiently, then lexically normalizes the target
and the selected base, strips their common prefix, adds `..` for the base's
remaining components, and appends the target's remaining components. When no
lexical relative path can be produced, it reports `RelativePath`.

## Feature Flag

File-reference support is gated behind the default `file-reference` feature,
which enables repository discovery, recursive traversal,
cross-platform home discovery, and URL classification.

```toml
[dependencies]
biscuit-file = { version = "0.1", default-features = false, features = ["file-reference"] }
```
