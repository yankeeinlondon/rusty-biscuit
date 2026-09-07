# `FileReference` Technical Specification

## Context

`FileReference` parses a compact file descriptor once and resolves it later
against either captured request state or ambient compatibility state. The
authoritative user-facing reference is
[`file-references.md`](../topics/file-references.md); this document records the
implementation contract.

## Goals

- Keep parsing passive and independent of the filesystem, process CWD, and
  environment.
- Give every reference kind one deterministic, closed candidate plan.
- Prefer explicit `FileResolutionContext` snapshots for request-scoped work.
- Preserve typed distinctions among a match, a clean miss, malformed syntax,
  unavailable context, repository escape, remote input, and I/O failure.
- Keep completion and execution on the same parser and candidate planner.

## Reference Grammar

After stripping at most one leading recursive `%` modifier, parsing recognizes
references in this order:

1. HTTP(S) URLs;
2. `vault:` / `vault::`;
3. magic `@`;
4. repository-root `&`;
5. repository-scoped `^`;
6. the removed `!` sigil, which is a typed syntax error suggesting `^`;
7. home `~`;
8. absolute POSIX, Windows drive, UNC, and device paths;
9. unsupported scheme-shaped prefixes;
10. explicit `./` / `../` (and Windows separator variants);
11. implicit relative paths.

The `@`, `&`, and `^` forms consume exactly one optional `/`. Empty or rooted
payloads are invalid. A reserved introducer never falls through to an implicit
filename. `{{NAME}}` is the only interpolation syntax; interpolation cannot
inject a sigil into an implicit reference.

## Candidate Plans

| Kind | Ordered bases |
|---|---|
| Explicit relative | Composition CWD only |
| Implicit relative | Composition CWD, then repository root |
| Absolute | The authored absolute path |
| Home | Captured home directory |
| Magic `@` | Registered prepends, package root, package-area root, repository root, home, registered appends |
| Repository root `&` | Repository root only |
| Repository scoped `^` | Package root, package-area root, repository root |
| Vault | Registered roots, then captured `VAULT` roots |
| HTTP(S) | Typed remote target; never a local candidate |

Missing and duplicate bases are omitted while preserving first-seen order.
Direct resolution probes candidates in order and returns the first existing
regular file. Implicit relative paths are therefore composition-CWD first; a
document-local file shadows a repository-root file.

The base is the launch directory for top-level caller-authored references and
the authoring document's directory for document-authored references. Package
and package-area scopes are selected from a caller-supplied
`RepositoryScopeCatalog` for that same base; they are never inherited from an
unrelated launch document.

## Repository Containment

`&` and `^` require a repository containing the reference base. Their
candidates must remain inside it after lexical normalization. If the target or
its deepest existing ancestor can be canonicalized, that canonical path must
also remain inside the canonical repository root, preventing a final symlink,
junction, or reparse point from escaping the repository.

An outside-repository use and a containment escape are typed errors, not clean
misses and not fallback conditions. Other reference kinds retain their normal
symlink behavior.

## Resolution Context

`FileResolutionContext` captures the request's base directory, source path,
home directory, environment, repository scope catalog, magic roots, and vault
roots. Explicit operations read only this snapshot. `for_source` and `for_base`
derive a new authoring base and reselect package/package-area anchors from the
same catalog without ambient discovery.

`resolve()` and `resolve_from(base)` are compatibility entry points. They build
an ambient context at call time; construction-time CWD never affects later
resolution. `resolve_from` treats `base` as the composition CWD for implicit,
explicit-relative, `@`, `&`, and `^` planning while still capturing live home
and environment values.

## Public Outcomes

- `Ok(Some(path))`: an existing regular file matched.
- `Ok(None)`: the reference was valid but no candidate matched.
- `Err(FileReferenceError)`: parsing, context, containment, remote/local type,
  or filesystem evaluation failed.

`resolve_detailed` preserves the authored and effective kinds, ordered probed
candidates, root provenance, probe dispositions, match, and typed failure.
`candidate_plan` returns the complete ordered plan without probing.

`RootProvenance` distinguishes `Source`, `PackageRoot`, `PackageArea`,
`Repository`, `Home`, `Magic`, `Vault`, and `Absolute`. Public reference kinds
distinguish `RepositoryRoot` (`&`) from `RepositoryScoped` (`^`).

## Recursive Resolution

A single leading `%` changes exact probing into deterministic recursive search
under the same ordered roots. Traversal does not follow directory symlinks.
Matches are sorted lexically by full path and the first is selected. Remote
references remain remote when modified with `%`.

A second `%` is invalid syntax. A recursive lazy schema-bound file parameter
cannot materialize one path without I/O and must instead use `file(eager)`.

## Completion

`complete_partial_in_context` accepts magic `@`, repository-root `&`,
repository-scoped `^`, and implicit-relative entry forms. It emits the entry
form plus the same ordered roots used by `candidate_plan`. A consumer can pass
an emitted value unchanged to execution and resolve the displayed file under
the same context.

The ambient completion API exists only for compatibility and cannot observe
request-configured magic roots.

## Test Strategy

Unit and integration coverage must include:

- every grammar kind, removed-`!` diagnostics, malformed sigils, unsupported
  schemes, Windows drive/device/UNC classification, and interpolation;
- composition-CWD-first implicit collision matrices, including equal-root
  deduplication and outside-repository behavior;
- the complete intrinsic and registered `@` order;
- exact `&` and most-specific-first `^` resolution with lexical and canonical
  containment, including Unix symlinks and native-Windows junctions;
- package/package-area selection for nested, top-level, and second-repository
  sources without ambient rediscovery;
- deterministic recursive search and symlink-loop avoidance;
- completion/execution round trips for `@`, `&`, `^`, and implicit forms;
- ambient capture-at-call behavior and explicit snapshot immutability;
- `resolve_relative` across sibling and parent paths; and
- typed remote, missing-context, clean-miss, and I/O outcomes.

Use `tempfile` for isolated trees. Tests that mutate process CWD or environment
must be serialized; most context-explicit fixtures should require neither.

## Documentation Discipline

Any grammar, candidate-order, scope, provenance, or completion change must
update this specification, the topic reference, public rustdoc, and tests in
the same change. Markdown documents carrying a `hash:` frontmatter property
must be refreshed with `md hash <file>`.
