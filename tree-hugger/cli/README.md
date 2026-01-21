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
```
