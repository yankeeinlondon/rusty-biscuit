# Darkmatter CLI

The Darkmatter CLI works off of a "subcommand" structure but if no "subcommand" is specified it will default to _rendering_ with the `render` subcommand. Here's the layout of the help system you'll get when you run `md --help` (or just `md`):

```txt
Command-line interface for the darkmatter markdown renderer.

Use `md --help` to see all available options.

Usage: md [OPTIONS] [INPUT] [COMMAND]

Commands:
  render      Render a markdown document (same as default behavior without a subcommand)
  clean       Clean up markdown formatting
  compose     Compose a document through the compose pipeline
  toc         Show markdown table of contents
  delta       Compare two markdown documents
  get         Get frontmatter properties from a markdown document
  set         Set a frontmatter property on a markdown document
  rm          Remove one or more frontmatter properties from a markdown document
  edit        Open a markdown file in your preferred editor
  validate    Validate references in a markdown document
  hash        Hash a markdown document's frontmatter and body
  schema      Schema authoring and validation commands
  graph       Visualize a markdown file's dependency graph
  code-block  Render a single code block from a file or literal content

Arguments:
  [INPUT]  Input file path (use "-" for stdin)
```

Three things work the same way on every command:

- **[Output and Width](./output-and-width.md)**: one `--output` selector with two families (formatted and data), one `--width`, one rule for where things are written, and one meaning for exit statuses 0, 1, and 2.
- **[Data Contracts](./data-contracts.md)**: the value each command prints with `--output json`, `json5`, `yaml`, or `toml`, including every diagnostic.
- **[Policy Flags](./policy.md)**: commands that have a policy choice share three verbs, `--allow`, `--deny`, and `--disable`.

## Subcommands

> Use the links to move into a more detailed description of each of the commands

- [`render <file-reference>`](./render.md) - renders a Markdown file into one of the supported outputs; if no `--output <target>` is specified then it uses `terminal` (escape codes to make the Markdown look nice), which falls back to `plain` on a pipe
- [`clean <file-reference>`](./clean.md) - cleans up the Markdown file by ensuring good headings structure, consistent indent levels, use of proper vertical spacing. Sends fixed file to STDOUT unless the `--save` switch is used
- [`compose <file-reference>`](./compose.md) - runs the referenced Markdown file through the [composition pipeline](../darkmatter-compose-pipeline.md) and returns the composed Markdown to STDOUT, or the document and every problem as data; pipe it to `md render` for any other output
- [`toc <file-ref>`](./toc.md) - shows the table of contents for the file referenced
- [`delta <file-ref> <file-ref>`](./delta.md) - describes the changes found between the two Markdown files. Uses a structured analysis to describe semantically what's changed. Adds visual diffing if you use the `--verbose` flag.
- [`get <file-ref> <prop>`](./get.md) - extracts a property from the Markdown file referenced
- [`set <file-ref> <prop> <value>`](./set.md) - sets a frontmatter property on the referenced Markdown file
- [`rm <file-ref> <prop>`](./rm.md) - removes a Frontmatter property on the referenced Markdown file
- [`edit <file-ref>`](./edit.md) - opens the referenced Markdown file and blocks until the user closes that file in their editor.
- [`validate refs <file-ref>`](./validate.md) - validates references (links, images, transclusions) in the document and prints the reference report.
- [`hash <file-ref>`](./hash.md) - computes Markdown-aware hashes, stores them, and reports drift from a stored hash.
- [`graph <file-ref>`](./graph.md) - visualizes the dependency graph of the document.
- [`schema <subcommand>`](./schema.md) - validates frontmatter and YAML documents against a schema, detects a schema from documents, explains schema triggers, and prints the schema language reference.
- [`code-block <input>`](./code-block.md) - renders one code block from a file or from literal content.

## File References

All file references in the CLI use the `FileReference` struct from the `biscuit-file` library (in this monorepo). The introducer you write selects a **closed, ordered list of base directories**; resolution joins the path onto each base in turn and takes the first candidate that is an existing regular file. There is no cross-kind fallback — a `./foo.md` that misses is never retried as `@foo.md`.

| Written as            | Kind                     | Candidate order                                                                       |
|-----------------------|--------------------------|---------------------------------------------------------------------------------------|
| `./x.md`, `../x.md`   | explicit relative        | the base directory only                                                               |
| `x.md`, `docs/x.md`   | implicit relative        | the base directory **first**, then the repository root                                |
| `/x.md`, `C:\x.md`    | absolute                 | used verbatim                                                                         |
| `~/x.md`              | home                     | the user's home directory only (`~user` is rejected)                                  |
| `@x.md`               | magic (multi-homed)      | the local tree (package, package area, repository root or launch directory, plus registered roots inside it) before home and registered roots outside it; see [Magic Paths](../topics/magic-paths.md) |
| `&x.md`               | repository root, exactly | the repository root only                                                              |
| `^x.md`               | repository scoped        | package, package area, then repository root; never consults home                      |
| `vault:x.md`          | vault                    | registered vault roots, then the paths in `$VAULT`                                    |
| `http://`, `https://` | remote                   | a typed remote reference; never a local candidate                                     |

**"Base directory"** means the directory a reference was authored from: for an argument you type at the shell that is the directory you ran `md` in; for a reference written inside a document it is the directory of the document being composed.

A relative reference may not climb out of the repository that holds its base directory, whether you type it as an argument or a document contains it: inside `/work/repo`, `md compose ../../elsewhere.md` fails with `failure: invalid-reference` when it would leave `/work/repo`. An absolute path may name any file. See [Relative references stay in their tree](../topics/file-referencing.md#relative-references-stay-in-their-tree).

Two modifiers compose with every kind above:

- a leading `%` (`%@README.md`) switches from exact-path probing to a recursive search of the same roots
- any path segment may contain `{{VAR}}` environment interpolation (`@configs/{{APP}}/settings.toml`)

A few things worth knowing before typing one of these at a shell prompt:

- `&` is a shell control operator, so repository-root references have to be quoted: `md render '&docs/plan.md'`
- `&` and `^` are repository-contained — using either outside a repository, or writing a payload that escapes it (`&../outside.md`), is a typed error rather than a silent miss
- `@` is the one multi-homed introducer that can leave the repository, because `$HOME` is in its search list; reach for `^` when repository containment is the point
- the `!` sigil has been removed from the grammar; a reference beginning with `!` is now a parse error (`failure: invalid-reference`) that suggests `^`

### Every file argument uses this grammar

Every command that reads a file you name opens it through the table above, from the directory you ran `md` in: the document argument of `md <file>`, `render`, `compose`, `clean`, `toc`, `get`, `set`, `rm`, `edit`, `hash`, both inputs of `delta`, `graph`, `validate refs`, `schema validate`, `schema detect`, `schema triggers`, and `code-block` (see below). So `md schema validate '@notes.md'`, `md validate refs '&docs/plan.md'`, and `md get '^README.md' title` work like `md compose` does. Output destinations, cache directories, and `--magic-root` directories are settings, not files to read, and are plain paths.

An argument is parsed before anything is read, so text that is not valid reference syntax fails with `failure: invalid-reference` even when a file with that literal name exists:

```text
$ ls
@   doc.md
$ md get @ title --output plain   # a bare `@` is an introducer with no path
⤫ FileReferenceError: file argument not resolved
┃ failure: invalid-reference
$ md get ./@ title --output plain # `./` names the literal file
Literal
```

The same holds for a bare `&` or `^`, and for the removed `!` sigil (`md get '!old.md' title`). Spell a file whose name starts with one of these characters with `./` in front.

A few commands add a rule of their own:

- **`md code-block <input>`** renders `--content` input as typed and requires `--file` input to resolve to a file. With neither flag, the input is read as a file only when it is one line of reference syntax naming an existing file (`md code-block '&src/main.rs'`); anything else, including a reference that matches nothing, a malformed one such as `@`, and a relative path leaving the repository, is rendered as literal code and no file is read.
- **`md hash <directory>`** names its directory with the same grammar (`md hash '&docs'`); a relative directory may not leave the repository. The first place the reference looks that holds a file or a directory decides between hashing one document and a tree, so `md hash notes.md` hashes a nearer `notes.md` file before a repository-root `notes.md/` directory, and a place that cannot be checked fails with `failure: io`.
- **`md edit <file>`** creates the file when the reference matches nothing, at the first place the reference would look: beside you for `./new.md` or `new.md`, at the repository root for `&new.md`. A malformed reference or a path leaving the repository is refused before any file is created or editor opened.
- **`md schema triggers <file>`** inspects a document inside a repository and prints the file it opened on its `Document:` line, listing any trigger it [skipped as invalid](../topics/schemas/schema-triggers.md#invalid-discovered-triggers) under "Skipped invalid triggers" (the warning itself goes to stderr); a document outside every repository fails with `failure: missing-context`, and a schema root it cannot inspect (such as one below a folder without search permission) fails with an I/O error naming that root instead of listing the roots.
- **`md schema validate`** reports an argument that does not open on that file's line (`could not be opened`, with its `failure:` row) and exits `1`, as for a file whose frontmatter cannot be parsed. Whether an argument is a whole YAML document is judged on the file it opened, so `md schema validate '@service.yaml'` validates YAML (see [YAML documents on the command line](../topics/schemas/definition.md#yaml-documents-on-the-command-line)).

To search more directories for `@` references, pass `--magic-root <DIR>` before the subcommand (`md --magic-root shared compose '@notes.md'`); it is repeatable and applies to every route. See [Magic Paths](../topics/magic-paths.md#from-the-md-command-line).

The canonical reference for the full grammar — including the error vocabulary and the `resolve_detailed()` diagnostics — is [File References in `biscuit-file`](../../../biscuit-file/docs/topics/file-references.md). For how darkmatter injects its own `@` search roots, see [Magic Paths](../topics/magic-paths.md).

`render`, `compose`, `clean`, `toc`, `get`, `set`, and `hash` can consume STDIN as their (first) file reference; the other commands work on named files:

```sh
# composes the file and then passes STDIN to `md` which to renders the Markdown for the terminal
md compose foobar.md | md
```

## Shell Completions

`md` completes subcommands, flags, Markdown files (`.md`, `.dm`), and directories. Add the line for your shell to its startup file:

```bash
# Bash (~/.bashrc)
source <(COMPLETE=bash md)

# Zsh (~/.zshrc)
source <(COMPLETE=zsh md)

# Fish (~/.config/fish/config.fish)
COMPLETE=fish md | source

# PowerShell ($PROFILE)
$env:COMPLETE = "powershell"; md | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE
```

`md --completions <SHELL>` prints the setup command for your shell, for `bash`, `elvish`, `fish`, `powershell`, or `zsh`.

## Working on the CLI

If you are changing `md` itself, see [Developing the CLI](./developing.md) for where the code lives and how its tests launch the binary.
