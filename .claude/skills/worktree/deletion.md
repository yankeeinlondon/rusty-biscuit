# Plain deletion engine (`remove::delete`)

The only deletion engine: the lifecycle ([lifecycle.md](lifecycle.md))
deletes every placed checkout with it. There is no watcher and no inventory
to compare against.

## Shape

`delete_tree(Target { root, identity }, &Env { clock, faults, mounts,
holders, cancelled }, &mut dyn Progress) -> Deletion` (`mounts` is the live
`&dyn MountInfo`, never a list read ahead of time):

| Result | Meaning |
| --- | --- |
| `Done` | Root gone, including "was already gone" (a resumed run). |
| `RetainedMounts(mounts)` | Everything else gone; mount points and their parents kept. |
| `Permanent { failures, retained_mounts }` | Everything else gone; `Failure` displays the R29 sentence `<path> could not be deleted: <plain cause>`. |
| `Cancelled { waiting_on }` | Ctrl-C; `waiting_on` empty unless a wait was in progress. |
| `RootReplaced` | Root is a link, not a directory, or not the bound identity. Nothing deleted. |

`root` is the quarantine (moved) or the checkout itself (in place); the same
engine serves both. There is no inventory: the directory is the list of what
is left, so a rerun after a crash just runs it again.

## Rules that are easy to break

- **Every directory is reached through its parent's handle and its identity
  rechecked after `open_dir`** (`LOOKS` = 3 re-looks, then a failure "kept
  changing"). A failed open re-looks *before* any permission fix: a link
  swapped in makes cap-std refuse (escape), and that must become
  `Step::Changed` → removed as a link, never `make_traversable`.
- **Mount boundary = `MountInfo::mount_at(spelled, real)` asked afresh
  before every open below the root (each refill reopen too) OR a different
  device/volume than the root** (live, in `entry`). Never snapshot the table
  at engine start: a same-device bind mount published mid-deletion has no
  other sign (R2-F3). The engine resolves the root once (`real_root`) and
  joins each relative path to it, since native tables spell real paths
  (`/private/var/…` on macOS); the default `mount_at` matches either
  spelling in a fresh `mounts()`. An unreadable answer is no mount.
- **Native `mount_at` is per-directory, not a table read:** macOS one
  `statfs` (`f_mntonname == real`); glibc Linux `statx`
  `STATX_ATTR_MOUNT_ROOT` via `libc::syscall` (no glibc 2.28 symbol; musl
  and kernels before 5.8 fall back to the table); Windows `None`, since a
  mounted folder is a reparse point `entry` already keeps (`is_volume_mount`)
  and listing every volume per directory costs milliseconds. A 100,000-entry,
  20,000-directory tree on macOS: +3% with `statfs`, +13% re-reading the
  table per directory.
- **`Mount` carries `fs_type`** (every OS; empty when Windows cannot read
  it) **and `super_options`** (Linux mountinfo only; empty elsewhere). WSL2 lists `/mnt/c` as `9p` with `aname=drvfs` in the
  superblock options (`,` and `;` both separate them), so `Mount::is_drvfs`
  reads both; `mounts::containing` finds the checkout's own mount.
- **Classification (`classify`)**: Gone / Temporary / Denied / Permanent per
  OS table; unknown → Permanent, never a loop. Unix EBUSY is Permanent.
  Errors with no OS code classify by `ErrorKind`, so tests use raw codes
  (`EAGAIN`/`ERROR_SHARING_VIOLATION`) for a portable "held file".
- **Denied**: Unix makes the *parent* writable via `fchmod` on its handle
  (an unlistable child: `fchmodat(…, AT_SYMLINK_NOFOLLOW)` on macOS, an
  identity-checked `O_PATH | O_NOFOLLOW | O_DIRECTORY` open on Linux), retries
  once, then Permanent. Windows never changes attributes: deletion uses
  `FileDispositionInfoEx` with `POSIX_SEMANTICS | IGNORE_READONLY_ATTRIBUTE`
  on a handle opened `OPEN_REPARSE_POINT` and checked against the looked-at
  identity; ACCESS_DENIED is a wait only if `Holders` names a program at that
  path (a running image), else Permanent. Fallback for filesystems without
  the Ex flags clears read-only only when `nNumberOfLinks == 1`.
- **Linux: cap-std directory handles are `O_PATH`.** `fchmod` on them is
  `EBADF` (so `chmod_handle` falls back to `/proc/self/fd/<fd>`), and opening
  an unlistable directory *succeeds* — the denial arrives at listing, so
  `clear` makes the directory writable and lists once more. macOS fails at
  the open instead. Both read-only tests passed on macOS and failed on Linux
  until this was handled: run `just cross-check worktree --os linux remove::`
  after touching permission code.
- **Waits**: a full pass first deletes everything it can; then the blocked
  set is explained by `Holders::query` — once on entry and again only when
  the blocked set changes (never on a timer) — and `Progress::waiting` fires
  only when the `Wait` value changes. Backoff 50 ms ×2 to 2 s, reset when the
  blocked set shrinks. Only a successful delete ends a wait; a holder
  disappearing from the query proves nothing on Windows.
- **Refill (R24)**: `REFILL_PASSES` = 3 failed `remove_dir` (relisting and
  deleting between) → blocked `Refilling`, then the same wait.
- **The root's `.git` goes last**, so an interrupted run leaves a checkout
  Git still recognizes.
- `Progress::batch(n)` fires every `BATCH_ENTRIES` (1024) deletions: the
  lifecycle's `DeletionBatch(n)` crash boundary hooks here.

## Fault points (`FsFaults`)

`ReadDir` is consulted before opening a directory **and** before listing it;
`RemoveFile` covers files, links and special files; `RemoveDir`; and
`MakeWritable` (path = the directory being made writable). A fault that
mutates the tree from `inject` is how tests stage races (`SwapForLink`,
`Refill` in `delete/tests.rs`). `ScriptedHolders` in `test_support.rs` scripts
holder reports and records what each query asked. `LiveMounts` (a shared,
changing table) plus `MountDuringDeletion` (publishes a same-device mount
under the root at its first `ReadDir`) stage a mount made mid-deletion.
