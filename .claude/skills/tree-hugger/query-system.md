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

; Import captures: only the token the import binds (never the statement,
; a qualifier or an aliased original)
(use_as_clause alias: (identifier) @local.definition.import)
```

**Capture the binding field, never "any identifier child".** A pattern such
as `(parameter (identifier) @local.definition.parameter)` also matches a
parameter's type (`W x` in C#) and a bare default (`x=external` in Python),
inventing a declaration whose span then hides the real read. Use the
grammar's name field (`default_parameter name: (identifier)`), an anchor
(`(const_element . (name))`), or list the parent kinds whose identifier
children all bind (Rust `let_condition`). `drop_non_declarations`'
`is_false_capture` only runs for Rust and TS/JS, so a limited language has
no second line of defense. `symbols_occurrences::PARAMETER_CAPTURE_MATRIX`
holds one row per site (declarations and the reads they must keep).

### Capture to SymbolKind Mapping

| Capture | SymbolKind |
|---------|------------|
| `@local.definition.function` | Function |
| `@local.definition.method` | Method |
| `@local.definition.constructor` | Constructor |
| `@local.definition.type` | Type |
| `@local.definition.type_alias` | TypeAlias |
| `@local.definition.class` | Class |
| `@local.definition.interface` | Interface |
| `@local.definition.enum` | Enum |
| `@local.definition.trait` | Trait |
| `@local.definition.module` | Module |
| `@local.definition.namespace` | Namespace |
| `@local.definition.field` | Field |
| `@local.definition.property` | Property |
| `@local.definition.enum_variant` | EnumVariant |
| `@local.definition.var` (or bare `@local.definition`) | Variable |
| `@local.definition.parameter` | Parameter |
| `@local.definition.macro` | Macro |
| `@local.definition.constant` | Constant |
| `@local.definition.import` | (imports) |

The capture is only the starting kind. `refine_symbol_kind`
(`file/extract/classify.rs`) decides the final kind from the declaration node:
node-kind rules (TS `constructor` → Constructor, class fields → Property,
`type_alias_declaration` / Rust `type_item` → TypeAlias, Rust `enum_variant` →
EnumVariant, `mod_item` → Module, `static_item` → Variable; Python `__init__`
and PHP `__construct` methods and C++ in-class constructors → Constructor;
Scala type-body `val`/`var` → Field) outrank the capture;
a function declared directly in a class/struct/enum/trait/interface/protocol/
object/`impl`/extension body becomes Method; a `var` capture becomes Parameter,
Field or Constant only from the path between the name and its own declaration
(a Swift `let` in a function body stays Variable).
A Python chained assignment (`first = second = 0`) is one declaration, the
outermost `assignment`; a later target, captured only as `var`, becomes a
Field when that statement is directly in a class body.
A Scala grouped `val a, b: Int = 1` puts its names under one
`pattern: (identifiers …)` and a grouped declaration `val a, b: Int` repeats
the `name` field, so the vendored query captures each `identifier` under
`identifiers` and `extract_scala_body_fields` reads every name
(`children_by_field_name`), giving each list entry the shared type; a tuple
pattern (`val (a, b) = …`) is never captured. Any reader of a declaration
that can carry several names must enumerate them, never take the first.

JS/TS quoted member names capture the `string_fragment` inside the `string`,
so `"label": string` is reported as `label`.

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
- The record's `declaration_span`

Without a context capture, the declaration is found by walking up from the
name (`resolve_declaration_node`, `file/extract/declaration.rs`) to the
nearest declaration-like node, never past a body or block. Visibility, static
flags, signatures, `declaration_text` and the container search all read that
declaration node, so a capture does not need `.context` to get them. A C/C++
`function_declarator` context is widened to its enclosing definition or
declaration, where storage class and access sections live.

C/C++ names can sit under any chain of declarators, which a query cannot
follow, so those queries capture a name by its **immediate parent**
(`attributed_declarator`, `parenthesized_declarator`, `pointer_declarator`,
or `function_declarator` for a C++ member function) and leave the rest to
Rust: `classify_symbol_kind` turns a captured variable or field into a
function (a method in a type body) when `declared_function_named`
(`file/extract/c_declarators.rs`) says its declarator declares one, and
`implied_c_function_context` supplies the context such a pattern could not
capture (the member's declaration, or a free function's own
`function_declarator`). Add a C/C++ capture the same way, not as a fixed
outer shape like `(field_declaration declarator: (function_declarator …))`.

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

Severities are mapped by `severity_for_rule()` in `queries/mod.rs`:

```rust
pub fn severity_for_rule(rule_id: &str) -> DiagnosticSeverity {
    match rule_id {
        // Error-level (semantic)
        "unreachable-code" | "invalid-syntax" | "undefined-variable"
        | "undefined-symbol" => DiagnosticSeverity::Error,
        // Warning-level (semantic)
        "unused-variable" | "shadowed-variable" | "unused-symbol"
        | "unused-import" | "dead-code" | "undefined-module" => DiagnosticSeverity::Warning,
        // Warning-level (pattern)
        "unwrap-call" | "expect-call" | "dbg-macro" | "eval-call"
        | "exec-call" | "debugger-statement" | "breakpoint-call"
        | "deprecated-syntax" => DiagnosticSeverity::Warning,
        _ => DiagnosticSeverity::Info,
    }
}
```

Human-readable messages are generated by `format_rule_message()` in the same module.

## References Query

Three capture names, one query run (`TreeFile::reference_captures`, one
capture per node span):

- `@reference`: identifier usages. Reference occurrences read them, and so
  does `references_from` (diagnostics and `referenced_symbols()`).
- `@reference.member` (Rust, TypeScript, JavaScript, Go, C, C++, Perl):
  method-call and `new` member names (`x.f()`, `obj.run()`, `new ns.W()`,
  Go `pkg.F()`, C `s.cb()`, C++ `obj.template m<T>()`, Perl `$x->m()`) that
  no `@reference` pattern captures. A member callee behind wrappers
  (`((obj.m))()`, `(obj.m as T)()`) cannot be matched at every depth by a
  pattern; `reference_captures` adds those from
  `occurrences::wrapped_member_callees` (TS/JS/Go/C/C++), which peels the
  same wrapper set `is_call_target` uses (`wrapper_operand`). Only reference occurrences read them, so adding one never
  changes diagnostics; it can change occurrences and therefore the edges and
  `is_recursive` derived from them (`symbols_occurrences`).
- `@reference.pattern` (Rust): struct-pattern field shorthands
  (`S { x }`), which bind a new name or read a constant in view; reference
  occurrences decide which (`bare_pattern_meaning`). Like
  `@reference.member`, only reference occurrences read them.

```scheme
; Rust
(identifier) @reference
(type_identifier) @reference
(call_expression
  function: (field_expression
    field: (field_identifier) @reference.member))

; TypeScript / JavaScript
(identifier) @reference
(call_expression
  function: (member_expression
    property: [(property_identifier) (private_property_identifier)] @reference.member))
```

`references.scm` files are Tree Hugger originals, not vendored, so editing
them needs no provenance note.

Whether a captured name is a `Call` is decided after the query, by
`analysis/occurrences.rs::is_call_target`, never by which pattern matched:
it looks through transparent wrappers (`transparent_wrapper`: parentheses
except PHP's, Swift one-element tuples, TS `as`/`satisfies`/`<T>x`/`x!`),
generic instantiations (`generic_instantiation`) and qualified paths, and
reads Perl and shell invocations separately. `symbols_occurrences::INVOCATION_MATRIX`
(one row per language, negative controls included) is the table to extend.

A vendored C++ `locals.scm` change: `template_function`/`template_method`
names are definitions only inside a `function_declarator` (explicit
specializations); upstream captured every use (`f<int>()`) as a function.

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

// Get a query (cached). The query text comes from `grammar.language`'s query
// files but is compiled against `grammar.grammar` — the exact grammar that
// parsed the tree — so TSX trees use a TSX-compiled query, not a plain
// TypeScript one.
let query = query_for(GrammarRef { language, grammar: &grammar, id }, QueryKind::Locals)?;
```

The cache is:
- Thread-safe via `Mutex`
- Lazily initialized per grammar variant (`GrammarRef::id`) and kind, so the
  TypeScript and TSX grammars are compiled and cached independently
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

5. **Register file extensions** in `ProgrammingLanguage::extensions()` (auto-detected via `from_extension()`)

6. **Add builtins** in `builtins.rs`

7. **Write tests**:
   - Add fixture files: `lib/tests/fixtures/sample.<ext>`, `types.<ext>`
   - Add test in `lib/tests/l1/tree_file.rs`

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
