---
$schema: feature-review.yaml
ready: false
findings:
  - title: A missing wrapper executable is reported as healthy kache
    priority: high
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T22:14:38-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/2026-09-23-ensuring-kache-support/implementation-log.md
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-3.md
previous: 2026-09-23-ensuring-kache-support/review-2.md
next: 2026-09-23-ensuring-kache-support/review-4.md
---

# Review 3

**Not production-ready.** The four findings in review 2 are implemented and their regression tests pass. Review 2 has no `## Blocked Findings` section, so none needed reclassification. A separate wrapper-identity defect still lets init and status certify a build environment in which Cargo cannot start its configured wrapper.

## Previous findings

| Review 2 finding | Review 3 assessment |
| --- | --- |
| Cargo configuration in ancestor directories escapes wrapper detection | Addressed. `scripts/kache-host.sh wrapper` walks ancestor `.cargo` directories in Cargo precedence order. A selected L1 test compares its result with a real Cargo invocation, and the init failure and status tests exercise parent configuration. |
| Init can claim activation while Cargo uses another wrapper | Addressed for empty and foreign `RUSTC_WRAPPER`, `CARGO_BUILD_RUSTC_WRAPPER`, and repository or parent config overrides. Init now reports `activation INCOMPLETE`; selected L1 fixtures exercise each override. The new finding below concerns a different case: a wrapper classified as kache by filename that Cargo cannot execute. |
| Init accepts a running daemon at the wrong version after restart | Addressed. The bounded post-restart wait requires the installed version. A selected L1 fixture keeps the old version after a successful restart command and verifies that activation does not occur. |
| Worktree restore behavior has no repeatable real-cache test | Addressed. `real_kache_worktree_restore` is compiled and selected by the live `tools/test-real` recipe. With its required-resource flag set, it built three cacheable crates in worktree A, removed A, then built B: three misses followed by three hits and zero misses, with unchanged entries, store bytes, and artifact-tree size. |

## Findings

### High — A missing wrapper executable is reported as healthy kache

`scripts/kache-host.sh:660-664` classifies any wrapper path whose basename is `kache` or `kache.exe` as active kache without checking whether Cargo can execute that path. `_ensure-kache` uses this classification for its post-activation success report (`justfile:1140-1147`), and `kache-status` uses it for its healthy verdict (`justfile:1345-1363`, `1452-1517`). On this host, `RUSTC_WRAPPER=/definitely/missing/kache just kache-status` exited **0** and printed `active YES` plus the healthy clone verdict. A scratch `cargo check` under the same environment exited **101**: Cargo could not execute `/definitely/missing/kache`. The selected `status_reports_a_kache_config_wrapper_as_active_by_name_or_path` test currently expects a nonexistent `/opt/kache/bin/kache` to be healthy, so it reinforces the false result.

Resolve the effective wrapper to an executable and verify that the binary being certified is the one whose version and macOS passthrough were checked. Report an incomplete activation or status drift when the path is missing or points elsewhere. Add selected L1 cases for a missing path and a different executable named `kache`, including a real Cargo invocation for the missing-path regression.

## Verification level by user-facing requirement

| Requirement | Strongest verification | Required level / result |
| --- | --- | --- |
| Filesystem qualification, placement, clone checks, and worktree-base drift | L1 subprocess fixtures, plus recorded live host probes | L1 process and filesystem verification is appropriate; no terminal encoder or rendered-pane claim exists. |
| Non-qualifying host stays off; CI init does not manage kache | L1 subprocess fixtures for non-qualifying filesystems and a recipe contract for the CI guard | L1 is appropriate; the recorded WSL ext4 run confirms the negative host path. |
| Install/upgrade, macOS `DYLD_*` forwarding, and wasm link | L1 recipe/probe fixtures and recorded live Mac release probe and wasm link | L1 process verification and the specified real-host link check are appropriate. |
| Old store is reported without deletion | L1 recipe contract and recorded live Mac abandoned-store report | L1 is appropriate; the recipe never issues a store deletion. |
| User config pin, daemon lifecycle, launcher parity, and idempotence | L1 subprocess fixtures and recorded live shell, launcher, daemon, and repeat-init checks | L1 is appropriate. The old-daemon-after-restart regression is selected and passes. |
| Activation and status verdicts | L1 subprocess fixtures, including an ancestor-config case checked against real Cargo | L1 is the appropriate level, but the missing-path case proves that the current assertions do not verify an executable effective wrapper. This is the high finding above. |
| Below-floor prompt and non-interactive refusal | L1 PTY and subprocess fixtures | L1 is appropriate; no real terminal input encoder is part of the requirement. |
| Destroy and recreate a worktree, then restore from the store | A selected real-resource test with actual kache, Cargo, git worktrees, and a scratch daemon/store | The real-cache boundary is appropriate. It ran with `BISCUIT_KACHE_REAL_REQUIRED=1`, so a missing resource could not turn into a passing skip. |

The six targeted kache L1 test binaries passed **78/78** with nextest. `just check-tier-coverage tools` reported **zero stranded tests**. `BISCUIT_KACHE_REAL_REQUIRED=1 just test-real --no-capture` selected and passed the real-cache test, reporting A's three misses, B's three hits and no misses, and unchanged `entries`, `store_bytes`, and artifact-tree size. The status/Cargo mismatch above was reproduced separately. Cross-OS results are CI evidence and do not determine this readiness decision.
