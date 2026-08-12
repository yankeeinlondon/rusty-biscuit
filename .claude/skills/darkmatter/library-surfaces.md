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
- Persistent cache keys include the source/reference identity and freshness
  state needed to prevent stale or cross-context reuse.
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
