---
name: os
description: |-
  Non-obvious, repo-specific knowledge for handling operating-system matters in
  rusty-biscuit: which hosts can produce macOS, Linux, native-Windows, and WSL2
  evidence and how to reach them; how the WSL2 CI leg runs (nextest archives)
  and how to reproduce its failures faithfully; Windows path-spelling,
  handle-inheritance, cross-compile, and console traps; macOS symlinked temp
  dirs, Docker-for-Linux, and host-diagnosis methods; hosted CI runner
  sizes, per-leg timing profiles, and the cross-run comparison rules. Load
  this before claiming "cannot test on X here", before tuning a CI thread cap
  or comparing leg timings, before touching `#[cfg(windows)]` or path
  comparison code, and whenever a test is red on exactly one CI environment.
---

# OS Matters

Every package must compile and behave on macOS, Linux, native Windows, and
WSL2. This skill records what is *not* obvious about doing that from this
repository: where each OS can actually be exercised, the traps that have
already cost real time, and the recipes that resolved them. Generic Rust
portability guidance lives in `prompts/cross-platform.md`; the `wsl` skill
covers configuring WSL itself; the `rust-testing` skill owns test design.

## Where each OS can be exercised

| Need | Use | Detail |
|---|---|---|
| Real Linux run | The host in `$BUILD_LINUX` via `just cross-check <pkg> --os linux`, or Docker Desktop on a macOS host | [build-hosts.md](build-hosts.md), [macos.md](macos.md) |
| Native Windows run | The host in `$BUILD_WIN` via `just cross-check <pkg> --os windows` | [build-hosts.md](build-hosts.md) |
| WSL2 run, exactly as CI does it | The guest in `$BUILD_WSL` via `just cross-check <pkg> --os wsl` (nextest archive, builder target dir hidden) | [wsl.md](wsl.md) |
| Another macOS | The host in `$BUILD_MACOS` (reserved; no recipe consumes it yet) | [build-hosts.md](build-hosts.md) |

Build hosts are declared by environment variables (`BUILD_LINUX`, `BUILD_WIN`,
`BUILD_WSL`, `BUILD_MACOS`), each an SSH destination. A set variable means the
host is available from this machine; unset means it is not. Never hardcode an
alias; check `env | grep '^BUILD_'` first and report which hosts you had.
| Windows compile evidence only | `cargo check --target x86_64-pc-windows-gnu` (never msvc from macOS); an isolated probe crate for `#[cfg(windows)]` code | [windows.md](windows.md) |
| Authoritative proof | Hosted CI (`ubuntu-latest`, `macos-latest`, `windows-latest`, `wsl2-ubuntu`) | [ci-runners.md](ci-runners.md), `.github/ci/README.md`, `.github/ci/environments.json` |

CI is the final proof, not the discovery loop. A full-scope run takes hours
and every push cancels the previous one, so surface an OS's exact failure on
the matching host first, then push once.

A user's instruction not to rerun an environment also constrains CI triggered
by a push. Verify every requested exclusion before pushing; a successful macOS
receipt alone does not suppress WSL. Evidence is now combined **per cell**
across environments and across commits: `local_evidence.py verify --cells`
reads every note on every `refs/notes/ci-local/<environment>` ref between the
merge base and the outgoing head, so a macOS receipt from this push and a prior
`cross-check` WSL receipt suppress their own cells together, and a WSL receipt
that covered one package does not hide an earlier one that covered another.
The JUnit reports behind a hook receipt stay on the
host that produced it, at the receipt's `host.report_dir`
(`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/`, root default
`~/.rusty-biscuit/ci-evidence`), so investigating a reused cell means asking
that host. Record the restriction rather than remembering it —
`<home>/.rusty-biscuit/ci-constraints/<repository>/` (`BISCUIT_CI_CONSTRAINTS_DIR`
overrides it; the home is Python's `Path.home()`, so `USERPROFILE` on native
Windows) holds prohibition records that `just ci-local --plan` and the pre-push
hook refuse on, and that CI deliberately never reads.
If the plan still schedules a prohibited cell, explain the gap before
triggering CI; do not treat automatic jobs as exempt.

`scope-only` resolves and prints the plan — so a recorded constraint is still
enforced and the run is still reviewable — but runs no gate, publishes no
outcomes, and excludes no CI cells. It does publish the *scope* receipt on
`refs/notes/ci-local/scope`, as every mode does, and CI takes it on an exact
`{base, head, tree}` match. The hook reviews every pushed branch's COMMITTED
plan (a temporary worktree unless the revision is the clean checkout) under that
update's remote branch and remote, against the base of each run it triggers
(the remote's `main` for a push to `main`; each open pull request's target tip
on a GitHub remote, read through `gh`, plus the incoming revision when the same
push updates that target too; otherwise a provisional plan against the
remote's `main`), and publishes HEAD's; `just ci-local --plan`
previews the working tree and differs exactly when the checkout is dirty. `off` is its deprecated alias. `git push
--no-verify` produces no new evidence and does not invalidate already-published
matching receipts, which CI still verifies. See the `rust-devops` skill's
[evidence and execution contract](../rust-devops/ci-cd.md) before selecting a
push mode.

## Read this first when a test is red on one environment only

- **Red only on `wsl2-ubuntu`:** the guest runs a nextest *archive* built on
  `ubuntu-latest`. Anything resolved at compile time to a builder path
  (`env!("CARGO_BIN_EXE_*")`, `CARGO_MANIFEST_DIR` fixtures outside the
  archived paths, toolchain lookups) breaks there. Reproduce with the recipe in
  [wsl.md](wsl.md); the fix for binaries is `biscuit_test_harness::bin_exe!`.
- **Red only on `windows-latest`:** GitHub's runner has an 8.3 short-name TEMP
  (`RUNNER~1`) that no developer machine has, plus verbatim `\\?\` spellings
  from `canonicalize`. Read the path-spelling traps in
  [windows.md](windows.md) before reading the test.
- **Red only on `windows-latest` with an elapsed time equal to some child's
  timeout:** handle inheritance. Detached grandchildren keep the parent's pipe
  ends open on Windows; see [windows.md](windows.md).
- **Red only on the macOS host, L2, with a shell prompt in the captured
  frame:** a host shell-startup prompt swallowed the input; see
  [macos.md](macos.md). Not a repo defect.
- **Red in the WSL guest at provisioning with a 403:** anonymous GitHub API
  rate limit from a shell-script installer; fixed once, recorded in
  [wsl.md](wsl.md) so it is not re-diagnosed.
- **Slow on one leg only, or a timing delta under 15%:** read the runner
  sizes and per-leg profile in [ci-runners.md](ci-runners.md) before calling
  it a regression. macOS has the fewest cores, Windows the slowest build,
  WSL2 the slowest execution, and run-to-run noise is 5 to 15% per leg.

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
  pins the target dir to the `W:` volume for a reason ([build-hosts.md](build-hosts.md)).

## Files

- [build-hosts.md](build-hosts.md) — the `BUILD_*` declaration contract,
  standing clones, storage rules, `just cross-check` usage and gotchas,
  remote-process hygiene.
- [wsl.md](wsl.md) — the archive-mode contract, faithful reproduction on the
  `BUILD_WSL` guest, `bin_exe!`, the guest's GitHub API 403 history.
- [ci-runners.md](ci-runners.md) — hosted runner sizes (macOS is the
  tightest), per-leg build and execution profiles, the anonymous API limit,
  cache-quota and `main`-cancellation behavior, merge-gate bypass, and the
  cross-run noise and comparison rules.
- [windows.md](windows.md) — path spelling, home directory lookup, handle
  inheritance, batch-file argument rule, Ctrl+C status, cross-compile targets,
  Markdown backslash escapes.
- [macos.md](macos.md) — `/var` symlink, Docker for Linux evidence, L2
  capture wedges, subprocess forks from terminal detection, perf triage,
  lldb work counters, shell-init and `cd` traps.

When you learn a new OS-specific fact the hard way, add it to the matching
file in the same change that fixes it. That is what keeps this skill portable
across agents instead of living in one agent's memory.
