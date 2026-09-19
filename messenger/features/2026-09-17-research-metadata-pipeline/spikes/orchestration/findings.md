---
title: Research orchestration feasibility spike
date: 2026-09-17
updated: 2026-09-17
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

## Phase 1 production-code probes (2026-09-17)

This section extends the spike with probes of the checked-out Claudine code.
It supersedes the earlier "inline-compose was not exercised" limit. Earlier
sections are unchanged.

### Evidence sources

- **Scripts.** [probe_budget.py](probe_budget.py) is an offline Unix fixture with
  a fake `goose` provider. It runs a Claudine binary unchanged and also runs a
  Python prototype of the proposed ledger. [probe_tree_windows.py](probe_tree_windows.py)
  replays Claudine's Windows Job Object calls through ctypes. Results are in
  [probe-results.json](probe-results.json). Fixtures live in the system temp
  directory and are removed on exit. Surviving probe processes were checked
  and reaped on every host.
- **macOS binary.** The macOS binary was built from committed HEAD `d57faf7e8`,
  extracted with `git archive` into a temp directory. The working tree was
  excluded because it carries another effort's in-flight Claudine edits, and
  they do not compile.
- **Linux binary.** The Linux binary is the `build-linux` standing-clone build
  from 2026-09-13 (`3efdbe450` plus a cross-check patch). Its termination and
  spawn code matches HEAD apart from one debug assertion. It predates the
  provider-overlay commit `9ef76132f`, which explains the `--repo` difference
  below.
- **No real providers.** No real provider, model service, or credential was
  used.

### Probe results

The macOS and Linux results agree on every row unless the row says otherwise.
Timings are observations.

| Probe | Method | Observed result | What it establishes |
|---|---|---|---|
| Discovery launched inside a repository | `compose` from `repo/work/` | Worker cwd was the **repository root**, not the launch directory. `CLAUDE.md` and `AGENTS.md` were visible, and a prior-prose file was reachable under cwd. The prompt had no prior marker. argv was `-t --system` | Separation happens only at the prompt level. The child cwd comes from `LaunchWorkspaceContext::from_repo_info` (`claudine/lib/src/composition/launch_workspace.rs`) |
| Discovery launched from a scratch directory | `compose` from a non-repository temp directory | cwd was the scratch directory. No instruction files or prior files were reachable under cwd | Launching from a prepared scratch directory keeps repository-relative context out of the worker's cwd. This needs no Claudine change |
| `--repo` overlay | `compose --goose --repo` | macOS HEAD refused with `provider.overlay_unsupported` (exit 1). The older Linux binary accepted it | The overlay is per-provider and is not a sandbox. `HOME` passes through unchanged (`docs/topics/repo-isolation.md`) |
| Child environment | Parent env set `FIXTURE_SECRET_TOKEN` and `OPENAI_API_KEY` | Both were removed. Claudine injected `AGENT`, `AGENT_CWD`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, `CLAUDINE_PID`, `INTERACTIVE`, `YOLO`, and `PWD`. `HOME` was inherited | Sanitization is by key name (`is_sensitive_key`, `cli/src/commands/wrap/env/sanitize.rs`). User-scope provider configuration stays visible |
| Inline-compose refresh | `inline-compose` on a research doc whose body holds prior prose | The prompt did not embed the body but did name the document's absolute path. The worker read the prior prose and edited the file in place. Exit 0: "agent updated" | Inline refresh **exposes prior prose by design** |
| Timeout with a process tree | `--timeout 1s`; the worker spawns a same-group grandchild and a `setsid` grandchild | Exit 143. The worker and the same-group grandchild were gone; the **`setsid` grandchild survived**. Elapsed 11.3 s / 11.2 s | On Unix, termination reaches the process group only. The escaped grandchild held the pipes, which added two 5 s reader-join timeouts |
| SIGTERM ignored | `CLAUDINE_KILL_GRACE=2s`, `--timeout 1s` | Exit 137 after 3.4 s / 3.2 s. The worker was gone | Escalation works, but the grace period is wall time the budget must charge |
| Wrapper crash | SIGKILL Claudine mid-worker | **The worker survived as an orphan** | On Unix, no mechanism kills the tree when the wrapper dies. An in-flight worker can keep running and billing |
| Lifecycle retry with delay | `failure` stack `retry` with `max_attempts: 2`, `delay: 1s` | **3 launches in one command**, 1.09 s apart. Exit 7 after 2.4 s / 2.3 s | Retries and backoff are invisible to a coordinator outside Claudine. Admission must be inside the launch funnel |
| Sequence restart | Run a two-step sequence (step 2 fails) twice | Each run relaunched step 1 and got a new `sequence_id` | Sequence progress is not persisted and nothing resumes |
| State on disk | List the private `HOME` after all runs | Only `.claudine/config.json` and `.claudine/state/system_prompts/*.txt` | Claudine persists no run, attempt, or budget state |
| Windows: `TerminateJobObject` | ctypes mirror of `windows_wait_loop` on `build-win-native` | Child and grandchild were gone. `CREATE_BREAKAWAY_FROM_JOB` was denied (WinError 5) | The whole assigned tree is terminated, and descendants cannot break away |
| Windows: closing the job handle only | Mirror; drop the job handle | Tree gone | `KILL_ON_JOB_CLOSE` kills the tree when the wait scope ends |
| Windows: wrapper killed | `TerminateProcess` on the process that owns the job | Tree gone | **Unlike Unix**, a wrapper crash kills its workers |
| Windows: late assignment | Assign 1.5 s after spawn | A grandchild spawned before assignment **survived** | Claudine assigns after a non-suspended spawn, which leaves a narrow escape window |
| Prototype: retry plus backoff | Ledger admission with a 1 s wait between attempts | 2 invocations and 1.50 s charged, including the wait | Wall-segment charging captures automatic waits |
| Prototype: explicit suspension | Suspend, sleep 1 s, attempt a launch, resume | Launch refused while suspended. Only the 0.15 s launch after resume was charged | Pausing works when the run is explicitly suspended |
| Prototype: crash | SIGKILL the coordinator mid-worker, then recover | Invocations stayed at 4 → 4 (no refund); time +0.5 s bounded tail. The orphan group was killed; state became `interrupted`. Restart was refused until an operator resumed | Charge-before-launch plus a heartbeat gives crash accounting without refunds |
| Prototype: exhaustion and grant | Launch until 5/5, then record an operator grant | Refused once exhausted. Admitted after a recorded one-invocation grant | Extra budget only comes from a recorded operator decision |

Rows marked "Prototype" test the Python protocol in `probe_budget.py`, not
Claudine behavior.

### Area findings

**Discovery input isolation.** The child cwd is the repository root that
contains the launch directory. The worker also inherits:

- `HOME`, which carries user-scope provider instructions, memory, MCP servers,
  and skills.
- Claudine's `--system` prompt, resolved from `.claudine/` in the repository
  and the home directory (`claudine/lib/src/system_prompt/resolve.rs`).
- `AGENT_CWD`, which names the launch directory.

Every launch starts a fresh session. A session is resumed only through a
lifecycle `resume` action, and `retry` starts from scratch.

Production launch preparation must do three things:

1. Write the prepared discovery document into a fresh per-run scratch
   directory outside the repository, and launch from there.
2. Declare no `resume` in discovery lifecycles.
3. Run non-interactively.

This is input separation, not confidentiality. A real agent can still read
repository files by absolute path. Report that as a limitation; do not claim a
sandbox.

**Inline-compose.** The relevant code is `prepare_inline` in
`claudine/lib/src/composition/prepare.rs`. It builds the prompt from three
parts:

1. `build_inline_prompt_header`, which carries the absolute document path and
   the schema table.
2. The composed frontmatter `prompt`.
3. The guardrails.

The guardrails direct the agent to edit the document in place. `sequence`
switches every step to this mode when its source declares a top-level `prompt`
(`inline_mode` in `execute_sequence`). Discovery must therefore never run as an
inline-compose of the existing research document, or as a step of such a
sequence. Reconciliation may use inline-compose against a candidate copy,
never against the accepted baseline.

**Process-tree cancellation (local only).**

- **Unix.** `isolate_into_process_group` (`exec/spawn/setup.rs`) spawns the
  child with `process_group(0)`. `send_signal_to_child` (`exec/termination/unix.rs`)
  sends SIGTERM to `-pid`, waits `kill_grace` (default 10 s, set by
  `CLAUDINE_KILL_GRACE`), then sends SIGKILL. After the child exits,
  `kill_process_group` (`exec/mod.rs`) repeats the escalation for any group
  members left, and the reader threads get a 5 s join each. The Kimi wire path
  (`exec/wiring/session.rs`) uses the same group setup. Interactive
  passthrough does not isolate the child (`child_in_own_pgroup == false`), so
  only the direct child is signaled there.
- **Unix escapes.** Nothing catches `setsid` or daemonized descendants: there
  is no cgroup and no parent-death signal. A wrapper crash orphans the whole
  group.
- **Windows.** `windows_wait_loop` (`exec/termination/windows.rs`) spawns with
  `CREATE_NEW_PROCESS_GROUP` and assigns the child to a Job Object with
  `KILL_ON_JOB_CLOSE`. Escalation is a `CTRL_BREAK_EVENT`, then
  `TerminateJobObject`. The job is assigned only when the child is in its own
  group, and only after a non-suspended spawn.
- **Remote work is out of reach.** None of this cancels provider-side requests.
  Report every cancellation as "local tree terminated; remote cancellation and
  billing unverified".

### Recommended hook points and impact

The GitNexus index was incomplete: it reported `incremental-in-progress`, was
indexed at `4399b3a` against HEAD `d57faf7`, and a concurrent
`analyze --watch` owned it. Most sequence symbols did not resolve, so callers
were confirmed by text search. A graph risk of `UNKNOWN` below is unresolved,
not low. Before editing, rerun impact on a fresh index (`just gitnexus`).

| Candidate | Location | Callers (text search) | Graph risk | Role |
|---|---|---|---|---|
| **Admission (primary)**: `execute_attempt_phase` immediately before `execute_harness_attempt` | `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1915` | `execute_harness_attempt`: 1 (`execute_attempt_phase`, reached from `run_harness_loop_inner`) | `UNKNOWN` (0 callers resolved; index incomplete) | The single spawn funnel for every provider attempt. It covers `retry`/`resume`/`proxy` re-entry, sequence body steps, group tasks (`task_run.rs:509` → `execute_composition_request_inner`), compose, inline-compose, and wrappers. Charge here before spawn, and set the attempt's wall-clock timeout to min(configured, remaining) |
| Ledger threading: `run_harness_loop` | `harness_orch/loop_control.rs:142` | 2 (`wrapper_stages.rs:592`, `composition/runner.rs:458`) | `UNKNOWN` | Carry the ledger handle the way `handoff_ledger` is carried today |
| Retry wait: the `ControlDispatch::Retry` branch | `harness_orch/loop_control/control_dispatch.rs:116-129` (`std::thread::sleep(delay)`) | Via `dispatch_terminal_control` (3 sites) | Not queried | Refuse or cap a delay longer than the remaining budget. The wall clock charges it automatically |
| Stage boundary: the step loop in `run_sequence_steps` | `sequence/iterate.rs:109`; caller `sequence/mod.rs:665` | 1 | Symbol missing from the index | Record the stage and check exhaustion between steps |
| Open, lock, and recover the ledger: `execute_sequence` / `run_sequence_inner` | `sequence/mod.rs:171`; `commands/sequence.rs:220` | 1 / 2 | Symbol missing from the index | Add `--budget-ledger <path>`, refuse it with `--interactive`, and run recovery before phase 1c |
| Avoid: `isolate_into_process_group` | `exec/spawn/setup.rs:117` | 3 | **CRITICAL** (3 direct, 14 modules) | Do not modify. Record the child's identity after spawn instead |
| `send_signal_to_child` | `exec/termination/unix.rs:391` | 1 | LOW | Reuse unchanged |

Two related structures are not suitable homes for the budget:

- The existing `RunLedger` (`claudine/lib/src/composition/coordinator/invocation.rs`)
  is in-memory. It is scoped to one sequence step (`iterate.rs` builds one per
  step) and tracks proxy hops. Keep the budget ledger separate.
- The per-document retry/resume budgets in `ActiveDocumentState` are also
  in-memory. They reset on every command run.

The proposed hooks avoid `task_run.rs`, which currently carries another
effort's uncommitted edits.

### Proposed ledger schema and protocol

- **Location.** Messenger passes an explicit path, for example
  `<local working-record dir>/runs/<platform>/<run-id>/budget.json`. Claudine
  never picks a hidden default. A sibling `budget.json.lock` holds an OS file
  lock for the run's lifetime. Claudine already uses that pattern for overlay
  roots. The lock enforces one writer and one platform at a time.
- **Fields.**
  - `schema`, `run_id`, `platform`, `stage`
  - `state`: `active`, `suspended`, `stopped`, `interrupted`, `exhausted`, or
    `completed`
  - `limits`: `{invocations, active_seconds}`. Both are required and have no
    defaults.
  - `grants[]`: `{operator, reason, invocations, active_seconds, recorded_at}`
  - `used`: `{invocations, active_seconds}`
  - `segment`: `{opened_at, heartbeat_at}`, or `null` when the run is not
    active
  - `in_flight`: `{invocation, stage, attempt, pid, process_start, pgid_or_job,
    admitted_at, deadline}`, or `null`
  - `events[]`: append-only entries of kind `admitted`, `settled`,
    `suspended`, `resumed`, `crash_recovered`, `exhausted`, or `granted`
- **Write protocol.**
  - Every write replaces the file atomically: write a temp file, fsync, rename,
    then fsync the directory on Unix.
  - Admission first checks `used` against `limits` plus grants. If allowed, it
    increments `invocations` and writes `in_flight` **before spawn**, then
    records the child's identity after spawn.
  - A ticker in the wrapper folds wall time into `used.active_seconds` at each
    heartbeat. Settlement folds the final delta and clears `in_flight`.
  - Time is charged by wall-clock segments, not child durations. That covers
    orchestration, validation, review, backoff, grace periods, and reader
    joins. Deltas are measured with a monotonic clock; timestamps are
    persisted as UTC.
- **Suspension and stop.** A transition to `suspended` or `stopped` folds the
  open segment and closes it. Resumption opens a new segment. Only these
  explicit states pause the clock.
- **Crash semantics.** An invocation is never refunded, because it is charged
  at admission. Time is charged through the last heartbeat plus one heartbeat
  interval, which bounds the unseen tail from above.
  - Recovery sets the state to `interrupted`. That is an operator-resumption
    state, so downtime after a crash is not charged.
  - The Architecture Record must confirm this choice, or instead charge the
    downtime.
  - On Unix, recovery must check the recorded `pid` and `process_start` before
    it kills the orphaned group, because PIDs are reused. It must also report
    the orphan's extra runtime.
  - On Windows, the job has already died with the wrapper.
- **Exhaustion.** Admission refuses once `used` reaches the limits. The
  in-flight attempt's deadline is the remaining time, so the existing timeout
  path stops it. The ledger records the stage, consumed budget, and stop
  reason. Only an appended operator grant can reopen an exhausted run.

### What the Phase 5 Claudine extension must add

1. A library `BudgetLedger` with the schema, lock, atomic writes, heartbeat,
   recovery, and grant records. Messenger initializes it, suspends and resumes
   it, and appends grants through the library. The CLI gets
   `--budget-ledger` on `sequence`, and on `compose` and `inline-compose` if
   Messenger uses them.
2. Admission in `execute_attempt_phase`, with the per-attempt timeout set to
   min(configured, remaining). This covers retry, resume, proxy, and
   group-task launches.
3. Retry and backoff waits capped to the remaining budget, plus stage and
   exhaustion checks between sequence steps.
4. Child-identity recording and crash recovery that kills a verified orphaned
   Unix process group before any new admission.
5. A distinct exit status and structured report for exhaustion, suspension,
   and recovery. The report states the limitation on remote cancellation and
   billing.
6. A refusal of `--budget-ledger` combined with `--interactive`, because
   passthrough mode does not isolate the child.
7. Optional hardening, if the Architecture Record accepts it:
   - Windows: assign the job at creation (a suspended spawn, or a job-list
     attribute) to close the late-assignment window.
   - Unix: document that `setsid` descendants escape, and that an escapee
     holding the pipes adds about 10 s of charged time.
8. Fake-provider fixtures for criteria 24, 33, and 37 on all four operating
   systems. Native Windows needs a fake provider `.exe`, such as a small test
   binary through `biscuit_test_harness::bin_exe!`, because Rust's batch-file
   argument rule blocks a `.cmd` shim.

No Claudine change is needed for discovery cwd isolation. Messenger launches
from the scratch directory it prepares.

### Per-OS evidence and gaps

| OS | Host | Evidence | Kind |
|---|---|---|---|
| macOS 27.0 | Local | Full `probe_budget.py` against the HEAD build | Behavioral, Claudine binary |
| Linux (kernel 7.0.14) | `build-linux` | Full `probe_budget.py` against the standing-clone binary | Behavioral, Claudine binary (older revision; same termination code) |
| Native Windows (NT 10.0.26200) | `build-win-native` | `probe_tree_windows.py` Job Object mirror; code reading of `termination/windows.rs` | OS semantics only; **no Claudine binary run** |
| WSL2 (Ubuntu-26.04) | `build-win` | Unreachable. Two direct SSH attempts were reset; `wsl.exe` relay attempts failed with `execvpe(bash) I/O error` and exit 11 | **Gap.** WSL2 uses the `cfg(unix)` paths, but Linux results are not WSL2 evidence |

Open gaps:

- A Claudine-binary run on native Windows.
- Any evidence at all on WSL2.
- Remote model cancellation and billing on every OS.
- Real providers' own subprocess trees, such as Node children and MCP servers
  that daemonize.
- A crash-recovery run through Claudine itself, since only the prototype
  exercised it.

The portability contract is unchanged. Phase 5 acceptance requires the
criterion 33 and 37 fixtures to pass on all four operating systems.
