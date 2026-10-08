# Linux

Load when a test is red on `ubuntu-latest`, `wsl2-ubuntu`, or the
`BUILD_LINUX` host and green on macOS, or before writing a test that depends on
directory order, the process table, git history depth, or a Unicode glyph.
WSL2 follows these Linux code paths; its archive-mode traps are in
[wsl.md](wsl.md).

## Hook fixtures red only on Ubuntu

- Temporary repositories need **local** `user.name` and `user.email` for later
  `git notes` writes under `env -i`; command-scoped identity on the initial
  commit does not persist.
- Initialize fixture bare remotes with `-b main` rather than inheriting the
  host's default branch.
- A missing-tool test must use a controlled `PATH`: Ubuntu's `/usr/bin`
  already contains `gh`, so adding it defeats a missing-`gh` fixture.

## Shallow checkouts: L2 red only on Ubuntu, a timeout

- CI's checkouts are depth-1, and macOS L2 is satisfied by local evidence from
  a full-history clone, so Ubuntu is where git-history code first meets a
  shallow boundary.
- That commit diffs against the empty tree, so every file is an addition.
- Reproduce in a `git clone --depth 1` (Docker, see [macos.md](macos.md)).
- On 2026-09-22 sniff's rewrite tracker turned this into a quadratic 30 s+
  `claudine context --values`.

## `read_dir` order: red on Linux and WSL, green on macOS and Windows

- APFS and NTFS return entries sorted by name; ext4 returns hash order (`lib`
  before `app`). When only an order differs, suspect this first.
- Sniff's leaf-marker layers (Bazel, Pants, Buck2) list
  `MonorepoLayer::packages` in walk order, so a complete-JSON assertion must
  sort those lists; `lockfile_provenance::normalized_any` does so only for
  `provenance == "leaf-markers"` (2026-09-26).

## `LANG=C` on the `BUILD_LINUX` host: ASCII where a glyph was expected

- Symptom: an ASCII `>` where the test expects `▶` (or another glyph), red only
  on that host.
- That host's shells run with `LANG=C`, and both in-process terminal detection
  and `CliProcessFixture`'s children read it, so `biscuit-terminal` selects its
  ASCII fallbacks.
- Hosted CI runs `LANG=C.UTF-8`; `cross-check` now exports the same
  (2026-09-28), but an ad hoc SSH session on that host does not.
- A test whose assertion needs a Unicode glyph should pin `LC_ALL=C.UTF-8` on
  its spawned command (`sequence_groups`).

## Temp-path length changes word wrapping

- Symptom: a substring count in rendered CLI output is red on Linux and green
  on macOS.
- The temp path in the message has a different length (`/tmp/<name>-<pid>-…`
  vs `/var/folders/…`), so the terminal wraps at a different word and splits
  the phrase being counted.
- Count in whitespace-collapsed output, never in the wrapped text
  (`lifecycle_downgrade_outcome`, 2026-09-28).

## `sysinfo` lists threads as processes

- On Linux (and WSL2), `sysinfo`'s process table also lists every **thread** of
  a process, each with the process's own argv.
- A test that counts processes by argv over-counts as soon as the process
  spawns a thread, while macOS and Windows count it once.
- Filter with `process.thread_kind().is_none()`
  (`worktree/cli/tests/perf_support::refresh_workers`).

## `ld terminated with signal 9` on `BUILD_LINUX` during cross-check

- `just cross-check worktree-cli --os linux` once failed while linking a
  test binary (`collect2: fatal error: ld terminated with signal 9 [Killed]`):
  the host killed the linker for memory while parallel links ran (possibly
  beside other sessions' builds). It is not a code error.
- Retry once with fewer parallel builds; extra arguments go to
  `cargo nextest run`: `just cross-check <package> --os linux --build-jobs 2`.
  The retry reused the cached build and passed.

## Reading `/proc` for another process's descriptors

Measured 2026-10-05 on `BUILD_LINUX` for Sniff's path-usage query:

- **`procfs` 0.18 hides per-fd permission errors.** `FDsIter` skips every fd
  whose info fails. For a same-uid, non-dumpable process (`systemd --user`),
  `/proc/<pid>/fd` listed 34 entries, every per-fd stat gave `EACCES`, and
  `Process::fd()` returned `Ok` with zero items. Read `/proc` directly when a
  denial must stay visible. There are two denial shapes: the directory itself
  is unreadable (other uid), or the listing succeeds and each entry fails.
- Take an fd target's identity from following `std::fs::metadata
  ("/proc/<pid>/fd/<n>")`, not from the link text (lossy and descriptive for
  deleted files), and not from a no-follow stat (that is the magic link's own
  inode).
- `readdir("/proc")` lists thread-group leaders only, but `/proc/<tid>` still
  resolves for any thread ID and looks like a process (`Tgid` != `Pid` in
  `status`). Enumerate only through `readdir`.
- inotify `fdinfo` lines carry `sdev` as the kernel-internal `dev_t`
  (`major << 20 | minor`), not `st_dev`. Decode it and rebuild with
  `libc::makedev(major, minor)` before comparing. On `BUILD_LINUX` every mount
  is major 0 with a small minor, so the raw values happen to agree there and
  a missing decode goes unnoticed. A real block device (WSL2 ext4, major 8)
  exposes it.
- Parse `/proc/<pid>/stat` after the last `)`: `comm` can contain spaces and
  parentheses. Field 22 (`starttime`) is the PID-reuse token.
