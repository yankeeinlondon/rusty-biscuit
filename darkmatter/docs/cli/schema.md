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
itself is a run diagnostic. `-vv` timings are absent from data. A validated record whose
verdict relied on a `default(...)`, filling a missing or `null` value of a required or
optional property, lists each one in `defaults`, as
`{ "path": "greeting", "value": "hello", "schema_location": "/properties/greeting/default" }`
(`schema_location` is the JSON Pointer of the `default` keyword in the lowered
schema, and a `schema_file` is added when a schema file declares the default: a
referenced schema file, a trigger file, or the file a `Name@file` import read the
type from); the key is absent when there are none. The file itself is never changed.

A problem whose fix the schema proves also carries a `hint` (absent
otherwise), and the formatted output shows it on a `hint:` line under the
problem. A key the schema declares `removed(<replacement>)` is a `removed`
problem whose hint is the replacement; an unknown key near one declared key
gets `` did you mean `name`? ``:

```json
{ "kind": "removed", "property": "fail_fast", "path": "", "line": 3, "column": 1,
  "message": "key `fail_fast` has been removed",
  "hint": "set fail_fast on a group object", "arm_index": null, "description": null }
```

A default is still checked with the rest of the value. When it fails a check that
needs the file system, such as `doc: file(required; eager; default(./note.md))` with
no `note.md`, the problem is the default's, not the document's: its record carries
the same `default` object, its `line` and `column` are `null`, and its message ends
by naming the default's schema location (and schema file). The formatted output
adds a `hint:` line under the problem saying the same: repair the default where
the schema declares it, or write the property in the document.

```json
{ "kind": "invalid", "property": null, "path": "/doc", "line": null, "column": null,
  "message": "no existing file matched reference `./note.md` while resolving from `/work` (the value is the schema default at schema location `/properties/doc/default`)",
  "default": { "path": "doc", "value": "./note.md", "schema_location": "/properties/doc/default" },
  "arm_index": null, "description": null }
```

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
(Claudine's sandboxing schema loads its `os` list from `os.yaml`) is always resolved
before anything is judged, whether the argument is the schema file or a document under
it. Both go through the same read approval:

- **A schema file argument** has its shape checked, then every slot it holds is
  resolved exactly as a document naming the file would resolve it, and the resolved
  schema is checked: a template type's returned definition, the imports that
  definition brings, and its suffix. A definition that is not a type, a data file that
  is gone, or a read nobody approved makes the check unavailable (`error: slots`,
  exit 1); the file is never reported valid as written.
- **A document** under the template is checked against the resolved schema. A
  `<prop>=<value>` assignment is converted by the resolved type, so `label=123` under a
  slot that returns `alphanumeric` arrives as the text `123`; when the template does not
  resolve, the assignment is never judged and the check is unavailable. A file
  your `permissions.file-read` pre-approves is read without asking (see
  [How a file read is decided](../topics/schemas/schema-slots.md#how-a-file-read-is-decided)). On a
  terminal, every other read whose target is written in the slot is asked about in one
  batch, just before that document's schema resolves, with the numbered prompt
  `md compose` shows (see [Prompts](../topics/schemas/schema-slots.md#prompts)); a read
  whose target is computed is asked about when its slot evaluates. An answer holds for
  every later file in the run. A data output never asks, and neither does a run without
  a terminal. A read nobody approved makes the file's check unavailable, so the run
  exits 1:

```json
{
  "file": "provider.md",
  "valid": false,
  "schema": null,
  "error": "slots",
  "message": "the schema's slots could not be resolved: …",
  "problems": [],
  "diagnostics": [{ "code": "dm.schema.slot_approval_required", "...": "..." }],
  "grants_needed": [
    { "kind": "file", "target": "/repo/claudine/docs/os.yaml", "scope": "once" },
    { "kind": "file", "target": "/repo/claudine/docs/os.yaml", "scope": "persisted" }
  ]
}
```

Adding an entry that matches `/repo/claudine/docs/os.yaml` to `permissions.file-read`
(for example `&claudine/docs/os.yaml` in your `~/.darkmatter.yaml`) approves the read in
every later run, as choosing "Always allow this file" at the prompt does; see
[What a slot may read](../topics/schemas/schema-slots.md#what-a-slot-may-read).
`md schema validate` takes no `--allow host=HOST`, so a slot's URL read needs
`permissions.network: allow` or an answer for this run here (see
[Schema slot reads](./policy.md#schema-slot-reads)).

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
envelopes shadowed by a nearer root, triggers skipped as invalid, the registered
host that identifies the document, and which triggers matched. See
[Schema Triggers](../topics/schemas/schema-triggers.md) and
[Host Registration](../topics/host-registration.md).

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
Registered host:
- none
Triggers:
- none
```

When a [registered host](../topics/host-registration.md) identifies the document,
the "Registered host" section names it, its version, the manifest it was loaded
from, and its content revision, then lists each of the host's triggers in manifest
order. Those triggers layer below the schema roots' triggers:

```text
Registered host:
- acme 1.2.0 (manifest /Users/me/.acme/darkmatter-host.yaml, revision 5b1c…)
- /Users/me/.acme/schemas/widget.yaml — matched
  - condition 1 — matched
- /Users/me/.acme/schemas/gadget.yaml — not matched
  - condition 1 — defeated: `gadget` is required but absent
```

When two matched triggers of the host
[cannot be layered as one](../topics/host-registration.md#when-two-triggers-declare-one-property),
the section lists both matches and then the conflict, naming both trigger files,
and the run fails with status 1: the same `dm.host.trigger_conflict` that
`md schema validate` and `md compose` refuse the document with. Matching
triggers whose declarations agree report success.

```text
Registered host:
- acme 1.2.0 (manifest /Users/me/.acme/darkmatter-host.yaml, revision 5b1c…)
- /Users/me/.acme/schemas/a.yaml — matched
  - condition 1 — matched
- /Users/me/.acme/schemas/b.yaml — matched
  - condition 1 — matched
- conflict [dm.host.trigger_conflict]: /Users/me/.acme/schemas/a.yaml and /Users/me/.acme/schemas/b.yaml conflict on agent: their types are incomparable
```

A trigger that declares a `name` is written `name (path)` in the conflict line
(`` `team-agent (/Users/me/.acme/schemas/a.yaml)` ``).

In a data output, `registered.conflict` holds `first`, `second`, their `name`
labels `first_name` and `second_name` (`null` for a trigger without one),
`subject` (the property, `null` for the root rules), and `reason`, and
`diagnostics` holds the
`dm.host.trigger_conflict` error; `outcome` is `failure`. A matched trigger whose
payload is a template is left out of this check, since listing it reads nothing.

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
run, and `registered` (`host`, `version`, `manifest`, `revision`, the host's
`triggers`, and `conflict`), which is `null` when no single registered host identifies the
document; see [`schema-triggers.yaml`](../../cli/schemas/output/schema-triggers.yaml).

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
