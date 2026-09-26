# Phase 1 spikes

Run `python3 spike_git.py` on macOS or Linux, or
`powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File spike_windows.ps1`
on native Windows. Both probes create only temporary repositories and print
their observations. They exit nonzero when an asserted contract fails.

## S1 — Git matching

The Python probe passed with Git 2.55.0 on macOS and 2.47.3 on Linux. The
Windows probe passed the NUL-delimited candidate/check pipeline with Git
2.55.0.windows.3. `ls-files --others --ignored --exclude-from=<rules> -z`
provided candidates; `check-ignore --stdin -z --verbose --non-matching`
identified standard-ignore matches. A verbose response has four NUL-delimited
fields per path. A pattern beginning with `!` means the path is *not* ignored.
Use byte-oriented child-process I/O; PowerShell 5.1's text pipeline added a
UTF-8 BOM to the first path and spoiled the result.

The Python assertions cover tracked, ignored, and ordinary untracked files;
nested `.gitignore`, `.git/info/exclude`, and `core.excludesFile`; ordered
negation; a child under an ignored parent; `/` anchoring; `**/` into an
ignored directory; spaces and newlines in names; and an absolute rules file
applied from a different worktree. Git did not enumerate files inside a nested
repository, submodule, nested linked worktree, or directory symlink. A
directory-matching rule **did** return `foreign/` and `nested-worktree/` as
entries on macOS and Linux. Git for Windows **traversed a junction** and
returned `junction/secret.env`. Phase 2 must inspect every candidate's
ancestors for links and directory reparse points, and reject directory
entries, even when Git itself avoids descent on Unix. The
two-step intersection needs no design replacement.

## S2 — Clone API and publication

Use [`reflink-copy` 0.1.30](https://docs.rs/reflink-copy/0.1.30/reflink_copy/fn.reflink.html)
in `worktree`, not `biscuit-file`: the policy and temporary-publication flow
are specific to worktree creation. `cargo info --verbose` reports MIT or
Apache-2.0 and direct dependencies `cfg-if`, `libc`, `rustix`, and `windows`
(with optional tracing). Its `reflink` creates a new destination and does not
byte-copy on failure. The [source](https://docs.rs/reflink-copy/0.1.30/src/reflink_copy/lib.rs.html)
uses `clonefile` on macOS, `FICLONE` on Linux, and
`FSCTL_DUPLICATE_EXTENTS_TO_FILE` on Windows; it marks the Windows path as
untested. Keep the fallback under `CopyOps` so Phase 2 can test the clone and
byte-copy outcomes independently. Retry as a byte copy for unavailable clone
capability, cross-volume, or clone-specific refusal; surface genuine read,
destination, and permission failures. The fallback should make a fresh
temporary path because the failed clone may have created one.

Clone into a unique *path* in the destination directory, then set permissions,
digest the completed copy, and publish without replacement. For regular files
on Unix, `hard_link(temp, final)` is a same-directory no-replace operation;
unlink the temp afterward. On Windows, use a no-replace `MoveFileExW` or
equivalent, since hard links are not a portable ReFS assumption. Symlinks
need their own no-replace publication path and must not become hard links.
The macOS APFS `cp -c` probe succeeded, preserved `0600`, and proved later
writes to the clone leave the source unchanged. The Linux temporary volume
returned `Operation not permitted` for `cp --reflink=always`, so the fallback
is necessary. Phase 2 must still verify the crate's real Windows behavior on
the ReFS build volume; the library's own documentation does not claim it is
fully tested.

## S3 — Registration identity

The Git admin directory name was reused after remove, prune, and re-add on
macOS, Linux, and Windows. The `<admin>/gitdir` inode changed on the two Unix
hosts and its creation time changed on Windows, but those are filesystem
metadata with platform-specific availability or precision. A disposable
`wt-copy-registration` file placed in the admin directory was removed with
the worktree on all three hosts. Use a random 128-bit marker, create it with
`create_new`, and store the same value in the copy record. A missing or
mismatched marker makes the record untrusted. This avoids unstable Windows
file-ID APIs and detects an externally recreated registration at the same
path. Keep the existing planned pre-add deletion of the old record.

## S4 — Windows links and junctions

The Windows probe created a junction with `Directory, ReparsePoint` attributes
and a file symlink with `Archive, ReparsePoint` attributes. Git for Windows
followed the junction during `ls-files` and returned a file underneath it.
Use
`symlink_metadata`, `FileTypeExt`, and the `FILE_ATTRIBUTE_REPARSE_POINT` bit
from `MetadataExt::file_attributes` to reject junctions and other directory
reparse points as ancestors or traversal roots. The build host allowed
symlink creation, so the privilege-denied path was not observed directly.
[Rust's Windows symlink documentation](https://doc.rust-lang.org/std/os/windows/fs/fn.symlink_file.html)
describes the privilege requirement; Windows reports lack of the privilege as
[`ERROR_PRIVILEGE_NOT_HELD` (1314)](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--1300-1699-).
Classify that raw error as a skipped link with a warning. Do not assume its
`ErrorKind` is `PermissionDenied`. The OS skill records this trap.
