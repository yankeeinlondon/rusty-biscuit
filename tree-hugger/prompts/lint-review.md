
## Context

You are performing a code review along with some research for 'lint' functionality in the **tree-hugger** package area; it has the following packages in it:

- `tree-hugger` - library package
- `tree-hugger-cli` - a CLI which leverages the 'clap' crate and the **tree-hugger** library to allow callers to evaluate their code bases via **tree-sitter** static analysis

### CLI

The CLI's subcommands provide a useful view on the structuring of this functionality:

```txt
Tree Hugger diagnostics and symbol tooling

Usage: hug [OPTIONS] <COMMAND>

Commands:
  functions    List functions in the file(s)
  types        List types in the file(s)
  symbols      List all symbols in the file(s)
  imports      List imported symbols in the file(s)
  classes      List classes and their members
  lint         Run lint diagnostics on the file(s)
  completions  Generate shell completions
  help         Print this message or the help of the given subcommand(s)

Options:
      --language <LANGUAGE>     Force a specific language [possible values: rust, javascript, typescript, go, python, java, php,
                                perl, bash, zsh, c, c++, c#, swift, scala, lua]
      --json                    Output as JSON
      --plain                   Disable colors and hyperlinks (plain text output)
      --comments                Show symbol-level documentation comments in output
      --group-by-file           Group symbol output by file path
      --group-by-module         Group symbol output by module path (directory/module scope)
      --sort-by-kind            Sort symbols by kind before name
      --sort-by-module          Sort symbols by module before other sort keys
      --exclude-files <GLOB>    Glob patterns for files to exclude from scanning
      --exclude-symbols <GLOB>  Glob patterns for symbol names to exclude from output
  -h, --help                    Print help
  -V, --version                 Print version
```

Your job is to take a critical/adversarial position to the results of the current lint command:

- is this outcome REALLY a lint warning/error? The test passing doesn't mean it's a real error. Dig into these warnings/errors.
- what are the categories of lints? what categories are we missing?
- how much better would Javascript and Typescript be if we switched over to Oxlint instead of tree-sitter?
- are we effectively leveraging large open source projects like Neovim? These projects use tree-sitter and are completely open source.
- what caching, if any are we using? where would caching add the most value?

Write all your identified opportunities to @tree-sitter/reviews/{{ ctx.today }}-lint-review.md; for every opportunity:

- specify the benefit using examples
- provide a high/medium/low rating for the impact and the level-of-effort
- indicate the remaining design and research each opportunity requires before we're ready to implement
- provide a true/false recommendation
