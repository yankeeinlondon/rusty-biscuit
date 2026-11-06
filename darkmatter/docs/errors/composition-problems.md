---
kind: canonical-documentation
---
# Composition Problems

Use this page to decide how to read a failed `md compose`: whether fixing the
problem you see is likely to reveal another, and where each reported position
points. It applies to `md compose` and to library callers. The exact fields of a
data output are in [Data contracts](../cli/data-contracts.md#diagnostics), and the
exit statuses in [Output and width](../cli/output-and-width.md#exit-status).

## What stops a run

A run finds problems in **families**. Each family has a *stopping rule*, which
says whether the run keeps looking for more problems of that family after it
finds the first one. It tells you what to expect after a fix: a references report
is the whole list, while an expression error may have another behind it.

| Family | Stopping rule | What you see |
| --- | --- | --- |
| References | Every problem is collected, then reported together | One grouped report, before anything is composed |
| Schema | Every problem in one document is collected, then the run stops | Every problem in that document, none from later documents |
| Transclusion | A fatal failure, such as a cycle, stops the run; a tolerated failure is reported and composition continues | A fatal error, or a notice in the document and a warning |
| Expression | The run stops at the first error it meets, which is not always the earliest in the document | One error |
| Shell | The run stops at the first error | One error |
| Lifecycle | The run stops at the first error, then runs the [cleanup events](../topics/lifecycle/error-handling.md) | One error, plus what cleanup reports |

A warning never stops a run on its own. To make one stop it, name its code with
[`--deny`](../cli/policy.md).

For example, this document has two broken links and an expression that fails.
The first run reports only the links, together, because references are checked
before anything is composed. The expression's error appears once they are fixed:

```md
[one](./missing-one.md)
[two](./missing-two.md)

{{ 1 / 0 }}
```

```text
$ md compose doc.md
Broken references in doc.md

doc.md
  file not found in hyperlink  `./missing-one.md`  doc.md:1:1
  file not found in hyperlink  `./missing-two.md`  doc.md:2:1

Checked 2 references across 1 document: 0 ok, 2 broken
```

Only the reference report groups its problems. Every other family keeps its own
error block in a terminal.

`md validate refs --fail-fast` and the library's `fail_fast` option stop reference
checking at the first error. `md compose` always collects them.

## How a problem is located

In a data output, a problem's `source` says where to look. Its positions are always
**authored**: they point at the file as you wrote it, not at the text composition
was working on, so you can jump straight to them. Here `shared/intro.md` has a
broken `::code` target on line 9, and `types.md` pulled it in on line 8:

```json
{
  "code": "dm.reference.missing_transclusion",
  "source": {
    "file": "/work/repo/shared/intro.md",
    "line": 9,
    "column": 1,
    "section": "getting-started",
    "inserted": false
  },
  "reason": "file not found",
  "construct": "::code",
  "reference": "./examples/basic.rs",
  "failure": "no-match",
  "included_via": [
    { "file": "/work/repo/types.md", "line": 8 }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `source.line`, `source.column` | The authored position. Both are 1-based and count the complete file, frontmatter included. For a reference, the column counts characters rather than bytes, and CRLF line endings do not change either number. `null` when the document has no authored positions (built in memory or fetched from a URL), and never a placeholder. |
| `source.inserted` | `true` when an expression, a shell step, or a `replace:` value wrote the reference. It has no authored text of its own, so `line` and `column` name the construct that wrote it: the `{{ … }}`, the `$( … )`, or the replaced key. |
| `source.section` | The composed anchor slug of the innermost heading at or before the problem. `# Intro {{ title }}` with `title: Root` gives `intro-root`, the anchor that exists in the composed document. |
| `reason`, `construct`, `reference`, `failure` | Reference problems only: the report's phrase, the construct, the reference as written, and the [failure class](./file-reference-failures.md). |
| `included_via` | Reference problems only: the directives that pulled the document in, outermost first, each with its `file` and authored `line`. Empty for the named document. |

`source` is `null` when the problem has no known place, as with a shell failure or a
transclusion cycle. Schema problems report a [location](../topics/schemas/definition.md#problem-locations)
when their producer has one.

Reference graphs keep a second set of numbers. `ReferenceOrigin::line` and `span`
count lines in the prepared body, after frontmatter removal and expansion, because
code that walks the graph works on that text. No report or data output shows them.
