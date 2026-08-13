# CI policy — packages and environments

CI selects, executes, and records work by **package** — a Cargo workspace
member. Every result identity — artifact name, JUnit manifest record, rollup
cell — is keyed on `{package, environment, tier}`.

- **Package policy** lives in the package's own manifest, under
  `[package.metadata.ci]`. `scripts/ci/affected_scope.py` reads and validates
  it.
- **Environment capabilities** live in `environments.json` (below). One
  versioned, schema-validated table.
- **Known-red legs** live in `ci-baseline.toml`, keyed by package.

There is no per-directory policy store and no concept of a "package area" in
CI. `just test` in a directory still runs that directory's packages for local
use (R8); CI does not read that list.

## `environment` is not `os`

Windows, macOS, and Linux are operating systems. **WSL2 is a distinct supported
Linux environment that a Windows runner hosts.** Policy and every result
identity are keyed by **environment**; only `runs-on` and the native-package
lookup are keyed by runner OS.

| environment | hosted by runner label | notes |
|---|---|---|
| `ubuntu-latest` | `ubuntu-latest` | |
| `windows-latest` | `windows-latest` | |
| `macos-latest` | `macos-latest` | |
| `wsl2-ubuntu` | `windows-latest` | runs through `wsl-bash`; see `.github/workflows/_wsl-ci.yml` |

`affected_scope.py` derives the per-package workflow inputs from the
environment capability table so the reusable workflow can never route
`wsl2-ubuntu` into a `runs-on` matrix: `native_environments` (the environment
names that *are* runner labels), `l2_environments`, `browser_environments`,
`node_environments`, and `wsl` (a boolean).

A WSL2 guest *is* Linux, so `_ensure-native-libs` keys off `uname -s` and reads
the package's `ubuntu-latest` list. `native` therefore stays a **runner OS**
map (keyed by `ubuntu-latest`/`macos-latest`/`windows-latest`) and must not
grow a `wsl2-ubuntu` key.

Cargo metadata — not this file — remains the source of truth for package
membership.

## `environments.json`

One versioned, schema-validated capability table. It defines, for each
environment: the `runner` that hosts it, the `native_key` that maps it to a
native-package installer, and a `capabilities` map over a closed vocabulary:

| capability | meaning |
|---|---|
| `tmux` | whether a headless L2 terminal backend can be provisioned here |
| `headless_browser` | whether a headless browser can be hosted here |
| `node_pnpm` | whether Node 22 + pnpm 10 are provisioned here |
| `archive_only` | whether this environment runs from a prebuilt nextest archive (no Cargo) |

A capability value is either a boolean or, for a **governed unavailability**, an
object carrying `available: false` plus `reason`, `owner`, and `expiry`. A
plain `false` is an **ungoverned** absence — the `POLICY GAP` cell it produces
is never excused and blocks. The facts the eight per-area `policy_gaps`
records used to restate — Windows has no tmux, the WSL2 leg is archive-only,
and the browser tier is Linux-hosted — are now declared once, here, with full
governance.

An L2 tier is hostable where ANY of its declared backends is: each backend is
looked up as a capability under its own name (`tmux` today; `wezterm`,
`kitty`, and `apple-terminal` get the same axis if they ever become
CI-hostable), and a backend with no capability entry is hostable nowhere.

Capability only: package policy decides *which* tiers are expected, so an
unsupported required tier becomes an explicit `POLICY GAP` in the grid rather
than disappearing. `affected_scope.py::load_environments` validates the schema
loudly; `affected_scope.py::package_ci_policy` cross-checks every package's
declared native packages against the runner labels the table defines.

## `[package.metadata.ci]`

Package policy lives in the package's own manifest, following the
`[package.metadata.benchmarks]` pattern already used across this workspace:

```toml
[package.metadata.ci]
gates = false
exclusion-class = "promotion-pending"
owner = "@yankeeinlondon"
reason = "…"
expiry = "2027-01-31"

[package.metadata.ci.native]
ubuntu-latest = ["libasound2-dev"]

[package.metadata.ci.tests]
tiers = ["L1", "L2"]
l2-backends = ["tmux", "wezterm"]
features = ["playa"]
all-features = false
l1-include-slow = false
runner-tools = ["ai-provider-stubs", "claudine-provider-fixture", "darkmatter-md-fixture"]
companion-suites = ["homelab-frontend"]
```

A package with no CI metadata defaults to `gates = true` and the L1 tier —
non-gating is never inferred from zero observed tests (AC15), because that
would silently exempt a package and miss its first test.

### Fields

| Field | Type | Default | Meaning |
|---|---|---|---|
| `gates` | bool | `true` | whether the package fans out in CI |
| `exclusion-class` | string | when `gates = false` | `capability`, `promotion-pending`, or `time-bounded` |
| `owner` | string | when `gates = false` | GitHub handle accountable for closing the exclusion |
| `reason` | string | when `gates = false` | why it does not gate, and what would unblock it |
| `expiry` | ISO date | when `gates = false` unless `capability` | a **past** date fails the scope calculation |

`[package.metadata.ci.tests]`:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `tiers` | string[] | `["L1"]` | CI-gating tiers this package owns. Must include `L1`; `L2`/`browser` are opt-ins |
| `l2-backends` | string[] | `[]` | L2 terminal backends this package's tests require; one of `tmux`, `wezterm`, `kitty`, `apple-terminal`. Required when `L2` is declared |
| `features` | string[] | `[]` | forwarded to check, archive, and the canonical recipe consistently. Conflicts with `all-features` |
| `all-features` | bool | `false` | run with `--all-features`. Conflicts with `features` |
| `l1-include-slow` | bool | `false` | keep `slow_` tests inside the L1 selection (darkmatter's contract) |
| `runner-tools` | string[] | `[]` | closed vocabulary: `ai-provider-stubs`, `claudine-provider-fixture`, `darkmatter-md-fixture`, `messenger-desktop-stubs`, `node-22`, `pnpm-10`, `l2-parallel-self-spawn`, `neovim` |
| `companion-suites` | string[] | `[]` | non-Cargo suites this package owns; closed vocabulary: `homelab-frontend` |

`[package.metadata.ci.native]`: a map of runner OS (`ubuntu-latest`,
`windows-latest`, `macos-latest`) → system packages needed to build/test. The
union of a selected package's `native` requirements across its dependency
closure reaches every job that compiles or runs it — a dependent job that
compiles `playa` needs ALSA even though it is not testing `playa` (R5).

Validation (`affected_scope.py::validate_package_ci`) rejects unknown fields,
invalid tier or tool names, conflicting `features`/`all-features`, expired
exclusions, an L2 tier without backends, l2-backends without L2, and a
companion suite whose canonical recipe does not exist in its owning directory's
justfile.

### `runner-tools` is a closed vocabulary

Implemented by the reusable workflow (`_package-ci.yml`), not an arbitrary
command surface:

- **`ai-provider-stubs`** — inert AI-provider CLI stubs for tests that require
  provider discovery (claudine-cli).
- **`claudine-provider-fixture`** — builds Claudine's non-production native
  provider example once before L1 and exports its deterministic path through
  `CLAUDINE_PROVIDER_FIXTURE_EXE`. Windows launch-anchor tests copy that
  executable under provider names, so multiline argv never crosses a batch
  trampoline and the test process never invokes Cargo or rustc.
- **`darkmatter-md-fixture`** — builds darkmatter's `md` binary into the
  workspace target dir, preserving Claudine's clean-checkout fixture that a
  direct `_test claudine-cli` would otherwise lose.
- **`messenger-desktop-stubs`** — builds and verifies Messenger's six desktop
  helper fixtures once before each native L1 suite, then exports their directory
  through `MESSENGER_STUB_BIN_DIR`. The WSL2 archive job builds a Linux sidecar;
  the WSL job copies it onto ext4 with executable permissions and unprivileged
  ownership. The guest verifies that Cargo and rustc are absent before running
  the archive, so helper resolution cannot fall back to a nested build.
- **`node-22` / `pnpm-10`** — the JavaScript toolchain a companion suite runs
  under (homelab-frontend, owned by homelab-server).
- **`l2-parallel-self-spawn`** — run the L2 tier in `_test_l2`'s parallel
  self-spawn mode (`min(cores, 8)`), for suites dominated by self-isolating
  tests (claudine-cli).
- **`neovim`** — provisions Neovim for packages whose L2 contract exercises
  the editor backend.

Messenger and the three Rendezvous packages use the ordinary package grid as
their coverage authority. Their native and `wsl2-ubuntu` L1 evidence is keyed
by `{package, environment, tier}` and consumed from JUnit plus producer-status
artifacts by `ci-verdict`; no specialized workflow or job name stands in for a
package result.

### Companion suites

`companion-suites` names non-Cargo test suites this package owns, from a
closed vocabulary. `homelab-frontend` invokes the existing non-focusing
frontend recipe (`homelab/justfile::test-frontend`) and attributes its
producer status to `homelab-server`/L1. A companion suite must emit
machine-readable evidence or a producer failure: a green Rust JUnit report
must never hide a failed OR SKIPPED companion suite
(the producer-status `failure` downgrades the cell in the rollup, and a
companion outcome other than `success` — or none at all — downgrades it the
same way).

### Exclusions must be owned and time-bounded

`gates = false` requires `reason`, `owner`, and `exclusion-class`, plus
`expiry` unless the class is `capability`. A **past** `expiry` fails the scope
calculation loudly.

- `capability` — excluded because the environment genuinely cannot host it.
  Permanent, so it must **not** carry an `expiry`.
- `promotion-pending` — a real package with real tests, blocked on identified
  work.
- `time-bounded` — nothing to gate yet (zero or near-zero tests). No current
  package uses this class: a package with no tests gates and records
  `NOTHING TO RUN` instead (AC15), which makes its first future test run
  automatic.

A `gates = false` package still appears in the grid as `NOT SCHEDULED` with its
governance metadata — never a pass, never a silent absence, never conflated
with `NOTHING TO RUN` (R10).

## Native libraries

`native` has exactly one installer: the root `justfile`'s
`_ensure-native-libs`. CI runs `just _ensure-native-libs <packages...>` before
every build, test, and lint command so a `-sys` crate never fails to compile
for a missing system library, and `just init` runs the no-argument form (every
workspace package's declarations) to cover a developer host. The dependency
closure union is computed by the scope job and passed as an explicit list; the
WSL guest has no Cargo but receives that same list. Non-Debian Linux hosts need
the apt name mapped to `dnf` / `pacman` / `apk` in that recipe's table.

## The results baseline — `ci-baseline.toml`

`ci-baseline.toml` records known-red legs and the approved skip budget, keyed
by `{package, environment, tier}`. `scripts/ci-rollup.rs` enforces it:

- a failure **not** listed blocks
- a listed entry that is scheduled and **passes** blocks, forcing cleanup
- an entry outside the run's affected scope is **ignored** — never a pass
- a scheduled entry that is cancelled, missing, or emits no result stays
  blocking; it cannot be accepted as a known test failure
- an entry past its `expiry` blocks

Every entry needs `owner`, `reason`, and `source_run`. `expiry` is optional
but strongly encouraged. See the file's header for the re-key status and the
Phase 4 bridging run.

## `ci-verdict` — the single required check

`ci.yml`'s `ci-verdict` job is the **only** check branch protection should
require. Every producer — `check`, `lint`, `test`, `l2`, `browser`, `wsl` —
stays visibly red when it fails and must **not** be a required check: a
required producer's failure blocks the merge directly, the baseline is never
consulted, and the whole mechanism is bypassed.

It runs `if: always()`, so a failed or cancelled producer cannot skip it, and
it is passed `--scope` (the affected package names) from the `scope` job and
`--policy` (the scope job's resolved-package policy artifact). Scope is
load-bearing: it is the only way `ci-rollup` learns a package was *scheduled*
and produced *nothing*.

### Artifact contract

Two artifact families, both walked by `ci-rollup rollup --artifacts`:

```
junit-<package>-<tier>-<environment>/
    manifest.jsonl            one JSON record per nextest invocation
    <tier>/<package>.xml      that invocation's verbatim JUnit document

status-<package>-<job>[-<environment>]/
    status.json               {"package","job","environment","result"[,"detail"][,"companion"]}
```

Every test job uploads the whole `target/nextest/ci-reports` **staging
directory**, not `target/nextest/ci/test-results.xml` — that single path is
overwritten by each nextest invocation. **The manifest is the identity
source.** Artifact-name parsing was retired with the area model: a staged XML
with no covering manifest record has no trustworthy identity and is dropped.

`result` is GitHub's own `job.status`. The status step and its upload both
carry `if: ${{ always() }}` so a **failed** job still reports itself; a job
that reports nothing at all is `MISSING`, never a pass. A package that
declares a companion suite also records the companion step's `companion`
outcome on every run — not only on failure — because a *skipped* companion
leaves no other evidence, and the rollup downgrades a green cell whose
declared companion produced no success evidence (R12).

### `job` is read as a tier

`ci-rollup` parses `status.json`'s `job` field with the same vocabulary as
`tier`. That is why the test jobs publish `L1` / `L2` / `browser` rather than
their GitHub job names. Publishing `test` would manufacture a phantom
`<package>/<environment>/test` cell beside the real L1 one and count the same
failure twice.

## The cache key — per package

`Swatinem/rust-cache` is keyed per package and per job kind:
`package-ci-<package>-check-<os>`, `package-ci-<package>-lint-ubuntu-latest`,
and `package-ci-<package>-test-<environment>`. The L2, browser, and WSL
archive jobs deliberately REUSE the `test` key for their environment: they
compile the same crates as the L1 leg, so one warm cache serves every tier
instead of three cold ones.

The per-package unit made the old per-directory key wrong, and the choice is
the single biggest influence on whether this work reduces runtime at all:
compilation is ~85% of a test job. The implementation starts from a
package-scoped key; Phase 6 measures it against a real run (including the
cache-quota pressure of ~5 keys × 63 packages against GitHub's 10 GB repo
quota) and records the selected strategy.

## Compile-check

A package's compile-check stays `cargo check --all-targets -p <package>` (plus
its declared feature flags), because there is no per-package canonical check
recipe. `--all-targets` compiles benches and examples here and nowhere else.
It deliberately does **not** deny warnings; `lint` does, through clippy, where
`just lint` enforces the same bar locally.

## Adding or changing a package's CI

1. Add/adjust the `[package.metadata.ci]` block in the package's own manifest.
2. Tier/backend/native/feature changes require evidence (measured durations,
   real backend/native requirements, the actual tier-test ownership) — not
   guesses.
3. `python3 scripts/ci/test_affected_scope.py` and
   `cargo nextest run -p test-toolkit --test ci_workflow_contracts` must pass.

## CI's own tooling

The merge-gate binary (`scripts/ci-rollup*.rs`), the scope calculator
(`scripts/ci/`), and the policy store (`.github/ci/`) are not Cargo packages,
so a change to them selects nothing. `affected_scope.py` maps those paths to a
`ci_tooling` flag and `ci.yml` runs their own suites (the scope tests and the
rollup's nextest suite) on a dedicated `ci-tooling` leg, classified in the
advisory summary like the specialized workflows. The durable fix for the R11
contract suite (`tools/test-toolkit`, `gates = false` promotion-pending,
expiry 2026-10-31) is its promotion to a gating package.
