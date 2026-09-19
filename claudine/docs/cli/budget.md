---
blast_radius:
  - claudine/cli/src/budget/
  - claudine/cli/src/commands/budget.rs
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
---
# Shared execution budgets (`claudine budget`, `sequence --budget-ledger`)

A **budget ledger** caps a whole `claudine sequence` run, not a single agent.
Every agent launch in the run draws on the same allowance: each step,
lifecycle `retry`/`resume`/`proxy` re-entry, parallel group task, restart of
the sequence, and launch after crash recovery. The allowance has two limits,
agent **invocations** and **active wall-clock time**. Both are required and
neither has a default.

Per-launch `--timeout` still works, but it bounds only one child process; it
cannot bound a multi-step run. A ledger does.

## Usage

```sh
# Create the ledger. Both limits are required.
claudine budget init runs/discord/2026-09-17-1a2b3c4d/budget.json \
  --run-id 2026-09-17-1a2b3c4d --platform discord \
  --max-seconds 1800 --max-invocations 8 \
  --exclusive-lock ../../fleet.lock

# Run a sequence under it.
claudine sequence --budget-ledger runs/discord/2026-09-17-1a2b3c4d/budget.json run.md

# Operate it between runs.
claudine budget show     <ledger> [--json]
claudine budget suspend  <ledger> --reason "awaiting approval"
claudine budget resume   <ledger> --operator ken
claudine budget grant    <ledger> --operator ken --reason "one recovery" --invocations 1 --seconds 300
```

`--exclusive-lock` names a lock file that several ledgers share. Only one run
holding that lock can execute at a time, which is how runs for different
platforms are kept one at a time. A relative path resolves against the
ledger's directory.

`--heartbeat` (default `5s`) sets how often a running sequence writes charged
time to disk.

## What is charged

- **Invocations** are charged when an agent is admitted, **before** it
  spawns, and are never refunded, whether the agent succeeds, fails, is
  retried, or dies.
- **Time** is charged by wall clock from the moment a budgeted run opens the
  ledger until it closes it. That covers agent work, composition,
  orchestration, `shell:` tasks such as validation, automatic retry backoff,
  kill grace, and output-reader joins.
- Only these states pause the clock: `stopped`, `suspended` (awaiting human
  approval), `interrupted` (awaiting an operator after a crash), and
  `exhausted`. Time outside a budgeted Claudine run is not observable, so any
  work that must be charged has to run inside the sequence.

## Enforcement

- **Admission.** Admission happens where every provider attempt is launched.
  A launch is refused once either limit is reached.
- **Deadline.** An admitted launch's wall-clock timeout is capped at the
  remaining time, rounded **up** to whole seconds because the timeout timer
  has one-second resolution. The existing termination path then stops the
  local process tree: the process group on Unix, the Job Object on Windows. A
  launch can therefore overrun by less than one second, and that overrun is
  charged.
- **Backoff.** A lifecycle `retry` delay is shortened to the remaining time.
- **Steps.** Each step boundary records the step as the ledger's `stage`. An
  exhausted budget stops the run before that step starts. Running shell tasks
  are stopped the same way Ctrl+C stops them, and the step reports
  `stopped: execution budget exhausted`.

Exhaustion stops **local** processes only. Remote model work and billing may
continue, and the ledger and summary say so.

## Resting states and exit statuses

| End of run | Ledger state | Exit status |
|---|---|---|
| Every step succeeded | `suspended` ("awaiting human review") | `0` |
| A step failed | `stopped` | the sequence's status (usually `1`) |
| Ctrl+C | `stopped` | `130` |
| Budget exhausted | `exhausted` (keeps the incomplete `stage`) | `76` |
| Refused at start: suspended, interrupted, or lock held | unchanged | `77` |
| Refused at start: exhausted | `exhausted` | `76` |

A run can start only from `stopped`. To leave `suspended` or `interrupted`,
use `resume`. To leave `exhausted`, only a recorded `grant` that restores
allowance for **both** limits works.

`--budget-ledger` is refused with `--interactive`, because passthrough mode
does not isolate the agent's process tree. It is also refused with
`--dry-run`, which launches nothing.

## Crash recovery

The ledger lock (`<ledger>.lock`) is an OS file lock held for the life of the
run. A ledger that says `active` while its lock is free was abandoned by a
crashed runner. The next `sequence` or mutating `budget` command recovers it:

1. For every launch still `in_flight`, it compares the recorded PID's start
   time with the recorded one; PIDs are reused. When they match and the
   process is still running, it kills the process: the whole process group on
   Unix, where a worker outlives its wrapper, and the process itself on
   Windows, where the Job Object normally takes the tree down with the
   wrapper. A descendant that called `setsid` has left the group and escapes.
2. It charges one heartbeat interval past the last persisted heartbeat, or
   the time a verified orphan kept running, whichever is larger. It never
   refunds anything.
3. It sets the ledger to `interrupted`. The downtime until an operator runs
   `resume` is not charged.

## Ledger document

The ledger is pretty-printed JSON (`format: claudine-budget-ledger/1`) with
`run_id`, `platform`, `state`, `stop_reason`, `stage`, `limits`, `grants[]`,
`used`, `heartbeat_ms`, `exclusive_lock`, `runs` (budgeted runs opened, so
restarts are visible), `segment`, `in_flight[]`, and an append-only
`events[]` history. Other tools may read it, for example with
`claudine budget show --json`. Claudine is the only writer, and it writes by
atomic replacement.

Design record:
[research metadata pipeline architecture](../../../messenger/features/2026-09-17-research-metadata-pipeline/architecture.md#orchestration-and-budget-boundary).
