---
$schema: feature-review.yaml
ready: false
findings:
  - title: Cargo configuration in ancestor directories escapes wrapper detection
    priority: high
  - title: Init can claim activation while Cargo uses another wrapper
    priority: high
  - title: Init accepts a running daemon at the wrong version after restart
    priority: high
  - title: Worktree restore behavior has no repeatable real-cache test
    priority: high
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T21:41:45-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/2026-09-23-ensuring-kache-support/implementation-log.md
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-2.md
previous: 2026-09-23-ensuring-kache-support/review-1.md
next: 2026-09-23-ensuring-kache-support/review-3.md
---

# Review 2

**Not production-ready.** The prior review's eight unblocked findings have implementation and test changes. There was no `## Blocked Findings` section in review 1, so none needed reclassification. The new behavioral suites pass, but the four gaps below still affect the spec's activation, status, daemon, and worktree outcomes.

## Previous findings

| Review 1 finding | Review 2 assessment |
| --- | --- |
| Status can certify a filesystem that cannot clone | Addressed: `report` repeats the clone probe from the effective store; status tests force checkout and base clone failures. |
| An empty Cargo wrapper is reported as active kache | Addressed for the sources the helper reads: empty, kache, foreign, and environment wrappers have behavioral tests. The ancestor-source omission below limits the effective-wrapper claim. |
| Status omits required daemon and configuration drift checks | Addressed: active status checks the floor, daemon, pin, and `ignore_env`, with a healthy control and individual drift tests. |
| Fresh ReFS hosts cannot place an off-device default store | Addressed: the Windows branch now selects a ReFS candidate; isolated ReFS and NTFS recipe tests run. |
| The placement cascade misses writable directories on the checkout volume | Addressed: a user-owned ancestor is considered, with success and failure fixtures. |
| A pre-activation failure can leave an inherited wrapper active | Addressed as an explicit manual-action state for inherited, unwritable, or undecidable activation; tests ensure the recipe does not claim it is off. The ancestor-source omission below still hides another form of active wrapper. |
| Init and status behavior lacks repeatable end-to-end tests | Addressed for recipe ordering, both restart triggers, failure recovery, non-interactive refusal, PTY confirmation, and status drift by subprocess fixtures. The actual worktree restore outcome remains untested as noted below. |
| Targeted config edits discard inline comments | Addressed: changed and repeated no-op writes preserve inline comments in fixture tests. |

## Findings

### High — Cargo configuration in ancestor directories escapes wrapper detection

`scripts/kache-host.sh:667-683` reads only the current directory's `.cargo` files and `$CARGO_HOME`; its comment explicitly omits Cargo's ancestor-directory search. This affects both `kache-status` and `_ensure-kache`'s failure recovery. I reproduced it in a scratch project: a parent `.cargo/config.toml` set `rustc-wrapper = "kache"`; `kache-host.sh wrapper` returned `wrapper=none`, while `cargo check` invoked a deliberately failing `kache` stub from that parent setting. Consequently, `just kache-status` can print `VERDICT: not in use` and exit zero while Cargo actually runs kache. A pre-activation failure can likewise claim `kache left OFF` although the ancestor config still activates it. Walk the same ancestor config locations Cargo reads and add an L1 fixture with a parent config to prove status and failure behavior against an actual Cargo invocation.

### High — Init can claim activation while Cargo uses another wrapper

On a qualifying host, `_ensure-kache` writes `$CARGO_HOME/config.toml` at `justfile:1115` and unconditionally prints activation at `justfile:1152`. It never checks the effective wrapper after the write. An inherited `RUSTC_WRAPPER=""`, `RUSTC_WRAPPER=sccache`, `CARGO_BUILD_RUSTC_WRAPPER`, or repository config can outrank the new host entry. Cargo then never runs kache, while init announces a completed setup; status even reports `not in use` for the same state. The existing precedence tests prove those values can shadow the host config, but no qualifying-init test supplies one. Verify the effective wrapper after activation and report a failed or incomplete activation rather than success; add L1 qualifying-host cases for the overriding values.

### High — Init accepts a running daemon at the wrong version after restart

`justfile:1094-1111` restarts a version-mismatched daemon, then `wait_daemon` succeeds as soon as `daemon_running=yes` (`justfile:1079-1085`). It never checks the newly reported `daemon_version` against the installed version before writing the Cargo wrapper. If a service manager reports a successful restart request but the old daemon remains reachable, init activates and prints a successful setup, despite the spec's same-version daemon requirement; a later `kache-status` would correctly report drift. The fixture's fake restart always updates the version synchronously, so the current restart tests cannot expose this. Require the expected version in the post-restart wait and add an L1 fake-daemon case where restart exits zero but the old version persists.

### High — Worktree restore behavior has no repeatable real-cache test

The outcome promises that a newly created worktree restores from the shared store without a kache setup step or material store growth. `implementation-log.md:659-677` records a successful live Mac run (395/395 local hits in the fresh worktree), which is useful evidence. The selected kache tests use a fake `kache` and fake clone command; none creates, removes, and rebuilds a worktree with a real store or asserts cache hits and store size. This user-visible outcome therefore has manual evidence but no repeatable integration test at the resource level it needs. Add a scratch-store test that builds a small fixed fixture in two successive worktrees, checks real kache hits and bounded store growth, and register it in a live tier/recipe that actually selects it. It needs a real cache and filesystem; terminal rendering capture and OS keyboard injection are unrelated.

## Verification level by user-facing requirement

| Requirement | Strongest verification now | Required level / result |
| --- | --- | --- |
| Filesystem qualification, ReFS placement, and base drift | L1 subprocess fixtures with controlled device and clone behavior; live host checks in the implementation log | L1 is appropriate. The probe and drift paths have selected tests. |
| Install, macOS `DYLD_*` forwarding, and wasm link | L1 passthrough tests and recipe fixture; live Mac probe and wasm link | L1 process verification plus the real host link check is appropriate; no L2/L3 input or rendering claim. |
| Store pin, `ignore_env`, and launcher parity | L1 config merge/status fixtures; live shell/launcher and daemon checks | L1 process checks are appropriate; the specified live launcher check was run. |
| Daemon lifecycle and activation last | L1 ordered recipe fixtures, plus live Mac daemon checks | L1 is appropriate, but no test models a successful restart that remains on the old version, and qualifying init does not verify Cargo's effective wrapper. |
| Below-floor prompt and refusal | L1 PTY confirmation/decline and L1 no-TTY refusal | L1 PTY/subprocess is appropriate; no terminal encoder behavior is required. |
| `kache-status` healthy and drift verdicts | L1 subprocess tests | L1 is appropriate, but the helper misses ancestor Cargo config, so this observable verdict is not fully verified. |
| Worktree destroy/recreate and restore hits | One manual real-host run; no selected repeatable test | A real-cache integration test is needed. The current L1 fake cannot establish this outcome. |

The six kache test binaries are compiled and selected by L1: `cargo nextest run --color=never -p test-toolkit --test kache_host_contracts --test kache_config_merge_contracts --test kache_recipe_contracts --test kache_ensure_contracts --test kache_init_contracts --test kache_status_contracts` passed **67/67** on macOS. `just check-tier-coverage test-toolkit` reported **zero stranded tests**. Cross-OS proof is left to CI and does not determine this readiness decision.
