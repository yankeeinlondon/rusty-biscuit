---
kind: canonical-documentation
---

# `md code-block`

`md code-block` renders one code block from a file or from literal text, without
writing a Markdown document around it. Use it to show a source file in a terminal
with highlighting, or to produce a fenced block for a document.

```sh
md code-block src/main.rs                       # highlight a file
md code-block 'let x = 1;' --language rust      # literal code
md code-block src/main.rs --line-numbering --highlight 1,4-6 --title "Entry point"
md code-block src/main.rs --output markdown >> notes.md
```

## Arguments and options

| Argument or option | What it does |
|---|---|
| `<INPUT>` | a file reference or literal code; see [How the input is read](#how-the-input-is-read) |
| `--file` | require `<INPUT>` to be a file; a miss fails with status 1 |
| `--content` | treat `<INPUT>` as literal code even if a file has that name |
| `--language <LANG>` | the language, such as `rust` or `yaml`; aliases such as `yml`, `py`, and `ts` work, and it overrides the language a file's extension implies |
| `--theme <THEME>` | a code theme override |
| `--title <TITLE>` | a title in the block's header row |
| `--line-numbering` | show line numbers |
| `--highlight <RANGE>` | highlight lines, such as `1,4-6`; a value that does not parse is a usage error (exit status 2) |
| `--code-block <MODE>` | the theme variant: `inverse` (default), `dark`, `light`, or `same` |
| `--output <TARGET>` | what to print; see [Output](#output) |
| `--width <WIDTH>` | the width of a styled panel; see [Width](#width) |

`--file` and `--content` cannot be combined.

### How the input is read

With neither `--file` nor `--content`, the input is read as a file only when it is
one line of [file reference](./index.md#file-references) syntax naming an existing
file. Anything else, including a reference that matches nothing, is rendered as
literal code and no file is read. A `-` is the literal text `-`; this command does
not read stdin.

## Output

`md code-block` accepts both output families. The block goes to stdout.

| `--output` | What you get |
|---|---|
| `terminal` (default) | a highlighted panel on a color terminal, with the title, line numbers, and highlights; otherwise the `plain` bytes |
| `plain` | a fence with its language tag and the code; the title, line numbers, and highlights have no plain form |
| `markdown`, `markdown-plus` | a fence whose info string carries the title, line numbering, and highlights |
| `html` | an HTML code block |
| `json`, `json5`, `yaml`, `toml` | one value with the code and its settings |

The block does not paint on a pipe, so a redirect never captures escape codes.

````text
$ md code-block hello.rs --output plain
```rust
fn main() {
    println!("hi");
}
```

$ md code-block hello.rs --output markdown --title Hi --line-numbering --highlight 2
```rust title="Hi" line-numbering=true highlight=2
fn main() {
    println!("hi");
}

```
````

A data output has these fields on every run, as defined in
[`code-block.yaml`](../../cli/schemas/output/code-block.yaml):

```json
{
  "file": "/work/hello.rs",
  "code": "fn main() {\n    println!(\"hi\");\n}\n",
  "language": "rust",
  "title": null,
  "line_numbering": false,
  "highlight": [{ "start": 2, "end": 2 }],
  "outcome": "success",
  "diagnostics": []
}
```

`file` is `null` for literal content. On a failure, `code` and `language` are `null`
and `diagnostics` says why. See [Data Contracts](./data-contracts.md#other-commands).

## Width

Code is never wrapped. In a styled terminal the width sets the panel's size and cuts
off a longer code line. In `plain`, Markdown, and data outputs a code line is always
printed whole. See [Choosing a width](./output-and-width.md#choosing-a-width).

## Exit status

| Status | When |
|---|---|
| 0 | the block was printed |
| 1 | `--file` named a file that does not resolve, or a file could not be read |
| 2 | a usage error, such as a bad `--highlight` or `--file` with `--content` |

## Removed spellings

These are usage errors (exit status 2).

| Removed | Use instead |
|---|---|
| `--json` | `--output json` |
