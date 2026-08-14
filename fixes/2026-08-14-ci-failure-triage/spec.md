---
status: draft
created: 2026-08-14
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-14
review_iterations: 2
area: claudine
depends-on: ../2026-08-13-finalize/spec.md
evidence: ../2026-08-13-finalize/failing.md
packages:
    - darkmatter
    - darkmatter-cli
    - claudine-cli
    - biscuit-terminal-cli
---

# Attribute and clear the failed producer cells of run 31753281913

## Summary

CI run `31753281913` on `fix/ctx-launch-anchor` at commit `a00ea7c08` exposed
three populations that require different treatment:

- **P1 — unattributed, branch-relevant failures.** One Darkmatter CLI Level-1
  identity on Windows, eight Level-2 rendering identities across
  `claudine-cli`, `darkmatter-cli`, and `biscuit-terminal-cli` on macOS and
  Ubuntu, and the failed `claudine-cli/wsl2-ubuntu` producer cell omitted from
  the provisional catalog. These must be compared with `main` on the same
  environment before either code or policy changes are made.
- **P2 — known WSL2 archive assumptions.** Six Darkmatter identities and three
  Claudine identities depend on `rustc` being discoverable at test runtime.
  The CI WSL2 guest deliberately runs a prebuilt nextest archive without Cargo
  or rustc. Prior evidence records these identities on `main`; a real WSL2
  development host with a toolchain is a different environment and is expected
  to pass them.
- **P3 — previously verified main drift.** The remaining `sniff`, `sniff-cli`,
  `rendezvous-daemon`, `unchained-ai`, `biscuit-tui-cli`, `dmls`, `messenger`,
  `model_id`, and `biscuit-speaks-cli` failures already have main-side
  attribution or handoffs. Their disposition still needs fresh identity
  evidence because the parent spec's comparison predates this run.

The checked-in [`failing.md`](../2026-08-13-finalize/failing.md) was captured
while the workflow was still running. It reports 601 jobs, 24 failures, one
running job, and 51 identities; the job API later exposed 603 completed jobs.
The catalog is therefore provisional and must not be treated as the final
denominator or complete cell inventory.

> **Reader's note — corrections made during review.** The draft attributed the
> WSL2 group to an inability to spawn shells, described a July 10 Darkmatter
> test as branch-added, stated that `darkmatter/wsl2-ubuntu` was green and
> unbaselined on `main`, and proposed using `.github/ci/environments.json` to
> govern arbitrary shell-dependent Level-1 tests. The repository evidence
> contradicts each claim. The guest has Bash but intentionally has no Rust
> toolchain; three of the six Darkmatter failures are parser-only tests whose
> classification happens to consult `PATH`; the cited interpolation test and
> the existing Darkmatter WSL2 baseline both predate this branch; and the
> environment table governs the repository's known CI tier/backend
> capabilities, not per-test executable prerequisites. This specification
> retains the established full-Level-1 WSL2 contract and addresses fixture
> assumptions separately.

## Scope

**In scope:**

1. Reconcile the completed producer-job inventory with `failing.md` before
   relying on its counts or classifications.
2. Attribute and, when branch-caused, fix the Darkmatter CLI Windows Level-1
   failure.
3. Attribute every currently unattributed Level-2 identity against `main` on
   the same host and fix any branch-caused failure.
4. Correct the WSL2 attribution and prove that this branch adds no failing
   identity to an already-baselined WSL2 cell.
5. Re-run the identity-aware comparison before applying the parent spec's
   ratified baseline changes or accepting any existing cell-wide baseline.
6. Correct the parent fix's verification record and gate based on the outcome
   of Open Question 1.

**Out of scope:** implementing main-drift product fixes for `sniff`, Neovim
provisioning, messenger D-Bus handling, daemon security descriptors, ConPTY
shutdown, or other unrelated packages. Redesigning the WSL2 workflow from a
canonical Level-1 environment into a targeted interop suite is also out of
scope; that would be a repository-wide CI contract change requiring its own
specification and migration plan.

## Fix specification

### F1 — Reconcile the run inventory before disposition

Refresh [`failing.md`](../2026-08-13-finalize/failing.md) from all completed
producer jobs in run `31753281913`. Count producer failures separately from the
expected downstream `ci-verdict` failure so the gate is not mistaken for an
additional product defect.

For each failed producer cell, record:

- `{package, environment, tier}` and the job identifier;
- the complete JUnit failing-identity set, when JUnit exists;
- the exact log-derived diagnostic set for lint or other zero-identity cells;
- whether the cell was already baselined at `a00ea7c08`;
- whether its identity set is equal to, a subset of, or a superset of the
  comparable `main` cell; and
- a link to its attribution evidence or main-side handoff.

Correct the catalog's WSL2 claims while refreshing it: the six Darkmatter
identities are not branch-owned, and
`interpolation_literal_pipeline::frontmatter_literal_survives_shell_bracketed_interpolation_passes`
was introduced before this branch. Recover the complete identity set for the
failed `claudine-cli/wsl2-ubuntu` producer, which is present in the completed
job inventory but absent from the provisional catalog. Preserve any newly
discovered identity rather than forcing the final catalog back to the
provisional count of 51.

**Acceptance.** The catalog accounts for every completed failed producer job,
its cell and identity totals reconcile with the source artifacts, and no
classification rests only on package ownership or subsystem scope.

### F2 — Resolve the Darkmatter CLI Windows contradiction

`schema_validate_baseline::schema_validate_legacy_pretty_output_is_byte_identical`
failed in `darkmatter-cli/windows-latest`, while the recorded native Windows 11
run reports all 655 Darkmatter CLI Level-1 tests passing at the same commit.
This identity was omitted from the later draft even though no evidence closed
it.

Reproduce the exact test on the branch and `main` using the same native Windows
host and test command. Record whether each run executed the identity and
compare the relevant inputs: feature flags, nextest profile and filters,
`NO_COLOR`/terminal and hyperlink state, working-directory spelling, and the
actual-versus-expected bytes for each fixture case. Do not infer equivalence
from an aggregate package count.

If the failure is branch-caused, repair the owning rendering or path-projection
layer and add the narrowest OS-independent regression coverage possible. If
the expected output legitimately changes, re-derive it through the baseline's
documented review workflow; never update it merely to match CI. If it is
main-red or environment-specific, record the handoff and disposition it only
through F5.

**Acceptance.** The contradiction has a byte-level explanation, the identity
passes on the branch when branch-caused, and the native Windows and
`windows-latest` results no longer disagree without a documented environmental
reason.

### F3 — Attribute the Level-2 rendering failures

The currently known unattributed identities are:

| Cell | Identity |
| --- | --- |
| `claudine-cli` macOS + Ubuntu | `level2_context_capture::level2_context_default_at_140_fills_cap_in_tmux` |
| `claudine-cli` macOS + Ubuntu | `level2_context_capture::level2_context_default_caps_at_140_in_wide_tmux` |
| `claudine-cli` macOS + Ubuntu | `level2_context_capture::level2_context_default_narrow_preserves_type_and_wraps_in_tmux` |
| `claudine-cli` Ubuntu | `level2_context_capture::level2_context_default_preserves_columns_at_min_width_in_tmux` |
| `darkmatter-cli` macOS | `level2_code_block_styling::level2_code_block_clears_inherited_dim_before_theme_colors` |
| `darkmatter-cli` Ubuntu | `level2_schema_about::level2_schema_about_light_terminal_uses_dark_code_theme` |
| `biscuit-terminal-cli` Ubuntu | `level2_diagrams::level2_diagram_fallback_when_no_image_protocol` |
| `biscuit-terminal-cli` macOS | `level2_apple_terminal_prose::level2_apple_terminal_double_underline_plain_text_visible` |

Run the canonical Level-2 suites with `--no-fail-fast` for the affected package
areas on macOS and Linux. Compare branch and `main` on the same host, terminal
backend, dimensions, color environment, and fixture binaries. A green
expectation from another host is not attribution.

For each identity, capture the rendered-byte or screen-state delta and classify
it as branch regression, main drift, harness/environment defect, or confirmed
flake. A flake classification requires repeated evidence and a stated trigger;
one passing rerun is insufficient. Fix branch regressions without weakening
the terminal assertions. Record main-side handoffs for the other classes.

All Level-2 runs must use the repository harness and must never open or focus a
host terminal window.

**Acceptance.** Every listed identity, plus any added by F1, has same-host
branch-versus-main evidence and a disposition. All branch-caused identities
pass through the canonical Level-2 recipe, and the parent fix's verification
record is corrected to state that Level 2 was not run before this CI result.

### F4 — Preserve the WSL2 contract and record the fixture handoff

The established WSL2 contract in `.github/workflows/_wsl-ci.yml` is a package's
canonical Level-1 suite executed from a Linux-built nextest archive inside a
real WSL2 guest. The guest intentionally has Bash and Git but no Cargo or
rustc. Its value is precisely that it reveals runtime environmental differences
from `ubuntu-latest`; passing on a development WSL2 host with a toolchain does
not invalidate that signal.

The six Darkmatter identities divide into two fixture classes:

- the three `no_cache` parser tests use the bare token `rustc`, although they
  test suffix parsing. Darkmatter's token-resolution ladder consults `PATH` for
  an ambiguous lone bare name, so absence of rustc changes the parse class even
  though no process is launched; and
- the interpolation-pipeline and two coordinate/diagnostic tests genuinely
  execute `rustc` to obtain output or a controlled nonzero exit.

The three Claudine identities likewise compile or execute a rustc-based probe.
Prior WSL evidence records all nine as main drift. The Darkmatter cell already
has a WSL2 Level-1 baseline, and the dependency spec ratified a short-lived
entry for the Claudine cell subject to exact identity evidence.

For the main-side handoff, parser tests must use syntax that is unambiguously a
command without consulting host `PATH`, such as a path-bearing dummy token.
Runtime tests must use a repository-owned, cross-platform fixture executable
that is carried in the nextest archive, or another hermetic mechanism that
still exercises the intended subprocess result. Do not silently return early
from an ordinary Rust test and call that a visible skip; nextest records such a
test as passed. Do not add a generic `shell` capability to
`.github/ci/environments.json`, because that table does not select individual
Level-1 tests and the guest can spawn available processes.

**Acceptance.** A fresh per-cell comparison, including the omitted
`claudine-cli` producer, shows no branch WSL2 identity set is a superset of
`main`; existing and ratified baseline entries name the actual environmental
assumption; and the durable fixture cleanup is recorded as a main-side
handoff. No WSL2 cell is narrowed or removed by this fix.

### F5 — Re-run identity comparison before using any baseline

The dependency spec ratified evidence-backed, short-lived baseline entries, but
its drafted entries compare branch run `31651014023`, which predates the fixes
and failures under review here. Re-run the comparison against the newest
completed branch run at or after `a00ea7c08` and a comparable current `main`
run for each environment.

Apply these rules to both newly proposed entries and already-baselined cells
that are red in the branch run:

1. A branch identity set that is a superset of `main` blocks; a cell-wide
   baseline must not hide the added identity.
2. Equality or subset status must be supported by exact JUnit identities. For
   lint and other zero-identity cells, compare normalized log diagnostics
   instead of treating two empty identity sets as evidence.
3. Every new entry must retain the owner, reason, exact source run, and
   `2026-09-30` expiration ratified by the dependency spec. Do not extend an
   existing entry's expiration as part of this triage.
4. Passing, missing, or expired baselines must continue to block according to
   `.github/ci/README.md`.

**Acceptance.** `ci-rollup` reports neither `baseline-no-result` nor
`baseline-now-passing`; every accepted failed cell has current comparison
evidence; no branch-superset cell is accepted; and the final `just ci-diff`
review covers existing as well as newly added baseline entries.

## Phases

1. **Phase 1 — Evidence integrity.** F1, including the final producer-cell and
   identity inventory.
2. **Phase 2 — Branch attribution.** F2 and F3 on matched Windows, macOS, and
   Linux hosts. Make no baseline change during attribution.
3. **Phase 3 — Branch-owned fixes.** Repair only failures attributed to this
   branch and run the relevant Level-1 or Level-2 package gates.
4. **Phase 4 — WSL2 and main-side handoffs.** F4, including corrections to the
   catalog and baseline reasons; do not redesign the WSL2 tier here.
5. **Phase 5 — Gate policy.** F5, followed by one authoritative full CI run and
   an identity-aware comparison.

## Verification matrix

- Native Windows 11 and `windows-latest` — the focused
  `schema_validate_baseline` identity, then Darkmatter CLI Level 1.
- Linux — canonical Level 2 for `claudine-cli`, `darkmatter-cli`, and
  `biscuit-terminal-cli`, with same-host `main` comparisons for red identities.
- macOS — the same three canonical Level-2 package suites and same-host `main`
  comparisons.
- `wsl2-ubuntu` — Darkmatter, Claudine, and Claudine CLI Level 1 from the
  canonical archive workflow; compare identities with `main`, not with a
  toolchain-equipped local WSL2 host.
- A full CI run as authoritative proof after local attribution and fixes, then
  `just ci-diff` plus raw diagnostic comparison for zero-identity cells.

The Level-2 harness must remain headless with respect to host terminal windows.

## Success criteria

1. Every completed failed producer cell in run `31753281913` has a complete,
   evidence-backed disposition; the provisional count of 51 is corrected if
   necessary.
2. No branch-caused Level-1 or Level-2 identity remains red on any applicable
   host.
3. The Darkmatter CLI Windows contradiction is explained at byte level.
4. WSL2 failures are described as concrete missing-toolchain, timing, service,
   or fixture assumptions rather than a generic inability to execute shells.
5. The established full-Level-1 WSL2 contract remains intact.
6. The parent fix records its Level-1-only verification gap and adopts the
   Level-2 decision selected below.
7. No baseline accepts a branch identity set that is a superset of `main`,
   including cells that were already baselined before this fix.

## Ratified decisions

### 1. What Level-2 coverage should join the parent fix's acceptance gate?

**Decision (ratified 2026-08-14): Option A.** Canonical Level-2 coverage is
required for package areas whose terminal-rendered behavior, descriptor
catalogs, or rendering inputs changed. For the parent branch, that means the
Claudine and Darkmatter package areas. The pre-CI local acceptance record was
Level 1 only; run `31753281913` demonstrated that leaving these suites until CI
creates a late attribution gap.

The parent specification concluded that Level 2 added no relevant assurance,
but this branch changes context descriptors and terminal-visible composed
values, and the subsequent CI run contains failures in both surfaces.

**Option A — Require affected-package Level 2 (selected).** Run canonical
Level-2 suites for package areas whose terminal-rendered behavior, descriptor
catalogs, or rendering inputs changed; for this branch that includes Claudine
and Darkmatter.

- **Pros:** exercises the behavior the branch can plausibly move; catches
  layout and escape-sequence regressions before full CI; keeps the gate bounded
  by impact.
- **Cons:** requires an explicit impact judgment and same-host attribution when
  a package's existing Level-2 suite is already red.

**Option B — Require all repository Level 2 for every cross-package fix.** Make
the complete Level-2 matrix a universal acceptance gate.

- **Pros:** simple rule; maximizes regression detection across downstream
  renderers.
- **Cons:** expensive, absorbs unrelated environmental debt, and conflicts with
  surgical package-area verification.

**Option C — Keep Level 1 only and attribute Level 2 after CI.** Leave the
parent gate unchanged.

- **Pros:** lowest local cost and no new dependency on real-terminal harnesses.
- **Cons:** repeats the exact late-discovery gap that opened this fix and gives
  no pre-CI evidence for terminal-visible changes.

Option A aligns the verification level with the changed behavior without
turning every fix into a workspace-wide terminal test run. The parent
verification matrix now records the selected gate.
