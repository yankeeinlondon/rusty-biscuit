# Activity observer (`remove::observation`)

The watcher `wt remove` runs from before the quarantine move until the
removal ends (`2026-10-05-graceful-file-contention`, Phases 6-9). User-facing
behavior is in `worktree/docs/cli/remove.md`.

`Observation::start(Targets::new(original, quarantine, admin)?, options)`
watches the parent (only the original's and the quarantine's names), every
directory of the tree, and the admin files in `ADMIN_FILES` plus
`sharedindex.*`. `drain()` returns a `Drain { events, lost }`: events carry a
`Scope` (`Tree(rel)` wherever the tree now is, `Original(rel)`,
`Quarantine(rel)`, `Admin(name)`) and a receipt time. Events are facts; the
deletion engine decides which are the operation's own.

- **Lost coverage is sticky.** The first `CoverageLost` (`Overflow`,
  `QueueFull`, `WatchLost { root }`, `ReadFailed`) is returned by every later
  drain. Events received before the loss are still returned, for the report.
- **Register before the usage query.** `register_handles(&mut OwnedHandles)`
  must run before `check_usage`. On Linux, Sniff reports each watch inside the
  checkout as `WatchRegistration` with the inotify instance's fd as
  `descriptor`; unregistered, `wt` refuses its own removal (pinned by
  `inotify::tests::sniff_sees_the_watches_until_the_session_ends`).
- **Call `moved_to_quarantine()` right after the rename** and
  `release_admin()` before the step that unregisters the worktree. Without the
  release, Git deleting the admin directory is `WatchLost { Admin }`.
- **Out-of-scope events are dropped before the bound counts them**
  (`Targets::classify`, used by `Sink` and `EventQueue`), so a busy sibling
  never fills the queue.
- **Linux (`inotify.rs`)** has no thread: a drain reads until `EAGAIN`. inotify
  queues an event inside the causing system call, so that read is the barrier;
  no marker file is written. A tree directory's `IN_IGNORED` is expected only
  after its own `IN_DELETE_SELF`; the parent's and admin directory's never
  are. Setup failures (`EMFILE`, `ENOSPC` at `max_user_watches`, a link at the
  tree root via `IN_DONT_FOLLOW | IN_ONLYDIR`) are `ObservationError::Setup`,
  exit 4.
- **macOS and Windows** return `ObservationError::Unsupported` until their
  adapters exist. The native contract tests in `observation/tests.rs` run
  wherever `start` succeeds and otherwise assert `Unsupported`; widen their
  `cfg!(target_os = "linux")` when an adapter lands. `Sink::push` and its
  `targets` field carry `expect(dead_code)` off Linux, which fails the lint
  once another adapter uses them, so remove it then.
- **Test seams:** `Observation::injected(targets, options)` returns an
  `Injector` (`event`, `delayed`/`deliver_delayed`, `overflow`, `lose_watch`,
  `hold_handle`) feeding the same `EventQueue` a threaded adapter uses.
  Linux kernel signals are fed through `Inotify::translate` in
  `inotify/tests.rs`. A real `IN_Q_OVERFLOW` needs `max_queued_events` ≤ 32,768:
  WSL2 has 16,384 and runs it; build-linux (8,388,608) skips it.
