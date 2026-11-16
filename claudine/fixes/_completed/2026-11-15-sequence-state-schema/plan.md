---
kind: plan
total_phases: 6
created: 2026-11-15
phase: 7
agent: codex/gpt-6.1-sol
yolo: true
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
packages:
- claudine
- claudine-cli
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

# Separate sequence document schemas from step-state schemas

## Summary and Definition of Done

Implement `2026-11-15-sequence-state-schema`: a sequence document's root
`$schema` describes its document properties and caller inputs; `$state`
passively validates its normalized immediate step states. Markdown and YAML
inline lists share template filling and state validation. Referenced whole
documents prepare their own properties, schemas, and caller layers; inline
containers check their own `$schema` against current values before opening
their run. Declaration handling must preserve the approved, frozen task graph.

Completion means all of the following are demonstrated:

- A frozen reproduction of `complete.yaml`, invoked with a caller-relative
  `spec` setter after the file, resolves `file(eager)` before deriving `plan`
  and passes those prepared values to the nested planning task. Missing inputs
  produce the typed combined report without a prompt in a non-TTY session.
- Direct YAML, direct Markdown, root whole-document references, and nested
  whole-document references give the same document/state schema verdicts.
  Existing data selections and dynamic sources keep their normalization rules.
- `$schema` and `$state` work together without sharing scope. Empty lists
  still load declarations and validate document inputs. Each owner fills and
  validates only its own immediate rows, including a container row's own data.
- Inline checks see preceding runtime writes, obey guards, precede their own
  `initialize`, leave shared values unchanged, and use existing task-failure
  routing. Declaration syntax is checked before execution, even behind false
  guards. Retries, loops, rereads, and approval retain frozen children and state.
- Reserved writes and misplaced declarations fail with typed, located errors;
  the existing state-violation message retains its step index, id, property,
  and validation text, with additional owner provenance for nested errors.
- Public-API robustness cases pass, disk and embedded schemas agree, the
  shipped artifact is fresh, canonical documentation and caller skills agree,
  and `just test`, `just lint`, and `just check-tier-coverage claudine` pass
  sequentially. The implementation is ready for review, without a commit or
  a lifecycle-directory move.

All work stays in the session's worktree. Every phase runs without human
approval or interactive commands. Waves are globally numbered; only tasks in
the same wave may be delegated concurrently. Each wave completes before the
next starts. This plan authorizes parallel implementation of the explicitly
independent tasks below, not concurrent execution of shared Cargo gates.

## Phase 1: Establish declarations and diagnostics

### Necessary Rules

1. **Make the key split directly, with no migration mode.** Rename
   `FormalKeys::schema` to `state_schema`; read `$state` there. Keep root
   `$schema` in document composition. Update the row-schema tests and examples,
   without editing the live internal prompts. Evidence: the specification's
   compatibility decision and `sequence/formal.rs::{FormalKeys,formal_keys}`;
   `resolve.rs::FORMAL_SEQUENCE_KEYS` currently strips the wrong key.

2. **Keep the implementation in Claudine and use Darkmatter's existing schema
   APIs.** Add `composition/sequence/declarations.rs` for declaration extraction,
   placement, locations, and prepared passive contracts, and
   `composition/sequence/document.rs` for owning-document views/preparation.
   Keep template filling in `formal.rs`, graph traversal in `preflight`, and
   execution in `task`. Do not change the schema language or add a new parser,
   dependency, trigger family, or editor-only contract. Evidence:
   `DarkmatterSchemas::{effective_for,effective_for_with_override}` resolves
   inline maps, root unions, references, and imports, and
   `EffectiveSchema::validate` already validates a converted working copy.

3. **Retain authored and composition views separately.** Change
   `FORMAL_SEQUENCE_KEYS` to `["$state", "template"]`; keep the public
   `without_formal_sequence_keys` name and remove its YAML-only gate. Its
   composition view removes these two root declarations and all three
   declarations (`$schema`, `$state`, `template`) from inline sequence task
   subtrees, recursively through sequence children and group legs. It keeps
   root `$schema` and unrelated data intact. Traverse only recognized task
   structure, never arbitrary user objects with coincidental key names.
   The authored view retains every declaration, original text, mapping order,
   and source path. Evidence: `resolve.rs::{without_formal_sequence_keys,
   reload_composition_source}`, `formal.rs::is_direct_formal_document`, and
   the specification's Markdown interpolation example. A sanitized map alone
   cannot rebuild a nested declaration on a reread.

4. **Use the same separation at every preparation boundary.** Initial loading,
   bootstrap, passive probes, per-step composition, nested source parsing,
   retries, and source reloads all obtain composition values from the sanitized
   view and declarations from the authored view. Preserve `MappingOrders` and
   `AuthoredOrder` cursors when projecting views; do not serialize YAML to
   create the projection. Evidence: `prepare.rs::{canonical_compose_options,
   bootstrap_compose_options,prepare_sequence_probe}`, `resolve.rs`, and
   `preflight/mod.rs::{parse_nested_source,prepare_nested_sequence}` currently
   enter through different views. Put the projection at the shared preparation
   entry as well as the sequence CLI, so library callers receive the same rule.

5. **Reserve declarations without broadening unrelated restrictions.** Add
   `SEQUENCE_DECLARATION_KEYS = ["$state", "template"]` in `reserved.rs` and
   include it in `is_reserved_state_key`. Add `$schema`, `$state`, and `template`
   to task-option classification and the sequence executable's allowed
   options; grammar then restricts them to an authored inline array.
   `$schema` remains governed by ordinary document-schema rules outside that
   task position. Use a shared declaration-write predicate for sequence
   setters, `with:`, and runtime writes, including dotted destinations rooted
   at these keys. Scope runtime declaration refusal to sequence execution:
   add `RuntimeState::{new_sequence,from_sequence_snapshot}` and a private
   sequence-execution flag; keep `new`, `Default`, and `from_snapshot` unchanged
   for ordinary document runs. Use the sequence constructors in the CLI root,
   task fallback cells, and group branches, so the flag survives branching.
   Do not newly prohibit unrelated generated-name document writes.
   Evidence: `normalize.rs::parse_object_step` extracts options before
   rejecting state keys; `RuntimeState::set_batch` currently checks only
   `ROOT_OVERLAY_KEYS`; `compose/setters.rs::reject_reserved_key` checks only
   `argv`; `task/mod.rs::evaluate_with` already uses the shared state predicate.

6. **Enforce placement before discarding values.** Root `$state`/`template`
   require a root inline `sequence` array. All three task declarations require
   that task's inline array, including group legs and `kind: task` files.
   Refuse them on body-only, prompt, shell, side-effect, external-task-reference,
   and file-reference sequence tasks. Refuse declaration writes supplied by
   setters, `with:`, runtime `set`, and template defaults. Group objects keep
   their closed grammar; a group's inline sequence leg owns its declarations.
   Evidence: the specification's explicit placement ruling and
   `grammar.rs::{validate_sequence_root,validate_task_grammar,
   group_object_grammar}`. Presence, including null, triggers placement checks.

7. **Extract by key presence and validate declarations on empty lists.**
   Missing `$state` means no state contract; missing `template` means no
   defaults. Explicit null is invalid for both. A template must be a map;
   `{}` is an explicit no-op. A schema accepts an inline map, a nonempty root
   union of schema maps/references, or a valid reference string; `{}` is a valid
   open schema, while `[]`, `""`, null, numeric/boolean scalars, and invalid
   arms fail. Preserve normal root `$schema` parsing rather than treating null
   as absent. Evidence: `formal.rs::formal_keys` already rejects null templates;
   `simplified_schema::parse_schema_declaration_located` and
   `parse_yaml_schema_located` reject null and empty unions. The current
   `validate_state_schema` loop never resolves a schema for an empty plan.

8. **Prepare passive schemas once per owner revision, not once per row.**
   Extract a `PreparedSequenceSchema` wrapping `EffectiveSchema` and its
   declaration location. Build it through `DarkmatterSchemas` in the declaring
   source's context, using a schema-only Markdown view containing the selected
   declaration and the owner's sibling `types` definitions. Use no document
   trigger discovery or baseline on this passive state/check view, matching
   today's `DarkmatterSchemas::new` state-validation path. Validate each
   `StepState::to_value()` with the retained effective schema; discard all
   validation copies. Resolve malformed/unloadable contracts even with zero
   rows. Cache unchanged prepared declarations with the invocation's parsed
   owner revision; a changed schema dependency invalidates its prepared
   contract using `EffectiveSchema::dependencies`. Evidence:
   `formal.rs::validate_state_schema` currently constructs and validates a
   synthetic document for every row; `preflight::NestedSources` already shares
   source revisions, and Darkmatter provides the dependency list.

9. **Separate identity from expression provenance.** Keep
   `invocation_path` as the existing sequence-id seed. Anchor template
   expressions, authored file reads, and schema references to
   `document_path`, with `request_context.for_source(document_path)` and the
   invocation's captured `ComposeContext`. Add the captured context and
   `CallerInputLayers` to `SequenceSourceOptions` through builder methods;
   keep `SequenceSourceOptions::new` as the no-caller-input entry. Update all
   struct-literal callers to the builder. Do not capture CWD or environment
   inside the normalizer. Evidence: `formal.rs::normalize_formal_plan` currently
   gives `apply_template` the invocation path; `SourceExpressionLookup` and
   `source.rs::resolve_sequence_reference` already accept explicit context.

10. **Prepare owner values before filling templates.** Implement
    `prepare_sequence_document_values(&ResolvedCompositionSource,
    PrepareOptions, CompositionMode) -> Result<SequenceDocumentPreparation,
    CompositionError>` in `sequence/document.rs`, using a frontmatter-only,
    shell-passive branch of
    the canonical option assembly in `prepare.rs`. Return prepared values,
    schema outcome, and provenance, not a launchable prompt. Reuse the
    canonical conversion, defaults, interpolation, deferred-value rules, and
    typed error classification. `PrepareOptions::defer_schema_verdict` carries
    the existing bootstrap-versus-verdict stage. Do not use the lossy result of
    `read_selection_hints` as a document preparer. Root and referenced whole
    documents use their own properties, invocation setters with original
    `CallerInputRecords`, and then available runtime data writes. Referrer
    authored properties never enter the referenced owner's template/schema
    instance. Inline owners use the enclosing effective document values.
    Item fields shadow those values whole, as today. Evidence:
    `prepare.rs::{read_selection_hints,bootstrap_compose_options}` already
    demonstrate frontmatter-only canonical assembly; `sequence.rs` currently
    normalizes before using `set_overrides`; `Loader::load_sequence` currently
    normalizes a reference before switching its frontmatter to the target.

11. **Preserve document lifecycle stages.** Root bootstrap can coerce supplied
    values while deferring its verdict; root `initialize` still precedes
    ordinary preflight. Required caller-input collection and launch validation
    occur at preflight, and existing per-step completion validation remains.
    A document-only validation pass also runs for zero steps. Referenced
    sequence documents receive the same passive launch preparation during
    inventory and normal document preparation at their existing entry/reread
    boundaries; their own initialization is not simulated during collection.
    Do not replace this with strict completion validation before initialization.
    Evidence: `prepare/service.rs::{SchemaStage,prepare_bootstrap}`,
    `wrap/sequence/mod.rs::execute_sequence`, and the specification's explicit
    preservation of runtime-supplied required values and launch/completion rules.

12. **Inventory document inputs before final normalization and approval.**
    Extend the recursive library loader with an input-discovery pass over
    authored structure. Retain a `SequenceInputInventory` containing the root
    owner, referenced whole sequence owners, and prompt-reference contexts in
    first-encounter order; include owners with no children. Share external
    task/group/catalog loading, cycle detection, and parsed-source caches with
    graph discovery. This pass performs no lifecycle actions, shell expansions,
    provider selection/launch, or approval. Merge literal template structure
    before inspecting executable references; leave value interpolation pending
    until its owning inputs are available. A reference dependent on a known
    missing document input remains pending under that owner's missing report,
    rather than becoming a spurious not-found/template error. Evidence:
    `phase1c.rs::run_phase_1c_attempt` only loops over root steps, while
    `PreflightGraph::{prompt_documents,prompt_references}` already records
    transitive prompt contexts and `Loader` owns the reference walk.

13. **Keep collection in one pre-approval service.** Extend Phase 1c to consume
    the inventory before plan finalization. Aggregate independent missing
    inputs from all discoverable owners and prompt contexts, retain the
    existing per-step entries, and deduplicate collection by
    `(name, type_label, description)`. Non-interactive execution returns the
    combined typed report immediately after the discovery pass. Interactive
    execution collects unresolved inputs, resolves pending references, and
    repeats discovery only while it discovers a new owner or supplies a new
    input; a no-progress pass returns the combined error. Never prompt for the
    same collection key twice or prompt after graph approval. The loop is
    bounded by newly discovered owners/inputs, replacing the current hardcoded
    two-attempt loop. `$state` requirements and inline check requirements never
    join this inventory. Evidence: `phase1c.rs::{run_phase_1c_with_schema,
    collect_sequence_missing_values}` already owns collection, but has only
    root-step subjects and two attempts. Missing owner inputs must be reported
    before `dirname(spec)` or an item template can obscure them.

14. **Keep missing-input subjects honest.** Replace
    `SequenceMissingPropertiesStep` with `SequenceMissingPropertiesEntry`,
    whose `subject: SequenceMissingSubject` is
    `Step { number, name }` or `Document { property }`; keep `source_path`,
    missing metadata, description, and pointer paths. Preserve existing step
    headings and add `Document <file> (<container path>)` headings, including
    the root/empty owner. Keep `CompositionError::SequenceMissingProperties`
    and its aggregate role. Update `composition/mod.rs` exports,
    `error/render/sequence_loop.rs`, `error/tests.rs`,
    `wrap/sequence/{phase1c,tests}.rs`, and their callers together. Evidence:
    the current type requires a one-based step even for an owner that has none;
    repository-wide symbol search found these consumers confined to Claudine.

15. **Every inline owner uses the formal normalizer exactly once.** Route
    root Markdown and YAML lists, referenced whole-document lists, and
    `Loader::load_sequence` inline lists through `normalize_formal_plan`.
    Add an owner property cursor to this pipeline and use `normalize_plan_at`
    so nested diagnostics and mapping order retain full paths. The order is
    shorthand expansion, this owner's template defaults, generated fields,
    then this owner's passive `$state`. The parent validates a container row's
    own extracted state, never its declarations or children. Evidence:
    `sequence/mod.rs::resolve_sequence_plan_with` has a YAML-only branch,
    `source.rs::plan_referenced_sequence` has another formal entry, and
    `Loader::load_sequence` bypasses formal normalization for inline children.

16. **Carry root reference ownership explicitly.** Add
    `SequencePlan::document_scope: Option<Arc<SequenceDocumentScope>>` for a
    whole-document source, with declaring path, property cursor, and shared
    authored/prepared owner values. Reuse it in root reference inventory and
    template preparation; never infer the owner from the wrapper's source
    path. Keep the invoking root's existing body/lifecycle and root row shape;
    do not insert an artificial wrapper task or alter `seq` positions for
    `sequence: child.yaml`. Nested whole-document containers continue to use
    the target as their event/body base. Data selections carry no document
    scope. Evidence: root `SequenceSource::External` currently records a path
    but `Loader::walk_plan` and CLI execution still receive the invoking
    source; nested `PreflightSequence::frontmatter` already distinguishes
    target documents from selections. The specification requires both
    reference paths to judge the target's document inputs and state without
    changing the existing data-source row contract.

17. **Inline `$schema` is a passive check, not the document's schema.** Store
    it separately as `PreflightSequence::inline_schema` with its source
    location; referenced whole documents use their document-scope metadata
    instead. Parse/resolve it during early grammar/preflight, but validate its
    values only after `when` admits the task. Check current enclosing values
    plus setters and runtime writes before `run_sequence` changes the lexical
    sequence scope, opens `LifecycleRunGuard`, or resolves the container loop.
    Return an unstarted failed `TaskOutcome` on refusal: no container
    initialize, children, terminal events, cleanup, mutation, or output.
    Ancestor failure handling remains active. A false guard skips checking.
    Evidence: `TaskExecution::{run_guarded,run_admitted,unstarted_outcome}`
    provide the admission seam; `run_sequence` currently opens the guard
    immediately; the specification explicitly places this check before it.

18. **Keep dry-run checks passive.** Dry runs resolve declaration syntax,
    collect real document inputs, and validate normalized state, but describe
    inline runtime checks without evaluating their value requirements.
    Earlier runtime writes and guards are not executed in a preview. Reuse
    the existing dry-run note rendering. Evidence: `TaskExecution::guard`
    deliberately does not evaluate a guarded task in a dry run, and the
    sequence dry-run contract does not execute `side_effect: set`. This avoids
    rejecting a valid sequence merely because its earlier writer was previewed.

19. **Refresh declarations and values without refilling the graph.** Preserve
    `RootRun::graph`, frozen `PreflightSequence::{plan,steps,loop_config}`,
    and frozen group tasks. Split nested refresh from initial child
    normalization: `prepare_nested_sequence` reloads owner values, current
    check declarations, and events, audits refreshed command sites against
    closed authorization, and keeps the original normalized children/ids/state.
    Validate refreshed inline checks at initial admission and the existing
    retry/pass preparation boundaries, never between children of one pass.
    Initial admission checks precede initialize; the existing post-initialize
    preparation still reads changes made during initialize. Referenced
    document schemas use current values at those existing boundaries.
    Reread `$state`/`template` declarations receive syntax/placement checking,
    but state validation and template filling remain properties of the
    original frozen plan; a changed declaration never retypes or refills rows.
    Evidence: `prepare_nested_sequence` currently builds a fresh plan then
    overwrites it with `frozen.plan`; `run_sequence_attempts` rereads at Start;
    `verify_nested_graph` and `RootRun::approved` enforce closed approval.

20. **Keep declaration failures distinct from instance failures.** Add typed
    `SequenceDeclarationMisplaced`, `SequenceDeclarationShape`, and
    `SequenceDeclarationLoad` errors with declaring `source_path` and full
    property path; the load error retains its boxed `SchemaError` cause.
    Add `SequenceContainerSchemaViolation` for inline value failures.
    Extend `SequenceStateSchemaViolation` with source/container provenance,
    retaining its index/id/property/message and existing message clause.
    Render locations with the existing source-context helpers and Prose
    components; schema-reference causes must remain downcastable. Evidence:
    `error/mod.rs::SequenceStateSchemaViolation` has only row-local identity,
    while `PreflightTask::diagnostic` and `AuthoredOrder` already retain
    source-rooted task locations. Nested step zero is otherwise ambiguous.

21. **Preserve loader contracts and keep the fix portable.** Whole referenced
    Markdown/YAML/JSON/JSON5 documents activate declarations. Offset/operator
    selections and JSONL/NDJSON stay data, with strict authored lists and
    lenient data lists unchanged. JSON and JSON5 intentionally share the
    JSON5 parser, including comments/trailing commas; JSONL blank lines remain
    ignored. No new direct CLI JSON/JSON5 support is added. Use FileReference
    and captured file contexts for resolution, portable diagnostic paths, and
    cross-platform fixtures; no new platform branches or host shell stubs.
    Evidence: `source.rs::{is_formal_sequence,select_items}`,
    `data.rs::{load_document_from_text,parse_line_delimited}`, and
    `commands/sequence.rs::resolve_sequence_source` establish these boundaries.

22. **Ship the existing schema contracts and regenerate once.** Add root
    `$state: schema` to `schemas/sequence.yaml`; retain root `template: object`.
    Add `$schema: schema`, `$state: schema`, and `template: object` to the
    recursive `task` type in `schemas/sequence-types.yaml`. Use descriptions
    that name the inline-container scope. Placement remains parser-owned
    because task objects intentionally accept arbitrary state. Run root
    `just shipped-schemas` after schema edits; keep its deterministic generated
    changes. Evidence: both files are the runtime/editor authority;
    `lib/tests/l1/{sequence_schema,claudine_schema_parity,shipped_artifacts}.rs`
    already check that authority. No payload-size ceiling is raised.

23. **Use existing test seams and documentation owners.** All added cases are
    L1, using Test Toolkit and `CliProcessFixture`, with detailed matrices
    in-process and only route-wiring subprocesses. Extend the existing
    consolidated modules registered by `lib/tests/l1/main.rs` and
    `cli/tests/l1/main.rs`; no new test binary. Freeze the reproduction under
    `cli/tests/fixtures/frozen_prompts/sequence-state-schema/` and reference it
    with literal `include_str!` paths. Update canonical topic pages and the
    source caller skill, then publish it. `.claude/skills/claudine/topics` is a
    tracked symlink to the canonical topics, so edit those topics once.
    The package README contains no old row-schema example and needs no change.
    Evidence: the test manifests, `sequence_sources_cli.rs`, the README search,
    Git's symlink entry, and the specification's frozen-prompt rule.

24. **No spike, benchmark matrix, approval gate, or new CI scope.** The
    specification explicitly rules out a timing spike and identifies no
    platform-specific cost. Preserve parsed-source/prepared-schema reuse and
    prove it with existing work-counter seams. Run the local area gates once
    after integration; do not run `cargo fmt`, commit, push, or complete/move
    the fix. No required product ruling remains.

### Wave 1

- [x] **Declaration contracts**
  - Implement rules 1, 5–8, and 20 in `sequence/declarations.rs`, `formal.rs`,
    `reserved.rs`, `grammar.rs`, and `composition/error/{mod,render}.rs`.
    Register the module and error exports. Keep `FormalKeys` as the raw
    declaration view; use `PreparedSequenceSchema` for resolved validation.
  - Validate schema syntax and template map/reserved-key shape independently
    of row count. Preserve typed causes and locate nested declaration keys.
  - Update declaration-sensitive comments on touched behavior, especially
    `GENERATED_STATE_KEYS` and the state-violation error. Add focused
    public-resolver tests for placement, absent/null/empty distinctions, and
    an invalid declaration on an empty list.
  - Prerequisite: none. This is the shared contract for all later tasks;
    do not parallelize edits to its symbols.

- [x] **Shipped contracts**
  - In parallel with Declaration contracts, edit only
    `schemas/{sequence,sequence-types}.yaml` and their declaration descriptions
    according to rule 22. Keep the recursive task/group shapes and trigger
    selection unchanged. Add no defaults to declaration fields.
  - Leave artifact regeneration and schema-parity assertions to Wave 2, after
    the declaration contract is integrated.

### Wave 2

- [x] **Contract checkpoint**
  - Run root `just shipped-schemas`; verify only the expected schema/artifact
    files change. Extend the existing disk/embedded declaration controls in
    `lib/tests/l1/{sequence_schema,claudine_schema_parity}.rs`.
  - From `claudine/`, run focused `just test-library` filters for the new
    declaration/placement tests and schema parity/freshness tests sequentially.
    A typed declaration error on an empty list and successful resolution of
    the new shipped meta-types are the exit criteria.

## Phase 2: Prepare owning values and normalize lists

### Wave 3

- [x] **Composition views**
  - Implement rules 3, 4, and 10 in `sequence/document.rs`, `resolve.rs`, and
    the canonical preparation entry points in `prepare.rs`/`prepare/service.rs`.
    Introduce `SequenceDocumentScope` and the passive
    `prepare_sequence_document_values` result. Keep declaration-bearing
    authored snapshots shared through `Arc`, without cloning every subtree
    per step/reference.
  - Expose separate authored-reload and composition-reload operations. Retain
    the existing `reload_composition_source` contract for composition callers;
    nested declaration refresh explicitly uses the authored operation.
  - Reuse canonical options for caller records, type conversion before derived
    expressions, default application, captured context, and passive shell
    handling. Withhold bootstrap verdicts; classify document schema outcomes
    through the existing schema layer. Never read the body for value-only
    preparation.
  - Test Markdown/YAML projection parity, nested/group/task declaration
    protection, preservation of root `$schema` and mapping order, and supplied
    `file(eager)` conversion before a derived property reads it. Own these
    files exclusively during this wave.

- [x] **Input subjects**
  - In parallel, implement rule 14 in the error types/renderer, public exports,
    and existing error/Phase 1c test helpers. Preserve existing step rendering
    and add document rendering without inventing a numeric step zero.
  - Adapt Phase 1c's entry construction to the new subject type without yet
    changing collection ordering. This task owns `error/`,
    `composition/mod.rs`, and `wrap/sequence/{phase1c,tests}.rs` for this wave.
  - Run focused error-rendering and Phase 1c unit tests after both edits have
    landed; avoid simultaneous Cargo invocations.

### Wave 4

- [x] **Owner normalization**
  - Apply rules 9, 10, 15, and 16 to `formal.rs`, `source.rs`,
    `sequence/{mod,model,expr}.rs`, and the library exports. Extend
    `SequenceSourceOptions` with builders for caller layers and captured
    context; update its concrete callers and test struct literals.
  - Remove `is_direct_formal_document`'s format distinction from dispatch.
    All authored inline lists use the formal pipeline; source selections keep
    the existing data pipeline. Carry explicit declaration owner/cursor and
    `document_scope` metadata. Preserve root-reference row count, ids, and
    invocation identity semantics.
  - Compile `$state` before iterating rows, then validate normalized extracted
    state with that prepared schema. Reject reserved template defaults before
    looking at items, including an empty list. Keep validation passive.
  - Use prepared owning values for templates and declaring-file context for
    expressions; retain item shadowing, literal non-string defaults, and
    authored values winning over defaults. Protect nested templates from
    interpolation by a parent's template or document preparation.
  - Checkpoint: library tests prove direct Markdown/YAML and referenced
    Markdown/YAML/JSON/JSON5 parity, declaration reference/import provenance,
    passive state conversion, and selections ignoring the selected document's
    declarations. Phase 2 exits with the library resolver usable independently
    of the CLI.

## Phase 3: Discover inputs and freeze owner-scoped graphs

### Wave 5

- [x] **Input inventory**
  - Implement rules 12 and 13's library responsibilities in
    `sequence/preflight/mod.rs` and a new `preflight/inputs.rs`. Introduce
    `SequenceInputInventory`, sharing `Loader`'s reference/cycle and
    `NestedSources` machinery rather than a second file loader.
  - Inventory the root, root whole-document reference, nested sequence
    documents, group-leg sequences, external task files containing sequences,
    and transitive prompt references. Include empty owning documents and
    distinguish each document from a check-only inline container.
  - Retain authored structure and pending input-dependent template references;
    continue discovering other concrete references and collecting their
    missing inputs. Keep immutable caller-origin records separate from
    prepared values and runtime layers. Malformed declarations remain immediate
    typed failures even behind guards.
  - Add in-process inventory controls proving owning file/property metadata,
    combined independent requirements, pending-reference behavior, no inline or
    `$state` input collection, and zero lifecycle/shell/provider effects.

### Wave 6

- [x] **Scoped graph**
  - Wire owner preparation and formal normalization into
    `Loader::{walk_plan,seed_steps,load_sequence,load_sequence_children}`.
    Root whole-document source metadata must reach graph input validation;
    nested whole documents use their own prepared values before normalizing
    their children. Inline lists normalize with their own options and current
    enclosing effective values.
  - Add `PreflightSequence::inline_schema` and explicit document-scope metadata
    in `preflight/shape.rs`; adapt constructors and the existing fixture
    builders together. Keep check schemas out of document type scopes, provider
    prediction, missing-input collection, and parent state.
  - Split nested refresh from child normalization per rule 19. Refresh owner
    values/checks/events and discover refreshed command sites against frozen
    child nodes. Retain the existing refusal of unapproved commands/prompts,
    schema dependency freshness, and per-invocation parsed-source reuse.
  - Checkpoint: `just test-library` focused preflight/referenced-document
    filters prove immediate-child ownership, group/task-file locations,
    parent-row validation at a container boundary, frozen ids/template state,
    and unchanged-source parse/preparation reuse. No schema is resolved per row.

## Phase 4: Connect collection and execution boundaries

### Wave 7

- [x] **CLI preparation**
  - Reorder `commands/sequence.rs::run_sequence_inner` and
    `wrap/sequence/mod.rs::{execute_sequence,execute_sequence_inner}` so
    argument ownership/setter parsing still reads the authored root schema,
    grammar/declaration syntax checks occur early, root initialization retains
    its existing lifecycle stage, and input inventory/collection precede final
    template normalization and command approval. Allocate the invocation
    sequence token before root initialization and reuse it in the final plan.
  - Move final root plan creation into the preflight preparation boundary;
    retain the authored source alongside its composition view. Reuse the first
    frozen plan/graph on retries. Initializer runtime writes are available to
    initial graph templates; later child writes never refill them.
  - Extend `phase1c.rs` to consume the inventory, prepare owners even for zero
    rows, aggregate document and existing step/prompt failures, and apply the
    progress-bounded collection algorithm. Rebuild pending discovery before
    final approval, using the same caches and updated caller input records.
    Continue to preserve existing invalid-optional treatment and ordinary
    launch/completion categorization.
  - Keep root references as existing root row sources, while judging their
    declaring document separately. Pass prepared caller provenance through
    `jit.rs` and referenced-document contexts. Empty valid sequences still
    run their existing lifecycle; invalid empty owners cannot silently succeed.
  - This task owns CLI sequence/Phase 1c/JIT files and setter validation.
    Add sequence-context declaration-setter checks after all setter forms
    merge, leaving unrelated composition commands' setter behavior intact.

- [x] **Container admission**
  - In parallel, implement rules 17–19 in
    `sequence/task/{mod,sequence,group}.rs`. Both `run_guarded` and the root's
    `run_admitted` route must reach the same inline-check admission code.
    Container proxy entry must use this check as well rather than bypassing
    it via `run_sequence`.
  - Build the check snapshot from current effective enclosing values, setters,
    mutations, and current lexical roots. Validate only the retained inline
    contract; never install it as a document schema or commit its converted
    values. Return typed unstarted failures before the container guard opens.
  - At existing Start/retry/pass refresh boundaries, check refreshed contracts
    against current snapshots while preserving frozen plans and closed
    approval. Keep referenced document launch checking distinct from inline
    admission, including empty referenced containers.
  - Implement declaration-write refusal in `RuntimeState::set_batch` and
    `with:` checks using the shared catalog. Validate an entire runtime batch
    before committing it. Keep existing unrelated runtime restrictions intact.
    Use rule 5's sequence-scoped constructors in fallback cells and group
    branches; add a control proving ordinary document runtime writes retain
    their previous contract. CLI preparation installs the sequence constructor
    in the root runtime.
  - Dry runs emit the inline-check preview without testing runtime-dependent
    values. This task owns library task/runtime files and their unit tests;
    it does not edit the CLI files owned by CLI preparation.

### Wave 8

- [x] **Boundary checkpoint**
  - Integrate both tasks, update `task_run.rs` admission wiring, and run focused
    library admission tests and CLI schema/source tests sequentially.
  - Prove: an earlier `set` satisfies an inline check; a missing value refuses
    before that container's initialize/finalize; an ancestor observes the typed
    failure; false guards skip value checking; malformed declarations fail
    early; converted/defaulted validation copies never change enclosing state.
  - Prove root and referenced empty lists participate in input validation,
    required state/check fields never prompt, and input collection launches
    nothing. Keep schema-aware setters after the filename order independent.

## Phase 5: Complete regression and robustness coverage

### Input Robustness Matrix

Invalid declaration/format shapes produce a typed error naming the owning
file and declaration/element path. Discovery and validation are passive: no
provider, shell, mutation, or publication occurs from the rejected operation.
Malformed inline declarations are caught by grammar before their owner opens;
referenced declarations are checked when the recursive discovery loads them.
The root retains its existing initialize-before-preflight order, so errors in
referenced files discovered there preserve the root's already performed
initialization. Value errors detected at an existing later lifecycle boundary
also preserve effects already performed by enclosing/earlier stages; an inline
refusal opens no run of its own. This is the existing lifecycle contract, not
a rollback mechanism.

Test the following exceptions and valid distinctions once per shared validator:

| Validator / load-bearing input | Outcomes differing from the invalid-shape default |
| --- | --- |
| Declaration placement/presence | An absent declaration supplies no contract/defaults. Present null still has placement and is then a shape error. Inline container declarations are extracted as options, not row data. |
| Schema declaration (`$state`, inline `$schema`; root uses normal document loader) | `{}` is an explicit open schema. A map of valid type declarations, valid reference, or nonempty root union is accepted. Test mixed valid/invalid union arms and all-invalid arms, preserving the failing arm location; empty union/string/null are rejected. Use one full declaration-validator matrix, then a positive and one invalid wiring case for each other declaration home. |
| Template map | Absent means no defaults; `{}` is a valid explicit no-op. Mapping values may be null, empty strings, arrays, or objects and are copied as data; only top-level string defaults are interpolated. Scalar/array/null whole templates fail. Reserved destination keys fail even for zero rows. |
| Sequence source/list | `[]` is an explicit valid empty sequence after declaration/document validation. Missing `sequence` retains the public resolver's `Ok(None)` contract and the CLI's missing-sequence error. Null and wrong whole-field types fail. Strict authored lists reject null/numeric/boolean/array items; object items need their existing valid name/executable shape. Lenient selected-data lists retain scalar coercion/ordinal naming, but never drop invalid entries. Cover one invalid item among valid items and an entirely invalid list. |
| State field validated by `$state` | Use one `color: string(required)` / `rank: number(required)` / collection-field control. Cover absent, explicit null, whole-field wrong type, mixed invalid elements, entirely invalid elements, and allowed empty string/array/map shapes against their declared types. Normal passive conversion remains accepted where the declared type permits it; it never rewrites the returned plan. Apply the full field-shape matrix to one shared state validator, then a control and one invalid field per owner route. |
| Document input values | Preserve canonical required/default/launch rules: absent values with declared defaults are filled; supplied values undergo ordinary coercion; optional-invalid scrubbing is unchanged. Test absent and null separately through the public result, including direct-mode required-null classification. Caller file values resolve from their recorded origin, authored values from the declaring file. |
| Inline check values | Missing/null/wrong values fail only at admitted entry. Defaults/conversions may make the validation copy valid but never publish to runtime state. A false guard skips value checks; declaration syntax remains checked. Dry-run previews the runtime check. |
| Declaration write routes | Authored inline options are accepted; setter, `--set`, prompt `with`, runtime batch `set`, and template-generated declaration keys are refused. A refused batch publishes no partial writes. |

Document shapes belong to the actual shared loaders, not each declaration:

- **YAML loader:** `load_yaml_document` and `data::load_document_from_text`
  both use `biscuit_file::Yaml`/serde_yaml_ng. Cover duplicate keys at root
  and nested declaration depth, invalid trailing syntax, and a second YAML
  document as errors. Cover YAML `key:` as explicit null, not absence. Direct
  YAML and referenced YAML each need a route control and one invalid wiring
  case; do not multiply the full parser matrix by both routes.
- **JSON-family loader:** `.json` and `.json5` share
  `biscuit_file::Json5` plus `DuplicateScan`. Keep one shared duplicate-key,
  trailing-garbage, and nested-duplicate matrix. Comments and trailing commas
  are valid under both extensions because that is the current loader contract.
- **Line-delimited loader:** declarations remain inactive data. Reuse the
  existing duplicate-key/line-location cases; add blank-line acceptance and
  trailing-invalid-record controls only where missing. Do not duplicate this
  loader's cases for `.ndjson`.
- **Markdown frontmatter:** test declaration protection and the YAML null/
  duplicate wiring at the public composition/resolver boundary. Markdown body
  grammar is outside this matrix. Do not multiply body parser tests.

Build each negative case from a passing frozen/real-tool-shaped control with
one edit. Assert typed public resolver, preparation, graph, or `TaskOutcome`
results, including location and untouched state. CLI subprocesses prove route
wiring only: a control and one or two invalid cases for each CLI-supported
loader (Markdown-frontmatter/YAML), with located rendered diagnostics and
zero launches. Referenced JSON/JSON5/JSONL use public library results.

The planning scan found no serde defaults or silent filtering in
`formal.rs`/`source.rs` declaration parsing; it did find
`unwrap_or_default()`/`.ok()` in preflight caches, setter extraction, and
optional group lookups. Audit uses in the new declaration/value paths rather
than rewriting unrelated cache policy. Before completing this phase, rerun
`rg` for `serde(default)`, `Option<`, `filter_map`, `unwrap_or_default()`, and
`.ok()` in changed readers. Presence-sensitive declarations must use map
presence checks, not an `Option<T>` deserializer that collapses null.
Every matrix finding is resolved with the status of all shapes for its field
validator and all cases for its loader, not a single isolated cell.

### Wave 9

- [x] **Library coverage**
  - Extend `sequence/tests.rs`, `sequence/preflight/tests.rs`,
    `sequence/task/tests.rs`, and its existing `tests/referenced_documents.rs`
    module with the detailed matrices above. Reuse fixture builders; do not
    create another standalone integration binary.
  - Rename old row-schema cases/comments to `$state`. Cover map/reference/
    imported named-type/root-union declarations; put the caller, referrer,
    declaring document, schema, and eager file in distinct directories.
    Assert underlying schema-load causes as well as owner paths.
  - Cover Markdown/YAML owner equivalence, immediate container-row state,
    noninheritance into inline/referenced children, item shadowing, document
    setter precedence, no referrer-only property leakage, nested/group/task-file
    declarations, and reserved-write atomicity.
  - Extend work-counter controls for unchanged declarations and referenced
    parsed sources: many rows resolve one owner contract, and rereads never
    refill templates/regenerate state. Keep existing payload-size ceilings.
  - Own library behavior-test modules only; CLI coverage owns subprocess files.

- [x] **CLI regressions**
  - In parallel, freeze a minimal valid version of the motivating YAML plus
    its tiny prompt/schema/spec files in
    `cli/tests/fixtures/frozen_prompts/sequence-state-schema/`. Include it by
    literal path from `sequence_sources_cli.rs` and copy it into the isolated
    fixture. No test reads `prompts/` or fixes unrelated live prompt grammar.
  - Update the three existing formal-schema tests in `sequence_sources_cli.rs`
    to `$state`, including their names/comments. Assert successful concrete
    values reaching a real composed nested prompt through `--dry-run`.
  - Extend `sequence_schema.rs` and `sequence_referenced_state.rs` for root and
    nested reference input collection, empty lists, setter ownership and
    provenance, combined document/property reports, and no lifecycle actions
    during the collector itself. Use portable fixture launch-count seams,
    never new Unix-only shell stubs.
  - Extend `sequence_frozen_graph.rs` and existing sequence-loop/retry coverage
    with declaration edits, changed document inputs, frozen ids/template state,
    failed admission, and no approval reopening. Extend dry-run coverage for
    a runtime-written inline requirement. Assert the intended schema outcome,
    not an arbitrary later provider/empty-body failure.
  - Keep subprocess cases to route wiring; leave format/shape combinations to
    Library coverage. All tests remain silent, hermetic L1 and steal no focus.

### Wave 10

- [x] **Regression checkpoint**
  - Run focused library, CLI source/schema, referenced-state, frozen-graph,
    and loop/admission filters sequentially. Run the schema parity/freshness
    filters after both tasks are integrated.
  - Complete the robustness smell audit and matrix accounting. Resolve every
    failure in scope, including a previously uncovered sibling shape exposed
    by a finding. Record the cases covered and test results in the
    implementation log. Do not expand into unrelated prompt or CI work.

## Phase 6: Publish documentation and verify completion

### Wave 11

- [x] **Current documentation**
  - Update `docs/topics/flow-control/sequences.md`: the formal example uses
    `$state`; root `$schema` describes the document in both formats; show
    both keys together, immediate ownership, inline declarations, reserved
    writes, declared-file versus caller-file context, input collection,
    template freezing, runtime admission, empty-list validation, and dry-run
    checks. Explain the existing `$schema` row-reading rename explicitly.
  - Add a compact Mermaid flow from authored extraction through owner
    preparation/collection, template normalization, approval, guard/check,
    initialization, and frozen execution. Distinguish syntax preflight from
    value checks and state plainly which effects are already owed at failure.
  - Update `docs/topics/schemas.md`, `schemas/README.md`, and the CLI sequence
    reference for the same public rules. Update `.claude/skills/claudine/SKILL.md`
    at its container/preflight/schema sections. Edit canonical topic files
    once; the repo skill's topic symlink follows them.
  - Add caller-facing sequence schema guidance in
    `docs/use-skills/use-claudine/sequences.md`, preferring a bounded
    transclusion of the canonical topic's fenced examples. Publish from
    `claudine/` with `just publish-skills` and inspect the composed output for
    literal interpolation examples and the new key split. Never edit the
    installed caller skill directly. Leave the package README overview intact.
  - Current docs must not link to or name this dated fix. Record any departure
    from the snapshot specification only in `implementation.md` beside this
    plan; do not rewrite the specification or move the fix directory.

### Wave 12

- [x] **Final verification**
  - Run from `claudine/`, sequentially: `just test`, `just lint`, then
    `just check-tier-coverage claudine`. The test recipe supplies Nextest,
    work counters, and the CLI test-fixture features. Do not run `cargo test`,
    `cargo fmt`, L2/L3, real-provider, prompt-test, or benchmark recipes.
  - Validate the frozen motivating case with the actual `CliProcessFixture`
    dry-run assertion, confirm zero provider launches, and confirm output
    contains the prepared `spec`/derived `plan` values reaching its nested
    step. The new tests provide this evidence; do not run the live prompt.
  - Check generated artifact freshness, disk/embedded parity, published skill
    content, plan task completion, declaration docs/comments, and the final
    diff. Use the normal existing cross-platform CI scope; add no matrix cell
    or pipeline change. Record local evidence as macOS evidence, without
    claiming another OS was executed.
  - Write `implementation.md` with changes, tests, known practical limits,
    and departures, including the necessary inventory-before-template ordering
    and root empty-list validation. End at implementation complete, ready for
    review; no commit, push, human gate, or completion-directory move.