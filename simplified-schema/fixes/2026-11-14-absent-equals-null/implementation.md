# A Missing Key and `null` Are the Same Value: Implementation Record

This records what was built and the evidence for it. Decisions awaiting review
are in `decisions_for_review` in `implementation-log.md`. The log has the detail
for each phase, including every test file each phase added or changed.

## Completed tasks

All six phases of `plan.md` are complete. Phases 1–5 were committed one phase at
a time. Phase 6 changed one doctest import (`simplified-schema/lib/src/triggers/assemble.rs`)
and the planning records.

## What changed, by area

### SimplifiedSchema (library)

- **`required` means presence only.** A required key must be written. A written
  `null` meets presence and is then judged by the property's type. A missing key
  and `null` are otherwise one value (`simplified/types.rs`, `simplified/convert.rs`,
  `resolve/recursive.rs`).
- **Defaults are a separate step that runs before any validator.** `defaults.rs`
  (`DefaultPlan`, `prepare_defaults`) fills missing or `null` properties
  that have a default, whether selected literally, by a pattern, or by a catch-all,
  and whether or not the property is required. Review 2's selected-entry and
  combined-selector cases are repaired, as are review 3's direct conjunctive
  required-child cases. Review 4's independent object-union cases are also
  repaired: each independent union is checked as a group before child
  completion. Review 5's non-object and closed-object alternatives are now
  repaired too: an alternative without a definition for the key counts as
  accepting only when a static reading admits an object holding that key.
  Invalid parent defaults in those cases are refused before child completion
  (review 5, R5-F1). The check's documented boundary excludes conditional
  keywords and satisfiability of the unfinished parent's other constraints.
  Strict validation judges
  the completed candidate, returns the input borrowed, and lists `defaults`. Soft
  validation applies them. Every shortcut that skipped preparation because the
  input was already valid is gone (`prepared.rs`, `coerce.rs`).
- **Union-default conflicts fail when the schema is compiled.** Union arms with
  different defaults at one location fail lowering or `Prepared::new` with
  `ConflictingDefaults`. They never fail at run time based on which arm matched.
  One exception: a property-level union whose arms disagree is still a
  `SchemaError::Convert` (see decisions). Conflicts between overlapping dictionary
  selectors can instead surface when a value needs the selected key's default;
  `Prepared::new` does not refuse every such conflict.
- **`non-null` is removed.** Every spelling of it is a located schema error.
  `not-empty` refuses empty or whitespace-only text, an empty list (exactly like
  `min(1)`), and an empty mapping. On `number`, `boolean`, `null`, or `unknown`
  it is a schema error.
- **Phase projection keeps the declared value type.** `phase::lower` replaces
  `project`/`project_definitions`. Only the parent `required` arrays follow the
  launch and completion policy.
- **Envelope carriers read `null` as absent** (`simplified/standalone.rs`,
  `triggers/envelope.rs`). A trigger whose `$schema` is empty (`{}`, `[]`, blank
  text) is refused with a located message.
- Exported JSON Schema keeps `required`, and `default` there is only an
  annotation. A plain validator still refuses an absent, defaulted, required key.

### Darkmatter

- `EffectiveSchema` uses the library's preparation: no "already valid"
  shortcut, `data_copy` withdraws applied defaults so `md set` never writes
  them, and `required_defaults` was renamed `applied_defaults`.
- Composition fills a missing or `null` value with its default (`insert_defaults`).
  `materialize_optional_document_bindings` is deleted, so a declared but unwritten
  property is never written as a synthetic `null` into the working data,
  lifecycle inputs, or `--frontmatter` output.
- Lifecycle flags use the library's binder (`lifecycle/action_shape.rs`):
  `no_error` and `create_new` default to `false`, `gui` missing or `null` is
  unset, Boolean-like text converts, and a native number is refused. Where a flag
  was written is tracked as data (`LifecycleParseOptions::authored_values`), so
  filled defaults never count as authored, and two written placements fail.
- The lifecycle schema generator (`schema_projection.rs::long_arms`) emits one
  optional `no_error: boolean(default(false))` instead of separate flag arms and
  a `min-keys` workaround.
- Expression typing treats an optional property with a non-null literal default
  as never null.
- Darkmatter config: `permissions: null` and similar keys now take their scope's
  default instead of a "null" error.

### Claudine

- **Keyed action and route payloads (migration).** Each `messenger-route` and
  `action` in `claudine-config.yaml` is a closed mapping with one required key,
  for example `{ discord: { channel_id: "…" } }`, `{ slack_webhook: {} }`,
  `{ sound_effect: { effect: "…" } }`, `{ bash: { command: "…" } }`.
  - **Why:** the old flat, `type`/`provider`-tagged union put Discord's and
    Slack's different `bot_token_env` defaults (and the two webhook variants'
    `webhook_url_env` defaults) at the same location. Under the new rule that is
    a conflicting default and the schema would not compile. Keying each payload
    gives every provider's defaults their own path and keeps every capability and
    default.
  - The runtime `MessengerProviderConfig` and `HookAction` enums are unchanged.
    The private `config/wire.rs` serde adapters translate at the config boundary
    and refuse multiple keys, a `null` payload, a wrong container, or an unknown
    key.
  - Flat YAML gets a migration diagnostic with a keyed example. The legacy
    JSON5 converter writes the keyed form. Keys are snake_case only
    (`discord_webhook`, `slack_webhook`).
- **A `null` messenger override used to disable messaging; it no longer does.**
  `RepoOverrideConfig::active_messenger` was `Option<Option<String>>`, and a
  repository's `active_messenger: null` turned messaging off for that repository.
  It is now `Option<String>`: missing or `null` inherits the user's active route.
  The config TUI's repository "(disabled)" option was removed. There is no
  repository-level off switch now; adding one needs a distinct value (see
  decisions).
- Repository `null` overrides of `actions`, `matchers`, `steering`, and nested
  event entries inherit, through typed null-as-unset deserializers. Empty
  overrides that are now `not-empty` (`steering: {}`, an empty `action[]`) are
  refused.
- `claudine init` no longer writes an empty action list for an event that has no
  action.
- Launch keeps the declared value type. A required non-eager `string` whose
  expression resolves to `null` refuses the inline launch. An omitted one is
  still deferred to completion. `complete_launch_defaults` is deleted.

## Schema migration (`non-null` → `not-empty`)

Applied by resolved type (plan Rule 16): strings, lists, and mappings became
`not-empty`. Booleans lost the marker. Named-union references
(`model-override(non-null)`) became bare references.

| File | Change |
|---|---|
| `simplified-schema/schemas/{schema,yaml-schema,schema-trigger}.yaml` | `description`/`categories` → `not-empty`; optional `types` → `schema(mapping; not-empty)`; optional `$schema` → `schema(not-empty)` |
| `darkmatter/schemas/darkmatter-config.yaml`, `partials/lifecycle.yaml`, `partials/md-lifecycle.yaml` | migrated markers; regenerated lifecycle arms; conditionals declare `no_error: literal(false)` |
| `claudine/schemas/claudine-config.yaml` | TTS `provider` and the 16 matcher strings `string(not-empty)`; 16 event lists `action[](not-empty)`; `prewarm: boolean(default(true))`; keyed routes and actions |
| `claudine/schemas/{action,lifecycle}.yaml` | conditional `no_error: literal(false)` |

## Completed values that changed

Documents that were valid before now complete differently:

| Input | Before | Now |
|---|---|---|
| Darkmatter `hash: {}` | `{}` | `{ kind: simple }` |
| Darkmatter `style.markdown: {}` | `{}` | every cleanup rule default |
| Claudine TTS mapping without `prewarm` | no `prewarm` | `prewarm: true` |
| Claudine user `steering.automatic` mapping | no `enabled` | `enabled: true` |
| An optional `number(default(3))` written as `null` | `null` | `3` |
| A declared, unwritten optional property in composition | synthetic `null` key | no key |

Absent and `null` parents without their own default gain nothing. Repository
steering gets no new default.

## Generated artifacts and identities

| Item | Before | After |
|---|---|---|
| SimplifiedSchema `PREPARATION_REVISION` | 2 | 3 |
| Darkmatter `PREPARATION_REVISION` | 3 | 4 |
| Claudine `PREPARATION_REVISION` | 3 | 4 |
| `PROJECTION_REVISION` | 1 | 2 (projection identity `e89539b57da19d99` → `2da28dfe4c5dcd41`) |
| `MERGE_ALGORITHM_REVISION` | unchanged | unchanged |

All three `lib/generated/shipped.json` artifacts were regenerated by the root
`just shipped-schemas`, which reports them current. 14 Darkmatter
recursive-schema goldens were re-blessed (all derived from lifecycle).
`claudine/schemas/globals.lifecycle.json` regenerated unchanged.

## Schema and API departures from the plan

The decisions list in `implementation-log.md` holds the full reasoning. In short:

- The schema migration and artifact regeneration moved from Phase 3 to Phase 1,
  because removing the grammar made the shipped schemas unparseable.
- `declared-required` still refuses a declaration with a null-admitting
  alternative.
- `EffectiveSchema::coerce_frontmatter_with_pending` keeps its "already valid"
  check. It only converts, and composition has already applied defaults.
- `data_copy` withdraws applied defaults instead of only losing its early
  return.
- Property-level union default conflicts are `SchemaError::Convert`, not
  `ConflictingDefaults`.
- `initialize` still runs before schema validation (documented order).
- Overflowing YAML numbers (`1e400`) load as text. Phase 5 corrected the
  SimplifiedSchema foundations and Darkmatter numbers pages to explain the
  YAML/text versus `.json` loader distinction.
- `yaml(not-empty)` / `json(not-empty)` judge the text, so `"{}"` is content.
- The first review's lifecycle gaps are repaired: an authored `no_error: false`
  beside `then`, `no_error: 1` on an `unknown`-typed md verb, and `{ when }`
  without an action or branch are refused by both schema validation and parsing.

## Validation coverage (input robustness matrix)

Default outcome: an invalid field or document returns a located public error,
runs nothing, and publishes nothing. The exceptions are in the plan's matrix.

| Validator or loader | Tests |
|---|---|
| Defaulted and required mapping properties | `simplified-schema/lib/tests/l1/required_defaults.rs` (`presence_table`, `null_and_falsey`, `parents`, `nesting`, `unions`, `input_shapes`, `raw_schemas`, `raw_schema_inputs`), `required_presence.rs` |
| `not-empty` | `not_empty_values.rs`; Darkmatter `not_empty_frontmatter.rs` |
| Envelope metadata and carriers | `envelope_admission.rs`, `envelope_null_carriers.rs` (selected and uncached modes) |
| Lifecycle flags and operands | Darkmatter `lifecycle/tests/authored_flags.rs`, `lifecycle_edit_schema_parity.rs`, `lifecycle_schema_parity.rs`; CLI `compose_lifecycle.rs` |
| Claudine overrides and keyed payloads | `config_keyed_entries.rs` (one test per route and action, plus a guard against the schema's unions), `config_schema_matrix.rs`, `config/wire_tests.rs` |
| Raw JSON Schema defaults | `required_defaults.rs::raw_schema_inputs`, `input_robustness.rs::json_files` |
| Loaders | admitted YAML `input_robustness.rs` + Darkmatter `yaml_document_loading.rs`; Markdown `frontmatter_extraction.rs`; Claudine JSON5 `config_json_conversion.rs` |
| CLI wiring | Darkmatter `compose_loader_wiring.rs`, `compose_schema.rs`; Claudine `config_json5_wiring.rs`, `config_yaml_wiring.rs` |

## Acceptance (spec "Verification")

| Spec item | Evidence | Result |
|---|---|---|
| Author-ruling table, strict and soft, borrowed vs owned, reports, unchanged input, repeat stability, falsey values, null defaults, unused vs used invalid defaults | `required_defaults.rs` (`presence_table`, `null_and_falsey`, `optional_invalid_defaults`, `strict_and_soft`) | pass |
| Parents, arrays of objects, recursive and imported types, hoisted markers, nullable unions, pending templates, union default rules | `required_defaults.rs` (`parents`, `nesting`, `unions`, `pending_templates`), `required_presence.rs`, `defaults.rs` unit tests | pass |
| Exported JSON Schema under a plain validator; raw JSON Schema inputs | `a_plain_validator_still_rejects_an_absent_defaulted_required_property`, `raw_schemas::*` | pass |
| Trigger matching and launch/completion/generated presence | `triggers/matcher.rs`, `triggers/lint.rs`, Darkmatter `schemas_effective_presence.rs`, Claudine `defaults_tests::launch` | pass |
| Envelopes and Claudine override inheritance and key counts | `envelope_admission.rs`, `envelope_null_carriers.rs`, `config_schema_matrix.rs`, `config_keyed_entries.rs`, Darkmatter key-count tests | pass |
| `md schema validate`, composition, whole YAML, DMLS; lifecycle parity in schema and parser | CLI `compose_schema.rs`, `composition_defaults.rs`, DMLS `required_defaults.rs`, `lifecycle_*_schema_parity.rs`, `authored_flags.rs` | pass |
| `not-empty` per type, `min(1)` equivalence, interpolation that produces nothing | `not_empty_values.rs`, `not_empty_frontmatter.rs::an_interpolation_that_produced_no_yaml_is_refused` | pass |
| Declared is not written (parent/child `title` table) | `absent_property_contract.rs` (`an_unwritten_parent_property_is_missing_in_a_child_that_requires_it`, `a_written_null_fails_a_string_child_s_type`, `a_written_null_meets_an_unknown_child_s_required_contract`, `an_unwritten_parent_property_renders_empty_without_a_warning_or_a_key`, `a_written_title_reaches_both_children`) | pass |
| Stale `non-null` / old-contract search | Phase 5 audit (log) | pass |
| `just test` and `just lint` in all three areas | see below | see below |

## Validation evidence (final tree, Phase 6)

The gates ran on macOS, one area at a time, on the integrated tree (Phases 1–5
committed, plus Phase 6's doctest fix). Load average was 6–17 on 16 cores.

| Area | `just test` | `just lint` | `just doctest` | `check-tier-coverage` |
|---|---|---|---|---|
| `simplified-schema` | 2237 L1 + 1878 `--no-default-features`, all passed, none at or above 5.0 s (rerun after the doctest fix, same result) | exit 0 | 40 passed, 3 ignored (after fixing one stale import) | 0 stranded |
| `darkmatter` (library, CLI, DMLS) | 16657 passed, 10 skipped by the recipe, none at or above 5.0 s | exit 0 (includes the `zed-dmls` wasm32 build) | 214 passed, 8 ignored | 0 stranded |
| `claudine` (catalog types, library, contract, CLI, gen) | 12164 passed, 15 skipped by the recipe, none at or above 5.0 s | exit 0 | all passed | 0 stranded |

- `just slow-tests fresh`: exit 0, current for the working tree.
- Root `just shipped-schemas`: "every shipped artifact is current", no diff.
- The single Phase 6 failure was the doctest on
  `simplified_schema::triggers::PayloadMemoTally`. It still imported the type
  from `darkmatter`, left over from when the trigger code moved out of
  Darkmatter, so it was not caused by this fix. The import now uses
  `simplified_schema::triggers::PayloadMemoTally`.
- **Not tested here:** Linux, native Windows, and WSL2. No cross-OS run was
  made: across all six phases, no path, process, or `#[cfg]` code changed, and
  every new test builds its paths from temp dirs. CI proves Linux on the pull
  request; the nightly proves Windows and WSL2. No L2, L3, or browser tier was
  needed. CLI wiring uses hermetic subprocesses, and lifecycle execution uses
  fake services.
- Unchanged input, no publication on failure, defaults visible to later reads,
  lifecycle schema/parser agreement, and cached/uncached parity are all asserted
  by tests in the suites above. The tests are listed in "Validation coverage"
  and in each phase's log section.

implementation complete, ready for review.

## Review record correction — 2026-11-15

The departures list incorrectly said the two overflow-number documentation
pages were left unchanged. Both pages already explain that YAML loads `1e400`
as text and `.json` file loaders reject it. Corrected that statement after
reading the current pages and rerunning SimplifiedSchema's L1 suites,
including `setter_values::spellings_yaml_reads_as_strings_are_never_rounded`.

## Review 2 record correction — 2026-11-15

The record still described the first review's lifecycle disagreements as current.
Darkmatter's full `just test` run (16,730 passed, no tests at or above 5 s)
verified the replacement agreement tests, including
`side_effect_numeric_flag_is_refused_by_both`,
`md_conditional_1_authored_false_flag_is_refused_by_both`, and
`bare_when_is_refused_by_both`. Corrected that statement above.

Also narrowed the blanket default-filling claim after public
`SimplifiedSchema::validate_strict` / `validate_soft` probes showed that
`'<string>': number(default(3))` still refuses an existing null value, and that
some overlapping selectors need a second preparation pass. These remaining
defects were inventoried in review 2 as R2-F1 and have since been repaired.
Qualified the parent statement:
a parent with its own default can be created; the passing literal-parent
control produced the parent and its reachable child default in one pass.

## Review 3 record correction — 2026-11-15

The behavior summary still called review 2's selected-entry defaults and combined
selectors defective. All 20 repair-ledger instances now check out against the
implementation and the passing `pattern_defaults` tests in both library builds
(2,290 feature-enabled tests and 1,931 minimal tests). Updated the summary in place.
Also qualified the blanket compile-time-conflict claim: a direct public
`Prepared::new` call accepted two overlapping patterns with defaults 1 and 2,
then `validate_soft` on `{ "x-settings": null }` returned
`Unprepared(ConflictingDefaults)`. Separate public-API probes found the
required-child/default-validation gap recorded as R3-F1; that is not one of the
previously repaired reproductions.

## Review 4 record correction — 2026-11-15

The behavior summary still described review 3's original conjunctive cases as
unrepaired. All 18 child rows in that repair's ledger now check out: the two
SimplifiedSchema feature builds pass their regressions, the guidance was
updated, and Darkmatter's full L1 run keeps the repaired corpus test below
five seconds. Corrected that statement in place. Public `SimplifiedSchema`,
`Prepared`, and `prepare_defaults` probes separately showed that an independent
union's required-child constraints can still be skipped before completing a
parent default. That newly found sibling is R4-F1, rather than a failure of
one of review 3's listed reproductions.

## Review 5 record correction — 2026-11-15

The behavior summary still described review 4's original independent-union
cases as unrepaired. All 22 ledger children now check out against the code
and the two SimplifiedSchema builds (2,325 feature-enabled and 1,966 minimal
tests). Corrected that statement in place. Separate public API probes,
in both builds, found that replacing the second object alternative with
`string` still permits the invalid parent default to be repaired. Direct
`DefaultPlan` preparation and `Prepared` validation reproduce that sibling;
a plain validator rejects the uncompleted default and accepts the completed
object. Review 5 records it as R5-F1, rather than reopening the earlier
object-only reproductions. The same route sweep also verified that a closed
object alternative's Boolean catch-all denial is dropped, producing the same
bypass; explicit false property and pattern schemas remain clean controls.

## Review 6 record correction — 2026-11-15

The behavior summary still said review 5's non-object and key-forbidding union
alternatives bypassed strict default checking. That statement was no longer
true after the latest repair. Corrected it in place: all 27 R5-F1 instances
and controls now check out. Verified by reading the selecting and admitting
code, running SimplifiedSchema's canonical L1 suites (2,332 feature-enabled
and 1,973 minimal tests), and running temporary public-API probes in both
builds. Each build passed 252 field/API checks plus 520 route controls,
including nested `allOf`/`anyOf`/`oneOf`, local references, closed alternatives,
authored values, and false property/pattern definitions. The probes were
removed afterward. The spec's explicit approximation boundary remains in
force; this correction does not claim general parent satisfiability checking.
