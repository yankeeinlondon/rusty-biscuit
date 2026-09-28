---
total_phases: 5
created: 2026-09-27
phase: 5
agent: claude/opus
yolo: true
source_files_during_phase_1:
    - claudine/cli/tests/common/webhook_listener.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/cli/tests/l1/main.rs
docs_updated_during_phase_1:
    - claudine/docs/topics/testing.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/os/build-hosts.md
source_files_during_phase_2:
    - claudine/lib/src/messaging/delivery.rs
    - claudine/lib/src/messaging/delivery/tests.rs
    - claudine/lib/src/messaging/mod.rs
    - claudine/lib/src/messaging/send.rs
    - claudine/lib/src/messaging/send/tests.rs
    - claudine/lib/tests/l1/messaging_delivery.rs
    - claudine/lib/tests/l1/messaging_spawn_guard.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/Cargo.toml
    - Cargo.lock
docs_updated_during_phase_2:
    - claudine/docs/topics/messaging.md
    - claudine/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/hook-actions.md
source_files_during_phase_3:
    - claudine/cli/src/shutdown.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/interrupt.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/handle.rs
    - claudine/cli/tests/l1/exit_site_guard.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/lib/src/messaging/delivery.rs
    - claudine/lib/src/messaging/delivery/tests.rs
    - claudine/lib/src/messaging/mod.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/messaging.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/pipeline.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/hook-actions.md
source_files_during_phase_4:
    - claudine/cli/tests/common/webhook_listener.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/cli/tests/l1/handle_message_drain.rs
    - claudine/cli/tests/l1/lifecycle_message_drain_interrupt.rs
    - claudine/cli/tests/l1/main.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/testing.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
    - claudine/docs/topics/messaging.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/building-an-agent-wrapper.md
    - claudine/lib/README.md
    - claudine/cli/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/hook-actions.md
    - .claude/skills/claudine/unified-hooks.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/timeline.md
packages:
    - claudine-cli
    - claudine
source_code:
    - claudine/cli/tests/common/webhook_listener.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/lifecycle_message_drain.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/lib/src/messaging/delivery.rs
    - claudine/lib/src/messaging/delivery/tests.rs
    - claudine/lib/src/messaging/mod.rs
    - claudine/lib/src/messaging/send.rs
    - claudine/lib/src/messaging/send/tests.rs
    - claudine/lib/tests/l1/messaging_delivery.rs
    - claudine/lib/tests/l1/messaging_spawn_guard.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/Cargo.toml
    - Cargo.lock
    - claudine/cli/src/shutdown.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/interrupt.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/handle.rs
    - claudine/cli/tests/l1/exit_site_guard.rs
    - claudine/cli/tests/l1/handle_message_drain.rs
    - claudine/cli/tests/l1/lifecycle_message_drain_interrupt.rs
documentation:
    - claudine/docs/topics/testing.md
    - claudine/docs/topics/messaging.md
    - claudine/docs/dependencies.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/pipeline.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/building-an-agent-wrapper.md
    - claudine/lib/README.md
    - claudine/cli/README.md
completed_phase: 5
implemented: true
---

# Plan: outbound messages sent near process exit are silently dropped

Spec: `2026-09-25-lifecycle-message-exit-race` (`spec.md` in this directory).

## Summary and Definition of Done

### The work

Three library helpers in `claudine/lib/src/messaging/send.rs` start a Tokio
task and drop its handle:

- `execute_message` (line ~163), used by the hook runner in
  `lib/src/dispatch/runner/mod.rs:373`
- `execute_resolved_message` (line ~206), used by `DefaultLifecycleEmitter` in
  `lib/src/composition/lifecycle/mod.rs:436`
- `execute_notification` (line ~320), used by the lifecycle emitter at
  `lib/src/composition/lifecycle/mod.rs:454`

Every affected CLI command then ends in a direct `std::process::exit`, which
kills any send still in flight:

| Site | What it does |
| --- | --- |
| `cli/src/commands/compose/mod.rs:648`, `:662` | `compose`, `inline-compose` |
| `cli/src/commands/sequence.rs:70` | `sequence` |
| `cli/src/commands/wrap/mod.rs:245` | provider wrappers |
| `cli/src/commands/wrap/mod.rs:441` | wrapper exits when no model is provided |
| `cli/src/commands/handle.rs:95`, `:109` | `handle`, and `handle` on deadline (exit `124`) |
| `cli/src/main.rs:198` | top-level error, after `run()` has already dropped the runtime |

The fix has three parts:

1. A process-wide **delivery tracker** in `claudine::messaging` that every send
   registers with.
2. One **ordinary-exit path** in the CLI. Before exiting it drains the tracker
   (10 s total, or less under `handle`'s deadline), reports anything still
   unfinished, flushes stdout and stderr, and exits with the original code.
3. **Guards** that keep both invariants from regressing, **tests** for R6,
   and **documentation** updates for R7.

### Code facts the plan relies on

- `async_main` runs inside `runtime.block_on(...)` on the main thread
  (`main.rs:283-292`). `compose`, `inline-compose`, and `sequence` are
  synchronous functions called from it, so spawned sends run on the
  multi-thread worker pool while the main thread is blocked. The drain can
  therefore be awaited in `async_main` after the command returns.
- `handle` reads stdin with `spawn_blocking` (`handle.rs:~120`). Dropping a
  Tokio `Runtime` waits indefinitely for blocking tasks, so **the process must
  exit while the runtime is still alive**. Returning out of `block_on` and
  then exiting would reintroduce the hang that `handle`'s deadline prevents.
  See Rule 2.
- `UserInterruptGuard` (`compose/interrupt.rs`) is created and dropped inside
  the compose run. On Windows, dropping it withdraws the notice registration,
  and after that a Ctrl+C with no compose run registered does nothing (test
  `a_press_with_no_compose_run_registered_is_inert`). If the guard drops before
  the drain, a second Ctrl+C during the drain is **ignored** on Windows. R4
  therefore requires the guard to outlive the drain.
- Redaction (`send.rs:473`) matches only `https://discord.com/...` URLs. A
  loopback test URL is not redacted, which is why the spec requires a dummy
  token.
- No CLI L1 test uses a messaging route or a loopback HTTP listener yet.
  Phase 1 builds that fixture.

### Definition of done

- [x] On the fixed code, the Phase 1 reproduction test passes: the child waits
      for the withheld reply and the listener receives exactly one POST
      containing the `success` text before the process exits. The old-code
      observation is recorded in `implementation-log.md`.
- [x] `grep` finds no bare `tokio::spawn` or `handle.spawn` in
      `lib/src/messaging/` outside the tracker module, and a guard test enforces
      it.
- [x] The only direct `process::exit`, `_exit`, or `ExitProcess` calls in
      `claudine/cli/src` are the four allowlisted sites. The exit-site guard
      enforces this with an exact allowlist, and an allowlist entry that matches
      no live call site fails the guard.
- [x] Every R6 bullet has a passing test (L1 unless noted). The Ctrl+C test
      runs on macOS, Linux, and Windows.
- [x] A run that sends nothing takes no longer to exit than before. The drain
      returns immediately when nothing is pending.
- [x] Exit codes are unchanged in every scenario, including `124` for `handle`.
- [x] R7 is done: the docs and the `claudine` skill no longer describe
      messaging as fire-and-forget, and the incorrect "3-second timeout" claim
      is removed.
- [x] `just test`, `just test-l2`, and `just lint` in `claudine/` are green.
- [x] Spec frontmatter is updated to `status: implemented` and
      `implemented: true`. The fix is **not** moved to `_completed`; the author
      does that.

### Input Robustness Matrix

Not applicable. The fix adds no parser, config loader, or deserializer, and it
does not change how the messaging config is read. The test fixture writes a
known-good route config and never parses user input.

## Phase 1: Rulings, spikes, and reproduction

### Necessary Rules

Each rule below answers something the spec leaves open. Implementers treat
them as binding. If a spike disproves one, update the rule here and record the
change in the implementation log before moving on.

1. **Where the tracker lives and what it stores.** Put it in a new module,
   `lib/src/messaging/delivery.rs`, re-exported from `messaging`. Its state is
   `static REGISTRY: LazyLock<Mutex<Registry>>`, where each entry is
   `{ id, label: DeliveryLabel, handle: JoinHandle<()> }`. `DeliveryLabel` is
   either `Route(String)`, holding `ResolvedMessagingRoute::name`, or
   `DesktopNotification`. There is no other variant, so a URL, body, token, or
   image path cannot become a label. The lock is a `std::sync::Mutex` held only
   for push, take, and prune. It is never held across `.await` or while a
   warning renders.
2. **One ordinary-exit function, called inside the runtime.** A new module
   `cli/src/shutdown.rs` provides:
   - `async fn finish(code) -> !`, which drains the tracker, reports,
     flushes, and calls `std::process::exit(code)`;
   - a synchronous `fn exit_before_runtime(code) -> !` for errors raised
     before the runtime exists, where no delivery can have started.

   These are the only allowlisted exit sites outside the three the spec
   carves out. `async_main` returns only through `finish`, and `main()`
   never sees an `Ok` from the runtime. Commands that currently return
   `Ok(())` go through `finish(0)` too, which gives one uniform path. Their
   locals have already dropped when they return, so skipping the runtime drop
   loses nothing.
3. **Commands return an exit code; they never exit.** `run_compose`,
   `run_inline_compose`, `run_sequence`, `run_provider_wrapper`, and
   `handle::run` return `Result<i32>`. The dispatch `match` in `async_main`
   becomes `Result<i32>`. An `Err` is rendered by `render_top_level_error`,
   which moves inside `async_main`, and then goes to `finish(1)`. Rendering
   before draining keeps the error block ahead of any drain warnings.
4. **Drain API for embedders.** The library exposes
   `pub async fn drain_deliveries(deadline: tokio::time::Instant) -> DrainOutcome`.
   `DrainOutcome` has `pending: Vec<DeliveryLabel>` and
   `fn report(&self)`, which renders the R3 `Warning` status with the same
   `Status` style as `report_send_failure`. `drain_deliveries` itself prints
   nothing except panic reports (Rule 6), so embedders choose whether to call
   `report()`. After the deadline, `drain_deliveries` aborts and removes every
   unfinished task before returning. The loop takes all entries, awaits them
   with `timeout_at(deadline)`, and repeats until the registry is empty or the
   deadline passes. That covers tasks registered during a drain.
5. **The budget is a named constant, and `handle` caps it.** Add
   `pub const DELIVERY_DRAIN_BUDGET: Duration = Duration::from_secs(10);` in
   `delivery.rs`. No env var or flag controls it. `cli/src/shutdown.rs` holds a
   process-wide `OnceLock<Instant>` drain deadline. `handle::run` sets it to
   `start + resolve_deadline()` **before** `run_inner`, so its error path, which
   goes through `?` to the top-level render, also receives the capped budget.
   `finish` uses `min(now + 10 s, handle_deadline)`. If the handle deadline has
   already passed (the `124` path), the effective deadline is in the past:
   `drain_deliveries` polls once without waiting, then aborts and reports the
   pending tasks.
6. **Panics and pruning.** When the drain or a prune finds a `JoinError` where
   `is_panic()` is true, it reports that entry through the existing failure
   path, with the label and wording "delivery task panicked". The exit code is
   unchanged. Every registration first prunes entries whose
   `handle.is_finished()` is true, so a long-running wrapper does not
   accumulate entries. A task that finishes before it is registered is still
   correct: its `JoinHandle` resolves immediately.
7. **`execute_notification` without a runtime** keeps today's behavior: it logs
   a warning and returns. The tracker cannot register a task that never
   started.
   - *Amended after Spike A:* the same rule applies to **all three** helpers.
     `track` uses `Handle::try_current()` and, when there is no runtime, logs
     a warning and returns instead of panicking. Today `execute_message` and
     `execute_resolved_message` call bare `tokio::spawn`, which panics on a
     thread with no runtime, such as a parallel sequence-group member thread
     (`lib/src/composition/sequence/task/group.rs`, `std::thread::scope`).
8. **Interrupts during the drain.** A compose-family command returns its
   `UserInterruptGuard` (or an opaque `ShutdownHold`) together with its exit
   code. `finish` holds that value until the process exits. A repeat press
   during the drain has `wait_loop_active` and `terminal_lifecycle_active` both
   false, so it lands on `PressRung::ForceExit` immediately. A `GraceExit`
   deadline armed before the drain still fires, because its watcher thread
   lives for the whole process. The drain adds no new interrupt state. Spike B
   confirms the wrapper and `handle` paths.
   - *Amended after Spike B:* the rule holds for `compose` and
     `inline-compose`, which hand `finish` their `UserInterruptGuard`. It does
     not hold for `sequence` or the provider wrappers. `sequence` has no
     `UserInterruptGuard`: its run-scoped flag handler only stores a flag, and
     on Unix the signal-hook handlers it and the wait loops register are never
     unregistered, so a press after the command returns sets a dead flag and
     the process does not die. On Windows the last guard drop restores the
     default disposition, so a press kills the process with
     `STATUS_CONTROL_C_EXIT` instead of `130`. A stalled 10 s drain would
     therefore ignore Ctrl+C on Unix. For `sequence` and the provider
     wrappers, `finish` installs the compose ladder with
     `install_user_interrupt_guard(..)` for the duration of the drain, and
     only when the registry is non-empty, so a run that sent nothing is
     unaffected. `handle` hands over nothing: it installs no handler, and the
     default disposition already exits immediately.
   - A press during the drain by a user who already pressed during the run
     lands on `ForceExit` at once. That is R4's "second Ctrl+C still wins".
9. **Clap and completion exits are outside the guard's scope.** `err.exit()`,
   `Cli::parse_from`, and `completion::maybe_complete()` exit before any
   delivery can start. The guard pattern does not match them textually, so
   they are not allowlisted. The guard module's docs name them explicitly
   (R2).
10. **Reproduction test lifecycle.** The reproduction test is written with the
    **fixed-code** assertions. It is expected to fail on the unfixed tree. That
    failure is observed locally, recorded in the implementation log, and never
    pushed. The test turns green in Phase 3 without edits.
    - *Amended in Phase 1:* each phase must end with `just test` green, so the
      reproduction test carries a reasoned
      `#[ignore = "red until the CLI drains deliveries before exit; …"]`.
      CI's completion check records an ignored test as expected, not missing
      (`scripts/ci/test_completion.py`). Phase 3 deletes that one attribute
      and changes no assertion. Run it meanwhile with
      `--run-ignored all`.

### Spikes

- [x] **Spike A: runtime context of every send site**
    - Trace every caller of the three helpers, including wrapper-side in-process
      dispatch through `handle.block_on(dispatch_event_meta_with_runtime(...))`
      in `wrap/exec/wiring/dispatch.rs:152` and `wrap/live_semantic_sink/mod.rs:339`.
      Confirm each one spawns onto the **main** multi-thread runtime, and not
      onto a temporary runtime that drops before `finish`. Temporary runtimes
      exist in `model_catalog/service.rs`, `harness/speech.rs`, and
      `lifecycle/audio.rs`.
    - Output: a table in `implementation-log.md` (call site, thread, runtime).
      If any site spawns onto a temporary runtime, amend Rule 1 so the tracker
      captures the main runtime `Handle` at first use and spawns there.
- [x] **Spike B: interrupt state after each command returns**
    - For `compose`, `inline-compose`, `sequence`, the provider wrappers, and
      `handle`, record which SIGINT or console handler is installed at the
      moment the command returns, on Unix and on Windows. Record what a second
      Ctrl+C does there today.
    - Output: confirm or amend Rule 8, and name the value each command hands to
      `finish`. A wrapper that keeps the default SIGINT disposition already
      exits immediately, which is acceptable.
- [x] **Spike C: a silent seam for desktop notifications**
    - Read `messenger/lib/src/provider/desktop/` for each OS and work out how to
      make a terminal `notify` either fail fast or stall **without** showing
      host UI. Candidates: a PATH without `notify-send` on Linux, or a missing
      `APPDATA` app ID on Windows.
    - Output: a ruling on which assertions are CLI-level L1 and which are
      library unit tests. The fallback is to exercise tracking through a
      `#[cfg(test)]` or `pub(crate)` entry point that registers an arbitrary
      future under `DeliveryLabel::DesktopNotification`. Do **not** add a
      production silencing override.

### Tasks

**Wave 1** (all parallel; the spikes are read-only)

- [x] **Spike A**, **Spike B**, **Spike C**, as above.
- [x] **Webhook listener fixture**
    - Add `cli/tests/common/webhook_listener.rs`: a `std::net::TcpListener` on
      `127.0.0.1:0`, served by one thread. It supports these modes:
      `Reply(status)`, `WithholdUntilReleased` (it reads the request and
      replies only once the test calls `release()`), and `NeverReply`. It
      records each request's method, path, and body, exposes
      `wait_for_request(timeout)`, and has a watchdog that shuts it down.
    - Add a helper that writes an env-backed Discord webhook route into the
      fixture home's messaging config (`webhook_url_env`, not an inline URL),
      and that sets the env var on the child to
      `http://127.0.0.1:{port}/webhooks/1/dummy-token`. Confirm the config
      location and schema against `lib/src/messaging/config.rs`.
    - Build on the existing `CliProcessFixture` and provider-stub helpers,
      which have `.cmd` variants for Windows. Spawn only through
      `command_std()` or `command_builder()`, so `spawn_site_guard.rs` stays
      green.
- [x] **Reproduction test** (depends on the listener fixture; run it after
      that fixture lands, still in Wave 1)
    - Add `cli/tests/l1/lifecycle_message_drain.rs` with
      `compose_success_message_is_delivered_before_exit`. A stub `claude`
      prints one line and exits `0`. The document has a `success: message:`.
      The listener is in `WithholdUntilReleased` mode.
    - Fixed-code assertions: the child is still running after the request
      arrives; after `release()` the child exits `0`; exactly one POST arrived,
      and its body contains the rendered text. Do **not** assert that the old
      code sent no request. Give both the child and the listener a watchdog.
    - Run it on the unfixed tree. Record in `implementation-log.md` whether the
      child exited while the reply was withheld or before any request arrived.
      Either outcome reproduces the bug.

**Validation checkpoint 1**

- [x] The reproduction is recorded, the three spike outputs are in the log,
      Rules 1–10 are confirmed or amended, and the spawn guard still passes.

## Phase 2: Delivery tracker in the library (R1, R5)

**Wave 2**

- [x] **Tracker module**
    - Implement `lib/src/messaging/delivery.rs` per Rules 1, 4, 5, 6, and 7:
      `track(label, future)`, which spawns, prunes, and registers;
      `drain_deliveries(deadline)`; `DrainOutcome::report()`; and
      `DELIVERY_DRAIN_BUDGET`.
    - Module `//!` docs state the contract: any helper that starts a delivery
      task must go through `track`, and the guard enforces this.
- [x] **Tracker unit tests** (paired with the module)
    - A task that finishes before `drain` is not reported.
    - A task registered while a drain is running is awaited.
    - Several stalled tasks share **one** deadline: drain time is about the
      deadline, not a multiple of it.
    - After the deadline, `pending` has one entry per stalled task, and those
      tasks are aborted (a drop sentinel fires).
    - A panicking task is reported and not re-raised.
    - Registering prunes finished entries, so after N quick sends the registry
      stays small.
    - A deadline that has already passed returns without waiting.
    - `DeliveryLabel` rendering never contains the URL, token, or body.
    - Use a paused Tokio clock (`start_paused`) wherever possible so the tests
      stay fast.

**Wave 3** (parallel; both depend on Wave 2)

- [x] **Migrate the three helpers**
    - `execute_message`, `execute_resolved_message`, and
      `execute_notification` call `delivery::track(...)` instead of spawning
      directly. `report_send_failure` and `report_notification_failure` stay
      inside the tracked future, so R5 needs no new code.
    - Rewrite the "fire-and-forget" wording in the module and function docs
      (R7, first bullet). Update the existing tests in `send/tests.rs` if any
      depended on untracked spawning.
- [x] **Messaging spawn guard**
    - Add an L1 source-scan guard, reusing `cli/tests/common/source_scan.rs`
      (`sanitize`, so comments and strings are ignored), or the equivalent
      library-side test. It fails if `tokio::spawn`, `.spawn(`, or
      `spawn_blocking` appears in `lib/src/messaging/**` outside
      `delivery.rs`. Include a self-test that the detector catches a planted
      site.
- [x] **Library embedder test**
    - A library test runs `execute_resolved_message` against an in-process
      loopback listener, calls `drain_deliveries`, and asserts the request
      arrived. This pins the opt-in drain contract for embedders.

**Validation checkpoint 2**

- [x] `just test` in `claudine/` passes for the lib crate. The reproduction
      test is still red, as expected, because the CLI does not drain yet.

## Phase 3: One ordinary-exit path in the CLI (R2, R3, R4)

**Wave 4**

- [x] **Shutdown module and `main`**
    - Add `cli/src/shutdown.rs` per Rules 2, 3, and 5: `finish`,
      `exit_before_runtime`, `set_drain_deadline`, and a `ShutdownHold` holder
      for Rule 8.
    - Change `async_main` to compute `Result<i32>` from the dispatch, render
      any error in place, and call `finish(code).await`.
    - `main()` keeps `run_audio_worker_if_requested`'s exit, which is
      allowlisted. Pre-runtime errors go through `exit_before_runtime(1)`.
      Delete the old `render_top_level_error` + `exit(1)` pair in `main()`.

**Wave 5** (parallel; each owns different files, and each depends on Wave 4)

- [x] **Compose family**
    - `run_compose`, `run_inline_compose`, and `run_sequence` return
      `Result<(i32, ShutdownHold)>` (or register the hold with `shutdown`) and
      no longer call `process::exit`. Carry the `UserInterruptGuard` out of
      `run_*_inner` so it lives through the drain (Rule 8; follow Spike B's
      outcome for `sequence`).
    - Check that the budget-ledger wrapper in `run_sequence` still returns the
      code unchanged.
- [x] **Provider wrappers**
    - `run_provider_wrapper` returns the code instead of calling exit at
      `wrap/mod.rs:245`, and the no-model branch at `:441` returns `Ok(1)`.
      Its perf report and `AgentErrorReport` still render before the return.
      Update the early-return call site in `async_main`.
- [x] **`handle`**
    - Compute `start` and `deadline_at` before `run_inner`, call
      `shutdown::set_drain_deadline(deadline_at)`, and use
      `tokio::time::timeout_at`.
    - `Ok(Ok(code))` returns the code after `flush_streams()`. On elapsed, print
      the existing diagnostic and return `124`. The drain sees the past
      deadline and reports without waiting. The `Err` path returns through
      `?`, unchanged.
    - Update the `## Exit discipline` doc comment: the handler flushes, and the
      shared shutdown path exits.
- [x] **Exit-site guard**
    - Add `cli/tests/l1/exit_site_guard.rs`, modeled on `spawn_site_guard.rs`.
      It scans `claudine/cli/src/**` with `source_scan::sanitize` for
      `process::exit(`, `_exit(`, and `ExitProcess(`, and checks every match
      against an exact allowlist of (file, reason) entries:
      `shutdown.rs` (the ordinary exit and the pre-runtime exit),
      `main.rs` (the audio worker), `commands/compose/interrupt.rs` (forced
      exits), and `commands/wrap/exec/termination/windows.rs` (generated
      child-program text).
    - Both failure directions are covered: a live site with no entry fails, and
      an entry that matches no live site fails.
    - Module docs list the Clap and completion exits as out of scope (Rule 9).
      Add a detector self-test.
    - *Amended in Phase 3:* `wrap/exec/termination/windows.rs` has **no**
      entry. Its `process::exit` calls sit inside a raw string literal (the
      generated child program), which `source_scan::sanitize` blanks, so it
      has no live site, and an entry would fail the stale-entry check. The
      allowlist is exact per file (site counts): `shutdown.rs` 2, `main.rs` 1,
      `commands/compose/interrupt.rs` 5.

**Validation checkpoint 3**

- [x] The reproduction test from Phase 1 passes **unchanged**.
- [x] Existing suites that pin exit codes and deadlines still pass:
      `handle_deadline.rs` (including the `124` path),
      `handle_blocking_output.rs`, `compose_cli.rs`, `sequence_cli.rs`,
      `sequence_budget.rs`, and `sequence_ctrl_c_windows.rs`.
- [x] Spot check: `claudine compose` with no messaging route exits as fast as
      on `main` (compare wall time over a few runs).

## Phase 4: Verification matrix (R3, R4, R5, R6)

**Wave 6** (parallel; every test uses the Phase 1 fixture and lives in
`lifecycle_message_drain.rs` or a sibling file, split by file to avoid edit
conflicts)

- [x] **Failure reported (R5)**
    - The listener replies `400`. Stderr contains "Failed to send lifecycle
      message", the exit code is `0`, and stderr does not contain
      `dummy-token`.
- [x] **Stalled route and shared budget (R3)**
    - Two stalled deliveries: two routes, or one lifecycle `message` plus a
      stalled second event. Assert that the run exits in no more than the
      10-second budget plus a margin, that the Warning names each route, says
      delivery is unknown, and contains neither the token nor the body, and
      that the exit code is `0`.
    - Name it `slow_...`. Add no production override.
    - *Amended in Phase 4:* the test keeps an ordinary L1 name
      (`stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown`).
      Claudine does not declare `l1-include-slow`, so every recipe and CI leg
      filters `slow_` tests out, and the marker would strand it. It takes
      about 10.4 s, inside nextest's 30 s terminate limit.
- [x] **`handle` hook message**
    - A hook `message` action, with the listener withholding the reply briefly
      and then releasing it. The POST arrives before the handler exits.
    - A second case sets `CLAUDINE_HANDLE_DEADLINE_SECONDS` small (for example
      `2`) with a stalled route. It asserts that the handler exits within that
      deadline plus a margin, not after 10 s, and that the exit code is `0` or
      `124` as appropriate.
    - A third case makes the handler work itself exceed the deadline. It
      asserts exit `124` and a pending-delivery warning with no extra wait.
- [x] **`sequence` last step**
    - The last step's `success` sends a message, and the listener receives it
      before the process exits.
- [x] **Terminal `notify`**
    - Follow Spike C's ruling. At minimum, the unfinished notification is
      reported with the `desktop notification` label and no host UI appears.
- [x] **Top-level error after a send**
    - The document sends in `start`, and a later step fails with an error that
      propagates through `?`. The error block renders, the send is delivered
      (or reported), and the exit code stays `1`.
- [x] **Second Ctrl+C during drain (R4)**
    - With a stalled route, send one Ctrl+C after the drain starts: the notice
      appears and the drain continues. Send a second: the process exits `130`
      promptly, well before the 10 s budget.
    - Use the existing signal helpers in `cli/tests/common/signal.rs` and
      `terminal_interrupt.rs`: SIGINT on Unix, and the
      `CREATE_NEW_PROCESS_GROUP` console-event pattern from
      `sequence_ctrl_c_windows.rs` on Windows.
    - No terminal or browser window may gain focus.

**Wave 7**

- [x] **Cross-OS evidence**
    - Load the `os` skill. Run the new tests on native Windows and Linux via
      the hosts it lists, and on macOS locally. Record the results in the
      implementation log. Any single-OS failure gets a fix plus an `os` skill
      note in the same change.

**Validation checkpoint 4**

- [x] Every R6 bullet maps to a named passing test. List the mapping in the
      implementation log.

## Phase 5: Documentation and close-out (R7)

**Wave 8** (parallel)

- [x] **`docs/topics/messaging.md`**
    - Explain the CLI drain (10 s total, shared across deliveries, a Warning
      for unfinished deliveries saying delivery is unknown, exit code
      unchanged), the cap from `handle`'s shared overall deadline, and the
      library's opt-in `drain_deliveries` contract. Add a small Mermaid
      sequence diagram: send → track → command returns → drain → exit.
    - Remove the incorrect "3-second timeout /
      `CLAUDINE_MESSENGER_TIMEOUT_SECONDS`" claim.
- [x] **`docs/topics/flow-control/lifecycle.md`**
    - Add one sentence near the `message` row in the communication-properties
      section: a message from a terminal event finishes or is reported as
      unfinished before an ordinary exit, within 10 s, and a timeout leaves
      delivery uncertain.
- [x] **`claudine` skill**
    - Correct any messaging "fire-and-forget" wording and the 3-second claim in
      `.claude/skills/claudine/hook-actions.md`, `SKILL.md`, and
      `architecture.md`. Audio stays fire-and-forget; only messaging changes.
    - Add the tracker and the single-exit-path rule to `architecture.md`, plus
      a timeline entry.
- [x] **`docs/topics/signal-handling.md`**
    - If the rules changed wording, note that a second Ctrl+C during the drain
      takes the force-exit rung.

**Wave 9**

- [x] **Final gates**
    - Run `just lint`, `just test`, and `just test-l2` in `claudine/`. Grep the
      docs for stale "fire-and-forget" wording about messaging.
    - Finish `implementation-log.md` with the reproduction, spike outputs, any
      rule amendments, departures from the spec, and the R6 test map.
    - Set the spec frontmatter to `status: implemented`, `implemented: true`,
      and `implemented_by: claude/opus`. Leave the directory in place; the
      author moves it to `_completed` after review.
