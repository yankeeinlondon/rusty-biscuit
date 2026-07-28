# CI area policy — `areas.json`

`areas.json` is the single policy surface for **every** package area in the repo.
Each area `sniff repo package-areas` discovers has exactly one record, whether or
not it gates in CI. Each record describes one area; `scripts/ci/affected_scope.py` validates every record
against the schema below (via `validate_area_schema`) and fails loudly on a
missing required field, an unknown field, an unsupported environment or runner
OS, an unknown L2 backend, an unowned or lapsed exclusion, an uncovered L2 policy
gap, or a mistyped value.

## `environment` is not `os`

Windows, macOS, and Linux are operating systems. **WSL2 is a distinct supported
Linux environment that a Windows runner hosts.** Policy and every result identity
— artifact name, JUnit manifest record, rollup cell — are keyed by
**environment**; only `runs-on` and the native-package lookup are keyed by runner
OS. Conflating the two merges a WSL leg into the native Windows cell without
anyone noticing.

| environment | hosted by runner label | notes |
|---|---|---|
| `ubuntu-latest` | `ubuntu-latest` | |
| `windows-latest` | `windows-latest` | |
| `macos-latest` | `macos-latest` | |
| `wsl2-ubuntu` | `windows-latest` | runs through `wsl-bash`; see `.github/workflows/_wsl-ci.yml` |

`affected_scope.py` derives three workflow inputs from the single `environments`
field so the reusable workflow can never route `wsl2-ubuntu` into a `runs-on`
matrix: `native_environments` (the environment names that *are* runner labels),
`l2_environments` (those with a provisionable L2 backend), and `wsl` (a boolean).

A WSL2 guest *is* Linux, so `_ensure-native-libs` keys off `uname -s` and reads
the area's `ubuntu-latest` package list. `native` therefore stays a **runner OS**
map and must not grow a `wsl2-ubuntu` key.

Cargo metadata — not this file — remains the source of truth for package
membership. `areas.json` only assigns policy to the curated areas, and its `area`
order must match the root `justfile` `areas :=` list.

## Fields

| Field | Type | Required | Default | Meaning |
|-------|------|----------|---------|---------|
| `area` | string | yes | — | Area directory; used for `cd`, cache keys, and ownership. |
| `ci` | bool | no | `true` | Whether the area gates in the fan-out matrix. |
| `check_args` | string | when `ci` | — | Args for the macOS compile-check `cargo check --all-targets` (e.g. `-p foo -p foo-cli`). |
| `reason` | string | when not `ci` | — | Why the area does not gate, and what would unblock it. |
| `owner` | string | when not `ci`, and on every policy gap | — | GitHub handle accountable for closing the exclusion or gap. |
| `expiry` | string | when not `ci` unless `capability` | — | ISO `YYYY-MM-DD`. A **past** date fails the scope calculation. |
| `exclusion_class` | string | when not `ci` | — | One of `capability` (permanent), `promotion-pending`, `time-bounded`. |
| `environments` | string[] | no | `["ubuntu-latest","windows-latest","macos-latest"]` | Environments that run the full canonical L1 suite. |
| `check_os` | string[] | no | `["windows-latest"]` | Runner OSes that only compile-check, under `RUSTFLAGS: -D warnings`. |
| `policy_gaps` | object[] | no | `[]` | Tiers this area owns tests for that a declared environment cannot host. |
| `shards` | string[] | no | `["1/1"]` | nextest `--partition count:i/N` specs; `["1/1"]` = no sharding. Sized from measured cold-run duration. |
| `l2` | bool | no | `false` | Run the real-terminal (L2) tier on every environment with a provisioned backend. |
| `browser` | bool | no | `false` | Run the headless-browser tier on Linux. |
| `kache` | bool | no | `true` | Enable the kache `RUSTC_WRAPPER` (Linux/macOS only — `kache-action@v1` rejects `win32-x64`). |
| `ai_provider_stubs` | bool | no | `false` | Install inert AI-provider CLI stubs for tests needing provider discovery. |
| `backends` | string[] | no | `[]` | L2 terminal backends this area's tests require. One of: `tmux`, `wezterm`, `kitty`, `apple-terminal`. |
| `native` | object | no | `{}` | Map of runner OS → system packages needed to build/test (e.g. `{"ubuntu-latest": ["libasound2-dev"]}`). |
| `canary` | bool | no | `false` | Whether this area is a global-change canary (Phase 4). |

Supported environments: `ubuntu-latest`, `windows-latest`, `macos-latest`,
`wsl2-ubuntu`. Supported runner OS values (for `check_os` and `native`):
`ubuntu-latest`, `windows-latest`, `macos-latest`.

### Retired fields

Both are rejected **by name**, with an actionable message, rather than appearing
in a generic unknown-field list.

- `soft_os` — it marked a test leg `continue-on-error`, which did not merely stop
  the leg from blocking the merge, it removed the leg from the run's verdict, so
  a permanently red platform read as a normal run. Every configured environment
  now gates; a known failure is recorded in the results baseline instead, which
  keeps the signal visible.
- `full_os` — it named a *runner OS* list, but WSL2 is an environment a Windows
  runner hosts rather than a runner label of its own. Replaced by `environments`.

### Why macOS runs the real suite now

macOS used to be compile-check-only, justified by "runner minutes bill ~10x".
The repo is **public**, so standard runners are free and that justification is
void. macOS is in the default `environments` list.

"macOS is healthy" remains an assumption to be measured, not asserted: `sniff`
was the only area testing there and it fails on macOS today.

### What the `check` job is for now

It is no longer "the macOS floor". A compile-check is strictly weaker than a test
run on the same environment, so it would be a wasted slot beside the new macOS
test leg. What survives is the one thing a test leg cannot do: carry
`RUSTFLAGS: -D warnings`. Denying warnings on a test job deletes the run's test
evidence over a plain rustc hint, so warning enforcement needs its own
compile-only slot.

`lint` already denies warnings on Linux, and macOS is the primary development
host where drift is caught locally. So the default `check_os` is
`["windows-latest"]` — nobody's dev box, and where warning drift actually hides.
Same job count per area as before, aimed better. The trade-off is explicit:
**macOS-only warning drift is no longer detected in CI.**

### Policy gaps

A policy gap is a tier an area owns tests for that some declared environment
cannot host. It exists so the rollup renders **POLICY GAP** for that cell instead
of a green `0 run / N skipped`.

```json
"policy_gaps": [
  {
    "tier": "L2",
    "environments": ["windows-latest"],
    "reason": "tmux has no Windows port; WezTerm/Kitty need a live GUI session",
    "owner": "@yankeeinlondon",
    "expiry": "2027-01-31"
  }
]
```

`tier` is one of `L1`, `L2`, `browser`. Every listed environment must be one the
area actually declares — a gap describes a cell that *is* scheduled but cannot
execute. Validation is not merely structural: an area with `"l2": true` that runs
on an environment outside `L2_PROVISIONED_ENVIRONMENTS` and has **no** matching
gap record fails the scope calculation. That is what keeps Windows L2 from
quietly becoming a green cell.

`L2_PROVISIONED_ENVIRONMENTS` is `{ubuntu-latest, macos-latest}`: tmux is the only
backend headless CI can host and it installs on both (apt / brew). Windows has no
tmux port and no proven alternative; `wsl2-ubuntu` runs from a `nextest archive`,
which carries no broker binary and hosts no tmux server.

#### A declared gap does not block the merge; everything else about it does

Acknowledged is not acceptable, and neither is invisible. Eight areas declare an
owned Windows-L2 gap, so if a `POLICY GAP` cell blocked unconditionally the one
required check could never go green and nothing could merge. `ci-rollup verdict`
therefore treats an owned, unexpired gap the way it treats a baselined failure:

- the cell still renders **POLICY GAP** in the grid, never `PASS`, `SKIP`, or
  `N/A`, and is still listed under "Cells failing the summary gate"
- the verdict reports it as a `note` (`policy-gap-accepted`) naming the owner and
  expiry, so it stays legible in the summary
- it does **not** block

Four things forfeit that acceptance and block at `BLOCK` severity:

| Rule | Case |
|---|---|
| `cell-policy-gap` | **undeclared** — no `policy_gaps` record; the gap was inferred from backend provisioning. This is the case that catches a tier being quietly switched off |
| `policy-gap-incomplete` | no `owner`, no `reason`, or a missing/malformed `expiry` |
| `policy-gap-expired` | `expiry` is in the past |
| `cell-failed` | the cell produced **real failures**. `FAIL` outranks `POLICY GAP`, so a gap declaration can never suppress genuine evidence |

There is deliberately **no** "the tests passed, so the gap is stale" rule, close
as the analogy to `baseline-now-passing` is. A `require_level!` gate that skips
for want of a backend early-returns, and nextest records that as a JUnit
**pass** — so on JUnit evidence alone a passing count is exactly what a
correctly-declared gap looks like, and such a rule would block the case it was
meant to protect. Detecting a gap that has genuinely closed needs plan §1.1's
per-backend execution proof. Until that lands, `expiry` is the only forcing
function, which is why an undated gap is rejected outright.

Expiry is checked in **two** places on purpose. `affected_scope.py`
(`validate_expiry`) fails the scope job at config time with the most actionable
message and is the better place to learn about a lapsed gap. But `ci-verdict`
runs `if: always()` precisely so a failed or skipped scope job cannot suppress
it — so if expiry lived only in the Python, an expired gap would be waved
through by the only check that actually gates merging whenever the check that
catches it did not run.

This is deliberately *not* `soft_os`. `continue-on-error` removed a leg from the
run's verdict entirely (plan §1.4); an accepted policy gap stays in the grid, in
the summary, and attributable to a person and a date.

### Exclusions must be owned and time-bounded

An exclusion without an end date is a permanent one wearing a temporary label, so
`"ci": false` requires `reason`, `owner`, and `exclusion_class`, plus `expiry`
unless the class is `capability`. A **past** `expiry` fails the scope calculation
loudly — close the item or move the date out with fresh justification.

- `capability` — excluded because the environment genuinely cannot host it
  (physical IoT hardware, say). Permanent, so it must **not** carry an `expiry`.
- `promotion-pending` — a real area with real tests, blocked on work that is
  identified. The `reason` must name the blocker precisely.
- `time-bounded` — nothing to gate yet (zero or near-zero tests).

## The results baseline — `ci-baseline.toml`

`ci-baseline.toml` records known-red legs and the approved skip budget. It
replaces `baseline-failures.txt`, which listed GitHub *display names* and had no
consumers at all. `scripts/ci-rollup.rs` enforces it:

- a failure **not** listed blocks
- a listed entry that is scheduled and **passes** blocks, forcing cleanup
- an entry outside the run's affected scope is **ignored** — never a pass
- a scheduled entry that is cancelled, missing, or emits no result stays
  blocking; it cannot be accepted as a known test failure
- an entry past its `expiry` blocks

Entries are keyed by `{area, environment, tier, shard}`, never by a job name.
For a matrix leg skipped by `needs:`, GitHub reports the raw un-interpolated
name expression, so `os` and `shard` are not recoverable from it at all.

Every entry currently in the file was migrated mechanically from
`baseline-failures.txt` and is marked **unverified**. See the header of
`ci-baseline.toml` and `fixes/2026-07-27-refactor/ci-verdict-job.md`.

## `ci-verdict` — the single required check

`ci.yml`'s `ci-verdict` job is the **only** check branch protection should
require. Every producer — `check`, `lint`, `test`, `l2`, `browser`, `wsl2` —
stays visibly red when it fails and must **not** be a required check: a required
producer's failure blocks the merge directly, the baseline is never consulted,
and the whole mechanism is bypassed.

It runs `if: always()`, so a failed or cancelled producer cannot skip it, and it
is passed `--scope` from the `scope` job. Scope is load-bearing: it is the only
way `ci-rollup` learns an area was *scheduled* and produced *nothing*. Inferred
scope reads the artifacts on disk, which by construction cannot see an area that
produced no artifact at all — exactly the case `MISSING` exists to catch.

### Artifact contract

Two artifact families, both walked by `ci-rollup rollup --artifacts`:

```
junit-<area>-<tier>-<environment>-<index>/
    manifest.jsonl            one JSON record per nextest invocation
    <tier>/<package>.xml      that invocation's verbatim JUnit document

status-<area>-<job>/
    status.json               {"area","job","environment","result"}
```

Every test job uploads the whole `target/nextest/ci-reports` **staging
directory**, not `target/nextest/ci/test-results.xml` — that single path is
overwritten by each nextest invocation, so a multi-package area published only
its last package's report. **The manifest is the identity source.** The artifact
directory name is parsed only when a staged XML has no covering manifest record,
and such a record is flagged `degraded` with an unknown shard.

`result` is GitHub's own `job.status`. The status step and its upload both carry
`if: ${{ always() }}` so a **failed** job still reports itself; a job that
reports nothing at all is `MISSING`, never a pass.

### `job` is read as a tier

`ci-rollup` parses `status.json`'s `job` field with the same vocabulary as
`tier`. That is why the test jobs publish `L1` / `L2` / `browser` rather than
their GitHub job names:

- a status naming a **test tier** explains a `MISSING` cell downstream of it (an
  L2 leg deleted by a failed L1, say). It is matched on `area` alone, on purpose:
  `lint` runs only on Linux, but `needs: lint` used to delete the test matrix for
  *every* environment.
- a status naming **anything else** (today `lint` and `check`) becomes a cell in
  its own right, so a job that emits no JUnit can still be baselined and can
  still block. This is what makes `tier = "lint"` a valid baseline key.

Publishing `test` instead of `L1` would manufacture a phantom
`<area>/<environment>/test` cell beside the real L1 one and count the same
failure twice.

### There is no per-area failure classifier

`_area-ci.yml` used to carry a `classify` job that wrote one "first actionable
failure class" line per area, because a reusable workflow's matrix legs cannot
report an output back to the caller. The status artifacts carry the same fact per
`{area, environment, tier}` — a resolution `classify` never had, since one
area-level line could not tell a Windows-only failure from a Linux-only one.

It was removed rather than kept alongside, because the two disagree: a baselined
known-red gate still fails its area workflow, so `classify` would print a failure
line under a verdict that correctly reads CLEAR. `ci.yml`'s remaining advisory
`summary` job now covers **only** the bootstrap stages and the specialized
workflows, which are not areas and are invisible to the rollup until plan §3.5
makes them emit the same result schema.

### Not wired yet

- **The expected-test manifest** (`--expected-manifest`). Without it, a test
  present on the target environment that produced no result cannot be told apart
  from one compiled out by `#[cfg]`, so `ci-rollup` records
  `skip_evidence_degraded` rather than guessing. Wiring it needs a `just` recipe
  that runs `cargo nextest list --message-format json` on each environment.
- **The specialized workflows.** `messenger-desktop`, `rendezvous`, the Claudine
  generator drift check, and coverage emit no `manifest.jsonl` and are not areas,
  so their baseline entries are reported `baseline-out-of-scope` and ignored —
  they neither block nor pass.
- **`check` cells have no baseline entries.** `baseline-failures.txt` recorded
  none, so the first full-scope run under this mechanism will surface any red
  compile-check as a new blocker. That is the mechanism working; add owned,
  dated entries or fix the warning.

`native` has exactly one installer: the root `justfile`'s `_ensure-native-libs`.
CI runs `just _ensure-native-libs <area>` before every build, test, and lint
command so a `-sys` crate never fails to compile for a missing system library,
and `just init` runs it with no argument to cover every area on a developer host.
A new requirement is therefore declared once and installed by one implementation.
Non-Debian Linux hosts need the apt name mapped to `dnf` / `pacman` / `apk` in
that recipe's table.

## Choosing canaries

A global change runs the `canary` areas before the rest fan out, and a canary
failure blocks that fan-out (D11). That only produces signal if the canary area
is **otherwise green**: the canary must fail because the shared change broke it,
not because the area already had failing tests. A red canary is worse than no
canary, because it hides every other area's result behind a known failure.

Current set: `biscuit-hash` (pure Rust, fast) and `playa` (native dependencies).

- `darkmatter` is the intended heavy/sharded canary and should be re-added once
  its L1 suite is green.
- Do **not** use `homelab` or `research` as canaries.

Keep the set small — it is a serial stage in front of everything else.

## Ownership completeness

Every Cargo workspace member (per `cargo metadata`) must live under an area that
has a record here. `validate_ownership` fails the scope calculation — naming the
package — when one does not, so a package in a brand-new directory cannot land
without someone deciding what its area is and whether it should gate.

Areas that do not gate set `"ci": false` and give a `reason`, an `owner`, an
`exclusion_class`, and (unless `capability`) an `expiry`. They still declare
`native`, still confer ownership, and still appear to `just _ensure-native-libs`;
they simply launch no area job.

**None of the ten current exclusions is a capability exclusion.** An audit of all
31 areas found no record that is permanently excluded on capability grounds, so
every one is backlog and every one is time-bounded:

- **`promotion-pending`** — real packages with real tests that run in **no** CI
  job today: `tools` (63 tests), `biscuit-test-harness` (92),
  `biscuit-browser-harness` (6 + 7 browser), `messenger` (564),
  `biscuit-visualized` (71), `biscuit-clipboard` (201, across three packages —
  `biscuit-clipboard-service` was covered by no record at all). The blocker is
  the canonical `just` recipe set that `check-canonical` requires. Complete the
  recipes, flip `ci` to `true`, and add the area to the root justfile's
  `areas :=` list.

  The old "exercised transitively by consumer areas" justification for the three
  harness areas does not hold: a consumer's suite exercises the harness code paths
  it happens to call, which is not the same as running the harness's own tests.
- **`time-bounded`** — nothing to gate: `agent-sandbox` (0 tests), `reaper` (0,
  despite carrying 10 of the 12 canonical recipes), `visualizer` (0), `tabby` (1).

`homelab` remains the reference case for what a *legitimate* capability exclusion
would look like — it targets physical IoT hardware — but it currently gates, so
no record uses that class.

Only gating areas appear in the root justfile's `areas :=` list;
`validate_area_config` keeps the two in step.

## Adding or changing an area

1. Add/adjust the record here **and** keep the `area` order aligned with the root
   `justfile` `areas :=` list.
2. Shard-count, backend, native, and OS-policy changes require evidence
   (measured durations, real backend/native requirements) — not guesses.
3. `python3 scripts/ci/test_affected_scope.py` and
   `cargo nextest run -p test-toolkit --test ci_workflow_contracts` must pass.
