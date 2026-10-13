---
name: tree-sitter
description: Comprehensive guide to Tree-sitter - incremental parsing library, query system, and practical implementation in Rust and TypeScript
created: 2025-12-20
last_updated: 2025-12-20T00:00:00Z
hash: ecb0d86ae465b3a3
tags:
  - tree-sitter
  - parsing
  - ast
  - rust
  - typescript
  - query-system
---

# Tree-sitter: Comprehensive Deep Dive

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Common Use Cases](#common-use-cases)
4. [Getting Started with TypeScript](#getting-started-with-typescript)
5. [Getting Started with Rust](#getting-started-with-rust)
6. [The Query System](#the-query-system)
7. [Advanced Query Techniques](#advanced-query-techniques)
8. [Multi-Language Support](#multi-language-support)
9. [Production-Ready Patterns](#production-ready-patterns)
10. [Schema Files (.scm)](#schema-files-scm)
11. [Popular Grammars](#popular-grammars)
12. [Performance Considerations](#performance-considerations)
13. [Quick Reference](#quick-reference)

## Introduction

**Tree-sitter** is a parser generator tool and incremental parsing library that builds and efficiently updates concrete syntax trees for source files. Originally developed for GitHub's Atom editor, it has evolved into a robust, language-agnostic parsing solution used across the development tools ecosystem.

### Key Characteristics

- **Language-Agnostic**: General enough to parse any programming language
- **Performance**: Fast enough to parse on every keystroke in a text editor
- **Robust**: Provides useful results even with syntax errors
- **Dependency-Free**: Written in pure C11 with no external dependencies
- **Cross-Platform**: Generates parsers that compile to native libraries or WebAssembly modules
- **Incremental**: Efficiently updates syntax trees when source code changes without reprocessing entire files

The incremental parsing capability is critical for real-time applications like code editors, allowing Tree-sitter to update only the affected portions of the syntax tree when code changes.

## Core Concepts

### Architecture Components

Tree-sitter's architecture consists of several key components that work together:

- **Parser**: The core engine that transforms source code into syntax trees
- **Language Grammar**: Language-specific parsing rules (one per language)
- **Syntax Tree**: The hierarchical representation of parsed code
- **Query System**: A DSL for pattern matching against syntax trees
- **Cursor**: An efficient iterator for traversing trees and executing queries

### Trees vs Cursors

- **Tree (`Tree`)**: The complete hierarchical representation of source code. Immutable once created.
- **TreeCursor**: A lightweight, stateful iterator for traversing the tree without allocating new objects
- **QueryCursor**: A specialized cursor optimized for executing queries against trees

### Nodes and Fields

Nodes represent syntactic elements in your code. Many nodes have **named fields** that make queries more precise and resilient to grammar changes:

```scheme
; Using fields (recommended - resilient to child order changes)
(function_item
  name: (identifier) @func_name
  parameters: (parameters) @params)

; Without fields (fragile - depends on child order)
(function_item
  (identifier) @func_name
  (parameters) @params)
```

## Common Use Cases

Tree-sitter's versatility makes it valuable across various development tools and applications:

### Syntax Highlighting

Tree-sitter provides built-in support for syntax highlighting through the `tree-sitter-highlight` library, used by GitHub.com for highlighting code. The highlighting system uses tree queries to pattern-match against syntax trees, enabling precise and context-aware highlighting.

The system is controlled by three types of query files:

- **highlights.scm**: Defines basic syntax highlighting patterns
- **locals.scm**: Identifies local variable definitions and references
- **injections.scm**: Manages language injection (e.g., CSS within HTML, SQL in Python strings)

### Code Analysis and Navigation

Development tools use Tree-sitter for:

- **Code structure analysis**: Identifying functions, classes, and other code constructs
- **Symbol navigation**: Jumping to definitions and references
- **Code intelligence**: Providing contextual information and suggestions
- **Semantic search**: Finding code patterns beyond simple text matching

### Code Refactoring

Tree-sitter enables automated code transformations by:

- Precisely identifying code structures
- Safely modifying code while preserving syntax correctness
- Handling complex transformations across multiple languages
- Understanding context (distinguishing `x` in `let x = 1` from `"x"` in a string)

### Language Server Protocol (LSP) Implementation

Many language servers use Tree-sitter as their parsing backend to provide:

- **Precise code diagnostics** without requiring full compilation
- **Real-time error detection** as code is edited
- **Efficient semantic analysis** for IDE features
- **Fast response times** due to incremental parsing

### Custom Tooling

Tree-sitter is used for building:

- **Linters and static analysis tools** (detecting code smells, security vulnerabilities)
- **Code formatting utilities** (understanding structure for intelligent formatting)
- **Documentation generators** (extracting doc comments and function signatures)
- **Automated code transformation tools** (refactoring, migration scripts)

## Getting Started with TypeScript

### Installation and Setup

To use Tree-sitter in a TypeScript/Node.js environment:

```bash
# For Node.js projects
npm install tree-sitter tree-sitter-javascript

# For web projects (WebAssembly)
npm install web-tree-sitter
```

### Basic Usage (Node.js)

```typescript
import Parser from 'tree-sitter';
import JavaScript from 'tree-sitter-javascript';

// Create a parser instance
const parser = new Parser();
parser.setLanguage(JavaScript);

// Parse source code
const sourceCode = 'let x = 1; console.log(x);';
const tree = parser.parse(sourceCode);

// Inspect the syntax tree
console.log(tree.rootNode.toString());

// Access specific nodes
const callExpression = tree.rootNode.child(1)?.firstChild;
console.log(callExpression);
```

### Basic Usage (Web/WebAssembly)

```typescript
import { Parser } from 'web-tree-sitter';

// Initialize the library (required for WebAssembly)
await Parser.init();

// Load language grammar
const JavaScript = await Parser.Language.load('path/to/tree-sitter-javascript.wasm');

// Create parser and use it
const parser = new Parser();
parser.setLanguage(JavaScript);

const sourceCode = 'let x = 1; console.log(x);';
const tree = parser.parse(sourceCode);
console.log(tree.rootNode.toString());
```

### Incremental Editing

Tree-sitter's strength lies in efficiently updating syntax trees when code changes:

```typescript
// Original code
const sourceCode = 'let x = 1; console.log(x);';
let tree = parser.parse(sourceCode);

// Edit the code (replace 'let' with 'const')
const newSourceCode = 'const x = 1; console.log(x);';
tree.edit({
  startIndex: 0,
  oldEndIndex: 3,
  newEndIndex: 5,
  startPosition: {row: 0, column: 0},
  oldEndPosition: {row: 0, column: 3},
  newEndPosition: {row: 0, column: 5},
});

// Parse with the old tree for efficient updates
const newTree = parser.parse(newSourceCode, tree);
```

### WebAssembly Deployment

When deploying Tree-sitter in web environments:

```json
// package.json
{
  "scripts": {
    "postinstall": "cp node_modules/web-tree-sitter/tree-sitter.wasm public/"
  }
}
```

Ensure your server provides the `tree-sitter.wasm` file and configure your build system (e.g., Vite, Webpack) to handle WASM files correctly.

## Getting Started with Rust

### Installation and Setup

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
# Add other language grammars as needed
```

### Basic Usage

```rust
use tree_sitter::{Language, Parser};

extern "C" { fn tree_sitter_rust() -> Language; }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize parser
    let language = unsafe { tree_sitter_rust() };
    let mut parser = Parser::new();
    parser.set_language(language)?;

    // Parse code
    let source_code = "fn main() { println!(\"Hello\"); }";
    let tree = parser.parse(source_code, None)
        .ok_or("Failed to parse code")?;

    // Print the syntax tree (S-expression format)
    println!("{}", tree.root_node().to_sexp());

    Ok(())
}
```

### Incremental Parsing

```rust
use tree_sitter::{InputEdit, Point};

fn parse_with_updates(parser: &mut Parser, initial_code: &str) {
    // Initial parse
    let mut tree = parser.parse(initial_code, None).unwrap();
    println!("Initial tree: {}", tree.root_node().to_sexp());

    // Simulate an edit (replace "let" with "const")
    let new_code = "const x = 1; console.log(x);";
    let edit = InputEdit {
        start_byte: 0,
        old_end_byte: 3,
        new_end_byte: 5,
        start_position: Point { row: 0, column: 0 },
        old_end_position: Point { row: 0, column: 3 },
        new_end_position: Point { row: 0, column: 5 },
    };

    // Apply edit and reparse
    tree.edit(&edit);
    let new_tree = parser.parse(new_code, Some(&tree)).unwrap();
    println!("Updated tree: {}", new_tree.root_node().to_sexp());
}
```

### Building Custom Grammars

To create your own grammar for Tree-sitter:

```bash
# Install the CLI
npm install -g tree-sitter-cli

# Initialize a new grammar project
tree-sitter init-config

# Generate the parser
tree-sitter generate

# Build the grammar
tree-sitter build

# Test the grammar
tree-sitter test
```

## The Query System

Tree-sitter's **Query System** is a domain-specific language (DSL) for structural pattern matching on Abstract Syntax Trees. Think of it as **Regular Expressions for code structure**, where instead of matching characters, you match nodes, fields, and relationships.

### How Queries Work

Queries use a Lisp-like S-expression syntax. They define "patterns" that are matched against the nodes produced by the parser.

#### Core Components

- **Nodes**: Represented by parentheses `(node_type)`
- **Captures**: Indicated by `@name`. This tags a matched node for extraction
- **Fields**: Named children (like `name:` or `body:`) for more precise queries
- **Predicates**: Prefixed with `#`, these allow additional logic (regex matching, equality checks)

### Query Syntax Reference

| Syntax | Meaning | Example |
|--------|---------|---------|
| `(type)` | Match a node of a specific type | `(identifier)` |
| `field: (type)` | Match a child node with a specific field name | `name: (identifier)` |
| `(_)` | Wildcard: matches any named node | `(_)` |
| `[a b]` | Alternation: matches either a OR b | `[(identifier) (string)]` |
| `.` | Anchor: ensures nodes are immediate siblings | `(comment) . (function)` |
| `*` | Zero or more quantifier | `(parameter)*` |
| `+` | One or more quantifier | `(statement)+` |
| `?` | Optional (zero or one) quantifier | `(visibility)?` |

### Basic Query Examples

#### Finding Function Calls

```scheme
(call_expression
  function: (identifier) @function.name
  arguments: (argument_list) @args)
```

#### Filtering by Name (Predicates)

```scheme
; Find calls to a specific function
(
  (call_expression
    function: (identifier) @func_name)
  (#eq? @func_name "fetch_data")
)

; Find uppercase constants (regex)
(
  (identifier) @constant
  (#match? @constant "^[A-Z_]+$")
)
```

### TypeScript Query Implementation

```typescript
import Parser from 'tree-sitter';
import JavaScript from 'tree-sitter-javascript';

const parser = new Parser();
parser.setLanguage(JavaScript);

const sourceCode = `
  function greet(name) {
    console.log("Hello, " + name);
  }
  greet("World");
`;

const tree = parser.parse(sourceCode);

// Define the query
const queryString = `
  (call_expression
    function: [
      (identifier) @function.name
      (member_expression
        object: (identifier) @obj
        property: (property_identifier) @prop)
    ]
  ) @call
`;

const query = new Parser.Query(JavaScript, queryString);

// Execute the query
const captures = query.captures(tree.rootNode);

// Process results
captures.forEach(({ name, node }) => {
  console.log(`Capture: ${name}, Text: "${node.text}"`);
});
```

### Rust Query Implementation

```rust
use tree_sitter::{Parser, Query, QueryCursor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_code = "fn main() { println!(\"Hello World\"); }";

    // Initialize Parser
    let mut parser = Parser::new();
    let language = tree_sitter_rust::language();
    parser.set_language(language)?;

    // Parse Code
    let tree = parser.parse(source_code, None).unwrap();
    let root_node = tree.root_node();

    // Define Query - find macro calls
    let query_str = r#"
        (macro_invocation
            macro: (identifier) @macro_name
            !: (token_tree) @content)
    "#;
    let query = Query::new(language, query_str)?;

    // Execute Query
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, root_node, source_code.as_bytes());

    for m in matches {
        for capture in m.captures {
            let node = capture.node;
            let text = &source_code[node.byte_range()];

            println!("Captured @{}: {}",
                query.capture_names()[capture.index as usize],
                text
            );
        }
    }

    Ok(())
}
```

### Common Query Patterns

#### Finding Exported Functions (TypeScript/JavaScript)

```scheme
(export_statement
  declaration: (function_declaration
    name: (identifier) @name
    parameters: (formal_parameters) @params)) @exported_func
```

#### Finding Security Issues (Any Language)

```scheme
; Find eval() calls
((call_expression
  function: (identifier) @func_name)
  (#eq? @func_name "eval")) @security_risk

; Find hardcoded credentials
(
  (assignment_expression
    left: (identifier) @key
    right: (string_literal) @val)
  (#match? @key "(?i)password|api_key|secret|token")
)
```

## Advanced Query Techniques

### Distinguishing Public vs Private Symbols

Tree-sitter's Rust grammar exposes a `visibility` field for public/private distinction:

```scheme
; Capture Public Functions
(function_item
  visibility: (visibility)  ; Matches the 'pub' keyword
  name: (identifier) @symbol.public)

; Capture Private Functions (no visibility field)
(function_item
  name: (identifier) @symbol.private
  (#not-has-type? @symbol.private visibility))
```

#### Rust Implementation

```rust
use tree_sitter::{Parser, Query, QueryCursor};

const SYMBOL_QUERY: &str = include_str!("../queries/symbols.scm");

fn main() {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();

    let code = r#"
        pub fn start_engine() {}  // Public
        fn check_oil() {}         // Private
        pub struct Car {}         // Public
    "#;

    let tree = parser.parse(code, None).unwrap();
    let query = Query::new(tree_sitter_rust::language(), SYMBOL_QUERY).unwrap();
    let mut cursor = QueryCursor::new();

    for m in cursor.matches(&query, tree.root_node(), code.as_bytes()) {
        for capture in m.captures {
            let tag = query.capture_names()[capture.index as usize];
            let name = &code[capture.node.byte_range()];

            match tag {
                "symbol.public" => println!("EXTERN: {}", name),
                "symbol.private" => println!("LOCAL:  {}", name),
                _ => {}
            }
        }
    }
}
```

### Using Anchors for Adjacency

The `.` anchor ensures nodes are immediately adjacent, useful for associating doc comments with functions:

```scheme
; Match a comment immediately followed by a public function
(
  (line_comment) @doc
  .
  (function_item
    visibility: (visibility)
    name: (identifier) @symbol.public)
)
```

#### Rust Implementation

```rust
const SYMBOL_QUERY: &str = include_str!("../queries/symbols.scm");

fn main() {
    let code = r#"
        // This initializes the system
        pub fn init() {}

        pub fn undocumented_func() {}
    "#;

    // ... parser setup ...

    for m in cursor.matches(&query, tree.root_node(), code.as_bytes()) {
        let mut doc_content = "No documentation";
        let mut func_name = "";

        for capture in m.captures {
            let tag = query.capture_names()[capture.index as usize];
            let text = &code[capture.node.byte_range()];

            match tag {
                "doc" => doc_content = text,
                "symbol.public" => func_name = text,
                _ => {}
            }
        }
        println!("Function: {} | Docs: {}", func_name, doc_content);
    }
}
```

### Negative Predicates for Filtering

Use negative predicates to exclude unwanted matches:

```scheme
; Match functions that do NOT have the #[test] attribute
(function_item
  (attribute_item
    (attribute
      (identifier) @attr_name))? ; Optional attribute capture
  name: (identifier) @function.name

  (#not-eq? @attr_name "test")
)

; Match functions NOT prefixed with "internal_"
(function_item
  name: (identifier) @function.name
  (#not-match? @function.name "^internal_"))
```

### Extracting Nested Symbols

Nested S-expressions capture parent-child relationships:

```scheme
; Match an impl block and all methods inside it
(impl_item
  type: (type_identifier) @impl.type
  body: (declaration_list
    (function_item
      name: (identifier) @method.name) @method.def
  )
)
```

#### Rust Implementation

```rust
for m in cursor.matches(&query, tree.root_node(), code.as_bytes()) {
    let mut parent_struct = "";
    let mut method_name = "";

    for capture in m.captures {
        let tag = query.capture_names()[capture.index as usize];
        let text = &code[capture.node.byte_range()];

        match tag {
            "impl.type" => parent_struct = text,
            "method.name" => method_name = text,
            _ => {}
        }
    }
    println!("Method: {}::{}()", parent_struct, method_name);
}
```

### Capture Groups and Regex Refinement

For extracting specific substrings (like version numbers), combine Tree-sitter captures with Rust regex:

```scheme
(macro_invocation
  macro: (identifier) @macro_name
  (#eq? @macro_name "version_bump")
  !: (token_tree
    (string_literal) @version.raw)
)
```

```rust
use regex::Regex;

const SYMBOL_QUERY: &str = include_str!("../queries/symbols.scm");

fn main() {
    let code = r#"
        version_bump!("2.4.0");
        other_macro!("ignore me");
    "#;

    // ... parser setup ...

    let re = Regex::new(r#""(\d+\.\d+\.\d+)""#).unwrap();

    for m in cursor.matches(&query, tree.root_node(), code.as_bytes()) {
        for capture in m.captures {
            let text = &code[capture.node.byte_range()];

            if let Some(caps) = re.captures(text) {
                println!("Detected Version: {}", &caps[1]);
            }
        }
    }
}
```

### Scope-Aware Symbol Tables

Building a scope-aware symbol table requires tracking definitions and scopes:

```rust
use tree_sitter::{Parser, Node, TreeCursor};
use std::collections::HashMap;

struct Symbol {
    name: String,
    kind: String,
}

struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, kind: &str) {
        if let Some(current) = self.scopes.last_mut() {
            current.insert(name.clone(), Symbol {
                name,
                kind: kind.to_string()
            });
        }
    }
}

fn analyze_rust(root: Node, source: &str) {
    let mut table = SymbolTable::new();
    let mut cursor = root.walk();
    let mut reached_root = false;

    while !reached_root {
        let node = cursor.node();

        // Pre-order: entering a node
        match node.kind() {
            "block" => table.enter_scope(),
            "let_declaration" => {
                if let Some(id_node) = node.child_by_field_name("pattern") {
                    let name = &source[id_node.byte_range()];
                    table.define(name.to_string(), "variable");
                }
            }
            _ => {}
        }

        // Traversal
        if cursor.goto_first_child() {
            continue;
        }

        // Post-order: leaving a node
        loop {
            if cursor.node().kind() == "block" {
                table.exit_scope();
            }

            if cursor.goto_next_sibling() {
                break;
            }

            if !cursor.goto_parent() {
                reached_root = true;
                break;
            }
        }
    }
}
```

## Multi-Language Support

### Language Registry Pattern

To support multiple languages, create a registry mapping language names to grammars and queries:

```rust
use tree_sitter::{Language, Parser, Query, QueryCursor};

// Bundle query files
const RUST_QUERY: &str = include_str!("../queries/rust_symbols.scm");
const PYTHON_QUERY: &str = include_str!("../queries/python_symbols.scm");

struct Extractor {
    language: Language,
    query: Query,
}

impl Extractor {
    fn new(lang: &str) -> Self {
        match lang {
            "rust" => {
                let grammar = tree_sitter_rust::language();
                Self {
                    language: grammar,
                    query: Query::new(grammar, RUST_QUERY).unwrap(),
                }
            }
            "python" => {
                let grammar = tree_sitter_python::language();
                Self {
                    language: grammar,
                    query: Query::new(grammar, PYTHON_QUERY).unwrap(),
                }
            }
            _ => panic!("Language not supported"),
        }
    }
}
```

### Normalized Capture Names

Use the same capture names across different languages for unified processing:

**`queries/python_symbols.scm`:**
```scheme
; Match Python classes
(class_definition
  name: (identifier) @symbol.name) @symbol.class

; Match Python functions
(function_definition
  name: (identifier) @symbol.name) @symbol.method
```

**`queries/rust_symbols.scm`:**
```scheme
; Match Rust structs
(struct_item
  name: (type_identifier) @symbol.name) @symbol.class

; Match Rust functions
(function_item
  name: (identifier) @symbol.name) @symbol.method
```

### Unified Extraction Logic

```rust
fn extract_symbols(extractor: &Extractor, code: &str) {
    let mut parser = Parser::new();
    parser.set_language(extractor.language).unwrap();

    let tree = parser.parse(code, None).unwrap();
    let mut cursor = QueryCursor::new();

    for m in cursor.matches(&extractor.query, tree.root_node(), code.as_bytes()) {
        for capture in m.captures {
            let tag = extractor.query.capture_names()[capture.index as usize];
            let name = &code[capture.node.byte_range()];

            // Works for ANY language due to standardized tags
            match tag {
                "symbol.name" => println!("Found symbol: {}", name),
                _ => {}
            }
        }
    }
}
```

### TypeScript Multi-Language Implementation

```typescript
import Parser from 'tree-sitter';
import JavaScript from 'tree-sitter-javascript';
import Python from 'tree-sitter-python';

interface LanguageConfig {
    lang: any;
    query: string;
}

const configs: Record<string, LanguageConfig> = {
    javascript: {
        lang: JavaScript,
        query: "(function_declaration name: (identifier) @name)"
    },
    python: {
        lang: Python,
        query: "(function_definition name: (identifier) @name)"
    }
};

function getFunctions(source: string, langKey: string) {
    const { lang, query } = configs[langKey];
    const parser = new Parser();
    parser.setLanguage(lang);

    const tree = parser.parse(source);
    const tsQuery = new Parser.Query(lang, query);
    const matches = tsQuery.matches(tree.rootNode);

    return matches.flatMap(m =>
        m.captures.map(c => c.node.text)
    );
}
```

### Leveraging Community Queries

Instead of hardcoding query patterns, use battle-tested queries from the `nvim-treesitter` repository:

```rust
use reqwest::blocking::get;

fn fetch_query(lang: &str) -> String {
    let url = format!(
        "https://raw.githubusercontent.com/nvim-treesitter/nvim-treesitter/master/queries/{}/tags.scm",
        lang
    );
    get(url).expect("Fetch failed").text().expect("Invalid UTF-8")
}

fn main() {
    let lang_name = "rust";
    let scm_content = fetch_query(lang_name);

    // Use the professional query
    let query = Query::new(language, &scm_content).expect("SCM error");

    // Look for standardized captures like @definition.function
    for m in cursor.matches(&query, root_node, code.as_bytes()) {
        for cap in m.captures {
            let name = query.capture_names()[cap.index as usize];
            if name == "definition.function" {
                // Process function definition
            }
        }
    }
}
```

## Production-Ready Patterns

### Project Structure

```
my_project/
├── Cargo.toml
├── build.rs             # Build script for asset tracking
├── src/
│   └── main.rs
└── queries/
    ├── rust_symbols.scm
    └── python_symbols.scm
```

### Build Script for Asset Tracking

`build.rs` ensures query files trigger recompilation when changed:

```rust
fn main() {
    println!("cargo:rerun-if-changed=queries/");
}
```

### Embedding Queries with `include_str!`

Embed queries at compile-time for zero-latency loading:

```rust
use tree_sitter::{Parser, Query, QueryCursor};

// Load query at compile time
const SYMBOL_QUERY: &str = include_str!("../queries/symbols.scm");

fn main() {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();

    let code = "fn hello_world() {}";
    let tree = parser.parse(code, None).unwrap();

    // Query is already in memory as a string
    let query = Query::new(tree_sitter_rust::language(), SYMBOL_QUERY)
        .expect("Failed to parse .scm query");

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), code.as_bytes());

    for m in matches {
        for capture in m.captures {
            println!("Captured: {}", &code[capture.node.byte_range()]);
        }
    }
}
```

### Complete Production Example

**`Cargo.toml`:**
```toml
[package]
name = "symbol_extractor"
version = "0.1.0"
edition = "2024"

[dependencies]
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
regex = "1.10"
```

**`build.rs`:**
```rust
fn main() {
    println!("cargo:rerun-if-changed=queries/symbols.scm");
}
```

**`queries/symbols.scm`:**
```scheme
; Match public structs with doc comments
(
  (line_comment) @doc
  .
  (struct_item
    visibility: (visibility)
    name: (type_identifier) @symbol.name)
) @symbol.struct

; Match methods inside impl blocks, excluding private ones
(impl_item
  type: (type_identifier) @parent.name
  body: (declaration_list
    (function_item
      name: (identifier) @symbol.name
      (#not-match? @symbol.name "^_")
    ) @symbol.method
  )
)
```

**`src/main.rs`:**
```rust
use tree_sitter::{Parser, Query, QueryCursor};

const QUERY_STR: &str = include_str!("../queries/symbols.scm");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Parser
    let mut parser = Parser::new();
    let language = tree_sitter_rust::language();
    parser.set_language(language)?;

    let code = r#"
        /// Represents a system user
        pub struct User { id: u64 }

        impl User {
            pub fn login(&self) {}
            fn _internal_check(&self) {}
            pub fn logout(&self) {}
        }
    "#;

    let tree = parser.parse(code, None).ok_or("Failed to parse")?;

    // Initialize Query once and reuse
    let query = Query::new(language, QUERY_STR)?;
    let mut cursor = QueryCursor::new();

    println!("{:<15} | {:<15} | {:<15}", "Type", "Name", "Context");
    println!("{:-<50}", "");

    for m in cursor.matches(&query, tree.root_node(), code.as_bytes()) {
        let mut name = "";
        let mut kind = "";
        let mut context = "";

        for capture in m.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let text = &code[capture.node.byte_range()];

            match capture_name {
                "symbol.name" => name = text,
                "parent.name" => context = text,
                "symbol.struct" => kind = "Struct",
                "symbol.method" => kind = "Method",
                _ => {}
            }
        }

        if !name.is_empty() {
            println!("{:<15} | {:<15} | {:<15}", kind, name, context);
        }
    }

    Ok(())
}
```

### Performance Best Practices

**DO:**
- Initialize `Query` objects once and reuse them (compilation is expensive)
- Use `QueryCursor` for iteration (optimized in C)
- Prefer `.scm` files over manual tree walking for complex patterns
- Use `include_str!` to embed queries at compile-time
- Leverage predicates (`#match?`, `#eq?`) to filter in the C engine

**DON'T:**
- Call `Query::new()` inside loops
- Use manual tree walking for pattern matching
- Cross the FFI boundary unnecessarily
- Forget to implement incremental parsing for real-time applications

## Schema Files (.scm)

Schema files (`.scm`) are the standardized way to define Tree-sitter queries. They use S-expression syntax and are the foundation of the `nvim-treesitter` ecosystem.

### File Hierarchy

The `nvim-treesitter` repository organizes queries by language and feature:

```
queries/
├── <language_name>/
│   ├── highlights.scm       # Syntax highlighting patterns
│   ├── injections.scm       # Language injection (e.g., CSS in HTML)
│   ├── locals.scm           # Scopes and variable tracking
│   ├── folds.scm            # Code folding regions
│   ├── indents.scm          # Auto-indentation logic
│   └── textobjects.scm      # Text object definitions
```

### Functional Objectives

| File | Purpose | Key Captures |
|------|---------|--------------|
| **highlights.scm** | Maps syntax nodes to colors | `@keyword`, `@function`, `@variable` |
| **injections.scm** | Identifies embedded languages | `@injection.content`, `@injection.language` |
| **locals.scm** | Defines scopes and variable tracking | `@definition.var`, `@scope`, `@reference` |
| **indents.scm** | Auto-indentation logic | `@indent.begin`, `@indent.align` |
| **folds.scm** | Code folding regions | `@fold` |
| **textobjects.scm** | Text object navigation | `@function.outer`, `@class.inner` |

### Inheritance with `;; inherits`

Languages can inherit queries from related languages:

```scheme
;; inherits: javascript

; TypeScript-specific queries follow...
```

This tells the engine to load JavaScript queries first, then apply TypeScript-specific ones.

### Extension with `;; extends`

User overrides can extend existing queries:

`~/.config/nvim/queries/python/highlights.scm:`
```scheme
;; extends

; Additional highlighting rules for Python
(decorator) @function.decorator
```

### Capture Naming Convention

Captures use hierarchical dot-notation:

- **Generic to Specific**: `@keyword` → `@keyword.function` → `@keyword.return`
- **Fallback Logic**: Themes fall back to parent categories if specific ones aren't defined

Examples:
- `@variable` → `@variable.builtin` → `@variable.parameter`
- `@function` → `@function.method` → `@function.builtin`
- `@keyword` → `@keyword.control` → `@keyword.return`

### Standard Capture Names

From `nvim-treesitter` conventions:

**Definitions:**
- `@definition.function`
- `@definition.method`
- `@definition.class`
- `@definition.var`

**References:**
- `@reference`

**Scopes:**
- `@scope`

**Text Objects:**
- `@function.outer` / `@function.inner`
- `@class.outer` / `@class.inner`
- `@parameter.outer` / `@parameter.inner`

## Popular Grammars

Below are the most popular Tree-sitter grammars for programming languages:

| Language | Repository | Description |
|----------|------------|-------------|
| **JavaScript** | [tree-sitter-javascript](https://github.com/tree-sitter/tree-sitter-javascript) | Modern JavaScript (ES2022+) and JSX syntax |
| **TypeScript** | [tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) | Comprehensive TypeScript including recent features |
| **Python** | [tree-sitter-python](https://github.com/tree-sitter/tree-sitter-python) | Python 3 syntax with indentation handling |
| **Rust** | [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) | Full Rust support including macros and attributes |
| **C++** | [tree-sitter-cpp](https://github.com/tree-sitter/tree-sitter-cpp) | Modern C++ standards (C++20+) |
| **Java** | [tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java) | Complete Java grammar with recent features |
| **Go** | [tree-sitter-go](https://github.com/tree-sitter/tree-sitter-go) | Full Go support with error recovery |
| **Ruby** | [tree-sitter-ruby](https://github.com/tree-sitter/tree-sitter-ruby) | Comprehensive Ruby grammar |
| **PHP** | [tree-sitter-php](https://github.com/tree-sitter/tree-sitter-php) | PHP 8+ with embedded HTML handling |
| **C#** | [tree-sitter-c-sharp](https://github.com/tree-sitter/tree-sitter-c-sharp) | Complete C# with latest features |
| **HTML** | [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) | HTML5 with script/style tag support |
| **CSS** | [tree-sitter-css](https://github.com/tree-sitter/tree-sitter-css) | CSS with latest specifications |
| **JSON** | [tree-sitter-json](https://github.com/tree-sitter/tree-sitter-json) | JSON with error handling |
| **Bash** | [tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash) | Bash scripting with POSIX compliance |
| **SQL** | [tree-sitter-sql](https://github.com/tree-sitter/tree-sitter-sql) | Multi-dialect SQL for major databases |

For a complete list, see:
- [Tree-sitter Wiki: List of Parsers](https://github.com/tree-sitter/tree-sitter/wiki/List-of-parsers)
- [tree-sitter-grammars organization](https://github.com/tree-sitter-grammars)

## Performance Considerations

### Incremental Parsing Benefits

Tree-sitter's incremental parsing significantly reduces re-parsing time:

- **Updates Only Changed Regions**: Only affected tree portions are re-parsed
- **Memory Efficient**: Optimized for large files
- **Real-Time Performance**: Fast enough for every keystroke in editors

### Query Performance

Performance characteristics of different query features:

| Feature | Performance Impact | Reason |
|---------|-------------------|---------|
| **Simple Node Match** | Negligible | Direct AST lookup |
| **Wildcards `(_)`** | Moderate | Checks every possible child node |
| **Regex `#match?`** | Low | Optimized regex engine, still more than equality |
| **Deep Nesting** | Low | Engine is designed for this; often faster than manual walking |

### Manual Walking vs. SCM Queries

| Method | Development Speed | Readability | Execution Speed | Tooling Support |
|--------|------------------|-------------|-----------------|-----------------|
| **Manual Walking** | Slow (High Boilerplate) | Poor (Nested loops) | Moderate | Rust LSP |
| **Inline Query** | Fast | Moderate | High | Minimal |
| **.scm Files** | Fast | Excellent | High | High (Tree-sitter LSP) |

### When to Use Each Approach

**Manual Tree Walking:**
- Finding "the parent of this specific node"
- Moving exactly one step left/right
- Very simple, one-off checks
- Tree transformation/mutation

**Inline Query Strings:**
- Fast prototyping
- One-off scripts
- Keeping logic and query in one file

**External .scm Files (Recommended):**
- Large-scale symbol extraction
- Linters and static analysis
- LSP features
- Production tools requiring maintainability
- Multi-language support

### Efficiency Comparison: Loading Methods

| Method | Loading Cost | Logic Complexity | Best For |
|--------|-------------|------------------|----------|
| **Manual Walking** | None | High (Recursion) | Simple parent-child checks |
| **Runtime File IO** | High (Disk Read) | Low (S-expression) | Dynamic user-provided queries |
| **`include_str!`** | **Zero** (Baked in) | **Low** (S-expression) | **Production CLI tools** |

### Query Reuse Best Practice

```rust
// BAD: Creates new Query object for each file
for file in files {
    let query = Query::new(lang, query_str)?; // EXPENSIVE!
    process_file(file, &query);
}

// GOOD: Reuse Query object
let query = Query::new(lang, query_str)?; // Create once
for file in files {
    process_file(file, &query); // Reuse
}
```

### WebAssembly Performance

When using Tree-sitter in browsers via WebAssembly:

- **Near-Native Performance**: WASM compilation provides excellent speed
- **Lazy Loading**: Load language grammars on-demand
- **Worker Threads**: Offload parsing to Web Workers for UI responsiveness

## Quick Reference

### Common Predicate Functions

| Predicate | Purpose | Example |
|-----------|---------|---------|
| `#eq?` | Exact string match | `(#eq? @var "main")` |
| `#match?` | Regex match | `(#match? @var "^test_")` |
| `#not-eq?` | Negated equality | `(#not-eq? @attr "test")` |
| `#not-match?` | Negated regex | `(#not-match? @func "^_")` |
| `#any-of?` | Match any in list | `(#any-of? @kw "let" "const" "var")` |

### Node Navigation (Manual Walking)

```rust
// Rust navigation methods
node.parent()           // Get parent node
node.child(index)       // Get child by index
node.child_by_field_name("name")  // Get child by field
node.next_sibling()     // Next sibling
node.prev_sibling()     // Previous sibling
node.first_child()      // First child
node.last_child()       // Last child

// TypeScript/JavaScript
node.parent
node.child(index)
node.childForFieldName("name")
node.nextSibling
node.previousSibling
node.firstChild
node.lastChild
```

### Testing Queries

Use the Tree-sitter CLI to test queries without recompiling:

```bash
# Test .scm file against source code
tree-sitter query queries/symbols.scm src/main.rs

# Parse and display tree
tree-sitter parse src/main.rs

# Check for syntax errors
tree-sitter test
```

### Debugging Tips

**View the syntax tree:**
```rust
println!("{}", tree.root_node().to_sexp());
```

**Inspect node details:**
```rust
println!("Kind: {}", node.kind());
println!("Range: {:?}", node.byte_range());
println!("Has errors: {}", node.has_error());
```

**Check query validity:**
```rust
match Query::new(language, query_str) {
    Ok(query) => println!("Valid query"),
    Err(e) => eprintln!("Query error: {:?}", e),
}
```

### Language-Specific Node Names

When writing queries, node names vary by language:

| Language | Function Node | Variable Declaration | Class/Struct |
|----------|---------------|---------------------|--------------|
| **Rust** | `function_item` | `let_declaration` | `struct_item` |
| **Python** | `function_definition` | `assignment` | `class_definition` |
| **JavaScript** | `function_declaration` | `variable_declarator` | `class_declaration` |
| **TypeScript** | `function_declaration` | `variable_declarator` | `class_declaration` |
| **Go** | `function_declaration` | `var_declaration` | `type_declaration` |

**Pro tip**: Use the Tree-sitter playground or CLI to explore node names for your target language.

### Resources

**Official Documentation:**
- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)
- [Tree-sitter Playground](https://tree-sitter.github.io/tree-sitter/playground)

**Community Resources:**
- [nvim-treesitter Queries](https://github.com/nvim-treesitter/nvim-treesitter/tree/master/queries)
- [Tree-sitter Wiki](https://github.com/tree-sitter/tree-sitter/wiki)

**Tools:**
- [tree-sitter CLI](https://github.com/tree-sitter/tree-sitter/tree/master/cli)
- [ast-grep](https://ast-grep.github.io/) - Code search/refactoring using Tree-sitter

**Language Bindings:**
- **Rust**: `tree-sitter` crate
- **Node.js**: `tree-sitter` (native) or `web-tree-sitter` (WASM)
- **Python**: `py-tree-sitter`
- **Go**: `go-tree-sitter`
