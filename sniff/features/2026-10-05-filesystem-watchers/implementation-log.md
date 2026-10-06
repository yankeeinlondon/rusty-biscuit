---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-message/sniff/features/2026-10-05-filesystem-watchers/spec.md"
plan: "sniff/features/2026-10-05-filesystem-watchers/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/linux.md
  - .claude/skills/os/macos.md
  - .claude/skills/os/windows.md
packages: []
---

# Implementation Log for 2026-10-05-filesystem-watchers (7 phases)

## Phase 1

Phase 1 records rulings and runs spikes. It changes no shipped source; spike
crates were scratch projects outside the repository and were deleted afterward.

### Rulings R1-R11

The spec stays unchanged; these rulings are the decision record for Phases 2-7.

- **R1 - Deadline contract: option 1, shared scan budget (provisional, author
  confirmation requested).** The spec's Open Question has no author answer in
  the repository. Following the plan and the spec's recommendation, Phases 2-6
  proceed on option 1: one monotonic budget starts on entry to
  `query_path_usage`; when it expires, no new inspection is scheduled, collected
  evidence is kept, and the report records elapsed time and `budget_exhausted`.
  A native call already running may overrun the budget. Consequences:
  - `--timeout` help, rustdoc, and docs say "stops scheduling work after", never
    "returns within".
  - No helper executable, no worker thread that outlives the call, no thread
    termination.
  - `wt` adopting this latency contract is a separate decision outside this
    feature.
  Because the spec marks this "requires author decision", the spec carries
  `human_review: true` so the author can confirm before Phase 2's public API
  shape is fixed. Choosing option 2 or 3 would materially change Phases 2, 5,
  and 6.
- **R2 - Hang-prone native operations: omit and report.** Any operation that can
  block indefinitely without safe cancellation is not called. It is reported as
  a structured limitation. On Windows this specifically excludes
  `NtQueryObject(ObjectNameInformation)` and any name query on a handle that is
  not a disk file (see S2 for the confirmed call sequence). This release ships
  no helper executable. **Escalated after S2:** on Windows the hazard also
  covers every identity query on another process's *disk* handle, so applying
  this ruling unchanged removes Windows handle evidence. The author chooses
  between omission and a Windows-only killable helper before Phase 5 (spec
  `human_review_items`).
- **R3 - Linux enumeration: thread-group leaders only.** Read `readdir("/proc")`
  numeric entries (TGIDs) only, never `/proc/<pid>/task/*`; read fd tables,
  fdinfo, cwd, and stat from `/proc/<pid>`. `/proc/<tid>` resolves for a thread
  ID even though `readdir` does not list it, so code must never probe arbitrary
  numeric paths (see S3).
- **R4 - Result truncation caps: 1000 evidence records per process, 10000 in
  total.** A cap that is hit is recorded on the affected coverage record with
  the limit and the omitted count. It also adds a `truncated` limitation. Both
  caps are constants in the library, not public options in this release.
- **R5 - Human-output truncation: 50 processes, 20 evidence lines per process.**
  Applies to human/plain output only, and each shortened list prints how many
  records were omitted. JSON is never truncated by the CLI.
- **R6 - Outcome rule.** `usable` when at least one applicable usage mechanism
  is `complete`, or `partial` with at least one successful inspection.
  `unavailable` when supported mechanisms exist but every one is `failed`,
  `not_attempted`, or `partial` with zero successful inspections. `unsupported`
  when no applicable usage mechanism is supported. Process enumeration and tree
  identity collection are prerequisites, not usage mechanisms: they never make
  an outcome `usable` on their own. A record whose status is fixed at
  `unsupported` (FSEvents, fanotify, Windows cwd/subscriptions) never affects
  `usable` vs `unavailable`.
- **R7 - Serialized names.** snake_case keys: `schema_version` (integer, starts
  at `1`), `target`, `processes`, `coverage`, `limitations`, `evidence`, `kind`,
  `status`, `outcome`. Evidence `kind` values: `watch_registration`,
  `open_handle`, `working_directory`, `loaded_module`. Coverage `status`
  values: `complete`, `partial`, `unsupported`, `failed`, `not_attempted`.
  Outcome values: `usable`, `unavailable`, `unsupported`. Path object:
  `{"display": "...", "native": {"encoding": "unix_bytes" | "windows_utf16",
  "units": [...]}}`, with `native` present only when `display` does not
  round-trip. 64-bit identifiers (inode, device, file index, volume serial,
  creation tokens, masks) serialize as decimal strings. Unavailable identity
  fields serialize as an explicit `null`, never as an omitted key.
- **R8 - Wider measurement: not scheduled.** The spikes recorded single
  indicative timings only; no timing campaign. Whether `wt` wants latency
  numbers before adopting the API is the author's separate decision.
- **R9 - Crate selection:** decided by S1-S3 below; summary in "Crate
  decisions".
- **R10 - Windows elevation: none.** Not elevated by default and no opt-in
  elevated mode. Protected and other-user process limits are reported as
  limitations with counts.
- **R11 - Test inputs and CI scope.** Planned fixtures live in temporary
  directories, so no `[package.metadata.ci.tests] source-inputs` change is
  expected. One exception: the fdinfo matrix fixture is a captured real
  fdinfo text. If it is stored as a repository file, read it with
  `include_str!` so CI sees the input. Phase 7 confirms with
  `just ci-local --plan`.

### Spike S1 - macOS `libproc` scope (macOS 27.2, arm64)

Run in a scratch crate against python3 children with a stdout `ready`
handshake and kill-and-wait cleanup. Child A held `dir/sub/file` open with cwd
`dir/sub`; child B held `dir` open with `O_EVTONLY`; child C only had cwd
`dir/sub`.

- **Crate:** `libproc` 0.14.11, MIT. On macOS its `build.rs` runs bindgen
  0.72 (needs libclang at build time); the generated bindings are private.
- **`pids_by_type_and_path` / `pids_by_path` is not recursive.** It matches
  vnode identity via an fd or a cwd: querying `dir` returned B only, `dir/sub`
  returned A and C (cwd), and `dir/sub/file` returned A. Nothing beneath a
  queried directory matches. `/var` and `/private/var` spellings gave the same
  result.
- **Crate bug:** `sys/macos.rs` passes `pathflags` on the sizing call only and
  `0` on the fill call, so `is_volume` and `exclude_event_only` are ignored.
  Raw `proc_listpidspath` with flags works: `PROC_LISTPIDSPATH_PATH_IS_VOLUME`
  returned about 823 PIDs (everything on the device, not tree evidence). The
  crate also reads `errno` without clearing it first.
- **Descriptors:** `proc_pidinfo(PROC_PIDLISTFDS)` plus
  `proc_pidfdinfo(PROC_PIDFDVNODEPATHINFO = 2)` gives fd, `fi_openflags`,
  `vst_dev`, `vst_ino`, and the canonical `/private/...` path. `libc` has
  `vnode_info_path`, `vinfo_stat`, `proc_vnodepathinfo`, `proc_fdinfo`,
  `proc_pidinfo`, `proc_pidfdinfo`, and `O_EVTONLY`. It lacks
  `proc_fileinfo`, `vnode_fdinfowithpath`, `proc_listpidspath`, and the
  `PROC_PIDFDVNODEPATHINFO` constant; those need small local `#[repr(C)]`
  definitions.
- **cwd:** `libproc::proc_pid::pidcwd` returns `Err("pidcwd is not implemented
  for macos")`. Raw `proc_pidinfo(PROC_PIDVNODEPATHINFO)` works and gives the
  cwd path, dev, and inode.
- **Event-only opens are visible:** B's fd appeared with
  `fi_openflags = 0x8001` (`O_EVTONLY | FREAD`), so the `event_only` attribute
  can be set from bit `0x8000`.
- **Permission:** for PID 1, every per-PID call returns 0 with `errno = EPERM`.
  `proc_listpids` returns the full PID list (1217, same as `ps -ax`); only
  per-PID detail is denied. libproc wraps the errno in a `String`, which loses
  the typed error.
- **Buffer sizing:** `PROC_PIDLISTFDS` fills only the buffer given and gives no
  sign that more fds exist (`max_len = 2` returned 2). `pbi_nfiles` is table
  capacity, not the open count. The reader must size the buffer (NULL-buffer
  estimate or `pbi_nfiles` plus headroom) and retry larger whenever
  `returned == capacity`.

**Decision:** no `libproc` dependency. Phase 4 uses direct FFI through `libc`
plus about four local definitions. `proc_listpidspath` is not used as a
candidate source: it is not recursive, so it cannot narrow a tree query, and a
full fd/cwd scan is needed anyway. FSEvents is reported `unsupported`.

### Spike S3 - Linux `procfs` fit (build-linux, kernel 7.0 in an LXC container)

- **Crate:** `procfs` 0.18.0, `MIT OR Apache-2.0`. With
  `default-features = false` it pulls `procfs-core`, `rustix` 1.1,
  `linux-raw-sys`, `bitflags`, and `hex`.
- **Leaders only:** `all_processes()` uses `readdir("/proc")` numeric entries,
  which list TGIDs only. Trap: `/proc/<tid>` still resolves for a thread ID
  (its `status` shows `Tgid` != `Pid`), so never probe numeric paths that came
  from elsewhere.
- **Per-fd errors are swallowed by procfs:** `FDsIter::next` skips any fd whose
  info fails. For a same-uid, non-dumpable process (`systemd --user`),
  `read_dir("/proc/<pid>/fd")` listed 34 entries, every per-fd stat gave
  EACCES, and `procfs` `fd()` returned `Ok` with 0 items. This violates the
  "permission denial is never no matches" rule.
- **Identity:** procfs stats the fd link with `SYMLINK_NOFOLLOW` (the magic
  link's own dev/ino). Target identity needs a following
  `std::fs::metadata("/proc/<pid>/fd/<n>")`; that matched the held file's
  dev/ino exactly. Same for `/proc/<pid>/cwd`.
- **fdinfo:** procfs 0.18 has no fdinfo or inotify API at all.
- **Live inotify fdinfo** (child watching `tree/a`, `tree/b`, and the tree's
  parent):

  ```text
  pos:	0
  flags:	02004000
  mnt_id:	19
  ino:	1070
  inotify wd:3 ino:b84b5 sdev:2e mask:fff ignored_mask:0 fhandle-bytes:c fhandle-type:1 f_handle:0a00b5840b0000000a691300
  inotify wd:2 ino:b84b8 sdev:2e mask:200 ignored_mask:0 fhandle-bytes:c fhandle-type:1 f_handle:0a00b8840b0000000a691300
  inotify wd:1 ino:b84b7 sdev:2e mask:102 ignored_mask:0 fhandle-bytes:c fhandle-type:1 f_handle:0a00b7840b0000000a691300
  ```

  `ino` hex equals `st_ino`. `sdev` is the kernel-internal `dev_t`
  (`major << 20 | minor`); normalize with `major = sdev >> 20`,
  `minor = sdev & 0xfffff`, `st_dev = libc::makedev(major, minor)`. On
  build-linux every mount is major 0, minor < 256, so raw values coincide
  there (`2e` = `st_dev` 46). A raw comparison would be wrong on a real block
  device (`sdev:800011` is 8:17, `st_dev` 0x811). Phase 3 tests must cover the
  decode with a nonzero major; WSL2 ext4 is the live host for that.
- **Permission (uid 1000 vs PID 1):** `/proc/1/fd`, `/proc/1/fdinfo`,
  `/proc/1/cwd` give EACCES; `/proc/1/stat` is readable. Two denial shapes
  exist: directory-level EACCES, and the fd directory listing succeeding while
  every per-fd read fails. Both must be counted and reported.
- **Start time:** `/proc/<pid>/stat` field 22; parse after the last `)` because
  `comm` can contain spaces and parentheses.

**Decision:** no `procfs` dependency. Phase 3 uses a hand-rolled `/proc`
reader on `std` plus `libc` (already a sniff dependency) and a hand-written
fdinfo parser. The `inotify` crate is a dev-dependency only, for creating
watches in tests.

### Spike S2 - Windows handle enumeration (build-win-native)

The first S2 run stopped before running anything on the host. It produced only
the `windows` feature list below, so S2 was re-run (S2b), and a follow-up
(S2c) answered the helper question. Caveat: the SSH session is elevated (High
integrity, SeDebugPrivilege), so a non-elevated view was simulated with a
filtered Medium-integrity impersonation token.

- **Binding: the existing `windows` 0.62 dependency covers everything.**
  `windows-sys` and `ntapi` are not needed. Features to add:
  `Wdk_Foundation` (`NtQueryObject`), `Wdk_System_SystemInformation`
  (`NtQuerySystemInformation`; class 64 = `SystemExtendedHandleInformation` is
  not a named constant), `Win32_Storage_FileSystem`,
  `Win32_System_ProcessStatus`, `Win32_System_WindowsProgramming`, and
  `Win32_System_SystemInformation` (for `IsWow64Process2` machine types).
  `SYSTEM_HANDLE_TABLE_ENTRY_INFO_EX` (40 bytes on x64) needs a local
  `#[repr(C)]` definition.
- **Snapshot:** `NtQuerySystemInformation(64)` with a
  `STATUS_INFO_LENGTH_MISMATCH` retry loop. One retry from 64 KiB: 103,554
  handles, 4.1 MB, 7-12 ms. Pre-filter by `ObjectTypeIndex`, taken from one of
  the inspector's own file handles (42 on this host).
- **Controlled child found** on both ReFS (`B:`) and NTFS (`C:`) through
  `OpenProcess(PROCESS_DUP_HANDLE)`, `DuplicateHandle`, `GetFileType` (keep
  `FILE_TYPE_DISK` only), and `GetFileInformationByHandleEx(FileIdInfo)`,
  matched against a walk of the tree.
- **Working directory is visible in the handle table:** a directory handle
  with access `0x100020` (`FILE_TRAVERSE | SYNCHRONIZE`) whose `FileIdInfo`
  matched `tree\sub`. Windows can therefore report `working_directory`
  evidence *only as an open directory handle*: it cannot tell a cwd handle
  apart from any other directory handle without reading the PEB. Phase 5
  records it as `open_handle` and keeps the cwd mechanism `unsupported`, as the
  spec says.
- **Identity:** compare `(VolumeSerialNumber, 128-bit FileId)`. On ReFS the
  upper 64 bits carry the directory and the lower 64 bits the entry, so files
  in one directory share the upper half; the 64-bit
  `GetFileInformationByHandle` index is not sufficient.
- **R2 hazard confirmed, and wider than the plan assumed.** `GetFileType` never
  blocked. On a **disk** handle whose owner has synchronous I/O pending (a
  synchronous handle blocked in `LockFileEx`), `FileIdInfo`, `FileBasicInfo`,
  `FileModeInformation`, `GetFinalPathNameByHandleW`, and
  `NtQueryObject(ObjectNameInformation)` all blocked until the owner exited.
  `CancelSynchronousIo` on the blocked thread failed with 1168
  (`ERROR_NOT_FOUND`), because the wait is on the file object's lock, not on
  the inspector's own I/O. A synchronous named-pipe server blocked the name
  queries the same way. In the wild, one OneDrive handle stalled a worker for
  more than 1.5 s at Medium integrity before clearing by itself.
- **A killable helper process does release it (S2c).** Seven trials: a helper
  process blocked in `FileIdInfo`, `FileModeInformation`, or `FileBasicInfo`
  on the duplicated handle was ended by `TerminateProcess` in 1.0-1.5 ms
  every time. The owner and the lock holder were unaffected, and the helper
  left no process behind. Untested: a handle stuck inside a hung filesystem
  or minifilter driver (for example a network redirector) in a non-alertable
  wait could resist termination.
- **Denials:** of 234 processes holding file handles, `OpenProcess
  (PROCESS_DUP_HANDLE)` failed with error 5 for 14 when elevated and for 140 at
  Medium. The Medium count includes the user's own elevated processes. A PID
  that has exited gives error 87.
- **Modules:** `EnumProcessModulesEx(LIST_MODULES_ALL)` needs a size retry
  (32 bytes in, 592 needed; 74 modules). Denied processes give error 5, and the
  System process gives error 299 through a limited handle.
- **Identity enrichment:** `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`,
  `GetProcessTimes` (creation FILETIME as the reuse token), and
  `QueryFullProcessImageNameW` all worked.
- **Indicative full pass:** 0.7-1.0 s elevated (about 4,500 disk handles).
  3.0 s at Medium, because of the OneDrive stall.

**Consequence for R2 (escalated to the author).** Taken as written, R2 (omit
hang-prone calls, no helper executable) means the Windows `open_handle`
mechanism cannot read identity from any other process's handle. It would
report `unsupported`, and Windows would keep only loaded modules and process
identity. The alternative is a Windows-only killable helper process for the
per-handle identity reads. That is spec option 2 scoped to one mechanism, and
it adds packaging and protocol work to Phase 5. The spec forbids abandoned
in-process workers, which are the only in-process alternative. Recorded as a
`human_review_items` entry on the spec. Phase 5 must not start until it is
answered; Phases 2-4 are not blocked by it.

### Crate decisions (R9)

| Need | Decision | Version / license | Native build requirement |
| --- | --- | --- | --- |
| Linux/WSL2 `/proc` reading | Hand-rolled on `std` + `libc` (existing dep); **no `procfs`** | n/a | none |
| Linux inotify fdinfo parser | Hand-written (spec) | n/a | none |
| Linux test watches | `inotify` crate, **dev-dependency only**, Linux-gated | latest at Phase 3 | none |
| macOS descriptors/cwd | Direct FFI via `libc` + ~4 local `#[repr(C)]` items; **no `libproc`** | n/a | none (avoids libproc's build-time bindgen/libclang) |
| Windows handles/modules/identity | Existing `windows` 0.62 dep + new features (S2 list) | 0.62.x, MIT OR Apache-2.0 | none beyond current |
| Process identity enrichment | Existing `sysinfo` (candidate PIDs only) or the backend's own native calls | 0.38.x (existing) | none |
| Tree walk | Existing `walkdir` | 2.5 (existing) | none |

So Phase 1 adds no new runtime dependencies. Phase 3 adds one dev-dependency,
and Phase 5 adds features to an existing dependency.

### Departures from the plan

- S1's plan question offered "libproc as candidate source"; the evidence rules
  out both libproc and `proc_listpidspath` as a tree candidate source (not
  recursive). Phase 4's "Candidate source" task therefore resolves to "not
  used".
- R2 as proposed in the plan ("omit; no helper in this release") is
  contradicted by S2's evidence for Windows' core mechanism. It is escalated
  rather than silently applied.
- The plan and spec expect Windows cwd to be `unsupported`. S2 shows the cwd
  handle appears in the handle table, but it is indistinguishable from other
  directory handles. It surfaces as `open_handle` evidence; the cwd mechanism
  stays `unsupported`.

### Verification and handoff (Phase 1)

- **Requirement-to-test mapping:** Phase 1 changes no behavior and adds no
  tests. Each spike's evidence comes from a scratch program run on its host
  (S1 macOS locally, S2/S2b/S2c on build-win-native, S3 on build-linux).
  Every scratch directory and child process was removed and confirmed gone.
  The fdinfo text captured in S3 is the candidate real-tool fixture for
  Phase 3's input-robustness matrix.
- **Gates run on macOS:** `just lint` in `sniff/` passed; `just test` in
  `sniff/` passed (3133 run, 3133 passed, 32 skipped; no pre-existing
  failures).
- **Skill updates:** `.claude/skills/os/` gained the macOS `proc_pidinfo` and
  libproc facts, the Linux `/proc` and `procfs`/`sdev` traps, the Windows
  per-handle hang and helper-termination findings, and two symptom-router
  rows. The `sniff` skill is unchanged: no Sniff API or counter exists yet.
- **Human review:** the spec sets `human_review: true` for R1 (deadline
  semantics) and the Windows per-handle hang (R2 option A vs B). Phase 2 can
  proceed on R1 option 1; Phase 5 must wait for the Windows answer.
