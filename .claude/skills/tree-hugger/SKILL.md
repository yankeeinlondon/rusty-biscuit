---
name: tree-hugger
description: Expert knowledge for multi-language symbol extraction using Tree-sitter. Use when working with tree-hugger-lib or tree-hugger-cli (hug), extracting symbols/imports/exports, implementing lint diagnostics, adding new language support, writing tree-sitter queries, or troubleshooting the underlying tree-sitter runtime (grammar versions, ABI mismatches, query debugging).
---

## Purpose

Tree Hugger provides Tree-sitter-based symbol extraction and diagnostics across 16 programming languages. Use this skill when:

- Working in `tree-hugger/lib/` or `tree-hugger/cli/`
- Extracting functions, types, imports, or exports from source files
- Implementing or debugging lint diagnostics
- Adding support for new languages or fixing query issues
- Understanding the query vendoring system

## Quick Reference

### Supported Languages

Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++, Swift, Scala, PHP, Perl, Bash, Zsh, Lua

### CLI Commands (`hug`)

| Command | Description |
|---------|-------------|
| `functions` | List functions and methods |
| `types` | List type definitions (struct, enum, class, interface, trait) |
| `symbols` | List all symbols |
| `imports` | List imported symbols |
| `classes` | List classes with static/instance member partitioning |
| `lint` | Run lint and syntax diagnostics |
| `lint --deny <rule>` | Promote rule to error |
| `lint --warn <rule>` | Promote rule to warning |
| `lint --allow <rule>` | Demote rule to info |
| `lint --strict` | Promote all warnings to errors |
| `lint --experimental-semantics` | Enable experimental semantic rules |
| `god-files [DIR]` | Identify oversized source files ripe for refactoring |
| `god-files --high-risk` | Show only high-risk files |
| `completions` | Generate shell completions (Bash, Zsh, Fish, PowerShell) |

### Library API (`TreeFile`)

```rust
let file = TreeFile::new("src/lib.rs")?;
file.source();              // Source text of the parsed file
file.comment_ranges()?;     // Byte ranges of comment nodes
file.symbols()?;            // All symbols
file.symbol_records()?;     // v2 parse records
file.symbol_index_v2()?;    // v2 parse->bind->semantic->docs pipeline output
file.imported_symbols()?;   // Imports
file.exported_symbols()?;   // Exports
file.local_symbols()?;      // Local definitions
file.referenced_symbols()?; // Identifier usages
file.lint_diagnostics();            // Pattern lint rules (with metadata)
file.syntax_diagnostics();          // Parse errors
file.dead_code();                   // Unreachable code blocks
file.experimental_semantics = true; // Enable experimental semantic rules
```

### SymbolKind Enum

Function, Method, Type, Class, Interface, Enum, Trait, Module, Namespace, Variable, Parameter, Field, Macro, Constant, Unknown

## Key Architecture

- **Runtime**: Built on the tree-sitter Rust runtime (`tree-sitter = "0.26.3"`) plus one `tree-sitter-<lang>` grammar crate per language, all at independent versions — see [Tree-sitter Internals](./tree-sitter-internals.md)
- **Query vendoring**: Uses nvim-treesitter `locals.scm` in `lib/queries/vendor/<lang>/`
- **Custom queries**: `lint.scm`, `references.scm`, `comments.scm` per language
- **Capture naming**: `@local.definition.<kind>` for symbols, `@diagnostic.<rule-id>` for lint
- **Query caching**: Global `OnceLock<QueryCache>` for thread-safe caching
- **Builtin database**: Per-language builtin lists to avoid false positive undefined-symbol/undefined-module errors
- **Schema v2**: `shared/schema_v2` with facet-based `SymbolRecord` and staged `FileSymbolIndex`
- **CLI filters**: `hug <subcommand> <filter...>` with all-files default scan for symbol commands. A pathless scan honors `.gitignore` and skips test-data dirs by default (`**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`); an explicit path/glob re-includes a targeted file. `lint` scans hide clean files (single-file lint still shows `(no diagnostics)`).

## Detailed Documentation

- [API Reference](./api-reference.md) - Complete type definitions and methods
- [Query System](./query-system.md) - Tree Hugger's `.scm` queries, capture conventions, and how to extend them
- [Diagnostics](./diagnostics.md) - Lint rules, ignore directives, dead code detection
- [Tree-sitter Internals](./tree-sitter-internals.md) - The underlying runtime: Parser/Tree/Node API, query mechanics, AST debugging, and grammar version/ABI compatibility

## Diagnostic Metadata (Phase 1)

Every diagnostic now carries stable metadata for policy and display:
- **Category**: `correctness`, `suspicious`, `style`, `performance`, `pedantic`, `restriction`, `experimental`
- **Confidence**: `high`, `medium`, `low`, `experimental`
- **Source**: `tree-sitter-query`, `semantic-analysis`, `syntax-parser`, `external-tool`
- **Severity**: `default_severity` and `effective_severity` (after policy)

## Rule Registry

Built-in rules are registered in `RuleRegistry` with metadata:
- Rule id, version, title, category, default severity, confidence
- Language support, default enablement, examples, caveats
- `requires_experimental_semantics` gates low-confidence semantic rules

Default-off pattern rules are silent unless opted in with `--warn <rule>` or `--deny <rule>` (the registry's `enabled_by_default` flag is enforced by the CLI policy layer, `apply_lint_policy`):
- Debug artifacts: `console-log`, `print-call`, `fmt-println`.
- `restriction` category: `unwrap-call`, `expect-call` — valid, deliberate constructs, not anomalies. Enable per-rule or via `--deny category:restriction`. `--allow`/`--strict` never enable an off-by-default rule.

Experimental rules (`undefined-symbol`, `unused-symbol`, `undefined-module`) are disabled by default. Enable with `TreeFile::experimental_semantics = true` or `--experimental-semantics` in the CLI.

## Dead Code Detection

Dead code is detected only within the same block as a terminal statement (`return`, `panic!`, `break`, etc.). Terminal statements inside nested control-flow constructs (e.g., an `if` branch, a `match` arm, or a closure) do not propagate dead-code detection to the enclosing function or method body.

## CLI Policy

Policy selectors target rules by ID or category (`category:correctness`):
- `--deny <rule>`: promote to error
- `--warn <rule>`: promote to warning
- `--allow <rule>`: demote to info
- `--strict`: promote all warnings to errors (does not enable experimental rules)
- Syntax diagnostics are always errors and always fail the exit code

## Testing Requirements

When modifying queries or symbol extraction:

1. Every language with type constructs must have type distinction tests
2. All typed languages need `types.*` fixture files
3. Bug fixes require regression tests
4. Run `cargo test -p tree-hugger-lib` to verify

The CLI's real-terminal `god-files` test is behind the internal
`terminal-tests` feature. Ordinary local L1 omits its harness and target;
`just test-l2` and CI enable it.

## Resolver and Adapter Architecture (Phase 3)

### Semantic Resolvers

The `resolver` module defines interfaces for project-level semantic analysis:
- `ProjectContext`: Root path, manifests, language configs, dependency hints, generated-file markers, target environment
- `SemanticResolver` trait: Reports confidence and supported-scope metadata, returns `ResolverOutput` with diagnostics and resolved imports
- `ResolverScope`: `FileLocal`, `Module`, `Project`, `Workspace`

Resolvers are gated until corpus precision is measured. Low-confidence resolver-backed diagnostics use `DiagnosticConfidence::Low` or `Experimental`.

### External Diagnostic Adapters

The `adapter` module defines interfaces for delegating to external tools:
- `ExternalDiagnosticAdapter` trait: `name()`, `version()`, `is_available()`, `supported_languages()`, `run()`
- `AdapterConfig`: Tool path, config path, extra args, environment, strict mode, timeout
- `AdapterResult`: Normalized diagnostics + `AdapterMetadata` (tool name, version, config files, exit status, elapsed time, cache status, fix availability)
- `AdapterError`: `ToolNotFound`, `IncompatibleVersion`, `ExecutionFailed`, `ParseFailed`, `UnsupportedLanguage`, `ConfigError`

Adapters preserve source tool rule IDs and report fix availability without applying fixes.

### Oxlint Adapter

`OxlintAdapter` is the first external adapter for JavaScript/TypeScript:
- Discovers `oxlint` via explicit config -> project-local `node_modules/.bin/oxlint` -> `PATH`
- Parses Oxlint JSON output into normalized `Diagnostic` values with `DiagnosticSource::ExternalTool`
- Maps Oxlint severity and categories to Tree Hugger equivalents
- Falls back to native diagnostics when Oxlint is unavailable
- In non-strict mode, missing tools produce empty results with metadata rather than errors

## Corpus and Default-On Gates (Phase 4)

The `corpus` module provides infrastructure for measuring rule precision against real-world code:

### Corpus Harness

- **`CorpusManifest`**: Defines repositories/fixtures with metadata (SHA, license, paths, rules, oracles)
- **`CorpusTier`**: `Smoke` (fast PR checks), `Expanded` (scheduled CI), `Benchmark` (performance timing)
- **`CorpusResult`**: Analysis output with per-rule threshold reports
- **`ThresholdReport`**: Tracks diagnostic counts against rule thresholds (`Zero`, `Budget(n)`, `Unlimited`)

### Snapshot Stabilization

- **`redact_snapshot_text()`**: Redacts absolute paths, temp dirs, tool locations
- **`normalize_diagnostics()`**: Sorts and deduplicates for stable comparison
- **`compact_diagnostic_text()`**: Produces compact stable representations
- **`strip_ansi_codes()`**: Removes terminal escape sequences

High-confidence syntax rules require `Threshold::Zero` corpus false positives. Experimental semantic rules use `Threshold::Unlimited` until resolver-backed precision is proven.

## Caching (Phase 5)

The `cache` module provides multi-tier caching for parse trees, symbol indexes, and diagnostics:

### Pass-Specific Fingerprints

`PassFingerprint` includes all inputs that affect a pass output:
- Schema version, tree-hugger version, grammar version/id
- Query hash, rule metadata hash, enabled rules hash, config hash
- External tool version (for adapter-backed passes)

### Cache Units

Cache entries are split by analysis stage for granular invalidation:
- `Parse`: Source text and grammar fingerprint
- `Symbols`: Symbol records from parse pass
- `Imports` / `Exports`: Relation records from bind pass
- `References`: Identifier usages
- `Diagnostics`: Lint and syntax diagnostics
- `Comments`: Ignore directives
- `ProjectGraph`: Project-level structures

### In-Process Cache

`InProcessCache` stores parsed file snapshots and symbol indexes in memory for the duration of a command:
- `get_parse_tree()` / `put_parse_tree()` — reuse parse trees
- `get_symbol_index()` / `put_symbol_index()` — reuse symbol indexes
- `record_hit()` / `stats()` / `hit_history()` — cache hit/miss tracking with timing

### Persistent Cache

`PersistentCache` stores serialized entries on disk under a tool-owned temp directory:
- Enabled by default; disabled with `--no-cache`
- Recomputes on any fingerprint mismatch or corrupt entry
- `invalidate()` for single entries, `invalidate_all()` for full cache clear

### Cache Invalidation Reasons

`CacheInvalidationReason` identifies why a cache entry was rejected:
- `SourceChanged`, `QueryChanged`, `GrammarChanged`
- `RuleMetadataChanged`, `ConfigChanged`, `ExternalToolChanged`
- `CorruptEntry`, `PassOptionsChanged`, `CacheDisabled`

## Neovim Query Reuse (Phase 6)

### Query Provenance

Every query carries provenance metadata (`QueryProvenance`) tracking:
- **Source project**: e.g., `nvim-treesitter` for vendored queries, `tree-hugger` for originals
- **Upstream revision**: Tag or commit SHA when vendored
- **Original path** and **local path** for audit trails
- **License**: SPDX identifier (e.g., `Apache-2.0`)
- **Translation status**: `Unchanged`, `PredicatesAdapted`, `CapturesRenamed`, `MultipleChanges`, `Original`

Access via `tree_hugger::query_provenance(language, kind)` or `all_query_provenance()`.

### Nvim-treesitter Inventory

`QueryInventory` records capture groups from all upstream query suites:
- **Locals** (`locals.scm`): Reused by Tree Hugger for symbol definitions and references
- **Highlights** (`highlights.scm`): Inventoried but not yet reused
- **Injections** (`injections.scm`): Inventoried for future language-injection support
- **Folds** (`folds.scm`): Inventoried for future folding support
- **Indents** (`indents.scm`): Inventoried for future indentation support

`CaptureEntry` tracks whether a capture is reused and includes translation notes.

### Predicate and Directive Compatibility

`CompatibilityRegistry` documents how Neovim predicates/directives behave in the tree-sitter Rust runtime:
- **Native**: `eq?`, `match?`, `lua-match?`, `any-of?`, `contains?`, `any-contains?`, `set!`, `offset!`
- **RequiresHook**: `has-parent?`, `is?`, `has-ancestor?`, `not-has-ancestor?` — use named post-processing hooks
- **Unsupported**: `make-range!`, `strip!` — must not be silently dropped

Use `find_unsupported_predicates(query_text)` and `find_hook_predicates(query_text)` to audit queries.

### Upstream Drift Detection

`check_query_drift(language, kind, current_revision)` and `check_all_drift()` report when vendored queries diverge from upstream:
- Detects newer upstream revisions
- Flags content mismatches and capture divergences
- Only applies to vendored queries (not original overlays)

`summarize_drift()` produces human-readable reports.

## Known Limitations

- **Swift**: All types captured as `SymbolKind::Type` (grammar limitation)
- **Go**: Struct and interface both captured as `SymbolKind::Type`
