# Hosted CI Runners

The four CI legs (`ubuntu-latest`, `windows-latest`, `macos-latest`, and the
`wsl2-ubuntu` guest that rides on `windows-latest`) are not interchangeable
machines. This file records what each one actually is, so concurrency caps,
timing comparisons, and "why is this leg slow" questions start from facts
rather than from the two-core folklore. Environment declarations live in
`.github/ci/environments.json`; the workflow contract is `.github/ci/README.md`.

## Runner sizes

GitHub's standard free runners, as of 2026-09:

| Label | Cores | Memory |
|---|---|---|
| `ubuntu-latest`, public repository | 4 | 16 GB |
| `windows-latest`, public repository | 4 | 16 GB |
| `ubuntu-latest` / `windows-latest`, private repository | 2 | 7 GB |
| `macos-latest` | 3 | 7 GB |
| `macos-13` | 4 | 14 GB |
| `macos-latest-xlarge` (paid larger runner) | 6 | 14 GB |

This repository is public, so Linux and Windows get 4 cores and macOS gets 3.
**macOS is the tightest standard runner, not the roomiest.** Any comment,
thread cap, or override that assumes "two-core runners" predates GitHub's
January 2024 upgrade for public repositories and should be re-measured, not
trusted.

## Per-leg profile

The shapes below are stable across runs; the absolute seconds are not, so
none are recorded here (see "Noise" below).

### Linux (`ubuntu-latest`)

- Fastest leg for Claudine and Darkmatter.
- It is also the **builder** for the WSL2 nextest archive; the guest only
  executes what Linux compiled ([wsl.md](wsl.md)).
- Anonymous GitHub API calls from a runner are rate-limited at 60 per hour
  per IP, and the IPs are shared Azure addresses. Any shell-script installer
  that resolves "latest" through the API fails sporadically. Use the setup
  actions, which authenticate with the workflow token (5,000 per hour).

### macOS (`macos-latest`)

- Slowest leg for Claudine's CLI suite, taking more than twice the Linux
  time on fewer cores. Any concurrency cap gets its most conservative
  setting here, and a cap tuned on Linux is wrong here by default.
- The only leg where Level 3 (focus-stealing) tests could run, and they are
  not run on CI at all.

### Windows (`windows-latest`)

- Slowest **build** phase of the four legs, noticeably behind both Unix
  legs, while its test phase is not the bottleneck.
- Runs fewer test identities because `#![cfg(unix)]` binaries are excluded.
  Declare those as platform exclusions in the package's CI config rather than
  treating them as lost coverage.
- Handle inheritance, the batch-file argument rule, and the Unix-only Ctrl+C
  and exit-130 contract all apply on this leg; each is recorded in
  [windows.md](windows.md). The leg does not exercise Ctrl+C at all.
- Sniff's host-network detections can fail-fast a test process when they
  overlap, so its Windows L1 group runs one process at a time
  (`.config/nextest.toml`).
- Native Windows Level 2 (real terminal) legs do not exist yet. This is a
  provisioning gap, not a design decision; acceptance criteria must not be
  narrowed around it (see the standing rule in [SKILL.md](SKILL.md)).

### WSL2 (`wsl2-ubuntu`)

- A separate VM behind a bash wrapper on a `windows-latest` host. GitHub
  Actions cannot run inside the guest, so every tool there is installed by
  script, which is exactly where the anonymous API limit bites
  ([wsl.md](wsl.md), "Guest provisioning 403").
- It runs the archive built on Linux, never its own build. A test red only
  here is almost always an archive-mode failure; faithful local reproduction
  requires hiding the builder's target directory ([wsl.md](wsl.md)).
- Its build phase is only the archive download and extraction, so the
  leg's wall clock is almost entirely slow test execution.
- Follows **Linux** code paths. It is not evidence for native Windows
  behavior, and the two must never be compared as one environment.

## Cross-cutting

- **Test worker budgets follow the shared policy.** `_test_threads` in
  `just/devops.just` uses `max(1, logical_cores - 2)` locally; CI uses all cores
  through four and subtracts two above four. CI means `CI=true`,
  `GITHUB_ACTIONS=true`, or nonempty `BISCUIT_CI_ENVIRONMENT`, not merely the
  Nextest `ci` profile. Worker counts do not set CPU affinity, reserve cores,
  or change Cargo build jobs. Explicit thread settings and narrower CI-profile
  groups remain effective: Claudine L1 is capped at four, Claudine CLI L1 at
  one, and Sniff Windows L1 at one. Shared-resource L2 stays serial; isolated
  suites use the `l2-parallel-self-spawn` marker. See the
  [central policy](../../../docs/topics/ci-cd.md#layer-1--local-pre-push-hook)
  for override and direct Nextest behavior.

- **A clean pre-push replaces individual cells, not a whole environment.** The
  hook uses `sniff os --json` to distinguish macOS, Linux, native Windows, and
  WSL2, then publishes a validation receipt under
  `refs/notes/ci-local/<environment>` carrying one record per
  `{package, gate}` cell with its outcome, counts, duration, backend proof, and
  gate-input identity. CI reads **every** environment's ref reachable from the
  outgoing head, so this host's receipt and a prior `cross-check` WSL receipt
  omit their own cells in the same run. A cell from an older head is accepted
  only when its gate-input identity — the `git ls-tree` entries of the tested
  package's build closure plus that gate's global inputs — is unchanged, and
  the comparison is recomputed over both trees rather than read out of the
  receipt. `lint` and `check` stage no report and are always CI-origin; browser
  and companion-suite work remains in CI.

- **Caches do not warm across runs.** Per-package, per-environment caching
  cannot survive the 10 GB repository cache quota: one full run saves more
  caches than the quota holds and evicts its own predecessors. Only intra-run reuse works
  (the L2, browser, and WSL-archive jobs restore the key their own run saved).
  Do not diagnose a cold build as a cache-key bug.
- **A push to `main` cancels the in-flight `main` run.** The concurrency
  group is `ci-${{ github.ref }}`, and for `main` that ref is constant. A
  full-scope `main` run takes several hours. PR runs use
  `refs/pull/N/merge`, a different group, and are unaffected; pushing
  branches while waiting is free, merges must queue.
- **The merge-gate ruleset has no admin bypass by default.** `gh pr merge
  --admin` is refused with "Repository rule violations found" unless a
  bypass actor has been granted explicitly; that grant is Ken's decision.
- **Noise.** Run-to-run variation on an identical tree is 5 to 15% per leg,
  so a delta inside that bracket is not a result. The evidence standard for a
  timing claim is three consecutive green runs per leg. Legs are never merged
  or compared across OS.
- **Compare matched test identities within one environment only.** Report
  missing identities and platform exclusions separately from the timing
  comparison, so an exclusion is never mistaken for a speed-up.
