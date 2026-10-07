# macOS

macOS is the primary development host, so most of what is non-obvious here is
about what the host can do for the *other* operating systems and about host
conditions that masquerade as repository defects.

## Paths

- APFS on the primary macOS host rejects `U+FDD1` in a filename with
  "Illegal byte sequence". Git metadata can still record a missing path
  containing it. Exercise such renderer inputs through a real Git record's
  `gitdir` text or the public rendering API rather than creating that filename.
- `/var` and `/tmp` are symlinks into `/private`. A test that compares a
  `tempfile` path with what a child process reports must canonicalize on
  Unix (the `launched_spelling` helper does) or the two spellings differ.
  This is the macOS half of the same trap Windows has with short names
  ([windows-paths.md](windows-paths.md)).
- The same split reaches production code: a launch directory from
  `current_dir()` is physical (`/private/var/…`) while `$HOME` keeps its
  authored spelling (`/var/…`), so a lexical `launch == home` or
  "is this local path the user one?" check silently misses when `$HOME` has a
  symlinked component. An in-process unit test that builds both from one
  `TempDir` spelling cannot see it; an L1 subprocess launched from the
  fixture `$HOME` does. Compare by identity (`fs::canonicalize`) where two
  such spellings meet — Claudine's `with_prompt_magic_roots` does
  (`2026-09-23-local-before-home`).
- macOS periodically sweeps files under `/tmp` that have not been accessed
  for a few days while leaving populated directories in place. A git worktree
  created there (the `rb-*-baseline` and `rb-*-review` checkouts) loses its
  small `.git` pointer file first, which `git worktree list` then reports as
  `prunable`. Do not treat that state as corruption; `git worktree prune`
  clears the registration, and long-lived worktrees belong under
  `~/.claudine/worktrees`, not `/tmp`.
- Case-insensitive APFS folds names by a Unicode rule, not by lowercasing: `ς` opens a stored
  `Σ` and `ß` opens a stored `SS`, though their `to_lowercase()` strings
  differ. Decide whether a spelling is the stored one by an exact
  `read_dir` entry, never by comparing folded strings. When the parent can
  be traversed but not listed (mode `0111`), `fs::canonicalize` still
  reports the stored final name (`locked/anchor/DOCS` canonicalizes to
  `…/docs`), unless that name is a symlink. biscuit-file's absolute-glob
  spelling check relies on both facts (verified 2026-10-02).
- macOS temporary directories can live on a case-sensitive filesystem. Do
  not assert that a differently cased ASCII or Unicode name opens merely
  because `cfg!(target_os = "macos")` is true. Probe the fixture directory's
  behavior and keep the ordinary exact-name and mismatch assertions on both
  filesystem types. Reproduced 2026-10-02 with a temporary HFSX image:
  `hdiutil create -size 40m -fs HFSX -volname glob-review-case -type UDIF /tmp/glob-review-case.dmg`,
  attach with `hdiutil attach -nobrowse -mountpoint /tmp/glob-review-case-mount /tmp/glob-review-case.dmg`,
  then set `TMPDIR=/tmp/glob-review-case-mount` on the area's `just test`
  command. Detach the image and remove it afterward. These commands need no
  administrator prompt or foreground window.
- `dirs::home_dir()` honors `HOME` here, which is why a hermetic-home test can
  be green on macOS and read the real home directory on Windows. The shared
  `biscuit_file::home_dir()` follows `USERPROFILE` on Windows instead, so a
  fixture setting both variables lands in the same place on every OS.
- A Unix socket path holds at most 104 bytes (`sun_path`), and the per-user
  `$TMPDIR` (`/private/var/folders/xx/…/T/`) spends about half of that. A
  test that starts a socket-binding daemon under a `tempfile` directory can
  fail with "path must be shorter than SUN_LEN" — `kache daemon run` does,
  binding `daemon.control.v2.sock` inside its store. Root such a fixture
  under `/tmp` when `$TMPDIR` is long, as `real_kache_worktree_restore`
  (`tools/test-toolkit`) does; `/tmp` is on the same APFS volume.

## Process groups

- One `kill(-pgid, SIGKILL)` does not reach a child that a group member is
  forking at that moment: XNU does not make `fork` atomic with a group signal,
  so the child joins the group unsignalled and survives. Killing
  `sh -c 'echo started; sleep 300'` on reading `started` left `sleep` running
  in 63–74 of 2,000 tries on this host, and in 0 of 2,000 in a Linux
  container. Repeat the group kill until it fails. A group that holds only
  zombies answers `EPERM` on macOS but `0` on Linux, so "until it fails" ends
  on macOS while the unreaped leader is a zombie and spins on Linux; Claudine's
  `ProcessTree` drop repeats only off Linux, under a 250 ms bound.
- The escaped child holds whatever the group inherited. With a test's stderr
  inherited, nextest reports `LEAK-FAIL` about 30 s after the test passed,
  even though every pid the test watched was reaped.

## Linux and Windows evidence from this host

"macOS-only host, cross-platform runs not executable" is wrong here.

- **Real Linux runs:** Docker Desktop provides a `linux/arm64` kernel. `open
  -a Docker`, poll `docker info` (about 20 s), then run the package tests in a
  container. PTY-backed L2 tests pass there too. Mount the source as a copy
  (`rsync -a --exclude=target/ --exclude=.git --exclude=.gitnexus/`) because a worktree's `.git`
  is a file pointing outside the container. Excluding `.gitnexus/` is required: its
  parsed-file store churns during indexing, so rsync exits 23 on vanished files. Put that copy
  and the target under `$HOME` (e.g. `~/.cache/<scratch>`): Docker Desktop's default file
  sharing rejects both `/Volumes/...` and `/tmp` (`/private`) bind mounts with
  `mkdir /Volumes: read-only file system` (found 2026-09-16). Two traps: keep
  `CARGO_TARGET_DIR` on a host mount (`-v ~/.cache/x-target:/t -e
  CARGO_TARGET_DIR=/t`), since the VM overlay is small and fills; and a
  whole-package `cargo nextest run` OOM-kills the linker in the default VM,
  so use `--memory=7g`, `CARGO_BUILD_JOBS=2`, and targeted `--lib` /
  `--test <name>` runs. podman's VM does not start on this host.
  On 2026-09-17, bind mounts under `/tmp` failed ("error while creating mount
  source path '/private/tmp/…': mkdir /private: read-only file system"): the
  host's Docker file sharing does not cover `/private/tmp`. Put both the
  source copy and the target dir under `$HOME` (for example
  `~/.cache/rb-linux/{src,target}`). `rust:1` needs
  `apt-get install libdbus-1-dev pkg-config` for `messenger --features desktop`,
  plus `libasound2-dev` for anything that reaches Playa's `native-playback`
  (all of `claudine-cli`).
  Run the container command with `bash -c`, not `bash -lc`: the login shell
  resets `PATH` and drops `/usr/local/cargo/bin`, so every `cargo` call fails
  with "command not found" while a trailing pipe can still exit 0. `rust:1`
  has no `cargo-nextest`; `cargo test --test <name>` avoids a slow install.
- **Linux Level 3 keyboard tests** that use `biscuit_test_harness::xvfb`
  (a private `Xvfb` plus kitty, XTEST presses) run in the same container and
  never touch the Mac's desktop. `rust:1` needs `xvfb kitty libgl1-mesa-dri`
  on top of the packages above, and `cargo-nextest` from
  `https://get.nexte.st/latest/linux-arm`. kitty 0.41 under Xvfb works with
  software OpenGL and no window manager; `SetInputFocus` alone gives it focus
  (2026-09-28). Its "Failed to connect to DBUS" log lines are harmless.
- **Windows compile evidence:** the `x86_64-pc-windows-gnu` target; details
  and the msvc prohibition are in [windows.md](windows.md).
- **Behavioral Windows and WSL2 evidence:** the build hosts
  ([build-hosts.md](build-hosts.md), [wsl.md](wsl.md)).

## The `macos-latest` leg

GitHub's standard `macos-latest` runner has 3 cores and 7 GB, fewer than the
4-core Linux and Windows runners this public repository gets, and it is the
slowest leg for Claudine's CLI suite by more than 2x. Tune any concurrency
cap to this leg, not to Linux. It is also the only leg where Level 3
focus-stealing tests could run, and they do not run on CI at all. Details in
[ci-runners.md](ci-runners.md).

## Finding filesystem watchers

An open-file scan does not enumerate FSEvents directory watches. On 2026-10-05,
a Node `fs.watch(path, { recursive: true })` process with cwd `/` received a
created-file event, but `lsof -nP -a -p <pid> -Fftn` contained no watched path.
The probe used a temporary directory and waited for delivery before inspecting
its descriptors. A process whose cwd is inside a worktree may still be found
through cwd inspection; that is evidence of directory use, not a watch record.

Apple's `FSEventStreamCopyPathsBeingWatched` inspects a stream reference owned
by the caller, not a system-wide list of other processes' registrations. Do
not treat an empty `lsof` or vnode-descriptor scan as proof that no process
watches a directory. See the [FSEvents guide](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/UsingtheFSEventsFramework/UsingtheFSEventsFramework.html)
and [Node's platform mapping](https://nodejs.org/api/fs.html#availability).

### Reading other processes' descriptors and cwd

Measured 2026-10-05 on macOS 27.2 (arm64) for Sniff's path-usage query:

- The `libproc` crate (0.14.11) is a poor fit. `pids_by_path` /
  `pids_by_type_and_path` is **not recursive**: it matches only processes whose
  fd or cwd is the queried vnode itself, never anything beneath a directory.
  Its `is_volume` and `exclude_event_only` flags are silently ignored, because
  `sys/macos.rs` passes `pathflags` only on the sizing call. `pidcwd` returns
  "not implemented for macos". Its `build.rs` also runs bindgen, so the build
  needs libclang.
- Raw `libc::proc_pidinfo(PROC_PIDLISTFDS)` plus
  `proc_pidfdinfo(PROC_PIDFDVNODEPATHINFO = 2)` gives each vnode fd's canonical
  `/private/...` path, `vst_dev`, `vst_ino`, and `fi_openflags`.
  `proc_pidinfo(PROC_PIDVNODEPATHINFO)` gives the cwd the same way. `libc`
  lacks `proc_fileinfo`, `vnode_fdinfowithpath`, and that constant, so define
  them locally.
- `PROC_PIDLISTFDS` fills only the buffer you pass and does not report that it
  was too small. Retry larger whenever the returned count equals the capacity.
  `pbi_nfiles` is table capacity, not the open count.
- An `O_EVTONLY` open is visible (`fi_openflags = 0x8001`). It is a
  descriptor, not proof of an FSEvents subscription.
- Another user's process (PID 1) fails every per-PID call with `EPERM`, while
  `proc_listpids` still lists every PID. Treat EPERM as "not inspected", never
  as "no matches". That includes `PROC_PIDTBSDINFO`, the only flavor with a
  start time, so such a process has no creation token;
  `PROC_PIDT_SHORTBSDINFO` succeeds for it but carries no start time.
- `proc_pidinfo`/`proc_pidfdinfo` return **0, not -1**, on failure and set
  `errno`. Zero `errno` before the call: `PROC_PIDLISTFDS` returning 0 with
  `errno` still 0 is an empty table, not an error.
- `vinfo_stat.vst_dev` is `u32`, but `st_dev` is `i32` and
  `std::os::unix::fs::MetadataExt::dev()` sign-extends it. Compare with
  `vst_dev as i32 as u64`, or a device with the high bit set never matches.

## Host conditions that look like repo failures

- **A Homebrew upgrade of `libgit2` breaks linking with `ld: library 'git2'
  not found`.** `libgit2-sys` links the system library when `pkg-config`
  finds a compatible one, and its build-script output records the versioned
  Cellar path (`/opt/homebrew/Cellar/libgit2/1.9.4/lib`). After `brew upgrade`
  moves it (1.9.7 on 2026-10-03), every feature combination whose
  `libgit2-sys` build predates the upgrade fails to link (for example
  `cd sniff && just test`, `just check-tier-coverage sniff`), while another
  combination built later links fine, so it looks like a change-specific
  failure. Confirm with `grep link-search target/debug/build/libgit2-sys-*/output`.
  `cargo clean -p libgit2-sys` did **not** help: the rerun build script wrote
  the same stale path (the cached build under `kache` is the suspect, not
  proven). Prefixing the command with `LIBGIT2_NO_PKG_CONFIG=1` builds the
  vendored libgit2 instead and links (one-time C build, about 4 min under
  load).

- **`CDPATH` sends a relative `cd` from a linked worktree into the main
  checkout.** Agent shells on this host inherit a `CDPATH` that lists
  `/Users/ken/coding/personal/rusty-biscuit` and does not start with `.`. In
  Bash, `cd sniff` from a worktree root therefore lands in the **main
  checkout's** `sniff/`, prints that path, and exits 0. A gate script that
  loops `cd "$area" && just test` then tests and lints the wrong tree and
  reports green. On 2026-09-25 all 12 Phase 4 gates of the worktree fix
  `2026-09-24-ux-improvements` ran against the main checkout this way. The
  giveaways were a first log line naming `/Users/ken/coding/...` and
  test counts that did not match the branch. In scripts, `unset CDPATH` and
  `cd` to absolute paths (or `./area`), and log `pwd` first. It redirects
  edits too: on 2026-10-03 a `cd darkmatter/lib/src && sed -i …` issued while
  the shell was already inside that directory wrote into the main checkout.
  After any relative-`cd` edit, check `git -C <main checkout> status`.
- **Claudine currently shadows the login home for agent sessions on this
  host.** An agent can inherit `HOME=/Users/ken/.claudine` even though the
  login home and OpenPGP keyring are under `/Users/ken`. A signed Git command
  then looks in the wrong keyring or opens pinentry despite the Keychain-backed
  signing key needing no interaction. Run commit, verification, and push with
  both `HOME=/Users/ken` and `GNUPGHOME=/Users/ken/.gnupg`. If a wrong-home
  agent was already started, kill and relaunch `gpg-agent` with those same two
  values; setting only one did not prevent the dialog on 2026-09-12.
- **A background Kitty is not an L2 host.** The dev host has `kitty` but no
  remote-control instance, so every `*_in_kitty` test skips in `just test-l2`.
  On 2026-09-23 an instance started without taking focus (`open -g -n -a kitty
  --args -o allow_remote_control=yes --listen-on unix:<sock>
  --start-as=minimized`) gave panes a few columns wide and no graphics
  detection. 19 or 20 of `biscuit-terminal-cli`'s 25 Kitty tests failed, on the
  unmigrated base too, and the failing set varied between runs. Treat such a
  run as no evidence either way. It also ignores `SIGTERM`: stop it with
  `kitty @ --to unix:<sock> close-window --match all`.
  What does work (2026-09-25): a *normal* (not minimized, not hidden)
  instance per test, `open -g -n -a kitty --args --config NONE -o
  allow_remote_control=socket-only -o initial_window_width=<N>c …`, which
  stays unfocused, has exactly the cells asked for, and can be screenshotted
  with `screencapture -x -o -l <platform_window_id from kitty @ ls>`. That is
  `biscuit_test_harness::kitty::KittyInstance`. A hidden instance's
  screenshot is black (never drawn). Stop an instance with `kitty @ --to …
  action quit`; without `confirm_os_window_close=0` that opens a "Quit
  kitty?" window instead.
  An *occluded* instance's screenshot is empty too (2026-09-28): Kitty does
  not render a window another window covers, and `open -g` puts it behind
  the frontmost app. On the dev Mac it opened at the same spot on the second
  display, under a full-height Zed window, so 100×32 and 56×60 windows were
  always empty and 200×60 ones only when a strip stuck out; a second
  instance also covers the first. Grant Screen Recording and uncover that
  area for real pixel evidence; the Kitty graph tests skip visibly otherwise.
- **A script's `cd <area>` lands in the main checkout.** The dev Mac's
  shell exports `CDPATH` (with `~/coding/personal` and the main checkout
  among its entries), so in a script or subshell run from a linked worktree,
  a relative `cd worktree && just test` can resolve to
  `~/coding/personal/rusty-biscuit/worktree` and quietly test the wrong tree
  (seen 2026-09-27: every gate green, counts matching an older state). Use
  absolute paths or `env -u CDPATH`, and check that the recipe's first
  output line (the area path) names the worktree.
- **A shell prompt inside a captured L2 frame.** The L2 WezTerm harness
  spawns an interactive login shell, so anything the host's shell startup
  does interactively lands in the pane. A first-run tool prompt (seen with
  Atuin's "AI is not yet configured") swallows the typed command, the
  `claudine_rc:` exit marker never appears, and the test times out with an
  empty program frame. It can reproduce in isolation, so isolation does not
  discriminate; the tell is prompt text in the `plain:` block and no program
  output. Dismiss or configure the tool on the host; do not chase the diff.
- **`tmux send-keys failed` on the first send of an L2 run, green when rerun
  alone.** Another test run on this host (a second worktree's `just test-l2`
  or pre-push hook; check `ps` for `biscuit-harness-broker`, `nextest`, or
  `just ci-local`) spawned its own harness and its stale-resource reaper
  killed this run's shared session, whose tag named the already-exited
  broker. Fixed by tagging with the recipe's pid
  (`BISCUIT_HARNESS_OWNER_PID`, see the `biscuit-test-harness` skill); the
  send error now carries tmux's own message and the live session list. If
  it recurs, read that list before blaming the test.
- **Terminal detection forks `defaults`.** On an iTerm2 host,
  `biscuit_terminal::Terminal::default()` spawns `defaults read
  com.googlecode.iterm2 New Bookmarks` up to three times (font name, size,
  ligatures), gated only on `TERM_PROGRAM`, not on a TTY. A test that shims
  `defaults` on `PATH` must discriminate on argv, or it misattributes the
  appearance probe. Do not "fix" this by adding a TTY guard to the font
  queries; glyph selection for redirected output depends on them.
- **Shell-init deadlock without a TTY.** Editors that capture the project
  environment by running `$SHELL -l -i -c 'zed --printenv'` through pipes can
  wedge when shell init sources completions via process substitution. The
  symptom is "no language server starts at all" in Zed. Guard
  completion-loading in shell init with `if [ -t 0 ] || [ -t 1 ]`. Diagnose
  with `pgrep -fl printenv` and `sample <pid>` showing `run_init_scripts`.
- **Relative `cd` into a package area can land elsewhere** when `CDPATH` is
  set; the shell stays at the repo root and `just lint`/`just test` run the
  root recipes, surfacing unrelated packages' failures. Use absolute paths and
  confirm with `pwd`. An *exported* `CDPATH` also makes `cd` print the path,
  so `$(cd … && pwd)` captures it twice: `.githooks/tests/test-pre-push.sh`
  then fails at once with `FAIL: hook not executable at <root>\n<root>/…`.
  Run it as `env -u CDPATH bash .githooks/tests/test-pre-push.sh`.
- **`#!/usr/bin/env bash` selects `/bin/bash` 3.2 on a stock Mac**, which
  under `set -u` rejects `"${arr[@]}"` on an empty array as unbound and aborts
  the script. Homebrew's Bash 5 accepts it, so the script runs for whoever has
  Homebrew first on PATH and fails for everyone else. Write
  `${arr[@]+"${arr[@]}"}` and avoid `mapfile`, `declare -A`, and `${v,,}` in
  any shell script that is `#!/usr/bin/env bash`.
- **`dyld: Library not loaded: @rpath/libLLVM.dylib` / `failed to invoke LLD:
  signal: 6 (SIGABRT)` on a kache-wrapped link** (typically wasm, e.g.
  `just zed-wasm`). The prebuilt kache binary carries the hardened runtime,
  and dyld strips every `DYLD_*` variable from a hardened process. rustup's
  `DYLD_FALLBACK_LIBRARY_PATH` never reaches `rust-lld`. The toolchain is not
  broken: do not symlink `libLLVM.dylib` into it or export `DYLD_*` in
  recipes. Re-sign kache ad hoc (`just install-kache` does this and gates on
  `scripts/kache-host.sh probe-passthrough`). Measured 2026-09-23; details in
  the `kache` skill's `installation.md`.

## Diagnosing a slow host

- Pair load average with idle CPU: `sysctl -n vm.loadavg` and `top -l 1 -n 0
  | grep 'CPU usage'`. High load with an idle CPU means blocked threads,
  never CPU shortage; the usual cause is a hung network mount (an SMB
  automount reached over a WAN never fails cleanly, and every `/Volumes`
  enumeration parks a thread). Quit the automounter before `umount`, or the
  share re-mounts within seconds and the fix looks like it failed.
- `ps -o %cpu` is a decaying average, not an instantaneous reading; use
  `top -l N` and read the second or later sample. In `sample` output count
  the leading sample counts, not stack lines. Aggregate process CPU hides
  which thread is hot.
- After an OS upgrade, third-party menu-bar and window overlays are the first
  suspect for WindowServer load.

## Audio fixture discovery outside PATH

Sniff's program discovery falls back to macOS application bundles after PATH.
A private PATH containing only a fake `aplay` still discovered `mpv.app` during
the silent-audio fix. Therefore a private PATH does not prove that the installed
player list contains only fixture programs. Playa currently launches host players
by bare binary name, so an out-of-PATH bundle produces a spawn error; it does not
launch that bundle. Keep a volume-capable recording player in PATH for controlled
fallback tests, and test incapable-player rejection at command construction.

## Proving a call was eliminated

When a change claims a walk, scan, or launch was *removed*, count it with an
lldb breakpoint on the test binary; no root and no instrumentation needed:

```bash
xcrun lldb --batch \
  -o "breakpoint set -r '<function>' --auto-continue true" \
  -o "process launch -- <filter> --test-threads=1" \
  -o "breakpoint list" -- target/debug/deps/<test-binary>
```

Count only the location whose `where` is the bare function in its defining
file; a regex breakpoint also resolves generic and closure instantiations.
Strip ANSI before matching libtest's output. lldb refuses Apple-signed
platform binaries but attaches to locally built ones. Resolve the binary with
the same `-p` selection the recipe uses, or feature unification names a
different artifact.

## CI shell and runtime inspection (2026-09-15)

`/bin/bash` is 3.2: associative arrays and `${args[*]@Q}` are unavailable, and
empty indexed arrays need `${args[@]+"${args[@]}"}` under `set -u`. The
cross-check shipping tests must use `/bin/bash` explicitly on macOS. Python CI
helpers support Python 3.9; `TestCase.enterContext` requires a newer interpreter,
so temporary resources use `addCleanup` or a context manager.

An L1 fixture that narrows the child `PATH` to `<fixture>/bin:/usr/bin:/bin`
also narrows `python3` to that 3.9 interpreter, so a script needing `tomllib`
fails only on macOS. `tools/test-toolkit/tests/common/kache.rs` links the test
host's `python3` into the fixture `bin/` for this reason (2026-09-23).

System dylibs may exist only in dyld's shared cache. `otool -L` reads an emitted
binary's dependencies, but an absent `/usr/lib/*.dylib` file does not establish
that its dependency is missing. `dyld_info -uuid <install-name>` observes cached
library identities without loading test programs or opening a window.

When walking shared-cache dependencies, `dyld_info -linked_dylibs` marks
`weak-link` imports that dyld permits to be absent. macOS 27's libobjc lists
`/usr/lib/libobjc-env.dylib` this way; treating it as a required library falsely
rejects valid programs. Traverse required dependencies and preserve the weak
import distinction.
