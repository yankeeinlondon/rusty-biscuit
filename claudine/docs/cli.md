# Claudine as a CLI

**IMPORTANT:** At the heart of the Claudine program is a hard to miss CLI interaction model and as a result we use the `clap` crate and ALL tasks done on this repo should leverage the `clap` crate!

## Subcommands

1. `about`
      - provides a rich terminal output describing how to use this program
      - think of this as the help system on steroids
      - it will rely on the `darkmatter` library's (in this monorepo) ability to convert markdown documents to nicely formatted documents for the terminal
2. `completions`
      - provides shell completions for those of you like having your hand held (who doesn't)
3. `handle <evt> <params>` (this is the default event which is trigger if no subcommand is provided)
      - handles events across providers based on your `~/.hooker` configuration
4. `dry-run <evt> <params>`
5. `init`
      - brings the user through an interactive process to help them define an initial configuration
      - once the user configuration at `~/.hooker` has been established any subsequent calls will be used to setup a repo-scoped configuration
6. `link`
      - links the repo based skills across all agentic providers

## Best Practices

- provide both static and dynamic shell completions
    - the dynamic variant requires the "unstable-ext" feature for `clap` (you should also always use the `derive` feature)
- create and ensure that writes to STDOUT and STDIN use a standardized `log` utility:
    - `log.message()` - writes to STDERR and does not require verbose flag
    - `log.info()` - writes to STDERR when user has verbose flag
    - `log.data()` - writes output data to STDOUT
    - `log.warn()` - writes a warning message out to STDERR in a standardized way
    - `log.error()` - writes an error message out to STDERR in a standardized way
- provide `--verbose`,`-v` flags for MORE input and have this fully separate from DEBUG messaging
- allow DEBUG messaging to be exposed by user setting the DEBUG environment variable (e.g., `DEBUG=INFO claudine about`)
- separate pure CLI handling logic from business logic into different modules
- if any source file has more than 500 lines then strongly considering refactoring it to be across multiple files
- any changes to the CLI interface must be immediately updated in the README.md file for the CLI!


