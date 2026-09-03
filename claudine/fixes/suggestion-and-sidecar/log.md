## Implementation of Review Findings #1

> **started at:** 2026-09-03T21:22:44+01:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/suggestion-and-sidecar/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'budget the real walker's depth-zero root item' at 21:23:26
        - GitNexus reported a CRITICAL upstream impact for `repository_basename_suggestions`: one direct caller, 12 affected symbols across five modules, and no indexed execution flows; the orchestrator was warned before implementation proceeded
        - GitNexus reported a HIGH upstream impact for `collect_repository_suggestions`: one direct caller, six affected symbols across three modules, and no indexed execution flows; the orchestrator was warned before implementation proceeded
        - replaced the pre-budget root filter with a shared `SuggestionWalkItem` stream so the real walker's root, errors, and ordinary entries are all counted by the collector's 20,000-item bound
        - replaced the pure boundary test with an instrumented production-collector seam that includes a root item, an error, and ordinary entries, and panics if item 20,001 is consumed
