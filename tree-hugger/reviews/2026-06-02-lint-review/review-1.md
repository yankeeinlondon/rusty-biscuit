# Lint Review Implementation Validation

Date: 2026-06-03

Source review used: `tree-hugger/reviews/2026-06-02-lint-review/review.md`.

Requested source path note: `tree-hugger/features/20206-06-02-lint-review/review.md` does not exist in this worktree. The repository has `tree-hugger/reviews/2026-06-02-lint-review/review.md`, so this validation used the existing review and saved this report at the requested output path.

## Summary

The implementation substantially addresses the review: diagnostic metadata, a rule registry, default gating for library-level experimental semantic checks, CLI severity policy flags, expanded pattern rules, dead-code contract tests, cache infrastructure, corpus scaffolding, query provenance/drift checks, resolver interfaces, and an Oxlint adapter were added.

Verification passed for the package area:

- `cargo test --workspace --color=never` from `tree-hugger`: passed.
- `cargo clippy -p tree-hugger --all-targets --color=never -- -D warnings` with `CARGO_TARGET_DIR=/private/tmp/tree-hugger-clippy-target`: passed.
- `cargo clippy -p tree-hugger-cli --all-targets --color=never -- -D warnings` with `CARGO_TARGET_DIR=/private/tmp/tree-hugger-clippy-target`: passed.

Initial parallel Clippy runs against the default target directory failed with shared build-artifact write errors, not lint diagnostics. Sequential Clippy runs using a temp target directory were clean.

## Findings

### 1. `hug lint --experimental-semantics` does not enable semantic diagnostics

Severity: high

The CLI accepts `--experimental-semantics` and applies a policy filter for already-present experimental diagnostics, but it never sets `TreeFile::experimental_semantics = true` before computing the symbol index or lint diagnostics.

Evidence:

- `TreeFile` defaults `experimental_semantics` to `false`: `tree-hugger/lib/src/file/tree_file.rs:91`.
- Semantic diagnostics are only computed when that field is true: `tree-hugger/lib/src/file/tree_file.rs:1066`, `tree-hugger/lib/src/file/tree_file.rs:1075`, and `tree-hugger/lib/src/file/tree_file.rs:1087`.
- The CLI builds `TreeFile::with_language(...)` and immediately analyzes it without setting the flag: `tree-hugger/cli/src/main.rs:827` and `tree-hugger/cli/src/main.rs:1280`.
- The cache fingerprint also hard-codes `config_fingerprint: "default"` and does not include semantic/lint policy inputs: `tree-hugger/cli/src/main.rs:1290`.

Black-box probe:

- `hug --plain lint lint-validation-probe/a.rs`: exit `0`, no diagnostics.
- `hug --plain lint --experimental-semantics lint-validation-probe/a.rs`: exit `0`, no diagnostics.
- `hug --plain lint --experimental-semantics --strict lint-validation-probe/a.rs`: exit `0`, no diagnostics.

Expected behavior from the review: default lint should hide experimental semantic findings, but `--experimental-semantics` should surface them and `--strict --experimental-semantics` should make enabled warnings fail.

Suggested fix: thread lint analysis options into `analyze_tree_file`, set `tree_file.experimental_semantics` before `symbol_index_v2()` or compute lint diagnostics after policy setup, and include semantic enablement plus policy/config fingerprints in diagnostic cache keys.

### 2. Diagnostic metadata is not attached by the library for all diagnostics

Severity: medium

The review called for diagnostics to carry category/confidence/source/default/effective severity metadata. The CLI fills metadata for lint diagnostics after applying policy, but the library still constructs pattern and semantic `LintDiagnostic` values with `metadata: None`, and syntax diagnostics cannot carry metadata at all.

Evidence:

- `LintDiagnostic.metadata` is optional and skipped when absent: `tree-hugger/lib/src/shared/symbol.rs:642`.
- `SyntaxDiagnostic` has no metadata field: `tree-hugger/lib/src/shared/symbol.rs:656`.
- Syntax conversion to unified diagnostics sets `metadata: None`: `tree-hugger/lib/src/shared/symbol.rs:742`.
- Pattern and semantic diagnostics are created with `metadata: None`: `tree-hugger/lib/src/file/tree_file.rs:1028`, `tree-hugger/lib/src/file/tree_file.rs:1227`, and `tree-hugger/lib/src/file/tree_file.rs:1279`.
- The CLI policy layer attaches metadata later: `tree-hugger/cli/src/main.rs:2196`.

Observed JSON behavior: CLI JSON includes metadata for lint diagnostics after policy, but syntax JSON lacks metadata.

Suggested fix: populate metadata in the library using the rule registry for lint/semantic diagnostics, add metadata to syntax diagnostics or unified syntax diagnostics, and keep the CLI responsible only for effective severity overrides.

### 3. Oxlint integration is implemented as an adapter but not wired into CLI lint

Severity: medium

The review’s JS/TS recommendation was to delegate JS/TS linting to Oxlint where available and normalize diagnostics into Tree Hugger output. The adapter exists and has tests, but no CLI code imports or invokes `OxlintAdapter`; `hug lint` still uses native Tree Hugger diagnostics only.

Evidence:

- Adapter implementation exists in `tree-hugger/lib/src/adapter/oxlint.rs:176`.
- CLI imports do not include adapter types and `rg` found no `OxlintAdapter` use under `tree-hugger/cli/src`.

Suggested fix: add an explicit adapter execution path for JavaScript/TypeScript lint runs, preserve native syntax/pattern diagnostics as fallback, expose configuration/strict behavior, and add CLI integration tests with recorded JSON fixtures or a fake adapter.

### 4. Cache implementation is broad, but diagnostic invalidation is still incomplete

Severity: medium

The review specifically asked to separate symbol extraction cache from diagnostic cache and include enabled-rule/config hashes in diagnostic cache keys. The implementation adds pass fingerprints and persistent cache infrastructure, but CLI analysis currently caches a full `FileSymbolIndex` with diagnostics under a coarse fingerprint.

Evidence:

- CLI cache key uses `query_fingerprint: "locals+imports+references"` and `config_fingerprint: "default"`: `tree-hugger/cli/src/main.rs:1290`.
- The cached unit returned to callers is a full `FileSymbolIndex`: `tree-hugger/cli/src/main.rs:1300`.
- Policy is applied after reading the cached index: `tree-hugger/cli/src/main.rs:1261`.

Suggested fix: split cache units in the CLI path as well as the library structs, include enabled rules, severity policy, experimental semantics, ignore/config, query hashes, rule metadata, and adapter versions in diagnostic cache fingerprints, and avoid storing diagnostics in a symbol-only cache entry.

### 5. Rule alias registration is incomplete for post-registration alias mutation

Severity: low

`dead-code` is mutated to include `unreachable-code` after registration, but the alias map is not updated. `RuleSelector::matches` still sees the alias when matching against `dead-code`, but `RuleRegistry::get("unreachable-code")` will not resolve through `alias_map`.

Evidence:

- Alias is pushed after `register_rule`: `tree-hugger/lib/src/rule_registry.rs:567`.
- `register_rule` is the only place that populates `alias_map`: `tree-hugger/lib/src/rule_registry.rs:135`.
- `RuleRegistry::get` consults `alias_map`: `tree-hugger/lib/src/rule_registry.rs:85`.

Suggested fix: define aliases before calling `register_rule`, or provide a `register_alias` helper that updates both the stored metadata and `alias_map`. Add a test that `registry.get("unreachable-code")` returns `dead-code`.

## Coverage Assessment

Coverage is high for the newly added library-level behavior:

- Experimental semantic gating is covered at `TreeFile` level.
- Rule registry parsing, selector matching, validation, and policy helpers are covered.
- Query compilation is covered across supported languages.
- Dead-code positive/negative cases include same-block terminal behavior and no-CFG false-positive guards.
- Reference/import precision regressions cover Rust imports/patterns/fields, JS imports/destructuring, Python keyword arguments/decorators/imports, and Swift declarations.
- Cache structs, fingerprints, invalidation helpers, and persistent round trips are covered.
- Corpus redaction, threshold, manifest, and result helpers are covered.
- Oxlint adapter interfaces, missing-tool behavior, normalization, severity/category mapping, and serialization are covered.
- Neovim query provenance, inventory, compatibility, drift, and capture parity are covered.

Coverage gaps remain around user-facing integration:

- No CLI tests cover `lint --experimental-semantics`, `--strict --experimental-semantics`, `--deny`, `--warn`, or `--allow`.
- No CLI JSON tests assert lint metadata shape or syntax metadata shape.
- No CLI tests assert advisory exit-code behavior for warning-only lint runs and failing behavior for syntax or denied warnings.
- No CLI path invokes or tests Oxlint delegation.
- No tests prove cache keys change when lint policy, experimental semantics, rule metadata, or external adapter version changes.

## Review Recommendation Status

- Reclassify semantic errors as experimental warnings: partially implemented. Library default gating exists; CLI opt-in currently does not work.
- Split lint categories explicitly: partially implemented with `DiagnosticCategory`, confidence, source, and policy selectors. The taxonomy is narrower than the review’s proposed syntax/pattern/reachability/file-local-name/project-semantics split, and syntax metadata is missing.
- Disable/gate file-local undefined/unused by default: mostly implemented at library level, but CLI opt-in is broken.
- Replace JS/TS linting with Oxlint integration: partially implemented as library adapter only; not integrated into CLI lint.
- Use Neovim/nvim-treesitter systematically: implemented as provenance, inventory, compatibility, and drift scaffolding with tests.
- Build language-specific semantic resolvers: implemented as interfaces/scaffolding, not production resolvers.
- Add rule metadata and documentation: partially implemented as an in-code registry. Structured files next to queries, generated docs/list-rules, alias/group validation, and capture-to-rule coverage are not implemented.
- Expand pattern rule coverage: partially implemented with `console-log`, `print-call`, and `fmt-println` queries and tests.
- Improve dead-code analysis or keep it narrow: implemented with narrow same-block behavior and negative tests.
- Add project-level caching: partially implemented. Infrastructure exists; CLI cache keys and diagnostic separation remain incomplete.
- Separate symbol extraction cache from diagnostic cache: partially implemented in types/concepts, not in the CLI cache path.
- Add real-world corpus testing: scaffold implemented with manifest/threshold/redaction tests; no real pinned corpus execution is present.
- Add lint output contracts for CI: partially implemented. Exit-code behavior works for syntax and warning-only pattern diagnostics, but CLI policy tests are missing and semantic opt-in does not work.
- Improve import/reference query precision: partially implemented with focused regression tests and query/code filtering.
- Delegate to established language tools: partially implemented as Oxlint adapter scaffolding; not wired to CLI and no other language adapters.

## Final Recommendation

Do not consider the review fully implemented yet. The codebase is in a much better state, and the test/lint baseline is clean, but the user-facing lint command does not yet satisfy key review requirements for opt-in semantic diagnostics or JS/TS delegation. Fix the CLI semantic flag wiring and cache fingerprints first, then add CLI contract tests before treating this review as closed.
