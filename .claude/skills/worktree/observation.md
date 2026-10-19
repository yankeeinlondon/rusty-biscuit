# Activity observer (`remove::observation`)

The watcher `wt remove` runs from before the quarantine move until the
removal ends, and the deletion inventory and checked deletion engine built on
it (`2026-10-05-graceful-file-contention`, Phases 6-9). User-facing behavior
is in `worktree/docs/cli/remove.md`.

`Observation::start(Targets::new(original, quarantine, admin)?, options, interrupted)`
watches the parent (only the original's and the quarantine's names), every
directory of the tree, and the admin files in `ADMIN_FILES` plus
`sharedindex.*`. `drain(interrupted)` returns a
`Drain { events, lost, interrupted }`: events carry a
`Scope` (`Tree(rel)` wherever the tree now is, `Original(rel)`,
`Quarantine(rel)`, `Admin(name)`) and a receipt time. Events are facts; the
deletion engine decides which are the operation's own.

- **Lost coverage is sticky.** The first `CoverageLost` (`Overflow`,
  `FsEventsDropped { raw, flags }` on macOS, `QueueFull`, `WatchLost { root }`,
  `ReadFailed`, `DeliveryStalled`, `DrainCeiling`) is returned by every later
  drain. Events received before the loss are still returned, for the report.
  `CoverageLost` is serialized into `report.json` (tagged by `kind`, durations
  as `waited_ms`), so changing a variant changes the report's shape.
- **Diagnostics.** `Observation::diagnostics()` (and `diagnostics_handle()`,
  which outlives the session) holds one `ObservationDiagnostics` per attempt:
  backend, `startup_ms` (set by `Observation::start`), last and longest
  confirmed marker round trip (`Outcome::Confirmed { round_trip }` from
  `await_marker`, kept by each adapter's `Backend::round_trips`), and the
  first loss with the `ObservationStage` of the drain that received it. The
  stage follows `moved` until the deletion engine and unregistration call
  `enter_stage`; the engine drains through `drain_after(entries)`, which
  records `entries_since_drain`. A startup confirmation that loses coverage is
  `ObservationError::NotDelivering(diagnostics)` (still exit 4), not `Setup`.
  The lifecycle's `Attempt::observed_by` copies the snapshot into every saved
  report.
- **Register before the usage query.** `register_handles(&mut OwnedHandles)`
  must run before `check_usage`. On Linux, Sniff reports each watch inside the
  checkout as `WatchRegistration` with the inotify instance's fd as
  `descriptor`; unregistered, `wt` refuses its own removal (pinned by
  `inotify::tests::sniff_sees_the_watches_until_the_session_ends`).
- **Call `moved_to_quarantine(interrupted)` right after the rename** and
  `release_admin(interrupted)` before the step that unregisters the worktree.
  Without the release, Git deleting the admin directory is
  `WatchLost { Admin }`. An interrupted release returns `WaitInterrupted` and
  keeps the watch; unregistration then stops (`UnregisterError::Interrupted`).
- **Every wait for delivery runs under `ObserverOptions::wait`
  (`WaitPolicy`, default 30 s stall / 5 min ceiling) and polls the
  interrupt.** `wait.rs` owns it: `WaitState` keeps one stall clock per
  awaited key plus one ceiling, and `await_marker` reads a `Source` (the
  adapters' `Delivery`: highest marker and in-scope event count per key, a
  generation, a condvar) and returns `Confirmed | Lost | Interrupted |
  Stalled | Ceiling`. Progress is only a higher marker or an in-scope event
  (`EventQueue::push` returns whether the event classified); siblings,
  unreported admin files, and a marker's own cleanup are not. The interrupt
  and recorded loss are polled every `POLL_INTERVAL` (50 ms). An interrupted
  wait records no loss: startup returns `ObservationError::Interrupted`
  (the lifecycle maps it to `Refusal::Interrupted`, exit 1, not exit 4), a
  drain sets `Drain::interrupted` (`confirm_unchanged` refuses before the
  move, the deletion engine and the post-move drain stop as
  `Stop::Interrupted`), and an interrupted `moved_to_quarantine` is reported
  by the next drain. Once coverage is lost, a drain does not wait again.
  Linux ignores both: reading until empty never waits.
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
  - **A flush alone is not a barrier** (1 of 50 write-then-flush rounds
    missed the write). The drain writes a marker and waits for its event,
    calling the non-blocking `FSEventStreamFlushAsync` every 1 ms
    (`FLUSH_INTERVAL`, via the `Flushing` source) while it waits; about
    11 ms idle. The blocking `FlushSync` is no longer used, so a wait can
    always check its deadlines and the interrupt. Flushing only once per
    round left each marker to the stream's 0.1 s latency and made the
    macOS native tests about 7x slower (`native_drain_delivers_every_change_kind`
    0.8 s → 6.4 s). Works with the checkout on another volume than the
    marker.
  - **One confirmed marker is not a barrier either** (2026-10-13).
    fseventsd does not keep the order changes were made in: in a burst of
    2,000 creations it gave a contiguous run of 1-9 of them event IDs
    *after* a marker created once all writes had returned, and delivered
    them in the next drain, with no drop flag (about 1 drain in 15, also
    with the test alone). A second `FlushSync` does not help. The drain
    therefore repeats marker rounds until one delivers no in-scope event
    (`Shared::delivery`'s count). The rounds share one `WaitState`: each
    round re-`expect`s the key (its stall clock restarts), and the ceiling
    spans every round, so a stream that keeps delivering ends as
    `DrainCeiling`, and silence as `DeliveryStalled { watch: None }`. 500 stressed
    bursts, 200 of them under full-suite load, then passed. It is still
    not a barrier: with all 11 FSEvents tests stressed at once, a quiet
    round returned with 240 of 2,000 creations pending. The macOS contract
    is therefore "delivered so far, settled": late events arrive in a later
    drain, never dropped without `CoverageLost`, and the per-entry checks
    before unlink (inode, size, ns mtime, mode, BLAKE3) and unregistration's
    index/HEAD reread are what protect work. The burst test asserts eventual
    delivery, and the shared adapter tests use `drain_until`
    (`DRAIN_IS_A_BARRIER` is false only on macOS): Linux and Windows must
    show each change in the very next drain, macOS within `LATE_DRAINS`
    later ones, with no loss. Even a single write was seen to miss a quiet
    round. A true barrier would need kqueue (an fd per directory and file). To diagnose, record the callback's event IDs and compare a late
    path's ID with the marker's.
  - **`FSEventStreamStart` returns before events flow**; `start` runs one
    barrier before returning, or a change right after it is missed.
  - Tests hold delivery with `Shared::held` (the callback spins while set;
    `FsEvents::start_held` sets it from the start, and `Drop` clears it) and
    shorten `adapter.policy` after start, so a stall, the ceiling, and an
    interrupt are provoked against a real stream in under 1.5 s each.
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
    Name a new native test `native_*` (any `remove::…` module) or put it in
    `fsevents/tests.rs` so the group's filter selects it.
- **Windows (`read_directory_changes.rs`)** opens two overlapped
  `ReadDirectoryChangesExW` handles, the parent with the subtree flag (1 MiB
  buffer, 64 KiB on a share that refuses more) and the admin directory, read by
  one thread `wt-read-directory-changes` feeding `EventQueue`. Facts that
  shape it:
  - Paths arrive relative to the parent; an entry matching the tree's current
    name (ASCII case-insensitively) is `Root::Tree`, siblings are dropped
    before any work, and parent entries carry the session's own names.
  - The drain is a marker per watched directory: a hidden file
    `.wt-observe-<hex>-<n>` opened with `FILE_FLAG_DELETE_ON_CLOSE`, so a
    crash leaves nothing. Delivery is FIFO per handle. Each awaited handle
    (`Root::Parent`, `Root::Admin`) is its own `WaitState` key, so a busy
    parent never hides a silent admin handle; a stall is
    `DeliveryStalled { watch: Some(root) }` and the ceiling names the first
    handle still awaited. An ended watch is `Delivery::end`ed (marker
    `u64::MAX`) and never awaited. `start` runs one barrier. Tests hold
    completions with `Shared::held` (set from the start through
    `start_with_buffer(.., held)`), alongside the older `gate` mutex.
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
  - **Nothing inside the watched trees is opened**, not even to stat an
    entry: a handle inside the tree, however brief, makes the move fail
    with `ERROR_ACCESS_DENIED`. That once made
    `the_watch_does_not_block_the_move_or_the_deletion` fail about 1 in 100
    under parallel load ([os skill](../os/windows.md#directory-change-notifications)).
    Reads use `ReadDirectoryChangesExW` with the extended class, and an
    event's `directory` comes from `FileAttributes`. A volume that refuses
    the class (ReFS, with `ERROR_INVALID_FUNCTION`; maybe a share) is read in
    the plain class with `directory: None`
    (`an_entry_type_is_known_without_opening_the_entry`, NTFS only).
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
- **A resumed removal starts with the tree already moved:**
  `Observation::start_at_quarantine` (each adapter's `start` takes the tree's
  current location: inotify watches it, FSEvents resolves it, and
  `ReadDirectoryChangesW` requires it and maps the parent's entries from
  `moved = true`). Tests use `Observation::injected_at_quarantine`.
- **Test seams:** `Observation::injected(targets, options)` returns an
  `Injector` (`event`, `delayed`/`deliver_delayed`, `overflow`, `lose_watch`,
  `hold_handle`) feeding the same `EventQueue` a threaded adapter uses.
  `Observation::injected_confirming` also waits like an adapter: every drain,
  move labeling, and admin release awaits a marker through `await_marker` on a
  virtual clock that jumps to the next `Injector::at(time, Scripted::…)`
  delivery (`Acknowledge`, `Event`, `Overflow`, `Interrupt`) instead of
  sleeping, so a 30 s stall or 5 min ceiling is proven in milliseconds on
  every platform (`observation::tests`, the deletion engine's, the
  lifecycle's `Observers::confirming`, and unregistration's tests). Each
  `Acknowledge` confirms the wait in progress when it fires; schedule one per
  wait at distinct times, since every delivery due by a park fires at once.
  Linux kernel signals are fed through `Inotify::translate` in
  `inotify/tests.rs`; FSEvents flags through `Shared::deliver` in
  `fsevents/tests.rs`. A real `IN_Q_OVERFLOW` needs `max_queued_events` ≤ 32,768:
  WSL2 has 16,384 and runs it; build-linux (8,388,608) skips it.

## Deletion inventory and engine (`inventory.rs`, `deletion.rs`)

The order a caller follows (`remove::lifecycle` runs it; see
[lifecycle.md](lifecycle.md)): consent and `Inventory::consent_sensitive_paths()` first (they run
`git status`, which may rewrite the index), then `Observation::start`,
`DeletionInventory::capture(original, &sensitive)`, `confirm(original,
&observation.drain())`, `missing::bind_index`, `RecoveryStore::begin(lock,
record, Arc<inventory>)` (the inventory is journal line 2), the move,
`moved_to_quarantine()`, then `delete_content` and `unregister_quarantine`.

- **The move's own events reach the next drain:** `Original("")` moved away,
  `Quarantine("")` moved in, and `Admin("gitdir")` from the association
  rewrite. The engine explains the first two by kind and absence; an admin
  event is always activity to it, so the lifecycle must drain after the move
  and check the `gitdir` bytes before calling `delete_content`.
- **Compared fields:** identity, type, and for files size, mtime, permission
  bits (Windows: read-only/hidden/system), plus BLAKE3 for consent-sensitive
  files; a link's target. Never ctime or link count: deleting one Cargo hard
  link (`target/debug/<bin>` ↔ `deps/`) changes both on the other. Never a
  directory's mtime: the operation's own deletions change it.
- **Identity on Windows is by path** (`rename::identity`, 128-bit file ID),
  one handle open per entry in both capture and revalidation; a directory
  handle's identity uses `rename::handle_identity`. That costs about 0.5 ms an
  entry on build-win-native (600 files took ~1.2 s per test); keep Windows
  fixtures small.
- **Drains are per depth level** (plus every `DEFAULT_DRAIN_EVERY` entries:
  512 on macOS, 2,048 elsewhere), never per batch or entry: a macOS drain is
  ~11 ms. The interval is checked after each batch, so a drain can come one
  batch minus one entry late. 512 paces bursts that made FSEvents drop
  events at about 7,000 deletions a second; it cannot rule a drop out. Journal batches
  stay one directory, at most 256 entries. `.git` and everything under it are
  deleted after all other content.
- **Event resolution is by exact path, not kind:** a path the engine deleted
  must be absent, and its kinds must be `Removed`/`Metadata` (`Modified` too
  for a directory, which Windows reports when its children go). With
  `DeletionOptions::cumulative_kinds` (default on macOS) any kinds that
  include `Removed` (or, at the original, `MovedFrom`) are the deletion's
  history. A write-like kind otherwise is `Stop::Uncertain`. An approved entry
  not yet reached is re-checked against the inventory. Anything else is
  activity.
- **Found the hard way (macOS):** FSEvents reported the original path's move
  with a `Created` flag from the fixture's creation moments earlier, so the
  native tests stopped on their own move until `own_move` applied the
  cumulative rule there too.
- **Test seams:** `delete_content_with(.., before_batch)` acts at a known
  batch; `Injector::drains()` counts drains for the structural test.
  `test_support::link_dir` makes a directory symlink, or on Windows a
  junction when symlinks need Developer Mode. Controlled writers are `sh`/`cmd`
  children that read a line, act, and print `done` (`Writer` in
  `deletion/tests.rs`). The nextest group `fsevents-l1` selects every
  `remove::…::native_*` test.
