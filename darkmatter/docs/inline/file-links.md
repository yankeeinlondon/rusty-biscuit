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

The glob is resolved relative to the containing document. Only files matching
the glob **and** passing the extension filter are included.

```md
::file-links "docs/**/*.md"
::file-links "*.pdf"
::file-links "reports/*.xlsx"
```

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

Both glob and directory paths are resolved relative to the directory containing
the source document. If the document has no source file context (e.g. composed
from stdin), the directive errors with a missing-source-context message.

## Self-Exclusion

The containing document itself is always excluded from the results, even when
the glob or directory would otherwise match it.

## Repository / CWD Boundary

Discovery is bounded for security:

- When the source file is inside a git repository, the boundary is the
  repository root.
- Otherwise, the boundary is the current working directory.

Any candidate file (after following symlinks) that resolves outside this
boundary is ignored. This prevents `..` escapes and symlink-based traversal
attacks.

## Root Rendering

The rendered tree uses the common ancestor of all matched files as its root.
The root line shows:

- A dimmed prefix with the path from the boundary to the target directory
  (e.g. `/docs/`)
- A highlighted target directory name (e.g. `topics`)
- A repository icon when the root is the repository root, or a folder icon
  otherwise

Every file in the tree is wrapped in an OSC8 hyperlink when rendered to a TTY.

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

This renders a tree of all `.md` files under `docs/`, with links.

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
| `InvalidGlob` | The glob pattern failed to compile |

All errors render as line-aware `StatusBlock` diagnostics with hints showing
valid syntax.

---

[< back to **Pipeline Documentation**](../darkmatter-compose-pipeline.md)
