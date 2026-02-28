---
name: frontmatter
---

# Tree Hugger

A library (and CLI) for generating diagnostics and symbol summaries across multiple programming languages using Tree-sitter.

## Packages

```txt
tree-hugger
├── cli - uses `clap` to provide a CLI application (`hug`)
├── lib - library APIs for symbol and diagnostic extraction
```

## Supported Languages

| Language   | Functions | Types | Imports | Exports | Type Distinction                       |
|------------|:---------:|:-----:|:-------:|:-------:|----------------------------------------|
| Rust       | Yes       | Yes   | Yes     | Yes     | struct, enum, trait                    |
| TypeScript | Yes       | Yes   | Yes     | Yes     | type, interface, enum, class           |
| JavaScript | Yes       | Yes   | Yes     | Yes     | class                                  |
| Go         | Yes       | Yes   | Yes     | -       | type (struct/interface)                |
| Python     | Yes       | Yes   | Yes     | -       | class                                  |
| Java       | Yes       | Yes   | Yes     | -       | class, enum, record                    |
| C#         | Yes       | Yes   | -       | -       | class, struct, interface, enum, record |
| C          | Yes       | Yes   | -       | -       | struct, enum                           |
| C++        | Yes       | Yes   | -       | -       | class, struct, enum                    |
| Swift      | Yes       | Yes   | Yes     | -       | type, interface (protocol)             |
| Scala      | Yes       | Yes   | -       | -       | class, trait, module, enum             |
| PHP        | Yes       | Yes   | -       | -       | class, interface, trait, enum          |
| Perl       | Yes       | -     | -       | -       | (minimal support)                      |
| Bash       | Yes       | -     | -       | -       | -                                      |
| Zsh        | Yes       | -     | -       | -       | -                                      |
| Lua        | Yes       | -     | -       | -       | -                                      |

## Symbol Kinds

Tree Hugger distinguishes between different kinds of type definitions where the language supports it:

- **Type** - Generic type definition (structs in Rust/C/C++/Swift/Go, type aliases in TypeScript)
- **Class** - Class definitions (C#, Swift, Scala, PHP, Python, Java)
- **Interface** - Interface/protocol definitions (TypeScript, C#, Swift, PHP)
- **Enum** - Enumeration types (Rust, TypeScript, Java, C#, C/C++, Swift, Scala, PHP)
- **Trait** - Trait definitions (Rust, Scala, PHP)
- **Module** - Module/namespace definitions (Scala objects, Rust modules)

## Using the CLI

**Syntax:** `hug <COMMAND> <...filter>`

`hug` now scans all supported source files by default. Positional filters are optional and
classified as:

- file filters (path-like / extension-like tokens) that narrow scanned files
- symbol filters (name globs) for `functions`, `types`, `symbols`, and `classes`

### Commands

- `functions` - list functions and methods
- `types` - list type definitions (includes structs, enums, classes, interfaces, traits)
- `symbols` - list all discovered symbols
- `imports` - list imported symbols
- `classes` - list classes with members (static/instance partitioning)
- `lint` - run lint and syntax diagnostics

### Options

- `--language <LANG>` - override language detection
- `--ignore <GLOB>` - exclude paths
- `--json` - output JSON format
- `--plain` - disable colors and hyperlinks
- `--group-by-file` - group symbol output by file
- `--group-by-module` - group symbol output by module (directory scope)
- `--sort-by-kind` - sort symbols by kind then name
- `--sort-by-module` - sort symbols by module then other keys

### Examples

```bash
# Scan all source files and list symbols whose names contain `cache`
hug symbols cache

# Restrict to one file and one symbol pattern
hug functions tree-hugger/lib/tests/fixtures/sample.rs greet

# List imports as JSON
hug imports --json "tree-hugger/lib/tests/fixtures/**/*.js"

# List types showing struct vs enum distinction
hug types "src/main.rs" --plain
# Output:
#   - type Cli { ... } [21:8]
#   - enum Command { ... } [64:6]
#   - type OutputConfig { ... } [122:8]

# List classes with members
hug classes "src/**/*.ts" --name MyClass

# Run lint diagnostics only
hug lint "src/**/*.rs" --lint-only
```

## JSON Output

When `--json` is selected, output includes v2 schema metadata and pass-aware symbol indexes:

- `schema_version` - active symbol schema version (`2.0`)
- `root_dir` - the directory scanned
- `language` - primary language for the run
- `files` - command-specific v1-compatible `FileSummary` list
- `symbol_indexes` - per-file `FileSymbolIndex` with staged pass metadata

Each `FileSummary` includes:

- `file`, `language`, `hash`
- `symbols` (for `functions`, `types`, or `symbols`) - each with a `kind` field
- `imports` (for `imports` and `symbols`)
- `exports` (for `symbols`)
- `locals` (for `symbols`)
- `lint`, `syntax`

### Example JSON Output

```json
{
  "name": "Message",
  "kind": "Enum",
  "range": { "start_line": 11, "start_column": 10, ... },
  "language": "Rust",
  "doc_comment": "Message types for the application.",
  "type_metadata": {
    "variants": [
      { "name": "Quit" },
      { "name": "Write", "tuple_fields": ["String"] },
      { "name": "Move", "struct_fields": [{ "name": "x", "type_annotation": "i32" }] }
    ]
  }
}
```

## Query Vendoring

Tree Hugger vendors `nvim-treesitter` query files in `tree-hugger/lib/queries/vendor/<lang>/` and uses
`locals.scm` for symbol discovery and imports. Captures follow a naming scheme including:

| Capture                       | SymbolKind |
|-------------------------------|------------|
| `@local.definition.function`  | Function   |
| `@local.definition.method`    | Method     |
| `@local.definition.type`      | Type       |
| `@local.definition.class`     | Class      |
| `@local.definition.interface` | Interface  |
| `@local.definition.enum`      | Enum       |
| `@local.definition.trait`     | Trait      |
| `@local.definition.module`    | Module     |
| `@local.definition.namespace` | Namespace  |
| `@local.definition.import`    | (imports)  |

Additional languages may be added by vendoring the corresponding `locals.scm` file. Currently,
Perl lacks an upstream `locals.scm`, so symbol extraction for Perl is minimal until a query is added.

## Testing

Tree Hugger maintains comprehensive test coverage across all supported languages to ensure correct symbol extraction and type distinction.

### Test Coverage Requirements

**IMPORTANT:** When modifying tree-sitter queries or symbol extraction logic, ensure tests cover:

1. **Type distinction tests** - Verify that different type constructs (struct vs enum, class vs interface) are correctly identified with distinct `SymbolKind` values
1. **Cross-language parity** - All typed languages should have fixture files that exercise their type system features
1. **Regression tests** - Any bug fix should include a test that would catch the bug if it recurs
1. **Diagnostics coverage** - Lint and syntax diagnostics should include regression tests for representative languages

### Running Tests

```bash
# Run all library tests
cargo test -p tree-hugger-lib

# Run CLI tests
cargo test -p tree-hugger-cli

# Run a specific test
cargo test -p tree-hugger-lib -- distinguishes_rust_struct_from_enum
```

### Test Fixtures

Test fixtures are located in `lib/tests/fixtures/` and include:

- `sample.*` - Basic syntax for each language (functions, classes)
- `types.*` - Type system features (struct, enum, interface, trait)
- `generics.*` - Generic/parameterized types

Each language with type variability should have a `types.*` fixture that tests all its type constructs.
