# Darkmatter Base Frontmatter Schema

The `darkmatter` library exposes a base frontmatter schema for Darkmatter-owned
properties. `md compose` injects this schema by default as its baseline. `md
schema validate` uses a baseline only when one is supplied with `--schema` or
`BASELINE_SCHEMA`.

The schema is authored in the same `SimplifiedSchema` language that Markdown
authors can use with the `$schema` frontmatter property, so the baseline behaves
like an ordinary schema file.

The library loads the schema from the file below, and this page transcludes it
so the documentation and validation source stay in lock step.

## What the base schema covers

- Properties that Darkmatter defines, interprets, or mutates, such as
  `title`, `description`, `tags`, `draft`, `hash`, `style`, `change`,
  `replace`, `ctx`, `prologue`, `epilogue`, `ignore_invalid`, and
  `interpolate_code_blocks`.
- The `$schema` property itself, which documents may use to declare their own
  schema or override baseline definitions.

## Merge behavior and extensibility

- Document-level `$schema` declarations override baseline properties on
  conflict. If a document declares `title: number`, the document schema wins.
- Unknown frontmatter keys remain allowed. The base schema validates
  Darkmatter-owned properties but does not close the frontmatter namespace to
  user-defined keys.
- `ctx` is a Darkmatter-owned namespace. Darkmatter may merge an authored
  `ctx` object for compatibility, but document authors should not define custom
  `ctx.*` keys because they collide with the runtime context namespace.
- `generated` properties, such as `ctx.today`, are supplied by the host
  (Darkmatter context capture, Claudine, or another runtime) and are not
  expected to be authored in static frontmatter. They are omitted from static
  `required` checks, but their type and nullability semantics are preserved
  for runtime validation and editor tooling.

## Schema source

::code ./darkmatter.yaml
