# Release Notes

Changes to Tree Hugger that alter what it reports or how its output looks,
newest first. Each note shows the change with a before/after example where
the output changed; the linked pages describe the current behavior.

## Query licenses, readable command errors, unknown rule warnings

- **Tree Hugger's own queries are labeled `AGPL-3.0-only`**, the
  repository's license, in their provenance records; they were labeled
  `MIT`. Vendored nvim-treesitter queries keep `Apache-2.0`. See
  [Tree-sitter Queries](./queries.md#provenance).
- **A failing command prints a readable error.** It used to print the
  error's internal structure:

  ```text
  $ hug god-files nope-dir
  Error: Io { path: "nope-dir", source: Custom { kind: Other, error: "god-files scan directory does not exist" } }
  ```

  Now it prints one line, still with exit status `1`:

  ```text
  $ hug god-files nope-dir
  error: Failed to read `nope-dir`: god-files scan directory does not exist
  ```

- **Unknown rule names are accepted with a warning.** `--deny`, `--warn` and
  `--allow` values that name no registered rule, including the removed
  `unused-variable`, `shadowed-variable` and `deprecated-syntax`, print
  `warning: unknown rule '<name>'` once per run and leave the exit status
  unchanged. See [Using the `hug` CLI](./cli.md#lint).

## Unused library APIs removed

- **`TreeFile::reexported_symbols()` is removed.** It always returned an
  empty list. Use `TreeFile::reexports()` for the forwarding report, or
  filter `imported_symbols()` on `ImportSymbol::is_reexport`.
- **`is_reexport` and `unused-import` follow the re-export report.** The flag
  is decided from forwarding entries in every language, not from a Rust
  `visibility_modifier`, so a Rust `pub(self) use` is no longer a re-export
  and loses its `unused-import` exemption.
- **The query drift module is removed** (`check_query_drift`,
  `check_all_drift`, `summarize_drift`, `expected_upstream_revisions`,
  `DriftReport`, `DriftItem`). It only compared two revision strings.
  Each vendored query's provenance still records its upstream revision; see
  [Tree-sitter Queries](./queries.md#refreshing-a-vendored-query).

## Lint severities follow the rule registry; PHP `exec` and `EVAL` reported

- **One severity.** A library diagnostic now carries its rule's registry
  default severity. `undefined-symbol` was returned by the library as
  `Error` while `hug lint --experimental-semantics` showed a `Warning`; both
  now say `Warning`. Cached entries from earlier builds are recomputed.
- **Rules that never fired are gone.** `unused-variable`,
  `shadowed-variable` and `deprecated-syntax` are no longer registered or
  listed by `hug lint --list-rules`. Use `unused-symbol` for unused locals.
- **PHP `exec()` is reported** under `exec-call` (now titled "Dynamic code
  or command execution (exec)"), including `EXEC()` and the global
  `\exec()`, but not `Foo\exec()`. `--deny exec-call` therefore also fails on
  PHP files.
- **PHP `eval` matches in any letter case**, so `EVAL(...)` and similar
  spellings gain new `eval-call` findings. Python stays case-sensitive.

Before:

```text
$ hug lint --plain run.php   # exec('ls'); EVAL('1;');
(nothing reported)
```

After:

```text
$ hug lint --plain run.php
run.php (PHP)
[lint] warning [exec-call]: Dynamic code or command execution via exec() is discouraged
...
[lint] warning [eval-call]: Use of eval() is discouraged
...
```

See [Diagnostics](./library/diagnostics.md).

## Automatic package sections in stats

`hug functions|types|symbols|classes --stats` now prints overall totals followed
by package sections. Counts and documentation coverage keep their meanings.

Before:

```text
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 1 of 3 documented (33.3%); 0 unavailable
```

After (tables abbreviated):

```text
All selected declarations
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 1 of 3 documented (33.3%); 0 unavailable

utils (crates/utils)
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 1 of 2 documented (50.0%); 0 unavailable

Unassigned
Kind / Count, Visibility / Count, Language / Count tables
Doc coverage: 0 of 1 documented (0.0%); 0 unavailable
```

JSON retains overall `stats` and report version 1, adding `package_root_dir`
and `groups`. Group paths resolve against the discovery root, which can differ
from the scan root. See [Package Stats](./symbol-reports/package-stats.md).

The intentional snapshot changes are `symbols.stats` (overall heading and
Unassigned section) and `symbols-stats.json` (discovery root and one null-package
group with the same stats as the overall totals). Both cover the existing
16-language fixture corpus; no declaration counts changed.

## Scope-aware references and `hug refs`

References are now resolved through each file's lexical scopes instead of
being bound to the first declaration with the same name:

- **Occurrences with a confidence.** Every reference is an occurrence with
  its owner, kind (`Call`/`Usage`) and confidence: `Scoped` (one proven
  binding), `NameOnly` (same-named candidates, none proven) or `Unresolved`.
  Rust, TypeScript and JavaScript resolve; every other language reports
  occurrences with `Limited` coverage and never claims a binding.
- **Honest summaries.** `relations.references`, `dependencies`,
  `referenced_by` and `semantics.is_recursive` come only from `Scoped`
  occurrences, so a shadowed local or `other.parse()` no longer links to
  `fn parse`, and a direct self-call makes `is_recursive` `Yes`:

  ```rust
  fn parse() {}
  fn run() { let parse = || 1; parse(); } // before: run -> fn parse; now: run -> the local
  ```

- **`hug refs`** lists one declaration's references, callers or callees,
  grouped by owner, with name-only suggestions kept apart:

  ```sh
  hug refs parse 'src/**'
  hug refs --id <ID> --callees 'src/**' --json
  ```

  Resolution never leaves a file, and every report says so.

Records move to schema **5.0** (a change of meaning, not of layout). Cache
entries written by earlier versions are re-analyzed once. The rules, the
language table and the recursion decision are in
[Scope-Aware References](./references.md).

## Generics, base types, modifiers, attributes and body lines

Records now fill fields that were declared but empty, all read from the
syntax tree in the same single parse (no compiler, no extra pass):

- **Generic parameters** (`type_info.generic_params`): name, kind (`Type`,
  `Lifetime`, `Const`), bounds and default as written, variance and position,
  for Rust, TypeScript, Go, Python, Java, C#, C++, Swift and Scala.
  Requirements on anything other than a declared parameter (`where T::Item:
  Copy`, a C++ `requires` clause) go to `type_info.constraints`.
- **Base types** (`relations.extends`, `relations.implements`, `relations.unclassified_bases`): each base as
  written, with a same-file `id` only when the name names exactly one
  declaration. A Rust trait impl adds its trait to its members and to the
  type it implements. C#, Swift and Scala preserve ambiguous bases in `relations.unclassified_bases`.
- **Modifiers and attributes**: `const fn`, `unsafe`, `mut`, `readonly`,
  `abstract`, `final`, `override`, `virtual`, `sealed`, `mutating`, in one
  fixed order; `attributes` lists the declaration's own attributes,
  annotations and decorators with their raw arguments.
- **Body size**: `metrics.lines` counts the lines of a function, method or
  constructor's body (`null` for other kinds and bodiless declarations).
- **Cyclomatic complexity**: `metrics.cyclomatic` is 1 plus one per decision
  (`if`, a conditional loop, a ternary, `&&`/`||`, a non-default arm, a
  guard, a `catch`) in that same body, nested functions and types left out.
  Rust, JavaScript and TypeScript have counting rules; every other language,
  a bodyless declaration and a body with a syntax error report `null`, which
  never means zero. The rules, with an example each, are in
  [Cyclomatic Complexity](./cyclomatic-complexity.md).
- **Outline rows show bases**: `hug types --tree` and `hug symbols --tree`
  end a type's row with them.

  ```text
  - class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]
  ```

These facts arrived with record schemas **4.2** and **4.3** (unclassified bases, relation source spans and the required cyclomatic metric). Cache entries written by
earlier versions are re-analyzed once. Records converted to the older
`SymbolInfo` model lose these facts, since that model has no field for them.

## Precise, filterable symbol reports

The declaration commands (`functions`, `types`, `symbols`, `classes`) gain
record selectors and new report shapes. Two changes alter existing output.

- **Name filters use smart case.** A pattern without an uppercase letter now
  ignores case; any uppercase letter makes it case-sensitive. Add `!` or
  `--exact` to match a whole name.

  ```text
  # before: `hug functions parse` was case-sensitive
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
  # after: `parse` also finds ParseError; `Parse` still finds only ParseError
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
    - public function ParseError() src/parse.rs:6:1
  ```

- **`hug symbols` hides locals.** Parameters, function-local variables and
  nested functions are `local` scope and are no longer listed by default
  (over this package, 4,635 of the old 16,681 lines remain).
  `--all-scopes` or `--scope local` brings them back; `functions` and `types`
  still list every scope.

  ```text
  # before: hug symbols src/parse.rs
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
    - parameter path src/parse.rs:1:21
    - let parsed src/parse.rs:2:9
    - public function ParseError() -> () src/parse.rs:6:1
  # after (add --all-scopes for the old list)
    - public function parse_config(path: &str) -> Config src/parse.rs:1:1
    - public function ParseError() src/parse.rs:6:1
  ```

- **New selectors:** `--kind`, `--scope`/`--all-scopes`, `--visibility`,
  `--exported`, `--in`, `--exclude-tests`/`--tests-only`, `--undocumented`,
  `--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params`, and
  `--file`/`--name`/`--exact` to say what a filter is. One selector path
  serves every command and output; a filtered run reuses the cache entry an
  unfiltered run wrote.
- **New outputs:** `--jsonl` (one self-describing line per declaration),
  `--count`, `--stats` (tables, or `--stats --json`), `--format plain|table`,
  `--tree` (members under their type) and `--signatures full|short|none`.
  Exactly one output mode is accepted per run (`--stats` composes with
  `--json` or a `--format`); `--json --json-full` together is now an argument
  error instead of silently meaning `--json-full`.
- **Records carry more facts.** Every record has a `scope` (`Item`, `Member`,
  `Local`), an `Async` modifier where declared, and a `qualified_name` in the
  language's own spelling (`.` for Java, `::` for Rust, …), prefixed with the
  file's path from the scan root (`src/shapes.rs#Point::norm`) unless a
  namespace or package roots it. `--json` and `--jsonl` always compute
  `is_test` and `is_deprecated`. Because IDs hash the qualified name, some
  record IDs changed; existing cache entries are re-analyzed once.
- **Interactive output wraps** at the terminal width with a hanging indent
  (`COLUMNS` overrides the width). Forced color (`CLICOLOR_FORCE=1`) counts as
  interactive, so it wraps too; `--plain`, `--format plain` and redirected
  output never wrap.
- **Source text cannot style the terminal.** Control characters in names,
  signatures and comments print as `\u{..}` escapes.

## Record-native symbol reports

The declaration commands (`functions`, `types`, `symbols`, `classes`) now
read the library's schema 4.0 records directly and analyze only what they
show.

- **Faster.** No listing runs lint, references or semantic analysis, files
  are analyzed in parallel, and a warm cache hit parses nothing. On this
  package, `hug functions --no-cache` went from 1.86 s to 0.12 s and a warm
  `hug functions` from 1.08 s to 0.04 s; peak memory of `hug symbols` over
  the whole monorepo fell from 3.48 GB to 1.95 GB.
- **New default order.** Declarations are listed by file, then source
  position, instead of alphabetically. `--sort name` and `--sort kind` give
  the other orders; `--sort-by-kind` and `--sort-by-module` are removed.

  ```text
  # before: alphabetical across files
    - public method area(self) -> f64 [in type Circle] src/shapes.rs:12:5
    - public struct Circle { radius: f64 } src/shapes.rs:3:1
  # after (default --sort file): file, then position
    - public struct Circle { radius: f64 } src/shapes.rs:3:1
    - public method area(self) -> f64 [in type Circle] src/shapes.rs:12:5
  ```

- **New JSON shape.** `--json` prints a versioned report holding only the
  command's records, with paths relative to `root_dir` and a trailing
  `errors` array. The old shape carried every file's full index, timestamps
  and absolute paths (39 MB for `hug --json functions` on this package, now
  2.7 MB on the same files), and its output changed on every run.

  ```text
  # before: {"schema_version":…,"files":[…],"symbol_indexes":[…every record…],…}
  # after:  {"report_schema_version":1,"schema_version":{"major":4,"minor":0},
  #          "root_dir":"…","files":[{"file":"src/shapes.rs","symbols":[…]}],"errors":[]}
  ```

- **`--json-full`** prints every file's complete index when you need it.
- **Labels come from the source's keyword.** `struct Meter`, `my $queue`,
  `local M` replace guesses such as `type Meter` or `= MAX_SIZE`.
- **A failed file no longer aborts the run.** It is printed to stderr, listed
  in `errors`, and makes `hug` exit `1`; the other files are still reported.
- **Quiet by default.** Cache problems are recoverable and silent;
  `--verbose` reports them with a cache and work stats line.

## Related pages

- [Using the `hug` CLI](./cli.md): every command and option as it works today
- [Symbol Reports](./symbol-reports.md)
- [Scope-Aware References](./references.md)
