# Tree Hugger Diagnostics

Tree Hugger provides three categories of diagnostics: lint (pattern-based), semantic (symbol analysis), and syntax (parse errors).

## Diagnostic Categories

### Lint Diagnostics

Pattern-based rules defined in `<lang>/lint.scm` queries.

| Language | Rule | Description |
|----------|------|-------------|
| **Rust** | `unwrap-call` | Explicit `.unwrap()` call |
| **Rust** | `expect-call` | Explicit `.expect()` call |
| **Rust** | `dbg-macro` | Debug macro `dbg!()` usage |
| **JS/TS** | `debugger-statement` | `debugger;` statement |
| **JS/TS** | `eval-call` | Usage of `eval()` |
| **Python** | `eval-call` | Usage of `eval()` |
| **Python** | `exec-call` | Usage of `exec()` |
| **Python** | `breakpoint-call` | Usage of `breakpoint()` |
| **PHP** | `eval-call` | Usage of `eval()` |

### Semantic Diagnostics

Symbol analysis rules computed at runtime:

| Rule | Severity | Description |
|------|----------|-------------|
| `undefined-symbol` | Error | Reference to undefined symbol |
| `unused-symbol` | Warning | Symbol defined but never used |
| `unused-import` | Warning | Imported symbol never referenced |
| `dead-code` | Warning | Code after unconditional return/throw/panic |

### Syntax Diagnostics

Parse errors detected by tree-sitter (ERROR and MISSING nodes).

## Getting Diagnostics

### Via Library

```rust
use tree_hugger_lib::TreeFile;

let file = TreeFile::new("src/main.rs")?;

// Individual types
let lint = file.lint_diagnostics();
let syntax = file.syntax_diagnostics();

// Unified format
let all = file.diagnostics();
for d in all {
    println!("{:?}: {} at {}:{}",
        d.kind, d.message,
        d.range.start_line, d.range.start_column);
}
```

### Via CLI

```bash
# All diagnostics
hug lint "src/**/*.rs"

# Lint only (pattern + semantic)
hug lint "src/**/*.rs" --lint-only

# Syntax only (parse errors)
hug lint "src/**/*.rs" --syntax-only

# JSON output
hug lint "src/**/*.rs" --json
```

## Ignore Directives

Suppress diagnostics with comments.

### Line-Level Ignore

Ignores diagnostics on the **next** line:

```rust
// tree-hugger-ignore: unwrap-call
let value = option.unwrap();  // Not reported

// tree-hugger-ignore: unwrap-call, expect-call
let a = x.unwrap();  // Not reported
let b = y.expect("msg");  // Not reported
```

### Ignore All Rules

Omit rule names to ignore all rules:

```rust
// tree-hugger-ignore
let risky = data.unwrap();  // No diagnostics reported
```

### File-Level Ignore

Ignore a rule for the entire file:

```rust
// tree-hugger-ignore-file: unused-import
// All unused-import diagnostics suppressed in this file
```

### Comment Styles

Supported comment prefixes:

| Style | Languages |
|-------|-----------|
| `//` | Rust, C, C++, Java, JavaScript, TypeScript, Go, Swift, Scala, PHP |
| `#` | Python, Bash, Zsh, Perl |
| `--` | Lua |
| `;` | Lisp-like languages |

### How It Works

The `IgnoreDirectives` parser:

1. Prefers tree-sitter-based parsing (avoids false positives in strings)
2. Falls back to line-based parsing if no comment query available
3. Tracks both line-level and file-level ignores
4. Directives apply to the line **immediately following** the comment

```rust
use tree_hugger_lib::IgnoreDirectives;

let directives = IgnoreDirectives::parse(&source);

// Check if ignored
directives.is_ignored("unwrap-call", 42)  // Line 42
directives.is_file_ignored("unused-import")
```

## Dead Code Detection

Tree Hugger detects unreachable code after terminal statements.

### Terminal Statements by Language

| Language | Terminal Statements |
|----------|---------------------|
| **Rust** | `panic!()`, `unreachable!()`, `todo!()`, `unimplemented!()`, `process::exit()` |
| **Go** | `panic()`, `os.Exit()` |
| **C/C++** | `exit()`, `abort()`, `_exit()`, `_Exit()`, `quick_exit()` |
| **Swift** | `fatalError()`, `preconditionFailure()`, `assertionFailure()` |
| **Perl** | `die`, `exit` |
| **Lua** | `error()`, `os.exit()` |
| **JS/TS** | `throw`, `process.exit()` |
| **Python** | `raise`, `sys.exit()`, `os._exit()` |
| **Java** | `throw`, `System.exit()` |

### Example

```rust
fn example() {
    panic!("fatal error");
    println!("unreachable");  // Reported as dead code
    let x = 5;                // Also dead code
}
```

### API

```rust
use tree_hugger_lib::{is_terminal_statement, find_dead_code_after};

// Check single statement
let is_terminal = is_terminal_statement(node, source, language);

// Find all dead code regions
let dead_regions = find_dead_code_after(tree, source, language);
```

## Builtin Symbol Detection

Semantic rules avoid false positives by recognizing language builtins.

### What's Considered Builtin

| Language | Examples |
|----------|----------|
| **Rust** | `Option`, `Result`, `Vec`, `String`, `println!`, `format!`, `Box`, `Rc`, `Arc` |
| **JavaScript** | `console`, `Array`, `Object`, `Promise`, `JSON`, `Math`, `Date`, `Error`, `Map`, `Set` |
| **TypeScript** | All JS builtins + `Partial`, `Required`, `Pick`, `Omit`, `Record`, `Exclude`, `Extract` |
| **Python** | `print`, `len`, `range`, `str`, `int`, `list`, `dict`, `set`, `open`, `type`, `None`, `True`, `False` |
| **Go** | `make`, `len`, `cap`, `append`, `copy`, `delete`, `panic`, `recover`, `error`, `string`, `int` |
| **Java** | `System`, `String`, `Object`, `Integer`, `Boolean`, `List`, `Map`, `Set`, `Exception` |
| **C#** | `Console`, `String`, `Object`, `List`, `Dictionary`, `Task`, `Exception`, `int`, `bool` |

### Usage

```rust
use tree_hugger_lib::{is_builtin, ProgrammingLanguage};

// Won't report "Option" as undefined in Rust
if !is_builtin(language, &symbol_name) {
    // Report undefined-symbol
}
```

## Source Context

Diagnostics include source context for display:

```rust
pub struct SourceContext {
    pub line_text: String,        // The source line
    pub underline_column: usize,  // Where to start underline
    pub underline_length: usize,  // How long to underline
}
```

CLI output with context:

```
[lint] warning [unwrap-call]: Explicit `.unwrap()` call
  --> src/main.rs:42:15
    |
 42 |     let value = result.unwrap();
    |                        ^^^^^^
```

## Adding New Lint Rules

1. **Add pattern to lint.scm**:
   ```scheme
   ; Example: detect todo!() macro in Rust
   (macro_invocation
     macro: (identifier) @_macro
     (#eq? @_macro "todo")
   ) @diagnostic.todo-macro
   ```

2. **Add severity mapping** (optional, defaults to Warning):
   ```rust
   // In queries/mod.rs
   fn default_severity(rule: &str) -> DiagnosticSeverity {
       match rule {
           "todo-macro" => DiagnosticSeverity::Info,
           // ...
       }
   }
   ```

3. **Add message formatting** (optional):
   ```rust
   // In queries/mod.rs
   fn rule_message(rule: &str) -> &'static str {
       match rule {
           "todo-macro" => "Unfinished `todo!()` macro",
           // ...
       }
   }
   ```

4. **Test the rule**:
   ```rust
   #[test]
   fn detects_todo_macro() {
       let file = TreeFile::new(fixture_path("lint_samples.rs"))?;
       let diagnostics = file.lint_diagnostics();
       assert!(diagnostics.iter().any(|d| d.rule == "todo-macro"));
   }
   ```
