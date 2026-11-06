---
kind: canonical-documentation
---
# File Reference Failures

When a file reference cannot be resolved, `md` says **why** with one stable
class: a `failure: <class>` row in an error block, or a `failure` field in a
[data output](../cli/data-contracts.md). Read the class, not the message, when
you decide how to fix a reference, and read the data output's field when you
script against `md`. The message names the reference and the directories it
searched, and its wording may change; the class does not.

```text
⤫ TransclusionError: file reference failure
┃
┃ relative reference `../../outside.md` leaves file tree `/work/repo` through
┃ candidate `/work/outside.md`
┃
┃ failure: invalid-reference
┃
┃ Check sigil usage: `@` magic, `&` repository root, `^` repository-scoped.
```

## What each class means

| Row | Meaning | What to do |
|---|---|---|
| `failure: no-match` | The reference was valid, but no candidate is a file. | Check the spelling, or that the file exists where the reference starts (`./` beside the document, `&` at the repository root, `@` along the magic search order). |
| `failure: invalid-reference` | The reference cannot be used as written: it is not valid reference syntax (a bare `@`, `&`, or `^`, or the removed `!` sigil, even when a file of that literal name exists), `../` leaves the document's file tree (or, for an `md` argument, the repository you ran `md` in), or `{{VAR}}` expands to a relative path where an absolute one is required. | Rewrite the reference: spell a literal file named `@` as `./@`, use `&` or `@` instead of climbing out with `../` (an `md` argument may also be an absolute path), or fix the variable's value. |
| `failure: missing-context` | The reference needs an anchor this request does not have: `&` or `^` outside a repository, `~` with no home directory, `{{VAR}}` with `VAR` unset, or a request whose context could not be built. | Run from inside the repository, set the variable, or open the document so its anchor exists. |
| `failure: io` | A candidate exists but could not be read, usually a permission error. | Fix the file's permissions. |
| `failure: unsupported-remote` | A remote URL was used where only local files are allowed. | Use a local path, or a directive that supports remote reads (`::file`, `::code`) with the host allowed. |

The rows are the kebab-case names of biscuit-file's `ResolutionFailure`
classes (`NoMatch`, `InvalidReference`, `MissingContext`, `Io`,
`UnsupportedRemote`).

## Where the class appears

Every surface that reports a failed file reference names the class. The
[reference report](#the-reference-report) states it as a reason phrase and in
its data; the other surfaces print the row.

A **`md` argument** that resolves to nothing:

```text
$ md compose '&/nothing.md'
⤫ FileReferenceError: file argument not resolved
┃
┃ The argument &/nothing.md did not resolve to a file.
┃
┃ failure: no-match
```

Commands that read a file argument report it this way (see
[Every file argument uses this grammar](../cli/index.md#every-file-argument-uses-this-grammar)),
including a malformed one:

```text
$ md toc '@'
⤫ FileReferenceError: file argument not resolved
┃
┃ The argument @ did not resolve to a file.
┃
┃ file reference syntax is invalid: the `@` sigil requires a relative
┃ payload: `@`
┃
┃ failure: invalid-reference
```

`md schema validate` prints the same block under the file's
`could not be opened` line, and its `--output json` record carries
`"error": "file_reference"` and `"failure": "<class>"`. `md schema triggers`
on a document outside every repository prints `failure: missing-context`.

`md schema detect`'s formatted output is the exception: it prints
`Error: Failed to load file: "@"` with no row. In a data output, every other command
reports such an argument as a `dm.input.read_failure` diagnostic whose `failure` field
is the class and whose `reference` field is the argument as written.

A **schema `file` value**, both in `md compose`'s error block and in
`md schema validate`, here for a `docs/schema.md` whose fourth line is
`target: ./nope.md`, so the location is the start of that value:

```text
- ✗ the document docs/schema.md failed schema validation:
    - target no existing file matched reference `./nope.md` while resolving
      from `/work/repo/docs` (at line 4, column 9)
        a union type of: enum | string
        failure: no-match
```

A **tolerated failure**, where composition replaced the content with a
notice and carried on, prints the same row under its warning on stderr. A library
caller that composes without the reference check gets the row at the end of the
typed error's block (`TocLinkingError::Unresolved`, a transclusion error).

A **glob reference** reports the class its pattern's prefix would give a
single file reference. `::file-links <glob>` prints the row under its
warning, and a `find_files()` expression ends its error block with it:

```text
⤫ MarkdownError: interpolation failed
┃
┃ A document expression failed to evaluate `find_files('../../**/*.md')`:
┃
┃ find_files(): glob reference `../../**/*.md` leaves file tree `/work/repo`
┃ through `/work`
┃ …
┃ failure: invalid-reference
```

A `::toc-linking` chain reports the class of its first (authored) target.
A failure inside a nested transclusion reports the class of the reference
that actually failed, however deep it is.

`claudine compose`, `inline-compose`, and `sequence` print the same row:
on a failed transclusion, on a schema `file` value (one row per failing
value, below the problem list), and under a tolerated failure's warning.

### The reference report

`md compose` checks the references in a document and everything it transcludes
before composing, and prints every broken one together. `md validate refs` and
`md graph --validate` print the same report. Problems are grouped by the document
that holds them, and each row gives the reason, the construct, the reference as
written, and where it is:

```text
$ md compose guide.md
Broken references in guide.md

guide.md
  file not found in hyperlink  `./missing.md`  guide.md:3:5

shared/intro.md (transcluded at guide.md:8)
  file not found in `::code`  `./examples/basic.rs`  intro.md:9:1

Checked 5 references across 2 documents: 3 ok, 2 broken
```

The report has no `failure:` row, because its reason phrase says the same thing
in plain words. To script against it, ask for a data output and read each
diagnostic's `failure` field, which holds the class in the table above:

```text
$ md compose guide.md --output json | jq -r '.diagnostics[] | "\(.failure) \(.reference)"'
no-match ./missing.md
no-match ./examples/basic.rs
```

A diagnostic's `line` and `column` are
[authored positions](./composition-problems.md#how-a-problem-is-located).
References are collected before anything stops; see
[What stops a run](./composition-problems.md#what-stops-a-run).

## A miss that looks like a glob

A single file reference never reads glob syntax, so `docs/*.md` names one
file called `*.md`. When such a reference matches nothing, every surface that
reports the `no-match` adds a `hint:` explaining the literal reading and
pointing at a form that accepts a glob reference:

```text
$ md compose 'docs/*.md'
⤫ FileReferenceError: file argument not resolved
┃
┃ The argument docs/*.md did not resolve to a file.
┃
┃ failure: no-match
┃
┃ `*`, `?`, and `[` are literal in a file reference; to match a set of
┃ files, use a form that accepts a glob reference (for example `::file-links`)
```

The same hint follows a `::file`, `::code`, or `::toc-linking` target, a
read-side function argument such as `markdown_title('docs/*.md')`, a schema
`file` value, a `$schema` reference, a broken link or transclusion
diagnostic in the language server, and `claudine compose`'s operation file.
A plain missing name (`docs/missing.md`) and every class other than
`no-match` carry no hint. To list or transclude a set of files, use
`::file-links <glob>` or `find_files()`.

## From the library

Library callers read the same class without parsing text:
`MarkdownError::resolution_failure()`, `ComposeWarning::resolution_failure`,
and `ContextBuildError::resolution_failure()`. See
[compose requests](../topics/compose-requests.md#reading-a-failures-class).
The language server attaches the class to its file-reference diagnostics as
`data: {"resolution_failure": "<Class>"}`.
