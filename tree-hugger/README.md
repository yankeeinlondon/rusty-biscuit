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
| Go         | Yes       | Yes   | Yes     | -       | type (struct/interface), alias         |
| Python     | Yes       | Yes   | Yes     | -       | class                                  |
| Java       | Yes       | Yes   | Yes     | -       | class, interface, enum, record         |
| C#         | Yes       | Yes   | -       | -       | class, struct, interface, enum, record |
| C          | Yes       | Yes   | -       | -       | struct, union, enum, typedef alias     |
| C++        | Yes       | Yes   | -       | -       | class, struct, union, enum, alias      |
| Swift      | Yes       | Yes   | Yes     | -       | type, interface (protocol), alias      |
| Scala      | Yes       | Yes   | -       | -       | class, trait, module, enum, alias      |
| PHP        | Yes       | Yes   | -       | -       | class, interface, trait, enum          |
| Perl       | Yes       | -     | -       | -       | package (module)                       |
| Bash       | Yes       | -     | -       | -       | -                                      |
| Zsh        | Yes       | -     | -       | -       | -                                      |
| Lua        | Yes       | -     | -       | -       | -                                      |

## Symbol Kinds

Tree Hugger distinguishes between different kinds of type definitions where the language supports it:

- **Type** - Generic type definition (structs and unions in Rust/C/C++, Swift types, Go types, Java and C# records)
- **TypeAlias** - Type aliases (TypeScript, Rust and Scala `type`, Go `type A = B`, Swift `typealias`, C `typedef`, C++ `typedef`/`using`)
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
- `--jsonl`, `--count`, `--stats` - JSON Lines, the number of matches, or counts and doc coverage (`--stats --json` for JSON)
- `--format plain|table`, `--tree`, `--signatures full|short|none` - unstyled or tabular output, members nested under containers, and how much of each signature to show
- `--file`, `--name`, `--exact` - say whether a filter is a file or a name, and match names whole
- `--kind`, `--scope`, `--all-scopes` - narrow by kind and by scope (`hug symbols` hides locals unless `--all-scopes`)
- `--visibility`, `--exported`, `--in` - narrow by visibility, export, or container
- `--exclude-tests`, `--tests-only`, `--undocumented`, `--deprecated` - narrow by test code, docs, deprecation
- `--async`, `--returns`, `--param-type`, `--min-params` - narrow functions by signature shape

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
`report_schema_version` (`1`), `schema_version` (`4.3`, the records' schema),
`root_dir`, `files` (one entry per file with a match: `file`, `language`,
`completed_passes`, `completed_capabilities` and the selected `symbols`
records; `classes` adds `members` keyed by container ID) and a trailing
`errors` array that lists every file that could not be analyzed. Paths are
`/`-separated and relative to `root_dir`, including a file outside it
(`../vendor/lib.rs`). `--json-full` replaces `files` with
every file's complete `indexes`. The CLI README has a sample.

`imports` and `lint` keep their own shape: `schema_version`, `root_dir`,
`language`, `files` (v1 `FileSummary` entries with `imports`, `lint` and
`syntax`), `symbol_indexes` and `adapter_metadata`.

[Symbol Reports](./docs/symbol-reports.md) describes the declaration
commands end to end: kind sets, the JSON report and its `errors`, ordering
and tie-breakers, `--json-full` and `--verbose`.

[Declared Type Relations](./docs/type-relations.md) describes the
`extends`/`implements` lists on each type record: what each language
records, when a base gets a same-file ID, and how Rust impls attach to their
self type.

## Release Notes: generics, base types, modifiers, attributes and body lines

Records now fill fields that were declared but empty, all read from the
syntax tree in the same single parse (no compiler, no extra pass):

- **Generic parameters** (`type_info.generic_params`): name, kind (`Type`,
  `Lifetime`, `Const`), bounds and default as written, variance and position,
  for Rust, TypeScript, Go, Python, Java, C#, C++, Swift and Scala.
  Requirements on anything other than a declared parameter (`where T::Item:
  Copy`, a C++ `requires` clause) go to `type_info.constraints`.
- **Base types** (`relations.extends`, `relations.implements`, `relations.unclassified_bases`): each base as
  written, with a same-file `id` only when the name names exactly one
  declaration. A Rust trait impl adds its trait to its members and to the
  type it implements. C#, Swift and Scala preserve ambiguous bases in `relations.unclassified_bases`.
- **Modifiers and attributes**: `const fn`, `unsafe`, `mut`, `readonly`,
  `abstract`, `final`, `override`, `virtual`, `sealed`, `mutating`, in one
  fixed order; `attributes` lists the declaration's own attributes,
  annotations and decorators with their raw arguments.
- **Body size**: `metrics.lines` counts the lines of a function, method or
  constructor's body (`null` for other kinds and bodiless declarations).
- **Cyclomatic complexity**: `metrics.cyclomatic` is 1 plus one per decision
  (`if`, a conditional loop, a ternary, `&&`/`||`, a non-default arm, a
  guard, a `catch`) in that same body, nested functions and types left out.
  Rust, JavaScript and TypeScript have counting rules; every other language,
  a bodyless declaration and a body with a syntax error report `null`, which
  never means zero. The rules, with an example each, are in
  [Cyclomatic Complexity](./docs/cyclomatic-complexity.md).
- **Outline rows show bases**: `hug types --tree` and `hug symbols --tree`
  end a type's row with them.

  ```text
  - class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]
  ```

The record schema is now **4.3** (unclassified bases, relation source spans and the required cyclomatic metric). Cache entries written by
earlier versions are re-analyzed once. Records converted to the older
`SymbolInfo` model lose these facts, since that model has no field for them.

## Release Notes: precise, filterable symbol reports

The declaration commands (`functions`, `types`, `symbols`, `classes`) gain
record selectors and new report shapes. Two changes alter existing output.

- **Name filters use smart case.** A pattern without an uppercase letter now
  ignores case; any uppercase letter makes it case-sensitive. Add `!` or
  `--exact` to match a whole name.

  ```text
  # before: `hug functions parse` was case-sensitive
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
  # after: `parse` also finds ParseError; `Parse` still finds only ParseError
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
    - public function ParseError() src/parse.rs:6:1
  ```

- **`hug symbols` hides locals.** Parameters, function-local variables and
  nested functions are `local` scope and are no longer listed by default
  (over this package, 4,635 of the old 16,681 lines remain).
  `--all-scopes` or `--scope local` brings them back; `functions` and `types`
  still list every scope.

  ```text
  # before: hug symbols src/parse.rs
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
    - parameter path src/parse.rs:1:21
    - let parsed src/parse.rs:2:9
    - public function ParseError() -> () src/parse.rs:6:1
  # after (add --all-scopes for the old list)
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
    - public function ParseError() src/parse.rs:6:1
  ```

- **New selectors:** `--kind`, `--scope`/`--all-scopes`, `--visibility`,
  `--exported`, `--in`, `--exclude-tests`/`--tests-only`, `--undocumented`,
  `--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params`, and
  `--file`/`--name`/`--exact` to say what a filter is. One selector path
  serves every command and output; a filtered run reuses the cache entry an
  unfiltered run wrote.
- **New outputs:** `--jsonl` (one self-describing line per declaration),
  `--count`, `--stats` (tables, or `--stats --json`), `--format plain|table`,
  `--tree` (members under their type) and `--signatures full|short|none`.
  Exactly one output mode is accepted per run (`--stats` composes with
  `--json` or a `--format`); `--json --json-full` together is now an argument
  error instead of silently meaning `--json-full`.
- **Records carry more facts.** Every record has a `scope` (`Item`, `Member`,
  `Local`), an `Async` modifier where declared, and a `qualified_name` in the
  language's own spelling (`.` for Java, `::` for Rust, …), prefixed with the
  file's path from the scan root (`src/shapes.rs#Point::norm`) unless a
  namespace or package roots it. `--json` and `--jsonl` always compute
  `is_test` and `is_deprecated`. Because IDs hash the qualified name, some
  record IDs changed; existing cache entries are re-analyzed once.
- **Interactive output wraps** at the terminal width with a hanging indent
  (`COLUMNS` overrides the width). Forced color (`CLICOLOR_FORCE=1`) counts as
  interactive, so it wraps too; `--plain`, `--format plain` and redirected
  output never wrap.
- **Source text cannot style the terminal.** Control characters in names,
  signatures and comments print as `\u{..}` escapes.

## Release Notes: record-native symbol reports

The declaration commands (`functions`, `types`, `symbols`, `classes`) now
read the library's schema 4.0 records directly and analyze only what they
show.

- **Faster.** No listing runs lint, references or semantic analysis, files
  are analyzed in parallel, and a warm cache hit parses nothing. On this
  package, `hug functions --no-cache` went from 1.86 s to 0.12 s and a warm
  `hug functions` from 1.08 s to 0.04 s; peak memory of `hug symbols` over
  the whole monorepo fell from 3.48 GB to 1.95 GB.
- **New default order.** Declarations are listed by file, then source
  position, instead of alphabetically. `--sort name` and `--sort kind` give
  the other orders; `--sort-by-kind` and `--sort-by-module` are removed.

  ```text
  # before: alphabetical across files
    - public method area(self) -> f64 [in type Circle] src/shapes.rs:12:5
    - public struct Circle { radius: f64 } src/shapes.rs:3:1
  # after (default --sort file): file, then position
    - public struct Circle { radius: f64 } src/shapes.rs:3:1
    - public method area(self) -> f64 [in type Circle] src/shapes.rs:12:5
  ```

- **New JSON shape.** `--json` prints a versioned report holding only the
  command's records, with paths relative to `root_dir` and a trailing
  `errors` array. The old shape carried every file's full index, timestamps
  and absolute paths (39 MB for `hug --json functions` on this package, now
  2.7 MB on the same files), and its output changed on every run.

  ```text
  # before: {"schema_version":…,"files":[…],"symbol_indexes":[…every record…],…}
  # after:  {"report_schema_version":1,"schema_version":{"major":4,"minor":0},
  #          "root_dir":"…","files":[{"file":"src/shapes.rs","symbols":[…]}],"errors":[]}
  ```

- **`--json-full`** prints every file's complete index when you need it.
- **Labels come from the source's keyword.** `struct Meter`, `my $queue`,
  `local M` replace guesses such as `type Meter` or `= MAX_SIZE`.
- **A failed file no longer aborts the run.** It is printed to stderr, listed
  in `errors`, and makes `hug` exit `1`; the other files are still reported.
- **Quiet by default.** Cache problems are recoverable and silent;
  `--verbose` reports them with a cache and work stats line.

## Query Vendoring

Tree Hugger vendors `nvim-treesitter` query files in `tree-hugger/lib/queries/vendor/<lang>/` and uses
`locals.scm` for symbol discovery and imports. Captures follow a naming scheme including:

| Capture                       | SymbolKind |
|-------------------------------|------------|
| `@local.definition.function`  | Function   |
| `@local.definition.method`    | Method     |
| `@local.definition.constructor` | Constructor |
| `@local.definition.type`      | Type       |
| `@local.definition.type_alias` | TypeAlias |
| `@local.definition.class`     | Class      |
| `@local.definition.interface` | Interface  |
| `@local.definition.enum`      | Enum       |
| `@local.definition.trait`     | Trait      |
| `@local.definition.module`    | Module     |
| `@local.definition.namespace` | Namespace  |
| `@local.definition.enum_variant` | EnumVariant |
| `@local.definition.field`     | Field      |
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

Tests run through `just` (nextest), from `tree-hugger/`:

```bash
# Library and CLI unit/integration tests (L1)
just test

# Real-terminal tests (L2)
just test-l2

# Clippy over both packages
just lint
```

### Test Fixtures

Test fixtures are located in `lib/tests/fixtures/` and include:

- `sample.*` - Basic syntax for each language (functions, classes)
- `types.*` - Type system features (struct, enum, interface, trait)
- `generics.*` - Generic/parameterized types

Each language with type variability should have a `types.*` fixture that tests all its type constructs.
