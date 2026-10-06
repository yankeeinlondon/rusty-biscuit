---
area: darkmatter
status: draft-spec
created: 2026-10-04
owner: Ken Snyder <ken@ken.net>
depends-on:
    - 2026-09-28-recursive-schema-types
related:
    - 2026-09-28-content-policy
---

# Compile Content Policy's Schema into the Base Schema

Content Policy offers its editor schema to callers as
`content_policy::EDITOR_SCHEMA`, the compiled-in text of
`content-policy/schemas/content-policy.yaml` (ruling 34 of
`2026-09-28-content-policy`). Darkmatter should incorporate it into its base
document schema so that every Markdown document's `content_policy` list is
checked, with no `$schema` of its own, wherever Darkmatter validates.

## Decided

- **Darkmatter depends on the `content-policy` crate** (default features off)
  and embeds `EDITOR_SCHEMA`. It does not `include_str!` a path in another
  package area, so the schema always matches the parser version Darkmatter
  links.
- **The base schema types the list:**
  `content_policy: "policy[]@…"` beside `last_updated`, in whichever base
  schema file Darkmatter loads at the time (today
  `darkmatter/docs/schemas/darkmatter.yaml`).
- **The base schema is the default wherever Darkmatter validates.**
  `md schema validate` applies it to a document with no `$schema`, as
  `md compose` and DMLS already do. Today the command applies a baseline only
  from `--schema` or `BASELINE_SCHEMA` and otherwise reports the document
  "valid by default" (`darkmatter/cli/src/commands/schema/validate.rs`,
  `load_api`), and `darkmatter/docs/topics/schemas/definition.md` ("Compose
  and Validate Defaults") documents that contract; both change.

## Work

1. Wait for `2026-09-28-recursive-schema-types`: today `policy[]` over a
   union-typed named type fails to convert, and DMLS panics on an unconvertible
   baseline (`expect("baseline schema must convert")` in
   `darkmatter/lib/src/markdown/schemas/mod.rs`).
2. Let the embedded baseline resolve a reference to another embedded file: a
   small table of embedded files keyed by virtual path, expanded once where the
   baseline loads. Spike S1 of `2026-09-28-content-policy`
   (`spikes/embedded-schema-ref/findings.md`) prototyped this in about 110
   lines.
3. Make `md schema validate` fall back to the embedded baseline, keeping
   `--schema`, `BASELINE_SCHEMA`, and any opt-out the other commands share.
4. If the dependency also lets constraints apply to union references, mark
   `long_form.rule` `(required)` in Content Policy's schema and invert the
   assertion in `darkmatter/lib/tests/l1/content_policy_editor_schema.rs` that
   a long form without `rule` is not flagged.
5. Test through `md schema validate` and DMLS with an ordinary document and no
   `$schema`: a mixed list of compact and `{rule, action}` entries is valid,
   and `Duration(3mo)` and `action: delete` are each flagged.
6. Add a parity test: Content Policy's parser and the schema agree on a shared
   set of valid and invalid rule strings, so the two grammars cannot drift
   silently. Darkmatter is the first package that can run both.
7. Remove the **planned** markers about the base schema in
   `content-policy/README.md`, `content-policy/docs/topics/policy-lifecycle.md`,
   and the `content-policy` skill, and update Darkmatter's schema docs.
