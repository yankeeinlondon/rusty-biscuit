---
kind: plan
area: claudine
total-phases: 7
# Hold: Phase 4 must not start; see "Hold before Phase 4" below.
stop-before-phase: 4
created: 2026-11-23
status: planned
phase: 1
agent: codex/gpt-6.2-sol
yolo: true
---

# Ergonomic sequences implementation plan

> **⛔ HARD STOP: Phases 1–3 only.** Do not start Phase 4 or any later
> phase. See [Hold before Phase 4](#hold-before-phase-4).

## Summary and Definition of Done

Claudine authors can state the operation with `compose:`, `inline-compose:`, or
`sequence:`, or delegate it to `prompt:`. Each call carries overrides and
fallbacks with predictable scope. The same document classification, provider
resolution, and source provenance drive passive previews and real runs.
A child sequence runs the same written inline or referenced by file. Schema annotations
control which keys cross implicit inheritance edges in Claudine and Darkmatter.

- The task and document matrices in [the specification](./spec.md) pass through
  the public library and CLI, including dynamic targets, formal and informal
  sequences, nulls, group legs, immutable task references, and nested trails.
- Recursive preflight finds uncovered tasks before execution. Dry-run exit codes
  are 0 for clean or informational results, 1 for warnings, and 2 for errors.
- All ten provider defaults resolve from bounded, passive config reads.
  Preflight and execution report the same model and its source. The actual
  launch determines whether Tier 6 is forwarded.
- All four generated schema artifacts and provider metadata pass freshness
  checks. The canonical docs, caller skills, and 21 research artifacts reflect
  the new behavior.
- Complete the verification in Phase 7 and leave the work ready for review with
  commands, results, environment evidence, and any spec clarifications recorded.

## Phase 1 — Grammar and schema metadata

### Necessary Rules

1. **Start from the diagnostics prerequisite.** Schedule these phases after the
   code for `2026-11-22-better-errors-and-dry-run` is integrated. Its plan is
   marked planned and this checkout's `ExecutableField` has no `noun` method
   yet. Build on its shared findings sink, loaded-source diagnostics, shared
   task schema, and `removed(...)` work. Preserve the prerequisite snapshot.
   Decision 7 supersedes its severity-to-status mapping, and Decision 1
   retains `prompt:`. Move each affected test expectation with the behavior.
2. **Place isolation on property metadata.** Use `Constraint::Isolated` and
   public `DARKMATTER_ISOLATED_KEYWORD` in Simplified Schema. Model parser,
   export, and resolution wiring on `Generated`. Preserve static `required`,
   reject root `$constraints`, and keep it out of matcher assertions and type
   identity. Matchers in `registration/sequence-document.yaml` therefore keep
   their current annotation-free forms. See Decision 3 and
   [the isolation audit](./spike-isolated-schema.md).
3. **Derive isolation from the existing prepared payloads.** Add the key set to
   runtime `TriggerLayer` and `EffectiveSchema` only. In Darkmatter
   `markdown/schemas/mod.rs`, extract from original JSON or pending root
   before `without_properties`, `checked_layer`, and layer replacement.
   Reuse the registered host's matched payloads before the checked projection.
   A matched pruned or deferred template still contributes authored isolated
   keys without evaluating slots. Keep existing shipped record shapes and
   merge algorithms. Increment Simplified Schema's preparation revision once
   over the integrated prerequisite (4 to 5 on the audited baseline). The
   Darkmatter and Claudine runtime additions and YAML edits use the existing
   owner revisions. Dependency and source digests invalidate their artifacts,
   as described in `simplified-schema/docs/topics/shipped-artifacts.md`.
4. **Use shared annotations for the Markdown trigger overlap.** Isolate common
   controls at their definitions in `shared.yaml`, `document.yaml`, and
   `linking.yaml`. Annotate composition-specific and inline-specific controls
   in `composition.yaml` and `inline-composition.yaml`. Annotate
   `replace`, `tags`, `draft`, `kind`, `sequence`, `template`, `$state`,
   and removed `fail_fast` in the sequence root schemas. Declare the
   isolated `default-values` mapping once in `common-attributes`, so all
   Markdown classes and formal data roots admit the Decision 6 fallback.
   Extend
   `common-attributes@./shared.yaml` in `formal-sequence-file.yaml` and inherit
   the shared `description` declaration there. The Markdown host co-matches
   composition and sequence triggers, and
   `EffectiveSchema::for_yaml_document` selects the formal file schema alone.
   Keep `agent` and `model` unmarked across both roots.
5. **Centralize task options by executable and authored form.** In
   `sequence/{model,reserved,grammar,normalize}.rs` and
   `preflight/mod.rs::parse_task_object`, use the Decision 1.3 allowlist.
   Store the authored executable key on diagnostic provenance. Keep unknown
   domain keys as step data on open steps, and enforce closed standalone-task
   and group-leg shapes through `registration/sequence.yaml`. Preserve
   `kind: task` as the file discriminator. Run the legality check before
   value-shape checks for forbidden options, and give retired keys the
   prerequisite's removed-key diagnostic. Validate allowed `with:` and
   `default-values:` as mappings before cloning values. Reserve
   `default-values` and its dotted and indexed destinations through
   `reserved::declaration_written_by`, including writes from templates,
   setters, step state, and `RuntimeState`.
6. **Classify authored Markdown by non-null key presence.** Add public
   `composition::DocumentClass` and `classify_document(path: &Path, authored:
   Option<&darkmatter::markdown::Frontmatter>) -> Option<DocumentClass>` in
   `composition/document_class.rs`. Variants are `Compose`, `InlineCompose`,
   `ComposeSequence`, `InlineSequence`, and `DataSequence`. The outer
   `None` denotes unsupported format. Use the existing case-insensitive
   `.md`/`.markdown` predicate and supported data extensions, including
   extensionless YAML for sequence references as in `data::SourceFormat`.
   Pass the loaded Markdown's frontmatter by reference. Darkmatter uses a
   `FrontmatterMap` backed by `IndexMap`, so this avoids a JSON map copy.
   Callers classify Markdown after successful load and pass no frontmatter
   for the data-format check.
   Classify first and validate shape separately. A non-null malformed marker
   still selects its class and reports the shape error on that marker.
   A `sections`-only document remains Compose. Keep the verbatim
   `capture_frontmatter_yaml` helper and existing specialized top-level
   inline-sequence hint in `commands/compose/mod.rs`. The task dispatcher
   uses Decision 4.2 and reports `composition.operation_mismatch`.
7. **Keep reference syntax and file classification distinct.** In
   `grammar.rs`, reuse the existing span scanner for plain target validation
   and for the full sequence reference grammar. For plain targets reject
   top-level suffixes on authored and resolved text, and reject arrays,
   mappings, empty strings and whole shell expansions with a located target
   error. Keep quoted and expression-internal delimiters literal. A nested
   `sequence: "{{ next_seq }}"` uses reference interpolation and must resolve
   to a file reference. Root whole expressions and shell sources keep their
   existing list-producing semantics in `sequence/mod.rs` and CLI
   `phase1c.rs`. Resolve with `FileReference` from the authoring document.
   `preflight::Loader::load_prompt` currently rejects any `sequence` key and
   assumes a Markdown file. Replace this classification at the load
   boundary and share it with `load_sequence_with` and runtime refresh.
8. **Carry formal ownership explicitly.** In `sequence/source.rs`, use
   the shared `is_formal_sequence` predicate for whole references to a
   non-null `sequence` declaration in Markdown or data documents. No offset
   or operator means a whole reference. Keep formal data roots strict and
   foreign lists lenient. Keep Markdown's own valid source forms by routing
   a whole Markdown sequence through its document planner, including root
   references; the current `select_items` formal branch expects a list
   and needs this routing for another Markdown document's source declaration.
   Apply the class matrix to whole-document calls. An offset or operator on
   `sequence:` addresses step data and takes precedence over whole-document
   dispatch, including an offset into ordinary Markdown frontmatter.
   Selections keep the invoking Markdown owner's body mode. For a formal child Markdown sequence, its own authored
   class controls the body mode. For a bodyless data owner, zero executables
   are an error even when the call originates in Markdown. Use the existing
   `SequencePlan::document_scope` and `SequenceSource` to distinguish these
   cases. Keep referenced shape, lifecycle and cycle checks on both first
   read and refresh rather than inferring ownership from extension alone. Formal
   ownership decides parsing strictness, body mode and class checks only; it
   never changes value passing (Rule 10).
9. **Route top-level data by file format.** Extend
   `commands/sequence.rs::resolve_sequence_source` beyond its current YAML
   fallback. Resolve once with detailed outcomes; dispatch Markdown to its
   existing source loader, formal data mappings to the bodyless document path,
   and bare data arrays to the lenient root plan, preserving the actual
   origin and mapping order. Reuse `sequence/data.rs` for JSON/JSON5, YAML,
   and JSONL/NDJSON, including the YAML aliases and case handling. Do not
   synthesize source text for diagnostics. Reject a data root that supplies
   neither a usable formal declaration nor a bare list. Decision 5 applies
   fallback flags to all supported data formats and uses format before
   inspecting the authored class.
10. **Use one origin-preserving fold.** Add `composition/sequence/caller_values.rs`
    for sequence-specific layering and project the result through the
    existing `CallerInputLayers` / `PrepareOptions` boundary. Retain separate
    authored owner, CLI, live runtime, and current call-site layers through
    the fold. Fold every call in Decision 2.2 order whatever the callee: a
    prompt document, a group leg, or a child sequence written inline or
    referenced by file (Decision 2.4). A child sequence's folded result is
    the live state it passes to its own tasks, preserving CLI and mutation
    origins; its call-site `with:` is part of that result and never mutates
    the enclosing sequence. Do not branch value passing on
    `is_formal_sequence`, inline versus referenced, or file format; today's
    referenced-document path, which reads only the child's own values under
    setters and runtime writes, is replaced by this fold. Pass default
    layers nearest first. Merge at the
    top-level key boundary, matching the shared runtime and caller-input
    maps. An explicit `with:` null replaces the effective pinned value before
    fallback, so no lower defined value is consulted. Authored null and null
    fallback entries mean absence. A standalone prompt can supply its own
    `default-values` as a document-local fallback, below sequence flow
    defaults and above schema defaults. This preserves Decision 2.2's caller
    ranks and satisfies Decision 6's authored fallback case. A child
    sequence's own defaults sit above incoming call and ancestor defaults
    for its tasks per Decision 2.4.
11. **Filter implicit values against both owners.** Use caller ∪ callee ∪
    baseline isolation before projecting implicit defined and inherited fallback
    values. Keep the defaults declaration itself in the dedicated scoped
    channel. Explicit `with:` passes isolated keys but refuses the reserved
    namespaces in Decision 2.3. Keep `seq`, `group`, and `loop` lexical and
    keep `template` on step state. Apply the same rule to initial preflight,
    `phase1c.rs` root rechecks, `sequence/jit.rs` and
    `composition/sequence/task/{sequence,group}.rs` runtime branches. Evaluate the
    callee's isolation against its authored source and matched schema before
    applying caller values, so inputs cannot preempt callee control keys.
12. **Reuse the caller-file pipeline for winning data defaults.** In Darkmatter
    add `ComposeOptions::with_data_defaults(CallerInputRecords)`. Reuse
    `CallerInputRecord` to hold the value and authoring
    `FileResolutionContext`. Install only winning keys after the effective
    defined merge and before schema fallback/validation. Mark inserted
    leaves as data, skip re-evaluation of expression-shaped contents, and use
    `schema_validation.rs::resolve_caller_file_overrides` for schema-selected
    file occurrences. Remove displaced defined-input provenance for those
    keys and install the winner's context only for inserted occurrences.
    Preserve child-authored paths, including equal text written in two
    different directories. Include the defaults input and origin identity in
    the appropriate options fingerprints in `context/options.rs`. Isolation
    filtering alone needs no new hash input because
    `inherited_child_options` already emits the filtered `external_state`.
13. **Keep a deferred target in the frozen graph.** Represent file calls as a
    resolved action or a deferred reference with authored key, options, origin,
    and trail. Use passive state for static interpolations in
    `preflight/mod.rs::Loader`; use the task's current lexical state in JIT
    for deferred ones. Validate the operation and apply the same dispatch as
    soon as the target resolves. Reuse the existing refresh, cycle, and
    late-shell admission paths. Emit one `dm.check.not_checked` info for a
    deferred check in dry run. No inner shell approval is implied by an
   unresolved reference. A real run uses existing approval policy and
    `--yolo` behavior, including refusal of newly discovered unapproved
    non-interactive commands. Preserve approval-time freezing of static
    targets and shell bytes.
14. **Use the preflight graph for agent availability.** Extend
    `preflight/providers.rs` and Loader's traversal and
    `preflight/containers.rs` with per-branch ordered agent-write evidence,
    keeping existing provider hint dependency prediction. Traverse all
    potential tasks, including guarded tasks. Credit only enclosing/target
    `initialize` and prior sequential `side_effect.set.agent` as specified.
    An explicit null write alone provides no agent evidence. A known null on
    a preceding write invalidates prior conditional evidence on that branch.
    No later, sibling, or child-container write receives static credit.
    Warn once per site relying solely on a set. An unresolved dynamic target
    without a usable upstream agent reports `agent_unverifiable_target`;
    conditional-only upstream coverage reports `agent_conditional_set`.
    Errors for statically invalid provider hints retain their current typed
    diagnostics. Use `Trail` at both preflight and runtime failures. Keep
    `review::review_selections` and seed the picker from `preferred_agent`
    only. Real non-TTY and `--silent` runs fail on known unresolved tasks before
   task execution. Dry run never enters the picker.
    Run the passive structural and known-agent check before the root
    `Initialize` in `wrap/sequence/mod.rs`. Its current graph is built later by
    `execute_sequence_inner` because root templates can read initialize writes.
    Reuse passive discovery and retained sources for the early walk, leaving
    initialize-dependent rows and references deferred. Keep the post-initialize
    plan and recheck before the first task to admit the newly reachable graph.
15. **Normalize flags once at the shared CLI boundary.** After
    `SharedComposeArgs` and positional setters are parsed, inspect both
    setter forms for `agent` and `model` conflicts before applying flags,
    including presence with null. The first `--` ends Claudine argument
    ownership and help keeps its existing precedence. Markdown operations
    inject agent/model flags as CLI-origin defined state. Data sequence flags
    replace only the matching root fallback keys. Ordinary setters stay
    defined everywhere. Delete `explicit_provider` from composition
    `types.rs`, `coordinator/invocation.rs`, and selection arguments in
    `select.rs`; migrate callers in `compose/{ownership,prep}.rs`,
    `wrap/composition/{target,runner}.rs`, and
    `wrap/sequence/{mod,providers,task_run,iterate}.rs`. Use a resolved
    provider for launch and wrap identity. The bare wrapper's fixed provider
    in `wrapper_stages.rs` and harness loop `target_launch.rs` is an execution
    identity, so preserve it as a typed launch input outside the removed
    composition override path. Evidence is the direct-wrapper constructor
    and the loop's separate intent selector.
16. **Resolve the model once per site and preserve its tier.** In
    `composition/select.rs` use one model selection result with model and
    source for CLI/with, declared provider env, generic `MODEL`, defined or
    authored document value, fallback value, and configured default. Store
    CLI model provenance outside the flattened `fm` projection so it stays
    Tier 1 above runtime set for as long as it reaches a task. A non-null
    task `with.model` wins Tier 1. A null `with.model` clears the pinned
    model and its CLI origin at that call, then normal selection starts at
    the env tiers. Authored/runtime models are Tier 4. A schema-supplied
    model belongs at the bottom of Tier 5, consistent with the general
    six-rank value fold. Preserve existing model-list behavior from
    `resolve_model_with_env`. Choose the first catalog match, then the first
    authored entry. An empty list supplies no model. Use explicit context
    model identity for `ctx.model`, carried through JIT and reprepare. Darkmatter
    `context/runtime.rs::values_with_env_agent_overrides` currently derives
    that property from `MODEL`; give explicit context identity priority
    without modifying the captured env or sharing it with child argv.
17. **Implement the audited config matrix.** Add
    `provider/default_model.rs` with the public
    `get_default_model(Provider, &DefaultModelContext<'_>) -> DefaultModelLookup`
    and context, source, unreadable, outcome and lookup types from
    [the config audit](./spike-default-model.md#41-proposed-public-api-in-claudinelib).
    Consume the exact paths, file/key priority, formats, and platform table
    in audit §§1.2–2.3, with Decision 5 taking precedence. Single-directory
    project reads use repo root then cwd as its fallback. Codex project
    reads are passive and ignore trust settings for prediction; document the
    provider's trust precondition. For ancestor-walking providers, start
    at cwd and stop at the repo root inclusive or at 32 directories,
    whichever comes first. An unknown or unrelated repo root limits the
    project search to cwd.
    Invalid path env values produce an informational unreadable entry for
    the env source and do not become candidate paths. A valid absolute
    exclusive redirect terminates its provider's search. For invalid
    `KIMI_CODE_HOME`, skip legacy config because the variable is set and
    probe the current-generation home config. Use the Windows ASCII
    case-insensitive lookup within this explicit config context. Resolve
    collisions with exact spelling first, then lexical key order. Reuse
    `HomeBaseline` for the home and do not discover one in the reader.
18. **Bound config reads before decoding.** Stat candidates before opening
    and skip non-regular files with a located info finding. Use
    `OpenOptionsExt::custom_flags(libc::O_NONBLOCK)` on Unix and check
    handle metadata after opening, then read at most 1 MiB + 1 for the size
    verdict. Claudine already depends on Unix `libc`. This corrects the
    audit's illustrative open-before-stat sequence, which could block at
    `open` on a FIFO. Use component-wise paths, lexical ancestor matching,
    no canonicalization, and no directory enumeration. Move the existing
    `sequence/data.rs::DuplicateScan` to a crate-private `json_duplicates.rs`
    module and use it before value materialization in both readers. Keep
    strict JSON for JSON and inline env candidates and use `Json5` for JSONC.
    Accept the existing local symlink-to-regular-file behavior. Charge a
    wrong-typed model field to the candidate with a typed finding;
    fall through after any malformed candidate. The first valid model ends
    lookup, except Pi's independent two-field merge.
19. **Cache config results and diagnostics for one invocation.** Copy the
    audit §4.2 key scope and env variable lists, include
    `include_project_config`, and normalize cwd/repo to none for Goose,
    Kimi, and Antigravity. Use the same normalized env lookup for candidate
    building and cache keys. Keep full env values out of diagnostics and
    use a shared oversize cache-key marker for values above 1 MiB since they
    all produce the same per-source diagnostic. Cap env input before copying
    it into the cache key. Collapse concurrent same-key misses under the
    invocation cache lock, with one cold lookup and one stored result.
    Share lookup and reported-source sets across group legs through
    `InvocationContext`; key reports by provider plus exact file path/env
    name. Include the same Pi project-config switch in prediction and
    execution using structured mode's `--no-approve` rule in
    `provider/pi/data.rs`.
20. **Forward Tier 6 at final overlay state.** Use the spec's
    `OverlayPlan::hides_native_config` predicate. Evaluate forwarding in
    `exec_prep::resolve_model_and_validate` after overlays, including late
    `ensure_provider_overlay` in `wrap/composition/pipeline.rs`. Update the
    other consumers in `wrap/mod.rs` and `wrap/launch_plan.rs` in the same
    change and retire `WrapperProfile::configured_default_model` and the
    OpenCode-only parser in `profile/opencode.rs`. Bare wrappers preserve
    their existing explicit argv and env selection behavior while using the
    shared passive config reader in place of their old fallback. Composition
    forwards Tiers 1–5, and Tier 6 only for the required non-TTY provider or
    a native-config-hiding overlay. Export child `MODEL` exactly when the
    model is forwarded. For Goose composition launches use its current
    cataloged `goose run --model` flag, documented in
    `docs/research/agent-cli/goose.md`; the old
    `profile/goose.rs::apply_model` env-only behavior stays with the bare
    wrapper. Keep the opaque provider tail intact. A native model flag already
    in that tail owns its value and suppresses the duplicate generated flag.
    Report the Claudine selection and identify the explicit native override
    through the redacted forwarding notice. Only export a generic `MODEL`
    matching the forwarded value. Preserve resolved model and provenance in
    `ctx.model` and logs even when argv omits it. Missing Tier 6 produces
    the display label `default`, or `composition.model_required` for
    non-TTY OpenCode. Keep the placeholder out of argv, child env, and later
    resolution. The existing exec stage converts every provided model to
    `CliSwitch`, so pass the typed result through that stage.
21. **Use shared findings for all severity and exit decisions.** Register the
    six codes in Decision 7 and update its reused diagnostic hints and
    attribution. Reuse the prerequisite's missing-document builder with
    nearby suggestions for confirmed no-match; preserve typed refusals,
    ambiguity and parse errors. Preserve locked lifecycle codes. Map the
    maximum emitted dry-run severity in one CLI boundary, covering human
    and JSON output and all three commands. With no errors, info-only is
    successful and warning-only returns 1. Usage errors and findings with
    any error return 2. Real-run exit and flow semantics stay with the
    existing execution result.
22. **Use committed research fixtures and actual runnable source.** Migrate
    only the named coordinators in Phase 6 and the coordinator example in
    `_TEMPLATE.md`. The template labels itself a prose standard and already
    has a repository artifact test. Validate the first fenced coordinator
    as a temporary Markdown document with the real model-config research
    prompt substituted for `<topic>`. Retain the row-prompt example's
    authored fields. Inject `research_prompt` from each live fleet's root
    initialize setter on dry runs, because the shared roster's `template.prompt`
    reads it and passive preview fires no lifecycle. Leave the three excluded
    fleet files and their research prompts intact.
23. **Use the completed investigations.** Schedule no new spike. The isolation
    audit names the consumers and the config audit supplies candidate order,
    bounds and latency estimates. Phase 3 includes a quick, local measurement
    of the implementation against the target. It uses temporary fixtures for
    all providers and the documented deep Kilo case. Real provider installs,
    system config edits, network probes, and a multi-OS performance campaign
    are outside that measurement. A median at or over 10 ms on a representative
    cold lookup requires correction of extra probes, parsing, or cache work
    before the checkpoint passes.

### Wave 1

- [ ] **Lower Isolation**
  - Add the constraint through Simplified Schema's `grammar.rs`,
    `resolve.rs`, `resolve/recursive.rs`, `convert.rs`, `validate.rs`,
    `narrowing.rs`, `serialize.rs`, and `schema/export.rs`. Include
    semantic `schema`, arrays of builtin/import/union types, inline/block
    objects, named references, and YAML-list property unions. Hoist through
    optional `anyOf` and strip arm metadata. Test a single annotated arm
    against every arm annotated, static required parity, rejection at the
    root and in trigger matches, and round-trip export.
  - Add the public keyword constant, bump the Simplified Schema preparation
    revision, and run its targeted L1 tests using source schemas. Phase 2 owns
    the coordinated generated artifact write.
- [ ] **Define Task Grammar**
  - Extend `ExecutableField::{key,from_key,noun}` and the canonical reserved
    catalog. Make the form-aware option validator shared by strict and
    lenient normalization, group legs, and standalone task parsing.
    Implement Rules 5–7's pure classifier and reference validators, including
    non-null malformed markers and the inactive `sections` marker. Thread
    both new fields through the existing Markdown prompt arms in
    `preflight/{inputs,mod}.rs`, checking the explicit Markdown class in the
    loader. Phase 4 completes generic sequence and deferred dispatch.
  - Update the common task properties in `registration/sequence.yaml`, keeping
    the prerequisite's open/closed distinction. Register typed operation,
    option and fallback error codes and preserve located source paths and
    properties. Add public grammar controls and negatives from the matrix in
    Phase 7.

### Phase 1 checkpoint

- [ ] **Verify Parser Contracts**
  - Run focused L1 tests in Simplified Schema and Claudine using the respective
    package recipes. Confirm that the shared executable list, no-executable
    diagnostic, and authored option partition agree. Check reference examples
    containing literal and top-level suffix delimiters.

## Phase 2 — Effective schemas and Darkmatter composition

### Wave 2

- [ ] **Resolve Isolated Keys**
  - Add `EffectiveSchema::isolated_keys()` and
    `darkmatter_baseline_isolated_keys()` over the shipped baseline using
    `OnceLock<Arc<BTreeSet<String>>>`. Add original-payload extraction in
    `resolve_trigger_layers` and `registered_trigger_layers`. Cover
    `for_yaml_document` and effective-schema rebuilds so sticky metadata
    survives layer pruning and projection.
  - Use `PreparedSchemas::isolated_keys_for` to union the fallback baseline.
    Update `call_binding/translate.rs` for inline objects and literals, and
    the four closed parity tests in `markdown/schemas/about.rs`.
    Add the 16 baseline annotations from Decision 3.3.
- [ ] **Annotate Host Schemas**
  - Apply Rule 4 to all seven Claudine registration files. Declare
    `default-values` through the shared attributes inherited by each root
    so co-matching schemas agree. Mark `sections`
    isolated while preserving classifier behavior. Add the L1 isolation
    parity checks in `claudine/lib/tests/l1/sequence_schema.rs`, testing
    both trigger overlap and the YAML root.
- [ ] **Prepare Default Origins**
  - Apply Rule 12 in the Darkmatter request and schema-validation pipeline.
    Use the same phase order for normal composition, passive preparation,
    and required-input collection. Include default context in cache identity.
  - Test default versus child-authored paths from distinct directories,
    explicit null clearing a value before a fallback, non-file strings with
    expression-shaped data, nested file occurrences, and a required key
    satisfied by a fallback before schema judgment.

### Phase 2 integration

- [ ] **Filter Child Inheritance**
  - Pass one computed isolation set per phase into
    `transclusion/engine.rs::render_markdown_transclusion`, the
    `compose/nested.rs` `as_markdown()` path via caller state, and
    `preflight/collect.rs` and `reference/graph.rs::inclusion_child_options`
    via `ComposeReport.isolated_keys`. Populate the report field only for
    inline-pre-only passes, as with `body_data`. Remove the three-key list.
  - Verify baseline-free and deferred/broken-schema safety, each of the
    three pruning traps, body/prologue/epilogue sites, and all six plain
    Darkmatter partial-compatibility keys. Include one public runtime,
    preflight, or reference-graph regression per inheritance entry point.
- [ ] **Regenerate Schema Artifacts**
  - Run root `just shipped-schemas` after both schema waves and all metadata
    wiring are integrated. Verify the exact four outputs from spec Decision
    3.3 and use the per-area `just shipped-schemas --check` recipes. Run
    L1 isolation, binding parity, Claudine host schema parity, and shipped
    artifact validation tests. Treat a trigger conflict as a shared
    declaration defect and correct the annotated definition.

## Phase 3 — Passive provider defaults

### Wave 3

- [ ] **Read Provider Configs**
  - Implement Rules 17–18, including the audit's default source paths and
    extraction order for all ten providers. Add fixtures independent of the
    live home and provider installs. Cover redirect precedence, project
    switch behavior, Windows path and env rules, the 32-directory bound,
    exact 1 MiB and over-limit input, malformed fallthrough, and FIFO
    refusal on Unix. Use the public lookup matrix in Phase 7.
- [ ] **Correct Provider Metadata**
  - Pin `model_env_vars: []` in `claudine/docs/providers/overrides/pi.yaml`
    and `kilo.yaml`. Run the workspace-built
    `claudine providers generate --yes` and `claudine providers generate --check`.
    Verify `antigravity` remains empty and all seven remaining providers
    retain order. Keep unrelated generated mappings out of the diff.
- [ ] **Detect Hidden Config**
  - Add the spec's predicate in `provider_overlay/plan.rs`. Cover additive,
    materialized config relocation, config-neutral, and no-op plans with L1
    tests. This task touches overlay planning only. Phase 5 consumes it.

### Phase 3 integration

- [ ] **Cache Provider Lookups**
  - Add the invocation cache and synchronized per-source reporting from
    Rule 19. Register `config.provider_default_unreadable` as info. Verify
    repeat lookups, concurrent group-leg reuse, candidate origin
    invalidation, and exact-source reporting deduplication. Ensure diagnostics
    do not echo config text or inline env contents.
  - Run targeted provider, overlay and source-guard L1 suites. Measure
    median cold elapsed time per provider with local fixture configs and a
    fresh lookup per sample, plus the audit's four-level Kilo case. Use 20
    calls per case, one process, and record medians and cached reuse in the
    implementation log. Apply Rule 23 before proceeding.

## Hold before Phase 4

> **⛔ HARD STOP.** Implement Phases 1–3, close the Phase 3 integration, and
> stop. Do not start Phase 4, 5, 6, or 7, in whole or in part, even if Phase 3
> leaves time. If you are asked to continue, stop and report this hold
> instead. `stop-before-phase: 4` in this plan's frontmatter makes the
> `implement-plan` prompt refuse to launch Phase 4.

**Why.** On 2026-11-26 the author ruled that a Markdown sequence document is
an action run once per row and has no children: its rows are states only, and
a Markdown root `sequence:` naming a file supplies states, never a child
sequence. Phase 4 builds the opposite: "Wire Source Routes" routes whole
Markdown and formal child sources named by a Markdown root as child
sequences, and "Fold Caller Values" tests that model. Building it would also
break the research and terminal fleets, whose rosters are state lists. The
new model is specified in `2026-11-26-list-documents` (Darkmatter: list
documents and their rendering) and `2026-11-26-markdown-sequence-states`
(Claudine: Markdown rows are states).

**Lifting the hold.** The author lifts it after those two specs are
implemented and this spec and plan are amended as
`2026-11-26-markdown-sequence-states` lists under "Required corrections to
`2026-11-18-ergonomic-sequences`". Remove `stop-before-phase` and this section
in the same change.

**Phases 1–3 under the new model.** Phase 1's "Define Task Grammar" adds
`compose:` and `inline-compose:` to the shared task properties. Add them as
planned; do not add tests or docs that show them, or any other action, on a
Markdown row, since those rows lose their actions under the new model.

## Phase 4 — Sequence planning and execution

### Wave 4

- [ ] **Fold Caller Values**
  - Implement the Rule 10 fold and per-hop isolation set from Rule 11.
    Add the Darkmatter data-default records to the Claudine preparation
    handoff. Carry the same inputs through prepared owner values, step
    planning, proxy/retry and runtime lifecycles. Keep mutations and branch
    snapshots in `RuntimeState`. Test ranked values, leaf origin, nested
    default ordering, and `with:` null against a selected task value.
  - Validate the interchangeability invariant of Decision 2.4: one child
    sequence run inline, from a referenced YAML file and from a referenced
    Markdown file resolves identical values, providers, models, outputs and
    diagnostics. Caller authored state passes down at every hop and only
    isolated keys are filtered. An informal list reuses its Markdown owner.
- [ ] **Wire Source Routes**
  - Wire the shared classifier to `commands/compose/mod.rs` and the
    top-level `commands/sequence.rs` loader. Support bodyless data root
    mappings and arrays with the normal preflight and launch path.
    Verify the existing inline mismatch hint and authored YAML fidelity.
  - In `sequence/source.rs`, share formal recognition with graph planning
    and route whole Markdown child sources through their own planner.
    Preserve ordering, supported list selections and current root dynamic
    list semantics. The graph task below owns runtime dispatch.

### Phase 4 integration

- [ ] **Dispatch Nested Targets**
  - Update `PreflightAction` and `PreflightSequence` in
    `preflight/shape.rs`, and `Loader::{load_task,load_prompt,load_sequence_with}`
    in `preflight/mod.rs`. Update the earlier input discovery walk in
    `preflight/inputs.rs` and the CLI `phase1c.rs` rechecks to consume the
    same classified actions and value layers. Those paths discover owners
    and required inputs before the graph freezes. Route direct documents to the selected
    composition operation and child sequences to the existing sequence
    container path with the same cycle stack. Apply explicit options before
    falling back to group operation/flow on direct prompts. Keep group
    defaults out of a child sequence's execution state.
  - Carry an unresolved authored target as a deferred graph action. Resolve
    it at the task turn in `sequence/task/mod.rs` and the referenced/nested
    paths in `task/sequence.rs`, `task/group.rs`, and CLI `sequence/jit.rs`.
    Use the same grammar/classification check on the resolved target, its
    actual origin, and existing shell admission and source freshness
    checks. Check both mixed-executable and same-executable cycles. Keep
    effects and publications downstream of validation.
- [ ] **Validate Agent Availability**
  - Thread the value fold and ordered evidence through the graph walk described
    in Rule 14. Use the same resolved hints for JIT, preflight and nested
    diagnostics, keeping per-branch state from escaping into parallel siblings.
    Add focused public graph tests for conditional set sources, dynamic
    coverage, and valid/invalid agent hints. Ensure known missing descendants
    are reported before launching an earlier valid step.
  - Update CLI site collection in `wrap/sequence/providers.rs` to consume
    graph results, replacing its enclosing-document fallback with the
    defined/default chain and full trail. Preserve interactive selection
    through `review.rs`, with `--silent` using the non-TTY branch.

## Phase 5 — CLI selection and dry-run parity

### Phase 5 integration

- [ ] **Normalize Caller Flags**
  - Wire Rule 15 through the three command entry points and their prep
    targets. Test both shorthand and named provider flags against both
    setter forms for both conflicts. Prove positional order independence,
    help precedence, and provider tail preservation.
  - Replace selectors and request fields with the caller projection. Update
    forced-provider ownership/validation call sites and fixed wrapper
    identity paths listed in Rule 15. In existing provider review tests,
    replace the global-flag fixture with separate Markdown defined and
    data fallback fixtures. Run direct wrappers and loop target launch tests.
- [ ] **Resolve Launch Models**
  - Apply the Rule 16 six-tier selector in `select.rs`, passing selected
    provider, typed caller provenance, captured environment and explicit
    config context. Carry the typed result into prep, provider prediction,
    logs, and `ctx.model`. Add the explicit Darkmatter context identity to
    request fingerprinting and preserve it through extension, refresh and
    terminal output. Keep the inherited environment out of the identity
    channel.
  - Implement Rule 20 across the composition pipeline, bare wrapper, and
    harness launch plan. Test tier and source survival into execution, a
    no-`--repo` Codex and Gemini NativeRoot MCP overlay, additive OpenCode
    overlay behavior, non-TTY required-model failure and discovery, and
    child argv/env for all forwarding outcomes.
- [ ] **Report Dry Runs**
  - Apply Rule 21. Update shared diagnostics, selection hints, renderers
    and structured fixtures. Reuse the terminal component path for human
    findings and review. Print typed Tier 6 provenance or provider-default
    display text in previews and execution logs.
  - Run focused `claudine/justfile` `test-library` and `test-cli` cases for
    grammar, nested source, owner state, planned model, and dry run. Verify
    0/1/2, both conflicts, a model-less non-TTY OpenCode error, deduplicated
    config info with status 0, and `--silent` fail-fast independent of TTY.
    Assert no provider spawn, lifecycle event or output publication on
    failing real preflight and ordinary dry run.

## Phase 6 — Author documentation and research migration

### Wave 5

- [ ] **Migrate Research Fleets**
  - Move root `agent` and `model` into `default-values` for
    `claudine/docs/research/{acp,agent-errors,agent-logging,agent-models,agent-permissions,hooks,mcp,memory,model-config,non-interactive-sessions,plugins,resume,signals,skills,slash-commands,steering,subagents,system-prompt,usage,local_runners}/_fleet.md`.
    Do the same in the coordinator fence of `_TEMPLATE.md`. Preserve values,
    roster, initialize hooks and the row skeleton. Leave
    `agent-cli/_fleet.md`, `reasoning-level/_fleet.md`, and
    `sandboxing/providers/_fleet.md` untouched.
  - Run 20 live coordinators, the extracted coordinator example, and the
    three excluded coordinators through the built CLI `sequence --dry-run`
    using Rule 22. Use isolated fixtures with provider availability and
    config inputs supplied by the test harness. Assert exit 0, the intended
    model/provider on runnable rows, and preserved lexical row names.
    Reuse `research_sequence_artifacts.rs` for repository-source coverage.
- [ ] **Document Claudine Behavior**
  - Update Claudine's README overview, the full file list in spec
    “Documentation to update,” and all named Claudine repo and caller skills.
    Give the sequence topic the option and class matrices, CLI flag versus
    setter example, inline-to-file decomposition example and a Mermaid precedence diagram.
    Replace the CLI provider flowchart and report the new diagnostic and
    status behavior. Write `docs/topics/default-models.md` with the full
    cross-platform config matrix from Rule 17 and link it from the model
    resolution topic. Document intentional tier and provenance behavior
    for `with.model`, CLI model, native overlays, and child `MODEL`.
- [ ] **Document Schema Inheritance**
  - Update both package README overviews and the Simplified Schema and
    Darkmatter docs named in the spec. Document annotation syntax and
    optional/union behavior, sticky keys, the four inheritance sites,
    baseline-free safety and file-origin examples. Update both repo skills
    and Darkmatter's `use-darkmatter/authoring.md` and `use-dmls` schema
    authoring material. Keep canonical docs self-contained and remove
    planned markers for shipped behavior.

### Phase 6 checkpoint

- [ ] **Publish Caller Skills**
  - Run `just publish-skills` in Claudine and Darkmatter. Verify the
    installed composition of `use-claudine`, `use-darkmatter`, and
    `use-dmls`, including examples that need literal expressions.
    Use canonical docs transclusions for duplicate reference material.
    Review symbol and module comments touched by changed behavior.

## Phase 7 — Contract verification

### Input Robustness Matrix

For invalid authored input, return a located error through the public result,
execute nothing, and publish nothing. For a malformed third-party provider
candidate, return a located unreadable info result, continue in priority order
and produce no value from that candidate. Dedicated validation still requires
a model for non-TTY OpenCode. Each group below has a positive control based on
a real fixture and one edit per row. Exercise shared decoding once per loader
and shared field validation once, with a control and one invalid edit at each
other field using the same validator.

| Validator or loader | Positive controls, exceptions, and rejection probes |
| --- | --- |
| Task file target | Omitted field is unselected. No selected executable follows the Markdown-body/data-owner rule. A selected null, empty string, mapping, array, or other wrong scalar is invalid. Use one valid file ref with quoted or interpolated delimiters and a top-level suffix edit. Dispatch mismatch is distinct from missing and invalid file references. |
| Sequence source and task list | Markdown root null behaves as absent for classification. Formal data root requires the declaration or a bare list. Empty plan is a no-op. Test null for required data source and nested executable, wrong root type, mixed invalid entries and all-invalid entries using unsalvageable list/map shapes. Lenient scalar and unnamed-item normalization retains the existing skip/shorthand policy. Assert the normalized plan and located errors. |
| Value-passing mappings | Absence supplies no layer, an empty object explicitly supplies no entries, and a null value inside the map clears a defined key or leaves fallback lookup open. Null in place of the map, wrong whole-field type and reserved keys are errors. Test the validator on `default-values`, then a working object and one invalid shape on `with`, formal roots, inline containers, and closed legs/tasks. Keep illegal-option results separate from allowed-option shape errors. |
| Authored classifier and selection hints | Absent/null `prompt` and `sequence` markers classify as absent; empty or wrong-typed non-null markers are routed to their operation's shape validation. For `agent`/`model`, null is absent; invalid non-null values use existing typed hint diagnostics. Cover native supported hint scalar and provider candidate-list shapes, including invalid member and all-invalid lists through selection. |
| Markdown and YAML sources | Exercise duplicate keys, trailing invalid content within the frontmatter boundary or YAML document, YAML implicit null, and rejection of a second YAML document. For Markdown, the body after the closing fence is valid. Cover the Markdown frontmatter loader and shared YAML data loader independently. |
| JSON-family sequence sources | `.json` and `.json5` share `data::load_document_from_text`'s JSON5 branch. Cover a duplicate, trailing invalid content, and accepted JSON5 comments and trailing commas there. JSONL and NDJSON share `parse_line_delimited`; accept blank lines, reject duplicate keys and garbage with the actual record line. |
| Provider config loaders | Test strict JSON file and inline env with a shared document validator, plus JSONC, TOML and YAML with separate fixtures. Test duplicates and trailing garbage for each, JSONC comments and trailing commas, TOML's legal trailing comment, and YAML implicit null and a second document. Inline env tests add the size limit and an env-name location. Non-regular files, invalid UTF-8, and over-limit bytes are candidate fallthrough cases. |
| Provider config field extraction | A missing model, empty/whitespace-only string, and optional JSON/YAML null produce no candidate value. Wrong root/model type and wrong intermediate type in `model.name` are unreadable. Test string extraction once, then a control and one bad type through flat, nested, Claude's `env.ANTHROPIC_MODEL` priority, and Pi's two independent fields. Pi's valid provider alone yields no model. A missing optional field can use the lower layer. |
| Config environment path fields | Unset env leaves its normal candidates. Whitespace-only or relative overrides are invalid and cannot become file probes. Absolute redirects and exclusive scope follow Rules 17–18. The Goose path list follows platform separators, skips empty components, and probes valid absolute members last-first. One invalid member produces a located info result and the valid members remain usable. |

- [ ] **Exercise Public Contracts**
  - Use in-process library preflight, preparation and lookup results for
    matrix coverage. Spawn one CLI control plus one or two representative
    negatives per sequence loader. Assert the rendered location, typed
    diagnostic and no launched work. Provider config is a passive lookup
    surface, so group CLI wiring checks around info fallthrough, fatal
    required-model resolution, and source rendering instead of spawning a
    process per config field.
  - Inspect the changed validators for `#[serde(default)]`, `Option<T>`
    that loses required presence, string-filtering `filter_map`, silent
    `unwrap_or_default()` and discarded parse errors via `.ok()`.
    For a matrix regression record results for the rest of its validator
    or loader row and shape column. Preserve source guard coverage.
- [ ] **Verify Package Gates**
  - Use the rust-testing and OS skills and the repository's tier gates.
    Run `just test` and `just lint` in Simplified Schema, Darkmatter and
    Claudine. Run Claudine `just test-l2` for provider review and silent
    behavior with the shared terminal harness and non-focusing spawns.
    Keep L1 provider and config fixtures deterministic and environment-local.
  - Track Linux, macOS, native Windows and WSL2 evidence per package and
    tier. Reuse qualifying exact-tree evidence, run Linux here, and use the
    documented unattended hosts or matching CI runs for the other three.
    Run lint on the four native environments where available, separately
    from CI's Linux lint gate. The documented `just cross-check` recipe
    creates a synthetic commit, so use exact-tree CI/receipts for that
    route and run package recipes on authorized host copies for uncommitted
    validation. Use `BatchMode=yes` and the OS skill's host inventory.
    Missing host/evidence is an environment verification gap, recorded with
    the package, environment, recipe and actual failure or unavailability.
    Continue independent verification. The all-platform criterion requires
    passing evidence.
- [ ] **Close Implementation Log**
  - Run the three schema freshness checks and the provider generation check
    against the final tree, then `git diff --check`. Confirm only intended
    outputs and the exact coordinator migration set changed. Record executed
    tests, the performance sample, report examples, and differences from
    the spec for review after Phase 7. Check off completed tasks and record
    the actual verification status. Leave the feature in its active directory.
