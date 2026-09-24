---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-24T03:53:47-07:00"
spec: 2026-09-23-ensuring-kache-support/spec.md
implemented: false
description: "A **fix** review of `2026-09-23-ensuring-kache-support/spec.md`"
fix: 2026-09-23-ensuring-kache-support/review-7.md
previous: 2026-09-23-ensuring-kache-support/review-6.md
---

# Review 7

**Production ready.** Both review 6 findings are addressed. The remaining contract paths have tests at the boundary needed to observe their behavior, and this review found no blocking implementation or verification gap. No human design decision is needed. Cross-OS CI evidence and any later human review are separate release processes.

## Previous findings

| Review 6 finding | Review 7 assessment |
| --- | --- |
| An existing unwritable store directory stops the placement cascade | Addressed. Every placement now passes through `use_candidate`, which requires a writable directory on the checkout's device and records cleanup ownership only for directories this call created. The selected L1 subprocess cases cover both macOS and shared mount-point attempts, an off-device existing directory, fallback placement, preservation of existing directories, and the no-usable-placement verdict. |
| An alternate kache config can bypass init's store pin | Addressed. After the pin write, init checks the CLI's selected config and doctor's resolved store before daemon work; after daemon setup, it checks the running daemon's reported config path before activation. An override leaves the wrapper off with a named warning. Status reports the same condition as drift. L1 recipe fixtures exercise both override sources, and a required real-kache test confirms the selector, resolved stores, and daemon-reported path against scratch configs. |

Review 6 has no `## Blocked Findings` section, so there was nothing to reclassify.

## Findings

None.

## Verification level by user-facing requirement

| Requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Filesystem qualification, placement cascade, and clone verdict | L1 filesystem subprocess fixtures invoke the real host script and clone commands; live macOS status re-probes the store and checkout | L1 is appropriate for filesystem and process behavior. The prior fallback and ownership defect has selected regression tests. |
| Worktree-base discovery and coverage | L1 fixtures cover valid, absent, invalid, off-device, and emulated ReFS bases; the real-resource tier builds successive Git worktrees | L1 verifies decisions; the real-resource run verifies the promised restore behavior across worktrees. |
| Latest installation, macOS environment passthrough, and wasm linking | L1 installer and passthrough fixtures, plus recorded real macOS compiler-link and pristine-release control checks in `implementation-log.md` | L1 process tests and the real compiler check exercise the required boundary. |
| Config pin, launcher parity, daemon lifecycle, and activation | L1 recipe fixtures cover ordering, restarts, idempotence, failure handling, Cargo precedence, and CLI/daemon config overrides; a real-kache scratch daemon confirms the reported config path; live status checks the host | L1 and real-process verification are appropriate. The selector can no longer silently bypass the managed pin during init. |
| Below-floor interactive confirmation | L1 PTY tests answer the prompt with manufactured input | L1 is appropriate: the requirement is prompt handling, not terminal input encoding. |
| Cache reuse after deleting and recreating a worktree | Real-resource test builds two Git worktrees against a scratch daemon and store, then checks hits and artifact/store deltas | Appropriate real-cache boundary. This review observed three misses in A, three hits and zero misses in B, with unchanged artifact entries and bytes. |

The spec does not require terminal-specific rendering, physical key-press detection, mouse, paste, or IME behavior. No requirement calls for Level 2 terminal capture or Level 3 OS keyboard injection. The kache L1 tests are compiled through Cargo's integration-test discovery and selected by `just test`; the `real_` tests are selected by the live `tools/test-real` recipe. `just check-tier-coverage tools` reported zero stranded tests.

## Checks run

- `just test` in `tools/`: 459 passed, 4 skipped by the L1 tier filter.
- `BISCUIT_KACHE_REAL_REQUIRED=1 just test-real --no-capture` in `tools/`: 2 passed. The worktree restore case recorded A misses=3 and B hits=3, misses=0, with 3 entries and 26,115 artifact bytes at both measurements of the shared store.
- `just lint` in `tools/`, `shellcheck scripts/kache-host.sh`, and `just check-tier-coverage tools`: passed.
- Live `just kache-status` on the dev Mac exited zero: kache 0.26.3 active, daemon running at the same version, config pin matching, checkout and worktree base clone-capable, and macOS passthrough passing.

The spec's section 2 precedence list predates the discovered `KACHE_CONFIG` selector; the operational docs now describe it. Correcting that historical wording would make the spec clearer, but the implemented contract and its tests handle the selector.
