---
title: Research orchestration feasibility spike
date: 2026-09-17
status: completed
---

# Research orchestration feasibility

The installed Claudine CLI can launch separate research workers with different
explicit inputs. Its `--timeout` flag is a per-worker limit, not a shared
platform-run deadline. A persisted platform-run coordinator is still needed to
meet the specification's combined execution-budget requirement. This spike does
not implement that production coordinator.

## Evidence and reproduction

Run from the repository root:

```text
python3 messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/run_spike.py
```

The fixture generates local synthetic documents and a fake `goose` executable,
then invokes the installed Claudine binary. It requires no network, credentials,
model service, message recipient, new dependency, or focused terminal window.
The synthetic website name is a string marker and is never fetched.

[run_spike.py](run_spike.py) contains the reproducible experiment and assertions.
[results.json](results.json) records commands, exit codes, measured durations,
input-presence checks, and ledger snapshots. Durations are observations, not
performance targets; subsequent runs replace that evidence file.

The tested executable was `/Users/ken/.cargo/bin/claudine`, reporting
`claudine 0.1.0`, with a size of 118,538,944 bytes. Its build revision was not
verified; results are evidence about that installed executable, not proof that
the current checkout has an identical build. Sniff identified the host as macOS
27.0. Linux, native Windows, and WSL2 were not exercised in this spike. The fake
executable launcher is a disposable Unix fixture, not production portable code.

Child processes receive a fixture-owned home, a restricted PATH containing only
the fake provider and system utilities, disabled rendezvous reporting, and
`PLAYA_DRY_RUN=1` with a private audio spool. Parent environment variables and
the user's configuration are not changed. No audio spool was created. Temporary
documents and synthetic output are removed when the fixture exits. No raw agent
transcripts are retained.

## Results

| Experiment | Observed result | What it establishes |
|---|---|---|
| Native four-step sequence | Exit 0; four distinct worker processes | Existing sequence execution can separate discovery, reconciliation, source maintenance, and evidence review into workers |
| Explicit discovery inputs | Discovery marker present; previous-prose and curated-link markers absent | Prepared discovery prompt can omit prior research |
| Explicit reconciliation inputs | Reconciliation, previous-prose, and curated-link markers present | A later worker can receive the richer research input |
| Native sequence with `--timeout 1s` | Two workers sleeping 0.8 seconds each complete; total elapsed exceeds 1 second | The flag does not establish a shared platform deadline |
| Native worker with `--timeout 1s` | Worker scheduled to sleep 4 seconds exits through Claudine with code 143 after about 1.2 seconds; worker PID no longer exists | The installed wrapper stops this single local child at its deadline; code 143 alone is not a complete explanation of why it stopped |
| Proposed five-invocation ledger | Worker exits are 0, 7, 0, 0, 0 | The local prototype charges discovery, failed reconciliation, its retry, maintenance, and review to one allowance |
| Prototype restart at invocation exhaustion | Separate coordinator process reloads limit 5 / used 5; no worker launches | Persisted admission checks can prevent a restart from resetting the invocation allowance |
| Prototype restart with consumed time exhausted | Separate process reloads recorded consumed time and matching limit; no worker launches | Persisted admission checks can retain consumed-time accounting |
| Accepted sentinel | Byte-identical after all experiments | This isolated fixture never changes its accepted baseline |

The prototype's time-exhaustion case explicitly sets the remaining allowance to
zero using time already measured during earlier calls. It verifies restart
admission, not a full workflow naturally exhausting a live shared deadline.
Its elapsed-time accounting charges measured execution only; this does not
decide whether future offline pauses count against the research budget.

The `resume_probe` function in the fixture is newly written experimental code.
Its ledger is **not** evidence of a native Claudine persistent run budget. It
does not implement in-flight shared-deadline enforcement, crash-safe accounting,
concurrent access, descendants, publication, approvals, or production recovery.

## Interpretation and implementation boundary

Claudine's documentation describes the observed distinction: its timeout starts
at child spawn, and document-level retry/resume budgets have a different lifetime
from proxy and subsequent-loop budgets. See
[timeout semantics](../../../../../.claude/skills/claudine/timeouts.md) and
[composition state ownership](../../../../../claudine/docs/topics/composition.md#retry-and-resume-re-entry).
The checked-out graph resolves the sequence entry to `run_sequence_inner` in
`claudine-cli`; this was read-only context, not a source modification.

Recommend one explicit platform-run record outside individual worker sessions.
Keep its identity, remaining invocation allowance, consumed elapsed allowance,
current stage, and incomplete result across retries and resumption. Reuse
Claudine's worker launch and cancellation machinery rather than creating another
provider launcher. All worker dispatches, including review and recovery, must
pass the same admission boundary. The production design must pass the remaining
time to each launch and account for orchestration work as required by its chosen
clock semantics; the experiment does not establish that merely reusing the
original timeout value is sufficient.

Two implementation placements were considered before the human ruling below:

- **Research-specific coordinator around Claudine commands:** smaller scope and
  keeps Messenger's approval/research records near its maintenance tooling, but
  must avoid bypassing accounting through nested retries inside a command.
- **Claudine orchestration extension:** owns every launch and recovery transition
  in one place, but changes a shared package and requires broader impact review.

The post-spike human ruling selected a narrow Claudine orchestration extension
for shared per-platform budgeting. Messenger retains research records,
validation, and approval policy. Reuse Claudine's launch machinery without a
second Messenger-specific coordinator or a generic research framework. The
specific production API and persistence mechanism remain implementation work;
this spike did not modify Claudine.

The human also selected an active-workflow clock: charge research, fetching,
validation, independent review, orchestration, and automatic waits/backoff.
Pause only for an explicit stopped or suspended state awaiting human approval
or operator resumption. Consumed allowance persists across retries and restarts;
crashes must not silently refund it, and extra budget requires a separate
operator decision. These are approved requirements, not behaviors proved by
the limited prototype.

## Limits and remaining implementation checks

- Separate `compose` prompt files and sequence tasks were exercised. Refreshing
  an existing document through `inline-compose` was not; the implementation must
  route discovery through a separately prepared input rather than assuming the
  existing document's prompt automatically hides its body.
- Input-marker absence is not filesystem sandbox isolation. A real agent's
  available tools, repository instructions, and resume context need appropriate
  launch preparation. The fixture does not prove a malicious worker cannot read
  sibling files.
- A local PID disappearing does not establish cancellation of remote model work,
  provider subprocess trees, or billable requests. Report those limitations.
- The unchanged accepted sentinel is not a test of multi-artifact publication.
  Publication consistency remains implementation work under the existing spec.
- Source evaluation, schema completeness, API facts, and research quality were
  intentionally outside this experiment.

Implementation planning must explicitly account for the shared budget and the
subsequently approved active-workflow clock instead of assuming a per-worker
timeout supplies them. In-flight enforcement, crash-safe accounting, and the
other limitations above remain to be implemented and verified.
