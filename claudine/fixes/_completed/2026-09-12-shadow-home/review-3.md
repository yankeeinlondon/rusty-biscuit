---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T16:31:12-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-12-shadow-home/implementation-log.md
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-3.md
previous: 2026-09-12-shadow-home/review-2.md
next: 2026-09-12-shadow-home/review-4.md
findings:
    - "[high] Recovery still deletes state after metadata, source-read, or marker-write failures"
---

# Review 3: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The fix is **not ready for production**. The native-Windows Level 2 harness is
now structurally capable of reaching the launch, and Kilo's published inline
MCP capability is wired through direct, composition, proxy, and retry paths.
The write-back repair also preserves a rotated credential when the final atomic
write itself fails.

The durability contract remains incomplete, however. Other ordinary I/O
failures are still classified as an absent entry or a concurrent source change,
which makes the lease delete the only changed copy. Even a correctly detected
failure becomes sweepable when the recovery marker cannot be written. These
are the same data-loss class as review #2's first finding.

Human review is not required. The specification already decides the policy:
any changed provider state that cannot be persisted must remain recoverable.

## Prior Review Closure

Review #2 had no `## Blocked Findings` section, so no blocked finding became
unblocked before this implementation.

| Previous finding | Status in this iteration |
| --- | --- |
| Failed provider-state write-back is logged and then destroyed | **Partially implemented.** Atomic-write failure is now typed, reported, retained, marked, and protected from later sweeps. Metadata/source-read failures and marker-write failure still permit deletion; finding 1 remains. |
| The native-Windows Level 2 test still cannot reach the launch | **Implemented.** The launcher supplies a fixture-owned `CODEX_HOME` source and absolute `CLAUDINE_OVERLAY_DIR`, keeps the raw home variables unchanged, and waits on a bounded `cmd.exe` readiness condition instead of a fixed delay. |
| Kilo still advertises inline MCP delivery that is not wired into the runtime | **Implemented.** `KiloInjector` emits the researched `KILO_CONFIG_CONTENT` shape, preserves existing inline config, and is exercised through direct and provider-transition L1 launches. |

## Findings

### 1. Recovery still deletes state after metadata, source-read, or marker-write failures (high)

`WriteBack::apply_entry` documents that read/write failures enter
`WriteBackOutcome::failed`, but it discards the error from
`symlink_metadata(&entry.overlay)` as if the provider removed the entry
(`lib/src/provider_overlay/write_back.rs:208-212`). A permission or transient
filesystem error therefore produces `Ok(None)`. It also turns a failed source
read into “different,” then discards any source fingerprint error with `.ok()`
and reports a source conflict (`write_back.rs:216-226`). In both cases
`outcome.failed` stays empty, so `OverlayLease::finish` removes the launch root
and the only changed credential copy.

The detected-failure path has a second hole. If writing `<root>.retained`
fails, `finish` records `marker: None` but still removes the lock file and drops
the lock (`lib/src/provider_overlay/lease.rs:146-175`). The next launch's sweep
then treats the retained root as abandoned and deletes it. Full disk is named
as a supported failure case in the spec and documentation, but it is also a
direct reason the marker write can fail.

The tests exercise only a Unix read-only source directory that lets the
overlay metadata/read and marker write succeed. Both the unit and process-level
tests return successfully without assertions when their permission premise is
not enforced (`lib/src/provider_overlay/tests.rs:781-824` and
`cli/tests/level1_provider_overlay_home.rs:1751-1811`). That is a false-green L1
path, and there is no deterministic Windows sharing-violation or injected-I/O
equivalent even though native Windows is a primary motivation for write-back.

Required change: distinguish `NotFound` from every other overlay metadata
error, propagate source read/fingerprint errors into `failed`, and make sweep
protection durable even when the notice file cannot be created. Add
deterministic, cross-platform fault injection at the filesystem boundary for
metadata, source read, atomic write, and marker write; L1 must fail rather than
return as a pass when a test premise is unavailable. Each case must prove that
the changed bytes survive a later sweep and that the user receives a
non-secret recovery notice.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, composed, retry, resume, and proxy launches preserve the launch home variables | Level 2 tmux on Unix plus Level 1 fake-provider launches | Appropriate and green for exercised paths. The native-Windows L2 test is now reachable by design; collecting cross-OS execution evidence is external to readiness. |
| No launch writes an overlay or null device into a global home variable | Level 2 tmux plus Level 1/unit guards | Appropriate and green. |
| Explicit provider roots are sources while provider selectors point at the correct overlay shape | Level 1 fake-provider launches plus units | Appropriate and green, including the explicit source/storage shape used by the Windows L2 test. |
| Repository resources stay masked and overlapping launches remain isolated | Level 2 tmux for the interactive Unix launch, Level 1 overlapping real-binary launches, and units | Appropriate and green. |
| Codex prompt/MCP injection works while SQLite stays outside the overlay | Level 2 tmux plus Level 1 fake-provider launches | Appropriate and green. |
| Gemini MCP writes beneath `GEMINI_CLI_HOME` | Level 1 direct/composition fake-provider launches | Appropriate and green. |
| OpenCode and Kilo MCP stay inline without unnecessary filesystem overlays | Level 1 direct and transition fake-provider launches plus injector units | Appropriate and green. |
| Unsupported provider/reason pairs refuse before provider spawn | Level 1 fake-provider launches plus units | Appropriate and green. |
| Mutable top-level provider state survives successful write-back | Level 1 real-binary launch plus units | Appropriate and green. |
| Mutable state remains recoverable after every persistence I/O failure | Level 1/unit coverage for one Unix atomic-write denial | Incomplete: metadata, source-read, marker-write, and deterministic native-Windows-equivalent cases are missing, and the implementation deletes state in those branches; finding 1. |
| Nested `git`, `gpg`, and `gh` observe the original home | Level 2 tmux plus Level 1 fixture tools | Appropriate and green on the exercised Unix path; the Windows L2 implementation covers the same boundary when run. |
| Native Windows recursively materializes directories without links | Level 1/unit copy-mode tests plus a reachable Level 2 WezTerm test | Correct test level and harness design. Cross-OS result collection itself does not affect readiness. |

Level 3 is not required: no requirement depends on a terminal emulator encoding
an OS keyboard or mouse event.

## Verification Performed

- Refreshed GitNexus for this exact worktree. Its graph finds the new lease
  finalization and Kilo injector but reports lower-bound/dispatch boundaries,
  so direct text search was used for unresolved dynamic-dispatch callers.
- `CARGO_TARGET_DIR=<fresh temp> just test-library provider_overlay`: **36
  passed**.
- `CARGO_TARGET_DIR=<fresh temp> just test-library mcp::inject`: **14 passed**.
- `CARGO_TARGET_DIR=<fresh temp> just test-cli --test
  level1_provider_overlay_home`: **33 passed, 0 skipped**.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux CARGO_TARGET_DIR=<fresh temp> just
  _test_l2 claudine-cli --features terminal-tests --test
  level2_provider_overlay_capture`: **1 passed**, with tmux execution evidence.
- `CARGO_TARGET_DIR=<fresh temp> just lint`: **passed**. The linker emitted the
  existing non-fatal `__eh_frame section too large` warning while building the
  debug CLI binary.

The shared target directory was read-only, so the first focused command failed
before reviewed code compiled. Every result above uses a fresh temporary target
directory.

## Design Assessment

The provider overlay plan remains well-factored: provider capability metadata
selects the strategy, launch-owned leases isolate concurrency, and Kilo reuses
the verified OpenCode-shaped emitter without adding a new provider dispatch
site. The Windows test override moves only overlay storage and does not weaken
the home-identity contract.

Representing failed write-back as typed data is also the right direction. The
remaining defect is that the filesystem boundary does not preserve that type
for every error, and the sweep's durable keep decision depends on successfully
creating another file in the same failure environment. Recovery needs one
fail-closed ownership state that survives all of those branches.
