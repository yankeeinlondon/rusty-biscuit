---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-27-shared-launch-trigger-discovery/spec.md"
plan: "claudine/features/2026-10-27-shared-launch-trigger-discovery/plan.md"
implemented_by: "claude/sonnet"
started_phase: "1"
source_files_during_phase_1:
  - claudine/lib/src/composition/schema/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/tests/l1/invalid_trigger_findings.rs
  - claudine/lib/src/composition/schema/findings.rs
  - claudine/lib/benches/pipeline_work_alloc.rs
docs_updated_during_phase_2:
  - claudine/docs/topics/schemas.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - claudine/lib/src/composition/schema/mod.rs
  - claudine/lib/tests/l1/invalid_trigger_findings.rs
  - claudine/lib/src/composition/schema/findings.rs
  - claudine/lib/benches/pipeline_work_alloc.rs
documentation:
  - claudine/docs/topics/schemas.md
  - .claude/skills/claudine/SKILL.md
completed_phase: 3
implemented: true
packages:
  - claudine
decisions_for_review:
  - |-
    The plan's rematching example declared the optional `count` inside the trigger payload.
    Cleanup treats any property without authored-schema typing as required, so it never drops
    a trigger-declared value, and that example could not pass. I declared `count` in the
    document's own `$schema` instead (the trigger only requires `ticket`). Alternative: change
    cleanup to also drop optional trigger-declared properties, which is a behavior change
    outside this refactor. Redo cost if reversed: rewrite the three rematch tests and the
    schemas.md example.
  - |-
    The plan expected the caller-origin control (`input` naming a missing file) to return
    `UnresolvedFileReference`. With a trigger-declared `file(...)` property the same
    unchanged code returns `SchemaValidation` with problem `/input` (the typed error needs the
    authored schema's shape). The control asserts that instead; the positive case (file only
    at the launch anchor, so success proves caller origins survive the record replacement) is
    as planned. Alternative: declare `input` in an authored `$schema` to get the typed error.
---

# Implementation Log for 2026-10-27-shared-launch-trigger-discovery (3 phases)

## Phase 1

- Added private `discover_schema_owner` (scan, record findings, then configure launch
  context/fallback), `prepare_effective_schema` (match + effective_for) and a shared
  `schema_load_error` mapper in `schema/mod.rs`.
- `load_effective_schema` keeps its signature; it discovers, replaces caller records, prepares.
- `pre_validate_verdict` discovers once (error returns immediately), moves the owner through
  `with_caller_input_records` for original callers (cleanup) and cleaned callers (verdict).
- `drop_invalid_optionals_with_origin` replaced by `drop_invalid_optionals_with_schemas`
  (borrowed owner). Public `drop_invalid_optionals` discovers itself and returns the input
  unchanged on a failed scan.
- No Darkmatter change, no clone of the owner/registry, no signature change.
- Departure noted: standalone cleanup now discovers (and records findings) even when the
  frontmatter instance is not an object; previously it returned before discovery. Harmless.

## Phase 2

- Extended `lib/tests/l1/invalid_trigger_findings.rs` (14 tests, all pass with
  `just test-library --features work-counters invalid_trigger_findings::`):
  - counters: 1 discovery / 2 preparations for a matching trigger (repeated on the same
    source/context), for the rematching cleanup case, and 1 / 0 for the failed scan
    (feature-gated on `work-counters`)
  - cleanup removes the selecting value; final verdict stops applying the trigger; one
    `Frontmatter`/`PreValidation` drop; control returns `MissingProperties`
  - override variant: `Override` drop, `input` survives and resolves from the launch anchor
  - no-applicable-schema: source and overrides unchanged, zero drops, warning still recorded
  - malformed authored `$schema` gives `SchemaParse` (path, property `spec`) with findings recorded
  - first-reason retention across file edits; freshness sequence (create, replace, remove,
    malformed, repair) on one source/context with one ledger
  - invalid UTF-8 root file: `SchemaLoad` naming the file, no findings, standalone cleanup
    returns input unchanged, next call recovers
- Docs: `docs/topics/schemas.md` (call-lifetime subsection with Mermaid), `schema/findings.rs`
  module doc, `.claude/skills/claudine/SKILL.md` discovery paragraph, D3 bench comment.
- Debugging note: a temporary `eprintln!` in `drop_invalid_optionals_with_schemas` was added
  and removed; production code is unchanged in this phase.
- Gates (macOS, claudine area): `just test` 11,767 passed / 15 skipped, 0 failed (217.8 s);
  `just lint` clean. No new test uses a tier marker prefix, so tier coverage is unchanged.
  Dependent packages: no public signature changed (Phase 1), so no downstream run was needed.

## Phase 3

No source or documentation file changed in this phase; it measured D3 and ran the final gates.

### D3 allocation measurement (macOS only)

- Command (from `claudine/`): `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- D3`
- Base revision `c4915d621ac52d0c9e42f32fe39cd69062d84021`, with the uncommitted Phases 1-2 working-tree changes
  (no commit made for measurement identity). Profile: `bench` (optimized). Host: aarch64-apple-darwin.
- Toolchain: rustc 1.98.1 (48a229cea 2026-09-01), LLVM 22.1.8.
- Baselines are the specification's historical figures, not a fresh same-tree control run.

| Root files | Baseline warm bytes | New warm bytes | Saved | Saved % | Baseline counts | New counts |
| ---------- | ------------------- | -------------- | ----- | ------- | --------------- | ---------- |
| 1          | 331,851             | 171,606        | 160,245   | 48.3% | discovery 2 / prep 2 | discovery 1 / prep 2 |
| 10         | 1,940,903           | 976,132        | 964,771   | 49.7% | 2 / 2 | 1 / 2 |
| 100        | 18,023,939          | 9,017,650      | 9,006,289 | 50.0% | 2 / 2 | 1 / 2 |

One-discovery controls (baseline -> new, warm bytes): 157,365 -> 157,373; 961,891 -> 961,899;
9,003,409 -> 9,003,417 (counts 1/0 as expected). New pre-validation bytes decreased at all three
sizes, and remain within 14,233 / 14,233 / 14,233 B of the new control (the second preparation).
Timing is informational only (148 us / 442 us / 3.29 ms).

Raw rows:

```
SPIKE item=D3 size=schema-root-files=1/pre_validate_schema time_ns=148416 allocs=1042 bytes=171606 peak=76427 work=dm.TriggerDiscovery:1,dm.EffectivePreparation:2
SPIKE item=D3 size=schema-root-files=1/one-discovery time_ns=124375 allocs=848 bytes=157373 peak=76427 work=dm.TriggerDiscovery:1
SPIKE item=D3 size=schema-root-files=10/pre_validate_schema time_ns=442208 allocs=3016 bytes=976132 peak=86644 work=dm.TriggerDiscovery:1,dm.EffectivePreparation:2
SPIKE item=D3 size=schema-root-files=10/one-discovery time_ns=418917 allocs=2822 bytes=961899 peak=86644 work=dm.TriggerDiscovery:1
SPIKE item=D3 size=schema-root-files=100/pre_validate_schema time_ns=3292875 allocs=22660 bytes=9017650 peak=178763 work=dm.TriggerDiscovery:1,dm.EffectivePreparation:2
SPIKE item=D3 size=schema-root-files=100/one-discovery time_ns=3202584 allocs=22466 bytes=9003417 peak=178763 work=dm.TriggerDiscovery:1
```

### Gates (macOS only; no runtime evidence from other OSes)

- `just test` (in `claudine/`): 11,767 run, 11,767 passed, 15 skipped, 0 failed (299 s).
- `just lint`: clean (exit 0).
- No cross-OS check run: no OS-sensitive code changed in this phase; new tests are portable L1.
- Final diff review: one call-local owner, two matches/preparations, replaced caller origins,
  early finding recording, unchanged public signatures, no new cache/API/dependency/CI scope.
- No implementation departure beyond the two `decisions_for_review` items above. Dependent
  packages: no public signature changed, so no downstream run.
