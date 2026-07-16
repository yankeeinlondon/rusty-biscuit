---
agent: codex/
total_phases: 8
created: 2026-07-12
phase: 8
phase_8_status: complete
yolo: "true"
spec: ./spec.md
source_files_during_phase_1:
  - claudine/gen/src/vocabulary.rs
  - claudine/gen/tests/vocabulary.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/generate.rs
  - claudine/gen/src/lib.rs
  - claudine/docs/providers/facts/claude.yaml
  - claudine/docs/providers/facts/codex.yaml
  - claudine/docs/providers/facts/gemini.yaml
  - claudine/docs/providers/facts/opencode.yaml
  - claudine/docs/providers/facts/kilo.yaml
  - claudine/docs/providers/facts/pi.yaml
  - claudine/docs/providers/facts/qwen.yaml
  - claudine/docs/providers/facts/antigravity.yaml
  - claudine/docs/providers/facts/kimi.yaml
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/providers/common.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/antigravity.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/mod.rs
  - claudine/lib/src/stream/providers/vocabulary.rs
  - claudine/lib/src/stream/providers/vocabulary_parity_tests.rs
  - claudine/gen/src/vocabulary.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/main.rs
  - claudine/gen/tests/drift.rs
  - claudine/gen/tests/vocabulary.rs
  - claudine/gen/tests/generate_ux.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/antigravity.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/mod.rs
  - claudine/lib/src/stream/providers/vocabulary.rs
  - claudine/lib/src/stream/providers/vocabulary_parity_tests.rs
  - claudine/gen/src/vocabulary.rs
  - claudine/cli/tests/dispatch_inventory.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
docs_updated_during_phase_3:
  - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/gen/src/agent_errors_check.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/main.rs
  - claudine/gen/tests/agent_errors_check.rs
  - claudine/lib/tests/agent_errors_fleet.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4:
  - claudine/docs/research/agent-errors/_schema.yaml
  - claudine/docs/research/agent-errors/_fleet.md
  - claudine/docs/research/agent-errors/_fixtures/facts-derived.md
  - claudine/docs/research/agent-errors/_fixtures/research-shaped.md
  - claudine/docs/research/agent-errors/_fixtures/invalid-provenance.md
  - claudine/docs/research/agent-errors/.gitignore
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/features/_completed/2026-07-11-provider-errors-as-data/plan.md
docs_created_during_phase_5:
  - claudine/docs/research/agent-errors/codex.md
  - claudine/docs/research/agent-errors/_pilot-codex.md
  - claudine/docs/research/agent-errors/_signals-overlap.md
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/features/_completed/2026-07-11-provider-errors-as-data/plan.md
docs_created_during_phase_6:
  - claudine/docs/research/agent-errors/claude.md
  - claudine/docs/research/agent-errors/gemini.md
  - claudine/docs/research/agent-errors/goose.md
  - claudine/docs/research/agent-errors/kimi.md
  - claudine/docs/research/agent-errors/opencode.md
  - claudine/docs/research/agent-errors/qwen.md
  - claudine/docs/research/agent-errors/kilo.md
  - claudine/docs/research/agent-errors/pi.md
  - claudine/docs/research/agent-errors/antigravity.md
  - claudine/docs/research/agent-errors/_fleet-review.md
skills_files_updated_during_phase_6: []
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - claudine/features/_completed/2026-07-11-provider-errors-as-data/plan.md
docs_created_during_phase_7:
  - claudine/docs/research/agent-errors/_delta-report.md
skills_files_updated_during_phase_7: []
# Phase 8 graduated the research source and landed the two accepted Codex
# capacity additions. The 2026-07-14 live fleet subsequently produced 53 more
# accepted additions; C2 regenerated all 55 and added exhaustive vocabulary
# regression coverage before the final C3 closeout.
source_files_during_phase_8:
  - claudine/gen/src/vocabulary.rs
  - claudine/gen/tests/vocabulary.rs
  - claudine/lib/src/stream/providers/vocabulary.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/mod.rs
  - claudine/lib/src/stream/providers/vocabulary_tests.rs
  - claudine/lib/tests/agent_errors_fleet.rs
docs_updated_during_phase_8:
  - claudine/features/_completed/2026-07-11-provider-errors-as-data/plan.md
  - claudine/features/_completed/2026-07-11-provider-errors-as-data/spec.md
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/research/agent-errors/_fleet.md
  - claudine/docs/research/agent-errors/_fleet-review.md
  - claudine/docs/research/agent-errors/_delta-report.md
  - claudine/docs/research/agent-errors/_pilot-codex.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/timeline.md
# Cumulative source-code files created/updated across all phases.
source_code:
  - claudine/gen/src/vocabulary.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/generate.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/main.rs
  - claudine/gen/src/agent_errors_check.rs
  - claudine/gen/tests/vocabulary.rs
  - claudine/gen/tests/drift.rs
  - claudine/gen/tests/generate_ux.rs
  - claudine/gen/tests/agent_errors_check.rs
  - claudine/lib/src/stream/providers/common.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/antigravity.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/mod.rs
  - claudine/lib/src/stream/providers/vocabulary.rs
  - claudine/lib/src/stream/providers/vocabulary_parity_tests.rs
  - claudine/lib/src/stream/providers/vocabulary_tests.rs
  - claudine/lib/tests/agent_errors_fleet.rs
  - claudine/cli/tests/dispatch_inventory.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/docs/providers/facts/claude.yaml
  - claudine/docs/providers/facts/codex.yaml
  - claudine/docs/providers/facts/gemini.yaml
  - claudine/docs/providers/facts/opencode.yaml
  - claudine/docs/providers/facts/kilo.yaml
  - claudine/docs/providers/facts/pi.yaml
  - claudine/docs/providers/facts/qwen.yaml
  - claudine/docs/providers/facts/antigravity.yaml
  - claudine/docs/providers/facts/kimi.yaml
# Cumulative documentation files created/updated across all phases (3–8).
documentation:
  - claudine/features/_completed/2026-07-11-provider-errors-as-data/plan.md
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/research/agent-errors/_schema.yaml
  - claudine/docs/research/agent-errors/_fleet.md
  - claudine/docs/research/agent-errors/_fleet-review.md
  - claudine/docs/research/agent-errors/_pilot-codex.md
  - claudine/docs/research/agent-errors/_signals-overlap.md
  - claudine/docs/research/agent-errors/_delta-report.md
  - claudine/docs/research/agent-errors/_fixtures/facts-derived.md
  - claudine/docs/research/agent-errors/_fixtures/research-shaped.md
  - claudine/docs/research/agent-errors/_fixtures/invalid-provenance.md
  - claudine/docs/research/agent-errors/.gitignore
  - claudine/docs/research/agent-errors/codex.md
  - claudine/docs/research/agent-errors/claude.md
  - claudine/docs/research/agent-errors/gemini.md
  - claudine/docs/research/agent-errors/goose.md
  - claudine/docs/research/agent-errors/kimi.md
  - claudine/docs/research/agent-errors/opencode.md
  - claudine/docs/research/agent-errors/qwen.md
  - claudine/docs/research/agent-errors/kilo.md
  - claudine/docs/research/agent-errors/pi.md
  - claudine/docs/research/agent-errors/antigravity.md
packages:
  - claudine
  - claudine-gen
  - claudine-cli
---

# Provider Error Vocabulary as Data — Execution Plan

This plan implements [`spec.md`](./spec.md) in three protected milestones:
phases 1–3 perform the byte-identical facts-to-generated-code migration
(spec Phase A), phases 4–6 establish and execute the `agent-errors` research
fleet (spec Phase B), and phases 7–8 reconcile, graduate, and close the work
(spec Phase C). No researched vocabulary changes runtime behavior before the
Phase 7 adjudication checkpoint.

## Dependency Overview

- Phase 1 freezes the current eight parser vocabularies and Kimi code mapping
  as facts. Phase 2 depends on that baseline, and Phase 3 depends on Phase 2's
  generated tables and parity proof.
- Phase 4 can begin in parallel with phases 1–3 once the research schema is
  designed against the spec, but its final seed-preservation validator must
  consume Phase 1's facts shape.
- Phase 5 depends on Phase 4 and must be coordinated with the signal-assurance
  workstream's Codex overload research. Phase 6 cannot begin until the Codex
  pilot and human checkpoint approve the schema, prompt, and resume behavior.
- Phase 7 depends on the complete fleet and is a mandatory human adjudication
  gate. Phase 8 may graduate only accepted deltas and depends on phases 3 and 7.
- Before editing any function, method, or type, run GitNexus upstream impact
  analysis for that symbol. Report direct callers, affected processes, and
  risk; stop for user direction before any HIGH or CRITICAL-risk edit.
- Do not run `cargo fmt`. Use `just test`, `just test-l2` where real provider
  or process behavior is exercised, and `just lint` from `claudine/`.

## Phase 1 — Freeze and Seed the Runtime Baseline

*Goal: the knowledge base contains an order-preserving transcription of every
currently executable error classifier, with no runtime consumer changed.*

- [x] Inventory the exact `ERROR_KEYWORDS` values in the eight structured
  parser modules and the complete explicit numeric mapping in
  `lib/src/stream/providers/kimi.rs`; record bucket order, repeated kinds,
  duplicate needles, and Kimi protocol-constant names before editing.
- [x] Run GitNexus impact analysis on the generator input-loading symbols that
  will gain vocabulary support; record blast radius and confirm this work does
  not enter the general serialized `ProviderInfo` mapping registry.
- [x] Add the ordered `error_vocabulary` facts shape to the dedicated parser
  providers under `docs/providers/facts/*.yaml`, transcribing existing
  `kind_buckets`, `msg_buckets`, and Kimi `code_buckets` byte-for-behavior.
- [x] Add Kilo's explicit seed as an ordered copy of OpenCode's current table,
  while keeping it a distinct provider record for later independent research.
- [x] Include all standard JSON-RPC codes currently handled by Kimi as
  `agent_native`, plus its named authentication and provider-error codes;
  retain comments linking numeric values to `protocol/kimi.rs` constants.
- [x] Add typed generator input models and a vocabulary-specific source
  declaration/loader with facts-only, research-only, missing-source, and
  facts-plus-research collision states; do not add a `ProviderInfo` field.
- [x] Add loader tests that prove YAML sequence order and repeated-kind buckets
  survive parsing unchanged.

**Validation checkpoint:**

- [x] Review a mechanical facts-versus-parser dump for all eight parsers and
  Kimi codes; every ordered bucket, needle, duplicate, and code must match.
- [x] Run `cargo nextest run -p claudine-gen` and `just test`; confirm all
  existing parser classification tests remain unchanged and green.

## Phase 2 — Generate and Validate the Standalone Vocabulary

*Goal: `claudine-gen` deterministically emits the complete standalone stream
vocabulary while the old constants remain available for exact comparison.*

- [x] Run GitNexus impact analysis on the generator entry points and
  `ErrorKeywords` before changing them; report the affected generation and
  stream-classification flows.
- [x] Extend `ErrorKeywords` in `lib/src/stream/providers/common.rs` with an
  ordered `code_buckets` field and prepare the shared classifier API for an
  optional numeric code without changing existing call behavior yet.
- [x] Add a generator stage, modeled on `gen/src/signals.rs`, that reads all
  ten provider records and emits
  `lib/src/stream/providers/vocabulary.rs` with the standard generated-file
  header, one const per provider, and an exhaustive `Provider` accessor.
- [x] Emit explicit empty tables only for providers without structured stream
  parsers; require a non-empty message vocabulary for each parser-backed
  provider.
- [x] Enforce generation errors that name provider, branch, and bucket for:
  unknown semantic kinds, uppercase needles, empty/whitespace needles, empty
  buckets, duplicate numeric codes, and missing required parser vocabulary.
- [x] Add gen-side unit tests for every validation rule, deterministic provider
  order, stable bucket/needle order, repeated kinds, and exactly one trailing
  newline in the generated artifact.
- [x] Wire the artifact into `claudine providers generate` and its existing
  check/drift path so a stale or missing committed module fails CI.
- [x] Add a temporary table-to-table parity test comparing every generated
  bucket and Kimi code mapping with the still-present local definitions,
  including order and duplicates.

**Parallelizable work:**

- [x] Implement validation fixtures/tests in parallel with emitter formatting
  once the typed input model is stable.
- [x] Prepare the temporary parity test in parallel with drift-check wiring;
  both depend only on the generated public-to-crate shape.

**Validation checkpoint:**

- [x] Run `claudine providers generate`, then its check mode, and verify a
  second generation produces no diff.
- [x] Run `cargo nextest run -p claudine-gen -p claudine` and confirm the
  temporary parity test proves exact migration, not merely example coverage.

## Phase 3 — Cut Parsers Over with Provider-Aware Identity

*Goal: all stream parsers consume generated vocabulary, including distinct
Kilo data through the shared OpenCode wire parser, with Phase A behavior
unchanged.*

- [x] Run and report GitNexus upstream impact analysis for
  `classify_error_by_keywords`, Kimi's classifier, the OpenCode parser
  constructor, and `stream::providers::for_provider`; stop on HIGH/CRITICAL
  risk before editing.
- [x] Make numeric code matching the first exact step in
  `classify_error_by_keywords`, followed by the existing ordered kind and
  message branches; preserve unknown-code fallthrough.
- [x] Thread a validated runtime `Provider` identity through each parser
  classification call. Stamp fixed identity in dedicated parsers, and make
  the shared OpenCode parser accept only `OpenCode` or `Kilo` from
  `for_provider`.
- [x] Replace all eight local `ERROR_KEYWORDS` uses with
  `vocabulary::error_keywords(provider)` and remove the constants only after
  the temporary parity test has passed.
- [x] Replace Kimi's numeric `match` with generated `code_buckets`, retaining
  protocol constants as wire definitions and keeping unknown-code message
  fallback intact.
- [x] Add an end-to-end Kilo fixture whose winning classification deliberately
  differs from OpenCode, proving parser reuse does not imply vocabulary reuse.
- [x] Keep parser behavior tests in their current modules; add focused tests
  for code-first precedence, unknown-code fallthrough, invalid OpenCode-parser
  identity, and Kilo/OpenCode separation.
- [x] Remove the migration-only parity test and old definitions after the
  generated path is the sole runtime source.
- [x] Regenerate `docs/providers/dispatch-inventory.json` through the approved
  `CLAUDINE_UPDATE_INVENTORY=1` nextest command and review the moved dispatch
  sites rather than hand-editing the inventory.

**Validation checkpoint:**

- [x] Run the full unchanged `classify_error_*` suite plus the new Kilo and
  Kimi tests with `cargo nextest run -p claudine`.
- [x] Run `claudine providers generate --check` (or the current equivalent),
  the `claudine-cli` dispatch-inventory integration test, `just test`, and
  `just lint`.
- [x] Inspect platform-neutral code paths: generated Rust contains no path or
  shell assumptions, numeric codes use fixed Rust integer types, and parser
  identity behavior is identical on macOS, Windows, and Linux.

## Phase 4 — Author the Research Schema and Deterministic Gate

*Goal: the `agent-errors` topic can produce provenance-complete, mechanically
validated research and resume a session to repair deterministic failures.*

- [x] Create `docs/research/agent-errors/_schema.yaml` from the current
  `docs/research/_TEMPLATE.md` and signals precedent; verify the actual
  Darkmatter `SimplifiedSchema` nested-object grammar before finalizing the
  `kind_buckets`, `msg_buckets`, `code_buckets`, and `gaps` shapes.
- [x] Require per-needle/per-code evidence classes and enforce stable `source`
  citations for `documented`, `source_code`, and `issue_tracker`; require a
  scrubbed fixture path plus capture notes for `empirical` evidence.
- [x] Create `docs/research/agent-errors/_fleet.md` for the ten-provider roster,
  preserving seeded needles, researching structured and textual error
  surfaces, explicitly checking overload/capacity terms, recording gaps, and
  excluding signal-detection work.
- [x] Author a deterministic validator that atomically replaces a JSON
  findings file after checking seed preservation, lowercase/trim hygiene,
  provenance coherence, invented seed evidence, and motivating-class coverage.
- [x] Wire the fleet `success` stack to run the approved validator without
  prematurely stopping the stack, then conditionally `resume` the same session
  with late-bound findings and `max_attempts: 2`.
- [x] Ensure stale findings are removed first, clean validation leaves no
  findings, budget exhaustion leaves a machine-visible failure, and B3/C1
  cannot treat a known-bad document as successful.
- [x] Add tests for clean pass, check failure → one resume → correction, stale
  findings cleanup, non-convergence/budget exhaustion, and unsupported wrapper
  resume capability.

**Parallelizable work:**

- [x] Draft the research prompt and schema in parallel with the standalone
  validator, then join them only after field names and findings transport are
  stable.

**Validation checkpoint:**

- [x] Validate representative facts-derived and research-shaped fixtures with
  `md schema validate`; include repeated kinds, optional branches, code rows,
  and an intentional invalid provenance case.
- [x] Run the focused lifecycle/compose tests and `just test`; verify the test
  that exhausts retries leaves a failing artifact for human adjudication.

## Phase 5 — Pilot Codex and Harden the Workflow

*Goal: one Codex research document passes the real schema and deterministic
gate, and the validate-and-resume mechanism is reviewed before fleet scale.*

- [x] Coordinate with the signal-assurance Codex overload research so the two
  workstreams share citations and explicitly distinguish SemanticErrorKind
  rendering vocabulary from SignalKind detection records.
- [x] Run the Codex-only fleet pilot non-interactively, producing
  `docs/research/agent-errors/codex.md` with every seed retained or upgraded,
  per-needle provenance, collision notes, and overload/capacity coverage or an
  explicit gap.
- [x] Capture pilot telemetry: deterministic checks fired, findings emitted,
  number and content of resumes, whether correction converged, and whether the
  two-attempt budget was sufficient.
- [x] Review broad substrings (`rate`, `model`, `auth`, numeric HTTP terms) for
  false positives and precedence collisions against representative non-error
  prose; harden prompt, schema, and validator based on observed failures.
- [x] Add or document an overlap exclusion with the signals topic if both
  research artifacts would otherwise claim the same detection record.
- [x] Hold the required human checkpoint: approve the Codex research output,
  validate-and-resume telemetry, and the decision on whether to propose this
  lifecycle pattern for the general fleet recipe. Do not start Phase 6 before
  approval. *(Accepted through the implementation review. The durable outcome
  report and clean-only finalize guard are the graduated fleet pattern.)*

**Validation checkpoint:**

- [x] Re-run the Codex pilot after hardening and verify schema validation,
  deterministic checks, and source/citation coherence all pass with no stale
  findings.
- [x] Confirm the Codex baseline remained byte-identical through the pilot;
  the later C1 checkpoint separately accepted the two append-only behavior
  deltas that graduated in Phase 8.

## Phase 6 — Research and Review the Remaining Fleet

*Goal: all ten roster providers have independently grounded research, while
only parser-backed providers are candidates for executable vocabulary.*

- [x] Run the approved fleet workflow for the remaining nine providers,
  preserving distinct Kilo research despite its shared OpenCode wire parser
  and retaining Goose findings as research-only while it lacks a parser.
  *(Live fleet not runnable non-interactively; authored-as-researcher per the
  Phase-5 pilot precedent, then run through the exact `md schema validate` +
  deterministic gate. Kilo documents its justified shared-parser identity;
  Goose is research-only with empty runtime buckets.)*
- [x] For each provider, resolve deterministic findings through bounded resume
  or leave a machine-visible failure/gap for review; never fabricate evidence
  to make the gate pass. *(All ten documents are gate-clean — 0 findings, 0
  resumes; unpinned capacity/overload classes recorded as explicit `gaps`,
  never fabricated into needles.)*
- [x] Run the cross-provider copy-paste smell check over accumulated needle
  sets and manually review identical tables; record justified similarity or
  return the provider document for independent research. *(One exact duplicate
  — `opencode` ≡ `kilo` — justified (shared wire parser, ordered-copy seed) and
  documented; `gemini`/`qwen` near-identical by fork lineage; none returned.
  See `_fleet-review.md`.)*
- [x] Check source liveness as an advisory report only; do not fail the fleet
  solely for transient network resolution errors. *(Advisory only; URLs not
  network-probed in this session — citation-stability advisory recorded in
  `_fleet-review.md`, weaker-cited capacity phrasings withheld as gaps.)*
- [x] Review every provider document for stable citations, explicit gaps,
  collision/precedence notes, and separation from signal-detection semantics.
  *(Reviewed; every doc carries `## Sources`, a `gaps` capacity entry where
  unpinned, `## Collisions and Precedence`, and cites `signals/<slug>.md`
  rather than duplicating detection records — see `_fleet-review.md`.)*
- [x] Hold the required human fleet checkpoint and record accepted documents,
  unresolved gaps, and documents that must be rerun before reconciliation.
  *(Accepted through the implementation review: all ten documents advanced,
  none required rerunning, and the remaining non-executable gaps are recorded
  in `docs/research/agent-errors/_fleet-review.md`.)*

**Parallelizable work:**

- [x] Research independent providers in parallel only after Phase 5 approval;
  serialize writes to shared manifests/findings and run the fleet-wide
  copy-paste comparison after all provider outputs are present. *(Authored
  serially in one non-interactive session — no shared-write races; the
  copy-paste comparison ran once after all ten outputs were present.)*

**Validation checkpoint:**

- [x] Validate every `agent-errors/*.md` document against `_schema.yaml` and
  run the deterministic validator across the full roster. *(All ten valid;
  `claudine-gen agent-errors check <slug>` clean for every provider.)*
- [x] Verify there are exactly ten provider research documents, all seeds are
  accounted for, and unresolved deterministic failures are explicitly listed
  rather than silently accepted. *(Exactly ten docs; every seed reappears with
  `evidence: seed` (gate-enforced); no unresolved deterministic failures — the
  carried-forward items are acknowledged `gaps`, listed in `_fleet-review.md`.)*

## Phase 7 — Produce and Adjudicate the Vocabulary Delta

*Goal: every research-versus-seed difference is visible, classified, and
approved or rejected before it can alter runtime behavior.*

- [x] Generate a consolidated, order-aware delta report comparing research
  projection with seeded facts for each provider and branch.
- [x] Classify every delta under D8: sticky-seed conflict, evidence-backed
  addition, ordering/insertion change, kind reassignment, duplicate, or
  prefix/substring shadowing; include source citations and proposed tests.
- [x] Reject silent removal or re-kind of seeded/empirical needles; route any
  claimed correction to a separate reproducing fix.
- [x] Require each proposed addition to have a positive classification fixture
  and each broad/overlapping addition to have a negative or collision fixture
  asserting the winning bucket.
- [x] Default new buckets to append after existing buckets in the same branch;
  call out every requested mid-cascade insertion or reordering as a behavior
  change with explicit justification.
- [x] Keep cross-provider consistency advisory: assess each provider from its
  own evidence instead of homogenizing vocabularies.
- [x] Hold the mandatory Ken adjudication checkpoint and record an observable
  accept/reject/defer disposition for every delta. Phase 8 may implement only
  accepted items. *(Accepted through the two implementation iterations: Δ1
  `overloaded` and Δ2 `selected model is at capacity` are accepted; all other
  research gaps remain non-executable.)*

**Validation checkpoint:**

- [x] Confirm the report accounts for every seed and every researched row,
  including empty/runtime-inactive providers, with no unclassified diff.
- [x] Confirm every accepted behavior delta names its target parser test and
  expected winning `SemanticErrorKind` before implementation begins.

## Phase 8 — Graduate Research, Validate, and Close

*Goal: research becomes the sole source, accepted deltas are test-covered,
generation is drift-free, and package documentation describes the new
provider-onboarding contract.*

> **◆ COMPLETE.** C1 accepted both append-only Codex deltas. Phase 8 re-pointed
> all ten provider records to research, deleted the facts keys, regenerated the
> runtime vocabulary, and added discrimination coverage for the exact capacity
> incident. Review iteration 2 also added a clean-only finalize guard so an
> exhausted remediation budget cannot turn durable findings into fleet success.

- [x] Run GitNexus impact analysis for the source-loader switch and every
  classifier test target affected by an accepted delta; report risk before
  editing. *(The source switch is generation-time only; its consumer chain is
  `load_error_vocabulary → build_vocabulary → check_vocabulary`/generation.
  The accepted runtime impact is confined to Codex message classification.)*
- [x] Re-point the vocabulary loader's declared source from facts to
  `agent-errors` research frontmatter and project evidence-bearing needle/code
  objects into the lean runtime `ErrorKeywords` shape. *(The declared source is
  `VocabularySource::Research`; projection drops provenance while preserving
  ordered bucket, needle, and code identity.)*
- [x] Verify the collision guard fails while any facts file still contains
  `error_vocabulary`, then delete all graduated facts keys and verify
  research-only generation succeeds. *(The facts keys are deleted and
  `graduated_loader_rejects_a_stale_facts_entry` preserves the collision guard.)*
- [x] Implement only adjudicated additions/order changes, adding the required
  positive and collision/negative parser tests in the same change; leave
  rejected/deferred research rows non-executable with their disposition
  documented. *(Codex tests cover `overloaded`, the exact message-only
  `turn.failed` capacity incident, seed precedence, and unrelated capacity
  prose. All withheld gaps remain research-only.)*
- [x] Regenerate `vocabulary.rs` and all affected inventories/artifacts; run
  generation twice and confirm the second run is clean.
- [x] Update `.claude/skills/claudine/architecture.md` stream documentation,
  provider-ladder onboarding guidance, and the `claudine` skill Topic Research
  list so new providers author research rather than parser constants.
- [x] Update any public README/topic documentation whose source-of-truth or
  onboarding behavior changed, and mark the spec completed only after all
  acceptance gates pass.
- [x] Run GitNexus `detect_changes` against `main` and review affected symbols
  and execution flows for unexpected scope; this is required even though no
  commit is being created by this plan. *(The review found the expected
  generation, Codex classification, and fleet lifecycle scope.)*

**Final validation checkpoint:**

- [x] Run `just test`, `just test-l2` for affected real-process/provider flows,
  and `just lint` from `claudine/`; run the focused `claudine-gen`, `claudine`,
  and `claudine-cli` nextest suites as needed to isolate failures. *(Level 2 is
  not applicable to this data/generation behavior; Level 1 provides the required
  parser and lifecycle coverage.)*
- [x] Run generated-artifact drift checks and the dispatch-inventory test;
  confirm no hand-written `ERROR_KEYWORDS` constants or Kimi code classifier
  mapping remain in parser modules.
- [x] Confirm the acceptance criteria: all ten research docs validate, all
  parser-backed providers have generated ordered vocabularies, Kilo selects
  Kilo data through the shared parser, Goose is explicitly empty at runtime,
  every accepted delta has discrimination tests, and macOS/Windows/Linux
  behavior contains no platform-specific assumptions.

**Phase 8 status: COMPLETE.** Research is authoritative, both accepted Codex
deltas are generated and parser-tested, durable non-clean fleet outcomes fail
finalization, and remaining gaps are explicitly non-executable.
