# Filesystem process usage queries

Query a file or directory tree to find observable processes using it:

```rust
use sniff::filesystem::query::{Outcome, PathUsageOptions, query_path_usage};
use std::path::Path;

let report = query_path_usage(Path::new("./checkout"), &PathUsageOptions::default())?;
if report.outcome != Outcome::Usable {
    // Discovery could not run here; read `report.coverage` for why.
}
```

The library and CLI support Linux/WSL2 descriptors, working directories, and
inotify registrations; macOS vnode descriptors and working directories; and
Windows file and directory handle owners and loaded modules. Each report
states which mechanisms ran and what discovery could not observe.

A directory query includes descendants by default;
`PathUsageOptions::default().target_only()` inspects the target alone. Normal
host and repository reports never perform this scan, and the query does no
repository inventory, network request, privilege elevation, process
termination, or caching.

A handle identifies usage; it does not necessarily prevent deletion. An empty
result does not establish that the target is unused.

## How a query runs

```mermaid
flowchart LR
    A[validate options] --> B[resolve root<br/>capture identity]
    B --> C[walk tree once<br/>collect identities]
    B --> S[or check the caller's<br/>supplied index]
    C --> D[enumerate processes]
    S --> D
    D --> P[Windows: query holders<br/>of each walked path]
    P --> E[inspect process mechanisms<br/>match by identity]
    D --> E
    E --> F[recheck root]
    F --> G[enrich identity]
    G --> H[coverage + outcome]
    B -. budget expired .-> X[root_validation_timeout error]
    C -. budget expired .-> H
    D -. budget expired .-> H
    E -. budget expired .-> H
```

One monotonic budget (default two seconds, `with_deadline` to change it)
covers every phase. When it expires the query **stops scheduling work** and
returns what it collected, with `budget_exhausted: true` and incomplete
coverage. A native call already running can overrun the budget, so the
deadline is not a guarantee of when the function returns.

The budget is also checked between the mechanisms of one process. On Linux
and macOS, a process's working directory is read only after its descriptors,
and only while the budget lasts. If the budget runs out during the descriptor
scan, the descriptors already read stay in the report, the working directory
is not read, and that process counts toward a `budget_exhausted` limitation
on `working_directories`. The coverage record is `not_attempted` when no
process's working directory was read, and `partial` otherwise. Each process's
start time is still rechecked after an expired inspection, because that check
is what attributes the evidence already read to the right process.

## You already walked the tree: supply its index

A caller that has just walked, hashed, copied, or inventoried a tree should
not pay for a second walk inside the query's budget. Hand the query the index
you built, and the budget goes to processes instead:

```rust
use sniff::filesystem::query::{
    FileIdentity, PathUsageOptions, TreeEntryKind, TreeIndex, query_path_usage,
};
use std::sync::Arc;

// Identities you read while walking, without following links.
let mut builder = TreeIndex::builder(root_identity);
builder
    .entry("src", TreeEntryKind::Directory, src_identity)
    .entry("src/main.rs", TreeEntryKind::File, main_identity)
    .entry("current", TreeEntryKind::Link, link_identity);
let index = Arc::new(builder.build()?);

let options = PathUsageOptions::default().with_tree_index(Arc::clone(&index));
let report = query_path_usage(checkout, &options)?;
assert_eq!(report.index_provenance, sniff::filesystem::query::IndexProvenance::Supplied);
```

The rules:

- **Entries are relative to the root**, spelled with native names (no UTF-8
  conversion), one per path: list every hard-link alias, and a link as itself
  (`TreeEntryKind::Link`), never its target. The root is implicit. `build()`
  rejects, naming the entry, an absolute path, a `..` or leading `.`
  component, an empty path, a path listed twice (`a/b` and `a//b/` are one
  path), and anything below a link. An empty index is valid: a root with no
  descendants.
- **The query still verifies the root.** It resolves the target and reads its
  identity as usual, then rejects with `invalid_option` an index whose root
  identity differs, a file target, and `target_only()`. The final root
  recheck still runs.
- **An index is a point-in-time capture.** Entries created after you built it
  are not matched, and the query does not reread its descendants. Callers
  decide how to handle changes made meanwhile; `wt remove`, for instance,
  watches the tree from before its capture and confirms its inventory before
  deleting.
- **Build once, share freely.** The options hold an `Arc`, so one index can
  serve several queries of the same tree with no copy; only matched paths are
  joined onto the root.
- **Say when it is incomplete.** `builder.partial("why")` makes the report's
  `tree_identity` coverage `partial` with an `incomplete_tree_index`
  limitation carrying your reason, which in turn marks every dependent
  mechanism `partial`.

The report records where the index came from in `index_provenance`:
`walked` (the query collected it), `supplied`, or `unknown` for a report
written before the field existed. A supplied index's `tree_identity` coverage
carries the caller's completeness, `null` counts, and a `reason` saying the
query did not read its descendants: provenance and coverage are separate, so
a complete supplied index is never reported as `walked`.

Supplying an index removes only the walk. On Windows the file-side holder
scan still opens each indexed regular file and directory (never a link), and
on a large tree that scan can still exhaust the budget; working-directory
discovery stays unsupported there.

## Errors versus reports

`query_path_usage` returns `Err(PathUsageError)` only when it cannot build a
report about a verified target. Each error has a stable `kind()`:

| Kind | When |
| --- | --- |
| `invalid_option` | Zero deadline, or one too large to form a deadline |
| `missing_target` | The path, or a root symlink's target, does not exist |
| `unsupported_target_kind` | A socket, FIFO, or device; Windows pipe namespaces are rejected before metadata lookup, which could connect to a waiting server |
| `root_identity` | The root's identity cannot be read |
| `root_validation_timeout` | The budget expired before the root identity was captured |

Every later problem (a denied process, an unreadable descendant, the budget
running out mid-scan) becomes a coverage status and a limitation in the report.

## Matching rules

Evidence is matched by file identity: `(st_dev, st_ino)` on Unix, the volume
serial plus 128-bit file ID on Windows.

- The tree is walked once per query, however many processes are inspected.
  Hidden and Git-ignored entries are in scope.
- A root symlink is resolved once; the report keeps the `requested` and
  `resolved` spellings. Descendant symlinks, junctions, and reparse points are
  indexed as themselves and never followed.
- A hard link outside the tree that names an in-scope file is a match:
  `matched_paths` lists every in-scope path (none is preferred) and
  `observed_path` keeps the spelling the OS reported.
- When a backend supplies only a path (no identity), the path must sit under
  the root component by component (`/work/app-copy` is outside `/work/app`)
  and is then looked up; the match is labeled `match_basis: path_lookup`. On
  Windows the lookup, not text comparison, settles drive/UNC/`\\?\`
  spellings, 8.3 short names, and case-sensitive directories. An object known
  to be deleted is never matched by its former path.
- That lookup runs as the backend reads each object, and the match travels
  with the evidence; building the report never looks a path up. So a budget
  that runs out later keeps every match already made, and an object the
  budget stopped before its lookup is counted as a `budget_exhausted` gap
  rather than reported unverified.

## Process identity

A PID can be reused after a process exits. Observations merge only when both
the PID and the native creation token match; different tokens stay separate
records. A record without a token (and no retained process handle) is marked
`identity_uncertain` and is never merged or enriched. Name, executable, and
user are read only after the creation token is re-read and found unchanged;
otherwise the evidence is kept without a name and a limitation explains why.

## Coverage and outcome

Each mechanism, plus the two prerequisites (`process_enumeration`,
`tree_identity`), gets a coverage record:

| Status | Meaning |
| --- | --- |
| `complete` | Finished over the visible processes and target scope with no known gap |
| `partial` | Started, but permissions, races, malformed records, an incomplete tree, an unverified root, truncation, or the budget left gaps. A watch on an ancestor of the target is noted as a limitation but is not a gap |
| `unsupported` | This OS or build cannot inventory the mechanism; `reason` says why |
| `failed` | Attempted, but no inspection succeeded |
| `not_attempted` | Supported but never started, for example because the budget ran out first; `reason` says why |

A gap in a prerequisite makes every dependent mechanism `partial`. The report's
`outcome` is decided by the library:

- `usable`: at least one usage mechanism is `complete`, or `partial` with a
  successful inspection or retained evidence. Matches may be empty.
- `unavailable`: usage mechanisms are supported, but none of them is usable.
- `unsupported`: no usage mechanism is supported here.

Prerequisites never make an outcome `usable` on their own.

| Environment | Discovery | Coverage limit |
| --- | --- | --- |
| Linux/WSL2 | Open descriptors, cwd, inotify registrations | Permissions; fanotify and polling are `unsupported` |
| macOS | Vnode descriptors and cwd, including observable event-only opens | Permissions (another user's processes are denied without root); FSEvents and polling are `unsupported` |
| Windows | File and directory handle owners; loaded modules (executables and DLLs) | Delete-pending paths, permissions, protected processes; cwd, `ReadDirectoryChangesW` subscriptions, and polling are `unsupported` |

For example, a Node process with its cwd in a checkout will appear as working
directory evidence. A Node process watching that checkout through FSEvents may
have no observable path association and be absent from matches.

Retained evidence is capped at 1,000 records per process and 10,000 per query;
a hit cap makes the mechanism `partial` with a `truncation` record and the
omitted count.

## Linux and WSL2

The backend reads `/proc` directly. WSL2 runs the same code and sees only
Linux processes, never native Windows ones.

- **Processes** are the numeric entries of `/proc`, which are thread-group
  leaders, so a thread is never reported as a process. The creation token is
  `starttime` (field 22 of `/proc/<pid>/stat`); the process name comes from
  the same file. If the start time changes between enumeration and the end of
  a process's inspection, the PID was reused and nothing read for it is kept.
- **Open descriptors** (`open_handles`) and the **working directory**
  (`working_directories`) are identified by `stat` through
  `/proc/<pid>/fd/<n>` and `/proc/<pid>/cwd`. The link text becomes
  `observed_path` only; for an unlinked file it reads `<path> (deleted)` and
  is never opened. Open access (`read`/`write`) comes from the `flags` line of
  `/proc/<pid>/fdinfo/<n>`; an `O_PATH` descriptor has neither.
- **inotify registrations** are read from the `fdinfo` of anonymous-inode
  descriptors. A descriptor is an inotify instance when its `fdinfo` has
  `inotify` lines; its event queue is never read. Each line is one
  registration, kept separately even when several share a descriptor:

  ```text
  inotify wd:1a ino:b84b5 sdev:800011 mask:fff ignored_mask:0
  ```

  becomes watch evidence with `descriptor`, `watch.watch_id: 26`,
  `watch.mask`, and `watch.recursive: false` (inotify has no recursive
  watches). Every field of the line is hexadecimal, `wd` included: the
  kernel writes watch descriptor 16 as `wd:10`, and `watch_id` reports the
  number, not the text. `sdev` is the kernel's device number (`major << 20 | minor`, here
  8:17) and is converted to the `st_dev` encoding before it is compared, so
  `sdev:800011` matches `st_dev` `0x811`. A watch has no path of its own:
  `observed_path` is always `null`.

A watch counts only when it names the target or an included descendant:

| Watched object | Result |
| --- | --- |
| The target, or a descendant in a recursive query | `watch_registration` evidence |
| The target directory under `target_only()` | Evidence; descendants are out of scope |
| An ancestor of the target (`/work` when querying `/work/app`) | No evidence; an `ancestor_watch` limitation on `inotify`, which stays `complete` |
| An in-scope inode number on a different device | No evidence; a `device_identity_unreliable` limitation, `partial` |
| Anything else (`/work/app-copy`) | Ignored |

The device check exists because btrfs subvolumes and overlayfs report a
`st_dev` that differs from the kernel device inotify records, so such a watch
may be in scope but cannot be proven to be.

Problems stay visible instead of reading as "no matches":

- A denied descriptor table (`/proc/<pid>/fd`, typical for another user's
  processes) makes that process `permission_denied` for `open_handles` and
  `inotify`. So does a table that lists but whose every entry is denied.
- A malformed `inotify` line is a `malformed_record` limitation naming the
  descriptor and line; valid lines beside it are kept. A field missing, empty,
  repeated, out of range, or not a number makes its line malformed; unknown
  `key:value` fields are ignored. A record that is empty, or whose every line
  is malformed, means that process's inotify inspection did not succeed.
- A descriptor whose `fdinfo` inode no longer matches what `stat` saw was
  replaced mid-inspection; it is dropped with a `descriptor_replaced`
  limitation.
- fanotify marks and polling watchers are `unsupported`. A complete
  descriptor scan is still not complete watcher discovery.

## macOS

The backend calls `libproc` (`proc_listallpids`, `proc_pidinfo`,
`proc_pidfdinfo`) directly; it reads no other process's memory and runs no
subprocess.

- **Processes** come from `proc_listallpids`. The creation token is the start
  time in microseconds since the epoch from `PROC_PIDTBSDINFO`, which also
  supplies the name. If it changes between enumeration and the end of a
  process's inspection, the PID was reused and nothing read for it is kept.
- **Open descriptors** (`open_handles`) are listed with `PROC_PIDLISTFDS`.
  A listing that fills its buffer may have been cut short, so it is read
  again with a larger buffer until it is not full. Each vnode descriptor is
  read with `PROC_PIDFDVNODEPATHINFO`; sockets, pipes, and kqueues carry no
  path and are counted but not matched. Identity is the vnode's
  `(dev, ino)`, in the same encoding `std::fs::Metadata::dev` reports; the
  kernel's path becomes `observed_path` only, and it uses the canonical
  spelling (`/private/var/...` for a target queried as `/var/...`).
- **Access and event-only opens.** `access` comes from the descriptor's open
  flags. A descriptor opened with `O_EVTONLY` (how kqueue-based watchers
  hold a file) is still `open_handle` evidence, with `event_only: true`;
  every other macOS descriptor has `event_only: false`. It shows that a
  process can be notified about the object, not that it is subscribed.
- **The working directory** (`working_directories`) comes from
  `PROC_PIDVNODEPATHINFO` and is matched the same way.
- **FSEvents** subscriptions are `unsupported`: macOS has no supported
  systemwide inventory of them, so a process watching through FSEvents alone
  can be absent from matches. Polling watchers are `unsupported` everywhere.

For example, a target `/var/tmp/app` held open read-write by PID 812, with
PID 812's cwd in `/var/tmp/app/sub`, yields one process record with two
pieces of evidence:

```text
open_handle        /private/var/tmp/app/file.txt   fd 3  read+write  event_only: false
working_directory  /private/var/tmp/app/sub
```

Problems stay visible instead of reading as "no matches":

- Without root, every per-process call for another user's process (for
  example `launchd`, PID 1) fails with `EPERM`. That process is
  `permission_denied` for `open_handles` and `working_directories`, which
  are then `partial`. Process enumeration itself stays `complete`: the PID
  list is whole, only the detail is denied.
- A descriptor that closes between the listing and its read is no longer
  usage and is not a gap. A descriptor that cannot be read for any other
  reason is a `permission_denied` or `inspection_failed` limitation naming
  it, and the rest of the table is still read.
- A process whose start time cannot be read for a reason other than
  permission has no creation token: enumeration records an
  `identity_uncertain` limitation and the record is kept apart.

## Windows

The backend lists processes with `EnumProcesses` and reads each one through
a process handle. It runs no subprocess; the file-side scan uses scoped
workers. It reports file and directory handle owners, and `loaded_modules`
for every executable and DLL a process has mapped. A module is observed by path only, so its path is looked up and
matched by identity (`match_basis: path_lookup`). That settles `\\?\`,
8.3 short-name, and case spellings, and a case-sensitive directory keeps
`Module.dll` and `module.dll` apart. Each module is looked up as soon as its
path is read: if the budget runs out after `tool.exe` matched but before
`helper.dll` was read, the report keeps the `tool.exe` evidence and its
process, and `loaded_modules` is `partial` with a `budget_exhausted`
limitation.

For example, a build tool started from `C:\work\app\bin\tool.exe` that has
also loaded `C:\work\app\bin\helper.dll` yields, for a query of
`C:\work\app`:

```text
loaded_module  \\?\C:\work\app\bin\helper.dll
loaded_module  \\?\C:\work\app\bin\tool.exe
```

Matched paths sit under the canonical root, which on Windows is the verbatim
`\\?\` spelling; `observed_path` keeps the spelling Windows reported.

- **Processes and identity.** Each listed PID is opened once with
  `PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ`, or with the limited
  right alone when memory access is denied. The creation token is the
  process's creation `FILETIME`, which also gives `start_time`; the image path
  gives `name` and `executable`. While that handle is open the PID cannot be
  reused.
- **Handle lifetime.** A process with no in-scope module or file-side handle evidence has its handle
  closed as soon as it has been inspected. A process holding something in
  scope keeps its handle until the query returns, so `user` (the account SID
  from its token) and the creation-time recheck are read through the same
  handle, never by opening the PID again.

```mermaid
flowchart LR
    A[EnumProcesses] --> B[OpenProcess per PID]
    B -->|denied| D[candidate without a token]
    B --> P[query file-side holders]
    P --> C[inspect modules]
    C -->|nothing in scope| E[close handle]
    C -->|in scope| F[enrich through the handle]
    F --> G[close when the query returns]
```

- **Module lists** come from `EnumProcessModulesEx(LIST_MODULES_ALL)`. A list
  larger than the buffer is read again at the reported size, so no module is
  dropped; a list that keeps outgrowing its buffer, or reports more than
  65,536 modules, fails that process instead.
- **Module paths** are the loader's own record of each module, the string
  `GetModuleFileNameExW` returns. That call walks the process's loader list
  from its head for every module, about 130 µs each, which was 90% of a
  query on a host with 130 processes and 5,300 modules (750 ms). Sniff reads
  the list once per process instead (`PEB.Ldr`, then each entry's
  `FullDllName`, the documented `winternl.h` fields): about 70 ms for the
  same host. A module the walk does not cover is read with
  `GetModuleFileNameExW` as before: a 32-bit process's own modules (only its
  64-bit loader list is walked), or every module of a process whose list
  could not be read whole (one still starting, or one loading a module
  during the walk). A module unloaded between the walk and its turn is still
  matched by the path the walk read, so evidence can only grow.
- **Unsupported mechanisms.** A working directory and a
  `ReadDirectoryChangesW` subscription are directory handles that cannot
  be distinguished from other directory handles, so those two mechanisms
  stay `unsupported`.
- **Open handles from the path side.** Sniff opens regular files and
  directories from the existing tree walk with attribute-only access and
  full sharing, and asks Windows which processes hold each path
  (`NtQueryInformationFile`, `FileProcessIdsUsingFileInformation`). It never
  duplicates another process's handles. The call returns even while a holder
  waits on a file lock or synchronous directory-change notification. Each
  holder becomes `open_handle` evidence matched by the identity read from
  the same file handle. Its `observed_path` is the queried path; Windows does
  not report which spelling the holder used. Replacement after the walk is
  reported as a gap.
  The kernel excludes the calling process from this mechanism. Loaded modules
  can still identify the querying process's own mapped files.

  For example, a query of `C:\work\app` while a dev server watches
  `C:\work\app\src` reports:

  ```text
  open_handle  \\?\C:\work\app\src
  ```

  Each call costs a few milliseconds, and more on a host with many open
  handles. Sniff therefore visits directories first, then files, and
  visits `target`, `node_modules`, and git-ignored entries last. It uses
  at most 8 scoped threads, joins them before returning, and stops scheduling when the query's budget runs
  out. Entries it did not reach make the mechanism `partial`. A file
  waiting to be deleted cannot be opened, so its holders cannot be seen;
  the failure is reported as `delete_pending`, with `STATUS_DELETE_PENDING`
  in the limitation detail.
  `open_handles.attempted` and `succeeded` count paths, including paths with
  no holders, rather than processes. A completed empty path query is usable.

  Process handles captured before the scan preserve each known holder's
  lifetime through enrichment. A holder absent from enumeration is opened
  and confirmed by a second query of its path while that process handle is
  retained. Evidence without a confirmed lifetime stays separate and has
  `identity_uncertain: true`; it never borrows a later process's identity.
  Only the confirmed path is associated with a newly acquired lifetime;
  other earlier evidence is kept in a separate, uncertain record.

  Scheduling uses `.gitignore` files inside the queried tree, without
  excluding any entries or performing repository inventory. Ignore files
  outside that tree and global Git exclusions do not affect scheduling.
  Sorting and a native operation already running can overrun the budget.

A loaded module or an open handle shows that a process uses a file. It does
not establish that deleting or renaming the file will fail. A handle opened
with `FILE_SHARE_DELETE` permits deletion, and a module's file can often be
renamed while it is mapped.

Problems stay visible instead of reading as "no matches":

- A process that cannot be opened at all (a protected process, or another
  user's when not elevated) is `permission_denied` for `loaded_modules`. So
  is a process opened without memory access; that one still has its creation
  token, so it is not `identity_uncertain`. The System process's modules are
  never readable, so `loaded_modules` is `partial` on every host.
- A module list that cannot be read while the process is running is
  `inspection_failed`, with the Windows message (for example, a process still
  starting, or of an architecture this build cannot read). The same failure
  after the process has exited is `process_disappeared`.
- A module unloaded between the listing and its path read is no longer usage
  and is not a gap.
- An open that fails for a reason other than permission leaves the process
  without a token: enumeration records an `identity_uncertain` limitation.

## Serialized form

The report serializes with snake_case keys and `schema_version: 2`.

- `index_provenance` is `walked`, `supplied`, or `unknown`. It is required in
  a version 2 report and may not be `null`. A version 1 report, written
  before the field existed, still loads with `unknown` (never `walked`); one
  that carries the field may carry only `unknown`, which is how a version 1
  report reads back after it was loaded and written again.

- Paths, executables, and process names are objects:
  `{"display": "caf�"}` plus, only when `display` cannot reproduce the
  value, `"native": {"encoding": "unix_bytes", "units": [99, 97, 102, 233]}`
  (`windows_utf16` units on Windows, which preserve unpaired surrogates).
  Never match or open files using `display`.
- Inodes, devices, file IDs, creation tokens, descriptors, and watch masks are
  decimal strings, because they can exceed what a JSON number holds exactly.
- An unavailable field is an explicit `null`, never an omitted key. Reading a
  report back requires every key, rejects unknown keys, and rejects duplicates.
- `elapsed_us` is monotonic microseconds; `started_at`/`finished_at` are
  wall-clock times for presentation.

## Work counters

Under a performance collector the query records these
`performance::counters`: `filesystem.query.tree_walks` (one per recursive
directory query), `tree_identity_reads`, `process_enumerations`,
`process_inspections`, `descriptor_inspections`, `watch_registration_reads`,
`module_inspections`, `path_holder_inspections`, `identity_enrichments`, and `path_lookups`, all under
`filesystem.query.`.
On Linux, `descriptor_inspections` counts each `/proc/<pid>/fd/<n>` entry and
`watch_registration_reads` each `fdinfo` read of an anonymous-inode
descriptor, failures included. On macOS, `descriptor_inspections` counts
each listed descriptor, vnode or not; `watch_registration_reads` stays zero.
On Windows, `module_inspections` counts each module path read, failures
included, and `path_lookups` counts only modules whose path could lie inside
the target; `path_holder_inspections` counts scheduled file-side path inspections
(including failures), and worker counts are attributed to the same request.
Buffer retries and lifetime-confirmation queries do not add scheduled paths.
The descriptor and watch counters stay zero.
Root resolution also records `filesystem.io.metadata_probes` and
`filesystem.io.canonicalizations`. With a supplied tree index,
`tree_walks` and `tree_identity_reads` stay zero.
