---
ready: false
agent: codex
model: ""
---

# Review: More Repo

## Findings

### High: bare `sniff repo --json` omits `package_manager` and `test_runner`

The consolidated aggregate spec includes `package_manager: string | string[] | null` and `test_runner: string | string[] | null` as repo-wide facts, but the implemented `SniffRepo` projection does not define or populate either field (`sniff/cli/src/output/repo_json.rs:623`). `build_aggregate_value` fills packages, package areas, dependency projections, git status, branches, worktrees, context, change buckets, and commit families, then serializes without the package-manager/test-runner facts (`sniff/cli/src/output/repo_json.rs:772`). The aggregate tests encode the same incomplete key list, so they pass while the contract is still missing two informational children (`sniff/cli/tests/cli.rs:350`, `sniff/cli/src/output/repo_json.rs:2427`).

Verification level: L1 integration/unit coverage exists for aggregate shape, but it verifies the wrong contract. No L2/L3 coverage is required for this JSON shape.

### High: repo test-runner detection can report false positives for section-sensitive configs

The test-runner strategy treats several config signals as section-sensitive: `tox.ini [pytest]`, `setup.cfg [tool:pytest]`, `tox.ini [tox]`, `setup.cfg [unittest]`, and package.json config keys for Jest/Mocha/AVA/tap (`sniff/features/2026-06-14-more-repo/test-runner-strategy.md:207`, `sniff/features/2026-06-14-more-repo/test-runner-strategy.md:221`). The implementation checks only `pkg_dir.join(glob).exists()` for config hits (`sniff/lib/src/filesystem/repo/test_runner_usage.rs:111`), while the catalog lists broad filenames like `tox.ini` and `setup.cfg` for both pytest/nose2/tox (`sniff/lib/src/programs/test_runner_spec.rs:243`, `sniff/lib/src/programs/test_runner_spec.rs:262`, `sniff/lib/src/programs/test_runner_spec.rs:271`).

That means a package with `tox.ini` containing only `[tox]` is detected as both `tox` and `pytest`; a package with `setup.cfg` for nose2 can be detected as pytest as well. The current tests cover positive cases, but not negative/disambiguation cases.

Verification level: L1 unit tests are the right level for manifest/config detection, but coverage is incomplete for false positives and evidence-source correctness.

### Medium: hard-break call-site/docs audit is incomplete

The spec requires every in-repo invocation of removed command surfaces to be migrated in the same change (`sniff/features/2026-06-14-more-repo/spec.md:420`). `git grep` still finds stale references to removed commands in authoritative skill docs, including `sniff programs`, `sniff editors`, and `sniff programs install` in `.claude/skills/sniff/programs.md:73`, plus `sniff repo deps --ui` in `.claude/skills/biscuit-visualized/SKILL.md:49` and `.claude/skills/biscuit-visualized/graph-rendering.md:239`.

These are not runtime call sites, but the feature explicitly calls out skill/doc migration for a hard break, and the local skill catalog is authoritative for agent workflows.

Verification level: L1 grep/audit is sufficient; no terminal-level testing is needed.

## Test Notes

- Passed: `cargo test --color=never -p sniff --lib filesystem::repo::test_runner_usage -- --nocapture`
- Passed: `cargo test --color=never -p sniff-cli repo_json::tests -- --nocapture`

I did not run L2/L3 tests. The reviewed requirements are JSON/CLI command contracts and library detection logic; Level 1 is the appropriate verification level except for any future requirement that asserts terminal-rendered styling through a real emulator.
