# WSL2

WSL2 (`wsl2-ubuntu`) is a distinct supported environment, not "Linux again".
The CI contract that makes it different is in `.github/workflows/_wsl-ci.yml`
and `fixes/2026-07-27-refactor/wsl-archive-requirements.md`; this file records
what that contract means for a test author and how to reproduce its failures.

## The archive-mode contract

CI builds `x86_64-unknown-linux-gnu` test binaries once on `ubuntu-latest`
with `cargo nextest archive` and only *runs* them inside the WSL2 guest. The
guest has no rustup, no cargo, and no toolchain. Consequences:

- Anything resolved at **compile time** to a builder path does not exist in
  the guest. `env!("CARGO_BIN_EXE_<name>")` names the builder's target
  directory; the test dies with `Os { code: 2, kind: NotFound }` before it can
  observe anything. Use `biscuit_test_harness::bin_exe!("<name>")`, which
  prefers nextest's run-time republication (`NEXTEST_BIN_EXE_<name>`, with
  hyphens rewritten) and falls back to the compile-time value.
- Anything that shells out to `cargo` or `rustc` at test time fails in the
  guest. That includes recipes: the canonical `just` test chain has a
  passthrough (`BISCUIT_NEXTEST_BIN`, `BISCUIT_JUNIT_TARGET_DIR`,
  `BISCUIT_JUNIT_WORKSPACE_ROOT`) that the workflow verifies is still wired.
- The archive does include the package's non-test binaries and the linked
  paths nextest knows about, so a correctly resolved `bin_exe!` works.

The guest follows **Linux** code paths on a `windows-latest` host. Its build
phase is only the archive download and extraction, and its wall clock is
dominated by slow test execution, so it is never evidence for
native Windows behavior and is never compared with the `windows-latest` leg
as one environment ([ci-runners.md](ci-runners.md)).

## Faithful reproduction on the WSL host

A test that is green natively but red only on `wsl2-ubuntu` is almost always
an archive-mode failure. Reproduce it on the guest declared by `BUILD_WSL`
(if unset, this machine has no WSL host; say so and use CI):

```bash
just cross-check <package> --os wsl <test-name-substring>
```

That recipe syncs the standing clone to your local tree, builds a nextest
archive, hides the builder's target directory, and runs the archive with the
workspace remapped, which is the same shape as the CI leg. Proven
non-vacuous on 2026-09-09: shipping the broken `CARGO_BIN_EXE` form of
`claudine-gen::steering_check` through it fails with CI's exact panic, and the
`bin_exe!` form passes 162/162.

Running the archive on the same machine that built it does **not** reproduce
the failure on its own, because the baked builder path still resolves; hiding
the target directory is the load-bearing step. The manual equivalent, for
when you need to vary the procedure:

```bash
ssh -o BatchMode=yes "$BUILD_WSL" 'bash -lc "
  cd ~/ci-verification/rusty-biscuit
  cargo nextest archive -p <pkg> --archive-file /tmp/a.tar.zst
  mv target target.hold
  rm -rf /tmp/x && mkdir -p /tmp/x
  cargo nextest run --archive-file /tmp/a.tar.zst \
    --workspace-remap ~/ci-verification/rusty-biscuit --extract-to /tmp/x \
    --no-fail-fast <test-name-substring>
  mv target.hold target
"'
```

- `--test <name>` is rejected together with `--archive-file`; filter by a
  name substring or a filterset.
- The `--extract-to` directory must already exist.
- Confirm the "before" failure matches CI's panic line, then apply the fix
  (`scp` the file into the clone), rebuild the archive, and rerun both the
  hidden-target archived run and a native `cargo nextest run -p <pkg>`.
- A cheaper approximation for toolchain-dependence bugs on any host: run the
  built test binary directly with `cargo` and `rustc` stripped from `PATH`.
  That is the guest's world.

## Reading the CI failure

The WSL job's log rarely shows the panic. Download the `junit-<pkg>-L1-
wsl2-ubuntu` artifact from the run (`gh run download <run-id> -n <artifact>`)
and read the `<failure>` element in `L1/<pkg>.xml`; it carries the panic
message and the source line.

## Guest provisioning 403 (fixed, do not re-diagnose)

The guest once produced no test report at all, intermittently, with a `403`
while installing `just` from `just.systems/install.sh`. The script resolves
"latest" through `api.github.com` **anonymously** (60 requests per hour per
IP, shared across GitHub's runner fleet). Every other environment installs
`just` through a GitHub Action with the job token. A GitHub Action cannot run
inside the guest, so the installer is called with an explicit `--tag` and a
token instead. The nearby log text "the WSL2 guest failed to provision" is a
later diagnostic step, not the cause.

## Level 2 on WSL

There is no Level 2 terminal backend in the WSL2 CI environment (recorded as a
policy gap with owner and expiry in `.github/ci/environments.json`). This is
temporary. Do not write it into a spec as an authorized exclusion; record the
criterion as unmet and name provisioning as the required change. Locally,
the guest can run tmux-backed L2 suites when tmux is installed — and it does:
on 2026-09-10 the claudine partial-file family (tmux capture, the `expectrl`
PTY suite, and its L1 neighbors) passed 17/17 inside the guest in archive
mode with `--features terminal-tests`. The recipe is
`just cross-check claudine-cli --os wsl --features terminal-tests <filters>`;
add `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` when running by hand so a skip
cannot print as a pass (a WezTerm test skips in the guest in ~0.03 s and
nextest still says PASS).

## The guest's `~/.config` is a network share

`~/.config` in the guest is a CIFS mount of `//192.168.100.97/config`, a
share on the Synology NAS — the same share the Windows side's `wezterm.lua`
reaches for. When the NAS is down (`ping -c1 -W2 192.168.100.97` from the
guest), two things fail before any test runs:

- **Every `git` command** dies with `fatal: unable to access
  '/home/ken/.config/git/config': Host is down`, so `cross-check --os wsl`
  fails at its first `git fetch`. `GIT_CONFIG_GLOBAL=/dev/null` bypasses it;
  fetch/reset/clean/checkout/apply need no identity.
- **`cargo nextest archive` (any cargo build)** dies with `failed to determine
  package fingerprint for build script for playa` → `Could not read
  repository exclude` → `Host is down`. Cargo lists package files through
  gitoxide, which honors the global excludes at `$XDG_CONFIG_HOME/git/ignore`
  — on the dead mount — and `GIT_CONFIG_GLOBAL` does not reach it. Point
  `XDG_CONFIG_HOME` at an empty local directory for the build.

Nothing the tests read lives on that share, so both bypasses are safe for a
run. A login shell in the guest is also slow while the share is down
(profile tooling stats paths under `~/.config`), which shows up as a
20-second-plus first tmux capture; that is the host, not the test.
`scripts/cross-check.sh` does not yet export either variable in its
`unix_prelude`; doing so would let the recipe survive the share being down.
