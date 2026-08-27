---
status: draft
created: 2026-08-27
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-27
area: claudine
packages:
    - biscuit-file
    - darkmatter
    - claudine
depends-on: ../../fixes/2026-08-12-ctx-launch-anchor/spec.md
---

# Finalized file-reference grammar: one sigil catalog, one CWD model

## Summary

The design-intent document `claudine/docs/topics/file-referencing.md` defines
the target grammar and resolution semantics for file references across
Claudine, Darkmatter, and biscuit-file. Today's implementation diverges from
that target in specific, chronicled ways (`sigil-delta.md` in this feature
directory). This specification closes the gap: it adds the `&` and `^`
sigils, removes the `!` sigil, flips the implicit-relative candidate order to
match the target, re-anchors multi-homed sigil bases to the reference's own
scope, materializes caller-passed file parameters as anchored values, adds
`ctx.cwd`, and sets `AGENT_CWD` in the environment of every spawned child
process. It also makes the repo-only claim of `&` and `^` enforceable:
their payloads and resolved targets may not escape the repository.

The design-intent document is normative for semantics. Where this
specification restates its rules, it does so to make them implementable and
testable; where the two disagree, the design-intent document wins and this
specification must be corrected.

There are no production users. No backward-compatibility shims, deprecation
periods, or migration tooling are in scope — call sites and fixtures are
updated directly.

> **Reader's note (review 2026-08-27):** The draft originally left source-scope
> discovery, repo-boundary enforcement, and lazy parameter materialization to
> implementation judgment. Those choices affect determinism and security, so
> this revision makes them explicit: explicit resolution remains snapshot-only,
> repo-only sigils enforce both lexical and resolved-target containment, and a
> lazy local parameter materializes from the first unprobed candidate while
> recursive lazy values are rejected. Both former open questions are now
> ruled (2026-08-27): OQ1 rejects and reserves the unsupported local
> namespace forms, and OQ2 sets `AGENT_CWD` on every spawned child (D8.7).
> No open design questions remain.
>
> **Layering pass (2026-08-27):** a final review of *where* each change lands
> found that repository/package-scope discovery is implemented three times
> today (biscuit-file via `cargo_metadata`, Darkmatter via
> `sniff::detect_repo_structure` per reference, Claudine via retained Sniff
> `RepoInfo`) and that Darkmatter still performs per-reference `.git` walks at
> its resolver sites. The new "Ownership and reuse boundaries" section fixes one
> owner per responsibility; D7, D8.7, Scope, AC4, and AC6 were tightened to
> match, and the unverified "existing environment inventory tests" claim was
> replaced with a concrete guard requirement.

## Baseline

This work directly depends on `2026-08-12-ctx-launch-anchor`. That fix is still
in an active lifecycle directory, so this specification does not describe it
as completed; the `depends-on` frontmatter requires its contract and acceptance
criteria to be complete before implementation starts. It established:

- prepared `ctx.*` values are projections of the immutable **launch context**
  (the caller's invocation directory and the repository/package facts derived
  from it), never the prompt's storage location;
- one early-binding context snapshot per document preparation epoch, shared by
  preflight, body, frontmatter, and lifecycle; and
- `SourceContext` as the authority for document/file resolution.

That fix's **AC10** locked the *current* repository-first implicit ordering as
a non-regression guard. Once the dependency is complete, this specification
deliberately supersedes that ordering (D3) and re-rules review-3 Finding 4 of
that fix accordingly: the finding's observation (document and implementation
disagreed) was correct, and the resolution is to change the implementation,
not the design-intent document. The implementation baseline must be green
before this feature changes it; this specification does not rely on an
unverified claim that every Claudine test is already passing.

## The CWD model (restated for implementation)

Three distinct directories exist per invocation. Every rule in this
specification is expressed against them:

- **Launch directory** — where the caller invoked Claudine. Captured once,
  immutable, reported by `ctx.cwd` (D8), and used by the established
  caller-scoped `ctx.area`, `ctx.current_package_area`, and
  `ctx.current_package` projections.
- **Composition CWD** — per the design document's Ruling: during composition,
  the effective CWD for a document's references is **the directory of the
  file being composed**. A proxy target, sequence task, or nested document
  becomes the active file for references it authors.
- **Process working directory** — the mutable OS working directory of the
  Claudine/provider/shell process. Never a resolution candidate for
  document-authored references.

**Exception clause (caller-passed parameters).** A file reference passed in
as a frontmatter parameter (`spec=...`) is resolved using the caller's launch
directory as its CWD. An `eager` `file(...)` parameter is resolved *and*
validated at bind time; a lazy `file` parameter is resolved to a path shape
without an existence check.

## Ownership and reuse boundaries

Each responsibility below has exactly one owning crate. The dependency
direction is fixed — `biscuit-file` ← `darkmatter` ← `claudine` — and a lower
layer never grows a dependency to satisfy a higher one.

| Responsibility | Owner | Consumers |
|---|---|---|
| Sigil grammar, detection order, `FileReferenceKind`, parse errors (D1, D2, D9) | biscuit-file `file_reference::parse` | everyone, only through `FileReference` — no prefix checks upstream |
| Candidate plans, first-match resolution, completion roots, `RootProvenance` (D3–D6, AC8) | biscuit-file `file_reference::resolve` | Darkmatter resolvers and expression functions; Claudine completion |
| Lexical **and** canonical repository containment for `&`/`^` (D4, D5, AC11) | biscuit-file — one helper, reused by direct, recursive, completion, and lazy-ancestor paths | Darkmatter must not add a second containment implementation; its `file_links` boundary check is a different contract and stays where it is |
| `RepositoryScopeCatalog` — the pure-data description of a repository's root, package-area roots, and package roots (D7) | biscuit-file (type only; **no I/O, no Sniff dependency**) | populated by Darkmatter, forwarded by Claudine |
| Sniff `RepoInfo` → `RepositoryScopeCatalog` projection (the single discovery adapter) | Darkmatter, at its request boundary next to the existing repository capture group | Darkmatter's ambient `md` path and Claudine's `derive_source` both call it; there is no second Sniff→catalog code path |
| Repository/topology *observation* (running Sniff, caching per repository) | Claudine `InvocationContext` for Claudine invocations; Darkmatter's request-boundary capture for standalone `md` | — |
| `ctx.cwd`, `ContextGroup::Invocation`, parameter materialization, raw-vs-effective value layering (D8.1–D8.6) | Darkmatter compose/context and schema layers | Claudine's prepared-context catalog projects them; Claudine adds no materialization logic of its own |
| `AGENT_CWD` and every other Claudine-owned child-environment contribution (D8.7) | `claudine` lib — one shared contribution helper | every `Command`-spawning site in `claudine` lib **and** `claudine-cli`'s `build_child_env` |

Retirements that follow from this table:

- biscuit-file's `find_package_area` (a `cargo_metadata` shell-out that only
  understands Cargo workspaces) is deleted along with the `cargo_metadata`
  dependency of the `file-reference` feature; package selection comes only
  from the supplied catalog. `find_git_root` and
  `ResolutionContext::from_ambient` remain for biscuit-file's own ambient
  convenience API and the `bf` CLI, but neither Darkmatter nor Claudine may
  use them to populate a `FileResolutionContext`.
- Darkmatter's `compose/util.rs::find_package_area_from` and
  `package_area_for_reference` are deleted. The per-reference
  `find_git_root_from` fallbacks in `document_resolution_context`, the
  transclusion resolver, `link_resolve`, `link_normalization`,
  `schema_validation`, `schemas/resolve`, `expression/path_projection`, and the
  `git_root`-style expression functions are replaced by reads from the
  request's catalog. `find_git_root_from` may survive only for display-only
  helpers such as `abbreviate_path`, which never feed a resolution candidate.
- Claudine's `derive_source` stops computing `package_root` /
  `package_area_root` from `RepoInfo` itself and calls Darkmatter's projection
  on its retained observation, so Claudine and standalone `md` cannot disagree
  about which package contains a document.

## Design decisions

### D1 — The final sigil catalog

| Spelling | Meaning | Candidate order |
|---|---|---|
| `./x`, `../x` (and `.\`, `..\`) | Relative to the composition CWD | Single candidate |
| bare `path/to/x` | Implicit relative | Composition CWD, then repo root (D3) |
| absolute (`/x`, `C:\x`, UNC) | Verbatim | Single candidate |
| `~/x` | User home | Single candidate; `~user` rejected |
| `@x` | Magic (multi-homed, may leave the repo) | Registered prepends → package → package area → repo root → home → registered appends (D6) |
| `&x` | Repo root, exactly, and repo-contained | Single candidate; invalid outside or escaping a repo (D4) |
| `^x` | Repo-bounded, most-specific-first | Package → package area → repo root; invalid outside or escaping a repo (D5) |
| `vault:x` | Configured vaults | Registered roots, then captured `VAULT` roots; retained unchanged while the design document treats enhancements as future scope |
| `http://`, `https://` | Typed remote reference | Never a local candidate (unchanged) |

Modifiers, unchanged from today: a single leading `%` switches a local kind to
recursive search; `{{VAR}}` environment interpolation applies within every
kind, with the existing post-interpolation anchoring re-derivation and
sigil-injection rejection. Remote references remain remote when prefixed by
`%`; the modifier does not turn a URL into a local traversal. `{{VAR}}` remains
the only interpolation grammar — the design document's `${HOME}` /
`%USERPROFILE%` passages are conceptual analogies, not supported reference
syntax.

`@`, `&`, and `^` are defensively coded: exactly one `/` following the sigil
is consumed (`&path` ≡ `&/path`); a payload that remains rooted after that is
rejected with `InvalidSyntax`. `/` is the only portable optional sigil
separator; `@\x`, `&\x`, and `^\x` are rejected on every host rather than
having host-dependent meaning. Empty payloads are also `InvalidSyntax`. This
closes the existing gap where `!/foo` resolved as absolute `/foo` and keeps the
grammar identical on macOS, Linux, and Windows.

### D2 — Remove the `!` sigil

The `!` (package) reference kind is deleted from the grammar. Rationale:

- Ken's ruling (2026-08-27); the design document no longer defines it.
- `^` covers its useful behavior with a superset (most-specific-first,
  repo-bounded).
- `!` is already the logical-NOT operator in the expression language
  (`when="!file_exists(x)"`), so the same character carried two unrelated
  meanings in adjacent grammars.
- Unquoted `!` triggers history expansion in interactive shells, making it
  hazardous to type as a CLI argument.
- A repo-wide audit (2026-08-27) found zero `!` file references in existing
  prompts — every `!` occurrence is expression negation — so removal has no
  migration burden.

After removal, a reference beginning with `!` is a **parse error**
(`InvalidSyntax`, naming the removed sigil and suggesting `^`), not an
implicit-relative path with a literal `!` — see D9.

### D3 — Implicit relative: composition-CWD first, repo root second

Per the design document's dual-pathed approach, a bare
`path/to/file.md` resolves through this closed candidate list:

1. `{composition-CWD}/path/to/file.md` — the active document's directory for
   document-authored references; the launch directory for caller-passed
   parameters (exception clause);
2. `{repo-root}/path/to/file.md`, when the composition CWD is inside a
   repository.

Outside a repository, candidate 1 is the only candidate. When the two bases
coincide, they collapse to one. Direct and eager resolution select the first
existing regular file and return `Ok(None)` on a miss; D8 defines the distinct
no-existence-check projection for a caller-supplied lazy `file` value.

This **reverses today's order** (repository first, source second). The
existing conflict fixtures that lock repository-first — cited by
ctx-launch-anchor AC10 — are updated to lock the new order instead. The
shadowing consequence is intended: a document sitting next to its own copy of
a file gets that copy, not the repo root's.

### D4 — `&`: repo root, exactly

- `&file/to/path.md` resolves as `{repo-root}/file/to/path.md`. One
  candidate, no fallback, packages and package areas not consulted.
- Using `&` when the reference's CWD is not inside a repository is a typed
  error (new `FileReferenceError` variant naming the sigil and the CWD), not
  a miss.
- The normalized payload must remain lexically inside the repository root;
  `&../outside.md` is a typed repository-escape error. When a candidate exists,
  its canonical target must also remain inside the canonical repository root,
  so a final symlink, junction, or reparse point cannot turn `&` into a path
  outside the repository. The containment check is specific to `&`/`^`; other
  reference kinds retain their current symlink behavior.
- Documentation must note that `&` is a shell control operator and show the
  quoted form for CLI usage (`spec='&docs/plan.md'`).

### D5 — `^`: repo-bounded, most-specific-first

- Candidate order: the package root containing the reference's CWD (when
  inside a package of a monorepo), then the package-area root (when inside a
  package area), then the repo root. Missing levels are skipped; duplicates
  collapse.
- Every candidate is subject to D4's lexical and resolved-target containment
  checks. An escape is a typed error and stops resolution; it does not advance
  to the next scope.
- Never consults the user's home directory — by design, so a repo-scoped
  agent operating with repo-level permissions cannot be walked into
  user-scoped files by a reference.
- Using `^` outside a repository is the same typed error as `&`.
- There is **no automatic rewriting** of implicit relative paths into `^`
  (that clause was removed from the design document on 2026-08-27). Implicit
  paths follow D3; `^` is always author-explicit.

### D6 — `@`: intrinsic scope chain plus registered convention roots

The sigil's intrinsic base list becomes the design document's order:

1. package root (when the reference's CWD is inside a package),
2. package-area root (when inside a package area),
3. repo root (when inside a repo),
4. the user's home directory.

Application-registered magic roots remain supported as an extension
mechanism: prepend roots are searched before the intrinsic list, append roots
after it, exactly as biscuit-file's `PathPosition` works today. Claudine's
convention registrations (`prompts/` directories, `docs/`, peer-agent skills
directories, `~/.claudine/prompts`) continue through that mechanism and are
not part of the sigil's definition. Because prepend roots intentionally outrank
the intrinsic package root, the documentation and collision fixtures must show
Claudine's complete *effective* order, not only biscuit-file's intrinsic order.
The package root, package-area root, repository root, and home directory are no
longer also registered as convention roots; they come from the intrinsic list
exactly once.

The skill-lookup example from the design document must hold verbatim:
`@.claude/skills/name/SKILL.md` finds the repo's copy first and falls back to
`~/.claude/skills/name/SKILL.md`.

### D7 — Multi-homed bases derive from the reference's own scope

For `@` and `^` (and the implicit candidates of D3), the package and
package-area bases are derived from the **reference's CWD** — the composition
CWD for document-authored references, the launch directory for caller-passed
parameters. They are *not* taken from a launch-time snapshot that rides along
regardless of which document authored the reference.

This does **not** authorize late discovery in biscuit-file or Darkmatter.
Explicit resolution remains snapshot-only:

- `FileResolutionContext` gains a distinct package-root anchor and a
  caller-supplied `RepositoryScopeCatalog` (a biscuit-file data type — see
  "Ownership and reuse boundaries") sufficient to select the package root and
  package-area root containing a derived base. `for_source` and `for_base`
  recompute source-specific anchors from that catalog by component-aware,
  most-specific containment; they must not blindly copy the previous
  document's package anchors.
- Claudine's `InvocationContext::derive_source` remains the *observation*
  owner: it runs and caches Sniff per repository and builds each definitive
  `SourceContext`. The catalog itself comes from Darkmatter's single
  `RepoInfo` → `RepositoryScopeCatalog` projection, the same function
  Darkmatter's standalone `md` path uses at its request boundary. A source in
  another repository gets a context for that repository from the invocation
  cache; no source inherits the launch repository merely because the launch
  context found it.
- A trusted external derivation not covered by a supplied repository catalog
  clears repository/package/package-area anchors. It therefore gets the
  documented outside-repository behavior unless the composition owner supplies
  a definitive context for that external repository.
- Source-relative Claudine convention roots are recomputed with the same source
  scope. They are not stored as launch-derived absolute prepend roots that a
  nested document can inherit accidentally. Request-stable roots such as the
  captured home directory and explicitly application-global append/prepend
  roots remain stable.

The public provenance vocabulary is made unambiguous: retire the current
`RootProvenance::Package` meaning (which actually denotes a package area) in
favor of distinct `PackageRoot` and `PackageArea` variants. The new public
reference kinds are `FileReferenceKind::RepositoryRoot` for `&` and
`FileReferenceKind::RepositoryScoped` for `^`; internal kind names follow the
same distinction.

Consequence: a reference in `darkmatter/lib/docs/guide.md` searches
darkmatter's package/area scopes no matter where the operator launched; a
caller-passed `^`/`@` parameter searches the caller's scopes. A document
outside any repository contributes no package/area levels (its `^` is the
typed outside-repo error; its intrinsic `@` chain skips to home while any
application-global registered roots retain their D6 positions; its implicit
reference is CWD-only per D3).

This is the file-reference counterpart of the ctx-launch-anchor separation:
`ctx.*` is always caller-scoped; file references are always scoped to the
document (or, for parameters, to the caller who authored them on the command
line). Nothing crosses implicitly — a document that wants caller-scoped
paths uses `ctx.*` interpolation or derives from a caller-passed parameter
(D8).

### D8 — Caller-scope tooling: `ctx.cwd` and materialized parameters

1. **`ctx.cwd`** is added to the prepared context catalog: the caller's
   launch directory as an absolute path. It uses the same invocation-owned
   launch snapshot as the established caller-scoped package/area catalog
   values and is immutable for the entire invocation, including fresh
   retry/resume epochs. Darkmatter's ambient compatibility entry points
   capture their process CWD once at the request boundary; downstream
   composition never calls `current_dir()` to populate it. Path text uses
   biscuit-file's portable-path conversion rather than ad hoc separator
   replacement, while remaining an absolute path that expression functions
   can consume on the current host. `ctx.cwd` belongs to a new no-I/O
   `ContextGroup::Invocation`, not `ContextGroup::Repo`, so asking for it does
   not trigger repository capture and it remains available outside a repo.
   Claudine treats launch-CWD capture failure as its existing invocation error;
   Darkmatter's ambient compatibility path projects `null` plus the existing
   partial-capture diagnostic if `current_dir()` fails.
2. **Only schema-typed values materialize.** A string is treated as a file
   parameter only when the effective SimplifiedSchema selects a `file` or
   `file(eager)` arm for that caller-supplied property. Ordinary string
   overrides are never parsed or probed. Arrays and schema unions follow the
   selected file arm recursively, matching the existing eager-file behavior.
3. **Origin decides the anchor.** CLI key/value and `--set` file values use the
   immutable launch file-resolution context. A document's own frontmatter and
   defaults use that document's `SourceContext`. A `proxy.with` file value is
   evaluated and materialized in the proxying source document before handoff;
   a sequence task parameter uses the sequence document that authored it.
   Once materialized, an absolute value is not re-anchored by a proxy target,
   retry, resume, loop iteration, or sequence task.
4. **Eager values.** `file(eager)` probes the complete ordered candidate list,
   requires an existing regular local file, and materializes the winning
   absolute native path. The effective frontmatter/lifecycle value keeps that
   native identity; Markdown presentation uses the existing portable sidecar.
5. **Lazy values.** A non-recursive local `file` value builds the unprobed
   candidate plan and materializes its first candidate as a lexically
   normalized absolute path. It does not probe for the first existing file and
   absence is not an error. The `&`/`^` containment check is the sole safety
   exception: it inspects an existing target or deepest existing ancestor to
   reject a symlink/junction escape without turning a missing final file into
   validation failure. This makes the rule deterministic: for a caller-passed
   implicit value the launch-directory candidate wins even when it does not
   exist; for `@`, `^`, or `vault:` the first configured/intrinsic root wins
   without fallback probing. Authors who want existence-based multi-root
   selection must declare `file(eager)`.
   A recursive lazy value has no single path shape without filesystem I/O and
   is therefore a typed parameter-binding error with a suggestion to use
   `file(eager)`; a lazy HTTP(S) value remains a typed remote reference and is
   not converted to a local path.
6. **Raw input and effective value remain separate.** Canonical preparation
   retains the caller's raw override plus its origin context in the input
   layer so a fresh epoch can reapply schema selection, but every downstream
   expression, body, lifecycle, proxy, and launch-plan consumer sees the
   materialized effective value. No downstream consumer reparses the raw
   relative string.
7. **`AGENT_CWD` for spawned children** *(OQ2, ruled 2026-08-27)*. Every
   Claudine-spawned child process — provider CLIs, hooks, and `::shell`
   commands — receives the `AGENT_CWD` environment variable set to the
   captured absolute launch directory: the same immutable invocation state
   behind `ctx.cwd`. Claudine **overwrites** any inherited `AGENT_CWD` so a
   nested invocation cannot leak a stale value, and the value is stable
   across retry, resume, loop, and sequence re-entry. The un-namespaced name
   is the design document's ruling; the overwrite rule protects Claudine's
   own children, and the residual risk — an unrelated tool reading
   `AGENT_CWD` with different expectations — is accepted and must be noted in
   the environment documentation.
   The variable is contributed by **one** helper in the `claudine` lib that
   every child-spawning site calls — today that is at least the provider
   launch (`claudine-cli` `build_child_env`), hook runners
   (`dispatch/runner/bash.rs`), `::shell` execution (`harness/shell.rs`,
   `composition/sequence/task/shell.rs`), and the lifecycle executor — rather
   than N independent `.env("AGENT_CWD", …)` insertions. Only the provider
   seam has an environment assertion today (`debug_assert_child_env`); a
   spawn-seam inventory guard in the style of `dispatch_inventory.rs` must
   fail when a `Command` construction in `claudine` lib or CLI bypasses the
   shared helper.

This extends the existing eager-`file` normalization so that derivation and
proxy boundaries cannot strand a value without its anchor. The 2026-08-26
`CompositionError` (a success-guard crash on a `dirname(spec)`-derived relative
path in a proxied review prompt) is the canonical regression this must prevent:
the agreed authoring pattern for "other files in the caller's directory" is
`{{ parent_dir(spec) }}/other-file.md`, and it must work from any document in
the composition chain.

### D9 — Reserved introducers and schemes are rejected, not filename characters

Detection has one documented, host-independent order. After stripping at most
one leading `%`, the parser recognizes HTTP(S), `vault:`, `@`, `&`, `^`, the
removed `!`, `~`, supported absolute forms, the unsupported-scheme guard,
explicit-relative forms, then an implicit path. The exact rules are:

- A reference beginning with reserved punctuation `@`, `&`, `^`, `~`, `!`, or
  `%` is either valid syntax for that introducer or `InvalidSyntax`. A second
  leading `%` is invalid rather than a recursive filename.
- Drive-absolute paths are classified before the generic scheme guard. Any
  other leading RFC-scheme-shaped prefix (`[A-Za-z][A-Za-z0-9+.-]*:`) must be
  one of the supported schemes or a typed unsupported-scheme parse error —
  `file:` included, per OQ1's ruling. This prevents `file:/...`,
  `C:relative`, and misspelled schemes from silently becoming implicit paths
  while preserving `C:/absolute` and `C:\absolute`.
- `!x` has a dedicated removed-sigil diagnostic suggesting `^`; it does not
  fall through to an implicit filename.

These rules reserve concrete syntax rather than claiming that every unknown
future sigil is already reserved. Filenames legitimately beginning with a
reserved character are reachable via `./` (for example,
`./!weird-name.md`). A leading scheme-shaped token is not portable as an
implicit filename; a POSIX-only colon-bearing filename remains reachable with
an explicit-relative spelling such as `./name:part`.

### D10 — What does not change

- `%` recursive search, `{{VAR}}` interpolation semantics, `http(s)://`
  remote typing, absolute-path forms, `~` home pinning and `~user` rejection.
- `vault:` — implemented behavior is retained but stays documented as future
  scope; no new work.
- Supported native absolute paths remain unchanged. The `file:` URI and
  Windows device-prefix forms are rejected and reserved (OQ1, ruled
  2026-08-27); Windows drive-relative forms are rejected by D9 rather than
  treated as implicit.
- `$schema` resolution stays document-relative and outside the implicit
  candidate order.
- The launch directory remains diagnostic-only for document-authored
  reference *fallback* (no revival of the old launch-directory candidate);
  caller anchoring flows only through the exception clause, `ctx.*`, and
  materialized parameters.
- Prepared-`ctx.*` launch anchoring, epoch snapshots, and the capture-owner
  guard from ctx-launch-anchor.

## Scope

- `biscuit-file/lib/src/file_reference/`:
    - parse: add `&` and `^` kinds with defensive-slash handling; remove the
      `Package` kind; implement D9 reserved-introducer/scheme rejection; keep
      detection order well-defined and documented.
    - resolve: new candidate builders for `&`/`^`; flip
      `implicit_relative_roots` to CWD-first; intrinsic scope chain for `@`
      (D6) with registered prepend/append preserved; package/area selection
      driven only by the supplied source-scope catalog (D7); lexical and
      canonical containment for `&`/`^` (D4/D5).
    - context/error: add the package-root anchor and the pure-data
      `RepositoryScopeCatalog` input; delete `find_package_area` and the
      `cargo_metadata` dependency; ensure normal and trusted-external
      derivations cannot retain stale repository scopes; add distinct
      outside-repository and repository-escape errors rather than repurposing
      `MissingPackageContext`; replace ambiguous package provenance per D7;
      one shared containment helper for `&`/`^`; add direct (non-`%`)
      completion for `&`/`^` matching execution order.
- `darkmatter/lib`:
    - one `RepoInfo` → `RepositoryScopeCatalog` projection at the request
      boundary (beside the repository capture group), used by the ambient `md`
      path and exported for Claudine; delete `find_package_area_from` /
      `package_area_for_reference` and replace every per-reference
      `find_git_root_from` fallback in resolver code with catalog reads (see
      "Ownership and reuse boundaries" for the site list);
    - expression functions, transclusion, TOC linking, preflight, reference
      graphing, schema import/validation/rewrite, and completion consume the new
      grammar through `FileReference` rather than prefix checks;
    - caller-file binding handles both lazy and eager schema arms, retains raw
      origin separately from effective native/presentation values, and carries
      materialized values through proxy/lifecycle/re-entry boundaries (D8);
    - `ctx.cwd` is added to `darkmatter/docs/schemas/darkmatter.yaml`, the typed
      descriptor catalog, the no-I/O invocation capture group, context help,
      and every single-sourced projection; ambient compatibility captures once
      at the request boundary; and
    - resolver and schema documentation that states repository-first or
      eager-only caller normalization is updated.
- `claudine/lib` + `claudine/cli`: `ctx.cwd` in the prepared-context catalog
  inputs and its schema/single-sourcing projections; `derive_source` calls
  Darkmatter's catalog projection on its retained `RepoInfo` instead of
  computing package/area roots itself; anchor provisioning per D7
  (document-scoped bases for document-authored references, launch-scoped for
  parameters); convention magic-root registration reviewed against D6;
  sequence/harness/proxy/system-prompt/overlay surfaces consume the correct
  source context and materialized parameter values; one shared
  child-environment contribution helper carrying `AGENT_CWD`, wired into
  every spawn seam (providers, hooks, `::shell`, lifecycle executor, sequence
  shell tasks) plus a spawn-seam inventory guard, per D8.7. The only
  remaining biscuit-file ambient call in Claudine
  (`cli/src/commands/providers.rs` → `find_git_root`) is switched to the
  invocation context.
- Consumer audit: use compiler exhaustiveness plus repository search and
  GitNexus impact analysis. At minimum, audit all `FileReferenceKind`, internal
  `ReferenceKind`, `RootProvenance`, `FileReferenceError`, candidate-plan,
  completion-root, and `resolve_in_context` consumers. The known blast radius
  includes Darkmatter schema resolution/rewrite, transclusion, expression
  helpers, preflight, TOC linking, Claudine sequence source resolution,
  harness diagnostics/resolution, system prompts, and CLI composition
  completion; the three files named in the original draft were not a complete
  inventory.
- Documentation: refresh `biscuit-file/docs/topics/file-references.md` to the
  new grammar after implementation; the design-intent document
  (`claudine/docs/topics/file-referencing.md`) is updated only if Ken
  approves specific wording changes; `.claude/skills/` entries that describe
  file referencing follow the drift rule.
- Tests: L1 unit/conflict fixtures for every sigil and both orderings’ flip;
  repo-containment and source-scope matrices; the D8 regression (proxied-prompt
  success guard reading a parameter-derived path); passive schema-corpus tests;
  and L2 coverage on the real compose/sequence surfaces per the repo's testing
  taxonomy.

## Acceptance criteria

- **AC1 — catalog behavior.** For each sigil in D1, fixtures demonstrate the
  documented local candidate order, first-match-wins, miss-as-`Ok(None)`, and
  typed errors. HTTP(S) fixtures instead prove typed-remote behavior and no
  local candidates. Parse fixtures cover empty/rooted/backslash payloads after
  `@`/`&`/`^`, `~user`, `%%`, removed `!`, drive-relative input, and unknown
  schemes on every host.
- **AC2 — implicit order flip.** Conflict fixtures where the composition CWD
  and the repo root both contain the referenced file prove the CWD copy wins
  for document-authored references, and the launch-directory copy wins for
  caller-passed parameters. The superseded repository-first fixtures are
  removed or inverted, with ctx-launch-anchor AC10's fixtures explicitly
  reconciled.
- **AC3 — `!` removal.** `!x` fails parsing with a diagnostic that names the
  removed sigil and suggests `^`. No workspace code or prompt still produces
  or consumes the `Package` kind or the ambiguous `RootProvenance::Package`.
  Expression-language negation is unaffected.
- **AC4 — scope derivation.** A `^`/`@`/implicit reference authored in a
  document inside package area X resolves against X's scopes regardless of
  the launch directory; the same reference passed as a CLI parameter from
  package area Y resolves against Y's scopes regardless of where the target
  prompt lives. Moving through package, area, repository, trusted-external,
  and second-repository documents recomputes or clears scopes exactly as D7
  requires. Work counters/seeded guards prove explicit resolution performs no
  ambient CWD, HOME, Git, Cargo metadata, or topology discovery. A
  standalone `md compose` and a `claudine compose` of the same document
  produce identical `^`/`@`/implicit candidate plans, proving both consume
  the one catalog projection; repository search shows no remaining
  `cargo_metadata` use in biscuit-file and no `find_package_area_from` /
  resolver-side `find_git_root_from` in Darkmatter.
- **AC5 — materialization and provenance.** The matrix covers caller CLI
  overrides, document defaults, `proxy.with`, sequence task parameters,
  direct/proxy/retry/resume/loop/sequence consumers, scalar/array/union schema
  arms, lazy first-plan selection, eager first-existing selection, lazy
  recursive rejection, and lazy remote preservation. Ordinary string
  overrides stay untouched. A regression reproduces the 2026-08-26
  `CompositionError` shape (repo-root shared prompt, launch from a package
  area, relative `spec`, `parent_dir(spec)`/`dirname(spec)`-derived sibling
  read in the proxied prompt's `success` guard) and passes.
- **AC6 — `ctx.cwd`.** `ctx.cwd` reports the launch directory as an absolute
  path on direct, inline-compose, proxy, retry/resume, loop, sequence,
  overlay, harness, and system-prompt routes. It is caller-scoped (moving the
  prompt does not change it), immutable across the invocation, and projected
  from the same stored snapshot in preflight, body, effective frontmatter,
  and lifecycle. Ambient Darkmatter captures once at its request boundary;
  seeded inventory/work guards reject downstream `current_dir()` capture.
  Outside-repository coverage proves `ctx.cwd` remains populated without
  requesting the repository group; a forced ambient CWD failure produces
  `null` plus a typed partial-capture diagnostic rather than an empty or
  relative path. `AGENT_CWD` is present in every spawned child's environment
  (provider, hook, `::shell`, lifecycle executor, sequence shell task) as the
  captured absolute launch directory, overwrites an inherited value, and stays
  stable across re-entry. The spawn-seam inventory guard (D8.7) fails on any
  `Command` construction in `claudine` lib or CLI that does not go through
  the shared environment helper, and is proven non-vacuous by neutering one
  seam.
- **AC7 — magic conventions preserved.** The skill example
  (`@.claude/skills/.../SKILL.md`: repo first, home fallback) and Claudine's
  prompt-lookup conventions keep working through registered roots. Collision
  fixtures lock the complete effective prepend → intrinsic → append order,
  prove intrinsic package/area/repository/home roots occur once, and show that
  a nested document gets its own source-relative convention roots.
- **AC8 — completion/execution parity.** Completion for `@`, `^`, `&`, and
  implicit tokens enumerates the same roots in the same order execution
  probes, per the existing parity contract. Direct `&`/`^` completion is in
  scope; a well-formed recursive `%` completion remains unsupported and
  returns `Ok(None)` without reinterpreting the token, while malformed rooted
  payloads retain their typed parse error.
- **AC9 — cross-platform.** New parsing and resolution honor Windows drive,
  UNC, separator, case, junction/reparse-point, and verbatim-prefix behavior;
  fixtures use `Path`/`PathBuf` and portable-path helpers rather than manual
  separator replacement. Host-independent parser tests run everywhere, and
  filesystem-specific containment tests run on their native OS.
- **AC10 — validation.** `just test`, `just test-l2`, and `just lint` pass in
  the `biscuit-file/`, `darkmatter/`, and `claudine/` package areas. L2
  fixtures do not take focus. The complete affected suites are green on the
  local macOS, `build-linux`, `build-win`, and `build-win-native`
  environments before hosted CI runs.
- **AC11 — repository containment.** Direct, recursive, and completion paths
  reject lexical `..` escapes for `&`/`^`. Existing direct symlink files and
  Windows junction/reparse targets that resolve outside the repository are
  rejected; lazy materialization checks the deepest existing ancestor, and
  in-repository links still resolve. The typed error identifies the sigil,
  authored reference, repository root, and escaped candidate without leaking
  an unrelated ambient path. Tests state the normal time-of-check/time-of-use
  limitation: later external filesystem mutation does not make biscuit-file a
  sandbox.
- **AC12 — passive and public contracts.** Validation-only schema APIs remain
  passive and non-mutating; only successful composition materializes caller
  file values. Public enum/error changes compile exhaustively across the
  workspace, detailed diagnostics preserve distinct package/package-area
  provenance, and shipped schema/prompt corpora plus normal CLI invocation
  paths cover the new grammar.
- **AC13 — ratification gate (satisfied 2026-08-27).** Both former open
  questions are ruled. OQ1: unsupported local namespace forms are rejected
  and reserved; because that differs from the design-intent document, the
  document amendment distinguishing forms authors may *encounter* from forms
  the grammar *accepts* lands in the same change as this ruling. OQ2:
  `AGENT_CWD` is set on every spawned child (D8.7), matching the document, so
  no amendment was required. No two-normative-answers window remains;
  implementation may start.

## Non-goals

- `vault:` enhancements or Obsidian-specific behavior (future scope).
- Implementing `file:` URI or Windows device-prefix support — OQ1 ruled both
  rejected and reserved. Windows drive-relative (`C:path`) resolution is not
  a silent non-goal: D9 rejects it explicitly so it cannot escape an implicit
  base.
- Named namespace schemes (`prompt:`, `skill:`) replacing Claudine's
  registered magic roots — discussed and deferred; D6 keeps the registration
  mechanism.
- Any change to prepared-`ctx.*` anchoring, epoch snapshot semantics, or
  `current.ctx.*`.
- Turning general file resolution into a filesystem sandbox. Canonical
  repository containment is an explicit semantic guarantee only for `&` and
  `^`; `@`, explicit-relative, home, vault, absolute, and remote references
  retain their documented ability to leave a repository.
- Backward compatibility or migration tooling (no production users).

## Open questions

### OQ1 — Unsupported local namespace forms *(RESOLVED 2026-08-27)*

**Ruling: reject and reserve all unsupported forms.** RFC 8089 `file:` URIs
and Windows device prefixes (`\\?\`, `\\.\`) are not reference grammar: each
is a typed parse error pointing at the equivalent native absolute path, per
D9's scheme guard and D10. This keeps the grammar small and deterministic —
no host-dependent URI decoding, authority/share ambiguity, or
device-namespace bypass of the `&`/`^` containment checks — and future
support can be added without changing what an implicit path means. Because
this ruling differs from the design-intent document, that document is amended
in the same change (AC13) to distinguish forms authors may *encounter* from
forms the grammar *accepts*.

Rejected alternatives, for the record: implementing `file:` URIs (a
standardized interchange form, but it adds percent-decoding,
authority/host, and round-trip rules plus a dedicated cross-OS matrix to an
already broad feature, and no concrete workflow needs it) and implementing
both URI and device forms (widest coverage, largest security and portability
surface). Loud reservation was judged safer than accidental implicit
fallback in every case.

### OQ2 — Child-process launch-directory environment *(RESOLVED 2026-08-27)*

**Ruling: set `AGENT_CWD` on every Claudine-spawned child** — now normative as
D8.7. The design-intent document promises `${AGENT_CWD}`; the value already
exists as immutable invocation state (the launch snapshot behind `ctx.cwd`),
so honoring the promise costs one environment insertion at spawn seams that
already carry environment inventory tests, and it serves the consumers that
cannot read `ctx.*` (provider CLIs, hooks, `::shell` commands). Because the
ruling matches the design document, no amendment was required.

Rejected alternatives, for the record: a namespaced `CLAUDINE_LAUNCH_CWD`
(clearer ownership, but renames what the normative document promises) and
exposing only `ctx.cwd` (smallest surface — most scripts can take
`{{ctx.cwd}}` as an argument — but leaves the document's environment claim
dangling). The accepted trade-off is documented in D8.7: `AGENT_CWD` is
un-namespaced, Claudine's overwrite-on-spawn rule protects its own children,
and the residual risk of an unrelated tool reading the variable with
different expectations is accepted and noted in the environment
documentation.
