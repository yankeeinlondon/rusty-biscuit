# Darkmatter CLI

Binary: `md`

`md` is a command-line tool for Markdown: render a document for reading, compose
documents together, tidy and validate them, and read or change their frontmatter.
Every command can print for a person or as data for a script, through the same
`--output` and `--width` options, with the same [exit statuses](../docs/cli/output-and-width.md#exit-status).
With no subcommand, `md` renders: `md FILE …` is `md render FILE …`.

## Installation

```sh
# From source
cargo install --path .

# Or with just, from the repository root
just -f darkmatter/justfile install
```

To enable tab completion, see [Shell Completions](../docs/cli/index.md#shell-completions).

## Examples

```sh
# Read a document in the terminal; `md FILE` is `md render FILE`
md README.md

# Style it for this run only, without editing the file
md README.md style.max-width=72 style.page.theme=github
md README.md --style ./print.yaml

# The theme names `style.page.theme` accepts
md list-themes

# Compose a document, then render the result as HTML
md compose doc.md | md render --output html > doc.html

# Format a document in place
md clean README.md --save

# Check its links, images, and transclusions
md validate refs README.md

# Read and change frontmatter
md get doc.md title --output plain
md set doc.md title "New Title" --save

# The outcome, every problem, and the document, for a script or CI job
md compose doc.md --output json
```

## Documentation

- [CLI index](../docs/cli/index.md): every command, and the file reference grammar
- [Output and Width](../docs/cli/output-and-width.md): `--output`, `--width`, streams, and exit statuses
- [Data Contracts](../docs/cli/data-contracts.md): what each command prints as data
- [Policy Flags](../docs/cli/policy.md): `--allow`, `--deny`, and `--disable`
- [Styling from the command line](../docs/cli/render.md#styling-from-the-command-line): style setters, `--style` files, and the [migration table](../docs/cli/render.md#presentation-flags) for the removed layout and theme flags
- [Developing the CLI](../docs/cli/developing.md): the source layout and how tests launch `md`

| Command | What it does |
|---|---|
| [`render`](../docs/cli/render.md) | render a document for a terminal, as plain text, Markdown, HTML, or data |
| [`compose`](../docs/cli/compose.md) | evaluate expressions, include other documents, and write Markdown |
| [`clean`](../docs/cli/clean.md) | tidy a document's formatting without changing its meaning |
| [`validate refs`](../docs/cli/validate.md) | report broken references |
| [`graph`](../docs/cli/graph.md) | show a document's dependency graph |
| [`toc`](../docs/cli/toc.md), [`delta`](../docs/cli/delta.md) | show a table of contents, or compare two documents |
| [`get`](../docs/cli/get.md), [`set`](../docs/cli/set.md), [`rm`](../docs/cli/rm.md) | read and change frontmatter properties |
| [`edit`](../docs/cli/edit.md) | open a file in your editor and wait |
| [`hash`](../docs/cli/hash.md) | hash a document, store the hash, and report drift |
| [`schema`](../docs/cli/schema.md) | validate against a schema, detect one, and look up the language |
| [`code-block`](../docs/cli/code-block.md) | render one code block |
| [`list-themes`](../docs/cli/list-themes.md) | list the prose and code themes a style can name |

For programmatic access, see [the Darkmatter library](../lib/README.md).
