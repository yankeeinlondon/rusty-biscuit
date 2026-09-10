# macOS

macOS is the primary development host, so most of what is non-obvious here is
about what the host can do for the *other* operating systems and about host
conditions that masquerade as repository defects.

## Paths

- `/var` and `/tmp` are symlinks into `/private`. A test that compares a
  `tempfile` path with what a child process reports must canonicalize on
  Unix (the `launched_spelling` helper does) or the two spellings differ.
  This is the macOS half of the same trap Windows has with short names
  ([windows.md](windows.md)).
- `dirs::home_dir()` honors `HOME` here, which is why a hermetic-home test can
  be green on macOS and read the real home directory on Windows.

## Linux and Windows evidence from this host

"macOS-only host, cross-platform runs not executable" is wrong here.

- **Real Linux runs:** Docker Desktop provides a `linux/arm64` kernel. `open
  -a Docker`, poll `docker info` (about 20 s), then run the package tests in a
  container. PTY-backed L2 tests pass there too. Mount the source as a copy
  (`rsync -a --exclude=target/ --exclude=.git`) because a worktree's `.git`
  is a file pointing outside the container. Two traps: keep
  `CARGO_TARGET_DIR` on a host mount (`-v /tmp/x-target:/t -e
  CARGO_TARGET_DIR=/t`), since the VM overlay is small and fills; and a
  whole-package `cargo nextest run` OOM-kills the linker in the default VM,
  so use `--memory=7g`, `CARGO_BUILD_JOBS=2`, and targeted `--lib` /
  `--test <name>` runs. podman's VM does not start on this host.
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

## Host conditions that look like repo failures

- **A shell prompt inside a captured L2 frame.** The L2 WezTerm harness
  spawns an interactive login shell, so anything the host's shell startup
  does interactively lands in the pane. A first-run tool prompt (seen with
  Atuin's "AI is not yet configured") swallows the typed command, the
  `claudine_rc:` exit marker never appears, and the test times out with an
  empty program frame. It can reproduce in isolation, so isolation does not
  discriminate; the tell is prompt text in the `plain:` block and no program
  output. Dismiss or configure the tool on the host; do not chase the diff.
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
  confirm with `pwd`.

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
