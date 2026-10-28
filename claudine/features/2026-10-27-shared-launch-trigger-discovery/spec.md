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
human_review: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-27
review_iterations: 0
related:
    - 2026-10-25-pipeline-work-reduction
implemented: true
implemented_by: "claude/sonnet"
---

# Share one trigger discovery across a launch's schema pre-validation

## Why this exists

Before launching a prompt, Claudine checks its frontmatter against the
combined schema selected by triggers and its authored `$schema`. Within the
claudine package, [pre_validate_verdict](../../lib/src/composition/schema/mod.rs)
performs this check. Its normal successful path discovers the schema triggers
on disk twice:

1. the optional-value scrub (`drop_invalid_optionals_with_origin`) calls
   `load_effective_schema`, which builds `DarkmatterSchemas` with
   `with_trigger_discovery()`;
2. the verdict itself then calls `load_effective_schema` again, which repeats
   the same discovery from the same document context.

Both discoveries read the same five schema locations plus Claudine's embedded
bundle for the same document, inside one call. Nothing between them can change
which triggers exist on disk except an outside edit during the call, and the
second result is what the verdict uses.

The duplicate trigger-discovery finding in `2026-10-25-pipeline-work-reduction`
qualifies for its own specification under that feature's allocated-byte share
rule. The estimate below records why. The existing benchmark calls this
scenario `D3`; that label is used here only to identify its output rows.

## Measured evidence (2026-10-27, macOS development host)

Counts and bytes come from `claudine/lib/benches/pipeline_work_alloc.rs`
(the `D3` duplicate-discovery and `A4` sequence/group cases) and the
crate-private `pipeline_spike_a4` scenario at
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
- **Share of the sequence/group workload (estimated).** The benchmark's
  `A4` case is one sequence of 50
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
- The parent feature's scaling rule is not met: at 100 schema files the
  repeat is 9 MB and a few milliseconds per launch. Its unbounded-memory rule
  is not met either: peak live memory stays flat.

## Behavior to build

One pre-validation call discovers triggers once and reuses the loaded registry
for the optional-value cleanup and the final verdict. A registry contains
loaded trigger rules and their resolved payloads; it is not the combined
schema selected for a particular frontmatter value.

### Ownership and lifetime

- Discover once at the start of `pre_validate_verdict`, from the document's own
  context (`file_resolution_context.for_source(&source.resolved_path)`), with
  Claudine's embedded bundle included exactly as today.
- Keep that discovered registry in one call-local
  [DarkmatterSchemas](../../../darkmatter/lib/src/markdown/schemas/mod.rs)
  object from the darkmatter package, which owns the registry. Reuse that
  owner for both effective-schema preparations, moving it when configuration changes are needed and borrowing
  it for preparation. A private Claudine helper can separate discovery from
  preparing the effective schema. No new public API or dependency is needed.
- Do not clone the registry or the schema owner between preparations.
  Darkmatter's [TriggerRegistry](../../../darkmatter/lib/src/markdown/schemas/triggers/discovery.rs)
  owns vectors of loaded rules and resolved payloads; cloning it would retain
  much of the allocation cost this change is intended to remove.
- Release the owner when the call returns, including on error. Do not store it
  on the invocation, document epoch, prepared result, or in a global cache.
  Every later call discovers afresh, including calls made for retry, resume,
  proxy adoption, or interactive rechecking. File creation, modification,
  removal, and repair between calls must be visible to the next call.

> **Reader's note:** The draft proposed passing the registry to two new
> schema owners with Darkmatter's `with_trigger_registry` method. That method
> takes ownership, and the registry's clone copies loaded data. Reusing one
> configured owner achieves the same discovery-sharing goal without requiring
> a Darkmatter API change or copying the data being saved.

### Prepare twice, match twice

The cleanup and final verdict must still prepare their own effective schema.
They use the same discovered trigger contents, but different frontmatter:

1. Build the effective instance and caller-origin records from the original
   source and overrides. Prepare its effective schema and perform the existing
   optional-value cleanup, checking all declared properties without a
   launch-phase projection, as today.
2. Rebuild the effective instance and caller-origin records from the cleaned
   source and overrides. Replace the owner's caller records with these records
   before preparing the final effective schema. Preserve the existing
   mode-dependent validation phase for the verdict.

For example, a trigger can match the presence of `count` and declare
`count: number(optional)`. With `count: wrong`, cleanup drops `count`.
The final preparation must stop selecting that trigger. Reusing the first
combined schema would incorrectly keep its declarations active.

Keep document-relative discovery separate from launch-relative resolution:
after discovery, configure the same launch file-resolution context and optional
fallback directory as today. Caller-supplied file patterns still use each
caller's origin. Neither a value overlay nor a caller's launch directory may
change the document used for trigger path matching or replace its authored
`$schema`.

### Findings and failures

- Record the successfully completed discovery's skipped-trigger findings once
  per pre-validation, before either preparation resolves the authored
  `$schema`. Keep the invocation's existing finding record and delivery
  mechanism: warnings remain ordered by first observation and deduplicated by
  full trigger-source identity, with the first reason retained. A later call
  may discover again without showing the same warning twice.
- Distinguish an invalid individual trigger from a failed discovery.
  Darkmatter skips an invalid envelope, payload, or import and returns an
  `InvalidTriggerFinding`; valid triggers continue to apply. Failure to read
  a schema root or enumerated file, a case-fold collision, or an invalid
  embedded-bundle structure is an infrastructure error. Preserve the current
  schema-error translation and return that error; never substitute an empty
  registry or report it as a skipped trigger.
- Cleanup retains its current best-effort handling of effective-schema load
  errors. The final preparation reports an authored `$schema` failure through
  the existing `SchemaLoad` or `SchemaParse` mapping, with already-recorded
  trigger findings still available. Discovery failure ends this call without
  retrying the same scan; a subsequent call can try again.

### Deliberate consistency change

An outside edit between cleanup and verdict no longer changes the trigger
rules within that one call. Both preparations use the discovery result from
the start of the call, including loaded trigger payloads and imports. This
avoids cleaning values under one set of rules and judging them under another.
Discovery itself is not an atomic filesystem snapshot, and the authored
`$schema` is still resolved by each preparation; this feature does not promise
atomic reads of all schema files. No filesystem watcher, locking, or new
invalidation mechanism is required.

## Contracts and scope

For inputs that remain unchanged during the call:

- The same triggers apply in the same layer order and with the same origins.
  Preserve root order, filename shadowing (including invalid files shadowing
  embedded triggers), embedded fallback, and authored-schema precedence.
- Required values, Claudine-owned controls, eager file values, templated
  values, and literal data overrides retain their existing cleanup and
  validation rules. Preserve dropped-value metadata and source attribution,
  as well as the existing preference for retired-grammar diagnostics.
- No applicable schema remains a successful pre-validation with unchanged
  source and overrides. `Ok(None)` is the internal effective-schema loader's
  result; the public API returns `Ok(PreValidatedSchema { ... })`.
- An authored `$schema` naming a trigger file still fails. Skipping that
  trigger during discovery does not make it a valid document schema.

Standalone cleanup and schema-status calls still discover for themselves.
Keep their public signatures and finding behavior unchanged; route standalone
cleanup through the same private cleanup implementation with its own owner.
`load_authored_schema`, which resolves only the author's declared schema,
continues to do no trigger discovery.

This feature changes only the two preparations within one pre-validation
call. Composition, transcluded documents, shell approval, post-shell validation,
sequence-wide checks, and other preparations keep their existing discovery
boundaries. A whole launch can still discover triggers more than once.
Darkmatter's public ownership and caching contracts remain unchanged; this
feature should be implementable entirely in Claudine.

## Acceptance

1. Extend the existing L1 work-count coverage using claudine's `work-counters`
   feature. A successful public `pre_validate_schema` call with a matching
   disk trigger records `TriggerDiscovery` **1** (today 2) and
   `EffectivePreparation` **2** (unchanged). Scope the counter observation to
   this call, excluding fixture and source-context construction. The existing
   `just test` recipe enables this feature. Error exits need not perform two
   preparations.
2. Use isolated, temporary schema roots and explicit resolution contexts for
   public pre-validation behavior tests. Extend existing tests where they
   already prove these contracts:

   | Case | Observable result |
   | --- | --- |
   | Trigger matches only before an invalid optional is removed | Cleanup records the drop; the final verdict no longer enforces that trigger's other required properties. |
   | A caller override is cleaned away | The returned override and drop attribution are correct; final matching uses the cleaned values and remaining callers' origins. |
   | Invalid trigger alongside a valid trigger | Valid rules still apply; the invalid trigger is recorded once and delivered once across repeated checks. |
   | Authored schema cannot load or parse | The existing typed error is returned and findings remain available before error delivery. |
   | No authored schema and no matching trigger | Public pre-validation succeeds with unchanged source and overrides. |
   | Trigger created, changed, removed, or repaired between calls sharing a context | The next call sees the new rules or absence; no invocation-level cache masks the edit. |

   Keep existing coverage for disk/embedded precedence, caller-relative file
   patterns, authored references to trigger files, mode-dependent validation,
   and standalone cleanup/status findings passing. Add focused coverage only
   where the refactor could escape those assertions. Use the shared test
   toolkit; no provider, terminal, browser, fixed sleep, or live user schema
   directory is needed.
3. Re-run only the existing `pipeline_work_alloc` duplicate-discovery (`D3`)
   rows for 1, 10, and 100 schema-root files on one host. Record allocated
   bytes and work counts beside the baseline above in the implementation log,
   with revision, profile, and toolchain. The regression counter proves the
   removed scan; the byte comparison checks that copying the registry has not
   consumed the saving. Do not introduce a timing threshold or another spike:
   the existing figures already justify the work, and no OS-specific cost is
   known. No additional hosts or workloads are required for this measurement.
4. Run `just test` and `just lint` in `claudine/`. Run the corresponding
   `darkmatter/` checks only if implementation changes Darkmatter code; the
   existing API should make that unnecessary. Record any resulting departure
   from this ownership design in the implementation log.
5. Update Claudine's [schema topic page](../../docs/topics/schemas.md) and
   relevant source comments, including the finding record's discovery
   description, to say that **one pre-validation call** shares one discovery.
   Preserve the distinction from a whole launch. Update the claudine skill
   if it describes the affected workflow. Current documentation must describe
   the behavior directly without linking to this specification.

## Open Questions

None. The review resolves the sharing lifetime, ownership cost, error handling,
and documentation scope using the existing APIs. A broader cache across
preparations or launches is outside this feature.
