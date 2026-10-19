# macOS

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

macOS is the primary development host. Most traps here are about what it can
do for the *other* operating systems, and about host conditions that look like
repository defects.

## Paths

- **APFS rejects non-UTF-8 filenames** (`EILSEQ`, "Illegal byte sequence",
  error 92), and also some valid ones such as `U+FDD1`. A fixture cannot create
  such a path in the worktree (Linux can). Git can still record one: build it
  through the index and commit trees (`git2::Index::add_frombuffer` with raw
  bytes) and compare committed-tree output, as sniff's
  `filesystem::git::changed_paths_tests` does. Or build a catalog of
  nonexistent paths under a canonicalized temporary root, or gate the test to
  an OS whose filesystem allows such names.
- **`/var` and `/tmp` are symlinks into `/private`.** A `tempfile` path and the
  path a child process reports have different spellings. Canonicalize before
  you compare (see `launched_spelling` in
  `claudine/cli/tests/l1/ctx_launch_anchor.rs`). This is the macOS form of the
  Windows short-name trap ([windows-paths.md](windows-paths.md)).
- **The same split happens in production.** `current_dir()` is physical
  (`/private/var/…`), while `$HOME` keeps the spelling it was set with
  (`/var/…`). A lexical `launch == home` check silently misses. An in-process
  test that builds both paths from one `TempDir` spelling cannot see this; an
  L1 subprocess launched from the fixture `$HOME` can. Compare with
  `fs::canonicalize` where two such spellings meet, as Claudine's
  `with_prompt_magic_roots` does.
- **macOS sweeps old files under `/tmp` but leaves populated directories.** A
  git worktree there loses its small `.git` pointer file first, and
  `git worktree list` then shows it as `prunable`. That is not corruption: run
  `git worktree prune`. Keep long-lived worktrees under `~/.claudine/worktrees`.
- **Case-insensitive APFS folds by a Unicode rule, not by `to_lowercase()`.**
  `ς` opens a stored `Σ`, and `ß` opens a stored `SS`. To tell whether a
  spelling is the stored one, compare against an exact `read_dir` entry, never
  against folded strings. If the parent can be traversed but not listed (mode
  `0111`), `fs::canonicalize` still reports the stored final name, unless that
  name is a symlink. biscuit-file's absolute-glob spelling check relies on both
  facts.
- **Temporary directories can be case-sensitive.** `cfg!(target_os = "macos")`
  does not mean a name with different casing will open. Probe the fixture
  directory's behavior, and keep the exact-name and mismatch assertions for
  both filesystem types. To reproduce without admin rights or a window, create
  and attach an HFSX image, then point `TMPDIR` at it:

  ```sh
  hdiutil create -size 40m -fs HFSX -volname case-test -type UDIF /tmp/case-test.dmg
  hdiutil attach -nobrowse -mountpoint /tmp/case-test-mount /tmp/case-test.dmg
  TMPDIR=/tmp/case-test-mount just test   # then detach and delete the image
  ```
- **`dirs::home_dir()` honors `HOME` here but not on Windows.** That is why a
  hermetic-home test passes on macOS and reads the real home directory on
  Windows. Use `biscuit_file::home_dir()`, which follows `USERPROFILE` on
  Windows. A fixture that sets both variables then lands in the same place on
  every OS.
- **A Unix socket path holds at most 104 bytes** (`sun_path`), and the
  per-user `$TMPDIR` (`/private/var/folders/…/T/`) uses about half of that. A
  socket-binding daemon under a `tempfile` dir fails with "path must be shorter
  than SUN_LEN" (`kache daemon run` does). Root the fixture under `/tmp`, as
  `real_kache_worktree_restore` (`tools/test-toolkit`) does.
- **`getcwd` after the cwd is deleted** fails with `ENOENT`, but only after
  searching the parent's entries: about 2 s in general, and about 9 s when the
  deleted directory sat directly in the shared `$TMPDIR`
  (`/var/folders/…/T/`; 2 ms under `/private/tmp`, 70 µs inside an empty
  private parent). A test of an unreadable-cwd branch (for example sniff-cli's
  `relative_base_with_unreadable_cwd_fails`) should make only one lookup, and
  should create the directory to delete inside its own `TempDir`, removing
  only the child.

## File flushes cost ~25 ms each

On Apple platforms std's `File::sync_all` **and** `File::sync_data` call
`fcntl(F_FULLFSYNC)`, which also empties the drive's write cache: about 25 ms
per call here. The `worktree` recovery-store tests took 0.3–4 s each with
std's flushes and about 30 ms with plain `libc::fsync`. Plain `fsync` survives
a process or kernel crash, not a power loss; when the code claims only
process-crash durability, call `libc::fsync` under
`#[cfg(target_vendor = "apple")]` (`flush` in
`worktree/lib/src/remove/recovery/storage.rs`). Never call std's flush in a
per-batch loop on macOS without measuring it.

## Process groups

- **One `kill(-pgid, SIGKILL)` can miss a child that is being forked.** XNU
  does not make `fork` atomic with a group signal, so the new child joins the
  group without the signal and survives. Repeat the group kill until it fails.
  A group that holds only zombies returns `EPERM` on macOS but `0` on Linux.
  So "repeat until it fails" ends on macOS and spins forever on Linux. This is
  why Claudine's `ProcessTree` drop
  (`claudine/lib/src/composition/sequence/task/shell.rs`) repeats the kill only
  off Linux, with a time limit.
- The surviving child keeps whatever the group inherited. If it holds a
  test's stderr, nextest reports `LEAK-FAIL` after the test passed, even
  though every pid the test watched was reaped.

## Linux and Windows evidence from this host

"This host is macOS-only, so cross-platform runs are not possible" is wrong.

- **Real Linux runs:** Docker Desktop provides a `linux/arm64` kernel. Run
  `open -a Docker`, poll `docker info`, then run tests in a container. PTY
  L2 tests pass there too. podman's VM does not start on this host.
  - Copy the source rather than mounting the worktree, because a worktree's
    `.git` is a file that points outside the container. Use
    `rsync -a --exclude=target/ --exclude=.git --exclude=.gitnexus/`. The
    `.gitnexus/` store changes during indexing, so rsync exits 23 if you
    include it.
  - Put the source copy and the target dir under `$HOME`, for example
    `~/.cache/<scratch>/{src,target}`. Docker's file sharing rejects bind
    mounts from `/Volumes/…` and `/tmp` (`/private`) with
    `mkdir …: read-only file system`.
  - Keep `CARGO_TARGET_DIR` on a host mount (`-v …:/t -e CARGO_TARGET_DIR=/t`),
    because the VM overlay is small. A whole-package build OOM-kills the
    linker in the default VM, so use `--memory=7g`, `CARGO_BUILD_JOBS=2`, and
    targeted `--lib` / `--test <name>` runs.
  - Use `bash -c`, not `bash -lc`. A login shell resets `PATH` and drops
    `/usr/local/cargo/bin`, so `cargo` is "command not found", and a trailing
    pipe can still exit 0.
  - `rust:1` needs `libdbus-1-dev pkg-config` for `messenger --features desktop`,
    and `libasound2-dev` for anything that reaches Playa's `native-playback`
    (all of `claudine-cli`). It has no `cargo-nextest`; `cargo test --test <name>`
    avoids a slow install.
- **Linux Level 3 keyboard tests** that use `biscuit_test_harness::xvfb` (a
  private `Xvfb` plus kitty, with XTEST key presses) run in the same container
  and never touch the Mac's desktop. Add `xvfb kitty libgl1-mesa-dri`, plus
  `cargo-nextest` from `https://get.nexte.st/latest/linux-arm`. kitty works
  under Xvfb with software OpenGL and no window manager; `SetInputFocus` is
  enough to give it focus. Its "Failed to connect to DBUS" lines are harmless.
- **Windows compile evidence:** use the `x86_64-pc-windows-gnu` target. The
  details, and why msvc is not allowed, are in [windows.md](windows.md).
- **Behavioral Windows and WSL2 evidence:** use the build hosts
  ([build-hosts.md](build-hosts.md), [wsl.md](wsl.md)).

## The `macos-latest` leg

`macos-latest` is the smallest hosted runner and the slowest leg, so tune any
concurrency cap for it, not for Linux. Level 3 focus-stealing tests do not run
on CI at all. See [ci-runners.md](ci-runners.md).

## Finding filesystem watchers

An open-file scan does not list FSEvents directory watches. A process running
a recursive `fs.watch` receives events, but `lsof -nP -a -p <pid> -Fftn` shows
no watched path. Apple's `FSEventStreamCopyPathsBeingWatched` covers only
streams that the caller owns; it is not a system-wide list. So an empty `lsof`
or vnode-descriptor scan does not prove that nothing watches a directory. A cwd
inside the directory shows that the directory is in use, not that it is
watched. See the [FSEvents guide](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/UsingtheFSEventsFramework/UsingtheFSEventsFramework.html)
and [Node's platform mapping](https://nodejs.org/api/fs.html#availability).

### Receiving FSEvents reliably

Measured with `fsevent-sys` 4.1 at latency 0:

- **Bursts drop events even when the handler never blocks.** 2,000-file
  bursts produced `MustScanSubDirs` "user dropped" events; 64–1,000 did not.
- **Delivery is not ordered against a marker.** `FSEventStreamFlushSync`
  alone is not a drain: it delivers only what fseventsd has already processed
  (1 of 50 write-then-flush rounds missed its write). One confirmed marker is
  not a drain either: under a 2,000-file burst fseventsd gave a few creations
  IDs after a later marker (about 1 drain in 15). `wt remove` repeats marker
  rounds (create a marker, flush until it arrives) until a round delivers no
  watched-path event, and treats anything later as arriving in a later drain
  (`worktree/lib/src/remove/observation/fsevents.rs`). A round costs ~11 ms
  idle, up to ~650 ms under suite load.
- **Wait without the blocking flush, but keep flushing.** `FlushSync` blocks
  for as long as fseventsd takes, so a wait cannot check a deadline or Ctrl-C
  while inside it. `FSEventStreamFlushAsync` returns at once and may be called
  from another thread than the run loop's. One async flush per marker round is
  not enough: the stream then holds the marker for its latency (0.1 s in
  `wt`), which made each round ~100 ms and the macOS observation tests about
  7x slower. Re-issue it every ~1 ms while waiting (`wt` does this in the
  wait's park step).
- **`FSEventStreamStart` returns before events are routed.** Confirm with one
  marker before relying on the stream.
- **Removing or renaming a directory replaces its pending child events** in
  the same batch, even at latency 0. Drain between changing a directory's
  children and renaming or removing it.
- **fseventsd serializes stream registration:** ~20 ms alone, up to 5 s each
  with ~15 test processes starting together. Run FSEvents tests in a nextest
  group with `max-threads = 1` (`fsevents-l1`). Events also lag 100–600 ms
  under load, past any fixed wait.
- **History flags are cumulative.** A created-then-deleted (or renamed) path
  can report `Create` with `Remove` or the rename flag. Match "the
  history includes the expected flag" plus the path's current state, never
  "every flag is expected".
- **Watches are path-based.** After a rename, the new name is unobserved until
  watched, so watch both names before the move. Avoid a recursive watch on a
  shared parent: every sibling's activity reaches the stream, and a busy
  sibling (a build) is enough to make FSEvents drop events.

### Reading other processes' descriptors and cwd

`sniff/lib/src/filesystem/query/macos.rs` implements this. Traps it handles:

- **Do not use the `libproc` crate.** `pids_by_path` is not recursive: it
  matches only an fd or cwd that is exactly the queried vnode. It silently
  ignores the `is_volume` and `exclude_event_only` flags, `pidcwd` is "not
  implemented for macos", and its `build.rs` needs libclang.
- **Use raw `libc` instead.** `proc_pidinfo(PROC_PIDLISTFDS)` plus
  `proc_pidfdinfo(PROC_PIDFDVNODEPATHINFO = 2)` gives each fd's canonical
  `/private/…` path, `vst_dev`, `vst_ino`, and `fi_openflags`.
  `PROC_PIDVNODEPATHINFO` gives the cwd. `libc` does not define
  `proc_fileinfo`, `vnode_fdinfowithpath`, or that constant, so define them
  locally.
- `PROC_PIDLISTFDS` fills only the buffer you pass and does not say when it was
  too small. Retry with a bigger buffer whenever the returned count equals the
  capacity. `pbi_nfiles` is the table capacity, not the number of open files.
- These calls return **0, not -1**, on failure and set `errno`. Zero `errno`
  before each call: a return of 0 with `errno` still 0 means an empty table.
- Another user's process fails every per-PID call with `EPERM`, while
  `proc_listpids` still lists it. Treat that as "not inspected", never as "no
  matches". `PROC_PIDTBSDINFO`, the only flavor with a start time, fails too,
  so such a process has no creation token.
- An `O_EVTONLY` open shows up (`fi_openflags = 0x8001`). It is a descriptor,
  not proof of an FSEvents subscription.
- `vst_dev` is `u32`, but `MetadataExt::dev()` sign-extends the `i32`
  `st_dev`. Compare with `vst_dev as i32 as u64`, or devices with the high bit
  set never match.

## Host conditions that look like repo failures

- **`ld: library 'git2' not found` after `brew upgrade libgit2`.**
  `libgit2-sys` links Homebrew's library and records its versioned Cellar path
  in its build-script output. Builds made before the upgrade fail to link, and
  builds made after it link fine, so the failure looks specific to one change.
  Confirm with `grep link-search target/debug/build/libgit2-sys-*/output`.
  `cargo clean -p libgit2-sys` did not help. Prefix the command with
  `LIBGIT2_NO_PKG_CONFIG=1` to build the vendored libgit2, which takes a
  one-time C build. For a scoped local run, a process-local
  `LIBRARY_PATH=/opt/homebrew/opt/libgit2/lib` also links, once you confirm the
  library exists there.
- **`CDPATH` sends a relative `cd` into the main checkout.** Agent shells here
  inherit an exported `CDPATH` that lists the main checkout and does not start
  with `.`. From a linked worktree, `cd sniff` lands in the main checkout's
  `sniff/` and exits 0. Gates then test and lint the wrong tree and report
  green, and `sed -i` edits go to the wrong file. You can tell when the first
  log line names the main checkout or the test counts don't match the branch.
  Fixes:
  - In scripts, `unset CDPATH` (or use `env -u CDPATH`) and `cd` to absolute
    paths or `./area`. Log `pwd` first.
  - After any edit made through a relative `cd`, check
    `git -C <main checkout> status`.
  - An exported `CDPATH` also makes `cd` print the path, so `$(cd … && pwd)`
    captures it twice. Run `.githooks/tests/test-pre-push.sh` as
    `env -u CDPATH bash .githooks/tests/test-pre-push.sh`.
- **Signing fails or opens pinentry because Claudine shadows `HOME`.** An agent
  session can inherit `HOME=~/.claudine` (Claudine's directory), while the
  OpenPGP keyring lives under the login home. Run commit, verify, and push with
  both `HOME=<login home>` and `GNUPGHOME=<login home>/.gnupg`. If a
  `gpg-agent` already started with the wrong home, kill it and relaunch it with
  both variables set; setting only one is not enough.
- **Kitty L2 runs need a normal, uncovered window.** A background instance
  with no remote control makes every `*_in_kitty` test skip. Minimized
  instances get panes only a few columns wide and give results that vary from
  run to run; such a run is no evidence either way. Hidden instances are never
  drawn (their screenshots are black) and delay OSC 10/11 and
  device-attribute replies until the program exits, which corrupts the next
  shell line. What works is a normal, unfocused instance per test with an
  exact cell size: `biscuit_test_harness::kitty::KittyInstance`
  (`biscuit-test-harness/src/kitty.rs`). It still renders nothing while
  another window covers it (`open -g` puts it behind the frontmost app). For
  real pixel evidence, grant Screen Recording and make sure nothing covers the
  window; the graph tests skip visibly otherwise. To stop an instance, use
  `kitty @ … action quit` with `confirm_os_window_close=0`; it ignores
  `SIGTERM`.
- **A shell prompt shows up in a captured L2 frame.** The WezTerm harness
  starts an interactive login shell. A first-run tool prompt (such as Atuin's)
  swallows the typed command, the `claudine_rc:` marker never appears, and the
  test times out with an empty program frame. This reproduces in isolation
  too. The tell is prompt text in the `plain:` block and no program output.
  Fix the tool's setup on the host instead of chasing the diff.
- **`tmux send-keys failed`, or WezTerm `attach/spawn`/`initial capture`
  timeouts that hit different tests each run.** Another harness on this host
  (another worktree's `just test-l2`, a pre-push hook, or anything else driving
  `wezterm cli`) is competing for the same sessions or mux. Check `ps` for
  `biscuit-harness-broker`, `nextest`, or `just ci-local`. When `wezterm cli
  list` stalls too, and `sample <gui-pid> 5` shows the GUI main thread in
  `GetPaneRenderChangesResponse`, you have hit the WezTerm client-turn limit.
  Ownership, throttling, and cleanup are covered in the `biscuit-test-harness`
  skill's `sharing-and-parallel-runs.md`.
- **Terminal detection runs `defaults`.** On an iTerm2 host,
  `biscuit_terminal::Terminal::default()` runs `defaults read
  com.googlecode.iterm2 …` for font settings. It checks only `TERM_PROGRAM`,
  not whether there is a TTY. A test that shims `defaults` must match on argv.
  Don't add a TTY guard: glyph choice for redirected output depends on these
  queries.
- **Shell init can deadlock without a TTY.** Editors capture the environment by
  running `$SHELL -l -i -c '… --printenv'` through pipes. If shell init loads
  completions through process substitution, this hangs, and in Zed "no
  language server starts". Guard completion loading with
  `if [ -t 0 ] || [ -t 1 ]`. To diagnose, run `pgrep -fl printenv`, then
  `sample <pid>`, and look for `run_init_scripts`.
- **`#!/usr/bin/env bash` runs `/bin/bash` 3.2 on a stock Mac.** It has no
  `declare -A`, `mapfile`, `${v,,}`, or `${args[*]@Q}`. Under `set -u` it
  rejects an empty `"${arr[@]}"`; write `${arr[@]+"${arr[@]}"}`. Homebrew's
  Bash 5 hides the problem from whoever has it first on PATH. The cross-check
  shipping tests call `/bin/bash` explicitly.
- **System Python is 3.9.** CI Python helpers must run on 3.9 (no
  `TestCase.enterContext`; use `addCleanup`). An L1 fixture that narrows
  `PATH` to `<fixture>/bin:/usr/bin:/bin` also picks up that `python3`, so a
  script that needs `tomllib` fails only on macOS.
  `tools/test-toolkit/tests/common/kache.rs` links the host's `python3` into
  the fixture `bin/` for this reason.
- **`dyld: Library not loaded: @rpath/libLLVM.dylib` / `failed to invoke LLD:
  signal: 6` on a kache-wrapped link** (typically wasm, for example
  `just zed-wasm`). The prebuilt kache binary is signed with the hardened
  runtime, and dyld removes every `DYLD_*` variable from a hardened process,
  so rustup's `DYLD_FALLBACK_LIBRARY_PATH` never reaches `rust-lld`. The
  toolchain is fine. Re-sign kache ad hoc (`just install-kache` does this and
  checks it with `scripts/kache-host.sh probe-passthrough`). The `kache`
  skill's `installation.md` has the details. A fake wrapper in a fixture that
  models this stripping must `unset DYLD_FALLBACK_LIBRARY_PATH` explicitly,
  because a plain Bash passthrough can keep the variable on hosted macOS.
- **CoreAudio client setup is serialized across processes.** The first
  `AudioObjectGetPropertyData*` call initializes the HAL, about 0.75 s of
  serial time each here (seven third-party HAL plug-ins): 1 process 1.1 s,
  6 together 4.6 s each, with under 0.2 s of CPU apiece. Tests that enumerate
  audio devices slow down by their count, not core count; keep them few
  (sniff's `testing.md`).
- **L1 timeouts when heavy jobs run at the same time.** Sniff's CoreAudio and
  network-interface probes, and tree-hugger's corpus tests, can exceed the
  30 s L1 budget when they run beside Clippy, other compiles, or other suites
  at default concurrency. If unrelated tests slow down together, finish the
  competing work and rerun with bounded workers (`--test-threads 4`) before
  you change timeouts or suspect a code defect.

## Diagnosing a slow host

- **Compare load average with idle CPU:** `sysctl -n vm.loadavg` and
  `top -l 1 -n 0 | grep 'CPU usage'`. High load with idle CPU means threads
  are blocked, not that the CPU is short. The usual cause is a hung network
  mount: an SMB automount over a WAN never fails cleanly, and every `/Volumes`
  listing gets stuck on it. Quit the automounter before `umount`, or the share
  mounts again within seconds.
- `ps -o %cpu` is a decaying average. Use `top -l N` and read the second or a
  later sample. In `sample` output, count the leading sample counts, not the
  stack lines.
- After an OS upgrade, third-party menu-bar and window overlays are the first
  suspects for WindowServer load.

## Shared-cache dylibs

Some system dylibs exist only in dyld's shared cache, so a missing
`/usr/lib/*.dylib` file does not mean the dependency is missing. Use
`otool -L` for an emitted binary's dependencies and `dyld_info -uuid
<install-name>` for cached libraries. `dyld_info -linked_dylibs` marks some
imports as `weak-link`, meaning dyld allows them to be absent (libobjc's
`libobjc-env.dylib`, for example). Follow only the required dependencies, or
valid programs get rejected.

## Audio fixture discovery outside PATH

Sniff's program discovery looks in macOS application bundles after PATH. So a
private PATH that holds only fake players does not prove that the discovered
player list holds only fixtures. Playa launches players by bare name, so a
bundle outside PATH causes a spawn error instead of launching. For controlled
fallback tests, keep a recording player that supports volume in PATH. Test
that incapable players are rejected when the command is built.

## Proving a call was eliminated

To show that a walk, scan, or launch was removed, count calls with an lldb
breakpoint on the test binary. This needs no root and no instrumentation:

```bash
xcrun lldb --batch \
  -o "breakpoint set -r '<function>' --auto-continue true" \
  -o "process launch -- <filter> --test-threads=1" \
  -o "breakpoint list" -- target/debug/deps/<test-binary>
```

Count only the location whose `where` is the bare function in its own file. A
regex breakpoint also matches generic and closure instantiations. Strip ANSI
codes before you match libtest output. lldb refuses Apple-signed platform
binaries but attaches to ones you built. Pick the binary with the same `-p`
selection the recipe uses; feature unification can otherwise give you a
different artifact.
