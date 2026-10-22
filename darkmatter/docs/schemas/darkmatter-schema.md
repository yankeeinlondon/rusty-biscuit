# Darkmatter Base Frontmatter Schema

The `darkmatter` library exposes a base frontmatter schema for Darkmatter-owned
properties. `md compose`, `md schema validate`, and DMLS apply it by default as
the baseline of every document. Each CLI command can replace it or opt out;
see
[Selecting the Baseline](../topics/schemas/definition.md#selecting-the-baseline).

The schema is authored in the same `SimplifiedSchema` language that Markdown
authors can use with the `$schema` frontmatter property, so the baseline behaves
like an ordinary schema file.

The library compiles in the file below, and this page transcludes it so the
documentation and validation source stay in lock step. The schemas it imports
are compiled in too, so the baseline is prepared without reading the disk, and
works the same in an installed binary as in a checkout (see
[The Built-in Baseline](../topics/schemas/definition.md#the-built-in-baseline)).

## What the base schema covers

- Properties that Darkmatter defines, interprets, or mutates, such as
  `title`, `description`, `tags`, `draft`, `hash`, `style`, `change`,
  `replace`, `ctx`, `prologue`, `epilogue`, `ignore_invalid`, and
  `interpolate_code_blocks`.
- `content_policy`, typed by importing Content Policy's editor schema
  ([`content-policy/schemas/content-policy.yaml`](../../../content-policy/schemas/content-policy.yaml))
  as an optional, non-empty list of `policy` entries. The baseline never adds
  the property to a document that omits it, and checking it is passive: no
  watched file is read, no policy is renewed, and no date is stamped. Content Policy's
  [editor guide](../../../content-policy/docs/topics/policy-lifecycle.md#get-help-in-the-editor)
  lists what the schema checks and the values only evaluation rejects.
- The lifecycle events `md compose` runs (`initialize`, `composed`, `failed`,
  `cancelled`, and `finalize`), typed with md's actions from the shipped
  [`md-lifecycle.yaml`](../../schemas/partials/md-lifecycle.yaml) partial,
  which builds on the neutral action types of
  [`lifecycle.yaml`](../../schemas/partials/lifecycle.yaml). An action md does
  not provide passes untyped, and another host's event keys stay ordinary
  frontmatter; see
  [Lifecycle Event Typing](../topics/schemas/definition.md#lifecycle-event-typing).
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

::code ../../schemas/partials/fm.yaml
