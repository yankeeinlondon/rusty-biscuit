# Tree Hugger CLI

Tree Hugger provides the `hug` CLI for exploring symbols, imports, and exports across multiple
languages.

## Installation

```bash
cargo install --path tree-hugger/cli
```

## Usage

```bash
hug symbols "src/**/*.rs"
hug functions --language rust "src/**/*.rs"
hug imports --json "tests/fixtures/**/*.js"
```

## Commands

- `functions` - List function and method definitions
- `types` - List type definitions (structs, enums, classes, interfaces, traits)
- `symbols` - List all discovered symbols
- `exports` - List exported symbols
- `imports` - List imported symbols
- `classes` - List classes with members partitioned by static/instance
- `lint` - Run lint and syntax diagnostics

## Options

- `--language <LANG>` - Override language detection
- `--ignore <GLOB>` - Exclude files matching pattern
- `--json` - Output as JSON
- `--plain` - Disable colors and hyperlinks

## Output Format

### Pretty Output (default)

The default output shows symbols with their kind, metadata, and location:

```
/path/to/file.rs (Rust)
  - type Cli { ignore: Vec<String>, json: bool, command: Command } [21:8]
  - type CommonArgs { inputs: Vec<String> } [57:8]
  - enum Command { Functions(CommonArgs), Types(CommonArgs), Symbols(CommonArgs) } [64:6]
  - enum OutputFormat { Pretty, Plain, Json } [112:6]
  - type OutputConfig { use_colors: bool, use_hyperlinks: bool } [122:8]
```

Note how `type` (structs) and `enum` are clearly distinguished.

### JSON Output

With `--json`, output includes full symbol metadata:

```json
{
  "root_dir": "/path/to/project",
  "language": "Rust",
  "files": [
    {
      "file": "/path/to/types.rs",
      "symbols": [
        {
          "name": "Point",
          "kind": "Type",
          "range": { "start_line": 2, "start_column": 12 },
          "doc_comment": "A tuple struct representing a 2D point.",
          "type_metadata": {
            "fields": [
              { "name": "0", "type_annotation": "i32" },
              { "name": "1", "type_annotation": "i32" }
            ]
          }
        },
        {
          "name": "Message",
          "kind": "Enum",
          "range": { "start_line": 11, "start_column": 10 },
          "doc_comment": "Message types for the application.",
          "type_metadata": {
            "variants": [
              { "name": "Quit" },
              { "name": "Write", "tuple_fields": ["String"] },
              { "name": "Move", "struct_fields": [{ "name": "x" }, { "name": "y" }] }
            ]
          }
        }
      ]
    }
  ]
}
```

## Symbol Kinds

The CLI displays different symbol kinds with distinct labels:

| Kind | Display | Color |
|------|---------|-------|
| Function | `function` | Green |
| Method | `method` | Green |
| Type | `type` | Magenta |
| Class | `class` | Magenta |
| Interface | `interface` | Magenta |
| Enum | `enum` | Cyan |
| Trait | `trait` | Yellow |
| Module | `module` | Yellow |

## Examples

```bash
# List all types in a Rust project
hug types "src/**/*.rs"

# List functions in TypeScript files, excluding tests
hug functions "src/**/*.ts" --ignore "**/test/**"

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
```

## Classes Command

The `classes` command provides structured class inspection:

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
