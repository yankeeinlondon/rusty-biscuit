---
$schema: feature-review.yaml
ready: false
findings:
  - title: Status can certify a filesystem that cannot clone
    priority: high
  - title: An empty Cargo wrapper is reported as active kache
    priority: high
  - title: Status omits required daemon and configuration drift checks
    priority: high
  - title: Fresh ReFS hosts cannot place an off-device default store
    priority: high
  - title: The placement cascade misses writable directories on the checkout volume
    priority: high
  - title: A pre-activation failure can leave an inherited wrapper active
    priority: high
  - title: Init and status behavior lacks repeatable end-to-end tests
    priority: high
  - title: Targeted config edits discard inline comments
    priority: medium
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T19:53:50-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: true
next: 2026-09-23-ensuring-kache-support/review-2.md
implemented_by: claude/opus
log: fixes/2026-09-23-ensuring-kache-support/implementation-log.md
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-1.md
---

# Review 1

**Not production-ready.** The macOS implementation log records successful live checks, and the new toolkit tests run, but several host states produce incorrect setup or status results. The issues below are implementation and repeatable-test gaps, rather than a request for more cross-OS CI evidence or human sign-off.

## Findings

### High — Status can certify a filesystem that cannot clone

The spec requires `kache-status` to agree with the filesystem qualification and fail loudly on drift. `scripts/kache-host.sh:294` decides qualification with a same-device **clone probe**, but `do_report` at `scripts/kache-host.sh:337-374` reports only device IDs. `justfile:1310-1322` then treats matching IDs as a healthy clone-capable setup. A store and checkout on the same ext4 or non-cloning ZFS volume can therefore receive `VERDICT: active on a filesystem that clones blocks`, even though `qualify` rejects them. This can hide copy restores and undermine the reason for activating kache only on qualifying filesystems.

Run the same clone check from the effective doctor-reported store to the checkout, and to the configured worktree base where applicable, when producing status. Add an L1 subprocess test with a controlled clone failure that asserts a nonzero status and the actual reason. The current L1 `qualify_prints_a_stable_verdict_line_that_matches_its_exit_code` checks only output shape on whichever filesystem happens to host the test.

The “verdict and report cannot disagree” claims in `scripts/kache-host.sh` and `docs/initialization.md` drifted from this behavior. Update them with the implementation; the code path establishes the defect.

### High — An empty Cargo wrapper is reported as active kache

`justfile:1244-1259` marks a Cargo config active whenever it contains the text `rustc-wrapper`, without parsing its value. Yet the failure path deliberately writes `rustc-wrapper = ""` at `justfile:906`, which disables the wrapper. I reproduced this with a scratch `CARGO_HOME/config.toml` containing only `[build] rustc-wrapper = ""` and `RUSTC_WRAPPER=''`: `just kache-status` exited 0 and printed both `active YES` and `VERDICT: active on a filesystem that clones blocks`. On a non-qualifying host the same setup would be falsely reported as drift.

Determine the **effective value** through Cargo configuration semantics, distinguishing empty from `kache` and from another wrapper. Add an L1 status test for empty, kache, and unrelated wrapper values, including the post-failure neutralized state. The present recipe text assertion checks only that a drift exit exists.

### High — Status omits required daemon and configuration drift checks

The outcome explicitly requires `just kache-status` to report the installed version, daemon, store pin, environment override protection, and activation, and to fail when they drift. The recipe at `justfile:1225-1343` prints a below-floor version but does not fail for it. It never reads the daemon's installed/running/version state, `[cache] local_store`, or `ignore_env`. Thus a stopped daemon, old daemon binary, removed or changed pin, disabled `ignore_env`, or below-floor binary can receive a healthy verdict while the wrapper remains active. Checking `kache doctor`'s `Cache dir` alone does not establish that the user config is the source of truth or that the daemon is usable.

Have status inspect those state sources and report each failing invariant. Use L1 subprocess fixtures with a fake `kache doctor --json`/`kache daemon --json` and isolated config files to prove both healthy and drift verdicts. The current L1 status contract is a source-text assertion, not an invocation of the recipe.

### High — Fresh ReFS hosts cannot place an off-device default store

The spec says a ReFS Dev Drive gets the full qualifying setup. The Windows qualification branch at `scripts/kache-host.sh:256-282` emits `candidate=-` for ReFS. In `_ensure-kache`, if `kache doctor` reports its default store on a different device, `justfile:983-991` has no candidate and calls `kache_off`. A fresh Windows host with a ReFS checkout and a default store under AppData on an NTFS system drive is exactly this case. The outcome's store placement and activation are therefore unavailable on that qualifying layout.

Select and validate a writable store directory on the ReFS device before install, and carry it through the same placement step as macOS/Linux. Add a platform-scoped L1 test for ReFS checkout plus off-device default; the existing `kache_host_contracts` suite has no Windows branch test. This is a code-path defect, independent of whether native-Windows CI has run yet.

### High — The placement cascade misses writable directories on the checkout volume

The spec's final fallback is **a user-owned directory on the serving volume**. After rejecting an off-device user cache, `candidate_store` at `scripts/kache-host.sh:219-230` only tries `<mount point>/kache`. If the mount point is root-owned but the checkout or one of its parents is writable by the user, it reports `no-user-writable-store-location-on-the-checkout-volume` before attempting a clone. This is a plausible Linux btrfs/XFS layout with a separate home and a repository in a user-owned subdirectory.

Search for a suitable user-owned location on the checkout volume and validate it with the same device and clone checks. Add an isolated L1 fixture that controls the candidate layout and proves this fallback; the fresh-home test only exercises a cache directory on the checkout's device.

### High — A pre-activation failure can leave an inherited wrapper active

The spec's failure contract says every failure before activation leaves kache off. `kache_off` at `justfile:902-913` can only neutralize one double-quoted spelling in `$CARGO_HOME/config.toml`. If the invoking environment already has `RUSTC_WRAPPER=kache`, Cargo still uses it after `kache_off` prints `kache left OFF`. If the config write fails, the helper also prints that it could not neutralize an existing activation and then claims kache is off. The remedy printed for a failure must reflect the effective wrapper state rather than asserting a guarantee it cannot provide.

Check the effective wrapper after attempted neutralization and make an unresolved activation a clear error or explicit manual-action state. Add L1 subprocess tests for an inherited wrapper and a non-writable activation config. The current failure-contract test only looks for the helper's text in the justfile.

### High — Init and status behavior lacks repeatable end-to-end tests

The central behavior is an ordered host setup with failure recovery. `tools/test-toolkit/tests/kache_recipe_contracts.rs` checks recipe substrings and their textual order, so it cannot distinguish a successful daemon restart from a failed restart followed by activation, or an actual non-interactive refusal from a printed promise. The implementation log's macOS/WSL runs are useful live evidence, but they are not repeatable regression tests. It also notes that the daemon-version mismatch branch was not fired live. There is no L1 PTY test for the interactive below-floor confirmation branch and no controlled L1 test of the non-interactive error branch.

Run `_ensure-kache` and `kache-status` in isolated homes with controlled fake `kache`, `cargo`, daemon JSON, and filesystem-probe results. Assert the observable config files, exit codes, command order, restart triggers, and failure recovery. Exercise the yes/no prompt via an L1 PTY and the non-interactive branch without a TTY. Keep a small real-host smoke path for the macOS signing and daemon behavior. These requirements need L1 subprocess/PTY verification; they do not require Level 2 rendering capture or Level 3 keyboard injection because no terminal-emulator rendering or special key event is part of the contract.

### Medium — Targeted config edits discard inline comments

The spec requires the merge to preserve existing comments, including the host's rationale for tuned settings. `scripts/kache-config-merge.py:120-124` replaces an entire matching key line. In a scratch config, `local_store = "/old" # retain this placement reason` and `ignore_env = false # intentional for old setup` became lines without either comment after a successful merge. This loses user-authored context on the very keys the helper updates. The current tests explicitly exclude lines containing `local_store` from their preservation assertion and provide no inline-comment fixture for a targeted key.

Retain each target line's trailing TOML comment when replacing its value, and add an L1 round-trip fixture that proves it survives a changed write and a repeated no-op write.

## Verification level by user-facing requirement

| Requirement | Strongest evidence found | Appropriate level and remaining gap |
| --- | --- | --- |
| Filesystem qualification and worktree coverage | L1 host-dependent `qualify` tests; manual APFS/WSL checks in the implementation log | L1 deterministic subprocess fixtures are needed for clone success/failure, fallback, and base drift. No L2/L3 requirement. |
| Latest install, macOS `DYLD_*` passthrough, and usable wasm link | L1 discriminating passthrough tests; manual pristine-release and wasm runs on macOS | L1 tests cover the probe itself. Installer wiring is text-only; add isolated command-path tests. The real macOS link run is valuable host evidence. No L2/L3 requirement. |
| Config pin, `ignore_env`, and launcher parity | L1 merge tests; manual shell/launcher and daemon control runs | L1 merge covers serialization, while repeatable setup/status tests must cover the effective config and daemon behavior. No L2/L3 requirement. |
| Daemon install/restart, activation last, and failure recovery | Recipe-text assertions; manual macOS checks, with the version-mismatch trigger unexercised | L1 isolated subprocess tests must exercise command order, both restart triggers, and failure outcomes. No L2/L3 requirement. |
| Interactive and non-interactive below-floor handling | Recipe-text assertion; manual negative-host exercise | L1 PTY test for confirmation and L1 no-TTY subprocess test for refusal are missing. Special-key encoder testing is not relevant. |
| `kache-status` healthy and drift verdicts | Recipe-text assertion; one manual healthy status run | L1 invocations with controlled healthy and drift states are missing. No L2/L3 requirement. |
| Worktree destroy/recreate without store growth | Manual macOS lifecycle run with reported 395/395 local hits | A repeatable integration fixture with a controlled store, or a gated real-resource test, would protect this outcome. No terminal level applies. |

The three new toolkit binaries are declared test targets (`cargo metadata`) and selected by L1; `just check-tier-coverage test-toolkit` reported zero stranded tests. `cargo nextest run --color=never -p test-toolkit --test kache_host_contracts --test kache_config_merge_contracts --test kache_recipe_contracts` passed **30/30** on this macOS host. Passing those tests does not change the finding about their behavioral boundary.
