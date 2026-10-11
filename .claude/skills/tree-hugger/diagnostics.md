# Tree Hugger Diagnostics

Tree Hugger provides three categories of diagnostics: lint (pattern-based),
semantic (symbol analysis), and syntax (parse errors).

## Diagnostic Categories

### Lint Diagnostics

Pattern-based rules defined in `<lang>/lint.scm` queries.

| Language | Rule | Description | Default |
|----------|------|-------------|---------|
| **Rust** | `unwrap-call` | Explicit `.unwrap()` call | off (`restriction`) |
| **Rust** | `expect-call` | Explicit `.expect()` call | off (`restriction`) |
| **Rust** | `dbg-macro` | Debug macro `dbg!()` usage | on |
| **JS/TS** | `debugger-statement` | `debugger;` statement | on |
| **JS/TS** | `eval-call` | Usage of `eval()` | on |
| **Python** | `eval-call` | Usage of `eval()` | on |
| **Python** | `exec-call` | Usage of `exec()` | on |
| **Python** | `breakpoint-call` | Usage of `breakpoint()` | on |
| **PHP** | `eval-call` | `eval()`, any letter case | on |
| **PHP** | `exec-call` | `exec()`/`\exec()`, any letter case; not `Foo\exec()` | on |

`unwrap-call` and `expect-call` are `restriction`-category rules: the construct
is
valid and deliberate, so they are silent unless opted in with `--warn <rule>` or
`--deny <rule>` (or `--deny category:restriction`). `--allow` and `--strict`
never
enable an off-by-default rule.

### Semantic Diagnostics

Symbol analysis rules computed at runtime:

| Rule | Severity | Description | Default |
|------|----------|-------------|---------|
| `undefined-symbol` | Warning | Reference to undefined symbol | experimental |
| `undefined-module` | Warning | Reference to undefined module or namespace | experimental |
| `unused-symbol` | Warning | Symbol defined but never used | experimental |
| `unused-import` | Warning | Imported symbol never referenced | off (low confidence) |
| `dead-code` | Warning | Code after unconditional return/throw/panic | on |

`unused-import` is off by default (opt in with `--warn unused-import`): a
syntactic
scan cannot resolve imports used only via trait methods or macros, so it has an
irreducible false-positive rate. Imports forwarded by a re-export report entry (`pub use`, not `pub(self) use`; `ImportSymbol::is_reexport`) and type-position usages
(`fn f(x: T)`, `field: T`) are **not** flagged — those were fixed.

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

// Tree-sitter-based parsing (preferred, avoids false positives in strings)
let directives = IgnoreDirectives::parse_with_tree(&source, &tree, language);

// Line-based fallback parsing
let directives = IgnoreDirectives::parse(&source);

// Check if a diagnostic should be suppressed
directives.should_ignore(42, Some("unwrap-call"))  // specific rule at line 42
directives.should_ignore(42, None)                  // any rule at line 42
directives.has_directives()                         // true if any directives found
```

## Dead Code Detection

Tree Hugger detects unreachable code after terminal statements.

### Terminal Statements by Language

| Language | Terminal Statements |
|----------|---------------------|
| **Rust** | `panic!()`, `unreachable!()`, `todo!()`, `unimplemented!()`, `process::exit()`, `process::abort()` |
| **Go** | `panic()`, `os.Exit()` |
| **C/C++** | `exit()`, `abort()`, `_exit()`, `_Exit()`, `quick_exit()` |
| **Swift** | `fatalError()`, `preconditionFailure()`, `assertionFailure()` |
| **Perl** | `die`, `exit` |
| **Lua** | `error()`, `os.exit()` |
| **JS/TS** | `throw`, `process.exit()` |
| **Python** | `raise`, `sys.exit()`, `os._exit()` |
| **Java** | `throw`, `System.exit()` |

A call's function may be parenthesized (`(exit)(1)`; Swift `(fatalError)()`);
a sequence such as `(g, exit)(1)` is not a terminal call.

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

// Check if a node is a terminal statement
let is_terminal = is_terminal_statement(node, language, source);

// Find dead code sibling nodes after a terminal statement
let dead_nodes = find_dead_code_after(terminal_node, language);
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

2. **Register the rule** in `rule_registry.rs` with its `languages` and
   `default_severity`. The registry is the only severity table: every
   diagnostic is stamped with its rule's `default_severity` (an unregistered
   ID gets `Info`). A rule emitted by Rust code instead of a query also goes
   in `SEMANTIC_RULE_IDS` (`file/tree_file.rs`). `rule_registry_consistency`
   fails if a query capture, registry entry or message arm is missing on one
   side, and `lint_severity_registry` needs an emitting case for the rule.
   Any registry or emitted-diagnostic change also bumps `EXTRACTOR_REVISION`.

3. **Add message formatting** (optional):
   ```rust
   // In queries/mod.rs — format_rule_message()
   "todo-macro" => "Unfinished `todo!()` macro".to_string(),
   ```

4. **Test the rule**:
   ```rust
   #[test]
   fn detects_todo_macro() {
       let file = TreeFile::new(fixture_path("lint_samples.rs"))?;
       let diagnostics = file.lint_diagnostics();
       assert!(diagnostics.iter().any(|d| d.rule.as_deref() == Some("todo-macro")));
   }
   ```

## Diagnostic Metadata

Every diagnostic now carries stable metadata for policy and display:
- **Category**: `correctness`, `suspicious`, `style`, `performance`, `pedantic`,
  `restriction`, `experimental`
- **Confidence**: `high`, `medium`, `low`, `experimental`
- **Source**: `tree-sitter-query`, `semantic-analysis`, `syntax-parser`,
  `external-tool`
- **Severity**: `default_severity` and `effective_severity` (after policy)

## Rule Registry

Built-in rules are registered in `RuleRegistry` with metadata:
- Rule id, version, title, category, default severity, confidence
- Language support, default enablement, examples, caveats
- `requires_experimental_semantics` gates low-confidence semantic rules
- `RuleRegistry::builtin()` is the shared, process-wide registry (a `LazyLock`;
  the registry has no mutators). Use it instead of `RuleRegistry::new()` in
  per-diagnostic or per-file code.
- The CLI applies `apply_lint_policy` once per diagnostic list (native in
  `summarize_file`, external in `add_external_lint_diagnostics`); rendering does
  not re-apply it. It is idempotent, pinned by
  `lint_policy_applied_twice_equals_once`.

Default-off pattern rules are silent unless opted in with `--warn <rule>` or
`--deny <rule>` (the registry's `enabled_by_default` flag is enforced by the CLI
policy layer, `apply_lint_policy`):
- Debug artifacts: `console-log`, `print-call`, `fmt-println`.
- `restriction` category: `unwrap-call`, `expect-call` — valid, deliberate
  constructs, not anomalies. Enable per-rule or via `--deny
  category:restriction`. `--allow`/`--strict` never enable an off-by-default
  rule.

Experimental rules (`undefined-symbol`, `unused-symbol`, `undefined-module`) are
disabled by default. Enable with `TreeFile::experimental_semantics = true` or
`--experimental-semantics` in the CLI.

## Dead Code Block Scope

Dead code is detected only within the same block as a terminal statement
(`return`, `panic!`, `break`, etc.). Terminal statements inside nested
control-flow constructs (e.g., an `if` branch, a `match` arm, or a closure) do
not propagate dead-code detection to the enclosing function or method body.

## CLI Policy

Policy selectors target rules by ID or category (`category:correctness`):
- `--deny <rule>`: promote to error
- `--warn <rule>`: promote to warning
- `--allow <rule>`: demote to info
- `--strict`: promote all warnings to errors (does not enable experimental
  rules)
- Syntax diagnostics are always errors and always fail the exit code
- An unknown rule ID (not `has_rule`, which accepts aliases) is kept as a
  selector that matches nothing; `main::warn_unknown_rules` prints
  `warning: unknown rule '<name>'` (neutralized) once per distinct name,
  right after `Command::kind()` builds the selectors, not in
  `parse_selectors`. `RuleSelector::parse` stays non-validating. Status is
  unchanged. Tests: `cli/tests/command_feedback.rs`.

## Resolver and Adapter Architecture

### Semantic Resolvers

The `resolver` module defines interfaces for project-level semantic analysis:
- `ProjectContext`: Root path, manifests, language configs, dependency hints,
  generated-file markers, target environment
- `SemanticResolver` trait: Reports confidence and supported-scope metadata,
  returns `ResolverOutput` with diagnostics and resolved imports
- `ResolverScope`: `FileLocal`, `Module`, `Project`, `Workspace`

Resolvers are gated until corpus precision is measured. Low-confidence
resolver-backed diagnostics use `DiagnosticConfidence::Low` or `Experimental`.

### External Diagnostic Adapters

The `adapter` module defines interfaces for delegating to external tools:
- `ExternalDiagnosticAdapter` trait: `name()`, `version()`, `is_available()`,
  `supported_languages()`, `run()`
- `AdapterConfig`: Tool path, config path, extra args, environment, strict mode,
  timeout
- `AdapterResult`: Normalized diagnostics + `AdapterMetadata` (tool name,
  version, config files, exit status, elapsed time, cache status, fix
  availability)
- `AdapterError`: `ToolNotFound`, `IncompatibleVersion`, `ExecutionFailed`,
  `ParseFailed`, `UnsupportedLanguage`, `ConfigError`

Adapters preserve source tool rule IDs and report fix availability without
applying fixes.

### Oxlint Adapter

`OxlintAdapter` is the first external adapter for JavaScript/TypeScript:
- Discovers `oxlint` via explicit config -> project-local
  `node_modules/.bin/oxlint` -> `PATH`
- Parses Oxlint JSON output into normalized `Diagnostic` values with
  `DiagnosticSource::ExternalTool`
- Maps Oxlint severity and categories to Tree Hugger equivalents
- Falls back to native diagnostics when Oxlint is unavailable
- In non-strict mode, missing tools produce empty results with metadata rather
  than errors

## Corpus and Default-On Gates

The `corpus` module provides infrastructure for measuring rule precision against
real-world code:

### Corpus Harness

- **`CorpusManifest`**: Defines repositories/fixtures with metadata (SHA,
  license, paths, rules, oracles)
- **`CorpusTier`**: `Smoke` (fast PR checks), `Expanded` (scheduled CI),
  `Benchmark` (performance timing)
- **`CorpusResult`**: Analysis output with per-rule threshold reports
- **`ThresholdReport`**: Tracks diagnostic counts against rule thresholds
  (`Zero`, `Budget(n)`, `Unlimited`)

### Snapshot Stabilization

- **`redact_snapshot_text()`**: Redacts absolute paths, temp dirs, tool
  locations
- **`normalize_diagnostics()`**: Sorts and deduplicates for stable comparison
- **`compact_diagnostic_text()`**: Produces compact stable representations
- **`strip_ansi_codes()`**: Removes terminal escape sequences

High-confidence syntax rules require `Threshold::Zero` corpus false positives.
Experimental semantic rules use `Threshold::Unlimited` until resolver-backed
precision is proven.
