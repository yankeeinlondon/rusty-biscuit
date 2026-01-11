# Darkmatter Markdown CLI

> A themed Markdown CLI which renders to both the console (escape codes) and the browser (HTML/CSS)

<table>
<tr>
<td><img src="./images/darkmatter.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Features</h2>
<ul>
  <li>Target both the <b>terminal</b> and the <b>browser</b></li>
  <li>
      Auto dark/light <b>theming</b> for both prose content <i>and</i> fenced code blocks
  </li>
  <li><b>Image</b> support for both targets (<i>terminal must support Kitty graphics</i>)</li>
  <li>
      <b>Mermaid</b> diagram rendering
  </li>
  <li>
      Provides <b>cleanup</b> of your documents to make them better CommonMark + GFM citizens
  </li>
</ul>

</td>
</tr>
</table>

## Installation

We have published this CLI to _both_ the **NPM** and **Cargo** package managers.

### Cargo

```sh
# cargo
cargo install darkmatter
```

### NPM

```sh
# npm
npm install -g darkmatter
```

## Basic Usage

```text
md <file> [...OPTIONS]
```

Renders a markdown file with syntax highlighting and rich formatting. Reads from stdin if no file is provided.

### CLI Switches

#### Output

By default, the CLI will render themed Markdown to the terminal. Use these flags for alternative output:

| Flag             | Description                                             |
|------------------|---------------------------------------------------------|
| `--html`         | Output raw HTML to stdout                               |
| `--show-html`    | Generate HTML, save to temp file, and open in browser   |
| `--ast`          | Output the markdown AST as JSON (MDAST format)          |
| `--toc`          | Display table of contents as a tree structure           |
| `--delta <FILE>` | Compare with another markdown file and show differences |
| `--clean`        | Normalize markdown formatting (output to stdout)        |
| `--clean-save`   | Normalize and save back to the source file              |

> **Note:** you can use the `--json` flag with `--toc` and `--delta` switches to produce a JSON output instead of an output intended for users in the terminal

#### Theming

| Option                | Description                                          |
|-----------------------|------------------------------------------------------|
| `--theme <NAME>`      | Theme for prose content (default: `one-half`)        |
| `--code-theme <NAME>` | Theme for code blocks (overrides auto-derived theme) |
| `--list-themes`       | List all available themes with descriptions          |

#### Rendering

| Option              | Description                                                                                                                                     |
|---------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `--line-numbers`    | Show line numbers in code blocks                                                                                                                |
| `--mermaid`         | Terminal output only. Renders mermaid diagrams as images (falls back to code blocks if unsupported). Browser _always_ renders mermaid diagrams. |

#### Frontmatter

| Option                   | Description                                                 |
|--------------------------|-------------------------------------------------------------|
| `--fm-merge-with <JSON>` | Merge JSON into frontmatter (JSON wins on conflicts)        |
| `--fm-defaults <JSON>`   | Set default frontmatter values (document wins on conflicts) |

#### Verbosity

| Option  | Description                  |
|---------|------------------------------|
| `-v`    | INFO level logging           |
| `-vv`   | DEBUG level logging          |
| `-vvv`  | TRACE level logging          |
| `-vvvv` | TRACE with file/line numbers |

## Color Themes

Theming for the **Darkmatter** CLI is done with _theme pairs_ where a **theme pair** is a theme name that actually indicates both a **light** and a **dark** theme. The intention is to have these pairs be comparably similar where possible so that people who switch back and forth have good contrast always but stylistically stay aligned to what they like.

By default you'll be assigned to the `github` color theme. This translates to:

- `github` _for light themed terminals_
- `vs-dark` _for dark themed terminals_

Because we want to make sure that any code blocks within the

Themes adapt to light/dark mode automatically. Use `md --list-themes` to see descriptions.

| Theme          | Light/Dark Support |
|----------------|--------------------|
| `one-half`     | Both (default)     |
| `github`       | Both               |
| `gruvbox`      | Both               |
| `solarized`    | Both               |
| `base16-ocean` | Both               |
| `nord`         | Dark only          |
| `dracula`      | Dark only          |
| `monokai`      | Dark only          |
| `vs-dark`      | Dark only          |

## Environment Variables

| Variable     | Description                                           |
|--------------|-------------------------------------------------------|
| `THEME`      | Default prose theme (e.g., `github`)                  |
| `CODE_THEME` | Default code theme (auto-derived from prose if unset) |
| `NO_COLOR`   | Disable colors ([no-color.org](https://no-color.org))                         |
| `COLORFGBG`  | Terminal color detection (`fg;bg` format)             |

## Examples

```bash
# Render a file to terminal
md README.md

# Pipe markdown from another command
cat notes.md | md

# Open rendered HTML in browser
md README.md --show-html

# Compare two versions of a document
md v1.md --delta v2.md

# View table of contents
md docs/guide.md --toc

# Use a specific theme with line numbers
md code.md --theme dracula --line-numbers

# Normalize markdown formatting in-place
md messy.md --clean-save
```
