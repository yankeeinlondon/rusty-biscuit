# `md graph` — Dependency Graph Visualization

Visualizes a Markdown file's dependency graph as a terminal tree, showing
references (links, images, imports) and transclusion directives (`::file`,
`::toc-linking`, etc.) with their insertion context.

## Usage

```
md graph <FILE> [--follow] [--validate]
```

| Flag         | Description                                              |
| ------------ | -------------------------------------------------------- |
| `--follow`   | Recursively expand followable transclusions (composed view) |
| `--validate` | Validate references and show inline status with exit code 2 on errors |

## Mental Model

- **Without `--follow`**: Reports all references found directly in the Markdown
  file, including transclusion directives shown as edges.
- **With `--follow`**: Reports references as if the document were **composed** —
  transclusions are expanded, child document references are shown inline, and
  `::toc-linking` synthesized hyperlinks appear in the root's reference groups.

## Layout Structure

Each file node renders in three zones:

```
Zone 1 — Reference groups (above the file head)
    ╭── 🔗  https://example.com           ← remote hyperlinks
    ├── 📸  ./logo.png                     ← images
    │
Zone 2 — File head
󰍔 document.md
    │
Zone 3 — Transclusion edges (below the file head)
    │◀─ 󰍔  ./child.md inserted into the '# Section' section
    │
    ╰─▶ 󰍔  ./other.md TOC elements linked into the '## Section' section
```

### Zone 1: Reference Groups

Non-transclusion references extracted from the document, grouped by kind:

| Group                | Nerd Font    | Unicode | Description            |
| -------------------- | ------------ | ------- | ---------------------- |
| Remote Hyperlinks    | `\u{eb15}`   | 🔗      | URLs (`https://...`)   |
| Local Hyperlinks     | `\u{f0354}`  | 📄      | Local `.md` links      |
| Images               | `\u{f03e}`   | 📸      | Image references       |
| CSS Imports          | `\u{e74a}`   | 📄      | `<link>` / `@import`   |
| Script Imports       | `\u{ed0d}`   | 📄      | `<script src="...">`   |
| Font Imports         | `\u{f031}`   | 🔤      | `@font-face` sources   |

Groups are separated by a `│` blank line. Duplicate URLs within a group are
deduplicated (only shown once).

### Zone 2: File Head

The root file label with a markdown icon (`\u{f0354}` nerd font / 📄 unicode).
If the file has inline CSS, scripts, or meta tags, a parenthetical count summary
is appended in dim text.

### Zone 3: Transclusion Edges

Each transclusion directive produces an edge with a directional indicator:

| Direction | Connector | Description                    |
| --------- | --------- | ------------------------------ |
| Incoming  | `│◀─`     | `::file`, prologue, epilogue   |
| Outgoing  | `├─▶`     | `::toc-linking`                |

Edge lines include the target path and a caption describing the insertion
context:

```
│◀─ 󰍔  ./child.md inserted into the '## Section' section
╰─▶ 󰍔  ./other.md TOC elements linked into the '## Section' section
```

## Conditional Transclusions

Directives with `when=` conditions are evaluated against the current
environment. Only directives whose condition evaluates to `true` appear in the
graph. For example:

```markdown
::file ./disclosure-cc.md when="env.AGENT == 'claude'"
::file ./disclosure-oc.md when="env.AGENT == 'opencode'"
::file ./disclosure.md when="!env.AGENT"
```

With `AGENT=claude`, only `disclosure-cc.md` appears.

## Literal Frontmatter Content

Frontmatter `prologue` and `epilogue` values that contain literal content
(newlines or `---` delimiters) are skipped — only file path references produce
transclusion edges.

## Line Art Rules

### Connector Characters

| Character | Unicode  | Usage                                 |
| --------- | -------- | ------------------------------------- |
| `╭`       | `U+256D` | First reference row (nothing above)   |
| `├`       | `U+251C` | Middle rows (vertical continues)      |
| `╰`       | `U+2570` | Last transclusion edge (terminates)   |
| `│`       | `U+2502` | Vertical continuation                 |
| `─`       | `U+2500` | Horizontal line segment               |
| `◀`       | `U+25C0` | Incoming transclusion arrow           |
| `▶`       | `U+25B6` | Outgoing transclusion arrow           |

### Vertical Line Termination

- The first reference row uses `╭──` (curved start, nothing above).
- Subsequent reference rows use `├──` (vertical continues to file head).
- The last transclusion edge uses `╰` (curved end, nothing below).
- Incoming transclusion edges use `│◀─` (vertical with no rightward nub).

### Spacing Between Transclusion Edges

- **Same-kind edges** (e.g., consecutive `::file` directives): No blank `│`
  separator — edges are adjacent.
- **Kind change** (e.g., `::file` → `::toc-linking`): A `│` blank line is
  inserted between them.
- **After followed children**: No extra separator — the child's indented content
  provides sufficient visual separation from the next sibling edge.

### Reference Group Spacing

- A `│` blank line separates different reference group kinds (e.g., remote
  hyperlinks from local hyperlinks).
- A `│` blank line separates the last reference group from the file head.
- No `│` separator between the last reference row of a followed child and the
  next sibling transclusion edge.

## Follow Mode (`--follow`)

When `--follow` is enabled:

1. **Transclusion edges merge with children** — the edge arrow line becomes the
   child's header. The child's own references and sub-transclusions render
   indented below it. No separate file head is rendered for the child.

2. **Child content indentation** — uses `│   ` (4-space indent with vertical
   continuation) for non-last edges, or plain spaces for the last edge (after
   `╰`, the vertical is terminated).

3. **TOC-linking references appear in Zone 1** — `::toc-linking` directives
   generate synthesized hyperlink references (e.g., `./other.md#heading-slug`)
   that appear in the root node's reference groups, reflecting the composed
   document's full reference set. These are hidden in non-follow mode.

4. **Recursive expansion** — followed children may themselves have transclusions
   that get expanded, with the same unified rendering applied at each depth.

## Validation Mode (`--validate`)

When `--validate` is enabled:

- Invalid references are colored by severity: red (error), yellow (warning),
  cyan (info).
- A suffix is appended to invalid references: `[missing]`, `[invalid url]`,
  `[unreachable]`, `[unsupported]`, `[missing fragment]`, or `[issue]`.
- A summary footer shows `N references scanned, M valid, K issues`.
- Exit code 2 is returned if any validation errors exist.

## TTY Formatting

When outputting to a terminal (TTY detected):

- **File head**: bold label, dim summary counts
- **Transclusion filename**: blue (`\x1b[38;5;75m`)
- **Transclusion caption**: dim + italic, with section heading name in normal
  weight (e.g., *inserted into the* '## Section' *section*)
- **Validation errors**: colored by severity (red/yellow/cyan)

When piped (non-TTY): plain text with no ANSI escape codes.

## Line Truncation

Lines exceeding the terminal width are truncated with an ellipsis (`…`).
Truncation is ANSI-aware (escape sequences don't count toward the column
budget) and Unicode-aware (multi-byte characters measured by display width).
