# Detailed Review: Tree Hugger Library and CLI

Reviewed on 2026-03-16.

## Scope

This review covers:

- `tree-hugger/lib`
- `tree-hugger/cli`
- query assets under `tree-hugger/lib/queries`
- existing design docs, especially the v2 schema and Neovim/Tree-sitter notes

The focus was:

- better ergonomics and maintainability
- better performance on real codebases
- richer symbol metadata for static analysis
- how well the current query strategy aligns with Neovim and Tree-sitter

## What I Verified

- `just test` passes for the package on 2026-03-16.
- `just hug symbols lib/tests/fixtures/sample.ts --json` reproduces multiple analysis issues:
  - `Greeter` is not emitted as a class symbol
  - `Greeter` is reported as an undefined symbol on its own class declaration
  - exported symbols include `constructor` and the instance method `greet`
  - every symbol in the v2 index inherits the same `readFile` dependency edge
- `just hug classes lib/tests/fixtures/sample.ts --plain` renders only `type GreetFn`, which confirms that the class command is currently operating on the wrong symbol set for TypeScript.

## Executive Summary

Tree Hugger already has three strong foundations:

- broad language coverage
- a good fixture/test culture
- a v2 schema that aims higher than the original `SymbolInfo` model

The main problem is that the implementation is still mostly v1-shaped while the public intent is v2-shaped. The result is that the current library can extract useful summaries, but the richer graph-oriented metadata is not yet trustworthy enough for precise code intelligence, API modeling, or repository-wide static analysis.

The biggest issues are:

1. the bind/semantic passes do not currently model real symbol-level edges
2. the query strategy depends too heavily on vendored `locals.scm` files as if they were a stable analysis contract
3. TypeScript/JavaScript class and export handling is incomplete or incorrect
4. many v2 facets exist in the schema but are not populated by the parser
5. the CLI repeatedly re-runs the same analysis work instead of consuming one file analysis result

## Priority Findings

### 1. The v2 bind and semantic passes are currently semantically invalid

Evidence:

- `lib/src/analysis/mod.rs:101-149`
- `lib/src/analysis/mod.rs:152-193`

What is happening:

- `bind_pass` clears `symbol.relations.references` and then pushes every file import into every symbol's `references` list.
- `dependencies` is then copied directly from that same file-global import list.
- `referenced_by` is populated as a self-edge whenever a symbol name appears anywhere in the file.
- `semantic_pass` computes `is_recursive` and `may_panic` from those invalid relation edges.

Why this matters:

- the relation graph is misleading, not merely incomplete
- recursion detection is not meaningful
- panic/throw flags are not tied to the actual body of the symbol
- any downstream feature that trusts `relations` or `semantics` will be built on bad data

Concrete example:

- In `sample.ts`, every symbol in the JSON index inherits a dependency on `readFile`, even symbols that do not reference it.

Recommendation:

- Introduce an explicit occurrence model.
- Track declaration, reference, import, export, and doc occurrences with spans and enclosing-symbol IDs.
- Build `relations.references`, `relations.dependencies`, and `relations.referenced_by` from occurrences scoped to the owning symbol, not from file-global name sets.
- Treat the current semantic pass as a placeholder until that graph exists.

### 2. Reference queries are too broad and create false positives

Evidence:

- `lib/queries/typescript/references.scm:1-27`
- `lib/queries/rust/references.scm:1-24`
- `lib/src/file/tree_file.rs:1547-1636`
- `lib/src/file/tree_file.rs:984-1319`

What is happening:

- The TypeScript references query captures every `(identifier)` and `(type_identifier)`.
- The Rust references query does the same for several broad node types.
- These files claim to capture usages, but they do not exclude declaration sites.

Observed failure:

- Running `just hug symbols lib/tests/fixtures/sample.ts --json` reports `Reference to undefined symbol 'Greeter'` on `export class Greeter {`.

Why this matters:

- undefined-symbol diagnostics become noisy
- unused-symbol/import analysis is less trustworthy
- any future call graph or rename support will have the same precision problem

Recommendation:

- Stop treating a broad `identifier` query as a complete reference system.
- Either:
  - derive references from a normalized symbol/occurrence query and subtract definition spans, or
  - use language-specific reference queries with structural exclusions for declarations, parameter sites, and type declarations.
- Prefer role-aware occurrences such as `value_ref`, `type_ref`, `module_ref`, `call_target`, `member_ref`.

### 3. TypeScript and JavaScript class coverage is incomplete

Evidence:

- `lib/queries/vendor/ecma/locals.scm:19-53`
- `lib/queries/vendor/typescript/locals.scm:1-43`
- `lib/src/shared/symbol.rs:240-243`
- `cli/src/main.rs:2699-2795`

What is happening:

- The vendored ECMA locals query does not capture `class_declaration`.
- The TypeScript overlay adds interfaces, type aliases, and enums, but still not classes.
- `SymbolKind::is_class()` treats `Type` as class-like, which causes the CLI to use type aliases and struct-like types as stand-ins.

Observed failures:

- `Greeter` is missing from `hug symbols ... sample.ts --json`
- `hug classes ... sample.ts --plain` renders `type GreetFn` instead of the actual class

Why this matters:

- the CLI command most directly aligned with OO structure is not reliable for JS/TS
- properties, constructors, and class-level metadata are blocked upstream by missing captures

Recommendation:

- Add explicit JS/TS captures for:
  - `@local.definition.class`
  - `@local.definition.method`
  - `@local.definition.field` or `@local.definition.property`
  - constructor definitions
- Narrow `is_class()` so the class command operates on real class-like symbols only.
- If you want a broader “member-bearing type” view, make that an explicit separate mode instead of overloading `classes`.

### 4. Export detection currently overstates the public API and collapses distinct symbols

Evidence:

- `lib/src/file/tree_file.rs:1857-1884`
- `lib/src/file/tree_file.rs:1467-1481`
- `lib/src/analysis/mod.rs:138-145`

What is happening:

- For non-Rust languages, `is_exported_definition` returns true as soon as the parent walk reaches the root node.
- That means top-level non-exported definitions in many languages are treated as exported.
- It also walks upward through class bodies, so members can become “exported” just because the class is exported.
- `symbol_records()` reduces export identity to a `HashSet<String>` of names, so any symbol sharing a name with an exported symbol can be marked exported.
- `bind_pass` repeats the same name-based export matching.

Observed failures:

- In `sample.ts`, the JSON export list includes `constructor` and the instance method `greet`.
- The top-level `greet` function and the instance method `greet` are not disambiguated by container when export status is assigned.

Why this matters:

- `--exported` becomes unreliable
- v2 visibility facets stop reflecting a real API boundary
- name collisions inside a file will poison export metadata

Recommendation:

- Make export analysis language-specific.
- Distinguish:
  - declaration visibility
  - top-level module export
  - re-export
  - default export
  - class member visibility
- Match exported symbols by stable symbol identity or qualified path, never by bare name alone.

### 5. Stable IDs are not actually stable across ordinary edits

Evidence:

- `lib/src/shared/schema_v2/mod.rs:79-95`
- `lib/src/shared/schema_v2/mod.rs:340-417`

What is happening:

- `stable_symbol_key` includes `declaration_start_byte`.
- A blank line or comment inserted above a symbol changes the byte offset and therefore changes `SymbolId`.

Why this matters:

- graph diffs across commits become noisy
- cached analysis invalidates too aggressively
- symbol IDs cannot serve as durable anchors for repository-wide indexing

Recommendation:

- Use a lexical identity scheme instead:
  - language
  - normalized file path
  - declaration kind
  - container chain
  - symbol name
  - optional sibling ordinal among same-name declarations in the same container
- Treat byte offsets as source location, not identity.

### 6. Container and qualified-name metadata is contaminated by self-container matches

Evidence:

- `lib/src/file/tree_file.rs:1562-1599`
- `lib/src/file/tree_file.rs:1703-1805`
- `lib/src/shared/schema_v2/mod.rs:344-417`

What is happening:

- `find_symbol_container` starts from the captured identifier node.
- For type/interface/enum name captures, the immediate parent is often the declaration node for the symbol itself.
- That declaration is then treated as the symbol’s container.

Observed failures:

- The TypeScript JSON output contains identities like:
  - `GreetingService::GreetingService`
  - `Status::Status`

Why this matters:

- `qualified_name`
- `module_path`
- `stable_key`
- relation edges

all become less meaningful.

Recommendation:

- Build container identity from the declaration context node, not the name node.
- Skip the owning declaration when searching for an enclosing container.
- Separate these concepts explicitly:
  - owning declaration
  - enclosing lexical scope
  - module/namespace path

### 7. The v2 schema is much richer than the populated data

Evidence:

- `lib/src/shared/schema_v2/kinds.rs:5-219`
- `lib/src/shared/schema_v2/mod.rs:340-460`

What is available in the schema but mostly unpopulated today:

- `Constructor`
- `Property`
- `TypeAlias`
- `EnumVariant`
- receiver metadata
- type ASTs
- doc spans
- body spans
- declaration/signature/body text
- inheritance and implementation edges
- field/property modifiers
- overload sets

Observed gaps:

- TypeScript constructors are still emitted as `Function`.
- Type aliases like `GreetFn` land in the generic `Type` bucket and then degrade to `kind_data_type = "Unknown"`.
- `SourceFacet` is mostly empty beyond the declaration span.
- attached docs reuse the declaration span instead of the actual comment span.

Recommendation:

- Add a parse-layer symbol taxonomy closer to `SymbolKindV2` instead of converting from the narrower v1 `SymbolKind`.
- Capture real `name_span`, `body_span`, and `doc_span`.
- Distinguish `TypeAlias`, `Constructor`, `Property`, and `EnumVariant` during parse, not after.

### 8. The CLI and library redo far too much work per file

Evidence:

- `cli/src/main.rs:801-816`
- `cli/src/main.rs:1376-1455`
- `lib/src/analysis/mod.rs:31-98`
- `lib/src/file/tree_file.rs:861-1516`
- `lib/src/cache/mod.rs`

What is happening:

- In JSON mode, the CLI builds `symbol_index_v2()` and then separately calls `summarize_file()`.
- `summarize_file()` separately requests lint, syntax, symbols, imports, exports, and locals.
- `symbol_index_v2()` internally repeats parse/export/import/diagnostic work again.
- The cache module exists, but nothing in the library or CLI uses it.

Why this matters:

- repeated Tree-sitter traversals
- repeated AST-to-symbol conversions
- repeated diagnostics work
- avoidable latency on large repositories

Recommendation:

- Build one `AnalyzedFile`/`FileSymbolIndex` per file and derive all render/JSON views from it.
- Wire the existing cache layer into the CLI scan path.
- After the per-file pipeline is consolidated, parallelize across files.

### 9. The class command uses line-range heuristics instead of semantic parentage

Evidence:

- `cli/src/main.rs:2699-2795`

What is happening:

- Classes are collected by symbol kind.
- Methods are assigned to the “nearest previous class” by line range.

Why this matters:

- nested types
- partial classes
- interleaved declarations
- languages with free functions between type members

all become hard to model correctly.

Recommendation:

- Use explicit parent/container relations from the analysis pipeline.
- Treat the current line-range strategy as a temporary UI hack, not a long-term data model.

### 10. Query ownership is currently ambiguous and too dependent on `locals.scm`

Evidence:

- `lib/src/queries/mod.rs:92-190`
- `lib/queries/vendor/ecma/locals.scm:35-47`
- `lib/queries/vendor/python/locals.scm:54-60`

Local observation:

- The vendored queries already contain metadata such as `#set! definition.var.scope parent`.
- `symbol_nodes()` only inspects capture names and ignores query properties entirely.

Neovim alignment concern:

- As of 2026-03-16, the upstream `nvim-treesitter` README says `locals.scm` queries are no longer used by the plugin itself and are kept mainly for backward compatibility.

Why this matters:

- Tree Hugger is currently treating `locals.scm` as a stable analysis substrate when upstream is not promising that level of stability.
- You are also leaving useful query metadata on the table.

Recommendation:

- Keep vendored upstream queries as a base layer, pinned to an explicit upstream commit.
- Add a Tree Hugger-owned overlay query layer for analysis-specific captures.
- Support both `inherits` and `extends` modelines so overlays do not require patching vendored files.
- Start consuming query property settings where they already exist.

### 11. The loader only implements part of the Neovim query modeline behavior

Evidence:

- `lib/src/queries/mod.rs:134-190`

What is happening:

- `split_inherits()` only recognizes `inherits:`.
- There is no support for `extends`.
- There is no notion of overlaying multiple query files for the same language and query kind.

Why this matters:

- you have to edit vendored queries directly to extend them
- local customizations become harder to track against upstream
- you cannot adopt the same layering discipline Neovim uses

Recommendation:

- Mirror the Neovim mental model:
  - vendored upstream query
  - Tree Hugger overlay query
  - optional language-family overlay
- Resolve modelines before compile and treat the final string as a build artifact.

### 12. There are practical codebase-analysis gaps beyond pure correctness

Evidence:

- `lib/src/shared/symbol.rs:73-92`
- `cli/src/main.rs`
- `lib/src/file/tree_file.rs`

Examples:

- `TypeScript` support does not include `.tsx` even though the dependency provides `LANGUAGE_TSX`.
- `cli/src/main.rs` is nearly 3,000 lines.
- `lib/src/file/tree_file.rs` is over 5,500 lines.

Why this matters:

- modern frontend repositories frequently depend on TSX
- adding new language features becomes slower and riskier when parsing, semantics, and formatting live in monolithic files

Recommendation:

- Add TSX support.
- Split extraction by concern and then by language family.
- Separate scan/filter/render code in the CLI into dedicated modules.

## Metadata That Should Be Captured Next

If Tree Hugger is meant to become the repository’s static-analysis backbone, these fields will pay off quickly:

- declaration category:
  - constructor
  - property
  - type alias
  - enum variant
  - package/namespace/module
- source layout:
  - `name_span`
  - `doc_span`
  - `body_span`
  - `signature_text`
  - optional `declaration_text`
- relation edges:
  - parent/container
  - extends/implements
  - overrides
  - re-exports
  - imports by source path and original name
  - type references separate from value references
- type-system detail:
  - generic parameter bounds
  - generic defaults
  - variance where the language exposes it
  - receiver/ownership mode
  - nullability
  - alias target
- modifiers and attributes:
  - async
  - static
  - readonly
  - abstract
  - final
  - sealed
  - open
  - partial
  - unsafe
  - mut
  - decorators/annotations/attributes/macros
- documentation:
  - attachment kind
  - multi-line tag parsing
  - `@throws`, `@deprecated`, `@see`, `@example`
  - doc comments on fields, variants, parameters, and properties

## Recommended Architecture Direction

### Short term

1. Fix TypeScript/JavaScript class captures, property captures, and reference false positives.
2. Make export detection language-aware and container-aware.
3. Stop matching exported symbols by bare name.
4. Rebuild `classes` on container relations instead of line ranges.
5. Add regression tests for the exact issues reproduced above.

### Medium term

1. Introduce an occurrence graph:
   - definition
   - reference
   - import
   - export
   - doc attachment
2. Derive `FileSummary` and CLI output from one analyzed file result.
3. Start using the existing cache module in the scan path.
4. Parallelize file analysis once the per-file pipeline is deterministic.

### Longer term

1. Replace the v1-first parse layer with a v2-first parse layer.
2. Move language-specific extraction into dedicated modules.
3. Treat vendored Neovim queries as an upstream source, not as the final analysis contract.
4. Add query overlay tooling so new captures can be layered without forking upstream files by hand.

## Test Coverage I Would Add Next

- A regression test proving `sample.ts` emits `Greeter` as a class symbol.
- A regression test proving no definition site is reported as `undefined-symbol`.
- A regression test proving exported top-level functions do not automatically export same-name methods.
- A regression test proving class methods are not exported just because the class is exported.
- A regression test proving `classes` on TypeScript returns `Greeter`, not `GreetFn`.
- A regression test proving stable IDs do not change after whitespace-only edits above a declaration.
- Query compile tests for locals and references, not only lint/comments.

## Suggested Source Layout Refactor

The current file sizes are a maintenance smell:

- `cli/src/main.rs`: 2,982 lines
- `lib/src/file/tree_file.rs`: 5,570 lines

I would split them like this:

- `lib/src/file/parse.rs`
- `lib/src/file/imports.rs`
- `lib/src/file/references.rs`
- `lib/src/file/docs.rs`
- `lib/src/file/types.rs`
- `lib/src/file/lint.rs`
- `lib/src/file/languages/<family>.rs`
- `cli/src/scan.rs`
- `cli/src/filter.rs`
- `cli/src/render.rs`
- `cli/src/json.rs`
- `cli/src/prelude.rs`

That refactor is not just cosmetic. It will make it materially easier to raise the fidelity of the analysis without constantly touching one giant file.

## Neovim and Tree-sitter Notes

The best way to use Neovim’s query ecosystem here is not to copy it verbatim and assume it is already a full static-analysis schema. It is better to:

- vendor upstream queries at a pinned revision
- preserve them as the compatibility/input layer
- own a Tree Hugger analysis overlay that adds the exact captures needed for symbol indexing
- support query layering semantics that match Neovim’s documented modelines
- harvest query property metadata where upstream already provides it

That approach keeps Tree Hugger compatible with the ecosystem without making its analysis quality depend on upstream editor-focused tradeoffs.

## External Sources

- [nvim-treesitter README](https://github.com/nvim-treesitter/nvim-treesitter)
- [Neovim Tree-sitter query modelines](https://neovim.io/doc/user/treesitter.html#treesitter-query-modelines)
- [Neovim Tree-sitter query API](https://neovim.io/doc/user/treesitter.html#vim.treesitter.query)
- [Tree-sitter query syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html)
