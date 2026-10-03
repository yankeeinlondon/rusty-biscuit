## File Reference Resolution

`biscuit_file::FileReference` is the syntax authority for compact file
descriptors. Do not classify raw strings with prefix checks. Parsing through
`FileReference::new()` is purely syntactic; state enters only through a
resolution context or an ambient compatibility call. For examples and the full
contract, see [the topic doc](../../../../biscuit-file/docs/topics/file-references.md).

### Reference kinds and precedence

| Prefix | `FileReferenceKind` | Resolution roots |
|--------|---------------------|------------------|
| `./`, `../` | `ExplicitRelative` | Authoring `cwd` only; no fallback |
| _(none)_ | `ImplicitRelative` | Authoring `cwd`, then repository root |
| POSIX root, Windows drive/UNC | `Absolute` | Authored absolute path only |
| `~`, `~/` (`~\` on Windows) | `Home` | Captured home only; `~user` is rejected |
| `@`, `@/` | `Magic` | Local tier (local prepends → package → package area → local root → local appends), then user tier (user prepends → home → user appends) |
| `&`, `&/` | `RepositoryRoot` | Repository root only; repository-contained |
| `^`, `^/` | `RepositoryScoped` | Package → package area → repository; repository-contained |
| `vault:`, `vault::` | `Vault` | Configured roots → captured `VAULT` paths |
| `http://`, `https://` | `Url` | Remote target; no local candidate |

`FileReference::class()` returns `FileReferenceClass { kind, recursive }`.
`FileReference::payload()` returns the authored text after `%`, the sigil,
and its optional `/` (`%@/@x.md` → `@x.md`); diagnostics use it instead of
trimming prefixes.
Recursive `%` is a modifier, not another kind. It traverses the same ordered
roots, does not follow directory symlinks, sorts all matches lexically, and
records roots as `ProbeDisposition::SearchRoot`.

Magic consumes exactly the authored `@` or `@/` sigil form. Its remaining
payload must be relative: repeated POSIX separators and Windows
drive-qualified, rooted, or UNC payloads return `InvalidSyntax` rather than
replacing a configured magic root. This also applies under recursive `%`.

### Magic (`@`) tiers

Every local-tier root is searched before any home-based root. The **local
root** is the launch repository root, or the request directory when there is
none (provenance `LocalRoot`, not `Source`). A configured root is local when
it lexically lies inside the local root (after normalization), otherwise user
tier — containment in the local root decides, never containment in `$HOME`.
`add_magic_path(path, position)` infers the tier (`MagicPathTier::Inferred`);
`add_magic_path_with_tier(path, position, MagicPathTier::User)` forces the
user tier, which a caller needs for home conventions when the local root *is*
`$HOME`. `PathPosition` orders a root only within its tier (`Start` before
that tier's intrinsic roots, `End` after). Roots are deduplicated after
ordering, keeping first-seen provenance. A relative configured root joins onto
the captured request directory — for ambient `resolve_from(cwd)`, onto
`cwd` rather than the process CWD.

### Authoritative explicit context

Document-backed consumers should capture one `FileResolutionContext` at the
request boundary and use it for resolution and completion:

```rust,no_run
use biscuit_file::{
    FileReference, FileResolutionContext, PackageAreaFallback, PathPosition,
    RepositoryScopeCatalog,
};

let scopes = RepositoryScopeCatalog::new(
    "/repo",
    vec!["/repo/claudine".into()],
    vec!["/repo/claudine/lib".into()],
    PackageAreaFallback::FirstComponent,
)?;

let request = FileResolutionContext::new("/repo")
    .with_repository_scope_catalog(scopes)
    .add_magic_path("/repo/prompts", PathPosition::Start)
    .add_vault("/notes");

let document = request.for_source("/repo/prompts/router.md");
let target = FileReference::new("prompts/next.md")?
    .resolve_in_context(&document)?;
# let _ = target;
# Ok::<(), biscuit_file::FileReferenceError>(())
```

`FileResolutionContext::new(cwd)` snapshots the process environment and
cross-platform home once. Supply a validated repository scope catalog,
and override the snapshot with `with_env()`, `with_home_dir()`, or
`without_home_dir()` when the caller has authoritative values. `with_env()`
replaces the whole environment (it does not merge); non-Unicode variables are
never captured. Context-owned
`add_magic_path()` and `add_vault()` configure the roots used by explicit APIs.

Every context directory and tree anchor (request and authoring `cwd`,
`with_base_dir`, repository/package/package-area roots, captured home, and every
launch `@` scope directory including a replaced scope's request directory) must
be absolute. Builders and derivations never fail; `validate()` (run first by every
resolver entry point and `PortablePath`) returns
`RelativeContextDirectory { anchor: ContextAnchor, path }`, trusted
derivations included. Relative env values, magic roots, and vault roots keep
their own rules and stay supported. `home_dir()` reports a relative `$HOME` as
`None`.

Use `for_source(source)` for each in-repository nested file-backed document. It
sets the source and changes the authoring `cwd` to `source.parent()`, while
preserving process-state inputs while selecting repository/package scopes for
the new `cwd`. Use `for_cwd(cwd)` for an in-repository in-memory child document.
Both methods validate the request boundary and their new authoring `cwd`.

Use `for_trusted_external_source(source)` or
`for_trusted_external_cwd(cwd)` only after a configured home, magic, or vault
root has deliberately accepted a document outside the tree. Outside the
current tree they select a **new** tree for the document (catalog repository >
containing vault > opening anchor > fallback) and drop the source repository,
package, and package-area anchors unless a catalog assigns them; the original
request is still validated against its own tree. Every derivation also keeps
the `validate()` failure of the context it came from (captured before a
trusted derivation drops the explicit root or a catalog replaces package
anchors), and a builder on the derived context cannot clear it; correct
settings before deriving. No derivation reads ambient state or performs
discovery. `with_source_path()` records provenance only and
does not derive the `cwd`.

**Tree root and boundary.** `base_dir()` is the tree root and
`base_dir_origin()` says where it came from (`BaseDirOrigin::{Repository,
Explicit, Vault, Home, Environment { name }, Fallback}`): the repository root,
then `with_base_dir(dir)` (must equal the repository root inside one, else
`validate()` gives `BaseDirNotRepositoryRoot`), then the deepest containing
vault, then the `~`/`{{VAR}}` anchor kept by `for_source_reference(&reference,
resolved)` / `for_trusted_external_source_reference`, then `cwd` (fallback, not
a boundary). Explicit and bare relative references must stay inside a boundary
tree as written and where they land (shared canonical check with `&`/`^`), or
fail with `RelativeTreeEscape`; this applies to `candidate_plan`,
`resolve_detailed`/`resolve_in_context`, and implicit-relative completion.
`allow_external_relative()` lifts it (copied to children). A normal
derivation leaving a non-repository tree fails `validate()` with
`CwdOutsideBaseDir`. Ambient `resolve()`/`resolve_from()` carry no tree. Do not
compare `base_dir()` with `cwd()` to infer the boundary; use
`base_dir_is_boundary()`.

`@` does not follow those re-anchored scopes. The context carries an immutable
`LaunchMagicScope` (request directory plus its repository, package, and
package-area roots), captured by `new` and the anchor builders and copied
unchanged by `for_source`, `for_cwd`, and the trusted-external derivations.
A nested `@x.md` in a prompt from `~/.claudine` or another repository searches
the launch tree first; `./`, bare, `&`, and `^` keep source anchors. A caller
that rebuilds a context around an external source (rather than deriving it)
seeds `with_launch_magic_scope(launch.launch_magic_scope().clone())`.
`magic_search_roots()` returns the ordered, deduplicated `@` chain with
provenance for diagnostics.

The explicit context is authoritative. `resolve_in_context()`,
`resolve_detailed()`, `candidate_plan()`, and
`complete_partial_in_context()` do not perform late CWD, HOME, environment,
repository, or package-area discovery. Roots configured on the `FileReference`
itself apply only to ambient resolution; add request-scoped roots to the context.

### Ambient compatibility APIs

- `resolve()` reads ambient CWD, environment/home, and repository state when called.
- `resolve_from(cwd)` fixes the authoring `cwd` but still reads the other live
  state. `cwd` is a directory; pass a source file's parent.
- `resolve_relative(base)` resolves ambiently and then computes a lexical
  relative path.
- `complete_partial(token, cwd)` performs live repository/home discovery and
  cannot see request-configured magic roots.

Use these for compatibility and simple top-level calls. Do not use them inside
a request that requires a stable snapshot.

### Detailed resolution and candidate plans

`resolve_in_context()` projects to `Result<Option<PathBuf>,
FileReferenceError>`: match is `Ok(Some)`, no-match is `Ok(None)`, and other
failures are `Err`.

`resolve_detailed()` never returns `Err`; it returns `DetailedResolution` with:

- authored `raw()` and `class()`;
- post-interpolation `effective_kind()`;
- `cwd()`, `source_path()`, and `repository_root()`;
- attempted `candidates()` as ordered `ProbedCandidate` values;
- `outcome()`, `error()`, and `matched_path()`.

`DetailedOutcome` is `Matched(PathBuf)` or `Failed(ResolutionFailure)`. The
failure vocabulary is `InvalidReference`, `MissingContext`, `NoMatch`, `Io`,
and `UnsupportedRemote`. `NoMatch` has no underlying error; other failed
outcomes retain a `FileReferenceError`.

`candidate_plan()` returns the complete ordered, unprobed plan. By contrast,
`DetailedResolution::candidates()` contains only attempts made before the first
match or terminal I/O error. Each `ResolutionCandidate` exposes its path and
`RootProvenance`: `Repository`, `Source`, `PackageRoot`, `PackageArea`, `Home`,
`Magic`, `Vault`, `Absolute`, or `LocalRoot`.

`candidate_plan_with_order()` applies an explicit `CandidatePlanOrder` to that
unprobed plan. The default `Resolution` order matches execution.
`AuthoringBaseFirst` stably moves `Source` candidates ahead of other roots for
consumers binding a lazy identity to its captured authoring context; parsing,
candidate construction, and validation remain inside `FileReference`.

Direct candidates are probed with fallible `std::fs::metadata`, never
`Path::is_file()`:

| `ProbeDisposition` | Action |
|--------------------|--------|
| `Missing` | `NotFound`; continue |
| `NonFile` | Exists but is not a regular file; continue |
| `Matched` | Regular file or symlink to one; stop successfully |
| `Io(ErrorKind)` | Other metadata failure; stop and retain `Io { path, source }` |
| `SearchRoot` | Recursive traversal root, not a direct probe |

### Interpolation and recursive anchoring

`{{VAR}}` values come from the selected snapshot. After one interpolation pass,
the resolver reclassifies filesystem anchoring for authored absolute,
explicit-relative, and implicit-relative values. This applies to direct and `%`
recursive references alike, so an interpolated absolute root is reported and
handled as `Absolute`.

Missing variables are errors on the local-path resolver. The remote-target API
retains an unresolved placeholder verbatim as part of its existing URL behavior.

Interpolation cannot inject a leading `@`, `&`, `^`, `%`, `vault:`, or
case-insensitive HTTP(S) scheme into a local reference. Both direct and
recursive resolution reject the result with `InvalidSyntax`; grammar sigils
must be authored. Authored magic/repository/vault/URL references keep their kind
and interpolate within it.

Recognized introducers are reserved: malformed `@`, `&`, `^`, `%`, `~`,
`vault:`, or HTTP(S) forms fail instead of becoming implicit references. The
removed `!` form fails with a `^` migration hint. Repository-root and
repository-scoped candidates receive lexical and canonical containment checks,
including the deepest existing ancestor for a missing lazy target. This is a
TOCTOU-aware resolver boundary, not a filesystem sandbox. Quote `&` in shell
arguments, such as `spec='&docs/plan.md'`.

### Completion/execution parity

Use `complete_partial_in_context(token, &ctx)` and
`resolve_in_context(&ctx)` with the same context. Completion supports direct
magic, repository-root, repository-scoped, and implicit-relative tokens. Its
`PartialCompletion` provides `entry_form()`, ordered `roots()`,
`active_segment()`, and `rendered_prefix()`.

The completion roots mirror execution: implicit is `cwd` then repository; magic
is the same tier-ordered chain (built once, from the launch `@` scope) with the
typed segment appended after root selection, with stable deduplication. Enumerate roots in order and execute the emitted string unchanged
through `FileReference::new()` plus `resolve_in_context()` so the displayed and
executed candidate cannot diverge. Completion rejects rooted magic tokens with
`InvalidSyntax`, including invalid recursive magic tokens that completion does
not otherwise support.

### Complete `FileReferenceError` vocabulary

```text
InvalidSyntax(String)
ForeignAbsolutePath { path }
MissingEnvironmentVariable { name }
CurrentDirectory(io::Error)
Git(Box<gix::discover::Error>)
BareRepository
VaultNotConfigured
UnsupportedUserHome(String)
MissingHomeContext
OutsideRepository { sigil, reference_cwd }
RepositoryEscape { sigil, reference, repository_root, escaped_candidate }
RepositoryRootNotContainingSource { repository_root, source_path }
RelativeContextDirectory { anchor, path }
RelativePath { from, to }
Io { path, source }
RemoteNotLocal(String)
InvalidUrl(String)                    # with `url`
```

`InvalidSyntax` also covers rooted magic payloads and interpolation-injected
sigils.
`ForeignAbsolutePath` rejects an absolute path (authored or interpolated) that
is not absolute on the resolving host (`C:\x` on POSIX, `/x` on Windows);
nothing is translated between operating systems.
`RepositoryRootNotContainingSource` is the lexical containment check on the
request `cwd` and normal derived authoring `cwd`s. `RemoteNotLocal`
means a URL reached a local path API; use the `url`-gated `resolve_target()`
when the caller accepts `Resolved::Remote`.
