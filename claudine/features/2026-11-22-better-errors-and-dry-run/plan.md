---
kind: plan
area: claudine
total-phases: 3
created: 2026-11-23
status: planned
phase: 1
agent: codex/gpt-6.2-sol
yolo: true
---

# Better errors and a diagnostic dry run implementation plan

## Summary and Definition of Done

Ship the passive check and preview together for `claudine sequence`, `compose`, and
`inline-compose`. Each command reports every independently provable finding, explains skipped
checks, and previews successful subjects in authored order. Human findings go to stderr and
previews to stdout. JSON emits one report on stdout and omits previews. Observed errors and
warnings exit 2, info-only and clean runs exit 0, and usage failures exit 2 before producing a
report. A dry run never prompts, launches, fires lifecycle events, executes commands, or writes
files. Allowed remote reads use invocation-local storage.

Darkmatter owns the finding record, collector, reference evidence, empty-body diagnosis, and
report renderer. `md compose` reports independent document problems together and retains its
reference recovery and policy. `md validate refs` and `md graph` preserve their published output
and exit contracts. Simplified Schema owns effective key advice and removals, with the shipped
Claudine schemas driving the same authored findings in DMLS and at runtime. A real sequence
keeps its existing lifecycle boundaries, prompts, approval order, and failure identity.

Completion requires the fleet reports, the equivalent `md` fixture, passive and real-run parity
proof, portable-path coverage on all four environments, the scoped error guards, and current
docs and published caller skills. The three phases follow the specification's delivery order.
Each numbered wave contains independent tasks that may use subagents within the session
worktree. Complete each wave before its dependent tasks. Every session is non-interactive. Use
the final implementation log for test evidence and any concrete departures. Review follows Phase
3.

## Phase 1 — Shared findings and passive dry runs

### Necessary Rules

1. **Use the checked-out interfaces.** Ship the current executable fields from
   `composition/sequence/model.rs::ExecutableField` and the current selector in
   `preflight/providers.rs::predict_key` and CLI `wrap/sequence/providers.rs::select_target`.
   Add `ExecutableField::noun(self) -> &'static str` for diagnostic construct names, use `key()`
   for authored property paths, and keep the authored executable inventory in
   `reserved::EXECUTABLE_KEYS`. Keep the sibling ergonomic sequence changes out of this patch.
   The eventual selector change will naturally alter the recorded chain because the new trace
   lives at its actual decision points. Consume the same shipped schema bundle and DMLS host
   registration interfaces already used by Claudine and DMLS. The doctor can consume validator
   hints through its own structured projection. Frontmatter repair warnings remain persisted
   document data. No runtime spike is required. The required distinctions follow from the code
   and spec, and the spec requests no performance threshold or wider measurement.
2. **Make one trail authoritative.** Add `darkmatter::findings` and keep `Diagnostic` and
   `Severity` at their existing public paths. Add the spec's `Trail`, tagged `TrailFrame`,
   `RelatedLocation`, and `Diagnostic::actionable(code, severity, message, hint)` with
   source/span/trail setters. Retain a constructor for projecting older diagnostics with
   optional hints. Move producers that set iteration and inclusion context to typed trail
   construction. Serialize the legacy `iteration_path` and `included_via` from that trail,
   preserving the empty iteration list and distinguishing an empty reference trail from an
   omitted inclusion field with the reference record's existing applicability. Match the current
   fields of `ReferenceTrailStep` and `IterationPath` when projecting, carrying any legacy
   information in typed frames. Keep all current null/omit contracts.
   `expression/diagnostics.rs::Diagnostic` is Serialize-only today and `ReferenceDetails`
   flattens the reference fields. Use additive optional reference evidence for the base,
   resolved candidate, ordered probes, and suggestions. Update
   `darkmatter/cli/schemas/output/compose.yaml` and the compose envelope serialization tests.
   Serialize filesystem path fields with `biscuit_file::to_portable_string` and retain URL
   spellings.
3. **Give the sink stable identities.** Use opaque `SubjectId` and `FindingId` indices assigned
   by `FindingSink::register_subject` and `push`. Register root and authored occurrences in walk
   order, with task occurrence identity from `preflight/shape.rs::TaskIdentity` and an ordinal
   path for an item that failed before normalization. Use file and full property for the
   location, with a separate display label. Deduplicate within subject by authored location and
   typed cause, including the construct/reference, missing input name, or skipped check name
   where relevant. A repeated push returns the original ID. `not_checked` emits one record per
   check and carries a related location only when the blocker has one. Human rendering groups
   those rows under the subject. JSON retains separate rows and serializes in subject/check
   order. These choices implement the spec's distinction between occurrences and references to
   the same file.
4. **Keep one renderer with the established reference presentation.** Move the render tree and
   layout helpers from `markdown/reference/report.rs::ValidationReportView` into `FindingReport`
   as its reference presentation. Keep `ValidationReportView` as a compatibility adapter for
   `commands/validate.rs` and `commands/graph.rs`, including listing, visibility, link handling,
   `ValidationReportData`, and the `DocumentEnvelope.summary` reference counts. Use the new
   generic presentation for the diagnostic dry run, preflight failures, and multi-family
   `md compose` reports. Compose the terminal output from renderable nodes and biscuit-terminal
   components. Render to the actual report stream and escape authored tokens. Give Claudine a
   CLI envelope with `file`, `outcome`, `diagnostics`, and severity-count `summary`. `outcome`
   is `failure` for exit 2 and `success` for exit 0. Use a known root path when a read or parse
   fails and null when there is no resolved input path. The spec's sample uses the existing
   Darkmatter outcome vocabulary and ties the operation's success to the dry-run exit rule.
5. **Reuse resolution evidence and one suggestion implementation.** Use
   `FileReference::resolve_detailed_admitted` with the originating context and its passive
   permission callback. Its first refused candidate stops the ordered search before touching
   that candidate. Keep the refusal as a typed result and never re-probe through the
   unrestricted resolver. Project `DetailedOutcome::Failed(ResolutionFailure::NoMatch)` as
   `dm.reference.missing_document` for host references and keep Darkmatter's existing reference
   codes for its directives. Preserve typed ambiguity, invalid reference, refusal, and I/O
   causes. Attach the written reference, source/base, genuine file candidates, and ordered
   search roots from `DetailedResolution::candidates`, and use `glob_hint()` where offered.
   Share a per-invocation directory listing memo keyed by authorized directory identity. Use
   ordered, permitted resolver directories for nested references and cap suggestions at three.
   Split passive candidate enumeration from the chooser in
   `claudine/cli/src/completion/operation_file.rs`. Move the general basename search and its
   bounds into Darkmatter, retaining Claudine's existing repository subtree filters and filename
   eligibility as inputs. Wire `resolve.rs::resolve_composition_source_in_context`,
   `preflight::Loader::load_prompt`, `source.rs::resolve_sequence_reference`, and
   `operation_file.rs::recover_operation_file` to the shared builder. Use directory entry
   evidence and existing platform matching to suppress case-only suggestions for an equivalent
   file. A recovery suggestion never selects a file.
6. **Carry unavailability through evaluation.** Add `ComposeOptions::with_dry_run()` and an
   invocation-local passive session in `ComposeRequest` that survives child/slot/callback option
   cloning. Carry a typed deferred reason and authored site alongside unavailable values, using
   the existing absence/evaluation machinery. Propagate through dependent lookups, conditions,
   collections, selector inputs, paths and schema slots. Preserve the authored text only at
   preview rendering. Skip only effectful or unprovable work, and retain independently decidable
   branches and siblings. Use the request's existing permission policy at each read. Read
   approval and network ask yield info and an unavailable read, deny retains its typed refusal,
   and network allow fetches through the existing fetcher without a persistent store. Use an
   in-memory data/read memo and avoid normal compose/shell/remote cache reads and writes. The
   shared dispatch sites are `pipeline`, `frontmatter_shell_expansion`, `shell_blocks`,
   `transclusion/resolver.rs`, `remote_fetch.rs`, `expression/functions` including callbacks and
   host functions, and `markdown/schemas/slots`. Reuse function and effect catalog
   classifications, including `effects/catalog.rs::EffectSafety`; a file read remains a read
   even when invoked by a function. Treat any effect not admitted by the passive policy as
   deferred before its handler runs. Never use real pre-approval, setup, or runtime lifecycle to
   learn a value.
7. **Prove empty bodies where content is evaluated.** Define `EmptyBodyCause` in Darkmatter's
   compose report with authored-empty, whitespace, excluded blocks with authored positions and
   proven condition facts, and empty rendered body with no narrower proven cause. Use the same
   visible-body predicate as Claudine's `prepare.rs` (strip embedded metadata before trimming).
   Inline composition judges its actual prompt text as well as the document body and uses the
   prompt's own provenance. A shell-, permission-, or runtime-dependent contribution leaves
   emptiness unchecked. Carry the cause through both `prepare.rs` empty checks and replace the
   guessed `render/provider.rs` text. For the real sequence, retain the current Phase 1c
   `ComposedBodyEmpty` for a bodyless source at that check and retain
   `SequenceStepNotExecutable` at the existing iterate caller. `Phase1cAttempt` stops at a
   non-missing prepare error today. The dry-run-specific bodyless finding is
   `composition.no_executable`, so suppress the redundant `dm.compose.empty_body` on that
   subject. A body-bearing document's implicit body step gets info and proceeds through the
   usual prompt checks.
8. **Keep real failure selection independent of report order.** Store the original typed failure
   at its original return boundary. At `wrap/sequence/mod.rs::execute_sequence_inner` preserve
   the sequence after root initialize: input discovery and root planning/collection, graph and
   lifecycle literal admission, shell audit/approval, provider selection/review, owner verdict
   recheck, forwarded-switch validation, then Phase 1c. Preserve Phase 1c's attempt behavior in
   `run_phase_1c_attempt`: it accumulates missing contexts, returns another failure immediately
   when it meets one, and requests missing values after the attempt reaches its end. Attach
   findings only once the original boundary has settled, using a passive view of the state
   there. Do not run later tasks' JIT composition as a new real preflight check. A later error
   cannot preempt an earlier prompt, approval, retry, or interruption.
   `CompositionError::CheckFailed { report: Box<FindingReport>, cause: Box<CompositionError> }`
   delegates code, category, disposition, origin, detail, and message used by lifecycle and
   persistence to the original cause. Delegate severity and the concise diagnostic message too.
   Route cause traversal through the original error’s registered source in
   `diagnostics/discovery.rs::next_registered_cause` and preserve
   `diagnostics/snapshot.rs::DiagnosticSnapshotExt::from_diagnostic` output. The wrapper must
   not insert a second primary entry into the stored cause chain. Render its report once.
   Preserve existing wrapper classification and outer lifecycle routing. Report newly observed
   advisory warnings once before task one on successful preflight. Recheck only answer-dependent
   findings after input or provider collection and preserve the frozen graph and closed retry
   approval.
9. **Use one authored walker with explicit recovery.** Split per-item normalization from list
   iteration in `sequence/normalize.rs::normalize_plan_at`. Use existing formal/list/source
   classification, `NestedSources`, cycle and catalog keys,
   `Loader::walk_plan/load_task/load_group/load_sequence/load_external_task`, and
   `inputs.rs::discover_sequence_inputs` and its owner walker. Share source bytes, parse
   results, and context-appropriate prepared values between both existing walkers and the
   check/preview. Collect at each independently bounded task or group leg and continue into its
   siblings. A bad item gets its original ordinal, a malformed or command-backed root list
   leaves its children unchecked, and a bad child file blocks only its subtree. Keep known task
   identities for repeated references even when sharing the loaded source. No second parser or
   independent task tree. Capture `TaskSite` in the library with occurrence, label, authored
   file/property, and optional proven span. Name the CLI selector's existing private `TaskSite`
   as `SelectionSite` to distinguish its input state, and have it carry the shared site.
   Transitively retain enclosing group and sequence frames. Locate authored places from existing
   source maps or `SourceRef::OnDiskSpan` and suppliers through `SourceRef::Supplied`. Use
   file-only for unproven positions.
10. **Observe each command's actual asking rules.** Record provider attempts in the same
    branches that currently choose them, including empty links and every consulted enclosing
    document. For sequence the order is flag, caller setter, task `with.agent`, prompt
    frontmatter, then nearest usable enclosing sequence. `preferred_agent` seeds review only.
    Capture installed observations from the read-only PATH snapshot without version or catalog
    refresh probes. Use `wrap/sequence/providers.rs::select_target` and `review::needs_review`
    with the real stderr TTY rule, removing the current dry-run exception that masks a task's
    invalid explicit hint. For direct and inline use `wrap/composition/target.rs` and the
    eligible preferred-agent fallback. Reuse the pure model precedence from
    `commands/exec_prep/mod.rs::resolve_model_and_validate`: flags, provider-specific variable,
    `MODEL`, hints, then configured default. Apply `ProviderInfo::model_required_in_non_tty`
    using actual launch mode. Sequence is always non-interactive. A deferred or unresolved
    provider leaves the model check unchecked. Apply the config, stdin, stderr, and silent
    collection policy plus the whole cohort’s shape predicate. Both
    `phase1c.rs::collect_or_report` and `compose/prep.rs::prepare_collecting_missing` refuse the
    collection batch when one missing input has an unsupported shape. Emit one row per missing
    input, at warning for a cohort the real command can collect and at error for a cohort it
    would refuse. A deferred value is info and remains outside that cohort. Evaluate missing
    inputs independently of unresolved provider where preparation supports it. Use passive
    chooser candidate enumeration to describe a pending selection. Redact secret-shaped values
    before adding findings or preview metadata.
11. **Enter dry run before startup can write.** Declare hidden `dry_run_shell` as a refusal-only
    argument in `SharedComposeArgs`, with no dependency on `dry_run`. Add `json` with a clap
    requirement on `dry_run`. Keep ownership derived from clap in `argv/partition.rs`, preserve
    help and the first `--`, and return the spec's targeted usage error through the selected
    clap subcommand before command execution. Include the existing `-o`/`--output` refusal in
    the usage path. Dispatch the three valid dry-run commands in `main.rs::dispatch` before
    launch setup can report a top-level error, before telemetry/progress,
    `request::darkmatter_config` in `MigrateLegacy`, `refresh_host_registration`, and
    `ensure_config_exists`. The dry-run handler owns launch snapshot and errors, with explicit
    passive config passed downstream. Use Darkmatter `ConfigLoadMode::Passive` and its passive
    repository projection. In `dispatch/loader.rs` share canonical YAML loading and merging, use
    schema defaults in memory for absent user config and an absent repo as no override. Reuse
    the pure parse/flat-upgrade/typed projection from `config/json_conversion.rs` for
    convertible legacy-only config. Ignore a legacy sibling when canonical YAML exists, treat
    retired pre-2026 config as absent with an info note, and surface invalid selected
    configuration with its typed location. Skip transaction cleanup and migrations. This follows
    the current YAML precedence and separates projection from the writer in `prepare_scope`.
12. **Finish one report and keep previews local.** Give
    `claudine/cli/src/commands/document_check.rs` the early dispatch, report envelope, and
    preview buffer for all three commands. Reuse source resolution and passive value preparation
    from `commands/sequence.rs`, `compose/prep.rs`, and the library. Preserve
    `commands/compose/ownership.rs::own_caller_arguments` against authored source before applying
    caller overrides. Feed its setter provenance and forwarded tail into the applicable checks.
    Apply `commands/sequence.rs::reject_sequence_interactive` as a located root finding and use
    non-interactive target requirements. Stop calling sequence
    preview through `task_run.rs`/the task runner and stop direct/inline document previews
    through the wrapper launch pipeline. Preserve wrapper-command dry runs outside these three
    document commands. Record expected failures with typed projectors, and record otherwise
    unclassified returned errors as `internal.bug` with the available message and a concrete
    reporting hint. Send the buffered human report once, then successful previews, and suppress
    normal notices/progress. Under JSON still perform composition and preview validation,
    suppress preview text and human rendering, and emit one envelope. Keep rendering errors
    scoped to their respective subject. Use the report's diagnostic severities for exit 0/2.
    Preserve direct compose's strict preparation and inline compose's launch-stage required-key
    rules. Use inline prompt composition and its actual frontmatter prompt origin, and report
    missing or wrong-type prompt input through the same report. A failed configuration verdict
    blocks only checks that need that configuration. Its unavailable permissions block document
    reads, while captured argument and path observations remain reportable. Effect and guard
    descriptions are preview annotations. Count info records for actual checks skipped. In the fleet
    fixtures the blocked model check and two blocked metadata checks give three info records.
13. **Collect at the existing Darkmatter gate.** Wire the shared collection into
    `darkmatter/cli/src/commands/compose.rs::check_references` and the call gate used by its
    ordinary run and `compose/lifecycle.rs` after initialize. Keep condition-blind reference
    discovery and its original recoveries, stdin eligibility and disabled operations. Share the
    data memo and read set via `with_reference_check_slot_session`. After a reference stop,
    evaluate only the independent passive parts of call, schema, and prospective-empty checks. A
    call refusal still stops real expressions and effects. Mark checks dependent on a refusal or
    failed preparation as unchecked. A known false condition accounts for an excluded block even
    when its reference is broken. A deferred condition never establishes an empty body. Continue
    through the real composer only when the existing typed failures and policy permit it. Grade
    `dm.compose.empty_body` warning in `md` and error for a proven empty Claudine provider
    prompt. Reuse the existing `BlockingPolicy` catalog and warning deduplication.
14. **Keep key rules with the effective owner.** Add `removed(replacement)` as a
    named-property-only declaration in Simplified Schema. Lower to a false property schema with
    owning-object removal metadata. Reject required/default combinations, preserve explicit null
    as authored presence, and omit removed keys from completions and suggestion pools. Use an
    object opt-in metadata key `$advise-unknown-keys` with a list of exact excluded names,
    lowered as an internal annotation. In the open step declaration opt in with exclusions
    `agent` and `model`. Match intentional pattern properties first, and treat the existing
    general step-data catch-all as admission, with advice applied to otherwise undeclared keys.
    Emit advice only when the best ranked candidate is unique. Move
    `markdown/schemas/clean.rs::nearest_declared_key` and its Damerau/normalization logic to
    `simplified-schema/lib/src/validation`; preserve clean's threshold and tie ordering. Resolve
    candidates through the effective object, handling imports, composed objects, and applicable
    union arms without using unrelated arms as evidence. Put hints and declaration provenance on
    the typed validation problems. Keep advisories out of strict failure. Extend
    `prepared.rs::CompiledValidators::collect_problems` for shared projection, including
    `validate.rs::build_problems` for union key problems. Carry advisories separately in the
    verdict from prepared validation so an advisory alone never fails strict validation. Point
    DMLS and Darkmatter at that structured projection.
15. **Keep the authored roles explicit.** In `claudine/schemas/sequence-types.yaml` keep open
    `task` under sequence `step`; extract a shared task declaration and compose a closed
    `task-file`/group-leg shape from it with the same removal metadata. Use the closed shape in
    `group.tasks` and add a shipped kind trigger for standalone tasks and referenced groups,
    including the current catalog-entry grammar. Match the parser's actual allowed keys in
    `parse_task_object` and `Loader::build_group`, leaving executable-dependent field
    restrictions and container lifecycle placement with the parser. Use prepared owner-specific
    metadata for root `fail_fast`, task `setup/teardown/execution/fail_fast`, and group
    `execution`, following the spec's replacements. Keep `group.fail_fast` and `kind: sequence`
    legal. Emit the same `RemovedGrammar` typed refusal from
    `sequence/mod.rs::resolve_sequence_plan_with`, `formal.rs`, `grammar.rs`, and the group
    loader at the same authored property and list the standardized schema replacement. Keep the
    old lifecycle `err.detail` representation from the selected original boundary, including a
    legacy detail path or value-sensitive legacy replacement wherever that field previously
    exposed it. Follow the existing `RemovedGrammar` render/detail split and keep
    container-event placement distinct.
16. **Split sequence wrappers without changing identity.** For Phase 3 replace
    `SequenceExternalLoad` with `SequenceReferenceResolution`, `SequenceSourceRead`, and
    `SequenceSourceParse`, retaining boxed typed causes and sites. Resolution includes detailed
    probe evidence where parsing the reference succeeds. Read and parse classify the actual
    operation, including Markdown frontmatter, YAML conversion and JSON5 duplicate detection.
    Route `source.rs`, `data.rs`, nested loaders, and their public convenience callers through
    those wrappers. Preserve the legacy diagnostic classification, persisted cause and lifecycle
    fields through explicit projection. Change `expect_reference` in both preflight walkers to
    take the referencing site. Replace `SequenceInvalid` string-only construction with a located
    variant and provide specific render arms for every `Sequence*` variant.
    `SequenceSelectionFailed` has no production constructor, so remove its variant and test-only
    uses. Base the parser's closed-key hint on the shared ranker and the executable-specific
    allowed field set. Render one-based step numbers in prose while keeping zero-based authored
    paths. Fix unterminated block excerpts using `loaded_source_context_for_errors` and the
    body-to-file offset. Follow the inventory's cut line.
17. **Validate headlessly.** Use fixture-injected terminal and provider observations for L1,
    test toolkit supervised process fixtures for CLI wiring, and local loopback HTTP with no
    external dependency. Assertions cover asking eligibility and prove the check never enters
    the UI. No new real-terminal tier is needed to make those assertions. Use the existing
    cross-environment coverage and OS skill's authorized host routes for the path fixtures. Keep
    a missing host result as an explicit evidence gap in the final log while completing all
    other checks. No CI configuration changes or performance probe are in scope.

### Establish baselines

- [ ] **Capture Contracts**
  - Record the old real-run failure facets and persisted result for graph, reference,
    input/provider, and Phase 1c failures before changing the paths. Include a case where
    grouped display order differs from the original failure order. Capture plain and JSON
    reference output and exit behavior for `md validate refs`, including listing, empty output
    and failures. Capture `md graph` reference-report behavior at its existing caller.
  - Use file-backed fixture data and existing test toolkit binary helpers. Use plain width 80
    with `NO_COLOR=1`, deterministic launch-relative paths and deterministic fake provider
    observations.

### Wave 1

- [ ] **Build Findings**
  - Darkmatter owns the types, constructors, collector, policy catalog, serializer and
    reference-renderer extraction in Rules 2–4. Update existing diagnostic
    construction/projectors and all report adapters. Compare the reference baselines
    byte-for-byte, including existing machine output and `md graph`. Test equivalent findings vs
    different occurrences, skipped-check linkage, tagged trails and legacy fields.
- [ ] **Isolate Startup**
  - Claudine owns the startup, config loading, and clap ownership changes in Rule 11. Define the
    early command boundary and its explicit read-only startup inputs. A separate integration
    task will supply the report and command check invocation. Exercise help and
    retired/forwarded flags at all positions, including an option after the file, a token after
    `--`, and `--json` without dry run. Prove configuration reads cannot migrate or create
    anything.

### Wave 2

- [ ] **Guard Composition**
  - Darkmatter owns passive dispatch, deferred-value propagation, and empty-body instrumentation
    in Rules 6–7. Keep `md` CLI wiring and reference-report code outside this task. Expose
    passive composition's full findings plus its optional partial markdown and empty-body cause.
    Exercise nested includes and callbacks, blocked lookup and slot propagation, normal policy
    refusal, and effectful calls and commands. Verify a passive invalid call prevents downstream
    effect dispatch.
- [ ] **Build References**
  - Darkmatter and biscuit-file own the reference finding builder and suggestion search in Rule
    5. Own reference projection and filesystem probe code. Supply the reusable directory-listing
    memo and root-filter interface. Leave Claudine caller edits to graph and CLI integration.
    Prove no-match versus denied/ambiguous/I/O, candidate order, literal glob hint, permission
    filtering, and reused directory reads.
- [ ] **Recover Graph**
  - Claudine library owns per-item recovery, locations and occurrence identity, the exhaustive
    graph/input traversal mode, and typed provider observations in Rules 8–10. Own
    `composition/sequence/{normalize,source,data,preflight}` and the shared task identity and
    site. Wave 3 owns integration with the reference builder. Continue through every known sibling
    without retaining invalid executable nodes in a real execution graph. Cover nested groups,
    catalog identity, cycles, repeated references, dynamic roots, local permission needs,
    lifecycle-written values and shape errors. Keep ordinary fail-fast return entry points
    available for real-run ordering.

### Wave 3

- [ ] **Combine Document Checks**
  - Darkmatter CLI owns Rule 13 and the `notes.md` fixture. Feed the shared sink from the
    reference, call and schema families and the passive body verdict. Test failed references
    with independently excluded content, unknown conditions, warning denial and permitted
    reference recovery, stdin behavior, and lifecycle initialize producing an input. Fix the
    file-frame excerpt in the block error renderer.
- [ ] **Integrate Claudine Checks**
  - Wire the graph and reference builders, provider/model/input eligibility, and Darkmatter
    passive compose into the command boundary from Wave 1. Own Claudine's composition resolver
    and chooser adapters, `commands/document_check.rs`, the live sequence integration in
    `wrap/sequence`, and `CheckFailed` projection/rendering. Retire the previous document
    preview paths. Keep per-task runner code used by real runs and wrapper previews. Run target
    and input preparation even for a missing provider when independent. Compose every eligible
    task once, keep its marked preview in order, and survive task and preview failures.

### Phase 1 checkpoint

- [ ] **Prove Passive Reports**
  - Add fixture directories and controls for every Phase 1 new or changed report code under
    `claudine/cli/tests/fixtures/check/` and `darkmatter/lib/tests/fixtures/findings/`. Include
    `fleet-run1/`, `fleet-run2/`, and the `md` equivalent. Use `insta` for width-80 plain and JSON and a
    catalog guard relating each scoped code to fixture and snapshot. Assert fleet counts 2
    errors, 2 warnings and 3 info on redirected stderr, 1 of 2 tasks previewed, and warning
    severity for a reviewable provider on terminal observations.
  - Verify three-task continuation past a failed second task, literal blocks and `file_exists`,
    unresolved and invalid provider distinctions, missing/unsupported input shape, pending
    bare-name and file-input choices, model precedence and OpenCode's model requirement. Assert
    usage, 0/2 outcomes, stream separation, report-only failures for invalid config and root
    input, and no standard `StatusBlock` in all dry-run failure fixtures. Exercise a
    layout/preview failure without losing subsequent output.
  - Use separate effect fixtures for frontmatter shell, body shell, command-backed list, shell
    task, side effect, lifecycle initialization/prewarm, file approval, remote
    include/data/schema, and interactive missing input. Compare fixture home/workdir before and
    after, include provider and command markers, and count loopback requests for allow, ask, and
    deny. Prove skipped constructs are visible and dependent checks stay unchecked. Include
    inline file bytes, config/registration files, and cache directories in no-write assertions.
  - Verify real preflight against the baselines from this phase. Cover answer-dependent
    rechecks, retry with closed approvals, an initialize-created input, and an earlier task
    providing a file or value needed only at a later task's execution boundary. Assert displayed
    findings plus the complete legacy typed cause, persisted result and lifecycle routing.
- [ ] **Enforce Phase Scope**
  - Add the site, noun, catch-all, snapshot and boundary-mapping guards beside
    `claudine/cli/tests/l1/error_guards.rs` and its `source_scan.rs`. Enumerate existing
    sequence omissions with a Phase 3 tag and reject new or stale allowlist entries. Cover all
    Phase 1 sites with located evidence and full cause projections, including a missing root or
    nested document, a failed prompt load and a Darkmatter transclusion. Store explicit mapping
    cases per blocking finding and boundary instead of inferring identity from display order.
  - Run focused and then area L1 from Claudine and Darkmatter. Run biscuit-file's L1 against the
    reference probe fixture and existing resolver suite. Update the Phase 1 public docs and
    their comments with the behavior shipped by this phase.

### Input Robustness Matrix

A rejected input produces a located error naming the file and the authored field when its
position is proven. No action runs and no artifact is published for that invalid input or its
dependents. The dry-run command continues independent checks. Exercise each validator or loader
once through a public result, with one edit per passing tool-authored control. Add one
representative invalid field for every additional caller that uses the same validator. Reuse
matching existing cases, including the grammar and duplicate-key tests in
`claudine/lib/src/composition/sequence/tests.rs`.

- **Sequence source and item validators.** At a formal root or an inline nested sequence a
  present null source is invalid. In composed Markdown an absent sequence key keeps the existing
  default body plan. An authored empty list is a valid zero-step plan. Test wrong whole type,
  one bad element, and all bad elements through recovered check results. For formally authored
  object steps, omitted, null, empty, and wrong-type names are errors. For lenient foreign-data
  rows, absent names take the one-based ordinal and non-null non-string names retain their
  existing scalar projection. Test name-only shorthand and number/boolean items on the lenient
  path. A null item is invalid in both modes. A resolved null from a whole-value source
  expression retains its existing empty-list meaning in `resolve_expression_source`; authored
  `sequence: null` still fails. Bare names and references must satisfy the existing nonempty
  checks. An absent executable on an ordinary step selects the source body; the same absence is
  invalid in a closed task file or group leg. Two executable keys are invalid, including a pair
  containing `sequence`. Presence of an invalid executable value, including null or an empty
  reference, is a located refusal and never falls through to implicit-body composition.
- **Task and group fields.** Exercise nonempty string, mapping, executable-specific options and
  required list validators as separate families. Missing optional `when`/`with` keeps their
  existing absence semantics; authored invalid values are validated at the property. Test
  `with: {}` as an explicit empty input overlay. A mixed or all-invalid task list reports its
  bad legs and never coerces them away. `groups` in the catalog remains a required list of group
  entries, and catalog selection preserves authored index and name. Exercise a control and one
  invalid child for every alternate entry point: inline group, referenced group, selected
  catalog entry, external task, and referenced sequence. Preserve `null` as authored input for
  prompt schema validation and runtime-dependent missing input as unchecked.
- **Sequence document loaders.** `data.rs::load_document_from_text` shares YAML across its
  extensions, JSON5 across `.json` and `.json5`, and per-line JSON across `.jsonl` and
  `.ndjson`. Cover duplicate keys and trailing invalid syntax once per loader. Prove single YAML
  document admission, explicit YAML null, JSON5 comments and trailing commas on the shared JSON5
  path, and blank JSONL lines without shifting the original line in a later error. Retain the
  Markdown loader's authored frontmatter diagnostics and body positions, and test a malformed or
  duplicate frontmatter field through the public Markdown result used by the check. Route only a
  control plus one or two invalid shapes per loader through the spawned CLI, asserting the
  resolved source location and zero execution. Distinguish an unreadable file from one that was
  read and failed to parse.
- **Passive config.** Preserve existing YAML schema defaults and explicit-null semantics for
  Claudine user and repo scopes and Darkmatter permissions. Use one YAML
  syntax/duplicate/multiple-document set for shared loaders and one public control plus invalid
  setting for each scope projected into the dry-run report. Exercise a valid empty user mapping,
  absent user file/defaults, absent repo/no override, legacy-only JSON converted in memory,
  canonical YAML taking precedence over a legacy sibling, and malformed selected legacy JSON
  with its location. Cover an allowed/ask/deny permission control and one wrong type per
  projected policy family. Reuse existing configuration validator shape tests and test the
  changed no-write startup through public results. The format reader stays the existing one per
  format.
- **Schema declarations.** In Phase 2 cover `removed(...)` with valid absence and both null and
  non-null presence. Cover empty replacement, unsupported placement or argument shape, defaults
  and required collisions, and precedence over overlapping patterns. Exercise the opt-in
  metadata through a passing sequence step declaration, wrong whole value, mixed and all-invalid
  exclusions, explicit empty list and absent list. Empty list opts in with zero named
  exclusions; absent metadata disables advice. Explicit null is invalid metadata. The open step
  control includes intentional extension keys. Closed branches and runtime provenance use the
  same public validation result.
- **Audit.** Before phase completion search load-bearing paths for silent `#[serde(default)]`,
  `Option<T>` conflating absent and null, and unchecked `filter_map`, `unwrap_or_default()`, or
  `.ok()` over validated fields. During review, any matrix defect calls for listing all shapes
  for that validator and all its wired fields, per the repository's input matrix rule.

## Phase 2 — Schema-derived checks

- [ ] **Extend Key Validation**
  - Simplified Schema owns parsing and round-tripping `removed(...)` and object opt-in metadata,
    named-property placement checks, lowering, effective-object metadata/querying, and
    structured key problems. Implement Rules 14–15 at the grammar/serializer, lowering,
    resolution, prepared validation and completion boundaries. Keep absent keys accepted, emit
    one removal per key even under a pattern, and return an authored value with the original
    path.
  - Move the `md clean` ranking with its characterization cases. Preserve threshold,
    case/separator variants, Damerau transposition, deterministic closed-object ties and
    no-match behavior. Require a unique best candidate for opted-in open advice. Include nested
    composition/imports, anyOf branch relevance, a real pattern extension, removed candidates
    and the dedicated selection-key exclusions. Run Simplified Schema's focused L1 before
    consumers start.

### Wave 4

- [ ] **Project Schema Findings**
  - Darkmatter owns `markdown/schemas/clean.rs`, `compose/report_diagnostics.rs`, and the
    `md schema validate` adapter. Use the shared ranker and typed problems for suggestions and
    removals. Preserve clean's existing message and repairs. Add new Darkmatter codes and
    preserve the existing grade of unknown keys. Provide one key-token location, hint, and
    related declaration location where proven. Route advisories outside strict validation
    failure and remove the unused deprecated-key catalog entry from DMLS.
  - DMLS owns the adapters in `diagnostics/frontmatter.rs` and `diagnostics/yaml_document.rs`.
    Use shared code/message/hint and project related locations to LSP `relatedInformation`.
    Convert authored spans once to the negotiated position encoding. For related sources with
    proven paths and positions use the existing URI/position mapping. Ensure completion filters
    removed fields from effective shapes. Use the working registered-host/standalone-kind
    interfaces, which already support Claudine YAML documents.
- [ ] **Declare Authored Roles**
  - Claudine owns the authored schema definitions. Add the removed declarations and open-step
    opt-in from Rules 14–15. Use kind-specific shipped triggers for root task, group and
    group-catalog shape, and assign closed task references to group legs in
    `sequence-types.yaml`. Preserve accepted root metadata and legal keys from the parser. Keep
    group option and task executable-dependent placement rules where the parser applies them.
  - Replace hardcoded removed-key dispatch in `sequence/mod.rs::resolve_sequence_plan_with`,
    `formal.rs::formal_keys`, `grammar.rs::validate_task_grammar/removed_task_key`, and
    `preflight/mod.rs::Loader::build_group` with an owner-scoped prepared declaration query.
    Keep existing typed refusal order, location and lifecycle projections. Use the effective
    declaration required by the active authored role even when validating a nested object
    inline. Leave the standalone task and leg unknown-key rendering changes to Phase 3.

### Phase 2 checkpoint

- [ ] **Integrate Schema Checks**
  - Run root `just shipped-schemas` and verify all three tracked artifacts are current. The root
    recipe regenerates Simplified Schema, Darkmatter and Claudine in dependency order. Wire
    `document_check` and its shared graph walker to judge the authored document and each valid
    task, group and sequence site, including externally loaded sources. Reuse prepared bundle
    and source provenance. Report a removal even where old parser aborts, and deduplicate it at
    that site. Check authored declarations and typed fields passively while leaving runtime
    `$schema` values unchecked and deferring their launch verdict.
  - Add editor/runtime parity fixtures using one shipped revision through DMLS's registered host
    and Claudine's embedded bundle. Assert code, text, hint, related position and equivalent
    authored ranges under both supported position encodings, including CRLF. Cover closed
    unknown keys, nested and imported owners, opted-in near-miss, unrelated open data,
    group/leg/task/root removals, null presence, and legal `group.fail_fast`. Prove the real
    parser preserves its typed and persisted failures at the old boundary.
  - Add fixtures, control and snapshots for all Phase 2 changed codes. Check `md clean` output
    compatibility, `md schema validate` hint projection, and DMLS related information. Run
    `just test` in Simplified Schema, Darkmatter, and Claudine. Update the language
    declaration/options and diagnostic docs, the DMLS schema guidance, and affected repo skills.

## Phase 3 — Sequence errors and delivery

### Wave 5

- [ ] **Complete Sequence Sites**
  - Claudine owns Rule 16 and the remaining rows of the spec inventory. Split
    resolution/read/parse in `composition/sequence/{source,data}.rs` and retain the typed
    reference outcome and actual parse or read cause. Use a boxed `TaskSite` for all surviving
    `Sequence*` variants that lack both `source_path` and `property`. Supply a root file site
    for root-only errors. Apply it to normalization, graph and input loads, nested task/provider
    selection, JIT/iterate and runtime wrappers. Fix `SequenceExclusiveExecutable` using
    `EXECUTABLE_KEYS`, carry the full path through `expect_reference`, add shared near-miss
    hints to `parse_task_object`, and remove `SequenceSelectionFailed`. Build the message from
    the authored construct and proven cause, with data for the site and a cause-specific hint.
  - Keep the first-error source and old failure facets in explicit mapping tests while adding
    accurate locations and prose. Exercise each changed variant or projection with positive
    fixture/control and plain+JSON snapshots. Empty the catch-all and site exceptions for
    sequence variants, leaving unrelated allowlisted variants outside the scope. Update the
    guards against actual enum and render matches so deleted or newly uncovered variants fail.
- [ ] **Complete Document Sites**
  - Darkmatter owns the scoped reference and schema finding sweep. Fix the remaining report rows
    that fail the spec's four-part standard. Preserve reference recovery codes and statuses. Use
    plain-text and JSON fixtures to confirm missing file versus permissions and parse errors,
    confirmed versus unproven positions, real source-frame excerpts, authored and supplied
    values, rendered hint and trail. Keep call-typing and lifecycle error rendering outside the
    sweep.
- [ ] **Update Caller Docs**
  - Claudine owns `docs/cli/sequence.md`,
    `docs/topics/{composition,output,error-architecture,pre-flight-checks}.md`,
    `docs/topics/flow-control/sequences.md`, README overview and `use-claudine`. Show the two
    stages, exit/status and stream contracts, passive table, request/asking policy, real-stop
    boundaries, removal message and the fleet-style example. Remove published references to the
    old effectful document previews.
  - Darkmatter owns `docs/cli/{compose,validate,data-contracts}.md`,
    `docs/errors/composition-problems.md`, new `docs/errors/message-standard.md`, README link,
    library compose option docs, and `use-darkmatter`/`use-dmls`. Document the shared records
    and compatibility projections, empty-body causes, report policy, message standard, and
    advisory semantics. Update Simplified Schema's type and diagnostics docs and relevant repo
    skills. Remove planned markers for shipped behavior, update the affected symbol
    documentation, and keep current docs free of references to feature directories.

### Final verification

- [ ] **Verify Portable Paths**
  - Add portable path assertions to the public output fixture suite. Verify JSON uses
    `to_portable_string`, relative human paths use `/`, and magic roots keep resolver order.
    Test Windows drive absolutes, authored backslashes and UNC base using native lexical path
    cases, no live share. On macOS use a case-insensitive test volume for the case-only file
    behavior. Assert the same line and column with LF and CRLF. Verify Linux, macOS, native
    Windows and WSL2 on the matching environments, including that target-specific tests
    executed. Use qualifying receipts and existing environment scheduling. Run the local Linux
    recipes now and attach remote evidence from the authorized host or existing CI execution for
    the exact revision. The normal `just cross-check` recipe creates a temporary commit. Use
    exact-revision receipts and normal CI for remote evidence in this no-commit implementation.
    Record unavailable execution evidence as pending in the implementation log without delaying
    the remaining tasks.
- [ ] **Verify Release Contracts**
  - Run `just test` and `just lint` in each changed package area. Use the published-schema
    freshness checks in Simplified Schema, Darkmatter and Claudine. Run `just publish-skills` in
    Claudine and Darkmatter and check the composed caller docs for all three affected skills.
    Simplified Schema has no caller publication recipe. Confirm help and completion, report and
    reference compatibility snapshots, old real-run cause projection, all catalog snapshots and
    source guards, no focused terminal windows, and no unplanned dependencies or generated
    changes. Run `git diff --check` and finish with the completed tasks, evidence and any
    remaining environment gaps in the implementation log.
