---
status: draft
created: 2026-09-17
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
related:
  - ../../docs/topics/file-referencing.md
  - ../../../biscuit-file/docs/topics/file-references.md
  - ../../../messenger/features/2026-09-17-research-metadata-pipeline/spec.md
---

# Hyperlinks Must Use Darkmatter File-Resolution Semantics

## Summary

Markdown hyperlink targets are file references. Darkmatter must resolve their
local path portion through `biscuit_file::FileReference` with the same captured
request context used by composition, transclusion, expressions, schemas, and
other file-valued surfaces.

That contract is currently broken in two places:

1. `md validate refs` can load a document by a relative CLI path and then try
   to construct repository-aware resolution state from the document's relative
   parent. A valid repository-scoped (`^`) link consequently fails with
   `OutsideRepository`, even when the command was launched inside the
   repository containing the document.
2. DMLS treats every non-URL Markdown hyperlink as an ordinary relative path
   and resolves it with a lexical join. It therefore cannot follow `^`, `&`,
   `@`, home, vault, recursive, or implicit repository-fallback references,
   and its navigation and broken-link diagnostics can disagree with
   Darkmatter's CLI and library.

This fix establishes one hyperlink-target resolution authority shared by
reference validation and DMLS. It does not introduce a second path grammar in
either consumer.

## Reproduction

From the `messenger` package-area directory:

```sh
md validate refs ./features/2026-09-17-research-metadata-pipeline/spec.md
```

The document contains this valid link:

```markdown
[Agentic Research as a Typed Knowledge Pipeline](^docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md)
```

Observed on macOS on 2026-09-17:

```text
ReferenceError: file reference failure

`^` repository reference requires a repository containing reference CWD `./features/2026-09-17-research-metadata-pipeline`
```

The process CWD is inside the repository, the source document is inside that
same repository, and `^docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md`
resolves from the Messenger package area. The error is therefore a context-
construction defect, not an invalid reference.

In DMLS, opening the same document produces the parallel failure: definition
navigation and `textDocument/documentLink` do not resolve the `^` target,
because the workspace graph currently joins the literal string beginning with
`^` onto the source document's directory.

## File-reference contract

### One syntax authority

For a local Markdown hyperlink, the path portion before an optional fragment
MUST be parsed by `biscuit_file::FileReference`. Darkmatter and DMLS MUST NOT
classify sigils with prefix checks or reproduce candidate ordering with custom
joins.

The supported local forms are the forms supported by `FileReference`:

| Authored form | Required behavior |
|---|---|
| `./x.md`, `../x.md` | Resolve only from the authoring document directory. |
| `x.md` | Try the authoring document directory, then the repository root. |
| absolute path | Resolve the authored absolute path. |
| `~/x.md` | Resolve from the captured home directory. |
| `@x.md` | Use configured magic roots and the captured package, package-area, repository, and home roots in `FileReference` order. |
| `&x.md` | Resolve only from the captured repository root and enforce repository containment. |
| `^x.md` | Try captured package, package-area, and repository roots in `FileReference` order and enforce repository containment. |
| `vault:x.md` | Use the captured/configured vault roots. |
| `%...` | Preserve `FileReference` recursive resolution semantics. |
| `{{VAR}}` in a local reference | Interpolate from the captured environment snapshot according to `FileReference` rules. |

Malformed reserved forms remain typed syntax failures. A missing well-formed
target remains a no-match. Repository escape, missing context, I/O failure,
and unsupported-local-remote cases remain distinct typed failures; consumers
must not flatten them all into "missing file."

### Fragments and external targets

Markdown fragments are navigation metadata, not part of the filesystem path.
For a target such as `^docs/topic.md#design`, resolution MUST:

1. preserve the authored target for diagnostics and rendering;
2. classify external/non-file targets before local resolution;
3. split the local file-reference portion from the fragment without corrupting
   URL syntax or cross-platform paths;
4. resolve the file-reference portion through `FileReference`; and
5. resolve `design` against the target document only when fragment validation
   or editor navigation requires it.

Fragment-only links such as `#design` remain same-document links. HTTP(S),
protocol-relative URLs, `mailto:`, `tel:`, `data:`, and other supported
non-file schemes retain their existing classification and MUST never be sent
through local file resolution. DMLS MUST NOT fetch a remote target.

## Request-context invariants

1. A request captures repository, package/package-area, home, environment,
   magic, and vault state once at its boundary.
2. The authoring base for each link is derived with `for_source` from that
   captured context. Nested documents do not rediscover ambient state.
3. The source path used for context derivation is absolute and normalized
   enough to belong to the captured repository boundary. A relative CLI
   spelling remains presentation data, not repository identity.
4. A repository-aware reference never discovers from the source document's
   relative parent.
5. Resolution must be deterministic if the process CWD, HOME, or environment
   changes after request capture.
6. Windows drive, UNC, separator, and case behavior remains owned by
   `biscuit-file`; Darkmatter MUST NOT normalize paths with string replacement.
7. Validation and editor analysis remain passive: they may perform the local
   reads needed to resolve and index files, but they execute no expressions or
   shell commands, fetch no remote content, and mutate no document.

## Required changes

### Darkmatter library

Add or expose one focused hyperlink-target resolver that accepts:

- the authored target;
- the authoring source path;
- an explicit `FileResolutionContext`; and
- the remote/local policy needed by the caller.

Its result must retain the authored target, parsed local reference kind,
optional fragment, resolved local path or remote target, and typed failure/no-
match state. The exact public/private type boundary is an implementation
choice; the semantic result must be reusable without either consumer
re-parsing the target.

Reference graph construction and reference validation MUST use this authority.
Transclusion resolution may continue through its existing shared
`FileReference` path, but hyperlink handling must have the same candidate
ordering and captured-context behavior.

Compatibility APIs that capture ambient state may remain for simple top-level
callers. Request-scoped APIs and long-lived DMLS state MUST use explicit
contexts.

### `md validate refs`

The CLI MUST capture file-resolution state at the command boundary, resolve
the input document to an absolute source identity, derive the document context
from the captured request context, and pass it into `ReferenceGraphOptions`.

This applies both to ordinary validation and the `--graph` branch. Other CLI
surfaces that build or validate the same reference graph, including
`md graph --validate`, MUST share the same boundary behavior rather than
reconstructing it independently.

The regression command above must succeed when all references exist,
regardless of whether the input is spelled relative to the package area,
relative to the repository root, or as an absolute path.

### DMLS navigation and diagnostics

DMLS MUST capture a stable file-resolution context for each initialized
workspace/repository boundary and derive a document context for each indexed or
open source file. It must not use the language server process CWD as the
semantic base for an open document.

The Markdown substrate must use the shared hyperlink-target result for all of
these surfaces:

- workspace graph `references` edges;
- `textDocument/definition`;
- `textDocument/documentLink`;
- broken-path and missing-fragment diagnostics;
- dependency and invalidation tracking for resolved local targets.

For a resolved local Markdown document, definition navigation opens the target
document, or the target heading when a fragment is present. A document link
returns a `file://` URI built from the resolved path and preserves a fragment
when present. A local file that is resolvable but is not a Markdown graph node
may still produce a document link; absence from the Markdown index is not the
same as a missing filesystem target.

For a missing or malformed target, DMLS returns no navigable target and emits
the existing applicable diagnostic with the typed resolution reason. It MUST
not publish a `dm.link.broken_path`-style diagnostic for a `^`, `&`, or `@`
reference merely because the literal sigil-prefixed string does not exist
beside the source document.

Multi-root workspaces select the captured context that contains the source
document. Roots from unrelated workspace folders are not implicit magic or
fallback candidates. A document outside every initialized workspace may use
forms whose required context is available from its absolute URI; repository-
dependent forms remain unresolved with a missing-context reason unless an
explicit trusted context was captured for that document.

## Scope boundaries

### In scope

- Markdown inline and reference-style hyperlinks extracted by Darkmatter's
  reference subsystem.
- Local hyperlink validation, cross-document fragment validation, graph
  construction, and graph-backed CLI output.
- DMLS definition navigation, document links, diagnostics, dependency edges,
  and incremental invalidation for those hyperlinks.
- All `FileReference` local forms and their typed errors.

### Out of scope

- Changing `FileReference` grammar or candidate order.
- Fetching or indexing remote hyperlinks in DMLS.
- Changing wiki-link identity/resolution rules; wiki links are a separate
  authoring syntax with workspace-specific matching semantics.
- Treating rendered browser URLs as file references.
- Adding a new CLI flag or a second Darkmatter path syntax.

## Verification

### Library tests

Use temporary repositories and explicit `FileResolutionContext` fixtures to
cover every row of the file-reference table, with and without fragments.
Tests must prove document-first implicit resolution, repository fallback,
package/package-area ordering for `^`, exact repository-root behavior for
`&`, configured magic/vault roots, recursive resolution, missing context,
repository escape, and a CWD/environment change after capture.

At least one test must load the root document through a relative path and
prove that its source identity and repository context become absolute and
stable before hyperlink resolution.

### CLI integration tests

Launch through `CliProcessFixture`. Build a fixture repository with a nested
package-area document linking through `^` to a real Markdown file, then run:

- `md validate refs` with a relative input from the package area;
- the same command from the repository root;
- the same command with an absolute input;
- `md validate refs --fragments` for a `^...#heading` target;
- `md validate refs --graph mermaid` or `dot`; and
- the existing graph-validation surface that shares this path.

The tests must not inherit ambient HOME, repository, CWD, or environment state
beyond the fixture's declared policy.

### DMLS protocol tests

Open real fixture documents through the normal LSP request path. For at least
`./`, bare repository fallback, `^`, `&`, and `@` links, assert both
`textDocument/definition` and `textDocument/documentLink` targets. Include a
cross-document fragment, a missing target, a repository escape, and a target
created/changed/deleted after initial indexing.

The incremental tests must prove that dependency edges and navigation update
without restarting DMLS and that an unrelated workspace root does not become
a candidate. Protocol assertions operate on URIs and ranges; they do not drive
or focus a real editor window.

### Cross-platform proof

All unit and protocol fixtures use portable path construction and run in L1 on
macOS, Linux, native Windows, and WSL2. Include Windows-specific coverage for a
drive-qualified absolute target and separator handling where the shared
`FileReference` contract differs from POSIX. No test may assert a Unix-only
leading slash or manually replace `\\` with `/`.

Run from the Darkmatter package area:

```sh
just test
just lint
```

No L2 test is required: the behavior is deterministic library, CLI-process,
and LSP-protocol behavior and does not need a real terminal or editor window.

## Risk and implementation constraints

GitNexus impact analysis on 2026-09-17 reported:

- `validate_local_path`: LOW risk, feeding the reference validation and file-
  tree paths;
- `classify_link_target`: MEDIUM risk, feeding substrate indexing and provider
  behavior; and
- DMLS `resolve_link`: CRITICAL risk, because workspace graph assembly feeds
  the server's main execution flow and incremental index operations.

Implementation must therefore preserve graph determinism, document IDs,
heading edges, wiki behavior, reverse indexes, and incremental invalidation.
A direct replacement of `normalize_join` inside `resolve_link` without
request-context ownership and graph regression coverage does not satisfy this
spec.

## Acceptance criteria

- **AC1 — observed regression:** the Messenger reproduction command succeeds
  and validates the `^docs/topics/...` hyperlink.
- **AC2 — shared semantics:** local Markdown hyperlinks use
  `biscuit_file::FileReference`; no consumer-owned sigil classifier or
  candidate-order implementation remains on the validation/navigation path.
- **AC3 — stable context:** relative and absolute spellings of the same source
  produce the same result, and later ambient CWD/environment changes do not
  affect a captured request.
- **AC4 — complete local forms:** explicit, implicit, absolute, home, magic,
  repository-root, repository-scoped, vault, recursive, and interpolated local
  references retain `FileReference` behavior and typed errors.
- **AC5 — fragments:** a resolved local target with a fragment validates and
  navigates to the target heading; fragment-only and external links retain
  their existing behavior.
- **AC6 — DMLS follow-through:** both definition navigation and document links
  follow `^`, `&`, and `@` targets, while broken-link diagnostics and graph
  invalidation use the same resolved identity.
- **AC7 — passive safety:** validation and DMLS perform no shell execution,
  expression execution, remote fetch, or mutation.
- **AC8 — multi-root isolation:** unrelated workspace roots never become
  fallback candidates.
- **AC9 — cross-platform:** L1 coverage passes on macOS, Linux, native Windows,
  and WSL2 without platform-specific path string hacks.
- **AC10 — documentation:** Darkmatter and DMLS documentation describe
  Markdown hyperlinks as `FileReference`-resolved local targets and distinguish
  that behavior from external URLs and wiki links.
