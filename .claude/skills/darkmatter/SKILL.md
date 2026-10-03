---
name: darkmatter
description: Expert guidance for the Darkmatter Rust library, `md` CLI, and DMLS language server. Use when composing or rendering Markdown, working with frontmatter, expressions, schemas, file references, remote content, hashing, browser or terminal output, or extending Darkmatter's CLI and editor integrations.
---

# Darkmatter

Darkmatter is the monorepo's Markdown composition, schema, expression, and
rendering system. The package area contains the `darkmatter` library,
`darkmatter-cli` binary (`md`), `dmls` language server, and `zed-dmls`
extension.

## Non-negotiable boundaries

- Capture one request-scoped composition context. Pass the same
  `FileResolutionContext`, repository observation, remote policy, cache state,
  identity, and meta-schema controls through `ComposeOptions`; never recapture
  CWD or repository state in a downstream resolver.
- Resolve file-like values through `biscuit_file::FileReference`. Preserve its
  source context, explicit-vs-implicit syntax, and typed errors.
- Read the home only through `biscuit_file::home_dir()` (`RequestSnapshot::from_process`
  is the one production site; tests build expectations with it too). A canonical
  path that leaves a private comparison (link targets, cache and preflight keys,
  `referenced_files`, diagnostics) comes from `biscuit_file::canonicalize_simplified`;
  `path_lookup_guard.rs` in lib and cli rejects any unlisted raw `canonicalize`.
- Treat composition as effectful and validation as passive. Schema parsing,
  trigger matching, completion, hover, and validation must not perform I/O,
  execute expressions, fetch remotes, or mutate documents.
- Keep network access deny-all by default. Route every allowed remote read or
  write through `biscuit_file::FetchPolicy` and exact-host consent.
- Render terminal output with `TerminalRenderable` components. The browser tier
  is headless and must use browser-protocol input, never host focus or OS input.
- Preserve source spans and typed diagnostic provenance through parsing,
  composition, schema validation, and DMLS projection.

## Choose the owning surface

Documentation under `docs/` becomes the authority when implementation lands;
it never links to a specification or feature/fix design. Preserve documented
file locations; an unimplemented grammar is not a reason to move schema
definitions away from their references or replace them with placeholders.

| Work | Start with |
|---|---|
| Compose APIs, stages, expressions, file resolution, cache | [compose.md](compose.md) |
| SimplifiedSchema, triggers, validation, meta-types | [schema.md](schema.md) |
| Composition requests, contexts, glob consumers | [requests.md](requests.md) |
| Remote reads and the transport cache | [remote-cache.md](remote-cache.md) |
| `md` file arguments and CLI conventions | [cli.md](cli.md) |
| Expression grammar, inserted text, warnings | [expressions.md](expressions.md) |
| Test fixtures, source guards, parity matrix | [testing.md](testing.md) |
| Public modules and extracted library surfaces | [library-surfaces.md](library-surfaces.md) |
| DMLS architecture, protocol behavior, and rollout history | [dmls.md](dmls.md) |
| Render tree, style lowering, disclosure blocks, code blocks | [rendering.md](rendering.md) |
| Terminal rendering options | [terminal.md](terminal.md) |
| Frontmatter model, literal tokens, in-place leaf edits | [frontmatter.md](frontmatter.md) |
| Error/status block conventions | [errors.md](errors.md) |
| Document comparison | [comparison.md](comparison.md) |
| Module layout | [structure.md](structure.md) |
| Parser details | [pulldown-cmark.md](pulldown-cmark.md) |
| Frontmatter ecosystem comparison | [frontmatter-crates.md](frontmatter-crates.md) |

Load only the topic needed for the task. For render-tree implementation work,
also load the `renderable` skill; for terminal components, load
`biscuit-terminal`; for file-reference or JSON/YAML/TOML conversion work, load
`biscuit-file`.

## Composition authority

`ComposeRequest` is the request authority: `ComposeOptions` (remote
configuration, cache root and policy, compose identity, baseline/meta-schema
controls, rendering options) plus the one required `FileResolutionContext`
built from a `RequestSnapshot`. A source-derived file reference must resolve
against a context derived from that one. User-facing docs:
`darkmatter/docs/topics/compose-requests.md` (API, builder, Mermaid),
`topics/file-referencing.md` (forms and tree rule), and
`errors/file-reference-failures.md` (the `failure:` row).

The root compose pipeline is ordered: frontmatter interpolation pass 1;
schema validation and coercion; frontmatter shell expansion; frontmatter
interpolation pass 2; literal replacement; conditional page blocks; body
interpolation; shell directives and blocks; link resolution; concurrent
transclusion; inline cleanup and optional reflow; root-only link
normalization. Keep this order stable. Whole-value `{{ ... }}` and `$(...)`
values are executable state: they must resolve or fail, never leak as literal
syntax. Demand-driven context capture must observe only referenced `ctx.*`
groups.

Every entry point that resolves file references takes a `ComposeRequest`
(`ComposeRequest::prepare(options, &RequestSnapshot)` or
`ComposeRequest::with_context`). The context is required, never `Option`;
only binaries call `RequestSnapshot::from_process()`, and magic roots enter
only through `RequestSnapshot::with_magic_root*`. `match()`, `find_files()`,
and `::file-links <glob>` are `GlobReference`s judged or listed in the
document's (or the caller's origin) context, never the process directory.
Read [requests.md](requests.md) before touching a request, a context view,
a validator's context, or a glob consumer.

Read [compose.md](compose.md) before changing a stage, expression function,
context property, cache key, transclusion directive, or file-resolution path.

## Schema authority

Darkmatter uses `SimplifiedSchema`, compiled to Draft 2020-12 JSON Schema.
`DarkmatterSchemas` owns baseline merging and validator caching. The standalone
classifier is `parse_standalone_schema_document`; it recognizes either a pure
root `$schema` document or a `kind: schema` document with a `types` mapping.

Important contracts:

- Optional schema properties accept missing or `null` values.
- Validation keeps source positions, origin information, typed problem codes,
  pending values, and file-reference diagnostics.
  `FileReferenceDiagnostic::resolution_failure()` gives the biscuit-file
  class; the diagnostic is re-derived in the validator's own request context.
- Eager `file(eager)` values may normalize only on successful composition;
  validation-only APIs remain read-only.
- `literal(value)` preserves YAML scalar typing and lowers to JSON Schema
  `const`; quoted and native YAML forms are observably different.
- `expression`, `yaml`, and `json` are passive content formats. They parse or
  coerce but never execute.
- `type-definition` and `schema` meta-types delegate to the same passive
  parsers used by authoring and DMLS.
- Trigger matching is schema-based and passive. A schema-trigger document is a
  kinded document and must retain its root `kind` declaration.
- Triggers and bare-name `$schema` lookup use the five schema roots of the
  **document's** context (`schemas::roots::SchemaRoots::for_document`):
  package, package area, `base_dir()`, `SCHEMAS_DIR` (the folder itself, from
  `ctx.env()`), `~/schemas`. Never walk ancestors or read the process
  environment for them. `$path` patterns are `GlobReference`s; see
  [schema.md](schema.md#schema-roots-and-path-triggers).

Read [schema.md](schema.md) for imports, unions, pattern dictionaries,
suggestions, triggers, and DMLS schema behavior.

The authored global catalog `darkmatter/schemas/darkmatter.yaml` is not the
embedded document baseline; see [schema.md](schema.md#authored-global-catalog).

## Remote and cache safety

HTTP(S) composition covers `::file`, `::code`, and read-side expression
arguments where a remote runtime is present; rendered links are never fetched,
and frontmatter interpolation and `$()` fail loudly on a URL. Reads are
deny-all until `--allow-host`; only transport artifacts (raw HTTP bodies) may
persist under a cache root; `Cache-Control` outranks every freshness flag;
host policy is checked before the cache is read. Read
[remote-cache.md](remote-cache.md) before changing any of this.

## Rendering authority

All public Markdown rendering folds through the render tree. Darkmatter builds
one context-aware `renderable::Document`, then performs one target fold.
Component policy is lowered during construction; do not add a post-fold HTML or
terminal decoration pass.

`DarkmatterPage` is only a viewport/page-frame assembler and never inspects or
mutates component content. Browser tests are headless (no window activation or
host input) and assert DOM, computed style, accessibility state, or geometry;
browser HTML and CSS come from the same lowered values, never string repair.

Read [rendering.md](rendering.md) before changing style claims, code-block
themes, disclosure blocks, browser safety, or the render-tree fold.

## CLI orientation

The binary is `md`: `compose`, `clean`, `read`, `toc`, `delta`, `schema
validate|detect|about|triggers`, `hash`, `frontmatter get|set|rm`,
`code-block`, `graph`, and reference commands.

Every source-file argument opens through `cli/src/io::open_argument`:
`FileReference` grammar first, resolved in the launch context, with the
document context derived from the opening reference. Read [cli.md](cli.md)
before adding a route that reads a file argument or changing CLI style
claims.

## Testing and verification

Use the package-area recipes in `darkmatter/`: `just build`, `just test`,
`just lint`.

Use `just test-l2` only for real-terminal behavior and `just test-browser` for
headless browser behavior. Parser, schema, prompt, template, or configuration
changes require both a passive shipped-artifact corpus test and an end-to-end
test through the normal invocation path. Persisted values require a repeated
read/write/read round trip.

Expression grammar, inserted-text, failure-policy, unknown-identifier, and
warning-identity rules are in [expressions.md](expressions.md); read it
before changing the lexer, an evaluation surface, or a warning family.
Fixture rules (`CliProcessFixture`, `git init` for repositories), the
source-scan guards (context construction, glob implementation, spawn sites,
semantic results), and the entry-point parity matrix (including the glob
rows) are in [testing.md](testing.md); read it before writing an `md`
integration test, adding a file-resolving entry point, or touching a guard's
allowlist.

Do not run workspace-wide Cargo gates for a Darkmatter-only change. Use Sniff
and GitNexus first to include actual downstream consumers such as Claudine when
a public Darkmatter type or behavior changes.
