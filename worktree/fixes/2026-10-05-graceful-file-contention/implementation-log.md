---
spec: /Volumes/coding/wt/rusty-biscuit/feat-filesystem-watchers/worktree/fixes/2026-10-05-graceful-file-contention/spec.md
plan: worktree/fixes/2026-10-05-graceful-file-contention/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - worktree/fixes/2026-10-05-graceful-file-contention/spikes/README.md
skills_files_updated_during_phase_1:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
  - .claude/skills/os/wsl.md
  - .claude/skills/os/macos.md
  - .claude/skills/os/linux.md
  - .claude/skills/worktree/remove.md
packages:
  - worktree
---

# Implementation Log for 2026-10-05-graceful-file-contention (8 phases)

## Phase 1

Started and finished 2026-10-11. Phase 1 changes no package source: it records
the rulings, runs the three spikes, and adds the OS and Git facts they turned
up to the `os` and `worktree` skills.

### Hosts and evidence

Hosts declared on this machine (`env | grep '^BUILD_'`):

| Host | OS / filesystem | Git |
| --- | --- | --- |
| local | macOS, APFS (`/tmp` and `/Volumes/coding` are separate volumes) | 2.56.0 |
| `BUILD_LINUX=build-linux` | Linux 7.0, ZFS home, tmpfs `/tmp` and `/dev/shm` | 2.47.3 |
| `BUILD_WSL=build-win` | WSL2 (kernel 6.18), ext4 home, tmpfs, drvfs/9p `/mnt/b` | 2.53.0 |
| `BUILD_WIN=build-win-native` | native Windows, ReFS `B:` and NTFS `C:` | 2.55.0.windows.3 |

Every host was reached and every spike ran on each host the plan named, and
more. The spikes are throwaway programs, so they ran as scratch crates over
`ssh -o BatchMode=yes` rather than through `just cross-check`, which runs
package suites. Each remote scratch directory was deleted afterwards. The
probe sources and raw output are kept in `spikes/` (see its README) so a
reviewer can rerun them. One Windows S1 run was invalid: `cmd /c ping`'s
orphaned `ping.exe` kept its cwd inside the source directory for 3 s and
skewed every later case. That run is kept as `results/INVALID-run1-*`, and
the corrected rerun (`run2-windows.txt`) is the evidence used below.

### Rulings

| Ruling | Status | Outcome |
| --- | --- | --- |
| R1 Git association | Author decision, **pending confirmation**; proceeding on the spec's recommendation | Option 1, controlled deletion with registration retained. S3 sharpens it: never `git worktree move` (it replaces an empty directory); use a native no-replacement rename plus a checked rewrite of exactly the admin `gitdir`; the final unregister is a plain `git worktree remove` of the then-absent quarantine (details under S3) |
| R2 Local observation | Author decision, **pending confirmation**; proceeding on the spec's recommendation | Option 1, native notifications plus one inventory and targeted checks. Unusable observation refuses before mutation as an operational error |
| R3 Sniff latency contract | Author confirmation, **pending**; proceeding on the landed Option 1 | `query_path_usage` with `PathUsageOptions::default()` (the two-second shared budget, `DEFAULT_DEADLINE`). The budget stops scheduling work; a native call already running may overrun it. `PathUsageError` is matched by variant (`kind()` is the stable machine name), never by message text |
| R4 Watcher adapter | Decided from S2 | **Focused native adapters on all three OSes behind one internal observer type, not `notify`** (reasons under S2) |
| R5 No-replacement rename | Decided from S1 | Linux `renameat2(RENAME_NOREPLACE)`; macOS `renamex_np(RENAME_EXCL)`; Windows `SetFileInformationByHandle(FileRenameInfo, ReplaceIfExists = FALSE)`. Error mapping under S1 |
| R6 `git worktree move` | Decided from S3 | Not used. Native rename plus a checked rewrite of the admin `gitdir`, preserving the record's link format. Verified against Git 2.47.3, 2.53.0, 2.55.0.windows.3 (association reads only), 2.56.0 |
| R7 Record and journal | Confirmed | As planned: 128-bit `getrandom` (0.4, already a dependency) operation ID rendered as lowercase hex; `<common-git-dir>/wt-removal/<operation-id>/` holding the record, `report.json`, and the journal; owner-only permissions (0o700/0o600 on Unix; on Windows the `.git`-inherited ACL); temp file plus atomic rename with file and directory `fsync` at batch boundaries; intents fsynced before a batch and results after; a crash between an unlink and its result is `uncertain` on recovery; a completed operation keeps `report.json` and deletes the journal; only the held `fs4` lock (0.13, already a dependency) means an active operation. **Batch bound: one directory or 256 entries, whichever is smaller** (S2 found 256-event bursts deliver without drops on FSEvents; 2,000-event bursts do not) |
| R8 Existing checks | Confirmed | The early `check_no_processes` in `worktree/cli/src/commands/remove/mod.rs` `run` stays as the preliminary check that spares the questions. The `check_no_processes` and Windows `check_not_in_use` probe inside `remove_worktree` are replaced by the fresh Sniff preflight and the recorded quarantine move |
| R9 Exit codes | Confirmed by the spec | The spec's exit table verbatim; the typed outcome is kept separately from the exit code. S1 adds a Windows classification rule (below) |
| R10 No performance spike | Recorded | None scheduled. The S2 burst measurements below were capability questions (when do events drop?), not timing gates |
| R11 CI scope | Planning ruling, verified in Phase 8 | Unchanged. The `spikes/` folder holds no compiled source (`probe.rs.txt`), so it selects no CI cell |
| R12 Self-exclusion | Confirmed against Sniff's report shape | Sniff has no exclusion parameter; `wt` filters its own records out of the returned report. A record is the operation's own only when its `pid` is the current process, its `creation_token` (or `start_time`) matches the current process where Sniff supplies one, and the evidence names a handle the operation registered: an inotify `watch.watch_id` with that instance's `descriptor`, or a Windows directory handle by `identity`. FSEvents needs no descriptor (fseventsd holds the watch), so a macOS watch never shows as `wt`'s evidence. Windows' file-side API already omits the calling process |

### S1: atomic no-replacement rename (R5)

The probe moved a directory onto: an absent destination, an existing empty
directory, a file, a non-empty directory, and a dangling link. It also tried a
missing source, another volume, and a source that is in use.

| Case | macOS APFS | Linux ZFS, tmpfs, WSL ext4 | WSL drvfs (9p) | Windows ReFS and NTFS |
| --- | --- | --- | --- | --- |
| absent destination | moved; same `dev`/`ino` | moved; same `dev`/`ino` | **`EINVAL` (22)** | moved; same volume serial and 128-bit file ID |
| destination exists (empty dir, file, non-empty dir, dangling link) | `EEXIST` (17), both sides untouched | `EEXIST` | `EEXIST` | `ERROR_ALREADY_EXISTS` (183) |
| missing source | `ENOENT` | `ENOENT` | `ENOENT` | `ERROR_FILE_NOT_FOUND` (2), at the open |
| other volume | `EXDEV` (18) | `EXDEV` | n/a | `ERROR_NOT_SAME_DEVICE` (17) |
| parent not writable | `EACCES` (13) | `EACCES` | `EINVAL` | not reproducible (below) |
| in use (open file, child cwd, own watcher) | moved | moved | `EINVAL` | see below |

For contrast, `std::fs::rename` **replaced an existing empty directory** on
every OS, and on Windows it also replaced an existing file. That confirms the
plan: it is unusable for the quarantine move.

Windows in-use detail, from the corrected run. Every failure was
`ERROR_ACCESS_DENIED` (5) from `SetFileInformationByHandle`. Opening the
source with `DELETE` access always succeeded.

| Holder | Result |
| --- | --- |
| an open file anywhere in the subtree, with or without `FILE_SHARE_DELETE` | denied |
| a handle on a subdirectory | denied |
| a child process whose cwd is a subdirectory | denied |
| a handle on the directory itself (std default sharing) | moved |
| our own `notify` watcher on the directory, or a recursive one on its parent | moved |
| ACL deny of DELETE on the source, or of add-subdirectory on the parent | moved (the parent's delete-child right still allows it) |

Rulings that follow:

- **The primitive is supported on every local filesystem tested, except WSL
  drvfs.** There `RENAME_NOREPLACE` returns `EINVAL` (unsupported), even for
  the plain move. `EINVAL` from the move maps to an operational error *before
  mutation* ("this filesystem cannot move without replacement"): nothing is
  moved, and the checkout and registration stay. `EEXIST` is returned even
  on drvfs because the VFS checks the destination before the filesystem
  rejects the flag, so `EEXIST` never proves support. The consequence is a
  behavior change: a WSL checkout on `/mnt/<drive>` that `wt remove` used to
  delete will be refused. Phase 4 documents it, and Phase 8 discloses it.
  The considered fallback (exclusively create an empty placeholder, then
  `rename(2)` over it, so the only possible replacement is an empty
  directory swapped in at a random hidden name) is **rejected** as weaker
  than the contract. It is an author override if wanted.
- **Windows classification (amends R9's mapping).** A directory rename denied
  by an in-use subtree returns the same code (5) as an ACL denial, so the
  code alone cannot tell sharing from permission. The rule:
  - `ERROR_ACCESS_DENIED` or `ERROR_SHARING_VIOLATION` (32) from the
    rename call, after the `DELETE` open succeeded, is a **sharing failure
    before the move** (exit 4, nothing removed). Phase 4 runs a
    target-scoped Sniff query only to *annotate* it with PIDs.
  - The same codes from the `DELETE` open are a permission/operational
    error.
  - `ERROR_ALREADY_EXISTS` (183), `ERROR_NOT_SAME_DEVICE` (17), and
    anything else are operational.

  A genuine ACL denial could not be produced at the rename stage on this
  host, so a rare permission failure may be reported as "in use". The exit
  code (4) and "nothing was removed" are still true. This is recorded as a
  known limit, not a departure from the exit table.
- **No special privileges** were needed on any OS.
- **Windows identity:** use `GetFileInformationByHandleEx(FileIdInfo)`
  (64-bit volume serial plus 128-bit file ID). The legacy 64-bit index is
  not guaranteed unique on ReFS.
- The existing Windows rename-and-back probe is confirmed redundant: the
  quarantine move fails the same way, and when it does nothing has moved.

### S2: watcher capabilities (R4)

Candidate: `notify` 8.2.0, the maintained portable adapter. It is built on
`inotify` 0.11, `fsevent-sys` 4.1, and `windows-sys` **0.60** (the workspace
pins 0.61). Two layouts were tried on each OS: A watches the parent
non-recursively and the checkout recursively; B watches the parent
recursively. Both also watched the admin directory non-recursively.

| Question | Linux inotify | macOS FSEvents | Windows `ReadDirectoryChangesW` |
| --- | --- | --- | --- |
| Our own rename of a watched checkout (layout A) | notify emits `Name(From)` for the root, then **silently drops the watch** (a later `unwatch` gives `WatchNotFound`); writes in the quarantine produce **no events** | path-based: the quarantine is **unobserved** until a new watch is added | the handle follows the directory, but events carry the **old path** (`checkout\sub\after-move.txt`) |
| Layout B (parent recursive) | works, but means one inotify watch per directory of *every sibling worktree* | works; sibling events filtered in-process | works with correct paths; one handle on the parent; does not block the rename |
| Recreation of the original path | `Create(Folder)` seen; files written before notify re-adds watches on the new directory are missed (recursive-add race) | seen | seen |
| Admin dir `index.lock` → `index` rename and `sharedindex.*` | seen | seen | seen |
| Overflow surfaced? (backend blocked, 40,000 creates) | WSL (`max_queued_events=16384`): one `Rescan`; build-linux (8,388,608): no overflow | `Rescan` "user dropped" (45–88 times) | **silent**: 2 create events of 40,000, no rescan, no error |
| Unprovoked drops | none | **every 2,000-file burst dropped events** (`MustScanSubDirs`, "user dropped") with a non-blocking handler; no drops at 64–1,000 | none |
| Marker barrier (write N files, then a marker; wait for the marker) | marker after all N, ~1 ms | in 1 of 9 rounds at 256 files, **8 files written before the marker arrived after it**; at 2,000 files the marker itself was lost twice | marker after all N, 40–80 ms |
| Instance limit | WSL `max_user_instances=128`: watcher 121 fails at creation with `EMFILE` (an `Io` error, not `MaxFilesWatch`) | none hit at 300 (~19 ms per stream start) | none hit at 300 |
| Threads after drop | back to baseline | back to baseline (pool threads remain) | back to baseline |
| Watcher drop while its handler is blocked in `send` | not hit | **`drop` hangs forever** (the backend thread waits on the full channel) | not hit |

`notify` source confirms the Windows behavior. On a zero-byte completion (the
overflow signal), `handle_event` parses a zeroed buffer as action 0 and emits
nothing. On any completion error other than `ERROR_OPERATION_ABORTED` or
`ERROR_ACCESS_DENIED`, it logs, unwatches, and tells the handler nothing.

**R4 ruling: focused native adapters on all three OSes, behind one internal
observer type in `worktree/lib/src/remove/observation/`.** `notify` fails the
coverage contract on every OS, each in a different way:

- **Windows:** overflow and lost watches are silent.
- **Linux:** the checkout watch is dropped on our own move.
- **macOS:** no flush primitive, latency 0 with `NoDefer` drops events on
  bursts, and delivery is out of order relative to a marker.

Per-OS shape:

- **Linux: inotify** (the `inotify` 0.11 crate notify already uses, or
  `libc` directly). Watch descriptors follow inodes across our own rename, so
  the adapter relabels its paths when the operation reports its move,
  without a gap. It watches the parent non-recursively, the checkout
  recursively, and the admin directory. `IN_Q_OVERFLOW` and an unexpected
  `IN_IGNORED` are lost coverage. A new directory in the quarantine is
  already a stop condition, so the recursive-add race needs no scan. A
  drain is a marker in each watched root, read until `EAGAIN`; ordering is
  FIFO per instance.
- **macOS: FSEvents** through `fsevent-sys` 4.1. One stream covers the
  checkout and the quarantine paths (path-based, so it is not invalidated by
  the rename) plus the parent and admin directory. Use a nonzero latency
  with file events rather than notify's 0/`NoDefer`. A drain is
  `FSEventStreamFlushSync`, never a marker. `MustScanSubDirs` and either
  `Dropped` flag are lost coverage. Coalesced history flags mean event kind
  alone never classifies an operation-owned unlink: S2 saw a `Create` for a
  file reported only when we deleted it.
- **Windows: `ReadDirectoryChangesW`** through `windows-sys` 0.61 (workspace
  version). It watches the parent with the subtree flag (one handle; S1
  shows it does not block the move) and the admin directory. A large buffer
  is used for local volumes. A zero-byte completion, `ERROR_NOTIFY_ENUM_DIR`,
  or any completion error other than the operation's own cancel is lost
  coverage. A drain is a marker, reliable because delivery is FIFO per
  handle. The admin-directory handle is closed before the final unregister.
  The spike did not test whether an open watch handle leaves a deleted
  directory delete-pending; it showed only that `remove_dir` of a watched
  quarantine root succeeds. Phase 5 verifies this.

Every adapter:

- uses a bounded queue fed by a **never-blocking** handler (`try_send`; a
  full queue is lost coverage);
- turns a creation failure (for example `EMFILE` at the WSL instance limit)
  into an operational error before mutation;
- leaves no thread once the session ends;
- registers its handles for R12's self-exclusion.

The cost is three small adapters instead of one dependency. It is
accepted because the plan's R4 already allowed "focused native adapters
where `notify` cannot surface overflow/lost-watch or rename invalidation",
and S2 shows `notify` cannot do so on any of the three. Candidate dependencies
for Phase 5 (both are what `notify` itself builds on; checked in the local
registry):

- `inotify` 0.11.5, ISC, `hannobraun/inotify-rs`. Linux only; `libc`
  directly is the dependency-free alternative.
- `fsevent-sys` 4.1.0, MIT, `octplane/fsevent-rust`. macOS only; it exposes
  `FSEventStreamFlushSync`.

Phase 5 records the final choice in `worktree/docs/dependencies.md`.
`windows-sys` 0.61 is already in the workspace.

Supported filesystem capabilities, as far as the spikes show:

- local APFS, ZFS, ext4, tmpfs, ReFS, and NTFS support every mechanism above;
- WSL drvfs is refused by R5's move before observation matters;
- network filesystems and FUSE are untested. Phase 5 must refuse when watch
  setup fails, and Phase 8 records the gap.

### S3: Git move and final unregister (R6, R1 detail)

The same script ran on macOS (2.56.0), Linux (2.47.3), and WSL (2.53.0), with
identical results except where noted.

- **`git worktree move` replaces an existing empty directory.** Moving
  `q1a` into `holder/`, where `holder/q1a/` existed empty, succeeded, and the
  inode at `holder/q1a` changed. It refuses a file or a non-empty directory
  (`'…' already exists`). An existing empty directory is accepted
  deliberately (as by `worktree add`). Git also turns a directory
  destination into `<dir>/<name>`. Its existence check is followed by
  `rename(2)`, so it fails the no-replacement contract. **R6: never used.**
- **What a move rewrites:** exactly the admin `gitdir` and the checkout's
  `.git` file. Unrelated records, including a prunable one, are untouched.
  The `.git` file is rewritten in the *invoking* config's link format: a
  relative-link record moved without `worktree.useRelativePaths` came out
  absolute.
- **Manual equivalent, verified.** After a native rename plus a rewrite of
  only `<common>/worktrees/<id>/gitdir`, `git worktree list` shows the new
  path, `status` works inside it, and `prune --dry-run` leaves it alone.
  Unrelated records are unchanged.
- **Relative links (Git ≥ 2.48, `extensions.relativeWorktrees = true`).**
  The admin `gitdir` is relative to **the admin directory itself**
  (`../../../../q3b/.git` from `main/.git/worktrees/q3b/`). The `.git` file
  is relative to the checkout (`../main/.git/worktrees/q3b`). A
  same-depth sibling move leaves the `.git` file valid, so only the admin
  `gitdir` changes. A move to another depth also breaks the `.git` file.
  The spike's own off-by-one rewrite (`../../../`) made the record prunable,
  so Phase 4 computes the relative form and verifies it by reading it back
  with Git. Git 2.47 ignores `worktree.useRelativePaths`, and reads a
  relative admin `gitdir` verbatim as a path.
- **Rule for Phase 4:** the quarantine is a sibling (same depth). Rewrite the
  admin `gitdir` in the record's existing format; leave the `.git` file
  unchanged; refuse a record whose `.git` file would need rewriting.
- **Locks:** `move` and `move -f` refuse a locked worktree, and so does
  `remove --force`. The `locked` file survives a manual move, and a plain
  remove of a locked absent record refuses. So the native move must refuse
  a locked worktree itself (re-listed immediately before moving), and the
  final step cannot bypass a lock.
- **Submodules:** `move` and plain `remove` refuse ("working trees
  containing submodules cannot be moved or removed"). The submodule's git
  directory lives under the admin directory
  (`worktrees/<id>/modules/<name>`). An early, invalid run of the script
  also showed `remove --force` succeeding on such a worktree. That is the
  path today's `wt remove` takes when files may be discarded, so refusing
  submodule checkouts before mutation is the regression R1 already
  discloses.
- **Index files:** a split index puts `sharedindex.<sha>` in the
  *per-worktree admin directory*, not the common directory. Intent-to-add
  and skip-worktree show in `ls-files -v` and in the `--stage --debug`
  flags. The existing `missing.rs` binding (BLAKE3 of the `index` bytes,
  plus BLAKE3 of `ls-files --stage --debug -z` with `GIT_INDEX_FILE`)
  already covers both. The admin watch must cover `index`, `index.lock`,
  `sharedindex.*`, `HEAD`, `gitdir`, and `locked`.
- **Final unregister:**
  - A plain `git worktree remove` of a quarantine holding only `.git`
    **refuses**, because Git counts the deleted tracked files as
    modifications.
  - A single `--force` works, but it would delete whatever is there.
  - A plain remove of the **absent** directory is record-only and works.
  - A recreated directory without `.git` is refused ("validation failed").

  **Ruling:** delete approved content children-first with the quarantine's
  `.git` file last. Then `rmdir` the quarantine root; that fails if anything
  reappeared. Then reuse the tested `missing.rs` `remove_missing_record`
  path. It re-lists the record and rechecks admin identity and HEAD,
  re-confirms the absence, rebinds the index digests, and runs a plain
  `git worktree remove <quarantine>`. No `--force` is ever passed.

### Departures and items for the author

These do not block Phase 2 (records, journal, and typed outcomes touch none
of them). They need the author's confirmation before the phase each one
gates:

1. R1, R2, R3: still proceeding on the spec's recommendations, pending
   confirmation (gate Phases 3–6).
2. R4: three native observer adapters instead of `notify` (gates Phase 5),
   adding `inotify` and `fsevent-sys`.
3. R5: a WSL checkout on a drvfs (`/mnt/<drive>`) volume is refused, not
   removed (gates Phase 4). The placeholder fallback is rejected unless the
   author overrides.
4. R9 amendment: Windows `ERROR_ACCESS_DENIED` from the rename call is
   classified as "in use" (exit 4), so a rare ACL denial may be described as
   usage (gates Phase 4).

### Verification

No package source changed in this phase, so no behavior needs a new test.
The phase's requirements were capability questions, each answered by the
spike that maps to it:

| Requirement | Answered by |
| --- | --- |
| R5 | S1 on four hosts |
| R4 | S2 and B1 on four hosts |
| R6 | S3 on Git 2.47, 2.53, and 2.56 |

Close-out gates in `worktree/`, on macOS, 2026-10-11:

- `just test`: 1173 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`.

Not run: `just test-l2` (not required for a phase without source changes)
and cross-OS package suites (nothing compiled changed).

Phase 1 is complete; ready for review.
