# Linux

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

Load when a test is red on `ubuntu-latest`, `wsl2-ubuntu`, or the
`BUILD_LINUX` host and green on macOS, or before writing a test that depends on
directory order, the process table, git history depth, `/bin/sh`, or a Unicode
glyph. WSL2 follows these Linux code paths; its archive-mode traps are in
[wsl.md](wsl.md).

## ARM Linux initialization requests an x86_64 toolchain

- Cross reads workspace metadata even for `cross --version`; on ARM Linux that
  can request the pinned `x86_64-unknown-linux-gnu` toolchain and fail
  rustup's non-host guard.
- `scripts/ensure-ci-tools.sh` therefore probes Cross from an empty temporary
  directory. Keep such probes outside Cargo workspaces.

## Hook fixtures red only on Ubuntu

- Temporary repositories need **local** `user.name` and `user.email` for later
  `git notes` writes under `env -i`; command-scoped identity on the initial
  commit does not persist.
- Initialize fixture bare remotes with `-b main` rather than inheriting the
  host's default branch.
- A missing-tool test must use a controlled `PATH`: Ubuntu's `/usr/bin`
  already contains `gh`.

## Shallow checkouts: L2 red only on Ubuntu, a timeout

- CI checkouts are depth-1, while macOS L2 is usually satisfied by local
  full-history evidence, so Ubuntu is where git-history code first meets a
  shallow boundary.
- The boundary commit diffs against the empty tree, so every file is an
  addition; history walkers can go quadratic and surface as a timeout.
- Reproduce in a `git clone --depth 1` (Docker, see [macos.md](macos.md)).

## `read_dir` order: red on Linux and WSL, green on macOS and Windows

- APFS and NTFS return entries sorted by name; ext4 returns hash order. When
  only an order differs, suspect this first.
- Sort walk-order lists before a whole-value assertion (for example sniff's
  leaf-marker layers, normalized in `normalized_any` in
  `sniff/lib/tests/l1/lockfile_provenance.rs`).

## `LANG=C` on the `BUILD_LINUX` host: ASCII where a glyph was expected

- Symptom: an ASCII `>` where the test expects `▶`, red only on that host.
- Its shells run with `LANG=C`; in-process terminal detection and
  `CliProcessFixture` children read it, so `biscuit-terminal` picks ASCII
  fallbacks.
- Hosted CI and `cross-check` use `LANG=C.UTF-8`; an ad hoc SSH session does
  not.
- A test whose assertion needs a glyph pins `LC_ALL=C.UTF-8` on its spawned
  command (`claudine/cli/tests/l1/sequence_groups.rs`).

## Temp-path length changes word wrapping

- Symptom: a substring count in rendered CLI output is red on Linux, green on
  macOS.
- The temp path in the message differs in length (`/tmp/…` vs
  `/var/folders/…`), so wrapping splits the counted phrase.
- Count in whitespace-collapsed output, never in wrapped text.

## `sysinfo` lists threads as processes

- On Linux (and WSL2) `sysinfo`'s process table lists every **thread**, each
  with the process's argv, so counting processes by argv over-counts once a
  thread spawns.
- Filter with `process.thread_kind().is_none()`
  (`refresh_workers` in `worktree/cli/tests/perf_support/mod.rs`).

## `ld terminated with signal 9` on `BUILD_LINUX` during cross-check

- `collect2: fatal error: ld terminated with signal 9 [Killed]` while linking
  is the host killing the linker for memory under parallel links, not a code
  error.
- Retry with fewer parallel builds:
  `just cross-check <package> --os linux --build-jobs 2`. The flag must
  follow the package name; before it the script exits 2 with
  `unknown flag --build-jobs`.
- `--build-jobs 2` is not always enough: on 2026-10-18 `worktree-cli` (about
  25 test binaries) was killed at 2 jobs and linked only at
  `--build-jobs 1`, which then took a few minutes longer.

## inotify: watches follow inodes, new directories are not watched

Measured 2026-10-11 on `BUILD_LINUX` (ZFS) and the WSL guest (ext4) for
`wt remove`'s former activity watcher (removed 2026-10-23; nothing in the
repository watches with inotify now):

- A watch descriptor follows its inode, so a renamed directory keeps
  reporting through the same watch. A wrapper that tracks watches by path
  can lose this; read the kernel events directly.
- A directory created after its parent's watch was added is not watched
  until something adds a watch for it; files created in it before then are
  missed.
- `renameat2(RENAME_NOREPLACE)` works on ZFS, ext4, and tmpfs. Limits on
  `BUILD_LINUX`: `max_user_watches` 4194304, `max_user_instances` 65536,
  `max_queued_events` 8388608, so a blocked reader did not overflow at
  40,000 events there. The WSL guest's much lower limits are in
  [wsl.md](wsl.md).

## Reading `/proc` for another process's descriptors

- **`procfs` hides per-fd permission errors.** `FDsIter` skips every fd whose
  info fails, so for a same-uid, non-dumpable process `Process::fd()` returns
  `Ok` with zero items. Read `/proc` directly when a denial must stay visible.
  Denial has two shapes: the directory is unreadable (other uid), or the
  listing succeeds and each entry fails with `EACCES`.
- Take an fd target's identity by following `std::fs::metadata
  ("/proc/<pid>/fd/<n>")`, not from the link text (lossy, descriptive for
  deleted files) and not from a no-follow stat (the magic link's own inode).
- `readdir("/proc")` lists thread-group leaders only, but `/proc/<tid>`
  resolves for any thread and looks like a process (`Tgid` != `Pid`).
  Enumerate only through `readdir`.
- inotify `fdinfo` `sdev` is the kernel-internal `dev_t`
  (`major << 20 | minor`), not `st_dev`. Decode and rebuild with
  `libc::makedev(major, minor)` before comparing. On hosts whose mounts are
  all major 0 the raw values happen to agree; a real block device (WSL2 ext4,
  major 8) exposes a missing decode.
- Parse `/proc/<pid>/stat` after the last `)`: `comm` can contain spaces and
  parentheses. Field 22 (`starttime`) is the PID-reuse token.

## cap-std directory handles are `O_PATH`: permission code green on macOS, red on Linux

- On Linux, `cap_std::fs::Dir` holds a directory as an `O_PATH` descriptor.
  `fchmod` on it fails with `EBADF`; change the mode through
  `/proc/self/fd/<fd>` instead (`remove::delete::platform::chmod_handle`).
- `Dir::open_dir` on a mode-`000` directory **succeeds** on Linux (no
  permission is needed for `O_PATH`), so the `EACCES` arrives when the
  directory is listed. On macOS the open itself fails. Handle the denial at
  both points.
- To change a child directory's mode without following a link, open it with
  `O_PATH | O_NOFOLLOW | O_DIRECTORY`, check its device and inode, and change
  the mode through that descriptor. Do not rely on glibc's
  `fchmodat(…, AT_SYMLINK_NOFOLLOW)`, which needs glibc 2.32+ and `/proc`.
- Found in `2026-10-22-confirmed-remove-always-deletes` by
  `just cross-check worktree --os linux remove::` after every macOS test had
  passed.

## `/bin/sh` is dash: `set -m` hangs or silently does nothing

Debian and Ubuntu (WSL2 included) link `/bin/sh` to dash; macOS runs bash as
`sh`. A fixture relying on `sh -c 'set -m; cmd & …'` to put a background job
in its own process group breaks two ways:

- **With a controlling terminal** (interactive nextest), dash's `set -m`
  loops on `SIGTTIN` until it is the foreground group. Nextest gives each test
  its own group, so the test hangs to its timeout. It passes without a
  terminal (CI, an agent's shell), so it reads as "only on my box".
- **Without a terminal**, dash prints `can't access tty; job control turned
  off` and the job stays in the shell's group: the test passes without
  exercising its premise.

Run `/bin/bash` explicitly for job-control fixtures and assert the split with
`getpgid`. Reproduce the terminal case with
`script -qec "cargo nextest run …" /dev/null`.

dash also defers a `SIGINT` sent to the shell's PID (not its group) until its
foreground child exits: `sh -c 'sleep 30'` keeps running for the whole
`sleep`, where bash-as-`sh` execs the `sleep` and dies at once. A
`SIGINT`-then-grace stop of an `sh -c` command therefore takes the full grace
period on Linux and WSL2; shorten the grace in tests rather than relying on
the shell to exit.
