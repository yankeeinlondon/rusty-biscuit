---
created: 2026-07-13
status: draft
reviewed: false
related:
    - ../2026-07-13-error-propogation/spec.md
---

# Unified File-Reference Resolution

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
| `vault:foo.md` | Vault | Existing configured-vault resolution |
| `%...` | Recursive | Existing recursive semantics over the underlying kind's roots |
| `http://...`, `https://...` | Remote URL | Existing surface-specific fetch policy; never treated as a local path |

The first existing file in the applicable ordered candidates wins.

This specification ratifies **repository root before source document** for
implicit document references. Current
[`biscuit-file` documentation](../../../biscuit-file/docs/topics/file-references.md)
and implementation describe implicit resolution as base/CWD before Git root.
That is contract drift and must be reconciled as part of this feature; Claudine
must not compensate with another private resolver whose behavior differs from
the shared authority.

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
  and platform-native absolute reference forms.
- Return typed, candidate-aware diagnostics for invalid and unresolved
  references.
- Make the behavior deterministic and testable on macOS, Linux, and Windows.

## Non-goals

- Changing the meaning or syntax of `@`, `!`, `vault:`, `%`, or environment
  interpolation beyond making all Claudine surfaces use the shared authority.
- Adding new recursive-search roots or following symlinks.
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
    Vault,
    Url,
}
```

The exact API is an implementation decision. Claudine must not duplicate the
grammar to obtain the classification.

### D2 — Resolution context is explicit data

Document-backed resolution receives a context containing at least:

```rust
pub struct FileResolutionContext<'a> {
    pub source_path: &'a Path,
    pub repository_root: Option<&'a Path>,
}
```

The shared API may use a source directory rather than full source path, but the
caller passes the context explicitly. Repository discovery happens once per
composition/harness run where practical and is reused; resolution must not
depend on later `set_current_dir` behavior.

For a file-backed source:

- `source_dir = source_path.parent()`;
- `repository_root` is the nearest Git root containing the source;
- a caller-provided root is accepted only when it contains the source or the
  operation explicitly documents a different trust boundary.

For a top-level CLI reference with no source document, the launch context is
used: repository root first, then launch directory. In-memory/stdin surfaces
without either context must return a typed missing-context error rather than
consulting an unrelated ambient CWD.

### D3 — Candidate order is kind-specific and observable

Candidate construction for local non-recursive references is:

```text
ExplicitRelative: [source_dir / authored_path]
ImplicitRelative: [repo_root / authored_path, source_dir / authored_path]
Absolute:         [authored_path]
Magic:            existing configured magic roots
Package:          existing package area or Git-root fallback
Vault:            existing configured vault roots
```

Duplicate lexical candidates are removed without changing the first-seen
order. The first candidate that is an existing file wins.

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

If compatibility concerns require a configurable policy rather than changing
the existing method immediately, the new policy must live in `biscuit-file`,
be the documented canonical mode for document references, and be adopted by
all Claudine/Darkmatter surfaces in scope. A Claudine-only candidate loop is
not acceptable.

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
- system-prompt file references when an explicit reference is supplied.

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

All routes receive the same source path and repository context and call the
same resolver. No route may use `PathBuf::join` directly for the target.

### D7 — Cross-platform absolute paths

Absolute-path detection uses platform-aware `Path::is_absolute` semantics and
must correctly recognize:

- POSIX roots on macOS/Linux (`/tmp/a.md`);
- Windows drive-qualified paths (`C:\\work\\a.md`);
- Windows UNC paths (`\\\\server\\share\\a.md`).

Tests that cannot execute a foreign platform's path semantics on the host may
use target-gated cases, pure parser fixtures, or CI runners. The author-facing
grammar and serialized diagnostics remain portable.

Path display must use platform-native formatting; persisted logical reference
strings retain their authored spelling.

### D8 — Typed resolution result and diagnostics

An unresolved reference is not represented as a formatted `detail: String`.
The shared result/error shape must retain:

- raw authored reference;
- parsed reference kind;
- source document/base;
- repository root when available;
- ordered candidates attempted;
- failure classification (invalid syntax, missing context, no match,
  permission/I/O, unsupported remote, and so on);
- underlying `FileReferenceError` where one exists.

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
- When both repo and source candidates exist, repo wins.
- When only the source candidate exists, implicit resolution falls back.
- When no Git root exists, implicit resolution uses only the base.
- Special reference kinds retain their existing behavior.
- macOS/Linux absolute paths and Windows drive/UNC paths are covered.
- Worktree and nested-repository roots are anchored to the supplied base.

### Claudine L1

- Proxy resolution delegates to `FileReference` and returns the shared typed
  diagnostic on failure.
- Every proxy orchestration route produces the same result for the same
  source/context/reference tuple.
- External sequences and composition sources honor the same explicit/implicit
  contract.
- No production Claudine resolver manually classifies file-reference prefixes
  or directly joins document reference strings.

### Claudine/Darkmatter L2

- The motivating `prompts/implement.md` router successfully resolves the bare
  `prompts/_implement/implement-suggestions.md` reference.
- A paired fixture proves `./prompts/_implement/implement-suggestions.md`
  remains source-relative and fails when that exact source-local path is absent.
- Schema `file(...)`, expression functions, sequence references, and
  transclusions resolve shared fixtures identically.
- Completion-produced references execute successfully without rewriting.
- No-color/TTY errors show the ordered attempted candidates through the typed
  error pipeline.

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
- Record the precedence change as a behavior change in the relevant package
  timelines and release notes.
