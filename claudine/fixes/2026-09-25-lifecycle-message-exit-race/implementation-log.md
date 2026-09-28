---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-25-lifecycle-message-exit-race/spec.md"
plan: "claudine/fixes/2026-09-25-lifecycle-message-exit-race/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
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

# Implementation Log for 2026-09-25-lifecycle-message-exit-race (5 phases)

## Phase 1

- Started 2026-09-27.

### Reproduction (R6, Rule 10)

- Fixture: `cli/tests/common/webhook_listener.rs`. A loopback
  `std::net::TcpListener` served by one polling thread, so several withheld or
  stalled connections can be held at once. Modes `Reply(status)`,
  `WithholdUntilReleased`, `NeverReply`, plus a watchdog (default 90 s).
  `write_webhook_route(home)` writes an active env-backed `discord_webhook`
  route (`messenger.active_config` / `messenger.configurations` in
  `~/.claudine/config.json`, the `ClaudineConfig` shape read by
  `load_claudine_config` → `bridge_messaging_settings`).
  `apply_route_env(cmd)` sets `DRAIN_TEST_WEBHOOK_URL=http://127.0.0.1:{port}/webhooks/1/dummy-token`,
  removes inherited `*_PROXY` variables, and sets `NO_PROXY`, so a host proxy
  cannot intercept the loopback post. A `200` reply carries the
  `{id, channel_id, webhook_id}` fields the messenger's `wait=true` parser needs.
- Test: `cli/tests/l1/lifecycle_message_drain.rs` →
  `compose_success_message_is_delivered_before_exit`, written with the
  fixed-code assertions. It spawns only through `fixture.command_std()`.
- **Observed on the unfixed tree (macOS, two runs):** the child exited `0`
  about 0.35 s after spawn, **before any request reached the listener**:
  `claudine exited (exit status: 0) before the success message reached the listener`.
  The compose output ends with `✓ 0.1s · no tool calls`, then nothing. The
  spawned send task was killed by `std::process::exit` before it opened a
  connection. This is the reported loss.
- Control: `compose_start_message_reaches_the_listener_during_the_run`. The
  same fixture with a `start` message and a ~3 s stub passes on the unfixed
  tree: one POST containing the start text. This proves the route, env var,
  and listener are wired correctly, so the red reproduction is the race and
  not a misconfigured route.
- Listener self-tests live in the same file (`listener_fixture::*`) rather
  than in `common`, because `common` is compiled into the level2, level3, and
  real binaries too.
- **Rule 10 amendment:** `just test` must be green at the end of the phase,
  so the reproduction carries
  `#[ignore = "red until the CLI drains deliveries before exit; …"]`. CI's
  completion validator treats an ignored test as expected, not missing.
  Phase 3 removes the attribute and changes nothing else.
  Run: `cargo nextest run -p claudine-cli --features test-fixtures --test l1 -E 'test(lifecycle_message_drain)' --run-ignored all`.

### Spike A: runtime context of every send site

| Call site | Path | Thread | Runtime |
| --- | --- | --- | --- |
| `lib/src/dispatch/runner/mod.rs:373` → `execute_message` | `claudine handle` (`handle.rs` → `dispatch_canonical().await`) | main thread inside `runtime.block_on(async_main)` | main multi-thread |
| same runner site via `dispatch_event_meta_with_runtime` | Kimi wire hooks, `wrap/exec/wiring/dispatch.rs:152` `handle.block_on(..)` | Kimi stdout-reader `std::thread`, handle captured on main thread | main (spawn lands on the handle's runtime) |
| same runner site | live semantic sink, `wrap/live_semantic_sink/mod.rs:339` `handle.block_on(..)` | stdout-reader `std::thread`, handle captured on main thread | main |
| `composition/lifecycle/mod.rs:436` (`execute_resolved_message`), `:454` (`execute_notification`) | compose / inline-compose (`compose/prep.rs`) | main thread; `run_compose` etc. are sync fns called from `async_main` | main |
| same emitter | wrapper composition (`wrap/wrapper_stages.rs`, `wrap/composition/pipeline.rs`, `staged_boot.rs`, …) | main thread | main |
| same emitter | sequence **shell** tasks, `wrap/sequence/task_run.rs:100` | main thread (serial) or scoped `std::thread` (parallel group) | main, or **no runtime** on group threads |

- Verdict: **no send site spawns onto a temporary runtime.** The temporary
  runtimes (`model_catalog/service.rs`, `harness/speech.rs`,
  `lifecycle/audio.rs`, `main.rs:208` audio worker) never reach dispatch or
  the lifecycle emitter. Rule 1 stands unchanged.
- Side findings:
  - `task_run.rs:102-105` hard-codes `RuntimeMessagingSettings { user: None, repo: None }`
    for sequence shell-task stacks, so they never send. **Phase 4's
    `sequence` test must use an agent step**, whose lifecycle goes through the
    compose pipeline with `bridge_messaging_settings`.
  - Parallel sequence-group members run on scoped `std::thread`s with no
    runtime. There `execute_notification` warns and drops, and a bare
    `tokio::spawn` in the other two helpers would panic. Rule 7 is amended so
    `track` warns and returns for all three helpers when no runtime exists.

### Spike B: interrupt state after each command returns

| Command | Guard at return | Unix: press after the command returns | Windows |
| --- | --- | --- | --- |
| compose / inline-compose | `UserInterruptGuard` created at `compose/prep.rs:121`, dropped when `run_composition_inner` returns, **before** `exit` | compose handler still installed (SigId has no Drop): no earlier press → `Notice`; earlier press → `ForceExit` (`_exit(130)`) | notice cell cleared, console handler released → default disposition (kill, `STATUS_CONTROL_C_EXIT`) |
| sequence | **no** `UserInterruptGuard`; run-scoped flag guard `_sigint_guard` (`wrap/sequence/mod.rs:249`) drops at end of `execute_sequence` | flag-only handler plus inert wait-loop handlers stay installed → **every press is swallowed** | `SequenceInterruptGuard` drop restores default → kill |
| provider wrappers | none; wait-loop handlers only | inert wait-loop handlers swallow SIGINT once any child was waited on | default → kill |
| handle | none | default disposition → kill | default → kill |

- Rule 8 **confirmed for compose/inline-compose** (hand over the
  `UserInterruptGuard`). **Amended for sequence and wrappers:** `finish`
  installs the compose ladder (`install_user_interrupt_guard`) for the drain
  when the registry is non-empty; otherwise a stalled drain ignores Ctrl+C on
  Unix. `handle` hands over nothing.
- A press during the drain by a user who already pressed during the run hits
  `ForceExit` immediately. That matches R4 ("a second Ctrl+C still wins").
- Drift noticed, not fixed here (Phase 5 doc pass):
  `docs/topics/signal-handling.md:74` says the guard's Drop "removes the
  registered handler", which is wrong on Unix.

### Spike C: a silent seam for desktop notifications

| OS | Mechanism (`messenger/lib/src/provider/desktop/`) | Silent fail-fast via env | Silent stall |
| --- | --- | --- | --- |
| macOS | helpers (terminal-notifier, alerter) found by sniff, else `osascript` via **blocking** `std::process::Command` on a Tokio worker | yes: PATH without `/usr/bin` and helpers, empty `HOME` | only with a sleeping `osascript` stub on PATH |
| Linux | helpers (notify-send 5 s, dunstify 3 s), else `notify-rust` over D-Bus | yes: PATH without helpers, `DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent`, unset `XDG_RUNTIME_DIR` | only with a stub helper (capped at 5 s) or a test-owned fake bus |
| Windows | WinRT toast needing an AppUserModelID | **always fails fast**: Claudine passes `DesktopConfig::default()` (`app_id: None`) → `MissingConfiguration`, no UI | impossible |

- **Ruling:**
  - CLI-level L1 (all three OSes, env scrubbed as above): a terminal `notify`
    does not change the exit code, the process exits promptly, and once
    tracking lands the existing desktop-notification failure warning always
    appears (today it may not, because of the race).
  - Library unit tests: an *unfinished* notification reported under the
    `desktop notification` label, drain timeout, and abort. They use a
    `#[cfg(test)]` / `pub(crate)` entry point that registers an arbitrary
    future under `DeliveryLabel::DesktopNotification`. No production
    silencing override is added.
- Phase 2 note: on macOS the native path runs `osascript` synchronously on a
  worker thread. `abort()` cannot stop it, but `finish` exits the process
  inside the runtime, and `process::exit` does not wait for worker threads.
  The drain awaits `JoinHandle`s with `timeout_at` on other workers, so the
  deadline still holds. Side finding: Claudine's `notify` never shows anything
  on Windows because it supplies no AppUserModelID. That is out of scope here.

### Rules 1–10 status

- Confirmed unchanged: 1, 2, 3, 4, 5, 6, 9.
- Amended in `plan.md`: 7 (no-runtime warn-and-return for all helpers),
  8 (sequence/wrapper drain installs the compose ladder), 10 (`#[ignore]`
  until Phase 3).

### Cross-OS evidence (`--run-ignored all lifecycle_message_drain`)

| OS | Listener self-tests (4) | Control (`start` message) | Reproduction (ignored) |
| --- | --- | --- | --- |
| macOS (local) | pass | pass (3.3 s) | fails as expected: exit 0 before any request |
| Linux (`build-linux`, `just cross-check --os linux … --features test-fixtures`) | pass | pass (3.1 s) | fails as expected: exit 0 before any request |
| Windows native (`build-win-native`, `just cross-check --os windows`) | pass | pass (4.0 s) | fails as expected: exit code 0 before any request |

- The first Linux attempt failed before any test ran:
  `target/release/deps/librenderable-*.rmeta is not writeable`. This is the
  stale kache-hardlink problem the `os` skill already records for another
  standing clone. The documented workaround (pass a build flag, which takes
  the native debug path) ran green. The `os` skill now notes that this
  clone is affected too.

### Gates

- `just test` (claudine area): 7381 passed, 10 skipped; the reproduction is
  one of the skipped (ignored). `spawn_site_guard` and `test_placement` pass
  with the new files.
- `just lint`: exit 0. The only warning is the pre-existing macOS linker
  `__eh_frame` notice.
- Pre-existing issue, not fixed: `just test-cli -E "'test(...)'"` fails with a
  shell syntax error inside the shared `_test` recipe. Filtered runs used
  `cargo nextest run` directly.

### Requirement-to-test mapping (Phase 1)

| Requirement | Test |
| --- | --- |
| R6 reproduce first; fixed-code delivery before exit, exactly one POST with the `success` text | `lifecycle_message_drain::compose_success_message_is_delivered_before_exit` (ignored until Phase 3) |
| fixture wiring is load-bearing (control row) | `lifecycle_message_drain::compose_start_message_reaches_the_listener_during_the_run` |
| listener behavior the later tests rely on | `lifecycle_message_drain::listener_fixture::{reply_mode_records_the_request_and_answers_with_its_status, withheld_connections_answer_only_after_release, never_reply_holds_the_connection_until_the_watchdog_fires, webhook_url_uses_the_dummy_token_on_loopback}` |

- Validation checkpoint 1: the reproduction is recorded, the spike outputs
  are logged, Rules 1–10 are confirmed or amended, and the spawn guard passes.
- The claudine skill needed no change in this phase. The fixture is documented
  in `docs/topics/testing.md`, which the skill links through `topics/`.

## Phase 2

- Started and finished 2026-09-27.

### What landed

- `lib/src/messaging/delivery.rs` (new, private module; public items
  re-exported from `claudine::messaging`):
  - `DELIVERY_DRAIN_BUDGET` (10 s), `DeliveryLabel::{Route(name), DesktopNotification}`
    whose `Display` is `route {name}` or `desktop notification`,
    `DrainOutcome { pending, panicked }` with `report()`, and
    `drain_deliveries(deadline)`.
  - `pub(crate) fn track(label, future)`: `Handle::try_current()` (warn and
    return without a runtime, per amended Rule 7), spawn, then under the lock
    partition out finished entries and push the new one. Finished entries are
    polled with `Waker::noop()` **after** the lock is released, and a panic is
    reported (Rule 6).
  - Drain loop: take all entries, `timeout_at(deadline, handle)` each, repeat
    until the registry is empty. Once the deadline has passed, one last sweep
    polls late registrations once, and then the loop stops, so a task that
    keeps registering cannot hold the exit open. Timed-out entries are
    `abort()`ed and listed in `pending`. A `Cancelled` join error is not
    reported (only the drain cancels).
  - The pending warning reads
    `Route <name> was still sending at exit; delivery is unknown`, and with
    several entries `…, route <b>, desktop notification were still sending …`,
    rendered as a `Warning` `Status`. The route name is prose-escaped.
- `send.rs`: the three helpers call `track(...)`. `execute_notification` lost
  its own `try_current` block because `track` owns that now.
  `report_delivery_panic` sits beside the other two reporters and uses the
  same "Failed to send …: delivery task panicked" shape. `prose_escape` became
  `pub(super)`. The "fire-and-forget" wording in the module docs and all three
  function docs was rewritten (R7, first bullet).

### Departures from the plan (Rules unchanged in intent)

- **No `id` field on registry entries** (Rule 1 listed `{ id, label, handle }`).
  Nothing reads an id: pruning uses `is_finished()` and the drain takes the
  whole `Vec`, so an id would be dead state.
- **`DrainOutcome` gained `panicked: Vec<DeliveryLabel>`** beside `pending`.
  It makes Rule 6's "panics are reported, not re-raised" observable to tests
  and embedders. Panics are still printed as they are found; `report()` prints
  only the pending warning.
- **The spawn guard is library-side**, in `lib/tests/l1/messaging_spawn_guard.rs`,
  not a CLI test reusing `cli/tests/common/source_scan.rs`. A CLI test that
  reads `lib/src` would not run on a lib-only change (CI selects the owning
  package's tests; dependents are only compile-checked), which is exactly the
  change this guard polices. The lib has no sanitizer, so the guard lexes with
  `proc-macro2` (new lib dev-dependency, `span-locations`, the same spec as
  `claudine-cli`). Comments, doc comments, and strings cannot trip it. It flags
  `spawn_blocking`, `spawn_local`, and any `spawn` ident after `.` or `:`
  (which also catches `std::thread::spawn`).
- **Tokio `test-util`** was added to the lib's dev-dependency features for the
  paused-clock tests.
- **Docs, ahead of Phase 5:** `docs/topics/messaging.md` gained a "Delivery
  tracking" section documenting the library contract that now exists, with the
  CLI drain marked **planned**. The stale "3-second timeout" bullet was left
  for Phase 5, which owns that rewrite. The claudine skill's
  `hook-actions.md` `message` row said `tokio::spawn`; it now names the
  tracker. Its 3-second line is also left for Phase 5.

### Verification

- Guard proven load-bearing: appending `fn _planted() { tokio::spawn(async {}); }`
  to `send.rs` made `messaging_starts_tasks_only_through_the_delivery_tracker`
  fail with `src/messaging/send.rs:777:24`. The file was restored.
- Rule 7 regression: `execute_resolved_message_without_a_runtime_does_not_panic`
  covers a real route outside a runtime. On the old code the bare
  `tokio::spawn` panics there ("must be called from the context of a Tokio
  1.x runtime").
- The reproduction `compose_success_message_is_delivered_before_exit` is
  still red with `--run-ignored all`: `claudine exited (exit status: 0) before
  the success message reached the listener`. This is expected until Phase 3.
- `just lint` (claudine): exit 0. The only warning is the pre-existing macOS
  linker `__eh_frame` notice.
- `just test` (claudine): 7398 passed, 10 skipped (same skip set as Phase 1,
  including the ignored reproduction).
- Cross-OS, new and touched messaging tests:

| OS | Command | Result |
| --- | --- | --- |
| macOS (local) | `just test` | pass |
| Linux (`build-linux`) | `just cross-check claudine --os linux --all-features -E '…messaging…'` | 57/57 pass |
| Windows native (`build-win-native`) | `just cross-check claudine --os windows messaging` | 116/116 pass (the 16 new tests included) |

- Linux again hit the known stale kache hardlink
  (`librenderable-*.rmeta is not writeable`) without a build flag. The claudine
  lib has no features, so `--all-features` is the flag that takes the native
  path; it ran green. The `os` skill already records this for this clone.
- Windows cross-check with `-E '…'` failed in the shared `_test` recipe
  (`syntax error near unexpected token '('`), the same pre-existing quoting
  bug Phase 1 logged for `just test-cli -E`. A positional name filter works.

### Requirement-to-test mapping (Phase 2)

| Requirement | Test |
| --- | --- |
| R1: a quickly finishing task is not lost or reported | `messaging::delivery::tests::a_delivery_that_finishes_is_not_reported` |
| R1: a task registered while draining is awaited | `…::a_delivery_registered_while_draining_is_awaited` |
| R3: one shared deadline; stalled tasks listed and aborted | `…::stalled_deliveries_share_one_deadline_and_are_aborted` (paused clock: elapsed is exactly 10 s for four stalled deliveries; drop sentinels fire) |
| R3 / Rule 5: a past deadline does not wait | `…::a_deadline_already_passed_returns_without_waiting` |
| R3: nothing pending returns at once | `…::nothing_tracked_returns_immediately` |
| R1 / Rule 6: a panic is reported, not re-raised, and the drain continues | `…::a_panicking_delivery_is_reported_and_not_re_raised` |
| R1: finished entries (including panics) are pruned | `…::registering_prunes_finished_deliveries` |
| Rule 7: no runtime means warn and return | `…::tracking_without_a_runtime_neither_panics_nor_registers`, `messaging::send::tests::execute_resolved_message_without_a_runtime_does_not_panic` |
| R1 / R3: labels and the warning hold no URL, token, or body; the warning says delivery is unknown | `…::labels_render_only_the_route_name_or_the_notification_label`, `…::the_pending_warning_names_each_delivery_and_says_delivery_is_unknown`, `…::panics_alone_produce_no_pending_warning` |
| R2: embedder opt-in drain (real HTTP) | `claudine::l1 messaging_delivery::a_drained_send_has_been_delivered_when_the_drain_returns` (the reply is withheld 300 ms, and the drain returns only after it is written) |
| R3 through the public API, no secret in the label | `messaging_delivery::a_stalled_send_is_pending_under_its_route_name_alone` |
| R6: guard that every messaging task goes through the tracker | `messaging_spawn_guard::{messaging_starts_tasks_only_through_the_delivery_tracker, the_detector_finds_every_spawn_form, the_detector_ignores_comments_strings_and_unrelated_names}` |

- Tier placement: all new tests are L1 (no tier markers). The lib unit tests
  compile into the lib target. The two lib L1 files are declared in
  `lib/tests/l1/main.rs`, and `test_layout` passes. Repository reads use
  `manifest_dir!().join("src/messaging")`.
- The tracker is process-wide. The tests rely on nextest's process-per-test
  model to stay isolated, and the tests module says so.
- Unrelated: `claudine/features/2026-09-21-lifecycle-ergonomics/spec.md` shows
  as modified in the worktree. This phase did not touch it.

## Phase 3

- Started and finished 2026-09-27.

### What landed

- **`cli/src/shutdown.rs` (new).** `finish(code) -> Infallible` computes
  `min(now + DELIVERY_DRAIN_BUDGET, handle deadline)`, installs the drain
  Ctrl+C ladder only when a delivery is still running and no compose guard
  is held (amended Rule 8), awaits `drain_deliveries`, calls `report()`,
  flushes stdout and stderr, and then calls `std::process::exit(code)` inside
  the runtime. It also provides `exit_before_runtime(code) -> !`,
  `set_drain_deadline(Instant)` (a `OnceLock`, first call wins), and
  `hold_interrupt_guard(UserInterruptGuard)` (a static `Mutex<Option<_>>`).
- **`main.rs`.** `run()` returns `Result<Infallible>`, so it returns only for
  a pre-runtime error, which `main` renders before calling
  `exit_before_runtime(1)`. `async_main` calls `dispatch()` (the former body,
  now `Result<i32>`), renders an `Err` with `render_top_level_error`, and then
  awaits `shutdown::finish(code)`. The error block therefore precedes any
  drain warning. Commands that return `Result<()>` map to `0`, and
  `handle`, `compose`, `inline-compose`, `sequence`, and the provider wrappers
  return their code directly.
- **Compose family.** `run_compose` and `run_inline_compose` return
  `Result<i32>`, and the `_inner` wrappers were folded in. `prep.rs` hands the
  `UserInterruptGuard` to `shutdown::hold_interrupt_guard` at install time,
  so it survives a `?` error return as well as a normal return (R4).
  `run_sequence` returns the code, and the budget-ledger wrapper is unchanged,
  so `76`, `77`, and the `130`-versus-exhausted mapping are unaffected.
- **Provider wrappers.** A private `WrapperOutcome { AgentExited{..},
  NotLaunched, NoModel }` replaces the tuple, so the no-model branch returns
  instead of calling `exit(1)`. It keeps the old behavior: no second agent
  error report and no perf report. Dry run and abandoned `--edit` still give
  `0` with the perf report.
- **`handle`.** `deadline_at = Instant::now() + resolve_deadline()` is
  computed and passed to `shutdown::set_drain_deadline` **before**
  `run_inner`, and `timeout_at(deadline_at, …)` replaces `timeout`. `Ok` and
  the elapsed case flush and return the code (`124` on elapsed). The `Err`
  path is unchanged. The `## Exit discipline` doc was rewritten.
- **Drain interrupt ladder (amended Rule 8).** `interrupt.rs` gained
  `install_drain_interrupt_guard()`, which is the same ladder with a drain
  notice ("User interrupted while waiting for outbound messages; press
  Ctrl+C again to exit now"). On Unix its press counter starts at 1 when a
  Ctrl+C was already observed during the run, so that user's next press
  force-exits. On Windows this already follows from the coordinator's
  process-wide press count. `install_user_interrupt_guard` now delegates to
  the shared `install_ladder`, and its behavior is unchanged.
- **Library.** `claudine::messaging::has_pending_deliveries()` returns whether
  any tracked task is still unfinished. This was the accessor Phase 2's
  message asked for. Unit test:
  `pending_deliveries_are_seen_until_they_finish`.
- **Reproduction.** Removed the `#[ignore]` attribute from
  `compose_success_message_is_delivered_before_exit`, and changed nothing
  else. It now passes on macOS, Linux, and Windows.

### Departures from the plan

- **The exit guard has no entry for `wrap/exec/termination/windows.rs`.** The
  `process::exit(2|3)` text there is inside an `r#"…"#` literal, which
  `sanitize` blanks, so the file has no live site. An entry would trip the
  required stale-entry check. The allowlist is instead **exact per file,
  with site counts** (`shutdown.rs` 2, `main.rs` 1, `commands/compose/interrupt.rs` 5),
  so a new exit in an allowlisted file fails too. The plan records this
  amendment. The detector also catches `use std::process::exit;`, so an
  import-then-bare-`exit(…)` cannot slip past.
- **`ShutdownHold` is not a separate type.** The only thing ever held is the
  compose `UserInterruptGuard`, so `hold_interrupt_guard` takes it directly
  (Rule 2: no single-use abstraction). Registering at install time, instead
  of returning the guard with the code, also covers error returns, which a
  returned `(i32, ShutdownHold)` could not do.
- `render_top_level_error` stays in `main.rs` and is called from both
  `async_main` and `main`, because a pre-runtime error still needs it.

### Verification

- Reproduction, fixed code (macOS): `compose_success_message_is_delivered_before_exit`
  passed in 1.9 s. The child stayed alive while the reply was withheld,
  exited `0` after release, and exactly one POST carried the `success` text.
- The guard is load-bearing: appending `fn _planted() { std::process::exit(9) }`
  to `commands/sequence.rs` failed `direct_exits_occur_only_at_allowlisted_sites`
  with `direct exit outside the shutdown path: commands/sequence.rs:589 (process::exit)`.
  The file was restored. On the unfixed tree the guard would have flagged
  `compose/mod.rs`, `sequence.rs`, `wrap/mod.rs` (×2), `handle.rs` (×2),
  and `main.rs` (a count of 2 against 1).
- Checkpoint suites: `handle_deadline`, `handle_blocking_output`,
  `compose_cli`, `inline_compose_cli`, `sequence_cli`, and `sequence_budget`
  gave 49/49 on macOS. `sequence_ctrl_c_windows` and `handle_deadline`
  (including the `124` path) pass on Windows native.
- Wall-time spot check (macOS, debug builds, stub `claude`, no route,
  hyperfine `-N`, 3 warm-ups, 30 runs): this tree 216.9 ± 7.9 ms against
  `HEAD` 217.6 ± 6.1 ms for `claudine compose doc.md`. `claudine handle stop`
  was about 0.36 s on both. `HEAD` was built in a temporary detached
  worktree, which has since been removed.
- `just lint` (claudine): exit 0. The only warning is the pre-existing macOS
  linker `__eh_frame` notice.
- `just test` (claudine): 7411 passed, 9 skipped. The skip count fell by one
  from Phase 2's 10 because the reproduction now runs.
- Cross-OS:

| OS | Command | Result |
| --- | --- | --- |
| macOS (local) | `just test` | pass |
| Linux (`build-linux`) | `just cross-check claudine-cli --os linux --features test-fixtures lifecycle_message_drain exit_site_guard handle_deadline handle_blocking_output shutdown:: interrupt::` | 28/28 |
| Windows native (`build-win-native`) | `just cross-check claudine-cli --os windows --features test-fixtures lifecycle_message_drain exit_site_guard handle_deadline sequence_ctrl_c_windows shutdown:: interrupt::` | 28/28 (the reproduction included) |
| Linux / Windows | `just cross-check claudine --os … delivery::` | 14/14 each |

- The Windows build compiles the `cfg(not(unix))` side of `install_ladder`
  and `Mutex<Option<UserInterruptGuard>>` (`Send`: both Windows fields are
  unit or `PhantomData` guards).
- Pre-existing, not from this phase: the Windows build warns
  `constant GENERATED_MARKER is never used` in
  `cli/tests/l1/sequence_initialize_include_preflight.rs:23`.

### Docs

- `docs/topics/messaging.md`: the **Planned** CLI-drain paragraph became
  "The CLI drains before every ordinary exit" (budget, `handle` cap, `124`,
  unchanged exit code, no cost when nothing was sent, Ctrl+C behavior, and
  the warning text). The Mermaid diagram and the stale 3-second bullet are
  left for Phase 5.
- `docs/topics/signal-handling.md`, "Hook handler deadline": drift found and
  fixed. The section said the deadline was checked "at each phase boundary"
  and ended in `_exit(124)`. In fact it is a Tokio timer, and the code is
  returned. The section now also states that the drain is capped by the same
  deadline.
- `docs/pipeline.md`, G2 and G4: "Drop SIGINT guard / RAII restores prior
  handler" and "`std::process::exit(code)`" were replaced with the hand-over
  of the guard and the drain-then-exit step.
- Skill `hook-actions.md`: "The CLI's drain on exit is planned" now names
  `shutdown::finish` and the exit guard.

### Requirement-to-test mapping (Phase 3)

| Requirement | Test |
| --- | --- |
| R6 reproduction passes unchanged on fixed code (R2 for compose `success`) | `claudine-cli::l1 lifecycle_message_drain::compose_success_message_is_delivered_before_exit` (macOS, Linux, Windows) |
| R2 exit-site guard, both directions, exact allowlist | `exit_site_guard::{direct_exits_occur_only_at_allowlisted_sites, a_site_in_an_unlisted_file_fails, an_extra_site_in_a_listed_file_fails, an_entry_with_no_live_site_fails, every_allowlist_entry_states_a_reason}` |
| R2 detector self-tests (forms caught; comments, strings, raw strings, `err.exit()`, `force_exit`, `exit_code` ignored) | `exit_site_guard::{the_detector_finds_every_exit_form, the_detector_ignores_comments_strings_and_similar_names}` |
| R3 budget and `handle` cap (min of the two; a passed deadline stays past) | `claudine-cli::bin/claudine shutdown::tests::{the_drain_budget_applies_when_no_command_set_a_deadline, an_earlier_command_deadline_caps_the_drain, a_later_command_deadline_does_not_extend_the_budget, a_passed_command_deadline_is_kept_in_the_past}` |
| R3 / Rule 8: only a running delivery installs the drain ladder | `claudine messaging::delivery::tests::pending_deliveries_are_seen_until_they_finish` |
| R3 exit codes unchanged: `handle` `124`, budget `76`/`77`, compose/sequence codes | existing `handle_deadline::handle_exits_on_deadline`, `sequence_budget::*`, `compose_cli::*`, `sequence_cli::*` (green) |
| R3 "no slower when nothing is sent" | hyperfine spot check above (not an automated test; Phase 4 does not add one either) |
| R4 second Ctrl+C during the drain | **Phase 4** (`Second Ctrl+C during drain`); the ladder rungs themselves are pinned by the existing `interrupt::tests::*` |

- Tier placement: `exit_site_guard.rs` is declared in `cli/tests/l1/main.rs`
  with no tier marker. It reads `manifest_dir!().join("src")`, which is the
  form CI recognizes. The `shutdown::tests` unit tests compile into the bin
  target. `test_placement` and `spawn_site_guard` stay green inside
  `just test`.

### Notes for Phase 4 and Phase 5

- Stale doc claims found but outside this phase: `.claude/skills/claudine/unified-hooks.md:532-534`,
  `cli-reference.md:282`, and `architecture.md:853` say the `handle` deadline
  defaults to **5 s** (the code default is 15 s) and repeat the "3 s messenger
  timeout". Phase 5's skill pass names only `hook-actions.md`, `SKILL.md`,
  and `architecture.md`, so add `unified-hooks.md` and `cli-reference.md`.

## Phase 4

- Started and finished 2026-09-27. Test-only phase: no production code
  changed.

### What landed

- **`common/webhook_listener.rs`.** `write_config_with_webhook_route(home, json)`
  writes the drain-test route next to other config keys, such as hook
  `actions`. `write_webhook_route` now delegates to it.
- **`l1/lifecycle_message_drain.rs`.** A `PipedRun` helper spawns
  `claudine` with piped output. It streams stderr into a shared buffer so a
  test can wait for a line while the child is alive. Every wait is bounded,
  and dropping the helper kills a child that is still running. The file also
  adds `pending_warning(stderr)`, which undoes the Status line's word wrap,
  plus five tests (see the mapping below).
- **`l1/handle_message_drain.rs` (new).** Three `claudine handle session_end`
  cases driven by a hook config: delivered, stalled under a 2 s deadline, and
  a handler past its deadline. In the last case the actions are
  `[message, call sleep 8]` (`ping -n 9` on Windows). Actions run in order,
  and a `call` is awaited under a Tokio timeout, so the handle deadline fires
  while the message is still in flight.
- **`l1/lifecycle_message_drain_interrupt.rs` (new).** R4 for `compose`
  (compose guard held through the drain) and `sequence` (drain ladder
  installed by `finish`). Unix uses `common::signal::SignalledRun`. Windows
  uses a local `CREATE_NEW_PROCESS_GROUP` + `CTRL_BREAK_EVENT` runner with
  file-captured output. Neither opens a window.
- `docs/topics/testing.md`: the Messaging fixtures section now covers the new
  helper, the silent desktop-notification seam, and the `slow_` trap below.

### Findings and departures

- **`slow_` would strand the stalled-budget test.** The plan says to name it
  `slow_…`. Claudine does not declare `l1-include-slow` (only darkmatter's
  packages do), so every recipe and CI leg filters `slow_` out, and
  `check-tier-coverage` does not flag it. `wrap_sigint.rs` already records
  this trap. The test keeps an ordinary L1 name, runs in about 10.4 s (inside
  nextest's 30 s terminate limit), and is amended in `plan.md`.
- **"Two stalled deliveries" uses one route.** Only one messaging route is
  active per scope (`resolve_effective_route`), and the repo scope overrides
  the user scope rather than adding to it. So the two deliveries are a
  `success` and a `finalize` message on the same route. The warning reads
  `Route drain-test, route drain-test were still sending at exit; delivery is unknown`.
- **Top-level error trigger.** A `success` stack that calls an unknown
  function (`{{ drain_test_unknown_fn() }}`) raises a lifecycle evaluation
  error after `start` has sent. The error goes through `?`, renders, and
  exits `1`. The test asserts the error block is on stderr *while* the
  child is still waiting for the withheld `start` reply, so the drain runs
  after the render.
- **Sequence messaging needs an agent step.** This confirms Spike A: the
  test uses a two-step agent sequence. `success` fires per step, and
  `{{ state.name }}` distinguishes them.
- **Terminal `notify` (Spike C ruling applied).** The CLI test covers the
  *failed* path on all three OSes: the existing "Failed to send desktop
  notification" warning, exit `0`, and no pending warning. It stays silent
  through `fake_only_path()` on Unix (on this Mac `osascript` is in
  `/usr/bin`, which the default fixture `PATH` includes), a dead D-Bus
  address for Linux, and the missing AppUserModelID on Windows. The
  *unfinished* notification label is pinned by the existing library tests
  `delivery::tests::stalled_deliveries_share_one_deadline_and_are_aborted`
  (a `DesktopNotification` entry is pending and aborted) and
  `the_pending_warning_names_each_delivery_and_says_delivery_is_unknown`.
  No new library test was needed.
- **The R4 test is load-bearing.** With `finish`'s drain-ladder install
  disabled (`false && has_pending_deliveries()`), the sequence case failed:
  the first press was swallowed, so no notice appeared. The compose case
  still passed because it uses its own held guard. `shutdown.rs` was
  restored with no diff.
- No production defect was found. The R5 report never contained the
  dummy token.

### Verification

| OS | Command | Result |
| --- | --- | --- |
| macOS (local) | `cargo nextest run -p claudine-cli --features test-fixtures --test l1 -E 'test(message_drain)'` | 16/16 |
| Linux (`build-linux`) | `just cross-check claudine-cli --os linux --features test-fixtures lifecycle_message_drain handle_message_drain` | 16/16 |
| Windows native (`build-win-native`) | `just cross-check claudine-cli --os windows --features test-fixtures lifecycle_message_drain handle_message_drain` | 16/16, no warnings in the new files |

- `just test` (claudine): 7421 passed, 9 skipped. That is 10 more than
  Phase 3's 7411, and all 16 drain tests ran in L1. `test_placement`,
  `spawn_site_guard`, and `exit_site_guard` pass.
- `just lint` (claudine): exit 0. The only warning is the pre-existing macOS
  linker `__eh_frame` notice.
- `just check-tier-coverage claudine`: no stranded tests.
- Not run in this phase: `just test-l2` (a Phase 5 final gate) and WSL2
  (nightly CI leg). No test here is WSL-specific.

### Requirement-to-test mapping (R6, complete)

All tests are `claudine-cli::l1` and pass on macOS, Linux, and Windows
unless noted.

| R6 bullet | Test |
| --- | --- |
| Reproduce first / delivered before exit, exactly one POST with the `success` text | `lifecycle_message_drain::compose_success_message_is_delivered_before_exit` (control: `compose_start_message_reaches_the_listener_during_the_run`) |
| `400` → "Failed to send lifecycle message", exit `0` (R5), no token | `lifecycle_message_drain::a_rejected_success_message_is_reported_and_the_exit_code_stays_zero` |
| Never-reply → exit within the 10 s budget, R3 warning, exit `0`, shared budget, no token or body in the warning | `lifecycle_message_drain::stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown` |
| `handle` hook message delivered before exit | `handle_message_drain::a_hook_message_is_delivered_before_the_handler_exits` |
| `handle` stalled send uses only the rest of the deadline | `handle_message_drain::a_stalled_hook_message_waits_only_for_the_rest_of_the_handler_deadline` |
| `handle` timed out still exits `124`, pending reported without extra wait | `handle_message_drain::a_handler_past_its_deadline_exits_124_and_reports_the_pending_message` |
| `sequence` last step's `success` message received | `lifecycle_message_drain::sequence_last_step_success_message_is_delivered_before_exit` |
| Terminal `notify` tracked and drained; failed → existing warning; silent | `lifecycle_message_drain::a_terminal_notify_is_drained_and_its_failure_reported_without_host_ui` |
| Unfinished notification → safe `desktop notification` label | `claudine` lib `messaging::delivery::tests::{stalled_deliveries_share_one_deadline_and_are_aborted, the_pending_warning_names_each_delivery_and_says_delivery_is_unknown}` |
| Top-level error after a send still drains, exit `1` | `lifecycle_message_drain::a_top_level_error_after_a_send_still_drains_and_exits_one` |
| Second Ctrl+C during the drain → forced exit `130` (macOS, Linux, Windows) | `lifecycle_message_drain_interrupt::{a_second_ctrl_c_during_the_compose_drain_exits_130, a_second_ctrl_c_during_the_sequence_drain_exits_130}` |
| R2 exit-site guard | `exit_site_guard::*` (Phase 3) |
| R1 tracker guard for `messaging/` | `claudine` lib `l1 messaging_spawn_guard::*` (Phase 2) |

### Notes for Phase 5

- Nothing in this phase changes the Phase 5 doc list. The Phase 3 note about
  stale 5 s / 3 s claims in `unified-hooks.md` and `cli-reference.md` still
  stands.
- `docs/topics/testing.md` already describes the drain fixtures, so the
  Phase 5 messaging doc can link there for the test seams.

## Phase 5

- Started and finished 2026-09-27. This phase changed documentation only. No
  source code changed.

### What landed (R7)

- `docs/topics/messaging.md`: removed the false claim that hook messages
  have a 3-second timeout set by `CLAUDINE_MESSENGER_TIMEOUT_SECONDS`. That
  variable does not exist, and a hook message has no timeout of its own. Its
  send is bounded only by `handle`'s overall deadline, through the exit
  drain. Added a Mermaid sequence diagram (send → track → command returns →
  drain → warn, flush, exit with the same code) and a link to
  `testing.md#messaging-fixtures` for the test seams. The drain rules (10 s
  shared budget, `handle` cap, unchanged exit code, warning text, and the
  library's opt-in `drain_deliveries` contract) were written in Phases 2
  and 3 and were checked again against `shutdown.rs` and `delivery.rs`.
- `docs/topics/flow-control/lifecycle.md`: added one paragraph after the
  Notification Fields table. A terminal-event `message`/`notify` finishes, or
  is reported, before an ordinary exit, within 10 s. A timeout leaves
  delivery unknown, and the exit code does not change.
- `docs/topics/signal-handling.md`: added a new subsection, "Ctrl+C during
  the exit drain". It covers the held compose guard, the drain ladder with its
  notice text (checked against `interrupt.rs:451`), and the rule that a
  second press, or the first after an earlier one, exits `130`. It has a
  state diagram. **Drift fixed:** "The compose-scoped guard" still said the
  guard is installed in `run_compose_inner`/`run_inline_compose_inner` and
  removed by `Drop` when the subcommand ends. Since Phase 3 it is installed in
  `run_composition_inner` and held by `shutdown::hold_interrupt_guard` until
  exit.
- Additional stale claims found by the Wave 9 grep, beyond the plan's list,
  and fixed:
  - `docs/topics/configuring-actions.md`: the intro and the `message` action
    said messaging was "fire-and-forget". The `message` action now describes
    the drain.
  - `lib/README.md`: the `Message` row said "Fire-and-forget (tokio::spawn)",
    and the lessons bullet claimed a 5 s deadline plus 3 s timeouts.
  - `cli/README.md` and `docs/topics/building-an-agent-wrapper.md`: the same
    5 s / 3 s claim.
- `claudine` skill:
  - `hook-actions.md`: changed the intro wording. The `message` action now
    says "No per-action timeout" instead of the false 3 s claim. **Drift
    fixed:** the `bash` action said it was "Fire-and-forget (`tokio::spawn`)"
    with an override variable, `CLAUDINE_BASH_ACTION_TIMEOUT_SECONDS`, that
    does not exist. The code awaits it inline with a fixed
    `BASH_ACTION_TIMEOUT` of 3 s.
  - `architecture.md` → Key Lessons: fixed the deadline (15 s, not 5 s) and
    added two bullets, one on the delivery tracker and one on the single
    ordinary-exit path, which records the exit-guard allowlist and the reason
    the process must exit inside the runtime.
  - `SKILL.md`: added a paragraph saying that messages are **not**
    fire-and-forget and naming both guards. Audio wording is unchanged.
  - `unified-hooks.md` and `cli-reference.md`: fixed the 5 s / 3 s claims
    (the Phase 3 note).
  - `timeline.md`: added a 2026-09-27 entry and bumped `last_updated`.
- The skill files' stored `hash:` values were **not** restamped. They
  already differed from `md hash` at `HEAD`, so nothing checks them, and
  restamping would be unrelated churn.
- Remaining "fire-and-forget" hits describe audio, the `FireAndForget`
  action, provider-side hook protocols, and wrapper session reports. None
  of them describes messaging, and all are correct. The 2026-04-14 timeline
  entry keeps its original 5 s / 3 s wording because it is historical.

### Final gates

| Gate | Result |
| --- | --- |
| `just lint` (claudine) | exit 0 |
| `just test` (claudine) | 7421 passed, 9 skipped (the same as Phase 4) |
| `just test-l2` (claudine) | 243 passed (claudine-cli level2) + 3 passed; exit 0 |
| Grep for messaging "fire-and-forget" / 3 s / 5 s in docs, READMEs, skill | clean, except the historical timeline entry |

- No cross-OS run was needed in this phase because no code changed. The
  Phase 3 and Phase 4 evidence for macOS, Linux, and Windows still applies.
  WSL2 is the nightly CI leg.

### Whole-fix summary

- **Reproduction:** `compose_success_message_is_delivered_before_exit` was
  written first. It was `#[ignore]`d on the old code, where the child exited
  before the withheld reply (Phase 1 log), and passes unchanged on the fixed
  code (Phase 3).
- **Spikes:** see Phase 1 (Spikes A–C), including the ruling that terminal
  `notify` is tracked.
- **Rule amendments and departures:** amended Rule 8 (the drain ladder is
  installed only when a delivery is running and no compose guard is held);
  the exact per-file exit allowlist with site counts and no
  `termination/windows.rs` entry (Phase 3); no separate `ShutdownHold` type
  (Phase 3); the stalled-budget test is not named `slow_`, and "two stalled
  deliveries" share one route (Phase 4); the extra stale-doc fixes above
  (Phase 5).
- **R6 test map:** complete in Phase 4 → "Requirement-to-test mapping
  (R6, complete)".
