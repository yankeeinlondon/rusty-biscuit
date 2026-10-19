---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-18
review_iterations: 0
completed: false
human_review: false
implemented: false
related:
    - 2026-10-01-remove-busy-folder
---

# `wt remove` gives up when the change watcher is slow or overrun

## Outcome

`wt remove` tolerates delayed file-change delivery while that delivery keeps
making progress, reduces the deletion bursts that can overrun macOS's change
service, and explains why it stopped when observation fails. Resuming a
removal does not ask for consent solely because earlier attempts of that
same operation deleted tracked files.

Success means the existing checks still protect new work, delayed delivery
within the chosen limits can complete, and the macOS regression sample
completes without lost notifications. This does not promise that any deletion
rate can prevent a system-wide event overflow.

## What happened

On 2026-10-18 `wt remove feat/dmls-indexing` (about 240,000 entries, macOS)
failed repeatedly and still did not finish after five runs.

| Run | Result | Cause |
| --- | --- | --- |
| 1 | Stopped before the move. Nothing changed. | `CoverageLost::DrainTimedOut`: a check was not confirmed within 5 s |
| 2 | Moved to the hidden name, deleted 2,665 entries, stopped | `DrainTimedOut` |
| 3 | Resumed, deleted 2,242 more, stopped | `DrainTimedOut` |
| 4 | Refused: "473 uncommitted files" | all 473 were tracked files that runs 2–3 had deleted |
| 5 | With `--force-worktree`: deleted 25,845 entries in 3.9 s, stopped | `CoverageLost::Overflow`: "the platform dropped events (its queue overflowed)" |

Runs 1–3 were not caused by `wt`. fseventsd had been running for 26 days and
its `notify` thread was pinned at one full core (`sample` showed it in the
path-matching loop, `strncmp` under a mutex). That was with only about 5
events per second system-wide. An event took more than 10 s to arrive. Quitting
nine watching programs changed nothing; `sudo killall fseventsd` brought it to
0.1 % CPU and same-second delivery. Overall CPU stayed low throughout (one
thread on an efficiency core), so nothing pointed the user at fseventsd. It
took a long manual investigation to find.

Run 5 was caused by `wt`. fseventsd was healthy (0.6 % CPU afterward), and
`wt` deleted about 6,600 entries a second, faster than FSEvents could take
the events, so it dropped some.

### Second occurrence: a modest checkout, healthy fseventsd

Later the same day, `wt remove agent-a9a8d534778b78274` (an agent worktree
with 61,778 entries, 34,545 of them under `target/`) stopped on its first
attempt with `CoverageLost::Overflow`. The timeline in `report.json` shows the
move, then 7,035 entries deleted in about 1.0 s (about 7,000 a second), then
the loss. That is a little over three `DEFAULT_DRAIN_EVERY` intervals. Two
hours later fseventsd was at 0.5 % CPU, eight hours after its last restart.
This was run 5's failure without run 1–3's degraded fseventsd. So it is not
limited to huge trees: any worktree that has been built is large enough to
hit it. Retrying hits the same burst. The removal had to be finished by hand
(`rm -rf` of the hidden directory, `git worktree prune`, `git branch -d`).

The report could not say which notification-loss flags fired. The change to
loss reporting below closes that gap.

## Existing contracts and intended changes

The `worktree` library's [observation contract](../../lib/src/remove/observation/mod.rs)
already distinguishes the platforms: Linux and Windows drains confirm earlier
events, whereas macOS returns events delivered so far after a quiet marker
round. Events can still arrive in a later macOS drain. Direct checks of each
entry before deletion, and the final index/HEAD checks, remain essential.

This fix intentionally changes waiting limits, the macOS deletion interval,
loss diagnostics, and resume consent for confirmed deletions. It preserves:

- Sticky coverage loss: once observation fails, no later successful marker
  makes that session usable again. No force flag bypasses this.
- The existing deletion checks, bounded event queue, per-depth drains,
  journal durability, and deletion of `.git` last.
- Fresh consent for surviving work on every retry, and recovery decisions
  based on filesystem and Git state rather than a saved phase.
- Existing exit classes: observer startup failure is exit 4; loss found
  before the move follows the existing refusal path; a stop after the move
  is exit 1, even when no entry has been deleted. A retry that was already
  moved keeps the existing resume refusal rules before further deletion.

> **Review note:** The earlier draft treated a macOS drain as confirmation
> that all earlier events had arrived. The adapter's documented experiments
> disprove that assumption. Smaller bursts reduce pressure; they do not
> establish complete delivery or justify continuing after a loss. The
> existing direct checks and stop-on-loss policy remain the safety boundary.

## Decisions

### Wait for progress, with a bounded total wait

In the `worktree` library, replace the five-second `DRAIN_TIMEOUT` in the
[macOS adapter](../../lib/src/remove/observation/fsevents.rs) and
[Windows adapter](../../lib/src/remove/observation/read_directory_changes.rs)
with a **30-second stall limit** and a **five-minute ceiling per drain**.
The observed delays already justify exceeding five seconds; another timing
spike is unnecessary. These are internal defaults, with no new CLI flags.

- Start both clocks when the drain begins. Use monotonic time. The ceiling
  spans every marker round and never restarts during that drain.
- Reset the stall clock on a newly acknowledged marker or an in-scope event.
  Ignore unrelated sibling activity, marker cleanup, and duplicate or old
  marker acknowledgments. Returning repeatedly from a flush is not progress.
- On macOS, keep the existing quiet-round completion rule. Receiving late
  events from this removal is progress, but does not itself finish the drain.
- On Windows, track progress separately for each handle whose marker is
  still awaited. Traffic on the parent handle must not hide a stalled admin
  handle, or vice versa. A completed handle no longer needs progress.
- Apply the same waiting policy to startup and admin-release checks. On
  Linux, keep the existing read-until-empty behavior; it needs no marker wait.
- Actual loss, a full application queue, a lost watch, or a read error ends
  waiting immediately. Preserve the first loss and the events received
  before it. Stop further deletion while waiting.

Replace `CoverageLost::DrainTimedOut` with `DeliveryStalled { waited }` and
`DrainCeiling { waited }`; `waited` is actual elapsed time without relevant
progress for the former, and total drain time for the latter. If both expire
at the same check, report the ceiling. Include the affected Windows watch in
the diagnostic. A stall means confirmation could not be obtained, not proof
that events were dropped.

The wait must wake for progress, loss, and deadlines; a blocking synchronous
flush must not prevent checking the limits. Integrate the existing interrupt
signal so Ctrl-C stops a long wait without waiting for either deadline, and
retain the existing before/after-move interruption behavior. Do not busy-spin
or hold the callback's queue lock while waiting.

### Reduce macOS deletion bursts without claiming loss is impossible

The `worktree` library's [DeletionOptions](../../lib/src/remove/observation/deletion.rs)
controls how many processed entries trigger another drain. Set its macOS
default to **512**, retaining **2,048** on Linux (including WSL2) and Windows.
Keep drains between depth levels and the existing maximum journal batch of
256 entries. Evaluate the interval after each completed batch as today, so
the configured interval can be exceeded by at most one batch minus one entry.
Do not change journal batch size or durability to tune observation.

The number of deleted entries is not a count of queued platform events:
FSEvents combines flags, delivers late, and shares resources with other
programs. Remove the claimed equivalence between 2,048 deletions and a
particular kernel buffer size. Do not add queue-depth adaptation without a
reliable platform signal and a separate design decision.

Run one quick macOS sample using a generated checkout with at least 100,000
entries on one host. Record completion, any loss flags, and elapsed time;
use the existing incident rates as context. Do not require a parallel build,
a multi-host matrix, repeated trials, or a statistical threshold. If 512
still loses coverage in that sample, the mitigation is unproven: record the
result and revisit the interval or pacing design before claiming success.
A larger investigation requires the author's decision.

### Preserve precise loss reasons and useful delay diagnostics

In the `worktree` library's [FSEvents callback](../../lib/src/remove/observation/fsevents.rs),
retain the raw loss-flag mask and all named flags present in the callback:
`KernelDropped`, `UserDropped`, and `MustScanSubDirs`. Flags can occur together,
so do not discard all but the first or invent a precedence among them. Use a
macOS-specific loss payload; retain generic `Overflow` for Linux and Windows.
`MustScanSubDirs` alone means complete event coverage is unavailable and still
stops this removal. Keep the application queue's `QueueFull` distinct.

Record bounded, per-attempt observation diagnostics:

- Backend; last and longest successful marker round-trip in milliseconds,
  with no value when no marker has been confirmed. These measure marker
  acknowledgment, not the age of every watched-file event.
- Loss reason, raw macOS flags and their names where applicable, elapsed
  stall or drain time where applicable, and the affected watch.
- Where the loss was received: startup, pre-move check, post-move check,
  deletion, or unregistration. During deletion, include entries processed
  since the previous drain. This is timing context, not proof that `wt`
  caused a system-wide drop.

The `worktree` library's [OperationReport](../../lib/src/remove/outcome.rs)
is the persisted report contract. Add a structured optional observation field
to it and its earlier-attempt representation, and increment `REPORT_VERSION`
from its current value of 4. Read existing reports with absent diagnostics as
unknown, preserve earlier attempts when resuming, and never require new fields
in old recovery records or journals. Do not accumulate every marker sample.
Carry diagnostics through startup failures too; before a recovery record
exists, the CLI can show them without promising a saved report path.

The `worktree-cli` package renders these details through its existing
[recovery messages](../../cli/src/commands/remove/recovery.rs) and
`biscuit-terminal` components. If startup succeeds but its total confirmation
time exceeds one second, warn before moving anything. Warn once per attempt;
do not refuse solely for slow startup. Say that removal may take longer and
can stop if confirmation stalls.

For macOS stalls or ceiling expiry, say that the system file-change service,
fseventsd, may be delayed. The troubleshooting documentation may explain how
to inspect its CPU use and that an administrator can restart it with
`sudo killall fseventsd`. High CPU alone does not establish this diagnosis;
a restart affects other programs and is a manual troubleshooting step, never
an automatic action. On Windows, name the affected watched path/volume
without inventing an equivalent service-restart remedy. Keep the existing
locations, retry instruction, and full-report link in stop messages.

### Resume excludes only confirmed deletions from consent

The `worktree` library's [deletion journal](../../lib/src/remove/recovery/journal.rs)
distinguishes confirmed `Deleted` entries from `ExternallyAbsent`, `Uncertain`,
`Kept`, and `Planned`. Only durable `Deleted` results belonging to the same
validated operation may explain a missing tracked file. Read all attempts'
journals; a path's presence in the approved inventory or a batch intent is
not proof that this operation deleted it. Reports are not deletion authority.

Suppress a dirty entry from the resume consent list only when all of these
hold:

- Its Git porcelain status is exactly ` D`: an unstaged deletion, with no
  staged change, conflict, rename, or copy.
- Its exact relative path has a confirmed deletion result in this operation.
- Inspecting it without following links confirms absence. An inspection
  error is not absence; a recreated file, symlink, or directory is not absent.

Everything else continues to require the ordinary consent, including staged
deletions, staged additions whose working file is absent, modifications,
untracked files, protected included files, and externally missing or
uncertain entries. A missing directory is not blanket permission to suppress
its descendants; each status path needs its own confirmed result. Match paths
losslessly using the repository's existing native-path rules, never display
strings, lowercase conversion, or prefix matching.

Use one library-owned resume consent calculation in both the CLI's initial
report/policy decision and the lifecycle's recheck. The `worktree` library's
[consent_state](../../lib/src/remove/lifecycle.rs) currently collects the
inventory and binds consent to a fingerprint; ordinary first-attempt calls
retain their current behavior. A resume-specific calculation must validate
the operation and explain deletions before reporting and asking for consent,
then repeat that calculation under the operation lock before further deletion.
Do not implement this solely as hiding rows in the CLI.

Keep the raw inventory and its full fingerprint as the change-detection input;
use the explained inventory for consent decisions and lists. Bind the approval
to both views consistently so the lifecycle does not compare a filtered CLI
inventory with an unfiltered recheck. Include the raw index entries in the
fingerprint as today. A file recreated after the report, or a new staged
change while consent is pending, must still invalidate approval. Capture a
fresh deletion inventory of surviving entries and repeat all usage and branch
checks; no earlier approval authorizes new content.

Explain in the retry summary how many confirmed earlier deletions were
excluded from the consent list. Keep them in journal counts and reports;
never convert uncertain entries into confirmed deletions. An unreadable or
invalid journal follows the existing recovery refusal path and cannot enable
this exemption. No journal format change is needed.

## Non-goals

- Fixing or automatically restarting fseventsd.
- Removing watching, increasing queue capacity as a substitute for flow
  control, or letting a force flag bypass observation failures.
- Continuing after a drop by rescanning. That changes the safety contract and
  remains an author decision below.
- Speeding up deletion or adding a performance gate or new CI environment.

## Verification

Extend existing tests where they already cover the boundary. Use the
repository's Test Toolkit and nextest through the package-area `just test`
recipe. Filesystem and injected-observer tests are L1; a native watcher alone
does not make a test L2. No test should open or focus a terminal or browser.

- **Wait policy (L1, portable):** extract the deadline/progress decision into
  a small helper with a controllable monotonic clock. Prove delayed progress
  can exceed the old total budget, silence stalls, repeated progress cannot
  extend the ceiling, and unrelated/old marker activity cannot reset a
  deadline. Cover a busy Windows handle beside a stalled one. No real
  30-second waits or fixed sleeps.
- **Adapter wiring (L1):** the existing `Injector::delayed` and
  `deliver_delayed` feed only an event queue; they do not run native marker
  waits. Extend the test seam at the actual wait/progress boundary, and prove
  startup, drain, and admin-release paths use the policy. Verify interruption
  and sticky loss end waiting and prevent further deletion.
- **Loss flags (L1, macOS):** feed each flag and a combined mask through the
  existing callback seam. Verify all flags survive into structured reports,
  including `MustScanSubDirs` alone; preserve generic overflow and queue-full
  behavior elsewhere.
- **Deletion cadence (L1):** extend the existing drain-count test to prove
  the macOS default, bounded batch overshoot, and required depth-level drains.
  Keep the 100,000-entry native sample above as a manual regression check,
  not a slow new L2 or mandatory CI fixture. Existing native macOS tests stay
  in nextest's `fsevents-l1` group, which serializes across test processes.
- **Resume (L1, library):** stop after confirmed deletion, retry, and verify
  no extra consent is required solely for the resulting ` D` entries. Cover
  deletion records from multiple attempts, uncertain and externally absent
  entries, staged changes, a recreated path, and unreadable journals. Change
  a path or the index between reporting and recheck and verify refusal.
- **CLI/persistence (L1):** one noninteractive retry through the real CLI
  without `--force-worktree` proves policy wiring; a counterpart containing
  new work must refuse. Assert warning/stop messages and serialized loss
  details at their existing rendering/report boundaries, and that an old
  report remains readable with earlier-attempt diagnostics preserved.

Implementation verification is scoped to `worktree` and `worktree-cli` on
macOS, Linux, native Windows, and WSL2 using the existing recipes and evidence
rules. This spec review runs no implementation tests and claims no new
behavioral or performance evidence.

## Documentation

Update alongside implementation:

- `worktree/docs/cli/remove.md`: waiting limits, startup warning, loss reasons,
  manual troubleshooting, resume consent, and new report fields. Correct the
  existing claim that a macOS marker proves every earlier change arrived;
  describe quiet rounds, late events, and direct checks instead. This is
  documentation drift relative to the current adapter, not a new guarantee.
- `worktree/README.md`: only any overview behavior that changes; keep the
  command/output reference in `docs/cli/remove.md`.
- `.claude/skills/worktree/observation.md` and `lifecycle.md`: new wait policy,
  loss payload, diagnostics, deletion interval, and raw versus explained
  resume consent. Update the resume entry-point docs in the library too.
- `.claude/skills/os/macos.md`: the observed fseventsd failure mode and manual
  diagnostic/restart procedure, with its system-wide effects. Record the
  quick sample's result in the implementation log, not as a universal claim
  that a deletion rate cannot overflow.

## Open Questions

### Should a rescan ever permit continuation after lost notifications?

A full rescan can establish present state, but cannot recover an event
history. The current deletion inventory also hashes only consent-sensitive
content; other files are compared by identity and metadata. Therefore a
rescan is not automatically equivalent to the existing observation contract,
and per-entry checks do not eliminate the interval between checking and
unlinking a path.

Options:

1. **Keep stopping on loss (recommended for this fix).** Preserves the
   established safety rule and keeps this change bounded. Smaller batches
   and better waits address the observed failures without redefining what
   observation proves. The cost is that a real drop still needs a retry.
2. **Allow bounded rescans and a fresh observer.** Could avoid manual retries
   after bursts. It requires a separately specified comparison of surviving,
   recreated, and deleted paths, stronger content checks where needed, a safe
   observer handoff, and rules for races and repeated loss; it adds full-tree
   reads and intentionally gives up complete event history.
3. **Replace the affected observation mechanism with stronger tracking.**
   Could improve delivery guarantees without accepting gaps, but needs a
   separate feasibility design; approaches that open every file or directory
   can exhaust handles and add substantial startup work.

Unless the author chooses otherwise, implement the stop-on-loss policy in
this specification. Continuation by rescan is outside this fix and must not
be introduced as an overflow fallback during implementation.
