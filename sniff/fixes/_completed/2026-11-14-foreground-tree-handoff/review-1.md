---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): terminal capability probing stops background Claudine jobs"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-16T16:42:28-08:00
spec: 2026-11-14-foreground-tree-handoff/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-foreground-tree-handoff/spec.md`"
fix: 2026-11-14-foreground-tree-handoff/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-foreground-tree-handoff/log.md
implemented_by: claude/opus
next: 2026-11-14-foreground-tree-handoff/review-2.md
---

# Review 1: foreground tree handoff

The fix is **not production ready** because of one reproducible defect. The
shared handoff and both caller migrations meet the reviewed contract, but the
implementation deliberately bypassed a terminal-probing defect in its
background-job test. That defect needs repair under this review's rule that
realistic defects discovered in unchanged code also count. No author ruling
is needed, and no finding is blocked.

## Findings

### R1-F1 (high): terminal capability probing stops background Claudine jobs

**Authority: defect.** Running `claudine compose --goose --yolo doc.md &`
from an interactive shell stops Claudine before its lifecycle action or
provider runs. This also happens with a document containing no lifecycle
actions. In biscuit-terminal,
[RawModeGuard::new](/Volumes/coding/wt/rusty-biscuit/fix-sequences/biscuit-terminal/lib/src/discovery/raw_mode.rs:40)
changes the controlling terminal's settings without checking whether its
process owns the foreground. The kernel stops the background process with
`SIGTTOU`. This is the pre-existing defect deferred in the implementation log;
the review independently reproduced it.

**Defect class:** terminal capability discovery mutates a terminal from a
background process, turning optional discovery into suspension of the command.

The Unix query port in biscuit-terminal
([open_port](/Volumes/coding/wt/rusty-biscuit/fix-sequences/biscuit-terminal/lib/src/discovery/tty_query/unix.rs:25))
opens `/dev/tty` and calls this guard before sending or reading a query.
The shared `exchange` function propagates an ordinary port-opening error,
but it cannot propagate this stop because the settings syscall never returns
until the job resumes. Direct and tmux-wrapped startup probes, OSC queries,
cursor-position queries, and window-size queries all use that same exchange
and port. A source search found one production raw-mode construction site;
the `stdin()` convenience method delegates to `new()` and has no separate
production caller. The repair belongs at this shared boundary, rather than
in each report or query.

**Reproduction and class sweep.** In separate detached tmux panes, run an
interactive `sh` with a temporary HOME containing empty Claudine and
Darkmatter configuration files, and a fake `goose` executable that records
its invocation. Use this document:

```yaml
---
title: t
start:
  - shell: "echo lifecycle-ran > reached"
---
Body.
```

Change only the launch redirection between cases. Inspect the process from
outside the pane with `ps -o pid=,stat=,pgid=,tpgid= -p <pid>` and check the
two marker files. A stopped job has state `T` and a process group different
from the terminal's foreground group. All processes and panes were cleaned
up after observation.

| Child ID | Site and shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Shared probe through background Claudine; both output streams inherited | State `T`; neither marker written | Optional probes fall back; action and provider run |
| R1-F1.2 | Same command, stdout redirected to a file | State `T`; neither marker written | Same successful execution |
| R1-F1.3 | Same command, stderr redirected to a file | State `T`; neither marker written | Same successful execution |
| R1-F1.4 | Same command, both output streams redirected | Clean control: action and provider markers written | Action and provider run |
| R1-F1.5 | Same fixture, foreground launch | Clean control: action and provider markers written | Action and provider run |
| R1-F1.6 | Background launch with terminal output; remove the entire `start` field | A separate controlling-PTY parent observed `WSTOPSIG == SIGTTOU` through `waitpid(WUNTRACED)` | A document without terminal input completes |
| R1-F1.7 | Sniff foreground preparation for a background host | Clean: `a_background_host_is_inactive_and_mutates_nothing` passes; redirected-output Claudine L2 control also passes | No foreground transfer or settings mutation |

The new Claudine test
[a_background_claudine_leaves_the_foreground_alone](/Volumes/coding/wt/rusty-biscuit/fix-sequences/claudine/cli/tests/level2/level2_lifecycle_foreground_handoff.rs:442)
uses `> claudine.log 2>&1` specifically to bypass this defect. It proves the
inactive handoff path but does not protect the normal background launch.

**Smallest repair recommended:** before entering raw mode, reject a query
when the terminal's foreground group differs from the caller's group, or
when that ownership cannot be established. Return the existing unavailable
query error so callers use their existing fallback. Do not ignore or block
`SIGTTOU` to let background discovery manipulate another job's terminal.
Keep the redirected-output control and add a detached-terminal regression
for inherited output, including the single-stream redirection variants.
Verify that the action and provider complete, the shell retains foreground
ownership, and its settings remain unchanged. This is a bounded fix in a
shared helper with focused tests; it does not require another specification
or an author decision.

## Spec coverage and verification levels

| Requirement | Strongest evidence reviewed | Assessment |
| --- | --- | --- |
| Shared foreground handoff and restoration on success, failure, cancellation, and unwind | Sniff L1 scripted syscall failures and ordering; five Sniff tmux L2 tests; Claudine cancellation L2 tests; Darkmatter editor tests | Implemented; reviewed runs pass |
| Foreground lifecycle commands own their whole tree | Exact-byte and structured routes share one runner; L1 completion/cancellation/wait-error tests; L2 typed Ctrl-C and host-directed SIGINT assert root and background-child cleanup | Implemented; L2 passes |
| Darkmatter uses the same implementation and retains existing edit tests | Private handoff removed; shared guard used in owned terminal launches; existing edit L2 files unchanged against the pre-fix commit | Migration verified; both Darkmatter edit L2 tests pass |
| Native Windows policy retained; WSL2 uses Unix implementation | Windows guard is inactive; creation flags and Job attachment remain in the existing owner; Unix implementation selected on WSL2 | No policy change found; runtime evidence remains CI's responsibility |
| Canonical process-tree documentation describes handoff | Topic includes API order, `/dev/tty`, inactive guards, contention, cleanup order, interrupt delivery, restoration and limits | Implemented |
| Lifecycle command reads typed input, starts a background child, is cancelled, and restores the terminal without taking focus | Eight new detached-tmux L2 tests, including typed input, Ctrl-C, host SIGINT, structured commands, nested hosts and redirected stdin | Appropriate L2 evidence; tests pass |
| Existing editor terminal tests pass unchanged | Claudine's two edit-action and seven interactive-capture tests; Darkmatter's two edit-action tests | All pass unchanged |

These tests exercise terminal input bytes and the terminal driver's signal
delivery, rather than an application-specific keyboard encoder. L2 matches
the spec's explicit verification requirement. Focus-taking L3 execution is
neither required by this contract nor claimed as evidence.

No file-format or configuration reader changed, so the input robustness
matrix does not apply.

## Verification performed

- `cd sniff && just test process::foreground`: 12 passed; longest 0.030 s;
  no slow L1 test.
- `cd sniff && BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_foreground_handoff`:
  5 passed, backend proof `tmux run=5 skip=0`.
- Root `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests,test-fixtures level2_lifecycle_foreground_handoff level2_lifecycle_ctrl_c_tmux level2_edit_action level2_edit_interactive_capture`:
  23 passed, backend proof `tmux run=23 skip=0`. This uses the canonical
  package leg and avoids the unrelated zero-test generator leg.
- `cd darkmatter && BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_edit_action`:
  both darkmatter-cli tests passed (0.873 s and 0.908 s); the library and
  language-server legs selected no tests. The shared backend counter included
  the overlapping Claudine run, so its final count of 25 is not presented as
  25 Darkmatter tests.
- `just check-tier-coverage sniff darkmatter claudine`: zero stranded tests.
  New modules are declared by their consolidated targets, and the recipes
  and CI feature lists enable the required fixture and terminal features.
- Independent detached-tmux and controlling-PTY reproductions recorded in
  the finding above.

The implementation record also reports full macOS L1 runs and Linux runtime
runs. Its recorded 8.5 s live-network test was followed by isolated
measurements of 2.2–2.3 s and a full-suite measurement of 2.9 s; the recorded
peak load was 18 on 16 cores. The recorded isolated measurements clear the
under-10-second loaded-run case. No other recorded slow L1 test was found.
Cross-compilation is treated as compile evidence only; missing native
Windows and WSL2 runtime results do not block this review's readiness.

## Implementation decisions (informational)

Two independent agents read the specification and implementation log in
parallel, including all sixteen `decisions_for_review` entries. The short
spec supplies two author rulings and five requirements; it does not grant
a separate blanket autonomy exception. The supplied plan settles the
detailed API and caller policy. The following decisions need no human:

1. **Atomic claim instead of a mutex guard:** keep it. It preserves the
   one-active-handoff rule, releases during unwind, and permits sending the
   guard between threads. Tests cover contention, release and unwind.
2. **Terminal disappearance during the foreground query:** keep the same
   inactive outcome as failure to open a nonexistent terminal. This is a
   small, reversible extension of the inactive contract.
3. **Disarm after restoring the group and settings despite a mask-restoration
   error:** keep the terminal disarmed and return the error. Repeating an
   already completed terminal transfer would not repair the thread mask.
4. **Earlier documentation updates:** keep them; public behavior should be
   documented when introduced. Phase placement has no remaining consequence.
5. **Refuse simultaneous foreground lifecycle commands:** keep the plan's
   explicit `WouldBlock` policy, including noninteractive commands that
   happen to request the terminal. This is an already-decided contract.
6. **Private test switches for foreground status and wait failure:** keep
   them. They exercise otherwise inaccessible error paths without ambient
   terminal dependence, with separate L2 wiring checks.
7. **Move the interrupted substitution assertion to `finalize`:** keep it
   together with the new interrupted-`start` assertion. Cleanup remains
   runnable while normal work stops, as the plan requires.
8. **Use libc for the Unix editor PTY helper:** keep it. It avoids expanding
   a platform-specific development dependency and has no caller impact.
9. **Redirect background-test output around the raw-mode defect:** retain
   the case as a control, but repair the deferred defect and cover the
   inherited-output launch under R1-F1. Deferral needs no author ruling.
10. **Test-only hold behavior:** keep the early exit when its workspace
    disappears and the refusal to hold after a readiness-write failure.
    Keeping the hold in the `failure` event preserves the escalation test's
    existing meaning.
11. **Enable `test-fixtures` in local L3 builds:** keep it; the shared fixture
    requires the hold, and CI already enables the feature. Compile-only L3
    evidence is accurately labeled; no focus-taking run is needed here.
12. **Correct the stale libc dependency description:** keep it. No dependency
    changed; the sentence now reflects the migration to Sniff.
13. **Concise caller-skill summaries rather than whole-page transclusion:**
    keep them. They agree with the canonical pages and are cheap to update.
14. **Document platform facts without a dedicated test for every sentence:**
    keep the Windows console description and the new-session limitation.
    The Windows flags remain unchanged; a new session cannot hand off its
    former controlling terminal. Naming `start` and `loop` in shell-action
    guidance is appropriate because `initialize` cannot run shell actions,
    even though the executor also stops normal initialize work.
15. **Fix existing Windows test warnings:** keep the bounded test-only
    import and binding adjustments. They preserve runtime behavior and
    remove warnings in targets already being checked.
16. **Native Windows and WSL2 runtime evidence deferred:** accept the stated
    evidence limitation. Windows storage preflight refused execution; WSL2
    shares that machine and was not run. Neither cross-compilation nor Linux
    results are presented as their runtime proof.
17. **Standing-clone cross-check workflow instead of a tar snapshot:** keep
    the recorded workflow. This affects validation mechanics, not behavior.
18. **One-job Linux build and targeted Darkmatter validation after linker
    memory failures:** accept the narrower evidence as labeled. The plain
    nextest run is not claimed as a complete CI receipt.
19. **Remeasure the live-network timing outlier:** accept the recorded
    loaded-run and isolated timings described above; do not invent another
    implementation requirement from the choice of validation command.
20. **Leave unrelated concurrent author edits untouched:** keep that scope
    discipline. Completed gates describe their recorded tree, and the log
    explicitly limits later cross-check evidence.
21. **Public API details and interrupt policy:** keep the plan's adjacent
    guard, owned `/dev/tty` descriptor, single-spawn contract, child-side
    transfer, thread-local masking, explicit restoration, primary-error
    precedence, and foreground SIGINT/130 classification. Both callers use
    them consistently. Valid `start` examples correctly replace the spec's
    illustrative but unsupported `initialize: shell` placement without
    editing the snapshot's requirements.

No implementation-record statement was verified false, so no factual
correction was made. The deferred background-probe defect is confirmed and
reported as a finding rather than rewritten as a new implementation claim.
This is the first review of this fix; there are no carried findings or
partial repairs, and recurrence is false.
