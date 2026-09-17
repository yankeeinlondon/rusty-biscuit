---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T19:57:07-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-12-shadow-home/implementation-log.md
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-4.md
previous: 2026-09-12-shadow-home/review-3.md
findings:
    - "[medium] Unwritable-storage L1 test silently passes when its premise is unavailable"
---

# Review 4: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The fix is **not ready for production**. Review #3's high-severity durability
finding is implemented: metadata, source-read, source-fingerprint, atomic-write,
and recovery-notice failures retain the changed overlay state, and the new
fault-injection tests prove that a later sweep does not delete it. The working
tree also contains the corrected non-`NotFound` metadata arm that the
implementation log says was accidentally omitted from commit `57e23751f`.

One directly specified L1 contract still has a false-green branch. The
unwritable-storage test returns successfully when its permission premise cannot
be established, so a privileged runner can report that the typed pre-spawn
failure passed without exercising it. Human review is not required; the test
contract already determines the repair.

## Prior Review Closure

Review #3 contains neither an `## Unblocked Findings` nor an `## Blocked
Findings` section. Its `## Prior Review Closure` says review #2 had no blocked
findings, so no blocked item became newly actionable before this implementation.

| Review #3 finding | Status in this iteration |
| --- | --- |
| Recovery still deletes state after metadata, source-read, or marker-write failures | **Implemented.** `WriteBack::apply_entry` treats only `NotFound` as removal and propagates the other inspection failures. `OverlayLease::finish` protects retained roots by renaming or removing the lock before the best-effort notice write, and the sweep keeps marker-bearing and lockless retained roots. Deterministic tests cover every named failure and a later sweep. |

## Findings

### 1. Unwritable-storage L1 test silently passes when its premise is unavailable (medium)

`cli/tests/level1_provider_overlay_home.rs:1337-1363` is the process-level test
for specification contract 9: an unwritable overlay storage root must produce
`provider.overlay_failed`, must not spawn the provider, and must not substitute
a null home. The test checks whether mode `0o555` is actually unwritable, but
lines 1351-1353 print `skipping` and return normally when it is not. Nextest
therefore records a pass, not a skip or failure, despite none of the contract
assertions running.

This is the same premise problem that review #3 required the write-back L1 test
to stop hiding. That sibling now fails explicitly with a useful message on a
privileged runner (`a_rotated_token_that_cannot_be_written_back_stays_recoverable`),
while this test retains the old false-green behavior. The usual CI legs are
expected to run unprivileged, but the test itself promises portable L1 evidence
and cannot silently weaken that promise based on the runner account.

Required change: replace the successful early return with a failing premise
assertion, matching the repaired write-back test, or add deterministic
cross-platform filesystem fault injection at the materialization boundary and
use the process test only as an explicitly enforced unprivileged-host check.
The test must never report success without observing the typed diagnostic,
absence of a provider spawn, and absence of overlay storage.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, composed, sequence, retry, resume, and proxy launches preserve the launch home variables | Level 2 tmux on Unix plus Level 1 fake-provider launches | Appropriate for the behavior; green for the exercised paths. Cross-OS result collection is external to readiness. |
| No launch writes an overlay or null device into a global home variable | Level 2 tmux plus Level 1 process and unit guards | Appropriate and green. |
| Explicit provider roots remain sources while selectors point at the provider-visible overlay shape | Level 1 fake-provider launches and units | Appropriate and green, including paths with spaces, absent variables, non-UTF-8 Unix values, and native-Windows path construction. |
| Repository resources remain masked and overlapping launches remain isolated | Level 2 tmux for the interactive Unix launch, Level 1 real-binary launches, and units | Appropriate and green. |
| Codex prompt/MCP injection works while SQLite remains outside the overlay | Level 2 tmux plus Level 1 fake-provider launches | Appropriate and green. |
| Gemini MCP writes beneath `GEMINI_CLI_HOME` | Level 1 direct/composition fake-provider launches | Appropriate and green. |
| OpenCode and Kilo MCP remain inline without unnecessary filesystem overlays | Level 1 direct and provider-transition launches plus injector units | Appropriate and green. |
| Unsupported provider/reason pairs refuse before provider spawn | Level 1 fake-provider launches plus units | Appropriate and green. |
| Materialization failures are typed, pre-spawn failures with no null-home fallback | Level 1 real-binary tests plus units | **Incomplete evidence:** the unwritable-storage process case can pass without exercising the failure; finding 1. Other deterministic materialization failures are green. |
| Mutable provider state survives successful write-back | Level 1 real-binary launch plus units | Appropriate and green. |
| Changed provider state remains recoverable after persistence inspection/write/notice failures | Level 1 deterministic fault-injection units plus a real-binary permission-failure test | Appropriate and green. Each injected failure asserts retained bytes, non-secret recovery paths, and survival through a later sweep. |
| Nested `git`, `gpg`, and `gh` observe the original home | Level 2 tmux plus Level 1 fixture tools | Appropriate and green on the exercised paths. |
| Native Windows recursively materializes directories without links | Level 1 copy-mode units plus a reachable Level 2 WezTerm test | Correct verification level and harness shape. Cross-OS execution evidence itself is not a readiness input. |

Level 3 is not required because no requirement depends on an operating system
keyboard or mouse event, terminal input encoding, hotkey, paste, or IME path.

## Verification Performed

- Refreshed GitNexus for this exact worktree; it is current at `171b971`. The
  graph reports exact low-risk impact for `WriteBack::apply_entry` and a
  lower-bound low-risk result for `OverlayLease::finish`; direct source search
  was used because receiver typing dropped unresolved `finish` call sites.
- `CARGO_TARGET_DIR=<fresh temp> just test-library provider_overlay`: **42
  passed**.
- `CARGO_TARGET_DIR=<same fresh temp> just test-cli --test
  level1_provider_overlay_home`: **33 passed, 0 skipped**. This host enforced
  the read-only-directory premise, so the false-green branch did not execute.
- `CARGO_TARGET_DIR=<same fresh temp> just lint`: **passed**. The preliminary
  CLI guard build emitted the existing non-fatal macOS linker warning about the
  compact-unwind `__eh_frame` size; Clippy completed successfully.
- `git diff --check`: **passed** before the review metadata edits.

The default target directory is read-only, so the first focused command failed
before reviewed code compiled. The reported test results use a fresh temporary
target directory.

## Design Assessment

The retention design now fails closed. Turning every uncertain inspection into
a failed write-back is conservative, and making the sweep require an existing,
lockable lock file means both a renamed marker and a deliberately lockless root
remain recoverable. Rechecking marker and lock paths after taking the lock
closes the release/sweep race. The `OverlayIo` seam is narrowly scoped to the
filesystem decisions that determine survival and gives the failure matrix
portable, deterministic coverage without changing production dispatch.

No ergonomic or performance change is warranted in that path. Write-back runs
once per released overlay over a deliberately small list of top-level stable
files; retaining uncertain state is more important than avoiding its metadata
and read operations.
