---
name: tree-hugger
description: Expert knowledge for multi-language symbol extraction using Tree-sitter. Use when working with tree-hugger-lib or tree-hugger-cli (hug), extracting symbols/imports/exports, implementing lint diagnostics, adding new language support, writing tree-sitter queries, or troubleshooting the underlying tree-sitter runtime (grammar versions, ABI mismatches, query debugging).
---

## Purpose

Tree Hugger provides Tree-sitter-based symbol extraction and diagnostics across 16 programming languages. Use this skill when:

- Working in `tree-hugger/lib/` or `tree-hugger/cli/`
- Extracting functions, types, imports, or exports from source files
- Implementing or debugging lint diagnostics
- Adding support for new languages or fixing query issues
- Understanding the query vendoring system

## Quick Reference

### Supported Languages

Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++, Swift, Scala, PHP, Perl, Bash, Zsh, Lua

### CLI Commands (`hug`)

| Command | Description |
|---------|-------------|
| `functions` | List functions and methods |
| `types` | List type definitions (struct, enum, class, interface, trait) |
| `symbols` | List all symbols |
| `imports` | List imported symbols |
| `classes` | List classes with static/instance member partitioning |
| `lint` | Run lint and syntax diagnostics |
| `completions` | Generate shell completions (Bash, Zsh, Fish, PowerShell) |

### Library API (`TreeFile`)

```rust
let file = TreeFile::new("src/lib.rs")?;
file.symbols()?;            // All symbols
file.symbol_records()?;     // v2 parse records
file.symbol_index_v2()?;    // v2 parse->bind->semantic->docs pipeline output
file.imported_symbols()?;   // Imports
file.exported_symbols()?;   // Exports
file.local_symbols()?;      // Local definitions
file.referenced_symbols()?; // Identifier usages
file.lint_diagnostics();    // Pattern lint rules
file.syntax_diagnostics();  // Parse errors
file.dead_code();           // Unreachable code blocks
```

### SymbolKind Enum

Function, Method, Type, Class, Interface, Enum, Trait, Module, Namespace, Variable, Parameter, Field, Macro, Constant, Unknown

## Key Architecture

- **Runtime**: Built on the tree-sitter Rust runtime (`tree-sitter = "0.26.3"`) plus one `tree-sitter-<lang>` grammar crate per language, all at independent versions — see [Tree-sitter Internals](./tree-sitter-internals.md)
- **Query vendoring**: Uses nvim-treesitter `locals.scm` in `lib/queries/vendor/<lang>/`
- **Custom queries**: `lint.scm`, `references.scm`, `comments.scm` per language
- **Capture naming**: `@local.definition.<kind>` for symbols, `@diagnostic.<rule-id>` for lint
- **Query caching**: Global `OnceLock<QueryCache>` for thread-safe caching
- **Builtin database**: Per-language builtin lists to avoid false positive undefined-symbol/undefined-module errors
- **Schema v2**: `shared/schema_v2` with facet-based `SymbolRecord` and staged `FileSymbolIndex`
- **CLI filters**: `hug <subcommand> <filter...>` with all-files default scan for symbol commands

## Detailed Documentation

- [API Reference](./api-reference.md) - Complete type definitions and methods
- [Query System](./query-system.md) - Tree Hugger's `.scm` queries, capture conventions, and how to extend them
- [Diagnostics](./diagnostics.md) - Lint rules, ignore directives, dead code detection
- [Tree-sitter Internals](./tree-sitter-internals.md) - The underlying runtime: Parser/Tree/Node API, query mechanics, AST debugging, and grammar version/ABI compatibility

## Testing Requirements

When modifying queries or symbol extraction:

1. Every language with type constructs must have type distinction tests
2. All typed languages need `types.*` fixture files
3. Bug fixes require regression tests
4. Run `cargo test -p tree-hugger-lib` to verify

## Known Limitations

- **Swift**: All types captured as `SymbolKind::Type` (grammar limitation)
- **Go**: Struct and interface both captured as `SymbolKind::Type`
