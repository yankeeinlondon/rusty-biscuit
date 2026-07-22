---
created: 2026-07-13
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-16
review_iterations: 8
depends_on:
    - ../2026-07-13-error-propogation/spec.md
---

# Unified File-Reference Resolution

## Upstream Dependency Status — typed transport has LANDED

The [`2026-07-13-error-propogation`](../2026-07-13-error-propogation/spec.md)
feature this spec depends on is **complete**. Two things it finished are now
this feature's inputs, and one thing it could not finish is now this feature's
job.

### The reserved nulls are ready to fill

`composition.invalid_file_reference` was extended additively and already declares
the fields a real resolver will supply. They project `null` today by ruling, not
by oversight — see
[decisions.md §D-5](../2026-07-13-error-propogation/decisions.md):

| Field | Waiting for |
|---|---|
| `failure` | a typed classification that distinguishes `permission_io` / `missing_context` from `no_match`. **Do not derive it from `kind`** — Darkmatter's `FileRefFailure::classify` folds I/O, permission, and missing-context failures into `NotFound`, so `NotFound → no_match` would assert "no candidate matched" for a permission error that never probed a candidate |
| `candidates` | the ordered, provenance-carrying probe record this feature's search produces |
| `repository_root` | the resolved root the implicit-relative search anchored on |
| `source_path`, `property`, `event` | supplied today by the semantic wrapper; a resolver-side value must agree with it, not duplicate it |

`base_dir` and `fallback_dir` are **compatibility projections** of the pre-
`candidates` payload, retained so existing `when:` clauses keep matching.
`candidates` supersedes them; neither may be removed or re-typed.

The transport itself is done: a typed resolution error reaches the surface with
its facets and its rendered block intact, so this feature only has to produce the
typed value — not carry it. See
[`docs/topics/error-architecture.md`](../../docs/topics/error-architecture.md)
for the wrapper rules a new resolver error must follow.

### AC5 handoff — the two proxy resolvers must converge here

Error-propagation Acceptance Criterion 5 ("the same proxy failure has identical
diagnostic identity, headline, hint, and typed resolution detail regardless of
which lifecycle route initiated it") is **confirmed unsatisfiable** without a
routing change, and that change belongs to this feature
([decisions.md §D-12](../2026-07-13-error-propogation/decisions.md)).

Both routes are now typed and both render a `StatusBlock`. But they still fail at
different stages against different resolvers — the `initialize` route through
`resolve_proxy_target` at resolution time, the terminal route through
`resolve_harness_path`, which resolves successfully and only fails later reading
the adopted document. Wrapping cannot make those agree; converging the resolvers
can, and that convergence is exactly this feature's D-goal.

> ⚠️ **`level2_proxy_routes_share_a_typed_surface_but_diverge_on_identity_in_tmux`
> will fail when this feature lands.** That is by design. It pins the current
> divergence so the convergence cannot land silently and leave AC5 unverified
> forever. When it fails, do not weaken it — **promote** its assertions to full
> AC5 parity (identical code, headline, hint, and typed detail across both
> routes), keeping the event/property context assertions separate so
> intentional route-specific detail is not mistaken for drift.

### Known debt this feature may inherit

`error_guards/transport-allow.toml` carries 71 entries tagged
`error-propagation-followup` — pre-existing lossy boundaries frozen rather than
fixed ([decisions.md §D-11](../2026-07-13-error-propogation/decisions.md)). Any
that sit on a file-resolution path are fair game to close here; the rest need
their own spec.

## Motivating Incident

The implementation router at `prompts/implement.md` authored this lifecycle
handoff:

```yaml
- proxy: prompts/_implement/implement-suggestions.md
```

The target exists at the repository-root-relative location
`prompts/_implement/implement-suggestions.md`. Claudine's private harness
resolver nevertheless treated every non-absolute, non-`@` value as relative to
the source document. Because the source document already lives in `prompts/`,
the attempted path became:

```text
<repo>/prompts/prompts/_implement/implement-suggestions.md
```

The authored value is an **implicit relative reference**: it has no prefix that
pins it to one base. Under the intended shared file-reference contract it must
search the available implicit bases in precedence order. It is not equivalent
to the **explicit relative reference**
`./prompts/_implement/implement-suggestions.md`, which deliberately pins
resolution to the source document's directory and would produce the doubled
path.

This feature removes Claudine's private path grammar and aligns every
Claudine-executed document reference with `biscuit-file::FileReference`.

## Ratified Resolution Contract

For a file reference authored in a file-backed document:

| Form | Name | Resolution behavior |
|---|---|---|
| `./foo.md`, `../foo.md` | Explicit relative | Source document directory only; no fallback |
| `foo.md`, `path/to/foo.md` | Implicit relative | Repository root first, then source document directory |
| absolute OS path | Absolute | The authored path only |
| `@foo.md` | Magic | Existing `FileReference` magic-root search order |
| `!foo.md` | Package | Existing package-area resolution |
| `~/foo.md` | Home | The current user's home directory only |
| `vault:foo.md` | Vault | Existing configured-vault resolution |
| `%...` | Recursive | Existing recursive semantics over the underlying kind's roots |
| `http://...`, `https://...` | Remote URL | Existing surface-specific fetch policy; schemes are case-insensitive and never treated as local paths |

The first candidate that can be confirmed as a regular file wins. A missing
candidate advances the search; permission and other metadata failures are
typed I/O errors rather than being misreported as absence. Direct resolution
retains its existing ability to select a symlink whose target is a regular
file, while recursive traversal continues not to follow directory symlinks.

This specification ratifies **repository root before source document** for
implicit document references. Current
[`biscuit-file` documentation](../../../biscuit-file/docs/topics/file-references.md)
and implementation describe implicit resolution as base/CWD before Git root.
That is contract drift and must be reconciled as part of this feature; Claudine
must not compensate with another private resolver whose behavior differs from
the shared authority.

> **Reader note — intentional precedence change:** This is a breaking change
> to the current `biscuit-file` and Darkmatter source-first behavior, not a
> correction to make silently. Repository-shaped bare paths are the primary
> Claudine authoring form, so repository-first precedence is intentional. The
> migration must inventory collisions before switching the default and rewrite
> source-local intent to `./`; it must not preserve the old behavior behind a
> Claudine-only resolver.

## Terminology

### Explicit relative reference

A reference beginning with `./` or `../` expresses an exact relationship to
the active resolution base. When it is authored in a document, that base is the
source document's directory.

```text
source:     <repo>/prompts/router.md
reference:  ./_implement/next.md
candidate:  <repo>/prompts/_implement/next.md
```

There is exactly one candidate. If it does not exist, resolution fails. The
resolver must not silently reinterpret it from the repository root, launch
directory, package area, or HOME.

### Implicit relative reference

A bare path carries no explicit base. It is portable authoring syntax intended
to work naturally for repository-shaped references while retaining a
source-local fallback.

```text
source:     <repo>/prompts/router.md
reference:  prompts/_implement/next.md
candidates:
  1. <repo>/prompts/_implement/next.md
  2. <repo>/prompts/prompts/_implement/next.md
```

If both candidates exist, the repository-root candidate wins. If the source is
not contained in a Git repository, only the source-relative candidate is
available.

### Magic reference

`@` remains the broader `FileReference` magic-search form. It is not renamed to
"repository relative" and does not become an alias for implicit relative.
Magic references continue to support configured prepended roots, Git root,
HOME, and appended roots. Claudine may add its established prompt-directory
magic roots, but those additions must be explicit configuration on
`FileReference` rather than separate parsing or joining logic.

### Home reference

`~` and `~/...` pin resolution to the current user's home directory. Claudine
already supports this form for external sequence references, so routing every
value through `FileReference` would otherwise be a regression. This feature
promotes the form into the shared grammar rather than retaining a
sequence-only expansion. `~user` expansion is not portable and remains
unsupported.

## Goals

- Make `FileReference` the single parser and resolver for Claudine file
  references.
- Preserve the explicit-versus-implicit distinction at every document-backed
  surface.
- Implement the ratified implicit precedence: repository root, then source
  document directory.
- Anchor repository discovery to the source document/resolution context, not a
  process CWD that wrappers may have changed.
- Apply identical semantics across lifecycle proxying, composition, sequence,
  schema/file properties, Darkmatter expressions, and transclusion when those
  surfaces execute under Claudine.
- Preserve magic, package, vault, recursive, environment interpolation, URL,
  home, and platform-native absolute reference forms.
- Eliminate ambient CWD, HOME, environment, Git-root, and package-area reads
  from Claudine's document-backed resolution path after its request context has
  been captured.
- Return typed, candidate-aware diagnostics for invalid and unresolved
  references.
- Make the behavior deterministic and testable on macOS, Linux, and Windows.

## Non-goals

- Changing the meaning or syntax of `@`, `!`, `vault:`, or `%`, or changing
  environment interpolation beyond the decision recorded in OQ1.
- Adding new recursive-search roots or changing the recursive walker's
  `follow_links(false)` contract. Direct exact-path resolution may continue to
  select symlinks to regular files.
- Changing remote-host allowlists, fetch policy, caching, or network security.
- Preventing an explicitly authored `../` path from leaving the repository.
  Sandboxing/path-policy changes require a separate security design.
- Treating missing references as ambiguous when multiple candidates were
  possible. Ordered precedence remains the contract.
- Requiring authors to rewrite valid implicit references to `@` or `./`.
- Solving generic error transport. The related error-propagation specification
  owns preservation and rendering after a typed resolution error is created.

## Current Drift

Claudine and Darkmatter currently contain several distinct resolution models:

1. `biscuit-file::FileReference` parses explicit and implicit relative kinds,
   but currently searches the supplied base/CWD before Git root.
2. `claudine::harness::resolve_harness_path` recognizes only absolute, `@`, and
   "all other relative" paths; it joins the latter directly to the source
   directory and never attempts the repository root.
3. `resolve_composition_source` uses `FileReference::resolve()` plus
   Claudine-specific magic prompt roots.
4. External sequence references route only selected prefixes through
   `FileReference`; plain values are manually joined to the source directory.
5. Darkmatter expression/file-schema resolution tries document-relative and
   then a launch-area fallback.
6. Darkmatter transclusion has its own branch between special references and
   manually joined relative paths.
7. External sequence references privately expand `~`, although `FileReference`
   has no home-pinned kind; removing the private branch would change behavior.
8. Darkmatter link resolution falls back to a direct path join after a shared
   resolver miss, which can bypass shared classification and diagnostics.
9. `FileReference` currently reads HOME and the full process environment while
   resolving, and its HOME lookup uses `$HOME` alone, which is not a complete
   Windows contract.

These paths accept the same author-facing strings but do not share one
classification, candidate order, or diagnostic shape. The migration must
remove the behavioral duplication, not merely patch lifecycle `proxy`.

## Required Design

### D1 — `FileReference` remains the syntax authority

Every input is parsed once by `FileReference::new`. Callers must not use prefix
checks such as `starts_with('@')`, `starts_with("./")`, or custom
`is_file_reference_target` helpers to decide whether a value deserves shared
resolution.

The parsed kind must remain available to resolution and diagnostics. If the
current public API does not expose enough information without leaking internal
parser types, `biscuit-file` should add a small public classification surface,
for example:

```rust
pub enum FileReferenceKind {
    ExplicitRelative,
    ImplicitRelative,
    Absolute,
    Magic,
    Package,
    Home,
    Vault,
    Url,
}

pub struct FileReferenceClass {
    pub kind: FileReferenceKind,
    pub recursive: bool,
}
```

Recursive is a modifier over a kind, not a competing kind, and must remain
observable. The exact API is an implementation decision. Claudine must not
duplicate the grammar to obtain either classification.

### D2 — Resolution context is explicit data

Document-backed resolution receives a context containing at least:

```rust
pub struct FileResolutionContext<'a> {
    pub source_path: Option<&'a Path>,
    pub base_dir: &'a Path,
    pub repository_root: Option<&'a Path>,
    pub package_area: Option<&'a Path>,
    pub home_dir: Option<&'a Path>,
}
```

The context also owns or borrows the environment snapshot and configured
magic/vault roots needed by the request. The exact owned/borrowed representation
is an implementation decision, but candidate construction must not reread
ambient state. Repository and package-area discovery happen once per
composition/harness run where practical and are reused; resolution must not
depend on later `set_current_dir` or environment mutation.

Claudine and Darkmatter already depend on `sniff`, so they discover the trusted
worktree root through `sniff::filesystem::git::repo_root` (or
`GitRepo::discover`) and pass it into this context. `biscuit-file` must not add a
dependency on `sniff`, because `sniff` already depends on `biscuit-file`; that
would create a crate cycle. Its ambient compatibility methods may retain their
lower-level discovery implementation, while the canonical document-backed API
uses the caller-supplied root.

For a file-backed source:

- `source_dir = source_path.parent()`;
- `base_dir = source_dir` after absolutizing and lexical normalization;
- `repository_root` is the nearest trusted Git worktree root containing the
  source directory;
- a caller-provided root is accepted only when it contains the source or the
  operation explicitly documents a different trust boundary.

Containment is component-aware and lexical after absolutization; it does not
canonicalize through symlinks. This preserves the authored/worktree identity
and is not a sandbox boundary.

For a top-level CLI reference with no source document, the launch context is
used: repository root first, then launch directory. In-memory/stdin surfaces
without either context must return a typed missing-context error rather than
consulting an unrelated ambient CWD.

Every nested file-backed document establishes a new `base_dir` for references
authored inside it. A proxied target, external sequence, included schema, or
transcluded document therefore becomes the source for its own nested
references; the original entry document's directory must not leak inward. The
request-level worktree data remains available in the derived context. The
launch directory is a base only for a top-level reference and must not become a
third fallback for nested document references.

### D3 — Candidate order is kind-specific and observable

Candidate construction for local non-recursive references is:

```text
ExplicitRelative: [source_dir / authored_path]
ImplicitRelative: [repo_root / authored_path, source_dir / authored_path]
Absolute:         [authored_path]
Magic:            existing configured magic roots
Package:          existing package area or Git-root fallback
Home:             [home_dir / authored_path]
Vault:            existing configured vault roots
Url:              no local filesystem candidates
```

Duplicate lexical candidates are removed without changing the first-seen
order. Candidate records retain root provenance (`repository`, `source`,
`package`, `home`, `magic`, or `vault`) as well as the path so diagnostics and
completion do not infer provenance from string prefixes.

Recursive resolution uses roots from the same builder rather than a separate
kind classifier, but retains its established behavior of globally sorting all
matches lexically before selecting the first. Root-plan provenance remains
available to diagnostics. Changing recursive winner selection is outside this
feature.

Candidate generation and matching should be separable enough that diagnostics,
completion, and tests can inspect the exact ordered candidates without
reimplementing the algorithm.

### D4 — `resolve()` and `resolve_from()` follow the same terminology

The `biscuit-file` public API and documentation must make the distinction
unambiguous:

- `resolve()` uses launch/ambient context: enclosing repository root first,
  then CWD for implicit references; CWD only for explicit references.
- `resolve_from(base)` uses document context: enclosing repository root of
  `base` first, then `base` for implicit references; `base` only for explicit
  references.
- special reference kinds retain their documented roots.

`base` means a directory. Callers with a source file pass its parent; passing a
file path as the base is an API misuse and must not be guessed from filesystem
existence.

The shared library adds one context-aware detailed resolution operation that
returns the classification, ordered candidate/root plan, and match or failure.
The exact name is an implementation decision (`resolve_detailed` or
`resolve_with_context` are representative). Existing `resolve()` and
`resolve_from()` may retain their `Result<Option<PathBuf>, _>` convenience
shape for compatibility, but Claudine and Darkmatter use the detailed operation
where diagnostics are required. This avoids the current contradiction where an
`Ok(None)` has already discarded the candidates that D8 requires.

If another `biscuit-file` consumer requires a transition policy, that policy
must live in `biscuit-file`, be explicit at the call site, and remain unused by
all Claudine/Darkmatter surfaces in scope. Repository-first becomes the shared
default within this feature; a Claudine-only candidate loop or indefinite
legacy default is not acceptable.

### D5 — Claudine migration surfaces

The implementation must inventory and migrate every Claudine-executed file
reference, including:

- top-level `compose`, `inline-compose`, and `sequence` source arguments;
- lifecycle `proxy` targets from every supported event and orchestration path;
- target re-materialization after proxy/retry/resume/loop transitions;
- external sequence YAML/Markdown references;
- SimplifiedSchema `file(...)` values and `$schema` references evaluated while
  preparing a Claudine document;
- Darkmatter read-side file functions invoked by Claudine composition;
- Darkmatter `::file`, `::code`, prologue/epilogue, and related transclusion
  targets;
- Darkmatter local Markdown link/image resolution when that compose transform
  is enabled;
- system-prompt file references when an explicit reference is supplied.

The migration inventory must also find fallback `PathBuf::join`, `canonicalize`,
tilde expansion, prefix classification, and resolver-error suppression; the
named list above is a minimum, not an allowlist.

Discovery workflows with intentionally different semantics, such as searching
upward for an optional `system-prompt.md`, remain separate and must be named as
discovery rather than file-reference resolution.

`resolve_harness_path` may become a thin typed adapter over `FileReference` or
be removed. It must not retain its private path grammar.

### D6 — No semantic drift across proxy routes

The same proxy target value must resolve identically whether proxying occurs
from:

- the original document's `initialize` event;
- a proxied target's `initialize` event;
- `start`, `success`, `failure`, or `finalize` recovery where supported;
- loop-engine initialization;
- harness terminal-control dispatch.

All routes call the same resolver with the context belonging to the document
that authored the current target. Re-entering the same source/reference tuple
must produce the same candidate plan regardless of route. When a proxied target
authors another proxy, the target document becomes the new source; retaining
the original source path would be a context-provenance bug. No route may use
`PathBuf::join` directly for the target.

### D7 — Cross-platform absolute paths

Absolute-path detection uses platform-aware `Path::is_absolute` semantics and
must correctly recognize:

- POSIX roots on macOS/Linux (`/tmp/a.md`);
- Windows drive-qualified paths (`C:\\work\\a.md`);
- Windows UNC paths (`\\\\server\\share\\a.md`).

On Windows, explicit relative references also accept backslash spellings such
as `.\foo.md` and `..\foo.md`, while `C:foo.md` remains drive-relative rather
than absolute. Forward-slash spellings remain the portable authored form. URL
scheme recognition is ASCII case-insensitive, as required by URL syntax; a URL
must not fall through to a Windows drive/path classifier.

Tests that cannot execute a foreign platform's path semantics on the host may
use target-gated cases, pure parser fixtures, or CI runners. The author-facing
grammar and serialized diagnostics remain portable.

Path display must use platform-native formatting; persisted logical reference
strings retain their authored spelling.

### D8 — Typed resolution result and diagnostics

An unresolved reference is not represented as a formatted `detail: String`.
The detailed shared outcome must retain:

- raw authored reference;
- parsed reference kind;
- source document/base;
- repository root when available;
- ordered candidates attempted;
- the probe disposition for each attempted candidate (missing, non-file,
  matched, or I/O failure);
- failure classification (invalid syntax, missing context, no match,
  permission/I/O, unsupported remote, and so on);
- underlying `FileReferenceError` where one exists.

`NoMatch` is a typed detailed outcome even if legacy convenience methods map it
to `Ok(None)`. Direct candidate probing uses a fallible metadata operation:
`NotFound` and a non-regular-file candidate advance the ordered search, while
permission, invalid-path, and other I/O failures stop with the candidate path
and typed source attached. `Path::is_file()` is insufficient because it
collapses those failures into `false`.

Claudine adds the semantic surface/property through a typed wrapper, as defined
by the related error-propagation specification. A proxy-target miss is an
author-correctable composition error, not merely a generic environment read
failure.

The terminal report should be able to explain both intent and behavior, for
example:

```text
Reference: prompts/_implement/implement-suggestions.md
Kind: implicit relative
Tried:
  1. <repo>/prompts/_implement/implement-suggestions.md
  2. <repo>/prompts/prompts/_implement/implement-suggestions.md
```

Candidate data is also available to lifecycle `err.detail.*` and machine
output without parsing this prose.

### D9 — Completion and execution use the same candidates

Dynamic completion/autocomplete may rank and display candidates, but a value it
emits must resolve through the same candidate builder in the same context.
Completion must not teach source-relative-only syntax while execution uses
repository-first implicit resolution, or vice versa.

Interactive "did you mean" recovery remains optional. It consumes the typed
no-match diagnostic and does not replace deterministic resolution.

### D10 — Repository-root discovery is bounded and cached

Repository discovery uses the base/source path, not global mutable state. One
composition request should not repeatedly invoke Git discovery for every
reference when the root is already known.

Caching is scoped to the request or an immutable resolution context so it does
not leak one worktree's root into another. Linked Git worktrees must resolve to
their own worktree root.

### D11 — Home references are shared and cross-platform

`FileReference` recognizes only `~` and `~/...` (plus `~\\...` on Windows) as
the home-pinned kind. It does not implement shell expansion, `~user`, or
fallback to magic roots. Home discovery is supplied through the resolution
context and must use a cross-platform provider; `$HOME` alone is not sufficient
on native Windows. Missing home context is a typed missing-context failure.

Claudine removes its sequence-only tilde branch after this shared kind lands.
Magic `@` continues to include HOME in its ordered search, but it is not a
substitute because a home reference is pinned and has no repository fallback.

### D12 — Compatibility audit precedes the precedence switch

Before changing the default, inventory every workspace call site of
`FileReference::resolve()` and `resolve_from()`, plus committed Claudine and
Darkmatter fixtures/documents. For authored bare references where both
repository-root and source-relative candidates exist, classify each collision
by author intent and rewrite source-local intent to `./`. Call sites that
intentionally require the old CWD-first contract must select a shared explicit
transition policy and document why. Do not rewrite unambiguous bare references
or add a permanent dual-behavior compatibility layer.

The implementation notes and release/timeline entries must identify these
intentional changes explicitly:

- implicit resolution changes from source/base first to repository first;
- Darkmatter's separate launch-area fallback is removed for nested documents:
  only the repository and authoring-document candidates participate; the
  launch directory remains the base only for top-level references;
- external sequence `~` handling moves into `FileReference` without changing
  its user-visible meaning;
- I/O probe failures that previously appeared as `not found` become typed
  errors.

## Open Questions

### OQ1 — Can environment interpolation change anchoring?

`FileReference` currently classifies before interpolation. An implicit value
such as `{{PROJECT_ROOT}}/docs/spec.md` can then expand to an absolute path, and
`PathBuf::join` silently discards the candidate root. The diagnostic still
calls the value implicit even though it behaved as absolute. The draft's
requirement to preserve interpolation needs an explicit rule.

1. **Keep authored anchoring fixed and reject rooted expansion.**
   - Pros: classification and candidates are stable; an environment value
     cannot silently bypass the selected root.
   - Cons: breaks the documented absolute `{{PROJECT_ROOT}}` use case and
     provides no equally concise portable replacement.
2. **Reclassify filesystem anchoring after one interpolation pass.**
   - Pros: preserves existing absolute environment-root use cases; makes the
     effective behavior and candidate plan honest; works with platform-native
     absolute paths.
   - Cons: resolution kind depends on the environment and diagnostics must
     expose both authored and effective classification.
3. **Preserve the current join behavior as a legacy exception.**
   - Pros: least immediate implementation change.
   - Cons: keeps implicit absolute-root replacement accidental, obscures the
     candidate plan, and varies with `PathBuf::join` platform behavior.

**Recommendation: option 2.** Expand once from the captured environment, then
classify only filesystem anchoring (absolute, explicit relative, or implicit
relative) for the payload. Do not allow interpolation to inject `@`, `!`, `%`,
`vault:`, or a remote URL scheme; those grammar sigils remain author-controlled.
Diagnostics record both the authored kind and effective anchoring. This keeps
the established `{{PROJECT_ROOT}}` capability while making its behavior
portable and observable.

## Precedence Examples

Assume this tree:

```text
<repo>/
├── shared.md
├── prompts/
│   ├── router.md
│   ├── shared.md
│   └── _implement/
│       └── next.md
└── docs/
    └── guide.md
```

From `<repo>/prompts/router.md`:

| Reference | Result | Reason |
|---|---|---|
| `shared.md` | `<repo>/shared.md` | implicit; repository candidate wins |
| `./shared.md` | `<repo>/prompts/shared.md` | explicit; source only |
| `prompts/_implement/next.md` | `<repo>/prompts/_implement/next.md` | implicit repository-shaped path |
| `./_implement/next.md` | `<repo>/prompts/_implement/next.md` | explicit source-relative path |
| `../docs/guide.md` | `<repo>/docs/guide.md` | explicit source-relative traversal |

Deleting `<repo>/shared.md` makes bare `shared.md` fall back to
`<repo>/prompts/shared.md`. It does not affect `./shared.md`.

## Testing Strategy

### `biscuit-file` L1/L2

- Parsing distinguishes explicit and implicit relative references without
  filesystem access.
- Candidate ordering is repository root before base for implicit references.
- Explicit references produce exactly one base-relative candidate.
- Duplicate repo/base roots are deduplicated stably.
- `resolve()` and `resolve_from()` honor the same kind semantics.
- The detailed resolver retains candidate/root provenance on success and
  `NoMatch`; legacy convenience methods project it without changing order.
- When both repo and source candidates exist, repo wins.
- When only the source candidate exists, implicit resolution falls back.
- When no Git root exists, implicit resolution uses only the base.
- Special reference kinds retain their existing behavior.
- macOS/Linux absolute paths and Windows drive/UNC paths are covered.
- Worktree and nested-repository roots are anchored to the supplied base.
- Recursive resolution retains its established global lexical winner while
  consuming roots from the shared candidate builder.
- Direct symlink-to-file resolution and non-following recursive traversal keep
  their distinct existing behavior.
- Missing candidates fall through, while permission and other metadata errors
  remain typed and identify the candidate.
- `~` is home-pinned on macOS, Linux, and Windows; `~user` is rejected.
- Environment interpolation follows the selected OQ1 decision, including
  authored/effective-kind diagnostics.

### Claudine L1

- Proxy resolution delegates to `FileReference` and returns the shared typed
  diagnostic on failure.
- Every proxy orchestration route produces the same result for the same
  source/context/reference tuple.
- A nested proxy uses the proxied document as the source for its own targets.
- External sequences and composition sources honor the same explicit/implicit
  contract.
- Existing external sequence `~` references resolve through `FileReference`
  with unchanged user-visible behavior.
- No production Claudine resolver manually classifies file-reference prefixes
  or directly joins/expands document reference strings.

### Claudine/Darkmatter L2 — terminal-rendering claims only

Level 2 is reserved for behavior that only a real terminal exercises: the
rendered failure block and its ordered candidate list. Everything else in this
feature is deterministic in-process resolution, which Level 1 verifies with full
fidelity and none of the terminal flakiness (see the L1 note below).

- The motivating `prompts/implement.md` router successfully resolves the bare
  `prompts/_implement/implement-suggestions.md` reference.
- A paired fixture proves `./prompts/_implement/implement-suggestions.md`
  remains source-relative and fails when that exact source-local path is absent.
- A no-color/TTY implicit no-match renders the ordered attempted candidates in
  repository-then-source order through the typed error pipeline (the two-candidate
  ordered capture, not just the repository winner).

### Cross-surface parity and completion round trip — L1 process integration

The remaining strategy items are **resolution semantics**, not terminal
rendering, so their strongest faithful verification is Level 1. Every
document-backed Claudine/Darkmatter surface — lifecycle `proxy`, composition and
sequence sources, schema `file(...)`, expression `file(...)`, transclusion, and
local link resolution — builds its context through the single
`document_resolution_context` seam and resolves through
`FileReference::resolve_in_context`. Cross-surface parity is therefore a property
of that one shared builder; a shared-fixture Level 2 matrix would drive an
identical in-process algorithm through five terminals for no added fidelity.
Ordered rendering is the only part that genuinely needs a terminal, and it is
covered above.

- Schema `file(...)`, expression functions, sequence references, and
  transclusions resolve shared fixtures identically. The shared seam is proven
  repository-first on a real collision fixture, and each surface has a
  per-surface L1 adapter test that routes through it; the Claudine transclusion
  collision test additionally discriminates end-to-end through `claudine compose`.
- Nested transclusion/schema fixtures prove each authored document supplies its
  own base without losing the request's worktree context.
- A value emitted by completion resolves unchanged through the same candidate
  builder: completion consumes the request-scoped `FileResolutionContext`
  (`complete_partial_in_context`), and an L1 round trip enumerates an emitted
  value and executes it through `resolve_in_context` to the same file — including
  a magic reference through a configured magic root shared with execution.

## Acceptance Criteria

1. `FileReference` is the only file-reference syntax authority used by
   Claudine production code.
2. Explicit `./` and `../` references resolve only from the source/base and
   never fall back.
3. Implicit bare references resolve repository-root first and source/base
   second, with deterministic first-existing-file behavior.
4. The motivating router reference resolves successfully without adding `@`
   or rewriting it to `./`.
5. Lifecycle proxy resolution is identical across every supported
   orchestration route.
6. Composition sources, external sequences, schema/file values, expression
   functions, and transclusions use the unified contract when run by Claudine.
7. `biscuit-file` documentation, skill guidance, implementation, completion,
   and tests agree on the ratified terminology and precedence.
8. Missing references return typed candidate-aware diagnostics that the
   error-propagation pipeline can render and expose as `err.detail.*`.
9. macOS, Linux, and Windows absolute/reference behavior is covered and no
   implementation depends on POSIX-only prefix checks.
10. `just test` and `just lint` pass in `biscuit-file`, `darkmatter`, and
    `claudine`; relevant L2 suites pass in each affected package area.
11. Existing sequence `~` references retain home-pinned behavior through the
    shared grammar, including on native Windows.
12. Document-backed Claudine/Darkmatter resolution consumes an explicit,
    request-scoped context and performs no late ambient CWD, HOME, environment,
    repository, or package-area discovery.
13. Candidate probing distinguishes absence from permission/I/O failure, and
    detailed no-match diagnostics retain ordered candidate/root provenance.
14. Nested documents use their own source directory for references they author
    while retaining the request's explicit repository/launch context.
15. Every workspace caller of the changed `resolve()`/`resolve_from()` defaults
    is audited and either migrates to repository-first behavior or selects a
    documented, explicit shared transition policy.

## Documentation and Migration

- Update `biscuit-file/docs/topics/file-references.md` and the biscuit-file
  skill reference with the ratified implicit precedence and document-relative
  examples.
- Update Claudine lifecycle, composition, sequence, and system-prompt docs to
  use the explicit/implicit terminology consistently.
- Update Darkmatter file-reference/transclusion documentation wherever it
  currently claims that every plain relative path is source-relative.
- Prefer implicit repository-shaped examples when portability is intended and
  `./` examples when source-local intent is deliberate.
- Audit existing prompt documents. Rewrites are needed only where their current
  bare spelling relied on source-first precedence and a repository-root file of
  the same name now shadows it; those cases should become explicit `./` paths.
- Update the existing Darkmatter tests and comments that explicitly ratify
  document-first/launch-fallback behavior; leaving those comments in place
  after the code change would be contract drift.
- Document `~` as the shared home-pinned form and remove Claudine's private
  sequence expansion from implementation and documentation.
- Record the precedence change as a behavior change in the relevant package
  timelines and release notes.
