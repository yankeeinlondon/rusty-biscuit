# Tree Hugger

A library (and CLI) for generating diagnostics and symbol summaries across multiple programming languages using Tree-sitter.

## Packages

```txt
📁 tree-hugger
├── 📁 cli - uses `clap` to provide a CLI application
├── 📁 lib - library APIs for symbol and diagnostic extraction
```

## Supported Languages

- Rust
- Javascript
- Typescript
- Go
- Python
- Java
- PHP
- Perl
- Bash
- Zsh
- C
- C++
- C#
- Swift
- Scala (`tree-sitter-scala`)
- Lua (`tree-sitter-lua`)

## Using the CLI

**Syntax:** `hug <COMMAND> <...file-glob>`

You can provide one or more file-glob patterns to match files:

- Files matching **any** glob pattern are included
- Use `--ignore <glob>` to exclude matches
- Files ignored by `.gitignore` are never included

### Commands

- `functions` - list functions
- `types` - list type definitions
- `symbols` - list all discovered symbols
- `exports` - list exported symbols
- `imports` - list imported symbols

### Options

- `--language <LANG>` - override language detection
- `--ignore <GLOB>` - exclude paths
- `--format <pretty|json>` - switch output format

### Examples

```bash
hug symbols "tree-hugger/lib/src/**/*.rs"
hug functions --language rust "tree-hugger/lib/src/**/*.rs"
hug imports --format json "tree-hugger/lib/tests/fixtures/**/*.js"
```

## JSON Output

When `--format json` is selected, output is a serialized `PackageSummary`:

- `root_dir` - the directory scanned
- `language` - primary language for the run
- `files` - list of `FileSummary` objects

Each `FileSummary` includes:

- `file`, `language`, `hash`
- `symbols` (for `functions`, `types`, or `symbols`)
- `imports` (for `imports` and `symbols`)
- `exports` (for `exports` and `symbols`)
- `locals` (for `symbols`)
- `lint`, `syntax`

## Query Vendoring

Tree Hugger vendors `nvim-treesitter` query files in `tree-hugger/lib/queries/vendor/<lang>/` and uses
`locals.scm` for symbol discovery and imports. Captures follow the standard naming scheme used by
`nvim-treesitter`, including:

- `@local.definition.function`
- `@local.definition.type`
- `@local.definition.method`
- `@local.definition.import`

Additional languages may be added by vendoring the corresponding `locals.scm` file. Currently,
Perl lacks an upstream `locals.scm`, so symbol extraction for Perl is minimal until a query is added.
