---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-26T21:03:37-07:00"
spec: 2026-10-25-schema-diagnostic-provenance/spec.md
implemented: false
description: "A **feature** review of `2026-10-25-schema-diagnostic-provenance/spec.md`"
feature: 2026-10-25-schema-diagnostic-provenance/review-1.md
---

# Review 1: Schema Diagnostic Provenance

The feature is production ready. No blocking defects or observations remain.
This is the first implementation review; there are no previous findings to
carry forward.

## Course of action and decisions

Two independent reviewers first examined the spec and implementation log,
including its empty `decisions_for_author` list. Neither found a decision
requiring human review. The review therefore continued with the implementation
comparison.

The implementation's recorded choices are retained:

- Conflict participants carry actual values immediately, including schemes
  before a failing intersection. Bare alphanumeric options preserve the
  existing declaration origin. Controls share one escaping rule, and
  number-looking string defaults use the shared numeric parser to decide
  quoting.
- Internal failures carry structural paths; public located failures box the
  underlying error and can convert back to an ordinary schema error. Paths
  remain relative to their parsing root and acquire an envelope prefix only
  when needed. Cross-arm overload checking retains its message-only adapter.
- Standalone outer errors remain with the classifier. Alias uses can receive
  a whole-value range without inventing a token in the alias's target.
  Root-union mapping definitions receive definition diagnostics; invalid
  arms themselves retain shape diagnostics.
- Continuation completion uses the decoded scalar map, accepts trailing
  same-line spaces and tabs, and rejects empty holders. Meta-type acceptance
  remains with source-free parsing; source-aware parsing only locates a
  reported failure. Unmatched conversion errors keep the existing fallback.
- The corpus explicitly checks the unsupported multiline quoted presentation
  rather than silently skipping it. The shipped CLI failure test references
  its broken schema directly, since a broken trigger follows the separate
  skip-with-warning contract. Public API documentation stays on the existing
  definition page, with a fallback example matching the tested presentation.

The classification and trailing-whitespace interpretations, alternatives,
reasoning, and downsides are recorded under “Phase 4 review — iteration 1” in
[the implementation log](implementation-log.md). These are settled choices,
not requests for author confirmation.

The log did omit the full Claudine test gate required by acceptance criterion
7, substituting filtered runs. This review ran the required `just test` in
`claudine/` successfully, closing that verification gap. Intermediate
documentation handoffs were completed in Phase 4. No requested implementation
scope remains deferred or blocked on a decision.

## Implementation and verification

The review covered the participant constructors and consumers, shared
constraint formatting, parser recursion and source projection, standalone
recognition, DMLS diagnostic ranges and related locations, dependency
watching, continuation completion, and updated documentation.

| Spec requirement | Verification level and evidence |
| --- | --- |
| Conflict values and unchanged merge rules | L1 library tests in `schemas_named_type_merge`: bounds, defaults and numeric equality, item/container reconciliation, canonical options including the empty set, multi-hop imports, and effective schemes. |
| Consistent, safe messages | L1 serializer and error-renderer tests check scalar identity, compact JSON defaults, controls, terminal markup, and the 160-scalar limit. DMLS protocol tests check participant values, effective-scheme wording, related locations, and watched files through wrappers. |
| Located definition failures | L1 `schemas_source_projection` tests cover both envelopes, literal keys, nested mappings, sequence indices, a second suggestion across individually valid arms, structural failures, aliases, unsupported presentation, and semantic/source mismatch. The former candidate-string search is removed. |
| Inline and meta-type parity | L1 DMLS protocol sessions check inline mappings and sequence members, LF/CRLF, non-BMP text, negotiated UTF-8/UTF-16 positions, referenced-file ownership, and unchanged union acceptance. |
| Anchored continuation completion | L1 scalar/cursor tests and DMLS protocol sessions compare loaded and incomplete buffers, anchors/tags, replacement ranges, blank folding, comments, quoted/block values, aliases, dedented siblings, sequence indentation, and passivity. |
| Shipped consumers and passive parsing | L1 corpus tests check shipped standalone artifacts and at least 500 inline declarations. Shipped-schema CLI tests prove normal validation wiring for a grammar failure and an imported bound conflict, including failure status and no published output. |

These parser and LSP requirements need L1 verification: they concern returned
data and protocol positions, not a terminal's input encoding or rendering.
No new keyboard-interaction or terminal-style requirement needs L3 or a new
L2 case. The separately requested existing L2 regression gate remains part
of the implementation evidence.

The shared standalone YAML loader's duplicate-key, invalid/trailing-content,
and second-document cases are exercised through the public locator and
classifier. Definition shape tests cover scalar and collection failures;
null retains its existing null-type meaning, an empty mapping remains a valid
object, and an empty union is rejected. The code-smell check found no new
permissive defaults: optional ranges are projection results, `.ok()` retains
the helper's documented outer-error/fallback behavior, and the unsupported-key
filter is backed by the mapping-length check for non-string keys.

Checks independently run during this review:

- `just test` in `darkmatter/`: **12,669 passed**, 10 skipped; 150.014 seconds.
- `just test` in `claudine/`: **11,654 passed**, 10 skipped; 484.048 seconds.
- `just check-tier-coverage darkmatter`: **0 stranded tests**. Changed tests
  are declared in the consolidated targets and selected by L1. The new CLI
  and DMLS regression cases each completed below two seconds in this run.

The final implementation log additionally records passing `just lint` and
`just test-l2` in `darkmatter/`, with terminal harnesses that never take focus.
Those gates were not repeated during this review. Cross-OS evidence is left
to CI and does not block this review's readiness decision.

## Findings

None.

## Observations

None.
