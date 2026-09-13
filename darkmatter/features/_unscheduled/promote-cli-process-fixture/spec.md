---
area: darkmatter
status: unscheduled
created: 2026-09-08
owner: Ken Snyder <ken@ken.net>
origin: darkmatter/fixes/2026-09-07-faster-darkmatter-tests/results.md
packages:
    - test_toolkit
    - darkmatter-cli
    - claudine-cli
---

# Promote the stabilized CLI process fixture core into `test_toolkit`

Darkmatter and Claudine now carry independently proven deterministic L1
process fixtures. Extract only their stable common mechanics into
`test_toolkit`, then migrate both consumers without weakening their
area-specific policies or structural guards.

## Evidence and reason

- Darkmatter's `CliProcessFixture` and `MdCommandBuilder` are documented and
  exercised in `darkmatter/cli/tests/common/fixture.rs` and
  `md_process_fixture.rs`. They pin disposable CWD/home/config/cache/temp,
  scrub application/Git/rendering inputs, compose PATH portably, reject a
  checkout-contained root through symlinks, and apply one policy to
  `assert_cmd::Command` and `std::process::Command`.
- Claudine has the second concrete consumer in
  `claudine/cli/tests/common/mod.rs`, including its own lifecycle side-effect
  defaults.
- The 2026-09-07 Darkmatter faster-tests fix intentionally kept the first
  implementation area-local so the seam could be learned from real consumers.
  Duplicating it indefinitely risks containment, Windows, and environment
  scrub fixes landing in only one area.

This was deferred because promotion would have expanded a test-performance fix
into another package area and consumer before Darkmatter's final escape and
environment requirements were stable. Generic Darkmatter spawn migration is
not deferred: its migration and guard burn-down are complete.

## Scope

Extract the smallest shared core that both consumers already need:

- disposable workspace topology and checkout-containment rejection;
- portable minimal system PATH construction;
- ordered environment clear/remove/set policy data;
- adapters applying one policy to both command surfaces;
- fixture-owned executable-stub support where the byte/script mechanics are
  genuinely identical.

Keep binary selection, application-variable rosters, side-effect defaults,
named escapes, topology helpers, and source-scanning guards area-local unless
both migrated consumers prove a shared contract. Do not turn `test_toolkit`
into a general process runner or move production business logic into it.

## Acceptance

- Darkmatter and Claudine consume one shared fixture core from `test_toolkit`;
  neither retains a fork of the extracted containment/PATH/environment-policy
  implementation.
- Both suites retain per-test disposable roots and their current hostile-parent
  regression inputs, dependent child outputs, symlink containment negatives,
  Windows PATH/console compile guards, and assert/std command-surface parity.
- Each area's named escapes and raw-spawn/isolation guard remain readable and
  area-specific; both guards report zero generic migration exemptions.
- `just test` and `just lint` pass in `test_toolkit`, Darkmatter, and Claudine;
  Windows arms compile for all three affected packages.
- No production API or runtime dependency is introduced.

Owner: **Ken Snyder**. The work should be scheduled only when both package
areas can be validated together.
