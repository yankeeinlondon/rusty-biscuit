---
$schema: feature-review.yaml
ready: false
findings:
  - title: An existing unwritable store directory stops the placement cascade
    priority: high
  - title: An alternate kache config can bypass init's store pin
    priority: high
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-24T03:30:10-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: true
next: 2026-09-23-ensuring-kache-support/review-7.md
implemented_by: claude/opus
log: fixes/2026-09-23-ensuring-kache-support/implementation-log.md
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-6.md
previous: 2026-09-23-ensuring-kache-support/review-5.md
---

# Review 6

**Not production-ready.** Both review 5 findings are addressed. A placement defect can turn kache off despite a usable store location and can remove a pre-existing empty directory. A kache config-path override can also leave init's claimed store different from the one a shell uses for builds. Neither requires a human design decision.

## Previous findings

| Review 5 finding | Review 6 assessment |
| --- | --- |
| Invalid worktree-base settings can receive a healthy kache verdict | Addressed. `scripts/kache-host.sh` now distinguishes an absent base from missing `WT`, malformed config, and a Git-repository base. Init reports an invalid setting without gating filesystem qualification, while active `kache-status` exits with drift. The L1 host, init, and status cases run in the kache test binaries. |
| CI does not select the kache contract tests for their script inputs | Addressed. `test-toolkit` declares the two scripts under `source-inputs`, the contract binaries spell literal root joins, and the planner adds narrowed L1 cells. Resolved plans for each script include the relevant kache binaries; the `justfile` plan includes `kache_recipe_contracts`. The 11 focused planner tests passed. |

Review 5 has no `## Blocked Findings` section; there was nothing to reclassify.

## Unblocked Findings

### High — An existing unwritable store directory stops the placement cascade

The spec's placement cascade tries the mount-point `kache/` directory on macOS when writable, then a same-device user cache directory, then a user-owned directory on the volume. In `scripts/kache-host.sh:363-375`, the macOS branch checks `-w` only on its first attempt. If `<mount>/kache` already exists but is unwritable, `mkdir -p` still succeeds, so the script chooses that directory and marks it as newly created. The shared mount-point attempt at `scripts/kache-host.sh:404-412` has the same issue. `clone_check` then fails to create its source probe and `qualify` returns `no-qualify` without trying the later placements (`scripts/kache-host.sh:508-516`). On failure, `drop_created_candidate` may remove the pre-existing directory if empty (`scripts/kache-host.sh:451-463`).

I reproduced this on the dev Mac with a scratch mount-point stand-in: an existing mode-0500 `volume/kache`, a writable `home/Library/Caches/kache` on the checkout's device, and a successful `cp -c` from that cache into the checkout. `qualify` still returned 1 with `clone-unsupported`, and the pre-existing empty `volume/kache` was gone afterward. This violates the required fallback and makes `just init` leave kache off on a host whose filesystem and later placement qualify.

Accept an existing candidate only when it is writable and on the checkout's device. Record `CREATED_CANDIDATE` only when this invocation actually created the directory, and continue to the next placement when an existing one is unusable. Add a selected L1 fixture for the macOS first choice and the shared mount-point choice, asserting the fallback verdict and preservation of the original directory; include a negative case where every candidate is unusable.

### High — An alternate kache config can bypass init's store pin

The spec promises that `~/.config/kache/config.toml` is the store's single source of truth for shells, the daemon, editors, and jobs. Kache also supports `KACHE_CONFIG` as a config-file selector. With kache 0.26.3 and two scratch config files, both setting `ignore_env = true`, `kache doctor --json` reported the default file's store normally and the alternate file's store when `KACHE_CONFIG` named it. `ignore_env` does not prevent that selection.

`_ensure-kache` gets the store from `kache doctor` before the write (`justfile:1016-1029`), writes the pin only to the fixed user config path (`justfile:1036-1043`), and activates without checking the effective store again (`justfile:1131-1151`). If a shell exports `KACHE_CONFIG` pointing to an off-device store, init chooses a clone-capable cascade candidate and pins it in the default file, but that shell's later kache builds still read the alternate file and restore by copy. `kache-status` then reports pin drift, while init already claimed setup was successful. A service whose launch environment selects another config can diverge from the shell for the same reason; the daemon report currently reads its running version but does not compare its reported config path with the managed file (`scripts/kache-host.sh:601-624`).

Check the effective config path and store after writing the pin, before activation, for the init environment and the daemon. When either uses a different config file, name that override and leave the wrapper off until the single-source contract holds. Add an L1 fixture with an alternate `KACHE_CONFIG` file and an off-device store, asserting init's warning, inactive wrapper, and status drift; use a real kache doctor invocation in the fixture or a separate process test to pin the selector behavior.

## Verification level by user-facing requirement

| Requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Filesystem qualification and store placement | L1 subprocess fixtures run the real host script and clone commands; live macOS `kache-status` re-probes the store | L1 filesystem/process verification is the appropriate level, but the high finding shows that fallback placement is incomplete. |
| Worktree-base discovery and coverage | L1 fixtures cover valid, missing, malformed, Git-repository, and emulated ReFS bases; active status must fail on invalid or off-device bases | L1 is appropriate. The prior false healthy verdict is fixed. |
| Latest install, macOS environment passthrough, and wasm linking | L1 installer/probe fixtures plus the recorded real macOS passthrough and compiler-link checks in `implementation-log.md` | L1 process and real-compiler verification is appropriate. |
| Config pin, launcher parity, daemon lifecycle, and activation | L1 recipe fixtures cover order, restarts, failures, idempotence, and Cargo wrapper precedence; live status reports the daemon, pin, wrapper, and clone facts | L1 is appropriate, but the alternate-config scenario in the second high finding is untested and can break launcher parity. |
| Below-floor interactive confirmation | L1 PTY tests answer the prompt with manufactured input | L1 is appropriate: the contract concerns prompt handling, not a terminal emulator's key encoder. |
| Cache reuse after destroying and recreating a worktree | The live real-resource tier builds two Git worktrees against a scratch daemon/store and asserts hit and store deltas | Appropriate real-cache boundary. This review's required-mode run observed three misses in A, three hits and zero misses in B, and unchanged artifact entries and bytes. |

The spec does not require physical key-press detection, terminal-specific glyph widths or styling, scrolling, mouse, paste, or IME behavior. No requirement here calls for Level 2 terminal capture or Level 3 OS keyboard injection. The kache L1 binaries are compiled by Cargo's default integration-test discovery and selected by nextest; the `real_` binary has a live `tools/test-real` recipe. `just check-tier-coverage tools` reported zero stranded tests.

## Checks run

- Focused nextest kache L1 selection: 102 passed, 2 skipped.
- `BISCUIT_KACHE_REAL_REQUIRED=1 just test-real --no-capture` in `tools/`: 1 passed with the cache deltas above.
- The 11 `SourceInputSelectionTests` and `RealWorkspaceTestInputTests` passed; resolved plans were inspected for `scripts/kache-host.sh`, `scripts/kache-config-merge.py`, and `justfile`.
- `just lint` in `tools/`, `shellcheck scripts/kache-host.sh`, `python3 -m py_compile scripts/kache-config-merge.py`, and `just check-tier-coverage tools` passed.
- Live `just kache-status` on the dev Mac exited zero and reported an active kache 0.26.3, a running matching daemon, a matching config pin, clone-capable checkout and base, and passing `DYLD_*` passthrough.
- A scratch, read-only reproduction established the placement failure and pre-existing-directory removal described above.
- A scratch kache 0.26.3 doctor probe confirmed that `KACHE_CONFIG` selects a different store despite `ignore_env = true` in the default file.

Cross-OS CI evidence and any later human review are outside this readiness decision.
