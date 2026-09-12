---
title: Phase 1 blockers — prerequisite decision, OQ1–OQ4, and the GitHub fixtures
kind: blockers
created: 2026-09-11
for: fixes/2026-09-11-cicd-cleanup/plan.md
status: awaiting-ken
---

# Phase 1 blockers

Phase 1 tasks 3 and 4 cannot be completed by an agent. This page records what
each one needs, what was settled without authorization, and exactly what is
left for Ken. Nothing here is a recommendation substituted for a ruling.

## B0 — The prerequisite decision (blocks Phases 3 and 4)

`prerequisite-audit.md` finds `fixes/2026-09-10-local-affected-scope/spec.md`
**not implemented**: 3 of 16 acceptance criteria met, 2 vacuous, 2 partial,
9 absent.

The 2026-09-11 plan's Phases 3 and 4 are written to *modify* contracts that
do not exist — the per-cell result set, `scope-only`, `off`-as-alias, and
failure-path receipt publication. Ken must choose:

1. **Land 2026-09-10 first** as its own fix; resume this plan at Phase 2.
2. **Absorb it** — rewrite Phases 3 and 4 to *build* those contracts. This is
   a real scope increase and changes what Phase 2's "freeze the shared
   contracts" covers.

Option 2 is the cheaper total path, because this specification already
supersedes three of 2026-09-10's decisions (compile-only reverse dependency,
one-environment receipt, `ci-verdict`). Implementing them and then reversing
them is wasted work. But it must be a recorded decision.

**Ruled 2026-09-12: absorb.** Recorded in the spec's `## Rulings` (B0);
the formal pass over the 09-10 objectives is `absorption-audit-2026-09-12.md`,
which supersedes `prerequisite-audit.md`. The plan is no longer stopped on B0.

## B1 — OQ1: where the downstream seam gets compiled

**Status: ruled 2026-09-12 — Option B (compile direct reverse dependencies
inside the changed package's check job); see the spec's `## Rulings`.**

Measured input for the decision, from `baseline-2026-09-11.md`: the AC1
fixture (Claudine + Playa source) currently produces four compile-only
reverse-dependent entries — `biscuit-speaks`, `claudine-cli`,
`claudine-contract`, `playa-cli` — across three areas. Under Option B these
four become one reported line inside the `claudine` and `playa` check cells
and create no area, job, or result cell.

Nothing further can be established without the ruling. No implementation task
in Phase 3 may begin.

## B2 — OQ2: where an execution constraint is persisted

**Status: ruled 2026-09-12 — Option B (per-repository directory on the host);
see the spec's `## Rulings`.** The store must work identically on macOS,
Linux, native Windows, and WSL2, resolving the home directory portably.

One fact worth having before the ruling: the evidence directory the spec
points at, `~/.rusty-biscuit/ci-evidence/`, is a host convention established
by PR #74, not a repository artifact. Option B puts the constraint store
beside it, so both live or die with the host. That is the stated trade-off
(cross-host loss accepted), and it is unchanged by anything measured here.

## B3 — OQ3: the merge gate after `ci-verdict`

**Status: ruled 2026-09-12 — Option C (ruleset "Require workflows to
pass"), Option B (fixed-name conjunction job) as the fallback; see the spec's
`## Rulings`.** The scratch-repository experiment is authorized by the ruling;
the live ruleset edit on this repository is still a separate approval.

Settled read-only:

| Fact | Value | Why it matters |
|---|---|---|
| Repository visibility | **public** | Repository rulesets are available. |
| Owner type | **User**, not an organization | Some ruleset capabilities are organization-scoped; this repo has no org to fall back on. |
| Rulesets present | exactly one — `protect-your-bacon` (19747338) | The migration touches one object. |
| Current required contexts | exactly one — `ci-verdict` | A stale context here blocks every PR forever. |
| Bypass actors | `RepositoryRole` 5 (admin), `bypass_mode: pull_request` | Left alone; the spec makes it a separate item for Ken. |

**Not settleable read-only.** Whether the `workflows` ruleset rule type is
accepted on a user-owned public repository, and how it behaves for
`workflow_dispatch` and reruns, can only be established by *writing* a
ruleset. The REST API rejects an unsupported rule type at write time and
offers no capability query. This is precisely why the specification mandates
a scratch repository — and it is why B3 cannot be closed without B4.

## B4 — OQ4: `cancelled` or `neutral` for accepted policy gaps

**Status: ruled 2026-09-12 — Option B (`neutral`); see the spec's
`## Rulings`.** The fixture records the presentation; it no longer decides
the conclusion.

**Concrete blocker found while measuring:** no job in `.github/workflows/ci.yml`
currently holds `checks: write`. The top-level grant is `contents: read`; the
two jobs that widen it take `actions: read`, `pull-requests: read`, and
`checks: read`. Whichever job publishes accepted-gap check runs must acquire
`checks: write`, and that widening should be scoped to that job alone rather
than hoisted to the workflow. Record this in Phase 6.

## B5 — The GitHub fixtures (Phase 1 task 3)

**Status: authorized 2026-09-12 (both fixtures); see the spec's `## Rulings`, B5.**

The task needs two things an agent must not do unasked:

1. a push to a throwaway branch of `yankeeinlondon/rusty-biscuit`, which
   triggers workflows; and
2. creation of a scratch repository under the same account, plus ruleset
   writes inside it.

The plan's own execution constraints say "No task in this plan authorizes a
commit, push, workflow dispatch, ruleset edit, or WSL rerun." This session is
non-interactive, so authorization cannot be requested. The fixtures are
therefore deferred with their requirements written down, so they can be run
immediately once authorized.

### Already proven live — no fixture needed

Two of the six things the task lists are already demonstrated by run
`34638047631`, at no additional cost:

- **Skipped-job naming.** 63 of 144 check entries carry a literal
  `${{ … }}` name, e.g.
  `claudine / test-l2 (${{ inputs.package }} on ${{ matrix.environment }})`.
  This confirms the documented collapse behavior and fixes AC10's target as
  worded: a job skipped through `if:` still appears, so the achievable goal
  is that no skipped placeholder carries an unresolved label.
- **Reusable-workflow display nesting.** Names such as
  `claudine / wsl2 (claudine) / archive (claudine for wsl2)` confirm the
  `<caller job> / <called job>` rendering **and** that the chain is already
  three levels deep (`ci.yml` → `_package-ci.yml` → `_wsl-ci.yml`). The
  spec's area-level workflow makes it four, GitHub's maximum. **There is no
  headroom.** Phase 6 must treat four as a hard ceiling, and the Phase 1
  validation checkpoint's "no deeper than four levels" is satisfied only
  exactly, with no margin for a later insertion.

### Still requiring the throwaway branch

- Synthetic check runs created through the Checks API with conclusions
  `cancelled`, `neutral`, and `skipped`, each carrying title, summary,
  Markdown text, and `details_url`.
- Whether such a check run alters the enclosing workflow run's conclusion
  (the spec asserts it does not; `reuse_validation.py` and `release-plz.yml`
  both key on that conclusion, so a wrong answer here breaks releases).
- The PR merge-box wording produced by a non-required `cancelled` check —
  the one input only Ken can weigh, and the deciding factor for OQ4.

### Still requiring the scratch repository

- Whether the `workflows` ruleset rule type is accepted on a user-owned
  public repository at all (OQ3 Option C).
- Its behavior for `workflow_dispatch` and for reruns.
- That a failed selected area blocks, an unselected area creates no pending
  requirement, missing required coverage blocks, and a successful run
  satisfies the exact ruleset configuration to be used.
- The Option B fallback under the same cases, if Option C is unavailable.

## Summary for Ken

Five decisions are needed before any implementation task in this plan may
start:

| # | Decision | Depends on |
|---|---|---|
| B0 | Land 2026-09-10 first, or absorb it into Phases 3–4 | **ruled 2026-09-12: absorb**; audit D1/D2 pending |
| B1 | OQ1 seam location | **ruled 2026-09-12: Option B**, in-job step |
| B2 | OQ2 constraint store | **ruled 2026-09-12: Option B**, any host OS |
| B5 | Authorize the throwaway branch and scratch repository | **authorized 2026-09-12; scratch half DONE**, branch half waits for the publisher and area workflow |
| B3 | OQ3 merge gate | **ruled 2026-09-12; experiment shows Option C unavailable, Option B proven** (`fixtures/scratch-2026-09-12.md`) |
| B4 | OQ4 gap conclusion | **ruled 2026-09-12: `neutral`** |

B0 through B5 are all ruled or authorized as of 2026-09-12. Nothing in this
document blocks implementation any longer.
