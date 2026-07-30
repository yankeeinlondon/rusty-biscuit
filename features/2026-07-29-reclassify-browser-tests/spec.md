---
title: Retire the Browser tier — Chrome becomes an L2 backend
status: draft
created: 2026-07-29
depends_on:
  - PR #19 (docs/cross-platform-ci-plan) must merge first
builds_on:
  - fixes/2026-07-27-refactor/plan.md
source_code:
  - .config/nextest.toml
  - .github/ci/areas.json
  - .github/workflows/_area-ci.yml
  - biscuit-browser-harness/
  - just/devops.just
  - scripts/ci/affected_scope.py
  - tools/test-toolkit/src/backend.rs
---

# Retire the Browser tier

## Objective

Fold the Browser tier into L2 as a `chrome` backend, so the taxonomy is three
levels (L1 → L2 → L3) plus two orthogonal resource kinds (`real_`, `slow_`),
rather than six sibling categories.

After this change an area declares `backends: ["tmux", "chrome"]` instead of
carrying a separate `browser: true` flag, and `just test-l2` runs every test
that needs a provisioned external runtime.

## Why

The taxonomy's own rule is *"tier is about the resource a test needs."* By that
rule L2 means **needs a provisioned external runtime**, and Chrome is exactly
that. The L1→L2→L3 ladder encodes escalating environmental invasiveness; Browser
was bolted on beside it as an orthogonal category at the same invasiveness as
L2, which is why the taxonomy currently has six entries for what is really three
levels plus two resource kinds.

Three concrete consequences of the split, all observed:

1. **Per-backend execution proof does not cover the browser tier.** The
   `BISCUIT_TEST_REQUIRED_BACKENDS` work declined to wire `_test_browser`
   specifically because `Backend` has no browser variant — and because a `reset`
   there would erase the L2 evidence, since `just all` runs `test-l2`
   immediately before `test-browser`. So the browser tier can still exit 0
   having executed nothing, which is the hazard plan §1.1 exists to close.
2. **A second provisioning mechanism.** `browser: true` in `areas.json`,
   `BISCUIT_BROWSER_REQUIRED=1`, and a bespoke `browser` job in `_area-ci.yml`
   duplicate machinery the `backends` array already provides for terminals.
3. **The naming has drifted from the meaning.** A measured inventory found
   `renderable`, `claudine`, and `biscuit-icon` all carry `browser_*`-named
   tests that need no browser, while only `biscuit-terminal`, `darkmatter`, and
   `biscuit-browser-harness` own genuinely harness-driven ones.

The machinery to do this cleanly now exists — `Backend`,
`BISCUIT_TEST_REQUIRED_BACKENDS`, backend-tagged execution evidence, and the
`backends` array — all built during the cross-platform CI refactor.

## Prerequisite — already done

The 2026-07-29 reclassification (step 1) has landed:

- tier predicates anchored to path-segment boundaries, so a marker must be a
  prefix rather than appearing anywhere in the test path;
- unit tests mis-named `browser_*` renamed so they land in L1;
- a guard that fails when an area's tier filter excludes tests while that
  tier's recipe is a stub.

**This spec assumes every remaining `browser_*` test genuinely needs Chrome.**
Verify that assumption before starting rather than trusting it — the same
inventory that produced this spec found the naming unreliable once already.

## Scope

Roughly 150 tests across `biscuit-terminal` (~51), `darkmatter` (~92), and
`biscuit-browser-harness` (~7). Re-measure; these are point-in-time counts.

### 1. `Backend::Chrome`

Add the variant to `tools/test-toolkit/src/backend.rs`. Its `as_str()` is
`chrome`, matching the vocabulary in `KNOWN_L2_BACKENDS` and the `backends`
arrays. `ChromeHarness::available()` becomes its availability probe, and
`biscuit_browser_harness::require_browser()` folds into `require_level!`.

`BISCUIT_BROWSER_REQUIRED=1` is retired in favour of
`BISCUIT_TEST_REQUIRED_BACKENDS=chrome`. Reject the retired variable by name, as
`affected_scope.py` does for `soft_os` and `full_os` — a stale invocation must
fail loudly rather than silently stop enforcing anything.

### 2. Rename the tier

`browser_*` → `level2_*`. Mechanical, but note two things:

- the `browser` tier's tests do not currently carry a backend identifier, so the
  rename is also where each gains `Backend::Chrome` in its `require_level!`;
- naming alone must not decide the tier. Any test that turns out **not** to need
  Chrome goes to L1, not L2.

### 3. Merge the recipes

`_test_browser` disappears; `_test_l2` runs everything. Preserve the two
behaviours that differ, both of which are already per-filter nextest overrides:

- **`-j 1` for Chrome.** `#[serial(browser)]` cannot serialize across nextest's
  process-per-test model, so the tier runs single-threaded. Chrome tests must
  keep that even if the surrounding L2 run is parallel (`BISCUIT_L2_THREADS`).
  A filterset scoped to the chrome subset is the natural expression.
- **5s `leak-timeout` for Chrome.** Headless Chrome's helper and crashpad
  processes inherit the test's stdout and need longer than the 100ms default;
  without the grace they trip spurious `LEAK-FAIL`s. Keep `result = "fail"` so a
  genuinely runaway browser still fails.

Both live in `.config/nextest.toml` today and should survive as scoped
overrides, not as a separate tier.

### 4. Schema and CI

- `areas.json`: `browser: true` → `chrome` in the area's `backends` array.
  Retire the field by name in `affected_scope.py`.
- `_area-ci.yml`: the `browser` job merges into `l2`. Chrome provisioning joins
  tmux provisioning, verified by a named step in the same shape as `tmux -V`.
- `l2_environments` derivation already intersects declared backends with
  provisioned ones, so Chrome-only and tmux-only areas schedule correctly with
  no new mechanism.
- The rollup's tier enum loses `browser`. Decide whether historical baseline
  entries with `tier = "browser"` are migrated or rejected, and say which.

### 5. Docs

`docs/testing-strategy.md`, `.claude/skills/rust-testing/SKILL.md` (regenerate
its `hash:` with `md hash`), `.claude/skills/biscuit-test-harness/SKILL.md`, and
`.github/ci/README.md` all document six tiers and must document five.

## Explicitly out of scope

- The `real_` and `slow_` tiers. `real_` is a genuinely different resource class
  (external devices, live APIs, paid services) that CI never provisions, and
  `slow_` is not a resource at all — it is an L1 subset excluded from `sanity`.
  Neither collapses into the ladder.
- L3. OS-level input injection is a strictly higher invasiveness than a
  provisioned runtime and keeps its own level.

## Success criteria

1. `just test-l2` runs every test needing a provisioned external runtime,
   terminal or browser.
2. A tier that requires `chrome` and executes zero chrome tests **fails**, via
   the same execution-proof path that already covers tmux. This is the gap that
   motivates the change; a migration that does not close it has not succeeded.
3. No test changes tier as a side effect of its name. Every reclassification is
   justified by the resource the test actually needs.
4. Chrome tests still run `-j 1` with the 5s leak grace, whatever the
   surrounding L2 concurrency.
5. `browser: true` and `BISCUIT_BROWSER_REQUIRED` are rejected by name, not
   silently ignored.
6. The area × environment rollup shows no `browser` tier and no lost coverage:
   per-area L1 + L2 counts after equal L1 + L2 + browser counts before.

## Risks

**The rename is the easy half; the tiering judgement is the hard half.** ~150
mechanical renames are low-risk. Deciding which of them actually need Chrome is
not, and getting it wrong in the permissive direction (leaving a unit test in
L2) is invisible — it just makes L2 slower — while getting it wrong in the
strict direction (moving a real browser test to L1) breaks CI loudly. Prefer the
loud failure mode; measure with `ChromeHarness` availability rather than reading
names.

**Do not run this concurrently with other work in the same areas.**
`biscuit-terminal` and `darkmatter` are the two largest areas and the two with
real browser tiers; a 150-test rename across them will conflict with almost
anything else touching their tests.

**Verify on a branch PR run.** Per the devops handoff, five CI-only bugs passed
every local check. A merged `l2` job that provisions both backends is exactly
the kind of change that behaves differently on a runner.
