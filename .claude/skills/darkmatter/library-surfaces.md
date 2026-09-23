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

- `ComposeOptions` is the request authority for context, file resolution,
  remote policy, cache, schemas, and rendering options.
- `EffectiveContext` and the expression runtime expose `ctx.*`, `doc.*`, and
  `env.*` without ambient recapture.
- Expression descriptors are typed catalogs used by both library callers and
  CLI documentation. Add a function once in the registry and keep parsing,
  evaluation, descriptors, and completion aligned.
- Author functions in `docs/schemas/expression-functions.yaml` and `ctx.*` in
  the `ctx:` block of `docs/schemas/darkmatter.yaml`. The function registry
  panics at load when a cataloged function has no runtime binding, so every
  catalog entry needs a `FunctionBinding` in the same change.
- Catalog-only parameter types refine `string`: `ip-address` (IPv4, IPv6, or a
  scoped `fe80::1%en0` literal; no DNS) and `agentic-cli` (the closed enum from
  `expression/functions/agentic_cli_generated.rs`). Inline `enum(...)`
  parameters stay rejected. Returns may add `literals: [...]` and
  `nullable: true`; the typed signature renders `value | "literal" | null | error`.
- R29 pairs: a function marked `pair: true` shares its name with a `ctx.*`
  variable and authors no `description`, `returns.type`, or `returns.array`.
  The parser copies all three from the variable, and
  `ExpressionFunctionDescriptor::pair` names it. `recent_commits` is the first pair.
- `agentic_cli_generated.rs` is written by `claudine-gen generate` from
  `claudine/docs/providers.yaml`. Never hand-edit it; `claudine-gen check` and
  its nextest drift test fail on a stale table. It stores `AiCli` variant
  names as strings, so a stale table cannot block building the generator
  (claudine-gen depends on this crate).
- `reserved_root_descriptors()` lists `doc`, `ctx`, `env` and the lazy mirrors
  `current` (of `ctx`) and `current_env` (of `env`). The evaluator reserves all
  five: `context/current.rs` resolves the two lazy roots ahead of frontmatter,
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
