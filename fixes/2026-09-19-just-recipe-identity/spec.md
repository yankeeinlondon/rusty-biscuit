---
kind: fix
name: just-recipe-identity
date: 2026-09-19
status: implementing
related:
  - 2026-09-18-ci-cadence
  - 2026-09-19-hosted-evidence-reuse
---

# Gate-input identity reads Just files by recipe

## Problem

The planner already treats Just files as global **by recipe**: a change to
`justfile` or `just/*.just` selects packages only when it touches a recipe CI
executes (`_lint`, `_test`, `_test_l2`, `_test_browser`,
`_ensure-native-libs`) or one those reach through header dependencies and
`just <name>` calls (`just_change_gates`, `just_recipe_closure`). Editing
`pre-push`, `cross-check`, or all of `just/plan.just` schedules nothing.

Evidence reuse did not. `local_evidence.gate_global_inputs` added the root
`justfile` and the whole `just/` prefix to every cell's gate-input identity as
`git ls-tree` entries, so any edit to any Just file changed every published
cell's identity. The pre-push hook then re-ran every gate for an edit the
planner would have ignored, and the same asymmetry decides how often a merge
behind an unrelated pull request can reuse anything
(2026-09-19-hosted-evidence-reuse). Since 2026-08-19, 63 of 849 commits
touched a Just file; most reach no gate recipe.

## Fix

`gate_input_identity(paths, ref, gate)` keeps Git's `ls-tree` boundary for
every path input and appends, when a gate is named, the Just text that gate
executes at `ref`: the recipes reachable from the gate's CI entry recipes
through the same closure selection uses, comments and blank lines dropped,
line order preserved, plus every line outside a recipe (settings, imports,
assignments), which stays conservatively global. The files are read from the
tree (`just_sources_at_ref`), so two revisions compare without a checkout.
`gate_global_inputs` no longer lists the Just paths. Both consumers of the
identity — the receipt writer and the verifier — pass the cell's gate.

Receipts written before this fix stay usable: verification recomputes both
sides of the comparison rather than trusting the stored identity.

## Proof

`JustRecipeIdentityTests` in `scripts/ci/test_evidence_reuse.py` asserts, for
each edit, both the gates whose identity moved and the planner's
`just_change_gates` answer for the same edit: a recipe CI never reaches moves
nothing; a helper the test gate reaches through a `just` call in a module
moves only the test tiers; `_lint` moves only lint; text outside every recipe
moves every gate; a comment-only edit moves nothing; swapping two commands
inside a recipe moves it. Two end-to-end cases drive `verify_cells`: an older
receipt survives a `notify` edit and is rejected with `gate-inputs-changed`
for a `_tier_filter` edit.

## Not done

- Narrowing which recipes are gate inputs. If `just/devops.just`'s gate
  recipes prove to move too often, that is a separate decision.
- Reusing CI's own results across trees: 2026-09-19-hosted-evidence-reuse.
