# Sigil Delta: Proposed Grammar vs. Today's Implementation

This document chronicles the differences between the **proposed** file-referencing
grammar in `claudine/docs/topics/file-referencing.md` (aspirational, not yet
reality) and the **implemented** behavior as of 2026-08-26. The implementation
baseline is:

- `biscuit-file/lib/src/file_reference/{parse,resolve,context}.rs` — grammar,
  candidate plans, context capture;
- `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs` — the
  document-authored resolution surface;
- `claudine/lib/src/invocation_context.rs` and
  `claudine/lib/src/composition/resolve.rs` — launch-time anchor capture and
  convention magic roots.

"Proposal" below always means the topics document; "today" always means the
code on `main`/current branches at the baseline date.

> **Revision history (2026-08-27):** overnight, the agent implementing the
> ctx-launch-anchor fix rewrote the topics document — directed by review-3
> Finding 4, which treated the document as drifted user documentation and
> required it to state the implemented repository-first implicit ordering. The
> topics document is a *design-intent* document, so that rewrite was reverted
> to Ken's committed version (`9009c35a6`) on 2026-08-27, with three approved
> edits applied on top: the `!` section removed, the `^` "ambiguous relative
> path" auto-rewrite sentence removed, and a design-intent banner added. The
> reverted rewrite is preserved in the session scratchpad and in review-3's
> finding text. Deltas below are measured against the restored + edited
> document.

## Summary Table

| Sigil / form | Proposal meaning | Today's meaning | Delta class |
|---|---|---|---|
| `./` `../` | CWD-relative, where the Ruling sets composition CWD to the composed file's directory | Pinned to the resolution base directory (the authoring document's directory during composition) | Aligned in effect, different framing |
| bare path | Composition-CWD first, then repo root | **Repo root first, then base directory** | **Divergent — order reversed; the open decision of this feature** |
| `~` | User home | User home only; `~user` rejected with a typed error | Aligned (today is stricter) |
| `@` | Multi-homed: package → package area → repo root → home | Multi-homed: *configured prepends* → repo root → home → *configured appends*; package/area precedence exists only because Claudine registers it | Partially aligned |
| `&` | Repo-root-pinned, 1:1 | **Does not exist** — `&foo` parses as an implicit-relative path whose filename contains `&` | Not implemented |
| `^` | Multi-homed repo-only (package → area → repo root), no home fallback | **Does not exist** — same literal-character parsing as `&` | Not implemented |
| `!` | *Removed from the target document on 2026-08-27* (was "immediate context": relative to the current file) | **Package reference**: resolves against the Cargo package area (or repo-root fallback) | Ruled: `!` is to be removed; implementation still ships the Package meaning |
| `vault:` | "Obsidian Vault(s)" (stub) | General vault search: `add_vault()` roots, then `$VAULT` env (platform path-separator split); `vault:` and `vault::` equivalent; `VaultNotConfigured` when empty | Aligned but today is broader |
| `/`, `C:\`, UNC | Absolute roots | Recognized absolute forms, used verbatim | Aligned |
| `file:/`, `file:///`, `file://server/share` | Local-file URI forms | **Not in the grammar** — would parse as an implicit-relative path | Not implemented |
| `\\?\`, `\\.\` | "More obscure Windows referencing" | Not documented grammar; verbatim prefixes are *reduced during containment normalization*, not parsed as a reference form | Partially implemented |
| `C:path\to\file` (drive-relative) | Called out as a footgun distinct from `C:\path` | No explicit handling documented | Gap on both sides |
| `%` (recursive) | Not mentioned | Implemented: recursive traversal modifier on any kind | Implemented, not proposed |
| `{{VAR}}` | Not mentioned (proposal discusses `${HOME}`/`%USERPROFILE%` conceptually) | Implemented: `{{VAR_NAME}}` interpolation, `[A-Z0-9_]+`, with post-interpolation anchoring re-derivation and sigil-injection rejection | Implemented, not proposed (different syntax than the proposal implies) |
| `http://` `https://` | Not mentioned | Implemented: typed remote reference, never a local candidate | Implemented, not proposed |

## Detailed Deltas

### 1. `!` — the headline collision *(ruled 2026-08-27: remove the sigil)*

Ken ruled that `!` should be removed from the grammar entirely; the target
document's `!` section was deleted on 2026-08-27. The implementation still
ships the Package meaning below, so the removal is now an implementation task
for the finalized spec. The original collision, for the record:

The proposal assigned `!` to **document-relative** resolution: `::file !baz.md`
inside `foo/bar/doit.md` means `foo/bar/baz.md`, independent of where the
operator ran the command.

Today `!` is the **package reference** (`biscuit-file` `ReferenceKind::Package`):
one candidate rooted at the Cargo workspace package area (the first path
component of the workspace member containing the base), falling back to the
repository root, with `MissingPackageContext` when neither anchor exists. It is
actively used with this meaning, and Claudine provisions its anchor from the
launch directory (`invocation_context.rs`), making `!README.md` the one
spelling that reliably means "the caller's sub-project's README".

The proposal's document-relative semantics are today spelled `./baz.md` — the
explicit-relative kind is already pinned to the authoring document's directory
during composition, so the proposed `!` duplicates an existing, shorter
spelling while destroying the only package-area sigil. Adopting the proposal
as written would silently re-anchor every existing `!` reference.

### 2. Implicit relative — resolution order conflict; the open decision

- **Target document:** try the composition CWD first (per the Ruling, the
  composed file's directory), then `{repo-root}/path`.
- **Today:** the repository root is tried **first**, then the base directory
  (`implicit_relative_roots`, `biscuit-file` `resolve.rs`: "a
  repository-shaped bare path is the primary authoring form, so the repository
  candidate is tried before the source-local one"). When no repository is
  known, the base is the only candidate; when base == repo root, the two
  collapse.

This is not cosmetic: in a monorepo where both `<repo>/README.md` and
`<source-dir>/README.md` exist, the two orders pick *different files*.
Review-3 Finding 4 flagged exactly this conflict against AC10's conflict
fixtures; the finding's observation stands, and the ordering ruling is this
feature's to make. (The document's former claim that ambiguous relative paths
are automatically treated as `^` was removed on 2026-08-27 — today's
implementation performs no kind rewriting, and implicit relative has its own
closed two-candidate plan.)

### 3. `&` and `^` — proposed, not implemented, and currently unreserved

Neither sigil exists in today's grammar. `&foo` and `^foo` parse as
implicit-relative references whose first path component contains a literal `&`
or `^` — they would probe `<repo>/&foo` then `<base>/&foo`. Nothing rejects
them, so introducing these sigils later is a **breaking change for any path
that legitimately starts with those characters** (rare, but currently legal).

Closest current equivalents:

- `&path` (repo-pinned, 1:1) ≈ today's bare path *when the file exists at the
  repo root*, but bare paths also fall back to the base directory, so today
  has no way to say "repo root or fail". The proposal's "INVALID outside a
  repo" rule also has no analogue — today a bare path outside a repo quietly
  resolves against the base only.
- `^path` (package → area → repo, no home) ≈ the *effective* behavior of `@`
  under Claudine's registered prepends, minus the home fallback and minus
  Claudine's convention roots (`prompts/`, `docs/`, peer skills directories).
  Nothing in `biscuit-file` itself provides this order.

### 4. `@` — same shape, different ownership of the search list

The proposal presents package-root → package-area-root → repo-root → home as
the *intrinsic* order of `@`.

Today the intrinsic order (`biscuit-file`) is: **configured prepends → repo
root → home → configured appends**. The package/area steps are not built in;
they appear only because:

- Claudine registers them as prepends via `prompt_magic_roots()`
  (`composition/resolve.rs:467`), which *also* prepends convention roots the
  proposal does not mention: `<package>/prompts`, `<area>/prompts`,
  `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`, the peer-agent
  skills directories (`.claude/skills`, `.codex/skills`, `.gemini/skills`,
  `.opencode/skills`, `.goose/skills`, `.qwen/skills`, `.kimi/skills`), and
  `~/.claudine/prompts`; or
- an ambient caller opts in with `with_package_area_magic_path()`.

So the proposal's skill example (`@.claude/skills/.../SKILL.md` → repo first,
then home) *does* work today, but via the built-in repo→home tail rather than
any package-aware step. The delta is one of specification ownership: the
proposal treats the full precedence as part of the sigil's definition, while
today it is split between a small library default and a Claudine-owned
registration that carries substantially more roots than the proposal lists.

### 5. Defensive leading `/` — only `@` has it

The proposal states that `@`, `&`, `^`, and `!` all tolerate a following `/`
(`@path` ≡ `@/path`, etc.).

Today only `@` implements this: the parser consumes exactly one optional `/`
and rejects payloads that remain rooted (`InvalidSyntax`). `!` strips only the
sigil itself with **no rooted-payload guard** (`parse.rs:100`), so `!/foo`
keeps an absolute payload — and since joining an absolute path onto a base
*replaces* the base, `!/foo` today effectively resolves as `/foo`. That is a
latent surprise regardless of which grammar wins, and worth a defensive fix
independent of this feature.

### 6. The CWD model — Ruling and Exception Clause match today's mechanics

The target document's Ruling — during composition the effective CWD is the
directory of the file being composed — matches today's mechanics for
explicit-relative references: they pin to the resolution context's base
directory, which is the authoring document's directory. Its Exception Clause —
caller-passed frontmatter file parameters resolve against the caller's
original launch directory — matches today's top-level behavior, where
parameters go through the request-scoped resolver captured at launch. (The
implicit-relative ordering conflict is Section 2, not this section.)

Two gaps remain against today's code:

- **`ctx.cwd` and `AGENT_CWD` do not exist.** The target document states that
  `ctx.cwd` and the `${AGENT_CWD}` environment variable report the launch
  directory. Verified 2026-08-27: neither is implemented. Today the launch
  directory is captured internally and drives the caller-scoped `ctx.area`,
  `ctx.package_area`, and `ctx.package`, but is not directly readable from a
  prompt. `ctx.cwd` (caller-scoped, like its siblings) must be added by the
  finalized spec; whether `AGENT_CWD` is worth setting for child processes is
  an open call.
- **The Exception Clause needs one strengthening.** The clause resolves the
  *parameter* against the caller's launch directory — matching today's
  top-level behavior — but must additionally require that the parameter's
  **value**, as seen by expressions like `parent_dir(spec)` and by proxied
  documents, is the resolved anchored path, not the caller's raw relative
  string. Today the raw string leaks into derived expressions and across
  proxy boundaries, where nested-document resolution re-anchors it wrongly;
  the 2026-08-26 `CompositionError` during the ctx-launch-anchor review
  (guard crash on `fixes/2026-08-12-ctx-launch-anchor/review-1.md`) is a live
  instance. This materialization rule is load-bearing: it is the agreed
  replacement for any caller-anchored sigil (the `{{ parent_dir(spec) }}`
  pattern).

Startup behavior partially matches the proposal. The Claudine CLI does change
directory to the repo root at startup, and the launch directory is captured
into the invocation context, where it drives the caller-scoped `ctx.area`,
`ctx.package_area`, and `ctx.package` values. However, the proposal's two
direct reporting mechanisms do not exist today: there is no `ctx.cwd`
variable, and no `AGENT_CWD` environment variable is set (verified 2026-08-27
— no occurrence in `claudine/lib` or `claudine/cli`). A prompt today has no
way to read the caller's exact launch directory; only its derived
package/area facts are exposed.

### 7. `vault:` — proposal is a stub; today is general

The proposal names the sigil "Obsidian Vault(s)" with no rules. Today it is a
general configured-roots search: `add_vault()` roots in registration order,
then paths from the `$VAULT` environment variable split on the platform path
separator; `vault:` and `vault::` are equivalent; an empty root set is the
typed `VaultNotConfigured` error rather than a miss. Nothing is
Obsidian-specific.

### 8. Absolute and URI forms

Today's grammar recognizes POSIX-root, Windows drive, and UNC absolute paths,
used verbatim as a single candidate. The proposal additionally enumerates RFC
8089 `file:` URI forms and the `\\?\` / `\\.\` device prefixes:

- `file:` URIs are **not recognized** today. Detection covers only
  `http://`/`https://` (ASCII-case-insensitive); a `file:///x` string falls
  through to implicit-relative classification, which is certainly not the
  author's intent.
- `\\?\` verbatim prefixes are handled today only in path *normalization*
  (containment validation reduces them); they are not a documented reference
  form.
- Drive-relative `C:path` is called out by the proposal as a footgun; today's
  code has no explicit stance.

### 9. Implemented today, absent from the proposal

The proposal is silent on several load-bearing pieces of today's grammar and
semantics. A finalized-references spec must either adopt or explicitly retire
each:

- the `%` recursive-search modifier (traversal roots, filename +
  parent-suffix matching, lexicographic first);
- `{{VAR}}` environment interpolation, including post-interpolation
  *effective anchoring* re-derivation and the rule that interpolation may
  never inject a sigil;
- `http(s)://` typed remote references and their separation from local
  resolution;
- the closed-candidate-list principle (no cross-kind fallback) and the
  `Ok(Some)` / `Ok(None)` / `Err` outcome trichotomy;
- the typed diagnostic model (`ResolutionFailure`, `RootProvenance`,
  `ProbeDisposition`) and completion/execution parity;
- eager-`file` frontmatter normalization (rewriting caller values to resolved
  paths), which is the mechanism the proposal's Eager/Lazy section implies but
  does not name.

## Condensed Verdict

| Area | Verdict |
|---|---|
| `./` `../`, `~`, absolute, `vault:` | Proposal and implementation agree in substance; proposal under-specifies |
| `@` | Order compatible, but ownership of the search list (intrinsic vs. Claudine-registered) must be decided; convention roots undocumented in proposal |
| bare paths | Open conflict on candidate order: target document says composition-CWD first, implementation says repo-root first — the central ruling this feature must make (review-3 Finding 4 is deferred to it) |
| `!` | Ruled 2026-08-27: removed from the target document; implementation removal of the Package meaning is pending the finalized spec |
| `&`, `^` | New sigils; unimplemented; currently-legal path characters, so reserving them is a (minor) breaking change |
| `file:` URIs, device prefixes, drive-relative | Unimplemented; need an explicit accept/reject decision in the final grammar |
| `%`, `{{VAR}}`, URLs, diagnostics, eager rewrite | Implemented and battle-tested; proposal must incorporate or consciously retire them |
