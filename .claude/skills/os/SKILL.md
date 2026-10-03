---
name: os
description: |-
  Non-obvious, repo-specific knowledge for handling operating-system matters in
  rusty-biscuit: which hosts can produce macOS, Linux, native-Windows, and WSL2
  evidence and how to reach them; how the WSL2 CI leg runs (nextest archives)
  and how to reproduce its failures faithfully; Windows path-spelling,
  handle-inheritance, console, and cross-compile traps; Linux directory-order,
  locale, shallow-clone, and thread-count traps; macOS symlinked temp dirs,
  Docker-for-Linux, and host-diagnosis methods; hosted CI runner sizes,
  per-leg timing profiles, cross-run comparison rules, and per-OS evidence
  reuse. Load this before claiming "cannot test on X here", before tuning a CI
  thread cap or comparing leg timings, before touching `#[cfg(windows)]` or
  path comparison code, and whenever a test is red on exactly one CI
  environment.
---

# OS Matters

Every package must compile and behave on macOS, Linux, native Windows, and
WSL2. This skill records what is *not* obvious about doing that from this
repository: where each OS can be exercised, the traps that have already cost
real time, and the recipes that resolved them. Generic Rust portability
guidance lives in `prompts/cross-platform.md`; the `wsl` skill covers
configuring WSL itself; the `rust-testing` skill owns test design.

## Where each OS can be exercised

Build hosts are declared by environment variables (`BUILD_LINUX`, `BUILD_WIN`,
`BUILD_WSL`, `BUILD_MACOS`), each an SSH destination. A set variable means the
host is available from this machine; unset means it is not. Never hardcode an
alias; check `env | grep '^BUILD_'` first and report which hosts you had.

| Need | Use | Detail |
|---|---|---|
| Real Linux run | `$BUILD_LINUX` via `just cross-check <pkg> --os linux`, or Docker Desktop on a macOS host | [build-hosts.md](build-hosts.md), [macos.md](macos.md) |
| Native Windows run | `$BUILD_WIN` via `just cross-check <pkg> --os windows` | [build-hosts.md](build-hosts.md) |
| WSL2 run, exactly as CI does it | `$BUILD_WSL` via `just cross-check <pkg> --os wsl` (nextest archive, builder target dir hidden) | [wsl.md](wsl.md) |
| Another macOS | `$BUILD_MACOS` (reserved; no recipe consumes it yet) | [build-hosts.md](build-hosts.md) |
| Windows compile evidence only | `cargo check --target x86_64-pc-windows-gnu` (never msvc from macOS); an isolated probe crate for `#[cfg(windows)]` code | [windows.md](windows.md#compile-evidence-from-macos) |
| Authoritative proof | Hosted CI (`ubuntu-latest`, `macos-latest`, `windows-latest`, `wsl2-ubuntu`) | [ci-runners.md](ci-runners.md), `.github/ci/README.md`, `.github/ci/environments.json` |

CI is the final proof, not the discovery loop. Surface an OS's exact failure
on the matching host first, then push once. Before pushing, read
[ci-evidence.md](ci-evidence.md) for which receipt satisfies which
environment's cell and how to record an execution ban.

## Red on one environment only: start here

| Symptom | Likely cause | Read |
|---|---|---|
| Hook fixture red only on Ubuntu | missing local git identity, inherited default branch, or `gh` already in `/usr/bin` | [linux.md](linux.md#hook-fixtures-red-only-on-ubuntu) |
| L2 timeout only on Ubuntu, green on the macOS host | depth-1 CI checkout meets git-history code | [linux.md](linux.md#shallow-checkouts-l2-red-only-on-ubuntu-a-timeout) |
| Linux and WSL red, macOS and Windows green, only an order differs | ext4 `read_dir` hash order | [linux.md](linux.md#read_dir-order-red-on-linux-and-wsl-green-on-macos-and-windows) |
| ASCII `>` where `▶` was expected, only on `BUILD_LINUX` | `LANG=C` in that host's shells | [linux.md](linux.md#langc-on-the-build_linux-host-ascii-where-a-glyph-was-expected) |
| A substring count in rendered output red on Linux | temp-path length changes where text wraps | [linux.md](linux.md#temp-path-length-changes-word-wrapping) |
| A process count too high on Linux or WSL | `sysinfo` lists threads as processes | [linux.md](linux.md#sysinfo-lists-threads-as-processes) |
| Red only on `wsl2-ubuntu` | the guest runs an archive built on `ubuntu-latest`; builder paths baked in at compile time (`env!("CARGO_BIN_EXE_*")`, `CARGO_MANIFEST_DIR` fixtures, toolchain lookups; fix binaries with `biscuit_test_harness::bin_exe!`) | [wsl.md](wsl.md) |
| WSL guest red at provisioning with a 403 | anonymous GitHub API rate limit (fixed; do not re-diagnose) | [wsl.md](wsl.md) |
| WSL guest "lost communication with the server", killed at ~45 min, no log | runner agent died during provisioning (open, instrumented) | [wsl.md](wsl.md) |
| Red only on `windows-latest`, a path in the message | 8.3 short-name TEMP (`RUNNER~1`) or verbatim `\\?\` spelling | [windows-paths.md](windows-paths.md) |
| Red only on `windows-latest`, elapsed time equals some child's timeout | handle inheritance keeps pipes open | [windows.md](windows.md#environment-and-processes) |
| Red only on `windows-latest` with an empty failure message | a std handle redirected to `CONOUT$` | [windows-console.md](windows-console.md#attaching-a-console-inside-a-nextest-process) |
| Green gates that did not test your worktree | Bash `cd <area>` followed `CDPATH` into the main checkout | [macos.md](macos.md) |
| macOS host L2 red with a shell prompt in the captured frame | a host shell-startup prompt swallowed the input; not a repo defect | [macos.md](macos.md) |
| Slow on one leg only, or a timing delta under 15% | runner size and per-leg profile; noise is 5–15% per leg | [ci-runners.md](ci-runners.md) |

## Standing rules

- The Windows and WSL2 Level 2 CI cells are a **temporary policy gap**
  (`.github/ci/environments.json`, owner and expiry recorded there). Do not
  amend an acceptance criterion to exclude them; record the criterion as unmet
  with provisioning as the required change.
- A cross-compile is compile evidence, never behavioral evidence. Say which
  one you have.
- CI legs are compared within one environment only, on matched test
  identities, with three consecutive green runs as the evidence standard.
  WSL2 follows Linux code paths and is never evidence for native Windows
  ([ci-runners.md](ci-runners.md)).
- `ctx.*` path values and Markdown presentation are portable (`/`); an eager
  `file()`'s effective frontmatter value keeps native spelling. Tests compare
  against the spelling the surface actually emits (`biscuit_file`'s
  `to_portable_string` for portable surfaces).
- `#[cfg(unix)]` and `#[cfg(windows)]` test the **target**, not the build
  host. A `#![cfg(unix)]` inside a test file does not stop Cargo building that
  test's dev-dependencies for a Windows target; gate the dependency graph.
- Never override `CARGO_TARGET_DIR` on the `BUILD_WIN` host; its checkout
  pins the target dir to the `W:` volume for a reason
  ([build-hosts.md](build-hosts.md)).

## Files

| File | Covers |
|---|---|
| [build-hosts.md](build-hosts.md) | The `BUILD_*` contract, standing clones, storage rules, `just cross-check` usage and gotchas, compiler cache, remote-process hygiene |
| [ci-evidence.md](ci-evidence.md) | Per-cell evidence reuse across environments and commits, execution bans, `scope-only`, what the pre-push hook reviews |
| [ci-runners.md](ci-runners.md) | Hosted runner sizes (macOS is the tightest), per-leg build and execution profiles, the anonymous API limit, cache quota, `main` cancellation, merge-gate bypass, cross-run noise and comparison rules |
| [linux.md](linux.md) | Ubuntu hook fixtures, shallow checkouts, `read_dir` order, `LANG=C`, wrap-dependent counts, `sysinfo` threads |
| [wsl.md](wsl.md) | The archive-mode contract, faithful reproduction on `BUILD_WSL`, `bin_exe!`, the 403 history, the lost-runner investigation, the guest's apt set, Level 2 on WSL, which red step means what |
| [windows.md](windows.md) | Environment and processes (handle inheritance, Job Objects, stack size, killing `git`, `python3` alias, batch arguments, Ctrl+C), current-directory locks, the `windows-latest` leg, compile evidence from macOS |
| [windows-paths.md](windows-paths.md) | Verbatim `\\?\` paths, home and cache known folders, 8.3 short names, separators, `file://` URIs, Markdown escapes, junctions and reparse points |
| [windows-console.md](windows-console.md) | WezTerm on `build-win`, PowerShell output streams and encoding, a windowless ConPTY console for L2, attaching a console under nextest |
| [macos.md](macos.md) | `/var` symlink, Docker for Linux evidence, L2 capture wedges, terminal-detection forks, perf triage, lldb work counters, shell-init and `cd` traps, the kache `DYLD_*` link failure |

When you learn a new OS-specific fact the hard way, add it to the matching
file in the same change that fixes it, and add a row to the symptom table
when the symptom points somewhere non-obvious. Keep this file a router: facts
belong on topic pages.
