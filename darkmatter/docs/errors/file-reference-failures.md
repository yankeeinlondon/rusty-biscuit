# File Reference Failures

When a file reference cannot be resolved, `md` says **why** in one stable
row: `failure: <class>`. Read that row, not the message, when you script
against `md` output or decide how to fix a reference. The message names the
reference and the directories it searched, and its wording may change; the
row does not.

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

## Where the row appears

Every surface that reports a failed file reference carries the row.

A **`md` argument** that resolves to nothing:

```text
$ md compose '&/nothing.md'
⤫ FileReferenceError: file argument not resolved
┃
┃ The argument &/nothing.md did not resolve to a file.
┃
┃ failure: no-match
```

Every command that reads a file argument reports it this way (see
[Every file argument uses this grammar](../cli/index.md#every-file-argument-uses-this-grammar)),
including a malformed one:

```text
$ md schema detect '@'
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
`could not be opened` line, and its `--format json` record carries
`"error": "file_reference"` and `"failure": "<class>"`. `md schema triggers`
on a document outside every repository prints `failure: missing-context`.

A **transclusion** (`::file`, `::code`, `::toc-linking <file>`) found invalid
before composition:

```text
Invalid Transclusion Target(s)
- the guide.md reference to ./missing.md is not valid

  failure: no-match
```

A **schema `file` value**, both in `md compose`'s error block and in
`md schema validate`:

```text
- ✗ the document docs/schema.md failed schema validation:
    - target no existing file matched reference `./nope.md` while resolving
      from `/work/repo/docs` (at line 4 of frontmatter)
        a union type of: enum | string
        failure: no-match
```

A **tolerated failure**, where composition replaced the content with a
notice and carried on, prints the same row under its warning on stderr.

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

## From the library

Library callers read the same class without parsing text:
`MarkdownError::resolution_failure()`, `ComposeWarning::resolution_failure`,
and `ContextBuildError::resolution_failure()`. See
[compose requests](../topics/compose-requests.md#reading-a-failures-class).
The language server attaches the class to its file-reference diagnostics as
`data: {"resolution_failure": "<Class>"}`.
