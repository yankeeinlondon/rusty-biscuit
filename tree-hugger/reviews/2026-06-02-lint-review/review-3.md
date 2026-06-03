# Lint Review Follow-Up Validation

Date: 2026-06-03

Validated review: `tree-hugger/reviews/2026-06-02-lint-review/review-2.md`.

## Summary

Most of the actionable follow-ups from `review-2.md` have been implemented and are covered by focused CLI tests. Tree Hugger package tests and clippy are clean. The remaining gaps are narrow:

- The persistent-cache regression covers `--experimental-semantics`, but it does not also assert cached behavior for `--deny`, `--warn`, or `--allow` selector changes.
- Rule metadata was not moved to structured query-adjacent files. The implementation instead corrected the registry docs to describe the current built-in registry model.
- Repo-wide validation is not clean because unrelated workspace tests and clippy fail outside tree-hugger.

## Review-2 Suggestion Status

### 1. CLI coverage for `--deny`, `--warn`, and `--allow`

Status: implemented.

Evidence:

- `test_lint_policy_selectors_affect_severity_and_exit_code` covers `--warn unwrap-call`, `--deny unwrap-call`, `--deny category:suspicious`, and `--allow unwrap-call` against real lint output and exit-code behavior.
- The test intentionally uses `--no-cache`, so this validates policy behavior directly without cache effects.

### 2. End-to-end persistent-cache coverage for lint policy and experimental semantics

Status: partially implemented.

Evidence:

- `test_lint_cache_separates_experimental_semantics_policy` runs the same file twice with cache enabled: first default lint, then `--experimental-semantics`, and verifies `undefined-symbol` appears only after opt-in.
- `AnalysisOptions::fingerprint()` includes experimental semantics, deny/warn/allow selectors, strict mode, and external adapter enablement.

Remaining gap:

- No end-to-end cached CLI test changes only `--deny`, `--warn`, or `--allow` and verifies severity/exit-code behavior under cache reuse. This is lower risk because diagnostics are stripped before symbol cache storage and policy is applied after summarization, but it is still part of the review-2 suggestion.

### 3. Oxlint unavailable metadata or warning

Status: implemented.

Evidence:

- CLI JSON output now includes `adapter_metadata`.
- Pretty/plain output emits a warning when Oxlint is unavailable.
- `test_lint_warns_when_oxlint_is_unavailable` covers the human warning.
- `test_lint_json_reports_unavailable_oxlint_metadata` covers JSON adapter metadata.

### 4. Query-adjacent rule metadata

Status: not implemented; design consciously remains built-in registry based.

Evidence:

- `RuleMetadata` docs now say rule metadata is registered by the built-in registry and validated at startup.
- No structured metadata files were added next to lint queries.

Outstanding suggestion:

- If query-adjacent metadata is still a design requirement, add structured rule metadata files and load/validate them from the query layer. If not, consider closing this as intentionally declined and document the built-in registry as the supported rule metadata source.

### 5. User-visible rule metadata docs or `hug lint --list-rules`

Status: implemented.

Evidence:

- `hug lint --list-rules` was added.
- Plain output lists rule id, category, default severity, confidence, default-on/off status, and title.
- JSON output returns structured rule metadata including aliases, language support, and experimental-gating flags.
- CLI README documents the new command and lint policy flags.
- `test_lint_list_rules_exposes_rule_metadata` and `test_lint_list_rules_json_exposes_rule_metadata` cover both output modes.

## Test Coverage Assessment

Tree Hugger coverage is high for the updated surfaces:

- Selector policy behavior and exit codes are covered through real CLI invocations.
- Rule metadata listing is covered for plain and JSON output.
- Persistent-cache behavior is covered for the experimental semantics gate.
- Oxlint available and unavailable paths are covered, including JSON metadata.
- Existing library coverage remains broad for rule registry behavior, adapter normalization, cache primitives, diagnostic metadata, query compilation, and semantic gating.

Suggested test addition:

- Add one cached CLI regression that runs the same file with cache enabled under two different policy selectors, for example default `unwrap-call` warning followed by `--deny unwrap-call`, and asserts the second run exits failure and renders `error`.

## Validation Commands

- `cargo test -p tree-hugger-cli --test cli --color=never`: passed, 77 tests.
- `cargo test -p tree-hugger --color=never`: passed.
- `cargo test -p tree-hugger-cli --color=never`: passed.
- `cargo clippy -p tree-hugger --all-targets --color=never -- -D warnings`: passed.
- `cargo clippy -p tree-hugger-cli --all-targets --color=never -- -D warnings`: passed.

Repo-wide gates are not clean:

- `cargo test --workspace --color=never`: failed outside tree-hugger in `biscuit-clipboard-service` test `tests::test_args_default_port_is_17530`; observed port was `12345`, expected `17530`.
- `cargo test --workspace --color=never` also emitted unrelated warnings in `sniff-cli` tests for unused variables `worktree_path` and `dir`.
- `cargo clippy --workspace --all-targets --color=never -- -D warnings`: failed outside tree-hugger in `biscuit-test-harness/src/layout_invariants.rs` for two `clippy::useless_format` diagnostics.

## Recommendation

The review-2 implementation is acceptable for the tree-hugger package area, with one small follow-up recommended for cached selector-policy coverage. Do not claim repo-wide tests or repo-wide clippy are clean until the unrelated workspace failures above are fixed or excluded from the validation scope.
