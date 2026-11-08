---
kind: canonical-documentation
---

# `md schema`

The `md schema` commands work with the
[SimplifiedSchema](../../../simplified-schema/README.md) language Darkmatter uses to
describe frontmatter and YAML documents: check a document against its schema, write
a schema from existing documents, find out which schemas apply to a file, and look up
the language itself.

```sh
md schema validate docs/post.md                # is this document valid?
md schema detect docs/*.md --merge --output yaml > schema.yaml   # write a schema
md schema triggers docs/post.md                # which schema triggers apply?
md schema about                                # the language reference
```

Every subcommand accepts both output families and `--width`, with the common streams
and exit statuses; see [Output and Width](./output-and-width.md). Each one defaults to
`terminal`, which falls back to the `plain` bytes on a pipe.

## `md schema validate`

Checks Markdown frontmatter, or whole YAML documents, against a schema, and reports
each file on its own row. How schemas are layered and what each message means is in
[the schema reference](../topics/schemas/definition.md#cli-md-schema-validate).

```text
$ md schema validate good.md bad.md
- ✔ the document good.md has a schema definition and is valid based on its inline $schema and the Darkmatter default schema.
- ✗ the document bad.md failed schema validation:
    - count is a required property (at line 2, column 1)
```

| Argument or option | What it does |
|---|---|
| `<FILE_OR_PROP=VALUE>...` | Markdown or YAML files, and `prop=value` assignments applied to every Markdown document before validating |
| `--schema <PATH>` | use this baseline schema instead of the built-in one |
| `--disable <baseline-schema\|schema-triggers>` | stop a layer; see [Policy Flags](./policy.md) |
| `--quiet` | print only failures |
| `-v`, `-vv` | `-v` adds each document's link; `-vv` also adds validation timings to stderr, in formatted outputs only |

The report is the deliverable, so a formatted output writes it to stdout. It exits 0
when every file is valid and 1 otherwise, including when a file cannot be opened or
parsed, or a schema cannot be loaded.

A data output is one object whose `files` array has one record per file argument, in
order. A failing run has `outcome: "failure"` and an empty run-level `diagnostics`:
the problems stay in each file's record, and only a problem with the invocation
itself is a run diagnostic. `-vv` timings are absent from data.

```json
{
  "files": [
    {
      "file": "bad.md",
      "valid": false,
      "schema": null,
      "layers": { "baseline": { "form": "darkmatter" }, "document": { "form": "inline" }, "triggers": [] },
      "problems": [
        { "kind": "missing", "property": "count", "path": "", "line": 2, "column": 1,
          "message": "\"count\" is a required property", "arm_index": null, "description": null }
      ]
    }
  ],
  "outcome": "failure",
  "diagnostics": []
}
```

A baseline that cannot be loaded leaves nothing to validate:

```json
{
  "files": [],
  "outcome": "failure",
  "diagnostics": [{ "code": "dm.schema.baseline", "severity": "error", "...": "..." }]
}
```

A clean success has only `files`. A record that could not be validated names why in
`error`: `file_reference`, `frontmatter_parse`, `yaml_parse`, `schema`, or `slots`. See
[JSON Output](../topics/schemas/definition.md#json-output) for every record field.

### Schema templates

A schema whose definitions hold `{{ … }}` [slots](../topics/schemas/schema-slots.md)
(Claudine's sandboxing schema loads its `os` list from `os.yaml`) is treated two ways:

- **A schema file argument** is checked as written. Its record is `valid: true` with
  `slots: 1` (how many slots it holds); nothing the slot names is read.
- **A document** under the template is checked against the resolved schema. A data
  file committed in the schema's repository is read without asking. Any other read
  is asked about on a terminal, once per run, before the document is checked; a data
  output never asks. A read nobody approved makes the file's check unavailable, so the
  run exits 1:

```json
{
  "file": "provider.md",
  "valid": false,
  "schema": null,
  "error": "slots",
  "message": "the schema's slots could not be resolved: …",
  "problems": [],
  "diagnostics": [{ "code": "dm.schema.slot_approval_required", "...": "..." }],
  "grants_needed": [{ "kind": "file", "target": "/repo/claudine/docs/os.yaml", "scope": "persisted" }, "..."]
}
```

Adding `allow file /repo/claudine/docs/os.yaml` to `~/.config/darkmatter/schema-read-policy`
grants the read in every later run; see
[What a slot may read](../topics/schemas/schema-slots.md#what-a-slot-may-read).

## `md schema detect`

Infers a SimplifiedSchema from one or more documents' frontmatter. Use it to start a
schema from documents you already have.

To write a schema file, ask for the `yaml` output:

```sh
md schema detect a.md --output yaml > schema.yaml
md schema detect a.md b.md --merge --output yaml > schema.yaml
```

| `--output` | Prints |
|---|---|
| `terminal` (default) | the schema as a YAML code block, highlighted on a color terminal |
| `plain`, `markdown`, `markdown-plus` | the same YAML as a fenced ` ```yaml ` block |
| `html` | the same block as HTML |
| `yaml` | the SimplifiedSchema YAML, bare, with no comment header |
| `json`, `json5`, `toml` | the same SimplifiedSchema, in that format |
| `json-schema` | the schema as JSON Schema Draft 2020-12, a value only this command accepts |

The default is a fenced block, because `terminal` falls back to `plain` on a pipe, so
a plain redirect does not produce a loadable schema:

````text
$ md schema detect a.md
```yaml
$schema:
  title: string
  count: number(integer)
  tags: string[]
```

$ md schema detect a.md --output yaml
$schema:
  title: string
  count: number(integer)
  tags: string[]
````

**Several files.** `--merge` combines them into one schema, widening types that
disagree and marking a property required only when it appears in every file. Without
`--merge`, each file keeps its own schema: a data output is
`{ "files": [{ "file", "schema" }] }`, so that form is not itself a loadable schema,
and a formatted output separates them with `# --- a.md ---` comment lines.

**Failures and warnings.** A clean single or merged result is the bare schema. A run
with a warning or a failure wraps it, as `{ "outcome", "diagnostics", "value" }`, with
`value: null` on failure. See [Data Contracts](./data-contracts.md#other-commands).

## `md schema triggers`

Explains schema-trigger discovery for a document: the roots searched, in order,
envelopes shadowed by a nearer root, triggers skipped as invalid, and which triggers
matched. See [Schema Triggers](../topics/schemas/schema-triggers.md).

```text
$ md schema triggers notes.md
Document: /work/notes.md
Schema roots (search order):
1. SCHEMAS_DIR: unset
2. package root: none (the document is not in a package)
3. package-area root: none (the document is not in a package area)
4. file tree root: /work/schemas (absent)
5. home: /Users/me/schemas (absent)
Shadowed envelopes:
- none
Skipped invalid triggers:
- none
Triggers:
- none
```

A trigger whose payload holds `{{ … }}` slots is listed with
`(payload pending: holds schema slots)`, and its data record has `pending: true`:
it is valid, nothing is read to list or match it, and its slots resolve only when
a document it matches is composed (see
[Schema Slots](../topics/schemas/schema-slots.md#where-slots-may-appear)).

```text
Triggers:
- /work/schemas/sandbox.trigger.yaml — matched (payload pending: holds schema slots)
```

A document outside every repository fails with status 1. A data output has `file`,
`roots`, `shadowed`, `skipped`, `triggers`, `outcome`, and `diagnostics` on every
run; see [`schema-triggers.yaml`](../../cli/schemas/output/schema-triggers.yaml).

## `md schema about`

Prints the SimplifiedSchema language reference: schema shapes, the type vocabulary,
constraints, and what happens to values during validation and compose. It reads no
document and uses no network.

```sh
md schema about                  # a summary
md schema about -v               # with every detail
md schema about --output markdown > schema-language.md
md schema about --output json    # the whole catalog, for a tool
```

A formatted output depends on `-v`: the default is a summary, and `-v` adds the
details. A data output always holds the whole catalog in `sections`, whatever the
verbosity, with `outcome` and `diagnostics`. It paints only on a color terminal; on a
pipe it is plain text with no escape bytes.

## Removed spellings

These are usage errors (exit status 2).

| Removed | Use instead |
|---|---|
| `md schema validate --format json` | `md schema validate --output json` |
| `md schema detect --format json` | `md schema detect --output json-schema`, which prints the same JSON Schema |
| `md schema detect --format yaml` | `md schema detect --output yaml` |
