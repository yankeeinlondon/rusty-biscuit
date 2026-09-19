---
total_phases: 7
created: 2026-09-17
phase: 1
agent: codex/gpt-5.6-sol
yolo: "true"
---

# Plan: Capture Agent Skills Discovery, Scope, and Frontmatter Behavior

Implements [spec.md](spec.md) by replacing the descriptive Agent Skills
research shape with a revision-2 relational contract, adding deterministic
domain validation, refreshing every active provider through the existing fleet,
and regenerating the downstream comparison and published Claudine skill copy.

## Work Summary and Success Criteria

This fix is a coordinated migration across four owned layers:

1. **Research contract:** `docs/research/skills/_schema.yaml` must describe
   versioned locations, root coverage, collision behavior, evidence, gaps,
   recognized frontmatter properties, and the required `name`, `model`, and
   `allowed-tools` behavior records. Darkmatter continues to validate document
   shape only.
2. **Relational gate:** a new `claudine-gen::skills_check` module must enforce
   stable IDs, foreign keys, required case products, outcome-dependent fields,
   evidence policy, gap ownership, provider identity, revision 2, and active
   roster coverage. It follows the architecture of `steering_check` without
   sharing topic-specific rules.
3. **Fleet lifecycle and CLI:** `claudine providers skills check [<slug>]
   [--json]` must expose the gate through the existing generator passthrough.
   The fleet must use a single freshness predicate and manage
   `contract_validated` so no failed or incomplete report can be skipped on the
   next run.
4. **Research outputs:** Claude, Codex, and one non-Claude pilot establish the
   vocabulary before all remaining active roster reports are migrated. The
   Agent Skills summary and its published skill copy are regenerated only after
   the complete fleet is green.

The migration is intentionally temporarily red: once the revision-2 schema and
fleet-level checker land, legacy active reports must fail until refreshed. Do
not add compatibility defaults, weaken case coverage, or treat the research as
runtime linker policy to hide that state.

### What Successful Completion Looks Like

- Every non-`skip_research` roster entry has a non-empty revision-2 skills
  report whose frontmatter passes `md schema validate` and whose relationships
  pass `claudine providers skills check`.
- Every report separately answers user and repository `.agents/skills`
  discovery, native and Claude-compatible roots, nested/configured/plugin
  inquiries, all ten collision cases, and the required frontmatter cases.
- Concrete support findings cite non-inference evidence with version, date, OS,
  environment, execution mode, configuration, claim, and limitations; every
  unknown points to one live gap, and no orphan gap remains.
- The checker rejects duplicate IDs, dangling references, missing coverage,
  illegal outcome fields, inference-only concrete claims, provider mismatches,
  legacy revisions, duplicate behavior tuples, and missing/orphan gaps while
  accepting complete override, coexistence, and reasoned-unknown examples.
- A recent legacy or invalid report is researched; a recent revision-2 report
  is skipped only when its body is non-empty, its schema validates, and
  `contract_validated` is true. A failed run leaves the marker false.
- Human CLI output uses `TerminalRenderable` components; JSON remains stable,
  pipeable stdout, with diagnostics/status isolated from it.
- The refreshed summary separately compares scope, root family, collision
  outcomes, frontmatter behavior, and evidence gaps, and explicitly covers
  `.agents/skills`, `name`, `model`, and `allowed-tools` portability.
- No runtime linker, provider precedence, generated provider metadata, or
  runtime permission behavior changes. Any justified runtime follow-up is
  recorded only through `requires_claudine_update` and its non-empty `reason`.
- `just test`, `just test-l2`, `just lint`, passive prompt/corpus checks, full
  fleet schema validation, and the skills checker pass without real home
  mutation, provider side effects, audio, or focused terminal/browser windows.

### Plan Conventions

- Phases execute in order. Tasks marked **‖ wg-N** within the same phase are a
  work-group and may run concurrently because their primary files are
  disjoint. Tasks without a shared work-group label are dependency-ordered.
- Every phase ends with an observable checkpoint. Do not begin the next phase
  until it passes.
- Rust tests use nextest through the package-area `just` recipes. Do not run
  `cargo fmt` as part of this work.
- Research probes use disposable homes, repositories, config roots, and
  harmless markers. They must be non-interactive and must never modify a real
  user skill installation.

## Phase 1 — Rulings, Spikes, and Baseline

### Necessary Rules

- [ ] **Ruling 1 — Shape versus relationships**
  - Keep types, enum vocabularies, required properties, and nested record shape
    in `skills/_schema.yaml`.
  - Keep ID uniqueness, foreign keys, case coverage, conditional fields,
    evidence sufficiency, gap ownership, provider/revision checks, and active
    roster coverage exclusively in `claudine-gen::skills_check`.
  - Do not add skills-specific behavior to Darkmatter or reuse
    `steering_check` rules through a generic abstraction.
- [ ] **Ruling 2 — Required collision cardinality**
  - Require every stable collision case ID to appear.
  - For an applicable case, require a discovery record plus distinct explicit-
    and implicit-activation records; permit additional root-pair and execution
    records.
  - Represent a wholly inapplicable case with a reasoned `not_applicable`
    record. Treat untested applicable behavior as `unknown` with the required
    stage/mode records and a live gap, not as not-applicable.
- [ ] **Ruling 3 — Evidence and gaps**
  - Stable evidence, location, property, and gap IDs are provider-report-local;
    all references resolve within the same document.
  - `supported` and `unsupported` findings require at least one referenced
    non-`inference` record. `inference` may explain uncertainty but never
    upgrades a gap into a negative result.
  - Every unknown location, root inquiry, collision, property, or behavior
    references exactly one existing gap; every declared gap must be referenced.
- [ ] **Ruling 4 — Marker state machine**
  - Evaluate the complete freshness predicate before mutating a report. Leave a
    genuinely current report untouched when skipping it.
  - Before research begins, set an existing selected report's
    `contract_validated` to false; create new reports with false.
  - In `success`, run shape validation, the provider-scoped skills checker, and
    the `last_updated == ctx.today` assertion before setting the marker true as
    the final action. Any earlier failure therefore leaves false.
- [ ] **Ruling 5 — Checker and CLI failure contract**
  - Aggregate and deterministically sort semantic findings per provider, return
    non-zero when any selected report is unclean, and preserve roster order in
    fleet output.
  - Continue to use `load_validated_frontmatter`; unreadable, malformed, or
    shape-invalid inputs remain loader/gate failures rather than partially
    checked documents.
  - `--json` serializes the same validation result used by human output; the
    human projection uses `Prose`/`UnorderedList` or another existing
    `TerminalRenderable`, never hand-authored ANSI output.
- [ ] **Ruling 6 — Pilot selection**
  - Use Claude and Codex as required pilots. Select Gemini as the initial
    non-Claude pilot if the probe-readiness spike confirms version-pinned
    evidence for both native and Claude-compatible discovery; otherwise choose
    the first active non-Claude provider that meets that criterion and record
    why.
- [ ] **Ruling 7 — Migration boundary**
  - The checker, schema, fleet prompt, reports, summary prompt/body, and
    published skill copy are in scope.
  - The shared-resource linker, provider precedence, permissions engine,
    provider generator mappings, and generated `data.rs` files are not changed
    from research findings in this fix.

### Spikes

- [ ] **Schema expressiveness**
  - Build a disposable minimal revision-2 report and validate it with
    `md schema validate` to confirm `literal(2; required)`, required arrays,
    nested enum arrays, empty-array behavior, and required non-empty strings.
  - Record which invariants SimplifiedSchema cannot express so the checker owns
    them explicitly; do not duplicate expressible shape rules in Rust.
- [ ] **Lifecycle marker**
  - Exercise a disposable sequence document using `set_frontmatter` to confirm
    atomic write/rehash behavior and determine how a failing `shell` action in
    `success` routes to failure.
  - Verify the ordered stack can keep `contract_validated: false` on schema,
    checker, or date failure without adding a lifecycle expression function.
- [ ] **Checker seam**
  - Trace `steering_check`, `inputs::load_validated_frontmatter`, generator CLI
    dispatch, report rendering, and the Claudine CLI passthrough. Record the
    minimum additive files and existing fixtures that can be reused.
  - GitNexus planning evidence reports `run_steering` as **HIGH** risk through
    the CLI entry flow (one direct caller, three affected processes). Treat the
    new sibling command as additive, but cover dispatch, completion/help, human
    output, JSON stdout, and non-zero exit behavior. The index returned
    `UNKNOWN` for generic `check_provider`/`roster_active_slugs` names; confirm
    those call sites with targeted text search rather than treating zero graph
    callers as safety evidence.
- [ ] **Probe readiness**
  - Load the repository `sniff` and `os` guidance before making host/OS claims.
    Inventory provider versions, source revisions, deterministic catalog
    surfaces, available disposable-test routes, and reachable macOS/Linux/
    native-Windows evidence environments.
  - Select the third pilot under Ruling 6 and record environment gaps rather
    than substituting WSL2 evidence for native Linux or Windows.
- [ ] **Migration inventory**
  - Enumerate active skills reports from `docs/providers.yaml`, excluding only
    `skip_research: true`; do not hard-code the current count.
  - Inventory every downstream skills-summary copy, fleet/prompt regression
    test, Claudine skill reference, and Markdown `hash` field that will need
    regeneration.

### Tasks

- [ ] **Graph impact refresh**
  - Run `just gitnexus`, then upstream impact for the concrete steering-check
    analogs, generator CLI dispatch, report renderer, providers CLI dispatch,
    and validated-frontmatter/roster loaders before editing those symbols.
  - Warn and resolve any HIGH/CRITICAL result; confirm every `UNKNOWN` result
    with text search and record the affected tests/processes.
- [ ] **Baseline capture**
  - Run `just test` and `just lint` from `claudine/`; record pre-existing
    failures separately from implementation regressions.
  - Record current `md schema validate` results for the active skills corpus
    and the current summary/published-copy hash state. These legacy results are
    migration baselines, not revision-2 acceptance evidence.

**Checkpoint:** rulings and spike findings are recorded; the third pilot and
evidence environments are named; impact and test baselines are captured; no
production or research contract behavior has changed.

## Phase 2 — Revision-2 Shape Contract

Establish the complete document vocabulary before implementing relational
rules or rewriting any provider report.

### Tasks

- [ ] **Core report fields**
  - Replace the unversioned sidecar with `schema_revision: literal(2;
    required)` and require `provider`, `created`, `last_updated`, `agent`,
    `model`, `contract_validated`, `changes`, `requires_claudine_update`, and a
    non-empty `reason`.
  - Preserve the existing support, format, portability, CLI-parameter, and
    environment-variable information that remains useful, but remove
    `format.required_fields` and `format.optional_fields`; the typed property
    inventory becomes authoritative.
- [ ] **Root inventory**
  - Define the revision-2 `locations` record with stable ID, OS, environment,
    scope, root family, exact path, support mode, non-empty conditions,
    evidence references, reason, and optional gap.
  - Add `root_coverage` for the exact nine required inquiry IDs and the
    specified statuses, location/evidence references, reasons, and gaps.
  - Preserve separate OS/environment records; do not add `all`, infer native
    support from configured/symlink visibility, or create meaningless synthetic
    locations.
- [ ] **Collision contract**
  - Add `collisions` with the exact identity, stage, activation, outcome,
    winner/survivor/selection, condition, evidence, reason, and gap fields and
    enum vocabularies from R4.
  - Ensure the shape can represent override, coexistence, merge, namespace
    separation, rejection, unknown, and not-applicable without embedding a
    universal precedence assumption.
- [ ] **Evidence and gaps**
  - Add the single top-level evidence registry with all R3 provenance fields
    and the five established methods.
  - Add uniquely identified gap records with area, question, reason, and next
    check. Keep reference integrity and orphan detection in Rust.
- [ ] **Property inventory**
  - Add typed `frontmatter_properties`, `frontmatter_behaviors`, and
    `frontmatter_global_behaviors` records with the exact R6 enums and fields.
  - Add the `name_behavior`, `model_behavior`, and
    `allowed_tools_behavior` summary objects, including evidence, reason, and
    optional gap references.
  - Ensure `value_types` can represent every YAML type and that empty types are
    shape-valid only so the checker can permit them solely for a recorded gap.
- [ ] **Schema examples**
  - Maintain small disposable valid examples for override, coexistence,
    directory-name fallback, required frontmatter name, recognized/ignored
    model, model error/fallback, nonexistent-tool rejection, and mixed-tool
    partial acceptance.
  - Include at least one reasoned unknown/gap example and one wholly
    not-applicable example; these are representation fixtures, not claims about
    a real provider.
- [ ] **Shape validation**
  - Run `md schema validate` against every positive example and targeted
    malformed variants for illegal enum values, missing required fields, and
    wrong field types.

**Checkpoint:** the revision-2 sidecar accepts every required representational
shape and rejects shape-level invalidity; relational-invalid examples remain
shape-valid for Phase 3 to reject.

## Phase 3 — Relational Checker and CLI

Implement the deterministic offline gate and expose it through both binaries.
The checker reads only the roster and repository-local, schema-validated
reports/fixtures.

### Work-group 1 — Checker Core

- [ ] **Checker scaffold** **‖ wg-1**
  - Add `claudine/gen/src/skills_check.rs`, a serializable validation result,
    `check_provider`, and `check_fleet`; export them from `claudine-gen` without
    coupling to `steering_check` internals.
  - Load through `inputs::load_validated_frontmatter` and enumerate fleet scope
    through `roster_active_slugs` so `skip_research` remains authoritative.
- [ ] **Identity rules** **‖ wg-1**
  - Validate revision 2, provider/document-slug equality, unique stable IDs,
    all location/evidence/property/gap foreign keys, and duplicate property
    behavior tuples.
  - Reject missing references and references to the wrong mandatory property.
- [ ] **Root collision rules** **‖ wg-1**
  - Enforce the exact root-coverage inquiry set and supported-location
    requirements.
  - Enforce all ten collision case IDs, stage/activation products for applicable
    cases, location-reference applicability, and outcome-dependent
    winner/survivor/merge fields.
- [ ] **Frontmatter rules** **‖ wg-1**
  - Require unique records for `name`, `description`, `model`, and
    `allowed-tools`, consistent recognition/requirement/type/gap state, and the
    complete R7-R9 behavior-case sets.
  - Require global `unknown_key`; validate name/model/allowed-tools summary
    objects and reject missing, duplicate, or cross-property case coverage.
- [ ] **Evidence gap rules** **‖ wg-1**
  - Require non-inference evidence for concrete support findings, enforce live
    gaps for every unknown, reasons for unknown/not-applicable states, and reject
    every orphan gap.
  - Sort and deduplicate findings so fixture and CLI output are deterministic.

### Work-group 2 — Fixture Suite

- [ ] **Positive fixtures** **‖ wg-2**
  - Add complete override and coexistence provider fixtures plus the required
    frontmatter representation variants from Phase 2.
  - Keep fixtures compact through test builders or focused repository-local
    documents; do not weaken production checks to reduce fixture size.
- [ ] **Negative fixtures** **‖ wg-2**
  - Add one focused failure for duplicate IDs, dangling root/evidence/property
    references, missing required root/collision/property cases, illegal
    outcome-dependent fields, inference-only concrete findings, missing/orphan
    gaps, provider mismatch, legacy revision, duplicate behavior tuples, empty
    unknown types without a gap, and skipped-roster handling.
  - Assert each negative fixture fails for its intended reason so an earlier
    unrelated error cannot make the test pass accidentally.

### Integration Tasks

- [ ] **Generator command**
  - Add `claudine-gen skills check [<slug>] [--json]`, returning non-zero when
    any selected result is unclean and roster-ordered results for fleet scope.
  - Add a skills validation renderer to `report.rs` using existing
    `TerminalRenderable` components; keep raw JSON on stdout.
- [ ] **Claudine command**
  - Add the parallel clap surface under `claudine providers skills`, forwarding
    to `claudine-gen` through the existing installed-binary/dev-checkout seam.
  - Cover help/completion, optional slug, JSON forwarding, stdout/stderr
    separation, clean exit, and failing exit without introducing an interactive
    prompt.
- [ ] **Checker verification**
  - Run focused `claudine-gen` unit/integration tests and Claudine CLI command
    tests, then `just test` in the package area.

**Checkpoint:** all positive fixtures pass both gates; every negative fixture
fails for the named relation; provider and fleet human/JSON commands return the
correct status and channel behavior.

## Phase 4 — Fleet Lifecycle and Research Prompt

Update the shipped fleet as one coherent orchestration contract before running
live research.

### Work-group 1 — Prompt Contract

- [ ] **Research instructions** **‖ wg-1**
  - Rewrite scope, questions, deliverables, frontmatter instructions, body
    structure, and exit criteria to cover scope versus root family, exact OS/
    environment evidence, identity, collision stages, activation modes,
    description budget, disabling aliases, and every R7-R9 behavior case.
  - Require root, collision, and frontmatter-behavior prose tables that agree
    with structured frontmatter; cross-reference slash-command syntax and
    model/model-config research instead of duplicating those catalogs.
- [ ] **Neutral examples** **‖ wg-1**
  - Replace the repository-over-user example bias with verified-shape examples
    demonstrating both override and coexistence.
  - Demonstrate supported, unsupported, unknown-with-gap, and not-applicable
    states without presenting any as a universal provider default.

### Work-group 2 — Lifecycle State Machine

- [ ] **Freshness predicate** **‖ wg-2**
  - Replace the date-only branches with the single predicate: file exists,
    body non-empty, revision 2, date within fourteen days,
    `contract_validated == true`, and `validate_schema(file)` passes.
  - Route every other state into research, including recent legacy/invalid
    documents.
- [ ] **Marker transitions** **‖ wg-2**
  - Set the existing selected report's marker false before agent work; make the
    new-document template start false. Use the existing `set_frontmatter`
    effect so writes stay atomic and hashes update automatically.
  - Order the success stack as shape gate, provider-scoped relational command,
    date assertion, then marker true and success messaging. Ensure failure and
    interruption cannot reach the final mutation.

### Integration Tasks

- [ ] **Passive prompt guard**
  - Add a shipped-prompt regression that parses/composes `_fleet.md`, checks the
    required vocabulary/cases and lifecycle ordering, uses child-only
    `PLAYA_DRY_RUN=1` with a private spool, and asserts no audio publication.
- [ ] **Sequence lifecycle tests**
  - Through the normal composition/sequence invocation path and
    `CliProcessFixture`, cover: recent revision 1 refreshes; recent valid
    revision 2 skips; false/missing marker refreshes; empty body refreshes;
    schema failure stays false; relational failure stays false; stale date
    stays false; full success sets true last; next run retries a failed report.
  - Use isolated fake providers and disposable report/roster trees. Never call
    live providers or inherit ambient repository/home state.
- [ ] **Migration-red assertion**
  - Run the fleet checker after installing the contract and confirm legacy
    active reports are reported as failures. Record this as expected migration
    state, not a green checkpoint or reason to relax the checker.

**Checkpoint:** the shipped fleet prompt passes passive and isolated lifecycle
tests; every invalid/current/stale state routes correctly; the full real corpus
is expectedly red only because provider reports have not yet been migrated.

## Phase 5 — Pilot Research and Contract Ratification

Run three pilots through the normal fleet route, using the new contract and
marker gates. These reports test whether the vocabulary captures real provider
differences before the remaining fleet is rewritten.

### Work-group 1 — Provider Pilots

- [ ] **Claude pilot** **‖ wg-1**
  - Refresh Claude with isolated root/collision fixtures and versioned evidence;
    distinguish native discovery from linked/symlink visibility and discovery
    from activation/execution.
  - Cover name fallback/mismatch, model selection failures/fallbacks, and
    allowed-tools semantics without extrapolating across OSes.
- [ ] **Codex pilot** **‖ wg-1**
  - Refresh Codex independently, testing user/repo `.agents/skills`, branded
    roots, collision identity, selector coexistence versus effective activation,
    configured/managed/plugin sources, and frontmatter error stages.
  - Cross-reference current agent-models/model-config findings and record model
    identifier portability limits rather than copying a model catalog.
- [ ] **Non-Claude pilot** **‖ wg-1**
  - Refresh the Phase-1-selected provider, explicitly comparing its native,
    Claude-compatible, and shared roots at both user and repository scope where
    meaningful.
  - Use reproducible observations or pinned source/docs for collision and
    frontmatter behavior; retain honest gaps where deterministic activation
    cannot be established.

### Ratification Tasks

- [ ] **Vocabulary review**
  - Compare the three reports for facts the schema cannot express, enum values
    being overloaded, contradictory interpretations of stage/activation, and
    evidence/gap friction.
  - Make only justified schema/checker/prompt corrections, migrate all three
    pilots together, and add regression fixtures for each correction. Do not
    introduce provider-specific fields when `conditions`, `selection`, or
    explanatory text already represent the distinction.
- [ ] **Pilot validation**
  - For each pilot, run `md schema validate`, provider-scoped skills check, and
    prose-versus-structure review; confirm `contract_validated: true` was set
    only by the final fleet action.
  - Verify harmless markers identify which duplicate was discovered and which
    body executed; repeated implicit probes must remain `unknown` unless they
    establish a deterministic rule.
- [ ] **Contract freeze**
  - Freeze revision-2 vocabulary after the pilots. Any later discovery that
    truly cannot be represented requires an explicit return to this checkpoint
    and migration of already-completed pilots, never an ad hoc per-report key.

**Checkpoint:** all three pilots pass both gates, show materially different
behavior without schema contortions, and have attributable evidence or explicit
gaps for every required case.

## Phase 6 — Fleet Refresh, Summary, and Publication

Migrate every remaining active provider after the pilot vocabulary is frozen,
then regenerate downstream comparison artifacts from the green corpus.

### Work-group 1 — Remaining Providers

- [ ] **Roster refresh** **‖ wg-1**
  - Enumerate all active providers from `docs/providers.yaml`, subtract the
    three validated pilots, and refresh the remainder through the normal fleet
    route. Provider tasks may run concurrently because reports and disposable
    probe homes are isolated.
  - Preserve each report's `created`, set `last_updated` to the research date,
    document `changes`, and retain every honest unknown/gap. Never revive or
    validate a `skip_research: true` entry.
- [ ] **Environment evidence** **‖ wg-1**
  - Preserve macOS, Linux, native Windows, WSL2-Linux, and Windows-from-WSL2 as
    distinct evidence. Use documentation/source as `not_applicable` only when
    genuinely platform-independent.
  - Do not report configured paths or symlinks as native discovery, absence from
    docs as unsupported, catalog visibility as execution, or a single model
    choice as deterministic implicit activation.

### Integration Tasks

- [ ] **Full corpus gate**
  - Run `md schema validate` for every active report and
    `claudine providers skills check` with no slug. Resolve every finding in the
    responsible report, checker, or contract; do not whitelist a provider.
  - Audit prose tables against structured locations, root coverage, collisions,
    frontmatter properties/behaviors, evidence, gaps, and update verdict.
- [ ] **Summary instructions**
  - Update `docs/research/summary/agent-skills.md` generation instructions to
    consume only active provider reports and explicitly synthesize scope,
    root-family, collision, frontmatter, and evidence-gap comparisons.
  - Require separate user/repo `.agents/skills` findings, name requiredness/
    fallback, `model` and `allowed-tools` recognition/invalid-value behavior,
    and the effects of linked duplicate catalogs or unusable identifiers.
- [ ] **Summary regeneration**
  - Regenerate the comparison through its existing `claudine sequence`
    workflow; verify every comparative claim resolves to refreshed evidence and
    that unknown/not-applicable results remain visible.
  - Preserve/update the summary's Darkmatter hash with `md hash`, never a custom
    hashing implementation.
- [ ] **Skill publication**
  - Run `just publish-summary-research` from `claudine/` and verify the published
    `.claude/skills/claudine/summaries/agent-skills.md` copy is byte-current.
  - Update the Claudine skill's research guidance for the revision-2 contract,
    skills checker, freshness marker, and pilot/fleet workflow; do not make the
    published copy a second source of truth.

**Checkpoint:** every active provider is revision 2 and green; the fleet-level
checker has no omissions; summary source, generated body/hash, and published
skill copy agree.

## Phase 7 — Acceptance and Review Handoff

Run the complete verification matrix, audit scope boundaries, and leave the fix
implementation-complete for the author to review.

### Work-group 1 — Automated Validation

- [ ] **Focused gates** **‖ wg-1**
  - Run all `skills_check` unit/integration fixtures, generator CLI human/JSON
    tests, Claudine passthrough tests, passive fleet-prompt tests, and isolated
    sequence lifecycle tests.
  - Re-run every acceptance mutation: missing case, dangling reference, illegal
    outcome, inference-only concrete support, missing/orphan gap, duplicate ID/
    behavior tuple, provider mismatch, legacy revision, and skipped provider.
- [ ] **Package gates** **‖ wg-1**
  - Run `just test`, `just test-l2`, and `just lint` in `claudine/`.
  - Run any applicable docs/schema/corpus recipes discovered in Phase 1 and the
    full active fleet's `md schema validate` plus skills checker.
- [ ] **Cross-platform compile** **‖ wg-1**
  - Run the package's Windows compile check where available and review all new
    filesystem/process code for macOS, Linux, native Windows, and WSL2
    portability. The checker itself must remain offline and path-neutral.

### Work-group 2 — Content and Boundary Audit

- [ ] **Evidence audit** **‖ wg-2**
  - Verify versions/revisions, observed dates, modes, OS/environment labels,
    configuration, claims, limitations, evidence methods, and local artifact
    references across the final corpus.
  - Confirm live probes were non-interactive, harmless, isolated, silent, and
    did not focus terminal/browser windows.
- [ ] **Scope audit** **‖ wg-2**
  - Confirm no runtime linker, provider precedence, permissions, provider
    generator mapping, or generated provider data changed because of a research
    conclusion.
  - Confirm every requested runtime follow-up is expressed only through
    `requires_claudine_update: true` and a concrete reason suitable for a
    separately scoped issue/fix.
- [ ] **Downstream drift** **‖ wg-2**
  - Re-run `just publish-summary-research` and require no content drift.
  - Check Markdown hashes through Darkmatter and ensure docs and the Claudine
    skill describe the implemented command/workflow without stale revision-1
    guidance.

### Final Tasks

- [ ] **Graph change review**
  - Run GitNexus `detect_changes --scope all`; re-run if partial or truncated,
    and review every affected CLI/generator process before handoff.
- [ ] **Validation record**
  - Record commands, results, provider versions/source revisions, environment
    coverage, accepted unknowns, and remaining evidence gaps in the
    implementation log or review evidence.
- [ ] **Review handoff**
  - Review the diff for unrelated edits and comment/doc drift, especially
    around the fleet freshness definition and CLI help.
  - Leave the fix in its active directory with status “implementation complete,
    ready for review.” The author, not the implementation agent, runs the
    completion workflow and moves it to `_completed` after review closes.

**Checkpoint:** every acceptance criterion in `spec.md` has linked evidence;
all required gates pass; GitNexus change analysis is complete; no stale
published copy or out-of-scope runtime behavior remains.
