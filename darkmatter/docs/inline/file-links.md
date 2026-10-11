# File Links Directive

The `::file-links` directive discovers a bounded set of document files and
replaces the directive with a linked `FileSystem` tree. It runs during the
**Transclusion** phase of the compose pipeline alongside `::file`, `::code`,
and `::toc-linking`.

## Syntax

Two source forms are accepted:

### Glob Form

```md
::file-links <glob>
```

The keyword `::file-links` must be followed by ASCII whitespace or the end of
the line; near-miss prose such as `::file-linksXYZ`, `::file-links-extra`, or
`::file-links2` is left untouched and is **not** parsed as a directive.

The glob is a **glob reference**: an optional `!`, an optional file-reference
prefix, then the glob. The prefix picks the folders searched, exactly as it
does for a single `::file` reference, and every folder's matches are listed.
Only files matching the glob **and** passing the extension filter are
included.

```md
::file-links "docs/**/*.md"
::file-links "*.pdf"
::file-links ^docs/*.md
::file-links &fixes/**/spec.md
::file-links ~/notes/*.md
```

| Glob | Folders searched |
|------|------------------|
| `*.md` (bare) | The document's folder, then the repository root |
| `./docs/*.md`, `../x/*.md` | The document's folder only |
| `&fixes/**/spec.md` | The repository root |
| `^docs/*.md` | The package, the package area, then the repository root |
| `@prompts/*.md` | The `@` magic folders |
| `~/notes/*.md` | The home directory |
| `/abs/dir/*.md` | That folder |

`*` and `?` stay within one path segment and `**` crosses segments, so a bare
`*.md` in `area/pkg/guide/index.md` lists `area/pkg/guide/*.md` and the
repository root's `*.md`, but not `*.md` at other depths. A file reached from
two folders is listed once. The full pattern grammar is in biscuit-file's
[file references](../../../biscuit-file/docs/topics/file-references.md)
("Glob References").

A glob lists exactly the files the
[`find_files()`](../topics/darkmatter-expressions.md#finding-files) expression
function returns for the same pattern, less the files the extension filter and
self-exclusion drop. `find_files()` returns them in native order (most local
folder first); the tree shows them in directory order instead — directories
before files, then case-insensitively by name — because a tree has only one
place for each file. No other filter applies: hidden, gitignored, and
`_`-prefixed files are listed (dotfiles in italics, gitignored entries
dimmed), so a `&**/*.md` glob also reaches build output such as `target/`.

### Directory Form

```md
::file-links --dir <path> [--depth <u32>]
```

Scans the given directory for document files. The default depth is `0` (only
immediate files); pass `--depth N` to recurse `N` levels into subdirectories.

```md
::file-links --dir docs
::file-links --dir docs/topics --depth 2
::file-links --dir "my documents" --depth 1
```

## Supported Extensions

Only the following extensions are included, compared case-insensitively:

| Extension | Description |
|-----------|-------------|
| `.md` | Markdown documents |
| `.txt` | Plain text files |
| `.doc`, `.docx` | Word documents |
| `.xls`, `.xlsx` | Excel spreadsheets |
| `.pdf` | PDF documents |

Files with other extensions (images, binaries, source code, etc.) are silently
excluded.

## Source-Relative Resolution

A bare or `./` glob starts at the directory containing the source document (a
bare glob then also searches the repository root), and a `--dir` path is
resolved relative to that directory. If the document has no source file
context (e.g. composed from stdin), the directive errors with a
missing-source-context message.

## Self-Exclusion

The containing document itself is always excluded from the results, even when
the glob or directory would otherwise match it.

## File-Tree Boundary

Discovery is bounded by the document's **file tree**: the repository root when
the document is in a repository, else the tree its compose request chose. The
process's current directory never decides it.

- A bare, `./`, or `../` glob may not climb out of the tree. `::file-links ../*`
  in a document at the repository root fails with a `RelativeTreeEscape`
  error (in permissive mode, the directive is removed with a warning).
- `~`, `@`, absolute, and vault globs name their own folders and may lie
  outside the tree, as a `::file` reference may. `&` and `^` need a repository.
- Symlinked directories are never descended.
- A file symlink a bare, `./`, or `../` glob matches whose target lies outside
  the tree is left out, and the compose report gets one
  `dm.glob.skipped_symlink` warning naming the link and its target.
- `--dir` scans drop any file whose target resolves outside the tree.

An **in-bound** symlink — one whose target also resolves within the tree —
is kept under the **path it was matched at**, not its canonical target. For
example a matched `docs/alias.pdf -> ../assets/report.pdf` renders (and links)
as `docs/alias.pdf`. The canonical target is used only for the boundary check
and for deduplication.

## Root Rendering

The rendered tree uses the common ancestor of all matched files as its root.
The root line shows:

- A dimmed prefix with the path from the tree root to the target directory
  (e.g. `/docs/`)
- A highlighted target directory name (e.g. `topics`)
- A repository icon when the root is the repository root, or a folder icon
  otherwise

Every file in the tree is wrapped in an OSC8 hyperlink when rendered to a TTY.

### Lossless Rendering Through Compose

Compose produces a Markdown document, but the directive's styling (dimmed
prefix, highlighted target, repository/folder icons, italic dotfiles, dimmed
gitignored entries, and OSC8 links) cannot be expressed in portable CommonMark.
To avoid losing any of it, the directive embeds the fully-styled `FileSystem`
render subtree into the composed document via
[`renderable::tree::embed`](../../../renderable/docs/tree-rendering.md): the
subtree is projected once at compose time (no second filesystem walk) and the
render-tree fold splices it back when the composed document is rendered, so
terminal and browser output reproduce the live component exactly — color and
all. Consumers that render the composed Markdown without darkmatter's fold see
the embedded **portable fallback**: a plain nested link list between the
embedding markers.

## Empty Results

When no files match, the behavior depends on the compose strictness:

- **Strict mode** (`fail_fast = true`): the directive is replaced with a subtle
  `No matching files` notice.
- **Permissive mode** (`fail_fast = false`): the directive is removed and a
  compose warning is recorded.

## Examples

### Basic glob

```md
## Related Documents

::file-links "docs/**/*.md"
```

This renders a tree of all `.md` files under the `docs/` folder next to the
document and under the repository root's `docs/`, with links.

### Directory scan with depth

```md
## Reports

::file-links --dir reports --depth 1
```

This lists all document files in `reports/` and its immediate subdirectories.

### Mixed-case extensions

```md
::file-links "archive/*"
```

Matches `archive/notes.md`, `archive/budget.XLSX`, `archive/spec.PDF`, etc.

### Inside a list item

```md
- Related files:
  ::file-links "*.md"
```

The tree is indented to preserve list placement.

## Errors

| Error | Cause |
|-------|-------|
| `ParseDirective` | Invalid syntax, missing target, or unknown option |
| `MissingSourceContext` | The directive requires a source file but none was provided |
| `TargetNotFound` | The `--dir` path does not exist |
| `TargetNotDirectory` | The `--dir` path is a file |
| `GlobReference` | The glob is not a valid glob reference, a relative glob leaves the file tree (`RelativeTreeEscape`), `&`/`^` is used outside a repository, or the search must enter a directory it cannot read (`Io`, naming the directory) |
| `Unreadable` | A `--dir` scan reached a directory (within `--depth`) or an entry it cannot read; names the path |

An unreadable directory is never rendered as a shorter tree or as "no
matching files". In strict mode composition fails; in permissive mode the
directive does not render and the compose report gets a warning naming the
directory, with `resolution_failure` `Io` for a glob. A directory the search never needs to
enter is not an error: `::file-links "docs/*.md"` and `--dir docs --depth 0`
ignore an unreadable `docs/locked/`. A dangling symlink is skipped.

All errors render as line-aware `StatusBlock` diagnostics with hints showing
valid syntax.

---

[< back to **Pipeline Documentation**](../darkmatter-compose-pipeline.md)
