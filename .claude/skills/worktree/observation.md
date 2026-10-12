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
  `QueueFull`, `WatchLost { root }`, `ReadFailed`, `DrainTimedOut`) is returned by every later
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
- **macOS (`fsevents.rs`)** runs one FSEvents stream (`fsevent-sys`, file
  events, 0.1 s latency) on a run-loop thread feeding `EventQueue`. It watches
  the original, quarantine, and admin paths plus a private marker directory
  (`<confstr(_CS_DARWIN_USER_TEMP_DIR)>/wt-observe-<hex>`), not the parent:
  FSEvents is recursive, so a parent watch would carry every sibling's
  activity. Facts measured in Phase 7 that shape it:
  - **`FSEventStreamFlushSync` alone is not a barrier** (1 of 50
    write-then-flush rounds missed the write). The drain writes a marker,
    flushes every 1 ms until the marker's event arrives (about 11 ms idle),
    and `DRAIN_TIMEOUT` (5 s) without it is `CoverageLost::DrainTimedOut`.
    Works with the checkout on another volume than the marker.
  - **`FSEventStreamStart` returns before events flow**; `start` runs one
    barrier before returning, or a change right after it is missed.
  - **A directory's removal or rename hides pending child events** in the
    same batch (even at latency 0). A caller must drain between changing a
    directory's children and removing or renaming the directory itself; the
    shared tests do that through `settle` where `COALESCES_BELOW_DIRECTORY`.
  - Parent and admin moves or deletions produce no event under the watched
    paths, so each drain compares their `(dev, ino)` with start.
  - Flags are cumulative history: one event yields one `EventKind` per flag,
    and a rename's direction is inferred from whether the path exists when
    read.
  - Events arrive with resolved paths (`/private/var/...`, on-disk case);
    `Paths::resolve` canonicalizes the watched paths and reports parent
    entries under the session's own names.
  - fseventsd holds the watch, so `handles()` is empty and Sniff sees nothing
    (`fsevents::tests::sniff_sees_nothing_of_the_session`).
  - Stream registration is serialized by fseventsd: concurrent test streams
    each took up to 5 s to start, so the macOS native tests run in the
    nextest group `fsevents-l1` (`max-threads = 1`, `.config/nextest.toml`).
    Name a new native test `native_*` in `observation/tests.rs` or put it in
    `fsevents/tests.rs` so the group's filter selects it.
- **Windows (`read_directory_changes.rs`)** opens two overlapped
  `ReadDirectoryChangesW` handles, the parent with the subtree flag (1 MiB
  buffer, 64 KiB on a share that refuses more) and the admin directory, read by
  one thread `wt-read-directory-changes` feeding `EventQueue`. Facts that
  shape it:
  - Paths arrive relative to the parent; an entry matching the tree's current
    name (ASCII case-insensitively) is `Root::Tree`, siblings are dropped
    before any work, and parent entries carry the session's own names.
  - The drain is a marker per watched directory: a hidden file
    `.wt-observe-<hex>-<n>` opened with `FILE_FLAG_DELETE_ON_CLOSE`, so a
    crash leaves nothing. Delivery is FIFO per handle; `DRAIN_TIMEOUT` (5 s)
    without the marker is `DrainTimedOut`. `start` runs one barrier.
  - A zero-byte completion and `ERROR_NOTIFY_ENUM_DIR` are `Overflow`; any
    other completion error, or a failed reissue, is `WatchLost { root }`. The
    session's own cancel (release, drop) is never read as a completion.
  - Every drain also compares the parent's and admin directory's
    `quarantine::rename::identity` with start, as on macOS.
  - `release_admin` runs a barrier, then the thread cancels, waits for, and
    closes the admin handle before the call returns.
  - `FILE_ACTION_MODIFIED` is always `Modified` (never `Metadata`); a move
    between directories is `Removed` plus `Created`; on ReFS a replacing
    rename is `Removed` plus `Modified` and a POSIX-semantics directory delete
    is not reported at all ([os skill](../os/windows.md#directory-change-notifications)).
    The shared tests encode these with `METADATA_IS_DISTINCT` and
    `MOVE_ACROSS_DIRECTORIES_IS_A_RENAME`.
  - Neither handle is inside the checkout, so `handles()` is empty and Sniff
    does not refuse (`read_directory_changes::tests::sniff_sees_nothing_of_the_session`).
  - The overflow test holds a test-only `gate` mutex the thread takes before
    each completion, with a 4 KiB buffer, so the system's buffer overflows
    deterministically.
  - The shared tests' fixtures are on NTFS `C:`; to check ReFS on
    `build-win-native`, temporarily root `Layout::new` under
    `env!("CARGO_MANIFEST_DIR")` (`B:`) and run `just cross-check worktree
    --os windows native_`.
- `Sink::push` and its `targets` field carry `expect(dead_code)` off Linux
  (macOS and Windows feed `EventQueue`, not the sink), so an adapter that
  pushes into the sink must widen it.
- **Test seams:** `Observation::injected(targets, options)` returns an
  `Injector` (`event`, `delayed`/`deliver_delayed`, `overflow`, `lose_watch`,
  `hold_handle`) feeding the same `EventQueue` a threaded adapter uses.
  Linux kernel signals are fed through `Inotify::translate` in
  `inotify/tests.rs`; FSEvents flags through `Shared::deliver` in
  `fsevents/tests.rs`. A real `IN_Q_OVERFLOW` needs `max_queued_events` ≤ 32,768:
  WSL2 has 16,384 and runs it; build-linux (8,388,608) skips it.
