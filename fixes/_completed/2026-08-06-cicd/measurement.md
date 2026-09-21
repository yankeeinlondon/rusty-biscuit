# Phase 6 — measurement

Spec § Measurement: "Justified by compute saved, so measured rather than
asserted." Both datapoints are measured runs, not estimates.

## The narrow-change comparison

**Change shape:** a doc-comment-only edit to one package (`dmls`) in a
multi-package directory (`darkmatter/`, three packages). PR #48 (measurement
vehicle, closed unmerged), run **31379745447** on the new system; the
before-figures are the darkmatter + claudine + research area jobs of run
**30812737709**, the last completed full fan-out under the old system — the
exact set the old area-level scope selected for a darkmatter-directory change
(the darkmatter area plus its area-level dependents).

| Metric | Old (area system) | New (per-package) |
|---|---|---|
| Packages tested | every package in 3 areas (~10) | **1** (`dmls`) |
| Result-producing jobs | 53 | **17** (9 estimated + specialized/coverage riders) |
| Runner-minutes (compute) | **~725** | **71** |
| Largest single job | 30 min | 8.7 min |
| Unrelated same-directory packages scheduled | darkmatter, darkmatter-cli (plus claudine ×5, research ×2 areas) | **none** — AC1 observed live |

**Compute: ~10× less for the same change**, with the reverse-dependency
guarantee intact (dmls has no dependents; a `biscuit-speaks` change was
separately verified to select its exact 7-package closure).

Wall-clock start→verdict was 97 minutes, but is not comparable evidence in
either direction: the run queued behind another in-flight run for its macOS
L2 slot and WSL leg, and every cache was cold (below). Job compute is the
faithful metric; the old system's 53 jobs faced the same queueing physics
with 10× the demand.

Billed-minutes caveat: the billing API reports zeros for this repository, so
compute is summed job durations — the same quantity billing counts, without
per-OS multipliers (which favor the new system further: the old fan-out ran
9 macOS jobs at the 10× rate; the new run ran 3).

## The cache-key decision (Phase 3a → closed, with a condition)

Key strategy under measurement: per-package
(`package-ci-<pkg>-{check,test,lint}-<env>`, with L2/browser/WSL-archive
deliberately reusing the `test` key).

**Measured restore result on the narrow run: MISS ("No cache found") on every
leg** — but not because the strategy is wrong. GitHub scopes caches to the
branch that saved them: PR runs save to their own PR scope and can read only
their base branch's scope, and the base branch (`docs/…-spec`) never runs CI
on push, so its scope stays empty. Every stacked PR therefore starts cold
pre-merge. **After the squash-merge, main's runs populate the shared base
scope every PR can read** — the strategy's real hit rate only exists
post-merge.

**Decision: keep per-package keys.** Grounds: (1) even fully cold, the
narrow run costs 71 runner-minutes — ~10× under the old system's warm-cache
cost for the same change, so the strategy cannot lose to the old baseline;
(2) per-package caches are right-sized for per-package jobs (the dmls test
cache is a fraction of an area cache); (3) the alternative (coarse shared
keys) reintroduces the churn the spec's open question warned about.

**Condition attached:** after the cutover merges, measure main-scope hit
rates and total cache size on the first few full-scope runs against the
10 GB repo quota (~315 possible keys). If eviction thrash materializes,
the recorded fallback is coarsening the `check`/`lint` keys first (they
share the most), before touching `test` keys. Until that data exists this
decision is closed on cold-path economics, not warm-path hit rates — stated
plainly so nobody mistakes it for a warm-cache validation.

## Exit condition

"A reorganized matrix that does not reduce observed runtime has not met the
objective." Observed: ~10× compute reduction on the narrow change, matrix
scheduling exactly the impacted closure, no directory fan-out. Met — with
the warm-cache follow-through owed post-merge as recorded above.
