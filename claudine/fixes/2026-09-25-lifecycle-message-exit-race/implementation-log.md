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
packages:
    - claudine-cli
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
