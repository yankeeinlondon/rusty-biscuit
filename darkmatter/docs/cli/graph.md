# `md graph` — Dependency Graph Visualization

Visualizes a Markdown file's dependency graph as a terminal tree, showing
references (links, images, imports) and transclusion directives (`::file`,
`::toc-linking`, `::file-links`, prologue/epilogue) with their insertion context.

## Usage

```text
md graph <FILE> [--follow] [--validate] [--output <TARGET>] [--width <WIDTH>]
```

| Flag         | Description                                              |
| ------------ | -------------------------------------------------------- |
| `--follow`   | Recursively expand followable transclusions (composed view). Alias: `--compose` |
| `--validate` | Validate references, show inline status, and exit with status 1 on errors |
| `--output`   | What to print: `terminal` (default), `plain`, `markdown`, `markdown-plus`, `html` (alias `browser`), `json`, `json5`, `yaml`, or `toml`. See [Output and Width](./output-and-width.md) |
| `--width`    | Width lines are cut off at. See [Line Truncation](#line-truncation) |

## Output

`md graph` accepts both output families, and everything it prints is its deliverable,
so it goes to stdout.

| `--output` | What you get |
|---|---|
| `terminal` (default), `plain` | the tree described below; `terminal` falls back to the `plain` bytes on a pipe |
| `markdown`, `markdown-plus`, `html` | the tree as a code block, then with `--validate` the report in that form |
| `json`, `json5`, `yaml`, `toml` | [the graph as data](#data-output) |

## Mental Model

- **Without `--follow`**: Reports all references found directly in the Markdown
  file, including transclusion directives shown as edges and frontmatter
  prologue/epilogue entries.
- **With `--follow`**: Reports references as if the document were **composed** —
  transclusions are expanded and child document references are shown inline
  under each edge. `::toc-linking` synthesized hyperlinks and links extracted
  from literal frontmatter content appear in the root's reference groups.

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
    ├─▶ 󰍔  ./other.md TOC elements linked into the '## Section' section
    │
    ╰◀─  epilogue  includes static text
```

### Zone 1: Reference Groups

Non-transclusion references extracted from the document, grouped by kind:

| Group                | Nerd Font    | Unicode | Description            |
| -------------------- | ------------ | ------- | ---------------------- |
| Remote Hyperlinks    | `\u{eb15}`   | 🔗      | URLs (`https://...`)   |
| Local Hyperlinks     | `\u{f0354}`  | 📄      | Local `.md` links      |
| Images               | `\u{f03e}`   | 📸      | Image references       |
| Media                | `\u{f03e}`   | 📸      | `<video>`, `<audio>`, `<source>` |
| Iframes              | `\u{eb15}`   | 🔗      | `<iframe src="...">`   |
| CSS Imports          | `\u{e74a}`   | 📄      | `<link>` / `@import`   |
| Script Imports       | `\u{ed0d}`   | 📄      | `<script src="...">`   |
| Font Imports         | `\u{f031}`   | 🔤      | `@font-face` sources   |

Groups are separated by a `│` blank line. Duplicate URLs within a group are
deduplicated (only shown once). All nerd font icons use an extra trailing space
for full-width rendering.

### Zone 2: File Head

The root file label with a markdown icon (`\u{f0354}` nerd font / 📄 unicode).
If the file has inline CSS, scripts, or meta tags, a parenthetical count summary
is appended in dim text.

### Zone 3: Transclusion Edges

Each transclusion directive produces an edge with a directional indicator:

| Direction | Connector | Description                                 |
| --------- | --------- | ------------------------------------------- |
| Incoming  | `│◀─`     | `::file`, `::code`, `::file-links`, prologue, epilogue |
| Outgoing  | `├─▶`     | `::toc-linking`                             |

**Captions** describe the insertion context:

| Directive         | Caption format                                            |
| ----------------- | --------------------------------------------------------- |
| `::file`          | `inserted into the '{## Section}' section`                |
| `::toc-linking`   | `TOC elements linked into the '{## Section}' section'     |
| `::code`          | `inserted code into the '{## Section}' section`           |
| `::url`           | `transcluded from URL into the '{## Section}' section`    |
| `::file-links`    | `file links rendered into the '{## Section}' section`     |

### Frontmatter Prologue/Epilogue Edges

Frontmatter `prologue` and `epilogue` values appear as edges with special
formatting. The display depends on whether the value is a file path or literal
content:

| Value type       | TTY rendering                                              | Non-TTY rendering                     |
| ---------------- | ---------------------------------------------------------- | ------------------------------------- |
| File reference   | ` epilogue ` (inverse) + *references* + **./file.md** (blue) | `[epilogue] references ./file.md`     |
| Literal content  | ` epilogue ` (inverse) + *includes static text*             | `[epilogue] includes static text`     |

Literal content values (containing newlines or `---` delimiters) are not
followable — they cannot be recursively expanded. However, any markdown links
within literal content (e.g., `[text](./file.md)`) are extracted and appear as
synthesized references in the root's Zone 1 when `--follow` is used.

## Conditional Transclusions

Directives with `when=` conditions are evaluated against the current
environment (frontmatter + external state + `env.*` variables). Only directives
whose condition evaluates to `true` appear in the graph. For example:

```markdown
::file ./disclosure-cc.md when="env.AGENT == 'claude'"
::file ./disclosure-oc.md when="env.AGENT == 'opencode'"
::file ./disclosure.md when="!env.AGENT"
```

With `AGENT=claude`, only `disclosure-cc.md` appears.

## Line Art Rules

### Connector Characters

| Character | Unicode  | Usage                                        |
| --------- | -------- | -------------------------------------------- |
| `╭`       | `U+256D` | First reference row (nothing above)          |
| `├`       | `U+251C` | Middle rows / outgoing edges (continues)     |
| `╰`       | `U+2570` | Last transclusion edge (terminates branch)   |
| `│`       | `U+2502` | Vertical continuation / incoming edge prefix |
| `─`       | `U+2500` | Horizontal line segment                      |
| `◀`       | `U+25C0` | Incoming transclusion arrow                  |
| `▶`       | `U+25B6` | Outgoing transclusion arrow                  |

### Connector Positions

| Position                    | Reference rows | Incoming edges | Outgoing edges |
| --------------------------- | -------------- | -------------- | -------------- |
| First (nothing above)       | `╭──`          | `│◀─`          | `├─▶`          |
| Middle (continues)          | `├──`          | `│◀─`          | `├─▶`          |
| Last (terminates branch)    | `├──`          | `╰◀─`          | `╰─▶`          |
| Only item (first AND last)  | `╭──`          | `╰◀─`          | `╰─▶`          |

Note: Reference rows never use `╰` because the vertical always continues down
to the file head below. Incoming edges use `│` (no rightward nub) instead of
`├` so the arrow sits flush against the vertical.

### Spacing Rules

#### Between File Head and First Edge

A single `│` blank line separates the file head from the first transclusion edge:

```
󰍔 test.md
    │                    ← always present
    │◀─ 󰍔  ./child.md
```

#### Between Same-Kind Transclusion Edges

No blank `│` separator — same-kind edges are adjacent:

```
    │◀─ 󰍔  ./a.md inserted into the '# Section' section
    │◀─ 󰍔  ./b.md inserted into the '# Section' section
    │◀─ 󰍔  ./c.md inserted into the '## Other' section
```

#### Between Different-Kind Transclusion Edges

A `│` blank line is inserted when the kind changes:

```
    │◀─ 󰍔  ./c.md inserted into the '## Other' section
    │                    ← kind change: ::file → ::toc-linking
    ├─▶ 󰍔  ./links.md TOC elements linked into the '## Links' section
    │                    ← kind change: ::toc-linking → epilogue
    ╰◀─  epilogue  includes static text
```

#### Between Reference Groups

A `│` blank line separates different reference group kinds:

```
    ╭── 🔗  https://example.com     ← remote hyperlinks
    │                                ← group separator
    ├── 󰍔  ./local.md               ← local hyperlinks
    │                                ← separator before file head
```

#### Between Last Reference Group and File Head

A `│` always separates the last reference group row from the file head.

#### Between Followed Child Content and Next Sibling Edge

When a followed edge has child reference groups, a `│` separates the last
child reference row from the next sibling transclusion edge:

```
    │◀─ 󰍔  ./preparation.md inserted into the '# Section' section
    │       ╭── 🔗  https://example.com
    │       ├── 󰍔  @docs/related.md
    │       │                        ← trailing separator from child refs
    │◀─ 󰍔  ./next.md inserted into the '# Section' section
```

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

4. **Literal content links appear in Zone 1** — markdown links within literal
   frontmatter content (e.g., `[animals](./animals.md)` in an epilogue string)
   are extracted and appear in the root's reference groups when following.

5. **Recursive expansion** — followed children may themselves have transclusions
   that get expanded, with the same unified rendering applied at each depth.

## Validation Mode (`--validate`)

When `--validate` is enabled:

- Invalid references are colored by severity: red (error), yellow (warning),
  cyan (info).
- A suffix is appended to invalid references: `[missing]`, `[invalid url]`,
  `[unreachable]`, `[unsupported]`, `[missing fragment]`, or `[issue]`.
- After the tree, error-severity issues are printed as the
  [reference report](./validate.md#the-reference-report), the same report
  `md validate refs` and `md compose` print:

  ```text
      ╭── 📄 ./missing.md [missing]
      │
  📄 broken.md

  Broken references in broken.md

  broken.md
    file not found in hyperlink  `./missing.md`  broken.md:3:5

  Checked 1 reference across 1 document: 0 ok, 1 broken
  ```

- Exit status 1 is returned if any validation errors exist.

## Data Output

A data output prints the graph to stdout as one value. Validation status is in
`validation.valid`, and the exit status agrees: 0 without `--validate` or when the
document is valid, and 1 when `--validate` finds errors. A failing run also has
`outcome: "failure"` and one entry in `diagnostics` per problem, in the
[diagnostic format](./data-contracts.md#diagnostics). A file that cannot be read has
no graph to print, so it is
`{ "outcome": "failure", "diagnostics": [...], "value": null }`.

### Structure

```json
{
  "file": "test.md",
  "source": "/absolute/path/to/test.md",
  "references": [
    {
      "id": "b44f128d...",
      "kind": "hyperlink",
      "target": { "type": "remote_url", "raw": "https://example.com" },
      "syntax": "markdown_link",
      "line": 12,
      "attributes": { "display": "Example" }
    }
  ],
  "transclusions": [
    {
      "kind": "file",
      "target": "/absolute/path/to/child.md",
      "line": 24,
      "followable": true,
      "section": "# Section Title",
      "section_level": 1,
      "node": { ... }
    }
  ],
  "validation": {
    "valid": false,
    "references_scanned": 19,
    "references_valid": 17,
    "issues": [ ... ],
    "warnings": [ ... ]
  }
}
```

### Key behavior

- **Without `--follow`**: Only the root file's references and transclusions
  are reported. Transclusion entries have no `node` property.
- **With `--follow`** (or `--compose`): Each followable transclusion includes
  a nested `node` object with the child document's own references and
  transclusions, recursively expanded.
- **Without `--validate`**: The `validation` key is omitted entirely.
- **With `--validate`**: The `validation` key is always present. Check
  `validation.valid` to determine whether the document has errors.
- `attributes` is only present on references that carry extra metadata
  (display text, CSS classes, image dimensions, etc.).

### Reference kinds

`hyperlink`, `image`, `transclusion`, `css_import`, `inline_css`,
`script_import`, `inline_script`, `font_import`, `meta_tag`

### Target types

`local_path`, `remote_url`, `fragment`, `data_uri`, `other_scheme`, `inline`

### Transclusion kinds

`file`, `code`, `url`, `toc_linking`, `prologue`, `epilogue`

## TTY Formatting

On a color terminal, `--output terminal` uses these styles:

| Element                | Style                                                    |
| ---------------------- | -------------------------------------------------------- |
| File head label        | Bold                                                     |
| File head summary      | Dim                                                      |
| Transclusion filename  | Blue (`\x1b[38;5;75m`)                                   |
| Transclusion caption   | Dim + italic, section name in normal weight               |
| Prologue/epilogue label| Inverse (white-on-black)                                 |
| Prologue/epilogue desc | Dim + italic (literal) or dim + italic + blue (file ref) |
| Validation errors      | Red (error), yellow (warning), cyan (info)               |

Anywhere else, including on a pipe, the output is `plain`: no ANSI escape codes.

## Line Truncation

A tree line longer than the resolved [width](./output-and-width.md#choosing-a-width)
is cut off with an ellipsis (`…`) rather than wrapped, so the line art stays aligned.
Truncation is ANSI-aware (escape sequences don't count toward the column
budget) and Unicode-aware (multi-byte characters measured by display width).

On a pipe with no `--width` and no `OUTPUT_COLUMNS`, nothing is cut off.

```text
$ md graph broken.md --width 20
    ╭── 📄 ./missin…
    ├── 📄 #setup
    │
📄 broken.md
    │
    ╰◀─ 📄 ./child.…
```

## Removed spellings

| Removed | Use instead |
|---|---|
| `--json` | `--output json` |
