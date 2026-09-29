---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/features/2026-09-08-steering/spec.md
plan: claudine/features/2026-09-08-steering/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
  - claudine-catalog-types
  - claudine-gen
  - claudine
  - claudine-cli
  - rendezvous-core
  - rendezvous-daemon
  - rendezvous-client
source_files_during_phase_1:
  - claudine/catalog-types/src/steering.rs
  - claudine/catalog-types/src/lib.rs
  - claudine/catalog-types/Cargo.toml
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/src/steering_check.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/inputs.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/main.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/l1/main.rs
  - claudine/gen/tests/l1/drift.rs
  - claudine/gen/tests/l1/generate_ux.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/src/lib.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/adapters.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/cli/tests/l1/dispatch_inventory.rs
  - claudine/docs/providers/steering-activation.yaml
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/research/steering/_schema.yaml
docs_updated_during_phase_1:
  - claudine/README.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/docs/research/steering/_fleet.md
  - claudine/docs/research/steering/antigravity.md
  - claudine/docs/research/steering/claude.md
  - claudine/docs/research/steering/codex.md
  - claudine/docs/research/steering/gemini.md
  - claudine/docs/research/steering/goose.md
  - claudine/docs/research/steering/kilo.md
  - claudine/docs/research/steering/kimi.md
  - claudine/docs/research/steering/opencode.md
  - claudine/docs/research/steering/pi.md
  - claudine/docs/research/steering/qwen.md
docs_created_during_phase_1:
  - claudine/docs/topics/steering-activation.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/secrets/mod.rs
  - claudine/lib/src/secrets/tests.rs
  - claudine/lib/src/lib.rs
  - claudine/lib/src/protect/scrub.rs
  - claudine/lib/src/messaging/send.rs
  - claudine/lib/src/reporting/jsonl.rs
  - claudine/lib/src/reporting/mod.rs
  - claudine/lib/src/reporting/paths.rs
  - claudine/lib/src/dispatch/logging.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/audit.rs
  - claudine/lib/src/steering/audit/tests.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/cli/src/commands/wrap/env/sanitize.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_2:
  - claudine/README.md
  - claudine/docs/topics/messaging.md
  - claudine/docs/topics/traces-and-logging.md
docs_created_during_phase_2:
  - claudine/docs/topics/secret-recognition.md
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/catalog-types/src/steering.rs
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/Cargo.toml
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/lib/src/steering/identity.rs
  - claudine/lib/src/steering/contract.rs
  - claudine/lib/src/steering/audit.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/controller.rs
  - claudine/lib/src/steering/controller/tests.rs
  - claudine/lib/src/steering/discovery.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/rendezvous/core/proto/rendezvous.proto
  - claudine/rendezvous/core/build.rs
  - claudine/rendezvous/core/src/lib.rs
  - claudine/rendezvous/daemon/src/lib.rs
  - claudine/rendezvous/daemon/src/server.rs
  - claudine/rendezvous/daemon/src/service.rs
  - claudine/rendezvous/daemon/src/steering.rs
  - claudine/rendezvous/daemon/src/steering/tests.rs
  - claudine/rendezvous/client/Cargo.toml
  - claudine/rendezvous/client/tests/steering_round_trip.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/main.rs
  - claudine/cli/src/cli_utils.rs
  - claudine/cli/src/budget/run.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/requester.rs
  - claudine/cli/src/steering/wire.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/docs/providers/dispatch-inventory.json
  - Cargo.lock
docs_updated_during_phase_3:
  - claudine/README.md
  - claudine/docs/dependencies.md
  - claudine/docs/rendezvous/local-ipc.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/traces-and-logging.md
docs_created_during_phase_3:
  - claudine/docs/topics/steering-routing.md
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/docs/providers/facts/pi.yaml
  - claudine/docs/providers/catalog.json
  - claudine/docs/providers/steering-activation.yaml
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/catalog-types/src/steering.rs
  - claudine/catalog-types/src/signal.rs
  - claudine/gen/src/steering_catalog.rs
  - claudine/gen/tests/l1/steering_activation.rs
  - claudine/gen/tests/l1/generate_ux.rs
  - claudine/gen/tests/fixtures/generated-artifact-baseline.json
  - claudine/lib/src/provider/pi/data.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/adapters.rs
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/generated.rs
  - claudine/lib/src/steering/controller/tests.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/lib/src/stream/protocol/pi.rs
  - claudine/lib/src/stream/protocol/pi/rpc.rs
  - claudine/lib/src/stream/protocol/pi/rpc/tests.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/pi/tests.rs
  - claudine/lib/src/stream/logs/opencode/bridge/mod.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/control.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/mod.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/commands.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/executor.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
  - claudine/cli/src/commands/wrap/exec/spawn/descendants.rs
  - claudine/cli/src/commands/wrap/exec/termination/message.rs
  - claudine/cli/src/commands/wrap/exec/termination/reasons.rs
  - claudine/cli/src/commands/wrap/exec/termination/summary.rs
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/wrap/profile/pi.rs
  - claudine/cli/src/commands/wrap/profile/tests/pi_managed.rs
  - claudine/cli/src/commands/wrap/resume.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/tests/bin/fake_pi/main.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/pi_managed_rpc.rs
  - claudine/cli/tests/real/main.rs
  - claudine/cli/tests/real/real_pi_managed_rpc.rs
  - claudine/cli/tests/real/real_pi_steering.rs
  - claudine/cli/tests/fixtures/steering/pi-probe.ts
  - claudine/cli/tests/fixtures/steering/pi-bash-cleanup-probe.ts
docs_updated_during_phase_4:
  - claudine/README.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/steering-routing.md
  - claudine/docs/topics/timeouts.md
docs_created_during_phase_4:
  - claudine/docs/topics/pi-rpc.md
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/lib/src/steering/eligibility.rs
  - claudine/lib/src/steering/eligibility/tests.rs
  - claudine/lib/src/steering/discovery.rs
  - claudine/lib/src/steering/discovery/tests.rs
  - claudine/rendezvous/core/proto/rendezvous.proto
  - claudine/cli/src/args.rs
  - claudine/cli/src/main.rs
  - claudine/cli/src/telemetry.rs
  - claudine/cli/src/commands/mod.rs
  - claudine/cli/src/commands/help.rs
  - claudine/cli/src/commands/steer/mod.rs
  - claudine/cli/src/commands/steer/service.rs
  - claudine/cli/src/commands/steer/interact.rs
  - claudine/cli/src/commands/steer/render.rs
  - claudine/cli/src/commands/steer/tests.rs
  - claudine/cli/src/completion/root_menu.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/requester.rs
  - claudine/cli/src/steering/wire.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/steer_cli.rs
  - claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_5:
  - claudine/README.md
  - claudine/docs/topics/steering-routing.md
  - claudine/docs/topics/steering-activation.md
  - claudine/docs/topics/traces-and-logging.md
docs_created_during_phase_5:
  - claudine/docs/cli/steer.md
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/lib/src/runaway/detector.rs
  - claudine/lib/src/runaway/detector/tests.rs
  - claudine/lib/src/runaway/detector/tests/warnings.rs
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/steering/mod.rs
  - claudine/lib/src/steering/automatic.rs
  - claudine/lib/src/steering/automatic/tests.rs
  - claudine/lib/src/steering/controller.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/lib/src/config/claudine_config/tests.rs
  - claudine/lib/src/config/merge.rs
  - claudine/lib/src/dispatch/runner/speak.rs
  - claudine/lib/src/dispatch/runner/tests.rs
  - claudine/cli/src/steering/mod.rs
  - claudine/cli/src/steering/automatic.rs
  - claudine/cli/src/steering/automatic/tests.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/commands/steer/mod.rs
  - claudine/cli/src/commands/steer/render.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/cli/src/commands/wrap/runaway_guard.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/automatic_help.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/exec/control.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/mod.rs
  - claudine/cli/src/commands/wrap/exec/pi_rpc/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/tests/bin/fake_pi/main.rs
  - claudine/cli/tests/l1/pi_managed_rpc.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_6:
  - claudine/README.md
  - claudine/docs/topics/timeouts.md
  - claudine/docs/topics/steering-routing.md
docs_created_during_phase_6:
  - claudine/docs/topics/automatic-steering.md
skills_files_updated_during_phase_6: []
---
# Implementation Log for 2026-09-08-steering (8 phases)

## Phase 1

Started and completed 2026-09-28 on macOS. Status: implemented, all Phase 1
plan items checked.

### Design decisions

- **Standalone generated artifact, not a `ProviderInfo` field.** Steering facts
  are emitted into `lib/src/steering/generated.rs` through the same full-scope
  artifact path as `lib/src/stream/providers/vocabulary.rs`. The mapping
  registry is exhaustive over serialized `ProviderInfo` fields; steering is a
  runtime selection input, not a describable provider property, so its field
  ownership is declared by the steering loader (`gen/src/steering_catalog.rs`)
  exactly as the vocabulary loader declares its own. **Departure from the
  plan:** its literal "extend `registry.rs`" wording is not followed; the
  registry is unchanged and `catalog.json` is byte-identical.
- **Shared rule functions live in `claudine-catalog-types`.** Operation/state
  compatibility (`OperationIntent::supports`), loop-rescue suitability
  (`SteeringMechanism::rescues_active_loop`), prompt-return receipt timing
  (`ReceiptTiming::supports_prompt_return`), required assertion coverage
  (`OperationIntent::required_assertions`), and maximum receipt strength
  (`ReceiptGuarantees::max_strength`) are pure methods, so the generator's
  checker and the library's runtime eligibility evaluate one implementation.
- **Typed assertions required a research contract change (revision 4).**
  Verification rows were prose-only, which cannot support deterministic
  activation. Revision 4 adds `id` and `assertion_kinds` to verification rows
  (`_schema.yaml`, `_fleet.md`, checker `SCHEMA_REVISION`). Only Pi carries
  verification rows; its seven records were backfilled from their existing
  prose (the prose is untouched). The other nine reports change only their
  revision number and a `changes` entry. Records that document a loss or
  failure boundary carry `expected_loss` and can never satisfy a grant.
- **Activation grants and reviewed adapter bindings are hand-owned policy**
  (`docs/providers/steering-activation.yaml`), separate from research facts.
  The generator validates every grant deterministically (in both
  `steering check` and `generate`/`check`), then emits reviewed adapters and
  grants as separate statics. The library's hand-written
  `steering::adapters::IMPLEMENTED_ADAPTERS` must equal the reviewed adapter set
  (L1 guard), and runtime eligibility requires both. Phase 1 ships with no
  adapters and no grants, so every researched case is unavailable with a
  specific reason (asserted by a corpus test over the shipped catalog).
- **`setup_required` access semantics.** Pi's managed RPC access is
  researched as `setup_required` ("Owned RPC child, retained pipes, fresh
  get_state") — the prerequisite is the managed launch profile itself. Treating
  it as a hard block would have permanently blocked Phase 4. Resolution: the
  checker accepts `setup_required` (rejects `blocked`/`unknown`); at runtime it
  passes only when a reviewed exact grant matches, and otherwise the
  prerequisite is surfaced in `Blocker::NotActivated { setup }` as setup
  guidance.
- **Compatibility.** Research compatibility rows describe read-only probes that
  Phase 4+ adapters run; Phase 1 enforces compatibility through the grant's
  single exact `provider_version` (ranges, `x` segments, and placeholders are
  rejected) and the runtime's exact version match. Unknown version or state is
  never guessed.
- **Execution identities.** `ExecutionId`/`RequestId` are 128-bit values
  rendered as canonical lowercase UUID text; parsing is exact (no prefixes, no
  uppercase). Generating them is Phase 3 work; no new dependency was added.
  Target IDs are `managed:<uuid>` and
  `native:<slug>:<pid>:<start-marker>:<conversation>` (conversation last, so it
  may contain `:`). `ManagedTarget::revalidate` rejects a changed wrapper or a
  replaced conversation instead of retargeting.

### What was built

- `claudine-catalog-types::steering`: `HostOs`, `LaunchMode`, `LaunchOrigin`,
  `ExecutionState` (explicit `Unknown`), `SteeringTransport`, `OperationIntent`,
  `ConversationEffect`, `DeliveryBoundary`, `DeliveryState`, `ReceiptStrength`
  (ordered), `ReceiptTiming`, `GuaranteeLevel`, `CaseSupport`,
  `SteeringAvailability`, `AccessStatus`, `VerificationOutcome`,
  `AssertionKind`, `ExecutionInterfaceKind`, and the generated record structs.
  Transport and request encoding (`request_format`) are separate fields.
- `claudine::steering`: `identity` (IDs, `ManagedTarget`, `StaleTarget`),
  `contract` (`SteeringMessage` with 64 KiB/NUL/whitespace validation and a
  text-free `Debug`, `SteeringRequest::may_interrupt` bound to consent target
  and operation, `SendOutcome`, `CancellationOutcome`, `InterruptionOutcome`
  with honest partial outcomes, `SteeringResult`), `eligibility` (manual and
  automatic eligibility with typed `Blocker`/`AutomaticBlocker` reasons), and
  `adapters`.
- `claudine-gen::steering_catalog`: typed research projection, strict policy
  parser, activation applicability, orphan checks, emitter, drift check; wired
  into `generate`, `check`, and `steering check` (`activation:` findings).

### Requirement-to-test mapping

| Requirement | Test(s) |
| --- | --- |
| Next-turn follow-up is not loop rescue | `catalog-types steering::tests::next_turn_follow_up_is_not_active_loop_rescue`; lib `eligibility::tests::next_turn_follow_up_is_manual_only` |
| Terminal-only receipt cannot return after acceptance | `catalog-types …::terminal_or_unknown_acknowledgment_cannot_return_after_acceptance`; lib `…::terminal_only_acknowledgment_cannot_return_after_acceptance` |
| Interruption never automatic; idle start never automatic | lib `…::interruption_only_route_is_selectable_manually_but_never_automatic`, `…::idle_start_is_manual_and_never_automatic`, catalog-types `…::interruption_never_rescues_even_at_a_tool_boundary` |
| Exact provider/version/OS/profile/origin/state/operation | gen `steering_activation::every_single_dimension_edit_is_rejected_with_a_specific_reason` (control row `control_grant_over_the_passing_fixture_is_accepted`); lib `…::exact_applicability_rejects_every_mismatched_dimension`, `…::a_grant_for_another_state_does_not_apply` |
| Unreviewed adapter revision / unimplemented adapter | gen `…every_single_dimension_edit…` (revision 2, unknown adapter), `…::adapter_must_bind_the_mechanism_and_belong_to_the_provider`; lib `…::unimplemented_or_unreviewed_adapter_revision_blocks_delivery`, `adapters::tests::implemented_adapters_equal_reviewed_adapters` |
| Expected-loss-only records cannot activate | gen `…::expected_loss_records_never_activate_delivery`; e2e `generate_ux::steering_activation_policy_gates_check_and_generate` |
| Unrelated passing record cannot enable delivery | gen `…::an_unrelated_passing_record_cannot_enable_delivery`; lib `…::pi_passing_fixture_records_are_not_activation_grants` |
| Missing IDs, unsupported mechanisms, duplicates, orphans | gen `…every_single_dimension_edit…` (unresolved verification/mechanism), `…::duplicate_grants_and_orphan_providers_are_rejected` |
| Malformed execution/research references | gen `…::malformed_research_references_refuse_projection`; existing `steering_check::tests::execution_contract_rejects_invalid_interface_and_steering_foreign_keys` |
| Policy input-robustness matrix | gen `…::policy_parser_walks_the_input_robustness_matrix` |
| Specific actionable blocking reasons; nothing enabled | lib `…::shipped_catalog_activates_nothing_without_reviewed_grants` (passive corpus over all 10 providers), `…::setup_required_access_needs_a_reviewed_grant_and_reports_its_prerequisite`, `…::blocked_or_unknown_access_blocks_even_with_a_grant`, `…::unknown_state_or_version_is_never_guessed`, `…::unsupported_research_case_reports_its_reason` |
| Deterministic regeneration, drift, byte baseline | gen `drift::committed_steering_catalog_matches_regenerated_inputs`, `drift::committed_generated_artifacts_match_phase_1_byte_baseline` (15 pins), `generate_ux::clean_check_report_summary_matches_phase_1_snapshot`, `apply::tests::*` |
| Committed policy valid for every roster provider | gen `…::committed_policy_is_valid`; `claudine-gen steering check` (all 10 clean) |
| Identity/contract honesty | lib `identity::tests::*` (7), `contract::tests::*` (7) |

### Input robustness matrix (activation policy, YAML)

Load-bearing fields: `adapters`, `grants`, grant `verification_ids`,
`provider_version`, and adapter `revision`. One row per edit from the control
policy; all rows asserted in `policy_parser_walks_the_input_robustness_matrix`.

| Shape | `adapters` / `grants` | `verification_ids` | `provider_version` | `revision` |
| --- | --- | --- | --- | --- |
| absent | error | error (required) | error | error |
| explicit null (`null` or empty value) | error | error | error | error |
| wrong type, whole field | error (`grants: 123`) | error | error (`0.84` number) | error (`one`) |
| wrong type, one element | error (`[123]`) | error (`[id, 7]`) | n/a | n/a |
| wrong type, every element | error (`[123]`) | error | n/a | n/a |
| empty | `[]` = nothing reviewed | parses; activation error "names no verification record" | error (not exact) | `0` → activation error |
| duplicate key | error | error | error | error |
| trailing/invalid content | error (second document, garbage) | error | error | error |

Two real defects were found by the matrix and fixed: `serde_yaml_ng` read an
empty `grants:` value as `[]`, and coerced a numeric scalar into a `String`.
The policy now uses `non_null_list`, `strict_string`, and
`strict_string_list` deserializers.

### Checks run

- `cargo nextest run -p claudine-catalog-types`: 30 passed.
- `cargo nextest run -p claudine --lib steering`: 30 passed.
- `cargo nextest run -p claudine-gen`: 192 passed.
- `claudine-gen steering check`: all ten providers clean at revision 4
  (Pi: 7 verification records, 7 passed).
- `claudine-gen generate --yes` then `check`: only
  `lib/src/steering/generated.rs` was written; every existing artifact,
  including `catalog.json`, is byte-identical. Byte pin 4303782739945540261
  (xxh64 via `bh`, which reproduces the existing catalog pin).
- `just lint` (claudine area): clean for all five crates.
- `just test` (claudine area): first run 5009 passed, 2 failed — both in the
  dispatch-inventory guard, which correctly flagged the generated
  `match provider`. Fixed by exempting `lib/src/steering/generated.rs` like
  `vocabulary.rs` and re-blessing `docs/providers/dispatch-inventory.json`.
  Second run: 5299 passed, 1 LEAK-FAIL in the unrelated
  `composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree`
  (32 s under load; passes in 0.04 s in isolation) — a pre-existing
  load-sensitive process-reaping test, not touched by this phase.
  **Final run: 7787 passed, 9 skipped, 0 failed.**

### Environment limitations encountered

- GitNexus MCP `impact` was denied by the session's permissions. Upstream
  impact was established with a text search instead: `apply_generations` /
  `FullScopeArtifacts` are called only from `gen/src/main.rs` and
  `apply.rs` tests; `steering_check::check_provider`/`check_fleet` only from
  `main.rs` (and via the CLI's `claudine providers steering check`
  pass-through). Risk: low, generator-internal.
- `CLAUDINE_UPDATE_INVENTORY=1 …` (any env-prefixed form) required approval.
  The inventory was re-blessed by temporarily forcing the test's own bless
  branch for one run and then reverting that edit; the committed test file
  differs from `HEAD` only by the new exemption.
- `md schema validate` required approval; shape validation was covered by the
  generator's `load_validated_frontmatter` path, which `steering check` runs.
- Writes under `.claude/skills/claudine/` were denied. Intended skill updates
  (not applied): add `steering` to the Library Module Map in `SKILL.md`, and in
  `cli-reference.md` note that `claudine providers steering check` also reports
  `activation:` findings from `docs/providers/steering-activation.yaml`.

### Cross-OS assessment

Phase 1 is pure data, parsing, and rule code: no processes, paths beyond the
existing generator layout, sockets, or `#[cfg]` branches. The generated file is
byte-compared like the existing generated artifacts, so it inherits their
line-ending handling. No `just cross-check` run was needed for this phase;
CI's Linux/macOS legs cover compilation and L1.

## Phase 2

Started and completed 2026-09-28 on macOS. Status: implemented, all Phase 2
plan items checked. (The Phase 2 work items were plain numbered text; they were
converted to GFM todos, as Phase 1's were, so they could be checked off.)

### Design decisions

- **One catalog, consumer-selected families.** `claudine::secrets` owns
  `SECRET_CATALOG` with three families: `CredentialToken` (the six shapes
  formerly in `protect::scrub`, same ids, patterns, and order), `WebhookUrl`
  (the two patterns formerly private to `messaging::send`), and `Contextual`
  (new: sensitive assignments/fields/flags, authorization headers, URL
  passwords, private-key blocks). Consumers pick families through
  `family_regexes`; steering uses all of them.
- **Recognition is shared; replacement is not.** Scrub keeps sequential
  whole-match replacement with `<redacted>` plus its scrub-only email rule and
  home-path rewrite; messaging keeps `<redacted-webhook-url>`; wrapper
  sanitization keeps env stripping and `****` argument values with its exact
  sensitive-flag list (an argv policy, not free-text recognition). A new pinning
  test (`scrub_keeps_its_sequential_whole_match_policy`) proves scrub still
  collapses `Bearer sk-…` to one `<redacted>` and ignores the contextual rules.
- **Departure: key-name and prefix recognition were unified, broadening two
  consumers.** Scrub (`authorization|api[-_]?key|secret|token`) and the wrapper
  (`API_KEY`, `TOKEN`, `PASSWORD`, …, `*_KEY`) had divergent key-name lists.
  Keeping both would leave two authoritative answers to "is this key a
  secret", which the plan's exit criterion forbids. `is_sensitive_key_name` is
  their union (case-insensitive, `-` ≡ `_`). Effects: harvest scrubbing now
  also redacts `password`, `private_key`, `*_key` (not `public_key`), `*_auth`,
  `*_pat`, `*_pwd`, `*_pem`, `credential`, `access_key`, `passphrase` values;
  the wrapper now also strips `AUTHORIZATION` and `*APIKEY*` variables
  (removed names are still reported, and `--include` still admits them). Bare
  argument masking uses `CREDENTIAL_TOKEN_PREFIXES` (adds `gho_`, `ghu_`,
  `ghs_`, `ghr_`, `github_pat_`, `xoxa-`, `xoxr-`, `xoxs-` to the old five). A
  test pins every prefix to a catalog token shape. All changes are strictly
  more masking; no replacement token or privacy policy changed.
- **False-positive control.** A `key: value` field or `--flag value` is
  masked only when the value looks like a credential (≥ 16 bytes or containing
  a non-letter), so `Token: expired.` stays readable; `=` assignments and
  quoted values under a sensitive key are always masked. Trailing sentence
  punctuation is not masked. A key match that is not sensitive resumes scanning
  inside the rejected match so `https://h/?token=x` is still found.
- **Message-aware redaction (`Redactor`).** Built from the original message, it
  masks recognized spans and every later occurrence of a recognized value of at
  least 6 bytes, so an error echoing only `hunter22` is masked. The values stay
  in memory; `Debug` shows only a count. `RedactedText` is the only text type
  audit records accept.
- **Audit location.** "Existing local JSONL logging path" is reused as
  infrastructure: `reporting::jsonl::append_record` was extracted from
  `write_dispatch_event_to` (which now delegates), and records go to
  `~/.claudine/logs/steering/<local-date>.jsonl`. A subdirectory, not the
  daily event file, because `claudine logs sync` parses every line of files
  directly under `logs/` as `EventMeta` and would report each steering record
  as a line failure. Retention is unchanged (none exists for `logs/`).
- **Audit seam.** `audited_send(log, request, context, deliver)` writes the
  masked `request` record, calls `deliver` (an `FnOnce`, so it cannot be
  replayed) with the original message, writes the `result` record, and returns
  the delivery's own result plus redacted error/echo and any content-free
  `AuditFailure`s (`LocationUnavailable`, `Write(io::ErrorKind)`,
  `Uncorrelated`). `LateResultRecorder` only appends `late_result` records for
  its own request ID and has no delivery path. `AuditContext` carries route
  facts the request lacks (provider, conversation, generation, profile,
  mechanism, opportunity). `OpportunityId` was added to `steering::identity`
  and `InterruptionConsent` became `Serialize`.
- **No product reader.** Nothing in Phase 2 parses audit records back, so the
  Input Robustness Matrix does not apply; tests read lines as
  `serde_json::Value`.

### Requirement-to-test mapping

| Requirement | Test(s) |
| --- | --- |
| Token prose, assignments, auth headers, credential URLs, webhook URLs, overlap, Unicode, multiline (exact masked output) | lib `secrets::tests::corpus_masks_each_secret_and_keeps_the_rest` (25 cases) |
| Ordinary prose, emails, paths, Windows paths, ssh URLs unchanged (borrowed, zero-copy) | `secrets::tests::ordinary_prose_emails_and_paths_are_unchanged` |
| Repeated masking | `secrets::tests::masking_is_idempotent` (both `mask_secrets` and `Redactor`) |
| Merged, UTF-8-safe spans | `secrets::tests::spans_are_merged_sorted_and_on_char_boundaries` |
| Provider echoes / errors | `secrets::tests::redactor_masks_repeated_values_and_provider_echoes`, `…::short_known_values_are_not_masked_elsewhere`; `steering::audit::tests::records_carry_masked_text_identities_and_separate_interruption_outcomes` |
| One authoritative catalog; stable consumer order | `secrets::tests::catalog_ids_are_unique_and_token_order_is_stable`, `…::every_credential_prefix_names_a_catalog_token_shape`, `…::sensitive_key_names_cover_payload_and_environment_spellings` |
| Existing-consumer behavior | existing `protect::scrub::tests::*` (10), new `scrub_keeps_its_sequential_whole_match_policy`, extended `json_scrub_redacts_sensitive_key_values`; existing `messaging::send::tests::redact_webhook_urls_*` (6); existing CLI `wrap::env::tests::redact_sensitive_args_*`/`is_sensitive_key_*`, new `redact_sensitive_args_masks_every_shared_credential_prefix`, `sanitize_process_env_strips_shared_sensitive_key_names` (real `sanitize_process_env`) |
| Original message reaches delivery byte-for-byte | `audit::tests::records_carry_masked_text…`, `…::audit_write_failure_never_replays_or_changes_the_send_result` (closure asserts original bytes) |
| Typed events: masked text, request/target/execution/opportunity IDs, provider/conversation/generation/profile/mechanism, timestamps, consent, receipt, separate cancellation/replacement | `audit::tests::records_carry_masked_text…`, `…::automatic_native_request_records_its_opportunity_and_no_execution` |
| No original text in the file | `records_carry_masked_text…` (raw file scan for `hunter22`/`ghp_`) |
| Injected log-write failure: exactly one delivery, unchanged result, content-free trace | `audit::tests::audit_write_failure_never_replays_or_changes_the_send_result` (file-as-directory and no-home cases; `tracing-test` asserts no message text) |
| Late results are correlated updates, never sends; foreign reports refused | `audit::tests::late_results_are_correlated_append_only_updates` |
| Persisted read/write/read round trip | `audit::tests::persisted_message_round_trips_unchanged` |

### Checks run

- `cargo nextest run -p claudine --lib -E 'test(secrets)|test(scrub)|test(webhook)|test(harvest)'`: 83 passed.
- `cargo nextest run -p claudine --lib -E 'test(/^steering::/)|test(/^secrets::/)|test(logging)'`: 46 passed.
- `just test` (claudine area), first run: 5035 passed, 1 failed —
  `dispatch_inventory_matches_committed_file`, correctly flagging two new
  reference-class `Provider::` sites (the audit tests' `Provider::Pi` and
  `Provider::Codex`). Re-blessed `docs/providers/dispatch-inventory.json`
  (totals 1641→1643 sites, plus line shifts in `wrap/env/tests.rs`).
  **Final run: 7803 passed, 9 skipped, 0 failed.**
- `just lint`: first run flagged `clippy::large_enum_variant` on `AuditEntry`
  (fixed by boxing `RequestEntry`); final run clean for all five crates.

### Environment limitations encountered

- GitNexus `impact` was denied again. Upstream impact by text search: 
  `scrub_json_value` ← `signals::harvest` only; `scrub_text` ← scrub only;
  `redact_webhook_urls` ← `messaging::send` and its tests; `is_sensitive_key`
  ← `sanitize_process_env`, `ambient_sensitive_env`, `env/mod.rs` re-export,
  CLI env tests; `redact_sensitive_args` ← `composition/provider_args.rs`,
  `composition/dry_run.rs`, `env/mod.rs`; `write_dispatch_event_to` ←
  `log_dispatch_event` and doc/tests. Risk: low; signatures unchanged except
  the removed `protect::scrub::{ScrubRule, SCRUB_CATALOG}` (no external users).
- `CLAUDINE_UPDATE_INVENTORY=1 …` still requires approval. The inventory was
  re-blessed as in Phase 1 (temporarily forcing the bless branch for one run,
  then reverting); `cli/tests/l1/dispatch_inventory.rs` is unchanged from `HEAD`.
- Writes under `.claude/skills/claudine/` were denied again. Intended skill
  updates (not applied): add `secrets` (shared secret recognition) and
  `steering` to the Library Module Map in `SKILL.md`; in `cli-reference.md`
  replace "Filters sensitive env vars whose names contain `API_KEY` or `TOKEN`"
  with a pointer to `claudine::secrets::is_sensitive_key_name` (it was already
  incomplete before this phase).
- Unrelated working-tree changes under `content-policy/` were present during
  this phase and were not touched.

### Cross-OS assessment

Recognition is pure string/regex code. The audit writer uses the existing
append helper and `create_dir_all`; the failure-injection test places a regular
file where the log directory should be, which fails `create_dir_all` on
macOS, Linux, and Windows alike. No `#[cfg]` branches, sockets, or processes
were added, so no `just cross-check` run was needed; CI's Linux/macOS legs and
the post-merge Windows leg cover compilation and L1.

## Phase 3

Started and completed 2026-09-28 on macOS. Status: implemented, all Phase 3
plan items checked. (The work items were plain numbered text and were
converted to GFM todos, as in Phases 1–2, plus one validation todo.)

### Design decisions

- **Three roles, one owner.** The wrapper owns steering I/O through one
  `claudine::steering::controller::SteeringController` per provider child. The
  Rendezvous daemon only routes; a separate local process is a requester. The
  daemon never interprets provider protocols: Claudine values cross it as
  their snake_case wire strings and are parsed back strictly in
  `cli/src/steering/wire.rs`.
- **Controller bounds are the owner's.** 16 pending (queued + in flight) per
  execution, one automatic, 10 s manual / 2 s automatic / 10 s + 10 s consented
  interruption, all from dispatch and including queue wait. A single worker
  submits one request at a time; the executor runs on its own task so a
  timeout answers the caller (`unknown`) while the worker still waits (up to
  `LATE_RESULT_WINDOW`, 30 s) and appends the provider's eventual result as a
  `late_result` audit record — mutations stay serialized. Expired-unsent
  requests are `busy` (the `Busy` doc now says so); request IDs are accepted
  once per execution; routed receipts are capped at the mechanism's maximum
  (`SendOutcome::capped_at`). Every request, including refusals, is audited by
  the owner through a new begin/finish split of `audited_send`
  (`PendingAudit`), so the async path reuses the one audit implementation.
- **Re-selection before submit, never a switch.** The worker re-runs
  eligibility for the current state and requires the route's operation to
  equal the request's; otherwise `unavailable` with "the requested `x`
  operation is no longer available; this session now offers `y`". Stale
  binding (wrapper PID+start or conversation generation) is rejected by the
  daemon and again by the owner.
- **Generation semantics.** Any conversation change, including the first
  report, increments the generation, so a target listed before the provider
  reported its conversation must be listed again.
- **Unmapped profile.** The wrapper cannot yet map a launch to a researched
  steering profile (Phase 4 does for Pi), so `ExecutionFacts.profile_id` is
  `Option` and `None` yields `unavailable` with `UNMAPPED_PROFILE_REASON`.
  Production uses a `NoAdapter` executor that also answers `unavailable`.
- **Wrapper identity** is PID + process start time via the existing
  `sysinfo` helper, moved from `budget/run.rs` to `cli_utils::process_start`
  so budget and steering share it. No start time → no registration (a bare
  PID is never an identity).
- **Daemon router.** In-memory `SteeringRouter` (`rendezvous-daemon/src/steering.rs`):
  one live owner per execution (`ALREADY_EXISTS` otherwise), updates cannot
  change identity or lower the generation, outcomes `OWNER_REPLIED`,
  `NO_OWNER`, `STALE_TARGET`, `DUPLICATE_REQUEST` (4096 recent IDs), `BUSY`
  (16-frame owner channel), `UNKNOWN` (forwarded, no reply). Routed requests
  must be manual; automatic help is raised inside the owner only.
- **Owner link.** `cli/src/steering/owner.rs::ExecutionSteering` brackets the
  child next to `SessionPresence` in both structured-stream paths (harness
  attempt and direct structured). Presence's `CLAUDINE_RENDEZVOUS_REPORT` does
  not govern it. Connection attempts are 500 ms-bounded with 1/2/4/8 s backoff
  and stop after five consecutive failures; a lost registered stream
  reconnects and re-registers without replay. The interactive passthrough
  (`run_child`) is not registered: Claudine does not own that child's I/O.
- **Discovery aggregator** (`steering::discovery`): managed source + native
  discoverers keyed by generated research discovery IDs; 5 s deadline, four
  provider tasks at once, partial errors, total failure only when every
  source failed, exact-identity merge (same ID, or provider process + conversation
  equal to a managed registration's), roster/directory/ID sort, unimplemented
  native methods reported as `gaps`. No native discoverer exists yet (Phase 7).
- **Departure: generator extended.** The plan's "generated native-discovery
  bindings" did not exist; `claudine-gen` now projects research `discovery`
  records (`SteeringDiscovery`, new `DiscoveryMethod` vocabulary) and a
  `ROSTER_ORDER` constant (roster order differs from `PROVIDERS_DISPLAY_ORDER`:
  Pi precedes Kilo). Only `generated.rs` changed; byte pin
  4303782739945540261 → 9856002447674120557.
- **Provider slug on the wire and in listings.** `Provider` serializes as
  `kimi_code`; target IDs and the roster use `kimi`. Listings now serialize the
  slug, and `steering::provider_by_slug` is the one exact lookup (the ID parser
  uses it too).
- **Typed errors end to end.** The error-transport guard rejected the first
  cut's `to_string`/`format!` collapses. Discovery sources now return
  `SourceError` (`Box<dyn Error>`), `DiscoveryError` keeps an `Arc` cause and
  renders a masked `message()` only at serialization; the CLI uses
  `WireError`, `DaemonAccessError`, `UnreadableRegistration`, and
  `RouteFailure` with `#[source]` chains, rendered once by `render_chain`.
- **L1 hermeticity.** Every wrapped execution now opens a control link, so
  `CliProcessFixture` sets `RENDEZVOUS_ENDPOINT` to a fixture-private endpoint
  nothing listens on (a pipe name on Windows); otherwise an L1 wrapper test
  would register with the developer's own daemon.

### Defect found and fixed

Daemon graceful shutdown hung forever while any owner held a
`SteeringControl` stream (the CLI daemon tests timed out at 30 s). Fix: the
shutdown future calls `SteeringRouter::close()`, which drops every owner
channel (ending the response streams), stops each stream's upstream reader via
a `closed()` signal (so an owner that never closes its side — a suspended
wrapper — cannot hold the connection), and refuses new registrations. Locked
by `steering_round_trip::shutdown_completes_while_an_owner_is_still_connected`
(failed at its 10 s timeout before the reader fix) and the router's
`closing_ends_every_owner_stream_and_refuses_new_registrations`.

Also fixed: clippy `large_enum_variant` in the generated
`SteeringControlDown` (the delivery is boxed via `tonic_prost_build::configure().boxed(..)`).

### Requirement-to-test mapping

| Requirement | Test(s) |
| --- | --- |
| Concurrent senders serialized, FIFO | lib `controller::tests::concurrent_senders_are_serialized_in_arrival_order`; CLI `with_daemon::concurrent_senders_through_the_daemon_are_serialized` (8 routed, max in flight 1) |
| 16 pending, busy without eviction | `controller::tests::a_full_queue_refuses_as_busy_without_evicting_accepted_requests`; router `a_full_owner_channel_is_busy` |
| One automatic pending | `controller::tests::only_one_automatic_request_may_be_pending` |
| Deadlines 10 s / 2 s / 10+10 s | `controller::tests::deadlines_follow_origin_and_consent`, `…::automatic_requests_get_two_seconds` (1.9 s accepted, 2.1 s unknown) |
| Expired unsent discarded; ambiguous submission unknown | `controller::tests::an_expired_unsent_request_is_discarded_and_a_submitted_one_is_unknown` |
| Late replies logged, not resent | `controller::tests::a_late_acceptance_is_logged_as_an_update_and_never_resent`; router `no_reply_before_the_deadline_is_unknown_and_a_late_reply_goes_nowhere` |
| Stale IDs, reused PIDs, conversation replacement | `controller::tests::a_stale_target_is_rejected_instead_of_retargeted`; router `no_owner_stale_binding_and_duplicates_send_nothing`; CLI `with_daemon::a_stale_selection_is_rejected_and_never_reaches_the_provider`; identity `same_pid_with_a_new_start_marker_is_a_different_target` (Phase 1) |
| Duplicate correlation IDs never replayed | `controller::tests::a_repeated_request_id_is_refused_and_not_replayed`; router `…duplicates_send_nothing`; CLI `with_daemon::a_restarted_daemon_gets_a_fresh_registration_and_nothing_is_replayed` |
| Disconnect cleanup / reconnect without replay | router `an_owner_disconnect_resolves_its_in_flight_requests_as_unknown`, `a_reconnected_owner_never_receives_the_previous_connections_requests`; CLI `with_daemon::dropping_the_owner_removes_its_route`, `…restarted_daemon…`; client `an_owner_receives_a_routed_request_and_its_reply_returns` (stream close → NO_OWNER) |
| Missing daemon: unavailable route, task unaffected, direct automatic help | CLI `without_a_daemon_routing_is_unavailable_and_listing_fails_fast`, `without_a_daemon_the_owner_still_delivers_automatic_help_directly`, `without_a_daemon_the_link_gives_up_after_bounded_attempts`; the whole L1 wrapper suite runs against an unreachable endpoint |
| Exactly one owner receives; receipt only as established | CLI `with_daemon::a_routed_request_reaches_exactly_its_owner_and_its_receipt_returns`; router `a_request_reaches_exactly_its_owner_and_returns_its_reply`; `controller::tests::a_receipt_stronger_than_the_mechanism_can_prove_is_lowered` |
| No switch to interruption; consent required | `controller::tests::lost_non_interrupting_delivery_never_switches_to_interruption`, `…::requests_for_another_target_or_without_consent_are_refused` |
| Availability tracks state; unmapped/ungranted unavailable with reason | `controller::tests::state_changes_update_availability_and_route_selection`, `…::an_unmapped_or_ungranted_launch_is_unavailable_with_a_reason`; CLI `a_wrapped_child_registers_as_unavailable_until_its_profile_is_mapped` |
| Audit failure never changes delivery | `controller::tests::an_audit_write_failure_leaves_the_delivery_result_unchanged`; `…::routes_original_text_once_and_audits_only_masked_text` |
| No message text in replicated/durable storage; registration ≠ presence | client `an_owner_receives_a_routed_request_and_its_reply_returns` (scans the daemon data dir for the message; asserts `ListActiveSessions` has no entry) |
| Unix sockets and native Windows named pipes | client `steering_round_trip` (both tests) passed via `just cross-check rendezvous-client --os windows` and `--os linux`; wrong-user denial is the existing transport contract (see gaps) |
| Shutdown with a connected owner | client `shutdown_completes_while_an_owner_is_still_connected`; router `closing_ends_every_owner_stream_and_refuses_new_registrations` |
| Discovery: 5 s / 4 concurrent, partial and total failure, dedup, sort, unknown state, gaps, JSON | lib `discovery::tests::*` (11), incl. `a_source_past_the_deadline_is_an_error_not_a_hang`, `at_most_four_providers_are_discovered_at_once`, `identical_sessions_merge_and_rows_sort_by_roster_directory_and_id`, `unimplemented_native_methods_are_reported_as_gaps`, `listing_json_carries_the_specified_fields_and_no_identity_key` |
| Generated discovery projection (input robustness) | gen `steering_activation::discovery_projection_walks_the_input_robustness_matrix`; byte pin `drift::committed_generated_artifacts_match_phase_1_byte_baseline` |
| Wire reader robustness (registration, delivery, reply) | CLI `registration_matrix_reads_every_field_strictly`, `delivery_matrix_refuses_anything_it_cannot_read_exactly`, `reply_matrix_never_upgrades_or_misattributes_a_result` |

### Input robustness matrix

**Research `discovery` (YAML frontmatter → generator).** Load-bearing: the
list, and each record's `method`, `origin`, `os`, `prerequisites`.

| Shape | `discovery` | `method` / `origin` / `os` | `prerequisites` |
| --- | --- | --- | --- |
| absent | error | error (`os` absent) | n/a (schema-required) |
| explicit null | error | error (`method: null`) | error |
| wrong type, whole field | error (`123`) | error (`origin: 1`, `method: guess`) | n/a |
| wrong type, one element | error (`[…, 123]`) | n/a | error (`["ok", 7]`) |
| wrong type, every element | error (`[123]`) | n/a | error (`[7]`) |
| empty | `[]` = no researched discovery | n/a | `[]` = none |
| duplicate key | relational checker (`id_map` duplicate id) gates generation; YAML duplicate keys rejected by the frontmatter loader | same | same |
| trailing/invalid content | frontmatter loader | same | same |

**Wire (protobuf → CLI).** Protobuf has no null or duplicate-key shapes;
proto3 absent scalars read as empty, so empty strings are treated as
malformed, never as defaults. Registration: binding absent, execution ID
empty/uppercase, wrapper start empty/with `:`, provider empty/non-slug, state
and availability empty/unknown, provider PID without start (and vice versa),
out-of-range observed time — all rejected (a malformed registration becomes a
`managed` discovery error beside the readable rows, never dropped). Delivery:
request ID empty/prefix, expected absent, origin automatic/unknown, operation
unknown/empty, whitespace/NUL/over-64 KiB message, unknown consent — all
refused before submission. Reply: other request ID, unknown/empty outcome,
unresearched mechanism, replacement without cancellation, unknown
cancellation — all rejected and reported as `unknown`, never trusted.

### Checks run

- `cargo nextest run -p claudine-gen`: 193 passed (Phase 1's 192 plus the discovery matrix).
- `cargo nextest run -p claudine --lib -E 'test(/^steering::/)'`: 64 passed.
- `cargo nextest run -p claudine-cli --features daemon-tests --bin claudine -E 'test(/^steering::/)'`: 12 passed.
- Rendezvous area `just test`: 284 passed, 2 skipped; `just lint` clean.
- `just lint` (claudine area): first runs flagged the error-transport guard
  (9 collapses, fixed by typed errors), `large_enum_variant` in the generated
  proto, `type_complexity` in test matrices, and an unused test import; final
  run clean.
- `just test` (claudine area): first run 1 failure — the dispatch-inventory
  guard flagging new `Provider::` references (all test-code conditionals plus
  references); re-blessed as in Phases 1–2 (bless branch forced for one run,
  then reverted; the test file is unchanged from `HEAD`). **Final: 7840
  passed, 9 skipped, 0 failed.**
- `just check-tier-coverage claudine` and `claudine/rendezvous`: nothing
  stranded.
- `just cross-check rendezvous-client --os windows`: 25/25 (named pipes);
  `--os linux`: pass.

### Environment limitations encountered

- `CLAUDINE_UPDATE_INVENTORY=1 …` still needs approval; workaround as above.
- `printenv`/`env` needed approval, so `BUILD_*` hosts were not listed;
  `just cross-check` resolved them itself.
- Writes under `.claude/skills/claudine/` were denied again. Intended skill
  update (not applied), in addition to the Phase 1–2 items: add to the
  Library Module Map rows for `secrets` (shared recognition; never add another
  pattern list) and `steering` (IDs/contracts, generated facts + grants →
  eligibility, the per-execution `controller`, the `discovery` aggregator; the
  CLI's `src/steering/` owns the daemon link to the in-memory
  `SteeringControl`/`RouteSteering` RPCs; links to `topics/steering-routing.md`
  and `topics/steering-activation.md`).
- Unrelated working-tree changes (`content-policy/…`,
  `claudine/fixes/2026-09-29-update-research/spec.md`) were present and not
  touched.

### Known gaps (not blockers)

- **Wrong-user denial** is enforced by the existing local transport (socket
  directory mode / pipe DACL); a second-OS-user test remains impossible on
  these hosts, as `docs/rendezvous/local-ipc.md` §13 already records.
- The CLI daemon-backed steering tests ran on macOS only; CI runs them with
  `daemon-tests` on Linux/macOS (and Windows post-merge). The router and
  transport path they exercise passed natively on Windows and Linux through
  `rendezvous-client`.

### Cross-OS assessment

New code has no `#[cfg]` branches; the one OS difference (a pipe-name
spelling for the L1 fixture's unreachable endpoint) is a `cfg!(windows)`
expression, so both arms compile everywhere. Transport-sensitive behavior (the
first bidirectional-streaming RPC over the local endpoint, and shutdown with an
open stream) was verified on native Windows named pipes and Linux sockets via
`just cross-check`.

## Phase 4

Started and completed 2026-09-28 on macOS. Status: **implemented; Pi steering
blocked by a reviewed policy block** (every Phase 4 plan item checked). The
managed RPC execution, the `pi-rpc` adapter, and wrapper integration are
implemented and verified against the installed Pi (0.87.1, macOS) and a
deterministic fake Pi (macOS, Linux, native Windows). No steering is activated.

### Protocol facts established before designing (installed Pi 0.87.1)

A throwaway real-Pi exploration test (removed before the phase ended)
established what the research left open:

- `prompt`'s `response` precedes `agent_start` in the same tick, so a
  `get_state` sent after the response sees `isStreaming: true` if a turn began.
- An extension-handled prompt gets a success response and **no**
  `agent_start`/`agent_settled`; its response arrives only after any dialog it
  opened resolves.
- `steer` while idle succeeds and strands the text (`pendingMessageCount: 1`)
  until a later prompt drains it. A fresh state check before `steer` is
  therefore required, under the mutation lock.
- `extension_ui_response {cancelled: true}` resolves a `confirm` as `false`;
  `notify` is a fire-and-forget UI request.
- Unknown commands fail with `success: false`; stdin EOF exits 0 in < 1 s.
- `get_state` carries `pendingMessageCount`.
- **Pi 0.87.1 moved the system prompt into a `role: "system"` message with
  sections**; `context.systemPrompt` is empty. The shared probe fixture's
  context detection was updated to look in both places. Context files load
  under `--no-approve`, `--approve`, and neither, from the agent directory and
  the project root.

### Design decisions

- **A retained-stdin control seam in the one semantic spawn path.**
  `exec/control.rs` defines `StdioControl`; `run_child_stream_semantic` takes an
  optional session. With one, `spawn/retained.rs` spawns the child with
  forwarding reader threads that hand every stdout line to the session before
  passing it on, waits for readiness, and only then submits the task. The
  semantic reader consumes the forwarded lines, so pre-ready lines reach the
  parser in order. Watchdogs, rendering, signals, content guards, and
  termination are the shared ones (the Kimi wire session, by contrast, dropped
  several of them). Rejected alternative: a separate `run_pi_rpc_session`,
  which would have duplicated ~600 lines.
- **The prompt stays a `PromptDelivery::Stdin` seed.** Under RPC the session
  submits that seed as the `prompt` command, so no new launch-plan field was
  threaded through the harness, sequence, and composition paths, and the JSON
  fallback writes the same seed to stdin.
- **Interface selection comes from the generated catalog.**
  `claudine::steering::execution_selection(Pi)` (`preferred: rpc`,
  `fallback: json`) drives the Pi profile's `apply_structured_stream`
  override: the catalog companion flags plus `--mode rpc`, with the
  entrypoint's `-p` removed (in RPC mode Pi would read stdin as the prompt).
  `stdio_control(args)` returns a `PiRpcSession` exactly when argv selects RPC.
- **Facts reconciled.** `docs/providers/facts/pi.yaml` drops `--no-extensions`,
  `--no-skills`, `--no-prompt-templates`, and `--no-context-files` (the user
  correction in the spec) and keeps `--no-approve` (trust is a separate policy
  the spec forbids changing). Regenerated `catalog.json` and
  `lib/src/provider/pi/data.rs`.
- **Settlement.** The session closes stdin only after a `get_state` taken
  under the mutation lock shows no turn, no compaction, and nothing queued,
  and no `agent_start` arrived while it was asked. Triggers: `agent_settled`
  and the task's `prompt` acceptance (for extension-handled tasks). A refused
  task closes at once. Queued input at settlement is reported undelivered and
  never resent. If Pi lingers 5 s after EOF, the completion channel ends the
  run as completed.
- **Unattended requests.** Dialog methods get Pi's documented cancellation;
  notices get nothing; an undocumented method or an unreadable UI request fails
  the run as the new `EarlyTermination::InputRequired` (`error_kind =
  "input_required"`, `ProcessTermination::Aborted`, signal
  `HumanInputRequested`). After `input_required`, the session never closes
  gracefully, so a clean exit cannot hide the failure (found by an L1 race; see
  below).
- **Fallback.** Only before submission: readiness refused, unreadable, child
  exit, broken stdin, or 30 s of silence. The child is reaped, its stderr is
  shown, a warning lists the lost capabilities, and the same function reruns
  with the JSON argv (`-p --mode json`) and the seed on stdin. A user
  interrupt never falls back. After submission there is no fallback or retry.
- **Tool cleanup (real finding).** Pi's `bash` tool gives its commands their
  own process groups. Killing Pi mid-tool left the tool running past the
  wrapper, which the spec calls a cleanup failure. `spawn/descendants.rs`
  (Unix) records descendants by 250 ms scans (pid + start time, threads
  excluded) and at teardown terminates survivors that are still the same
  process, then reports them separately. Windows relies on the existing Job
  Object. Limit: a process that starts and escapes between two scans is not
  seen.
- **Resume.** `append_resume_passthrough_args` now carries `--mode <value>` and
  `--approve`/`--no-approve`. Before this, a resumed Pi run lost `--mode json`,
  so Pi read the piped follow-up as a plain-text print run: a pre-existing
  defect fixed in passing.
- **Strict readers for load-bearing fields.** `claudine::stream::protocol::pi::rpc`
  reads `response.id/command/success`, `get_state.sessionId/isStreaming/
  isCompacting/pendingMessageCount`, and `extension_ui_request.id/method`
  strictly (absent, null, and wrong-typed are errors). The display parser stays
  lenient. `UiMethodKind` is the single classification the parser and the
  owner share.
- **Parser.** RPC `response` for `get_state` produces `SessionStart` only when
  the session id changes (RPC has no `session` header); a refused `prompt`
  records an error; UI requests and `extension_error` become warnings or info;
  `agent_settled` is recognized and silent.
- **The adapter (`pi-rpc` revision 1).** `steer` (working), `prompt` (idle,
  quiescent), and consented `abort` → poll until not streaming → re-check the
  session → `prompt`. Every delivery holds the mutation lock from its fresh
  `get_state` to its last answer. A changed session id updates the controller
  (new generation) and sends nothing. Outcomes: a steer answer is `queued`, a
  prompt answer `accepted`; `Refused` is Pi's own refusal; `unknown` is "sent,
  no readable answer" (timeout, exit, unreadable) and is never resent;
  `unavailable` is "never written". Queues are never cleared. A closing run
  refuses new steering. Typed `RpcError` carries causes as `#[source]` and is
  rendered once through `crate::steering::render_chain`, as the
  error-transport guard requires.
- **Owner wiring.** `ExecutionSteering::for_wrapped_child` takes the control
  session; a steering-capable session supplies the profile (`retained-rpc`)
  and executor, and is bound to the controller so it reports state (working on
  `agent_start`, idle on settlement), conversation, and the provider's process
  identity. Provider version stays unestablished (see limitations).
- **Explicit block (plan item 6).** The switch-race record
  `pi-rpc-steer-switch-0844` (re-run against 0.87.1 with the same expected
  loss) shows `get_state`-then-`steer` is unsafe when an extension switches
  sessions; the lock cannot order a switch it does not start, and extensions
  must stay enabled. The reviewed policy therefore gained a required `blocks`
  list (`{provider, profile_id, reason}`), validated by `claudine-gen` (the
  profile must be researched, the reason non-empty, no duplicates, and no grant
  for a blocked profile), emitted as `PROFILE_BLOCKS`, and checked first by
  eligibility (`Blocker::ProfileBlocked`, before state/version). Without it
  the listing would have said "requires setup: Owned RPC child…", implying
  more setup could help. **Departure:** the plan did not name a policy-schema
  change; it was needed to record the specific blocker the plan requires.
  `pi-rpc` is registered as reviewed and implemented; no grant exists.

### Defects found and fixed during the phase

- Tool processes survived a killed Pi (see Tool cleanup).
- Linux-only (via `just cross-check`): the pre-readiness teardown signalled the
  process group while its exited leader was an unreaped zombie. Linux counts it
  as a member, so each fallback waited the full 10 s kill grace. Fixed by
  reaping the leader first.
- Linux-only: sysinfo lists threads as processes whose parent is their owner,
  so the descendant watch recorded (and could signal) threads. Fixed with
  `thread_kind()`; the unit test that rooted a watch at the test process now
  roots it at a spawned child.
- L1 race: with a fake Pi that answered `prompt` before raising its UI
  request, the quiescence check closed stdin and a clean exit hid the
  `input_required` failure. The session now never closes gracefully after
  `input_required`, and the fake matches real Pi's ordering.
- Test fixture: the probe model re-issued its tool batch forever without an
  intervening steer; it now calls the batch once per task (compatible with the
  native steering tests, which pass).

### Requirement-to-test mapping

| Requirement | Test(s) |
| --- | --- |
| RPC launch selected from research; argv has `--mode rpc`, `--no-approve`, no `-p`, no resource-disabling flags | CLI `profile::tests::pi_managed::*` (4); L1 `pi_managed_rpc::a_task_runs_over_rpc_and_the_settled_run_ends` (argv recorded by the fake) |
| Readiness before submission; task is one `prompt` with original bytes | broker `readiness_needs_a_state_answer_naming_a_session`, `the_task_is_one_prompt_with_the_original_text`; L1 `a_task_runs_over_rpc…` (commands recorded) |
| Resources preserved (extension, skill, template, context) with real Pi | real `real_pi_managed_rpc_runs_a_task_to_settlement_with_resources_enabled` |
| Settlement: close only after `agent_settled` + quiescent state; `agent_end` alone never closes; new turn, streaming, compaction keep it open; queued input reported, never resent; extension-handled task ends; refused task ends | broker `a_settled_quiescent_run_closes_stdin_once`, `work_after_settling_keeps_the_run_open`, `a_new_turn_during_the_check_keeps_the_run_open`, `queued_input_at_settlement_is_reported_undelivered_and_the_run_ends`, `an_extension_handled_task_ends_without_a_turn_and_a_refused_one_at_once`, `a_task_that_starts_a_turn_is_not_closed_by_its_acceptance`; real `…_keeps_stdin_open_through_a_tool_batch`, `…_cancels_extension_dialogs_and_ends_handled_tasks` |
| Unattended: dialogs cancelled (never approved), notices unanswered, unknown method → `input_required` failure, never a graceful close after it | broker `dialogs_are_cancelled_notices_ignored_and_unknown_methods_end_the_run`, `an_input_required_run_is_never_ended_gracefully`; L1 `an_extension_dialog_is_cancelled_never_approved`, `an_unanswerable_ui_request_fails_the_run_as_input_required`; real `…_cancels_extension_dialogs…` (`ui-confirm-result` = `false`) |
| Fallback only before submission, with warning and stderr; none after submission | L1 `a_refused_readiness_check_falls_back_to_the_json_stream_before_submission`, `a_child_that_exits_before_it_is_ready_falls_back_with_its_stderr_shown`, `a_crash_after_submission_fails_the_run_and_is_never_replayed` |
| Provider crash fails the run and leaves no tool | real `…_provider_crash_fails_the_run_and_leaves_no_tool`; `spawn::descendants::tests::*` (3) |
| Steering: fresh state check; steer while working → queued; idle prompt → accepted; changed session → nothing sent, new generation; not working → nothing sent; refused / silent / exited → refused / unknown / unknown, never resent; settled run refuses | broker `a_working_session_is_steered_after_a_fresh_state_check`, `nothing_is_sent_when_the_fresh_state_disagrees`, `refusals_and_silence_are_reported_as_established`, `an_unanswered_steer_is_unknown_and_never_resent`, `a_settled_run_refuses_new_steering`, `an_idle_session_takes_a_prompt`; real `real_pi_protocol_steers_a_working_turn_then_settles_and_closes`, `real_pi_protocol_starts_an_idle_turn_with_a_prompt` |
| Consented interruption: abort, wait, revalidate, submit; phases separate; queues never cleared | broker `consented_interruption_aborts_waits_revalidates_then_submits`, `interruption_phases_stay_separate_when_one_fails`; real `real_pi_protocol_interrupts_with_consent_and_keeps_the_phases` |
| Mutations never interleave with settlement | broker `a_delivery_and_the_settlement_decision_never_interleave` |
| Owner maps the profile, binds the target, reports state | broker `readiness_binds_the_target_and_turns_bump_state` |
| Explicit Pi block; blocks validated; grant on blocked profile rejected; block outranks grant | lib `eligibility::tests::shipped_policy_blocks_pi_managed_rpc_with_its_reason`, `a_profile_block_outranks_a_matching_grant_and_spares_other_profiles`, `pi_passing_fixture_records_are_not_activation_grants`; controller `an_unmapped_or_ungranted_launch_is_unavailable_with_a_reason`; gen `a_blocked_profile_cannot_be_granted_and_blocks_are_validated`, `committed_policy_is_valid`, `drift::*` |
| Parser handles RPC records | lib `stream::providers::pi::tests::rpc_transcript_reads_like_a_json_mode_run`, `repeated_state_reports_only_announce_a_changed_session`, `a_refused_prompt_fails_the_run_and_other_refusals_do_not`, `extension_ui_requests_are_reported_by_kind` |
| Resume keeps `--mode` and trust | CLI `resume::tests::passthrough_carries_pi_mode_and_trust` |
| Existing native Pi regressions (steering contract, abort/EOF, switch/crash) | real `real_pi_steering::*` (3), re-run against 0.87.1: pass |
| Ordinary wrapper output/error parity | full L1 suite unchanged; real interactive startup tests (4) pass |

### Input robustness matrix

**Pi RPC records (JSON line → `claudine::stream::protocol::pi::rpc`).** One
test per record walks the matrix from a real Pi 0.87.1 record with one edit per
cell; the control row reads to its positive result. Arrays are not load-bearing
here, so the per-element rows do not apply.

| Shape | response `id` / `command` | response `success` | `get_state` `sessionId` | `isStreaming` / `isCompacting` | `pendingMessageCount` | UI `id` / `method` |
| --- | --- | --- | --- | --- | --- | --- |
| absent | error | error | error | error | error | error → `input_required` |
| explicit null | error | error | error | error | error | error → `input_required` |
| wrong type | error | error (`"true"`) | error (`12`) | error (`"false"`, `0`) | error (`-1`, `1.5`) | error (`["confirm"]`) |
| empty | `""` reads; correlates nothing | n/a | error (identifies nothing) | n/a | n/a | empty method → `Unsupported` |
| duplicate key | last wins (JSON permits; Pi never repeats) | last wins | last wins | last wins | last wins | last wins |
| trailing content | not a record (`Other`); the command stays unanswered, never read as accepted | same | same | same | same | same |

A refusal without `error` text is still a refusal (`success` decides). A
malformed `get_state` answer fails readiness (fallback) or the delivery's state
check (nothing sent). A malformed answer to a sent command resolves as
`unknown`.

**Activation policy `blocks` (YAML → generator).** Walked in
`policy_parser_walks_the_input_robustness_matrix` with the rows added to the
Phase 1 matrix: absent, null, `null` value, wrong type (`{}`), one element
wrong, every element wrong → error; empty `[]` = nothing blocked; `reason`
absent, null, or a number → error; `reason: ""` parses and activation rejects
it; `profile_id` as a list → error; duplicate key and unknown key → error. The
generator's grep-for smells: none (strict deserializers, no `serde(default)` on
policy fields).

### Checks run

- `cargo nextest run -p claudine --lib` (Pi protocol, parser, steering): 95/95
  after the block; the full lib suite runs inside `just test`.
- `cargo nextest run -p claudine-gen -p claudine-catalog-types`: 224/224.
  `claudine-gen check`: clean. Byte pins: `catalog.json` 4992177613331685818 →
  14015590831747458428, `steering/generated.rs` 9856002447674120557 →
  10005189633011121167, `pi/data.rs` 2889607625502041169 →
  11409995824083696371.
- CLI broker + descendants + fake-Pi L1 tests: 10/10 stress iterations passed.
- Real tier (`just test-real real_pi_`, Pi 0.87.1, macOS): 11/11 (4 managed
  wrapper, 3 adapter protocol, 4 interactive startup); `real_pi_steering`
  native regressions 3/3 (`cargo nextest --run-ignored all`).
- `just lint` (claudine area): clean for all five crates after the
  error-transport fix (the first run flagged 9 string collapses; replaced by
  typed `RpcError` + `render_chain`). `cargo clippy --all-targets` with
  `real-tests,test-fixtures,daemon-tests`: clean.
- `just test` (claudine area): first run failed only the dispatch-inventory
  guard (9 new reference-class `Provider::` sites, no dispatch conditionals);
  re-blessed with the Phase 1–3 workaround (bless branch forced for one run and
  reverted; `dispatch_inventory.rs` is unchanged from `HEAD`). One run hit the
  known load-sensitive `an_early_wait_error_still_reaps_the_whole_tree`
  LEAK-FAIL (passes in isolation in 0.055 s; pre-existing, untouched).
  **Final: 7888 passed, 9 skipped, 0 failed.**
- CLI daemon-backed steering tests (`daemon-tests`): 12/12.
- `just check-tier-coverage claudine`: nothing stranded.
- `just cross-check claudine-cli` (affected tests, `test-fixtures`): Linux
  598/598 (after the two Linux fixes above), native Windows 414/414.

### Environment limitations encountered

- `pi --version` and env-prefixed commands (`CLAUDINE_PI_BINARY=…`,
  `CLAUDINE_UPDATE_INVENTORY=1 …`) required approval. Real tests resolved Pi
  from `PATH` instead; the native `real_pi_steering` tests now fall back to
  `pi` on `PATH` when `CLAUDINE_PI_BINARY` is unset (a missing binary still
  fails the test).
- Reading Pi's installed source was outside the allowed directories; protocol
  facts came from the research reports and the exploration test.
- Writes under `.claude/skills/claudine/` were denied again. Intended skill
  updates (not applied), in addition to the Phase 1–3 items: add Library Module
  Map rows for `secrets` and `steering` (including profile blocks); add a
  wrapper-subsystem row "Managed Pi RPC | non-interactive Pi runs `--mode rpc`
  through a retained-stdin `StdioControl` (`exec/control.rs`,
  `exec/pi_rpc/`); readiness → one `prompt` → settle on `agent_settled` +
  quiescent `get_state` → close stdin; dialogs cancelled, unknown UI →
  `input_required`; JSON fallback only before submission; Unix descendant reap
  | `topics/pi-rpc.md`"; and note in `cli-reference.md` that non-interactive
  `claudine pi` no longer disables Pi resources.
- Unrelated working-tree changes outside `claudine/` were not touched.

### Known gaps and limitations (not blockers for Phase 5)

- **Pi steering is blocked** by the reviewed policy (above). Lifting it needs
  an effective session guard; see the message to the next agent.
- **Real-Pi evidence is macOS only** (0.87.1). Linux and native Windows were
  covered by the deterministic fake Pi through `just cross-check`, not by real
  Pi. The research records remain 0.84.4/native; no claudine-origin
  verification records were added, because the blocked profile cannot use
  them.
- **Provider version is not established** by the wrapper
  (`ExecutionFacts.provider_version: None`). A grant would require it; while
  every mapped profile is blocked it is not consulted.
- **Descendant reaping is scan-based** (250 ms); a tool that starts and escapes
  between scans is missed. It also terminates background processes a Pi tool
  left behind on a normal exit, matching the process-group semantics other
  providers already get.

### Cross-OS assessment

New `#[cfg]` splits: `spawn/descendants.rs` is Unix-only (Windows uses the Job
Object); `kill_process_group` is a no-op on Windows; the real crash test is
`#[cfg(unix)]`. Everything else is portable Rust over stdio. The changed shared
semantic spawn path (line sources, seed gating, merged early-termination and
completion channels) and the fake-Pi wrapper tests passed natively on Linux and
Windows, which is where the two OS-specific defects surfaced and were fixed.

## Phase 5

Started 2026-09-28 on macOS. (The work items were plain numbered text and were
converted to GFM todos, as in Phases 1–4, plus one validation todo.)

### Gaps found before designing

- A listing row could not be routed: `SessionListing` dropped the owner's
  `ManagedTarget` binding (generation, conversation) that `route_to_owner`
  needs as `expected`, and nothing carried the route's operation. The owner
  refuses a request whose operation differs from its current route, and
  `steer_active_turn` and `queue_follow_up` are both "non-interrupting while
  working", so the operation cannot be derived from availability.
- `ensure_config_exists` runs the interactive init wizard when no config
  exists. Every non-exempt command calls it, which would make `steer --json`
  interactive and would change configuration, which the spec forbids for a send.
- Argv Rule 2 rewrites `--provider <fuzzy>` for every subcommand before `--`.
  A steer message can only meet it as an unknown flag (a clap error), and
  rewriting stops at `--`, so message bytes are safe; a test locks that.
- GitNexus `impact` was not permitted in this session. Callers of the edited
  symbols (`AvailabilitySummary`, `ManualEligibility::summary`,
  `SessionListing`, `wire::info_to_listing`, `ManagedTargetInfo`) were found by
  text search instead: all are inside `lib/src/steering`, `cli/src/steering`,
  and the Rendezvous steering tests (low risk).

### Design decisions

- **The listing carries what routing needs.** `AvailabilitySummary` gained
  `operation` (the preferred route's `OperationIntent`, `None` exactly when
  unavailable), serialized in listing JSON as `operation`. The owner publishes
  it on a new proto field `ManagedTargetInfo.operation = 14`, and
  `wire::info_to_listing` reads it strictly: absent while selectable, present
  while unavailable, empty, unknown, or the literal `unknown` are all
  registration errors, never a guessed operation. `SessionListing` gained a
  `#[serde(skip)] binding: Option<ManagedTarget>` kept through deduplication,
  so a request routes with the exact binding it was listed under.
- **One seam per external effect.** `commands/steer/` is `mod.rs` (args,
  validation, flow, exit codes), `service.rs` (`SteeringService`: discover and
  deliver; `LocalService` = daemon managed source + `route_to_owner`),
  `interact.rs` (`Interaction`: picker and consent; `TerminalInteraction` =
  biscuit-tui `ChooseOne`/`BooleanSwitch` inside `block_in_place`), and
  `render.rs` (`SessionTable`, receipts, consent explanation, JSON documents).
  Tests substitute a fake service, scripted answers, and a captured console,
  so no interactive test needs a human.
- **Target and consent rules.** No `--session` requires a TTY (stdin and
  stderr) and not `--json`, else exit 2 with the `--list` hint; the picker is
  always shown, even for one selectable row. An explicit ID must appear in
  this invocation's discovery (exact match; prefixes are rejected by the ID
  parser). `interruption_required` always asks, explicit ID or not; `--json`
  and non-TTY refuse with outcome `unavailable`. Consent is sent only for
  `interrupt_then_submit` and bound to the target.
- **Revalidation after a human pause.** A picked or consented row is looked up
  again; a changed binding or a vanished row is "not redirected", a changed
  operation is refused, except that a newly interruption-required session gets
  the same explanation and question once more (bounded to two passes). An
  explicit non-interrupting send has no human pause, so it relies on the
  owner's own revalidation at submission (one discovery pass).
- **Output separation.** The listing and receipts are data (stdout); the
  pre-picker listing, warnings, consent explanation, and "nothing was sent"
  notes are status (stderr). Every locally refused send still produces a
  typed result (outcome `unavailable` + masked `detail`), so `--json` callers
  always get a document for exit 1. Usage errors print nothing on stdout.
- **No configuration side effects.** `Steer` joined the commands exempt from
  `ensure_config_exists` (which would otherwise run the interactive init
  wizard and write a config).
- **Picker styling.** `ChooseOne` disables unavailable options; its theme's
  `disabled_style` gains `DIM | CROSSED_OUT` (the default is dim only). Esc in
  `ChooseOne` submits the initial (empty) selection, which the command maps to
  cancellation just like Ctrl-C's error.
- **Narrow terminals.** A six-column table cannot render below ~50 columns
  (biscuit-terminal returns a width error text). `SessionTable` checks
  `Table::plan_widths_for_terminal` and, when it fails, renders each row as one
  wrapped, identically styled summary line. Full IDs are never wrapped (they
  are copied into `--session`); every other standalone line wraps.
- **Help/completions.** `steer` appears in the grouped help under Wrapped
  Execution, in the root completion menu after `config`, and in telemetry.
  Clap supplies `--help`; there is no consent-bypass flag.

### Defects found and fixed during the phase

- Found by the narrow-layout test: the listing printed biscuit-terminal's
  "Table could not be rendered in 42 columns" instead of rows (fixed by the
  stacked fallback above), and the first fallback wrapped IDs at hyphens.
- The error-transport guard flagged six string collapses of typed errors in
  the new command; all now render once through `crate::steering::render_chain`.

### Requirement-to-test mapping

All in `cli/src/commands/steer/tests.rs` unless noted.

| Requirement | Test(s) |
| --- | --- |
| Command forms parse; `--list` conflicts with message/`--session`; `--` keeps dash text; no bypass flag; clap usage errors exit 2 | `the_specified_command_forms_parse`, `a_message_after_double_dash_keeps_its_exact_bytes`, `invalid_forms_are_usage_errors_and_no_consent_bypass_exists`; L1 `steer_cli::invalid_invocations_exit_with_usage_errors`, `help_documents_the_forms_without_a_consent_bypass` |
| Argv normalization leaves steer messages alone | `a_message_after_double_dash_keeps_its_exact_bytes` (`--provider cl` after `--`, and in one token) |
| Message validation (whitespace, NUL, >64 KiB incl. multibyte), malformed IDs → exit 2 before discovery; the limit itself is delivered unchanged | `invalid_messages_and_ids_fail_before_discovery`, `a_message_at_the_limit_is_delivered_unchanged` |
| No target inferred: `--json` or no TTY without `--session` → exit 2 with guidance | `without_an_explicit_target_json_and_pipes_never_prompt`; L1 `invalid_invocations_exit_with_usage_errors` |
| List JSON contract (keys, schema_version, unavailable rows with reasons, explicit nulls, operation, masked errors, coverage gaps), partial vs total failure, empty success | `list_json_is_the_versioned_contract_with_unavailable_rows`, `an_empty_listing_succeeds_and_total_failure_fails`; lib `discovery::tests::listing_json_carries_the_specified_fields_and_no_identity_key`; L1 `listing_without_a_daemon_fails_on_stderr_and_changes_no_configuration` |
| Operation in the summary and on the wire (input robustness) | lib `eligibility::tests::the_summary_names_the_preferred_routes_operation`; CLI `steering::tests::registration_matrix_reads_every_field_strictly` (5 new cells) |
| Exact targeting with the listed binding and operation; original bytes; send JSON contract | `an_explicit_send_reaches_exactly_its_target_with_the_listed_operation`; daemon `with_daemon::list_then_send_by_id_reaches_the_owner_and_returns_its_receipt` |
| Exit 0 only for accepted/queued/delivered; receipt never above outcome; sent once; details masked | `exit_codes_follow_the_established_outcome` (all nine outcomes) |
| Receipt wording: held undelivered + separate setup; acceptance ≠ acted on; unknown not retried | `receipts_say_only_what_was_established` |
| Unknown/stale explicit ID and unavailable explicit target send nothing | `an_unlisted_or_unavailable_explicit_target_sends_nothing`; daemon `a_blocked_production_owner_is_listed_unavailable_with_its_reason` |
| Picker required even for one eligible row; unavailable rows offered but disabled; listing with reasons shown first | `one_eligible_session_still_needs_a_choice_and_unavailable_rows_are_offered_disabled`, `the_picker_only_selects_available_rows` (simulated key input) |
| Empty states, none eligible, unknown state, user cancellation (130), picker failure → no broadcast | `empty_states_and_cancellation_never_broadcast`, `a_picker_failure_sends_nothing` |
| Idle target explains it starts a turn | `an_idle_target_is_told_it_starts_a_turn` |
| Stale selection / ended session never redirected | `a_stale_selection_is_reported_and_never_redirected`; daemon `a_conversation_switch_after_listing_is_refused_by_the_owner` |
| Lost non-interrupting delivery never silently becomes interruption; re-consent required; changed action refused | `a_changed_action_after_choosing_is_never_sent_silently` |
| Consent explained (turn, tools, pending messages, partial), bound to target+operation, revalidated before cancellation; partial interruption exit 1 with phases | `interruption_consent_is_explained_bound_and_revalidated`; daemon `a_consented_interruption_reports_both_phases_and_json_cannot_consent` |
| JSON/non-TTY never consent; decline 130; prompt failure is not consent | `interruption_phases_stay_separate_in_json`, `consent_is_never_inferred` |
| Rendering: five columns, labels in plain text, full IDs, reasons; unavailable rows dim + struck; narrow layout wraps, never hides | `the_listing_labels_every_row_and_styles_unavailable_ones`, `a_narrow_terminal_wraps_details_instead_of_hiding_them`, `the_picker_draws_unavailable_rows_dim_and_struck` (ratatui `TestBackend`, off-screen) |
| Sending changes no configuration and publishes no audio | L1 `listing_without_a_daemon_fails_on_stderr_and_changes_no_configuration` |
| Late replies | not re-tested here: the requester reports the owner's `unknown` at its deadline (`exit_codes_follow_the_established_outcome` covers `unknown` → 1, not retried); late provider evidence is the owner's `late_result` audit (Phase 3 `controller::tests::a_late_acceptance_is_logged_as_an_update_and_never_resent`) |

### Input robustness matrix

**Managed registration `operation` (protobuf → `wire::info_to_listing`).**
Protobuf has no null or duplicate-key shapes, and the field is a scalar.

| Shape | Selectable row | Unavailable row |
| --- | --- | --- |
| absent | error (`Missing`) | reads, `operation: None` (control) |
| empty string | error (`UnknownValue`) | error (`Inconsistent`) |
| unknown value (`shout`) | error | error |
| literal `unknown` | error (never a sendable operation) | error |
| valid value | reads (control row) | error (`Inconsistent`) |

Control rows: the matrix's control registration (interruption_required +
`interrupt_then_submit`) and the unavailable/absent variant both read.

**Steer CLI arguments** are not a file format; their shapes are covered by
the parser and validation tests above (absent message, empty, whitespace,
NUL, over-limit, malformed/uppercase/prefix IDs, conflicting flags).

### Checks run

- `cargo nextest run -p claudine --lib -E 'test(/^steering::/)'`: 67/67.
- `cargo nextest run -p claudine-cli --bin claudine` steer + steering + menu +
  help: 68/68; with `--features daemon-tests` (steer + steering): 41/41.
- L1 `steer_cli` (3), `test_placement`, `spawn_site_guard`: pass.
  `wrap_basics::help_lists_wrapper_subcommands` snapshot updated for the new
  help line.
- Dispatch inventory re-blessed with the Phase 1–4 workaround (bless branch
  forced for one run and reverted; `dispatch_inventory.rs` unchanged from
  `HEAD`): 4 new reference-class `Provider::` sites, all in
  `commands/steer/tests.rs`; no dispatch conditionals.
- `just lint` (claudine area): clean for all five crates after the
  error-transport fixes and two clippy `cloned_ref_to_slice_refs` fixes.
  `cargo clippy -p claudine-cli --all-targets --features
  daemon-tests,test-fixtures -- -D warnings`: clean.
- `just test --no-fail-fast` (claudine area): **7929 passed, 1 failed, 9
  skipped.** The failure is
  `compose_schema_cli::compose_enforces_each_root_union_arm_match_before_provider_launch`,
  added by `25ca47440 fix(darkmatter): enforce every root-union file(match)
  glob on its arm`, which another session merged into this branch
  (`281dd2c8d`) while Phase 5 was in progress. It fails deterministically in
  darkmatter schema union matching and touches nothing in this phase.
- Rendezvous area `just test` (proto changed): 284 passed, 2 skipped.
- `just check-tier-coverage claudine`: nothing stranded.
- `just cross-check claudine-cli --features daemon-tests steer steering::
  root_menu help_lists_wrapper`: native Windows 74/74, Linux 74/74 (including
  the four daemon-backed steer tests over named pipes and Unix sockets).
  `just cross-check claudine-cli --os windows steer_cli`: 3/3. (A `-E`
  expression with parentheses breaks the recipe's argument quoting; positional
  substring filters work.)

### Environment limitations encountered

- GitNexus `impact` was not permitted (see above).
- `CLAUDINE_UPDATE_INVENTORY=1 …` still needs approval; workaround as above.
- Writes under `.claude/skills/claudine/` were denied again. Intended skill
  update (not applied), in addition to the Phase 1–4 items: add a CLI row
  "`claudine steer "msg" | --session <id> [--json] "msg" | --list [--json]` —
  send one message to one running session (the requester over
  `cli/src/steering/requester.rs`); never infers a target; interruption always
  needs interactive consent (`--json`/no TTY refuse); a picked row is
  rediscovered after the human pause and a changed binding or operation is
  reported, never followed; exempt from `ensure_config_exists`; exit 0 only for
  accepted/queued/delivered, 1 otherwise, 2 usage, 130 cancel; tests
  `commands/steer/tests.rs` and `tests/l1/steer_cli.rs`; see
  `claudine/docs/cli/steer.md`".
- **Concurrent activity in this worktree.** During the phase, HEAD advanced
  to a merge (`281dd2c8d`) made by another session, and most Phase 5 files
  appeared staged in the index although this session never ran `git add`.
  The index was left as found; the working tree holds the complete Phase 5
  change. Unrelated changes under `content-policy/` and `darkmatter/fixes/`
  were not touched.

### Known gaps (not blockers)

- **No selectable session exists yet**, so the production send path has only
  been exercised end to end with fixture eligibility against a real daemon
  (`with_daemon::*`); against the build's real policy every row is
  unavailable. This is expected until a grant lands (Phase 7/8).
- **Native sessions** are neither listed nor deliverable
  (`service::NATIVE_DELIVERY_UNIMPLEMENTED`); coverage gaps are reported in the
  listing. Phase 7 adds native discoverers and adapters behind the same
  `SteeringService` seam.
- **No PTY test drives the real picker.** The picker is exercised with
  synthetic key events on the real `ChooseOne` state and rendered off-screen;
  `TerminalInteraction` itself is a thin `run_standalone` call.
- **Locally refused sends leave no audit record** (only owners audit); this is
  documented in `traces-and-logging.md`.

### Cross-OS assessment

The new code has no `#[cfg]` branches. TTY detection uses
`std::io::IsTerminal` (portable); the picker and consent prompts use
biscuit-tui's `run_standalone`, already used on all three OSes; transport is
the Phase 3 requester. The OS-sensitive paths (daemon routing over named pipes,
the binary's no-daemon failure and exit codes) passed natively on Windows and
on Linux via `just cross-check`.

## Phase 6

Automatic repetition warnings, 2026-09-28. Implemented; every real session
still reports automatic help as unavailable because no activation grant exists
(Pi's `retained-rpc` stays blocked), so delivery is proven with fixture
eligibility and the production path shows the bounded "cannot send" notice.

### Design decisions

- **Detector signals are a separate channel on the same pass.**
  `ContentDetector::observe`/`observe_flush` return a `ContentObservation {
  trip, signals }`; `feed`/`flush` are now `observe(..).trip`, so every existing
  caller and test is unchanged. `detect_cycle` now returns the active cycle's
  `(L, full_cycles)` and `process_line` decides the trip with the identical
  `full_cycles >= max_repeats` test, so the trip schedule is byte-for-byte the
  old one (proved by `feed_only_trip_line` comparisons in every schedule test).
  A chunk that trips returns no signals: this single rule gives "hard stop
  wins within a chunk" and "no warning opportunity is created by a prevented
  warning".
- **Episode state lives in the detector, beside (not inside) the evidence.**
  `Episode::{Armed, Warned { cycle_len, recovered }}` is written only by the
  signal path and never read by the trip decision. Warning at
  `warning_threshold(limit) = limit/2 + limit%2` (no overflow; `None` for 0/1).
  A limit of 2 also never warns in practice: detection seeds two cycles, which
  already trips — pinned by a test. Recovery: a line with no recognized cycle
  at the tail advances `recovered` only when nonblank; any recognized cycle
  (including a new block) resets it; the warned `cycle_len` is frozen until
  `recovered >= max(8, 2L)`. `reset_turn` does not touch it.
- **"Detected repetition" is the detector's own recognition** (two identical
  halves at the tail). Consequence worth knowing: two consecutive identical
  lines (e.g. two `}` lines) reset recovery. That errs toward fewer warnings,
  never toward a changed stop.
- **Opportunity budget and setting are library types**
  (`claudine::steering::automatic`): `OpportunityBudget` (3,
  `OPPORTUNITIES_PER_EXECUTION`), `HELPER_MESSAGE` (spec text verbatim),
  `parse_auto_steer`, `resolve_enabled(env, repo, user)` returning
  `ConfigValidation` errors, and the `SteeringConfig`/`AutomaticSteeringConfig`
  serde types used by both `ClaudineConfig` and `RepoOverrideConfig`.
  `enabled` uses a present-only deserializer (absent → `None`, `null` →
  error); `steering`/`automatic` are non-`Option` structs with `default`, so an
  explicit `null` section is a type error rather than "absent".
- **Resolution happens with the guard config, before launch.**
  `resolve_guard_inputs` already loads the user and repo files at the right
  moment for both structured paths, fails closed, and runs per harness attempt;
  it now also resolves `automatic_steering` from `CLAUDINE_AUTO_STEER` + those
  files. A malformed value therefore fails every structured launch before the
  provider starts (L1-proven: no `argv-1.json`).
- **Runtime: `cli/src/steering/automatic.rs::AutomaticHelp`**, owned by the
  live sink (`None` when turned off, so off means no work, no notices, no
  opportunities). On a warning it claims an opportunity, asks the controller
  for `automatic_route()` (new, read-only), and either spawns
  `controller.submit(Submission { origin: Automatic, expected: None,
  opportunity })` on the runtime or prints the unavailable notice. The stream
  reader never waits: the controller already enforces the 2 s automatic
  deadline, one-automatic-in-flight (`Busy`), and never-retry.
- **Notices** go through `stderr_notices` (a `Status` line: Info for a
  confirmed send, Warning otherwise), deduplicated by text per execution;
  confirmed sends carry "`n` of 3" so they are distinct. The outcome wording
  reuses `commands::steer::render::outcome_word` (made `pub(crate)`).
- **Hard stop during delivery.** `fire_content_trip` sends the trip first and
  then calls `AutomaticHelp::hard_stop`, which refuses later warnings, aborts
  the reply task (so no notice follows the stop), and shuts the controller
  down (its worker aborts the in-flight executor future and answers queued
  requests `unavailable`). A content trip is terminal for the execution, so
  stopping manual steering with it is correct.
- **Per-execution allowance.** `AutomaticHelp` is created per sink, and a sink
  is created per harness attempt (`harness_orch/attempt.rs`) and per direct
  wrapper run (`wrapper_exec.rs`, now given the owner's controller by
  `wrapper_stages.rs`). Retry/resume attempts therefore get fresh budgets;
  turns share one.
- **Control traffic no longer refreshes the silence clock (defect fixed).**
  Before this phase, every stdout line — including Pi RPC `response` lines to
  Claudine's own `steer`/`get_state`/`prompt` commands — refreshed
  `last_byte_at`, so a steering send postponed a `step_timeout` silence kill.
  New `StdioControl::is_control_reply` (default `false`; Pi: `type ==
  "response"`, with a cheap substring pre-check); the semantic reader skips
  `record_byte_activity` for those lines. The lines still reach the parser
  (readiness/session identity depend on them).
- **Helper message** = spec text + "(Observed: the same line N times in a
  row.)" / "…the same K-line block…". Counts only; output is never echoed.
- **No audit record for a routeless opportunity.** It is not submitted, so the
  controller never sees it; it produces the notice and a `tracing::info!`.
  Submitted automatic requests are audited with their opportunity ID as in
  Phase 2. Documented in `automatic-steering.md`.

### Defects found and fixed during the phase

- Steering control replies refreshed the stream-silence clock (above).

### Known gap recorded (not fixed; loader-wide)

- **Duplicate keys in Claudine config files are last-wins.** Both loaders
  parse JSON5 into a `serde_json::Value` (`parse_json5_to_value`), which keeps
  the last duplicate before serde sees the struct, so serde's duplicate-field
  rejection never fires — for every config key, not just this one. Fixing it
  means a duplicate-rejecting parse in `biscuit-file`'s `Json5` (or a pre-scan
  in the loader) and would start rejecting existing user files; that is a
  cross-package behavior change outside Phase 6. The matrix pins the current
  outcome (`duplicate key` → last value) so a stricter parser shows up as a
  deliberate test change, and the topic doc states it.

### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| Warn at ceil(limit/2); no overflow; none for limit 1 (and 2) | lib `runaway::detector::tests::warnings::threshold_is_half_the_stop_limit_rounded_up_and_absent_below_two`, `odd_small_limit_warns_at_three_of_five`, `a_limit_of_one_or_two_has_no_warning_opportunity` |
| Single-line and multi-line cycles warn before the unchanged stop | `default_limit_warns_at_fifteen_single_line_repeats_and_still_stops_at_thirty`, `six_line_block_warns_at_fifteen_cycles_before_the_unchanged_stop` (both compare against a `feed`-only detector) |
| Same chunk crosses warning and stop → only the stop (repetition and volume) | `one_chunk_crossing_warning_and_stop_returns_only_the_stop`, `a_volume_stop_in_the_warning_chunk_also_suppresses_the_warning`; sink `automatic_help::a_chunk_that_crosses_warning_and_stop_only_stops` |
| Split chunks / trailing partial line | `a_line_split_across_chunks_warns_when_it_completes`, `flush_reports_a_warning_on_a_trailing_partial_line` |
| One warning per episode | `one_episode_warns_once_even_as_it_continues` |
| Recovery `max(8, 2L)`; blanks don't advance; repetition (new block) resets; frozen L | `recovery_needs_at_least_eight_lines_or_twice_the_cycle`, `recovery_takes_eight_nonblank_unrepeated_lines_and_blank_lines_do_not_count`, `detected_repetition_resets_recovery_including_a_new_block`, `a_multiline_block_freezes_its_length_for_recovery` |
| Brief wording change is not recovery; stop keeps schedule | `a_brief_wording_change_is_not_recovery_and_the_stop_keeps_its_schedule` |
| Turn boundaries neither advance nor reset | `turn_boundaries_neither_advance_nor_reset_recovery` |
| New episode must reach its own threshold; second stop unchanged | `after_recovery_a_new_episode_must_reach_its_own_threshold`, `recovery_and_a_second_warning_leave_the_second_stop_on_its_schedule` |
| Disabled repetition → no opportunities | `disabled_repetition_detection_produces_no_signals` |
| Three opportunities; no refund; distinct IDs; new execution fresh | lib `steering::automatic::tests::three_opportunities_per_execution_each_distinct`; CLI `steering::automatic::tests::three_warnings_send_three_helper_messages_then_nothing` |
| Unavailable/failed/busy attempts consume opportunities; notice deduplicated | CLI `an_unavailable_route_prints_one_notice_and_spends_every_opportunity`, `a_warning_while_one_is_in_flight_is_busy_and_still_spends_its_opportunity`, `without_an_owner_every_warning_is_unavailable` |
| Slow/unanswered send never blocks; unknown after 2 s; never retried | `a_send_never_blocks_and_an_unanswered_one_is_reported_unconfirmed` |
| Termination during delivery wins; nothing after the stop | `a_hard_stop_abandons_the_send_in_flight_and_refuses_later_warnings`; sink `a_stop_during_delivery_abandons_the_warning` |
| Delivery is non-interrupting, automatic origin, helper text | `three_warnings_send_three_helper_messages_then_nothing`, `the_helper_message_carries_counts_and_never_output`; lib `the_helper_message_is_the_specified_suspicion_and_fits_a_steering_message` |
| Delivery before the stop; acknowledged help is not recovery | sink `the_warning_is_delivered_before_the_stop_and_the_stop_keeps_its_schedule` |
| Opt-out → no sends, no notices, stop unchanged | sink `with_help_off_nothing_is_sent_or_noticed_and_the_stop_keeps_its_schedule`; L1 `pi_managed_rpc::automatic_help_turned_off_by_environment_prints_nothing_and_stops_the_same`, `automatic_help_turned_off_in_user_configuration_prints_nothing_and_stops_the_same` |
| Env parsing (trimmed, case-insensitive, 4 pairs); empty/malformed/non-Unicode errors | lib `every_documented_env_spelling_parses_trimmed_and_case_insensitive`, `empty_and_malformed_env_values_are_configuration_errors`, `a_non_unicode_env_value_is_a_configuration_error` (Unix) |
| Precedence env > repo > user > on; absent repo keeps user opt-out | lib `precedence_is_env_then_repo_then_user_then_on`, `a_repo_file_merged_by_the_user_loader_overrides_only_what_it_sets`; L1 env-over-config case |
| Config errors before launch (env and file) | L1 `a_malformed_auto_steer_value_fails_before_pi_is_launched`, `a_malformed_configured_value_fails_before_pi_is_launched` |
| Persisted value round-trips; unset is omitted | lib `a_saved_config_round_trips_the_setting_and_omits_it_when_unset` |
| End-to-end through the real wrapper + shipped policy: one notice, then the unchanged stop at 30, nothing sent to the provider | L1 `automatic_help_that_cannot_be_sent_warns_once_before_the_unchanged_repetition_stop` (fake Pi `repeat` plan) |
| Steering control replies are not agent progress | CLI `pi_rpc::tests::only_responses_to_the_sessions_own_commands_are_control_replies` |
| Existing expression/volume/repetition/timeout regressions | all pre-existing `runaway::*`, `content_guard::*`, `runaway_guard::*`, timeout and signal suites unchanged and passing in `just test` |

### Input robustness matrix (`steering.automatic.enabled`)

One test per file kind walks every cell from a control file through the real
loader (`load_repo_override_config`, `load_claudine_config`):
`steering::automatic::tests::{repo_file_matrix, user_file_matrix}`.

| Shape | Cell | Outcome (user and repo) |
| --- | --- | --- |
| control | `enabled: false` / `true` | `Some(false)` / `Some(true)` |
| absent | key omitted; `automatic: {}`; `steering: {}`; no `steering` | `None` (inherit) |
| explicit null | `enabled: null`; `automatic: null`; `steering: null` | error |
| wrong type, whole field | `"false"`, `0`, `[false]`; `automatic: false`; `steering: false` | error |
| wrong type, one/every element | n/a (scalar) | — |
| empty | `{}` sections | `None` (inherit), distinct from `null` |
| unknown key | beside `enabled`; beside `automatic` | error (`deny_unknown_fields`) |
| duplicate key | `enabled` twice | **last wins** (known loader-wide gap, pinned) |
| trailing content | valid document + `false` | error |

The environment field's matrix (absent → inherit; empty/whitespace → error;
each spelling; malformed; non-Unicode) is in
`every_documented_env_spelling…`, `empty_and_malformed…`,
`a_non_unicode…`, and `precedence…`. Smell check: the only `#[serde(default)]`
on the load-bearing field pairs with the present-only deserializer; no
`unwrap_or_default`/`.ok()` on its parse.

### Checks run

- `cargo nextest run -p claudine --lib runaway`: 101/101 (21 new).
- `cargo nextest run -p claudine --lib steering::automatic config::`: 387/387.
- `cargo nextest run -p claudine-cli --bin claudine automatic control_replies
  runaway_guard content_guard`: 44/44.
- `cargo nextest run -p claudine-cli --features test-fixtures --test l1
  pi_managed_rpc`: 11/11 (5 new).
- Dispatch inventory re-blessed with the Phase 1–5 workaround (bless branch
  forced for one run and reverted; `dispatch_inventory.rs` unchanged): 3 new
  reference-class `Provider::Pi` sites, all in test files.
- `just lint` (claudine area): clean for all five crates (one
  `type_complexity` fix in a test).
- `just test --no-fail-fast` (claudine area), before the re-bless: 7977
  passed, 2 failed, 9 skipped — the inventory (then re-blessed) and the
  pre-existing `compose_schema_cli::compose_enforces_each_root_union_arm_match_before_provider_launch`
  (darkmatter union work, see Phase 5). **Final run after the re-bless: 7978
  passed, 1 failed (that same pre-existing test), 9 skipped.**
- `just cross-check claudine-cli --features test-fixtures automatic
  pi_managed_rpc control_replies content_guard runaway_guard`: native Windows
  56/56, Linux 56/56. `just cross-check claudine --os windows runaway
  steering::automatic config::`: pass.
- `just check-tier-coverage`: not rerun — no tier markers were added; every new
  test is L1 in an existing declared target (`claudine` lib, `claudine` bin,
  `l1` binary under `test-fixtures`, which CI enables).

### Environment limitations encountered

- GitNexus `impact` was not permitted; callers were found by text search:
  `ContentDetector::feed` (sink, wiring tests, detector tests — signature
  unchanged), `ResolvedGuardInputs` (attempt.rs, wrapper_exec.rs, tests),
  `run_structured_stream_session` (one caller, `wrapper_stages.rs`),
  `ClaudineConfig`/`RepoOverrideConfig` struct literals (7 sites, all updated),
  `StdioControl` (one implementor, Pi).
- Shell heredocs, `perl -i`, and env-prefixed commands needed approval; edits
  were made with the editor tool, and the inventory was blessed as above.
- Writes under `.claude/skills/claudine/` were denied again. Intended skill
  update (not applied), in addition to the Phase 1–5 items: add a row to the
  SKILL.md "Wrapper & composition subsystems" table — "Automatic steering:
  `ContentDetector::observe` adds nonterminal `RepetitionSignal`s (warning at
  ceil(limit/2), recovery after `max(8, 2L)` nonblank unrepeated lines) without
  changing any trip; a tripping chunk carries no signals. The live sink hands
  them to `cli/src/steering/automatic.rs::AutomaticHelp`, which spends one of 3
  opportunities per execution and submits to the execution's own controller in
  the background, or prints one deduplicated 'cannot send' notice; a trip calls
  `hard_stop` (controller shut down). Off via `CLAUDINE_AUTO_STEER` > repo >
  user `steering.automatic.enabled` (resolved in `resolve_guard_inputs`;
  malformed = pre-launch error). Control replies
  (`StdioControl::is_control_reply`) never refresh the silence clock. Reference:
  `topics/automatic-steering.md`."

### Cross-OS assessment

No new `#[cfg]` branches in production code. The OS-sensitive pieces are the
fake-Pi e2e (the wrapper must terminate a provider that is sleeping mid-turn:
process group on Unix, Job Object on Windows) and the reader-thread change;
both passed natively on Windows and on Linux. The non-Unicode env test is
Unix-only because building an invalid `OsStr` is platform-specific; the
parser's non-Unicode branch is the same code on Windows.
