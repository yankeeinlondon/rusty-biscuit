# Documentation, Flags and Metrics

Part of [Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

Beyond what a declaration *is*, a record can say whether it is documented,
deprecated or test-only, and how big it is. Given this Rust method:

```rust
impl Point {
    #[deprecated]
    pub async fn fetch(&self, scale: i32) -> i32 {
        if scale > 1 { self.x * scale } else { self.y }
    }
}
```

`hug --json-full` reports (trimmed):

```json
{ "visibility": { "is_deprecated": true, "is_test": false },
  "docs": {},
  "metrics": { "parameters": 1, "has_docs": false, "lines": 3, "cyclomatic": 2 } }
```

`#[deprecated]` sets the flag; there is no `///` doc comment, so
`has_docs` is `false`; `&self` is a receiver and not counted as a
parameter; the body spans three lines and holds one decision (`if`).

This page defines where documentation is read from in each language, which
markers set the deprecation and test flags, and exactly how each metric is
measured.

## Documentation

A declaration's documentation (`SymbolInfo::doc_comment`, the record's
`docs.raw_doc`) is the run of **doc comments** directly before it, with their
delimiters removed; ordinary comments are skipped, never read. Python reads
the docstring instead.

| Language | Documentation |
|---|---|
| Rust | `///` and `/** … */` (`//!` documents the enclosing module, not the next item); attributes between them and the item are skipped |
| Go | every comment in the group that ends on the line before the declaration (a blank line ends the group), minus its directive lines (see below) |
| Java, JavaScript, TypeScript, PHP, Scala | `/** … */` |
| C#, Swift | `///` and `/** … */` |
| C, C++ | Doxygen's `///`, `//!`, `/** … */` and `/*! … */`, but not their `<` forms (`///<`, `//!<`, `/**<`, `/*!<`), which document the member before them |
| Python | the docstring: a first statement of a `def` or `class` body that is a string constant (see below) |
| Lua, Perl, Bash, Zsh | none: these languages have no doc-comment detection |

```ts
/** Adds two numbers. */          // documents `add`
export const add = (a: number, b: number) => a + b;

// A plain comment documents nothing.
function helper() {}
```

**A comment that trails code documents nothing after it** in Go, JavaScript
and TypeScript. A comment that starts on the line where the previous code
ends, and any comment that starts on the line where such a comment ends,
belongs to that code: Go makes it the line comment of the previous line, and
TypeScript's JSDoc scan, used for both languages, starts after the line
break. Rust, Java, C#, PHP, Scala, Swift and a plain C or C++ `///` give a
same-line doc comment to the next declaration, as their own readers do.

```go
const (
	A = 1 // A's line comment     <- documents nothing: B is undocumented
	B = 2
)
```

```js
let a = 1; /** about a */         // trails `a`: `b` is undocumented
let b = 2;
```

```c
struct S {
  int a; ///< documents a, never b   <- not read: b is undocumented
  int b;
};
```

A declaration with no doc comments of its own is documented by those before
the wrapper around it:

- `export`, a JavaScript or TypeScript variable statement or `declare`, and a
  C++ `template<…>` header
- a Go `var`, `const` or `type` declaration, alone or grouped in parentheses:
  every name in it shares the declaration's doc, and inside a group a spec's
  own doc wins over the group's (as `go doc` reads them). A spec has its own
  doc whenever a comment group is attached to it, even one that reads as no
  text (a bare `//`, an empty `/* */`, only directives): that spec is
  undocumented and does not inherit the group's doc or its `Deprecated:`
  paragraph. Only a spec with no attached group shares the group's doc; a
  comment separated from the spec by a blank line, or trailing the previous
  spec on its line, is not attached
- a C or C++ `extern "C"` that declares one function or variable; the members
  of a braced `extern "C" { … }` block need their own docs
- a Scala `extension` that declares one method; the methods of a collective
  extension need their own docs

```go
// Limit documents both names.
var Limit, Floor int

// Deprecated: use Config.     <- documents A and C, and deprecates them
const (
	A = 1
	// B has its own doc.       <- B's doc, not the group's
	B = 2
	C = 3
	//                          <- attached but empty: D is undocumented
	D = 4
	//go:noinline               <- only a directive: E is undocumented
	E = 5
)

func run() {
	// Never documentation: a local variable has no doc.
	var count int
}
```

A doc comment that is empty after its delimiters are removed is no
documentation. Outside Go, such an empty doc does not stop the climb: a
declaration under one of the wrappers above whose own doc is empty is still
documented by the wrapper's.

**A Python docstring is a string constant, as Python decides it.** The first
statement of the body must be one ordinary string literal, with no prefix or
an `r`/`u` prefix and any quoting, or several of them written next to each
other (adjacent literals are one constant), optionally in parentheses. A
bytes literal (`b"…"`, `rb"…"`) or a formatted one (`f"…"`, `rf"…"`, `t"…"`)
is an expression, not a docstring, and so is a concatenation with one in it.
The text is the literals' content as written, joined and trimmed; escapes are
never evaluated, so `"\u0041"` documents and `""`, `"" ""` or a
whitespace-only string do not.

```python
def plain():
    "Documented."                # has_docs = true
def joined():
    "Docu" "mented."             # has_docs = true: one constant
def formatted():
    f"Not a docstring."          # has_docs = false
def raw_bytes():
    b"Not a docstring."          # has_docs = false
```

A doc comment is recognized by its opening delimiter alone: `/**/` and
`/***/` are ordinary block comments, and `////` is an ordinary line
comment.

**Block-comment decoration is removed the way each language's doc tool
removes it.** After the delimiters go (`/**` or `/*!` and `*/`; `///`,
`//!` or `//` for a line comment, which never loses a `*`, so `/// *`
documents), a leading `*` on a line is either decoration or text:

| Language | A leading `*` is decoration when… |
|---|---|
| Java, JavaScript, TypeScript, PHP, Scala, C, C++ | always: one leading `*` is removed from every line, whatever the other lines hold |
| Rust (rustdoc) | every line after the first (the first too when it starts with `*`; leading and trailing blank lines skipped) has its first `*` at the same column with only spaces or tabs before it; a line indented differently from the first keeps its `*`, and the `*` itself goes only when nothing, a space or another `*` follows it |
| C# (compiler) | the second line starts with spaces/tabs, `*`, spaces/tabs, and every later line shares a prefix with it that holds the `*`; a blank, unstarred or differently indented line cancels the removal for the whole comment |
| Swift (compiler) | the comment starts with a line break and its second line is ` * ` (or a lone ` *`) at the comment's own column; then only lines with that same decoration lose it |
| Go | never: see the Go paragraphs below |

A star that is not decoration is documentation text. In Rust, C# and Swift
a block of only stars that are not consistently placed therefore documents
its declaration, and so does the one-line `/** * */`; in the other block
languages both are undocumented.

```rust
/**
 *
  *
*/
pub fn misaligned() {}      // has_docs = true: the stars disagree, so one is text

/**
 *
 *
 */
pub fn aligned() {}         // has_docs = false: consistent decoration only

/**
Intro
 *b* is next
*/
pub fn mixed() {}           // raw doc "Intro\n*b* is next"
```

Every line of the cleaned doc is then trimmed and blank lines are dropped.
The structured-docs pass (`docs.parsed`, from `--json-full`) reads that
cleaned text as it is: a `*` or `//` still at the start of a line is part of
the summary.

## Deprecation and test flags

Two flags are computed when a query asks for `Capability::Flags`, which
`hug --json`, `--jsonl` and `--json-full` always do; without it both are
`false`, meaning "not computed".

**`visibility.is_deprecated`** is set by exactly these markers; every other
language and form (including an ordinary comment that says "deprecated")
leaves it `false`:

| Language | Marker |
|---|---|
| Rust | `#[deprecated]`, with or without arguments |
| Java | `@Deprecated` (or `@java.lang.Deprecated`), or a doc line that starts with the `@deprecated` tag |
| C# | `[Obsolete]`, also qualified (`System.Obsolete`) or with the `Attribute` suffix |
| JavaScript, TypeScript, PHP | a doc line that starts with the `@deprecated` tag (`@deprecated use load()` counts; `@deprecatedSoon`, a mid-line mention, a quoted or backticked `@deprecated` and a line inside a fenced code block do not; fences are described below the table) |
| Go | a doc paragraph that starts with `Deprecated:`, in `//` or `/* */` docs alike (a blank line starts a paragraph; an indented line is a code block, not a paragraph) |
| Swift | `@available(*, deprecated…)`: a top-level `deprecated` argument (a single-platform `@available(iOS, deprecated: 13)` and the word inside `message: "…"` do not count) |
| Scala | `@deprecated` (or `@scala.deprecated`) |
| C++ | `[[deprecated]]` before the declaration or right after the declared name (`C [[deprecated]] (int x);`, `int a [[deprecated]], b;` deprecates only `a`); one after a parameter list or array bound (`int f(int) [[deprecated]];`) belongs to the type and does not count, nor does a vendor form such as `[[gnu::deprecated]]` |
| Python | the `@deprecated` decorator, bare or as `typing_extensions.deprecated` or `warnings.deprecated` |

A fenced code block in a Java, JavaScript, TypeScript or PHP doc block
follows CommonMark, read after the leading `*` and spaces of each line are
removed:

- a fence opens on a line starting with three or more backticks or three
  or more tildes; a backtick fence's info string cannot contain a backtick,
  so a backtick run followed by text holding another backtick opens nothing;
- it closes only on a line made of the **same** character, at least as many
  as the opening run, followed by nothing but whitespace;
- every other line is example text: a shorter run, the other character, or
  a run followed by text (```` ```text ```` inside an open fence);
- a fence that never closes runs to the end of the doc block;
- a `@deprecated` line after the real closing fence is a tag again.

```text
/**
 * ````text
 * ```                      <- three backticks: content, the fence stays open
 * @deprecated example      <- inside the example: no effect
 * ````                     <- closes the four-backtick fence
 * @deprecated use other    <- a tag: is_deprecated = true
 */
```

A Go doc is read as Go reads it (`go/ast`'s `CommentGroup.Text`, then
`go/doc/comment`). Go defines no comment decoration, so only the delimiters
go: `//` and one space after it, or `/*` (with the whitespace after it on
its line) and `*/`. Every other character is doc text, including a third
opening `*` and a `*` at the start of a line: `/* *Deprecated: … */`,
`/**Deprecated: … */` and a body line ` * Deprecated: …` are not markers,
and a comment holding only `*` (`/* * */`, `// *`) documents its
declaration. The whitespace prefix that every nonblank line shares is then
removed, and a line still indented after that is a code block, so an
indented `Deprecated:` line is example text. The first unindented line after
a code block starts a new paragraph.

A Go **directive** is not documentation. A `//` line comment with no space
after `//` that starts `line `, `extern ` or `export `, or has the form
`name:x` with a lowercase-letter-or-digit name and a lowercase letter or
digit right after the colon (`//go:generate …`, `//go:noinline`,
`//a1:b2`), is dropped before anything else is decided. A declaration whose
group holds only directives is undocumented, and a directive leaves no blank
line behind, so it neither starts nor ends a paragraph. `// go:noinline`,
`//Go:noinline`, `//go: noinline`, `//go:` and `/*go:noinline*/` are prose.

```go
//go:generate echo generated     <- directive: ReviewGenerate is undocumented
func ReviewGenerate() {}

//go:generate echo generated
// Deprecated: use New.           <- still a paragraph start: is_deprecated = true
func ReviewBeforeMarker() {}

// Intro line.
//go:noinline
// Deprecated: use New.           <- same paragraph as the intro: not deprecated
func Mid() {}
```

Attributes are read from the syntax tree, not their text: a comment inside
an attribute is ignored (`#[deprecated /* why */]`, `[Obsolete /* why */]`
and `@java.lang. /* why */ Deprecated` count), and the marker word inside a
string argument or another attribute's arguments never counts
(`[[nodiscard("deprecated")]]`, `@nowarn("deprecated")`). A Python
decorator is read through parentheses (`@(deprecated)`). The record's
`attributes` list is this same parsed list; TypeScript and JavaScript
decorators appear there too but mark no flag (those languages' deprecation
reads only the doc block).

```go
// Deprecated: use NewPoint.      <- doc paragraph: is_deprecated = true
func MakePoint() {}

/* OldPoint makes a point.

Deprecated: use NewPoint.         <- block doc paragraph: is_deprecated = true
*/
func OldPoint() {}

func run() {
    // this path is deprecated      <- ordinary comment: no effect
}
```

**`visibility.is_test`** means **test-only code**. It is `true` for:

- every declaration of a test file, helpers included: Go `*_test.go`; Python
  `test_*.py`, `*_test.py` and `conftest.py`; JavaScript and TypeScript
  `*.test.*` and `*.spec.*`; Rust files below the `tests/` directory of the
  nearest enclosing Cargo package (the nearest directory holding a
  `Cargo.toml`; with none, the rule does not apply). The package layout is
  read on every query, cached or not, so adding, moving or removing a
  manifest changes the answer without touching the file (see
  [Cache Design](../cache-design/lookup.md#test-flags-source-facts-are-stored-file-wide-rules-are-not));
- a declaration with a test attribute: Rust `#[test]` and `#[<path>::test]`
  (`#[tokio::test]`), Java `@Test` (also qualified, `@org.junit.Test`), C#
  `[Test]`, `[Fact]` and `[TestMethod]`;
- everything inside a Rust item marked `#[cfg(test)]`, or a module or file
  whose inner attributes say `#![cfg(test)]`, nested declarations included.
  The predicate is read as tokens, so `#[cfg(/* why */ test)]` counts.

```rust
#[cfg(test)]
mod tests {                 // is_test
    fn helper() {}          // is_test (inside cfg(test))
}

#[cfg(any(test, feature = "x"))]
fn shared() {}              // not a test: compiled outside tests too

pub fn test_helper() {}     // not a test: the name decides nothing
```

**An out-of-line test module is not followed.** `#[cfg(test)] mod tests;`
marks only that `mod` declaration. The file holding its body is read on its
own, so its declarations are tests only when its path matches a file-name
rule, and a file under `src/` never does:

```rust
// src/parser.rs
#[cfg(test)]
mod tests;                  // is_test

// src/parser/tests.rs
fn helper() {}              // not a test: nothing in this file says cfg(test)
```

Add `#![cfg(test)]` at the top of the other file to mark it. Only the exact
predicate `cfg(test)` counts: `#[cfg(all(test, …))]` is not recognized.

The rules are a syntax and file-name heuristic, not a claim that a test
runner executes the symbol. Only the attributes named here are read; nothing
is resolved through imports, and other languages report `false`. There is no
rule for Python `unittest.TestCase` subclasses or JavaScript `describe`/`it`
calls; the file-name rules cover them in files named as tests.

## Metrics

The `metrics` facet is present when a query asks for `Capability::Metrics`,
which `hug --json-full`, `--stats` and the `--undocumented` and `--min-params`
filters do; without it the facet is absent. Inside a computed facet an
unsupported metric is `null` and a measured zero is `0`. A `null` never
matches a filter, and doc coverage counts it as unavailable rather than
undocumented (see [Symbol Reports](../symbol-reports.md)).

| Metric | Definition | `null` when |
|---|---|---|
| `parameters` | the declared parameters of a function, method or constructor: its own parameter list (every list of a curried Scala `def`), whatever binds each parameter (a name, a destructuring pattern or `_`), without a receiver (a Rust `self`, a Java `Type this` or a TypeScript `this: T`; a Go receiver is a separate list and never counted); a variadic parameter counts once; unnamed C, C++ and Go parameters count; `(void)` and `()` are `0` | the declaration is not a function, it has no parameter list (`def area: Int` in Scala), or the language is Perl, Bash or Zsh, whose parameters are not extracted |
| `has_docs` | whether the declaration has [documentation](#documentation): a doc comment that is not empty once its delimiters are removed, or a nonempty Python docstring (a string constant; bytes and formatted literals do not count); ordinary comments do not count | the language is Lua, Perl, Bash or Zsh, which have no doc-comment detection |
| `lines` | the physical lines a function, method or constructor's `source.body_span` touches (see [Body lines](#body-lines)): `{}` on one line is `1`, an empty body is `0` | the declaration is not a function, method or constructor (a class or module body is not counted), or it has no body: a prototype, an abstract, interface or protocol method, a defaulted or deleted C++ member |
| `cyclomatic` | 1 plus one per decision (`if`/else-if, a loop with a condition, a ternary, `&&`/`\|\|`, a non-default `match`/`switch` arm, a guard, a `catch`, …) in the same body `lines` measures, nested functions, closures and types left out (see [Cyclomatic Complexity](../cyclomatic-complexity.md)): an empty body is `1`, never `0` | as for `lines`; also when the language has no counting rules (every language but Rust, JavaScript and TypeScript), the counted body has a syntax error, or the total exceeds `u32` |

```python
class Receiver:
    def with_receiver(self, only):   # parameters = 2: `self` is an ordinary
        """Documented."""            # parameter in Python; has_docs = true
```

A parameter bound by a pattern is listed in the signature under the pattern
as written, with its declared type, so `--param-type` sees that type:

```rust
pub fn patterned((a, b): (u32, u32), _: String) {} // parameters = 2:
// `(a, b)` of type `(u32, u32)` and `_` of type `String`
```

```scala
def patterned(a: Int)(b: String): Int = a // parameters = 2, both lists
```

### Body lines

`lines` measures source size, not executable lines: every line the body
touches counts, braces, blank lines, comments and nested declarations
included. It is computed from the record's own `source.body_span`, so the two
always agree:

```rust
fn multi() {          // body_span: line 1, column 12 → line 5, column 2
    // a comment
                      // lines = 5 - 1 + 1 = 5
    fn nested() {}    // `nested` has its own body: lines = 1
}
```

For a body span from line `s` to line `e` (one-based), `lines` is `e - s + 1`,
or `e - s` when the span ends at column 1 (it stops before that line). CRLF
and LF files count the same. Which text is the body:

| Body form | The span | Example |
|---|---|---|
| a block | the block, braces included | `fn f() {}` → `{}`, 1 |
| an expression body | the expression only, never the `=>` or `=` that introduces it | TS/JS `(x) => x * 2` → `x * 2`; C# `int M() => 1;` → `1`; Scala `def f = 1` → `1` |
| Python and Lua | from the first statement or comment after the header (`:` in Python, the parameters in Lua) to the last one before `end`; a comment on the header's own line is not body | `def f():` with `# note` and `return 1` on the next two lines → 2 |
| an empty Lua body | an empty span at `end` | `local function f() end` → `0` |
| C and C++ | the definition's body; a C++ function-try-block is the whole `try … catch` | `int f() try { … } catch (...) { … }` |

The span never ends in whitespace: tree-sitter-scala's braceless (Scala 3)
body and Perl's `sub` block extend over the blank lines or the line break
after them, and those are left out. A default value (`def f(x=lambda: 1)`,
Swift `cb: () -> Int = { 1 }`), a return type (`n(): { a: number }`) and a
nested or returned callable are never taken for the body; a function that
returns a closure (`() => () => 1`) has that closure as its expression body.
