# WSL2

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

WSL2 (`wsl2-ubuntu`) is a distinct supported environment, not "Linux again".
The CI contract is `.github/workflows/_wsl-ci.yml`; this page records what it
means for a test author and how to reproduce its failures. The guest follows
**Linux** code paths on a `windows-latest` host, so it is never evidence for
native Windows ([ci-runners.md](ci-runners.md)); Linux traps
([linux.md](linux.md)) apply here too.

## The archive-mode contract

The guest compiles nothing. It downloads the same
`build-<package>-ubuntu-latest-<key>` nextest archive native Linux consumes and
publishes its own `{package, wsl2-ubuntu, L1}` cell. It has no rustup, cargo,
or toolchain. Path rules for archive-executed tests (`bin_exe!`,
`manifest_dir!()`, `archive_path_guard.rs`) are in the `rust-testing` skill's
[ci-execution.md](../rust-testing/ci-execution.md). What is WSL-specific:

- **Symptom of a baked builder path:** `Os { code: 2, kind: NotFound }` before
  the test observes anything.
- **No `cargo`/`rustc` at test time.** Recipes reach nextest through the
  `BISCUIT_NEXTEST_BIN` / `BISCUIT_JUNIT_*` passthrough. A suite that must
  shell out to the toolchain declares `requires-toolchain = true` under
  `[package.metadata.ci.tests]`; the planner then renders it as a governed
  `ACCEPTED GAP` (owner, expiry) on environments lacking `cargo_toolchain`.
  It declares absent coverage; never use it to quiet an ordinary failure.
- **`/bin/sh` is dash.** A bash builtin dies with `sh: 1: [[: not found`,
  which surfaces as the *recipe* failing, not the test.
- **The guest checks out to its own `GUEST_ROOT`** (`/home/biscuit/checkout`),
  matching no producer path on purpose. Do not recreate the producer's
  workspace path there; that hides the next baked path instead of surfacing
  it.
- **Verification runs in the guest**, because the predicates that matter
  (architecture, libc, resolved dynamic libraries) are the guest's;
  `host_runtime()` in `scripts/ci-build-archive.rs` reads `cfg!` and would
  report the Windows host.
- **The guest cannot write `$GITHUB_OUTPUT`** (a Windows path). Results cross
  the 9p workspace as files under `wsl-timing/`; a guest that dies leaves
  none, and its stages report absent rather than zero.

## Faithful reproduction on the WSL host

First identify the failing step and read the test summary. Passing tests
followed by an upload or finalization error are not a test regression; do not
rerun the suite to diagnose them. An instruction not to rerun WSL covers both
`cross-check` and pushes that trigger CI; if `BUILD_WSL` is unset or evidence
is insufficient, explain the gap before launching anything. When authorized:

```bash
just cross-check <package> --os wsl <test-name-substring>
```

It syncs the standing clone, builds an archive, **hides the builder's target
directory**, and runs with the workspace remapped. Running an archive on the
machine that built it does not reproduce a baked-path failure by itself,
because the baked path still resolves; hiding the target directory is the
load-bearing step. Manual variation, with `<clone dir>` from the `clone:` line
`cross-check` prints:

```bash
ssh -o BatchMode=yes "$BUILD_WSL" 'bash -lc "
  cd <clone dir>/rusty-biscuit
  cargo nextest archive -p <pkg> --archive-file /tmp/a.tar.zst
  mv target target.hold && rm -rf /tmp/x && mkdir -p /tmp/x
  cargo nextest run --archive-file /tmp/a.tar.zst \
    --workspace-remap <clone dir>/rusty-biscuit --extract-to /tmp/x \
    --no-fail-fast <test-name-substring>
  mv target.hold target
"'
```

- `--test <name>` is rejected with `--archive-file`; filter by substring or
  filterset. `--extract-to` must already exist.
- Confirm the "before" failure matches CI's panic line, then fix and rerun
  both the hidden-target archive run and a native `cargo nextest run`.
- Cheap approximation on any host for toolchain-dependence bugs: run the
  built test binary with `cargo` and `rustc` stripped from `PATH`.

## Reading the CI failure

The job log rarely shows the panic. Download `junit-<pkg>-L1-wsl2-ubuntu`
(`gh run download <run-id> -n <artifact>`) and read `<failure>` in
`L1/<pkg>.xml`; `expected-L1.json` beside it is the expected-test listing. The
cell name rides on the delegating `wsl2 (…)` job segment because the guest
job's own name is static. Which step went red:

- **"Resolve this cell's execution contract"**: a `cell-contract-*` refusal
  on the Windows host (`scripts/ci/cell_contract.py`) before any guest exists.
  A planner or workflow defect, never a test failure.
- **"List this cell's expected tests"**: runs each archived binary, so it
  fails for the gate's reasons (missing native library, sidecar, baked path),
  earlier.
- **"Certify this cell's completeness"** with tests green:
  `scripts/ci/completion.py` refused the cell; stderr names each
  `completion-*` reason. Usually `completion-test-missing`, which a skip
  approval cannot excuse; only an observed `<skipped/>` can be approved.
- **"Boot the guest and verify it is WSL2"** / **"Install guest packages"**:
  provisioning, before any repository code ran (see below).

## Provisioning failures

### Guest provisioning 403 (fixed, do not re-diagnose)

A `403` while installing `just` in the guest is GitHub's anonymous API rate
limit, which a "latest"-resolving installer hits
([ci-runners.md](ci-runners.md)). The guest now downloads a pinned
`JUST_VERSION` release tarball, so no API call remains. The nearby "the WSL2
guest failed to provision" text is a later diagnostic, not the cause.

### Lost runner during provisioning (open)

"The hosted runner lost communication with the server", the job killed about
45 minutes in, no log: the runner agent died during guest provisioning, cause
unknown (other projects see the same on Windows runners).

- Treat it as an unexplained failure, not noise; do not amend criteria or
  rerun by hand to clear it.
- `.github/workflows/ci-infra-retry.yml` reruns the job once automatically,
  only when every failure in the run was a lost runner. Fix any real failure
  first.
- Find the dying phase from the job's step list (`gh api
  .../actions/jobs/<id>` survives the loss); record it here when it recurs.

## Guest provisioning

The apt set is deliberately thin: `ca-certificates curl git xz-utils jq
python3`. Non-obvious entries:

- **`jq`** parses the native-prerequisite list before any recipe could
  bootstrap it; without it the guest cannot install the libraries archived
  binaries link.
- **`python3`** runs `scripts/ci/completion.py` inside the guest, because the
  reports and listing live on guest ext4 the host cannot read. It is stdlib
  Python because there is no compiler there.
- Both are proven executable in the step that installs them, so a broken
  package fails provisioning instead of looking like a validator defect later.

The guest receives one dispatch row and resolves the rest from the plan on the
Windows host **before** provisioning, so an unscheduled row is refused before
building a guest. `nextest list` runs as the unprivileged `biscuit` user
because the gate writes into its staging directory; the two extractions are
sequential so peak VHDX growth (which can exhaust the host disk) is unchanged.

## Level 2 on WSL

CI has no Level 2 backend in the guest: a temporary policy gap
(`.github/ci/environments.json`; see the standing rule in
[SKILL.md](SKILL.md)). Locally the guest runs tmux-backed L2 suites:

```bash
BISCUIT_TEST_REQUIRED_BACKENDS=tmux just cross-check claudine-cli --os wsl --features terminal-tests <filters>
```

Set `BISCUIT_TEST_REQUIRED_BACKENDS` (`cross-check` forwards it) so a missing
backend fails instead of skipping; a skipped WezTerm test still prints PASS.

## The guest's `~/.config` is a network share

`~/.config` in the guest is a CIFS mount of a NAS share (`findmnt ~/.config`
shows its source). When the NAS is down, two things fail before any test
runs:

- **Every `git` command:** `fatal: unable to access
  '~/.config/git/config': Host is down`. Bypass with
  `GIT_CONFIG_GLOBAL=/dev/null`.
- **Any cargo build:** `failed to determine package fingerprint` →
  `Could not read repository exclude` → `Host is down`. Cargo's gitoxide
  reads `$XDG_CONFIG_HOME/git/ignore`, which `GIT_CONFIG_GLOBAL` does not
  reach; point `XDG_CONFIG_HOME` at an empty local directory.

`scripts/cross-check.sh` sets both, so the recipe survives a down NAS. A slow
login shell (and a slow first tmux capture) while the share is down is the
host, not the test.

## Source snapshots and stale archive binaries

A tar overlay keeps the source host's modification times. When they predate
a package's existing build artifacts, Cargo can reuse an older test binary,
so the archive silently omits a new test the area run included. Touch the
overlaid sources with the guest clock (or clean that package in the private
target) before building the archive, and compare the expected test names with
the PASS listing. `--extract-to` needs an existing destination directory.

## drvfs volumes reject `RENAME_NOREPLACE` and ignore mode bits; inotify instances are scarce

Measured 2026-10-11 on the WSL guest:

- **`renameat2(RENAME_NOREPLACE)` returns `EINVAL` on drvfs** (`/mnt/<drive>`,
  9p), even for a move to an absent name; ext4 and tmpfs support it. An
  existing destination still returns `EEXIST` (the VFS checks it first), so
  `EEXIST` does not prove support, and a drvfs permission failure also returns
  `EINVAL`. Code that must move without replacing treats `EINVAL` as
  "unsupported here" and refuses before changing anything.
- **drvfs mode bits are made up and `chmod` does nothing** (re-measured
  2026-10-23 on `/mnt/c`, mounted `9p` with `aname=drvfs;…;uid=1000;gid=1000`
  and no `metadata` option): every file and directory reads `0o777`, owned
  by the WSL user, before and after `chmod 700`/`600`, and `mkdir` with mode
  `0o700` still gives `0o777`. A Unix "owner-only, no group/other bits"
  check can therefore never pass there; `wt remove`'s recovery store failed
  with "other users can access it" until it skipped the bit check on drvfs
  (access there is NTFS ACLs, as on native Windows). Check ownership, not
  bits, and recognize drvfs from the mount table (`9p` with `aname=drvfs`).
- **`max_user_instances` is 128** (`max_user_watches` 524288,
  `max_queued_events` 16384; `BUILD_LINUX` allows 65536 instances). Each inotify instance counts, so
  many concurrent watchers hit `EMFILE` there long before they would on
  `BUILD_LINUX`.
