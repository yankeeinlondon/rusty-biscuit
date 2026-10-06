# Tree Hugger CLI

Tree Hugger provides the `hug` CLI for exploring symbols, imports, and diagnostics across multiple
languages.

## Installation

```bash
cargo install --path tree-hugger/cli
```

## Usage

```bash
hug symbols
hug functions --language rust src/main.rs greet
hug imports --json "tests/fixtures/**/*.js"
```

## Commands

- `functions` - List function, method and constructor definitions
- `types` - List type definitions (structs, unions, enums, classes, interfaces, traits, type aliases)
- `symbols` - List all discovered symbols
- `imports` - List imported symbols
- `classes` - List classes, structs, interfaces and traits with members partitioned by static/instance
- `refs` - List the references to one declaration within its file, or its callers or callees (see [Refs Command](#refs-command))
- `lint` - Run lint and syntax diagnostics

## Options

- `--language <LANG>` - Override language detection (`js`/`javascript`, `ts`/`typescript`, `cpp`/`c++`, `c-sharp`/`c#`)
- `--exclude-files <GLOB>` - Exclude files matching glob patterns (in addition to the defaults below)
- `--exclude-symbols <GLOB>` - Exclude symbol names matching glob patterns
- `--json` - Output as JSON
- `--json-full` - Output every scanned file's complete index as JSON, diagnostics included (`functions`, `types`, `symbols` and `classes` only; symbol selectors such as name filters, `--exported`, `--prelude` or `--static-only` are rejected)
- `--plain` - Disable colors and hyperlinks
- `--verbose` - Print the cache stats line (persistent-cache hits, partial hits and misses, plus files parsed, definition captures and diagnostics runs) and any recoverable cache failure to stderr
- `--no-cache` - Neither read nor write the persistent symbol cache
- `--group-by-file` - Group symbol output by file
- `--group-by-module` - Group symbol output by module (the file's directory)
- `--sort file|name|kind` - Order of the listed declarations (`functions`, `types`, `symbols` and `classes` only). `file` (the default) lists by file, then source position; `name` by the case-sensitive name; `kind` by kind (modules and types, then callables, then values). Ties fall back to file, position, kind and ID. Give it once; `lint` and `imports` reject it
- `--jsonl` - One JSON object per selected declaration (JSON Lines); a `classes` line adds the container's `members` list
- `--count` - Print only the number of selected declarations (`classes`: containers)
- `--stats` - Counts by kind, visibility and language plus doc coverage, as tables; with `--json`, a stats envelope
- `--format plain|table` - The listing without styles, links or wrapping, or one table per file (Location, Kind, Visibility, Name, Signature)
- `--signatures full|short|none` - The declaration header as written, the compact form (default; field lists past four end in `… +N`), or names only
- `--tree` - Nest members under their container (`functions`, `types`, `symbols`); type rows end with their declared bases (`extends Base; implements Shape; unclassified bases External`)

### Selectors

`functions`, `types`, `symbols` and `classes` share these record selectors.
They combine with AND; the values of a repeated selector are alternatives.
Names use smart case: a pattern without an uppercase letter ignores case.

- `--file <PATH|GLOB>` / `--name <PATTERN>` (`-n`) - Say whether a filter is a file or a name instead of letting `hug` guess from a positional token; `--exact` matches names whole and literally
- `--kind <KIND>` - Only these kinds (snake_case: `function`, `type_alias`, `enum_variant`, …); narrows the command's kinds, never widens them
- `--scope item|member|local`, `--all-scopes` - Only these scopes; `symbols` hides locals (`item` and `member` by default), the other commands show every scope
- `--visibility <LEVEL>` - Exactly this visibility (`public`, `crate`, `private`, …, `unknown`)
- `--exported` - Only exported declarations
- `--in <PATTERN>` - Only members of a matching container (`classes`: the container itself)
- `--exclude-tests`, `--tests-only` - Drop, or keep only, test code and helpers in test files
- `--undocumented`, `--deprecated` - No doc comment / marked deprecated
- `--async`, `--returns <GLOB>`, `--param-type <GLOB>`, `--min-params <N>` - Signature shape; keeps only functions, methods and constructors

```bash
hug symbols --kind trait --exported            # exported traits
hug functions --in Widget --undocumented        # undocumented Widget members
hug functions --async --returns 'Result<*'      # async functions returning a Result
hug symbols --all-scopes --name '*/lib.rs#*'    # everything in any lib.rs, locals included
```

Every selector, with examples, is in `docs/symbol-reports.md`.

Exactly one output mode is active (terminal, `--json`, `--json-full`,
`--jsonl`, `--count`, `--stats` or `--format`); `--stats` alone composes,
with `--json` or a `--format`. Interactive output wraps at the terminal width
(a positive `COLUMNS` overrides it); `--plain`, `--format plain` and
redirected output never wrap. See `docs/symbol-reports.md`.

The old `--sort-by-kind` and `--sort-by-module` flags are gone: `--sort kind`
replaces the first, and the default `--sort file` (portable path, then source
position) covers the second.

## Default Scan Exclusions

A whole-package scan (no path/glob argument) honors `.gitignore` and additionally
skips test *data* directories, whose contents are inputs rather than compiled
source: `**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`.
Naming such a file explicitly (e.g. `hug lint tests/fixtures/sample.rs`) overrides
the exclusion. Normal test code (`tests/*.rs`) is still scanned.

For `lint`, a multi-file scan only prints files that have diagnostics; if every
file is clean it prints a single `No lint diagnostics in N files.` line. A
single-file lint still prints `(no diagnostics)` as explicit confirmation.

## Output Format

### Pretty Output (default)

Each symbol line shows its visibility, kind, name (with signature or members),
container and location:

```
  - public struct PublicStruct { public_field: u32, crate_field: u32, private_field: u32 } src/rust.rs:50:1
  - public method new_associated_function() -> Self [in type PublicStruct] src/rust.rs:94:5
  - crate field crate_field [in type PublicStruct] src/rust.rs:52:16
  - method fmt(self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result [in type PublicStruct] src/rust.rs:130:5
  - private enum variant PrivateEnumVariant [in enum PrivateEnum] src/rust.rs:72:5
  - private type alias PrivateAlias src/rust.rs:30:1
```

Declarations are listed by file, then source position; `--sort name` and
`--sort kind` change that, also within `--group-by-file` and
`--group-by-module` groups. The word before the name is the declaring keyword
for types, variables, fields and constants (`struct`, `let`, `const`, `my`,
`local`, ...), read from the record, and the kind otherwise.

The listing is printed as files are analyzed, so `hug symbols | head` stops
the scan after a batch or two. Flat `--sort name` and `--sort kind` listings
are the exception, as is `--prelude`: each package is printed once it is fully
analyzed. When the scanned files span two or more packages, the listing has
one section per package (`(no symbols)` for a package without declarations);
that is decided from the files before any is analyzed, so it never delays
output.

#### Visibility labels

The label is the access declared or implied at the declaration:
`public`, `protected`, `private`, `internal`, `package`, `crate`,
`module-private`, `protected internal` and `private protected`. When the
level is unknown (parameters and locals, Python/Perl/shell code, a Rust
`impl` of a trait declared in another file such as `fmt` above) no label is
printed rather than a guess. The rules per language are in
[Symbol Kinds, Visibility and `static`](../docs/symbol-kinds-and-visibility.md).

#### Containers

A member ends with `[in <kind> <name>]` naming its container, or
`[in <name>]` when the container's kind is not known from the file. Members of
a Rust `impl` block are grouped under the implemented type, so a method in
`impl Point` or `impl Display for Point` prints `[in type Point]`, never
`[in impl Point]`.

### JSON Output

With `--json`, `functions`, `types`, `symbols` and `classes` print one report:

```json
{
  "report_schema_version": 1,
  "schema_version": { "major": 5, "minor": 0 },
  "root_dir": "/path/to/project",
  "files": [
    {
      "file": "src/rust.rs",
      "language": "Rust",
      "completed_passes": ["Parse"],
      "completed_capabilities": ["declarations"],
      "symbols": [
        {
          "id": "…",
          "kind": "Method",
          "identity": { "name": "new_associated_function", "container_name": "PublicStruct", "container_kind": "Type", "keyword": "fn" },
          "source": { "file_path": "src/rust.rs", "signature_text": "pub fn new_associated_function() -> Self" },
          "visibility": { "level": "Public" },
          "modifiers": [{ "type": "Static" }]
        }
      ]
    }
  ],
  "errors": []
}
```

(Records are shown abridged.)

- `symbols` holds only the command's records, as schema 5.0 `SymbolRecord`s,
  ordered by `--sort` within each file. File entries are always in portable
  path order, and a file with no match has no entry; an empty selection is
  `files: []`.
- `classes` entries add `members`: each container's displayed methods and
  constructors, keyed by the container's ID (`--static-only` and
  `--instance-only` apply). Fields stay in the container's `kind_data`.
- Records carry their declared facts on every JSON format: generic
  parameters and constraints (`type_info`), `modifiers`, `attributes` and the
  written bases (`relations.extends`/`implements`/`unclassified_bases`). The `metrics` facet,
  including a callable's body size `metrics.lines` and cyclomatic complexity
  `metrics.cyclomatic`, is in `--json-full`, and in `--json`/`--jsonl` only
  when `--undocumented` or `--min-params` asks for metrics.
- Records carry the declaration header (`source.signature_text`) but not the
  declaration's source text, attached comments or work the command did not
  ask for (references, semantics); `completed_passes` and
  `completed_capabilities` say what was computed. `--json-full` has the same
  envelope with complete per-file `indexes` (every kind, every pass,
  diagnostics and `declaration_text`) in place of `files`.
- Every path is `/`-separated and relative to `root_dir`, including a file
  outside it (`../vendor/lib.rs`); record IDs are unchanged by that. A file on
  another Windows drive keeps its absolute path (`D:/vendor/lib.rs`).
- `errors` is always present and always last. Each failed file is one entry
  (`file`, `kind` — the error variant in snake case such as `io` — and
  `message`), is also written to stderr, and makes `hug` exit `1`.
- Records carry no timestamps, so two runs over the same files print the same
  bytes, with or without the cache and whatever the number of workers.

`imports` and `lint` keep their own JSON shape: `files` (per-file summaries
of imports or diagnostics), `symbol_indexes` and `adapter_metadata`.

[Symbol Reports](../docs/symbol-reports.md) covers the declaration commands
in depth: what each command analyzes, the flow from query to output, ordering
and grouping, streaming, failed files and exit status, and `--verbose`.

## Symbol Kinds

The CLI colors each kind:

| Kind | Display | Color |
|------|---------|-------|
| Function, Method, Constructor | `function`, `method`, `constructor` | Green |
| Type, TypeAlias, Class, Interface | `type`, `type alias`, `class`, `interface` | Magenta |
| Enum, EnumVariant | `enum`, `enum variant` | Cyan |
| Field, Property | `field`, `property` | Cyan |
| Trait | `trait` | Yellow |
| Module, Namespace | `module`, `namespace` | Yellow |
| Variable, Parameter | `variable`, `parameter` | Blue |
| Constant | `constant` | Bright blue |
| Macro | `macro` | Red |

Some lines show the declaring keyword from the source instead of the kind
name (`struct`, `union`, `const`, `static`, `let`).

## Examples

```bash
# List all types in the nearest package (default all-file scan)
hug types

# List functions in TypeScript files, excluding tests
hug functions "src/**/*.ts" --exclude-files "**/test/**"

# List symbols while excluding generated helper names
hug symbols "src/**/*.rs" --exclude-symbols "*_generated*"

# Export all symbols as JSON for tooling
hug symbols "lib/**/*.rs" --json > symbols.json

# Check imports in JavaScript files
hug imports "src/**/*.js" --plain

# List classes with their members
hug classes "src/**/*.ts"

# Filter classes by name
hug classes "src/**/*.java" --name Controller

# Show only static members
hug classes "src/**/*.ts" --static-only

# Run lint diagnostics
hug lint "src/**/*.rs"

# Run only lint rules (skip syntax errors)
hug lint "src/**/*.rs" --lint-only

# Run only syntax diagnostics
hug lint "src/**/*.rs" --syntax-only

# List registered lint rules
hug lint --list-rules
```

## Classes Command

The `classes` command summarizes Class, Type (struct, union, record),
Interface and Trait symbols. Methods and constructors are partitioned by
whether they need an instance, and so are the fields in the container's type
metadata. Storage `static` (a C++ function-local `static`, for example) is
never listed as a static member. Rust `impl` members appear under their type,
with associated functions under *Static Methods*; a Java interface constant
appears under *Static Fields*:

```bash
hug classes "src/**/*.ts"
```

Output shows classes with members partitioned:

```
src/service.ts (TypeScript)
  class UserService [5:1]
    Static Methods (2)
      - public getInstance() [7:3]
      - private validateConfig(config: Config) [12:3]
    Instance Methods (3)
      - public getUser(id: string): User [20:3]
      - private fetchData() [25:3]
    Static Fields (1)
      - private instance: UserService
    Instance Fields (2)
      - private cache: Map<string, User>
      - public readonly config: Config
```

For Rust, inherent and trait-impl methods are listed under the type they
implement:

```
src/rust.rs (Rust)
  type PublicStruct [50:1]
    Static Methods (2)
      - public new_associated_function() -> Self [in type PublicStruct] [94:5]
      - crate crate_associated_function() -> () [in type PublicStruct] [108:5]
    Instance Methods (4)
      - public instance_method(self) -> u32 [in type PublicStruct] [102:5]
      - private private_instance_method(self) -> () [in type PublicStruct] [106:5]
      - public required_trait_method(self) -> u32 [in type PublicStruct] [120:5]
      - fmt(self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result [in type PublicStruct] [130:5]
    Instance Fields (3)
      - public public_field: u32
      - crate crate_field: u32
      - private private_field: u32
```

Options:
- `--name <FILTER>` - Filter classes by name (substring match)
- `--static-only` - Show only static members
- `--instance-only` - Show only instance members

## Refs Command

`hug refs` lists where one declaration is referenced, with how sure the
analysis is about each reference:

```sh
hug refs parse 'src/**'                                  # incoming calls and usages
hug refs 'src/parser.rs#Parser::parse' --callers 'src/**' # incoming calls only
hug refs --id <ID> --callees 'src/**'                     # calls its body makes
```

- The first value is the declaration's name (exact, smart case, every kind
  and scope, locals included); the rest are file filters. `--id` selects by
  record ID instead, and then every value is a file filter.
- A name matching several declarations is an error (exit 1) listing each
  one's location and ID; an unknown name or ID is an empty report.
- References are resolved within one file, so a caller in another file is
  never listed, even under the same name. Every report says so.
- Text and `--json` only; `--exclude-files`, `--language` and `--no-cache`
  work as for the other commands.

The full contract is in [`docs/references.md`](../docs/references.md#hug-refs).

## Lint Command

The `lint` command runs pattern-based and semantic diagnostics:

```bash
hug lint "src/**/*.rs"
```

Output shows categorized diagnostics:

```
src/main.rs (Rust)
[lint] warning [unwrap-call]: Explicit `.unwrap()` call
  --> src/main.rs:42:15
    |
 42 |     let value = result.unwrap();
    |                        ^^^^^^

[semantic] error [undefined-symbol]: Reference to undefined symbol `foo`
  --> src/main.rs:50:5

[syntax] error: Missing semicolon
  --> src/main.rs:60:20
```

Options:
- `--lint-only` - Show only lint diagnostics (pattern rules and semantic analysis)
- `--syntax-only` - Show only syntax diagnostics (parse errors)
- `--deny <RULE|CATEGORY>` - Treat matching lint rules as errors
- `--warn <RULE|CATEGORY>` - Treat matching lint rules as warnings
- `--allow <RULE|CATEGORY>` - Treat matching lint rules as info
- `--strict` - Promote warnings to errors
- `--experimental-semantics` - Enable experimental semantic diagnostics
- `--list-rules` - Print registered rule metadata and exit
