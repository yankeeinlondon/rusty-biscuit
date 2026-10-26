---
kind: canonical-documentation
---

# Composition Reports

`md compose --output report-json` composes a document and prints one JSON
object that says whether it worked, lists every diagnostic the run reported,
and carries the composed document. Use it from scripts, CI jobs, and editors
that need the result and its problems in one machine-readable value instead
of a document on stdout and styled warnings on stderr.

```sh
md compose notes.md --output report-json
```

```json
{
  "outcome": "failure",
  "diagnostics": [
    {
      "code": "dm.expression.incompatible_call",
      "severity": "error",
      "message": "min: argument 1 (`a`) is boolean, expected number (category 5: no value of that type can bind)",
      "source": { "file": "notes.md", "line": 4, "column": 18 },
      "expected_type": "number",
      "actual_type": "boolean"
    }
  ],
  "document": null
}
```

Only `md compose` has this format. `md render`, `md FILE`, `md clean`, and
`md schema validate` refuse `report-json` as an unsupported output format
(exit 2) before they read anything; their own JSON formats (`--output json`
for the Markdown AST, `md schema validate --format json` for a multi-file
report) are unchanged.

## The object

Every field is always present.

| Field | Value |
|---|---|
| `outcome` | `"success"` or `"failure"` |
| `diagnostics` | every diagnostic the run reported, in the order below; `[]` when there were none |
| `document` | the composed Markdown on success (as `--output markdown` would print it, honoring `--frontmatter`); `null` on failure |

Each diagnostic has six fields:

| Field | Value |
|---|---|
| `code` | a stable `dm.<family>.<kind>` code, shared with the editor (DMLS) |
| `severity` | `"error"`, `"warning"`, or `"info"` |
| `message` | the problem in words |
| `source` | `{ "file", "line", "column" }` where the authored text starts, or `null` when the problem has no known position |
| `expected_type` | for a call argument, the parameter's declared type; otherwise `null` |
| `actual_type` | for a call argument, the argument's type; otherwise `null` |

`line` and `column` are one-based, and the column counts characters. `file`
names the root document as you gave it on the command line, and a transcluded
document by its resolved path; it is `null` for stdin. A problem inside
transcluded content points at the transcluded file, not at the `::file`
directive that included it.

The same problem has the same fields everywhere: a call the editor marks with
`dm.expression.incompatible_call` at `notes.md:4:18` is the record
`md compose` rejects the document with and the record this report prints.

## Exit status and streams

| Run | `outcome` | `document` | Exit status |
|---|---|---|---|
| composed | `"success"` | the Markdown | 0 |
| failed while composing (a rejected call, a schema problem, a failing expression or shell, a policy stop) | `"failure"` | `null` | 1 |
| a lifecycle run that failed | `"failure"` | `null` | the run's status (1, or 130 when interrupted) |
| a reference no recovery covers (a missing link target) | `"failure"` | `null` | 2, as without the report |
| a usage error (an unknown flag, `--allow x --deny x`) | no report | | 2, with clap's message on stderr |

Once the arguments parse, the command writes exactly one object, including
when the input file cannot be read. Stdout carries nothing else: a lifecycle
`stdout:` action's line and a lifecycle command's output go to stderr, and so
do `-v` progress and `--perf` timings. No warning row or error block repeats
the report on stderr.

`--show` (there is no document file to open) and `--shell` (it prints its own
report) cannot be combined with `report-json`.

```mermaid
flowchart LR
    A[arguments parse] -->|usage error| U[stderr, exit 2, no report]
    A --> R[read input, prepare, compose]
    R -->|each warning| D[(diagnostics)]
    R -->|success| S["outcome: success, document"]
    R -->|failure| F["outcome: failure, document: null"]
    D --> S
    D --> F
    S --> O[one JSON object on stdout]
    F --> O
```

## Which diagnostics, in which order

- **Warnings** come in the order the run found them: passive checks first,
  then what composition reports while it runs. The passive checks report a
  lifecycle stack's calls (event by event, each surface in authored order),
  then the document's calls by authored position: frontmatter first, then
  the body top to bottom, with `{{ … }}` expressions and `::block`/`::file`
  conditions interleaved where they are written. Transcluded documents report
  in the order their directives appear.
- **A failure** comes after the warnings reported before it. Earlier warnings
  are kept: a category 3 call reported before a failing `{{ 1 / 0 }}` is in
  the failure report too. A rejected call keeps the passive warnings found
  before it in that order, including an earlier call in its own expression;
  a finding after the rejected call is not reported, since the check stops
  there. A failure in a transcluded document comes after every warning of
  the documents that transclude it, outermost first, then its own: a
  parent's passive check covers the whole parent before any child composes,
  so a parent warning written after the `::file` line is reported too.

  ```text
  parent.md                         child.md
  1 {{ file_exists("") }}           1 {{ file_exists("https://") }}
  3 ::file ./child.md               3 {{ min(true, 5) }}
  5 {{ file_exists("http://") }}
  ```

  reports `always_false_call` at `parent.md` lines 1 and 5, then at
  `child.md` line 1, then `incompatible_call` at `child.md` line 3.

  ```json
  { "outcome": "failure",
    "diagnostics": [
      { "code": "dm.expression.uncertain_call", "severity": "warning", "…": "…" },
      { "code": "dm.expression.evaluation_failure", "severity": "error", "…": "…" } ],
    "document": null }
  ```

- **A policy stop** (`--deny`) lists each warning once, with its own severity:
  a denied `warning` is still `"warning"`. The policy decides the outcome, not
  the diagnostic.
- A check that stops a run before composition starts reports what that
  check found. A rejected call the pre-flight walk finds (the walk checks
  each document before it runs a shell) reports the same warnings ahead of
  it as the compose pass would: those of the document that holds it, after
  those of every document that transcludes it.
- **A rejected call** reports the call's own record wherever it is authored:
  in the body, in frontmatter, in a `::block` or `::file` condition, or on a
  lifecycle surface (an action's text, a `when:` condition, a `shell:`
  command, a side-effect argument such as `ensure_file:`, a `set:` value, or
  a control reason such as `error:`). The record is the one DMLS shows for
  the same bytes, located in the file as written. A lifecycle rejection stops
  the run before `initialize`, so no lifecycle effect runs; the lifecycle
  warnings found before it come first.

  ```yaml
  ---
  composed:
    - stdout: "{{ min(true, 5) }}"
  ---
  ```

  ```json
  { "code": "dm.expression.incompatible_call", "severity": "error",
    "message": "min: argument 1 (`a`) is literal true, expected number (…)",
    "source": { "file": "doc.md", "line": 3, "column": 21 },
    "expected_type": "number", "actual_type": "literal true" }
  ```

## Codes

Call findings use the editor's codes (`dm.expression.incompatible_call`,
`uncertain_call`, `always_false_call`, `incomplete_analysis`), schema problems
the editor's schema codes (`dm.schema.type_mismatch`, `constraint`,
`missing_required`, `unknown_key`, `invalid_file_reference`, `prepare`), and a
recovered problem the code [`--deny`](./policy.md) takes for it. A failure is
classified by what failed:

| Code | Failure |
|---|---|
| `dm.frontmatter.yaml_parse` | the frontmatter YAML, or its `---` fence, does not parse; located |
| `dm.input.read_failure` | the input, or a file it needs, cannot be read |
| `dm.expression.parse_failure` | an expression does not parse |
| `dm.expression.evaluation_failure` | an expression fails to evaluate; located when the expression is authored in the file |
| `dm.expression.unknown_function` | an expression calls a function that does not exist |
| `dm.directive.parse_failure` | a `::` directive does not parse |
| `dm.shell.failure` | a shell command fails, times out, or is refused |
| `dm.transclusion.failure` | a transclusion fails for another reason |
| `dm.reference.failure` | a file reference does not resolve |
| `dm.compose.parameter_binding` | a supplied value does not bind to its parameter |
| `dm.lifecycle.failure` | a lifecycle event fails (an `error:` action, a failed action) |
| `dm.compose.failure` | anything else |

A warning whose producer declares no code reports `dm.compose.warning`.

A [schema trigger skipped because it failed to load](../topics/schemas/schema-triggers.md#invalid-discovered-triggers)
is a `dm.schema.invalid_trigger` warning. It belongs to the trigger, not to
the composed document, so its `source` is `null`, and its `message` names the
trigger file (or `embedded:{bundle}/{file name}`) and the reason. The report
lists each trigger once, even when transcluded documents discover it again:

```json
{ "code": "dm.schema.invalid_trigger", "severity": "warning",
  "message": "ignoring invalid schema trigger /repo/schemas/memory.yaml: invalid schema-trigger: …",
  "source": null, "expected_type": null, "actual_type": null }
```

`md schema validate --format json`, which reports several files, puts a
trigger warning that files share in the `warnings` of the first file that
discovers it, once per invocation, and not in the later files' records.

## Privacy

A diagnostic never contains an evaluated argument value, an environment
value, or a file's contents, and producing the report evaluates, renders, and
reads nothing again. Messages can still quote authored source text, as the
terminal output does.

## Effects are not rolled back

The report waits for the outcome, so no partial document is printed and
nothing is published before blocking checks pass. It is not a transaction:
a lifecycle effect or cache write that already happened stays done when the
run later fails.
