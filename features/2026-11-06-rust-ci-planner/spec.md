---
kind: feature
name: rust-ci-planner
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-11-06-rust-evidence-client
next_spec: 2026-11-06-python-retirement
related:
  - 2026-11-06-any-host-evidence
reviewed: false
implemented: false
---

# Port the planner to Rust

## Why

`scripts/ci/affected_scope.py` is about 6,000 lines with a 10,600-line test file. It
decides what must be proven, and it is the hardest part of the system to read or
change. By this point the evidence code is gone, which should leave it smaller.

## Outcome

- One Rust planner, used unchanged by hosts and CI, produces the plan.
- CI uses the plan already published for a commit when one exists, and calculates
  it only when none exists. Recalculating when a valid plan exists is removed.
- Its rules are stated in `docs/`, readable without reading the code.
- `affected_scope.py` and its tests are deleted.

## Scope

- Reuse `sniff` for package and area discovery, `cargo metadata` for the graph, and
  `biscuit-hash` for identities.
- Move plan publication off `refs/notes/ci-local/scope`, the last Git note in use,
  most likely into the evidence store. `d1-model.md` has no table for it yet.
- Port the hook's remaining planner call, `affected_scope.py --apply-to`.
- Write the plan's rules out as documentation **before** porting them, and ask of
  each one whether it is still needed.
- Show equivalence by comparing plans on recorded real pushes, including the
  `a816f1447` regression.

## Spikes needed

- **Rule inventory.** Sort the 6,000 lines into live policy, workaround, and dead code.
  This sets the true size of the port.
- **Golden-plan corpus.** Can recorded historical plans replace most of the 10,600
  lines of tests, rather than translating them one by one?
- **Startup cost.** Each pre-push and CI run needs this. Confirm the prebuilt tool
  avoids compiling the workspace to plan it.

## Not in scope

Symbol-level blast radius, which is `2026-10-26-change-detection`. The port keeps the
package-level model.
