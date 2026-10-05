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

## Supported Languages

| Language   | Functions | Types | Imports | Exports | Type Distinction                       |
|------------|:---------:|:-----:|:-------:|:-------:|----------------------------------------|
| Rust       | Yes       | Yes   | Yes     | Yes     | struct, enum, trait                    |
| TypeScript | Yes       | Yes   | Yes     | Yes     | type, interface, enum, class           |
| JavaScript | Yes       | Yes   | Yes     | Yes     | class                                  |
| Go         | Yes       | Yes   | Yes     | -       | type (struct/interface)                |
| Python     | Yes       | Yes   | Yes     | -       | class                                  |
| Java       | Yes       | Yes   | Yes     | -       | class, interface, enum, record         |
| C#         | Yes       | Yes   | -       | -       | class, struct, interface, enum, record |
| C          | Yes       | Yes   | -       | -       | struct, union, enum, typedef alias     |
| C++        | Yes       | Yes   | -       | -       | class, struct, union, enum, alias      |
| Swift      | Yes       | Yes   | Yes     | -       | type, interface (protocol)             |
| Scala      | Yes       | Yes   | -       | -       | class, trait, module, enum             |
| PHP        | Yes       | Yes   | -       | -       | class, interface, trait, enum          |
| Perl       | Yes       | -     | -       | -       | package (module)                       |
| Bash       | Yes       | -     | -       | -       | -                                      |
| Zsh        | Yes       | -     | -       | -       | -                                      |
| Lua        | Yes       | -     | -       | -       | -                                      |

## Symbol Kinds

Tree Hugger distinguishes between different kinds of type definitions where the language supports it:

- **Type** - Generic type definition (structs and unions in Rust/C/C++, Swift types, Go types, Java and C# records)
- **TypeAlias** - Type aliases (TypeScript and Rust `type`, C `typedef`, C++ `typedef`/`using`)
- **Class** - Class definitions (TypeScript, JavaScript, C#, C++, Scala, PHP, Python, Java)
- **Interface** - Interface/protocol definitions (TypeScript, Java, C#, Swift, PHP)
- **Enum** - Enumeration types (Rust, TypeScript, Java, C#, C/C++, Swift, Scala, PHP)
- **Trait** - Trait definitions (Rust, Scala, PHP)
- **Module** - Module definitions (Scala objects, Rust modules, Perl packages)

## Using the CLI

**Syntax:** `hug <COMMAND> <...filter>`

`hug` now scans all supported source files by default. Positional filters are optional and
classified as:

- file filters (path-like / extension-like tokens) that narrow scanned files
- symbol filters (name globs) for `functions`, `types`, `symbols`, and `classes`

### Commands

- `functions` - list functions, methods and constructors
- `types` - list type definitions (includes structs, enums, classes, interfaces, traits, type aliases)
- `symbols` - list all discovered symbols
- `imports` - list imported symbols
- `classes` - list classes, structs, interfaces and traits with their members, split into static and instance methods and fields (a Java interface constant is a static field; a Rust `impl` block's methods are listed under the type it implements)
- `lint` - run lint and syntax diagnostics
- `god-files` - identify oversized source files that are strong candidates for refactoring

### Options

- `--language <LANG>` - override language detection (`js`/`javascript`, `ts`/`typescript`, `cpp`/`c++`, `c-sharp`/`c#`)
- `--exclude-files <GLOB>` - exclude files matching glob patterns
- `--exclude-symbols <GLOB>` - exclude symbol names matching glob patterns
- `--json` - output JSON format
- `--json-full` - output every scanned file's complete index as JSON (declaration commands only; symbol selectors are rejected)
- `--plain` - disable colors and hyperlinks
- `--verbose` - print cache and work stats, and recoverable cache failures, to stderr
- `--group-by-file` - group symbol output by file
- `--group-by-module` - group symbol output by module (directory scope)
- `--sort file|name|kind` - order of declarations (default `file`: by file, then source position); replaces the removed `--sort-by-kind` and `--sort-by-module`

### Examples

```bash
# Scan all source files and list symbols whose names contain `cache`
hug symbols cache

# Restrict to one file and one symbol pattern
hug functions tree-hugger/lib/tests/fixtures/sample.rs greet

# Exclude test files and private helper symbols
hug symbols "src/**/*.rs" --exclude-files "**/tests/**" --exclude-symbols "*_internal*"

# List imports as JSON
hug imports --json "tree-hugger/lib/tests/fixtures/**/*.js"

# List types showing struct vs enum distinction
hug types "src/main.rs" --plain
# Output:
#   - struct Cli { ... } src/main.rs:21:8
#   - enum Command { ... } src/main.rs:64:6
#   - struct OutputConfig { ... } src/main.rs:122:8

# List classes with members
hug classes "src/**/*.ts" --name MyClass

# Run lint diagnostics only
hug lint "src/**/*.rs" --lint-only
```

## JSON Output

`functions`, `types`, `symbols` and `classes` print a versioned report:
`report_schema_version` (`1`), `schema_version` (`4.0`, the records' schema),
`root_dir`, `files` (one entry per file with a match: `file`, `language`,
`completed_passes`, `completed_capabilities` and the selected `symbols`
records; `classes` adds `members` keyed by container ID) and a trailing
`errors` array that lists every file that could not be analyzed. Paths are
`/`-separated and relative to `root_dir`. `--json-full` replaces `files` with
every file's complete `indexes`. The CLI README has a sample.

`imports` and `lint` keep their own shape: `schema_version`, `root_dir`,
`language`, `files` (v1 `FileSummary` entries with `imports`, `lint` and
`syntax`), `symbol_indexes` and `adapter_metadata`.

## Query Vendoring

Tree Hugger vendors `nvim-treesitter` query files in `tree-hugger/lib/queries/vendor/<lang>/` and uses
`locals.scm` for symbol discovery and imports. Captures follow a naming scheme including:

| Capture                       | SymbolKind |
|-------------------------------|------------|
| `@local.definition.function`  | Function   |
| `@local.definition.method`    | Method     |
| `@local.definition.type`      | Type       |
| `@local.definition.type_alias` | TypeAlias |
| `@local.definition.class`     | Class      |
| `@local.definition.interface` | Interface  |
| `@local.definition.enum`      | Enum       |
| `@local.definition.trait`     | Trait      |
| `@local.definition.module`    | Module     |
| `@local.definition.namespace` | Namespace  |
| `@local.definition.enum_variant` | EnumVariant |
| `@local.definition.property`  | Property   |
| `@local.definition.import`    | (imports)  |

Additional languages may be added by vendoring the corresponding `locals.scm` file. nvim-treesitter
has no Perl `locals.scm`, so `queries/vendor/perl/locals.scm` is Tree Hugger's own query: named
subs, `package` statements and blocks, and `my`/`our`/`state` variables.

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
