---
area: darkmatter
status: unscheduled
created: 2026-09-08
owner: Ken Snyder <ken@ken.net>
origin: darkmatter/fixes/2026-09-07-faster-darkmatter-tests/results.md
packages:
    - darkmatter
---

# Darkmatter test-suite residuals from the 2026-09-07 faster-tests fix

Two findings remain after the fixture, route, passive-effect, and resource
ownership work. Neither is generic fixture migration; the deterministic `md`
spawn allow-list is empty.

## Review the existing preflight property-test override

- **Evidence:** `.config/nextest.toml` already gives
  `markdown::compose::preflight::acceptance_tests::execution_subset_of_approval_across_randomized_conditions`
  a `30s × 3` slow-timeout override. Baseline attribution measured 4.31–6.93
  seconds depending on cohort, matching its historical comment; see
  [`inventory.md` § Runner override census](../../2026-09-07-faster-darkmatter-tests/inventory.md#runner-override-census).
- **Why deferred:** the faster-tests fix freezes runner overrides and cannot
  remove or narrow an existing limit without compatible loaded CI evidence.
- **Closes when:** three comparable green runs per configured leg establish the
  loaded distribution, the override is retained with current evidence or
  narrowed/removed in its own change, and no retry or timeout failure is hidden.

## Remove three stale `#[ignore]` comments

- **Evidence:** module documentation in
  `lib/src/markdown/schemas/simplified/{grammar.rs,convert.rs,mod.rs}` says its
  phase tests remain `#[ignore]`-gated, but the attributes were removed when
  those phases landed. The source/runner reconciliation finds five real ignore
  attributes and these three prose-only mentions.
- **Why deferred:** this is comment-only cleanup. The repository's scope rule
  requires it to remain separate from behavior and assertion changes.
- **Closes when:** the three comments are deleted or corrected in a
  comment-only change and the schema tests retain their current execution
  route.

Owner: **Ken Snyder**.
