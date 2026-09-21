---
kind: spike-record
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 1
status: deferred-requires-push
questions: [matrix-label-from-include-only-rows, skipped-whole-job-label, four-level-chain-single-call]
---

# S1 — Matrix labels, whole-job skips, and nesting under a single area call

## Status: DEFERRED — requires a push

The spike's method (a minimal scratch workflow on a scratch branch, per
`fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`) requires a
commit and a push. This session may not commit or push. Per the plan's own
escape hatch, this record proceeds on documented behavior plus this
repository's proven in-repo evidence, and defers the hosted confirmation with
its exact open questions. No full-scope run is proposed to answer them.

## What is already proven, in this repository

These facts need no spike; they are proven by shipped contracts and the
2026-09-12 scratch run (`scratch-2026-09-12.md`, run numbers cited there):

1. **A matrix job that runs is labeled from every value in its row.**
   `ci.yml`'s `area-ci` matrix is deliberately a plain vector rather than an
   `include:` of package records so the label stays readable
   (`ci.yml:739`); `preflight (ubuntu-latest)` is the single-key form seen
   live.
2. **A job skipped as a whole never has its matrix context evaluated.** A
   declared `name:` containing `${{ matrix.* }}` on a skippable job reaches
   the Checks tab as raw expression text. This is why `preflight`, `area-ci`,
   `test`, `test-l2`, `test-browser`, `wsl2`, and `accepted-gaps` carry no
   `name:`, and is asserted by
   `no_skippable_job_is_labelled_with_an_unresolved_expression` and
   `empty_execution_matrices_skip_before_expansion`.
3. **A skipped matrix job's static label is its job id** (`area-ci`,
   `preflight`, `test`), and downstream `needs` read it as `skipped`, which
   `ci-gate` folds as a pass.
4. **The four-level chain resolves today** (`ci.yml` → `_area-ci.yml` →
   `_package-ci.yml` → `_wsl-ci.yml`), asserted by
   `the_reusable_workflow_chain_stays_within_githubs_four_levels`.

## What the documentation says (no run needed)

- GitHub documents up to **ten levels** of nested reusable workflows and
  **50 unique reusable workflows** per top-level caller workflow file
  (Reusing workflow configurations, "Limitations of reusable workflows").
  This design stays at four levels and three called workflows.
- A job that calls a reusable workflow supports exactly `name`, `uses`,
  `with`, `secrets`, `strategy`, `needs`, `if`, `concurrency`,
  `permissions`, `cache-mode` — so a matrix row-expanding delegator passing
  four row sets through `with:` is within the supported keyword set (this is
  also how `packages:` passes a per-area JSON document today).
- Nothing in the documentation distinguishes a reusable-workflow call made
  once with a row set from one made per package: the `with:` block is
  evaluated per matrix leg either way.

## Derived expectation (used until the hosted run confirms)

For a four-key `include:` row `{package, gate, environment, runner}` and a
job id `test`, the running label should be:

```
test (homelab-server, L1, ubuntu-latest, ubuntu-latest)
```

— every value in the row, comma-space separated, behind the job id; and the
skipped-whole label should be the bare job id `test`. R1 accepts the
four-token form and requires `runner_loss.py` to parse both it and today's
`test (ubuntu-latest)`, so even if the hosted run shows a different
tokenization than expected, attribution degrades to a parser update rather
than a silent loss.

## Open questions for the hosted confirmation

1. Does an `include:`-only matrix (no scalar matrix keys) of four keys label
   the running job with all four values in row order, exactly as derived
   above — including when one value repeats another (`environment` ==
   `runner` for native rows)?
2. Does a job whose scalar `if:` guard skips it before expansion show the
   bare job id in the Checks tab, with no unevaluated expression, when the
   matrix is an `include:`-only row set supplied through `fromJSON()`?
3. Does `ci.yml → _area-ci.yml → _package-ci.yml → _wsl-ci.yml` still resolve
   when the third level is called **once per area** (row sets in `with:`)
   rather than once per package — specifically, do the `wsl2` delegator's
   per-row calls to `_wsl-ci.yml` render `… / wsl2 / test (wsl2-ubuntu)`
   as today?

## How to close this (when a push is available)

Reuse the prepared reduced hosted fixture
(`fixes/_completed/2026-09-11-cicd-cleanup/fixtures/hosted-fixture.md`,
status `prepared-not-hosted`) or a minimal scratch branch carrying only: one
`scope`-shaped job, one `area-ci`-shaped matrix caller, and a row-driven
`_package-ci.yml` whose test job is an `include:`-only four-key matrix with a
scalar guard. One pull request answers all three questions. Record the
answers here and flip `status` to `complete`; update R1 only if the token
order differs.
