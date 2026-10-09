# Tree Hugger CLI

`hug` explores the declarations, imports, references and diagnostics in
source files across 16 languages, using Tree-sitter, and points out files
that have grown too large.

> **Full documentation:** [Using the `hug` CLI](../docs/cli.md) is the
> reference for every command and option. This README is only an overview.

## Install

```bash
cargo install --path tree-hugger/cli
```

## At a glance

```bash
hug symbols                                   # every declaration in the nearest package
hug functions --in Widget --undocumented      # undocumented members of Widget
hug types --changed=main --json               # types in files changed since main, as JSON
hug symbols --format ctags > tags             # a tags file for your editor
hug refs parse --callers 'src/**'             # who calls `parse`
hug lint 'src/**/*.rs'                        # lint and syntax diagnostics
hug god-files --high-risk                     # files that most need splitting
```

| Command | What it does |
|---------|--------------|
| `functions`, `types`, `symbols`, `classes` | List declarations, with filters, sorting, JSON, stats and export formats |
| `refs` | References to one declaration, its callers or its callees |
| `imports` | Imported symbols |
| `lint` | Lint and syntax diagnostics |
| `god-files` | Oversized files that are refactoring candidates |
| `completions` | Shell completions |

## Learn more

- [Tree Hugger Overview](../docs/overview.md): what Tree Hugger does and where to read next
- [Using the `hug` CLI](../docs/cli.md): every command, option, output format and exit status
- [Symbol Reports](../docs/symbol-reports.md): the declaration commands in depth
- [Scope-Aware References](../docs/references.md): how `hug refs` resolves names
- [Tree Hugger](../README.md): the library and the package as a whole
