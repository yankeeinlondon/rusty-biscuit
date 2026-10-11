Part of [Using the Tree Hugger Library](../library.md).

# Diagnostics

Tree Hugger reports three kinds of problem in a source file: **lint**
diagnostics from pattern rules (a `dbg!()` left in Rust code, a `debugger;`
statement in JavaScript), **semantic** diagnostics from what the file
declares and uses (an unused import, code after a `return`), and **syntax**
diagnostics where the parser had to recover from an error. This page shows
how to get them from the library, which rules exist, and how to decide which
of them to show.

```rust
// src/dbg.rs
pub fn total(values: &[u32]) -> u32 {
    let sum = dbg!(values.iter().sum());
    // tree-hugger-ignore: dbg-macro
    let again = dbg!(sum);
    let first = values.first().unwrap();
    return again + first;
    println!("never");
}
```

```rust
use tree_hugger::{TreeFile, TreeHuggerError};

fn main() -> Result<(), TreeHuggerError> {
    let file = TreeFile::new("src/dbg.rs")?;
    for d in file.try_diagnostics()? {
        println!("{}:{} {:?} {:?} {}", d.range.start_line, d.range.start_column,
                 d.kind, d.rule, d.message);
    }
    Ok(())
}
```

```text
2:15 Lint Some("dbg-macro") Debug macro dbg!() call
5:17 Lint Some("unwrap-call") Explicit unwrap() call
7:5 Semantic Some("dead-code") Unreachable code after unconditional exit
```

The second `dbg!` is not reported because of the ignore comment above it.
`unwrap-call` is reported even though `hug lint` hides it by default: the
library returns every rule it finds and leaves the choice to you (see
[Choosing which rules to show](#choosing-which-rules-to-show)).

## Getting diagnostics

| Method | Returns |
|--------|---------|
| `diagnostics()` / `try_diagnostics()` | Lint, semantic and syntax diagnostics as `Diagnostic`, each with a `kind` (`Lint`, `Semantic` or `Syntax`) |
| `lint_diagnostics()` / `try_lint_diagnostics()` | Lint and semantic diagnostics only, as `LintDiagnostic` |
| `syntax_diagnostics()` | Syntax diagnostics only, as `SyntaxDiagnostic` |
| `dead_code()` | The unreachable blocks behind the `dead-code` rule, as `CodeBlock` (range and source snippet) |

The methods without `try_` never fail: when an analysis stage cannot run,
it contributes nothing and the others still report. The `try_` forms return
that failure as an error instead, so an empty list from them always means a
clean file. Prefer them whenever "clean" matters, as in a CI check.

Each diagnostic has a `message`, a `range` (1-based lines and columns), a
`severity` (`Error`, `Warning` or `Info`), the `rule` ID (`None` for syntax
diagnostics), an optional `context` holding the source lines around it, and
`metadata` describing the rule. Diagnostics also come with a
[`SymbolQuery`](../symbol-records/file-index.md#querying-from-the-library) that requests `Capability::Diagnostics`, in
`result.index.diagnostics`, and are stored by the cache.

## Rules

Pattern rules:

| Rule | Languages | Finds | `hug` default |
|------|-----------|-------|---------------|
| `dbg-macro` | Rust | `dbg!()` | on |
| `unwrap-call` | Rust | `.unwrap()` | off (`restriction`) |
| `expect-call` | Rust | `.expect()` | off (`restriction`) |
| `debugger-statement` | JavaScript, TypeScript | `debugger;` | on |
| `eval-call` | JavaScript, TypeScript, Python, PHP | `eval()` (in PHP in any letter case, such as `EVAL()`) | on |
| `exec-call` | Python, PHP | `exec()`: in Python it runs a string as code, in PHP a shell command | on |
| `breakpoint-call` | Python | `breakpoint()` | on |
| `console-log` | JavaScript, TypeScript | `console.log()` | off |
| `print-call` | Python | `print()` | off |
| `fmt-println` | Go | `fmt.Println()` | off |

Semantic rules, for every language:

| Rule | Finds | `hug` default |
|------|-------|---------------|
| `dead-code` | Statements after an unconditional `return`, `throw`, `break`, `panic!` or process exit in the same block | on |
| `unused-import` | An import never used in the file (an import a re-export entry forwards is exempt: Rust `pub use` but not `pub(self) use`, a TypeScript `export { x }`, a Python `__all__` member) | off (low confidence) |
| `undefined-symbol` | A reference to a name that is not declared, imported or a builtin | experimental |
| `unused-symbol` | A declaration never referenced or exported | experimental |
| `undefined-module` | A qualified reference whose qualifier is not declared or imported | experimental |

Syntax diagnostics carry the rule ID `invalid-syntax` in their metadata and
are always on.

In PHP, where function names ignore letter case, `exec-call` reports
`exec('ls')`, `EXEC('ls')` and the global `\exec('ls')`. It does not report
`Foo\exec('ls')` or `namespace\exec('ls')`, which call a different function
that shares the name, nor PHP's other ways to run a command: `shell_exec`,
`system`, `passthru`, `proc_open`, `popen` and the backtick operator. So
`hug lint --deny exec-call` fails on PHP files that call `exec()` as well as
Python ones.

`undefined-symbol` reports a name a file uses but neither defines nor
imports, and that is not a language builtin:

```python
total = compute_total(items)   # reports `compute_total` and `items`
```

It skips qualified names such as `os.path` (`undefined-module` checks the
qualifier), single-character names and names starting with `_`. It sees one
file at a time, so wildcard imports (`from x import *`), names made by macros
or at run time, and gaps in the builtin lists produce false positives; that
is why it is experimental and low confidence.

`unused-import` is off by default because, without type resolution, a
syntactic scan cannot see an import used only through a trait method (Rust
`Write` for `.write_all()`) or a macro. The **experimental** rules run only
when experimental semantics are enabled on the file; without it they are
never computed:

```rust
use tree_hugger::SourceFile;

let file = SourceFile::read("src/main.rs")?.experimental_semantics(true).parse()?;
// or, on an existing TreeFile: file.experimental_semantics = true;
```

`hug lint --list-rules` prints every registered rule with its category,
severity, confidence and default. Every listed rule can be reported.

## Choosing which rules to show

The library does not apply a rule policy. `hug lint` decides what to show
with the rule registry, and a library caller can do the same:

```rust
use tree_hugger::rule_registry::apply_policy;
use tree_hugger::{RuleRegistry, RuleSelector};

let registry = RuleRegistry::builtin();
let warn = [RuleSelector::Rule("unwrap-call".to_string())]; // like `--warn unwrap-call`

for mut d in file.try_lint_diagnostics()? {
    let Some(rule) = d.rule.as_deref().map(|id| registry.get_or_default(id)) else { continue };
    let opted_in = warn.iter().any(|selector| selector.matches(&rule));
    if !rule.enabled_by_default && !opted_in {
        continue; // off by default and not asked for
    }
    d.severity = apply_policy(&rule, &[], &warn, &[], false);
}
```

`RuleMetadata` gives each rule's `category`, `default_severity`,
`confidence`, `languages`, `enabled_by_default` and
`requires_experimental_semantics`. A `RuleSelector` parses `all`, a rule ID,
or `category:<name>` (such as `category:restriction`), and `apply_policy`
returns the severity after allow, warn, deny and strict selectors, in that
order of precedence.

`RuleSelector::parse` does not check a rule ID against the registry: an
unknown ID parses and matches nothing. Check it with
`registry.has_rule(id)`, which also accepts aliases such as
`unreachable-code`. `hug lint` does this for its `--deny`, `--warn` and
`--allow` values and prints `warning: unknown rule '<name>'` once per
distinct unknown name, without changing the exit status.

The `severity` on a diagnostic as returned is its rule's registry
`default_severity`, whether or not the rule is enabled by default, so the
library and an unconfigured `hug lint` agree (`undefined-symbol` is a
`Warning` in both). A selector changes it only when you apply the policy as
above.

## Ignore comments

A comment in the source suppresses lint and semantic diagnostics (syntax
diagnostics are never suppressed):

```rust
// tree-hugger-ignore: unwrap-call
let value = option.unwrap();      // not reported

// tree-hugger-ignore: unwrap-call, expect-call
let other = option.expect("set"); // not reported

// tree-hugger-ignore
let risky = data.unwrap();        // no rule reported on this line

// tree-hugger-ignore-file: unused-import
// unused-import is ignored in the whole file; with no rule list, every rule is
```

A line directive applies to the line after the comment. Directives are read
from real comments in the syntax tree, so the same text inside a string
literal does nothing. `//`, `#`, `--` and `;` comments all work.
`IgnoreDirectives` is exported for callers that want to apply the same
directives to their own diagnostics (`parse_with_tree`, then
`should_ignore(line, Some(rule))`).

## Builtins

Semantic rules would report `Option` or `print` as undefined without a list
of names every program can use. `is_builtin` exposes that list for all 16
languages:

```rust
use tree_hugger::{ProgrammingLanguage, is_builtin};

assert!(is_builtin(ProgrammingLanguage::Rust, "Option"));
assert!(is_builtin(ProgrammingLanguage::Python, "print"));
assert!(is_builtin(ProgrammingLanguage::JavaScript, "console"));
assert!(is_builtin(ProgrammingLanguage::Go, "make"));
```

## Dead code

The `dead-code` rule and `TreeFile::dead_code()` report statements that
follow an unconditional exit in the same block. Besides each language's
`return`, `break`, `continue` and `throw` or `raise`, these calls end a
block:

| Language | Calls that never return |
|----------|-------------------------|
| Rust | `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `process::exit`, `process::abort` (also with `std::`) |
| Go | `panic`, `os.Exit` |
| C, C++ | `exit`, `abort`, `_exit`, `_Exit`, `quick_exit` |
| Swift | `fatalError`, `preconditionFailure`, `assertionFailure` |
| Perl | `die`, `exit` |
| Lua | `error`, `os.exit` |

For your own syntax-tree walks, `is_terminal_statement(node, language,
source)` tells whether one Tree-sitter node is such an exit, and
`find_dead_code_after(node, language)` returns the sibling nodes after it.
