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
packages:
  - sniff
  - sniff-cli
source_files_during_phase_2:
  - sniff/lib/Cargo.toml
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/query/mod.rs
  - sniff/lib/src/filesystem/query/backend.rs
  - sniff/lib/src/filesystem/query/budget.rs
  - sniff/lib/src/filesystem/query/identity.rs
  - sniff/lib/src/filesystem/query/matching.rs
  - sniff/lib/src/filesystem/query/native.rs
  - sniff/lib/src/filesystem/query/options.rs
  - sniff/lib/src/filesystem/query/process.rs
  - sniff/lib/src/filesystem/query/report.rs
  - sniff/lib/src/filesystem/query/root.rs
  - sniff/lib/src/filesystem/query/tree.rs
  - sniff/lib/src/filesystem/query/tests.rs
docs_updated_during_phase_2:
  - sniff/docs/topics/filesystem-query.md
  - sniff/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
source_files_during_phase_3:
  - sniff/lib/src/filesystem/query/linux.rs
  - sniff/lib/src/filesystem/query/linux_tests.rs
  - sniff/lib/src/filesystem/query/fixtures/inotify-fdinfo.txt
  - sniff/lib/src/filesystem/query/backend.rs
  - sniff/lib/src/filesystem/query/mod.rs
  - sniff/lib/src/filesystem/query/report.rs
  - sniff/lib/src/filesystem/query/tree.rs
  - sniff/lib/src/filesystem/query/tests.rs
docs_updated_during_phase_3:
  - sniff/docs/topics/filesystem-query.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_4:
  - sniff/lib/src/filesystem/query/macos.rs
  - sniff/lib/src/filesystem/query/macos_tests.rs
  - sniff/lib/src/filesystem/query/backend.rs
  - sniff/lib/src/filesystem/query/linux.rs
  - sniff/lib/src/filesystem/query/mod.rs
  - sniff/lib/src/filesystem/query/tests.rs
docs_updated_during_phase_4:
  - sniff/docs/topics/filesystem-query.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/os/macos.md
source_files_during_phase_5:
  - sniff/lib/Cargo.toml
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/src/os/mod.rs
  - sniff/lib/src/os/user.rs
  - sniff/lib/src/filesystem/query/win32.rs
  - sniff/lib/src/filesystem/query/win32_tests.rs
  - sniff/lib/src/filesystem/query/backend.rs
  - sniff/lib/src/filesystem/query/identity.rs
  - sniff/lib/src/filesystem/query/mod.rs
  - sniff/lib/src/filesystem/query/tests.rs
docs_updated_during_phase_5:
  - sniff/docs/topics/filesystem-query.md
  - sniff/docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/os/windows.md
source_files_during_phase_6:
  - sniff/cli/src/args/filesystem.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/filesystem_query.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/query.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/tests/l1/filesystem_query.rs
  - sniff/cli/tests/l1/main.rs
  - sniff/cli/tests/l1/snapshots/l1__snapshots__help_output.snap
docs_updated_during_phase_6:
  - sniff/docs/cli/filesystem_query.md
  - sniff/docs/topics/filesystem-query.md
  - sniff/cli/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/sniff/cli.md
  - .claude/skills/sniff/testing.md
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

## Phase 2

Phase 2 builds the platform-neutral library core in
`sniff/lib/src/filesystem/query/`, re-exported from `sniff::filesystem`
(`query_path_usage`, `PathUsageOptions`, `PathUsageReport`, `PathUsageError`).
It proceeds on R1 option 1 (shared scan budget), as Phase 1 directed; the
author has not yet answered R1, so the spec keeps `human_review: true`.

### What was built

| File | Contents |
| --- | --- |
| `options.rs` | `PathUsageOptions` (recursive by default, `target_only()`, `with_deadline()`, 2 s default); `PathUsageError` with stable `kind()` strings |
| `native.rs` | `NativeString` lossless path/name object (hand-written strict visitor); `decimal` adapters for 64/128-bit ids; `nullable` (absent is an error, `null` is unavailable) |
| `budget.rs` | One monotonic `Budget` per query; sticky `exhausted`; test-only check-count expiry and `expire()` |
| `identity.rs` | `FileIdentity` (`unix` dev/ino; `windows` volume serial + 128-bit `FileIdInfo`) and per-OS reads |
| `root.rs` | Root resolution from `symlink_metadata` (FIFO/socket/device rejected before any open), one canonicalization, identity capture, recheck |
| `tree.rs` | One streaming `walkdir` walk, identity -> every in-scope path (hard links), aggregated unreadable-entry limitations, budget check per entry |
| `matching.rs` | Identity-first matching; path-only evidence gets a component-aware pre-filter then an identity lookup (`match_basis: path_lookup`); deleted objects never matched by former path |
| `process.rs` | Merge only on equal PID **and** creation token; token-less records stay separate and `identity_uncertain`; enrichment re-reads the token after reading details and refuses on change |
| `report.rs` | Report types (R7 names), `Limitations` aggregator (kind + message, three examples), R6 `outcome` |
| `backend.rs` | Crate-private `UsageBackend` trait (enumerate once, inspect per process, `creation_token`, `details` defaulting to per-PID `sysinfo`), interim `PlatformBackend` |
| `mod.rs` | `query_path_usage` and the orchestrator: coverage per mechanism, prerequisite-gap propagation, R4 caps, sorting |
| `tests.rs` | Unit tests through a fake backend over real temp trees: 55 run on macOS, 56 on Linux, 52 on Windows (OS-gated cases differ) |

New counters (`performance::counters`): `filesystem.query.tree_walks`,
`tree_identity_reads`, `process_enumerations`, `process_inspections`,
`descriptor_inspections`, `watch_registration_reads`, `identity_enrichments`,
`path_lookups`. The descriptor and watch counters are for Phase 3-5 backends.

`sniff/lib/Cargo.toml` gains the `windows` feature `Win32_Storage_FileSystem`
(`CreateFileW`, `GetFileInformationByHandleEx(FileIdInfo)`). No new crates.

### Decisions and departures from the plan

- **Root recheck runs before enrichment.** The plan orders "enrich, recheck
  root". The recheck decides whether coverage can stay complete; enrichment is
  optional. Running the recheck first means a budget that runs out during
  enrichment cannot also leave the root unverified.
- **Path fallback is a lookup, not text comparison.** The plan asks for
  component-aware comparison that also handles Windows drive/UNC/verbatim
  spellings, short names, and per-directory case sensitivity. Text cannot
  decide short names or case-sensitive directories without querying the
  filesystem. So a path-only observation passes a component-aware pre-filter
  (never a string prefix; over-inclusive on Windows: case-folded, prefixes
  folded, `~` components let through) and is then looked up by identity. The
  match carries `match_basis: path_lookup` so a consumer knows the lookup
  could name a replacement. Evidence with a backend identity is matched on
  identity alone.
- **Windows tree identities use one attribute-only handle per entry.**
  `CreateFileW(FILE_READ_ATTRIBUTES, FILE_FLAG_OPEN_REPARSE_POINT)` plus
  `FileIdInfo`, closed immediately. This is not the S2 hang: the handle is the
  query's own, not another process's. If Windows trees exhaust the budget in
  Phase 5, the bulk alternative is `GetFileInformationByHandleEx
  (FileIdExtdDirectoryInfo)` once per directory. The Windows walker also
  refuses to descend into non-link reparse directories (cloud placeholders),
  which std reports as ordinary directories.
- **`elapsed_us: u64`, not `elapsed_ms: f64`.** The first Linux run failed the
  report round trip: serde_json (without `float_roundtrip`) read `0.399236`
  back as `0.39923600000000004`. An integer reads back exactly.
- **Mechanism names.** `open_handles` covers Unix descriptors and Windows
  handles; the rest are `working_directories`, `inotify`, `fanotify`,
  `fsevents`, `loaded_modules`, `directory_change_subscriptions`, `polling`;
  prerequisites are `process_enumeration` and `tree_identity`. Coverage
  `scope` is `visible_processes`, `target`, or `target_tree`.
- **Where limitations live.** Mechanism-specific limitations sit on that
  mechanism's coverage record. Report-wide ones (root changed or unverified,
  identity enrichment) are in the top-level `limitations`. An unsupported
  mechanism is reported once, as `status: unsupported` with a `reason`, not
  also as a limitation.
- **`attempted`/`succeeded` units.** Process inspections for usage
  mechanisms, tree entries for `tree_identity`, processes listed for
  `process_enumeration`. A mechanism with attempts and zero successes is
  `failed` (its denials remain listed); a mechanism never started because the
  budget ran out or enumeration failed is `not_attempted` with a `reason`.
- **Creation token is a plain `u64`** (Linux `starttime` ticks, macOS start
  microseconds, Windows creation FILETIME), serialized as a decimal string.
- **Interim backend.** `backend::platform()` declares each OS's mechanisms
  but implements none, so a real query today returns `outcome: unsupported`
  with every reason filled in. The fixed-unsupported reasons (FSEvents,
  fanotify, polling, Windows cwd and `ReadDirectoryChangesW`) are final.
- **Not built in Phase 2 (belongs to Phase 3):** the "watch on an ancestor is
  outside this query" limitation. It needs inotify data, so Phase 3 adds a
  `LimitationKind` and the ancestor identities.
- **`Budget::remaining()` was not added**; no caller needs it yet.
- **Windows trap found and fixed:** `io::Error::from(windows::core::Error)`
  keeps the HRESULT as the raw OS error, so `ErrorKind::NotFound` never
  matched; a deleted root was reported as unverified instead of changed.
  `identity.rs` now unwraps `FACILITY_WIN32` HRESULTs. Recorded in the `os`
  skill (`windows.md` plus a symptom-router row).

### Report-reader robustness matrix

The outcome table is in the plan under Phase 2 ("Report-reader robustness
matrix"). Structs use `deny_unknown_fields`; every `Option` field uses
`nullable` (or `decimal::option`), so an absent key is an error and `null` is
"unavailable"; `Vec` fields have no `serde(default)`. The control row (the
unedited serialized report reads back equal) passes on every host. Grep for
smells: no `#[serde(default)]`, `unwrap_or_default()`, or `.ok()` on a
load-bearing parse in `query/`. The one `.ok()` is in `matching.rs`, where a
failed lookup correctly means "no match".

### Requirement-to-test mapping

All tests are L1 unit tests in `sniff/lib/src/filesystem/query/tests.rs`
(lib target, no tier marker, default features).

| Requirement | Tests |
| --- | --- |
| Options, typed errors, stable kinds | `default_options_are_recursive_with_a_two_second_budget`, `zero_and_unrepresentable_deadlines_are_invalid_options`, `a_missing_target_is_a_typed_error_not_an_empty_report`, `a_dangling_root_symlink_is_a_missing_target` (Unix), `a_fifo_target_is_rejected_before_it_is_opened` (Unix), `every_error_kind_is_stable` |
| Root alias resolved once, both spellings | `a_root_symlink_queries_its_target_and_reports_both_spellings` (Unix), `a_relative_target_keeps_its_requested_spelling` |
| All five coverage statuses | `a_clean_inspection_is_complete_and_usable_with_no_matches`, `a_denied_process_makes_coverage_partial_and_stays_visible`, `every_inspection_failing_is_failed_and_unavailable`, `a_failed_enumeration_leaves_usage_mechanisms_not_attempted`, `a_process_that_exits_before_inspection_is_a_visible_gap` |
| Outcome rule (R6) | `the_outcome_rule_ignores_prerequisites_and_fixed_unsupported_records`, `a_partial_mechanism_with_retained_evidence_is_usable`, `an_empty_visible_process_set_is_complete`, `a_partial_enumeration_with_nothing_inspected_is_unavailable`, `the_shipped_backend_reports_unsupported_until_a_native_backend_exists` |
| Prerequisite propagation | `a_partial_enumeration_propagates_into_every_dependent_mechanism`, `an_unreadable_descendant_makes_dependent_coverage_partial` (Unix, non-root) |
| Budget before/during each phase | `a_budget_expired_on_entry_is_a_root_validation_timeout`, `a_budget_expiring_during_root_validation_is_a_root_validation_timeout`, `a_budget_expiring_during_the_tree_walk_keeps_a_report_with_a_partial_tree`, `a_budget_expiring_before_enumeration_leaves_mechanisms_not_attempted`, `a_budget_expiring_during_enumeration_inspects_nothing`, `a_budget_expiring_before_any_inspection_is_not_attempted`, `a_budget_expiring_during_inspection_keeps_collected_evidence` (also covers recheck and enrichment skipped) |
| Root replacement and removal | `a_root_replaced_during_the_query_keeps_evidence_and_reports_the_change`, `a_root_removed_during_the_query_is_a_root_change` |
| PID reuse, missing start time, enrichment | `the_same_pid_with_different_start_tokens_stays_two_processes`, `enrichment_after_pid_reuse_never_mislabels_prior_evidence`, `enrichment_after_the_process_exits_keeps_its_evidence`, `verified_enrichment_fills_identity_without_overriding_backend_fields`, `missing_start_tokens_never_merge_and_are_not_enriched`, `a_retained_handle_without_a_token_allows_enrichment` |
| Hard-link alias | `hard_links_report_every_in_scope_path_and_the_observed_alias` |
| Sibling-prefix rejection, path spellings | `a_similarly_prefixed_sibling_is_outside_the_tree`, `windows_spellings_are_compared_by_component_not_by_string` (Windows), `a_case_variant_path_lookup_matches_only_the_object_it_names` (Windows) |
| Path-only and identity matching rules | `a_path_only_observation_inside_the_tree_matches_by_lookup`, `a_deleted_object_is_never_matched_by_its_former_path`, `an_object_with_a_known_identity_is_never_matched_by_path_text` |
| Descendant symlink not expanded | `descendant_symlinks_are_indexed_as_links_and_never_expanded` (Unix) |
| Hidden/ignored in scope; target-only | `hidden_and_git_ignored_entries_are_in_scope`, `target_only_matches_the_directory_itself_and_not_descendants` |
| Malformed records, truncation (R4), sorting | `malformed_records_keep_valid_siblings_and_make_coverage_partial`, `evidence_beyond_the_per_process_cap_is_counted_as_omitted`, `processes_and_evidence_are_sorted_for_presentation` |
| One tree walk; no inventory | `one_tree_walk_serves_every_process_and_no_inventory_runs` (50 processes: 1 walk, 1 enumeration, only `filesystem.query.*`, metadata, and canonicalize counters) |
| Lossless paths, ids, explicit nulls | `a_unicode_native_string_serializes_as_display_only`, `a_non_utf8_unix_path_round_trips_losslessly` (Unix), `an_unpaired_windows_surrogate_round_trips_losslessly` (Windows), `foreign_native_encodings_decode_only_when_representable`, `a_non_utf8_file_in_the_tree_matches_and_its_report_round_trips` (Linux: APFS refuses non-UTF-8 names), `identifiers_serialize_as_decimal_strings`, `unavailable_fields_serialize_as_explicit_nulls`, `decimal_identifiers_reject_non_canonical_strings` |
| Input-robustness matrix | `the_report_reader_rejects_every_malformed_shape_of_its_load_bearing_fields`, `the_native_string_reader_rejects_every_malformed_shape` |

Mutation check: disabling the token re-read in `process::enrich` fails
`enrichment_after_pid_reuse_never_mislabels_prior_evidence`; replacing
`Path::starts_with` with a string prefix fails
`a_similarly_prefixed_sibling_is_outside_the_tree`. Both were restored.

### Gates run

- macOS (local): `just test` in `sniff/` 3188 run, 3188 passed, 32 skipped;
  `just lint` clean; `cargo clippy -p sniff --all-targets [--features remote]
  -- -D warnings` and the same for `sniff-cli` clean.
- Linux (`just cross-check sniff --os linux filesystem::query::`): 56/56.
  The first run failed the non-UTF-8 round trip (the `f64` finding above).
- Native Windows (`just cross-check sniff --os windows filesystem::query::`):
  52/52 (Unix-only tests excluded, Windows-only tests included). The first run
  failed `a_root_removed_during_the_query_is_a_root_change` (the HRESULT
  finding above).
- WSL2 not run: it compiles and runs the same Linux path build-linux
  exercised, and Phase 2 adds no backend. The nightly WSL2 leg covers it.
- `just cross-check` re-parses `-E 'test(...)'` through a shell and fails on
  the parentheses; pass a plain substring filter instead.
- No pre-existing failures; no skipped requirement.

## Phase 3

Phase 3 builds the Linux/WSL2 backend, `ProcBackend` in
`sniff/lib/src/filesystem/query/linux.rs`, and returns it from
`backend::platform()` on Linux. macOS and Windows keep the interim
`PlatformBackend` (now `#[cfg(not(target_os = "linux"))]`). Shipped code is
plain `std`: no new runtime or dev dependency.

### What was built

| File | Contents |
| --- | --- |
| `linux.rs` | `ProcBackend` over a configurable proc root (`/proc` in production): leader-only enumeration with `starttime` tokens and `comm` names; per-process descriptor, cwd, and inotify inspection; post-inspection start-time check; the hand-written `fdinfo` parser and `sdev` decode |
| `linux_tests.rs` | 16 synthetic-`/proc` tests (every Unix host) plus 4 live tests in `linux_tests::live` (Linux and WSL2) and the `#[ignore]` child fixture `usage_child` |
| `fixtures/inotify-fdinfo.txt` | The real `fdinfo` captured in S3, byte for byte (tabs kept), read with `include_str!` |
| `backend.rs` | `FANOTIFY_UNSUPPORTED`/`POLLING_UNSUPPORTED` shared with the Linux backend; `platform()` per OS; Linux branch of the interim mechanism list removed |
| `report.rs` | `LimitationKind::{AncestorWatch, DescriptorReplaced, DeviceIdentityUnreliable}`; `Limitations::has_gaps` |
| `mod.rs` | Module declarations; coverage uses `has_gaps` (an ancestor watch is not a gap) |
| `tree.rs` | `TreeIndex::identities()` for the device-mismatch check |
| `tests.rs` | `the_shipped_backend_reports_unsupported_until_a_native_backend_exists` gated to non-Linux |

Per process the backend does this:

1. List `/proc/<pid>/fd` and close the listing before reading any entry, so
   the query's own listing handle is gone when it inspects itself.
2. For each descriptor: `stat` through the magic link gives the identity. An
   in-scope match reads its link text (observed path, `(deleted)`) and its
   `fdinfo` (access from `flags`, replacement check against `ino:`).
   Otherwise, link text `anon_inode:*` means the `fdinfo` is read and parsed
   for `inotify` lines.
3. `stat` `/proc/<pid>/cwd` the same way.
4. Re-read `starttime`. If it changed or the process is gone, every mechanism
   reports `Vanished` and nothing read is kept.

### Decisions and departures from the plan

- **No `inotify` dev-dependency.** The plan names the `inotify` crate for test
  watches. The child fixture calls `libc::inotify_init1` and
  `inotify_add_watch` instead; `libc` is already a dependency, so no crate
  is added. The R9 table's "Phase 3 adds one dev-dependency" no longer holds.
- **The `/proc` reader also compiles in Unix test builds**
  (`#[cfg(any(target_os = "linux", all(test, unix)))]`). Shipped code stays
  Linux-only. Tests point the backend at a synthetic `/proc` whose `fd/<n>`
  and `cwd` entries are symlinks; `stat` and `readlink` treat those like the
  kernel's magic links. That way the `fdinfo` matrix and the permission/race
  tests run on macOS too, through the public report.
- **Matrix rows "every line bad" and "zero-length record".** The plan says
  "`partial` with zero successes". The Phase 2 core makes a mechanism with
  attempts and no successes `failed`, so a process whose only inotify record
  is unusable reports `inotify` `failed` (attempted 1, succeeded 0, limitation
  kept). With other processes inspected successfully the mechanism is
  `partial`. In neither case is it `complete` or empty, which is the row's
  intent. Successes count classified descriptors and valid registrations.
- **Matrix row "duplicate registration".** The plan says both copies are kept.
  The kernel never repeats a `wd` within one inotify instance, and the core
  already collapses evidence that is identical in every field. So an
  identical repeated line yields one record. The same `wd` with a different
  mask (or any other differing field) yields two records, and the test
  asserts both cases.
- **Ancestor watches are a scope note, not a gap.** They produce an
  `ancestor_watch` limitation on `inotify` (pid and `fd/wd` example) without
  making coverage `partial`; `Limitations::has_gaps` excludes that kind.
  Otherwise, an editor watching a checkout's parent would make every query
  inside it `partial`.
- **Mount namespaces become a device check.** `stat` through
  `/proc/<pid>/fd` returns kernel-global `(st_dev, st_ino)`, so descriptor and
  cwd identities compare reliably across mount namespaces. The unreliable
  case is inotify `sdev` on btrfs subvolumes and overlayfs, where `st_dev`
  differs from the superblock device. A registration whose inode is in the
  tree but whose decoded device is not adds a `device_identity_unreliable`
  limitation (coverage `partial`) and is never matched on inode alone.
- **Descriptor replacement** is detected when `fdinfo` prints `ino:` and it
  differs from the inode `stat` saw: the descriptor is dropped with a
  `descriptor_replaced` limitation. Kernels without the `ino:` line cannot
  show replacement this way.
- **Closed descriptors and zombies.** A descriptor that disappears between
  the listing and its `stat` (`ENOENT`/`ESRCH`) is no longer usage and is not
  a gap. A missing cwd counts as inspected with no evidence; the start-time
  re-read tells an exit apart.
- **Access is optional detail.** An unreadable `fdinfo` or malformed `flags`
  line on a matched descriptor leaves `access: null` with no limitation; the
  identity match stands.
- **Unreadable `stat` during enumeration.** The process is still a candidate,
  with no token (so `identity_uncertain`). An `identity_uncertain` limitation
  makes enumeration `partial`.
- **Counters.** `descriptor_inspections` counts each `/proc/<pid>/fd/<n>`
  entry; `watch_registration_reads` counts each anonymous-inode `fdinfo` read.
  A matched descriptor's access-flag read is part of its descriptor
  inspection.
- **`retained_handle` stays false.** The backend keeps no `/proc/<pid>`
  handle between enumeration and enrichment; enrichment relies on the
  `starttime` token.
- **`backend.rs`'s `#![cfg_attr(not(test), allow(dead_code))]` stays.**
  macOS and Windows non-test builds still construct no inspection results.

### `fdinfo` input-robustness matrix

Asserted by `the_fdinfo_reader_gives_every_malformed_shape_a_defined_outcome`,
which works from the captured fixture. One base edit aims `wd:3` at
`app/file.txt` and `wd:2` at the root directory; `wd:1` keeps its captured,
out-of-scope identity. Each cell then makes one further edit, applied to the
`wd:3` line unless noted. Every cell is asserted through `PathUsageReport`:
`wd:3`/`wd:2` evidence counts, `inotify` status, and the `malformed_record`
count. Every cell also asserts that `open_handles` stays `complete` and the
outcome `usable`.

| Cell | `wd` | `ino` | `sdev` | `mask` | Outcome |
| --- | --- | --- | --- | --- | --- |
| control | valid | valid | valid | valid | `wd:3` + `wd:2` evidence, `complete` |
| absent | removed | removed | removed | removed | line malformed, `wd:2` kept, `partial` |
| empty value | `wd:` | `ino:` | `sdev:` | `mask:` | malformed (never 0), `partial` |
| wrong type | `x`, `+3` | `zz` | `zz` | `zz` | malformed, `partial` |
| wrong type, every line | - | `zz` on all three | - | - | no evidence, 3 malformed, `failed` |
| out of range | - | 17 hex digits | `100000000` (> 32-bit `dev_t`) | - | malformed (never truncated) |
| duplicate key | `wd:3` twice | `ino:1` before / after the real one | - | - | malformed (never first/last-wins) |
| empty record | file has no `inotify` lines | | | | not inotify, no limitation, `complete` |
| zero-length record | `""` | | | | 1 malformed, `failed` |
| duplicate registration | identical line repeated | | | | one record (see departures) |
| same `wd`, different mask | | | | | two records |
| garbage line | `zzz` appended | | | | 1 malformed, all three valid lines kept |
| trailing token | ` garbage` on the line | | | | that line malformed |
| unknown field | ` foo:1` appended | | | | accepted, `complete` |

Mutation check: removing the duplicate-key rejection fails the matrix at
"duplicate ino, first" (first-wins). The check was restored. Smell grep: no
`serde(default)` or `unwrap_or_default()` in `linux.rs`. Every `.ok()` and
`filter_map` is one of three cases:

- inside a digit-checked parse helper whose `None` the caller turns into a
  malformed record;
- an unreadable `starttime`, which conservatively means "vanished";
- the ancestor-identity set, which only decides a scope note.

### Requirement-to-test mapping

All tests are L1 unit tests in the `sniff` lib target, with no tier marker
and default features. `linux_tests` runs on every Unix host;
`linux_tests::live` runs on Linux and WSL2.

| Requirement | Tests |
| --- | --- |
| Leaders only; threads are not processes | `only_numeric_proc_entries_are_processes_and_comm_may_hold_parentheses`, `live::the_querying_process_is_listed_once_and_its_threads_are_not_processes` |
| `starttime` token, `comm` with parentheses, unreadable stat | `only_numeric_proc_entries_are_processes_and_comm_may_hold_parentheses` |
| Controlled child: open file, cwd, live inotify watches with correct dev/inode | `live::a_controlled_child_is_found_by_its_descriptors_cwd_and_watches` |
| Multiple registrations on one descriptor preserved | same live test (3 distinct `wd`s, one descriptor), `a_watch_preserves_its_mask_descriptor_and_identity` |
| Sibling-prefix rejection | live test (`app-copy` open and watched), `open_descriptors_and_the_cwd_are_matched_by_identity_with_access_flags` |
| Ancestor watch excluded and reported | live test, `a_watch_on_an_ancestor_is_reported_as_outside_the_query` |
| Hard-link alias; unlinked `(deleted)` text descriptive only | live test, `a_hard_link_outside_the_tree_reports_the_in_scope_path_and_the_alias` |
| Target-only keeps the target watch only | `live::target_only_reports_the_target_watch_and_not_descendant_usage`, `target_only_keeps_a_watch_of_the_target_and_ignores_descendants` |
| `sdev` normalization (nonzero major) | `kernel_device_numbers_are_normalized_to_stat_encoding` (cross-checked against `libc::makedev` on Linux); live WSL2 run on ext4 (major 8) |
| Device mismatch never matched on inode alone | `a_watch_on_an_in_scope_inode_of_another_device_is_a_visible_limitation` |
| Access flags (`O_RDONLY`/`O_WRONLY`/`O_RDWR`/`O_PATH`) | `open_descriptors_and_the_cwd_are_matched_by_identity_with_access_flags` |
| Non-inotify anon descriptors; counters | `other_anonymous_descriptors_are_read_but_carry_no_registrations` |
| Permission denial: whole table, and listed-but-denied entries | `a_denied_descriptor_table_is_a_visible_gap_not_an_empty_result`, `denied_descriptors_inside_a_listed_table_are_counted` (non-root) |
| Descriptor replacement | `a_descriptor_replaced_during_inspection_is_dropped_and_reported` |
| Process exit and PID reuse during inspection | `a_process_that_exits_before_inspection_has_vanished`, `a_reused_pid_discards_everything_read_during_inspection` |
| Budget expiring between descriptors | `a_budget_expiring_between_descriptors_keeps_what_was_read` |
| Genuine caller usage kept; detector handles excluded | `live::the_querying_process_is_listed_once_and_its_threads_are_not_processes` |
| Shipped Linux mechanisms; fanotify/polling `unsupported` | `live::the_shipped_linux_backend_inventories_descriptors_cwd_and_inotify`, `open_descriptors_and_the_cwd_are_matched_by_identity_with_access_flags` |
| `fdinfo` matrix | `the_fdinfo_reader_gives_every_malformed_shape_a_defined_outcome` |

### Gates run

- macOS (local): `just test` in `sniff/` passed twice, 3204 run, 3204 passed,
  32 skipped. One earlier invocation exited 101 without printing any test
  result or summary (it stopped before nextest ran tests); its output was not
  captured and it did not reproduce. `just lint` clean. `cargo clippy -p
  sniff --all-targets [--features remote] -- -D warnings` and `cargo clippy
  -p sniff-cli --all-targets -- -D warnings` clean.
- Linux (`just cross-check sniff --os linux filesystem::query::`): 75/75,
  including the 4 live tests (all passed on the first run).
- WSL2 (`just cross-check sniff --os wsl filesystem::query::`, nextest
  archive path): 75/75, including the live tests on ext4 (major 8). This
  confirms the `sdev` decode against a real nonzero major.
- Native Windows (`just cross-check sniff --os windows filesystem::query::`):
  52/52. The Linux module is not compiled there. The build's warnings
  (`KEY_ALL_ACCESS`, `stage_raw_path`, unused `index`) are in unrelated files
  and predate this phase.
- Linux clippy: `cross-check` has no lint mode, so `cargo clippy -p sniff
  --all-targets -- -D warnings` and `--lib` ran over `ssh -o BatchMode=yes
  build-linux` in the clone that `cross-check` had just synced. Both are
  clean. This matters because `linux.rs` non-test code compiles only there.
- No pre-existing failures; no skipped requirement.

## Phase 4

Phase 4 builds the macOS backend, `LibprocBackend` in
`sniff/lib/src/filesystem/query/macos.rs`, and returns it from
`backend::platform()` on macOS. Windows keeps the interim `PlatformBackend`
(now `#[cfg(not(any(target_os = "linux", target_os = "macos")))]`). It uses
direct `libc` FFI plus two local `#[repr(C)]` structs and two constants, as
S1 decided. No new runtime or dev dependency is added.

### What was built

| File | Contents |
| --- | --- |
| `macos.rs` | `LibprocBackend<S: Source>`: enumeration with start-time tokens and names, descriptor listing with buffer growth, per-vnode classification (access, `event_only`, unlinked flag), cwd, and the post-inspection start-time check. `Source` is a five-method trait over the `libproc` calls. `ffi::Libproc` (macOS only) implements it with `proc_listallpids`, `proc_pidinfo` (`PROC_PIDTBSDINFO`, `PROC_PIDLISTFDS`, `PROC_PIDVNODEPATHINFO`), and `proc_pidfdinfo` (`PROC_PIDFDVNODEPATHINFO`) |
| `macos_tests.rs` | 15 scripted-source tests that run on every Unix host, plus 4 live tests in `macos_tests::live` (macOS) and the `#[ignore]` child fixture `usage_child` |
| `backend.rs` | Shared `Tally`, moved from `linux.rs`; `platform()` arm for macOS; interim backend narrowed to non-Linux, non-macOS; the macOS branch of `platform_mechanisms` removed; the `dead_code` comment updated |
| `linux.rs` | Uses the shared `Tally`; keeps its `io::Error` helpers (`io_problem`, `whole`) in a local `impl Tally` |
| `mod.rs` | `mod macos` (`any(target_os = "macos", all(test, unix))`) and `mod macos_tests` (`all(test, unix)`) |
| `tests.rs` | `the_shipped_backend_reports_unsupported_until_a_native_backend_exists` gated to non-Linux, non-macOS |

Per process the backend does this:

1. The task info (`PROC_PIDTBSDINFO`) read during enumeration gives the
   creation token (start time in microseconds), the name (`pbi_name`, else
   `pbi_comm`), and `pbi_nfiles`.
2. List descriptors with a capacity of `pbi_nfiles + 16`, or 256 when the
   task info was unreadable. Double the capacity while the list comes back
   full, up to 2^20 entries.
3. For each vnode descriptor, `PROC_PIDFDVNODEPATHINFO` gives `(dev, ino)`,
   the link count, the kernel's path, and `fi_openflags`. Access comes from
   `FREAD`/`FWRITE`; `event_only` comes from `O_EVTONLY` (`0x8000`). Every
   macOS handle carries `event_only: Some(true|false)`; Linux keeps `None`.
   A vnode with no links left is marked deleted, so its former path is
   never used as a match.
4. `PROC_PIDVNODEPATHINFO` gives the cwd. No cwd vnode (the kernel task)
   counts as inspected with no evidence.
5. Re-read the start time. If it changed or the process is gone, both
   mechanisms report `Vanished` and nothing read is kept.

### Decisions and departures from the plan

- **Candidate source: not used.** As S1 found, `proc_listpidspath` matches
  only the queried vnode, never a tree. A full descriptor and cwd scan runs
  instead. The plan task is checked off with this resolution.
- **Another user's process has no creation token.** A live probe on this host
  showed that `PROC_PIDTBSDINFO` returns `EPERM` for PID 1, just like the
  descriptor and cwd flavors. `PROC_PIDT_SHORTBSDINFO` succeeds but carries
  no start time. Such a process stays a candidate with no token and *no*
  enumeration limitation: the PID list is whole, and the same check denies its
  detail, so it cannot contribute evidence. Inspection reports it as
  `permission_denied` on `open_handles` and `working_directories`, which are
  then `partial` for any non-root caller. A task-info failure other than
  `EPERM`/`ESRCH` is an `identity_uncertain` enumeration limitation, as on
  Linux.
- **Device encoding.** `vst_dev` is `u32`, while `std`'s
  `MetadataExt::dev()` sign-extends macOS's `i32` `st_dev`. The FFI layer
  converts with `as i32 as u64`, so a device with the high bit set still
  matches the tree index. The live test asserts identity equality against
  `std::fs::metadata`. No local device has the high bit set, so the
  sign-extension case is covered by reasoning, not a test.
- **`libproc` failure convention.** These calls return 0 and set `errno`.
  The FFI layer zeroes `errno` before each call, so a `PROC_PIDLISTFDS`
  result of 0 with `errno` 0 is an empty table, not an error. `EPERM`/`EACCES`
  map to denied; `ESRCH`/`EBADF`/`ENOENT` map to gone. Short writes and other
  errors are failures carrying the OS message.
- **Non-vnode descriptors** (sockets, pipes, kqueues) count as inspected
  successes and increment `descriptor_inspections`, mirroring Linux, which
  counts every `/proc/<pid>/fd` entry.
- **Self-inspection.** No `libproc` call opens a handle, and the tree walk
  has closed before inspection. So the backend has nothing of its own to
  omit; the live test asserts that the querying process reports exactly its
  genuine open file.
- **Alias test location.** The plan's `/var` vs `/private/var` test creates
  its scene in `/var/tmp` rather than `$TMPDIR`. A CI runner may point
  `$TMPDIR` outside `/var`; `/var/tmp` always lives behind the `/var` link.
- **`Tally` is shared.** Phase 3's hand-off suggested reusing it. Moving it
  to `backend.rs` changes no Linux behavior; all 75 earlier query tests
  still pass on Linux and WSL2-equivalent paths.

### Requirement-to-test mapping

All tests are L1 unit tests in the `sniff` lib target, with no tier marker
and default features. `macos_tests` runs on every Unix host;
`macos_tests::live` runs on macOS.

| Requirement | Tests |
| --- | --- |
| Controlled child: open file and cwd found, with the correct dev/inode | `live::a_controlled_child_is_found_by_its_descriptors_and_cwd_through_the_var_alias` |
| `/var` vs `/private/var` alias: requested and resolved spellings kept; kernel path canonical | same live test |
| Event-only opens stay `open_handle` with `event_only` | same live test, `open_vnodes_and_the_cwd_are_matched_by_identity_with_access_and_event_only` |
| Access from open flags (read, write, read-write) | `open_vnodes_and_the_cwd_are_matched_by_identity_with_access_and_event_only`, live test |
| Sibling-prefix (`app-copy`) and ancestor rejection | `open_vnodes_...` (scripted), live test (both open in the child) |
| FSEvents and polling `unsupported` with a reason; mechanism order | `open_vnodes_...`, `live::the_shipped_macos_backend_inventories_descriptors_and_cwd` |
| Target-only keeps the target and drops descendants and cwd | `target_only_ignores_descendant_handles_and_cwd`, `live::target_only_reports_the_target_and_not_descendant_usage` |
| Hard-link alias outside the tree | `a_hard_link_outside_the_tree_reports_the_in_scope_path_and_the_alias` |
| Unlinked vnode never matched by its former path | `an_unlinked_file_is_matched_by_identity_and_not_by_its_former_path` |
| Permission denial recorded, never "no matches" | `a_denied_process_is_a_visible_gap_not_an_empty_result`, `only_denied_processes_make_the_outcome_unavailable`, `live::the_shipped_macos_backend_...` (non-root: denial count > 0) |
| Denied or failed descriptors inside a listed table; a closed descriptor is not a gap | `denied_and_failed_descriptors_inside_a_listed_table_are_counted`, `a_closed_descriptor_alone_leaves_coverage_complete` |
| Buffer grows while the descriptor list is full | `a_full_descriptor_list_is_reread_with_a_larger_buffer`, `an_unreadable_table_size_starts_from_the_default_capacity` |
| Enumeration failure and races | `a_failed_process_list_fails_enumeration_and_attempts_nothing`, `a_process_that_exits_after_listing_is_not_a_candidate`, `an_unreadable_table_size_...` (identity uncertain) |
| PID reuse and exit during inspection | `a_reused_pid_discards_everything_read_during_inspection`, `a_process_that_exits_during_inspection_has_vanished` |
| Budget expiring between descriptors | `a_budget_expiring_between_descriptors_keeps_what_was_read` |
| Genuine caller usage kept, nothing of the query's own | `live::the_querying_process_keeps_its_genuine_usage_only` |
| Counters: every listed descriptor counts once | `open_vnodes_...` |
| No test depends on a live FSEvents watcher being invisible (spec item 5) | none creates one |

The Input Robustness Matrix does not apply. The backend reads fixed-layout
kernel structs through FFI, not a file format or configuration; each
malformed outcome (short write, `errno` class) has a defined fault and is
covered above.

Mutation checks: making the descriptor listing never re-read fails
`a_full_descriptor_list_is_reread_with_a_larger_buffer`. Hard-coding
`event_only: Some(false)` fails four tests, two of them live. Both checks
were restored.

### Gates run

- macOS (local): `just test` in `sniff/` passed, 3222 run, 3222 passed,
  33 skipped. `just lint` is clean. `cargo clippy -p sniff --all-targets
  [--features remote] -- -D warnings` and `cargo clippy -p sniff-cli
  --all-targets -- -D warnings` are clean after one fix
  (`cloned_ref_to_slice_refs` in `macos_tests.rs`). The query tests passed
  three repeated runs (89/89), including the live tests.
- Linux (`just cross-check sniff --os linux filesystem::query::`): 90/90,
  including the scripted macOS tests and the Linux live tests on the shared
  `Tally`. Strict clippy (`--all-targets` and `--lib`, `-D warnings`) ran over
  `ssh -o BatchMode=yes build-linux` in the synced clone
  (`.../shazam--fix-wt-message/rusty-biscuit`) and is clean.
- Native Windows (`just cross-check sniff --os windows filesystem::query::`):
  52/52. Neither Unix backend compiles there; the interim backend still
  reports `unsupported`.
- WSL2 was not re-run: no Linux code path changed apart from the `Tally`
  move, which the Linux leg covers.
- `just check-tier-coverage sniff`: nothing stranded.
- An accidental `just lint` from the repository root (monorepo-wide) was
  stopped part way; it was not a gate for this phase and its partial output
  was discarded.
- No pre-existing failures; no skipped requirement.

## Phase 5

Phase 5 builds the native Windows backend, `ModuleBackend` in
`sniff/lib/src/filesystem/query/win32.rs`, and returns it from
`backend::platform()` on Windows. The interim `PlatformBackend` is gone;
an `UnsupportedBackend` (polling only, outcome `unsupported`) remains for
targets that are none of Linux, macOS, and Windows, so they still compile.

**The R2 decision is still unanswered, so the Handle owners task was not
built.** The spec's second `human_review_items` entry (Option A: omit
Windows handle evidence; Option B: a killable helper process) has no author
answer. Phase 4 left the instruction "if you are started anyway without an
answer, stop and report rather than picking one". This phase therefore built
only what both options need: loaded modules, process identity through a
retained handle, and the unsupported records. `open_handles` keeps reporting
`unsupported` on Windows, as it did before this phase, now with the hang as
its reason. That is the current state under either option, not a choice of
Option A: Option B would add handle evidence on top of this backend, and
Option A would leave it as is. The spec keeps `human_review: true`.

### What was built

| File | Contents |
| --- | --- |
| `win32.rs` | `ModuleBackend<S: Source>`: enumeration that opens each PID once (creation `FILETIME` token, image name, `start_time`), module listing with a size-retry loop, per-module path reads, a component-aware pre-filter, exit detection on failure, handle retention through enrichment, and `details` read through the held handle. `Source` is a seven-method trait; `ffi::Win32` (Windows only) implements it with `EnumProcesses`, `OpenProcess`, `GetProcessTimes`, `QueryFullProcessImageNameW`, `GetExitCodeProcess`, `EnumProcessModulesEx(LIST_MODULES_ALL)`, `GetModuleFileNameExW`, and the token SID reader |
| `win32_tests.rs` | 18 scripted-source tests that run on every host, 3 Windows-only spelling tests in `win32_tests::spellings`, 2 live tests in `win32_tests::live`, and the `#[ignore]` child fixture `usage_child` |
| `backend.rs` | Windows `platform()` arm; interim backend replaced by `UnsupportedBackend` for other targets; `dead_code` allowance extended to Windows test builds (no Unix backend compiles there, so `Tally::note`/`vanished` are unused) |
| `mod.rs` | `mod win32` (`any(windows, test)`) and `mod win32_tests` (`test`) |
| `identity.rs` | `io_error` (HRESULT unwrapping) made `pub(crate)` for the FFI fault mapping |
| `tests.rs` | `the_shipped_backend_reports_unsupported_until_a_native_backend_exists` deleted; `win32_tests::live::the_shipped_windows_backend_inventories_loaded_modules` replaces it |
| `os/user.rs`, `os/mod.rs` | The Windows token reader is generalized to `process_user_sid(HANDLE)`; `current_user_id` calls it with the current process and behaves as before |
| `performance/counters.rs` | `filesystem.query.module_inspections` |
| `Cargo.toml` | `windows` feature `Win32_System_ProcessStatus` |

Per process the backend does this:

1. At enumeration, `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION |
   PROCESS_VM_READ)`, falling back to the limited right alone when memory
   access is denied. The handle is kept in the backend. An invalid-PID error
   (an exited process, or PID 0) drops the candidate. A denial keeps it
   with no token. Any other failure keeps it with no token and an
   `identity_uncertain` enumeration limitation.
2. At inspection, list modules with a capacity of 256. While the reported
   count exceeds the buffer, re-read at the count plus 16, up to 8 reads and
   65,536 modules; past either bound, the process fails rather than
   silently dropping modules.
3. Read each module path. A path whose components cannot lie under a root
   spelling is dropped without a lookup; the rest become path-only
   `loaded_module` evidence that the core looks up by identity.
4. If the process holds nothing in scope, close its handle now. Otherwise
   keep it until the query returns, so enrichment's `details` (image and
   token SID) and the creation-time recheck read the same lifetime.

### Decisions and departures from the plan

- **Handle owners not built (R2 unanswered).** See above. The plan task is
  left unchecked with a "Blocked" note. The Windows tests task is left
  unchecked with a "Partly done" note, because its open-file and
  directory-handle child test needs handle owners.
- **No post-inspection start-time check.** The Unix backends re-read the
  start time after inspection to catch PID reuse. Windows cannot reuse a PID
  while a handle to the process is open, and the backend holds one through
  inspection. Instead, a failed module read checks `GetExitCodeProcess`:
  after an exit it is `Vanished` (`process_disappeared`), otherwise
  `inspection_failed`.
- **Enrichment through the handle, not `sysinfo`.** The plan requires the
  handle to be retained for the lifetime of enrichment. `details` therefore
  reads the image and the token's account SID through the held handle and
  never looks the PID up again. A process without a held handle cannot be
  enriched; that case is unreachable for a process with evidence.
- **Pre-filter, not a match, inside the backend.** `macos.rs` calls
  `match_object` per vnode, which is a cheap identity check there. For a
  path-only module, the same call would be a filesystem lookup, run again by
  the core, doubling `path_lookups`. The backend uses
  `matching::may_contain` against the root spellings instead, so each
  candidate module is looked up exactly once.
- **Module name `win32`, not `windows`.** A local module named `windows`
  would shadow the `windows` crate wherever `use super::*` brings it into
  scope.
- **Architecture failures.** `ERROR_PARTIAL_COPY` (299) while the process is
  running is reported as `inspection_failed`, with a detail naming the
  likely causes (starting, protected, or another architecture). A 64-bit
  build reads 32-bit (WOW64) modules through `LIST_MODULES_ALL`, so only a
  32-bit build reading a 64-bit process hits the architecture case. No
  32-bit build exists to test it.
- **New counter** `filesystem.query.module_inspections`. The existing
  `descriptor_inspections` is documented as descriptors or handles, and a
  module is neither.
- **UNC spelling** is covered by the Phase 2 pre-filter test
  (`windows_spellings_are_compared_by_component_not_by_string`). It is not
  covered by a live lookup, which would depend on administrative shares
  (`\\localhost\C$`).
- **Case-sensitive directory test fails rather than skips** where
  `fsutil file setCaseSensitiveInfo` is refused, because an L1 skip reads as
  a pass (`rust-testing`). It passes on build-win-native. Whether hosted
  `windows-latest` allows it is unverified; Windows runs on pushes to `main`,
  so a failure there is fixed forward.

### Requirement-to-test mapping

All tests are L1 unit tests in the `sniff` lib target, with no tier marker
and default features. `win32_tests` runs on every host;
`win32_tests::spellings` and `win32_tests::live` run on Windows.

| Requirement | Tests |
| --- | --- |
| Modules from the tree are `loaded_module` evidence; sibling-prefix and parent modules are not; counters | `modules_loaded_from_the_tree_are_evidence_and_others_are_not` |
| Distinct `loaded_modules` coverage; fixed unsupported records (open handles, cwd, `ReadDirectoryChangesW`, polling) with reasons; order | `the_windows_mechanisms_are_fixed_with_reasons_for_each_unsupported_one`, `live::the_shipped_windows_backend_inventories_loaded_modules` |
| Size retry; no silently dropped modules | `a_module_list_larger_than_the_buffer_is_reread_in_full`, `a_module_list_that_never_fits_fails_instead_of_dropping_modules` |
| Access denied recorded, never "no matches" | `a_process_that_cannot_be_opened_is_a_visible_gap_not_an_empty_result`, `only_denied_processes_make_the_outcome_unavailable`, `a_process_opened_without_memory_access_keeps_its_identity_and_denies_its_modules`, `live::the_shipped_windows_backend_...` (System process gap) |
| Per-module faults; unloaded module is not a gap | `module_read_faults_inside_a_listed_set_are_counted`, `a_module_unloaded_since_the_listing_leaves_coverage_complete` |
| Process exited vs inspection failed | `a_module_failure_after_the_process_exits_is_a_disappearance_not_a_failure`, `a_process_that_exits_before_it_is_opened_is_not_a_candidate` |
| Enumeration failure and open failure | `a_failed_process_list_leaves_modules_not_attempted`, `an_open_failure_is_an_identity_uncertain_enumeration_gap` |
| Creation time as the token; `start_time` | `creation_filetimes_convert_to_start_times`, `modules_loaded_from_the_tree_...`, live child test |
| Handle retained through enrichment; closed early otherwise; no reopen | `handles_close_after_inspection_unless_the_process_holds_something_in_scope`, `a_process_opened_without_memory_access_...` |
| Lifetime check at enrichment keeps evidence without new identity | `a_changed_creation_time_at_enrichment_keeps_evidence_without_new_identity` |
| Target-only | `target_only_matches_a_module_that_is_the_target_file` |
| Budget expiring between modules | `a_budget_expiring_between_modules_keeps_what_was_read` |
| Verbatim, case-variant, and 8.3 short-name spellings | `spellings::verbatim_case_variant_and_short_name_module_paths_match_the_long_path` |
| Case-sensitive directory | `spellings::a_case_sensitive_directory_matches_only_the_module_its_spelling_names` |
| UTF-16 path with an unpaired surrogate (matching, Windows) | `spellings::a_module_path_with_an_unpaired_surrogate_matches_and_round_trips`; serialization on any host stays `foreign_native_encodings_decode_only_when_representable` (Phase 2) |
| Controlled child (handshake, guard cleanup) found with correct identity | `live::a_controlled_child_is_found_by_its_loaded_executable` |
| No thread or worker | by construction: the backend spawns nothing; no test needed |
| Controlled child with open file and directory handle | **not built** (Handle owners, R2) |

The Input Robustness Matrix does not apply. The backend reads Win32 call
results through FFI, not a file format or configuration. Each fault class
(denied, gone, failed, undersized buffer) has a defined outcome tested above.

Mutation checks: making `list_modules` never re-read fails both size-retry
tests. Never retaining a handle fails three tests: the handle test, the
first module test (its `user` is no longer filled), and the enrichment
lifetime test. Both checks were restored, and 18/18 pass.

### Gates run

- macOS (local): `just test` in `sniff/` passed, 3240 run, 3240 passed,
  33 skipped. `just lint` is clean. `cargo clippy -p sniff --all-targets
  [--features remote] -- -D warnings` and `cargo clippy -p sniff-cli
  --all-targets -- -D warnings` are clean after two test-only fixes
  (`needless_borrow`, `field_reassign_with_default`).
- Native Windows (`just cross-check sniff --os windows filesystem::query::`):
  74/74, including the spelling, case-sensitivity, and live tests. The first
  run failed 8 tests because `matched_paths` are under the verbatim
  `\\?\` canonical root on Windows (existing core behavior); the tests now
  compare against `canonicalize()`. Recorded in the `os` skill.
- Windows clippy (`ssh -o BatchMode=yes build-win-native`, PowerShell, in the
  synced clone): `cargo clippy -p sniff --lib -- -D warnings` is clean, so
  `win32::ffi` lints cleanly. `--all-targets` fails only on lints in
  unrelated files that predate this phase (`KEY_ALL_ACCESS` in
  `programs/windows_apps.rs`, unused `index` in `executable_index.rs`,
  `stage_raw_path` and `set_readonly(false)` in `git_parity.rs`,
  `merge_conflict_prediction.rs`, and `remote_refresh.rs`). CI lints on Linux
  only.
- Linux (`just cross-check sniff --os linux filesystem::query::`): 108/108,
  including the scripted Windows-backend tests. Strict clippy (`--all-targets`
  and `--lib`, `-D warnings`) over `ssh -o BatchMode=yes build-linux` is clean.
- WSL2 not run: no Linux code path changed, and the scripted Windows tests
  ran on Linux.
- `just check-tier-coverage sniff`: nothing stranded.
- No pre-existing failures in the tests run; no requirement skipped except
  the R2-blocked handle-owner work.

## Phase 6

Phase 6 builds the `sniff filesystem query <PATH>` command in `sniff-cli`.
The library is unchanged: the CLI resolves the target, calls
`query_path_usage` once, and projects the captured report to text or JSON.
R1 is still unanswered, so `--timeout` help and docs use the option-1
wording ("stops scheduling work"; "not a limit on when the command
returns"). The R2 Windows handle-owner question does not affect the CLI,
which renders whatever coverage a report carries.

### What was built

| File | Contents |
| --- | --- |
| `cli/src/args/filesystem.rs` (new) | `FilesystemSubcommand::Query(FilesystemQueryArgs)`: `PATH` as `OsString` with `ValueHint::AnyPath` (clap's dynamic path completion), `--target-only`, `--timeout <MS>` through `parse_timeout`, subcommand after-help on advisory meaning and exit codes |
| `cli/src/args/mod.rs` | `Filesystem` gains `subcommand` and `args_conflicts_with_subcommands = true`; top-level help lists the command; 8 parser unit tests (`args::tests::filesystem_query`) |
| `cli/src/commands/filesystem_query.rs` (new) | `run` (exit-code mapping, JSON/text projection, fallible stdout write), `resolve_target`, the replay seam, 4 unit tests |
| `cli/src/commands/mod.rs` | Dispatch right after the help check, before any detection plan; the three `std::env::args()` pre-parse scans changed to `args_os` |
| `cli/src/output/filesystem/query.rs` (new) | `render_path_usage` through `Prose`, `InlineProse`, and `UnorderedList`; `literal`/`neutralize`; R5 caps; space-only wrapping; `UNWRAPPED_WIDTH` |
| `cli/src/output/{mod.rs,filesystem/mod.rs}` | Module registration and re-exports |
| `cli/tests/l1/filesystem_query.rs` (new), `cli/tests/l1/main.rs` | 18 L1 integration tests |
| `cli/tests/l1/snapshots/l1__snapshots__help_output.snap` | Two added help lines (metadata header left as it was) |

### Decisions and departures from the plan

- **Replay seam is an environment variable, always compiled.** The plan asks
  for a hidden test seam that is not a public flag. `SNIFF_FILESYSTEM_QUERY_REPLAY`
  names a JSON report file; the command strictly deserializes it (the Phase 2
  reader rejects malformed shapes) and renders it instead of querying. Gating
  it behind `test-fixtures` would have stranded the tests locally, because
  local L1 runs without that feature (`local-features = []`). The test
  fixture already scrubs inherited `SNIFF_*` variables. Documented in
  `docs/cli/filesystem_query.md` as "not a supported interface".
- **Only sigil references go through `FileReference`.** Relative, absolute,
  and non-UTF-8 paths are passed to the library unchanged, so a relative path
  resolves against the invocation directory (spec) and no string conversion
  happens. `@ & ^ ~ vault:` resolve with `resolve_detailed`; because
  `biscuit-file` matches regular files only, the target is the first
  candidate whose disposition is `Matched` or `NonFile` (directories), in the
  reference's own order. `&`, `^`, and `@` get the enclosing Git root from
  `find_git_root`; no package-scope catalog, since that needs repository
  inventory. URLs are rejected before probing (`nonlocal_reference`).
  Consequence: an ordinary path containing `{{VAR}}` is not interpolated.
- **The original reference is shown in text, not added to JSON.** The
  report's `requested` field is what the library received (the resolved
  path for a reference). Text shows a `Reference:` line; JSON stays the
  unmodified library report, per "without CLI-side filtering or inference".
- **No `print_json_value`.** It prints with `println!` and
  `unwrap_or_default()`, which would write an empty line on a serialization
  failure and panic on a closed pipe. The command uses the same
  `attach_performance` to make one document, serializes to a string first,
  then writes with `write_all`/`flush` and returns I/O errors through the
  CLI error path.
- **Rendered `--perf` summary goes to stderr in every mode**, text included
  (spec: rendered performance diagnostics belong on stderr).
- **Typed errors bypass `main`'s `Error:` path.** The command prints its own
  neutralized `Error: …` on stderr, the JSON error object on stdout in JSON
  mode, and exits 1, so the diagnostic is not printed twice.
- **Overflowing `--timeout`.** `parse_timeout` rejects non-integers, zero,
  values above `u64::MAX`, and durations `Instant::now()` cannot add. `u64::MAX`
  milliseconds is representable on macOS, so it is accepted there (the
  library would then run without a practical limit); the first test draft
  expected exit 2 and was corrected.
- **Wrapping.** The default `WordWrap` breaks at `-` and `/` and hyphenates
  long words, which split temp-dir paths across lines at 80 columns. Text now
  wraps only at spaces, and when stdout is not a terminal it lays out at
  4096 columns so piped output never splits a path. On a terminal a path
  longer than the window still wraps.
- **Neutralization before escaping.** `Prose::escape_text` passes CSI and OSC
  sequences through, so `literal` first rewrites control characters (C0, DEL,
  C1) and bidirectional overrides to `\n`, `\u{1b}`, … and then escapes.
  Stderr diagnostics are neutralized too; JSON keeps the original values.
- **Non-UTF-8 argv panic fixed.** `run()` scanned `std::env::args()` for
  `--plain` (and two other helpers did the same), which panics on a
  non-UTF-8 argument before clap sees it. All three now use `args_os`.
- **Completions.** Sniff had no path or file-reference completer to reuse;
  `ValueHint::AnyPath` gives clap's dynamic path completion (verified with
  `COMPLETE=fish`). Reference-sigil completion (`@…`) is not offered.
- **Human display of Windows paths** keeps the report's `\\?\` spelling of
  `matched_paths`; no lexical simplification was added.

### Requirement-to-test mapping

Integration tests are in `sniff/cli/tests/l1/filesystem_query.rs` (the `l1`
binary, no tier marker, default features); unit tests are in the `sniff-cli`
lib target.

| Requirement | Tests |
| --- | --- |
| Text and JSON are projections of one retained report; JSON is the unfiltered report | `text_and_json_are_projections_of_one_retained_report` |
| `--json` beats `--plain` | `json_wins_over_plain` |
| Partial report: usable, exit 0 | `a_partial_report_is_usable_and_exits_zero`, `text_and_json_...` |
| Empty report: exit 0, says no matches is not proof | `an_empty_usable_report_exits_zero_and_does_not_claim_the_target_is_unused` |
| Skipped (`not_attempted`, budget), total failure, unsupported: exit 1, full report on stdout, empty stderr | `non_usable_reports_exit_one_with_the_full_report_on_stdout`, `a_skipped_report_says_the_budget_ran_out` |
| Match presence does not change the exit code | `match_presence_does_not_change_the_exit_code` |
| R5 truncation in text only, with omitted counts | `long_lists_are_shortened_in_text_only_and_say_how_much_was_left_out` |
| Hostile names/paths/messages (markup, ANSI CSI, OSC 8, BEL, newline, CR, bidi) shown literally in styled and plain text; JSON exact | `hostile_names_and_paths_are_shown_literally_and_kept_exact_in_json` |
| `--plain` has no escape codes and keeps coverage/limitations | `plain_text_carries_no_escape_codes`, `text_and_json_...` |
| Never overstates deletion contention | `text_and_json_...` (no "blocks"; "does not prove … will fail") |
| `--json --perf` one object; structured performance; rendered summary on stderr; no inventory (only `filesystem.query.*`/`filesystem.io.*` counters, one tree walk, one enumeration) | `a_live_json_perf_query_is_one_object_and_does_no_inventory` |
| No `performance` without `--perf`; stdout/stderr separation | `without_perf_there_is_no_performance_field` |
| `--target-only`, `--timeout` reach the library | `target_only_and_timeout_reach_the_library`, `args::tests::filesystem_query::target_only_and_timeout_parse` |
| Relative path resolves against the invocation dir; `--base` ignored | `a_relative_path_resolves_against_the_invocation_directory_not_base` |
| Runtime error JSON shape (`missing_target`, `reference_not_found`, `nonlocal_reference`); text mode stdout empty | `runtime_errors_have_the_stated_json_shape`, `commands::filesystem_query::tests::*` |
| Malformed replay is an error, never an empty report | `a_malformed_replay_is_an_error_not_an_empty_report` |
| Invalid args exit 2 with no stdout (missing path, bad timeouts, inventory flags on either side of `query`) | `invalid_arguments_exit_two_without_a_report`, `args::tests::filesystem_query::*` |
| Bare `filesystem` and its flags unchanged | `args::tests::filesystem_query::the_bare_filesystem_report_keeps_its_inventory_flags`, existing `cli::test_filesystem_subcommand_*`, `global_flags::scoped_flags_parse_for_supported_commands` |
| Non-UTF-8 argument stays native (Unix) | `a_non_utf8_argument_reaches_the_query_as_native_bytes`, `args::tests::filesystem_query::a_non_utf8_path_stays_native`, `commands::filesystem_query::tests::a_non_utf8_path_passes_through_without_conversion` |
| Non-UTF-8 directory queried and serialized losslessly (Linux; APFS rejects such names) | `a_non_utf8_directory_is_queried_and_serialized_losslessly` |

The Input Robustness Matrix applies only to the replay reader, which is the
Phase 2 report reader unchanged (its matrix is asserted there); the CLI adds
`a_malformed_replay_is_an_error_not_an_empty_report` for its own path.

Mutation checks: rendering without `neutralize` fails the hostile-name test;
mapping every report to exit 0 fails the non-usable and match-presence tests.
Both were restored and the suite is green.

### Gates run

- macOS (local): `just test` in `sniff/` passed, 3270 run, 3270 passed,
  33 skipped (after accepting the two-line help snapshot change). `just lint`
  is clean. `cargo clippy -p sniff-cli --all-targets [--features
  test-fixtures] -- -D warnings` is clean. The library was not changed.
- Linux (`just cross-check sniff-cli --os linux filesystem`): 138/138,
  including the non-UTF-8 directory test.
- Native Windows (`just cross-check sniff-cli --os windows filesystem`):
  134/134.
- WSL2 not run: the CLI has no WSL-specific path, and the Linux leg ran the
  same code.
- `just check-tier-coverage sniff`: nothing stranded.
- Manual checks on macOS: a `tail -f` child in a temp tree is reported by
  working directory and open handle, the `app-copy` sibling is not; `&sniff`,
  `^justfile`, `@README.md` resolve; `~` with a 300 ms budget reports
  `unavailable` (exit 1) because the home-tree walk exhausts the budget.
- No pre-existing failures; nothing skipped.

## Phase 1 addendum: author rulings and spikes S2d-S2f (2026-10-06)

This section is recorded after Phase 6. It answers both `human_review_items`
and reopens the Phase 5 "Handle owners" task. No shipped source changed.

### Rulings

- **R1: option 1 confirmed by the author.** Nothing changes: the library,
  CLI help, and docs already say "stops scheduling work". Whether `wt`
  accepts that contract is a separate decision outside this feature (R8).
- **R2: option D, chosen by the author, replaces options A, B, and C.**
  The question changes from "which file is this foreign handle?" to "which
  processes have this path open?" Sniff opens each tree entry itself and
  calls `NtQueryInformationFile(FileProcessIdsUsingFileInformation)`.
  Restart Manager uses this call internally. No foreign handle is
  duplicated or resolved, so the S2 hang does not apply. It needs no
  elevation and no helper executable. The plan's Handle owners task is
  rewritten to match.

### Spikes

All spikes ran on build-win-native in scratch crates outside the
repository. The elevated SSH session was used except where noted.

**S2d: hang, coverage, edge cases.** Each scenario ran 5 trials on `B:`
(ReFS) and 5 on `C:` (NTFS).

| Scenario | Result |
| --- | --- |
| Holder blocked in `LockFileEx` on a synchronous handle (the S2 hang) | No hang; the waiter's and lock holder's PIDs returned |
| Holder blocked in synchronous `ReadDirectoryChangesW` | No hang; the watcher's PID returned; our open and close did not wake the watcher |
| Synchronous pipe server in `ConnectNamedPipe` | Open fails fast (231). Opening the pipe path connected to the server as a client, so the backend must never open a non-file, non-directory path |
| Open file, open directory, cwd, loaded DLL, running exe | PID reported, on the exact path only. Nothing rolls up to ancestors, so ancestor matching is Sniff's job |
| Holder opened with share mode 0 | Still found |
| Delete pending (classic and POSIX) | Open fails with `STATUS_DELETE_PENDING`; the holder cannot be seen |
| Path longer than 260 characters | Works with `\\?\`; fails without it |
| Restart Manager | No directory support (error 5), no long-path support (error 29), slower per path, and a missing path returns success with an empty list. Not used |

**S2e: cost.** The host had 12 logical cores and about 100k system handles.

- Each call scans system-wide state. It costs about 4 ms plus about 25 µs
  per 1,000 system handles, whether or not the path has holders: 6 ms at
  100k handles, 29 ms at 1.1M. The file system (ReFS or NTFS) made no
  difference.
- Parallel calls scale almost linearly up to the core count: 5,051 entries
  take 30 s on 1 thread, 4.5 s on 8, and 3.1 s on 12. No kernel lock
  serializes them.
- On real checkouts, 12 threads cover about 3,200 entries in 2 s:
  - a checkout without `target\` (2,345 directories) completed all its
    directories in 1.4 s;
  - clones with `target\` (4,300-5,300 directories, 32k-53k entries)
    completed about 63% of their directories in 2 s;
  - with `target\` excluded, those clones' directories fit in 2 s.
  Files alone would need 10-35 s.

**S2f: permissions and caller exclusion.**

- A normal, non-elevated desktop session saw the same holders as the
  elevated one: 240 PIDs on `ntdll.dll`, including System (PID 4) and
  other users' processes, against 236 elevated; 177 against 174 on
  `System32`. The full S2d matrix gave the same results non-elevated.
- Holders that opened with attribute-only access, `SYNCHRONIZE` only, or no
  access at all were all reported, for both files and directories. The
  calling process never appears in its own results. That is caller
  exclusion, not a blind spot for low-access handles.

### Consequences for Phase 5

- Scheduling follows from the cost: directories first, then files;
  `target/`, `node_modules/`, and git-ignored entries last; a scoped,
  always-joined pool of `min(available_parallelism, 8)` threads. Eight
  threads gave 6.8× in S2e and leave capacity for interactive callers.
  When the budget expires, unreached entries make the mechanism `partial`.
- A cwd or `ReadDirectoryChangesW` watcher appears as a holder of its
  directory. It is reported as `open_handle` evidence on that directory.
  The separate cwd and subscription mechanisms stay `unsupported`, because
  Windows cannot tell those handles apart from other directory handles.
- New limitations to document: delete-pending entries are invisible; cost
  scales with the system's handle count; opening an entry on a hung network
  redirector can block, which is the same R1 overrun risk every OS carries.
