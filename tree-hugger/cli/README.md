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
- `lint` - Run lint and syntax diagnostics

## Options

- `--language <LANG>` - Override language detection (`js`/`javascript`, `ts`/`typescript`, `cpp`/`c++`, `c-sharp`/`c#`)
- `--exclude-files <GLOB>` - Exclude files matching glob patterns (in addition to the defaults below)
- `--exclude-symbols <GLOB>` - Exclude symbol names matching glob patterns
- `--json` - Output as JSON
- `--plain` - Disable colors and hyperlinks
- `--group-by-file` - Group symbol output by file
- `--group-by-module` - Group symbol output by module
- `--sort-by-kind` - Sort symbols by kind then name
- `--sort-by-module` - Sort symbols by module then other sort keys

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

With `--json`, output is one document stamped `"schema_version": {"major": 3, "minor": 0}`.
`files[].symbols` holds the v1 symbols and `symbol_indexes` holds the full
schema records (with `visibility.level`, `modifiers`, `identity.container_kind`,
`source.declaration_text` and `relations.implements`):

```json
{
  "schema_version": { "major": 3, "minor": 0 },
  "root_dir": "/path/to/project",
  "language": "Rust",
  "files": [
    {
      "file": "/path/to/src/rust.rs",
      "symbols": [
        {
          "name": "new_associated_function",
          "kind": "Method",
          "range": { "start_line": 94, "start_column": 5 },
          "container_name": "PublicStruct",
          "container_kind": "Type",
          "visibility": "Public",
          "is_static": true,
          "signature": { "return_type": "Self", "is_static": true }
        },
        {
          "name": "PRIVATE_STATIC_MUT",
          "kind": "Variable",
          "range": { "start_line": 26, "start_column": 1 },
          "visibility": "Private",
          "is_static_storage": true
        }
      ]
    }
  ],
  "symbol_indexes": [ { "schema_version": { "major": 3, "minor": 0 }, "symbols": [] } ]
}
```

- `visibility` is a top-level symbol field (it used to be
  `signature.visibility`) and is omitted when the level is unknown.
- `is_static` marks a member usable without an instance; `is_static_storage`
  marks storage or linkage `static` (C `static`, Rust `static` items). Both
  are omitted when `false`.
- Every function, method and constructor has a `signature`, even with no
  parameters or return type.

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
