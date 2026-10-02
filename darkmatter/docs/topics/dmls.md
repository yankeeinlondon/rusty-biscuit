# DMLS: File References in the Editor

DMLS, the Darkmatter language server, resolves a file reference the same way
`md compose` does when you run it from the repository root. A reference that
composes is a working link in your editor; one that fails to compose is
flagged. That holds for every reference form Darkmatter accepts (`./`, bare,
`&`, `^`, `@`, `~`, `{{VAR}}`, and absolute; see
[Local File Referencing](./file-referencing.md)), wherever the reference
appears:

- Markdown links (`[setup](^setup.md)`), including `#anchor` completion and
  missing-anchor checks against the target document
- `::file`, `::code`, and `::toc-linking` targets
- `$schema` and `file(...)`-typed frontmatter values, and `prologue` and
  `epilogue` paths

```markdown
<!-- repo/area/pkg/docs/guide.md -->
[setup](./setup.md)          <!-- beside this document -->
::file &README.md            <!-- the repository root's README.md -->
::toc-linking ^CHANGELOG.md  <!-- the package's, else the area's, else the root's -->
::code {{NOTES}}/snippet.rs  <!-- NOTES as the editor's environment defines it -->
```

Links and directive targets get a clickable link and go-to-definition when
the target exists, and a broken-path diagnostic when it does not. The one
exception is a directive target containing `{{ … }}`, such as the `::code`
line above: DMLS links it when it resolves but never flags it as broken,
because a `{{ … }}` target may be an expression only composition can
evaluate.

A `::toc-linking` target can be a fallback chain, and DMLS reads it the way
composition does: the first alternative that exists is the target, and a
chain ending in `false` that matches nothing is intentional, not broken.

```markdown
::toc-linking "&missing.md | &CHANGELOG.md"   <!-- links to CHANGELOG.md -->
::toc-linking "&NOTES.md | false"             <!-- no NOTES.md: renders nothing, no warning -->
::toc-linking "&a.md | &b.md"                 <!-- neither exists: one warning over the chain -->
```

The link and go-to-definition point at the selected alternative, hover names
it (or says the chain renders nothing), and a broken chain's diagnostic
carries the first alternative's failure class, as `md compose` reports it.

A directive target has no `#anchor` syntax. `::file guide.md#setup` names a
file called `guide.md#setup`, in the editor as in composition, so it is a
broken target unless that file exists. (A Markdown link's `#anchor` is still
an anchor.)

## One context per repository

To resolve a reference, DMLS needs a **file-resolution context**: the
repository root, its packages (for `^`), the `@` search roots, `HOME` (for
`~`), and the environment (for `{{VAR}}`). DMLS builds it with Darkmatter's
`build_resolution_context`, the same builder `md` uses (see
[Compose Requests](./compose-requests.md)).

| Where the document lives | Request directory (where `@` searches start) | Contexts built |
|---|---|---|
| Inside a Git repository | The repository root, even when your editor's workspace folder is above it or is a subfolder of it | One for the whole repository |
| Outside any repository | The document's own folder | One per folder |

Every document then derives its own current directory from the shared
context, so `./setup.md` in `docs/guide.md` still means `docs/setup.md`.

```text
repo/README.md          ┐
repo/docs/guide.md      ├─ one context, request directory repo/
repo/pkg/notes/todo.md  ┘
~/scratch/idea.md       ── its own context, request directory ~/scratch/
```

Outside a repository, `&` and `^` have nothing to anchor to and fail with
the `MissingContext` class; `./`, bare, `@`, `~`, and `{{VAR}}` references
still work.

## `HOME` and the environment are the editor's

DMLS reads `HOME` and the environment once, when the server starts, from the
process the editor launched it in. They stay fixed for the server's lifetime.
**Restart the server to pick up a change.**

This affects `{{VAR}}` references, `~`, and the `HOME` tier of `@`. A GUI
editor on macOS launched from the Dock or Finder does not run your shell
profile, so a variable you export in `~/.zshrc` may be missing:

```text
::file {{NOTES}}/inbox.md
  terminal: md compose      → resolves ($NOTES is set by ~/.zshrc)
  editor launched from Dock → broken (DMLS never saw $NOTES)
```

Launch the editor from a terminal, or set the variable where the editor's
launcher reads it.

## When the context is rebuilt

A context is cached until something that can change its answer happens.
Then it is dropped and rebuilt the next time a document needs it.

| You do this | DMLS drops |
|---|---|
| Create, change, or delete a file inside a repository, including a package manifest (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`) or `.git/config` | The contexts for that repository |
| Change DMLS's configuration | Every context |

Adding a manifest is enough to make a new package visible to `^`:

```text
repo/pkg/doc.md:   ::file ^pkg-only.md
repo/pkg/pkg-only.md exists, but pkg/ is not a package yet → broken
add repo/pkg/Cargo.toml as a member of the repository's workspace → resolves
```

Editors without a reliable file watcher (Neovim on Linux, for example) get
the same result when you save: DMLS rescans the workspace and treats each
changed document or package manifest it finds as a watched change. The
rescan does not read `.git/config`.

## When the context cannot be built

If the builder fails, for example because `.git/config` is corrupt and
repository discovery errors, the document gets **one error diagnostic at its
top** (`dm.context.build_failure`) naming the failure and the directory.
Nothing in the document is resolved: no document links, no go-to-definition,
no link or transclusion diagnostics, and no schema validation. DMLS never
guesses a path instead.

```text
error dm.context.build_failure (darkmatter.context)
file references are not resolved in this document (MissingContext):
repository discovery failed at `/work/repo/docs`: …
```

Repair the file. The watched change to `.git/config` drops the cached
failure, and the diagnostic clears. Without a file watcher, change a DMLS
setting or restart the server.

## Untitled buffers

A new, unsaved buffer has no folder. It borrows a repository's context when
your editor's workspace folders all lie in exactly **one** repository, and
it resolves as if it sat at that repository's root. With folders in two
repositories, or in none, the buffer gets the `dm.context.build_failure`
diagnostic and resolves nothing.

## Links resolve as composition does

A Markdown link resolves to the same file `md compose` would read, whether
or not that file is inside the folder you opened. Opening only `repo/docs/`
does not break `[t](../target.md)`, `[t](&target.md)`, or `[t](~/notes.md)`:
the link, go-to-definition, and hover all reach the existing file. A link to
a document you have open but not yet saved also resolves.

DMLS checks a link's `#fragment` only in documents it has indexed. A
fragment on a file outside the opened folder (or on a non-Markdown file) is
not checked, so it is never reported as a missing anchor. Transclusion targets
and frontmatter file values are checked on disk, the way composition reads
them.

## Learn more

- [How DMLS resolves file references](../../dmls/docs/file-references.md):
  the editor-facing contract, including failure classes on diagnostics
- [LSP architecture](../lsp/architecture.md#file-resolution-contexts): the
  context cache, its keys, and invalidation
- [DMLS diagnostics](../../dmls/docs/diagnostics.md#file-resolution-context-source-darkmattercontext):
  `dm.context.build_failure` and the `resolution_failure` payload
- [Compose Requests](./compose-requests.md): the library side, the
  `RequestSnapshot` and `build_resolution_context`
