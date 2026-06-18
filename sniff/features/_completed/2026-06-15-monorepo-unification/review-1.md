---
ready: true
agent: codex
model: ""
---

# Review: Monorepo Type Unification

## Findings

No blocking findings.

The implementation matches the reviewed spec's breaking-contract intent: `MonorepoTool`,
`PackageDiscoverySource`, `RepoInfo.monorepo_tool`, `RepoInfo.workspace_tools`,
`Package.discovery_sources`, `LayerPackage`, `format_monorepo_tool`, and the
`standard_for_tool` bridge are removed from the Sniff library/CLI production
surface. The new topology model is the canonical shape: `RepoInfo.monorepo_standards`,
`RepoInfo.monorepo_layers`, `Package.standard`, and `Package.provenance`.

## Requirement Coverage

- Legacy JSON keys are absent: verified at Level 1 through serde/JSON builder
  tests and symbol greps. This is appropriate because the observable contract is
  deterministic JSON shape, not terminal-emulator behavior.
- `MonorepoLayer.packages` contains repo-relative path strings resolving to
  `RepoInfo.packages[].relative`: verified at Level 1 with integration tests,
  including nested Bazel workspace coverage.
- Package authority/provenance is assertable directly on `RepoInfo.packages`:
  verified at Level 1 through library integration tests and CLI JSON builder
  tests.
- The Nx + pnpm category-error fix is covered: packages are owned by
  `pnpm-workspaces`, while `nx` appears as a layer orchestrator. Verified at
  Level 1, which is appropriate for library/JSON data semantics.
- `PackageEcosystem` is retained and documented as distinct from
  `MonorepoStandard::spec().primary_language` and `Package.primary_language`.
- claudine no longer depends on Sniff's removed `monorepo_tool` field. Its
  compatibility alias is derived from `monorepo_standard`, documented as
  deprecated, and covered by Level 1 tests.
- CLI text derives its monorepo one-liner from `monorepo_layers`. The output uses
  `biscuit-terminal` renderables. There is no requirement here for keyboard
  input, paste/IME, mouse behavior, scrolling, or exact SGR/color capture, so
  Level 2/Level 3 coverage is not required for production readiness.

## Verification Run

- `git grep -n "MonorepoTool\|PackageDiscoverySource\|discovery_sources\|format_monorepo_tool\|standard_for_tool" -- sniff/lib sniff/cli`
  - Remaining hits are only tests asserting `discovery_sources` is absent.
- `git grep -n "monorepo_tool\|workspace_tools" -- sniff/lib sniff/cli`
  - Remaining hits are only tests/assertions proving legacy keys are absent.
- `cargo test --color=never -p sniff monorepo`
  - Passed: 26 matching tests across unit/integration/fixture targets.
- `cargo test --color=never -p sniff-cli topology`
  - Passed: 5 matching tests, including repo JSON topology and snapshot coverage.
- `cargo test --color=never -p claudine monorepo`
  - Passed: 4 matching claudine topology/template compatibility tests.

## Residual Risk

The full workspace test suite was not run. Given the scoped change and the
targeted Sniff/CLI/claudine coverage above, I do not see a production blocker.
