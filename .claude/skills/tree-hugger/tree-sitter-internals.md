# Tree-sitter Runtime Internals

Tree Hugger is built on the **tree-sitter Rust runtime** (`tree-sitter = "0.26.3"`) plus
one grammar crate per supported language. This document covers the underlying runtime
mechanics that surface when you add a grammar, bump a dependency, or debug why a query
returns nothing.

For Tree Hugger's *own* query conventions (`locals.scm`, `lint.scm`, capture naming, the
`QueryCache`), see [Query System](./query-system.md). This document is the layer *below*
that — the raw tree-sitter API and its sharp edges.

## The runtime model

Parsing flows through four types:

| Type | Role |
|------|------|
| `Parser` | Stateful parser; you set its `Language`, then call `parse` |
| `Language` | A grammar handle, obtained from a `tree-sitter-<lang>` crate |
| `Tree` | An immutable concrete syntax tree (CST) produced by `parse` |
| `Node` | A borrowed cursor into the `Tree` (byte ranges, kinds, children) |

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
// NOTE: pass a *reference* to the `.into()` Language — see gotcha below.
parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

let src = "fn add(a: i32, b: i32) -> i32 { a + b }";
let tree = parser.parse(src, None).unwrap();   // None = no prior tree (full parse)
let root = tree.root_node();

println!("{}", root.to_sexp());   // (source_file (function_item ...))
```

Tree Hugger wraps this per file in `TreeFile`: one `parse` call, no reuse of a prior
tree. There is no incremental parsing in the symbol-extraction path (see
[Incremental parsing](#incremental-parsing-not-used-by-tree-hugger) for when that changes).

### Nodes are anchored to byte offsets

Every `Node` carries a byte range into the original source. Extract text by slicing:

```rust
let snippet = &src[node.byte_range()];                 // original text
let (start, end) = (node.start_position(), node.end_position());  // row/col for diagnostics
```

`start_position()` / `end_position()` return `Point { row, column }` in **UTF-8 byte
columns**, which is what Tree Hugger's `CodeRange` is built from. Use byte ranges for text,
positions for human-facing diagnostics.

### Manual traversal vs queries

When you don't yet know the right query — or are reverse-engineering a grammar — walk the
tree directly:

```rust
fn walk(node: tree_sitter::Node, depth: usize) {
    println!("{:indent$}{}", "", node.kind(), indent = depth * 2);
    for i in 0..node.child_count() {
        walk(node.child(i).unwrap(), depth + 1);
    }
}
```

In production code Tree Hugger uses queries (compiled `.scm` files), not hand-rolled
walks — queries are declarative, cached, and far easier to maintain across 16 grammars.

## Query mechanics

Queries match syntax patterns without manual traversal. The runtime pieces:

- `Query::new(&language, source)` — compiles an s-expression pattern. **Expensive.**
  Tree Hugger compiles each query once and stores it in a global `OnceLock<QueryCache>`
  (see [Query System](./query-system.md)); never compile per-file in a hot path.
- `QueryCursor::new()` — the stateful matcher you run a compiled `Query` against.
- `cursor.matches(&query, node, src.as_bytes())` — yields matches; each match exposes
  `captures` with the capturing `Node` and its capture index.

```rust
use tree_sitter::{Query, QueryCursor};

let lang = tree_sitter_rust::LANGUAGE.into();
let query = Query::new(&lang, "(function_item name: (identifier) @name)")?;
let mut cursor = QueryCursor::new();

for m in cursor.matches(&query, tree.root_node(), src.as_bytes()) {
    for cap in m.captures {
        println!("function: {}", &src[cap.node.byte_range()]);
    }
}
```

### Predicates

Queries support predicates such as `#eq?`, `#match?`, and `#any-of?` to constrain
captures — this is how Tree Hugger's `lint.scm` rules pin a call to a specific method
name:

```scheme
(call_expression
  function: (field_expression field: (field_identifier) @_method)
  (#eq? @_method "unwrap")) @diagnostic.unwrap-call
```

The runtime evaluates `#eq?`/`#match?` itself. Less common predicates may be exposed as
*general predicates* the host must evaluate — Tree Hugger sticks to the runtime-handled
set to keep query evaluation self-contained.

### Capture-iteration changed across runtime versions

Older tree-sitter exposed `matches`/`captures` as plain iterators; newer versions
(including the 0.26 line Tree Hugger uses) return **streaming iterators**. Two
consequences:

1. Don't assume heavy `Iterator` adapter chains compile — collect into a `Vec` first if
   you need random access or to outlive the cursor.
2. Code copied from older tree-sitter tutorials may not compile verbatim. When a snippet
   fights the borrow checker around `cursor.matches(...)`, suspect this.

## Debugging an AST

The fastest way to author or fix a query is to look at the actual tree:

```rust
// In a test or scratch binary:
println!("{}", tree.root_node().to_sexp());

// Inspect a compiled query's captures:
for (i, name) in query.capture_names().iter().enumerate() {
    println!("capture {i}: {name}");
}
```

Or use the tree-sitter CLI against a real file — invaluable when node/field names differ
from what you assumed:

```bash
cargo install tree-sitter-cli
tree-sitter parse src/main.rs        # prints the full CST with node kinds + ranges
```

Common failure modes:

| Symptom | Likely cause |
|---------|--------------|
| Query compiles, **zero matches** | Node kind or field name differs from the grammar — confirm with `to_sexp()` / `tree-sitter parse` |
| `QueryError` at compile time | Malformed s-expression (unbalanced parens) or a capture/predicate the grammar doesn't expose |
| Captures land on the wrong node | Pattern matched a broader/narrower node than intended; add field constraints |
| Slow parsing/matching | Recompiling `Query` per file, or an unconstrained pattern — narrow with fields and predicates |

## Version & ABI compatibility

This is the gotcha most likely to bite when touching `Cargo.toml`. Tree Hugger pins the
runtime and 16 grammar crates at **independent, non-matching versions**:

| Crate | Version | Crate | Version |
|-------|---------|-------|---------|
| `tree-sitter` (runtime) | 0.26.3 | `tree-sitter-perl` | 1.1.2 |
| `tree-sitter-bash` | 0.25.1 | `tree-sitter-php` | 0.24.2 |
| `tree-sitter-c` | 0.24.1 | `tree-sitter-python` | 0.25.0 |
| `tree-sitter-c-sharp` | 0.23.1 | `tree-sitter-rust` | 0.24.0 |
| `tree-sitter-cpp` | 0.23.4 | `tree-sitter-scala` | 0.24.0 |
| `tree-sitter-go` | 0.25.0 | `tree-sitter-swift` | 0.7.1 |
| `tree-sitter-java` | 0.23.5 | `tree-sitter-typescript` | 0.23.2 |
| `tree-sitter-javascript` | 0.25.0 | `tree-sitter-zsh` | 0.56.0 |
| `tree-sitter-lua` | 0.5.0 | | |

**Grammar crate versions do not — and should not — track the runtime version.** Each
grammar carries its own semver; what actually has to line up is the **ABI** the grammar
was generated against versus the ABI the runtime supports. A given runtime supports a
*range* of grammar ABIs, which is why a 0.26 runtime happily loads grammars spanning
`tree-sitter-c-sharp` 0.23 through `tree-sitter-zsh` 0.56.

So the generic advice "align the runtime and grammar versions" is **wrong for this
codebase** — don't bump a grammar to chase the runtime's version number. The only
reliable compatibility test is empirical:

```bash
cargo build -p tree-hugger-lib && cargo test -p tree-hugger-lib
```

Symptoms of an ABI mismatch:

- `set_language` returns a `LanguageError` ("Incompatible language version" / version out
  of the supported range) at runtime.
- A grammar that compiled fine suddenly fails to load after a runtime bump.

When bumping a grammar (e.g. to add a language construct or pick up a parser fix):

1. Change only that one grammar crate's version.
2. `cargo build -p tree-hugger-lib` — a clean build means the ABI is accepted.
3. `cargo test -p tree-hugger-lib` — grammars occasionally rename node kinds between
   versions, which silently breaks `.scm` queries even when the ABI is fine. The type
   distinction and `types.*` fixture tests are your guardrail here.
4. If queries broke, re-run `tree-sitter parse` on a fixture to find the renamed node and
   update the affected `.scm`.

When bumping the **runtime** (`tree-sitter` itself), expect to verify *all* grammars,
since it shifts the supported ABI range for every one of them.

### Gotcha: pass a reference to `LANGUAGE.into()`

The single most common compile error wiring up a grammar:

```rust
parser.set_language(tree_sitter_rust::language())?;        // old/wrong shape
parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;  // correct: &(LanguageRef.into())
```

Modern grammar crates export a `LANGUAGE` constant (a `LanguageFn`), not a `language()`
function. Convert with `.into()` and pass it **by reference**.

## Incremental parsing (not used by Tree Hugger)

Tree Hugger parses each file once, top to bottom — there is no edit loop, so incremental
parsing is not on the symbol-extraction path. This section is for awareness if Tree Hugger
ever grows an LSP/watch mode.

Incremental parsing reuses a prior `Tree` to avoid a full re-parse after an edit:

```rust
use tree_sitter::{InputEdit, Point};

let mut tree = parser.parse(old_src, None).unwrap();
tree.edit(&InputEdit {
    start_byte, old_end_byte, new_end_byte,
    start_position, old_end_position, new_end_position,
});
let new_tree = parser.parse(new_src, Some(&tree)).unwrap();  // reuses unchanged subtrees
```

The dominant failure mode is **wrong `InputEdit` math** — byte offsets and row/column
points must describe the edit *exactly*, and multi-byte UTF-8 makes off-by-one errors
easy. A real implementation should maintain a rope/line index to map byte offset ↔
(row, col) rather than rescanning the string per edit.

## Ecosystem crates Tree Hugger deliberately does *not* use

The tree-sitter ecosystem ships higher-level helpers. Tree Hugger rolls its own
equivalents (vendored `locals.scm` + custom queries) for tighter control, but it's worth
knowing they exist:

- **`tree-sitter-highlight`** — turns a tree + `highlights.scm` into styled token events
  with precedence handling. Tree Hugger does symbol extraction, not syntax highlighting,
  so it doesn't pull this in.
- **`tree-sitter-tags`** — standardized "tags" queries for definitions/references
  (ctags-style navigation). Tree Hugger's `locals.scm` + `references.scm` cover this with
  richer per-language control.
- **WASM grammars** — load grammars as runtime WASM blobs (needs the runtime `wasm`
  feature + `wasmtime`). Tree Hugger statically links native grammar crates instead, so
  there's no runtime grammar loading and no `wasmtime` build complexity.

## Why tree-sitter (and when it isn't enough)

Tree Hugger uses tree-sitter because it is a mature, multi-language, **error-tolerant**
parsing substrate with one common API and a query system — it returns a usable (partial)
tree even on syntactically broken input, which is exactly what a linter needs.

Its limits are worth remembering when scoping a feature:

- It produces a **concrete syntax tree, not semantics.** Scope resolution, type inference,
  and cross-file binding are layers Tree Hugger builds *on top* (the schema-v2
  parse→bind→semantic pipeline), not things tree-sitter gives you.
- Grammar quality varies. Some grammars under-distinguish constructs — e.g. Swift exposes
  all types as one node kind, and Go does not separate struct from interface — which is
  why those land on `SymbolKind::Type` (see the SKILL's Known Limitations). These are
  grammar limitations, not bugs in Tree Hugger's extraction.
