# Tree-Hugger Linting Rules

## Overview

Tree-hugger provides file-level static analysis using tree-sitter for parsing and custom semantic analysis for detecting issues like undefined symbols, unused code, and dead code.

## Design Principles

1. **File-scoped analysis** - All checks operate within a single file (no cross-file resolution yet)
2. **Conservative approach** - Avoid false positives; when uncertain, don't flag
3. **Ignore directives** - Users can opt out of specific warnings
4. **Language builtins** - Recognize standard library symbols as "always defined"

---

## Semantic Lints (File-Level Analysis)

These require tracking symbol definitions, imports, and references within a file.

### `undefined-symbol`

**Severity:** Error

**Description:** Reference to an identifier that is not defined, imported, or a language builtin.

**Logic:**

```
referenced identifier
AND NOT defined in file
AND NOT directly imported
AND NOT a member access on a namespace/wildcard import
AND NOT a language builtin
→ undefined
```

**Wildcard import handling:**

```typescript
import * as fs from "fs";
fs.readFile(...)     // Valid - fs is namespace import, trust any member
unknownFunc(...)     // UNDEFINED
```

**Future enhancement:** Resolve local file wildcard imports to check exports.

---

### `unused-symbol`

**Severity:** Warning

**Description:** Symbol defined but never referenced and not exported.

**Logic:**

```
defined in file
AND NOT exported
AND NOT referenced elsewhere in file
→ unused
```

**Excludes:**

- Exported symbols (they may be used externally)
- Symbols starting with `_` (conventional "intentionally unused" marker)

---

### `unused-import`

**Severity:** Warning

**Description:** Import statement that brings in symbols never used in the file.

**Logic:**

```
imported symbol
AND NOT referenced in file
→ unused import
```

**Note:** Namespace imports (`import * as x`) are unused only if `x` is never referenced at all.

---

### `dead-code`

**Severity:** Warning

**Description:** Code that can never execute because it follows an unconditional exit.

**Patterns to detect:**

- Statements after `return`
- Statements after `throw` (JS/TS/Java)
- Statements after `break` / `continue`
- Language-specific:
    - Rust: after `panic!()`, `unreachable!()`, `todo!()`, `unimplemented!()`
    - Go: after `panic()`
    - Python: after `raise` (when unconditional)

**Example:**

```rust
fn example() -> i32 {
    return 5;
    let x = 10;  // DEAD CODE
    x + 1        // DEAD CODE
}
```

---

## Pattern Lints (Query-Based)

Simple syntactic patterns detected via tree-sitter queries.

### Rust

| Rule | Severity | Description |
|------|----------|-------------|
| `unwrap-call` | Warning | `.unwrap()` call - may panic |
| `expect-call` | Warning | `.expect()` call - may panic |
| `dbg-macro` | Warning | `dbg!()` macro - debug only |

### JavaScript / TypeScript

| Rule | Severity | Description |
|------|----------|-------------|
| `debugger-statement` | Warning | `debugger` statement left in code |
| `eval-call` | Warning | `eval()` call - security/maintainability concern |

### Python

| Rule | Severity | Description |
|------|----------|-------------|
| `breakpoint-call` | Warning | `breakpoint()` - debugger invocation |
| `eval-call` | Warning | `eval()` call |
| `exec-call` | Warning | `exec()` call |

---

## Syntax Diagnostics (Already Implemented)

The existing `syntax_diagnostics()` method detects parse errors via tree-sitter ERROR nodes:

- Invalid token combinations (`const const a = 1`)
- Missing required tokens
- Unexpected tokens

These are separate from lint diagnostics and report as errors.

---

## Ignore Directives

Users can suppress warnings with comment directives.

### Ignore next line

```rust
// tree-hugger-ignore: unwrap-call
let value = result.unwrap();
```

### Ignore all rules on next line

```rust
// tree-hugger-ignore
let value = result.unwrap();
```

### Ignore for entire file (at top of file)

```rust
// tree-hugger-ignore-file: unused-import
```

---

## Language Builtins

Each language needs a list of "always available" identifiers that should not be flagged as undefined.

### Rust

- Prelude: `Option`, `Some`, `None`, `Result`, `Ok`, `Err`, `Vec`, `String`, `Box`, `Rc`, `Arc`
- Macros: `println!`, `print!`, `eprintln!`, `eprint!`, `format!`, `vec!`, `panic!`, `assert!`, `assert_eq!`, `assert_ne!`, `dbg!`, `todo!`, `unreachable!`, `unimplemented!`
- Primitives: `bool`, `char`, `str`, `i8`-`i128`, `u8`-`u128`, `f32`, `f64`, `isize`, `usize`

### JavaScript / TypeScript

- Globals: `console`, `window`, `document`, `globalThis`, `global`
- Functions: `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`, `fetch`, `alert`, `confirm`, `prompt`
- Constructors: `Array`, `Object`, `String`, `Number`, `Boolean`, `Date`, `RegExp`, `Error`, `Map`, `Set`, `WeakMap`, `WeakSet`, `Promise`, `Symbol`, `Proxy`, `JSON`, `Math`
- Values: `undefined`, `null`, `NaN`, `Infinity`

### Python

- Builtins: `print`, `len`, `range`, `str`, `int`, `float`, `bool`, `list`, `dict`, `set`, `tuple`, `type`, `isinstance`, `hasattr`, `getattr`, `setattr`, `open`, `input`, `abs`, `min`, `max`, `sum`, `sorted`, `reversed`, `enumerate`, `zip`, `map`, `filter`, `any`, `all`
- Values: `True`, `False`, `None`
- Exceptions: `Exception`, `ValueError`, `TypeError`, `KeyError`, `IndexError`, `AttributeError`, `RuntimeError`, `StopIteration`

### Go

- Builtins: `make`, `new`, `len`, `cap`, `append`, `copy`, `delete`, `close`, `panic`, `recover`, `print`, `println`, `complex`, `real`, `imag`
- Types: `bool`, `string`, `int`, `int8`-`int64`, `uint`, `uint8`-`uint64`, `float32`, `float64`, `complex64`, `complex128`, `byte`, `rune`, `error`
- Values: `true`, `false`, `nil`, `iota`

*(Additional languages to be documented)*

---

## Implementation Requirements

### New Infrastructure

1. **Reference tracking**
   - Add `QueryKind::References` for each language
   - Create `references.scm` query files to capture identifier usages
   - Add `referenced_symbols()` method to `TreeFile`

2. **Builtin symbol lists**
   - Create `builtins.rs` module with per-language symbol sets
   - Function to check: `is_builtin(language, symbol_name) -> bool`

3. **Control flow analysis (for dead code)**
   - Identify terminal statements (return, throw, break, etc.)
   - Check for following siblings in the same block
   - Mark following statements as dead code

4. **Ignore directive parsing**
   - Scan comments for `tree-hugger-ignore` patterns
   - Track which lines/rules are suppressed
   - Filter diagnostics before returning

### Modified `lint_diagnostics()` Flow

```rust
pub fn lint_diagnostics(&self) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Gather data
    let definitions = self.symbols()?;
    let imports = self.imported_symbols()?;
    let exports = self.exported_symbols()?;
    let references = self.referenced_symbols()?;  // NEW
    let ignore_directives = self.parse_ignore_directives();  // NEW

    // 2. Semantic lints
    diagnostics.extend(self.check_undefined_symbols(&references, &definitions, &imports));
    diagnostics.extend(self.check_unused_symbols(&definitions, &exports, &references));
    diagnostics.extend(self.check_unused_imports(&imports, &references));
    diagnostics.extend(self.check_dead_code());

    // 3. Pattern lints (query-based)
    diagnostics.extend(self.run_pattern_queries());

    // 4. Filter by ignore directives
    diagnostics.retain(|d| !ignore_directives.should_ignore(d));

    diagnostics
}
```

---

## Phase Plan

### Phase 1: Reference Tracking

- Create `references.scm` queries for all 16 languages
- Implement `referenced_symbols()` method
- Add tests for reference extraction

### Phase 2: Language Builtins

- Create `builtins.rs` with symbol sets per language
- Start with Rust, TypeScript, Python, Go
- Add remaining languages incrementally

### Phase 3: Semantic Lint Rules

- Implement `undefined-symbol` check
- Implement `unused-symbol` check
- Implement `unused-import` check
- Add tests for each rule

### Phase 4: Dead Code Detection

- Identify terminal statements per language
- Implement sibling-after-terminal detection
- Add tests for dead code scenarios

### Phase 5: Ignore Directives

- Parse `tree-hugger-ignore` comments
- Support line-level and file-level ignores
- Filter diagnostics accordingly

### Phase 6: Cleanup

- Remove noisy rules (debug-print, empty-block)
- Update CLI output formatting
- Update documentation
