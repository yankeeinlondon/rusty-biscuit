---
ready: true
agent: codex
model: ""
---

# Review: Monorepo CLI Wiring Cleanup

## Findings

No blocking findings.

The previous review's issues are addressed:

- `format_monorepo_label` now renders every orchestrator in the layer, preserving layer order (`Nx + Lerna (using pnpm workspaces)`), and has L1 unit coverage for zero, one, and multiple orchestrators.
- `RepoInfo::primary_layer()` now has L1 unit coverage for the documented fallback cases: no repo-root layer selects the shallowest nested layer, and same-depth nested ties use `MonorepoStandard` declaration order rather than iteration order.

## Non-Blocking Note

- `sniff/cli/tests/cli.rs` still has a test comment that says "workspace tools" when describing preserved `repo structure --json --filter` fields. The production D3 comment drift in `sniff/cli/src/output/repo_json.rs` is fixed, and this test comment does not describe public behavior incorrectly, but it is stale cleanup worth folding into a follow-up.

## Test Rigor

All user-observable requirements in this feature are deterministic library or CLI output behavior, so Level 1 is the appropriate verification level:

- D1 primary-layer selection: L1 unit tests.
- D2 `MonorepoLayer.packages: Vec<String>` and package catalog join behavior: L1 integration/unit coverage.
- D4/D5 unified text label composition: L1 unit tests plus CLI integration tests.
- D5 focused `repo is-monorepo` text, JSON, `--no-error`, predicate exit code, valid JSON on predicate failure, and genuine failure behavior: L1 CLI integration tests.
- Aggregate `sniff repo --json` legacy `"is-monorepo": bool` compatibility: L1 integration coverage.

No Level 2 or Level 3 tests are required for this feature. The spec does not assert real terminal rendering fidelity, terminal encoder behavior, keyboard input, paste/IME, mouse behavior, or scrolling.

## Verification Run

- `cargo test --color=never -p sniff primary_layer`
- `cargo test --color=never -p sniff-cli monorepo_label`
- `cargo test --color=never -p sniff-cli is_monorepo_outcome`
- `cargo test --color=never -p sniff-cli repo_is_monorepo`

## Production Readiness

Ready for production. The designed behavior is implemented, the focused leaf and aggregate JSON contracts are separated as specified, and the strongest required verification level is present for each user-facing requirement.
