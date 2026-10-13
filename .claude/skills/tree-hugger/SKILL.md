---
name: tree-hugger
description: Expert knowledge for multi-language symbol extraction using Tree-sitter. Use when working with the tree-hugger library or tree-hugger-cli (hug), extracting symbols/imports/exports, symbol records and schema, scope-aware references (`hug refs`), the persistent symbol cache, declaration reports and export formats (JSON, TSV, Markdown, ctags), implementing lint diagnostics, adding new language support, writing tree-sitter queries, or troubleshooting the underlying tree-sitter runtime (grammar versions, ABI mismatches, query debugging).
---

## Purpose

Tree Hugger provides Tree-sitter-based symbol extraction, scope-aware
references and diagnostics across 16 languages. The library package is
`tree-hugger` (`tree-hugger/lib/`); the CLI is `hug` (`tree-hugger/cli/`).

**Languages:** Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++,
Swift, Scala, PHP, Perl, Bash, Zsh, Lua.

## Read next, by task

| Task | Read |
|------|------|
| Change how kinds, visibility, `static`, headers, modifiers, generics, relations or metrics are extracted | [extraction.md](./extraction.md) |
| Change the query planner, `SymbolQuery`, scanner identity, parent index, occurrences or name matching | [analysis-pipeline.md](./analysis-pipeline.md) |
| Change the persistent cache, its key, or its validation | [cache.md](./cache.md) |
| Change a `hug` command, flag, output mode, `--changed` or `--stats` grouping | [cli-architecture.md](./cli-architecture.md) |
| Write or fix tests: fixtures, goldens, cache-upgrade rows, L2 terminal checks | [testing.md](./testing.md) |
| Look up a type or method signature | [api-reference.md](./api-reference.md) |
| Edit `.scm` queries, add a language, Neovim query reuse | [query-system.md](./query-system.md) |
| Lint rules, policy flags, ignore directives, external adapters, corpus gates | [diagnostics.md](./diagnostics.md) |
| Tree-sitter runtime, AST debugging, grammar/ABI versions | [tree-sitter-internals.md](./tree-sitter-internals.md) |

User-facing behavior is documented in `tree-hugger/docs/`; the skill files
describe implementation. When they disagree, the code wins and both get fixed.

- [Tree Hugger Overview](../../../tree-hugger/docs/overview.md): the entry page and map of every doc
- [Using the `hug` CLI](../../../tree-hugger/docs/cli.md): **the canonical CLI usage document**: every command, option, output format and exit status. `cli/README.md` is only an overview that links here
- [Symbol Reports](../../../tree-hugger/docs/symbol-reports.md): the declaration commands end to end, JSON envelopes, `--json-full`, `--sort`, `--changed`, `--stats`, exit status
- [Symbol Kinds, Visibility and `static`](../../../tree-hugger/docs/symbol-kinds-and-visibility.md): per-language kinds, containers, visibility, grammar limits
- [Re-exports](../../../tree-hugger/docs/reexports.md): the forwarding report per language, evidence, coverage states and limit reasons
- [Declared Type Relations](../../../tree-hugger/docs/type-relations.md): `extends`/`implements`/`unclassified_bases` per language
- [Scope-Aware References](../../../tree-hugger/docs/references.md): occurrences, confidence, coverage reasons, recursion, `hug refs`
- [Cyclomatic Complexity](../../../tree-hugger/docs/cyclomatic-complexity.md): counting rules, when the value is `null`
- [Cache Design](../../../tree-hugger/docs/cache-design.md): how the cache is used, with detail pages for the entry format and damaged-entry tables
- [Using the Library](../../../tree-hugger/docs/library.md): the Rust API guide (canonical; `lib/README.md` is only an overview)
- [Symbol Records](../../../tree-hugger/docs/symbol-records.md): the record format, field by field
- [Tree-sitter Queries](../../../tree-hugger/docs/queries.md) and [Testing](../../../tree-hugger/docs/testing.md)

## CLI at a glance (`hug`)

| Command | Purpose |
|---------|---------|
| `functions`, `types`, `symbols`, `classes` | Declaration reports (`classes` splits static/instance members) |
| `refs <symbol>` / `refs --id <id>` | References to one declaration in its file (`--callers` / `--callees`) |
| `imports` | Imported symbols |
| `lint` | Lint and syntax diagnostics (`--deny`/`--warn`/`--allow <rule>`, `--strict`, `--experimental-semantics`) |
| `god-files [DIR]` | Oversized files ripe for refactoring (`--high-risk`) |
| `completions` | Shell completions |

Declaration commands share:

- **Selectors** (AND across, OR within, smart-case names): `--file`, `--name`, `--exact`, `--kind`, `--scope`/`--all-scopes`, `--visibility`, `--exported`, `--in`, `--exclude-tests`/`--tests-only`, `--undocumented`, `--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params`, `--changed[=<ref>]`
- **Exactly one output mode**: default listing, `--json`, `--json-full`, `--jsonl`, `--count`, `--stats` (grouped by package), `--tree`, `--format plain|table|tsv|markdown|prose|ctags`
- **Ordering and detail**: `--sort file|name|kind`, `--signatures full|short|none`
- **Discovery**: `--exclude-files`, `--language`, `--no-cache`
- **Global** (any position): `--json`, `--json-full`, `--plain`, `--verbose` (cache stats and failures on stderr)

A positional directory (`src/`) selects no files; inputs are globs, so use
`'src/**'`. The full flag table is in
[cli-architecture.md](./cli-architecture.md#command-and-flag-reference).

## Library at a glance

```rust
let file = TreeFile::new("src/lib.rs")?;
file.symbols()?;            // All symbols
file.symbol_records()?;     // v2 records
file.symbol_index_v2()?;    // complete index: every pass, diagnostics included
file.query(&SymbolQuery::new().kinds([..]).capabilities([..]))?; // selective
SourceFile::read(path)?.query(&query)?; // read+hash once, parse on demand
file.imported_symbols()?;   // also exported_symbols, local_symbols, referenced_symbols
file.reference_evidence()?; // occurrences + region coverage
file.lint_diagnostics();    // also syntax_diagnostics, dead_code
```

**SymbolKind:** Function, Method, Constructor, Type, TypeAlias, Class,
Interface, Enum, EnumVariant, Trait, Module, Namespace, Variable, Parameter,
Field, Property, Macro, Constant, Unknown.

## Architecture in brief

- **Runtime**: Built on the tree-sitter Rust runtime (`tree-sitter = "0.26.3"`) plus one `tree-sitter-<lang>` grammar crate per language, all at independent versions — see [Tree-sitter Internals](./tree-sitter-internals.md)
- **Query vendoring**: Uses nvim-treesitter `locals.scm` in `lib/queries/vendor/<lang>/`
- **Custom queries**: `lint.scm`, `references.scm`, `comments.scm` per language
- **Capture naming**: `@local.definition.<kind>` for symbols, `@diagnostic.<rule-id>` for lint
- **Query caching**: Global `OnceLock<QueryCache>` for thread-safe caching
- **Builtin database**: Per-language builtin lists to avoid false positive undefined-symbol/undefined-module errors
- **Records**: facet-based `SymbolRecord` in `shared/schema_v2`, at `SchemaVersion::CURRENT` (see [analysis-pipeline.md](./analysis-pipeline.md#schema))
- **Extraction** (`lib/src/file/extract/`): kinds, containers, visibility and `static` flags are decided once, from each declaration node
- **Analysis** (`lib/src/analysis/`, `lib/src/file/query.rs`): one planner runs only the passes a `SymbolQuery` needs
- **Cache** (`lib/src/cache/`): one strictly validated entry per file, shared by every scan root
- **CLI** (`cli/src/`): parallel scan with an ordered sliding window; workers analyze and render, one thread writes

## Rules that are easy to miss

- **Bump `EXTRACTOR_REVISION`** (`lib/src/file/mod.rs`) with any change to extracted records, including repairs on an unreleased branch, and add the matching cache-upgrade row ([testing.md](./testing.md)).
- **A cache result must equal the uncached result.** Every wider entry is narrowed by `analysis::coverage::project`; a damaged entry is refused and recomputed, never served as empty ([cache.md](./cache.md)).
- **Nothing scan-root-dependent** may enter a stored record or a cache key, and display paths are never inputs to identity.
- **`hug` will not scan the repo's `fixtures/` directories**, so CLI tests copy fixtures into a scratch Git repo (`Scratch`).
- **Every styled view needs an L2 check** in the real-terminal targets, called from all three backends ([testing.md](./testing.md#real-terminal-l2-tests)).
- **Run checks from `tree-hugger/`**: `just test`, `just test-l2`, `just lint`.
- Known extraction limits and grammar gaps are listed in [extraction.md](./extraction.md#known-limitations).
