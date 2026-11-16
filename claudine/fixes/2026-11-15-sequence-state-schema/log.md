---
implementation_1: "2026-11-15T19:23:02-08:00"
implementation_2: "2026-11-15T22:00:27-08:00"
implementation_3: "2026-11-15T22:46:46-08:00"
---

# Sequence state schema: review-fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-15T19:23:02-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs; no `human_in_the_loop` items, no blocked findings, no recurrence
- findings are worked serially, one foreground subagent per finding, in review order (R1-F1 → R1-F5)

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Referring-document properties still enter referenced templates | fixed | both fallback merges removed (`source.rs::plan_referenced_sequence`, `inputs.rs::Discovery::walk_referenced`); all children verified |
| R1-F1.1 | Root whole-document reference: child omits `reviewer` → `none` | fixed | `sequence_referenced_state::a_root_referenced_template_never_reads_the_referrers_property`; lib `document::tests::a_referenced_template_never_reads_a_value_only_its_referrer_defines` (fail on old code) |
| R1-F1.2 | Nested whole-document reference → `none` | fixed | `sequence_referenced_state::a_nested_referenced_template_never_reads_the_referrers_property` |
| R1-F1.3 | Referenced sequence in a group leg → `none` | fixed | `sequence_referenced_state::a_group_leg_referenced_template_never_reads_the_referrers_property` |
| R1-F1.4 | Referenced sequence via task file → `none` | fixed | `sequence_referenced_state::a_task_file_referenced_template_never_reads_the_referrers_property` |
| R1-F1.5 | Input discovery / prompt selection uses child fallback, not referrer value (root and nested) | fixed | `preflight::inputs::tests::a_referenced_template_never_selects_a_prompt_from_its_referrers_values` (absolute paths; root and nested) |
| R1-F1.6 | Direct owner control → `none` | clean | `sequence_referenced_state::a_directly_run_template_reads_its_fallback` prints `none` |
| R1-F1.7 | Owner value / setter precedence controls → `own`, then `caller` | clean | `…::a_referenced_template_reads_its_own_property_over_nothing_of_the_referrers` (`own`), `…::a_setter_fills_a_referenced_template_over_its_own_property` (`caller`) |
| R1-F1.8 | Referrer never satisfies a referenced required input | clean | `a_referrers_property_never_satisfies_a_referenced_documents_input` passes |
| R1-F1.9 | Research fleets migrated off the fallback; fallback-pinning test, docs, and skills updated | fixed | 22 `research_prompt` fleets moved to root `initialize` `set`; `research_sequence_artifacts::*` (7) pass; dry-run stdout byte-identical vs. original with value as setter; docs/skills/comments updated, `just publish-skills` run. Decision: fleet `--dry-run` needs a `research_prompt=` setter because dry run fires no `initialize` (documented; alternative of making dry run evaluate `initialize` rejected as a dry-run behavior change; listed in review `human_review_items`) |
| R1-F2   | Relative task references and file inputs lose their authoring folder | fixed | root-row origins now come from the declaring document in both `Loader::walk_plan` and `Discovery::walk_referenced`; `with:` origins carried through approval and the step barrier; two extra sibling sites fixed (step barrier `WrapperPromptRunner::validate`; `prepare_nested_{sequence,group}` reread without invocation through symlinked paths) |
| R1-F2.1 | Root-reference `prompt: ./p.md` resolves beside declaring child | fixed | lib `inputs::tests::a_documents_relative_references_resolve_beside_it_through_a_root_reference` (all four kinds, inventory + graph; fails with fix reverted); CLI `sequence_sources_cli::a_sequence_documents_relative_references_resolve_beside_it_on_every_route` |
| R1-F2.2 | Root-reference `task: ./t.yaml` resolves beside declaring child | fixed | same two tests (`task: ./t.yaml`) |
| R1-F2.3 | Root-reference `sequence: ./s.yaml` resolves beside declaring child | fixed | same two tests (`sequence: ./s.yaml`) |
| R1-F2.4 | Root-reference `group: ./g.yaml` resolves beside declaring child | fixed | same two tests (`group: ./g.yaml`) |
| R1-F2.5 | Direct/nested controls for the four executable references | clean | direct and nested controls inside the same two tests pass before and after |
| R1-F2.6 | `with.brief: ./spec.md` accepted author-relative, direct child | fixed | `with_file_provenance::a_sequence_task_with_file_resolves_beside_its_author_on_every_route` (direct route; fails when approval or barrier fix is reverted) |
| R1-F2.7 | Same, nested whole document | fixed | same test, root-reference and nested routes |
| R1-F2.8 | Absolute `with.brief` controls | clean | `with_file_provenance::an_absolute_sequence_task_with_file_is_the_same_on_every_route` |
| R1-F2.9 | Runtime `with:` preparation names child-adjacent file | clean | `with_file_provenance::a_sequence_task_with_file_prefers_its_authors_folder_over_the_launch_folder` renders `sub/notes.md`. Decision: offset/operator data selections keep their existing origin rule (review places them out of this finding); root rows still typed by the source schema |
| R1-F3   | Nested templates read raw document values instead of prepared values | fixed | `Loader::owner_template_values` reuses discovery-prepared owner values (prepares once per owner without discovery); all children verified. Runtime `task/sequence.rs::sequence_frontmatter` for referenced documents handed to R1-F4 (runtime preparation boundary) |
| R1-F3.1 | Root inline normalizer control (literal/absent-default/null-default/derived → `ada`) | clean | lib `preflight::tests::template_owner_values::every_inline_template_reads_its_owners_prepared_values` (6 sites × 4 variants × 3 graph builders; 48/72 fail pre-fix) — root cells pass before and after |
| R1-F3.2 | Inline nested normalizer → `ada` in all four | fixed | lib `preflight::tests::template_owner_values::every_inline_template_reads_its_owners_prepared_values` (6 sites × 4 variants × 3 graph builders; 48/72 fail pre-fix); CLI `sequence_referenced_state` template_fill absent/null/derived cases print `ada` |
| R1-F3.3 | Group leg's inline sequence → `ada` in all four | fixed | lib `preflight::tests::template_owner_values::every_inline_template_reads_its_owners_prepared_values` (6 sites × 4 variants × 3 graph builders; 48/72 fail pre-fix) |
| R1-F3.4 | External task file's inline sequence → `ada` in all four | fixed | lib `preflight::tests::template_owner_values::every_inline_template_reads_its_owners_prepared_values` (6 sites × 4 variants × 3 graph builders; 48/72 fail pre-fix) |
| R1-F3.5 | Inline child of a referenced whole document → `ada` in all four | fixed | lib `preflight::tests::template_owner_values::every_inline_template_reads_its_owners_prepared_values` (6 sites × 4 variants × 3 graph builders; 48/72 fail pre-fix) (nested and root references); CLI `a_referenced_documents_inline_template_reads_its_own_default` |
| R1-F3.6 | Input discovery's inline scope projection keeps prepared values | clean | `walk_executable` already kept prepared values; the graph now reuses them (same matrix) |
| R1-F3.7 | `$state` validation / shell consumption see prepared `ada` | fixed | rows freeze with `ada`, `$state` accepts, CLI tests assert real shell output; `setters_layer_over_prepared_values_and_an_item_field_shadows_them`, `a_literal_template_prepares_no_owner_values` |
| R1-F4   | Referenced document schemas are not checked against current runtime values | fixed | `TaskExecution::referenced_owner_values`, `PreflightGraph::prepare_whole_document`, CLI `judge_referenced_root_document`; all children fixed/clean, discovered siblings R1-F4.10–R1-F4.13 fixed, R1-F4.14 disputed (pre-existing, outside the spec) |
| R1-F4.1 | Root whole-document reference: write `rank: bad` then shell → rejected before shell | fixed | CLI `sequence_referenced_state::a_root_reference_judges_its_document_after_its_own_row_writes_an_invalid_value` (fails pre-fix) |
| R1-F4.2 | Nested whole-document entry → rejected | fixed | CLI `a_nested_reference_judges_…`; lib `task::tests::referenced_owner_schema::an_earlier_invalid_write_fails_…` (both fail pre-fix) |
| R1-F4.3 | Whole document in group leg → rejected | fixed | CLI `a_group_leg_reference_judges_…` (fails pre-fix) |
| R1-F4.4 | Whole document via external task file → rejected | fixed | CLI `a_task_file_reference_judges_…` (fails pre-fix) |
| R1-F4.5 | Direct document control → rejected | clean | `a_directly_run_document_judges_its_own_invalid_write` passes before and after |
| R1-F4.6 | Inline container check control → rejected | clean | `an_inline_container_check_judges_an_earlier_invalid_write` passes before and after |
| R1-F4.7 | Valid runtime value control (`rank: 2`) → runs | clean | four `*_runs_its_document_after_a_valid_write` tests, one per route |
| R1-F4.8 | Initial referenced-owner inventory accepts valid authored value | clean | four `*_accepts_its_documents_valid_authored_values` tests |
| R1-F4.9 | Shell-only and empty referenced-container coverage | fixed | shell-only fixture used by every rejection test; `an_empty_nested_document_…` (fails pre-fix), `an_empty_root_referenced_document_…` control; plus `a_looping_referenced_document_is_judged_again_at_its_next_pass`, `a_referenced_documents_events_and_children_read_its_prepared_values` (R1-F3 handoff, fails pre-fix) |
| R1-F4.10 | (discovered) root-reference rows bind shell placeholders to the invoking root's values at preflight (`rank` reads missing) — same owner class as R1-F1 | fixed | `Loader::seed_steps` seeds root-reference rows from the declaring document; CLI `judge_referenced_root_document` returns child prepared values used by `run_step_task`; lib `preflight::tests::prepared_shell_values` + CLI `sequence_referenced_state` direct/root-ref/nested agreement tests (fail pre-fix). Base `main` had the same defect |
| R1-F4.11 | (discovered) directly run root's own events read authored values plus setters (no defaults) while referenced documents' events now read prepared values — direct/referenced parity | fixed | CLI `sequence/mod.rs::root_event_values` prepares root event values before `initialize` (setters, no shell, verdict withheld) and at each `start` (`RootRun::refresh_event_values`); CLI agreement tests fail pre-fix. Decision: verdict withheld for root events (root steps already judge; avoids early refusal of `initialize`-supplied values); alternative of judging at `start` rejected |
| R1-F4.12 | (discovered) preflight shell binding refuses a row reading a schema-defaulted owner value as missing on every route | fixed | `Loader::prepared_view` lays prepared owner values (shared cache with `owner_template_values`) over the authored view; keys whose authored value is a template stay authored (preserves late provider hints and parse-count pins); `prepared_shell_values` lib tests (5/6 fail pre-fix) |
| R1-F4.13 | (discovered, reproduced by orchestrator) a root `initialize` `set` does not satisfy a root `$schema` required input: `who: string(required)` + `initialize: [{ set: { who: ada } }]` fails pre-flight "Missing required schema properties … `who`" | fixed | `phase1c.rs::run_phase_1c_{with_schema,attempt}` now take the root runtime snapshot (`None` in dry run) so preflight matches step composition; 8 CLI tests in `sequence_referenced_state.rs` (`a_root_initialize_write_supplies_the_*_required_input`, no-write control, dry-run note); YAML and Markdown root cases fail pre-fix. Markdown form was also defective on `main` |
| R1-F4.14 | (discovered) a derived document property (`greeting: "hi {{ name }}"`) read by a shell row binds as literal text at shell preflight, on every route | disputed | Pre-existing on `main` on every route and not an R1 finding; the spec's prepared-value rule governs templates and schema preparation, not shell early binding, and rendering derived values early would bind environment/provider-dependent values at launch. No spec sentence or regression authority |
| R1-F5   | Edited inline checks reject containers after initialization has already run | fixed | `admit_inline_container` + `preflight::reread_inline_check` judge the current declaration before the run opens (declaration cache reused, graph frozen); discovered proxy sibling tracked as R1-F5.8 |
| R1-F5.1 | Inline container after editing sibling → failure, no `INIT-RAN` | fixed | lib `task::tests::stale_admission::a_check_tightened_by_an_earlier_sibling_…`; CLI `sequence_frozen_graph::a_check_tightened_by_an_earlier_step_…` (both fail pre-fix) |
| R1-F5.2 | Inline sequence in group leg → no `INIT-RAN` | fixed | lib + CLI `a_tightened_check_on_a_group_leg_…` (fail pre-fix) |
| R1-F5.3 | Inline sequence in external task file → no `INIT-RAN` | fixed | lib + CLI `a_tightened_check_in_a_task_file_…` (fail pre-fix) |
| R1-F5.4 | Inline child of referenced sequence document → no `INIT-RAN` | fixed | lib + CLI `a_tightened_check_in_a_referenced_document_…` (fail pre-fix) |
| R1-F5.5 | Unchanged invalid declaration control → rejected, no marker | clean | lib `an_unchanged_rejecting_check_…`; existing CLI `an_inline_check_is_never_an_input_…` |
| R1-F5.6 | False-guard control → exit 0, nothing runs | clean | lib `a_false_guard_neither_rereads_nor_judges_the_check`; existing `inline_check::a_false_guard_skips_the_check` |
| R1-F5.8 | (discovered) container proxy `start` reread treats a whole target document's root `$schema` as an inline check | fixed | `prepare_nested_sequence` reads a self-whole-document task (proxy target) as a reference to itself, so its root `$schema` is the document schema judged at `start`; `task::tests::referenced_owner_schema` proxy cases (2, fail pre-fix) |
| R1-F5.7 | Retry/loop start checks keep existing boundaries | clean | lib `the_first_start_still_judges_what_initialize_wrote`; existing loop-pass and CLI retry tests pass. Also fixed: a loosened check was wrongly refused (`a_check_loosened_…`); malformed edited declaration is a typed declaration error |

### Progress

- starting the work on 'R1-F1: Referring-document properties still enter referenced templates' at 19:24:00
        - defect class: a referenced owner's value projection includes another document's authored properties
        - sibling sweep: the referrer layer existed at exactly two sites, `source.rs::plan_referenced_sequence` (shared by root, nested, group-leg, task-file, and graph `Loader` routes) and `preflight/inputs.rs::Discovery::walk_referenced`; both removed. `expr.rs::SourceExpressionLookup` reads the referrer for `::operator` data on purpose and is kept; `task/sequence.rs`, `document.rs`, `preflight/mod.rs` and the CLI build no referrer merge of their own
        - migrated the 22 `research_prompt` fleets to a root `initialize: [{ set: { research_prompt: … } }]` write; the other fleets' rosters (`providers.yaml`, `multiplexers.yaml`, `terminal-apps.yaml`) template only row fields and need nothing
        - discovery: `--dry-run` fires no lifecycle events, so a fleet's `initialize` write is absent in a dry run and the roster's `prompt` reads `null` (`must be a non-empty file reference, got null`); a real run is unaffected. Documented the setter preview (`research_prompt=…`) in the sequences topic page, the caller skill, `_TEMPLATE.md`, and `research-contracts.md`'s traps table
        - discovery: `agent-cli` and `reasoning-level` fleets fail `--dry-run` identically before and after the migration (`seq.index % 3` in their `_research.md` `assigned:` — "Remainder requires numeric operands"); pre-existing, unrelated to R1-F1, not repaired here
        - fixed: both fallback merges removed; replaced `a_referrer_value_is_only_a_lowest_layer_under_a_referenced_documents_own` with `a_referenced_template_never_reads_a_value_only_its_referrer_defines` and `a_referenced_template_reads_the_referrers_values_below_its_own` with `a_referenced_template_never_selects_a_prompt_from_its_referrers_values`; added eight CLI route/control tests in `sequence_referenced_state.rs`; both new library tests and the four CLI route tests fail on the old code, the controls pass on both
        - fixed: `research_sequence_artifacts::shipped_fleet_graphs_resolve_per_row_prompt_lifecycles` now requires the one `initialize` `set` of `research_prompt` and fills plans with it as the runtime layer; dry-run parity over all 22 fleets (migrated vs. original text, value supplied as a setter) is byte-identical on stdout
        - completed R1-F1; `just lint` passes; focused slices green
- work completed for 'R1-F1: Referring-document properties still enter referenced templates' at 19:41:25
- starting the work on 'R1-F2: Relative task references and file inputs lose their authoring folder' at 19:42:00
        - defect class: authoring-file provenance is lost before a relative file reference is resolved or validated
        - fixed (graph): `Loader::walk_plan` resolved root rows from the invoking source; it now uses `SequencePlan::document_scope.declaring_path` (canonical spelling), entering and capturing that file. A data selection (offset/operator) has no scope and keeps the root's origin, unchanged. `owner_schema` still types root rows with the source's schema (their step state is the source's)
        - fixed (inventory): `Discovery::walk_referenced` used the referrer for a root reference's whole-document rows; it now uses the document; selections unchanged. `walk_owner_list` lost its redundant origin parameter
        - downstream: every later consumer (task origin, `run_step_task`/`root_step_guard` contexts, prompt `with` evaluation, provider prediction `with_origin`, shell sites, group sources, prompt references) reads `PreflightTask.origin_path`, so the one origin fixes them together; `collect_sites` body steps and Phase 1c compose the root document itself and are clean
        - fixed (passive approval): `approve_preflight_graph` added `with:` as data only, so its files resolved from the launch folder; `PromptReference` now carries `with_origin` and approval installs `with` caller records from it, matching runtime
        - discovery + fixed (step barrier): `WrapperPromptRunner::validate` had the same loss (step caller records only); once approval passed, the barrier refused `./only-here.md` ("Schema validation failed"). `run` and `validate` now share `task_caller_records`. Each of approval and barrier fails the new CLI test on its own when reverted
        - discovery + fixed: a nested sequence or group reread at container start (`prepare_nested_{sequence,group}`) resolved through one context with no invocation, so a sequence named through a symlinked path (macOS `/var`/`/tmp`, any symlinked checkout) failed with "repository root … does not contain the resolution source"; it now resolves through the task's `InvocationContext` as the launch walk does. Same failure on the pre-fix nested route, independent of `with:`
        - discovery: `sequence_grammar_matrix::{json,json5}::nested_loop_is_accepted` pinned the root-reference defect as a reread limitation ("nested sequence disappeared before preparation"); both now run (`Runs(2)`). The JSONL/NDJSON selection cells keep it, since a selection has no document of its own
        - tests: lib `inputs::tests::a_documents_relative_references_resolve_beside_it_through_a_root_reference` (fails pre-fix in discovery and, with discovery fixed, in the graph); CLI `sequence_sources_cli::{a_sequence_documents_relative_references_resolve_beside_it_on_every_route, a_nested_document_reached_through_a_symlinked_folder_starts_and_runs_its_rows}`, `with_file_provenance::{a_sequence_task_with_file_resolves_beside_its_author_on_every_route, an_absolute_sequence_task_with_file_is_the_same_on_every_route, a_sequence_task_with_file_prefers_its_authors_folder_over_the_launch_folder}`
        - docs: sequences topic (step references follow the authoring-file rule), composition provenance table, `use-claudine` sequences page (published), repo `claudine` skill
        - completed R1-F2; `just lint` passes; lib (1152) and CLI (1291) focused slices green
- work completed for 'R1-F2: Relative task references and file inputs lose their authoring folder' at 20:02:28
- starting the work on 'R1-F3: Nested templates read raw document values instead of prepared values' at 20:03:00
        - defect class: a value projection drops schema defaults and leaves derived expressions unevaluated before nested template normalization
        - root cause: `Loader::load_sequence_with` filled an inline list's template from `step_frontmatter`, the shell early-binding view (authored root frontmatter plus setters and the step overlay; inside a referenced document, its authored values plus setters). Through a root `sequence:` reference it was even the referrer's view, so the literal variant failed there too
        - fixed: the `Loader` tracks the owning document it is inside (`owner`: the invoking document, the root reference's document, or a nested whole referenced document; task and group files are not owners) and fills inline templates from `owner_template_values`: discovery's `InputOwner::values` when the graph follows discovery (`build_preflight_graph_after_discovery`), otherwise `document::owner_values` prepared once per owner; nothing is prepared for a template with no string default (decision 7 kept)
        - sibling sweep: root plan (`resolve_sequence_plan_with` in Phase 1c) and Phase 1c step composition prepare the root — clean; discovery's inline scope (`walk_executable`) already used prepared owner values — clean; reread paths (`prepare_nested_{sequence,group}`, `reread_preflight_graph_with_invocation`, `refresh = true`) fill no template and keep frozen rows — unchanged; a whole referenced document's own template (`plan_referenced_sequence`) already prepared — clean
        - discovery (not repaired, adjacent): a referenced document's runtime state (`task/sequence.rs::sequence_frontmatter`, used for its events, its children's task state, and an inline `$schema` check inside it) and its graph shell early-binding view are still authored values plus setters; root-step state at runtime is the composed (prepared) root. Runtime preparation of referenced owners is R1-F4's repair boundary; left for it
        - tests: lib `template_owner_values_tests::every_inline_template_reads_its_owners_prepared_values` (6 sites × 4 variants × 3 graph builders; 48 cases fail on the pre-fix projection, root control passes), `setters_layer_over_prepared_values_and_an_item_field_shadows_them`, `a_literal_template_prepares_no_owner_values`; CLI `sequence_referenced_state::{an_inline_template_reads_an_absent_owner_value_by_its_default, an_inline_template_reads_a_null_owner_value_by_its_default, an_inline_template_reads_a_derived_owner_value, a_referenced_documents_inline_template_reads_its_own_default}` (all four fail pre-fix, print `ada` from the shell after)
        - docs: sequences topic (inline container templates read prepared values, with example), `use-claudine` sequences page (published), repo `claudine` skill
        - completed R1-F3; `just lint` passes; lib focused slice 907 and CLI focused slice 1302 green
- work completed for 'R1-F3: Nested templates read raw document values instead of prepared values' at 20:16:32
- starting the work on 'R1-F4: Referenced document schemas are not checked against current runtime values' at 20:16:32
        - defect class: schema validation is performed on an initial document snapshot but omitted at the later preparation boundary where runtime values have changed
        - reproduced: nested, group-leg, task-file and root-reference routes all printed the observing shell and exited 0 after an earlier `rank: bad` write; the direct-document control refuses at the next step's composition (`iterate.rs::run_one_step` → `jit::compose_step`)
        - fixed (lib): `TaskExecution::referenced_owner_values` (replaces `sequence_frontmatter`) prepares a whole referenced document through `PreflightGraph::prepare_whole_document` (current file, invocation parse memo, declaring document's derived context via `document::owner_resolution`), under the graph's caller layers (new `PreflightGraph::caller_layers`, recorded by `build_preflight_graph_after_discovery`) plus runtime writes as data; verdict deferred before `initialize`, judged at each `start` reread (nested entry, group leg, task file, loop pass, retry, container proxy); a dry run only prepares. A refusal blocks the started container like any reread failure
        - fixed (CLI): `iterate.rs::judge_referenced_root_document` judges a root `sequence: child.yaml` document at each root step, after the step's composition, with that composition's layers and context: the same boundary the direct control uses
        - handoff R1-F3 closed: the referenced owner's events, children's task state and inline checks inside it now read prepared values (defaults, coercion) layered with setters and runtime writes
        - discovery (not repaired, adjacent): preflight shell early binding reads authored values only, so a shell row reading a schema default (`{{ who }}`) is refused as missing in every route, the direct root included; root-reference rows are also bound to the invoking root's values at preflight (`rank` missing). The direct root's own events read authored values plus setters (no defaults). Left as found; outside the schema-verdict boundary
        - sibling sweep: `run_sequence` (loop open, `initialize` events), every event and child state in `run_sequence_attempts` (Start reread, terminal events, loop gate, retry `set_batch`), `check_inline_container` inside a referenced document (reads the child state, now prepared) and `run_container_proxy` (enters `run_sequence`) all read the one prepared base; group legs and task files reach the same `run_sequence`; root reference judged in `run_one_step`; root-reference empty list covered by launch discovery with `initialize` writes (verified, control)
        - tests: lib `task::tests::referenced_owner_schema` (5; 3 fail pre-fix); CLI `sequence_referenced_state` 17 new (rejections per route, direct and inline controls, valid-write and authored controls per route, empty nested and empty root reference, `initialize` correcting, loop re-entry, prepared defaults/conversion and setter); 7 fail pre-fix, controls pass on both; none over 5 s alone
        - docs: sequences topic (judged at its turn, table by route), `use-claudine` sequences page (published), repo `claudine` skill, `PreflightSequence::frontmatter` rustdoc drift corrected; the stale `sequence_frontmatter` doc glued above `check_inline_container` removed
        - completed R1-F4; `just lint` passes; lib focused slice 958 and CLI focused slice 1604 green
- work completed for 'R1-F4: Referenced document schemas are not checked against current runtime values' at 20:39:42
- starting the work on 'R1-F5: Edited inline checks reject containers after initialization has already run' at 20:40:35
        - defect class: admission uses a stale preflight declaration and discovers the current rejecting declaration only after opening the container run
        - repair: `TaskExecution::run_validated` admits through `admit_inline_container`, which calls new `preflight::reread_inline_check` (reads the task's own declarations from its file as it reads now, via `NestedSources::load` and the `prepared_schema` declaration cache keyed by file+property, invalidated by declaration or schema-file bytes); children, approval, `$state`, template untouched; a failed admission stays an unstarted failure; an edited malformed declaration is `SequenceDeclarationShape`/`SequenceDeclarationLoad`, distinct from the `SequenceContainerSchemaViolation` value refusal; dry run still only names the frozen check
        - discovery: the stale admission also refused a container whose check an earlier task loosened (lib `a_check_loosened_by_an_earlier_sibling_admits_the_container` fails pre-fix); same fix
        - kept: every `start` reread still judges the check, including the first start (now judges the values after `initialize` wrote; cache makes the reread prepare nothing for an unchanged declaration, pinned by `admission_prepares_only_an_edited_declaration`)
        - sibling sweep: root steps (`task_run.rs` `run_admitted`), nested children (`run`), group legs (`run_guarded` in the leg worker), `kind: task` files and inline children of referenced documents all funnel through `run_validated`, so all use the reread; group objects cannot declare a check (placement error), so no analogue; whole referenced documents keep the deliberate start-time verdict (R1-F4 rule) and are skipped by `reread_inline_check` (non-inline `plan.source`); root `sequence: child.yaml` judged per root step after compose, not stale
        - discovery (not repaired, adjacent): container proxy admission keeps the approved check: its synthesized task has a root pointer, so a reread would read the target file's root, whose `$schema` is a document schema; proxy targets are whole documents in practice (no inline check). The proxy's own `start` reread (`prepare_nested_sequence` at the root pointer) classifies a whole document's root list as inline and so reads its root `$schema` as a check; pre-existing, left as found
        - tests: lib `task::tests::stale_admission` (10; 6 fail pre-fix: nested, group leg, task file, referenced document, loosened, malformed); CLI `sequence_frozen_graph` 5 new (4 fail pre-fix: root step, group leg, task file, referenced document; unedited control passes both), edits via portable `append_line`, `initialized`/`worked` probes read by the root `failure`; none over 5 s (max 0.34 s lib, about 1 s CLI)
        - docs: sequences topic (check reads the declaration as the file reads at the turn, example, malformed edit, first start after `initialize`), `use-claudine` sequences page (published), repo `claudine` skill
        - completed R1-F5; `just lint` passes; lib focused slice 871 and CLI focused slice 1828 green
- work completed for 'R1-F5: Edited inline checks reject containers after initialization has already run' at 20:53:56
- starting the work on 'discovered sibling rows R1-F4.10, R1-F4.11, R1-F4.12, R1-F5.8' at 20:53:56
        - R1-F4.10 (defect class: a referenced owner's rows are bound against another document's values): reproduced at preflight (`root.yaml` → `sequence: sub/child.yaml`, child `rank: 1`, row `echo {{ rank }}`: "reads `rank`, which is missing"; direct and nested print `rank=1`) and at run time (a late row read the root's composed state: `{"rank":null}` from a `set`, then `who` missing). Same on `main` (seed is the invoking root's authored frontmatter; the run state is the root's composition)
        - fixed (R1-F4.10, lib): `Loader::seed_steps` seeds root-reference rows from the declaring document (authored root via `entered_document`, then the setters, then its prepared values); `owner_schema` no longer types those rows with the source's schema (as for a nested reference); `root_rows_origin` removed
        - fixed (R1-F4.10, CLI): `iterate.rs::judge_referenced_root_document` returns the child's prepared values, which `run_step_task` uses as the row's state (`referenced`); the row's `when:` reads the child's authored values (`PreflightGraph::authored_document`), as a direct run's guard reads its own
        - R1-F4.12 (defect class: a value projection drops schema defaults/coercion before it is consumed): reproduced on direct, inline, nested and root-reference routes (`who: string(default(ada))`, `echo {{ who }}` refused as missing). Same on `main`
        - fixed (R1-F4.12): `Loader::prepared_view` → `layer_prepared_values` lays each owner's prepared values (`Loader::prepared_owner_values`, the discovery/template cache, prepared once per owner otherwise; a failed preparation keeps the authored view) over the authored view for the root events, the root-step seed, and a referenced document's events and children; a key whose authored value holds a template stays authored, so late roots and provider-hint classes are unchanged (the first full-overlay attempt broke `scaling::a_with_hint_keeps_its_class_through_document_keys` and the steps×holes×6 parse counts); written keys stay late; exact approval still fixes resolved bytes
        - R1-F4.11 (defect class: a value projection drops schema defaults/coercion before it is consumed): a direct root's events read authored values plus setters; referenced documents' events read prepared values since R1-F4. On `main` both read authored plus setters (the disagreement was introduced by R1-F4)
        - fixed (R1-F4.11, CLI): `sequence/mod.rs::root_event_values` prepares the root (verdict withheld, no shell, caller layers with records) before `initialize`, and again at each `start` from the reread file with collected values and runtime writes as data (`RootRun::refresh_event_values`); a preparation failure keeps the authored view; a dry run prepares nothing. Decision: verdict withheld at both points (alternative: judge at `start` as a referenced document is) because the root's verdict is already its steps' compositions' and a strict check would add a second, earlier refusal path for the same values
        - R1-F5.8 (defect class: a document schema is misread as a check-only container declaration): reproduced in lib (proxy target with `$schema` `rank: number(required)`, authored `rank`: refused as the caller's missing `rank`; defaults and conversion lost at `start`). Same on `main` in substance (the proxy reread read the root pointer as inline)
        - fixed (R1-F5.8): `prepare_nested_sequence` reads a task that is its own whole document (origin == frozen whole document) as a reference to itself (no ancestry self-entry), so `$schema` stays the document schema and `referenced_owner_values` prepares and judges it at `start`
        - discovery (not repaired): a derived property (`greeting: "hi {{ name }}"`) binds a first-position shell exactly as its authored text (`echo hi {{ name }}` runs) on every route, before and after this change and on `main`; rendering derived values early would bind env-/provider-dependent values under the launch environment, so it needs its own ruling. Phase 1c ignores root `initialize` writes when it reports a root's missing required input (an `initialize` `set` of a `string(required)` is still reported missing); pre-existing, outside these rows
        - tests: lib `preflight/tests/prepared_shell_values.rs` (6; routes Direct/RootReference/Nested/GroupLeg × 3 builders; 5 fail pre-fix, the setter control passes both), `task/tests/referenced_owner_schema.rs` +2 proxy cases (both fail pre-fix); CLI `sequence_referenced_state.rs` +9 (direct events/start/rows, root-reference rows, nested rows/events/start; 5 fail pre-fix, the nested event controls pass both); none over 2.1 s
        - docs: sequences topic (what a referenced sequence reads: every route, example; direct root events and preflight shell binding read prepared values), `use-claudine` sequences page (published), repo `claudine` skill; rustdoc drift fixed (`Loader::step_seed`, `enter_step`, `owner_values`, `owner_schema`, `PreflightSequence::frontmatter`, enclosing-view comment, test module header)
        - completed; `just lint` passes; lib focused slice 967 and CLI focused slice 1933 green; guard slices 309 lib / 413 CLI green
- work completed for 'discovered sibling rows R1-F4.10, R1-F4.11, R1-F4.12, R1-F5.8' at 21:26:32
- work completed for 'discovered sibling rows R1-F4.10, R1-F4.11, R1-F4.12, R1-F5.8' at 21:27:52
- starting the work on 'R1-F4.13: root initialize writes do not satisfy required root inputs' at 21:27:52
        - defect class: a required-input decision ignores runtime writes that are available at its preparation boundary
        - reproduced: a YAML root `$schema: { who: string(required) }` with `initialize: [{ set: { who: ada } }]` and `shell: "echo who={{ who }}"` failed with "sequence missing properties … Step 1: work … `who`"; the Markdown root form the same
        - cause: `prepare_root_plan` discovered under the setters plus the `initialize` writes (root not missing), the graph's shell binding read them too, but Phase 1c's per-step probes composed with `step_set_overrides(…, None)`, without the root's runtime state; execution then composes step 1 with `runtime_state.snapshot()`
        - base: same on `main` (`run_phase_1c_with_schema` passed `None` there too, and root `initialize` already ran before preflight), so pre-existing for the Markdown root; for YAML on `main` `$schema` typed the steps, so the route is new in substance
        - fixed (CLI): `run_phase_1c_with_schema`/`run_phase_1c_attempt` take `runtime: Option<&RuntimeSnapshot>`; `execute_sequence` passes `root.runtime.snapshot()` (its `initialize` writes; on a retry also the earlier attempt's writes, which that attempt's steps also compose against) and `None` in a dry run; the interactive status report reads the same layered values
        - sibling sweep: empty root (`prepare_root_plan`, writes already layered), root reference (discovery + `judge_referenced_root_document` at the step), nested reference and group leg (start-time preparation with runtime writes), and a root prompt task whose document requires the value (composed and launched) were already correct; verified as controls
        - dry run: fires no `initialize`, so the value is reported missing (documented; preview with the setter), consistent with the existing template note
        - kept: an input nothing writes is still the non-interactive missing report (control); a shell row reading a truly missing root input is refused first by pre-flight shell binding ("reads `who`, which is missing"), unchanged and pre-existing
        - tests: CLI `sequence_referenced_state` +8 (YAML root, Markdown root, empty root, root reference, nested reference, group leg, no-write control, dry-run control); YAML and Markdown root fail pre-fix, the rest pass on both; all under 4 s in a parallel run
        - docs: sequences topic ("Requiring caller input": routes, example, dry run), `use-claudine` sequences page (published), repo `claudine` skill (Phase 1c probes read the root snapshot)
        - completed; `just lint` passes; CLI focused slice 1445 green (6 slow in the loaded run, all 1.3–2.5 s alone); no library change, lib slice not run
- work completed for 'R1-F4.13: root initialize writes do not satisfy required root inputs' at 21:37:20
- starting area gates (`just test`, `just lint`, `just check-tier-coverage claudine`) with no subagents running at 21:37:20
        - full `just test` run 1: 1 failure, `claudine_schema_shipped_corpus::research_agent_cli_fleet` — the R1-F1 fleet migration added `initialize`, so `claudine.yaml` now selects the `agent-cli` and `reasoning-level` fleets; updated both expectations to `CLAUDINE_AND_SEQUENCE` (69/69 corpus tests pass); rerunning the full suite
- area gates completed at 21:48:07
        - `just test` (claudine, rerun after the corpus fix): 12,369 passed, 15 skipped, `🐢 L1 run recorded: 12369 test(s), none at or above 5.0s`
        - `just lint` (claudine): exit 0
        - `just check-tier-coverage claudine`: stranded 0
        - `just cross-check claudine --os windows`: **not run** — the `build-win` host failed its Cargo storage preflight (41.5 GiB free, 50 GiB required; automatic reclaim freed 0 GiB). No test executed there, so this is an unavailable prerequisite, not a Windows failure; `os_failing` is not set. Native Windows proof remains with the nightly CI leg

### Completion Report

- **outcome: complete** — elapsed about 2 h 30 min (19:23 → 21:48)
- parent findings: **5 total = 5 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed**
        - R1-F1 fixed: a referenced whole document's template reads only its own values, then setters, then runtime writes; the 22 `research_prompt` fleets migrated to a root `initialize` `set`
        - R1-F2 fixed: root-reference rows resolve `prompt`/`task`/`sequence`/`group` beside the declaring document; relative `with:` files keep their authoring origin through approval and the step barrier; the nested reread now resolves through the invocation (symlinked-path failure)
        - R1-F3 fixed: every inline container template fills from its owner's prepared values (defaults for absent and null, derived values), reusing discovery's preparation
        - R1-F4 fixed: whole referenced documents are prepared and judged against current runtime values at `start` (nested, group leg, task file, loop, retry, proxy) and at each root step for a root reference; discovered siblings R1-F4.10–R1-F4.13 fixed (root-reference row binding, direct-root event values, defaulted values in shell binding, `initialize`-supplied required root inputs)
        - R1-F5 fixed: an edited inline check is reread and judged before the run opens, so `initialize` never runs for a refused container; discovered sibling R1-F5.8 (proxy target `$schema` misread as an inline check) fixed
- discovered rows outside the parents' counts: R1-F4.14 **disputed** (derived property bound literally by shell preflight; pre-existing on `main`, no spec authority)
- one item added to the review's `human_review_items`: fleet `--dry-run` needs `research_prompt=…` because dry runs fire no `initialize`
- noted, not a finding: `agent-cli` and `reasoning-level` fleets fail `--dry-run` on `seq.index % 3` identically before and after this cycle
- no deferred performance measurement; no slow tests
- open finding/child IDs: none (R1-F4.14 disputed as above)

#### Changed files

- `.claude/skills/claudine/SKILL.md`
- `.claude/skills/claudine/research-contracts.md`
- `claudine/cli/src/commands/wrap/sequence/container.rs`
- `claudine/cli/src/commands/wrap/sequence/iterate.rs`
- `claudine/cli/src/commands/wrap/sequence/mod.rs`
- `claudine/cli/src/commands/wrap/sequence/phase1c.rs`
- `claudine/cli/src/commands/wrap/sequence/task_run.rs`
- `claudine/cli/tests/l1/research_sequence_artifacts.rs`
- `claudine/cli/tests/l1/sequence_frozen_graph.rs`
- `claudine/cli/tests/l1/sequence_grammar_matrix.rs`
- `claudine/cli/tests/l1/sequence_referenced_state.rs`
- `claudine/cli/tests/l1/sequence_sources_cli.rs`
- `claudine/cli/tests/l1/with_file_provenance.rs`
- `claudine/docs/cli/sequence.md`
- `claudine/docs/research/_TEMPLATE.md`
- `claudine/docs/research/acp/_fleet.md`
- `claudine/docs/research/agent-cli/_fleet.md`
- `claudine/docs/research/agent-errors/_fleet.md`
- `claudine/docs/research/agent-logging/_fleet.md`
- `claudine/docs/research/agent-models/_fleet.md`
- `claudine/docs/research/agent-permissions/_fleet.md`
- `claudine/docs/research/hooks/_fleet.md`
- `claudine/docs/research/local_runners/_fleet.md`
- `claudine/docs/research/mcp/_fleet.md`
- `claudine/docs/research/memory/_fleet.md`
- `claudine/docs/research/model-config/_fleet.md`
- `claudine/docs/research/non-interactive-sessions/_fleet.md`
- `claudine/docs/research/plugins/_fleet.md`
- `claudine/docs/research/reasoning-level/_fleet.md`
- `claudine/docs/research/resume/_fleet.md`
- `claudine/docs/research/signals/_fleet.md`
- `claudine/docs/research/skills/_fleet.md`
- `claudine/docs/research/slash-commands/_fleet.md`
- `claudine/docs/research/steering/_fleet.md`
- `claudine/docs/research/subagents/_fleet.md`
- `claudine/docs/research/system-prompt/_fleet.md`
- `claudine/docs/research/usage/_fleet.md`
- `claudine/docs/topics/composition.md`
- `claudine/docs/topics/execution-flow.md`
- `claudine/docs/topics/flow-control/sequences.md`
- `claudine/docs/topics/schemas.md`
- `claudine/docs/use-skills/use-claudine/sequences.md`
- `claudine/fixes/2026-11-15-sequence-state-schema/implementation-log.md`
- `claudine/fixes/2026-11-15-sequence-state-schema/plan.md`
- `claudine/fixes/2026-11-15-sequence-state-schema/spec.md`
- `claudine/lib/src/composition/sequence/document.rs`
- `claudine/lib/src/composition/sequence/document/tests.rs`
- `claudine/lib/src/composition/sequence/preflight/inputs.rs`
- `claudine/lib/src/composition/sequence/preflight/inputs/tests.rs`
- `claudine/lib/src/composition/sequence/preflight/mod.rs`
- `claudine/lib/src/composition/sequence/preflight/shape.rs`
- `claudine/lib/src/composition/sequence/preflight/tests/scoped_graph.rs`
- `claudine/lib/src/composition/sequence/source.rs`
- `claudine/lib/src/composition/sequence/task/group.rs`
- `claudine/lib/src/composition/sequence/task/mod.rs`
- `claudine/lib/src/composition/sequence/task/sequence.rs`
- `claudine/lib/src/composition/sequence/task/tests.rs`
- `claudine/lib/tests/l1/claudine_schema_shipped_corpus.rs`
- `claudine/schemas/README.md`
- `claudine/fixes/2026-11-15-sequence-state-schema/implementation.md`
- `claudine/fixes/2026-11-15-sequence-state-schema/log.md`
- `claudine/fixes/2026-11-15-sequence-state-schema/review-1.md`
- `claudine/lib/src/composition/sequence/preflight/tests/prepared_shell_values.rs`
- `claudine/lib/src/composition/sequence/preflight/tests/template_owner_values.rs`
- `claudine/lib/src/composition/sequence/task/tests/referenced_owner_schema.rs`
- `claudine/lib/src/composition/sequence/task/tests/stale_admission.rs`

#### Verification tied to findings

- R1-F1: `sequence_referenced_state::*referenced_template*`, `document::tests::a_referenced_template_never_reads_a_value_only_its_referrer_defines`, `preflight::inputs::tests::a_referenced_template_never_selects_a_prompt_from_its_referrers_values`, `research_sequence_artifacts::*`, `claudine_schema_shipped_corpus::*` (69)
- R1-F2: `inputs::tests::a_documents_relative_references_resolve_beside_it_through_a_root_reference`, `sequence_sources_cli::*`, `with_file_provenance::*`, `sequence_grammar_matrix::{json,json5}::nested_loop_is_accepted`
- R1-F3: `preflight::tests::template_owner_values::*`, CLI template-fill cases in `sequence_referenced_state`
- R1-F4: `task::tests::referenced_owner_schema::*`, `preflight::tests::prepared_shell_values::*`, CLI route/control/agreement/`initialize` cases in `sequence_referenced_state`
- R1-F5: `task::tests::stale_admission::*`, `sequence_frozen_graph::*tightened*`
- all of the above in the final full `just test` (12,369 passed), plus `just lint` and `just check-tier-coverage claudine`

## Implementation of Review Findings #2

> **started at:** 2026-11-15T22:00:27-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs; no `human_in_the_loop` items, no blocked findings; recurrence: R2-F1 is a sibling of R1-F3 (first-position shell binding, lifecycle command and shell-value preparation, approval verification, inline reread membership)
- one parent finding, worked by one foreground subagent

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1    | Derived document values reach first-position shell sites as literal template text | fixed | `layer_prepared_values` now replaces a templated key the preparation changed with its prepared value unless it is unstable; unstable templated keys stay authored and join the view's late keys. Evidence: rows R2-F1.1–R2-F1.23 |
| R2-F1.1  | Direct root shell row, first row reads derived `greeting` → prints `hi ada` | fixed | lib `prepared_shell_values_tests::a_shell_row_reads_a_derived_value_as_prepared_on_every_route` (Direct × 3 builders); CLI `sequence_referenced_state::a_direct_first_row_prints_a_derived_value`. Both failed before the fix (`echo hi {{ name }}`) |
| R2-F1.2  | Root whole-document reference shell row → `hi ada` | fixed | lib same test (RootReference × 3 builders); CLI `a_root_reference_first_row_prints_a_derived_value` |
| R2-F1.3  | Nested whole-document shell row → `hi ada` | fixed | lib same test (Nested × 3 builders); CLI `a_nested_first_row_prints_a_derived_value` |
| R2-F1.4  | Referenced sequence in a group leg → `hi ada` | fixed | lib same test (GroupLeg × 3 builders); CLI `a_group_leg_first_row_prints_a_derived_value` (failed before the fix) |
| R2-F1.5  | Referenced sequence through a task file → `hi ada` | fixed | CLI `a_task_file_first_row_prints_a_derived_value` |
| R2-F1.6  | Inline child of a directly invoked owner → runs `hi ada`, no approval-change refusal | fixed | CLI `a_direct_inline_child_runs_its_approved_derived_command` (failed before the fix with the approval-closed refusal) |
| R2-F1.7  | Inline child through a root reference → runs `hi ada` | fixed | CLI `a_root_reference_inline_child_runs_its_approved_derived_command` |
| R2-F1.8  | Inline child through a nested reference → runs `hi ada` | fixed | CLI `a_nested_inline_child_runs_its_approved_derived_command` |
| R2-F1.9  | Inline child through a group-leg reference → runs `hi ada` | fixed | CLI `a_group_leg_inline_child_runs_its_approved_derived_command` (failed before the fix) |
| R2-F1.10 | Inline child through a task-file reference → runs `hi ada` (and diagnostic not truncated) | fixed | CLI `a_task_file_inline_child_runs_its_approved_derived_command`; the refusal no longer occurs. The truncation reproduced pre-fix is the generic one-line step-failure status (`iterate.rs` `step N/M failed: …` via `Status::from_prose`), shared by every step failure and outside this defect class; no change |
| R2-F1.11 | Direct root lifecycle `start` shell → `hi ada` | fixed | CLI `a_direct_start_shell_prints_a_derived_value` (failed before the fix) |
| R2-F1.12 | Referenced lifecycle `start` shell, nested/group-leg/task-file → `hi ada` | fixed | CLI `a_nested_start_shell_prints_a_derived_value`, `a_group_leg_start_shell_prints_a_derived_value`, `a_task_file_start_shell_prints_a_derived_value` |
| R2-F1.13 | Inline-container lifecycle `start` shell, direct and nested owners → runs `hi ada` | fixed | CLI `a_direct_inline_container_start_shell_prints_a_derived_value` (failed before the fix), `a_nested_inline_container_start_shell_prints_a_derived_value` |
| R2-F1.14 | Inline group lifecycle `start` shell, direct and nested owners → `hi ada` | fixed | CLI `a_direct_group_start_shell_prints_a_derived_value` (failed before the fix), `a_nested_group_start_shell_prints_a_derived_value` |
| R2-F1.15 | Direct root lifecycle shell-valued write `$(echo {{ greeting }})` → captures `hi ada` | fixed | CLI `a_direct_shell_valued_set_captures_a_derived_value` (failed before the fix) |
| R2-F1.16 | Referenced lifecycle shell-valued write, nested/group-leg/task-file → captures `hi ada` | fixed | CLI `a_nested_shell_valued_set_captures_a_derived_value`, `a_group_leg_shell_valued_set_captures_a_derived_value`, `a_task_file_shell_valued_set_captures_a_derived_value` |
| R2-F1.17 | Later-position shell control on five routes → `hi ada` (clean) | clean | CLI `a_direct_later_row_prints_a_derived_value`, `a_nested_later_row_prints_a_derived_value` |
| R2-F1.18 | Literal property controls across routes → unchanged literal output (clean) | clean | lib `a_literal_value_still_binds_exactly_on_every_route` (12 cases), `a_shell_row_reads_an_authored_value_on_every_route`; CLI `a_literal_property_is_read_as_written_by_first_position_shells`; existing literal CLI cases in `sequence_referenced_state` pass |
| R2-F1.19 | `side_effect: { shell: … }` typed refusal (clean) | clean | CLI `a_side_effect_shell_reading_a_derived_value_is_refused_with_its_hint` |
| R2-F1.20 | Template/state consumers of derived owner value → `ada` (clean) | clean | `template_owner_values` (3/3), CLI template-fill cases in `sequence_referenced_state` pass unchanged |
| R2-F1.21 | Late `seq`/`loop`/`group`, provider/env, shell-derived reads stay late (must not be bound at launch) | fixed | lib `a_derived_value_reading_a_late_root_stays_late_on_every_route` (`seq`), `a_derived_value_reading_the_environment_stays_late_on_every_route` (`env`), `a_derived_value_reading_a_runtime_written_key_stays_late_on_every_route` (`start` `set`; previously froze `echo hi {{ name }}`), `a_derived_value_reading_a_shell_substitution_is_not_bound_on_every_route`; CLI `a_first_row_reads_a_derived_value_a_start_write_changes` (prints the written `hi ada` over authored `bob`; failed before the fix) |
| R2-F1.22 | Early-binding comment in `layer_prepared_values` corrected | fixed | `layer_prepared_values` doc comment rewritten (it no longer claims early binding evaluates authored text); `LayeredView` documents the late and replaced outputs; `.claude/skills/claudine/SKILL.md` early-binding sentence, `docs/topics/flow-control/sequences.md` late-value paragraph, and `docs/topics/pre-flight-checks.md` first-task bullet updated |
| R2-F1.23 | Sibling: provider prediction reading a replaced derived key (`with: { agent: "{{ choice }}" }`, `choice: "{{ pick }}"`) still follows its authored reads to a key a later-walked document writes → decided at launch | fixed | `Loader::prepared_replacements` re-adds each replaced key's authored value to a prediction's dependencies in `finish`; lib `providers::tests::literal_hints::a_hint_reading_a_prepared_derived_value_follows_its_authored_reads_to_a_write` (failed with the re-add disabled) |

- starting the work on 'R2-F1' at 22:01:10
    - defect class: the value view used to approve and freeze a shell command retains authored expression text in place of the document's resolved value
    - reproduced first: lib `a_shell_row_reads_a_derived_value_as_prepared_on_every_route` failed 12/12 (`echo hi {{ name }}`); with the fix disabled, CLI `sequence_referenced_state` failed 9 of the first 15 derived cases (first row, group leg, inline children, start shells, group start, shell-valued `set`, start-write case) before fail-fast stopped the run
    - cause confirmed: `early_binding_state` never evaluates a frontmatter value's own expressions, so `layer_prepared_values` keeping every templated key authored froze its text; the runtime fill at a later position reads the composed value, which is why later rows printed correctly and inline rereads (prepared values) saw a different command
    - repair (`claudine/lib/src/composition/sequence/preflight/mod.rs`): `layer_prepared_values(view, prepared, written)` replaces a templated key that preparation changed with its prepared value unless it reads, through Darkmatter's `derived_deferred_keys` over the whole view, a late root (`seq`/`loop`/`group`), `env`, a `$(…)` key, `agent`/`model`, or a key in `written`; `Loader::prepared_view` builds `written` with `providers::collect_written_keys` over the owner view, the invoking document, and the walk's `written_keys` so far
        - kept keys holding `{{` (not `$(`) are returned as late and joined to `step_derived`/`seed_derived`/the referenced view's derived set, so their commands are approved by shape and filled with the value they run with (this also fixes the pre-existing freeze of a derived key a `start` `set` feeds)
        - the scan runs only when some templated key changed, so `scaling_tests::counted::*` parse counts are unchanged (they failed at 96 vs 72 when the scan was unconditional)
        - approval and verification agree because both read the same basis frontmatter (`ApprovalBasis.frontmatter`); runtime writes over a replaced key's inputs cannot occur, since such keys stay authored and late
        - alternative rejected: resolving derived values inside `early_binding_state` (per step, per verify) would add parses on every step and change the pinned scaling counts; and a blanket replace of every templated key would freeze late, environment, and runtime-written reads
    - sibling sweep: provider prediction reads the same `step_frontmatter`; a replaced key lost its authored reads, so `defer_runtime_written` could miss a write to its input made by a document walked later; fixed by `Loader::prepared_replacements` (R2-F1.23). `owner_template_values` already reads prepared values directly; runtime `root_event_values`, `referenced_owner_values`, and inline rereads use fully prepared values and now agree with the launch view (CLI inline cases pass). No other projection feeding shell approval or membership found
    - docs: `claudine/docs/topics/flow-control/sequences.md`, `claudine/docs/topics/pre-flight-checks.md`, `.claude/skills/claudine/SKILL.md`; no caller skill restates the passage
    - verification (from `claudine/`): `just test prepared_shell_values` 12/12, `referenced_owner_schema` 7/7, `sequence_referenced_state` 101/101, `sequence_frozen_graph` 25/25, `stale_admission` 10/10, `template_owner_values` 3/3, `preflight::` 368/368, `literal_hints` 9/9, `composition::sequence::` 757/757, `sequence_shell` 321/321, `sequence_task_providers::` 74/74, plus `sequence_containers::` 23, `sequence_groups::` 20, `sequence_jit::` 13, `sequence_dry_run::` 7, `sequence_cli::` 29, `sequence_schema::` 15, `sequence_planned_model::` 2, `sequence_initialize_include_preflight::` 5, `research_sequence_artifacts` 7; no test at or above 5 s; `just lint` clean
    - completed 'R2-F1' at 22:24
- work completed for 'R2-F1' at 22:30:04
        - orchestrator reconciliation: inspected the combined diff of `preflight/mod.rs` (`layer_prepared_values` now returns `LayeredView`; a templated key is replaced by its prepared value unless it reads, directly or transitively via Darkmatter's `derived_deferred_keys`, a late root, `env`, a `$( … )` key, `agent`/`model`, or a runtime-written key; kept `{{ }}` keys join the view's late set so approval and runtime verification share one basis)
        - R2-F1.17's later-position control is exercised by CLI on the direct and nested routes plus the library route matrix; the other routes share the same projection, so no separate row is open

### Completion Report

- outcome: **complete**; elapsed 2026-11-15T22:00:27-08:00 → 22:30:04 (about 29 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- child rows: 23 (R2-F1.1–R2-F1.23): 19 fixed (including the added sibling R2-F1.23, provider prediction over a replaced derived key), 4 clean (R2-F1.17–R2-F1.20); none open
- verified repair: first-position shell rows, container/group `start` shells, and shell-valued `set` writes read the document's prepared derived values on every route; inline children are no longer refused as introduced after approval; genuinely late reads stay late; the early-binding comment, `sequences.md`, `pre-flight-checks.md`, and the repo `claudine` skill are updated
- design decision (ledger only, not author-worthy): replace a changed templated key with its prepared value unless its reads are unstable; rejected resolving inside `early_binding_state` (extra parse per step/verify, breaks pinned scaling counts) and blanket replacement (freezes late reads)
- open findings: none

#### Changed files (this iteration)

- `claudine/lib/src/composition/sequence/preflight/mod.rs`
- `claudine/lib/src/composition/sequence/preflight/tests/prepared_shell_values.rs`
- `claudine/lib/src/composition/sequence/preflight/providers/tests/literal_hints.rs`
- `claudine/cli/tests/l1/sequence_referenced_state.rs`
- `claudine/docs/topics/flow-control/sequences.md`, `claudine/docs/topics/pre-flight-checks.md`
- `.claude/skills/claudine/SKILL.md`
- this log and `review-2.md` frontmatter

#### Verification tied to findings

- R2-F1 (all rows): subagent's scoped runs (`prepared_shell_values` 12, `referenced_owner_schema` 7, `sequence_referenced_state` 101, `sequence_frozen_graph` 25, `stale_admission` 10, `template_owner_values` 3, `preflight::` 368, `literal_hints` 9, `composition::sequence::` 757, `sequence_shell` 321, `sequence_task_providers::` 74, `research_sequence_artifacts` 7)
- final gates by the orchestrator, sequentially, with no subagent running:
        - `just test` in `claudine/`: **12,403 passed**, 15 skipped, none at or above 5.0 s (223 s of tests)
        - `just lint` in `claudine/`: exit 0
        - `just check-tier-coverage claudine`: 0 stranded
        - `just shipped-schemas`: no artifact change
- cross-OS check not run: this iteration changes value layering only, with no path, process, or OS-specific code

## Implementation of Review Findings #3

> **started at:** 2026-11-15T22:46:46-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs; no `human_in_the_loop` items, no blocked findings; recurrence reported (R2-F1 partial repair, R3-F1 new sibling)
- both findings come from one mechanism: the global, name-keyed `prepared_replacements` list that `Loader::finish` appends to predictions. The review asks for one shared repair: carry the correct authored dependency closure with the prediction, scoped to its owner. So one foreground subagent repairs both findings serially in a single brief (R2-F1 first, then R3-F1). Two separate subagents would edit the same lines twice
- defect class (both findings): a provider prediction's dependency graph is rebuilt after its scope information is gone. Reads are matched by bare property name across owners and layers, and only one edge is restored, so a dependency is lost (a chain longer than one link) or invented (a same-named property of another owner)

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1   | Prepared provider dependencies are restored only one level deep | fixed | every required child below fixed or clean; no discovered defect left open |
| R2-F1.1–.22 | Original shell-output children; review 3 verified them fixed | already fixed | review-3 verification table; reconfirmed by final `just test` |
| R2-F1.23 | Direct root step: `with.agent`/`with.model` reading two- and three-link derived chains to a runtime write → `DecidedAtLaunch`; CLI writer runs, reader uses Goose | fixed | `provider_dependencies_tests::derived_with_chains_wait_for_the_write::direct_root` (agent/model, 2 and 3 links; an unwritten 2-link chain stays `Static`); CLI `sequence_sources_cli::derived_provider_reads::a_provider_read_through_two_derived_values_waits_for_the_write` (exit 0, 2 Goose launches) |
| R2-F1.24 | Root whole-document reference: same → deferred | fixed | `provider_dependencies_tests::derived_with_chains_wait_for_the_write::root_reference` |
| R2-F1.25 | Nested whole-document reference: same → deferred | fixed | `provider_dependencies_tests::derived_with_chains_wait_for_the_write::nested_reference` |
| R2-F1.26 | Inline sequence child: same → deferred | fixed | `provider_dependencies_tests::derived_with_chains_wait_for_the_write::inline_child` |
| R2-F1.27 | Referenced sequence in a group leg: same → deferred | fixed | `provider_dependencies_tests::derived_with_chains_wait_for_the_write::group_leg` |
| R2-F1.28 | Referenced sequence through a task file: same → deferred | fixed | `provider_dependencies_tests::derived_with_chains_wait_for_the_write::task_file` |
| R2-F1.29 | Prompt-owned chain, zero to three links, all routes (clean control) | clean | `provider_dependencies_tests::prompt_owned_chains_wait_for_the_write::{6 sites}` (agent 0–3 links, model 0/1/3) |
| R2-F1.30 | Direct read and one-link controls, both fields, all routes (clean control) | clean | `provider_dependencies_tests::direct_and_one_link_with_reads_wait_for_the_write::{6 sites}` plus the existing one-link test in `literal_hints.rs` |
| R2-F1.31 | Shell approval and state/template consumers do not use the provider restoration (clean) | clean | `LayeredView.values`/`late` unchanged; `prepared_shell_values`, `sequence_referenced_state` (101), and the full suite pass |
| R2-F1.32 | Sweep sibling: a prompt-document hint that reads a `with:` key whose value reads a step-derived chain to a runtime write → deferred (the document scope never saw step keys) | fixed | `provider_dependencies_tests::a_document_hint_through_a_derived_with_entry_waits::{6 sites}`; the prompt's own literal `choice` proves scoping, with an unwritten-chain static control |
| R3-F1   | Dependency restoration imports unrelated document properties | fixed | every required child below fixed or clean |
| R3-F1.1 | Direct root derived `choice` vs prompt literal `choice`, `agent`/`model` expression → both static; CLI refuses before the writer | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::direct_root`; CLI `derived_provider_reads::a_prompts_own_property_is_not_the_sequences_derived_one` (nonzero exit, "Agent Not Installed:(Claude)", "No task has started", 0 launches) |
| R3-F1.2 | Root-reference owner vs prompt → static | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::root_reference` |
| R3-F1.3 | Nested referenced owner vs prompt → static | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::nested_reference` |
| R3-F1.4 | Inline enclosing owner vs prompt → static | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::inline_child` |
| R3-F1.5 | Group-leg sequence owner vs prompt → static | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::group_leg` |
| R3-F1.6 | Task-file sequence owner vs prompt → static | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::task_file` |
| R3-F1.7 | Independent referenced sibling owner vs prompt → static | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::independent_sibling` |
| R3-F1.8 | Literal-root controls, all seven sites (clean) | clean | `provider_dependencies_tests::literal_owner_controls_stay_static::{7 sites}` |
| R3-F1.9 | Literal `with` overrides, all seven sites (clean) | clean | `provider_dependencies_tests::literal_with_overrides_stay_static::{7 sites}` |
| R3-F1.10 | Caller-setter overrides, all seven sites (clean) | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::direct_root`; CLI `derived_provider_reads::a_prompts_own_property_is_not_the_sequences_derived_one` (nonzero exit, "Agent Not Installed:(Claude)", "No task has started", 0 launches) |
| R3-F1.11 | Sweep sibling: a row-overlay key that shadows a replaced owner key must not take the owner's authored reads | fixed | `provider_dependencies_tests::a_prompts_own_choice_is_not_another_owners::direct_root`; CLI `derived_provider_reads::a_prompts_own_property_is_not_the_sequences_derived_one` (nonzero exit, "Agent Not Installed:(Claude)", "No task has started", 0 launches) |
| R2-F1.33 | Sweep sibling (added during repair): a derived owner value reading `ctx.agent`/`ctx.model`, read by a `with:` hint → `Circular`, refused before any task starts | fixed | `provider_dependencies_tests::an_identity_derived_with_read_is_circular::{6 sites}` (fail with the rule disabled, pass with it); CLI dry-run reproduction before and after |

### Progress

- starting the work on 'R2-F1 and R3-F1 (shared repair)' at 22:49:12
        - subagent (foreground, `rust-developer`) implemented the shared repair; its report, verified by the orchestrator:
                - removed the walk-wide `Loader::prepared_replacements` list and the name-matching append in `Loader::finish`
                - `prepared_view` now returns the replaced keys' authored values. They travel with the view they belong to (`Loader::step_authored`/`seed_authored`), installed, swapped, and restored at every site that sets `step_frontmatter`/`step_seed`. `enter_step` drops overlay keys from that map; the reread loaders start empty
                - `ProviderPrediction.dependencies` is now `Vec<DependencyScope { entry, values, feeds }>`, recorded when the prediction is built: one closure per owner. A `with:` closure resolves names through `step_authored` before `step_frontmatter`, so every link of an authored chain is kept. A document closure treats an unoverridden `with:` key as having no document value and adds a sequence-scope `feeds` closure for it
                - `defer_runtime_written` marks fed `with:` names first, then defers a hint whose own closure reaches `written` plus those names
                - new `lib/src/composition/sequence/preflight/tests/provider_dependencies.rs` (macro-generated, one test per placement); two CLI tests in `cli/tests/l1/sequence_sources_cli.rs::derived_provider_reads`; docs in `docs/cli/sequence.md` ("Choosing providers") and the repo `claudine` skill
                - mutation check reported by the subagent: with the `step_authored` lookup removed, the chain and R2-F1.32 tests fail on every site
                - corrects the review's quoted error text: the real refusal reads "Agent Not Installed:(Claude) … No task has started"
        - orchestrator follow-up on a sibling the subagent raised (ledger row R2-F1.33, added): iteration 2's projection also replaced a derived value that reads the launching target's identity (`who: "{{ ctx.agent }}"`). `with: { agent: "{{ who }}" }` then predicted from the prepared literal (`unknown`) instead of reporting the circular read
                - reproduced with the shipped CLI binary in a private temp home, `--dry-run`. Without the fix: "Providers: reader → no provider", the step starts, then fails at launch with "Claude is not installed". With the fix: rejected before any task starts, with "it reads `ctx.agent`, which is the identity it selects"
                - fix: `layer_prepared_values` keeps authored any templated key whose value names a target-identity root (`shape::first_target_identity_root`: `ctx.agent`, `ctx.model`, `env.*`), as it already did for `env`, `$(`, `agent`/`model`, and written keys
                - test: `provider_dependencies_tests::an_identity_derived_with_read_is_circular::{6 sites}` (captures the agent context group, as the CLI does). Mutation check: all 6 fail with the rule disabled, all pass with it
                - docs: `docs/cli/sequence.md` (circular hint through another value) and the repo skill's early-binding passage
- work completed for 'R2-F1 and R3-F1 (shared repair)' at 23:17:03

### Completion Report

- outcome: **complete**; elapsed 2026-11-15T22:46:46-08:00 → 23:17 (about 30 minutes)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- child rows: R2-F1.1–.22 already fixed (verified by review 3; reconfirmed by the full suite); R2-F1.23–.28, .32, .33 fixed; R2-F1.29–.31 clean; R3-F1.1–.7 and .11 fixed; R3-F1.8–.10 clean; none open
- verified repair: a provider prediction now carries its authored dependency closure, one per owner, when it is built. It follows every link of a derived chain, never imports another owner's same-named property, and follows a `with:` entry in the sequence scope where it is evaluated. Early binding also keeps a value derived from the target identity authored, so a `with:` hint through it stays circular. The walk-wide, name-keyed replacement list is gone, which gives this requirement the boundary the review's Recurrence section asked for
- design decisions (ledger only, not author-worthy):
        - classification (late/circular/shell) still reads the prepared view; only dependency recording follows authored values. Switching classification to authored values would have changed other hints' classes
        - the identity sibling uses the existing `first_target_identity_root` reader instead of a new rule
- open findings: none

#### Changed files (this iteration)

- `claudine/lib/src/composition/sequence/preflight/mod.rs` (R2-F1, R3-F1, R2-F1.33)
- `claudine/lib/src/composition/sequence/preflight/providers.rs` (R2-F1, R3-F1)
- `claudine/lib/src/composition/sequence/preflight/providers/tests.rs`, `.../providers/tests/literal_hints.rs`, `claudine/lib/src/pipeline_spike/tests.rs` (`HintLayers` construction)
- `claudine/lib/src/composition/sequence/preflight/tests/provider_dependencies.rs` (new, 58 tests)
- `claudine/cli/tests/l1/sequence_sources_cli.rs` (`derived_provider_reads`, 2 tests)
- `claudine/docs/cli/sequence.md` ("Choosing providers")
- `.claude/skills/claudine/SKILL.md` (early binding and per-task provider selection passages)
- this log and `review-3.md` frontmatter

#### Verification tied to findings

- the subagent's scoped runs (R2-F1, R3-F1): `preflight::` 420, `composition::sequence::` 809, `sequence_sources_cli` 69, `sequence_referenced_state` 101, `sequence_task_providers` 74, `literal_hints` 9, `scaling` 14 (pinned counts unchanged), all passed
- orchestrator: `just test provider_dependencies` 58/58 passed; mutation checks for R2-F1.33 (6 fail without the rule)
- final gates by the orchestrator, sequentially, with no subagent running:
        - `just test` in `claudine/`: **12,463 passed**, 15 skipped, none at or above 5.0 s
        - `just lint` in `claudine/`: exit 0
        - `just check-tier-coverage claudine`: 0 stranded
- cross-OS check not run: value-layering and dependency analysis only, with no path, process, or OS-specific code
