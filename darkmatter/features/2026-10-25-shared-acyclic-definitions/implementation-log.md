---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/features/2026-10-25-shared-acyclic-definitions/spec.md"
plan: "darkmatter/features/2026-10-25-shared-acyclic-definitions/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/shared_acyclic_definitions.rs
    - darkmatter/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - claudine
human_review: false
message_to_agent: |-
    Phase 1 recorded the baseline only; no production code changed. Before
    sizes: claudine 1,403,266, sequence 1,618,148, both 1,844,516 bytes
    (compact JSON); M must reach <= 1,520,000 for a budget below 2,000,000.
    The largest duplicates are the `expression` action category (6 copies of
    128 KB, reached through `entry` in each family's `stack` and `item`) and
    the `loop` type (3 copies of 218 KB in `both`). Two new behavior probes
    must keep passing unchanged after sharing:
    darkmatter/lib/tests/l1/shared_acyclic_definitions.rs and
    claudine_schemas_embedded::a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written
    (it pins union explanations, which `$ref` arms could move). The timing
    scaffolding was removed from the suite; its exact source is in the log for
    Phase 4. Use absolute paths in shell commands: this host's CDPATH sends a
    relative `cd darkmatter/lib` into another worktree.
---

# Implementation Log for 2026-10-25-shared-acyclic-definitions (4 phases)

## Phase 1

Phase 1 is Wave 1 of the plan: capture baseline semantics and measurements
before any resolver, trigger, library, bundle, or snapshot change. No
production code changed in this phase.

### Baseline revision and working tree

- Baseline revision: `391569fc6` (`planning(darkmatter): record Phase 2 and 3
  close for schema-diagnostic-provenance`), branch `fix/pipeline-finalization`.
- The working tree already held uncommitted edits from
  `2026-10-25-schema-diagnostic-provenance` when this phase began (its plan,
  log, spec, review, two darkmatter docs, two skill pages,
  `darkmatter/cli/tests/l1/schema_validate.rs`,
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`) and an edit to this
  feature's `plan.md` (the Decision 4 landing-order refresh). They are not
  this phase's work and were left untouched. This phase's own files are listed
  in the frontmatter.
- `2026-10-26-dmls-frontmatter-typing-allocation` is present on this base
  (`darkmatter/dmls/benches/pipeline_work_a3.rs`,
  `completion::alternative_fingerprint`), so the landing-order check passes.

### Input inventory (unchanged frozen inputs)

Git tree hashes at the baseline, each with no working-tree change:

| Input | Tree |
| ----- | ---- |
| `darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs` (frozen corpus inputs) | `bdd4515083e0` |
| `darkmatter/lib/tests/fixtures/recursive_schema_golden/golden` (104 outputs: 90 JSON, 14 digest) | `4721b1d91761` |
| `darkmatter/lib/tests/fixtures/recursive_schema_frozen` (spike fixtures) | `42ba369c7aef` |
| `claudine/schemas` (shipped bundle) | `98a7d7e1e2a5` |
| `claudine/lib/tests/fixtures/claudine-schemas` (AC5 + deep corpus) | `9b3a459cf031` |
| `claudine/lib/tests/fixtures/sequence-schema` | `a9a36e265a91` |

The DMLS bench reads its own frozen bundle,
`darkmatter/lib/benches/fixtures/claudine-schemas/`; the metadata extraction
does not touch it, so only resolver sharing moves the bench.

Affected shipped artifacts: `claudine/schemas/{claudine,inline-compose,
lifecycle,action,linking,sequence,sequence-types}.yaml` (all lowered through
the resolver), `darkmatter/schemas/partials/*.yaml`, and
`content-policy/schemas/content-policy.yaml`. The golden manifest's 104
entries cover them; the 14 digest entries are the megabyte-scale ones
(Claudine lifecycle/sequence families, effective schemas, embedded baseline).

Representation-sensitive assertions to revisit in Phase 2–4:
`recursive_schema_golden.rs` (byte equality for all 104 entries; its module
doc promises acyclic byte stability), the budget test
`embedded_trigger_payloads_resolve_within_two_megabytes` (asserts which
definitions exist and that unrolled names do not), and the
`recursive_schema_*` tests that inspect `$defs`/`$ref` shapes directly.

Public consumers of `TypeExpr::Ref` / `DefinitionTable` (Phase 3 audit list):
darkmatter lib `schemas/{mod,catalog,completion,detect,file_match,
frontmatter_shape,phase}.rs`, `schemas/simplified/{convert,definitions,lint,
mod,query,serialize,source,standalone,subtype,types}.rs`,
`schemas/triggers/{assemble,matcher}.rs`, `schemas/resolve{,/graph,
/recursive}.rs`, `lifecycle/schema_projection.rs`,
`compose/context/catalog.rs`, `compose/expression/{call_check,
call_binding/translate,scope/projection,scope/shape}.rs`,
`compose/shell_expansion/template/typing.rs`; darkmatter cli
`commands/schema/assignment.rs`; dmls `overlay/{named_types,schema,
suggestions}.rs`, `providers/frontmatter.rs`; claudine
`lib/src/composition/schema/{classify,supplied}.rs`,
`gen/src/schema_compat.rs`.

### Semantic baseline: new probes

Two new tests pin public results that must survive sharing. Both pass on the
baseline resolver (every acyclic use inlined).

- `darkmatter/lib/tests/l1/shared_acyclic_definitions.rs` (6 tests, ~40 ms
  each). Fixture: `entry` (`{ name: string(required), size: number(min(0)) }`)
  imported at `first` (optional), `second` (`required`), `items` (`entry[]`),
  and twice inside `bundle` (`entries: entry[](required)`, `primary`); `slug`
  (`alphanumeric(allow-dashes)`) at `label: slug(min(3))` and `tag`. The
  schema is a referenced file, so each property's origin is
  `ReferencedFile(schema.yaml)`. Recorded tuples
  `(code, instance path, line, column, description)`:
  - two uses: `(ConstraintViolation, /first/size, 4, 9, "the entry size")`,
    `(TypeMismatch, /second/name, 6, 9, "the entry name")`;
  - nested list item and bundled uses:
    `(TypeMismatch, /items/1/size, 7, 11, "the entry size")`,
    `(MissingRequired, /nest/entries/1, 11, 7, "the entry name")`,
    `(TypeMismatch, /nest/primary/name, 13, 11, "the entry name")`;
  - presence/null: optional `first` missing `name` → `MissingRequired` at
    `/first` (property `name`) plus the use-required `second` at the root;
    `first: null` and `nest: null` accepted; `second: null` rejected at
    `/second`; `items: [null]` rejected at `/items/0` (an optional list does
    not make its items nullable);
  - primitive alias: `label: ab` fails `ConstraintViolation` at `/label`,
    `tag: ab` passes (the narrowing stays at its use);
  - conversion: numeric strings convert to numbers at every use, list item,
    and bundled use (`EffectiveSchema::coerce_frontmatter_with_pending`).
  - Observation: a YAML number where a `string` is declared (`name: 7`) is
    accepted by validation, so the type-mismatch probe uses a mapping.
    `MissingRequired` problems carry the missing property's description.
- `claudine/lib/tests/l1/claudine_schemas_embedded.rs::a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written`
  (~1 s, embedded bundle only, no disk). Every problem below is
  `ConstraintViolation`, declared by `embedded:claudine/claudine.yaml`:
  - `start: [inf0]` + `finalize: [say, inf0]` → `/start/0` (2:5) and
    `/finalize/1` (5:5), explanation "a zero-argument verb written as a bare
    list item";
  - a conditional body item → `/success/0/then/1` (5:9), same explanation;
  - a bundle beside an unknown verb → `/start/0` (2:5);
  - `initialize: [{ shell: ls }]` → `/initialize/0` (2:5); the same item under
    `cancelled` is accepted;
  - a loop action missing `prop` → `/loop/action` (3:5), "an iteration action
    such as increment(iteration)".

  The union explanation each use reports is part of what sharing must keep:
  `entry` and the categories become `$ref` arms, and explanation selection
  reads through them.

### Acceptance families and their test owners

| Spec acceptance family | Owner (existing or new) |
| ---------------------- | ----------------------- |
| 1 Selection and counting | New in Phase 2 (resolver unit tests; extend `shared_acyclic_definitions.rs` for representation). Cycle admission: `recursive_schema_types.rs`, `recursive_schema_conversion.rs` (unchanged). |
| 2 Constraints and fallback | Phase 2, beside `schemas_named_type_merge.rs`, `schemas_union_postfix.rs`, `meta_schema_reference_graph.rs` (32-hop chain). |
| 3 Acceptance and diagnostics | New: `shared_acyclic_definitions.rs`, `a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written`. Existing: `claudine_schema_shipped_corpus.rs` (67, both gates), `claudine_schemas.rs` (AC5 + deep corpus, `depth6_wrong_verb_is_positioned_at_the_verb`), `sequence_schema.rs`, `container_schema_recursion.rs`, `recursive_schema_reporting.rs`, `recursive_schema_frozen_fixture.rs`, `shipped_schema_corpus.rs`. |
| 4 Consumer and merge parity | `recursive_schema_typed_consumers.rs`, `recursive_schema_completion.rs`, `recursive_schema_merge.rs`, `schemas_union_typed_consumers.rs`, `schema_cache_validity.rs`; DMLS `recursive_schema_editor.rs`; extended in Phase 3. |
| 5 Claudine metadata | `claudine_schemas.rs` (parity), `claudine_schemas_embedded.rs` (embedded/shadowing/inventory); Phase 2. |
| 6 Representation and headroom | `recursive_schema_golden.rs`, `embedded_trigger_payloads_resolve_within_two_megabytes`; Phase 4. |
| 7 Final checks | Area gates; Phase 4. |

### Focused commands (all passing on the baseline)

```sh
cargo nextest run -p darkmatter --test l1 -E 'test(shared_acyclic_definitions)'
cargo nextest run -p claudine --test l1 -E 'test(a_shared_lifecycle_helper_reports)'
```

### Measurements (before)

Host: macOS (Darwin 27.2.0, Apple silicon), worktree
`fix-pipeline-finalization` at `391569fc6`. Build mode: the nextest `test`
profile (unoptimized + debuginfo). Inputs: exactly the budget test's three
cases through the embedded Claudine bundle with no disk schema root
(`Bare`), serialized with `serde_json::to_string` (compact UTF-8, includes the
baseline and `$defs`).

Stage boundaries: one `ValidationTimings::record` around trigger discovery
(`Bare::schemas`) plus `DarkmatterSchemas::validate`; discovery resolves the
trigger payloads, so their resolution lands in `resolve`. Build and process
startup are outside the recording; nextest runs each case in its own process,
so parsed-schema, baseline, and validator caches start cold.

| Case | Compact bytes | discover ms | resolve ms | compile ms | validate ms |
| ---- | ------------- | ----------- | ---------- | ---------- | ----------- |
| `claudine.yaml` (`start: []`) | 1,403,266 | 7.9 | 387.7 | 275.5 | 0.5 |
| `sequence.yaml` (`sequence: []`) | 1,618,148 | 7.9 | 392.1 | 270.1 | 0.1 |
| `both` | 1,844,516 | 9.2 | 529.1 | 444.4 | 0.6 |

Maximum before: `M_before = 1,844,516` (the draft's 1.83 MB estimate was
close). To reach a budget below 2,000,000 the Phase 4 maximum must be at most
1,520,000 bytes, a reduction of at least 17.6% on `both`.

Largest repeated helpers (identical compact subtrees larger than 2 kB,
maximal ones only; `waste` = (copies − 1) × size), `both` case:

| Copies × bytes | Wasted bytes | Authored target |
| -------------- | ------------ | --------------- |
| 6 × 127,959 | 639,795 | `expression@./action.yaml` (the expression action category, 132 verbs), an arm of `entry`/`initialize-entry`/`cancelled-entry`, copied into each family's `stack` and `item` |
| 3 × 218,050 | 436,100 | `loop@./lifecycle.yaml` (`loop-bundle` / `loop-single`), at the root `loop` and in `task`/sequence loops |
| 2 × 59,340 | 59,340 | `bundle@./action.yaml` (short-form bundle) in `stack` and `item` |
| 2 × 59,162 | 59,162 | `initialize-bundle@./action.yaml` |
| 2 × 57,536 | 57,536 | the cancelled family's bundle |
| 9 × 5,028 / 9 × 4,236 / 9 × 2,377 / 9 × 2,017 | 40,224 / 33,888 / 19,016 / 16,136 | loop verb short forms (`ping`, `ping_under`, `code_link`, …) from `@darkmatter:lifecycle`, repeated in loop arms and bundles |
| 6 × 4,332 / 4 × 5,373 | 21,660 / 16,119 | `communicate@./action.yaml`, `flow@./action.yaml` |

`claudine` alone has the same top entries without the third `loop` copy
(7 copies of each loop verb); `sequence` has two `loop` copies. `$defs` holds
13 definitions in `both` (the three recursive families plus the container
types), as today.

Measurement scaffolding: a temporary test block appended to
`claudine/lib/tests/l1/claudine_schemas_embedded.rs`, run with
`cargo nextest run -p claudine --test l1 -E 'test(zz_measure_)' --no-capture`
from `claudine/lib`, then removed from the file so no probe code sits in the
routine suite (it wrote to `/tmp`, which is not portable). Phase 4 re-appends
it verbatim to repeat the sample:

```rust
fn zz_measure(label: &str, frontmatter: &str) {
    use darkmatter::markdown::schemas::{ValidationStage, ValidationTimings};
    let bare = Bare::new();
    let markdown = memory_document(frontmatter);
    let ((schemas, report), timings) = ValidationTimings::record(|| {
        let schemas = bare.schemas(claudine_schema_bundle());
        let report = schemas.validate(&markdown).expect("validation");
        (schemas, report)
    });
    assert!(report.valid, "{:?}", report.problems);
    let effective = schemas.effective_for(&markdown).expect("resolves").expect("a trigger applies");
    let text = serde_json::to_string(&*effective.json_schema).expect("serializes");
    let out = std::path::PathBuf::from("/tmp/shared-acyclic-measure");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join(format!("{label}.json")), &text).unwrap();
    eprintln!(
        "MEASURE {label}: bytes={} discover_ms={:.1} match_ms={:.1} resolve_ms={:.1} compile_ms={:.1} validate_ms={:.1}",
        text.len(),
        timings.stage(ValidationStage::Discover).as_secs_f64() * 1000.0,
        timings.stage(ValidationStage::Match).as_secs_f64() * 1000.0,
        timings.stage(ValidationStage::Resolve).as_secs_f64() * 1000.0,
        timings.stage(ValidationStage::Compile).as_secs_f64() * 1000.0,
        timings.stage(ValidationStage::Validate).as_secs_f64() * 1000.0,
    );
}
#[test]
fn zz_measure_claudine() { zz_measure("claudine", "start: []"); }
#[test]
fn zz_measure_sequence() { zz_measure("sequence", "sequence: []"); }
#[test]
fn zz_measure_both() { zz_measure("both", "start: []\nsequence: []"); }
```

The repeated-helper table came from a throwaway script over the dumped JSON:
count every compact subtree over 2,000 bytes, keep those occurring more than
once, sort by `(copies − 1) × size`, and drop a subtree whose copies all sit
inside a larger repeated one.

### DMLS edit-loop allocation bench (before)

`cargo bench -p dmls --bench pipeline_work_a3` from the worktree root
(release `bench` profile, same host). `bytes_allocated` per phase:

| Case | overlay | diagnostics | completion | hover_verb | hover_expression | total |
| ---- | ------- | ----------- | ---------- | ---------- | ---------------- | ----- |
| a3x1-warm | 364,669,022 | 437,126,085 | 2,142,462 | 58,763 | 69,943 | 804,066,275 |
| a3x1-cold | 474,658,916 | 840,007,880 | 2,142,462 | 58,763 | 69,943 | 1,316,937,964 |
| a3x10-warm | 373,749,976 | 472,403,672 | 2,146,494 | 58,763 | 69,943 | 848,428,848 |
| a3x10-cold | 483,739,870 | 875,285,467 | 2,146,494 | 58,763 | 69,943 | 1,361,300,537 |

Against Decision 10's figures from `2026-10-26-dmls-frontmatter-typing-allocation`:
verb hover 58,763 (equal); diagnostics 437,126,085 vs 437,251,200 (a3x1
warm) and 472,403,672 vs 472,521,047 (a3x10 warm), both within 0.03%;
expression hover 69,943 vs 105,025, lower than that fix's after-number
(later commits on this base reduced it). The fix is present and effective, so
these rows, not Decision 10's, are Phase 4's comparison point. A rise in
either hover after sharing is investigated first as a
`completion::alternative_fingerprint` miss.

### Design Decisions

- **Probes pin observed behavior through public results.**
  - decision: the two new probes assert the exact tuples today's resolver
    produces, including the union explanation Claudine's lifecycle problems
    carry and the description on `MissingRequired`.
  - alternatives considered: assert only verdicts and paths; capture tuples
    into a generated expectation file.
  - why: the plan asks for code, path, line/column, and declaration origin per
    use; the explanation is what DMLS and the CLI show, and `$ref` arms are the
    most likely way sharing would move it. Inline expectations keep the test
    readable and need no bless mechanism.
  - downsides: an intentional future change to explanation selection must
    update these expectations.
- **Declaration origin is the property's `SchemaOrigin`.**
  - decision: "declaration origin" of a validation problem is
    `EffectiveSchema::origins` for its top-level property (`ReferencedFile`
    for the Darkmatter fixture, the embedded `claudine.yaml` trigger for
    Claudine), together with the declared `description`.
  - alternatives considered: a per-keyword schema-source position.
  - why: `ValidationProblem` carries no per-keyword declaration position;
    `origins` is the public provenance DMLS uses for related information.
  - downsides: an origin inside an imported library is not pinned below the
    top-level property.
- **Measurement scaffolding is kept in the log, not in the suite.**
  - decision: the timing/size probe was run once and removed; its exact
    source is above for Phase 4.
  - alternatives considered: leave it in `claudine_schemas_embedded.rs` until
    Phase 4 (the plan's wording).
  - why: it writes to `/tmp` (not portable to Windows) and prints rather than
    asserts; the plan keeps probe code out of routine correctness tests.
  - downsides: Phase 4 has to paste it back.
- **Timing build mode is the nextest `test` profile.**
  - decision: cold samples use the debug test build the budget test runs in.
  - alternatives considered: release.
  - why: the budget test and its inputs live in that build; only matched
    before/after comparison matters. Phase 4 must use the same profile.
  - downsides: absolute times are slower than release.

### Incident

A relative `cd darkmatter/lib` resolved through the shell's `CDPATH` into the
`feat-pipeline` worktree, and one `mod shared_acyclic_definitions;` line was
written to that worktree's `darkmatter/lib/tests/l1/main.rs`. It was reverted
immediately (its `git diff` for that file is empty again). All later commands
used absolute paths.

### Gates

The scope calculator over the uncommitted tree (which also holds the earlier
feature's uncommitted edits) reports `packages: [claudine, darkmatter,
darkmatter-cli, test-toolkit]`, `change_class: package`, so no gate was
skipped.

- `darkmatter/`: `just lint` clean; `just test` 12,675 passed, 10 skipped.
- `claudine/`: `just lint` clean (after factoring the new test's expectation
  slice into a type alias for `clippy::type_complexity`); `just test` 11,655
  passed, 10 skipped. The one SLOW test
  (`research_sequence_artifacts::shipped_research_fleet_runs_row_freshness_and_success_through_normal_cli`,
  claudine-cli) existed before and is not related to this phase.
- `just check-tier-coverage darkmatter` and `claudine`: nothing stranded.
- No cross-OS check: the new tests use temp dirs, `canonicalize_simplified`,
  and in-memory documents, with no path comparison beyond what existing tests
  already do, so they carry no notable OS risk.

### Requirement-to-test mapping (Phase 1)

| Requirement | Test |
| ----------- | ---- |
| Invalid value at each of two uses (code, path, line/column, origin) | `shared_acyclic_definitions::an_invalid_value_is_located_at_each_of_two_uses`; Claudine `a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written` (`start`/`finalize`) |
| Nested list item | `shared_acyclic_definitions::a_nested_list_item_and_a_bundled_use_are_located_at_full_paths`; Claudine conditional body case |
| Missing required field | same Darkmatter test (`/nest/entries/1`), `presence_and_null_belong_to_each_use`; Claudine loop action without `prop` |
| Optional/null, list item nullability | `presence_and_null_belong_to_each_use` |
| Coercion | `conversion_reaches_the_value_at_every_use` |
| Primitive alias narrowing stays at its use | `a_narrowed_primitive_alias_keeps_its_own_bound` |
| Profile restrictions (initialize vs cancelled) | Claudine probe's `initialize`/`cancelled` case; existing `claudine_schemas.rs` |
| Parser-versus-schema distinctions | existing `claudine_schema_shipped_corpus.rs` (both gates per document) |
| Before-sizes, helpers, cold timings | log above (scaffolding run) |
| DMLS bench before-numbers | log above |
