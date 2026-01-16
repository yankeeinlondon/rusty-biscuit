# Tree Hugger

A Library (and CLI) for generating diagnostics on a number of programming languages using static analysis (using the `tree-sitter` library).

## Packages

```txt
📁 tree-hugger
├── 📁 cli - uses `clap` to provide a CLI application
├── 📁 lib - a library which exposes cross-language
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

**Syntax:** `hug [CMD] <...file-glob>`

> Note: you can provide one or more "file-glob" patterns to match files:
>
> - the files which match _any_ of the glob patterns will be evaluated
> - you can use the `--ignore <glob>` to provide a negative glob pattern which will eliminate matched files
> - Files ignored by .gitignore will never be included

### Commands

- `functions` - provides a list of the functions
- `types` - provides a list of types in the file / files
- `symbols` - provides a summary of the Symbols defined in the file / files
- `exports` - provides a summary of the _exported_ symbols defined in the file /files
- `imports` - provides a summary of the Symbols imported


### Outputs

By default the CLI is geared toward outputting to the terminal. This means:

- we use escape codes to provide useful
