---
ready: true
agent: codex
model: ""
---

# Review: Scope-Complete JSON for `sniff repo` - Iteration 3

## Findings

No production-blocking findings.

The implementation now distinguishes bare `sniff repo` from explicit `sniff repo name`, routes bare `repo --json` through a scope-complete aggregate, excludes `remote`, `pr`, and `hash`, adds the three identity leaves, and keeps `repo name -v` leaf-only while preserving the rich parent `repo -v` output.

## Test Rigor Assessment

- `sniff repo --json` aggregate shape, required key presence, network/parameterized key exclusion, stable empty values, and child-key round-tripping are covered by Level 1 CLI integration tests in `sniff/cli/tests/cli.rs`.
- `sniff repo is-monorepo`, `sniff repo package-count`, and `sniff repo version` single-key JSON and text behavior are covered by Level 1 CLI integration tests, including `version: null` exit-code handling and `--no-error`.
- `sniff repo name --json`, `sniff repo name -v`, `sniff repo`, and `sniff repo -v` field inclusion/exclusion are covered by Level 1 CLI integration tests.
- Aggregate builder internals, file-list stable shapes, package-family stable shapes, locator/boolean outcomes, worktrees JSON, and commit-family aggregate objects are covered by Level 1 unit tests in `sniff/cli/src/output/repo_json.rs`.

Level 1 is appropriate for these requirements because they are JSON/stdout/stderr/exit-code and field-subset contracts. The spec does not require terminal-emulator rendering fidelity, keyboard input behavior, mouse/paste/IME behavior, scrolling, or glyph-width verification, so Level 2 or Level 3 coverage is not required for production readiness.

## Verification

- `cargo test -p sniff-cli repo_ --color=never` passed: 122 unit tests and 163 CLI integration tests ran under the `repo_` filter.
- `cargo run -p sniff-cli --bin sniff --quiet -- repo --json` exited `0`, emitted valid JSON on stdout, and emitted no stderr.
- `cargo run -p sniff-cli --bin sniff --quiet -- repo name -v` exited `0` and printed only `rusty-biscuit`.

## Notes

I noticed a few internal comments in `repo_json.rs` still refer to implementation phases from the plan, but they do not describe public behavior incorrectly and are not a readiness issue.
