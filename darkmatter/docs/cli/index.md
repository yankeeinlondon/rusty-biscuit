# Darkmatter CLI

The Darkmatter CLI works off of a "subcommand" structure but if no "subcommand" is specified it will default to _rendering_ with the `render` subcommand. Here's the layout of the help system you'll get when you run `md --help` (or just `md`):

```txt
Command-line interface for the darkmatter markdown renderer.

Use `md --help` to see all available options.

Usage: md [OPTIONS] [INPUT] [COMMAND]

Commands:
  render   Render a markdown document (same as default behavior without a subcommand)
  clean    Clean up markdown formatting
  compose  Compose a document through the compose pipeline
  toc      Show markdown table of contents
  delta    Compare two markdown documents
  get      Get frontmatter properties from a markdown document
  set      Set a frontmatter property on a markdown document
  rm       Remove one or more frontmatter properties from a markdown document
  edit     Open a markdown file in your preferred editor
  validate Validate references in a markdown document
  hash     Hash a markdown document's frontmatter and body
  graph    Visualize a markdown file's dependency graph

Arguments:
  [INPUT]  Input file path (reads from stdin if not provided, use "-" for explicit stdin)
```

## Subcommands

> Use the links to move into a more detailed description of each of the commands

- [`render <file-reference>`](./render.md) - renders a Markdown file into one of the supported formats; if no `--output <output>` is specified then the format defaults to being the Terminal (aka, using escape codes to make the Markdown look nice)
- [`clean <file-reference>`](./clean.md) - cleans up the Markdown file by ensuring good headings structure, consistent indent levels, use of proper vertical spacing. Sends fixed file to STDOUT unless the `--save` switch is used
- [`compose <file-reference>`](./compose.md) - runs the referenced Markdown file through the [composition pipeline](../darkmatter-compose-pipeline.md) and returns the composed result to STDOUT as plain text Markdown content
- [`toc <file-ref>`](./toc.md) - shows the table of contents for the file referenced
- [`delta <file-ref> <file-ref>`](./delta.md) - describes the changes found between the two Markdown files. Uses a structured analysis to describe semantically what's changed. Adds visual diffing if you use the `--verbose` flag.
- [`get <file-ref> <prop>`](./get.md) - extracts a property from the Markdown file referenced
- [`set <file-ref> <prop> <value>`](./set.md) - sets a frontmatter property on the referenced Markdown file; also supports the syntax `set <file-ref> <json5>` to set a dictionary of values.
- [`rm <file-ref> <prop>`](./rm.md) - removes a Frontmatter property on the referenced Markdown file
- [`edit <file-ref>`](./edit.md) - opens the referenced Markdown file and blocks until the user closes that file in their editor.
- [`validate <file-ref>`](./validate.md) - validates references (links, images, transclusions) in the document.
- [`graph <file-ref>`](./graph.md) - visualizes the dependency graph of the document.

## File References

All file references in the CLI use the `FileReference` struct from the `biscuit-file` library (in this monorepo) which allows for:

- relative paths
- absolute paths
- magic paths (e.g., starts with `@`) which match on repo relative, package relative, and user home relative paths.
- more details can be found in the [Biscuit File](../../../biscuit-file/README.md) package area

All subcommands can consume STDIN as their (first) file reference:

```sh
# composes the file and then passes STDIN to `md` which to renders the Markdown for the terminal
md compose foobar.md | md
```
