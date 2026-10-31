---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
implemented: true
created: 2026-10-30
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
    - simplified-schema
related:
    - 2026-10-30-reference-navigation-and-hover
    - 2026-10-28-schema-enhancements
    - 2026-10-28-extract-simplified-schema
implemented_by: "claude/opus"
---

# Schema Definitions for YAML Documents

## Problem

Darkmatter has two formal kinds of schema document, both YAML:

| Kind | Identifier | Shape |
|---|---|---|
| `schema` | `kind: schema` | `$schema`, `types`, `description`, `categories`; each optional, at least one of `$schema` or `types` |
| `schema-trigger` | `kind: schema-trigger` | match rules plus `$schema` |

(An informal `schema` omits `kind` and requires `$schema`.)

The intent was that these two kinds are themselves defined by YAML schema
files in `darkmatter/schemas/`, shipped in the binary, so that the definition
of a schema document is authored, reviewed, and edited like every other
schema. That never landed:

- Drafts were written: `schema-definition.yaml` (the `schema` kind),
  `schema-target.yaml` (the `schema-trigger` kind, under its earlier name), and
  `kind-document.yaml` (the repository's `kind` enum). Because they used
  grammar the parser did not support (`kind: const(schema)`) and the
  shipped-schema corpus tests read every file in those folders, they were
  moved out twice (`6d3400978`, 2026-09-19; `8b6d776bd`, 2026-09-22). They now
  sit in `2026-10-28-schema-enhancements`'s `drafts/` directory.
- `darkmatter/docs/schemas/schema-definition.yaml` is an empty placeholder,
  and `darkmatter/docs/topics/schemas/authoring-schemas.md` transcludes it
  with `::code`, so the published docs show an empty block.
- `docs/kind-documents.md` transcludes `kind-document.yaml` through a path
  without the `_completed/` lifecycle directory, so that embed is broken.
- The rules are enforced only by hand-written Rust: the envelope check in
  `simplified-schema/lib/src/simplified/standalone.rs` and the trigger
  envelope in `darkmatter/lib/src/markdown/schemas/triggers/envelope.rs`.
  Editing a YAML file changes nothing; the docs and the code can disagree
  silently.

The missing mechanism is the reason the drafts had nowhere to go: `$schema:`
and schema triggers apply a schema to **Markdown frontmatter** only. Nothing
applies a schema to a whole YAML document selected by its `kind`.

## Goal

1. `darkmatter/schemas/` holds `schema.yaml` and `schema-trigger.yaml`, the
   authoritative definitions of the two kinds.
2. Every YAML document whose top-level `kind` names one of them is validated
   against it, by `md` and by DMLS (diagnostics, completion, and hover on the
   YAML file itself), through the same validator Markdown frontmatter uses.
3. Editing either file changes behavior on the next build with no other code
   change.
4. The hand-written envelope rules shrink to what YAML cannot express, and
   whatever remains is checked against the YAML definitions by a test so they
   cannot drift.

## Getting the definitions into the binary

No code generation is required. `darkmatter/lib/src/markdown/schemas/embedded.rs`
already compiles `darkmatter/schemas/darkmatter.yaml` and its partials in with
`include_str!` (the `SHIPPED` table), so an edit is picked up by the next
build. Add the two new files to that table.

`SHIPPED` is a hand-maintained list: a new file in `darkmatter/schemas/` that
is not added to it ships nothing. Add an L1 test that every YAML file under
`darkmatter/schemas/` is in `SHIPPED` (or on an explicit, commented exclusion
list), so an added definition cannot be silently left out.

## Design questions

To be settled with the author before planning. Each is listed with the
current leaning.

### 1. How does a YAML document select its schema?

Options:

- **By `kind`, from a registry of shipped kind definitions** (leaning). A YAML
  file with `kind: schema` is validated against `darkmatter/schemas/schema.yaml`;
  `kind: schema-trigger` against `schema-trigger.yaml`. Only the shipped kinds
  apply; an unknown `kind` is not an error.
- By a `$schema`-like key in the YAML file. Rejected for these two kinds: in a
  schema document `$schema` already means "the schema this document exports",
  so it cannot also mean "the schema this document conforms to".
- By file location (every YAML file in a schema root). Rejected: schema roots
  also hold partials and type libraries with no `kind`.

Open within this option: whether hosts (Claudine) can register further kinds
for YAML documents, and whether the repository-wide `kind` catalog
(`kind-document.yaml`) belongs in this mechanism or stays documentation.

### 2. What can a kind definition not express?

Known rules the grammar may not express today, to be checked against the
current grammar (the `simplified-schema` extraction may already cover some):

- `kind: const(schema)` — a literal value (the reason the drafts were moved out)
- "at least one of `$schema` or `types`" — the drafts express this as a
  two-arm root union; confirm the union semantics give a usable error
- the value of `$schema` and `types` being itself a SimplifiedSchema (a
  `schema` type, already used by the `fm` base schema's `$schema` property)
- trigger `match` predicates (the draft marks its predicate catalog incomplete)

Each rule either becomes expressible (a grammar change, specified here) or
stays in Rust with a test tying it to the YAML definition.

### 3. The `name` property

Both drafts declare a `name` (a short, dasherized identifier). No current
schema document uses it and the author has not asked for it. Decide whether
it is part of either kind; the default is to drop it.

### 4. Bootstrapping

`schema.yaml` is itself a `kind: schema` document, so it validates against
itself. Loading must not recurse: the shipped definitions are parsed by the
existing standalone parser first, then used to validate other documents
(including themselves, as a test).

## Out of scope

- The navigation, hover, root-order, and envelope-rule changes in
  `2026-10-30-reference-navigation-and-hover`; this feature makes those
  envelope rules authoritative in YAML but does not change them.
- Validating arbitrary YAML files that declare no shipped `kind`.

## Acceptance

- `darkmatter/schemas/schema.yaml` and `darkmatter/schemas/schema-trigger.yaml`
  exist, are in `SHIPPED`, and describe exactly the rules in the table above.
- A YAML document with `kind: schema` or `kind: schema-trigger` that violates
  its definition gets a diagnostic from `md` and from DMLS, with the source
  range of the offending key; completion and hover work in that file.
- Changing a rule in either YAML file (for example, removing `description`)
  changes validation after a rebuild with no Rust change; an L1 test proves
  this by validating against a modified copy.
- Every shipped and repository schema document passes its kind definition
  (the shipped-schema corpus test).
- The `SHIPPED` completeness test exists.
- `darkmatter/docs/schemas/schema-definition.yaml` is removed or replaced by a
  transclusion of the shipped definition, and `authoring-schemas.md` shows it;
  the `docs/kind-documents.md` embed resolves.
- The drafts in `2026-10-28-schema-enhancements` are left as they are (a
  closed spec is a snapshot); nothing references them after this feature.

## Documentation

- `darkmatter/docs/topics/schemas/authoring-schemas.md`: the two kinds, shown
  from the shipped definitions.
- `darkmatter/docs/topics/schemas/definition.md`: how a YAML document is
  validated by its `kind`.
- DMLS docs: diagnostics, completion, and hover in schema YAML files.
- `docs/kind-documents.md`: fix the transclusion.
