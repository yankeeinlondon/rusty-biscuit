---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-18T20:15:49-07:00"
spec: "2026-10-18-dmls-cache-optimization-follow-ups/spec.md"
fix: "2026-10-18-dmls-cache-optimization-follow-ups/review-1.md"
implemented: false
description: "A **fix** review of `2026-10-18-dmls-cache-optimization-follow-ups/spec.md`"
ready: true
findings: []
observations: []
recurrence: false
human_in_the_loop: true
human_resolved: false
human_review: true
human_review_items:
    - |-
        Confirm whether recursive `%` file searches should remain outside this repair. These searches can select files several folders below a search root. The implementation follows the plan's recommended deferral: a newly created, higher-priority file deep in that search can still leave the cached schema and editor behavior unchanged.

        The spec explicitly leaves this choice open, and the plan's closing author item asks for confirmation. The implementation log reports completion but does not record author approval of this scope decision.

        Choose one:

        - **Defer recursive search tracking (recommended).** Keep this repair focused on nonrecursive references and retain the documented recursive limitation. This avoids recursive filesystem work on cache hits; recursive searches retain their existing stale-result risk until a separate repair.
        - **Include recursive tracking now.** Expand the design to record descendant searches and watch the relevant locations. This addresses the broader stale-result problem but needs a bounded performance and watch-coverage design before implementation.
        - **Bypass caching for recursive dependencies.** This avoids library payload-cache reuse for those schemas, at a performance cost. It still requires an editor invalidation and watching solution, so bypassing the cache alone is incomplete.
has_blocked_findings: false
blocked: false
review_scope: decision-only
implementation_review_performed: false
---

# Review 1: author scope decision required

This review stops before comparing the implementation with the specification.
Production readiness has **not been assessed**. Two independent agents reviewed
the specification and implementation log for decisions and deferred work. The
log alone reports no unresolved decisions, but the specification and plan leave
one meaningful scope choice awaiting the author: recursive `%` references.

The `ready: true` value follows the requested rule that only a high finding makes
it false; this decision-only review has no implementation findings. It is not a
production-readiness endorsement. The spec's `completed` flag follows the
requested metadata rule tied to `ready`; `human_in_the_loop: true` and
`human_resolved: false` record the outstanding stop before implementation review.

## Decision for the author

The repair records file locations tried before a schema dependency was selected.
That lets the cache and editor notice when a previously missing, preferred file
appears. Recursive `%` references search through descendants rather than probing
only a bounded sequence of individual files. Watching or listing just the search
roots cannot detect every new match below them.

For example, a recursive search that currently selects a fallback file can keep
using it after a preferred file appears under a nested folder such as
`schemas/team/new/types.yaml`. The plan chose to leave that existing limitation
in place and document it. The specification's **Open Questions → Should recursive
local references be repaired in the same change?** requires a separate decision;
the plan's **Open items for the author** explicitly requests confirmation. No
author ruling is recorded in the reviewed material.

Recommend accepting the documented deferral. It matches the concrete nonrecursive
defect this repair targets and avoids expanding cache validation into recursive
filesystem scans. The alternatives and their costs are recorded in
`human_review_items` above. After the author records a ruling, implementation
review should continue against that agreed scope.

## Other implementation decisions

These choices do not independently require human review:

- The biscuit-file package exposes normalized probe paths through one additive
  API, keeping path normalization in its existing owner.
- Existing namespace-cache behavior already replays the relevant searches, so
  the implementation avoided redundant cache bookkeeping.
- Editor invalidation follows candidate paths and ancestor-folder events,
  while unrelated sibling-file events retain cached results.
- Absent schema roots and candidate folders use the same nearest-existing-
  ancestor watch strategy. Failed filesystem observations use a private test
  seam to avoid operating-system-dependent permission tests.

The spec's separate question about an `unscheduled` status is obsolete: its
frontmatter already says `finalized-spec`. It needs no author design decision.

## Findings

None recorded. Code comparison, acceptance-test verification, and test execution
were intentionally not performed on this human-review route. No earlier
implementation review exists for this fix, so there are no carried findings.

## Observations

None recorded.
