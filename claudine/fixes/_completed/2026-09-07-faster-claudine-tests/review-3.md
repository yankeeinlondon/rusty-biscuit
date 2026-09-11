---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-09T13:26:03-07:00
spec: 2026-09-07-faster-claudine-tests/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-07-faster-claudine-tests/log.md
description: A **fix** review of `2026-09-07-faster-claudine-tests/spec.md`
fix: 2026-09-07-faster-claudine-tests/review-3.md
previous: 2026-09-07-faster-claudine-tests/review-2.md
---

# Review 3 — Faster Claudine Tests

## Verdict

The fix is **not ready for production**. Review 2's inventory/tooling failure is
partially resolved: the current 7,417-identity inventory reconciles successfully,
and the shared `tools/test-audit` package passes all 208 tests. The remaining
closure conditions were not implemented. The canonical Level-1 suite still fails,
CI performance evidence and budgets still do not exist, tmux and Terminal.app
still inherit host shell initialization, and the Level-1 PTY skip/readiness gaps
remain unchanged.

The new deferral document does not change that verdict. It explains why evidence
was not collected, but the specification makes compatible CI evidence, ratified
budgets, passing local gates, and resolved reachability/readiness findings
acceptance criteria. Recording those items as deferred cannot satisfy the same
criteria.

## Findings

### 1. High: the canonical Level-1 suite still fails, contradicting the local-complete claim

The specification requires relevant local gates to pass and requires the final
candidate evidence to be current (`spec.md:201-208,225-230`). A fresh
`claudine/just test` run against this working tree exits 100 after 4,641 of 6,898
tests: 4,640 passed, one failed, nine skipped, and 2,257 were not run because the
default nextest profile stopped after the failure.

The failing identity is unchanged from review 2:
`propagated_context_fixtures::isolated_fixture_can_opt_in_to_provider_memory_discovery`.
It reaches preflight and then exits 1 because the refreshed document changes the
launch plan without the recorded inputs needed to rebuild it. This directly
contradicts `results.md:25`, which says local verification is complete, and means
the required full-suite performance remeasurement remains impossible.

The reconciliation portion of review 2's finding is resolved:
`inventory-reconciler.ts` exits 0 over 7,417 runner identities, and
`tools/test-audit/just check` passes 208/208 tests. Those green metadata gates do
not compensate for a red application suite or the unexecuted 2,257 tests.

**Required change:** fix the propagated-context fixture or production regression,
then run the complete canonical L1 suite without failures. Refresh `results.md`
and the measurement artifacts from that final candidate rather than retaining the
historical green run or describing the current red state as locally complete.

### 2. High: required CI runs and budgets are absent, and the aggregator still cannot prove provenance

The specification requires three consecutive candidate CI runs on Linux, macOS,
Windows, and WSL2, intervening failures, matched per-environment comparisons, and
numeric per-family budgets backed by those runs (`spec.md:181-199,225-228`). The
implementation still records baseline **1 of 3**, candidate **0 of 3**, and no
budgets (`results.md:39-78,146-166`; `attribution/budgets-pending.json:16-27`).

The JUnit-to-family join is useful, but review 2's provenance defect remains.
`aggregate()` derives a run identity only from `basename(runDir)` and accepts a
caller-supplied `provenanceKind` (`tools/test-audit/src/attribute/aggregate.ts:72-79,200-205`).
The manifest schema still contains only tier, package, XML path, exit code,
environment, duration, and report presence (`tools/test-audit/src/junit/manifest.ts:11-19`).
There is no immutable source revision, workflow run number/attempt, ref, event, or
sequence to establish that samples are from the same source and are ordered,
consecutive CI runs. Consequently three arbitrary staging directories can still
be stamped `ci` and satisfy `runsPerLeg`.

Manifest cardinality is also incomplete. `readLeg()` uses `records.find(...)` for
each expected cell (`aggregate.ts:135-143`) and does not reject a second record for
the same tier/package or an unexpected extra cell. A duplicate record can therefore
be silently ignored instead of invalidating the evidence set.

**Required change:** collect and retain the required baseline and candidate runs
on all four legs. Stamp immutable CI/source provenance into staged artifacts,
validate ordered consecutive runs from one source state, and require a one-to-one
manifest-cell match with duplicate and unexpected cells rejected. Only then derive,
ratify, and enforce the per-leg/per-family budgets.

### 3. High: the CI tmux route and Terminal.app remain dependent on host shell startup files

The latest harness change suppresses interactive rc files for WezTerm and Kitty,
but it explicitly leaves tmux and Terminal.app on a single login shell
(`biscuit-test-harness/README.md:108-130`). Those backends can therefore still
source an interactive rc file through the host's login profile and inject Atuin,
starship, or another prompt hook before the test command. This is the same failure
class that previously swallowed the WezTerm test input.

This is high severity because tmux is the portable Level-2 backend used by CI. A
green run on one host does not establish deterministic real-terminal behavior on
macOS, Linux, Windows, or WSL2 when startup depends on user dotfiles. The new
`deferred-performance-measurement.md` addresses CI collection and local timing;
it does not provide a linked owner and acceptance criteria for this shell-isolation
finding as required by `spec.md:220-222`.

**Required change:** give tmux and Terminal.app the same tested login-profile plus
rc-suppressed interactive-shell policy, or create a specific linked owner document
with evidence, rationale, and closure criteria. Re-run the canonical L2 route on
the available backends afterward. Do not treat skipped GUI backends as passing
evidence.

### 4. Medium: Level-1 PTY tests still false-green when PTY setup is unavailable and retain a fixed readiness sleep

The migrated ordinary L1 PTY tests still call
`require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)")`; for example,
`level1_schema_prompt_pty.rs:123-126`. A missing or broken PTY therefore becomes a
skip in the mandatory L1 suite. Level 1 has no optional harness contract: explicit
compile-time platform exclusion is valid, but missing infrastructure on a selected
platform must fail rather than look green.

`level1_provided_partial_file_pty.rs:129-149` also retains the fixed 300 ms wait
before sending `y`. The comment confirms that this is a guessed settle interval,
not observation of a ready read loop. It adds 900 ms across the three confirmation
tests on every run and can still race under load, contrary to `spec.md:162-166`.
The inventory says the interactive family is remediated while assigning shared
helper sleeps to another row (`inventory.md:1197-1198`), so it does not disclose
this separate 300 ms site or its unresolved disposition.

**Required change:** make PTY construction failure fatal for selected Unix L1
tests, preserve only explicit platform exclusions, expose or observe a deterministic
ready condition before sending input, update the inventory, and repeat the changed
cases under representative suite load.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Fixture-owned CWD, home, cache, environment, PATH, and shared command policy | Level 1 process probes plus structural guards | Appropriate boundary. The inventory now reconciles, but the full L1 gate remains red. |
| Interactive schema, provided-partial, and dry-run behavior | Level 1 `expectrl` PTY tests with manufactured bytes | Appropriate for byte-level interaction and textual ordering. It does not verify terminal-emulator encoding or final rendering; the optional-L1 skip and readiness race remain. |
| Dry-run approval rendering and lifecycle output in a real terminal | Level 2 tmux/WezTerm capture with L1 interaction complements | Appropriate level exists, but tmux startup remains host-dependent and skipped backends are not evidence. |
| Wrapper summary textual ordering | Level 1 PTY transcript assertions | Appropriate after the rendering claim was narrowed. No SGR, width, or layout claim is treated as verified. |
| OSC 8 auto-detection in WezTerm | Level 2 real-WezTerm capture | Correct level for the behavior. The WezTerm-specific Atuin regression is repaired; backend-independent shell isolation is not. |
| Physical Ctrl+C and chooser keys through a terminal encoder | Level 3 tests exist; no Level-3 execution in this review | Correct level exists. This fix does not change keyboard encoding, so unavailable L3 remains pending and is not reported as passing. |
| Cross-platform performance budgets | CI JUnit evidence, outside L1/L2/L3 | Missing: one baseline run, no candidate runs, no ratified budgets, and insufficient provenance validation. |

## Verification Performed

- `npx tsx inventory-reconciler.ts`: **exit 0**, 7,417 runner identities and 55 declared cfg/feature exclusions.
- `tools/test-audit/just check`: **208 passed**, typecheck and tests exit 0.
- `claudine/just test`: **exit 100**; 4,640 passed, one failed, nine skipped, and 2,257 not run.
- Source review confirmed the unchanged 300 ms readiness sleep and the L1 `require_level!` skip gates.
- Source and documentation review confirmed that tmux and Terminal.app still use host-dependent login-shell startup.
- Stored evidence review confirmed baseline 1/3, candidate 0/3, and no derived budgets.

Level 2 and Level 3 were not run in this review. Level 3 would inject OS input and
can steal focus, which is prohibited in this non-interactive session. The current
L2 source/evidence defects are independently sufficient to reject production
readiness; prior stored runs are not restated as fresh verification.

## Closure Criteria

1. Fix the propagated-context failure and obtain a complete green canonical L1 run; refresh the local verification and measurement claims from the final candidate.
2. Remove false-green L1 PTY skips and the provided-partial 300 ms readiness sleep, update the inventory, and stress the changed cases under suite load.
3. Isolate tmux and Terminal.app shell startup or record a specification-compliant linked deferral, then obtain current canonical L2 evidence on available backends.
4. Add immutable CI/source provenance and exact manifest cardinality checks to the aggregator.
5. Collect three consecutive baseline and candidate CI runs for every required leg, record intervening failures, and ratify/enforce numeric per-family budgets.
6. Update `results.md`, `inventory.md`, residual-owner records, and stored evidence so every closure claim describes the same final revision.
