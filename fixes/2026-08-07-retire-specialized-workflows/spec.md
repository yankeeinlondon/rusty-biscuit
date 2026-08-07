# Retire the Messenger and Rendezvous Specialized Workflows

Status: draft — awaiting review

Builds on `fixes/2026-08-06-cicd/spec.md`, which made the package the unit of
CI selection, execution, and result identity, and explicitly left the
specialized workflows out of scope. This spec retires two of them.

## What this changes

`messenger-desktop-tests.yml` and `rendezvous-tests.yml` are deleted. The
packages they cover are tested by the normal per-package flow — the same
matrix, canonical recipes, result artifacts, and verdict as every other
package. No bespoke legs remain for them, and no test they run today is lost.

The other specialized legs (`playa-windows`, `biscuit-tui-captured-stdout`,
the darkmatter NO_COLOR job, the claudine drift job, coverage) follow the same
pattern but are separate decisions — see § Out of scope.

## Problem

Both workflows predate per-package resolution and exist for reasons that
per-package resolution has dissolved:

**messenger-desktop** exists because messenger is `gates = false` — this
bespoke workflow (`cargo test -p messenger --features desktop --lib` plus
`cargo test -p messenger-cli`, three OSes) is messenger's *entire* CI
ownership. The exclusion reason says promotion is "blocked on the canonical
just recipe set — messenger/ defines only test and lint." **That reason is
stale**: `messenger/justfile` now defines the full canonical set (`sanity`,
`test`, `test-l2`, `test-l3`, `test-browser`, `test-real`, `lint`, `bench`,
`coverage`, `doctest`, `fuzz`, `all`). The blocker evaporated; the exclusion
and the bespoke workflow both outlived it.

**rendezvous-tests** exists for two stated reasons, both now false or
subsumed, verified against the first full per-package run (31184682085):

| Bespoke step | Where the normal flow covers it |
|---|---|
| Compile-check `--all-targets` of sniff, rendezvous-core/client/daemon, claudine-cli on 3 OSes | The grid compile-checks each package on Windows and builds all test targets on every native environment through the L1 legs |
| "Rendezvous suite" = `cd claudine/rendezvous && just test` | The grid runs `rendezvous-core`, `rendezvous-client`, and `rendezvous-daemon` L1 on ubuntu, windows, and macos — same suites, per package, with JUnit evidence |
| "Local control-plane call sites" = claudine-cli filtered to `dashboard/session_report/requeue/commands::handle` | A strict subset of claudine-cli's L1 suite, which the grid runs in full on all three native environments |
| Its founding rationale: "the shared block compile-checks macOS and tests only Linux/Windows" | Stale — the per-package grid runs full L1 on macOS |
| protoc backstop for rendezvous-core's build.rs | The build.rs vendors protoc; run 31184682085's rendezvous legs built green with no installed protoc |
| Cross-boundary trigger: runs when *sniff* changes | Package resolution subsumes it: sniff is in claudine-cli's dependency closure, so a sniff change selects claudine-cli (and the rendezvous packages through their own edges) automatically |

What the bespoke workflows have that the normal flow lacks — and this cuts
the other way — is *nothing the verdict can see*: they upload no result
artifacts, so their failures never block a merge. Folding the packages into
the grid closes that visibility gap as a side effect.

One genuinely unique behavior must not be lost silently: rendezvous-tests
**redacts Windows user SIDs** from its uploaded logs, because a failing
endpoint assertion names a SID-qualified pipe. In the normal flow, the same
failure message lands in a JUnit artifact unredacted. See R5.

## Requirements

**R1 — messenger gates.** `messenger` and `messenger-cli` flip to
`gates = true`; their exclusion records are deleted. Their existing policy
survives: `features = ["desktop"]` and the D-Bus native requirement are
already declared in the manifests (recorded during the per-package cutover so
that "a promotion changes exactly one field" — this is that promotion).

**R2 — the extended tests run in the normal run.** Every test the bespoke
workflows execute today is selected by a canonical tier of a gating package
in the grid. This is already true by construction (the table above); the
requirement is that the retirement PR *demonstrates* it — for each bespoke
invocation, name the grid cell that covers it — rather than asserting it.
The grid's coverage is a superset in every case (full suites vs. `--lib`,
full L1 vs. a name filter).

**R3 — the bespoke legs are deleted.** Both workflow files, their
orchestration entries in `ci.yml` (the `messenger` scope flag stays only if
something still consumes it; otherwise it goes too), their `needs:` entries
in the rollup and final-summary jobs, and their rows in the contract tests'
`ORCHESTRATED` table. No workflow, recipe, or contract test references either
file afterward.

**R4 — evidence reaches the verdict.** The packages' results flow through the
standard `{package, environment, tier}` artifacts into `ci-verdict`. A
messenger or rendezvous failure blocks a merge exactly like any other
package's — closing the "specialized workflows are invisible to the merge
gate" gap for these two.

**R5 — SID redaction is decided, not dropped.** Either (a) the JUnit staging
path redacts `S-1-5-21-…` user SIDs in failure text the way the bespoke
workflow redacted its logs, or (b) it is ratified that SIDs of ephemeral
GitHub-runner accounts are not sensitive and redaction ends with the
workflow. Decision owner: Ken. The retirement PR must implement whichever is
chosen; silence is not an option because the redaction exists today.

**R6 — first-run reds join the baseline honestly.** These packages' first
gating runs may surface red cells (run 31184682085 already shows
`rendezvous-client` and `rendezvous-daemon` L1 failing on windows-latest —
the named-pipe half of the transport contract). Real failures enter
`ci-baseline.toml` as owned, dated entries like every other known-red; they
must not be "fixed" by re-excluding the packages (AC15 of the parent spec).

**R7 — messenger's first grid run is watched.** messenger has never run
under the canonical recipes in CI. Its promotion run may surface
environment-dependent failures (D-Bus on headless runners is the known
hazard; its native declaration covers the library). Treat them like R6:
baseline with owner and expiry, or fix, but never silently re-exclude.

## Acceptance criteria

1. `messenger-desktop-tests.yml` and `rendezvous-tests.yml` do not exist, and
   no workflow, recipe, script, or contract test references them.
2. `messenger` and `messenger-cli` appear in the package matrix when
   impacted, with their declared feature and native policy applied.
3. A change to `sniff` still exercises the rendezvous consumer boundary —
   via the dependency closure selecting the affected packages, verified by
   the scope calculation, not by a bespoke trigger.
4. Every test invocation the bespoke workflows ran is mapped, in the
   retirement PR description, to the grid cell that now covers it.
5. A rendezvous or messenger test failure produces a package-keyed FAIL cell
   that blocks the verdict.
6. The SID decision (R5) is recorded and implemented.
7. `ci.yml`'s job graph for a PR touching neither package shows no
   messenger- or rendezvous-specific jobs.

## Out of scope

- The remaining specialized legs (`playa-windows`,
  `biscuit-tui-captured-stdout`, darkmatter NO_COLOR, claudine drift,
  coverage). Same retirement pattern where a package-policy equivalent
  exists (e.g. a feature-scenario axis in `[package.metadata.ci.tests]`),
  but each has its own rationale to audit and this spec does not prejudge
  them.
- Reducing the rendezvous Windows failures themselves (R6 baselines them;
  fixing them is normal known-red burn-down).
- Any change to local developer workflow — `claudine/rendezvous/justfile`
  and `messenger/justfile` recipes are untouched.

## Sequencing

After the per-package cutover (PR #46) merges. The retirement rides the
normal narrow-scope flow: it touches two workflow files, two manifests, and
the contract tests, so its own CI run selects the affected packages and
proves R2/R4 on real evidence.
