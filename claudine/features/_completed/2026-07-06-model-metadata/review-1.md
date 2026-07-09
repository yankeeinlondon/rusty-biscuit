---
ready: false
implemented: true
agent: codex/default
created: 2026-07-06T16:19:10
---

# Review 1 — Model Metadata Source Migration

## Verdict

Not production ready.

The implementation lands the major shape of the migration: Parsera is removed,
models.dev is wired, schema v2 is emitted, Claudine's generator rejects v1, and
the relevant generator test suites pass. The remaining blockers are contract
gaps in generated output and guard coverage.

## Findings

### High — `release_date` values violate the serialized `YYYY-MM-DD` contract

Spec references:

- `spec.md:115` requires models.dev `release_date` to be preserved as a source
  date string in `YYYY-MM-DD` form.
- `spec.md:194-196` requires artifact schema tests to prove serialized
  `release_date` values are `YYYY-MM-DD`.

Implementation:

- `unchained-ai/gen/src/models_dev.rs:222-233` copies `model.release_date`
  directly into `ProviderModelMetadata` with no validation or normalization.
- The committed generated metadata contains month-only dates, for example
  `unchained-ai/lib/src/rigging/providers/models/metadata_generated.rs:4071`
  emits `release_date: Some("2026-01".to_string())`.
- The committed artifact also contains month-only dates, for example
  `unchained-ai/artifacts/models-catalog.json:2428` emits
  `"release_date": "2025-04"`.
- `unchained-ai/gen/tests/catalog_drift.rs:29-79` proves the artifact is
  byte-stable and schema-v2-shaped, but does not validate date format, so the
  invalid values are currently blessed by drift tests.

Impact:

Consumers were told they can treat `release_date` as a release date string in a
specific format. The committed v2 artifact already violates that contract, so
date parsing, sorting, and downstream chronology logic can fail or silently
misorder releases.

Recommendation:

Validate `release_date` at the models.dev mapping boundary and at artifact
emission. Either reject non-`YYYY-MM-DD` source values loudly during generation
or omit them with an explicit report line if partial dates are acceptable. Add
an offline fixture test and an artifact drift-adjacent test that scans every
serialized `metadata.release_date` for the exact format.

### Medium — `--dry-run` is an enum-only metadata-source bypass

Spec references:

- `spec.md:167-182` says metadata-source failures must fail generation loudly
  and that enum-only generation is not allowed to bypass metadata-source
  failures.
- `spec.md:191-193` calls for no-degraded-mode tests that prove degraded
  metadata-source conditions fail generation loudly.

Implementation:

- `unchained-ai/gen/src/main.rs:296-313` skips the models.dev fetch entirely
  when `--dry-run` is set and sets `models_dev_index` to `None`.
- The later metadata loop at `unchained-ai/gen/src/main.rs:356-365` therefore
  generates output without models.dev enrichment in dry-run mode.
- Existing no-degraded-mode tests exercise `validate_models_dev_index` and
  roster-critical validation helpers, but they do not exercise the generator
  command path that bypasses the fetch in dry-run mode.

Impact:

Dry-run output is not representative of a real generation and does not test the
anti-sunset guard. This is lower risk than a write-mode bypass because it does
not update committed files, but it weakens the main operational check people
use before regenerating.

Recommendation:

Make dry-run fetch and validate models.dev by default, while still suppressing
file writes. If offline dry-run is needed, add an explicit fixture-backed flag
or mode whose name makes the degraded behavior visible, and test both paths.

## Test Rigor Classification

- models.dev field mapping, pricing conversion, provider-key mapping,
  capability mapping, matching ladder, guard helpers, schema-v2 emission, and
  Claudine artifact version checks are all non-terminal, non-keyboard
  requirements. Level 1 tests are the appropriate verification level.
- No reviewed requirement asserts terminal rendering, modifier-key visibility,
  hotkey behavior, paste, IME, mouse, scrolling, or real terminal encoder
  behavior. Level 2 and Level 3 tests are not required for this feature.
- The strongest present verification for the relevant requirements is Level 1:
  `just test unchained-ai-gen` and `just test claudine-gen`.
- Coverage gap: the Level 1 suite does not validate the `YYYY-MM-DD`
  `release_date` contract across generated metadata or the committed artifact.
- Coverage gap: the Level 1 suite does not test the generator command path for
  degraded metadata-source failure in `--dry-run` mode.

## Verification Run

- `just test unchained-ai-gen` — passed, 82 tests.
- `just test claudine-gen` — passed, 73 tests run, 1 skipped.
