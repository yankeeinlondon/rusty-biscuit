# Tree Hugger Lint Review

Date: 2026-06-02

Scope: `tree-hugger` library and `tree-hugger-cli` lint behavior.

## Current State

The lint command currently combines:

- syntax diagnostics from Tree-sitter `ERROR` and missing nodes
- pattern query rules from `queries/*/lint.scm`
- file-local semantic rules: `undefined-symbol`, `undefined-module`, `unused-symbol`, `unused-import`, and `dead-code`
- ignore directives parsed through language comment queries

Pattern coverage is very small: Rust `unwrap`, `expect`, `dbg`; JavaScript/TypeScript `debugger`, `eval`; Python `eval`, `exec`, `breakpoint`; PHP `eval`. All other languages rely on generic semantic checks and syntax diagnostics.

I also ran small probes through `cargo run -p tree-hugger-cli -- --json lint ...`. They confirmed several common false positives:

- `import React from "react";` reported `React` as `undefined-symbol`.
- `use serde::Deserialize;` reported `serde` as `undefined-symbol`.
- `#[derive(Deserialize)]` reported `derive` as `undefined-symbol`.
- `serde_json::from_str(...).unwrap()` reported both `undefined-symbol` and `undefined-module` for `serde_json`.
- `render_template("index.html", user=current_user)` reported the keyword argument name `user` as `undefined-symbol`.
- a Python route function was reported as `unused-symbol` even though route functions are often externally registered.
- a Rust struct field was reported as `unused-symbol` even though fields are often used by serialization, reflection, FFI, templates, or external callers.

The conclusion is blunt: the current lint results are useful as analyzer development signals, but several should not be presented as production-grade lint errors.

## Opportunity: Reclassify Current Semantic Errors as Experimental Warnings

Recommendation: true
Impact: high
Level of effort: low

Benefit: This reduces false authority. Today `undefined-symbol` is an error even though it can be wrong for ordinary code. For example, a valid Rust file that references an external crate path can report `serde` or `serde_json` as undefined because the analyzer does not consult `Cargo.toml`, crate graph, macro expansion, or rustc name resolution. In Python, a keyword argument such as `user=` can be captured as an identifier reference even though it is a call-site parameter name, not a variable read.

Before implementation:

- Decide whether to demote only file-local semantic rules or all non-syntax diagnostics.
- Consider a `confidence` or `source` field so CLI output can say `experimental semantic`.
- Decide if `--strict` should restore error severity for teams that deliberately accept false positives.
- Add regression tests around common false positives before changing severity.

### Design details

Add explicit diagnostic metadata instead of relying on the rule name to imply trust. Extend the diagnostic schema with a stable `category` such as `syntax`, `semantic`, or `lint`; a `confidence` such as `high`, `medium`, or `experimental`; and keep `severity` as the user-facing outcome. Current file-local semantic rules, including `undefined-symbol` and `undefined-module`, should default to `category: semantic`, `confidence: experimental`, and `severity: warning`. Syntax diagnostics should remain errors with high confidence.

The CLI should render experimental semantic findings as warnings by default, with wording that makes the source of uncertainty visible, for example `warning[experimental semantic]`. JSON output should include the new metadata fields so downstream tools can distinguish a warning caused by weak semantic analysis from a stylistic lint warning. A `--strict` mode can promote experimental semantic warnings back to errors for teams that intentionally gate on these diagnostics, while preserving the metadata that explains why the rule is approximate.

This is a schema-visible change, so update snapshot and serialization tests for both human and JSON output. Add regression fixtures for known false positives before the severity change: Rust external crate paths such as `serde_json::...`, Rust macro-influenced names where practical, and Python keyword arguments such as `user=` at call sites. Migration should be non-breaking for consumers that only read message text and severity, but structured consumers must tolerate the additional fields and the changed default severity for existing semantic rule IDs.

Open design decision: whether only undefined-name style rules should become experimental semantic warnings, or whether every non-syntax diagnostic should first be assigned a category and confidence before severity changes are applied.

## Opportunity: Split Lint Categories Explicitly

Recommendation: true
Impact: high
Level of effort: medium

Benefit: Current output has `Lint`, `Semantic`, and `Syntax`, but rule behavior actually spans different trust levels. A better taxonomy would let users filter and understand results:

- Syntax: parser errors and missing nodes.
- Unsafe/debug patterns: `eval`, `exec`, `debugger`, `dbg`, `breakpoint`.
- Panic/error handling patterns: `unwrap`, `expect`.
- File-local reachability: `dead-code`.
- File-local name heuristics: current undefined/unused rules.
- Project/module semantics: import resolution, exports, public API use, package manifests.
- Style/convention: naming, redundant constructs, formatting-adjacent rules.
- Security: injection-prone APIs, shell execution, unsafe deserialization.
- Performance: avoidable allocations, inefficient loops, known slow APIs.
- Compatibility/deprecation: runtime or language-version-specific issues.
- Framework/domain rules: React hooks, test rules, accessibility, web platform, Python web frameworks.

Example: `debugger;` is a real high-confidence pattern lint in JavaScript. `route()` being unused in a Flask/FastAPI-style file is not the same kind of claim. They should not sit in the same CLI bucket without qualification.

### Design details

Introduce a stable `LintCategory` enum in the library schema rather than deriving categories from display strings. Start with syntax, pattern, reachability, file-local-name, project-semantics, style, security, performance, compatibility, and framework/domain categories, and treat future additions as schema-versioned changes. Each diagnostic should also carry confidence metadata (`high`, `medium`, `low`, `experimental`) and source metadata that identifies whether the finding came from parser state, a Tree-sitter query, a file-local heuristic, a project resolver, or a delegated external tool.

Keep public API values machine-oriented and stable, while allowing the CLI to render friendlier labels and grouped headings. JSON output should remain backward compatible by keeping existing fields and adding optional `category`, `confidence`, and `source` fields first; a later schema version can make them required after downstream consumers have had a migration window. Filters should operate on stable enum values, not labels, and `hug lint` should default to high-confidence syntax and pattern findings while gating low-confidence file-local semantic heuristics behind explicit flags or strict mode. Tests should cover JSON compatibility, category filters, CLI labels, and default behavior, and docs should define the category taxonomy with examples of rules that belong in each bucket.

Before implementation:

- Define a stable enum for category and confidence.
- Decide which categories are public API versus CLI display labels.
- Decide defaults for `hug lint`, `hug lint --lint-only`, and `hug lint --syntax-only`.
- Update JSON schema and docs once the categorization is stable.

## Opportunity: Disable or Gate File-Local Undefined/Unused Rules by Default

Recommendation: true
Impact: high
Level of effort: medium

Benefit: File-local undefined and unused checks need scope and module resolution that we do not have yet. They are noisy in languages with external dependencies, macros, decorators, framework entry points, dynamic globals, reflection, generated uses, and cross-file exports. Gating them behind `--experimental-semantics` or `--file-local-semantics` would make the default lint command more trustworthy.

Examples:

- Rust external crate paths should be resolved from Cargo metadata, extern prelude, `use` trees, and macro expansion context.
- Python decorators and route functions can be externally invoked.
- JavaScript default imports, globals, JSX, TSX, and bundler-provided symbols need parser and project context.
- Java/C#/Go symbols need package/module/classpath/package import semantics.

### Design details

Default `hug lint` should not report file-local semantic rules that require project context. Disable `undefined-symbol`, `undefined-module`, and broad `unused-symbol` checks by default unless a language-specific rule has proven file-local precision. Gate the disabled rules behind `--experimental-semantics` for all experimental semantic checks, with `--file-local-semantics` as the narrower flag if the CLI wants to expose only this rule family; the matching config key should be `lint.experimental_semantics = true` or `lint.file_local_semantics = true`. `--strict` should continue to raise enabled warnings to errors, but it should not implicitly enable disabled semantic rules; users who want both would pass `--strict --experimental-semantics`.

Suppressed or disabled rules should stay out of the human default output. JSON output can include an optional `disabled_rules` or `suppressed_rules` summary when a verbosity flag is requested, but it should not emit fake diagnostics because that would make CI contracts ambiguous. The rule scope should be per language and per rule, so Rust could later enable a narrower undefined-module check without enabling JavaScript unused-symbol checks. Migration should update CLI help, config docs, snapshots, and fixtures to assert that default linting is quieter while opt-in semantic diagnostics remain visible. Open decisions: whether the public flag should use the broader `--experimental-semantics` name, whether JSON should expose disabled-rule metadata by default, and what measured false-positive threshold is acceptable before any file-local rule becomes default-on.

Before implementation:

- Measure false-positive rate on representative repositories.
- Decide a minimum signal threshold for default-on rules.
- Add language-specific fixtures with real framework and package examples.
- Decide whether `unused-symbol` should ever report fields, parameters, exported functions, public methods, or top-level symbols without project context.

## Opportunity: Replace JavaScript/TypeScript Linting with Oxlint Integration

Recommendation: true
Impact: high
Level of effort: medium

Benefit: Oxlint is purpose-built for JavaScript and TypeScript linting on the Oxc compiler stack. Its docs state it is built for scale, reports benchmarks of 50-100x faster than ESLint, includes 801 built-in rules, supports JavaScript/TypeScript/JSX/TSX plus script blocks in Vue/Svelte/Astro, supports automatic fixes, inline ignores, multi-file analysis, and type-aware linting. Current Tree Hugger JS/TS lint has two pattern rules plus generic semantic heuristics.

Concrete improvement:

- Current Tree Hugger catches `debugger` and direct `eval()`.
- Oxlint can cover correctness, suspicious, performance, style, restriction, nursery, ESLint core, TypeScript, React, Jest, Vitest, import, Unicorn, jsx-a11y, Next.js, promise, node, Vue, and other plugin-derived rules.
- Type-aware Oxlint can report rules such as floating promises and unsafe assignments through `tsgolint`; current Tree Hugger cannot infer TypeScript types.
- Oxlint understands JSX/TSX and JS ecosystem config better than Tree-sitter reference captures.

This should be integration, not a full replacement of Tree Hugger. Tree Hugger should keep symbol extraction and multi-language structural analysis. For JS/TS lint, delegate to Oxlint where available and normalize diagnostics into Tree Hugger's schema.

Before implementation:

- Verify current Oxlint CLI JSON format, exit-code behavior, ignore semantics, config discovery, and diagnostic spans.
- Decide whether Oxlint is an optional runtime dependency, a feature-gated Rust crate dependency, or a subprocess adapter.
- Decide fallback behavior when Oxlint is absent.
- Map Oxlint rule categories and severities into Tree Hugger categories.
- Research whether automatic fixes should be exposed or intentionally omitted.
- Benchmark on this repo and at least one large JS/TS repo.

### Design details

Keep the Oxlint boundary behind a JS/TS diagnostic adapter, not inside Tree Hugger's query pipeline. The adapter should accept resolved file paths plus the lint configuration context, invoke Oxlint only for JavaScript, TypeScript, JSX, TSX, and supported script-block inputs, then translate Oxlint diagnostics into Tree Hugger diagnostics without changing symbol extraction.

Prefer an optional subprocess strategy for the first implementation. Discover an `oxlint` executable on `PATH` or from an explicit config/CLI setting, keep the core library usable without linking Oxc crates, and make dependency failures observable without requiring JS tooling for non-JS users. If a native Rust integration later becomes stable and materially faster, it can replace the subprocess behind the same adapter trait.

Config discovery should follow Oxlint's own behavior wherever possible. Tree Hugger should pass the target files and working directory through, optionally allow an explicit Oxlint config path, and avoid reimplementing ignore, plugin, or type-aware project discovery semantics. JSON output must be treated as the adapter contract: parse spans, rule names, messages, severities, suggestions, and fix metadata into a versioned internal shape before creating Tree Hugger diagnostics. Span mapping should preserve byte offsets when Oxlint exposes them and fall back to line/column ranges with UTF-8-aware conversion.

Severity and category mapping should be explicit and tested. Oxlint errors map to Tree Hugger errors only for rules Oxlint classifies as errors; warnings remain warnings. Rule families should map into stable categories such as syntax, correctness, suspicious, performance, style, accessibility, framework, test, import, and type-aware, while preserving the original Oxlint rule id and source as metadata for filtering and JSON consumers.

When Oxlint is unavailable, default to a non-fatal capability warning and keep existing Tree Hugger JS/TS syntax diagnostics and conservative pattern linting. A strict mode may convert missing Oxlint into a command failure for CI. Automatic fixes should not be applied in the initial integration; expose fix availability in metadata only, and add an explicit future command or flag if Tree Hugger later owns file mutation.

Benchmarks should compare current Tree Hugger lint, Oxlint subprocess cold start, and warm multi-file runs on this repo plus a large JS/TS project. Tests should cover adapter parsing with recorded JSON fixtures, path and span normalization, severity/category mapping, missing-binary fallback, strict-mode failure, explicit config path handling, and preservation of existing non-JS lint behavior.

Open decisions:

- Whether `--strict` should fail when Oxlint is absent for all invocations or only when JS/TS files are selected.
- Whether Tree Hugger should expose Oxlint's type-aware mode as an explicit flag or defer entirely to Oxlint config discovery.
- Whether fix metadata belongs in the stable diagnostic schema immediately or should remain adapter-internal until file mutation is supported.

References:

- https://oxc.rs/docs/guide/usage/linter
- https://oxc.rs/docs/guide/usage/linter/plugins
- https://oxc.rs/docs/guide/usage/linter/type-aware.html
- https://oxc.rs/docs/contribute/linter

## Opportunity: Use Neovim and nvim-treesitter More Systematically

Recommendation: true
Impact: medium
Level of effort: medium

Benefit: Tree Hugger already vendors nvim-treesitter `locals.scm`, but Neovim's Tree-sitter ecosystem is broader than locals. It has mature query conventions for highlights, injections, folds, indents, locals, predicates, directives, modelines, and runtimepath query extension. The current query loader supports inheritance/extends-like modelines for locals overlays, but lint/reference/comment queries appear to be entirely local and manually maintained.

Examples:

- Neovim query predicates/directives can model metadata and conditional captures more expressively than simple capture names.
- nvim-treesitter query suites can help identify where Tree Hugger's reference captures are too broad or too narrow.
- Highlights and locals queries can inform builtin/global classification, especially for languages where query authors already distinguish definitions, parameters, fields, properties, and builtins.

Before implementation:

- Inventory nvim-treesitter queries by language beyond `locals.scm` and identify reusable captures.
- Check license and vendoring update process.
- Compare each Tree Hugger `references.scm` against upstream query conventions.
- Decide whether to support Neovim-style predicates/directives or translate them into Tree-sitter Rust query features plus local post-processing.
- Add query provenance metadata so diagnostics can identify upstream versus local rules.

### Design details

Start with a per-language query inventory for nvim-treesitter `locals.scm`, `highlights.scm`, `injections.scm`, `folds.scm`, and `indents.scm`, recording which captures can inform Tree Hugger definitions, references, builtins, and diagnostic context. Treat upstream queries as vendored inputs with an explicit license check, pinned source revision, update command, and review step before any local copy changes.

Add provenance metadata to loaded query groups: source project, upstream revision, original path, local overlay path, license, and whether the query is upstream, translated, or Tree Hugger-specific. Keep upstream queries layered below local overlays so local fixes remain small, reviewable, and easy to rebase when nvim-treesitter changes.

Define a compatibility policy for Neovim predicates, directives, and query modelines. Prefer native Tree-sitter Rust query features where they match; translate unsupported Neovim-only behavior into named post-processing hooks instead of silently dropping it. Tests should include query compilation, capture parity snapshots for representative fixtures, provenance assertions in diagnostic JSON, and an update workflow that reports upstream query drift before replacing vendored files.

Open decisions: whether Tree Hugger should vendor selected nvim-treesitter query files directly or generate reduced query artifacts, how much Neovim directive behavior belongs in the core query runtime, and which query suites are allowed to affect lint severity versus only providing symbol/reference context.

References:

- https://github.com/nvim-treesitter/nvim-treesitter
- https://neovim.io/doc/user/treesitter/

## Opportunity: Build Language-Specific Semantic Resolvers Before Reporting Errors

Recommendation: true

Impact: high

Level of effort: high

Benefit: Undefined and unused diagnostics need language semantics. A file-local set comparison cannot distinguish many valid programs from errors.

Examples:

- Rust: use Cargo metadata, module paths, extern prelude, `use` trees, `pub` visibility, attributes, macro invocations, derive macro names, and maybe `rust-analyzer`/rustc data for serious correctness.
- Python: distinguish identifiers from keyword argument labels, decorator names, attributes, imported module attributes, `__all__`, framework entry points, and dynamic globals.
- JavaScript/TypeScript: handle import/export forms, globals, JSX, destructuring, type-only references, ambient declarations, bundler aliases, and config files.
- Go: use package imports and exported identifiers; many unused/import errors are better delegated to `go vet`, `gopls`, or compiler-like analysis.

Before implementation:

- Choose whether Tree Hugger wants lightweight heuristics or compiler/LSP-backed linting per language.
- Define minimum language-specific resolver scope for default-on diagnostics.
- Decide when to delegate to established tools: Oxlint, rustc/rust-analyzer, gopls, Pyright/Ruff, etc.
- Add benchmark and correctness gates on known open-source repositories.

### Design details

Add a resolver layer between parsed symbol records and semantic diagnostics. The layer should expose a language-specific `SemanticResolver` interface that accepts file symbols, imports, references, parser diagnostics, and optional project context, then returns resolved references plus unresolved/unused candidates with confidence and provenance. Keep the generic pipeline responsible for orchestration and diagnostic schema, while each language owns scoping rules, import/module resolution, visibility rules, and framework or runtime exceptions.

The minimum scope for default-on diagnostics should be language-specific and conservative. Rust needs Cargo package metadata, module path resolution, extern prelude awareness, `use` tree handling, public visibility, and macro/attribute escape hatches before undefined names become credible. Python needs lexical scopes, import aliases, decorators, keyword argument exclusion, attributes, module search paths, and common dynamic globals. JavaScript and TypeScript need ES/CJS import-export forms, destructuring, JSX/TSX, type/value namespaces, ambient globals, tsconfig/jsconfig paths, and package/bundler aliases. Go needs package imports, exported identifiers, module context, generated-file conventions, and compiler-like unused import/name behavior.

Project context should be an explicit input object rather than hidden global state. It should include the root path, package manifests, language config files, dependency graph hints, module search paths, generated-file markers, environment or target version, and optional cached outputs from external tools. When a language needs compiler-grade semantics or an ecosystem tool is already authoritative, delegate instead of reimplementing: Oxlint for JavaScript/TypeScript lint rules, rust-analyzer or rustc-derived data for Rust name resolution, gopls or `go vet` for Go, and Pyright or Ruff for Python where their outputs map cleanly to Tree Hugger diagnostics.

Default severity should remain gated until a resolver has measured precision on real projects. Experimental resolvers should emit warnings only when explicitly enabled, with `category`, `confidence`, and `source` metadata preserved in human and JSON output. A resolver-backed rule can become default-on only after corpus tests show low false-positive rates for the supported language subset, and unsupported constructs should degrade to lower confidence rather than hard errors.

Tests should include per-language scope fixtures, import/module fixtures, framework escape hatches, generated-code cases, and regression examples from representative open-source repositories. Corpus tests should assert both correctness and non-regression in diagnostic volume. Performance work should add per-project caches keyed by file content, manifest/config hashes, dependency metadata, and resolver version, with clear invalidation when manifests or config files change. Open decisions: the exact `SemanticResolver` trait shape, which languages get first-class resolvers first, the false-positive threshold for default-on rules, whether delegated tools are optional runtime integrations or feature-gated dependencies, and how much project context belongs in the stable public API.

## Opportunity: Add Rule Metadata and Documentation

Recommendation: true

Impact: medium

Level of effort: medium

Benefit: Current `lint.scm` captures only `@diagnostic.<rule-id>`, and message/severity live in Rust match statements. Rule metadata should describe category, default severity, confidence, language support, examples, false-positive caveats, and whether project context is required.

Example: `unwrap-call` can say "Rust, panic/error handling, warning, high syntactic confidence, policy-dependent." `undefined-symbol` can say "semantic, low confidence without project context, experimental." That makes CLI JSON more useful and helps downstream tools decide whether to fail CI.

Before implementation:

- Choose metadata storage: Rust table, TOML/JSON next to queries, or query directives.
- Decide whether metadata is included in JSON output or available through a separate command.
- Decide how ignore directives interact with aliases, categories, and rule groups.

### Design details

Store rule metadata in structured files next to the owning query files, with generated Rust checked in or built at compile time so `tree-hugger-lib` remains the single runtime source of truth. Prefer TOML for authoring because it is readable in review and maps cleanly to a typed schema; reserve query directives for capture-local hints only, not full rule documentation.

Each rule record should include `id`, `version`, `title`, `summary`, `category`, `default_severity`, `confidence`, `languages`, `requires_project_context`, `default_enabled`, examples, false-positive notes, and docs text. Rule identity should be stable across languages when the semantics match, with language-specific variants represented as metadata entries under the same rule id; breaking meaning changes should bump the rule version instead of silently reusing old CI policy.

Expose rule metadata in JSON diagnostics by adding a compact `rule` object or stable `rule_id` plus optional metadata fields, while keeping existing diagnostic fields compatible. The CLI should use the same registry to render `hug lint --list-rules`, category summaries, generated Markdown docs, and detailed rule help so documentation cannot drift from emitted diagnostics.

Ignore directives should resolve canonical rule ids first, then aliases and groups. Aliases provide migration paths for renamed rules; groups provide stable handles such as `syntax`, `semantic`, `experimental`, and language-prefixed groups. Validation must reject duplicate ids, dangling aliases, empty language support, unknown categories/severities, undocumented default-on rules, and metadata for captures that are not present in the query set.

Tests should cover metadata parsing, schema validation, capture-to-rule coverage, JSON output shape, CLI rule listing, generated docs snapshots, ignore directive resolution, alias compatibility, and group filtering. The main open decisions are whether generated Rust should be committed or produced during build, whether rule versions are independent from crate versions, and how much metadata should appear on every JSON diagnostic versus a separate rule registry payload.

## Opportunity: Expand Pattern Rule Coverage Where Tree-Sitter Is Enough

Recommendation: true

Impact: medium

Level of effort: medium

Benefit: Tree-sitter is still good for high-confidence syntactic patterns. We should add rules where AST shape alone is enough and avoid pretending to resolve semantics.

Examples:

- Debug artifacts: `console.log` in JS/TS, `println!`/`eprintln!` in Rust under production config, Python `print`.
- Dangerous APIs: JS `Function(...)`, Node `child_process.exec`, Python `subprocess(..., shell=True)`, PHP `eval`, shell unquoted expansions where the grammar supports it.
- Deprecated or discouraged syntax that is syntactically explicit.
- Language-specific TODO/dead placeholders if policy enables them.

Before implementation:

- Define a rule acceptance bar: low false-positive rate from syntax alone.
- Add pass/fail fixtures per rule, including non-trigger lookalikes.
- Decide whether policy-heavy rules are default-off.
- Consider using query directives/metadata instead of hard-coded Rust severity tables.

### Design details

Accept syntax-only rules only when the complete triggering condition is visible in a single Tree-sitter parse tree without name binding, type inference, module resolution, or project context. A rule should have an obvious AST shape, precise span selection, clear remediation text, and a fixture-backed false-positive budget; rules that require intent, dataflow, or alias tracking should move to a resolver-backed category instead.

Prioritize rule families where this bar is realistic: debug artifacts, syntactically dangerous APIs, explicit deprecated syntax, shell quoting hazards where captures are precise, and project-policy placeholders such as TODO/FIXME only when enabled. Default severity should be `warning` for high-confidence general rules and `info` or default-off for policy-heavy rules, production-only checks, and rules whose usefulness depends on repository conventions. `--strict` may promote opted-in pattern warnings, but it should not silently enable noisy policy rules.

Query mechanics should keep rule identity in `@diagnostic.<rule-id>` captures and attach rule metadata alongside the query rather than in scattered Rust match tables. Metadata should define title, category, default severity, default enablement, message, help text, language scope, and whether the rule is policy-sensitive. If query directives are used, keep them limited to stable fields that can be validated at load time and surfaced consistently through JSON output, CLI rendering, and future generated docs.

Each rule needs paired positive and negative fixtures. Negative fixtures should include lookalikes such as shadowed method names, string/comment mentions, non-production test/debug files when relevant, safe argument variants, and nested AST shapes that should not widen the capture. Rollout should start with a small set of high-confidence rules per language, preserve existing diagnostic IDs once published, and treat severity/default-enable changes as compatibility-sensitive. New rules should be documented as syntax-pattern diagnostics so users do not mistake them for semantic analysis.

Open decisions: where metadata should live relative to `.scm` query files, whether default-off policy rules should require explicit rule IDs or allow category-level enabling, and which first language/rule bundle should establish the fixture and documentation pattern.

## Opportunity: Improve Dead-Code Analysis or Keep It Narrow

Recommendation: true

Impact: medium

Level of effort: medium

Benefit: The current dead-code rule is more defensible than undefined/unused because it looks for statements after unconditional exits in the same block. Still, it is intentionally shallow. It should stay narrow unless we add control-flow analysis.

Examples:

- `return; x();` in the same block is a real issue.
- Code after `if condition { return }` is not necessarily dead.
- Calls like `process::exit`, `std::process::exit`, `panic!`, `throw`, or `raise` require language-specific terminal-call knowledge.

Before implementation:

- Document the exact contract as "same-block post-terminal unreachable statements."
- Add negative tests for conditionals, loops, closures, lambdas, async callbacks, and try/finally/defer-like constructs.
- Decide if terminal API lists are configurable.
- Consider a minimal CFG only if false negatives become important.

### Design details

Keep the rule contract exact and narrow: report only statements that appear later in the same syntactic block after a terminal statement has been seen. Model terminal statements as an explicit language-scoped enum rather than inference from arbitrary expressions; the initial set should cover structural exits such as `return`, `throw`, `raise`, `break`/`continue` where valid for the enclosing construct, and known aborting forms such as Rust `panic!`, Rust `std::process::exit`/`process::exit`, JavaScript/TypeScript `throw`, Python `raise`, and language equivalents that can be identified without type resolution.

The default scope should remain same-block only, with no cross-branch, loop, closure, callback, async, `finally`, `defer`, or destructor reasoning. Add negative fixtures for those cases so the rule does not grow accidental CFG-like behavior while matching obvious `return; x();`-style dead statements. Configurable terminal API lists should be deferred until there is a concrete caller need; hard-coded, documented per-language terminal calls are easier to test and avoid turning shallow linting into project-specific semantic analysis.

Only introduce a minimal CFG if the project decides false negatives are materially worse than false positives for this rule. The threshold should be specific: repeated real-world examples where same-block detection misses user-visible issues, plus tests showing CFG expansion does not report reachable code after conditionals, loops, exception edges, closures, or deferred cleanup. Open decisions: whether `break` and `continue` should report later statements inside the same loop body for every supported grammar, whether terminal calls should be surfaced as rule metadata, and which language-specific abort APIs belong in the first supported set.

## Opportunity: Add Project-Level Caching

Recommendation: true

Impact: high

Level of effort: high

Benefit: There is currently global compiled-query caching and CLI-local in-memory `SymbolSnapshot` caching. The snapshot cache is per process, bounded to the current file count, and therefore only helps if the same `TreeFile` is analyzed more than once in one command. It does not persist across invocations. The `SymbolSnapshot::from` fingerprint still uses placeholders such as `tree-sitter-unknown` and `query-fingerprint-unknown`, while the CLI cache key uses coarse strings like `locals+imports+references`.

Caching would add most value at:

- parsed Tree-sitter trees for unchanged files during a single run
- file-level symbol/import/export/reference/diagnostic snapshots across runs
- project module graphs for import resolution and cross-file semantic checks
- dependency/config discovery, such as Cargo metadata, package.json/tsconfig, Python project config, and ignore files
- Oxlint subprocess results if delegated JS/TS linting is expensive in large repos

Before implementation:

- Define real analyzer fingerprints: crate version, grammar versions, query hashes, config hashes, rule metadata hashes, and enabled-rule set.
- Decide cache storage location and invalidation strategy.
- Decide whether diagnostics are cached separately from symbols because rule config can change without symbol changes.
- Add cache stats to CLI output or debug logs so effectiveness is measurable.
- Avoid persistent caches until invalidation is trustworthy.

### Design details

Use layered caching rather than one project cache. Keep compiled query caching as a global in-process layer, add an in-process parse/tree layer for unchanged files during one command, and add a file snapshot layer for symbols, imports, exports, references, and diagnostics. Project-level derived data, such as module graphs, package metadata, config discovery, ignore rules, and external-tool results, should sit above file snapshots and depend on the set of participating files plus the relevant config fingerprints.

Start with in-process caches for parse trees and project graphs, then make file snapshots persistent only after the invalidation model is explicit and tested. Persistent cache storage should live under a tool-owned user cache directory, with a project identity component derived from the canonical project root and workspace metadata rather than from the current working directory alone. Cache entries need analyzer fingerprints that include the tree-hugger crate version, grammar versions, query hashes, rule metadata hashes, enabled-rule set, CLI/config flags, language-specific project config hashes, external-tool versions such as Oxlint, and the source file content hash plus path metadata that affects language detection.

Cache symbols and diagnostics separately. Symbol snapshots are invalidated by source content, grammar, query, and symbol schema changes; diagnostic snapshots are additionally invalidated by enabled rules, severities, lint config, resolver configuration, delegated tool versions, and project graph changes. This keeps symbol extraction reusable when a lint policy changes while avoiding stale diagnostics with different rule settings. Project graphs should invalidate when file membership, import/export snapshots, package manifests, or language config files change.

Expose cache behavior without making it part of normal lint noise. Add debug or verbose CLI output for hit/miss counts by layer, invalidation reasons, persistent cache path, analyzer fingerprint summaries, and timing deltas. JSON output can include optional cache stats under an explicit stats/debug flag, but default diagnostic output should remain stable for CI. Rollout should be opt-in or in-process-only first, with a kill switch such as `--no-cache`, conservative fallback to recomputation on fingerprint mismatch, and no cache writes after parse or diagnostic failures until partial-cache semantics are defined.

Tests should cover fingerprint changes, source edits, query changes, rule/config changes, project config changes, file moves, missing or corrupt cache entries, and cross-command persistent reuse. Benchmarks should measure cold, warm in-process, and warm persistent runs on small and large repositories, with separate timings for parse, symbol extraction, diagnostics, project graph construction, and delegated external tools.

Open design decisions: whether persistent caching should be enabled by default after validation, the exact project identity scheme for worktrees and symlinked roots, how long persistent entries should be retained, whether parse trees should ever be serialized, and which cache stats belong in public JSON versus debug-only output.

## Opportunity: Separate Symbol Extraction Cache from Diagnostic Cache

Recommendation: true

Impact: medium

Level of effort: medium

Benefit: `symbol_index_v2()` currently embeds diagnostics in the parse pass, and the CLI cache caches the entire `FileSymbolIndex`. A change to enabled rules, severities, ignore directives, project config, or linter adapter should not require re-extracting all symbols, but it also must not reuse stale diagnostics.

Example: changing `unwrap-call` from warning to allow should not invalidate the symbol graph. Changing `references.scm` should invalidate semantic diagnostics and reference relations. Changing `comments.scm` should invalidate ignore handling.

Before implementation:

- Split cache entries into parse tree, symbol records, reference records, and diagnostics.
- Make cache keys pass-specific.
- Include enabled-rule/config hash in diagnostic cache keys.
- Decide whether `FileSymbolIndex` should be a composed view rather than the cache unit.

### Design details

Treat cache entries as pass-specific units: parse tree or source fingerprint, symbol records, reference records, comments or ignore directives, and diagnostics. Each unit should have a key that includes only the inputs that can change that pass: file path or canonical identity, content hash, language and grammar version, relevant query hash, analyzer version, and pass options. Diagnostic cache keys additionally need enabled rule IDs, severities, ignore/config fingerprints, rule metadata version, and any external adapter fingerprint.

`FileSymbolIndex` should become a composed view over these units, not the persistent cache value. That keeps a symbol-only caller from invalidating or materializing diagnostics, while preserving the existing API by assembling symbols, references, comments, and diagnostics at the boundary. For example, changing `symbols.scm` invalidates symbol records and composed indexes; changing `references.scm` invalidates references and semantic diagnostics; changing `comments.scm` invalidates ignore handling and diagnostics; changing `unwrap-call` severity invalidates diagnostics only.

Tests should cover stale-diagnostic prevention, symbol-cache reuse when lint config changes, reference-query invalidation, comment-query invalidation, and compatibility for callers that still request `symbol_index_v2()`. Benchmarks should compare cold runs, warm symbol-only runs, warm lint-only runs, and mixed runs after config-only changes.

Open decisions: whether parse trees are cached in memory only or persisted as content-addressed artifacts, how much cache-key detail is exposed in debug output, and whether diagnostic adapters can declare their own fingerprint inputs through a shared trait.

## Opportunity: Add Real-World Corpus Testing

Recommendation: true

Impact: high

Level of effort: medium

Benefit: Passing unit tests prove the query matched a fixture, not that the warning is a real lint. The current tests are mostly trigger tests plus some local regressions. We need false-positive measurement on real projects.

Examples:

- Run `hug lint` on Rust crates using Serde, Clap, Tokio, macros, module trees, and generated code.
- Run JS/TS on React/Next/Vite packages with JSX, TSX, path aliases, globals, and test files.
- Run Python on Flask/FastAPI/Django examples with decorators and framework entry points.
- Run Go on packages where compiler/language-server diagnostics are available as a comparison oracle.

Before implementation:

- Pick pinned corpus repositories or local fixtures.
- Define accepted false-positive and false-negative thresholds per rule.
- Compare against established tools where possible.
- Store results as snapshots only after redacting paths and stabilizing nondeterminism.

### Design details

Corpus tests should use a small, explicit manifest of pinned repositories or vendored fixture archives per language, with commit SHAs, license notes, selected paths, and any intentionally excluded generated or dependency directories. Start with projects that exercise known hard cases: macros and module trees for Rust, JSX/TSX and path aliases for JavaScript/TypeScript, decorators and framework entry points for Python, and package-level diagnostics for Go.

The harness should run `hug lint` through the CLI for contract coverage and the library API for focused diagnostics, normalizing paths, line endings, ordering, and tool versions before comparison. Each corpus entry should declare enabled rules, language overrides, expected diagnostic classes, and whether established tools such as `cargo check`, `eslint`/`oxlint`, `pyright`/`ruff`, or `go vet` act as comparison oracles. Oracle mismatches should be classified as confirmed false positive, confirmed false negative, external-tool disagreement, or accepted limitation.

Thresholds should be rule-specific instead of global. Stable pattern rules can require zero known false positives in pinned corpora, while experimental semantic rules should use explicit budgets and confidence bands until language-specific resolvers exist. False negatives should be measured only where an oracle or seeded fixture proves the issue exists; otherwise the corpus primarily guards against noisy output.

Snapshots should redact user paths, temporary directories, repository checkout paths, and absolute tool locations. Snapshot output should avoid nondeterministic timing, cache-hit, and traversal-order data, and should store stable diagnostic identifiers plus compact context rather than full source files.

CI should split into tiers: fast smoke corpus on every PR, expanded language corpus in scheduled or labeled CI, and manual benchmark runs for cache or resolver changes. The maintenance workflow should include a documented command to refresh corpus checkouts, update snapshots, review threshold deltas, and record why any new accepted diagnostic is legitimate.

Open decisions: where pinned corpora should live, whether CI may fetch them from the network or must use prebuilt archives, which external tools are mandatory versus best-effort, and who owns periodic corpus refreshes when upstream projects change.

## Opportunity: Add Lint Output Contracts for CI

Recommendation: true

Impact: medium

Level of effort: low

Benefit: Users need to know whether `hug lint` is advisory or CI-failing. Syntax errors can reasonably be errors. Pattern rules can be warnings. Experimental semantic heuristics should not fail CI by default.

Example: a repository should not fail a CI lint job because Tree Hugger thinks a Serde-derived field is unused or a default JS import is undefined. It could fail on syntax errors or opt-in strict rules.

Before implementation:

- Define exit codes for syntax errors, warnings, and experimental diagnostics.
- Add CLI flags like `--deny`, `--warn`, `--allow`, or category-specific filters if that fits the CLI style.
- Ensure JSON output includes enough data for external CI policy.

### Design details

- Treat `hug lint` as advisory by default for non-syntax diagnostics: syntax errors exit non-zero, while warning and experimental categories render visibly but exit zero unless promoted by policy.
- Define an explicit exit-code contract: `0` for no failing diagnostics, `1` for failing diagnostics selected by the active policy, and reserve any existing CLI error code behavior for invalid flags, unreadable inputs, or internal failures.
- Add policy controls that compose by rule ID and category, such as `--deny <selector>`, `--warn <selector>`, and `--allow <selector>`, where selectors can target stable categories like `syntax`, `pattern`, `semantic`, and `experimental`; `--strict` can remain a convenience alias for denying all non-allowed diagnostics.
- Keep CLI defaults compatible with the proposed category split: syntax is failing, mature pattern rules are warnings, and semantic heuristics are experimental warnings until a resolver-backed implementation promotes them.
- Make JSON output CI-stable by including rule ID, category, default severity, effective severity after CLI policy, confidence, experimental status, file, span, message, and exit-code contribution for each diagnostic.
- Preserve compatibility by introducing new fields additively, documenting old behavior versus new behavior, and providing a migration note for CI users who currently assume any diagnostic should fail the job.
- Add tests for default advisory behavior, syntax-failing behavior, selector precedence, JSON effective severity, and strict mode; document the policy model in the CLI help and tree-hugger lint docs.
- Open decisions: whether selectors should use repeated flags or comma-separated values, whether `--deny warnings` should exist as a broad alias, and whether warning-only runs should have an optional distinct exit code for CI systems that support richer status handling.

## Opportunity: Improve Import/Reference Query Precision

Recommendation: true

Impact: high

Level of effort: medium

Benefit: Many false positives begin with overly broad reference captures. For example, JS imports captured `React` as a reference while imports were not recognized in the file summary. Python keyword argument names were captured as variable references. Rust crate path components in `use` declarations and attributes were captured as undefined references.

Before implementation:

- Audit every `references.scm` against grammar node fields and definition/import captures.
- Add negative tests for import declarations, attribute/decorator syntax, keyword argument labels, property keys, field declarations, type declarations, and pattern bindings.
- Prefer more specific captures over blanket `(identifier) @reference` patterns where grammars permit it.
- Keep a language-specific skip list in code only for cases queries cannot express.

### Design details

Start with a capture audit for every language's `references.scm` and import queries, comparing each `@reference` capture to grammar field names, existing definition captures, and import/export captures. Record each broad identifier rule with the false-positive class it currently admits, such as JavaScript/TypeScript import specifiers and JSX tag names, Python keyword argument labels and decorator names, Rust `use` paths and attribute segments, Go selector fields, object/property keys, field declarations, type positions, and pattern bindings. The default fix should be query specificity: capture only identifier nodes in value/reference positions when the grammar exposes those positions, and reserve code-level skip lists for grammar gaps, ambiguous nodes, or language constructs that cannot be expressed clearly in Tree-sitter queries.

Add negative fixtures per language before changing behavior. Each fixture should include import declarations, attributes/decorators, labels/keys, field and type declarations, destructuring or pattern bindings, and one positive nearby reference so the test proves the query is narrower rather than disabled. Roll out in stages by first adding inventory notes and failing or ignored regression fixtures, then tightening one language family at a time, and finally enabling corpus checks that compare reference counts, undefined-symbol diagnostics, and import recognition before and after the query changes. Keep staged output visible in review snapshots so large diagnostic drops can be inspected rather than treated as automatically correct.

Open decisions: decide whether import query precision and reference query precision should ship in one compatibility change or separate releases, define the acceptable corpus-level diagnostic delta before manual review is required, and choose where to document language-specific query limitations that remain code skip-list backed.

## Opportunity: Delegate to Established Language Tools Where Appropriate

Recommendation: true

Impact: high

Level of effort: high

Benefit: Some lint categories are not a good fit for Tree-sitter-only analysis. Using specialized tools avoids reimplementing compilers and ecosystem semantics.

Examples:

- JS/TS: Oxlint for lint rules, multi-file import analysis, JSX/TSX, and type-aware checks.
- Rust: rustc/rust-analyzer/Clippy for name resolution, unused code, macro expansion, and type-driven checks.
- Python: Ruff/Pyright for lint/type-style diagnostics.
- Go: compiler, `go vet`, or gopls for undefined/unused/package-aware results.

Before implementation:

- Decide whether Tree Hugger is a universal lint runner, a symbol analyzer with optional adapters, or both.
- Define adapter interfaces and normalized diagnostic schema.
- Handle tool availability, versions, config discovery, and reproducibility.
- Decide which languages keep Tree-sitter-native lint only.

### Design details

Keep the product boundary explicit: Tree Hugger should remain the structural symbol and syntax analyzer, with optional language-tool adapters for diagnostics that require compiler, type, package, or ecosystem semantics. Adapters should not replace Tree-sitter parsing or symbol extraction; they should be an additional diagnostic source that can be enabled per language, per project, or per command.

Define a shared adapter interface that accepts resolved paths, project root, detected language, lint configuration, environment policy, and a cache handle, then returns normalized diagnostics plus adapter metadata. The metadata should include tool name, version, invocation mode, config files used, working directory, exit status, elapsed time, and whether results came from a fresh run or cache. The normalized diagnostic schema should preserve Tree Hugger fields while adding `source_tool`, `source_rule_id`, `category`, `severity`, `confidence`, stable span data, optional fix availability, and reproducibility fields that record enough input state to explain how the result was produced.

Tool discovery should be deterministic and visible. Prefer explicit config or CLI paths first, then project-local executables where the ecosystem has a convention, then `PATH`. Each adapter should check supported version ranges, report incompatible versions as capability diagnostics, and delegate config discovery to the underlying tool where possible instead of reimplementing ecosystem rules. Reproducible CI use should support pinned executable paths, captured versions, stable working directories, and JSON output contracts that do not depend on localized human text.

Prioritize languages where external tools clearly dominate Tree-sitter-only linting: Oxlint for JavaScript/TypeScript first, Ruff and Pyright for Python where their diagnostic categories map cleanly, `go test`/`go vet`/gopls-backed diagnostics for Go, and rust-analyzer, rustc, or Clippy-derived diagnostics for Rust only after the adapter can account for Cargo workspaces and macro-heavy projects. Keep Tree-sitter-native linting for syntax diagnostics, high-confidence local pattern rules, unsupported languages, and cases where invoking a full external tool would be disproportionate.

Fallback behavior should be quiet by default for optional adapters. If a tool is unavailable, incompatible, or misconfigured, continue with Tree Hugger-native syntax and pattern diagnostics and expose the adapter issue as metadata or a capability warning rather than a file diagnostic. CI-oriented strict modes may fail when a requested adapter cannot run, but absence of an optional tool should not make ordinary multi-language linting unusable.

Tests should cover recorded JSON fixtures from each adapter, version and config discovery, missing-tool and incompatible-version fallback, span normalization, severity/category mapping, cache invalidation, and mixed-language runs where only some files have external-tool support. Benchmarks should compare native Tree Hugger lint, cold adapter subprocess startup, warm cached runs, and full project runs on representative repositories, with thresholds for acceptable overhead before an adapter becomes default-on.

Open decisions: whether adapters are configured through CLI flags, project config, or both; which adapter failures should be diagnostics versus command errors; whether fix metadata belongs in the stable schema before Tree Hugger supports applying fixes; how much environment state must be captured for reproducibility; and whether Rust should delegate to Clippy/rustc output, rust-analyzer diagnostics, or both.
