---
ready: true
agent: codex/default
created: 2026-07-06T16:51:03
---

# Review 2 — Model Metadata Source Migration

## Verdict

Production ready.

The two blockers from review 1 have been addressed. `release_date` is validated
at the models.dev mapping boundary before it can enter generated metadata, the
committed artifact now carries only full `YYYY-MM-DD` release dates, and
`--dry-run` still fetches and validates models.dev instead of acting as an
enum-only bypass. I did not find remaining functionality gaps against the spec.

## Findings

No blocking findings.

## Requirement Verification

- Parsera retirement: no live Parsera references remain in `unchained-ai/gen`,
  `unchained-ai/lib`, CLI code, README/artifact docs, or the unchained-ai skill
  docs outside feature history. `models_dev.rs` is the enrichment source.
- models.dev mapping: Level 1 tests cover provider-key mapping, field mapping,
  pricing conversion, canonical capability mapping, modality parsing, unknown
  fields, and invalid/blank `release_date` handling.
- Matching ladder: Level 1 tests cover exact matches, identity matches across
  dash/dot spelling, date-pin preference, unpinned fallback, cross-provider
  refusal, ambiguity refusal, and no-match behavior.
- Anti-sunset guards: Level 1 tests cover thin responses, missing
  roster-critical providers, empty roster-critical providers, and the CLI
  dry-run path fetching and rejecting degraded models.dev data.
- Artifact schema v2: Level 1 tests cover schema v2 emission, committed
  artifact drift, committed schema drift, `release_date` serialization, and
  committed artifact `metadata.release_date` shape.
- Claudine-side artifact consumer: Level 1 tests cover v2 acceptance and v1 /
  mismatched schema rejection.
- Live regen backstop: committed drift tests pass, and the generated catalog
  sanity floors remain in place.

## Test Rigor Classification

All reviewed requirements are data generation, JSON artifact, metadata mapping,
and cross-area consumer-version behavior. They are non-terminal and do not
assert modifier-key visibility, hotkeys, paste, IME, mouse behavior, scrolling,
glyph widths, SGR styling, or real terminal encoder behavior.

Level 1 verification is therefore the appropriate level. Level 2 and Level 3
tests are not required for this feature.

## Verification Run

- `just test unchained-ai-gen` — passed, 86 tests.
- `just test claudine-gen` — passed, 83 tests run, 1 skipped.
- `(cd unchained-ai && just lint)` — passed.
- `(cd claudine && just lint)` — passed.
