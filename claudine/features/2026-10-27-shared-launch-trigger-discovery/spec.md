---
kind: feature
created: 2026-10-27
status: draft-spec
area: claudine
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    next_spec: string -> the `{date}-{name}` of the spec expected to follow this one; absent when none is planned
human_review: false
reviewed: false
related:
    - 2026-10-25-pipeline-work-reduction
---

# Share one trigger discovery across a launch's schema pre-validation

## Why this exists

Every prompt launch pre-validates its frontmatter against the effective schema
(`composition/schema/mod.rs::pre_validate_verdict`). That pre-validation
discovers the schema triggers on disk twice:

1. the optional-value scrub (`drop_invalid_optionals_with_origin`) calls
   `load_effective_schema`, which builds `DarkmatterSchemas` with
   `with_trigger_discovery()`;
2. the verdict itself then calls `load_effective_schema` again, which repeats
   the same discovery from the same document context.

Both discoveries read the same five schema locations plus Claudine's embedded
bundle for the same document, inside one call. Nothing between them can change
which triggers exist on disk except an outside edit during the call, and the
second result is what the verdict uses.

This is item D3 of `2026-10-25-pipeline-work-reduction`. That feature's rule
says an item that meets one of the measurement clauses gets its own dated
spec, and D3 met clause 1 when it was re-measured on 2026-10-27.

## Measured evidence (2026-10-27, macOS development host)

Counts and bytes come from `claudine/lib/benches/pipeline_work_alloc.rs`
(`-- D3`, `-- A4`) and the crate-private `pipeline_spike_a4` scenario at
revision `d7ea96a1f`. Both dependencies,
`2026-10-26-sequence-quadratic-frontmatter` and
`2026-10-25-shared-acyclic-definitions`, are merged. The host was loaded
when these ran; counts and bytes do not depend on load, but times do.

| Schema-root files | Pre-validation: `TriggerDiscovery` / `EffectivePreparation` | Pre-validation bytes | One discovery bytes |
| ----------------: | ----------------------------------------------------------- | -------------------: | ------------------: |
| 1 | 2 / 2 | 331,851 | 157,365 |
| 10 | 2 / 2 | 1,940,903 | 961,891 |
| 100 | 2 / 2 | 18,023,939 | 9,003,409 |

- **The repeat is half of a warm pre-validation.** At every size, one
  discovery is 47–50% of a pre-validation's bytes.
- **Share of anchor A4 (estimated, clause 1).** A4 is one sequence of 50
  steps (49 prompt tasks and a 200-leg group). Its in-process measurement
  launches no provider, so it runs no pre-validation. The estimate adds one
  pre-validation per prompt launch at the realistic size of 10 schema files
  (`claudine/schemas/` holds 11):
  - A4 measured total: 91,614,330 B preflight + 624,837,390 B group step =
    716,451,720 B;
  - 49 pre-validations: 49 × 1,940,903 B = 95,104,247 B;
  - removable: 49 × 961,891 B = 47,132,659 B, which is **5.8%** of 811.6 MB.

  The spike's 2026-10-26 figure was 5.0%. The share rose because the sequence
  fix shrank the rest of A4. The estimate leaves out each launch's
  composition and provider start, so a real run's share is lower. It is an
  estimate, not an attributed measurement, and it is recorded as one.
- Clause 2 is not met: at 100 schema files the repeat is 9 MB and a few ms
  per launch. Clause 3 is not met: peak stays flat.

## Behavior to build

One pre-validation discovers the triggers once and reuses that registry for
every preparation it makes.

- Discover once per `pre_validate_verdict` call, from the document's own
  context (`file_resolution_context.for_source(&source.resolved_path)`),
  with Claudine's embedded bundle appended exactly as today.
- Give the scrub and the verdict the same `TriggerRegistry` through
  Darkmatter's existing `DarkmatterSchemas::with_trigger_registry`. Each
  preparation still matches triggers against its own snapshot (the scrub
  matches the values before dropping, the verdict the values after), so a
  dropped optional can still change which triggers apply.
- Record the discovery's skipped-trigger findings in `trigger_findings`
  exactly once per pre-validation, before the authored `$schema` resolves,
  as today. Each trigger keeps one `[schema_validation]` warning per
  invocation.
- Do not cache across calls. A later launch, retry, resume, or proxy
  adoption discovers afresh, so a trigger file edited between launches is
  seen.
- `load_authored_schema` does no trigger discovery and is unchanged.

## Contracts that must not change

- The same triggers apply, in the same layer order, for every document,
  overlay, and caller-record combination as today.
- An invalid trigger is still skipped with `InvalidTriggerFinding`, still
  recorded before any unrelated error, and never becomes a
  `CompositionError`.
- An authored `$schema` that fails still reports `SchemaLoad` or
  `SchemaParse` with the triggers skipped before it.
- `Ok(None)` (no schema applies) and the scrub's dropped optionals are
  unchanged.

## Acceptance

1. An L1 work-count regression with `claudine`'s `work-counters` feature:
   one `pre_validate_schema` call over a document with a matching disk
   trigger records `TriggerDiscovery` **1** (today 2) and
   `EffectivePreparation` **2** (unchanged).
2. Behavior tests through the public pre-validation API cover: a trigger
   that matches only before the scrub drops an optional; an invalid trigger
   file, reported once; an authored `$schema` failure carrying the findings;
   a document with no matching trigger (`Ok(None)`); and a trigger file
   created between two launches being seen by the second.
3. The `pipeline_work_alloc` D3 rows are re-run and the implementation log
   records them beside the 2026-10-27 figures above.
4. `just test` and `just lint` pass in `claudine/` and `darkmatter/`.
5. The schema topic page that describes launch pre-validation states that one
   launch discovers triggers once.
