# Darkmatter Library Surfaces

Use this reference when changing public exports or deciding which module owns a
behavior.

## Contents

- [Document and frontmatter](#document-and-frontmatter)
- [Composition and expressions](#composition-and-expressions)
- [Schemas and references](#schemas-and-references)
- [Rendering and style](#rendering-and-style)
- [Effects, cache, and remote access](#effects-cache-and-remote-access)
- [Package boundaries](#package-boundaries)

## Document and frontmatter

- `Markdown` is the primary parsed Markdown document.
- `Frontmatter` retains structured values and raw source information used for
  span-aware diagnostics.
- Markdown-aware hashing treats the frontmatter and body as distinct segments.
  Use the Darkmatter library or `md hash`; do not substitute a generic file
  hash.
- YAML analysis and span support are shared through `biscuit-file` rather than
  reimplemented in Darkmatter.

## Composition and expressions

- `ComposeRequest` is the request authority: it owns the required
  `FileResolutionContext` (built by `build_resolution_context` from a
  `RequestSnapshot`) and `Deref`s to `ComposeOptions`, which carries remote
  policy, cache, schemas, and rendering options but no context.
- `EffectiveContext` and the expression runtime expose `ctx.*`, `fm.*`, and
  `env.*` without ambient recapture.
- Expression descriptors are typed catalogs used by both library callers and
  CLI documentation. Add a function once in the registry and keep parsing,
  evaluation, descriptors, and completion aligned.
- Author functions in `schemas/partials/functions.yaml` (a tagged SimplifiedSchema
  file: `categories:` plus one `function(...)` type, or a list of them for
  overloads, per `types:` entry) and `ctx.*` in the `context` type of
  `schemas/partials/context-variables.yaml`. The loader in
  `expression/catalog/mod.rs` embeds the file and its
  `function-examples/*.yaml` files with `include_str!`; a new file-form
  example must be added to its `EXAMPLE_FILES` table. The function registry
  panics at load when a cataloged function has no runtime binding, so every
  catalog entry needs a `FunctionBinding` in the same change.
- Consumers read `expression_function_catalog()` (categories in list order,
  descriptors in documentation order: category, then canonical name, then
  authored overload) or `function_descriptors(name)`, which resolves aliases
  and case like dispatch. List canonicals only; never re-sort by a field or
  keep a second signature list.
- Non-function `types:` entries are named helper types that signatures use:
  `ip-address` (IPv4, IPv6, or a scoped `fe80::1%en0` literal; no DNS),
  `agentic-cli` (an `enum(...)` that a test pins to
  `expression/functions/agentic_cli_generated.rs`), and `recent-commits` (the
  return of `recent_commits(count)` and, through
  `recent-commits@darkmatter:functions`, the type of `ctx.recent_commits`; the
  two descriptions name each other). `functions.yaml` is in `SHIPPED` for that
  import, so on-disk copies of the partials need no copy of it. Typed signatures
  show the helper's name, not its expansion.
- `agentic_cli_generated.rs` is written by `claudine-gen generate` from
  `claudine/docs/providers.yaml`. Never hand-edit it; `claudine-gen check` and
  its nextest drift test fail on a stale table. It stores `AiCli` variant
  names as strings, so a stale table cannot block building the generator
  (claudine-gen depends on this crate).
- `reserved_root_descriptors()` lists exactly `fm`, `ctx`, `env`, and the lazy
  mirror `current` (of `ctx`). The evaluator reserves all four:
  `context/current.rs` resolves the lazy root ahead of frontmatter,
  external state, and injected globals. Public surface:
  `CurrentProvider`/`CurrentRefresh` (the invocation's per-key capability),
  `CurrentAuthority`, `ComposeOptions::with_current_provider`,
  `EffectiveStateBuilder::with_current_authority`, and `DeferredCapabilities`
  with `ComposePreflightReport::deferred_context`.
- Every cataloged `ctx.*` key has exactly one owning `ContextGroup`.
- File arguments resolve through the shared document-backed
  `FileResolutionContext`. Shape/probe helpers must agree with actual
  resolution.

## Schemas and references

- `DarkmatterSchemas` loads and merges document and baseline schemas and owns
  the validator cache.
- `StandaloneSchemaDocument` is the passive product of
  `parse_standalone_schema_document`.
- `ValidationProblem` carries legacy message/kind fields plus typed code,
  instance/schema paths, offending property, source location, and optional
  file-reference detail.
- `SchemaOriginMap` records the owner of each effective top-level property.
- `effective_property_shape`, `property_def_at_path`, `nested_property_shape`,
  `expression_atom`, and `frontmatter_expression_values` are the passive
  authority for which property definition governs a frontmatter key path and
  which values are Expression-typed. DMLS completion, hover, and
  `dm.expression.*` diagnostics and the corpus gate share them; do not
  re-derive arm selection or the Expression check elsewhere. Trigger payloads
  and pattern-dictionary keys contribute no definitions here.
- Reference graph nodes retain origin, dependency, identity, and freshness
  information; do not collapse these to plain paths.

## Rendering and style

- `Markdown::as_terminal`, `Markdown::as_html`, and `DarkmatterPage::render`
  route through the render-tree document fold.
- `DarkmatterPage` owns only viewport/page framing.
- `CliStyleClaims` represents explicitly supplied CLI flags. It is not a second
  style model.
- `CodeBlock` is the primary highlighted-code component. `YamlBlock` is a
  deprecated compatibility wrapper.
- Component layout and paint lower to `renderable` types before target folds.

## Effects, cache, and remote access

- The effect engine owns explicit writes, shell calls, and HTTP POST behavior.
- Remote reads and writes share `biscuit_file::FetchPolicy`.
- Only raw remote-URL bodies (transport artifacts) persist (R18, R36).
  Compose keys (source identity, state, context, options, overlay) are
  run-local only; no semantic-result persistence code exists until a
  `ContentPolicy` does.
- Passive schema, DMLS, and validation surfaces never invoke the effect engine.

## Package boundaries

| Package | Responsibility |
|---|---|
| `darkmatter` | Parsing, composition, schemas, references, hashing, rendering |
| `darkmatter-cli` | The `md` CLI and presentation/orchestration policy |
| `dmls` | Language Server Protocol implementation over passive Darkmatter APIs |
| `zed-dmls` | Zed extension integration for DMLS |

Keep CLI-only parsing and flags out of the library. Keep editor transport and
workspace state in DMLS, while grammar, schema, expression, and validation
authority remains in `darkmatter`.
