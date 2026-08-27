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

| Work | Start with |
|---|---|
| Compose APIs, stages, expressions, file resolution, cache | [compose.md](compose.md) |
| SimplifiedSchema, triggers, validation, meta-types | [schema.md](schema.md) |
| Public modules and extracted library surfaces | [library-surfaces.md](library-surfaces.md) |
| DMLS architecture, protocol behavior, and rollout history | [dmls.md](dmls.md) |
| Render tree, style lowering, disclosure blocks, code blocks | [rendering.md](rendering.md) |
| Terminal rendering options | [terminal.md](terminal.md) |
| Frontmatter model | [frontmatter.md](frontmatter.md) |
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

`ComposeOptions` is the request authority. It carries the captured resolution
context plus remote configuration, cache root and policy, compose identity,
baseline/meta-schema controls, and rendering options. A source-derived file
reference must resolve against that captured context.

The root compose pipeline is ordered:

1. Frontmatter interpolation pass 1.
2. Schema validation and coercion.
3. Frontmatter shell expansion.
4. Frontmatter interpolation pass 2.
5. Literal replacement.
6. Conditional page blocks.
7. Body interpolation.
8. Shell directives and shell blocks.
9. Link resolution.
10. Concurrent transclusion.
11. Inline cleanup and optional fixed-width reflow.
12. Root-only link normalization.

Keep this order stable. Whole-value `{{ ... }}` and `$(...)` values are
executable state: they must resolve or fail, never leak as literal syntax.
Demand-driven context capture must observe only referenced `ctx.*` groups.

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

Read [schema.md](schema.md) for imports, unions, pattern dictionaries,
suggestions, triggers, and DMLS schema behavior.

## Remote and cache safety

HTTP(S) composition is supported for `::file`, `::code`, and read-side
expression arguments only where a remote runtime is present. Ordinary rendered
links are never fetched. Frontmatter interpolation and `$()` branching are
local-only; a remote URL there fails loudly.

- CLI callers opt in with `md compose --allow-host <host>`.
- Persistent artifacts require `--cache-root` or
  `ComposeOptions::with_cache_root(...)`.
- Freshness is controlled by `RemoteReadConfig` and the CLI remote freshness,
  refresh, and TTL flags.
- `absolute` and `relative` are local path transforms, never remote fetches.
- `EffectEngine::http_post` uses the same host policy as remote reads.

## Rendering authority

All public Markdown rendering folds through the render tree. Darkmatter builds
one context-aware `renderable::Document`, then performs one target fold.
Component policy is lowered during construction; do not add a post-fold HTML or
terminal decoration pass.

`DarkmatterPage` is only a viewport/page-frame assembler. It may own page
margin, padding, background, max-width centering, and browser wrapper metadata,
but it must not inspect component node kinds or mutate component content.

The browser contract is strict:

- Browser tests are headless and cannot activate windows or inject host input.
- Assert DOM state, computed style, accessibility state, or used geometry.
- Browser HTML and CSS must be derived from the same lowered layout/style
  values; never repair output with string replacement.
- Remote links and images retain safe structured attributes and policies.

Read [rendering.md](rendering.md) before changing style claims, code-block
themes, disclosure blocks, browser safety, or the render-tree fold.

## CLI orientation

The binary is `md`. Major command families include:

- `md compose`, `clean`, `read`, `toc`, and `delta` for document processing.
- `md schema validate|detect|about` for schema workflows.
- `md hash` for Markdown-aware frontmatter/body hashes.
- `md frontmatter get|set|rm` for structured frontmatter changes.
- `md code-block` for direct terminal, HTML, or Markdown code rendering.
- `md graph` and reference commands for document/reference inspection.

Rendering flags are presentation policy. `CliStyleClaims` captures explicitly
supplied global CLI style claims so command handlers can merge them with
frontmatter without mistaking defaults for user intent. Keep parsing in
`cli/src/args`, command execution in `cli/src/commands`, and output in shared
renderable components.

## Testing and verification

Use the Darkmatter package-area recipes:

```sh
cd darkmatter
just build
just test
just lint
```

Use `just test-l2` only for real-terminal behavior and `just test-browser` for
headless browser behavior. Parser, schema, prompt, template, or configuration
changes require both a passive shipped-artifact corpus test and an end-to-end
test through the normal invocation path. Persisted values require a repeated
read/write/read round trip.

The ordinary local L1 recipe excludes `slow_` tests and leaves the internal
`terminal-tests` / `browser-tests` build features disabled. Tier recipes enable
their required targets; CI enables both features when constructing all-tier
coverage.

Do not run workspace-wide Cargo gates for a Darkmatter-only change. Use Sniff
and GitNexus first to include actual downstream consumers such as Claudine when
a public Darkmatter type or behavior changes.
