# Spike S1: Windows file-sharing check

Date: 2026-10-17. Intended as the "Windows file-sharing check" section of
`spike-s1.md`; kept separate because that file was being written concurrently.

## Setup

- Throwaway crate (not in the repo): `rusqlite 0.31` with `features = ["bundled"]`
  (SQLite 3.45.0), release build.
- Process A (`holder`): `Connection::open`, `PRAGMA journal_mode=WAL` (returns
  `wal`), creates a table, inserts rows, then holds the connection either
  **idle** or **mid-transaction** (`BEGIN IMMEDIATE; INSERT ...`, uncommitted).
- Process B (the orchestrator, a separate process): tries `File::set_modified`,
  `std::fs::remove_file` on `-wal`, `-shm`, then the main `.sqlite`, then a
  "skip on sharing violation" sweep, then delete-and-recreate, then asks A to
  close and retries.
- Hosts: native Windows x86_64 (`BUILD_WIN`, NTFS/ReFS volume `B:`), and the
  local macOS aarch64 (APFS). This is behavioral evidence on both.

## Results: native Windows

Identical for the idle and the mid-transaction connection.

| Action by B while A holds the connection | Result |
|---|---|
| `remove_file(*.sqlite-wal)` | `Err`, `raw_os_error=Some(32)`, `kind=Uncategorized`, "The process cannot access the file because it is being used by another process." |
| `remove_file(*.sqlite-shm)` | same, os error 32 |
| `remove_file(*.sqlite)` | same, os error 32 |
| All three files afterwards | still present; A keeps reading and writing normally |
| `OpenOptions::new().write(true).open(db)` + `set_modified` | **Ok** |
| `File::open(db)` (read-only) + `set_modified` | `Err`, os error 5 (`PermissionDenied`): a read handle lacks `FILE_WRITE_ATTRIBUTES`, not a sharing problem |
| Sweep treating os error 32 as "in use, skip" | `removed=0 skipped=3 failed=0` |
| Delete-and-recreate (corrupt-store recovery) | delete fails with os error 32, so the "delete failed -> memory fallback" branch is reachable |

After A closes its connection (clean close; the last connection checkpoints
and deletes its own `-wal`/`-shm`):

- only `*.sqlite` remains; `-wal` and `-shm` give `NotFound` (os error 2);
- `remove_file(*.sqlite)` succeeds, and a following sweep removes nothing and
  fails nothing.

So an idle-but-open connection blocks deletion exactly as a mid-transaction one
does; only closing the connection releases the files.

## Results: macOS (Unix unlink semantics)

- All three `remove_file` calls succeed while A holds the connection (idle and
  mid-transaction). A sees no error: it continues to read and insert
  (`count 3 -> 4`) into the now-unlinked inode.
- B can then create a fresh store at the same path (count 1); A still sees and
  writes its old data (`count 4 -> 5`). The two are now split-brain: A's
  writes land in a file nobody else can open and are lost when A exits.
- On A's close, SQLite deletes `-wal`/`-shm` **by path**; here B's new
  connection had already closed, so nothing collided, but a live second store
  at the same path could have its `-wal` removed by A's close.
- `set_modified` works with both write and read handles.

## Conclusions for the design

1. On Windows, every file of an open WAL store is undeletable by another
   process; the error is os error 32 (`ERROR_SHARING_VIOLATION`). The stable
   `io::ErrorKind` is `Uncategorized`, which cannot be matched, so the sweep
   must test `e.raw_os_error() == Some(32)` (under `cfg(windows)`).
   Accepting os error 5 as well is reasonable defense for delete-pending
   files, but this spike did not observe it from `remove_file`.
2. "Skip on sharing violation" is a correct, sufficient in-use test on
   Windows. It is **not** one on Unix: the unlink succeeds, so an age-based
   sweep on macOS/Linux would silently delete a live store from under a
   running server. Unix needs a separate liveness signal (a lock file, or
   recent mtime refreshed by the owner).
3. Corruption recovery by delete-and-recreate fails on Windows while any
   other process has the store open, so the memory-fallback path is real and
   must be tested. On Unix it "succeeds" but strands the other process on the
   old inode.
4. Touching mtime as a liveness heartbeat works on Windows while another
   process holds the store, provided the handle is opened for write.
   `File::open` (read-only) + `set_modified` fails with os error 5.

Not tested: a crashed holder (leftover `-wal`/`-shm` with no live handle),
network shares, and read-only connections (`SQLITE_OPEN_READ_ONLY`) as A.
