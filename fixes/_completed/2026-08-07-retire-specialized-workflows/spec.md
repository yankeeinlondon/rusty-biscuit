---
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-12
depends-on:
  - fixes/2026-08-06-cicd/spec.md
---

# Retire the Messenger and Rendezvous Specialized Workflows

Status: draft — inline review incorporated 2026-08-12

Builds on `fixes/2026-08-06-cicd/spec.md`, which made the package the unit of
CI selection, execution, and result identity, and explicitly left the
specialized workflows out of scope. This spec retires two of them.

> **Reader's note:** The original draft treated messenger's declared
> `features = ["desktop"]` policy as equivalent to the specialized workflow
> and assumed its helper binaries would be available to every normal L1 run.
> Neither assumption holds. The specialized workflow also compile-checks
> `--all-features`, and messenger's unit tests fall back to invoking Cargo when
> a helper binary is missing — impossible in the archive-only WSL2 guest. This
> revision makes `all-features` the package contract and requires the helper
> binaries to be delivered as explicit runner tooling on both native runners
> and WSL2. It also records the one intentional coverage reduction: the normal
> grid does not compile non-test examples on macOS and Linux.

## What this changes

`messenger-desktop-tests.yml` and `rendezvous-tests.yml` are deleted. The
packages they cover are tested by the normal per-package flow — the standard
native matrix plus WSL2, canonical recipes, package-keyed result artifacts,
and the shared verdict. No test they run today is lost.

The retirement does deliberately remove redundant compile-only work that the
package grid does not promise: `cargo check --all-targets` for every Rendezvous
consumer on macOS and Linux. The grid retains the all-targets check on Windows
and compiles and runs the packages' test targets on every native environment
and WSL2. Today the only extra target affected is
`rendezvous-daemon`'s `register_compaction_spike` example, a documented
development spike rather than a shipped binary or test. Preserving a
three-OS check for examples would require widening the repository-wide check
policy or introducing another package exception; neither is justified by this
retirement.

The other specialized legs (`playa-windows`,
`biscuit-tui-captured-stdout`, the Darkmatter `NO_COLOR` job, the Claudine
generator-drift job, and coverage) are separate decisions; see
§ Out of scope.

## Problem

Both workflows predate per-package resolution and exist for reasons that
per-package resolution has dissolved.

**messenger-desktop** exists because `messenger` and `messenger-cli` are
`gates = false`; this bespoke workflow is their entire CI ownership. It runs,
on three native operating systems:

- `cargo check -p messenger --all-features`;
- `cargo build -p messenger --features desktop --bins`;
- `cargo test -p messenger --features desktop --lib`;
- `cargo test -p messenger-cli`.

The exclusion reason says promotion is blocked on the canonical recipe set
because `messenger/` defines only `test` and `lint`. That reason is stale:
`messenger/justfile` now defines the full canonical set (`sanity`, `test`,
`test-l2`, `test-l3`, `test-browser`, `test-real`, `lint`, `bench`, `coverage`,
`doctest`, `fuzz`, and `all`). The exclusion and specialized workflow have
outlived the blocker.

The promotion cannot merely reuse the currently recorded
`features = ["desktop"]`, however. The package-policy contract intentionally
forwards one feature selection to compile-check, native L1, and the WSL2
archive. Selecting `desktop` would silently drop the existing all-features
compile coverage; selecting `all-features` preserves it and makes native and
archive test builds a strict superset of the old desktop-only library run.

Messenger's helper stubs are also an owned prerequisite, not incidental test
setup. The native specialized workflow builds them before running tests. The
tests can build a missing stub on demand for developer convenience, but that
fallback is unsuitable for parallel nextest processes and cannot run in the
toolchain-free WSL2 guest. The normal flow therefore needs a closed
`messenger-desktop-stubs` runner tool that builds the binaries once and makes
them available to the test process. On WSL2, the Linux archive builder ships
the stubs as a sidecar, the guest places them in an explicit directory, and the
tests resolve that directory through `MESSENGER_STUB_BIN_DIR` before
considering their local Cargo fallback. CI must never rely on compiling a
fixture from inside a test process.

**rendezvous-tests** exists for two stated reasons, both now false or
subsumed, verified against the first full per-package run (31184682085):

| Specialized step | Where the normal flow covers it |
|---|---|
| Compile-check `--all-targets` of sniff, rendezvous-core/client/daemon, and claudine-cli on three OSes | The Windows check cell retains `--all-targets`; native and WSL2 L1 cells compile and run each package's test targets. The removed macOS/Linux example-only check is the explicit decision in § What this changes. |
| "Rendezvous suite" = `cd claudine/rendezvous && just test` | The grid runs `rendezvous-core`, `rendezvous-client`, and `rendezvous-daemon` L1 separately on Ubuntu, Windows, macOS, and WSL2, using the same underlying canonical `_test` recipe and emitting JUnit evidence for each package. |
| "Local control-plane call sites" = claudine-cli filtered to `dashboard/session_report/requeue/commands::handle` | These are ordinary L1 tests. The grid runs the complete `claudine-cli` L1 suite on all four environments, a strict runtime superset of the name filter. |
| Founding rationale: "the shared block compile-checks macOS and tests only Linux/Windows" | Stale — the per-package grid runs full L1 on macOS. |
| protoc backstop for rendezvous-core's build.rs | The standard package workflow installs protoc, while the build script also vendors it; run 31184682085's Rendezvous cells built successfully. |
| Cross-boundary trigger: runs when sniff changes | Reverse-dependency expansion selects `rendezvous-core`, `rendezvous-daemon`, and `claudine-cli` through their actual Cargo edges; downstream expansion then reaches the remaining Rendezvous packages. Scope tests, rather than a hand-written area flag, prove the exact closure. |

The specialized workflows upload no package result or producer-status
artifacts. Although `ci-verdict` waits for them, it evaluates package evidence
only; a specialized failure is therefore advisory in the final summary and
cannot block the required verdict. Folding these packages into the grid closes
that visibility gap.

One genuinely unique behavior must not be lost silently: `rendezvous-tests`
redacts Windows user SIDs from its uploaded text logs because a failing
endpoint assertion names a SID-qualified pipe. In the normal flow, the same
failure can land in JUnit without redaction. R5 records the decision not to
carry that transform forward.

## Requirements

**R1 — messenger becomes normal gating policy.** Remove `gates = false` and
all exclusion-governance fields from both messenger manifests. The library
keeps its `ubuntu-latest = ["libdbus-1-dev"]` native requirement and replaces
`features = ["desktop"]` with `all-features = true`. Its tests declare the
`messenger-desktop-stubs` runner tool. `messenger-cli` needs no non-default CI
policy after its exclusion is removed: its dependency already selects the
provider and desktop features used by its existing specialized test command.

**R2 — every specialized test has normal-flow evidence.** The retirement PR
must include a command-to-cell mapping based on a clean checkout, not an
assertion based only on recipe names:

| Retired command | Replacement evidence |
|---|---|
| `cargo check -p messenger --all-features` | The messenger Windows check cell uses `--all-targets --all-features`; all-feature native and WSL2 L1 builds cover messenger's lib, bin, and test targets on the remaining environments. Messenger has no example or bench target. |
| `cargo build -p messenger --features desktop --bins` | `messenger-desktop-stubs` builds the all-feature helper bins once before native L1 and delivers Linux-built sidecars to WSL2. |
| `cargo test -p messenger --features desktop --lib` | The messenger L1 cell runs the full all-feature suite, including the desktop unit tests, on every environment. |
| `cargo test -p messenger-cli` | The messenger-cli L1 cell runs the full package suite on every environment. |
| `cd claudine/rendezvous && just test` | The three package-keyed Rendezvous L1 cells collectively run the same three suites on every environment. |
| Filtered `cargo nextest run -p claudine-cli` | The claudine-cli L1 cell runs the full L1 suite, including every selected call-site test. |

The fixture-delivery proof must include a WSL2 archive execution with Cargo and
rustc unavailable in the guest. It must demonstrate that the helper tests use
the shipped stubs and do not enter their build-on-demand fallback. Native
proof must also show the prebuild occurs once before nextest, avoiding
concurrent Cargo builds from individual test processes.

**R3 — the specialized graph is deleted completely.** Delete both workflow
files and remove:

- their jobs from `ci.yml`;
- their `needs` entries from `ci-verdict` and the advisory summary;
- their lines from the advisory failure classifier;
- their rows from the contract tests' `ORCHESTRATED` inventory;
- the messenger-specific scope output and prefix-matching logic, if no
  remaining consumer exists; and
- stale comments, cache-key documentation, and active CI documentation that
  describe either workflow as current ownership.

Historical completed specs, plans, reviews, handoffs, and measurements remain
historical records and are not rewritten merely to remove old filenames.

**R4 — evidence reaches the verdict.** Messenger and Rendezvous results flow
through the standard `{package, environment, tier}` JUnit and producer-status
artifacts into `ci-verdict`. A failure blocks a merge exactly like any other
package's unless a matching, valid known-failure entry applies. Waiting on a
job through `needs` is not evidence and does not satisfy this requirement.

**R5 — SID redaction ends with the workflow.** DECIDED (Ken, 2026-08-07): SIDs
of ephemeral GitHub-runner accounts are not sensitive. Do not add a redaction
transform to JUnit staging or the result pipeline. Delete the Rendezvous log
redaction and upload steps with the workflow. This decision does not prohibit
unrelated secret redaction or require removing SID terminology from endpoint
implementation and tests.

**R6 — first-run failures join the baseline honestly.** The promotion may
surface failures, including the `rendezvous-client` and `rendezvous-daemon`
Windows L1 failures already observed in run 31184682085. A failure may enter
`ci-baseline.toml` only when the promotion run proves the same package,
environment, tier, and failure are pre-existing or independently owned. Each
entry carries its real `source_run`, owner, reason, and expiry. A new failure
caused by the retirement machinery is fixed rather than baselined, and no
package is re-excluded (AC15 of the parent spec). Passing cells receive no
speculative baseline entries because stale entries themselves block the
verdict.

**R7 — every newly introduced environment is watched.** Messenger has not run
through the canonical package workflow, and neither messenger nor the
Rendezvous packages previously ran their specialized suites in WSL2. Inspect
every native and WSL2 cell. D-Bus availability, archive fixture delivery, and
Windows named-pipe behavior are the known risks. Resolve each failure under R6;
never silently re-exclude a package or disable an environment.

**R8 — active documentation and contracts follow the new authority.** Update
`docs/topics/ci-cd.md`, `docs/testing-strategy.md`,
`claudine/docs/rendezvous/local-ipc.md`, the active Rendezvous Windows
follow-up, manifest comments, and workflow-contract comments so they describe
package-grid ownership. Contract tests must prove:

- messenger is gating with `all-features` and the stub runner tool;
- no deleted workflow or orchestration job remains;
- a messenger-cli-only change still selects messenger-cli without needing a
  messenger prefix flag;
- a sniff change selects the exact Cargo-derived Rendezvous/Claudine closure;
- the WSL2 messenger archive contains usable stubs with no guest toolchain; and
- the specialized-workflow inventory retains only the workflows still
  specialized.

## Acceptance criteria

1. `messenger-desktop-tests.yml` and `rendezvous-tests.yml` do not exist, and
   no active workflow, recipe, script, contract test, or current CI document
   references them as live infrastructure.
2. `messenger` and `messenger-cli` appear in the package matrix when impacted.
   Messenger's resolved policy is gating, all-feature, and carries its D-Bus
   prerequisite and helper-stub runner tool.
3. A change to `sniff` exercises the Rendezvous consumer boundary through the
   Cargo reverse-dependency closure, verified by a non-vacuous scope test.
4. Every specialized test command is mapped to the replacement package cell,
   and the intentional macOS/Linux example-only compile reduction is recorded
   rather than mislabeled as equivalent coverage.
5. Messenger helper tests pass on native runners and in the WSL2 archive with
   no test-process Cargo invocation and no Cargo or rustc in the guest.
6. A Rendezvous or messenger failure produces a package-keyed FAIL cell that
   blocks the verdict unless a valid matching baseline entry applies.
7. No Rendezvous-specific SID redaction or raw-log upload machinery remains;
   no replacement transform is added to JUnit or result staging.
8. A PR touching neither package family schedules no messenger- or
   Rendezvous-specific job. A relevant PR schedules only the normal
   package-keyed cells derived from scope.
9. Baseline additions, if any, match observed failing cells from the promotion
   run and contain `source_run`, owner, reason, and expiry.
10. Active CI and Rendezvous documentation names the package grid, not the
    deleted workflows, as the coverage authority.

## Out of scope

- The remaining specialized legs (`playa-windows`,
  `biscuit-tui-captured-stdout`, Darkmatter `NO_COLOR`, Claudine generator
  drift, and coverage). The same retirement pattern may apply where a
  package-policy equivalent exists, but each leg has its own rationale to
  audit and this spec does not prejudge it.
- Fixing the Rendezvous Windows named-pipe failures themselves. R6 governs
  honest baseline treatment; normal known-red burn-down is separate work.
- Changing messenger or Rendezvous production behavior. The helper-directory
  override and archive sidecar are test infrastructure only.
- Changing local developer recipe behavior. `claudine/rendezvous/justfile` and
  `messenger/justfile` retain their public recipes and results; messenger's
  on-demand stub build remains a local fallback.
- Restoring three-OS compile-only coverage for development examples. That
  should be justified as a repository-wide check-policy change, not recreated
  as another specialized workflow.

## Sequencing

Begin only after the per-package cutover in the dependency spec is complete.
Land the policy, runner-tool/archive support, workflow deletion, scope cleanup,
contract updates, active documentation, and any run-proven baseline entries as
one atomic retirement. Iterate the PR against its own package-keyed results
until every scheduled cell either passes or has a valid R6 entry; do not merge
an intermediate state in which the specialized jobs are gone but messenger's
native or WSL2 fixtures are unavailable.
