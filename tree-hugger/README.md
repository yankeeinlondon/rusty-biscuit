---
name: frontmatter
---

# Tree Hugger

A library (and CLI) for generating diagnostics and symbol summaries across multiple programming languages using Tree-sitter.

<img src="../assets/tree-hugger-512.png" style="width: 250" />

## Packages

```txt
tree-hugger
├── cli - uses `clap` to provide a CLI application (`hug`)
├── lib - library APIs for symbol and diagnostic extraction
```

- [Tree Hugger CLI](./cli/README.md): the `hug` command
- [Tree Hugger Library](./lib/README.md): the `tree-hugger` crate

## Install

```bash
cargo install --path tree-hugger/cli   # from the repository root
just install                           # or, from tree-hugger/
```

## Supported Languages

Tree Hugger reports functions in all 16 languages, and types and imports
where the language has them:

| Language   | Type kinds                             | Imports |
|------------|----------------------------------------|:-------:|
| Rust       | struct, enum, trait, alias             | Yes     |
| TypeScript | type alias, interface, enum, class     | Yes     |
| JavaScript | class                                  | Yes     |
| Go         | type (struct/interface), alias         | Yes     |
| Python     | class                                  | Yes     |
| Java       | class, interface, enum, record         | Yes     |
| C#         | class, struct, interface, enum, record | Yes     |
| C          | struct, union, enum, typedef alias     | -       |
| C++        | class, struct, union, enum, alias      | -       |
| Swift      | type, interface (protocol), alias      | Yes     |
| Scala      | class, trait, module, enum, alias      | Yes     |
| PHP        | class, interface, trait, enum          | Yes     |
| Perl       | package (module)                       | -       |
| Bash       | -                                      | -       |
| Zsh        | -                                      | -       |
| Lua        | -                                      | -       |

Each declaration gets a **kind** (Function, Method, Class, Interface, Enum,
Trait, TypeAlias, Field and so on), kept as distinct as the language allows.
[Symbol Kinds, Visibility and `static`](./docs/symbol-kinds-and-visibility.md)
lists exactly what each language reports.

## Using the CLI

The `hug` binary lists declarations, references, imports and diagnostics, and
finds oversized files:

```bash
hug symbols                         # every declaration in the nearest package
hug functions --changed --json      # functions in uncommitted files, as JSON
hug refs parse --callers 'src/**'   # who calls `parse`
hug lint 'src/**/*.rs'              # lint and syntax diagnostics
```

[Using the `hug` CLI](./docs/cli.md) is the reference for every command and
option.

## Documentation

The [`docs/`](./docs/) pages are the reference; this README is only an
overview.

- [Tree Hugger Overview](./docs/overview.md): **start here**: what Tree Hugger
  does, its core ideas, and which page to read next
- [Using the `hug` CLI](./docs/cli.md): every command, option, output format
  and exit status
- [Using the Tree Hugger Library](./docs/library.md): the Rust API
- [Symbol Reports](./docs/symbol-reports.md): the declaration commands in depth
- [Symbol Records](./docs/symbol-records.md): what each record holds
- [Symbol Kinds, Visibility and `static`](./docs/symbol-kinds-and-visibility.md)
- [Scope-Aware References](./docs/references.md): occurrences and `hug refs`
- [Re-exports](./docs/reexports.md): what each language forwards, and how
  complete the report is
- [Declared Type Relations](./docs/type-relations.md) and
  [Cyclomatic Complexity](./docs/cyclomatic-complexity.md)
- [Tree Hugger's Persistent Symbol Cache](./docs/cache-design.md)
- [Tree-sitter Queries](./docs/queries.md): the query files behind every
  report, and what is vendored from nvim-treesitter
- [Testing Tree Hugger](./docs/testing.md): running the tests, for
  contributors
- [Release Notes](./docs/release-notes.md): what changed, with before/after
  examples
