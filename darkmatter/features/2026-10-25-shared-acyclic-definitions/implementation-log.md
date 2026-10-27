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
    - darkmatter-cli
    - dmls
    - claudine-cli
human_review: false
message_to_agent: |-
    Phase 3 landed consumer, editor and Claudine evidence plus three fixes
    to defects that sharing introduced. Darkmatter just test passes
    (12,721), lint is clean, and the Claudine schema subsets pass. Points
    for Phase 4:
    (1) Fixes: a type holding file(match(...)) is never shared
    (recursive.rs is_shareable/holds_file_match), because root-union arms
    enforce match inside definitions; subtype.rs identity opens references;
    a DMLS stored union hovers as its alternatives
    (frontmatter.rs stored_union_type_line). a3_lifecycle_key_hovers.json is
    back to its pre-feature bytes. Phase 2's note (1) is settled.
    (2) Docs owed by "Document shared behavior": dmls/docs/hover.md
    ("Recursive types") and docs/topics/schemas/dmls-schema-support.md call
    the named "defined as" hover recursive-only, but it now also covers
    shared acyclic objects, and the union section holds for stored unions
    too. The schema docs should state that file(match(...)) types are never
    shared. claudine/docs/topics/schemas.md should state the trigger merge
    order: embedded triggers come after disk files, so a later layer's
    declarations replace an earlier one's.
    (3) Still yours from Phase 2: the golden entry-by-entry review, the
    recursive_schema_golden.rs comments and corpus README, the budget
    constant (re-measure; 1,000,000 was the Phase 2 figure), and the DMLS
    bench (compare hover bytes with Phase 1 first). The DMLS hover change
    opens only the top-level references of the hovered type, but recheck
    the verb-hover bytes.
    Use absolute paths in shell commands (CDPATH trap).
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/graph.rs
    - darkmatter/lib/src/markdown/schemas/resolve/recursive.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/tests/l1/shared_acyclic_selection.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-action.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-claudine.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-lifecycle.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence-types.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-md-lifecycle.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-sequence.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-claudine.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-lifecycle.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence-types.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-md-lifecycle.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-sequence.digest
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_key_hovers.json
    - claudine/schemas/document.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/inline-compose.yaml
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/compose_prompt_tiers.rs
    - claudine/cli/tests/l1/entry_point_parity.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_schema.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/schemas.md
    - claudine/schemas/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/claudine/cli-commands.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/resolve/recursive.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/tests/l1/shared_acyclic_consumers.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/recursive_schema_editor.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_key_hovers.json
    - darkmatter/cli/tests/l1/schema_validate.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
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

## Phase 2

Phase 2 is Waves 2 and 3 of the plan. Two lanes ran in parallel, each owning
separate files: the resolver lane (discovery, selection, expansion, per-use
compatibility, pruning) and the Claudine metadata lane (`document.yaml`,
bundle inventory, graph parity). The orchestrator integrated their work,
fixed the seams where they met, and ran the gates.

### What changed

- **Discovery** (`resolve/graph.rs`): `discover` returns `Discovery { cyclic,
  shared }`. Each non-function value reference is recorded once per
  `(target DefKey, authoring namespace, full source path)`, so the root walk
  and a definition-body walk of the same `@this` reference count once. Only
  sites in the root or in a value-reachable body count; references reached
  only through `function(...)` signatures do not. Structured classification
  is memoized per `DefKey` over the unexpanded definition and follows aliases
  through the namespaces discovery already resolved (no new reads). Cycle
  admission is unchanged.
- **Expansion** (`resolve.rs`, `resolve/recursive.rs`): the engine holds
  `shared` beside `cyclic`; the two-pass path runs when either is non-empty,
  and the single-pass path only when both are empty. A shared body is
  expanded at its first use, through the same cache and import stack as an
  inlined use, then stored in `reached` in first-reached order. Pass 1 turns
  every eligible use into a reference so that every body is summarized;
  pass 2 checks each use against its summary (`can_reference`,
  `use_is_representable`, arm checks, `alias_use_keeps_reference`) and
  inlines any use it cannot represent. `publish_definitions` keeps only
  definitions reachable, transitively, from the expanded schema.
- **Catalog declarations** (`catalog.rs`, new `resolve_catalog_declaration` /
  `prepare_embedded_catalog_declaration`) resolve with sharing off; see the
  decisions below.
- **`frontmatter_shape.rs`/`mod.rs`**: added the public
  `frontmatter_expression_values_with_definitions`, so expression detection
  can follow references. No production caller used the variant without
  definitions.
- **Comments**: module and field docs in `resolve.rs`, `graph.rs`,
  `recursive.rs`, `simplified/definitions.rs`, and `simplified/convert.rs`
  that described definitions as cyclic-only, or acyclic types as always
  inlined, now describe stored shared definitions.
- **Claudine metadata**: new `claudine/schemas/document.yaml` holds `prompt`,
  `hash`, and `last_updated` verbatim. `claudine.yaml` and
  `inline-compose.yaml` import them (`X@./document.yaml`); match conditions,
  property order, and openness are unchanged. `bundle.rs` lists eight files,
  three of them triggers. The metadata lane confirmed that the old and new
  triggers lower to identical fragments and the same total size (366,588
  bytes) before the resolver changes took effect. The CLI test helper
  `seed_inline_metadata_schema` now writes `schemas/document.yaml` beside the
  trigger, and every Darkmatter test copy of the Claudine bundle includes the
  library. A disk trigger imports only from disk, so without the file it
  would be skipped.

### Measurements (after, matched to Phase 1)

Same host (macOS, Apple silicon), nextest `test` profile, and the Phase 1
`zz_measure` scaffolding. The scaffolding was pasted in, run once, and then
removed, so the file was restored byte-for-byte.

| Case | Bytes before | Bytes after | resolve ms (before → after) | compile ms (before → after) |
| ---- | ------------ | ----------- | --------------------------- | --------------------------- |
| `claudine` | 1,403,266 | 572,195 | 387.7 → 251.6 | 275.5 → 104.7 |
| `sequence` | 1,618,148 | 569,111 | 392.1 → 250.6 | 270.1 → 104.8 |
| `both` | 1,844,516 | 768,262 | 529.1 → 311.1 | 444.4 → 175.3 |

With `M = 768,262`, the formula gives `100_000 * ceil(1.25 * 768,262 /
100_000) = 100_000 * ceil(9.603) = 1,000,000`. Phase 4 re-measures and
checks in the constant; the 2,000,000 constant is still in place. The
largest remaining repeat is the `loop` helper: one copy in the Darkmatter
baseline layer (catalog declarations are not shared) and one in the
Claudine payload.

### Snapshot and representation changes

Frozen inputs are unchanged: `git status` shows nothing under
`recursive_schema_golden/inputs`, `recursive_schema_frozen`,
`lib/benches/fixtures`, or `claudine/lib/tests/fixtures`. These goldens were
re-blessed with `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`, only after the
Phase 1 probes passed:

- Now below the 128 KB digest threshold, so they changed from digest to
  full JSON: `claudine-schemas-{claudine,lifecycle,sequence,sequence-types}`,
  `darkmatter-schemas-darkmatter-schema-body`,
  `darkmatter-schemas-partials-{fm,md-lifecycle}`, and
  `effective-trigger-sequence`.
- Changed in place: `claudine-schemas-action.json`,
  `effective-trigger-inline-compose.digest`, and
  `effective-trigger-with-document-schema.digest`.
- The cause is the same for every entry. The action categories, bundles,
  `communicate`/`flow`/`shell`/`expression`, the loop helpers, the verb
  short forms, and the unrolled `stack-N`/`task-N` bodies each have two or
  more authored sites, so they are now stored definitions. Phase 4 still
  owns the entry-by-entry review, plus the `recursive_schema_golden.rs`
  comments and the corpus README.
- DMLS golden `a3_lifecycle_key_hovers.json`: hovering the `then` key typed
  by `initialize-stack-2` now shows `Type: initialize-stack-2(required) … is
  defined as`. DMLS already renders stored definitions this way. Before,
  the hover listed the expanded alternatives. This is visible to users, and
  Phase 3's "Preserve editor behavior" should confirm it is acceptable.

Assertions adapted to the intended representation:

- `schemas_named_type_merge::key_bounds_from_constraints_blocks_and_postfixes_are_one_scope`:
  `full` now uses its own type `wide`, so each type has one use and the test
  still checks the one-scope merge it targets.
- `schemas_union_typed_consumers` (two `frontmatter_shape` tests) now call
  the `_with_definitions` APIs.
- `base_schema_end_to_end`: baseline parity compares through
  `GlobalCatalog::from_file`, not `resolve_yaml_schema` (catalog
  declarations do not share).
- `shipped_schema_corpus::CORE_TYPE_USERS` lists `claudine/schemas/document.yaml`,
  which uses `unknown`.
- `claudine_schemas::resolved_exports` now dereferences acyclic local
  `$ref`s, and `arms_of` flattens an arm that is only a nested `anyOf`. An
  optional union alias now lowers to `anyOf: [null, {$ref → anyOf[…]}]`
  where inlining gave one flat list, and both accept the same values.
  `bundle_values_are_the_category_short_forms` failed until this change.
- `claudine_schemas::hash_imported_twice_accepts_both_stored_forms_at_each_use`
  is a test this phase added. Its value `{ "any": […] }` tripped
  `trigger_spelling_guard`, so it now uses `{ "mixed": […] }`.

### Design Decisions

- **A shared body is expanded at its first use, under that use's stack.**
  - decision: a selected acyclic body is expanded through the existing
    `expanded_cache` and import stack at the first use that reaches it, and
    then stored. It is not queued on the worklist with a reset depth.
  - alternatives considered: expand from the worklist starting at depth 0;
    expand from the worklist with a recorded base depth.
  - why: the 32-level import limit and the order of first expansions then
    match inlining exactly. A chain rejected today stays rejected, and no
    order-dependent boundary moves (Decision 3).
  - downsides: shared and cyclic bodies are filled at different points in a
    pass, so `reached` holds both kinds of entry.
- **Compatibility is a conservative predicate, decided in pass 2.**
  - decision: pass 1 makes every eligible use a reference, so every body is
    summarized. Pass 2 keeps a reference only when the use's postfix contains
    nothing but use-site markers and representable count bounds. A key
    exclusion keeps the reference only when both sides exclude the same
    keys, or when one side has no bound for the exclusion to change.
    `suggest`, `options`, and anything else not proven equivalent inline.
    Errors are never caught and retried as inlining.
  - alternatives considered: specialize stored bodies per use; reject
    unsupported uses; retry on conversion errors.
  - why: the plan (Decision 4) and the spec rule out the alternatives, and
    an inlined use is today's behavior by construction.
  - downsides: some valid uses stay inlined.
- **Body-level properties make a type ineligible.**
  - decision: some types are never shared, because no use of them could
    keep a reference:
    - a type whose top level admits null or carries `operands`/`deferred`;
    - a union whose arms carry presence or default markers, `suggest`, an
      optional `file` arm, or a reference into a cycle.

    A union target used as a union arm always inlines, so the arm list
    stays flat for consumers that rely on it. Any other arm reference needs
    an empty postfix and no inherited markers or description.
  - alternatives considered: let union arms be references with marker and
    suggestion hoisting through the reference.
  - why: flattening hoists arm markers, counts suggestions, and copies
    descriptions, and a reference hides all of that. The savings are large
    without it.
  - downsides: a category union stays copied inside each union that lists
    it.
- **An alias of a shared type that cannot keep the reference inlines its
  whole chain.**
  - decision: when an alias whose top level is a reference to a shared type
    is used with a postfix that reference cannot represent, the alias is
    re-expanded with forced inlining, cached separately in `forced_cache`.
    The force passes along the top-level non-array alias chain only.
  - alternatives considered: rebuild the inline form from the alias's
    merged constraints.
  - why: the merged constraints no longer say which use wrote each one;
    re-expanding reuses the existing inline rules.
  - downsides: a forced re-expansion may run under a deeper stack than the
    first expansion did.
- **Global catalog declarations resolve without sharing.**
  - decision: `catalog.rs` resolves each declaration with sharing off.
  - alternatives considered: carry a definition table with the baseline
    `fm` and `darkmatter_base_schema()`.
  - why: those are consumed as definition-free schemas (sharing made "the
    embedded baseline schema must lower" panic), and every consumer would
    need a table.
  - downsides: the baseline layer does not shrink and keeps one `loop` copy.
- **Graph parity in Claudine compares through a bijective reference mapping.**
  - decision: `GraphParity` follows each `$ref` pair into its root's
    `$defs`. It records the pairs in a two-way map, so one key cannot stand
    for two definitions, and skips visited pairs so recursive graphs end.
  - alternatives considered: compare bare `$ref` strings or serialized JSON
    (the spec rules both out).
  - why: the comparison must pass whether a type is inlined or shared. It
    was checked against both resolvers during this phase.
  - downsides: more test code, guarded by its own self-test.
- **The CLI fixture helper takes the library text as an argument.**
  - decision: `seed_inline_metadata_schema(schema, document_library)`, with
    each caller passing `include_str!` itself.
  - alternatives considered: read the file inside the helper.
  - why: a repository-file literal inside a helper widens the CI test-input
    cell to the whole binary.
  - downsides: mechanical edits at seven call sites.
- **Affected goldens were re-blessed in Phase 2.**
  - decision: the affected goldens were re-blessed in this phase, and the
    changes and their causes are recorded above.
  - alternatives considered: leave `recursive_schema_golden` red until
    Phase 4.
  - why: Phase 2 cannot finish with failing tests. Blessing waited until the
    Phase 1 semantic probes passed, as Decision 7 requires.
  - downsides: Phase 4 still reviews each entry and updates the comments and
    README that promise byte stability.

### Requirement-to-test mapping (Phase 2)

| Requirement | Test |
| ----------- | ---- |
| One definition, two references; single use inlined | `shared_acyclic_selection::a_repeated_object_is_stored_once_and_referenced_at_each_use`, `a_type_used_once_is_inlined` |
| Primitive alias, primitive array, array of a primitive-only union, primitive-only union, union whose only container arm is a primitive list: inlined | `repeated_primitive_shapes_stay_inlined` |
| Dictionary, tuple, object array, structured union: stored | `repeated_structured_shapes_are_stored` |
| Two sites in one body; alias sites | `two_sites_in_one_body_and_two_aliases_each_count` |
| Repeated parent does not multiply | `uses_of_a_parent_do_not_multiply_the_references_in_its_body` |
| Root/body revisit counted once | `a_root_declaration_walked_again_as_a_definition_counts_its_references_once` |
| Unused declaration, callable-only reference | `unused_declarations_and_callable_signatures_do_not_count` |
| Namespace identity (same names distinct, two spellings one target) | `a_target_is_its_namespace_and_name` |
| `min-keys(1)` beside one reference | `an_added_key_bound_stays_beside_its_own_reference` |
| Key-exclusion example (only `annotated` inlined) and an unchanged exclusion kept shared | `a_key_exclusion_that_changes_the_body_bound_inlines_that_use_only` |
| Alias fallback, including a union-arm use | `an_alias_used_with_an_unrepresentable_postfix_inlines_through_to_the_body` |
| All-fallback output has no `$defs` | `when_every_use_inlines_no_definition_is_emitted` |
| Typed conflict with inherited-first participants and `diagnostic_excerpt` | `a_conflicting_bound_keeps_its_typed_error_and_inherited_first_origins` |
| Default conflict through a shared list | `a_default_conflict_is_reported_through_a_shared_list` |
| Suggestion limit, optional-file empty string, nullable union | `context_dependent_union_bodies_stay_inlined` |
| Reachable callable rejected at the use's property path | `a_callable_in_a_shared_body_is_rejected_at_a_use` |
| 32-level limit, inlined and stored chains (accepted at the boundary, rejected one past) | `storing_definitions_keeps_the_import_depth_limit` |
| Deterministic repeated resolution | `shared_output_is_deterministic` |
| Cycle admission unchanged | existing `recursive_schema_types.rs`, `recursive_schema_conversion.rs`, `meta_schema_reference_graph.rs` (unchanged, passing) |
| Phase 1 semantic probes unchanged | `shared_acyclic_definitions` (6), Claudine `a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written` |
| Metadata graph parity per trigger and through the merge (disk and embedded) | `claudine_schemas::shared_payload_properties_have_identical_types`, comparator self-test `graph_parity_compares_definitions_through_a_bijective_reference_mapping` |
| Two-use `hash` fixture: one shared definition, both forms accepted at each use, invalid values located per use | `claudine_schemas::hash_imported_twice_accepts_both_stored_forms_at_each_use` |
| Absent/null metadata, both hash forms, invalid metadata, passive validation and launch verdict | `claudine_schemas_embedded::document_metadata_verdicts_agree_between_passive_validation_and_launch` |
| Embedded resolution without disk; eight files, three triggers; byte-for-byte inventory; exports | `claudine_schemas_embedded::embedded_document_exports_resolve_without_disk_schemas`, `embedded_files_are_the_repository_files_byte_for_byte`, `every_file_loads_and_every_export_resolves` |

The Input Robustness Matrix does not apply (plan Decision 9): no loader,
configuration field, or deserializer changed.

### Skills updated

- `.claude/skills/darkmatter/schema.md`: the lowering description now covers
  shared acyclic definitions (selection, counting, first-use expansion, per-use
  fallback, pruning, catalog declarations unshared) and drops the
  "stores only cycle members" claim.
- `.claude/skills/claudine/cli-commands.md`: the bundle row now lists eight
  files, including `document.yaml`, and the `$defs` note covers repeated
  helpers.

### Gates

The scope calculator over the uncommitted tree reports `packages: [claudine,
claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]`,
`change_class: package`, so no gate was skipped. `test-toolkit` appears
because the tree still holds the earlier feature's uncommitted edits.

- `darkmatter/`: `just lint` clean; `just test` 12,694 passed, 10 skipped,
  with no SLOW or timed-out tests.
- `claudine/`: `just lint` clean (only the build environment's linker
  `__eh_frame` warning); `just test` 11,659 passed, 10 skipped. Claudine is
  both changed directly and the dependent of darkmatter, so this full run
  covers the dependent-package step.
- `just check-tier-coverage darkmatter` and `claudine`: nothing stranded.
- Seams fixed during integration:
  `claudine_schemas::bundle_values_are_the_category_short_forms` (helpers now
  follow `$ref`) and `trigger_spelling_guard` (a test value spelled `any:`).
  `container_schema_recursion::deep_container_deferral_does_not_hide_literal_or_shape_errors_in_either_phase`
  failed once while the resolver was mid-edit and passed every run
  afterwards, including the full suite.
- No cross-OS check was run. The changes are resolver logic and test
  fixtures written into temp dirs, with no path comparison, platform
  conditional, or process change. CI covers the other OSes.

## Phase 3

Phase 3 is Wave 4 of the plan. Three lanes ran in parallel with separate
file ownership: library consumers (`darkmatter/lib`), editor behavior
(`darkmatter/dmls`, `darkmatter/cli` tests), and Claudine integration
(`claudine/lib/tests`). Only the library lane could edit `darkmatter/lib/src`.
The orchestrator ran the area gates once after all three lanes finished.

### Consumer audit (library)

Two consumers changed behavior once a type was shared. Both are fixed:

- **Root-union `match()` enforcement.** A root-union arm enforces a glob inside
  any definition it reaches (`MatchCopies`). An inlined acyclic type only
  suggests files. So a shared object, tuple, dictionary, or list holding
  `file(match(...))` started rejecting values that the inlined form accepted
  (`valid=true` became `valid=false`).
  - Fix: `resolve/recursive.rs::is_shareable` now refuses any type whose
    expansion holds a `file(match(...))` outside a reference, through
    `holds_file_match`.
  - No shipped schema uses `file(match(...))`, so no golden and no size
    changed.
- **Structural identity and subtyping** (`simplified/subtype.rs`). Identity
  compared a reference with an inline type by key. So `entry[]` against an
  identical inline list became unequal, and so did `{ x: entry }` against
  `{ x: { n: number } }`. Separately, a list of a reference to a union stayed
  one leaf, where the inlined list flattened into one leaf per alternative.
  - Fix: identity now opens references through the table (`Identities`,
    coinductive on reference pairs). `leaves` flattens a list of a union
    reference. The module doc was updated.

Verified correct and left unchanged:
- Phase projection (`phase.rs`), lowering (`convert.rs`), and eager-file
  normalization (`rewrite.rs`).
- Deferred and pending detection (`coerce.rs`).
- Completion (`completion.rs` through `resolved_alternatives`).
- The call check (`DeclaredTypes::for_schema`) and catalog declarations,
  which resolve unshared.

These consumers never meet a resolver reference:
- `detect.rs`
- `lifecycle/schema_projection.rs`
- `triggers/matcher.rs` (it rejects imports)
- `simplified/lint.rs`

The table-less helpers (`property_def_at_path`, `nested_property_shape`,
`suggestions_for_path`, `alternatives_for_def`) and `scope/shape.rs::admits`
have no production caller in the library. The editor lane checked the DMLS
caller of `suggestions_for_def`.

### Editor behavior (DMLS)

DMLS hover of a stored union had regressed. A union with two uses is now a
stored definition, and DMLS rendered a stored definition as
"`X` is defined as". The hover lost the "one of" alternative list that
`dmls/docs/hover.md` ("Union-typed properties") promises. It also lost the
list that the item note ("matches alternative N of M") numbers. This is what
Phase 2's note (1) reported for `initialize-stack-2`.
- Fix: `providers/frontmatter.rs::stored_union_type_line`. A stored reference
  whose body resolves to two or more alternatives now hovers as the inlined
  union did. Every other stored reference keeps the named "defined as" form.
- `a3_lifecycle_key_hovers.json` was re-blessed and is back byte-for-byte at
  its pre-feature content, so it no longer appears in `git diff`. Phase 2's
  frontmatter lists it, but its net change is now nil.

### Claudine integration

A gap audit of the Phase 2 tests found these missing:
- `pre_validate_schema` coverage with each trigger applying alone.
- The longhand `hash` default.
- A disk `inline-compose.yaml` shadowing the embedded one.
- A launch-path check of a shared `hash` definition.

The lifecycle corpus bullet (action permissions, initialization and
cancellation restrictions, launch verdicts, deferred templates, schema versus
parser distinctions) already had owners, and all of them pass. Examples:
`ac5_*`, `depth6_wrong_verb_is_positioned_at_the_verb`,
`ac5_generated_operands`, `initialize_stacks_exclude_shell_at_every_depth`,
the `*cancelled*` tests, `break_and_prep_shapes_agree_between_schema_and_parser`,
the 55 `passive_parity_*` tests, the `sequence_schema` and
`container_schema_recursion` modules, and `claudine_schema_shipped_corpus`.
No behavior changed under sharing.

Two findings, neither new behavior:
- `hash.kind`'s default `"simple"` is a schema annotation only. No path
  inserts it, consistent with "No default is ever inserted".
- When a disk `inline-compose.yaml` copy and the embedded `claudine.yaml`
  both match, the embedded trigger comes later in the merge order and its
  metadata declarations replace the copy's. The copy's typing wins only where
  it applies alone. `claudine/docs/topics/schemas.md` does not yet state the
  merge order; Phase 4's documentation task should add one sentence.

### Design Decisions

- **A type holding `file(match(...))` is never shared.**
  - decision: selection, not lowering, excludes such a type, so it always
    inlines.
  - alternatives considered: give acyclic arm copies a redirect-only lowering
    mode in `convert.rs`; let root-union arms enforce globs in shared bodies
    (a behavior change the spec forbids).
  - why: one predicate restores the old behavior exactly, and no shipped
    schema pays a byte cost for it.
  - downsides: such a type is never stored, even when no root union reaches
    it.
- **Structural identity opens references, cyclic ones included.**
  - decision: `subtype.rs` compares definition bodies through the table and
    treats a revisited reference pair as equal (coinductive).
  - alternatives considered: open only acyclic references (the table does not
    record which definitions are cyclic); keep comparing by key (breaks
    parity with the inlined form).
  - why: matching the inlined form requires opening references, and
    coinduction makes the comparison terminate on cycles.
  - downsides: two distinct recursive types with identical bodies now compare
    equal where they did not before. No test depended on the old answer, and
    structurally identical types are the same type.
- **A stored union hovers as its alternatives. A stored object keeps the
  named form.**
  - decision: as described under "Editor behavior" above. This settles Phase
    2's open note (1): the "defined as" rendering is accepted for stored
    objects but not for stored unions.
  - alternatives considered: keep the Phase 2 rendering everywhere (breaks the
    documented union hover); expand stored objects too (loses the type name);
    list the flattened, name-keeping arms (does not match the item note's
    numbering).
  - why: the spec asks consumers to treat a new reference like the value it
    replaces, and the union hover rule is documented.
  - downsides: a recursive union referenced directly now also lists its
    alternatives rather than "defined as", and a recursive arm inside that
    list shows as `object with …`.
- **One trigger is made to apply alone by shadowing the other with a disk
  copy that never matches.**
  - decision: Claudine tests hide one embedded trigger with a disk copy that
    never matches, so the other trigger applies alone through the real
    launch-verdict path.
  - alternatives considered: use `sequence:` to keep the inline trigger out
    (pulls in `sequence.yaml`); build a reduced bundle.
  - why: any document with a non-null `prompt` matches `claudine.yaml`, so
    only shadowing isolates the inline trigger. A non-matching disk file still
    shadowing is a documented rule.
  - downsides: the tests depend on that shadowing rule.
- **The `hash` default is tested as declared and never inserted.**
  - decision: the test asserts the resolved `kind` default `"simple"` and
    that the prepared frontmatter leaves an absent `kind` absent.
  - alternatives considered: assert default insertion (behavior that does not
    exist).
  - why: Darkmatter inserts no defaults, and the probe confirmed it.
  - downsides: none.

### Requirement-to-test mapping (Phase 3)

Library, all in `darkmatter/lib/tests/l1/shared_acyclic_consumers.rs` unless
noted. Most of them compare the two-use (shared) form against the one-use
(inlined) form and assert that the shared form has `$defs`.

| Requirement | Test |
| ----------- | ---- |
| Lowering parity (defaults, eager format, deferred, descriptions, required) | `each_use_lowers_to_the_inlined_type_once_its_reference_is_followed` |
| Launch/completion presence, required/null, property vs list-item nullability | `launch_and_completion_presence_is_unchanged_at_each_use` |
| Coercion | `conversion_reaches_each_use_as_it_reached_the_inlined_type` |
| Codes, full paths, line/column, descriptions at each use, nested item, missing required | `invalid_values_are_located_at_each_use_as_the_inlined_type_located_them`; Phase 1 `shared_acyclic_definitions` (6) |
| Inherited-first constraint-conflict origins | Phase 2 `shared_acyclic_selection::a_conflicting_bound_keeps_its_typed_error_and_inherited_first_origins` (re-run, passing) |
| Eager-file normalization | `eager_files_are_normalized_at_each_use` |
| Deferred operands and pending values | `deferred_templates_and_pending_values_are_judged_as_before` |
| Validation-only paths execute nothing | `validating_a_shared_use_executes_nothing` |
| Completion lookup | `completion_offers_the_inlined_choices_at_each_use` |
| Call-check categories | `the_call_check_sees_through_each_use` |
| Subtype parity (fix) | `subtyping_compares_a_shared_type_as_its_inlined_copy` |
| Root-union `match()` stays suggestion-only (fix), with a glob-free shared control | `a_root_union_arm_keeps_a_suggestion_only_match_through_a_repeated_type` |
| Trigger + document reuse one equal body | `a_trigger_and_a_document_sharing_one_imported_type_reuse_its_body` |
| Different bodies rekeyed, references between definitions, deterministic bytes | `different_bodies_are_rekeyed_with_the_references_between_definitions` |
| Baseline merge rekeying, deterministic bytes | `a_baseline_and_a_document_sharing_same_named_types_keep_their_own_bodies` |
| Root-union merge, deterministic bytes | `root_union_arms_sharing_same_named_types_keep_their_own_bodies` |
| Imported-file dependency invalidation | `editing_and_deleting_an_imported_shared_type_is_observed` |
| Embedded import | `an_embedded_payload_shares_a_library_type_at_each_use` |
| Reserved import (`darkmatter:timing`) | `a_reserved_partial_type_is_shared_at_each_use` |
| Passive shipped-artifact coverage | `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` (extended: every local `$ref` resolves; storing artifacts resolve to identical bytes twice; at least two shipped artifacts store definitions) |

Editor, in `darkmatter/dmls/tests/l1/recursive_schema_editor.rs` unless noted.
The fixture is an imported `types.yaml` with `entry`, the alias `item: entry`
(used as `items: item[]`), and a decoy `spare` type:

| Requirement | Test |
| ----------- | ---- |
| Fixture really shares (no vacuous pass) | `the_shared_fixture_stores_entry_once_and_references_it_at_every_use` |
| Diagnostics and ranges at each use and a nested list item | `a_failure_through_each_shared_use_is_published_at_its_value` |
| Completion at each use, incomplete object, empty item; only the cursor path's definition opened (lookup counters) | `completion_offers_the_shared_keys_at_each_use_and_in_an_empty_item` |
| Hover at each use | `hover_describes_the_shared_type_at_each_use` |
| Stored union hover equals inlined hover (fix) | `a_shared_union_hovers_as_its_inlined_alternatives`; golden `a3_lifecycle_key_hovers.json` |
| Definition navigation to the imported declaration and through the alias | `definition_from_each_shared_reference_reaches_the_imported_declaration` |
| Imported dependency refresh | `editing_the_imported_shared_type_refreshes_every_use` |
| `md schema validate` wiring, valid | `darkmatter/cli/tests/l1/schema_validate.rs::schema_validate_accepts_a_shared_type_at_every_use` |
| `md schema validate` wiring, invalid (path, line/column at each use and an item) | `schema_validate.rs::schema_validate_reports_a_failure_through_each_shared_use_at_its_location` |

Claudine, in `claudine/lib/tests/l1/claudine_schemas_embedded.rs`:

| Requirement | Test |
| ----------- | ---- |
| Each trigger alone, passive and `pre_validate_schema`, 25 metadata cases (absent/null, prompt/date validity, both hash forms, arbitrary `value`, invalid) | `document_metadata_verdicts_agree_through_each_trigger_alone` (new); `DOCUMENT_METADATA_CASES` extended from 11 to 25 cases |
| Merged triggers through the same cases | `document_metadata_verdicts_agree_between_passive_validation_and_launch` (now reads the shared case table) |
| Longhand hash default | `longhand_hash_kind_default_is_declared_and_never_inserted` |
| Inline-trigger shadowing and metadata typing | `a_disk_inline_compose_trigger_shadows_the_embedded_one_and_types_the_metadata` |
| Same-resolution two-property `hash` fixture through the launch verdict | `a_shared_hash_definition_keeps_both_stored_forms_through_the_launch_verdict` (beside Phase 2's passive `hash_imported_twice_accepts_both_stored_forms_at_each_use`) |
| Lifecycle/profile corpus unchanged | existing owners listed above (re-run, passing) |

The Input Robustness Matrix does not apply (plan Decision 9): no loader,
configuration field, or deserializer changed.

### Lane verification (scoped)

- Library: the `shared_acyclic|recursive_schema|schemas_union_typed_consumers|
  schema_cache_validity|schemas_named_type_merge|container_schema_recursion|
  shipped_schema_corpus|base_schema_end_to_end` filterset passed 286. The lib
  unit tests for `subtype|resolve|recursive|call_check|scope|convert` passed
  568. `recursive_schema_golden` passed 17, with no golden changes.
- Editor: DMLS L1 passed 705 (whole binary). DMLS lib
  `snapshot_tests|hover|sequence_tests` passed 77. `darkmatter-cli`
  `schema_validate` passed 151.
- Claudine: the scoped L1 filterset passed 937 (184 skipped). Lib
  `pre_validate|composition::schema` passed 126.
- `just lint` was clean in `darkmatter/` and `claudine/`. No load-sensitive
  failures.

### Skills updated

- `.claude/skills/darkmatter/schema.md`: a type holding `file(match(...))` is
  never shared, and structural identity in `subtype.rs` opens references.
- `.claude/skills/darkmatter/dmls.md`: the frontmatter key hover rule for
  stored definitions (unions list their alternatives; others say "defined
  as").

### Gates

The scope calculator over the uncommitted tree reports `packages: [claudine,
claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]` and
`change_class: package`, so no gate was skipped. The tree still holds
Phase 2's uncommitted work and the earlier feature's edits.

- `darkmatter/`: `just test` passed all 12,721 tests (10 skipped), with no
  failure or timeout. `just lint` exited 0 with no warnings.
  `just check-tier-coverage darkmatter`: nothing stranded.
- Dependents (run serially, scoped, because each suite has more than 200
  tests):
  - `cargo nextest run -p claudine -E 'test(/schema/) | test(/sequence/) |
    test(/operand/) | test(/pre_validate/) | test(/lifecycle/) |
    test(/cancelled/) | test(/document_metadata/) | test(/call_check/) |
    test(/subtype/) | test(/compose/) | test(/trigger/)'` passed 2,544.
  - `cargo nextest run -p claudine-cli --features test-fixtures -E
    'test(/schema/) | test(/sequence/) | test(/lifecycle/) | test(/inline/) |
    test(/call/) | test(/metadata/) | test(/parity/)'` passed 1,961.
  - Claudine `just lint` was clean (lane run).
- No cross-OS check was run. The changes are resolver selection, type
  comparison, hover text, and tests that use temp dirs. They add no path
  comparison, platform conditional, or process change, and CI covers the
  other OSes.
