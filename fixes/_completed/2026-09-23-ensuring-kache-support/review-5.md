---
$schema: feature-review.yaml
ready: false
findings:
  - title: Invalid worktree-base settings can receive a healthy kache verdict
    priority: high
  - title: CI does not select the kache contract tests for their script inputs
    priority: high
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-24T03:02:11-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: true
next: 2026-09-23-ensuring-kache-support/review-6.md
implemented_by: claude/opus
log: fixes/2026-09-23-ensuring-kache-support/implementation-log.md
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-5.md
previous: 2026-09-23-ensuring-kache-support/review-4.md
---

# Review 5

**Not production-ready.** Both review 4 findings are implemented. Two further gaps affect the promised worktree-base verdict and the selection of its contract tests when implementation files change alone. Neither needs a human design decision.

## Previous findings

| Review 4 finding | Review 5 assessment |
| --- | --- |
| A legacy Cargo config can hide the wrapper written by init | Addressed. `scripts/kache-host.sh` reads only the legacy `config` when both names exist in a directory; `_ensure-kache` writes and neutralizes that same file. Selected L1 tests exercise both filenames in the host and project directories, and real Cargo checks confirm which wrapper runs. |
| ReFS worktree-base reporting uses the Unix clone probe | Addressed. Qualification now calls `clone_check` for a covered base. The selected emulated ReFS test configures `WT`, makes a Unix clone attempt fail, and confirms the base still reports cloning with no such attempt. |

Review 4 has no `## Blocked Findings` section; nothing needed reclassification.

## Unblocked Findings

### High — Invalid worktree-base settings can receive a healthy kache verdict

The spec says the worktree base is resolved as `wt` resolves it and that `kache-status` polices its coverage once configured. `wt` rejects an existing `WT` path that names a Git repository, a missing `WT` path, malformed `~/.worktree.json`, and a configured path that does not exist (`worktree/lib/src/config.rs:58-108`). `scripts/kache-host.sh:173-190` instead suppresses JSON errors, checks only that the candidate is a directory, and returns the same failure for an invalid setting as for no setting. `resolve_base` converts every failure to `base=unconfigured` (`scripts/kache-host.sh:199-213`). Qualification then says “worktree base unconfigured,” and an otherwise healthy active `kache-status` exits zero because it judges only `covered` and `off-device` bases (`justfile:1547-1554`). For example, `WT=/definitely/missing/base` is an explicit broken configuration to `wt` but appears unconfigured to kache; `WT` pointing at an existing Git repository on the serving device appears covered.

Preserve the distinction between absent and invalid configuration, including the reason and path. Make status fail with drift for an invalid configured base, and make init report the invalid setting explicitly while retaining the spec's rule that the base does not gate filesystem qualification. Add selected L1 fixture cases for missing `WT`, malformed JSON, and a Git-repository base, checking the same outcome against `wt`'s resolver contract.

### High — CI does not select the kache contract tests for their script inputs

The executable kache behavior is in `scripts/kache-host.sh` and `scripts/kache-config-merge.py`, while its behavioral contract tests live in `tools/test-toolkit`. The planner classifies `.sh` and `.py` as source (`scripts/ci/affected_scope.py:607-611`) and deliberately removes source paths before scanning tests that read them (`scripts/ci/affected_scope.py:4350-4366`). A change to either script alone selects its owning `repo-deps` package under `scripts/`, not the `test-toolkit` tests that invoke it. A direct scan confirms references in `kache_host_contracts` and `kache_config_merge_contracts`, but the planner never uses those references for these source changes. The recipe-level `kache_init_contracts`, `kache_ensure_contracts`, and `kache_status_contracts` read the live scripts through a symlink to the top-level `scripts/` directory (`tools/test-toolkit/tests/common/kache.rs:111-116`), so they have no individual script references either. A script-only regression can therefore miss the tests that prove init order, activation, and status.

There is a narrower version of the same problem for a `justfile`-only change: `kache_recipe_contracts` reads it through `read("justfile")` and `repo_root().join(relative)` (`tools/test-toolkit/tests/kache_recipe_contracts.rs:25-38`). The test-input scanner does not resolve that dynamic join; an actual scan selected the kache init/ensure/status binaries for `justfile`, but no `kache_recipe_contracts` test. Those text checks guard the macOS source fallback and CI step-aside behavior, so they need to run when the recipe changes.

Give the planner an explicit, narrow way to schedule cross-package contract tests for changed script source, and spell the `justfile` read in a supported literal-root form in the recipe test binary. Verify a resolved plan for each of the three changed-file-only cases selects the relevant tests. This is test reachability, not a request for a broader full-suite CI run.

## Verification level by user-facing requirement

| Requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Filesystem qualification, store placement, and checkout clone verdict | L1 subprocess fixtures with actual probe calls; recorded live macOS checks | L1 filesystem/process boundary is appropriate. |
| Worktree-base discovery, clone coverage, and drift | L1 fixtures for valid bases, including emulated ReFS; no invalid-config case | L1 is the appropriate level, but the first high finding demonstrates a false healthy verdict. |
| Latest install, macOS `DYLD_*` forwarding, and wasm link | L1 installer/probe fixtures plus recorded live macOS passthrough and link checks | L1 process and real-compiler checks are appropriate. |
| Config pin, launcher parity, daemon lifecycle, and idempotence | L1 recipe fixtures and recorded live host checks | L1 is appropriate; script-only changes do not select the recipe fixtures in CI. |
| Cargo activation and status, including legacy config precedence | L1 recipe fixtures and real Cargo checks | L1 is appropriate; review 4's false activation is fixed. |
| Below-floor interactive confirmation | L1 PTY and subprocess fixtures | L1 is appropriate: the requirement concerns prompt/response handling, not terminal input encoding. |
| Cache restore after destroying and recreating a worktree | Real-resource test with two Git worktrees, Cargo, and a scratch kache daemon/store | Appropriate real-cache boundary; required mode passed with three misses in A and three hits in B, no new artifact entries or bytes. |

There is no specified glyph, styling, scroll, keyboard-encoder, mouse, paste, or IME behavior that calls for L2 or L3 terminal verification. The kache test binaries use Cargo's default integration-test discovery, have no tier marker in their L1 names, and are selected by `just test`; the `real_` test is selected by the live `tools/test-real` recipe. `just check-tier-coverage tools` reported zero stranded tests.

## Checks run

- `just test` in `tools/`: 438 selected L1 tests passed, 3 skipped.
- `BISCUIT_KACHE_REAL_REQUIRED=1 just test-real --no-capture` in `tools/`: the one real-cache test passed; B had three hits and zero misses, with the artifact tree unchanged.
- `just lint` in `tools/`, `shellcheck scripts/kache-host.sh`, and `python3 -m py_compile scripts/kache-config-merge.py`: passed.
- `just check-tier-coverage tools`: zero stranded tests.
- Ran `scripts/ci/test_inputs.py`'s scanner against actual `test-toolkit` Cargo metadata for `justfile` and the two scripts; it confirmed the missing recipe-test reference described above. Planner source filtering explains the script-only case.

Cross-OS CI results and later human review are outside this readiness decision.
