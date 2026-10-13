# Selector Details

Part of [Symbol Reports](../symbol-reports.md).

The selector table in
[Selecting declarations](../symbol-reports.md#selecting-declarations) says
what each selector keeps. This page holds the rules that need more room: how
name patterns match, how `hug` decides whether a filter is a file or a name,
how `imports` and `--prelude` select, and why filtering never costs a cache
entry.

## Name patterns

Names use **smart case**: a pattern with no uppercase letter ignores case, any
uppercase letter makes it case-sensitive. Case folding works on whole
characters, so `ünï` also matches `Ünï_big`.

| Pattern | Matches |
|---|---|
| `parse` | any name containing `parse`, `Parse`, `PARSE`, … |
| `Parse` | names containing exactly `Parse` |
| `parse*` | names starting with `parse` (a `*` makes the pattern cover the whole name) |
| `parse!` or `--exact parse` | the name `parse` (any case), and nothing longer |
| `--exact 'a*'` | the name `a*`: `*` is literal under `--exact` |

A pattern containing `#` or the language's qualified-name separator (`::`
in Rust, C++, Perl and PHP; `.` elsewhere) matches the displayed
**qualified name**, which starts with the file's path from the scan root
(`src/symbol.rs#SymbolInfo::new`) unless the language roots it in a
namespace (a C++ `namespace`, a Java `package`). A wildcard pattern is
anchored at the start of that name, so:

```bash
hug symbols --name 'symbol.rs#*'      # a symbol.rs at the scan root only
hug symbols --name '*/symbol.rs#*'    # a symbol.rs anywhere below it
hug functions --name 'Widget::*'      # nothing: the name starts with its file
hug functions --name '*Widget::*'     # every Widget member
hug symbols --name 'geometry::*'      # a C++ namespace: no file prefix
```

## `--file` and `--name` versus positional filters

A positional token is guessed to be a file or a name. `--file` and `--name`
say which it is, so a name with a `/` (a file-qualified pattern) or a file
without a source extension can be given. File values are alternatives, as are
name values; the two lists intersect.

- A `--file` value naming an existing file (relative to the working
  directory, then the repository root; `~/`, `./` and the other
  [file reference](../../../biscuit-file/docs/topics/file-references.md) forms
  work) is scanned even when `.gitignore` or the default test-data
  exclusions (`fixtures/`, `__fixtures__/`, `snapshots/`, `testdata/`) would
  skip it. `--exclude-files` still removes it.
- Any other value (a glob) is used like a positional file filter. A value
  that is not a valid file reference is an argument error.
- A directory, positional or `--file`, selects nothing today: `hug functions
  src/` reports that no source files were found. Write it as a glob instead,
  `hug functions 'src/**'`.

A positional file filter is a `.gitignore`-style pattern relative to the
package root, matched while walking the package (with `--language`, a
positional path naming an existing file is instead taken as given, like
`--file`). Naming a file this way
re-includes it when `.gitignore` or a fixture rule would skip it, but not
when a directory above it is ignored, and a symlink is never selected:

```bash
hug functions src/lib.rs          # src/lib.rs only, even if .gitignore lists it
hug functions vendor/x.rs         # nothing when vendor/ is ignored
hug functions lib.rs              # every lib.rs at any depth, as '**/lib.rs'
```

A literal path with a `/` and no glob characters (`src/lib.rs`, not
`src/*.rs`, `lib.rs` or `/src/lib.rs`) can match only itself, so `hug`
opens only the directories on the way to it instead of walking the whole
package. The rules above are applied exactly as in the full walk; the one
difference is that an unreadable directory elsewhere in the package no
longer fails the scan, because it is never opened. Several literal paths
are looked up together; one glob among them walks the whole package.

## `imports`

`imports` lists one entry per name an import binds (what each language's
imports bind is in
[Scope-Aware References](../references/language-support.md#what-an-import-binds)). It takes
only file filters positionally, and `--file`, `--name`, `--exact` and
`--exclude-symbols`. An import matches when its local name, its alias or its
original name matches, so both `--name HashMap` and `--name Map` find
`use std::collections::HashMap as Map;`. A qualified pattern
(`std::collections::*`) matches the imported path instead: the original, the
source, or the source joined with the original (`std::collections::HashMap`).

## `--prelude`

`--prelude` keeps the names a package re-exports from its Rust prelude
(`src/prelude.rs`, `src/prelude/mod.rs`, or the same under a direct child
package), plus any comma-separated names in the `PRELUDE` environment
variable. Each prelude re-export is listed as its own record under the
alias name, marked synthetic and re-exported (`visibility.is_synthetic`,
`is_reexported`), with its own ID. When the target is found the record
takes its kind, keyword, doc comment and declaration location; otherwise
it sits at the `pub use` line with kind Unknown. The prelude and its
targets are analyzed once and reused by the scan; a target outside the
scan is resolved but not listed. The selectors apply to these records too.

## Selectors and the cache

Kinds narrow what is extracted, and a selector that reads the test,
deprecation or metric facts asks for them (`--undocumented` and
`--min-params` compute the `metrics` facet, `--exclude-tests`,
`--tests-only` and `--deprecated` the flags), but every other predicate, and
the qualified-name prefix, is applied to each file's result **after** the
cache lookup. A filtered run therefore reuses the entry an unfiltered run
wrote, and two runs from different scan roots share one entry and the same
IDs; only the displayed prefix differs.
