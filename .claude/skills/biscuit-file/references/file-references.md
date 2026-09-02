## File Reference Resolution

`biscuit_file::FileReference` is the syntax authority for compact file
descriptors. Do not classify raw strings with prefix checks. Parsing through
`FileReference::new()` is purely syntactic; state enters only through a
resolution context or an ambient compatibility call. For examples and the full
contract, see [the topic doc](../../../../biscuit-file/docs/topics/file-references.md).

### Reference kinds and precedence

| Prefix | `FileReferenceKind` | Resolution roots |
|--------|---------------------|------------------|
| `./`, `../` | `ExplicitRelative` | Authoring base only; no fallback |
| _(none)_ | `ImplicitRelative` | Authoring base, then repository root |
| POSIX root, Windows drive/UNC | `Absolute` | Authored absolute path only |
| `~`, `~/` (`~\` on Windows) | `Home` | Captured home only; `~user` is rejected |
| `@`, `@/` | `Magic` | Configured prepends → package → package area → repository → home → configured appends |
| `&`, `&/` | `RepositoryRoot` | Repository root only; repository-contained |
| `^`, `^/` | `RepositoryScoped` | Package → package area → repository; repository-contained |
| `vault:`, `vault::` | `Vault` | Configured roots → captured `VAULT` paths |
| `http://`, `https://` | `Url` | Remote target; no local candidate |

`FileReference::class()` returns `FileReferenceClass { kind, recursive }`.
Recursive `%` is a modifier, not another kind. It traverses the same ordered
roots, does not follow directory symlinks, sorts all matches lexically, and
records roots as `ProbeDisposition::SearchRoot`.

Magic consumes exactly the authored `@` or `@/` sigil form. Its remaining
payload must be relative: repeated POSIX separators and Windows
drive-qualified, rooted, or UNC payloads return `InvalidSyntax` rather than
replacing a configured magic root. This also applies under recursive `%`.

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

`FileResolutionContext::new(base_dir)` snapshots the process environment and
cross-platform home once. Supply a validated repository scope catalog,
and override the snapshot with `with_env()`, `with_home_dir()`, or
`without_home_dir()` when the caller has authoritative values. Context-owned
`add_magic_path()` and `add_vault()` configure the roots used by explicit APIs.

Use `for_source(source)` for each in-repository nested file-backed document. It
sets the source and changes the authoring base to `source.parent()`, while
preserving process-state inputs while selecting repository/package scopes for
the new base. Use `for_base(base)` for an in-repository in-memory child document.
Both methods validate the request boundary and their new authoring base.

Use `for_trusted_external_source(source)` or
`for_trusted_external_base(base)` only after a configured home, magic, or vault
root has deliberately accepted a document outside the repository. These named
derivations exempt the current external authoring base but still validate the
original request boundary. No derivation reads ambient state or performs
discovery. `with_source_path()` records provenance only and does not derive the
base.

The explicit context is authoritative. `resolve_in_context()`,
`resolve_detailed()`, `candidate_plan()`, and
`complete_partial_in_context()` do not perform late CWD, HOME, environment,
repository, or package-area discovery. Roots configured on the `FileReference`
itself apply only to ambient resolution; add request-scoped roots to the context.

### Ambient compatibility APIs

- `resolve()` reads ambient CWD, environment/home, and repository state when called.
- `resolve_from(base)` fixes the authoring base but still reads the other live
  state. `base` is a directory; pass a source file's parent.
- `resolve_relative(base)` resolves ambiently and then computes a lexical
  relative path.
- `complete_partial(token, base)` performs live repository/home discovery and
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
- `base_dir()`, `source_path()`, and `repository_root()`;
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
`Magic`, `Vault`, or `Absolute`.

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

The completion roots mirror execution: implicit is base then repository; magic
is configured prepends, package, package area, repository, home, then configured
appends, with stable deduplication. Enumerate roots in order and execute the emitted string unchanged
through `FileReference::new()` plus `resolve_in_context()` so the displayed and
executed candidate cannot diverge. Completion rejects rooted magic tokens with
`InvalidSyntax`, including invalid recursive magic tokens that completion does
not otherwise support.

### Complete `FileReferenceError` vocabulary

```text
InvalidSyntax(String)
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
RelativePath { from, to }
Io { path, source }
RemoteNotLocal(String)
InvalidUrl(String)                    # with `url`
```

`InvalidSyntax` also covers rooted magic payloads and interpolation-injected
sigils.
`RepositoryRootNotContainingSource` is the lexical containment check on the
request base and normal derived authoring bases. `RemoteNotLocal`
means a URL reached a local path API; use the `url`-gated `resolve_target()`
when the caller accepts `Resolved::Remote`.
