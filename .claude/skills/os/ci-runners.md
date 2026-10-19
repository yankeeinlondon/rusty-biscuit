# Hosted CI Runners

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

The four CI legs (`ubuntu-latest`, `macos-latest`, `windows-latest`, and the
`wsl2-ubuntu` guest on a `windows-latest` host) are different machines. Start
concurrency caps, timing comparisons, and "why is this leg slow" from the facts
below. Environment declarations, capabilities, and gaps live in
`.github/ci/environments.json`; the workflow contract is `.github/ci/README.md`.

## Runner sizes

GitHub's standard runners (GitHub's published runner specs):

| Label | Cores | Memory |
|---|---|---|
| `ubuntu-latest`, `windows-latest` (public repository) | 4 | 16 GB |
| `ubuntu-latest`, `windows-latest` (private repository) | 2 | 7 GB |
| `macos-latest` | 3 | 7 GB |
| `macos-13` | 4 | 14 GB |
| `macos-latest-xlarge` (paid) | 6 | 14 GB |

This repository is public. **macOS is the tightest runner, not the roomiest.**
A comment, cap, or override that assumes "two-core runners" is out of date;
re-measure it rather than trust it.

## Per-leg profile

Shapes are stable across runs; absolute times are not, so none are kept here.

### Linux (`ubuntu-latest`)

- Fastest leg for the large suites (Claudine, Darkmatter).
- The **build owner** for itself and the WSL2 guest: one archive per planned
  key, consumed by L1, L2, browser, and the guest (`build.executes` in
  `environments.json`; [wsl.md](wsl.md)). Its test legs compile nothing.
- Shell installers that resolve "latest" through the GitHub API anonymously
  fail sporadically (60 requests/hour per shared runner IP). Use the setup
  actions, which authenticate with the workflow token.

### macOS (`macos-latest`)

- Slowest leg for Claudine's CLI suite, on the fewest cores. Give any
  concurrency cap its most conservative setting here; a cap tuned on Linux is
  wrong here by default.
- `/usr/bin/python3` is CPython 3.9 (see Cross-cutting).
- Level 3 (focus-stealing) tests never run on CI.

### Windows (`windows-latest`)

- Slowest **build** phase of the four; its test phase is not the bottleneck.
  The build is never shared with WSL2: the two are incompatible producers.
- Runs fewer test identities because `#![cfg(unix)]` binaries are excluded.
  Declare those as platform exclusions in the package's CI config, not as lost
  coverage.
- Process, console, and Ctrl+C traps for this leg are in
  [windows.md](windows.md) and [windows-console.md](windows-console.md).
- Sniff's host-network detections fail when they overlap, so its Windows L1
  group runs one process at a time (`sniff-windows-l1` in
  `.config/nextest.toml`).
- No L2 or browser cells: no terminal backend or headless browser is
  provisioned (`capabilities` in `environments.json`). This is a provisioning
  gap, not a design decision; see the standing rule in [SKILL.md](SKILL.md).

### WSL2 (`wsl2-ubuntu`)

- A VM behind a bash wrapper on a `windows-latest` host. Actions cannot run in
  the guest, so every tool there is installed by script, which is where the
  anonymous API limit bites ([wsl.md](wsl.md)).
- Runs the Linux-built archive, L1 only, never its own build. A test red only
  here is almost always an archive-mode failure; reproduce it with the
  builder's target directory hidden ([wsl.md](wsl.md)).
- Its wall clock is almost all test execution; the "build" is download and
  extraction.
- Follows **Linux** code paths. It is not evidence for native Windows, and the
  two are never compared as one environment.

## Cross-cutting

- **Python floor: 3.9 for anything a workflow step runs, 3.11 only for the
  planner.** `scripts/ci/companion_suites.py` runs on every native environment
  and imports `affected_scope`, so the planner must stay importable on macOS's
  3.9; it imports `tomllib` defensively and refuses by name only when it
  actually needs it. The planner itself runs only on `ubuntu-latest` and
  developer hosts. `ast.parse(source, feature_version=(3, 9))` catches new
  grammar but not new stdlib imports; check those by hand.
- **The working tree is LF on every OS** (`.gitattributes`: `* text=auto
  eol=lf`, overriding `core.autocrlf`), so a tracked file hashes the same on
  Windows and Linux. The plan's `skip_policy` hash relies on this.
- **Test worker budgets** follow the shared policy in the `rust-devops` skill
  ([ci-cd.md](../rust-devops/ci-cd.md)): on CI, all cores up to four. Per
  runner that means L1 uses 4 on Linux/Windows and 3 on macOS; L2 uses 2 on
  `ubuntu-latest` and 1 on `macos-latest`. Resource groups in
  `.config/nextest.toml` still cap Claudine L1 at four and Sniff Windows L1 at
  one; never add a package-wide Claudine CLI L1 cap, which would serialize
  thousands of tests on every runner.
- **Caches do not warm across runs.** The 10 GB repository cache quota cannot
  hold per-package, per-environment caches, so only the build owner, `check`,
  and `lint` cache; test tiers consume a verified archive and restore nothing.
  A cold build is not a cache-key bug, and never add a cache to a consumer: a
  restored cache can make a silent rebuild look fast.
- **A pre-push receipt from an older head counts only when its
  gate-input identity is unchanged:** the `git ls-tree` entries of the
  package's build closure plus that gate's global inputs, recomputed over both
  trees rather than read from the receipt.
- **A push to `main` cancels the in-flight `main` run**
  (`concurrency.group` in `.github/workflows/ci.yml` is keyed on the ref, and
  `main`'s ref is constant). Pull requests use their own merge ref and the
  nightly its own group, so they are unaffected; queue merges.
- **Merge-gate bypass.** The `protect-your-bacon` ruleset requires `ci-gate`;
  its bypass actors are recorded in `.github/ci/README.md`. Using or granting a
  bypass (`gh pr merge --admin`) is Ken's decision, never an agent's.

## Comparing legs and runs

- **Noise:** an identical tree varies 5 to 15% per leg run to run, so a delta
  inside that band is not a result. A timing claim needs three consecutive
  green runs per leg.
- **One environment at a time:** compare matched test identities within one
  environment only, never across OS. Report missing identities and platform
  exclusions separately so an exclusion is not read as a speed-up.
- **Stages, not job totals:** every cell reports seven windows (producer
  queue, compile+archive, upload; consumer download, verify, extract, execute;
  defined in [ci-cd.md](../rust-devops/ci-cd.md)). A flat total can hide a
  compile saving paid back in transfer. Report total runner compute and
  end-to-end critical path separately; moving a compile off several legs moves
  them in opposite directions.
- **Compiler-work counts** (`scripts/ci-build.rs`): exclude Cargo's stdin
  target-information query (`--crate-name ___ --print=...`, issued even on
  some warm builds) just like `rustc -vV`; keep invocations with `--emit` and
  real build-script probe compilations.
