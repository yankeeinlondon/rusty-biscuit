---
$schema: feature-review.yaml
ready: false
findings:
  - title: A legacy Cargo config can hide the wrapper written by init
    priority: high
  - title: ReFS worktree-base reporting uses the Unix clone probe
    priority: medium
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-24T02:28:11-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: true
next: 2026-09-23-ensuring-kache-support/review-5.md
implemented_by: claude/opus
log: fixes/2026-09-23-ensuring-kache-support/implementation-log.md
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-4.md
previous: 2026-09-23-ensuring-kache-support/review-3.md
---

# Review 4

**Not production-ready.** Review 3's missing-wrapper finding is addressed, but Cargo can still ignore the file in which init activates kache. No human design decision is needed for either finding.

## Previous findings

| Review 3 finding | Review 4 assessment |
| --- | --- |
| A missing wrapper executable is reported as healthy kache | Addressed. `scripts/kache-host.sh wrapper` now resolves a kache-named winner, checks that it is executable, and compares it with the installed `kache` on `PATH`. Init reports incomplete activation and status exits with drift for a missing or different executable. Selected L1 tests cover both recipes; the missing-path status test also confirms that real Cargo fails under that wrapper. |

Review 3 has no `## Blocked Findings` section, so no finding needed reclassification.

## Unblocked Findings

### High — A legacy Cargo config can hide the wrapper written by init

`_ensure-kache` always writes `$CARGO_HOME/config.toml` (`justfile:1127-1147`). The wrapper helper reads **both** `config` and `config.toml` in each directory (`scripts/kache-host.sh:699-724`) and treats a wrapper found in the latter as effective even when the former exists. Cargo instead uses the legacy `config` file and ignores `config.toml` when both exist. In an isolated scratch project, `$CARGO_HOME/config` contained only `[term] color = "never"`, while `config.toml` named `/definitely/missing/kache`. The helper reported that missing wrapper as the winner; `cargo check --offline --quiet` exited 0 and warned that it was using `config`. The same arrangement with init's valid `rustc-wrapper = "kache"` makes init report successful activation while Cargo builds without kache. Status can likewise report kache active based on an ignored file.

Make the helper follow Cargo's one-file-per-directory choice. Ensure activation writes the file Cargo will actually use, or explicitly report incomplete activation when an existing legacy file prevents that write. Add selected L1 cases with both filenames, including a real Cargo check that observes whether the wrapper ran. This is a correctness gap in the spec's activation and status outcomes, not a cross-OS evidence gap.

### Medium — ReFS worktree-base reporting uses the Unix clone probe

On Windows, `clone_check` decides by same-device ReFS filesystem type (`scripts/kache-host.sh:252-267`), as the spec requires. Qualification uses that check for the checkout, but when a worktree base is configured on the same device it calls `clone_probe` directly (`scripts/kache-host.sh:453-462`). That function invokes `cp --reflink=always` outside macOS (`scripts/kache-host.sh:224-240`), rather than the ReFS check. Init can therefore print `clone probe store -> base: FAILED` for a covered ReFS base even though qualification succeeds and `kache-status` reports it covered. This line does not gate activation, but it gives a conflicting answer to the person setting up worktrees. Use `clone_check` for the base too, and add a selected ReFS fixture with `WT` configured. The existing ReFS fixtures leave the base unconfigured.

## Verification level by user-facing requirement

| Requirement | Strongest verification found | Required level / result |
| --- | --- | --- |
| Filesystem qualification, placement, and checkout clone checks | Selected L1 subprocess fixtures plus live Mac probes | L1 process and filesystem checks are appropriate; there is no rendered-terminal or keyboard-encoder behavior. |
| Worktree-base coverage and drift | Selected L1 fixtures for Unix-like hosts; ReFS fixtures have no configured base | L1 is appropriate. The ReFS base path needs the case described above. |
| Install or upgrade, macOS `DYLD_*` forwarding, and wasm link | Selected L1 recipe and passthrough fixtures plus recorded live Mac probe and link | L1 process checks and the real compiler link are the appropriate boundaries. |
| User config pin, launcher parity, daemon lifecycle, and idempotence | Selected L1 subprocess fixtures plus recorded live launcher and daemon checks | L1 is appropriate. The prior wrong-version-after-restart regression remains selected. |
| Activation and status verdicts | Selected L1 fixtures and real Cargo checks for ancestor and missing-path cases | L1 is appropriate, but no test covers Cargo's choice when both `config` and `config.toml` exist. The high finding demonstrates a wrong verdict. |
| Below-floor confirmation and non-interactive refusal | Selected L1 PTY and subprocess fixtures | L1 is appropriate: the contract is prompt and response handling, with no special terminal key encoding. |
| Destroy and recreate a worktree, then restore from the shared store | Selected real-resource test using Cargo, kache, Git worktrees, and a scratch daemon/store | Appropriate real-cache boundary. With `BISCUIT_KACHE_REAL_REQUIRED=1`, it passed: A had three misses, B had three hits and no misses, with unchanged entries, store bytes, and artifact tree. |

`just test` in `tools/` passed **430/430** selected L1 tests (3 skipped). `just check-tier-coverage tools` found **zero stranded tests**; the real-cache test is compiled and selected by the live `tools/test-real` recipe. `BISCUIT_KACHE_REAL_REQUIRED=1 just test-real --no-capture` passed its one selected test. `just lint`, `shellcheck scripts/kache-host.sh`, and Python syntax compilation passed. These results do not resolve the two findings above. Cross-OS CI evidence and human review do not determine this readiness verdict.
