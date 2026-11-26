---
kind: canonical-documentation
---
# The Error-Message Standard

Most people who meet a Darkmatter or Claudine error are writing a document: a
prompt, a sequence, a schema. The message is the only thing between them and
a fix. This page is the standard every new or revised error and finding is
written to, in both packages, so that a reader can always answer the same
four questions from the same places. It applies to an error block in the
terminal, to a row of a report such as `md compose`'s
[combined report](../cli/compose.md#the-combined-report) or Claudine's
`--dry-run` report, and to the JSON record those rows are made from (see
[Data contracts — Diagnostics](../cli/data-contracts.md#diagnostics)).

## The four parts

Every message states four things:

1. **What failed, in the author's terms.** Name the construct by the word the
   author wrote (`prompt:` of task `metadata`, `::file`), the value as written,
   and the resolved form when there is one.
2. **Where.** The file and the position. Show a position only when it is
   proven to be where the author wrote the construct; otherwise name the file
   alone, and never guess a line.
3. **Why.** The cause that actually applies. A message never lists causes it
   has not established ("missing, or not readable, or excluded by a
   condition"). When one error can have several causes, the cause is carried
   as data and only the one that applies is rendered.
4. **How to fix it.** One concrete next step, in the hint.

### An example

A Claudine sequence task names a prompt file that does not exist:

```text
task metadata (sequence[1])
  ✗ the prompt file does not exist
      prompt: ./_frontmatter.md
      resolved to renderers/_frontmatter.md (relative to renderers/_fleet.yaml)
      at renderers/_fleet.yaml:12:15
      hint: create renderers/_frontmatter.md before a real run, or
            correct the reference
```

| Part | Where it is |
| --- | --- |
| What | `the prompt file`, under the subject `task metadata`, and the construct as written, `prompt: ./_frontmatter.md` |
| Where | `at renderers/_fleet.yaml:12:15`, the authored position of the value |
| Why | `does not exist`, with the path it resolved to and the file it was resolved against; nothing about permissions, because no permission problem was found |
| Fix | the `hint:` line |

The same finding as JSON (abridged) keeps each part in its own field, so a
program never has to parse the prose:

```json
{
  "code": "dm.reference.missing_document",
  "severity": "error",
  "message": "the prompt file does not exist",
  "source": { "file": "/work/renderers/_fleet.yaml", "line": 12, "column": 15, "inserted": false },
  "construct": "prompt:",
  "reference": "./_frontmatter.md",
  "failure": "no-match",
  "resolved": "/work/renderers/_frontmatter.md",
  "hint": "create renderers/_frontmatter.md before a real run, or correct the reference",
  "trail": [{ "kind": "host", "host": "task", "label": "metadata", "property": "sequence[1].prompt" }]
}
```

When the position is not proven, the row names the file alone:

```text
  ✗ `topic` (string) is required and has no value; a non-interactive run fails
    here
      in needs.md
      hint: add `topic` to the task's `with:`, or pass `topic=<value>`
            after the file
```

## The rules that go with it

### Nouns come from the construct

The noun in a message is the one for the construct the author wrote, never
the name of the code path that failed. A `prompt:` reference is "the prompt
file", a `sequence:` reference "the sequence file", and a `::file` target a
file "in `::file`", even when the same loader reads all three. Each side
supplies the nouns it owns: Claudine for its task fields (`prompt:`,
`task:`, `group:`, `sequence:`), Darkmatter for its directives and
frontmatter keys. A message that hard-codes a noun such as "external
sequence file" is a defect.

### Context travels as data

The task, key, transclusion, or `::map` item that led to a problem is not
glued onto the front of the message. It rides on the record as typed data:

- the **trail**, outermost first: an `included` frame per transclusion, an
  `iteration` frame per `::map`/`::filter` item, and a `host` frame per host
  construct, such as a Claudine task (see
  [Trail](../cli/data-contracts.md#trail));
- **related** locations, such as the finding that stopped a later check;
- **notes**, supporting lines with no location of their own, such as each
  link of a provider selection chain.

A report renders that context once, as the heading the rows are grouped
under (`task first → inner (sequence[0])`), so the same message reads the
same wherever it is reached from.

### Severity belongs to the finding

Each finding has one severity, decided by the check that produced it:

| Severity | Means |
| --- | --- |
| `error` | a proven failure of the run being checked |
| `warning` | an actionable concern, or a problem the real run could still resolve by asking |
| `info` | context, including a check that could not run (`dm.check.not_checked`) |

A message never says "error" when its severity is a warning, and its words
match the verdict: a missing input reads "an interactive run asks for it"
as a warning and "a non-interactive run fails here" as an error.

A caller's policy decides what *blocks*, never what a finding *is*. In
`md compose`, [`--deny`](../cli/policy.md) makes a warning fail the run and
`--allow` recovers a problem, and both leave `severity` as it was (the one
exception, a `style` key made blocking on `md render`, is listed in
[Data contracts — Diagnostics](../cli/data-contracts.md#diagnostics)); a
Claudine dry run exits `2` on any error or warning without changing either.

### Advice is not a failure

Some findings are **advisory**: a key one edit away from a declared field
(`promt` beside `prompt`, `dm.schema.near_miss_key`), or a step's own
`agent:` that a Claudine task reads as data rather than as its provider. An
advisory is a warning beside a value that is still accepted. It is never
among a strict validation failure's problems, it never makes a document
invalid, and a run that would otherwise succeed still runs.

### Origins are honest

A value the document did not write is never blamed on the document. When a
value came from a command-line `--set`, a setter, or a host, the message says
where it came from (`The value came from a command-line override (--set), not
from the document.`) and shows no excerpt of the document. In JSON such a
problem names the document with a `null` line and column.

### Rendering conventions are unchanged

The standard decides *what* a message says. How an error block is built
(a `StatusBlock` with a `Vec<Prose>` body, `SourceContext` excerpts, escaped
user tokens) is in [Error Rendering Conventions](./README.md). A source
excerpt is shown only for a proven position, and its line numbers count
lines of the file as written.

## How the standard is enforced

Rules that only live in a document decay, so each one has a test that fails
when it is broken:

| Mechanism | Owner | Fails when |
| --- | --- | --- |
| Finding builder: `Diagnostic::actionable(code, severity, message, hint)` takes the hint as a required argument | Darkmatter | a new actionable finding has no fix step |
| One catalog per code: each finding code is declared once, in `md compose`'s catalog when `md` emits it, else in the findings catalog | Darkmatter | a finding code is declared in both, or a code `md compose` emits is missing from its catalog |
| Catalog snapshots: every Claudine dry-run code has a fixture, a control that produces nothing, and plain-text and JSON snapshots | Claudine | a code has no snapshot, or a snapshot names no code |
| Site guard: every Claudine `Sequence…` error variant carries its site (file and authored property) | Claudine | a sequence error loses its task and key |
| Noun guard: construct nouns are refused in error texts outside the noun table | Claudine | a message hard-codes a noun |
| Catch-all guard: only listed variants may reach the generic "composition failed" block | Claudine | a new sequence error falls through to it |
| Mapping guard: a preflight report keeps the typed error a lifecycle handler matches on | Claudine | a report changes which `err.*` a `when:` clause sees |

Claudine's guards are described in
[Claudine's error architecture](claudine/docs/topics/error-architecture.md#the-lossy-boundary-audit).

## See also

- [Composition problems](./composition-problems.md): what stops a run, and
  how a problem is located
- [File reference failures](./file-reference-failures.md): the class every
  failed file reference carries
- [Data contracts — Codes](../cli/data-contracts.md#codes): every `dm.*` code
