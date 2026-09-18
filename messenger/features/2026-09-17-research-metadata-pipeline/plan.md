---
total_phases: 8
created: 2026-09-17
phase: 1
agent: codex/default
yolo: true
source_files_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/.gitignore
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/Cargo.toml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/fault.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/fsutil.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/generations.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/journal.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/lib.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/model.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/common/mod.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/generations_interruption.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/journal_interruption.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/open_handles.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_budget.py
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_tree_windows.py
docs_updated_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/findings.md
docs_created_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/architecture.md
    - messenger/features/2026-09-17-research-metadata-pipeline/fixture-matrix.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/surface-inventory/inventory.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/findings.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/findings.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe-results.json
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_schema.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_types.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_mappings.schema.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_mappings.types.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/mappings.pilot.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/discord.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/telegram.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/slack.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/signal.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/negative/
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
source_files_during_phase_2:
    - messenger/lib/Cargo.toml
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/fixtures/research/contract/
    - messenger/lib/tests/fixtures/research/interaction/
    - messenger/lib/tests/fixtures/research/diagnostics/
    - messenger/lib/tests/fixtures/research/negative/schema/
    - messenger/lib/tests/fixtures/research/negative/semantic/
    - messenger/docs/platforms.yaml
    - messenger/docs/platforms.schema.yaml
    - messenger/docs/research/platforms/_schema.yaml
    - messenger/docs/research/platforms/_types.yaml
    - messenger/docs/research/platforms/_overrides.schema.yaml
    - messenger/docs/research/implementation/_schema.yaml
docs_updated_during_phase_2:
    - messenger/docs/research/platforms/discord.md
    - messenger/docs/research/platforms/slack.md
    - messenger/docs/research/platforms/telegram.md
    - messenger/docs/research/platforms/whatsapp.md
    - messenger/docs/research/platforms/signal.md
    - docs/dependencies.md
docs_created_during_phase_2:
    - messenger/docs/research/platforms/_fleet.md
    - messenger/docs/research/platforms/_rules.md
    - messenger/lib/tests/fixtures/research/README.md
skills_files_updated_during_phase_2:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
packages:
    - messenger
---

# Provider Research Metadata Pipeline Implementation Plan

## Summary and Success

Build an offline-first, review-gated knowledge pipeline for Discord, Slack,
Telegram, WhatsApp, and Signal without changing Messenger's delivery behavior.
The implementation adds a roster and shared research contract, feature-gated
typed loading and semantic validation in `messenger`, deterministic maintenance
commands in `messenger-cli`, and a narrow shared-budget extension to Claudine's
existing sequence runner. Research candidates remain isolated until validation,
delta review, and the applicable human-approval rule permit promotion.

Completion means all five accepted platform documents cover the seven current
chat adapters and any required research-only companion interfaces; unknown and
conflicting facts remain explicit; identical inputs generate byte-identical
catalog and report artifacts; interrupted work cannot corrupt the last accepted
snapshot; implementation assessments are fingerprinted and review-gated; and
the truncation, diagnostics, formatting/media, attribution/context, and
interactivity handoffs are complete. The ordinary send path remains independent
of Darkmatter, Claudine, network access, research state, and generated catalog
freshness. Tests use local fixtures and fake workers only—no credentials, live
messages, focused terminal/browser windows, or automatic research runs.

The dependency order is:

```text
Phase 1 decisions and spikes
    -> Phase 2 authored contract and fixtures
        -> Phase 3 typed loading and validation
            -> Phase 4 deterministic projections and publication
                -> Phase 6 refresh/review lifecycle
Phase 1 orchestration findings
    -> Phase 5 Claudine shared budgeting
        -> Phase 6 refresh/review lifecycle
Phase 6
    -> Phase 7 reviewed baseline
        -> Phase 8 publication and cross-platform verification
```

Waves are globally numbered. Tasks inside the same wave may be assigned to
subagents concurrently only where their file ownership is disjoint; waves run in
numeric order unless a task explicitly names a narrower dependency.

## Phase 1 — Resolve Architecture and Risks

### Necessary Rules

- The implementation must preserve three authorities: the roster owns identity
  and coverage, accepted research owns external facts, and checked-out adapter
  code/tests plus reviewed mappings own implementation status. No projection may
  rewrite `CapabilitySet` or runtime delivery behavior.
- Put typed research support in a feature-gated `messenger::research` module and
  enable it only for maintenance consumers. Keep Darkmatter/schema and YAML
  dependencies out of ordinary Messenger send builds; do not add Claudine as a
  Messenger dependency.
- Resolve every authored file reference with `biscuit_file::FileReference` and a
  captured resolution context. Store repository-facing provenance with portable,
  repository-relative path spelling.
- Treat accepted documents and generated output as immutable inputs during a
  run. Candidates, failures, rejected reviews, and routine-renewal records live
  in a gitignored per-worktree state area; durable evidence reviews for accepted
  changes live under `messenger/docs/research/reviews/`. Finalize exact paths in
  the architecture record before code relies on them.
- Select and prove a cross-platform snapshot protocol before implementing
  promotion. Readers must either observe the complete previous snapshot or the
  complete next snapshot; a crash may require recovery, but must not make an
  incomplete mixture eligible for generation or reporting.
- A live fleet run is forbidden until an operator supplies positive per-platform
  elapsed-time and invocation limits. There are no defaults. One platform runs
  at a time, all passes/review/recovery share one retained budget, and resumption
  never resets consumed allowance.
- Initial baselines and every substantive change require human approval.
  Automatic acceptance is limited to verified unchanged renewals satisfying the
  specification's complete equality and successful-source-recheck rule.
- Keep the required public maintenance interface (`validate`, `generate`,
  `generate --check`, and `report`) stable. Decide during this phase whether
  promotion, cleanup, and run inspection need additional explicit subcommands or
  remain package-area recipes over library operations; do not hide lifecycle
  mutations inside `generate`.

### Wave 1 — Parallel Discovery

- [x] **Surface Inventory**
  - Inventory the payload surfaces, operations, interface identities, response
    handling, receipts, and current tests for Discord bot/webhook, Slack Web
    API/webhook, Telegram Bot API, WhatsApp Cloud API, and signal-cli JSON-RPC.
  - Record stable adapter IDs and the precise code/test inputs whose hashes will
    govern implementation-assessment reuse; include `MessageBody` rendering,
    attachment/location fallback, replies, delivery controls, and error loss.
  - Cross-check the Discord truncation spec so its field mapping becomes a
    regression input, not a generalized policy.

- [x] **Schema Pilots**
  - Draft minimal Discord and Telegram records that exercise individual and
    aggregate constraints, different measurement stages, parse modes, images,
    errors, and version applicability.
  - Add a Slack structured-surface sample and a Signal uncertainty/bridge sample
    before fixing vocabularies; include send-only and companion-receiver
    interactivity examples.
  - Prove Darkmatter SimplifiedSchema can express closed records, named reusable
    types, required arrays, and the composition-property allowlist. Log any
    semantic rules that necessarily belong in Rust rather than the schema.

- [x] **Publication Spike**
  - Prototype at least two snapshot strategies against the fixed artifact paths
    and choose one that supports atomic selection, input/schema hashes, rollback,
    and startup recovery without symlinks or Unix-only rename assumptions.
  - Exercise interruption before staging, during staging, before selection, and
    after selection on local fixtures. Document the Windows replacement and
    open-handle behavior the chosen protocol requires.

- [x] **Budget Spike**
  - Extend the existing orchestration findings with production-code probes for
    independent discovery input isolation, inline-compose, process-tree
    cancellation, and persistent active-time/invocation accounting.
  - Use fake providers to exercise restart, retry, explicit suspension, automatic
    wait/backoff, and exhaustion. Separate local-child termination evidence from
    claims about remote model cancellation or billing.
  - Check available `BUILD_*` hosts and capture macOS, Linux, native-Windows, and
    WSL2 evidence where available; record evidence gaps without weakening the
    portability contract.

### Wave 2 — Architecture Gate

- [x] **Architecture Record**
  - Convert Wave 1 findings into a short design record covering module ownership,
    the maintenance feature graph, schema/type boundaries, candidate and review
    paths, snapshot selection/recovery, CLI/recipe surfaces, budget ledger, and
    cleanup protection rules.
  - Define stable IDs, schema-version compatibility, accepted-snapshot manifest
    fields, relevant-input fingerprinting, and which outputs are authored versus
    generated.
  - Keep Messenger's coordinator limited to domain validation/approval and
    Claudine's extension limited to launches, cancellation, and shared accounting.

- [x] **Fixture Matrix**
  - Map all 37 acceptance criteria from the specification to named unit,
    integration, corpus, or fake-worker fixtures and to the phase that lands each
    one.
  - Identify feature combinations needed beyond `just test`, including all five
    chat providers and the maintenance feature, without adding speculative CI
    cells.

### Phase 1 Checkpoint

- [ ] Confirm the architecture record resolves every item under the spec's
  “Implementation Planning and Verification Items,” and obtain maintainer review
  for any choice that changes artifact layout or adds lifecycle commands.
- [x] Confirm the pilots cover aggregate limits, explicit uncertainty, structured
  surfaces, executable error signatures, interactivity, and implementation
  fingerprints before expanding the schema.

## Phase 2 — Author Contract and Corpus

### Wave 3 — Contract Core

- [x] **Roster Contract**
  - Create `messenger/docs/platforms.yaml` with exactly five active platforms,
    seven implemented sending interfaces, explicit research-only companion
    interfaces, output filenames, identification URLs, curated source records,
    refresh status/interval, and paused/excluded subjects.
  - Enforce the initial per-platform curated-source cap of 10 while leaving
    discovery and evidence citations uncapped.

- [x] **Metadata Schema**
  - Create `messenger/docs/research/platforms/_schema.yaml` as a versioned,
    closed SimplifiedSchema with reusable named types and the complete vocabulary
    for identity/evidence, versions, constraints, formatting/text bindings,
    media, attribution/location/expression, interactivity, eligibility/rates,
    errors, receipts, gaps, changes, and implementation mappings.
  - Encode structural state/value rules where SimplifiedSchema can do so and list
    the remaining deterministic semantic rules beside their Rust owners.

- [x] **Fleet Prompt**
  - Replace the five duplicated inline prompts with
    `messenger/docs/research/platforms/_fleet.md` implementing independent
    discovery, curated reconciliation, and source-list maintenance.
  - Preserve single-document inline-compose use by delegating each document's
    prompt to the shared instructions/schema. Discovery receives no previous
    prose or curated links; reconciliation receives both plus discovery output.

### Wave 4 — Negative Corpus

- [x] **Contract Fixtures**
  - Commit sanitized positive and negative fixtures for all required constraint
    units/stages, knowledge states, API/SDK/bridge version distinctions, source
    kinds, applicability conditions, and image/format/attribution/location/effect
    mappings.
  - Cover malformed/missing/duplicate identities, dangling evidence and fact
    references, incompatible aggregates, unknown properties/enums, stale
    overrides, ambiguous applicability, and incomplete investigated gaps.

- [x] **Interaction Fixtures**
  - Add fixtures for send-only, callback-only, conditional inbound visibility,
    companion-interface combinations, all four `QuestionKind` values, canceled
    versus empty answers, stale option IDs, correlation, and anonymous results.
  - Add form fixtures for atomic multi-field submission, independent controls,
    sequential questions, external forms, partial/absent/canceled values, and
    lifecycle deadlines.

- [x] **Diagnostic Fixtures**
  - Add sanitized envelopes and signatures for HTTP-success application errors,
    plain text, nested fields, local validation, success warnings, overlaps,
    unknown codes, ambiguous timeouts, and changed/missing envelope fields.
  - Prove fixtures contain no credentials, recipient IDs, message bodies, raw
    transcripts, or unbounded terminal-control-bearing provider text.

### Phase 2 Checkpoint

- [x] Run `md schema validate` over every pilot and corpus document and verify the
  schema is passive: no expression execution, process spawn, remote fetch, or
  document mutation.
- [x] Freeze schema version 1 only after Discord, Telegram, Slack, and Signal
  pilots all pass and every required category can express an investigated gap.

## Phase 3 — Implement Typed Validation

### Wave 5 — Typed Foundation

- [ ] **Research Types**
  - Add feature-gated `messenger/lib/src/research/` types matching schema version
    1 exactly, using closed enums and deterministic maps/sets where ordering is
    observable.
  - Separate authored records, validated records, executable projections,
    implementation assessments, diagnostics, and generated catalog DTOs so an
    unknown or conflict cannot accidentally inhabit an executable type.
  - Add typed errors carrying repository-relative paths, stable IDs, and precise
    causes without exposing secrets or raw response bodies.

- [ ] **Passive Loader**
  - Load roster, Markdown frontmatter, schema, optional overrides, fixtures, and
    manifests through captured file-resolution context; reject unsupported schema
    versions and unknown top-level properties outside the explicit composition
    allowlist.
  - Invoke Darkmatter's library schema validation passively, then deserialize the
    same vocabulary. Never shell out to `md` from the library.

### Wave 6 — Parallel Semantic Rules

- [ ] **Identity Validation**
  - Enforce active-roster coverage, platform/interface/adapter identity,
    operation scope, unique logical keys, source references, chronology,
    stable/preview distinctions, evidence dates/revisions, and required category
    coverage.
  - Distinguish established unversioned APIs from unresearched versioning and
    preserve chronology entries across refreshes.

- [ ] **Constraint Validation**
  - Enforce knowledge-state/value consistency, nonnegative and lower-bound rules,
    aggregate membership/unit compatibility, bounded typed conditions,
    simultaneous bounds, rendering/measurement stages, and explicit gaps.
  - Derive eligibility only for known facts with resolved evidence, units,
    stages, applicability, and consumer mappings; retain every ineligible fact in
    reports with reasons.

- [ ] **Binding Validation**
  - Validate format constructs and fixtures, text relationships/precedence,
    image role coverage and shared budgets, attachment/addressing/receipt facts,
    attribution controls, location subject/origin/inclusion, and expression
    support/fidelity.
  - Reject provider-controlled placement as caller control and app-only behavior
    as API support.

- [ ] **Interaction Validation**
  - Validate companion-interface relationships, inbound mechanisms and payload
    locators, typed options/answers, form identities/submissions, correlation,
    lifecycle units, acknowledgments, and conditional recipient scope.
  - Prevent callback-only reception, platform-level capability, sequential
    controls, or free-text interpretation from being promoted to stronger native
    capabilities.

- [ ] **Error Validation**
  - Validate envelope locators, signature predicate types, specificity ordering,
    overlap rejection, related facts, delivery certainty, remediation evidence,
    retry candidacy, and replay-safety prerequisites.
  - Replay executable signatures against positive, negative, and near-miss
    fixtures; unknown or ambiguous matches stay unknown.

### Wave 7 — Validation Assembly

- [ ] **Coverage Engine**
  - Assemble schema and semantic findings into deterministic, stable-order
    diagnostics and platform/interface/category completeness summaries.
  - Validate overrides against source/schema hashes and reject an override whose
    target or justification is stale.

- [ ] **Mapping Assessment**
  - Validate proposed versus accepted adapter mappings, code/test references,
    inspected revision, and relevant-input fingerprints.
  - Reuse accepted assessments only on fingerprint equality; otherwise emit
    `unassessed` and a structured `requires_messenger_update` gap without changing
    runtime capabilities.

- [ ] **Validator Tests**
  - Turn the Phase 2 corpus into table-driven unit and integration tests,
    including Unicode scalar/UTF-8/UTF-16/grapheme/parsed-markup distinctions and
    all specification rejection cases.
  - Prove stale but structurally valid research remains inspectable and that a
    missing platform/interface/category never means unrestricted support.

### Phase 3 Checkpoint

- [ ] Run `just test` and `just lint` in `messenger/`, plus explicit no-default,
  maintenance-feature, and all-chat-provider checks established in Phase 1.
- [ ] Confirm a build without the maintenance feature has no Darkmatter,
  Claudine, research-workspace, or network-refresh dependency in its normal send
  path.

## Phase 4 — Build Deterministic Consumers

### Wave 8 — Projection Engines

- [ ] **Catalog Projection**
  - Project all validated facts, explicit unknowns/conflicts, provenance,
    freshness, executable eligibility, and implementation gaps into a stable
    `catalog.json` model.
  - Sort every collection by documented stable keys, normalize repository paths,
    omit wall-clock generation timestamps, and bind output to schema/input hashes.

- [ ] **Delta Engine**
  - Produce a full fact-level before/after comparison and fixed flags for removed
    constraints, raised limits, support reversals, changed units, conflicts, and
    unmappable values.
  - Compare supporting evidence and substantive prose separately from typed
    values, preserve initial-baseline status, and keep mechanical conclusions
    distinct from independent-agent review.

- [ ] **Report Models**
  - Build filterable report models for constraints, formatting/text/image
    bindings, attribution/location/effects, inbound/questions/forms,
    errors/recovery, eligibility/freshness, capabilities, and implementation gaps.
  - Include the seven-adapter truncation and diagnostic handoffs with an
    enforceability reason for every emitted surface.

### Wave 9 — Safe Publication

- [ ] **Snapshot Writer**
  - Implement the Phase 1 snapshot protocol with staged validation, durable
    manifest/hash checks, atomic selection, rollback/recovery, and preservation of
    the previous usable snapshot on every error path.
  - Support compatible prior accepted documents for failed platform refreshes,
    while rejecting missing initial baselines, schema incompatibility, or
    insufficient required coverage.

- [ ] **Artifact Writers**
  - Generate the catalog and machine-derived comparison tables without editing
    authored prose. Validate every input and all cross-artifact references before
    selecting the new snapshot.
  - Add byte-for-byte drift checking and interruption fault injection around each
    publication stage.

### Wave 10 — Maintenance CLI

- [ ] **Research Commands**
  - Add `messenger research validate`, `generate`, `generate --check`, and
    `report` with platform/interface/operation filters and JSON output.
  - Keep machine output escape-free on stdout; render human reports and
    diagnostics with `TerminalRenderable` components such as `Prose`, `Table`,
    and `UnorderedList`.
  - Expose any Phase 1-approved promotion/inspection operations explicitly and
    keep agentic refresh separate from deterministic generation.

- [ ] **CLI Tests**
  - Test help, filters, JSON schema, exit codes, drift detection, stale reporting,
    invalid inputs, interrupted generation, and identical double generation in
    isolated fixtures.
  - Assert two runs from identical inputs are byte-identical and invalid inputs
    leave the selected snapshot unchanged.

### Phase 4 Checkpoint

- [ ] Run Messenger L1/lint and the explicit maintenance/all-provider feature
  matrix; run `messenger research generate` twice and `generate --check` against
  the fixture corpus.
- [ ] Manually inspect terminal output at narrow and normal widths and verify JSON
  stdout contains no ANSI/OSC escapes or human status lines.

## Phase 5 — Add Shared Claudine Budgets

### Wave 11 — Budget Core

- [ ] **Budget Ledger**
  - Add a narrow sequence-run budget model to `claudine-cli` with required
    positive elapsed-time and invocation limits, a stable run/platform identity,
    retained counters, active/suspended/exhausted state, and crash-safe writes.
  - Charge research, fetch, validation, review, orchestration, and automatic
    waits/backoff; stop charging only for explicitly persisted human/operator
    suspension. On uncertain crash intervals, preserve a conservative consumed
    allowance rather than refunding it.

- [ ] **Launch Accounting**
  - Debit every agent launch before dispatch, including each pass, independent
    reviewer, retry, restart, and recovery attempt. Refuse dispatch when either
    limit is exhausted and never let the two-recovery cap grant extra budget.
  - Pass discovery and reconciliation inputs through separate prepared artifacts
    so previous prose/curated links cannot leak into independent discovery.

### Wave 12 — Cancellation and Recovery

- [ ] **Deadline Enforcement**
  - Apply the shared remaining deadline to active orchestration and child waits,
    reusing Claudine's process termination coordinator on Unix and Windows.
  - Attempt to stop in-flight local work at exhaustion, record cancellation
    outcome and possible remote continuation, and preserve the incomplete stage,
    candidate, stop reason, and consumed budget.

- [ ] **Budget Fixtures**
  - Add fake-provider tests for every charged activity, explicit suspension,
    backoff, exhaustion before/during launch, retry, restart, crash recovery,
    operator-granted additional budget, and resumable candidates.
  - Verify one-platform-at-a-time scheduling, no silent refunds/resets, no
    relabeling exhausted work as an investigated unknown, and no host credentials,
    ambient repo state, audio, or focused windows.

### Phase 5 Checkpoint

- [ ] Run `just test` and `just lint` in `claudine/`, plus the relevant sequence
  and process-termination tests on macOS and each available cross-platform build
  host.
- [ ] Confirm existing `claudine sequence` behavior is unchanged when no research
  budget configuration is supplied, while the Messenger fleet rejects missing
  limits before launching a worker.

## Phase 6 — Implement Refresh and Review

### Wave 13 — Candidate Lifecycle

- [ ] **Refresh Selection**
  - Select missing, expired, forced, schema-invalid, prompt/schema-changed, or
    relevant-version-changed items from the roster; skip accepted current items
    with an auditable reason.
  - Preserve `created`, stable IDs, actual source check dates, and prior chronology;
    never treat an agent exit code or timestamp-only edit as success.

- [ ] **Three Passes**
  - Wire the shared fleet through Claudine's budgeted sequence: isolated discovery,
    curated reconciliation, then capped source-list proposals.
  - Require contribution notes for suggested/retained sources, an access-attempt
    record for every curated source, accountable investigated gaps, and human
    approval for every curated-list change.

- [ ] **Candidate Storage**
  - Persist isolated per-platform candidates, stage results, sanitized fixtures,
    source-check results, and resume metadata in the Phase 1 local state area.
  - Protect active and awaiting-review runs; never overwrite accepted platform
    documents or refresh their dates on failure/exhaustion.

### Wave 14 — Review Gates

- [ ] **Delta Review**
  - Run structural and semantic validation, deterministic delta, suspicious flags,
    and an independently launched evidence reviewer after reconciliation and
    before promotion.
  - Review changed evidence even at the same URL and meaningful prose changes
    even when typed values match; unresolved evidence remains unresolved.

- [ ] **Approval Policy**
  - Enforce human approval for the initial baseline and all substantive fact, gap,
    applicability, evidence, schema, curated-source, or prose changes.
  - Implement automatic unchanged renewal only when all substantive elements are
    identical, the same sources were successfully rechecked, and validation and
    publication consistency checks pass.

- [ ] **Independent Promotion**
  - Promote successful platforms independently and combine them only with prior
    accepted documents that satisfy the current schema and coverage.
  - Store concise accepted change summaries in the CHANGELOG, durable structured
    evidence reviews under the chosen review path, and routine unchanged-renewal
    maintenance records locally without false CHANGELOG entries.

### Wave 15 — Retention Controls

- [ ] **Cleanup Preview**
  - Add exact, dry-run-first cleanup reporting for local candidate, rejected,
    failed, and routine-renewal records older than the configured threshold
    (initially 30 days), including the resumability that would be lost.
  - Require an explicit cleanup action; protect active runs, awaiting-review
    candidates, and all evidence needed by accepted research. Cleanup never commits
    or publishes Git changes.

- [ ] **Lifecycle Tests**
  - Cover missing/unchanged/malformed/contradictory output, bounded recovery,
    partial failure, source inaccessibility, unchanged renewals, substantive
    changes, initial-baseline rejection, independent platform publication, and
    interrupted promotion with fake agents/transports.
  - Verify discovery suggestions cannot become evidence/curated sources merely by
    appearing, and proposed implementation assessments cannot become accepted
    claims without review.

### Phase 6 Checkpoint

- [ ] Replay the full lifecycle from an empty fixture and from a partially failed
  refresh; verify the accepted snapshot remains internally consistent at every
  injected interruption.
- [ ] Review stored artifacts for secrets, raw transcripts, wholesale social
  threads, message content, host-specific paths, and inaccurate freshness dates.

## Phase 7 — Establish Reviewed Baseline

Platforms are intentionally serialized in this phase because the specification
requires one platform at a time initially. Each wave uses explicit operator
limits, all three passes, validation, delta/evidence review, and human approval;
failure preserves the previous accepted state and does not block retrying that
platform.

### Wave 16 — Discord Baseline

- [ ] **Discord Research**
  - Migrate useful prose in place and complete bot/webhook records for content,
    summaries/embeds, aggregate fields, attachments/images, SDK versus HTTP
    validation, errors/warnings, receipts, versions, attribution/context,
    delivery controls, and companion interactivity.
  - Reconcile every applicable Discord truncation finding and explicitly classify
    unsupported, unknown, and unimplemented surfaces.

### Wave 17 — Telegram Baseline

- [ ] **Telegram Research**
  - Complete Bot API records for text/captions, modes/entities, parsed versus
    encoded measurement, media groups, hosted/local differences, location,
    effects, errors, versions, inbound/update routes, questions, and forms.
  - Preserve ambiguity where entity-offset units do not establish length units.

### Wave 18 — Slack Baseline

- [ ] **Slack Research**
  - Complete Web API and incoming-webhook records, keeping recommendations,
    service truncation, blocks, notification/accessibility fallbacks, warnings,
    receipts, images, author overrides, interactivity, and interface differences
    distinct.
  - Exercise structured surfaces and companion Events API/Socket Mode/interaction
    combinations without treating the outgoing webhook as bidirectional.

### Wave 19 — WhatsApp Baseline

- [ ] **WhatsApp Research**
  - Complete Cloud API records for free-form and template text, captions/media,
    versions, conversation windows and template eligibility, identity/location,
    errors/status callbacks, questions, lists/buttons, and form alternatives.
  - Keep API-version conditions and business/account prerequisites explicit and
    retain research-only capabilities as implementation gaps.

### Wave 20 — Signal Baseline

- [ ] **Signal Research**
  - Complete signal-cli JSON-RPC and relevant service records, separating bridge
    releases and validation/transformation from service and recipient-client
    behavior.
  - Record explicit investigated gaps where public evidence cannot establish a
    limit, error, receipt, interactivity, or service contract; do not substitute a
    REST wrapper's behavior for Messenger's actual interface.

### Phase 7 Checkpoint

- [ ] Obtain human approval for all five initial accepted documents, curated
  source lists, implementation assessments, evidence reviews, and handoff gaps.
- [ ] Run validation and generation over the complete accepted fleet; confirm all
  five platforms, seven adapters, required research-only companions, categories,
  handoffs, and version chronologies are covered by evidence or investigated gaps.

## Phase 8 — Publish and Verify

### Wave 21 — Documentation Publication

- [ ] **Generated Summary**
  - Generate `messenger/docs/research/platforms/catalog.json` and the
    machine-derived portions of `messenger/docs/research/summary/platforms.md`,
    linking every comparison and handoff to stable fact/evidence IDs.
  - Publish the truncation, formatting/image, attribution/context, interactivity,
    and diagnostic handoffs, including unresolved questions per adapter.

- [ ] **Skill Projection**
  - Generate `.claude/skills/messenger/platform-metadata.md` as a compact accepted
    projection with links to detailed research, then update the Messenger skill's
    routing documentation without duplicating the catalog.
  - Verify the summary, skill projection, catalog, accepted documents, and
    research CHANGELOG agree under the selected snapshot manifest.

- [ ] **Workflow Documentation**
  - Document validation, single/full/forced refresh, required budget arguments,
    source access rules, review/promotion, partial failure recovery, generation,
    drift checking, and cleanup in Messenger's README/user documentation.
  - Add package-area `just` recipes for validation, generation/check, reporting,
    fleet refresh, publication, and cleanup preview. Use portable Rust/CLI
    operations rather than Unix-only filesystem scripts.
  - Update root/per-area dependency documentation if crates or feature edges were
    added; ensure generated files are labeled and never documented as hand-edited.

### Wave 22 — Final Verification

- [ ] **Messenger Gates**
  - Run `just test` and `just lint` in `messenger/`, explicit maintenance and
    all-chat-provider checks, deterministic generation/check, help snapshots, and
    the full validation corpus through nextest-backed recipes.
  - Confirm ordinary offline builds/sends neither read research artifacts nor
    require fresh metadata, network access, an agent, or a research workspace.

- [ ] **Claudine Gates**
  - Run `just test` and `just lint` in `claudine/` and the focused fake-worker,
    sequence, cancellation, restart, and budget suites. Confirm tests use the
    existing isolated process fixture and keep audio/browser/terminal focus off.

- [ ] **Platform Gates**
  - Use `just cross-check` with available `BUILD_LINUX`, `BUILD_WIN`, and
    `BUILD_WSL` hosts for affected Messenger and Claudine packages; retain macOS
    evidence locally and report any unavailable environment precisely.
  - Exercise native path spelling, atomic snapshot recovery, process-tree
    cancellation, fixture binary lookup, and nextest-archive behavior on the OS
    where each contract matters. Treat cross-compilation as compile evidence only.

- [ ] **Change Analysis**
  - Run GitNexus `detect_changes` for the complete worktree and resolve any
    partial/truncated analysis before review. Investigate every HIGH/CRITICAL or
    UNKNOWN impact with source search and focused tests.
  - Review docs/comments for behavioral drift, confirm no runtime provider behavior
    or public message APIs changed, and leave the feature implementation complete
    and ready for author review without moving it to `_completed` or committing.

### Phase 8 Checkpoint

- [ ] Demonstrate from a clean fixture: validate accepted inputs, generate twice
  byte-identically, detect no drift, render human and JSON reports, simulate a
  failed partial refresh, recover the selected snapshot, and resume within the
  retained budget.
- [ ] Confirm every specification acceptance criterion is linked to passing
  evidence in the Phase 1 fixture matrix and that remaining unknowns are research
  gaps—not silent omissions, inferred unlimited support, or implementation claims.
