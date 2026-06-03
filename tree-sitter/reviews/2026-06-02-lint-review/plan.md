# Tree Hugger Lint Improvement Implementation Plan

Date: 2026-06-02

Source review: `tree-sitter/reviews/2026-06-02-lint-review.md`

## Success Criteria

- Default `hug lint` output is trustworthy: syntax errors can fail, mature pattern rules can warn, and file-local semantic heuristics do not present low-confidence findings as production-grade errors.
- Every diagnostic has enough metadata for humans, JSON consumers, CI policy, and future adapters to understand rule identity, category, confidence, source, and effective severity.
- Query-driven rules are expanded only where Tree-sitter syntax is enough, with positive and negative fixtures for each rule.
- No project-level or persistent cache is introduced until analyzer fingerprints and pass-specific invalidation are explicit and tested.
- External language tools are optional adapters, not replacements for Tree Hugger symbol extraction.

## Phase 1: Diagnostic Contract Foundation

1. Add diagnostic metadata to `tree-hugger-lib`.
   - Introduce stable category, confidence, source, default severity, and effective severity fields.
   - Keep existing JSON fields and add new fields additively.
   - Mark current file-local semantic rules as experimental semantic warnings.

2. Add a rule metadata registry.
   - Store metadata in structured files next to lint queries, with typed validation in the library.
   - Include rule id, version, title, category, default severity, confidence, language support, default enablement, examples, caveats, and project-context requirements.
   - Validate duplicate ids, dangling aliases, unknown categories, missing captures, and undocumented default-on rules.

3. Define CLI policy behavior.
   - Make syntax diagnostics failing by default.
   - Keep mature non-syntax diagnostics advisory unless selected by policy.
   - Add or refine `--deny`, `--warn`, `--allow`, and `--strict` behavior around stable rule/category selectors.
   - Ensure JSON includes effective severity and exit-code contribution.

4. Gate file-local semantic rules.
   - Disable broad `undefined-symbol`, `undefined-module`, and `unused-symbol` checks by default.
   - Add `--experimental-semantics` or an equivalent config key.
   - Keep `--strict` as a severity promotion mode, not an implicit semantic-rule enablement mode.

Verification:

- Snapshot tests for human and JSON output.
- Regression fixtures for Rust external crates/macros, Python keyword arguments, JavaScript imports, and existing syntax diagnostics.
- CLI tests for default exit behavior, strict mode, and selector precedence.

## Phase 2: Query Precision and Narrow Native Rules

1. Audit import and reference queries.
   - Inventory every `references.scm` and import query by language.
   - Replace blanket identifier captures with position-specific captures where grammar fields permit it.
   - Use code skip lists only for grammar gaps or ambiguous constructs that queries cannot express cleanly.

2. Add negative fixtures before query changes.
   - Cover import declarations, attributes, decorators, keyword argument labels, property keys, field declarations, type declarations, and pattern bindings.
   - Include nearby positive references in every negative fixture so tests catch disabled queries.

3. Keep dead-code analysis narrow.
   - Document and test the same-block post-terminal contract.
   - Add language-scoped terminal statement modeling.
   - Add negative tests for conditionals, loops, closures, lambdas, async callbacks, `finally`, and `defer`-like constructs.

4. Expand high-confidence syntax pattern rules.
   - Start with a small bundle of debug artifacts and syntactically dangerous APIs.
   - Make policy-heavy or production-context rules default-off.
   - Attach every rule to metadata instead of Rust-only severity tables.

Verification:

- Per-language fixture tests for changed queries and new rules.
- Diagnostic delta review for affected languages.
- Corpus smoke checks once Phase 4 harness exists.

## Phase 3: Resolver and Adapter Architecture

1. Define semantic resolver interfaces.
   - Add an explicit project-context input containing root path, manifests, language config, dependency hints, generated-file markers, and target environment.
   - Require resolvers to report confidence and supported-scope metadata.
   - Keep resolver-backed diagnostics gated until corpus precision is measured.

2. Define external diagnostic adapter interfaces.
   - Accept resolved paths, project root, language, lint config, environment policy, and cache handle.
   - Return normalized diagnostics plus adapter metadata: tool name, version, config files, working directory, exit status, elapsed time, and cache status.
   - Preserve source tool rule ids and fix availability without applying fixes.

3. Implement JavaScript/TypeScript Oxlint as the first adapter.
   - Start with an optional subprocess adapter.
   - Discover explicit configured paths first, then project-local conventions, then `PATH`.
   - Parse Oxlint JSON into normalized diagnostics with tested span, severity, category, and rule mappings.
   - Fall back to Tree Hugger syntax and conservative pattern diagnostics when Oxlint is unavailable.

4. Decide later language-tool delegation order.
   - Python: Ruff/Pyright after adapter contract stabilizes.
   - Go: `go vet`, compiler-like checks, or gopls after project-context handling is ready.
   - Rust: rustc, Clippy, or rust-analyzer only after Cargo workspace and macro-heavy behavior are understood.

Verification:

- Recorded JSON fixture tests for adapters.
- Missing-tool, incompatible-version, config-discovery, and strict-failure tests.
- Mixed-language runs where only some files use external adapters.

## Phase 4: Corpus and Default-On Gates

1. Build a corpus test manifest.
   - Pin selected repositories or fixture archives by commit SHA.
   - Record license notes, selected paths, excluded directories, enabled rules, and oracle tools.
   - Start with Rust Serde/Clap/Tokio macro projects, React/Next/Vite JS/TS, Flask/FastAPI/Django Python, and package-level Go projects.

2. Add corpus harness tiers.
   - Fast smoke corpus for PRs.
   - Expanded corpus for scheduled or labeled CI.
   - Manual benchmark mode for cache, resolver, and adapter changes.

3. Define per-rule thresholds.
   - Require zero known corpus false positives for high-confidence syntax pattern rules.
   - Keep experimental semantic rules budgeted and opt-in until resolver-backed precision is proven.
   - Classify oracle mismatches as false positive, false negative, external-tool disagreement, or accepted limitation.

4. Stabilize snapshots.
   - Redact absolute paths, tool locations, temp dirs, and checkout roots.
   - Normalize line endings, diagnostic ordering, and tool versions.
   - Store stable diagnostic ids and compact context rather than full source.

Verification:

- Corpus harness tests for normalization and redaction.
- Threshold reports that can be reviewed before default-on changes.
- Documentation for refreshing corpora and reviewing diagnostic deltas.

## Phase 5: Caching

1. Split cache units before adding persistence.
   - Separate parse/source fingerprint, symbol records, reference records, comments or ignore directives, diagnostics, and project graphs.
   - Treat `FileSymbolIndex` as a composed view over pass outputs rather than the cache value.

2. Define pass-specific fingerprints.
   - Include source content, canonical path identity, language, grammar version, query hash, analyzer version, pass options, rule metadata hash, enabled-rule set, config hash, and external-tool version where relevant.
   - Keep diagnostic cache keys stricter than symbol cache keys.

3. Add in-process caches first.
   - Reuse parse trees and project graphs within one command.
   - Add verbose or debug cache stats for hit/miss counts, invalidation reasons, and timing.

4. Add persistent snapshots only after invalidation is tested.
   - Store under a tool-owned user cache directory with a stable project identity.
   - Provide `--no-cache`.
   - Recompute on any fingerprint mismatch or cache corruption.

Verification:

- Tests for source edits, query edits, rule/config changes, comment-query changes, file moves, corrupt entries, and adapter version changes.
- Benchmarks for cold, warm in-process, and warm persistent runs.

## Phase 6: Neovim Query Reuse

1. Inventory nvim-treesitter query suites beyond `locals.scm`.
   - Record reusable captures from highlights, injections, folds, indents, and locals.
   - Compare upstream conventions against Tree Hugger references and builtins.

2. Add query provenance metadata.
   - Track source project, upstream revision, original path, local overlay path, license, and translation status.
   - Layer upstream queries below local overlays.

3. Define predicate and directive compatibility.
   - Use Tree-sitter Rust query features where equivalent.
   - Translate unsupported Neovim behavior into named post-processing hooks.
   - Do not silently drop unsupported query behavior.

Verification:

- Query compilation tests.
- Capture parity snapshots for representative fixtures.
- Provenance assertions in diagnostics or debug output where applicable.
- Update workflow that reports upstream drift before replacing vendored files.

## Recommended Implementation Order

1. Diagnostic metadata, rule registry, CLI policy, and semantic gating.
2. Import/reference query precision and false-positive regression fixtures.
3. Dead-code contract cleanup and high-confidence native pattern rules.
4. Corpus harness and thresholds.
5. External adapter interface and Oxlint subprocess adapter.
6. Semantic resolver interface and first resolver experiments.
7. Pass-specific in-process caching.
8. Persistent project cache.
9. Systematic nvim-treesitter inventory and provenance workflow.

This order reduces user-facing false authority first, then improves native precision, then adds external and project-level sophistication behind measured contracts.
