---
kind: canonical-documentation
---

# Schema Triggers in Darkmatter

A schema trigger applies a frontmatter schema only to documents that match its
conditions. Put a YAML file with `kind: schema-trigger` in one of the document's
[schema roots](./definition.md#schema-roots). Its `match` property selects the
documents, and its `$schema` property supplies their schema.

For example, this trigger applies to a report that declares either a prompt
or a list of steps:

```yaml
kind: schema-trigger
match:
    - kind: literal(report; required)
    - group:
          - prompt: string(required)
          - steps: unknown[](required)
$schema:
    title: string(required)
    summary: string
```

A document with `kind: report` and `prompt: Write a summary` selects this
schema; it must then supply a string `title`. A document with only
`kind: report` does not select it. The matching conditions do not insert
properties or run the prompt.

## Combining conditions

A top-level list means **AND**: every entry must match. A `group` means
**OR**: at least one alternative must match. A mapping's sibling property
conditions also mean AND. The example therefore requires `kind: report`
**and** either a string `prompt` or an array `steps`.

Nested conditions can use `all`, `group`, `none`, or
`min-match: { count, of }`. Write an explicit `group` for alternatives;
an anonymous nested list is an error. The retired `any` combinator is also
an error that points to `group` as its replacement.

Property conditions use passive SimplifiedSchema types and constraints. A
condition without `required` allows the property to be absent, but checks its
value when present. A required concrete type establishes presence. A
null-admitting type, such as `unknown(required)`, still permits omission and
cannot establish presence; use `unknown[](required)` when an array must exist.
Every possible way to satisfy the complete match must require a property or a
path match, so a trigger cannot accidentally match every document through
optional conditions alone. Empty condition mappings are rejected.

## Selecting by path

`$path` matches a document path with glob references. A positive pattern admits
the path, and any matching `!` pattern excludes it:

```yaml
kind: schema-trigger
match:
    $path:
        - "^docs/**"
        - "!**/_*.md"
$schema:
    title: string(required)
```

This selects documents under the applicable `docs` tree, except files whose
name starts with `_`. See [Path Matching](./definition.md#matching-a-path-with-path)
for root selection and allowed reference forms. Matching is passive: it never
executes an expression or shell command, fetches a URL, or writes a document.

## Discovery and precedence

The document's schema roots are searched in order. A trigger filename in an
earlier root shadows the same filename in later roots. A malformed file that
claims the trigger kind is skipped with a warning, and it still shadows that
filename rather than silently falling back to a later file; the other valid
triggers keep applying. The retired trigger-kind spelling is rejected the same
way, with guidance naming `schema-trigger`. See
[Invalid discovered triggers](./schema-triggers.md#invalid-discovered-triggers).

Matching trigger payloads override the baseline; the document's own `$schema`
overrides matching triggers. A trigger can put its payload directly under
`$schema`, as above, or reference another schema file. See
[Repository Trigger Schemas](./definition.md#repository-trigger-schemas) for
merging, diagnostics, and discovery details.

Use `md schema triggers document.md` to inspect the roots, shadowed files,
matching conditions, and defeat reasons. `md compose`, `md schema validate`,
and `md clean` accept `--disable schema-triggers` to turn off discovery; this
does not disable the baseline or the document's explicit schema.

## Planned predicates

Dedicated conditions for arbitrary document expressions, executables, files,
repository facts, operating systems, time zones, and time remain **planned**.
Their syntax and observation rules are not current trigger grammar. The
[authoring guide](./authoring-schemas.md#schema-triggers) distinguishes those
proposals from supported schema and path conditions.

For unconditional schema exports and importable type libraries, see
[External Schemas](./authoring-schemas.md#external-schemas).
