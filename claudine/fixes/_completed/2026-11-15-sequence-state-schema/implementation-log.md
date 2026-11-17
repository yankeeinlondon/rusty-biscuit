---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/spec.md"
plan: "claudine/fixes/2026-11-15-sequence-state-schema/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - claudine/lib/src/composition/sequence/declarations.rs
    - claudine/lib/src/composition/sequence/declarations/tests.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/group/branch.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/generated/shipped.json
    - claudine/schemas/sequence.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_2:
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/sequence_loop.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/sequence/document.rs
    - claudine/lib/src/composition/sequence/document/tests.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/frontmatter-properties.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
packages:
    - claudine
    - claudine-cli
decisions_for_review:
    - |-
        Template defaults may not set `$schema`, in addition to `$state` and `template`. The spec lists only `$state`/`template` as declarations a value route may not write. A template default is merged into every item, and on an inline nested sequence item a `$schema` key is that container's check, so a `$schema` default would silently give every container a check. Alternative: allow it and let the per-item placement rules decide (it would then be accepted on inline containers and refused elsewhere). Reversing means removing `CHECK_KEY` from `template_shape` in `sequence/declarations.rs` and its row in `template_defaults_cannot_write_declarations`.
    - |-
        A `$schema`, `$state`, or `template` written on a group object is a typed placement error from the parser (`SequenceDeclarationMisplaced` at `…group.$state`), with a hint pointing at an inline sequence leg. Before, the parser ignored unknown group keys and only the closed group schema reported them. Alternative: leave group objects to the schema alone. Reversing means deleting `reject_group_declarations` and its call in `grammar.rs::group_object_grammar`.
    - |-
        The `$state` violation message keeps its old text verbatim and now always appends `` (`$state` of `<container>` in <file>) ``, even for a root list. Alternative: append provenance only for nested or referenced owners, which keeps root messages byte-identical but makes the suffix's presence vary. Reversing touches the `#[error]` format of `SequenceStateSchemaViolation` and one assertion in `declarations/tests.rs`.
    - |-
        Error typing per value route. A setter or `--set` key naming a declaration reuses `SequenceDeclarationMisplaced` (located at the sequence document, property = the key, hint "a setter supplies values, not declarations"). A runtime `set` gets a new `RuntimeMutationError::DeclarationKey`, and `with:` keeps the existing `SequenceTaskWithReserved` (its message now mentions the declarations). Alternative: one dedicated `SequenceDeclarationWrite { route, key }` variant for all three. Reversing would touch `declarations::reject_declaration_write`, `RuntimeState::set_batch`, and `TaskExecution::evaluate_with`.
    - |-
        The passive `$state` view is just `$schema: <declaration>` in an empty in-memory frontmatter, resolved in the declaring file's context. Plan rule 8 also mentions including "the owner's sibling `types` definitions". Document frontmatter has no `types` key in Darkmatter (named types live in standalone schema documents and are imported with `Name@file`), so nothing was added. If a frontmatter-level `types` is wanted, it would have to be added in `PreparedSequenceSchema::prepare`, and Darkmatter would need to support it first.
    - |-
        During Phase 2, referenced templates kept the referring document's values as a lowest layer to keep research fleets working before initialization writes were available. Iteration 3 verification correction (2026-11-15): this is historical, not current behavior. The first repair removed both fallback merges and moved all 22 fleets to initialization writes. Source inspection and the referenced-template, input-discovery, and research artifact tests in this review's 2,103-test sequence run verify that only the declaring owner's values, invocation inputs, and available runtime writes reach referenced templates.
    - |-
        An owner's values are prepared through `prepare_sequence_document_values` (one frontmatter compose) only when its `template:` has at least one string (expression) default *and* the caller supplied a captured compose context. Otherwise the template reads the authored values under the caller's setters, unconverted. This avoids a compose per sequence resolution when no template expression reads the values. Alternative: always prepare when a context is supplied, so `SequenceDocumentScope::values` is always the prepared form, at the cost of one extra frontmatter compose per owner. Reversing touches only the condition in `sequence/document.rs::owner_values`.
    - |-
        The missing-input error's one-line text changed from "…failed for N step(s); …" to "…failed for N step(s) or document(s); …", and its rendered summary now counts each kind ("in 1 step and 2 documents of the sequence"). A document entry is headed `Document <file>`, plus `` (`<container path>`) `` for a referenced owner, and never gets a step number. Alternative: keep the old one-line text and change only the rendered block. Reversing touches the `#[error]` on `SequenceMissingProperties` and two tests in `composition/error/tests.rs`.
    - |-
        In the preflight graph, a root row's task is still located as `tasks[i]`, so an inline container's check and `$state` failures under a root row read `tasks[0].$schema` / `tasks[0].sequence`. The input inventory and the formal normalizer use the authored path `sequence[i]`, which is what a user sees in the file. The graph's `tasks[i]` predates this fix and appears in about 80 test assertions across lib and CLI, so I left it alone. Alternative: rename the graph's root prefix to `sequence[i]` so every location matches the file. Doing so means changing `walk_plan`'s `format!("tasks[{index}]")` and updating those assertions.
    - |-
        A nested reread (`prepare_nested_sequence`) still discovers command sites from the reread rows, read as authored with plain normalization (no template fill, no `$state`), and keeps the frozen plan for execution. Rule 19's "discover refreshed command sites against frozen child nodes" could also mean walking only the frozen children. I tried that first: it silently removes the existing refusal of a command changed on disk (`changed_nested_source_refuses_new_commands_before_start`), which the plan says to retain. Alternative: walk frozen children only and accept that an edited inline child runs its approved, frozen command without refusal. Changing this touches the `refresh` arms in `Loader::load_sequence_with`.
    - |-
        Rows of a root `sequence: child.yaml` resolve their `prompt:`/`task:`/`sequence:` references from the invoking file's directory, because `walk_plan` runs them as the root's rows. The inventory does the same, so the two passes agree. A nested `sequence: child.yaml` resolves from `child.yaml`'s directory. Rule 16 asks to keep the root row shape, so I did not change it. Alternative: resolve root-reference rows from the referenced file, matching nested references. Doing so means changing `walk_plan`'s origin and `inputs.rs::walk_referenced`'s `rows_origin`.
    - |-
        The root reference's document metadata reaches input collection as the inventory's `RootReference` owner (file, property `sequence`, prepared values, missing entry). I added no `PreflightGraph` field for it. Alternative: also record `plan.document_scope` on the graph for a later consumer. That is an additive field in `walk_plan`.
    - |-
        An authored executable reference containing `{{ … }}` (for example `prompt: "{{ plan }}"`) is pending only while its owner is missing an input. Once the inputs are supplied, it is read as the literal file name it always was, and fails as not found, exactly as the graph does. Only `template:` defaults are interpolated. Alternative: interpolate authored executable references in both the inventory and the graph. That would be a new feature beyond this fix.
    - |-
        Discovery judges declaration shape and placement but does not resolve `$state` or an inline `$schema` reference. An unloadable one fails at the graph build that follows, before approval. This avoids preparing every declaration twice. Alternative: prepare them in discovery too, through the shared `NestedSources` cache. Doing so means calling `NestedSources::prepared_schema` in `inputs.rs::walk_executable`.
    - |-
        The root document's own missing inputs are not collected by the new discovery pass when nothing else is missing, no reference waits on them, and the plan has steps. Phase 1c reports (or asks for) them per step, as it always did, so the existing "Step 1 / Step 2" report is unchanged. Discovery reports them as a `Document` entry only for an empty root or alongside another owner's missing inputs. A consequence: outside a terminal, a run missing both a nested document's input and a root input first reports the document entries (root included), and step-level gaps appear only once those are supplied. Alternative: always report the root as a `Document` entry, which changes every existing sequence missing-input report from step headings to one document heading. Reversing touches the `root only` branch in `phase1c.rs::prepare_root_plan` and the CLI tests asserting step headings.
    - |-
        Discovery no longer fails on an owner's own preparation error other than a missing input (an invalid value, an `agent: []`, a removed spelling). It records the owner (`SequenceInputInventory::failed_owners`, opt-in through `InputDiscoveryOptions::leaving_verdicts`), and the CLI reports the error later, in the order it always had: the graph's structural errors and provider selection first. Then a strict rediscovery (`RootPlanRequest::recheck_owners`) raises a referenced owner's error, and Phase 1c's per-step probes raise the root's. An empty root is rechecked strictly too. Without this, 48 existing CLI tests changed which error they report. Alternative: let discovery fail first and update those tests to expect the schema verdict. That gives earlier but less specific errors (for example "`/sequence/0/group/tasks: []` has less than 1 item" instead of "`tasks` must declare at least one task"). Reversing means dropping `leaving_verdicts()` in `prepare_root_plan` and the recheck call in `wrap/sequence/mod.rs`.
    - |-
        During Phase 4, discovery duplicated Phase 2's transitional referrer fallback. Iteration 3 verification correction (2026-11-15): the earlier present-tense claim is no longer true. The first repair removed the block in `preflight/inputs.rs::walk_referenced` together with the one in `source.rs::plan_referenced_sequence`; the discovery and CLI owner-isolation cases passed in this review's sequence run.
    - |-
        Root `initialize` writes are layered above the caller's setters as caller-layer values for discovery, the root plan's templates, and the graph walk, including the values a shell command reads at approval. The spec asks for them in template preparation. Passing them to the graph's approval-time reads as well keeps approval reading what the command will read at its turn. Alternative: give them to discovery and the plan only. Reversing touches `RootPlanRequest::layers` in `phase1c.rs`.
    - |-
        The plan is no longer built before `initialize`. Before it runs, the CLI judges only the root's declarations and the grammar of each inline root item. A root `sequence: child.yaml` is now loaded (and its errors reported) after `initialize`, at preflight, and so is a `$( … )` source's shell. Before, both happened before `initialize`. `initialize` is shell-free and runs before preflight, so nothing it does can depend on that. Alternative: also build a throwaway structural plan before `initialize`. That runs a shell source twice, and fails on a template reading a missing input before collection can report it. Reversing means restoring the `resolve_sequence_plan_with` call in `commands/sequence.rs`.
    - |-
        An inline container's `$schema` check judges the task's state (the enclosing effective values), then the caller's setters, then the runtime writes so far. It does not include the lexical `seq`/`loop`/`group` roots as properties. A schema types document properties, and adding those roots as instance keys would make a closed `$schema` fail on them. Alternative: add the roots under their names. That would let a check constrain them, at the cost of a closed schema refusing them. Reversing touches `check_inline_container` in `task/sequence.rs`.
    - |-
        The inline check also runs at every container `start` reread (the first pass after `initialize`, each retry, each loop pass), against the check as reread and the values of that moment. A refusal there blocks the container (its `blocked`/`failure` path runs), unlike admission's unstarted failure. The plan names "retry/pass refresh boundaries". Checking the first `start` too keeps the rule simple, and the values only differ from admission by what `initialize` wrote. Alternative: skip the first pass's `start`. Reversing touches the `start` arm of `run_sequence_attempts`.
    - |-
        A retry of the root reuses the first attempt's collected values (`RootRun::collected_overrides`) as well as its plan, so a retry never prompts again. Before this phase a root retry re-ran Phase 1c with the original setters and could ask again. Alternative: re-ask on retry. Reversing touches the first `match` arm in `execute_sequence_inner`.
    - |-
        A root retry now finds commands with `reread_preflight_graph_with_invocation`. Its walk reads nested containers the way a nested reread does: their declarations are judged again, and their rows are read as authored, with no template fill and no `$state`. Before, the retry walk planned nested references afresh, so a `$state` added on disk between attempts failed rows the first attempt had already planned. Consequence: a declaration made malformed between attempts still fails the retry, as it already did for a nested reread. Alternative: skip declaration judging on a retry walk and only discover commands. Reversing touches `reread_preflight_graph_with_invocation` in `sequence/preflight/mod.rs` and its call in the retry arm of `wrap/sequence/mod.rs::execute_sequence_inner`.
    - |-
        A referenced or nested document's own authored file values (for example `spec: work/spec.md` under `file(eager)`) now resolve from that document's folder, using the context the graph derives (`InvocationContext::derive_source`). Setter values still resolve from the caller's recorded origin. When a library caller passes setters without recorded origins, `PrepareOptions::resolving_document_from` first records them against the caller's context, so changing the base never moves a setter. Alternative: change the base only when origins were recorded. That is simpler, but a library caller's setters would then resolve from the document's folder. Reversing touches `resolving_document_from` in `composition/prepare.rs`.
    - |-
        At the first review, passive approval judged a sequence task's relative `with:` file input from the launch directory and could reject a valid author-relative file, while runtime preparation used the authoring file correctly. Iteration 3 verification correction (2026-11-15): the statement that this was left unfixed is no longer current. The first repair carries the authored `with:` origin through approval and the step barrier. Relative, absolute, and competing-folder `with_file_provenance` cases passed in this review's 2,103-test sequence run. This is no longer deferred work.
    - |-
        A line-delimited (JSONL/NDJSON) record that holds `template` or `$state` is refused as a misplaced declaration (`SequenceDeclarationMisplaced` at `[i].<key>`), because a row cannot supply a declaration. The plan's matrix says line-delimited declarations "remain inactive data". The test `tests::loader_matrix::line_delimited_records_skip_blank_lines_and_name_a_trailing_invalid_record` asserts the refusal, which is what the code does today. Alternative: let such keys through as plain row data, which means special-casing the lenient normalizer.
    - |-
        Windows coverage of templated `echo` rows comes from a new test-only `claudine-fake-echo` binary, installed as the fixture's `echo` by `CliProcessFixture::install_echo`. It runs on every OS, so Unix runs use it too, not `/bin/echo`. Alternatives: rewrite those rows to avoid a shell, or use the recorder binary and read its records. Review correction (2026-11-15): the reported `set`/shell discrepancy was an incorrect reading of the side-effect result. A `set` side effect returns the prior values, so `{"signed":null}` does not mean it wrote null. Direct executions through the current CLI, followed by a shell reading `signed`, produced `signed=ada` for direct, root-reference, and nested-reference rows with `template: { who: ada }`. The runtime write correctly reads `seq.who`; no unresolved defect is established by the earlier probe. Reversing the binary choice touches `cli/Cargo.toml`, `cli/tests/bin/fake_echo/`, `install_echo`, and the two `run_sequence` helpers.
    - |-
        The caller skill (`use-claudine/sequences.md`) carries its own short summary of the sequence declarations instead of transcluding the canonical topic section, as the plan preferred. Darkmatter's `::file` can only include a whole file or exclude named headings, so including one section of the 1,500-line `sequences.md` would mean excluding every other heading by name, which breaks whenever a heading is added. The canonical section also has inline-code interpolation examples, which a transclusion would evaluate. Alternative: split the declarations section into its own topic file and transclude that file whole. Reversing means moving the "Sequence documents" section of `docs/topics/flow-control/sequences.md` into a new file, linking it from there, and replacing the caller-skill section with a `::file` directive (after rewriting its inline examples as fenced blocks).
source_files_during_phase_3:
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/declarations.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/inputs.rs
    - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scoped_graph.rs
    - claudine/lib/src/work_counters.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/inline_check.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/inputs.rs
    - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_5:
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/document.rs
    - claudine/lib/src/composition/sequence/document/tests.rs
    - claudine/lib/src/composition/sequence/declarations/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/inputs.rs
    - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scoped_graph.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/inline_check.rs
    - claudine/lib/src/composition/sequence/task/tests/declaration_writes.rs
    - claudine/lib/tests/l1/claudine_schema_shipped_corpus.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/sequence_referenced_state.rs
    - claudine/cli/tests/l1/sequence_frozen_graph.rs
    - claudine/cli/tests/l1/sequence_dry_run.rs
    - claudine/cli/tests/fixtures/frozen_prompts/sequence-state-schema/complete.yaml
    - claudine/cli/tests/fixtures/frozen_prompts/sequence-state-schema/plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/sequence-state-schema/spec.md
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/bin/fake_echo/main.rs
    - claudine/cli/Cargo.toml
docs_updated_during_phase_5:
    - claudine/docs/topics/flow-control/sequences.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/use-skills/use-claudine/sequences.md
    - claudine/schemas/README.md
docs_created_during_phase_6:
    - claudine/fixes/2026-11-15-sequence-state-schema/implementation.md
skills_files_updated_during_phase_6:
    - .claude/skills/claudine/SKILL.md
source_code:
    - claudine/lib/src/composition/sequence/declarations.rs
    - claudine/lib/src/composition/sequence/declarations/tests.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/group/branch.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/generated/shipped.json
    - claudine/schemas/sequence.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/lib/src/composition/error/render/sequence_loop.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/sequence/document.rs
    - claudine/lib/src/composition/sequence/document/tests.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/inputs.rs
    - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scoped_graph.rs
    - claudine/lib/src/work_counters.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/inline_check.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/lib/src/composition/sequence/task/tests/declaration_writes.rs
    - claudine/lib/tests/l1/claudine_schema_shipped_corpus.rs
    - claudine/cli/tests/l1/sequence_referenced_state.rs
    - claudine/cli/tests/l1/sequence_frozen_graph.rs
    - claudine/cli/tests/l1/sequence_dry_run.rs
    - claudine/cli/tests/fixtures/frozen_prompts/sequence-state-schema/complete.yaml
    - claudine/cli/tests/fixtures/frozen_prompts/sequence-state-schema/plan.md
    - claudine/cli/tests/fixtures/frozen_prompts/sequence-state-schema/spec.md
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/bin/fake_echo/main.rs
    - claudine/cli/Cargo.toml
documentation:
    - .claude/skills/claudine/SKILL.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/use-skills/use-claudine/sequences.md
    - claudine/schemas/README.md
    - claudine/fixes/2026-11-15-sequence-state-schema/implementation.md
completed_phase: 6
implemented: true
---

# Implementation Log for 2026-11-15-sequence-state-schema (6 phases)

## Phase 1


### Wave 1 — Declaration contracts (rules 1, 5–8, 20)

- New module `lib/src/composition/sequence/declarations.rs` owns placement,
  shape, and preparation of sequence declarations:
  - `root_declarations` / `task_declarations` judge by key *presence*
    (explicit `null` included). Root `$state`/`template` require the root
    `sequence:` to be an inline list; task `$schema`/`$state`/`template`
    require the task's own inline list. Every other placement (prompt, shell,
    side-effect, body-only, `task:` reference, `sequence: file.yaml`, group
    object) is `SequenceDeclarationMisplaced` with a hint naming where the
    declaration belongs. `reject_group_declarations` keeps group objects
    closed to them.
  - Shapes: a schema declaration is an inline mapping (`{}` = open), a
    non-empty reference string, or a non-empty union of mappings/non-empty
    references; `null`, `[]`, `""`, numbers, booleans, and bad arms are
    `SequenceDeclarationShape` (arms located as `$state[1]`). `template` must
    be a mapping (`{}` = no-op); `null` is invalid. Template keys may not be a
    reserved state key, a declaration, or `$schema`
    (`SequenceReservedTemplateKey`), judged before any row is read.
  - `PreparedSequenceSchema` resolves a declaration once through
    `DarkmatterSchemas::new(ctx.for_source(declaring_path)).effective_for`
    (no triggers, no baseline — same as the old per-row path) and judges each
    `StepState::to_value()` passively with `EffectiveSchema::validate`.
    Unparsable/unresolvable declarations are `SequenceDeclarationLoad`
    carrying the boxed `simplified_schema::SchemaError`.
- `formal.rs`: `FormalKeys::schema` → `state_schema`, read from `$state`
  through `root_declarations`; `formal_keys` now takes the declaring path.
  `normalize_formal_plan` prepares `$state` before shorthand/template/rows,
  so an empty list still fails on a malformed or unloadable `$state`. The old
  per-row `validate_state_schema` (a synthetic document per row) is gone.
- `grammar.rs`: `validate_task_grammar` calls `task_declarations` first;
  `validate_sequence_root` calls `root_declarations`; `group_object_grammar`
  calls `reject_group_declarations`.
- `reserved.rs`: `SEQUENCE_DECLARATION_KEYS = ["$state", "template"]`, now in
  `is_reserved_state_key`; `$schema`/`$state`/`template` added to
  `TASK_OPTION_KEYS` and the sequence executable's allowed options; one
  `declaration_written_by`/`is_declaration_write` predicate (root of a dotted
  or indexed path) for every value route. `GENERATED_STATE_KEYS` doc now says
  `$state`.
- Value routes: `RuntimeState::{new_sequence, from_sequence_snapshot}` set a
  private sequence flag; `set_batch` refuses a declaration write only on such
  a cell (`RuntimeMutationError::DeclarationKey`). `new`/`Default`/
  `from_snapshot` are unchanged for ordinary document runs. The CLI root
  (`wrap/sequence/mod.rs`), the container fallback cells
  (`task/sequence.rs`), and group branches (`task/group.rs`,
  `task/group/branch.rs`) use the sequence constructors. `with:` evaluation
  refuses dotted declaration paths too. `claudine sequence` refuses a setter
  or `--set` key that writes a declaration (`reject_declaration_write`) before
  anything reads it.
- Errors (`composition/error/mod.rs`): new `SequenceDeclarationMisplaced`,
  `SequenceDeclarationShape`, `SequenceDeclarationLoad`,
  `SequenceContainerSchemaViolation` (defined now, raised from Phase 4's
  inline check). `SequenceStateSchemaViolation` gains `source_path` and
  `container`; its message keeps the old clause verbatim and appends
  `` (`$state` of `<container>` in <file>) ``. All render through the generic
  Prose catch-all, as `TaskLoopMisplaced` does.
- `resolve.rs::FORMAL_SEQUENCE_KEYS` is now `["$state", "template"]` so a
  directly invoked YAML document keeps its root `$schema` for composition
  (rule 1's "keep root `$schema` in document composition"); the YAML-only
  gate and nested protection remain Phase 2's (rule 3).

### Wave 1 — Shipped contracts (rule 22)

- `schemas/sequence.yaml`: root `$state: schema`, retained `template: object`
  with an inline-list description.
- `schemas/sequence-types.yaml`: recursive `task` gains `$schema: schema`,
  `$state: schema`, `template: object`, each described as an inline-sequence
  declaration beside `sequence:`. No defaults; group stays closed.

### Wave 2 — Contract checkpoint

- `just shipped-schemas` (repo root) regenerated only
  `claudine/lib/generated/shipped.json` (3 lines changed); no other
  generated file moved.
- Extended `lib/tests/l1/sequence_schema.rs` with
  `sequence_declarations_are_typed_alike_on_disk_and_embedded`: a valid
  document using root `$schema`/`$state`/`template` plus a nested container's
  `$schema`/`$state` (as a relative reference)/`template`, a root
  `$state: 42`, and a nested `template: 3`. It is judged through the disk
  bundle and the embedded bundle, and the verdicts must be equal. That covers
  disk/embedded parity for the new declarations. `claudine_schema_parity.rs`
  derives events/actions from the registry and has no declaration surface,
  so it needed no change. Freshness is the existing
  `shipped_artifacts::shipped_artifact_is_fresh`, which passes.

### Requirement → test mapping (Phase 1)

All of these are L1 tests at F1/F2 (in-process public resolver, plus one CLI
subprocess for setter wiring). The boundary is the sequence resolver
(`resolve_sequence_plan`) over direct YAML, direct Markdown, and referenced
documents.

- Placement of root `$state`/`template` beside a non-list source, with
  `null` still counting as present, in YAML and Markdown:
  `declarations::tests::root_declarations_beside_a_non_list_source_are_misplaced`
- Root `$schema` is not a state declaration:
  `…::root_schema_is_not_a_state_declaration`
- Absent / `{}` / `null` / `[]` / `""` / number / boolean / bad union arm /
  bad template shapes, on an empty list:
  `…::declaration_shapes_are_judged_by_presence_on_an_empty_list`
- Unloadable or unparsable `$state` on an empty list, with a downcastable
  typed cause: `…::an_unloadable_state_schema_fails_an_empty_list`
- Template defaults cannot write declarations or reserved state:
  `…::template_defaults_cannot_write_declarations`
- Inline containers (and group legs) own their declarations as options, not
  parent state: `…::inline_containers_accept_their_own_declarations`
- Task placement errors for prompt, shell, side-effect, body-only,
  reference, and group, plus a group object:
  `…::task_declarations_beside_a_non_list_are_misplaced`
- Task shape errors located at the full container path:
  `…::task_declaration_shapes_are_located_at_the_task`
- `$state` violation identity and owner provenance, including a referenced
  owner: `…::state_violations_name_the_step_and_the_declaring_owner`
- Relative `$state` reference resolved in the declaring file's directory:
  `…::a_state_reference_resolves_in_the_declaring_documents_context`
- One write predicate (dotted and indexed paths):
  `…::declaration_writes_are_detected_by_their_root`; step state cannot
  author a declaration: `…::step_state_cannot_author_a_declaration_key`
- Runtime `set` refusal only on sequence cells, branch included:
  `runtime_state::tests::only_a_sequence_cell_refuses_declaration_writes`
- Setter / `--set` wiring through the real CLI (setter before and after the
  file), with no step composed:
  `cli … sequence_sources_cli::a_setter_cannot_write_a_sequence_declaration`
- Renamed row-schema tests now use `$state`:
  `sequence_sources_cli::{a_referenced_formal_documents_state_declaration_validates_the_step_state,
  a_directly_invoked_documents_state_declaration_validates_the_step_state,
  a_formal_document_normalizes_identically_through_both_entry_paths}`
  and `sequence::tests::formal_documents::*`. `external_template_non_object_fails`
  now expects `SequenceDeclarationShape`.
- Shipped meta-types plus disk/embedded parity:
  `sequence_schema::sequence_declarations_are_typed_alike_on_disk_and_embedded`

Not yet covered (later phases own the behavior): Markdown `$state`/`template`
normalization, `$state` on nested inline lists, and the inline `$schema`
check at task admission. Phase 1 only validates the syntax and placement of
those declarations.

### Gates

- `just lint` (claudine/): passes. The first run failed the transport guard
  `no_unallowlisted_typed_error_collapses`, because `PreparedSequenceSchema::prepare`
  stringified a frontmatter insert error. It now keeps the typed
  `SequenceExternalLoad { source: SequenceLoadCause::Frontmatter }`.
- `just test` (claudine/): 12,187 tests passed, exit 0; 44 recorded at or
  above 5 s, all under 10 s. The host was under heavy unrelated load (load
  average 130–173 from other sessions' Blender and rustc jobs; no subagents
  of mine were running), so the 10 s allowance applied. I re-measured all 44
  with `-j 1` at load about 20–85: 41 were at or under 5 s. The other three were
  re-measured individually at load about 20: `deep_container_deferral…` 3.61 s
  (8.58 s in the -j1 batch), `shipped_artifacts::the_checked_in_artifact_round_trips…`
  2.67 s (5.36 s), `claudine_schemas_matrix::yaml_trigger_combinator::control`
  2.58 s (5.01 s). The new `sequence_declarations_are_typed_alike_on_disk_and_embedded`
  took 5.92 s in the package run and 3.71 s alone. All are recorded as
  load-sensitive, not defects.
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`)
  depend on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check was run. There is no new path handling:
  diagnostics use `to_portable_string`, and test path checks use
  component-wise `Path::ends_with`. CI's nightly legs cover Windows and WSL2.

### Notes for later phases

- `FORMAL_SEQUENCE_KEYS` moved to `["$state", "template"]` in this phase,
  because otherwise the key split from rule 1 would be inconsistent. The
  YAML-only gate in `without_formal_sequence_keys`, the nested protection,
  and Markdown routing are still rule 3 / rule 15 work.
- `PreparedSequenceSchema` is prepared once per `normalize_formal_plan` call
  (not per row). There is no invocation-level cache across owner rereads yet;
  that belongs with `NestedSources` (rule 8, Phase 3). `dependencies()` is
  exposed for that cache.
- `SequenceContainerSchemaViolation` is defined but not raised yet (rule 17,
  Phase 4). `PreparedSequenceSchema::validate` is the passive judge to use
  for it.

## Phase 2

### Wave 3 — Composition views (rules 3, 4, 10)

- `resolve.rs`: `without_formal_sequence_keys` no longer checks the file
  format. When the root `sequence:` is an authored list, the composition view
  drops root `$state`/`template` and `$schema`/`$state`/`template` from every
  inline container task beneath it (nested `sequence:` lists and inline
  `group:` legs). Only that task structure is walked, so a plain value holding
  a `template` key is left alone, and so is a document with no list.
  `composition_view` (crate-internal) returns a `Cow` and borrows when nothing
  needs dropping, so ordinary documents are not cloned. The root `$schema`
  stays. Mapping orders travel with the rebuilt frontmatter. Removing keys
  moves no list index, and an object's recorded order skips the keys it no
  longer holds.
- New `reload_authored_source` (declarations kept). `reload_composition_source`
  now means "authored reload, then the composition view". Phase 3's nested
  refresh should use the authored one.
- The shared preparation service applies the view at its entry points
  (`prepare_document_selecting`, `prepare_bootstrap`), so a library caller
  gets the same rule as the sequence CLI.
- New `sequence/document.rs`:
  - `SequenceDocumentScope` (declaring path, property, `Arc` authored root,
    `Arc` owner values).
  - `SequenceDocumentPreparation`.
  - `prepare_sequence_document_values(source, PrepareOptions, CompositionMode)`.
    It is the canonical option assembly narrowed to the frontmatter surface,
    with frontmatter `$( … )` expansion *disabled* (passive, not refused) and
    the body never read. `prepare.rs::prepare_values_on` composes, and the
    schema layer judges the result through a new
    `schema::PrepareMode::Values(mode)`. That reuses the existing
    categorization, drop-and-retry, and post-shell verdict unchanged.
    `defer_schema_verdict` withholds the verdict, as bootstrap does.
    `translate.rs` now reads the phase through `PrepareMode::is_inline`.

### Wave 3 — Input subjects (rule 14)

- `SequenceMissingPropertiesStep` was replaced by
  `SequenceMissingPropertiesEntry { subject: SequenceMissingSubject, … }`, where
  the subject is `Step { number, name }` or `Document { property }`. Both are
  exported from `composition`. The renderer keeps the old step headings and
  adds `Document <file>`, or `Document <file> (`<container path>`)` for a
  referenced owner. The opening line counts steps and documents separately
  ("in 1 step and 2 documents"). The `Display` text now reads
  "… for N step(s) or document(s); …". Phase 1c builds `Step` subjects, and
  its interactive status heading handles both subjects. Collection order is
  unchanged.
- Docs: `claudine/docs/topics/frontmatter-properties.md` named the old type
  and was updated.

### Wave 4 — Owner normalization (rules 9, 10, 15, 16)

- `SequenceSourceOptions` fields are now private, with builders:
  `new` (no caller input), `with_shell_runner`, `with_compose_context`, and
  `with_caller_layers`. All struct-literal callers (CLI `sequence.rs`, lib and
  CLI tests) use the builders.
- `formal::is_direct_formal_document` is gone. Every authored root list, YAML
  or Markdown, goes through `normalize_formal_plan`. Expression and shell
  sources keep the data pipeline. `normalize_formal_plan` now takes a
  `FormalOwner { keys, declaring_path, list_property, values }` and calls
  `normalize_plan_at`. Template expressions are anchored at the *declaring*
  file (rule 9). Before, they were anchored at the invoking file, which was
  wrong for a referenced document in another folder. `invocation_path` still
  seeds `sequence_id`.
- Owner values (`document::owner_values`): when the template has a string
  default and the options carry a captured `ComposeContext`, the owner is
  prepared with `prepare_sequence_document_values` (verdict withheld). Without
  a context, the values are the authored root under the caller's setters, by
  top-level key. The normalizer never captures a CWD or environment.
- A referenced whole document (`source.rs::plan_referenced_sequence`) is its
  own owner. Its template reads its own properties under the caller's
  setters, never the referrer's frontmatter. Its prepared source is built
  lazily (`whole_document_source`): Markdown is reread, data formats become
  body-less frontmatter. `::map`/`::name`/`::template` operators on data
  selections still read the referrer's frontmatter, as before. The preflight
  `Loader` passes its `caller_setters` as caller layers, so a nested
  referenced document fills its template the same way.
- `SequencePlan::document_scope` is set for a root inline list (the invoked
  file) and for a whole referenced document (the target file). It is `None`
  for selections and dynamic sources. `normalize.rs` initializes it to
  `None`.
- Transitional referrer layer during Phase 2: a whole referenced document's template then
  read the referrer's values, but only as the *lowest* layer, below the
  document's own values and the setters. Without it, all 22 shipped research
  fleets break (`research_sequence_artifacts` failed 4 tests): their shared
  rosters read `research_prompt` from the fleet, and the spec's intended
  channel (`initialize` `set` writes reaching root templates) is Phase 4's.
  See `decisions_for_review`. Iteration 4 correction (2026-11-15): this layer
  was removed in the first repair. The current source and passing referenced
  template/discovery tests verify owner isolation; fleets now use runtime writes.
- `cli/tests/l1/composition_seams.rs`: the R1 guard needed an allowlist entry
  for the new `compose_with` site `composition::prepare::prepare_values_on`.
  It is a stage of the sanctioned owner (frontmatter surface, shells disabled,
  no prompt), like `read_selection_hints`/`compose_bootstrap`.
- Not in this phase (per plan): the CLI root does not pass a captured context
  or caller layers to the resolver yet (Wave 7), and `Loader::load_sequence`
  inline children do not use the formal pipeline yet (Wave 6).

### Requirement → test mapping (Phase 2)

All L1, in-process at the library's public entry points (F1/F2). The
boundary is document loading and preparation (filesystem fixtures in temp
dirs, the captured `ComposeContext`, the real schema layer).
`lib/src/composition/sequence/document/tests.rs`:

- Markdown/YAML projection parity, root `$schema` kept, `$state`/`template`
  dropped at the root, in nested containers, and in a group leg's container,
  a plain value's `template` key kept, and root/nested key order kept:
  `both_formats_compose_without_their_declarations_and_keep_the_document_schema`
- An ordinary document's `template` property, a reference-only sequence, and
  an undeclared list are unchanged: `a_document_without_list_declarations_is_unchanged`
- Authored and composition rereads (fresh text, nested declarations kept vs.
  dropped): `the_two_rereads_differ_only_in_the_declarations`
- A `file(eager)` setter supplied from another directory is converted before
  `plan: "{{ dirname(spec) + '/plan.md' }}"` reads it, no frontmatter shell
  runs, and the body is not read (both formats):
  `document_values_are_converted_before_a_derived_property_reads_them`
- A missing document input on `sequence: []` is the typed `MissingProperties`
  report; a withheld verdict returns values:
  `a_missing_document_input_is_typed_unless_the_verdict_is_withheld`
- Markdown and YAML lists are filled and typed alike, with one template value
  that reads an item-only field and one that reads a setter-supplied document
  input; Markdown `$state` failure names the step; `document_scope` set:
  `markdown_and_yaml_lists_are_filled_and_typed_alike`
- Referenced whole YAML/Markdown/JSON/JSON5 documents are their own owners:
  own value over referrer, setter over own value, `file_exists` resolved
  beside the declaring file, `document_scope` is the target:
  `a_referenced_whole_document_fills_its_template_from_its_own_values`
- Iteration 4 correction (2026-11-15): the former lowest-referrer-layer test
  was replaced by
  `a_referenced_template_never_reads_a_value_only_its_referrer_defines`,
  which passed in this review's sequence run. The current contract excludes
  referring-document properties rather than assigning them low precedence.
- Passive `$state`: a declared default is never written into the step:
  `state_validation_never_writes_into_the_step`
- Offset and offset+operator selections ignore the selected file's
  declarations and carry no scope:
  `a_selection_ignores_the_selected_documents_declarations`
- Document missing-input subject rendering (root without a container, a
  referenced owner with its container path, no step 0, mixed counts):
  `composition::error::tests::sequence_missing_properties_heads_a_document_entry_by_its_file_and_container`;
  existing step-rendering tests were adapted to the new type.

The Input Robustness Matrix was not re-walked here. This phase adds no new
field validator (declaration shapes are Phase 1's validators), and the
whole-document loaders are unchanged. The JSON5 syntax row (comments,
trailing commas) is exercised by the referenced-document test above.

### Gates (Phase 2)

- `just lint` (claudine/): passes.
- `just test` (claudine/): 12,198 passed, 0 failed, exit 0. The first run
  failed only the R1 `compose_with` guard, fixed as above. 84 tests were
  recorded at or above 5 s; the slowest was 8.07 s, so none reached 10 s.
  The host load average was 145–210 on 16 cores from other sessions. None of
  the slow tests is new or touches Phase 2 code: they are the schema-matrix,
  shipped-corpus, container-schema-recursion, and preflight shell-template
  tests that drive `DarkmatterSchemas` or preflight directly, and
  `shipped.json` did not change. Re-measurement could not clear them on this
  host. A serial `-j 1` rerun at load 145–200 came out slower (one test hit
  the nextest timeout). Alone, at load about 120–166:
  `group_group_restricts_cancelled` 12.5 s, then 6.6 s;
  `deep_container_deferral_…` 15.1 s, then 8.8 s; `json_trigger_combinator::duplicate`
  12.4 s; `frozen_cli_review` 6.9 s; `shared_payload_properties_have_identical_types` 8.7 s.
  Phase 1 measured the same tests at 2.6–3.6 s alone on a quieter host. I
  tried to time them on build-linux with `just cross-check … -E '<filter>'`,
  but the remote recipe re-splits a filterset argument (bash syntax error at
  `(`), so no idle-host measurement exists. **These remain uncleared
  load-sensitive timings, not cleared by the 5 s re-measurement rule.**
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`) depend
  on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check suite run. The new path handling is limited to
  reading files the resolver already resolved. Test path checks compare
  trailing components, so the platform's path form (verbatim prefix, short
  names) cannot fail them. CI's nightly covers Windows and WSL2.

## Phase 3

Phase 2's changes were still uncommitted when this phase started, so the
working tree holds Phases 2 and 3 together. This phase's own files are listed
in `source_files_during_phase_3`.

### Wave 5 — Input inventory (rules 12, 13 library side)

- New `lib/src/composition/sequence/preflight/inputs.rs`:
  `discover_sequence_inputs(source, InputDiscoveryOptions)` returns a
  `SequenceInputInventory`:
  - `owners` (`InputOwner`): the root, the whole document a root
    `sequence:` references (`RootReference`), and every whole document a
    nested `sequence:` task references (`Nested`), whether it is reached
    inline, from a group leg (inline, group file, or catalog entry), or from a
    `kind: task` file. Owners are listed in first-encounter order, including
    owners with no steps. Each owner carries its file, the referencing file
    and container property (`sequence[2].group.tasks[0].sequence`), its
    prepared values (verdict withheld), and its missing inputs as a
    `SequenceMissingSubject::Document` entry.
  - `prompts`: each distinct prompt reference (path plus `with:`), with its
    nearest owner.
  - `inline_containers`: check-only inline containers, kept apart from owners.
    Their `$schema` and `$state` never become inputs.
  - `pending`: a reference that reads a `{{ … }}` (an authored one, or one a
    `template:` default fills) while its owner is missing an input. It is not
    resolved, and is reported only through that owner's missing entry.
  - `missing_entries()` / `missing_error()` give the combined typed
    `SequenceMissingProperties`.
- Owner preparation is Phase 2's `prepare_sequence_document_values`: verdict
  first, then, only on `MissingProperties`, once more with the verdict
  withheld to get the values. Any other preparation error (an invalid
  supplied value, an unloadable schema) fails discovery. Caller layers are
  applied by `CallerInputLayers::apply_to` and never mutated.
- The walk is a `Loader` (its `resolve_reference`, `load_data_document`,
  `load_prompt`, `enter`/`leave`, cycle check, and catalog selection).
  `InputDiscoveryOptions::with_invocation` gives it per-repository source
  contexts, as the graph has. The inventory keeps the walk's `NestedSources`,
  and `build_preflight_graph_after_discovery` hands them to the graph walk, so
  a referenced file is parsed once for both passes.
- Every authored item of an owner list is judged by `validate_task_grammar`,
  which also judges every inline list nested in it. Malformed declarations
  therefore fail behind a false `when:` too.
- A `template:` executable default is merged in before the item's executable
  is read: literal values directly, and expressions filled with
  `formal::fill_template_default`, a new one-key form of `apply_template`.
- Iteration 4 correction (2026-11-15): the former statement that root-reference
  rows resolve from the invoking file described the defect repaired by R1-F2.
  Root and nested whole-document rows now resolve from their declaring file.
  Verified in `Loader::walk_plan`, discovery, and the passing relative-reference
  and `with_file_provenance` CLI tests.

### Wave 6 — Scoped graph (rules 8, 15, 17, 19)

- `Loader::load_sequence` (inline) now goes through
  `formal::normalize_formal_plan_prepared` with the task's own
  `template`/`$state` (`declarations::inline_container_declarations`, a new
  shape-only form of `task_declarations` for a task known to hold a list). The
  values are the walk's current `step_frontmatter`, that is, the enclosing
  document's values. A parent's template and `$state` stop at the container
  row. The parent's `$state` still judges that row's own state.
- `normalize_formal_plan` now locates a nested `$state` at its owner's path
  (`sequence[1].$state`); a root `$state` keeps the property `$state`.
- A nested whole-document reference passes the walk's captured
  `ComposeContext`, so its template reads its prepared values (for example, a
  caller's `file(eager)` setter converted by the document's `$schema`).
- `PreflightSequence::inline_schema: Option<PreparedSequenceSchema>` is
  prepared beside the container (`tasks[0].$schema`). It is used nowhere else:
  not in `owner_schema` typing, provider prediction, inventory, or parent
  state. `PreflightSequence::document_scope()` exposes the plan's document
  scope (the referenced file for `sequence: child.yaml`).
- Declarations are cached in `NestedSources::prepared_schema`, keyed by
  (file, property). An entry is reused while the declaration value and the
  bytes of every schema file it read (`EffectiveSchema::dependencies`) are
  unchanged. New counter `Work::SequenceSchemaPrepare`, counted inside
  `PreparedSequenceSchema::prepare`.
- Rule 19: `prepare_nested_sequence` calls `load_sequence_with(…, refresh =
  true)`. It rejudges the reread container's declarations (shape and
  placement), prepares its current check, and rereads its events. It
  discovers command sites from the reread rows as authored, with plain
  `normalize_plan_at` (no template fill, no `$state`), so the existing refusal
  of unapproved commands still holds
  (`changed_nested_source_refuses_new_commands_before_start` passes). The
  frozen plan, ids, template-filled state, steps, and loop controls are kept.
  A reread whole document's `formal_keys` and root grammar are judged the same
  way. My first attempt walked the frozen children instead, which silently
  dropped that refusal. The existing test caught it.

### Requirement → test mapping (Phase 3)

All tests are L1, in-process, at the library's public entry points
(`discover_sequence_inputs`, `build_preflight_graph*`,
`prepare_nested_sequence`), with filesystem fixtures in temp dirs (F2). The
boundary is document loading and preparation through the real schema layer.

`preflight/inputs/tests.rs`:
- Owner file/property metadata for root, nested, group-leg, and task-file
  owners, plus an empty owner, in YAML and Markdown roots; no inline/`$state`
  inputs; the combined report's `Document` subjects; prompt ownership:
  `every_owning_document_is_inventoried_with_its_file_and_container`
- Independent requirements: `a_supplied_input_clears_only_the_owner_that_declares_it`,
  `supplying_every_owners_inputs_clears_the_report`
- The spec's `file(eager)` reproduction: pending template and authored
  references while `spec` is missing, a literal reference still resolved, and
  caller-relative conversion once `spec` is supplied:
  `a_reference_reading_a_missing_input_is_pending_until_the_input_is_supplied`
- Root reference (JSON5 whole document, with comments and a trailing comma) is
  an owner; an offset selection is not:
  `a_root_reference_is_an_owner_and_a_selection_is_not`
- Malformed declaration behind `when: "false"`, an invalid supplied value
  (`SchemaValidation`, not missing), and a cycle:
  `malformed_declarations_and_invalid_values_fail_discovery`
- No frontmatter shell, prompt shell, or `initialize` action runs:
  `discovery_runs_nothing`
- Shared parse with the graph: `counted::the_graph_walk_reuses_the_parses_discovery_made`

`preflight/tests/scoped_graph.rs`:
- Own template, enclosing values, root template stopping at the container:
  `each_inline_list_is_filled_by_its_own_template_from_the_enclosing_values`
- Immediate-row ownership; parent-row validation at the container boundary;
  file and full container path for inline, group-leg, and task-file owners:
  `each_state_contract_judges_only_its_immediate_rows`
- Inline check prepared and located; not a document scope; a referenced
  document has none; unloadable check is `SequenceDeclarationLoad`:
  `an_inline_check_is_prepared_beside_its_container_and_not_a_document_schema`
- Frozen rows, ids, and template state across a reread that changes the
  template, `$state`, and check; a reread malformed `$state` is a shape error:
  `a_reread_judges_declarations_but_keeps_the_frozen_rows`
- Nested whole document filled from its own prepared values:
  `a_referenced_document_fills_its_rows_from_its_own_prepared_values`
- Preparation reuse, with no schema resolved per row (12 rows → 2
  preparations; 3 unchanged rereads → 0):
  `counted::declarations_are_prepared_once_per_owner_revision`
- Schema-file freshness: `counted::a_changed_schema_file_invalidates_the_prepared_check`

Input Robustness Matrix: this phase adds no field validator. Declaration
shapes are Phase 1's validators, which the inventory calls, and one control
re-proves the wiring (`$state: 42` behind a guard). The loaders are the
existing ones, reused through the `Loader`. The JSON5-specific syntax row is
covered by the root-reference test.

### Observations for later phases

- Authored (not supplied) `file(eager)` values in a referenced document
  resolve from the request's file context, not the document's directory. A
  probe showed `spec: work/spec.md` in `nested/child.yaml` failing to resolve
  from the root. Setter values convert correctly. This is how the preparation
  layer behaves today; I did not change it.
- The `Loader` builds caller layers for nested references from its
  `caller_setters` against the referring file's context, so their recorded
  origins are not the caller's own. Wave 7 should pass the invocation's real
  `CallerInputLayers` through.

### Gates (Phase 3)

- Scope calculator (`git status … | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`, so the gates ran. The working tree still holds
  Phase 2's uncommitted files too. Only `claudine` (lib) source changed in this
  phase.
- `just lint` (claudine/): passes. The first run caught a `type_complexity`
  (now `DeclarationKey`) and a needless lifetime in a test helper.
- `just test` (claudine/): 12,213 passed, 0 failed, exit 0, at load 100–157 on
  16 cores from other sessions. 73 tests were recorded at or above 5 s, and
  the slowest took 8.65 s, so none reached 10 s. I re-measured all 73 with
  `-j 2` as load fell from 127 to 24: 72 finished at or under 5 s. The other,
  `claudine_schemas_embedded::disk_bundle_resolves_its_relative_imports_from_disk`,
  took 5.40 s there and 2.56 s alone at load 22. All are cleared. The one new
  test in the list, `inputs::tests::supplying_every_owners_inputs_clears_the_report`,
  took 5.92 s in the package run and passed under 5 s in the re-measure. I
  had already split it from a 7.4 s combined test, which took 1.8 s alone.
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`) depend
  on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check was run. The new code only adds path keys (it
  compares canonicalized paths the walk already produced) and reads schema
  files by the paths Darkmatter reports. Test path checks compare trailing
  components. CI's nightly runs cover Windows and WSL2.
- Docs: no `docs/` page changed in this phase. The plan gives the topic pages
  for nested declarations and input collection to Phase 6, and the
  user-visible collection behavior is not wired into the CLI until Phase 4.

## Phase 4

### Wave 7 — Container admission (rules 17–19, library)

- `TaskExecution::run_validated` now starts with
  `check_inline_container` (`task/sequence.rs`) for a `sequence:` task. Both
  `run_guarded` (nested children, group legs) and `run_admitted` (root steps,
  through `task_run.rs::run_step_task`) reach it, after the `when:` guard and
  before `run_sequence` replaces the lexical scope or opens the guard. A
  refusal is an unstarted failed `TaskOutcome` carrying
  `SequenceContainerSchemaViolation`: no `initialize`, no child, no
  `finalize`, no mutation, no output. The enclosing container routes it as
  any failed task. `task_run.rs` needed no change.
- The instance judged is the task's state (`self.state.data()`, the
  enclosing effective values), then the caller's setters, then every runtime
  write so far. It is passed by value to `PreparedSequenceSchema::validate`,
  whose converted copy is dropped.
- A dry run emits one note (`` `tasks[0].sequence[0]` checks its `$schema` at
  its turn (not evaluated in a dry run) ``) through the existing dry-run note
  path and judges nothing.
- At each `start` reread (first pass, retry, each loop pass),
  `prepare_nested_sequence`'s returned container is checked again with its
  current `$schema` against the current values. A failure there is treated
  like a failed reread: the container is `blocked`.
- `run_container_proxy` runs the same check before `run_sequence` for an
  approved sequence target.
- Runtime and `with:` declaration-write refusal, and the sequence-scoped
  runtime constructors, already landed in Phase 1 (`set_batch` checks the
  whole batch before it commits). Nothing was left to do there.

### Wave 7 — CLI preparation (rules 11, 13, 16)

- `commands/sequence.rs::run_sequence_inner` no longer builds the plan.
  Before `initialize` it judges the root's declarations (`formal_keys`) and
  the grammar of every inline root item (`validate_task_grammar`), and
  allocates the invocation token (`normalize::allocate_sequence_id`, new).
  Invalid optionals are dropped from the *authored* source first, then the
  composition view is projected from it. Both go to `execute_sequence` as
  `wrap::sequence::RootSequence { sequence_id, authored, shell_runner }`.
- `execute_sequence` binds `seq` with that token for `initialize`. On the
  first attempt, `execute_sequence_inner` calls
  `phase1c::prepare_root_plan`, then
  `build_preflight_graph_after_discovery`. The plan is re-stamped with the
  token (`SequencePlan::with_sequence_id`, new). `RootRun` keeps the plan and
  the collected setters (`plan`, `collected_overrides`), so a retry runs the
  first plan and never asks again.
- `prepare_root_plan` loop:
  1. `discover_sequence_inputs` under the caller's layers plus the writes
     `initialize` made (above the setters), with `leaving_verdicts()`.
  2. With nothing missing, fill the plan (captured context, the same layers)
     and return.
  3. When only the root is missing inputs, no reference is pending, and the
     plan builds with at least one step, return too: Phase 1c reports the
     root's inputs per step, as before.
  4. Otherwise collect, or return the combined report. A pass that would ask
     nothing new returns the report.
- Phase 1c's two-attempt loop is replaced by the same progress-bounded rule.
  `asked` (one `CollectionKey` per `(name, type_label, description)`) is
  shared with discovery, so no property is asked twice.
- `leaving_verdicts()` (new `InputDiscoveryOptions` builder) records owners
  whose own preparation failed for a reason other than a missing input in
  `SequenceInputInventory::failed_owners`, instead of failing discovery.
  Without it, the order of errors changed: 48 existing CLI tests failed,
  because the owner verdict reported a schema problem before the graph
  (`` `tasks` must declare at least one task ``, `when: ""`), provider
  selection (`agent: []`), or Phase 1c (removed-grammar paths) could. The
  root's failure is left to Phase 1c's per-step probes. Any other owner's,
  or an empty root's, is raised by `RootPlanRequest::recheck_owners`, a
  strict rediscovery right after provider selection.
- During Phase 4, discovery filled a referenced document's template from the referrer's
  values below the document's own values. Phase 2's transitional layer in
  `plan_referenced_sequence` already did this. Without it, every shipped
  research fleet failed (`prompt: must be a non-empty file reference, got
  null`), because discovery read `{{ research_prompt }}` from the roster's
  own values only. Iteration 4 correction (2026-11-15): both transitional
  fallback merges were removed by the first repair; current discovery excludes
  referrer properties. The owner-isolation and research artifact tests passed.
- `build_preflight_graph_after_discovery` now takes the invocation's
  `CallerInputLayers` instead of bare setters. The `Loader` keeps them
  (`caller_layers`) and hands them to a referenced document's template, so a
  caller's file value keeps the caller's recorded origin (Phase 3's
  handoff note).

### Requirement → test mapping (Phase 4)

Library, F2 (real files in temp dirs, in-process `TaskExecution`),
`task/tests/inline_check.rs`:
- An earlier `set` satisfies the check: `an_earlier_set_satisfies_the_check`
- A setter satisfies it: `a_caller_setter_satisfies_the_check`
- A missing value refuses before `initialize`/children/`finalize`, the
  ancestor's `failure` fires, and nothing is written:
  `a_missing_value_refuses_the_container_before_it_opens_and_the_ancestor_handles_it`
- A wrong type is refused with its property:
  `a_wrong_typed_value_is_refused_with_its_property`
- A false guard skips the check: `a_false_guard_skips_the_check`
- Converted (`"3"` under `number`) and defaulted copies are never committed:
  `a_defaulted_or_converted_validation_copy_never_changes_the_enclosing_values`
- Dry run names the check without judging it:
  `a_dry_run_names_the_check_without_judging_it`
- Loop pass boundary rejudges with the pass's values (passes 1 and 2 run,
  pass 3 is refused): `each_loop_pass_judges_the_check_against_its_own_values`

Library, `preflight/inputs/tests.rs`:
- Iteration 4 correction (2026-11-15): the former referrer-layer test was
  replaced by
  `a_referenced_template_never_selects_a_prompt_from_its_referrers_values`;
  it passed and verifies that discovery never selects a prompt using that layer.
- `leaving_verdicts` records instead of failing; strict discovery fails:
  `leaving_verdicts_records_a_failed_owner_instead_of_failing`

CLI wiring, F3 (spawned `claudine` through `CliProcessFixture`, portable, no
shell stubs), `sequence_sources_cli.rs::document_inputs`:
- A nested referenced document's missing input is a `Document` entry with
  its container path, not a step:
  `a_nested_referenced_documents_missing_input_is_reported_as_that_document`
- Setter order independence (before/after the file):
  `a_setter_before_or_after_the_file_supplies_a_nested_documents_input`
- Empty root list and empty root-referenced document still require inputs:
  `an_empty_root_list_still_requires_its_inputs`,
  `an_empty_root_referenced_document_still_requires_its_inputs`
- Collection starts nothing (no root `start` action):
  `collecting_a_missing_input_starts_nothing`
- A required `$state` field is a state violation, not an input:
  `a_required_state_field_is_a_state_violation_not_a_missing_input`
- An inline check is never an input; preview names it; a run refuses the
  container before its `start`:
  `an_inline_check_is_never_an_input_and_refuses_its_container_at_its_turn`
- `initialize` writes reach the root template (before this phase it read
  `nobody`): `a_root_template_reads_what_initialize_wrote`
- A malformed declaration behind a false guard fails before `initialize`:
  `a_malformed_declaration_fails_before_initialize`

Interactive collection (the progress-bounded loop under a TTY) has no new
PTY test in this phase. The loop shares `collect_or_report` with the
existing Phase 1c path. Phase 5's CLI regressions own the frozen
reproduction and the interactive cases.

Input Robustness Matrix: this phase adds no field validator or loader. The
inline check reuses Phase 1's prepared declaration. Its instance shapes
(missing, wrong type, converted, defaulted) are the library rows above.

### Focused runs (Phase 4)

- Library `composition::sequence` filter: 697 passed (two over 5 s at load
  60–90 from other sessions; both re-measured in the gate run below).
- CLI `sequence|schema|phase|preflight` filter: 1351 tests, 48 failed on the
  first run, all from the ordering and fleet issues above, then 1359 passed
  after the fixes. `sequence_task_providers::empty_hint_nested_sequence_list_fails_its_body_step_before_any_task_launches`
  took 5.77 s in a 470-test run, and 2.48 s alone at load 87.

### Gates (Phase 4)

- Scope calculator (`git status … | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`, so the gates ran.
- `just lint` (claudine/): passes.
- `just test` (claudine/): 12,232 passed, 0 failed, exit 0. 43 were
  recorded at or above 5 s, the slowest at 9.23 s, so none reached 10 s. Load
  stayed between 90 and 140 on 16 cores, from other sessions. I re-measured
  all 43 with `-j 2` as load fell from 121 to 39. All 7 CLI tests finished at
  or under 2.92 s. Four lib `claudine_schemas` tests (untouched by this
  phase) stayed at 5.3–6.7 s there. Each took 2.0–4.4 s alone (`-j 1`, load
  32–37). All 43 are cleared.
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`) depend
  on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check was run. The change adds no path handling, no
  platform branch, and no process spawn. The new CLI tests use only portable
  effects (`append_line`, dry-run rendering), with no shell stubs. CI's
  nightly runs cover Windows and WSL2.
- Docs: no `docs/` page changed. The plan gives the topic pages for input
  collection and inline checks to Phase 6. The repo skill's container
  preflight paragraph now describes the CLI wiring.

## Phase 5

Phase 5 is coverage. Two subagents worked in parallel: Library coverage
(library test modules only) and CLI regressions (CLI test files and the frozen
fixture). The CLI agent stopped before it reported. A second CLI agent
reviewed and finished its work. Library coverage found two real defects, and
both are fixed in this phase (see "Defects found and fixed").

### Wave 9 — Library coverage

All new tests are in existing library test modules under
`lib/src/composition/sequence/`. The one new submodule,
`task/tests/declaration_writes.rs`, is registered in `task/tests.rs`. No new
test binary was added. Short names below: `S` = `claudine::composition::sequence`,
`T` = `S::tests`, `D` = `S::declarations::tests`, `Doc` = `S::document::tests`,
`G` = `S::preflight::scoped_graph_tests`, `I` = `S::preflight::inputs::tests`,
`K` = `S::task::tests`.

Matrix accounting, row by row:

- **Declaration placement and presence.** Covered by existing tests:
  `D::{root_declarations_beside_a_non_list_source_are_misplaced, task_declarations_beside_a_non_list_are_misplaced, inline_containers_accept_their_own_declarations}`.
- **Schema declaration, full matrix (new).** `D::every_state_form_resolves_from_the_declaring_document`.
  - Accepted: `{}`, a map, a file reference, an imported `Name@file` type,
    `file(eager)`, and a union of a map and a reference.
  - Refused with the schema layer's cause and the owner path: an unloadable
    reference, a missing import file, a missing named type, and a union with
    a bad arm, reported at the failing arm (`$state[0]`, `$state[2]`).
  - Caller, declaring document, schema files, and eager file are each in
    their own directory.
  - Empty union, empty string, and null were already covered by
    `D::declaration_shapes_are_judged_by_presence_on_an_empty_list`.
- **Other declaration homes, one positive and one invalid case each.**
  - Inline `$state`/`$schema` (new): `G::inline_declarations_resolve_from_their_declaring_file`.
  - Referenced `$state`: existing `D::a_state_reference_resolves_in_the_declaring_documents_context`
    plus new `D::a_referenced_documents_unloadable_state_is_owned_by_that_document`.
- **Template map.**
  - Absent, `{}`, scalar/array/null whole templates, and reserved keys on
    zero rows were already covered.
  - New: `T::robustness_matrix::template_values_are_copied_as_data_and_only_top_level_strings_render`
    covers null, `""`, array, and object values, which are copied as data;
    only top-level strings render.
- **Sequence source and list.**
  - Existing: `T::empty_list_returns_empty_plan` and `T::no_sequence_key_returns_none`
    (the `Ok(None)` case).
  - New, in `T::robustness_matrix`: `a_null_or_wrong_typed_sequence_field_fails`;
    `strict_lists_refuse_each_non_step_item_by_index` (null, number, bool,
    and array items, one among valid items and as the whole list);
    `lenient_data_lists_coerce_scalars_but_never_drop_an_item`.
- **State field validated by `$state` (new).** `D::the_state_validator_judges_every_field_shape_passively`.
  - Refused: absent, explicit null, wrong whole type, mixed invalid
    elements, and all elements invalid.
  - Accepted: `""`, `[]`, `{}`, and `"3"` for a number. The returned plan
    keeps the value as authored.
  - Per-route controls already existed:
    `Doc::markdown_and_yaml_lists_are_filled_and_typed_alike` and
    `G::each_state_contract_judges_only_its_immediate_rows` (inline, group
    leg, task file).
- **Document input values.**
  - New: `Doc::absent_and_null_document_inputs_follow_the_canonical_rules`.
    A default fills both absent and null, a supplied value is coerced, and a
    required null is classified as missing.
  - New: `Doc::authored_file_values_keep_their_text_and_setters_are_converted_from_the_caller`
    and `Doc::a_referrers_property_never_satisfies_a_referenced_documents_input`.
  - Declaring-file resolution: `I::a_referenced_owners_authored_file_value_keeps_its_text_and_a_setter_its_origin`
    puts the file only beside the declaring document, and the setter's file
    only beside the caller. `I::a_referenced_owners_authored_file_value_is_not_found_beside_the_caller`
    is its negative control. Both rely on defect 1's fix.
- **Inline check values.** Phase 4's `K::inline_check::*` tests, plus new
  `K::inline_check::an_explicit_null_is_refused_and_an_admitted_empty_value_passes`.
- **Declaration write routes.**
  - New: `K::declaration_writes::a_with_key_naming_a_declaration_fails_before_the_provider_runs`.
  - New: `K::declaration_writes::a_set_batch_naming_a_declaration_publishes_none_of_its_writes`
    (the batch is atomic; a control batch commits).
  - Setters, template keys, and authored inline options were covered in
    earlier phases.
- **YAML loader (new, in `T::loader_matrix`).**
  - `both_yaml_loaders_refuse_duplicates_trailing_syntax_and_a_second_document`:
    a duplicate key at the root and at nested declaration depth, invalid
    trailing syntax, and a second document, plus a control.
  - `a_bare_yaml_key_is_an_explicit_null_on_every_route`: direct and
    referenced, each with a control.
  - `a_referenced_yaml_documents_duplicate_key_fails_the_resolver`.
- **JSON family.**
  - Root and nested duplicate keys were already covered by
    `T::json_duplicate_keys::*`.
  - New: `T::loader_matrix::the_json_family_refuses_trailing_garbage_and_accepts_comments_and_trailing_commas`,
    run under both `.json` and `.json5`.
- **JSONL (new).** `T::loader_matrix::line_delimited_records_skip_blank_lines_and_name_a_trailing_invalid_record`:
  blank lines are accepted, a trailing bad record is named by its line, and
  a file of records owns no declarations.
- **Markdown frontmatter.** New: `T::loader_matrix::markdown_frontmatter_nulls_and_duplicates_reach_the_resolver`.
  Declaration protection was already covered by
  `Doc::both_formats_compose_without_their_declarations_*`.
- **Ownership.**
  - Already covered in `Doc::*`, `G::*`, and `T::*`: Markdown/YAML
    equivalence, immediate container-row state, inline non-inheritance, item
    shadowing, setter precedence, and nested, group-leg, and task-file owners.
  - New: `G::a_parents_declarations_stop_at_a_referenced_child`.
- **Work counters (new).** `G::counted::a_referenced_owner_resolves_one_contract_and_a_reread_regenerates_nothing`:
  - 20 root rows and 20 child rows prepare one contract each;
  - three rereads prepare and parse nothing;
  - a template changed on disk never refills frozen rows.
- **Renames.** `T::formal_documents::templates_apply_before_generated_fields_and_satisfy_the_state_declaration`
  and `T::formal_documents::a_state_violation_reports_the_step_and_property`.
  No old row-schema names remain.

### Wave 9 — CLI regressions

All CLI tests are F3: they spawn `claudine` through `CliProcessFixture`, with
portable launch counting (`claudine-fake-goose` + `PROVIDER_MARKER`) and
`append_line` effects. None uses a shell stub. Paths are under
`cli/tests/l1/`.

- **Frozen `complete.yaml` reproduction.** Fixture files are
  `cli/tests/fixtures/frozen_prompts/sequence-state-schema/{complete.yaml,plan.md,spec.md}`.
  Each `#[test]` includes them by literal path with `include_str!`. The
  tests are in `sequence_sources_cli.rs::frozen_complete_reproduction`:
  - `a_setter_after_the_file_is_judged_as_a_document_input_and_reaches_the_nested_step`:
    the caller's directory differs from the sequence's. `spec` and the
    derived `plan` are absolute paths under the caller's `work/feature/`,
    and both reach the nested `plan.md`, with 0 launches.
  - `a_setter_before_the_file_is_judged_the_same_way`: argument order does
    not change the result.
  - `a_missing_spec_is_reported_as_a_missing_input_without_composing`: a
    non-TTY run exits nonzero with a typed report naming `` `spec`: file ``,
    and launches nothing.
  - `the_step_guard_reads_the_plan_derived_from_the_coerced_spec`: one
    launch while the plan is absent, then skipped once it exists.
- **The three formal tests** in `sequence_sources_cli.rs` use `$state`. The
  direct and referenced ones now assert that a composed `blue/5` reaches the
  step before they assert the located violation.
- **Route wiring.** `sequence_sources_cli.rs::declaration_routes::*` (6
  tests) is a control plus invalid cases for the Markdown-frontmatter and
  YAML loaders, each with a located diagnostic and 0 launches.
- **Referenced-document inputs**, in `sequence_referenced_state.rs`:
  - `a_root_reference_fills_its_template_from_its_own_input_value`
  - `a_setter_outranks_a_root_references_own_input_value`
  - `a_nested_reference_reads_its_own_input_value_and_no_referrer_property`
  - `a_setter_outranks_a_nested_references_own_input_value`
  - `a_root_references_missing_input_joins_the_roots_in_one_report_and_runs_no_event`
  - `a_nested_references_missing_input_joins_the_roots_in_one_report_and_runs_no_event`
- **Declaring-folder wiring**, for defect 1:
  `sequence_sources_cli.rs::document_inputs::a_nested_documents_authored_file_value_resolves_from_its_own_folder`.
- **Frozen graph**, in `sequence_frozen_graph.rs`:
  - Rereads after a declaration edit: `a_reread_root_state_declaration_never_retypes_a_referenced_documents_rows`,
    `a_reread_state_declaration_never_retypes_the_roots_inline_container`,
    `…_an_inline_container_inside_a_referenced_document`, and
    `…_a_group_legs_inline_container`.
  - Root retry after a declaration edit:
    `a_root_retry_after_a_state_declaration_edit_runs_the_untyped_rows`,
    for defect 2.
  - `a_template_keeps_its_planned_value_across_loop_passes`
  - `a_retry_that_changes_a_document_input_keeps_the_planned_ids_and_template_state`
  - `a_retry_after_a_failed_inline_check_runs_the_frozen_graph`
  - `a_failed_inline_check_never_reopens_command_approval`: `echo
    forbidden` is refused even with `--yolo`.
- **Dry run.** `sequence_dry_run.rs::a_dry_run_names_an_inline_check_that_a_runtime_write_satisfies`
  covers a write from `initialize` and one from an earlier task's `set`.

Interactive (PTY) collection still has no new test. `sequence_overlay_pty.rs`
and the L2 suite own terminal sessions. The non-TTY report and the
progress-bounded loop share `collect_or_report`.

### Defects found and fixed

1. **A referenced or nested owner's authored file values resolved from the
   caller's folder** (Phase 3 recorded this as an observation). Discovery's
   `Discovery::push_owner`/`options_for` and `document.rs::owner_values`
   built their `PrepareOptions` from the caller's file-resolution context.
   They now derive the owning document's context the way the graph does,
   through `Loader::source_context_for`, which calls
   `InvocationContext::derive_source`.
   - The new `PrepareOptions::resolving_document_from` swaps that context in
     after `CallerInputLayers::apply_to`, so setters still convert from the
     caller's recorded origin.
   - `SequenceSourceOptions::with_invocation` carries the invocation to
     `owner_values`. `phase1c::RootPlanRequest::plan` and the preflight
     `Loader` pass it.
   - An earlier attempt with `FileResolutionContext::for_source` failed on
     `/var` vs `/private/var`; `derive_source` does not have that problem.
   - `docs/topics/flow-control/sequences.md` ("What a referenced sequence
     reads") now says where relative file values resolve from.
2. **A root retry re-planned nested references under the declarations now
   on disk.** The retry's approval-check rebuild used
   `build_preflight_graph_with_invocation`, whose walk plans nested
   references fresh, so a `$state` added to `child.yaml` failed the frozen
   rows before the first attempt's graph ran. The new
   `reread_preflight_graph_with_invocation` (`preflight/mod.rs`) walks with
   `refresh = true`, as nested and group rereads already do: declarations
   are judged again, and rows are read as authored for command discovery
   only. The CLI retry arm in `wrap/sequence/mod.rs` calls it. The
   approval refusal still holds
   (`a_failed_inline_check_never_reopens_command_approval`).

3. **A nested reread re-typed containers below its top level.** The
   `refresh` flag was passed only to the container being reread, so an
   inline container inside a referenced document, or inside a group leg,
   was planned again with its current `$state`/`template:`. `Loader` now
   carries a `refresh` field, set by `prepare_nested_sequence` and
   `prepare_nested_group`, and every container the walk loads, at any
   depth, uses it. The first CLI agent made this change, and the edit is
   not in its (missing) report. I found it on disk with its skill sentence
   and the tests `sequence_frozen_graph.rs::a_reread_state_declaration_never_retypes_*_an_inline_container_inside_a_referenced_document`
   and `…_a_group_legs_inline_container`, which pass. The retry fix (2)
   uses the same field.

### Robustness smell audit

`rg 'serde\(default\)|filter_map|unwrap_or_default\(\)|\.ok\(\)|Option<'`
over `declarations.rs`, `formal.rs`, `source.rs`, `document.rs`,
`preflight/inputs.rs`, and `task/sequence.rs` found no `serde(default)` and
no `.ok()`. No hit is a defect:

- The `Option<&Value>` declaration fields come from `map.get`, so a present
  null stays `Some(Null)` and is refused by the shape check.
- `declarations.rs` returns `None` for an open `{}` schema.
- `source.rs`'s `unwrap_or_default()` applies to a data document's
  frontmatter text, which does not exist.
- `inputs.rs`'s `filter_map` collects missing-input entries and drops no data.
- `document.rs` returns `None` when a document has no `$schema`.
- The `task/sequence.rs` hits are runtime plumbing, plus a `filter_map` that
  skips body steps, which have no task.

### Wave 10 — Regression checkpoint and gates (Phase 5)

- Focused runs, in order:
  - The library's `composition::sequence` filter: 722, then 776 after the
    owner-context fix.
  - The CLI's source/schema/referenced-state/frozen-graph/dry-run/loop
    filter: 152 passed after the retry fix. The retry test failed before it.
  - Schema parity and freshness (`claudine_schema_parity`,
    `shipped_artifacts`, `claudine_schemas_embedded`, `sequence_schema`,
    `container_schema_recursion`): 479 passed.
- First `just test`: one failure. `claudine_schema_shipped_corpus::every_shipped_document_has_one_test`
  requires every `.md` under `frozen_prompts/` to have a corpus test, and
  the new fixture's `plan.md` and `spec.md` had none. I added
  `frozen_cli_sequence_state_schema_{plan,spec}`. The plan step's required
  inputs are absent when it is judged on its own, so it is `Invalid` with
  exactly those two problems.
- Second `just test`: one failure. `context_construction_guard::production_source_builds_contexts_only_through_the_builder`
  refused the new `Option<&FileResolutionContext>` parameter on
  `Discovery::options_for`. I restored that function's signature and moved
  the document-context swap into `push_owner`'s closure.
- Third `just test` (claudine/): 12,285 passed, 0 failed, exit 0, at load
  150–190 on 16 cores from other sessions. 17 tests were recorded at
  5.0–7.3 s, so none reached 10 s. Re-measured alone (`-j 1`, load 170 → 93),
  all 17 took 1.3–2.7 s and are cleared. Three of them are new this phase:
  `sequence_dry_run::a_dry_run_names_an_inline_check_that_a_runtime_write_satisfies`
  (7.2 s in the package run, 1.75 s alone) and the two
  `sequence_referenced_state::*_missing_input_joins_the_roots_in_one_report_and_runs_no_event`
  tests (6.4 s, 1.6 s alone). All three are load-sensitive.
- `just lint` (claudine/): passes.
- `just check-tier-coverage claudine` (repository root): 0 stranded.
- Scope calculator: `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`, so the gates ran. The working tree still holds
  Phase 4's uncommitted files.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`)
  depend on `claudine`, and `just test` covers both.

### Cross-OS check (Phase 5)

Defect 1 changes which folder a file value resolves from, so I ran the
Windows rig (`build-win`, through `scripts/cross-check.sh`).

- `claudine` (lib), full L1: 6,734 of 6,735 passed. The one failure,
  `document::tests::document_values_are_converted_before_a_derived_property_reads_them`
  (Phase 2), wrote a Windows path into a double-quoted YAML scalar, where
  `\U…` is read as an escape. It now uses a single-quoted scalar, and all 13
  `document::tests` pass on Windows.
- `claudine-cli`, full L1: 4,051 of 4,074 passed, 23 failed.
  - 10 new tests failed. Their rows run the templated command `shell: "echo
    {{ … }}"`. A template command runs its program directly, never through
    a shell, and on Windows `echo` is a `cmd` builtin with no program
    behind it.
    - The fix is a test-only `claudine-fake-echo` binary
      (`cli/tests/bin/fake_echo/main.rs`, declared in `cli/Cargo.toml` under
      `test-fixtures`). `CliProcessFixture::install_echo` installs it as
      `echo{EXE_SUFFIX}` in the fixture `bin`, which leads the child's `PATH`.
    - `sequence_referenced_state.rs` and `sequence_frozen_graph.rs` call it
      from `run_sequence`. On Windows, all 47 tests in those two files pass.
    - I first tried a `side_effect: { set: … }` row instead and misread its
      returned prior value as the newly written value. Review correction
      (2026-11-15): a following shell confirms the write is `ada`, including
      at a root-reference row; see the corrected decision above.
  - The 12 other failures are on Windows already, from commits before this
    fix: `implement_plan_commit_message::*` (7; an extra `git config` argv
    is recorded), `lifecycle_edit_action::a_sequence_container_runs_edit_in_its_start_and_finalize_around_its_children`
    (it runs the Unix `test` command), and `runaway_guard::tests::*` (4;
    they read the build host's real `~/.claudine/config.yaml`). None of
    their files changed in this fix, and this phase does not change them.
- I did not set `os_failing`: every Windows failure this phase caused is
  fixed and verified on the rig.

### Final gates (Phase 5, after the Windows fixes)

These runs supersede the gate results in the Wave 10 section above.

- `just lint` (claudine/): passes.
- `just test` (claudine/), first run after the Windows fixes: 12,285
  passed, 0 failed. Load was about 100, and 60 tests were recorded slow.
  Four existing tests crossed 10 s (`agent_text_is_data::inline_agent_values_are_stored_as_data_and_read_back_exactly`
  12.6 s, `shipped_artifacts::the_checked_in_artifact_round_trips_to_uncached_preparation_in_order`
  11.0 s, `agent_text_is_data::inline_agent_added_frontmatter_survives_a_second_run`
  10.4 s, and `container_schema_recursion::deep_container_deferral_does_not_hide_literal_or_shape_errors_in_either_phase`
  10.1 s). With `-j 2`, all 60 finished under 4 s.
- Rerun at lower starting load: `just test` (claudine/) passed 12,285
  tests, 0 failed, exit 0. Load was 43 at the start and 120 at the end. 18
  tests were recorded at 5.0–9.18 s, none at 10 s. Re-measured alone
  (`-j 1`, load 98 → 45), all 18 took 1.1–3.3 s and are cleared. Three are
  new this phase:
  - `sequence_dry_run::a_dry_run_names_an_inline_check_that_a_runtime_write_satisfies`
    (6.0 s in the run, 2.0 s alone)
  - `sequence_sources_cli::frozen_complete_reproduction::the_step_guard_reads_the_plan_derived_from_the_coerced_spec`
    (7.0 s, 2.4 s alone)
  - `preflight::inputs::tests::supplying_every_owners_inputs_clears_the_report`
    (5.6 s, 1.1 s alone)
- `just check-tier-coverage claudine`: 0 stranded.
- Windows: `claudine` (lib) `document::tests` 13/13, and `claudine-cli`
  `sequence_referenced_state` + `sequence_frozen_graph` 47/47.

## Phase 6

Phase 6 is documentation and final verification. No source file changed. I
did the work directly; no subagent ran.

### Wave 11 — Current documentation

Before writing, I checked each documented behavior against the built binary
in an isolated `HOME` (a scratch directory, never the user's config): the
non-TTY missing-input report (a `Document` entry for a root whose steps
compose nothing, `Step N` headings for a root with composing steps), that a
missing input or a `$state` failure after the root's `initialize` runs
`blocked` and `finalize` but no `start`, the `$state` violation text, the
inline `$schema` refusal and dry-run note, the three placement and
setter-write messages, an unloadable `$state` on an empty list, and the new
CLI example (`colors.yaml`, with the setter before and after the file).

- `docs/topics/flow-control/sequences.md`: the old "Formal sequence
  documents" section is replaced by "Sequence documents: `$schema`, `$state`,
  and `template:`". It covers the key split in both formats with both keys
  together, the explicit `$schema` → `$state` rename note, normalization
  order and passive validation, the template's value layers, the
  declaring-file context, declaration forms and load errors, immediate
  ownership, inline container declarations and the turn-time `$schema` check,
  where declarations may appear, reserved writes, exclusion from
  composition, empty lists, data selections, a Mermaid flow from authored
  file to frozen execution (syntax checks versus value checks, and what has
  already run when each fails), and template freezing. The page also changed
  in these places: the container grammar, "What a referenced sequence
  reads" (the document's own `$schema`; at that phase, the transitional referrer
  layer for `template:`), the reserved-keys list, the
  preflight bullet, the dry-run table (the inline check note), and
  "Requiring caller input" (the root's own inputs, which owners are
  collected, collection after `initialize`, the non-TTY report, and what
  never asks). Phase 5's declaring-folder paragraph is kept as it was.
  Iteration 4 correction (2026-11-15): current docs exclude referrer properties
  and describe initialization writes plus the equivalent dry-run caller setter;
  source inspection and the passing research artifact tests verify that behavior.
- `docs/topics/schemas.md`: the `sequence.yaml` row and a new "Sequence
  declarations" subsection. A referenced file is judged by no trigger
  schema, but its own `$schema`/`$state` still apply.
- `schemas/README.md`: the `sequence.yaml` row and a declarations paragraph.
- `docs/cli/sequence.md`: a new "Inputs and step types" subsection, and the
  dry-run note.
- `docs/topics/execution-flow.md`: the external-file shape now names
  `$state`.
- `docs/use-skills/use-claudine/sequences.md`: a new caller section, written
  as a summary rather than a transclusion (see decisions_for_review).
  `just publish-skills` ran from `claudine/`. In the installed copy the
  `{{ … }}` examples stay inside their fence, and the key split is present.
- `.claude/skills/claudine/SKILL.md`: the inline-check sentence now separates
  a refusal at admission from one at a `start` reread, and links the new
  user-doc section.
- No current doc links to or names this fix.
  `fixes/2026-11-15-sequence-state-schema/implementation.md` records the
  changes, tests, limits, and departures (inventory-before-template ordering,
  root empty-list validation, the referrer template layer, and report
  headings). Its kind is `implementation-record`, following the precedent of
  an earlier fix.

### Wave 12 — Final verification

- `just test` (claudine/): 12,285 passed, 0 failed, 15 skipped, exit 0, in
  5 m 23 s. Load was 65–90 on 16 cores. Three tests were recorded slow, at
  5.0–5.7 s, none at 10 s. Re-measured alone (`-j 1`, load 70 → 61):
  - `lifecycle_message_drain::sequence_last_step_success_message_is_delivered_before_exit`:
    2.85 s
  - `lifecycle_prep::prep_dispatches_an_inline_prompt_before_resuming_its_caller`:
    1.40 s
  - `sequence_sources_cli::frozen_complete_reproduction::the_step_guard_reads_the_plan_derived_from_the_coerced_spec`:
    2.33 s

  All three are cleared as load-sensitive.
- `just lint` (claudine/): exit 0.
- `just check-tier-coverage claudine`: 0 stranded.
- The frozen motivating case is covered by
  `sequence_sources_cli::frozen_complete_reproduction::*`, which passed in
  this run. It asserts 0 launches, and that the prepared `spec` and derived
  `plan` reach the nested step. The live prompt was not run.
- Artifact freshness (`shipped_artifacts::shipped_artifact_is_fresh`) and
  disk/embedded parity (`claudine_schemas_embedded`, `claudine_schema_parity`)
  passed in the same run.
- Scope calculator, run over the uncommitted tree:
  `packages: [claudine, claudine-cli, darkmatter]`, `change_class: package`.
  The gates were therefore required, and they ran. Darkmatter reads
  `claudine/schemas/README.md` in
  `trigger_spelling_guard::no_active_file_spells_a_renamed_trigger_form`. I
  ran `trigger_spelling_guard::` in darkmatter: 2 of 2 passed. Claudine's
  dependents are the in-area crates, which `just test` covers.
- Cross-OS: not run. This phase changed documentation only, so it has no OS
  risk. This is macOS evidence only.
- Plan tasks are all checked. Plan, log, and spec completion frontmatter are
  set.

## Review verification corrections — 2026-11-15

At the first review, the implementation record's blanket claim that nested templates read prepared
document values was too broad. With `reviewer: string(default(ada))` in the
enclosing document schema, a root template reads `ada`, but the identical
inline-container template reads null; a derived `reviewer: "{{ source }}"`
remains literal text there. Direct CLI runs reproduced this in an inline
sequence, group leg, external task file, and a referenced document's inline
child. These code defects are recorded in review-1.md.

At the first review, the claim that an inline check always precedes its `initialize` needed a
qualification: admission used the preflight check. When an earlier task edited
that declaration to reject the current value, direct CLI runs emit the
container's `initialize` marker before the `start` reread rejects it. This was
verified for inline sequences, group legs, external task files, and inline
children of referenced documents. The implementation record now states this
limit; review-1.md reports the underlying code defect.

Iteration 2 verification correction (2026-11-15): those two paragraphs describe
the implementation before the first repair, not its current behavior. Nested
templates now use prepared owner values, and inline admission rereads the
current check before initialization. The library's `template_owner_values_tests`
matrix and `task::tests::stale_admission` tests, plus the corresponding CLI
template-fill and tightened-check tests, passed in the iteration 2 sequence
run (2,069 tests, none at or above five seconds). The implementation record
was corrected in place to describe these repairs and the removed referrer
fallback. At iteration 2, a separate shell-binding defect remained; see review-2.md.

Iteration 3 verification correction (2026-11-15): the previous unqualified
statement that the shell-binding defect remains is no longer current. The
derived first-position rows, lifecycle shells, shell-valued writes, and inline
membership controls now pass on their original routes in this review's
`just test sequence` run (2,103 passed; none at or above five seconds).
At iteration 3, the repair-added provider-dependency restoration was only partially correct:
two-link dependencies can be lost and unrelated owners' same-named
dependencies can be imported, as independently observed through the graph
result and local-stub CLI runs. Review 3 records those defects; the original
shell-output issue itself is fixed.

Iteration 4 verification correction (2026-11-15): the preceding dependency
failures describe the implementation before the third repair. Predictions now
retain complete authored dependency closures in their evaluating scopes; the
global name-based restoration list has been removed. Source inspection,
all 58 `provider_dependencies_tests` cases, and both `derived_provider_reads`
CLI reproductions verify that multi-link reads defer correctly and unrelated
owners' properties do not defer a static provider. These passed within
`just test sequence` (2,163 passed, none at or above five seconds).

The side-effect correction above is verified independently of its printed
return value: `template: { who: ada }`, followed by a task setting
`signed: "{{ seq.who }}"` and a later shell reading `signed`, prints
`signed=ada`. The library's `TaskExecution::run_side_effect` serializes runtime
set *prior* values, which explains the misleading null in the original probe.
