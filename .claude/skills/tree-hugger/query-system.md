# Tree Hugger Query System

Tree Hugger uses Tree-sitter queries for symbol extraction and diagnostics. Understanding this system is essential for adding language support or fixing extraction issues.

## Query Organization

```
tree-hugger/lib/queries/
├── vendor/           # Vendored from nvim-treesitter
│   ├── rust/
│   │   └── locals.scm
│   ├── typescript/
│   │   └── locals.scm
│   └── <lang>/
│       └── locals.scm
├── rust/
│   ├── lint.scm
│   ├── references.scm
│   └── comments.scm
├── typescript/
│   ├── lint.scm
│   ├── references.scm
│   └── comments.scm
└── <lang>/
    ├── lint.scm
    ├── references.scm
    └── comments.scm
```

## Query Types

| Query | Location | Purpose |
|-------|----------|---------|
| `locals.scm` | `vendor/<lang>/` | Symbol definitions (from nvim-treesitter) |
| `lint.scm` | `<lang>/` | Pattern-based lint rules |
| `references.scm` | `<lang>/` | Identifier usage tracking |
| `comments.scm` | `<lang>/` | Comment node detection |

## Vendor Locals Query

The `locals.scm` queries follow nvim-treesitter conventions for capturing symbol definitions.

### Capture Naming Convention

```scheme
; Symbol definitions use @local.definition.<kind>
(function_item name: (identifier) @local.definition.function)
(struct_item name: (type_identifier) @local.definition.type)
(enum_item name: (type_identifier) @local.definition.enum)
(trait_item name: (type_identifier) @local.definition.trait)

; Import captures
(use_declaration) @local.definition.import
```

### Capture to SymbolKind Mapping

| Capture | SymbolKind |
|---------|------------|
| `@local.definition.function` | Function |
| `@local.definition.method` | Method |
| `@local.definition.type` | Type |
| `@local.definition.class` | Class |
| `@local.definition.interface` | Interface |
| `@local.definition.enum` | Enum |
| `@local.definition.trait` | Trait |
| `@local.definition.module` | Module |
| `@local.definition.namespace` | Namespace |
| `@local.definition.field` | Field |
| `@local.definition.variable` | Variable |
| `@local.definition.parameter` | Parameter |
| `@local.definition.macro` | Macro |
| `@local.definition.constant` | Constant |
| `@local.definition.import` | (imports) |

### Context Captures

Use `.context` suffix to capture the full node for metadata extraction:

```scheme
; Capture function name AND full function for signature extraction
(function_item
  name: (identifier) @local.definition.function
) @local.definition.function.context
```

The `.context` capture provides the full AST node for extracting:
- Function signatures (parameters, return type)
- Type metadata (fields, variants, generics)
- Doc comments
- Visibility modifiers

### Query Inheritance

Queries can inherit from other languages using the `inherits:` directive:

```scheme
;; inherits: ecmascript

; TypeScript-specific additions
(interface_declaration
  name: (type_identifier) @local.definition.interface
)
```

## Lint Query

Pattern-based lint rules use `@diagnostic.<rule-id>` captures.

### Example Lint Rules

```scheme
; Rust: unwrap() call detection
(call_expression
  function: (field_expression
    field: (field_identifier) @_method)
  (#eq? @_method "unwrap")
) @diagnostic.unwrap-call

; Rust: dbg!() macro detection
(macro_invocation
  macro: (identifier) @_macro
  (#eq? @_macro "dbg")
) @diagnostic.dbg-macro

; JavaScript: debugger statement
(debugger_statement) @diagnostic.debugger-statement

; JavaScript: eval() call
(call_expression
  function: (identifier) @_fn
  (#eq? @_fn "eval")
) @diagnostic.eval-call
```

### Built-in Lint Rules

| Rule | Languages | Description |
|------|-----------|-------------|
| `unwrap-call` | Rust | `.unwrap()` calls |
| `expect-call` | Rust | `.expect()` calls |
| `dbg-macro` | Rust | `dbg!()` usage |
| `debugger-statement` | JS/TS | `debugger;` |
| `eval-call` | JS/TS/Python/PHP | `eval()` usage |
| `exec-call` | Python | `exec()` usage |
| `breakpoint-call` | Python | `breakpoint()` usage |

### Severity Mapping

Default severities are defined in `queries/mod.rs`:

```rust
fn default_severity(rule: &str) -> DiagnosticSeverity {
    match rule {
        "unwrap-call" | "expect-call" => DiagnosticSeverity::Warning,
        "debugger-statement" | "dbg-macro" => DiagnosticSeverity::Warning,
        "eval-call" | "exec-call" => DiagnosticSeverity::Warning,
        _ => DiagnosticSeverity::Warning,
    }
}
```

## References Query

Captures all identifier references for semantic analysis.

```scheme
; Rust references
(identifier) @reference
(type_identifier) @reference
(field_identifier) @reference

; JavaScript references
(identifier) @reference
(property_identifier) @reference
```

## Comments Query

Simple capture for comment nodes (used by ignore directive parser).

```scheme
[(line_comment) (block_comment)] @comment
```

## Query Caching

Queries are cached globally for performance:

```rust
// Internal: queries/mod.rs
static QUERY_CACHE: OnceLock<Mutex<QueryCache>> = OnceLock::new();

// Get a query (cached)
let query = query_for(language, QueryKind::Locals)?;
```

The cache is:
- Thread-safe via `Mutex`
- Lazily initialized per language/kind
- Never invalidated (assumes queries are static)

## Adding a New Language

1. **Vendor the locals.scm**:
   ```bash
   # Copy from nvim-treesitter
   cp path/to/nvim-treesitter/queries/<lang>/locals.scm \
      tree-hugger/lib/queries/vendor/<lang>/locals.scm
   ```

2. **Create custom queries**:
   ```bash
   mkdir tree-hugger/lib/queries/<lang>
   touch tree-hugger/lib/queries/<lang>/lint.scm
   touch tree-hugger/lib/queries/<lang>/references.scm
   touch tree-hugger/lib/queries/<lang>/comments.scm
   ```

3. **Add language to ProgrammingLanguage enum** in `shared/symbol.rs`

4. **Add tree-sitter dependency** to `Cargo.toml`

5. **Implement language detection** in `ProgrammingLanguage::from_path()`

6. **Add builtins** in `builtins.rs`

7. **Write tests**:
   - Add fixture files: `lib/tests/fixtures/sample.<ext>`, `types.<ext>`
   - Add test in `lib/tests/tree_file.rs`

## Debugging Queries

### Test a query manually

```rust
use tree_sitter::Query;
use tree_sitter_rust::language;

let query = Query::new(&language(), r#"
(function_item name: (identifier) @fn)
"#)?;

// Check capture names
for (i, name) in query.capture_names().iter().enumerate() {
    println!("Capture {}: {}", i, name);
}
```

### Common issues

1. **Query syntax error**: Check s-expression syntax, matching parens
2. **No matches**: Verify node names match tree-sitter grammar
3. **Wrong captures**: Use `tree-sitter parse <file>` to see AST structure
4. **Performance**: Avoid overly broad patterns, use predicates

### View AST structure

```bash
# Install tree-sitter CLI
cargo install tree-sitter-cli

# Parse a file to see its AST
tree-sitter parse src/main.rs
```
