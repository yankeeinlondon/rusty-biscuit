# Lint Review Re-Validation

Date: 2026-06-03

Validated review: `tree-hugger/reviews/2026-06-02-lint-review/review-1.md`.

## Summary

The actionable findings from `review-1.md` have been implemented in the current worktree. The CLI now threads `--experimental-semantics` into `TreeFile`, syntax and lint diagnostics carry metadata in JSON output, the JS/TS lint path invokes the Oxlint adapter, diagnostic cache use no longer stores stale diagnostics in the symbol snapshot, and the `dead-code` alias resolves through `RuleRegistry::get("unreachable-code")`.

Package validation is clean:

- `cargo test --workspace --color=never`: passed.
- `cargo clippy -p tree-hugger --all-targets --color=never -- -D warnings`: passed.
- `cargo clippy -p tree-hugger-cli --all-targets --color=never -- -D warnings`: passed.

Test coverage is strong for the functionality updated after `review-1.md`: CLI semantic opt-in, strict-mode failure, syntax metadata JSON, Oxlint CLI integration via a fake executable, rule alias lookup, cache fingerprint primitives, library semantic gating, query compilation, adapter behavior, corpus helpers, and resolver scaffolding are all covered.

## Review-1 Finding Status

### 1. `hug lint --experimental-semantics` does not enable semantic diagnostics

Status: fixed.

Evidence:

- `AnalysisOptions` carries `experimental_semantics` from `CommandKind::Lint`.
- `analyze_tree_file` sets `tree_file.experimental_semantics = options.experimental_semantics` before analysis.
- CLI regression coverage exists in `test_lint_experimental_semantics_enables_semantic_diagnostics`, including default suppression, opt-in output, and `--strict` failure behavior.

### 2. Diagnostic metadata is not attached by the library for all diagnostics

Status: fixed for the reviewed surface.

Evidence:

- Pattern and semantic diagnostics now populate `metadata: Some(self.diagnostic_metadata(...))`.
- CLI conversion preserves diagnostic metadata in `diagnostics_from_index`.
- Syntax JSON metadata is covered by `test_lint_json_includes_syntax_metadata`.

### 3. Oxlint integration is implemented as an adapter but not wired into CLI lint

Status: fixed.

Evidence:

- The CLI imports `OxlintAdapter` and calls `add_external_lint_diagnostics` for JavaScript and TypeScript lint summaries.
- `test_lint_invokes_oxlint_for_javascript` installs a fake `oxlint`, verifies adapter invocation, and asserts the external diagnostic metadata.

### 4. Cache implementation is broad, but diagnostic invalidation is still incomplete

Status: materially improved; no stale diagnostic cache issue found.

Evidence:

- `AnalysisOptions::fingerprint()` includes experimental semantics, deny/warn/allow selectors, strict mode, and external adapter enablement.
- Cached symbol indexes are stripped through `symbol_cache_index`, which clears diagnostics before storage.
- Cache hits are rehydrated with `tree_file.try_diagnostics()` before use.

Remaining suggestion: add an end-to-end CLI cache regression that runs the same file with and without `--experimental-semantics` while cache is enabled and asserts the second run cannot reuse stale diagnostics.

### 5. Rule alias registration is incomplete for post-registration alias mutation

Status: fixed.

Evidence:

- `dead-code` is registered with `aliases: vec!["unreachable-code".to_string()]`.
- `registry_lookup_by_alias` asserts `registry.get("unreachable-code").unwrap().id == "dead-code"`.

## Outstanding Suggestions

These are not blockers for closing `review-1.md`, but they remain useful follow-ups from the broader original lint review:

- Add CLI coverage for `--deny`, `--warn`, and `--allow` selectors against real lint output and exit-code behavior.
- Add an end-to-end persistent-cache test for lint policy and experimental semantic changes.
- Surface Oxlint unavailable metadata or a capability warning for JS/TS lint runs when the adapter is absent; the current CLI silently falls back.
- Consider moving rule metadata out of hard-coded Rust tables into structured query-adjacent files if that original design requirement still matters.
- Add generated docs or a `hug lint --list-rules` style command if rule metadata is intended to be user-visible outside diagnostics.

## Recommendation

`review-1.md` can be considered implemented. The current validation gates are clean, and the previously reported user-facing lint regressions have direct test coverage.
