---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/messenger/features/2026-09-17-research-metadata-pipeline/spec.md
plan: messenger/features/2026-09-17-research-metadata-pipeline/plan.md
implemented_by: claude/opus
started_phase: 1
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
source_files_during_phase_3:
    - messenger/lib/Cargo.toml
    - messenger/justfile
    - Cargo.lock
    - messenger/lib/src/lib.rs
    - messenger/lib/src/research/mod.rs
    - messenger/lib/src/research/assess.rs
    - messenger/lib/src/research/canonical.rs
    - messenger/lib/src/research/diagnostics.rs
    - messenger/lib/src/research/error.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/src/research/paths.rs
    - messenger/lib/src/research/model/mod.rs
    - messenger/lib/src/research/model/common.rs
    - messenger/lib/src/research/model/document.rs
    - messenger/lib/src/research/model/roster.rs
    - messenger/lib/src/research/model/overrides.rs
    - messenger/lib/src/research/model/mappings.rs
    - messenger/lib/src/research/validate/mod.rs
    - messenger/lib/src/research/validate/identity.rs
    - messenger/lib/src/research/validate/constraints.rs
    - messenger/lib/src/research/validate/bindings.rs
    - messenger/lib/src/research/validate/interaction.rs
    - messenger/lib/src/research/validate/errors.rs
    - messenger/lib/src/research/validate/coverage.rs
    - messenger/lib/src/research/validate/replay.rs
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_validation.rs
    - messenger/lib/tests/fixtures/research/contract/overrides-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/pilot-discord.md
    - messenger/lib/tests/fixtures/research/contract/pilot-signal.md
    - messenger/lib/tests/fixtures/research/contract/pilot-telegram.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-coverage--surface-uncovered.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-enforceable--unit-unspecified-executable.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-evidence--secondary-only-known.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-gap--gap-placeholder.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-interactivity--callback-only-claims-text.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--adapter-platform-mismatch.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--proposed-as-implemented.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--unassessed-without-reason.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-origin-phase--sdk-phase-certainty-mismatch.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--expired-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--stale-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-ref--error-dangling-constraint.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--missing-roster-interface.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--research-only-with-adapter.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-adapter-mapped-twice.yaml
docs_updated_during_phase_3:
    - messenger/docs/research/platforms/_rules.md
    - messenger/lib/tests/fixtures/research/README.md
    - messenger/lib/README.md
    - docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/os/macos.md
source_files_during_phase_4:
    - .gitignore
    - Cargo.lock
    - messenger/cli/Cargo.toml
    - messenger/cli/src/lib.rs
    - messenger/cli/src/main.rs
    - messenger/cli/src/research.rs
    - messenger/cli/tests/research_cli.rs
    - messenger/lib/src/research/mod.rs
    - messenger/lib/src/research/assess.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/src/research/paths.rs
    - messenger/lib/src/research/model/common.rs
    - messenger/lib/src/research/validate/mod.rs
    - messenger/lib/src/research/validate/coverage.rs
    - messenger/lib/src/research/delta.rs
    - messenger/lib/src/research/generate.rs
    - messenger/lib/src/research/project.rs
    - messenger/lib/src/research/report.rs
    - messenger/lib/src/research/publish/mod.rs
    - messenger/lib/src/research/publish/fsutil.rs
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_lifecycle.rs
    - messenger/lib/tests/research_publication.rs
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/discord.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/slack.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/telegram.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/whatsapp.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/signal.md
docs_updated_during_phase_4:
    - messenger/lib/README.md
    - messenger/cli/README.md
    - messenger/lib/tests/fixtures/research/README.md
    - docs/dependencies.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/os/macos.md
source_files_during_phase_5:
    - Cargo.lock
    - claudine/cli/Cargo.toml
    - claudine/cli/src/main.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/budget/mod.rs
    - claudine/cli/src/budget/error.rs
    - claudine/cli/src/budget/model.rs
    - claudine/cli/src/budget/run.rs
    - claudine/cli/src/budget/store.rs
    - claudine/cli/src/budget/tests.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/budget.rs
    - claudine/cli/src/commands/help.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/tests/sequence_budget.rs
    - claudine/cli/tests/snapshots/wrap_basics__help_lists_wrapper_subcommands.snap
    - claudine/cli/tests/error_guards/transport-allow.toml
docs_updated_during_phase_5:
    - claudine/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/dependencies.md
    - docs/dependencies.md
docs_created_during_phase_5:
    - claudine/docs/cli/budget.md
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/os/macos.md
source_files_during_phase_6:
    - messenger/lib/src/research/mod.rs
    - messenger/lib/src/research/diagnostics.rs
    - messenger/lib/src/research/generate.rs
    - messenger/lib/src/research/model/common.rs
    - messenger/lib/src/research/publish/mod.rs
    - messenger/lib/src/research/publish/fsutil.rs
    - messenger/lib/src/research/refresh/mod.rs
    - messenger/lib/src/research/refresh/approval.rs
    - messenger/lib/src/research/refresh/check.rs
    - messenger/lib/src/research/refresh/cleanup.rs
    - messenger/lib/src/research/refresh/config.rs
    - messenger/lib/src/research/refresh/prepare.rs
    - messenger/lib/src/research/refresh/promote.rs
    - messenger/lib/src/research/refresh/records.rs
    - messenger/lib/src/research/refresh/review.rs
    - messenger/lib/src/research/refresh/select.rs
    - messenger/lib/src/research/refresh/state.rs
    - messenger/lib/tests/research_refresh.rs
    - messenger/cli/src/lib.rs
    - messenger/cli/src/main.rs
    - messenger/cli/src/research.rs
    - messenger/cli/src/research_lifecycle.rs
    - messenger/cli/tests/research_lifecycle_cli.rs
docs_updated_during_phase_6:
    - messenger/lib/README.md
    - messenger/cli/README.md
    - messenger/docs/research/platforms/_fleet.md
    - messenger/docs/research/platforms/_rules.md
    - messenger/features/2026-09-17-research-metadata-pipeline/fixture-matrix.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
source_files_during_phase_7: []
docs_updated_during_phase_7:
    - messenger/features/2026-09-17-research-metadata-pipeline/plan.md
    - messenger/features/2026-09-17-research-metadata-pipeline/implementation-log.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/messenger/research-contract.md
source_files_during_phase_8:
    - messenger/justfile
    - messenger/lib/src/research/refresh/state.rs
    - messenger/lib/src/research/refresh/prepare.rs
    - messenger/lib/src/research/refresh/select.rs
    - messenger/lib/tests/research_refresh.rs
docs_updated_during_phase_8:
    - messenger/README.md
    - messenger/cli/README.md
    - messenger/docs/user-guide.md
    - messenger/features/2026-09-17-research-metadata-pipeline/plan.md
    - messenger/features/2026-09-17-research-metadata-pipeline/implementation-log.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spec.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/os/macos.md
packages:
    - messenger
    - messenger-cli
    - claudine-cli
source_code:
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
    - messenger/justfile
    - Cargo.lock
    - messenger/lib/src/lib.rs
    - messenger/lib/src/research/mod.rs
    - messenger/lib/src/research/assess.rs
    - messenger/lib/src/research/canonical.rs
    - messenger/lib/src/research/diagnostics.rs
    - messenger/lib/src/research/error.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/src/research/paths.rs
    - messenger/lib/src/research/model/mod.rs
    - messenger/lib/src/research/model/common.rs
    - messenger/lib/src/research/model/document.rs
    - messenger/lib/src/research/model/roster.rs
    - messenger/lib/src/research/model/overrides.rs
    - messenger/lib/src/research/model/mappings.rs
    - messenger/lib/src/research/validate/mod.rs
    - messenger/lib/src/research/validate/identity.rs
    - messenger/lib/src/research/validate/constraints.rs
    - messenger/lib/src/research/validate/bindings.rs
    - messenger/lib/src/research/validate/interaction.rs
    - messenger/lib/src/research/validate/errors.rs
    - messenger/lib/src/research/validate/coverage.rs
    - messenger/lib/src/research/validate/replay.rs
    - messenger/lib/tests/research_validation.rs
    - messenger/lib/tests/fixtures/research/contract/overrides-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/pilot-discord.md
    - messenger/lib/tests/fixtures/research/contract/pilot-signal.md
    - messenger/lib/tests/fixtures/research/contract/pilot-telegram.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-coverage--surface-uncovered.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-enforceable--unit-unspecified-executable.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-evidence--secondary-only-known.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-gap--gap-placeholder.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-interactivity--callback-only-claims-text.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--adapter-platform-mismatch.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--proposed-as-implemented.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--unassessed-without-reason.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-origin-phase--sdk-phase-certainty-mismatch.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--expired-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--stale-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-ref--error-dangling-constraint.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--missing-roster-interface.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--research-only-with-adapter.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-adapter-mapped-twice.yaml
    - .gitignore
    - messenger/cli/Cargo.toml
    - messenger/cli/src/lib.rs
    - messenger/cli/src/main.rs
    - messenger/cli/src/research.rs
    - messenger/cli/tests/research_cli.rs
    - messenger/lib/src/research/delta.rs
    - messenger/lib/src/research/generate.rs
    - messenger/lib/src/research/project.rs
    - messenger/lib/src/research/report.rs
    - messenger/lib/src/research/publish/mod.rs
    - messenger/lib/src/research/publish/fsutil.rs
    - messenger/lib/tests/research_lifecycle.rs
    - messenger/lib/tests/research_publication.rs
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/discord.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/slack.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/telegram.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/whatsapp.md
    - messenger/lib/tests/fixtures/research/lifecycle/fleet/signal.md
    - claudine/cli/Cargo.toml
    - claudine/cli/src/main.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/budget/mod.rs
    - claudine/cli/src/budget/error.rs
    - claudine/cli/src/budget/model.rs
    - claudine/cli/src/budget/run.rs
    - claudine/cli/src/budget/store.rs
    - claudine/cli/src/budget/tests.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/budget.rs
    - claudine/cli/src/commands/help.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/tests/sequence_budget.rs
    - claudine/cli/tests/snapshots/wrap_basics__help_lists_wrapper_subcommands.snap
    - claudine/cli/tests/error_guards/transport-allow.toml
    - messenger/lib/src/research/refresh/mod.rs
    - messenger/lib/src/research/refresh/approval.rs
    - messenger/lib/src/research/refresh/check.rs
    - messenger/lib/src/research/refresh/cleanup.rs
    - messenger/lib/src/research/refresh/config.rs
    - messenger/lib/src/research/refresh/prepare.rs
    - messenger/lib/src/research/refresh/promote.rs
    - messenger/lib/src/research/refresh/records.rs
    - messenger/lib/src/research/refresh/review.rs
    - messenger/lib/src/research/refresh/select.rs
    - messenger/lib/src/research/refresh/state.rs
    - messenger/lib/tests/research_refresh.rs
    - messenger/cli/src/research_lifecycle.rs
    - messenger/cli/tests/research_lifecycle_cli.rs
documentation:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/findings.md
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
    - messenger/docs/research/platforms/discord.md
    - messenger/docs/research/platforms/slack.md
    - messenger/docs/research/platforms/telegram.md
    - messenger/docs/research/platforms/whatsapp.md
    - messenger/docs/research/platforms/signal.md
    - docs/dependencies.md
    - messenger/docs/research/platforms/_fleet.md
    - messenger/docs/research/platforms/_rules.md
    - messenger/lib/tests/fixtures/research/README.md
    - messenger/lib/README.md
    - messenger/cli/README.md
    - claudine/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/dependencies.md
    - claudine/docs/cli/budget.md
    - messenger/features/2026-09-17-research-metadata-pipeline/plan.md
    - messenger/features/2026-09-17-research-metadata-pipeline/implementation-log.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spec.md
    - messenger/README.md
    - messenger/docs/user-guide.md
completed_phase: 8
implemented: false
implementation_1: "2026-09-18T01:45:22-07:00"
implementation_2: "2026-09-18T09:01:12-07:00"
---

# Implementation Log for 2026-09-17-research-metadata-pipeline (8 phases)

## Phase 1

- Phase 1 is discovery and architecture: its outputs are spike evidence and
  design records under `spikes/` and `architecture.md`, not product behavior.
- Wave 1 tasks run as four parallel agents with disjoint output directories:
  `spikes/surface-inventory/`, `spikes/schema-pilots/`, `spikes/publication/`,
  and `spikes/orchestration/` (extended).
- Drafted `fixture-matrix.md` while Wave 1 ran: all 37 acceptance criteria map
  to named fixtures, a proof boundary, and a landing phase; every row is L1.
- Finding: `just test`/`just lint` build the `messenger` library with only
  `desktop`, so research code needs explicit `--features research` and
  `--all-features` checks (recorded in the matrix; no new CI cell — CI already
  runs `all-features`).
- Finding: `cargo tree -p messenger --all-features` already reaches
  `biscuit-file` and YAML crates through `sniff` → `schematic-*`; the
  "ordinary send build has no maintenance deps" check must therefore use
  no-default, default, and `desktop` feature sets (all clean on 2026-09-17).
- Wave 1 Surface Inventory complete → `spikes/surface-inventory/inventory.md`
  (revision `d57faf7e8`). Key outcomes: adapter IDs reuse the persisted
  `ProviderKind::as_str()` strings; interface IDs are separate snake_case
  names; `Summarized` bodies, location-with-text, and plain text behave very
  differently per adapter; `supports_location` is `true` for all seven even
  where only a text fallback exists; error detail (codes, subcodes, field
  errors, correlation IDs, success warnings) is lost in most adapters. Three
  suspected runtime defects (Slack webhook response parsing, Signal group
  method name, Telegram 429 `retry_after` loss) are recorded as research
  priorities, not fixed here — this feature must not change delivery behavior.
- Wave 1 Publication Spike complete → `spikes/publication/findings.md` and a
  standalone prototype crate (`spikes/publication/prototype/`, own empty
  `[workspace]`, not a monorepo member). Chosen: committed manifest
  `messenger/docs/research/publication.json` as the single selection point,
  local journal + staged/backup copies under gitignored
  `messenger/.research-state/publication/`, and a hash-guarded, idempotent
  roll-back/roll-forward recovery. All 17 prototype tests pass on macOS
  (re-run by the orchestrator: 2 + 9 + 6 tests OK), build-linux, and
  build-win-native. **Gap:** WSL2 not run (SSH to build-win reset twice; W:
  nearly full). The subagent could not write its report file; the orchestrator
  saved its content verbatim.
- New measured Windows fact added to `.claude/skills/os/windows.md`:
  `std::fs::rename` succeeds over a std-opened reader while
  `tempfile::persist` fails with error 5.
- Wave 1 Budget Spike complete → new section in
  `spikes/orchestration/findings.md` plus `probe_budget.py`,
  `probe_tree_windows.py`, `probe-results.json`. Built committed HEAD
  `d57faf7e8` from a `git archive` snapshot because the working tree's
  Claudine crates currently fail to compile due to another effort's
  uncommitted edits (not touched). Key outcomes: worker cwd is the repo root
  (prepared scratch cwd needed for discovery); inline-compose exposes prior
  prose by design (never use it for discovery); Unix timeout kills only the
  child's process group (a `setsid` grandchild survives; wrapper death
  orphans the worker); Windows Job Object kills the tree; lifecycle retries
  launch inside one Claudine command, so admission must hook inside
  Claudine's launch path (`execute_attempt_phase` before
  `execute_harness_attempt`); Claudine persists no run state today.
  GitNexus impact was unreliable (index mid-update by another session) —
  Phase 5 must re-run impact on a fresh index. **Gap:** WSL2 unreachable;
  native Windows evidence is OS-level (Job Object) only.
- Wave 1 Schema Pilots complete → `spikes/schema-pilots/` (pilot schema,
  four pilot documents, mappings pilot, negative corpus, probes). The subagent
  could not write `findings.md`; the orchestrator saved it and added a note
  that production IDs/fingerprints follow `architecture.md`. Independently
  re-verified with the installed `md`: the pilots exit 0, all 13
  `schema-rejects/` samples exit 1, and all 9 `schema-accepts-rust-must-reject/`
  samples exit 0 (as designed). Thirteen SimplifiedSchema limitations were
  recorded (L1–L13); the most consequential are root closure only through a
  pattern key (L1), type leakage requiring a separate `_types.yaml` (L2), no
  arrays of unions (L3–L5), and multi-line types failing to load from
  standalone files (L6, likely a Darkmatter bug).
- Wave 2 complete: `architecture.md` (IDs, module/feature graph, schema/type
  boundaries, artifacts, local state, snapshot protocol, fingerprints,
  CLI/recipes, budget boundary, cleanup rules, resolution of every spec
  planning item) and `fixture-matrix.md`. Confirmed that Darkmatter already
  exposes `DarkmatterSchemas::validate` as a library API, and that its
  `effects-instrumentation` feature can prove loader passivity.
- Decision recorded (not escalated): downtime after a crash is not charged.
  The run enters `interrupted`, an operator-resumption state, which the spec's
  active-clock ruling already excludes; time through the last heartbeat plus
  one interval stays charged, so no consumed allowance is refunded.

### Verification

- Phase 1 changes no product behavior: no workspace crate, public API,
  provider, or `CapabilitySet` changed. The requirement-to-test mapping for
  the whole feature is `fixture-matrix.md`; Phase 1's own claims are backed by
  spike evidence rather than workspace tests:
  - snapshot protocol: 17 prototype tests (interruption at every injection
    point, abort-based crashes, Windows open-handle cases), passing on
    macOS (re-run by the orchestrator), build-linux, and build-win-native;
  - schema expressiveness: `md schema validate` over pilots and negative
    samples (re-run by the orchestrator);
  - budget semantics: `probe_budget.py` on macOS and build-linux,
    `probe_tree_windows.py` on build-win-native (`probe-results.json`).
- Gates: `just test` in `messenger/` — 532 run, 532 passed, 2 skipped
  (pre-existing skips); `just lint` in `messenger/` — clean.
- Skipped / gaps: WSL2 evidence for both spikes (host unreachable); the
  Claudine binary was not run on native Windows; `just cross-check` was not
  used because no workspace package changed.
- Plan checkpoint item "obtain maintainer review" is left unchecked: it needs
  a human. `human_review: true` is set on the spec with two items (extra
  lifecycle subcommands; artifact-layout additions).
- Pre-existing: the working tree's Claudine crates did not compile during
  this phase because of another effort's uncommitted edits (not touched).

## Phase 2

- Proceeded under the architecture record's recommended option A for both
  pending human-review items (five explicit lifecycle subcommands; the layout
  additions). Phase 2 depends only on the layout item: the schema split into
  `_schema.yaml` + `_types.yaml` is unavoidable (pilot L2). No CLI surface was
  implemented, and `_fleet.md` names only the spec-required
  `messenger research validate`, so the subcommand decision stays open.
- Wave 3 **Roster Contract** → `messenger/docs/platforms.yaml` +
  `messenger/docs/platforms.schema.yaml`. Five active platforms, seven sending
  interfaces (adapter IDs = `ProviderKind::as_str()`), six research-only
  companions (`discord_gateway`, `discord_interactions`, `slack_events_api`,
  `slack_socket_mode`, `slack_interactivity`, `whatsapp_webhooks` — Telegram
  and signal-cli receive on their own interface, so they need none), and
  explicit exclusions for email, desktop, APNs, FCM. The curated-source cap is
  a schema ceiling (`max(10)` on the array, `curated_source_cap ≤ 10`); Rust
  enforces a lower configured cap (SR-CURATED). Seed curated lists come from
  URLs already cited in the prose research and the spec's spot checks, all
  marked `review: proposed` because every curated-list change needs human
  approval.
- Finding: the roster cannot be both an `md`-validated file and a raw
  Claudine sequence source. `md schema validate` only sees data inside
  `---` fences (pilot L10), but `biscuit_file::Yaml` (Claudine's sequence
  loader) rejects a fenced file as a two-document stream. Kept the fences;
  the fleet reaches roster data through Messenger's prepared per-run inputs
  (architecture "Local state area"), not by pointing `claudine sequence` at
  the roster. Recorded for Phase 6.
- Wave 3 **Metadata Schema** → `docs/research/platforms/_schema.yaml` +
  `_types.yaml` (schema version 1), `_overrides.schema.yaml`,
  `docs/research/implementation/_schema.yaml`, and `_rules.md`. One types
  file serves all four schemas (cross-directory `Name@../platforms/_types.yaml`
  imports were probed and work). Vocabulary beyond the pilot: dedicated
  `attachment_bindings`, `addressing`, `receipts`, `attribution_bindings`,
  `location_bindings`, `author_geolocation`, `expression_bindings`,
  `delivery_controls`, `eligibility`, `rate_limits`, `interaction_fixtures`;
  a per-interface `coverage` matrix with all 16 categories as required keys
  (each cell `researched` / `not_applicable` / `gap`), so an omitted category
  fails the schema and every category can express an investigated gap;
  `html_subset`; text-binding `packaging`/`visibility`; image
  `multiple_images`/`ordering`/`atomic`/`receipts`; question `native`,
  `cancellation`, `responses`; `link_button`, `free_text_interpretation`;
  form `external_form`/`sequential_application` containers; a shared
  `Lifecycle` with units in field names; operations constrained to
  snake_case; xxh64 fingerprints (`xxh64:` + 16 hex) instead of the pilot's
  SHA-256. The pilot's generic `capabilities` record is dropped.
- `_rules.md` lists 30 Rust-owned rules (the pilot's SR-* list plus
  SR-CURATED, SR-ATTRIBUTION, SR-LOCATION, SR-EXPRESSION, SR-SANITIZED,
  SR-OVERRIDE) with their planned `messenger::research` owner modules.
- Wave 4 fixtures → `messenger/lib/tests/fixtures/research/`:
  `contract/` (45: the four pilots ported to v1, the shipped roster copy,
  and one fixture per representation), `interaction/` (13), `diagnostics/`
  (6), `negative/schema/` (28, each with an `# expect-problem:` pointer), and
  `negative/semantic/` (79, schema-valid by design, stem = rule code). All
  171 checked with the installed `md`: every positive and semantic negative
  is valid; every schema negative fails at its declared pointer. Payloads use
  placeholder IDs (`USER_ID`, `MESSAGE_ID`); hosts are `*.example.com`.
- Pilots ported to v1 as `contract/pilot-{discord,telegram,slack,signal}.md`:
  interface IDs renamed to the production snake_case form, operations
  snake_cased (`sendMessage` → `send_message`, `chat.postMessage` →
  `chat_post_message`), new required fields filled conservatively
  (`unknown` or a `gap.<platform>.pilot_scope` gap), and the three generic
  capabilities converted to attribution, author-geolocation, and attachment
  records. All four pass the v1 schema.
- The fixtures were generated once by a throwaway script (not committed) and
  are now ordinary hand-maintained files. `contract/roster.yaml` was dropped:
  a copy of the shipped roster would drift, and the corpus test validates the
  shipped roster directly.
- Wave 3 **Fleet Prompt** → `docs/research/platforms/_fleet.md`: role limits,
  access policy (preauthorized versus approval-required), evidence rules,
  contract guidance, 15 category research questions, the spec's
  per-interface required investigations, and the three passes with their
  inputs and outputs (discovery report with `suggested_sources`; candidate
  plus source-check record; source-list proposal that never edits the
  roster). The five platform documents' duplicated inline prompts were
  replaced with a short delegating prompt: a single-document refresh is
  Pass 2 only, runs against a candidate copy, and never runs discovery
  through inline-compose. The prompts use repository-root paths, because a
  candidate copy lives elsewhere and relative `./_fleet.md` would break. The
  documents otherwise stay untouched legacy prose (no `$schema` until the
  Phase 7 migration).
- Not done in Phase 2, deliberately: `claudine sequence` frontmatter in
  `_fleet.md`. Running a sequence launches real agents (forbidden here), and
  its inputs depend on the Phase 5 budget ledger and the Phase 6 prepared-run
  layout.
- Passive corpus test → `messenger/lib/tests/research_corpus.rs` (14 tests)
  with `darkmatter` (`effects-instrumentation`) as a **dev-dependency only**,
  following `claudine-cli`'s precedent. It validates the shipped roster and
  every fixture through `DarkmatterSchemas::validate` (the `md schema
  validate` code path), checks the roster against `ProviderKind::as_str()`,
  checks that legacy documents delegate to the fleet, checks that every
  semantic rule code appears in `_rules.md`, proves passivity (effect-engine
  and network counters unchanged, validated bytes unchanged, including a
  `prompt` carrying `{{ shell(...) }}`), and scans the corpus for
  credentials, webhook URLs with concrete IDs, bot-token and phone shapes,
  control characters, escaped ESC sequences, and payloads over 1 KiB. The
  scanner has its own positive and negative samples so it cannot pass
  vacuously.
- Performance: Darkmatter re-resolves the schema for every document (about
  100 ms per document in a debug build). A shared `DarkmatterSchemas` did
  not help. Tests are split per directory and rule family, and the passivity
  test uses a representative sample, so the slowest test takes about 5 s,
  well inside the default 30 s termination.
- The test reads `CARGO_MANIFEST_DIR` at run time, falling back to `env!`,
  because the WSL2 CI leg runs a nextest archive whose compile-time path
  names the ubuntu-latest builder (os skill, "Red only on wsl2-ubuntu").
- Mutation checks (reverted): an unknown key in `contract/minimal-valid.md`
  failed `contract_fixtures_validate`; eleven Telegram curated sources failed
  both roster tests at the schema (`has more than 10 items`) and in Rust.

### Requirement-to-test mapping (Phase 2)

| Behavior changed | Test / evidence |
|---|---|
| Roster: 5 platforms, 7 adapters = `ProviderKind::as_str()`, companions without adapters, exclusions (criterion 1) | `shipped_roster_validates_against_its_schema`, `shipped_roster_maps_every_chat_adapter_exactly_once`; `negative/semantic/sr-roster--*.yaml` |
| Curated cap 10 and explained contributions (criterion 27) | `negative/schema/roster-cap-11.yaml`, `roster-retained-without-contribution.yaml`, `roster-cap-above-ceiling.yaml`; `contract/roster-cap-10.yaml`; `sr-curated--*` |
| Closed schema: unknown keys/enums, missing arrays/interfaces/knowledge, types, locators, incomplete matrices, version literal (criterion 6) | `schema_negative_fixtures_fail_at_their_declared_problem` (28 fixtures, each asserting its pointer) |
| Representations: field/recommended/aggregate/bytes/count/bridge-conditional bounds, units and stages, states, versions, bindings, images, attribution, location, expression, controls, gaps (criteria 4, 5, 15-19, 25, 28) | `contract_fixtures_validate` over `contract/*` |
| Interactivity (criteria 20-22) | `interaction_fixtures_validate` over `interaction/*` |
| Diagnostics (criteria 12, 13) | `diagnostic_fixtures_validate` over `diagnostics/*` |
| Rust-owned rules stay out of the schema and are documented | `semantic_negative_fixtures_*` (79), `rules_document_lists_every_rule_the_semantic_corpus_names` |
| Sanitized fixtures (criterion 14) | `research_corpus_is_sanitized`, `sanitization_scanner_rejects_known_unsafe_shapes` |
| Passive validation (Phase 2 checkpoint) | `validation_is_passive` |
| v1 freeze gate: four pilots pass and every category can hold an investigated gap | `freeze_gate_fixtures_validate` |
| Fleet delegation replaces duplicated prompts | `shipped_platform_documents_validate_or_delegate_to_the_fleet` |
| Real shipped artifacts through the normal path | `md schema validate` over the roster, all 170 fixtures, and the five documents (installed binary); the corpus test uses the same library entry point |

Phase 2 has no persisted read/write/read round trip: it writes no values
programmatically. The Phase 4 publication tests own round trips.

### Gates (macOS)

- `just test` in `messenger/`: 546 run, 546 passed, 2 skipped (pre-existing
  skips).
- `just lint` in `messenger/`: clean (covers the new test target).
- `cargo nextest run -p messenger --all-features --test research_corpus`:
  14/14.
- `cargo tree -p messenger -e normal` with `--no-default-features`, default
  features, and `--features desktop`: no `darkmatter`, `biscuit-file`, or
  YAML crate (the dev-dependency does not leak).
- `md schema validate` sweep: 0 unexpected results over 170 fixtures.

### Cross-OS evidence (`just cross-check messenger … research_corpus`)

None of the three remote legs produced a test result. All failures are host
availability problems, not test failures:

- **windows** (`build-win-native`): `scp` to `W:` failed, then `git fetch`
  failed with `No space left on device`. The `W:` volume is full (Phase 1
  noted it was nearly full). Not cleaned: freeing space on a shared host
  deletes other people's data and needs a human decision (see the
  `storage-strategy` skill).
- **wsl** (`build-win`): `kex_exchange_identification: Connection reset by
  peer`, the same SSH reset Phase 1 hit.
- **linux** (`build-linux`): queued behind a stale host lock held since
  2026-09-14T18:25Z by `reward-20260914-c3e60d0` (`nightly-reward-spike`,
  branch `feat-nightly-perf`). The script never removes a foreign lock, and
  neither did this phase; the queued waiter was stopped after about 25
  minutes.

OS-risk assessment for what Phase 2 changed: the only code is a test that
reads repository files. It uses a runtime `CARGO_MANIFEST_DIR` (for the WSL
archive leg), `Path::join` with relative segments (portable), and `str::lines`
(which also strips `\r`). `.gitattributes` forces `eol=lf`. Darkmatter's
relative `$schema` and `Name@../platforms/_types.yaml` resolution on native
Windows is the one unproven path; CI's `windows-latest` leg covers it, and
the next phase with host access should run `just cross-check messenger --os
windows research_corpus`.

## Phase 3

- Added the opt-in `research` feature to `messenger` (optional `darkmatter`,
  `biscuit-file` with only `file-reference`, `biscuit-hash`,
  `serde_path_to_error`) and the feature-gated module
  `messenger/lib/src/research/`: `model` (authored DTOs), `load` (passive
  loader), `diagnostics`, `error`, `paths` (`Workspace`, `RepoPath`),
  `canonical` (canonical JSON + xxh64), `validate` (six rule families plus
  `replay`), and `assess` (mappings and fingerprint reuse).
- Local L1 now covers the feature: `local-features = ["desktop", "research"]`
  and the package-area `test`/`sanity`/`lint` recipes build
  `messenger --features desktop,research`. Without this, `just test` and
  `just lint` would never compile the new module. CI is unchanged (it already
  runs `all-features`).
- Loader decisions:
  - One `FileResolutionContext` is captured per `Loader`, anchored at an
    explicit repository root (`FileResolutionContext::new(root)
    .with_repository_root(root)`). No sniff or Git probing, so no process
    spawn. Darkmatter shares the same context.
  - `$schema` must resolve (through `FileReference`) to the shipped schema
    for the file's kind; anything else is `SR-SCHEMA-BINDING` and schema
    validation is skipped (a document bound to another schema proves nothing).
  - Darkmatter validation problems become `SCHEMA` findings (`SR-TOP-LEVEL`
    for an unknown root key). Only a schema-clean file is version-gated
    (`SR-VERSION`: the raw value must be the integer 1) and then
    deserialized from the *authored* frontmatter, so Darkmatter's coercion
    cannot hide a quoted number or a float version (`SR-STRICT-SCALARS`
    comes from serde's `invalid type` errors, with a JSON Pointer from
    `serde_path_to_error`).
  - Host paths never reach findings: every finding carries a `/`-separated
    `RepoPath`; third-party messages are scrubbed of the repository root.
  - Manifests (`publication.json`) are not loaded yet: the manifest format
    is defined by the Phase 4 snapshot writer, which will add its reader.
- Validation scopes (the SR-GAP baseline-mode decision the Phase 2 handoff
  asked for): `Scope::Fragment` for fixtures, pilots, and candidates;
  `Scope::Accepted` for accepted research and baselines, which additionally
  requires every active roster interface, coverage and unknown facts resting
  only on investigated gaps, an image-role matrix for researched images, and
  an author-geolocation answer for researched location. Roster validation in
  `Accepted` scope also requires all five platforms and all seven adapters.
- Ruling: a known bound backed only by `secondary` sources is valid but
  ineligible (`IneligibleReason::SecondaryEvidenceOnly`), not rejected, because
  the spec says such claims "remain visible". Same treatment as
  `sr-enforceable--*`.
- Ruling: `recommended_max` is never executable (`Advisory`): no enforcing
  consumer exists for a recommendation.
- Ruling: the SR-UNIQUE / SR-APPLICABILITY scope key includes unit and
  measurement stage. 2000 Unicode scalars and 4000 UTF-8 bytes on one field
  are two simultaneous bounds, not a duplicate (`contract/units-ambiguous.md`
  relies on this).
- Ruling: `interactivity` coverage for an interface counts records on its
  related companions in either direction (Gateway, Interactions, Events API),
  plus question/form bindings naming it as `companion_interface`.
- Ruling: changes may name gap IDs as well as facts; a `removed` change must
  name an ID that is no longer present (removed IDs are never reused).
- Error replay (`validate::replay`): signatures are explicit conjunctions; a
  numeric or string native code is read at `code_locator`, and for `warning`
  outcomes also from `warnings_locator`. A discriminator may read a JSON
  Pointer beneath a defined envelope pointer (`/` is authored as the root).
  `no_match` and `unknown` both mean "no executable signature matches".
- Interaction replay: only `expect: answer` fixtures on question bindings are
  replayed (confirmation mapping, option IDs, strings). Timeout, cancellation,
  and stale-option outcomes are lifecycle facts that no payload proves.
- Fixture corrections. Holding every semantic fixture to "each finding
  carries the expected rule" exposed Phase 2 defects. The code was treated as
  correct and the fixtures were fixed:
  - `sr-coverage--surface-uncovered.md` gave both interfaces a matrix (so
    nothing was uncovered); the second matrix was removed.
  - `sr-roster--research-only-with-adapter.md` and
    `sr-interactivity--callback-only-claims-text.md` had a duplicated
    coverage matrix; `sr-roster--roster-adapter-mapped-twice.yaml` attributed
    Slack curated sources to `discord_bot_api`;
    `sr-ref--error-dangling-constraint.md` marked errors `researched` with no
    record; `sr-origin-phase--sdk-phase-certainty-mismatch.md` bound an SDK
    error to a service envelope (now an `sdk_error` envelope with an
    `sdk_error_variant` signature).
  - `pilot-discord.md`: dangling `alt_text_binding`, and
    `requires_messenger_update: true` without the required gap (added
    `gap.discord.embed_budget_preflight`). `pilot-signal.md`: operand-less
    bridge-version conditions without a gap (the pilot prose predated
    SR-CONDITION; now `gap: gap.signal.versions`), and a change naming the
    removed `cap.signal.attachments` (now `att.signal.send`).
    `pilot-telegram.md`: a prose selector default (now
    `default_state: absent`) and a gap naming the old `cap.*` ID.
  - Cross-file fixtures now name `c.fx.content.service_max`, a fact in the
    companion `contract/constraints-field.md`. `overrides-valid.yaml` and
    `sr-override--expired-override.yaml` carry the real target and schema
    fingerprints, so each negative fails for exactly one cause.
  - New headers: `# validate-scope: accepted` (missing roster interface, gap
    placeholder) and `# expect-ineligible: <fact> <reason>` (enforceable,
    secondary-only evidence). README updated.
- Performance: every `DarkmatterSchemas::validate` call re-resolves the
  schema's imports. With the typed corpus added, two tests hit nextest's 30 s
  termination in the full `--all-features` run. Two fixes: `Loader` now caches
  the resolved `EffectiveSchema` per contract kind (sound because binding
  pins each kind to one schema file), and the Phase 2 corpus helper caches by
  canonical `$schema` path. Both research suites dropped from about 18 s to
  3 s wall time.
- `_rules.md` updated: two new load-stage codes (`SR-SCHEMA-BINDING`,
  `SR-TOP-LEVEL`), the two scopes, and the rulings above (SR-UNIQUE key,
  SR-EVIDENCE, SR-ENFORCEABLE advisory, SR-COVERAGE counting, SR-GAP scope,
  SR-CHANGE gaps and removals, SR-ENVELOPE root pointer, and SR-MAPPING
  outcome versus finding).

### Requirement-to-test mapping (Phase 3)

| Behavior | Test(s) |
|---|---|
| Typed DTOs mirror schema v1 exactly (no lost, renamed, or extra field) | `research_validation::{contract,interaction,diagnostic}_documents_round_trip_through_the_schema` (load → serialize → Darkmatter schema → deserialize, twice for determinism) |
| Roster, overrides, mappings typed load | `typed::shipped_roster_is_a_complete_accepted_roster`, `typed::positive_roster_overrides_and_mappings_fixtures_are_clean` |
| Adapter IDs = `ProviderKind::as_str()` | `typed::adapter_ids_are_the_persisted_provider_kind_spellings` |
| SR-VERSION independent of schema coercion (`'1'`) | `a_quoted_schema_version_is_unsupported_even_though_the_schema_coerces_it` |
| SR-STRICT-SCALARS: quoted number, numeric operand, float version, with pointer | `strict_scalars_reject_authored_type_mismatches_at_their_pointer` + the two `sr-strict-scalars--*` fixtures |
| SR-TOP-LEVEL | `an_unknown_top_level_key_is_rejected_by_rule` |
| SR-SCHEMA-BINDING: missing or foreign `$schema`; shipped legacy docs | `the_schema_must_be_the_contract_for_the_file_kind`, `shipped_legacy_documents_are_reported_as_unbound` |
| Typed errors with repo-relative paths, no host paths | `unreadable_files_are_errors_with_repository_relative_paths`; every semantic finding's path is checked portable in `assert_semantic_rules` |
| All 30 SR rules reject their fixtures, with only that rule | `typed::{identity,constraint,coverage,binding,interaction,error}_rules_reject_their_fixtures`; `every_semantic_fixture_is_exercised_by_a_rule_family` |
| Positive corpus clean | `typed::{contract,interaction,diagnostic}_documents_pass_the_semantic_rules` |
| Eligibility keeps UTF-8 / scalar / UTF-16 / grapheme / parsed / serialized distinctions | `executable_constraints_keep_their_researched_unit_and_stage` |
| Ambiguous, unknown, conflicting, not-applicable, advisory, and secondary-only facts are never executable | `ambiguous_unknown_and_advisory_constraints_never_become_executable`; `expect-ineligible` fixtures |
| Unresolved condition makes a known bound ineligible (with and without the gap) | `an_unresolved_condition_makes_an_otherwise_known_bound_ineligible` |
| Applicability: co-holding vs exclusive vs identical conditions | `only_conditions_that_can_hold_together_make_two_values_ambiguous` |
| Missing interface, category, or platform never means unrestricted | `an_interface_without_a_matrix_reports_missing_not_unrestricted`, `accepted_scope_requires_the_whole_fleet_and_every_roster_interface` |
| Stale but valid research stays inspectable | `stale_or_invalid_research_remains_inspectable` |
| Deterministic, sorted findings | `findings_are_deterministic_and_sorted` |
| Overrides: expired (boundary day), orphaned, stale target, stale schema | `overrides_fail_precisely_when_expired_orphaned_or_stale` |
| Assessment reuse only on fingerprint equality; revision and unrelated files ignored; CRLF equal; code or fact change → `unassessed` + gap; removed fact; proposed never a claim | `assessments_are_reused_only_while_every_fingerprint_matches` |
| Error replay: match, near miss, unknown code, warnings, plain text, header locators | `diagnostics/*` fixtures via the positive corpus; `sr-fixtures--*`, `sr-match-overlap--*`; `replay::tests::*` |
| Canonical JSON and fingerprints | `canonical::tests::*` |
| Passivity of typed load and validation | `typed::typed_loading_and_validation_are_passive` (effect-engine and network counters, bytes unchanged) |
| Normal send build free of maintenance deps | `cargo tree -e normal` gate below |

There is no persisted read/write/read round trip in this phase: Phase 3
writes nothing. The DTO round trip through the schema stands in for it, and
Phase 4 owns persistence round trips.

### Gates (macOS)

- `just test` in `messenger/` (now `desktop,research`): 587 run, 587 passed,
  2 skipped (pre-existing skips).
- `just lint` in `messenger/`: clean.
- `cargo nextest run -p messenger --all-features`: 509 run, 509 passed,
  2 skipped (the first run had two corpus timeouts, fixed as described above).
- `cargo clippy -p messenger --all-features --all-targets -- -D warnings`: clean.
- `cargo nextest run -p messenger --features research` (default providers
  plus research, no desktop): 236 run, 236 passed, 2 skipped.
- `cargo tree -p messenger -e normal` with `--no-default-features`, default,
  and `--features desktop`: no `darkmatter`, `biscuit-file`, `biscuit-hash`,
  `serde_path_to_error`, YAML, or Claudine crate. `--features research` adds
  exactly those four, and no Claudine.
- Pre-existing, not from this phase: `cargo nextest run -p messenger
  --no-default-features` does not compile the lib's unit tests
  (`src/tests/validation.rs:447` uses `Target::discord_channel` without the
  `discord` feature). The same happens with or without `research`; the
  `--no-default-features` library itself compiles, including `research`.

### Cross-OS evidence

- **Linux:** Docker Desktop (`rust:1`, rustc 1.97.1, `linux/arm64`) on a copy
  of the worktree: `research_corpus` 28/28, `research_validation` 19/19,
  research lib unit tests 8/8. `just cross-check --os linux` is still queued
  behind the stale `build-linux` lock held since 2026-09-14 by
  `reward-20260914-c3e60d0` (`nightly-reward-spike`); the foreign lock was
  not removed.
- **Windows:** compile evidence only. `cargo check -p messenger --features
  research --target x86_64-pc-windows-gnu --tests` passed.
  `just cross-check --os windows` failed again because `W:` on
  `build-win-native` is full ("No space left on device"); it was not cleaned,
  because that is a human decision on a shared host.
- **WSL:** `just cross-check --os wsl` failed with an SSH connection reset
  to `build-win`, as in Phases 1–2.
- A first attempt passed `-E 'binary(…) | binary(…)'` to `just cross-check`,
  which breaks its argument line (already documented in the `os` skill's
  `build-hosts.md`). New fact added to `os/macos.md`: Docker bind mounts under
  `/tmp` fail on this host, so Linux containers need source and target dirs
  under `$HOME`.
- OS risk review: repository paths are built from `Path::components` and
  joined with `/`; schema binding compares canonicalized paths (verbatim
  `\\?\` on both sides); fingerprints normalize CRLF; tests read
  `CARGO_MANIFEST_DIR` at run time for the WSL archive leg. The macOS `/var`
  symlinked tempdir already exercises the non-canonical workspace-root path.

## Phase 4

### What landed

- `messenger::research` (feature `research`) gained five modules:
  - `publish`: the Phase 1 strategy-A protocol (committed manifest
    `docs/research/publication.json` + local journal under
    `messenger/.research-state/publication/`), ported from the spike
    with `publish`, `recover`, `read_verified`, `pending`, fault points
    (`Options::interrupt_at`), `std::fs::rename` replacement with the bounded
    Windows 5/32 retry, Unix directory fsync, and the `File::try_lock` lock.
    Manifest hashes use the `xxh64:` spelling of `research::canonical`.
  - `project`: the `catalog.json` DTOs (serialization only) and `project()`.
  - `delta`: `compare(previous, candidate, mappings)`.
  - `report`: `CatalogView` (read model of the published catalog), `build()`
    with platform/interface/operation filters, eleven fact sections,
    freshness, coverage, implementation gaps, and the seven-adapter
    truncation and diagnostic handoffs; `emitted_surfaces()` is Messenger's
    own per-adapter surface list (from the Phase 1 surface inventory).
  - `generate`: `load_fleet`, `Fleet::{validate, snapshot}`, `generate`,
    `check`, `published_catalog`.
- Supporting changes: `Loader::load_document_text` (judge candidate text as
  if it sat at the accepted path; `load` now reads text then shares that
  path), `ValidatedDocument::{frontmatter, fact_records}`, `Date::{plus_days,
  from_unix_days}`, `Workspace::{fleet_prompt, document, manifest,
  state_dir}` plus path constants, `paths::is_portable` (moved from
  `assess`, now shared with publication), and `CoverageState: Deserialize`.
- `messenger-cli`: `messenger research validate|generate [--check]|report|recover`
  (`cli/src/research.rs`), with `--root`, `--today`, `--json`, and exit
  statuses 0 / 1 (findings, drift, refusal) / 2 (usage) / 3 (cannot run).
  The CLI now enables `messenger`'s `research` feature.
- Root `.gitignore`: `messenger/.research-state/` and `*.publish-tmp`.
- Fixture: `lib/tests/fixtures/research/lifecycle/fleet/` — five
  Accepted-scope documents (every roster interface, one investigated gap per
  platform, Discord field/aggregate and Slack truncating constraints).

### Decisions and rulings

- **Wave 10 scope under pending human review.** The spec still has
  `human_review: true` for the lifecycle subcommands. Only the four
  spec-required commands plus `recover` were added. `recover` is exposed
  because the snapshot protocol cannot be operated without it (writers refuse
  with `RecoveryRequired`, and the architecture record forbids hiding recovery
  inside `generate`). `prepare`, `runs`, `promote`, and `cleanup` belong to
  Phase 6 and still need the ruling.
- **Snapshot contents (Phase 4).** Artifacts are the five accepted documents
  (byte for byte), `catalog.json`, and `summary/platforms.md`. CHANGELOG,
  review records, and the skill projection are added by Phases 6 and 8 as
  further artifacts of the same manifest.
- **Baselines.** With a published snapshot, documents come only from its
  verified bytes or from explicit `updates` (partial refresh); a document at
  the fixed path that the manifest does not list is treated as missing. With
  no snapshot, the fixed paths are the initial baseline. A hand edit of any
  published artifact makes `generate`, `check`, and `report` refuse (exit 3)
  until the file is restored or regenerated.
- **Summary is authored prose + one generated region.** Its manifest entry
  hashes only the generated region (`ArtifactScope::GeneratedRegions`, regions
  joined by NUL), so maintainers can edit the prose without drift; editing
  inside the markers is a verification failure. A summary without exactly one
  well-formed region refuses generation.
- **Freshness is a date, not a verdict.** The catalog stores `last_updated` and
  `refresh_due` (roster interval); staleness is computed by the reader
  (`--today`). Generation output therefore never changes merely because time
  passed. Override expiry still depends on `today` (it is a validation rule).
- **Reports read the published catalog, not re-validated inputs.** An expired
  override makes `generate`/`check` refuse, yet `report` still works: the spec
  requires expired accepted research to stay inspectable. `CatalogView` carries
  eligibility as reason codes, so no executable type can be deserialized from
  a file (the Phase 3 layering holds).
- **Overrides are reported, never executed.** The catalog shows researched and
  effective values; eligibility is computed from the researched value only.
  Wiring overrides into the executable projection is left to the truncation
  follow-up.
- **Delta flags** (fixed set): `removed_constraint`, `raised_limit` (max up or
  min down, same kind), `support_reversal` (any `…/support` crossing
  `unsupported`), `changed_unit` (unit or measurement stage),
  `conflicting_evidence` (newly `conflicting`), `new_unmappable_value` (a typed,
  non-knowledge leaf becoming `unknown`/`unmapped`/`unspecified_characters`).
  Record prose is `knowledge.explanation` and `note`; evidence references are
  `knowledge.evidence`; everything else is a typed value. Initial research
  reports `baseline: none` and still runs the flags over added records.
- **Catalog ordering** is by the spelled `platform_id` (alphabetical), not
  enum declaration order; a test caught the difference.
- **Security:** manifest and journal paths must pass `is_portable` before use.
  A hostile manifest could otherwise make publication delete files outside the
  repository (test: `a_manifest_naming_paths_outside_the_repository_is_refused`).
- **Human report layout.** Manual inspection at 60 and 120 columns showed
  dense `Table`s failing ("Table could not be rendered in N columns") and
  `Prose::escape_text` leaking backslashes into `UnorderedList` items. The
  report now uses one small freshness `Table` and word-wrapped
  `UnorderedList`s for everything else; list items are plain text.
- **WSL archive leg:** `research_cli.rs` resolves the binary with
  `biscuit_test_harness::bin_exe!` (new dev-dependency of `messenger-cli`)
  instead of `env!("CARGO_BIN_EXE_messenger")`, per the `os` skill.
- **Drift fixed in docs:** `messenger/cli/README.md` said "four subcommands"
  and omitted `replace`, `dismiss`, `info`, and `install`; the list is now
  complete.

### Requirement-to-test mapping (Phase 4)

| Behavior | Test(s) |
|---|---|
| Catalog: every fact, unknowns, ineligible reasons, provenance, freshness dates, sorted keys, no host paths | `research_publication::the_catalog_retains_unknowns_ineligibility_provenance_and_freshness`, `two_generations_from_identical_inputs_are_byte_identical` |
| Output bound to schema/input hashes; a changed schema invalidates the projection | `a_changed_schema_invalidates_the_published_projection`; manifest input assertions in the catalog test |
| Byte-identical double generation (library and real binary, and across two repositories) | `two_generations_from_identical_inputs_are_byte_identical`; `research_cli::generate_twice_is_byte_identical_and_check_detects_drift` |
| `--check` drift (input change) without writing; hand edits refused | `check_reports_drift_for_changed_inputs_and_refuses_hand_edits`; CLI drift test (exit 1 / exit 3) |
| Summary keeps authored prose; only the generated region is verified | `the_summary_keeps_authored_prose_and_regenerates_only_its_region`; `publish::tests::*` |
| Invalid inputs refused, snapshot untouched; published catalog still inspectable; overrides researched vs effective | `invalid_inputs_are_refused_and_leave_the_snapshot_untouched`; `research_cli::invalid_inputs_are_refused_and_leave_the_snapshot_unchanged` |
| Missing initial baseline refuses and writes nothing | `a_missing_initial_baseline_refuses_publication_and_writes_nothing` |
| Only verified documents carry over | `a_published_snapshot_is_the_only_baseline_for_carried_documents` |
| Partial refresh keeps prior documents byte for byte with their own dates | `a_partial_refresh_carries_prior_documents_with_their_own_dates` |
| Schema-version mix and insufficient coverage block publication | `incompatible_or_incomplete_documents_block_a_partial_refresh` |
| Interruption at every step (staging, journal, each of 7 replacements, manifest, selection, cleanup): verified reads see old, new, or refuse; recovery converges; follow-up succeeds; no leftovers | `a_generation_interrupted_before_selection_recovers_to_the_old_snapshot`, `a_generation_interrupted_after_selection_recovers_to_the_new_snapshot` |
| Interrupted recovery, interrupted initial publication, corrupt journal/manifest, hostile manifest paths | `an_interrupted_recovery_is_repaired_by_running_recovery_again`, `an_interrupted_initial_publication_rolls_back_to_no_snapshot`, `recovery_refuses_a_manifest_matching_neither_side_of_the_journal`, `a_manifest_naming_paths_outside_the_repository_is_refused` |
| Pending journal blocks CLI `generate`/`--check`/`report` until `recover` | `research_cli::an_interrupted_generation_requires_explicit_recovery` |
| Lock and atomic replacement primitives | `publish::fsutil::tests::*` |
| Delta: initial baseline, unchanged, raised/lowered limit, unit, removal, unmappable, support reversal, newly conflicting, same-URL evidence, prose-only (body and explanation), affected assessments, determinism | `research_lifecycle::*` (9) |
| Truncation handoff for all seven adapters; every emitted surface has an enforceability reason (enforceable / not enforceable with reasons / no researched bound with gap), aggregates, service-side truncation | `reports_hand_off_every_emitted_surface_with_an_enforceability_reason`; `report::tests::every_chat_adapter_declares_a_body_surface` |
| Diagnostic handoff (no envelope, unassessed handling) | same test |
| Freshness boundary (stale on the refresh date) | same test; `research_cli::report_json_is_filterable_and_marks_stale_research` |
| Filters (platform, interface, operation) | `report_filters_narrow_rows_and_handoffs`; CLI report test |
| CLI help, usage errors, exit statuses, JSON-only stdout without escapes, human output | `research_cli::*` (8) |
| Real shipped artifacts through the normal invocation path | `research_cli::validate_reports_the_shipped_legacy_documents_as_unbound` (exit 1, five `SR-SCHEMA-BINDING` findings) |
| Passive corpus over the new fixtures | `research_corpus::fleet_fixtures_validate`, `typed::fleet_documents_pass_the_semantic_rules_as_accepted_research`, sanitization scan now includes `lifecycle/fleet` |
| Date arithmetic | `project::tests::plus_days_crosses_months_years_and_leap_days` |

Persisted round trip: generate → verified read → regenerate → identical bytes
(`two_generations…`, CLI double generation), and publish → report reads the
published `catalog.json` bytes back through `CatalogView`.

### Gates (macOS)

- `just test` in `messenger/`: 634 run, 634 passed, 2 skipped (pre-existing).
  Re-run of `messenger-cli` after the `bin_exe!` change: 116/116.
- `just lint` in `messenger/`: clean (exit 0).
- `cargo nextest run -p messenger --features research`: 273 passed, 2 skipped.
- `cargo nextest run -p messenger --all-features`: 546 passed, 2 skipped.
- `cargo clippy -p messenger --all-features --all-targets -- -D warnings`: clean.
- `cargo tree -p messenger -e normal` with `--no-default-features`, default,
  and `--features desktop`: no `darkmatter`, `biscuit-file`, `biscuit-hash`,
  `serde_path_to_error`, YAML, or Claudine crate.
- Checkpoint run with the real binary on a copy of the fleet fixture tree:
  `generate` published (exit 0), a second `generate` reported "already
  current" with byte-identical catalog, summary, and manifest, `generate
  --check` reported no drift, and `report --json` contained no ESC byte.
- Manual inspection at 60 and 120 columns (`SHOW_RESEARCH_REPORT=1` on
  `the_human_report_renders_at_narrow_and_normal_widths`): see the layout
  ruling above.
- The shipped repository: `messenger research validate` exits 1 with
  `SR-SCHEMA-BINDING` for the five legacy documents (expected until Phase 7);
  `report` and `generate --check` exit 3 (no published snapshot).
- GitNexus `detect-changes --scope all`: 21 files, risk HIGH, all through
  `messenger-cli`'s `main` and `Commands` (one new `Research` arm). The send,
  replace, dismiss, setup, info, and install flows are unchanged, and all
  116 CLI tests, including `main.rs`'s parser tests, pass. The index predates
  the new research files, so they are not in the graph.

### Cross-OS evidence

- **Windows:** compile evidence. `cargo check -p messenger --features research
  --tests` and `cargo check -p messenger-cli --tests` for
  `x86_64-pc-windows-gnu` pass. `just cross-check messenger --os windows`
  failed again: `W:` on `build-win-native` is full ("No space left on
  device"). Not cleaned (shared-host decision).
- **WSL:** `just cross-check messenger --os wsl` failed with an SSH connection
  reset to `build-win`, as in Phases 1–3.
- **Linux:** `just cross-check messenger --os linux` is still blocked by the
  stale `build-linux` lock held since 2026-09-14 (`reward-20260914-c3e60d0`);
  not removed. Docker evidence instead
  (`rust:1`, `linux/arm64`, a copy of the worktree under
  `~/.cache/rb-linux`, `cargo test` because the image has no nextest):
  `research_corpus` 30/30, `research_lifecycle` 9/9, `research_publication`
  19/19 (every fault point, recovery, and the lock on a Linux kernel),
  `research_validation` 19/19, research lib unit tests 15/15, and
  `messenger-cli` `research_cli` 8/8 plus its 2 unit tests (the real binary).
  A first attempt used `bash -lc`, which drops `cargo` from `PATH` while the
  pipeline still exits 0; recorded in `os/macos.md`.
- OS risk review: publication uses `std::fs::rename` (POSIX semantics on
  Windows, measured in Phase 1), a bounded retry for errors 5/32, `File::try_lock`,
  and file reads that close immediately; repository paths are split on `/`
  and joined per component; manifest and journal paths must be portable;
  committed artifacts are LF (`.gitattributes` `eol=lf`) and generation emits
  LF only; the CLI test binary resolves through `bin_exe!` for the WSL archive
  leg; `CARGO_MANIFEST_DIR` is read at run time.

## Phase 5

### What landed

- **`claudine-cli` budget module** (`claudine/cli/src/budget/`). It holds the
  persisted ledger model and its pure transitions (`model.rs`, with time passed
  in as `elapsed_ms` so the arithmetic is unit-testable), the store (`store.rs`:
  a sibling `<ledger>.lock` OS file lock, an optional shared `exclusive_lock`,
  atomic replacement through `claudine::config::atomic::atomic_write`), and the
  live runner (`run.rs`: a monotonic clock, a heartbeat thread, orphan
  verification, and the process-wide hooks). `mod.rs` wraps a sequence body in
  `run_with_ledger` and renders the stderr summary with `Prose`.
- **`claudine budget init|show|suspend|resume|grant`**
  (`commands/budget.rs`). `init` requires both `--max-seconds` and
  `--max-invocations` (clap `range(1..)`, no defaults). Every mutation takes
  the ledger lock and first recovers a crashed runner.
- **`claudine sequence --budget-ledger <path>`**. It is refused with
  `--interactive` (passthrough does not isolate the process tree) and with
  `--dry-run`. Without the flag, every hook is a no-op.
- **Hook points** (each a line or two):
  - admission in `execute_attempt_phase`, just before
    `execute_harness_attempt`: debit, then cap `timeout_config.timeout` at the
    remaining time. The cap is applied **after** `session_compat_key`, so a
    resume is not refused over the shrinking deadline;
  - settlement right after the attempt returns;
  - the retry `delay` capped in `control_dispatch`;
  - `enter_stage` at each step boundary in `run_sequence_steps`;
  - `attach_stop_flag` on the sequence's interrupt flag in `execute_sequence`;
  - `record_child` at the four spawn sites (`captured`, `inherited`,
    `semantic`, Kimi `wiring/session`).
- **Documentation.** `claudine/docs/cli/budget.md` (new), plus the sequence
  CLI doc, README, and both dependency docs.

### Decisions and rulings

- **Process-wide active run instead of threading a handle.** The launch funnel
  is reached from step bodies, lifecycle retry/resume/proxy re-entry, and
  parallel group tasks. Threading a parameter through `run_harness_loop`'s
  callers and the group-task path would have touched many signatures. One
  Claudine process runs at most one budgeted sequence, so `budget::run`
  installs the run process-wide. A thread-local keeps each attempt's
  admission, spawn, and settlement together, so parallel group tasks are
  separate `in_flight` entries.
- **States.** `stopped` → `active` → one of: `suspended` (success, "awaiting
  human review"), `stopped` (failure or Ctrl+C), or `exhausted`. `interrupted`
  is reached only through crash recovery. A run starts only from `stopped`.
  `resume` leaves `suspended` or `interrupted`. Only a grant that restores
  **both** limits leaves `exhausted`.
- **Exit statuses.** `76` means exhausted (Claudine already uses `75` for
  `LOOP_RATE_LIMITED_EXIT_CODE`). `77` means blocked: suspended, interrupted,
  or a lock is held.
- **Time charging.** The clock runs by wall clock from ledger open to close.
  Time outside a budgeted run is not observable. Messenger must therefore run
  validation and delta as `shell:` steps inside the sequence, which the
  architecture already requires.
- **Deadline rounding (found by test).** The wall-clock timeout plumbing
  carries whole seconds (`Duration::from_secs(seconds)` in
  `exec/spawn/setup.rs`), so a 1.6 s cap became 1 s. The first fixture run
  killed the worker at 1342 ms of a 2000 ms budget, and the run ended
  `stopped`, not `exhausted`. The cap is now rounded **up** to whole seconds.
  A launch can overrun by less than one second, and that time is charged.
  Changing the timeout plumbing to sub-second precision was out of scope.
- **Only time exhaustion is detected mid-launch (found by test).** The
  heartbeat and settlement check only `is_time_exhausted()`. Invocation
  exhaustion is detected at the next admission or step boundary. The first
  version recorded `stage: "two"` for a step that had succeeded, and the
  heartbeat made that label depend on timing. The ledger's `stage` must name
  the step that could not run.
- **Crash accounting.** Recovery charges `max(one heartbeat interval,
  time a verified orphan kept running)` past the last persisted heartbeat. It
  never refunds, then sets the ledger to `interrupted`, so downtime until an
  operator resumes is not charged, as the architecture record decided. The
  orphan is signaled only when its PID's start time (`sysinfo`) matches the
  one recorded at spawn: the whole process group on Unix, `TerminateProcess`
  on Windows. Recovery runs in whichever command next takes the lock.
- **One platform at a time.** This is a generic `--exclusive-lock <path>` on
  `init`, a lock file every run sharing it must hold. Claudine knows nothing
  about platforms. Messenger (Phase 6) points every platform's ledger at one
  fleet lock.
- **`budget` belongs in the help "Administration" group**, not
  "Composition". The Composition group and `argv::COMPOSITION_SUBCOMMANDS`
  are defined as the three composition subcommands.
- **Error guard.** `OrphanOutcome::TerminateFailed` carries the typed
  `io::Error`. `run_with_ledger` copies the sequence error's text into the
  persisted `stop_reason` but returns the typed error unchanged. That one
  site has a `transport-allow.toml` entry (`retained`) saying so.
- **Not in Claudine.** Discovery-input isolation needed no Claudine change
  (orchestration findings). The fixture proves separately prepared prompt
  documents stay separate. Sequence progress is not persisted, so a
  restarted sequence re-runs from step 1. Candidate resumption is Messenger's
  Phase 6 work.

### GitNexus

- `just gitnexus` backed off: another `gitnexus analyze` (pid 78755) held the
  index lock. The CLI fallback (`node .gitnexus/run.cjs impact … --repo .`)
  returned `Target not found` / `risk: UNKNOWN` for every hook symbol,
  because the index was still being rebuilt. As the repository rules
  require, `UNKNOWN` was treated as unresolved, and callers were confirmed by
  text search:
  - `execute_attempt_phase`: 1 caller (`run_harness_loop_inner`);
  - `run_sequence_steps`: 1;
  - `execute_sequence`: 1;
  - `run_sequence`: 1;
  - `run_sequence_inner`: 2 (including 1 test);
  - `SequenceArgs {…}` literals: 2;
  - `Commands` matches: `main.rs` and `telemetry.rs`;
  - `*child_spawned = true`: 4 spawn sites.
- `execute_attempt_phase` is the funnel for every agent launch, so it was
  treated as high risk. The edit is guarded so that with no ledger it changes
  nothing: `admit_launch` returns `Ok(None)`. The full Claudine suite confirms
  unchanged behavior. `isolate_into_process_group` (CRITICAL) was not touched.
- `detect-changes --scope all` after implementation: 37 files, 24 symbols,
  11 affected flows, **risk high**. The high rating comes from the four spawn
  functions (`run_child_capture`, `run_child`, `run_child_stream_semantic`,
  `run_kimi_wire_session`) and `async_main`, each of which gained one
  `budget::record_child` call or command arm. Without a ledger that call is a
  no-op, and the full `claudine-cli` suite exercises those flows. The index
  still does not map the `loop_control` or sequence symbols, so this is not a
  complete picture. Re-run after `just gitnexus` before committing.

### Requirement-to-test mapping (Phase 5)

| Requirement | Test(s) |
|---|---|
| Both limits required and positive; no defaults; missing, zero, foreign, or unknown fields refused on load | `budget::tests::a_ledger_requires_both_positive_limits_and_an_identity`, `loading_rejects_missing_or_zero_limits_and_foreign_formats`; `sequence_budget::init_requires_both_positive_limits_and_a_sequence_refuses_a_missing_ledger` (real binary: exit 2 and no file; a sequence with no ledger exits 1 with **zero** launches) |
| Debit before spawn; never refunded; each pass, retry, and reviewer charged to one ledger | `every_admission_debits_one_invocation_before_spawn_and_settling_never_refunds`; `sequence_budget::every_pass_retry_and_reviewer_launch_is_charged_to_one_ledger` (5 real fake-provider processes = 5 `admitted`/`spawned`/`settled`, stages in order, validation `shell:` step included) |
| Discovery vs reconciliation inputs stay separate | same fixture: the discovery prompt contains `DISCOVERY-INPUT` and none of `PRIOR-PROSE`, `CURATED-LINK`, or `RECONCILE-INPUT`; both reconciliation launches contain them |
| Exhaustion before a launch stops dispatch and keeps the incomplete stage; a restart does not reset; resume cannot bypass it; only a grant adds allowance | `elapsed_time_exhausts_at_a_stage_boundary_and_is_recorded_once`, `only_a_recorded_grant_reopens_an_exhausted_ledger`; `sequence_budget::exhaustion_before_launch_stops_dispatch_and_only_a_grant_adds_allowance` (exit 76, 2 launches, `stage: three`, a rerun makes no launch and keeps `runs: 1`, and after a grant exactly one more launch runs with `runs: 2`) |
| Exhaustion during a launch stops the local worker at the shared deadline and records that remote work is unverified | `sequence_budget::a_running_worker_is_stopped_at_the_shared_deadline` (worker PID gone, no `done` marker, `active_ms ≥ 2000`, the `settled` detail says remote work is unverified) |
| Automatic backoff charged and capped | `sequence_budget::automatic_retry_backoff_is_charged_and_capped_to_the_remaining_budget` (a 60 s delay capped, `active_ms ≥ 3000`, 1 launch, the retry refused) |
| Suspension not charged; resume needed; consumption accumulates | `only_a_stopped_ledger_opens_and_suspension_requires_an_operator_resume`; `sequence_budget::a_suspended_ledger_is_not_charged_and_needs_an_operator_to_resume` (`used` byte-identical across a 1.2 s suspension and a refused run; after resume `invocations: 6`, `runs: 2`; explicit `suspend` behaves the same; `show --json` has no ESC) |
| Crash: no refund, conservative tail, orphan stopped, operator resume required | `crash_recovery_charges_the_unseen_tail_and_requires_resumption`, `a_terminated_orphan_charges_the_time_it_kept_working`; `sequence_budget::a_crashed_runner_is_charged_conservatively_and_its_orphan_is_stopped` (SIGKILL/TerminateProcess on the runner; the ledger stays `active` with the worker PID; the restart recovers to `interrupted` with exit 77; `invocations` stays 1; `active_ms ≥ persisted + 200`; on Unix the orphan is **terminated by recovery**) |
| One platform at a time | `a_held_ledger_or_exclusive_lock_refuses_a_second_holder`; `sequence_budget::runs_sharing_an_exclusive_lock_execute_one_platform_at_a_time` (the second platform exits 77 with 0 launches and its ledger untouched, then runs after the first finishes) |
| Persisted round trip | `a_ledger_round_trips_through_disk_byte_for_byte` (create → read → write gives identical bytes, LF only) |
| Existing `sequence` unchanged without a ledger | `sequence_budget::without_a_ledger_a_sequence_writes_no_budget_state`; full `claudine-cli` suite (all sequence, lifecycle-retry, termination, and ctrl-C tests pass) |
| `--interactive` / `--dry-run` refused before charging | `sequence_budget::a_ledger_refuses_interactive_and_dry_run_modes_before_charging` (ledger JSON unchanged) |

The fake provider is Rust compiled with `rustc` (the `sequence_cli.rs` Windows
pattern), so every fixture is portable to native Windows. There are no
credentials, network, audio, or windows, and the fixture environment is
hermetic.

### Gates (macOS)

- `cargo nextest run -p claudine-cli --test sequence_budget`: 10/10, three
  consecutive runs (about 4 s each). Ledger unit tests: 11/11.
- `just test` in `claudine/` (`--no-fail-fast`): 7329 run, 7316 passed,
  13 failed, 9 skipped. 3 failures were caused by this phase and are fixed
  (`commands::help::tests::composition_group…`,
  `wrap_basics::help_lists_wrapper_subcommands` snapshot, and
  `error_guards::production_sources_pass_every_scan_backed_guard`); a rerun
  of those binaries plus the budget tests gives 64/64. **10 failures predate
  this phase**. All come from drift in shipped `prompts/` files, which this
  phase did not touch:
  - `claudine::composition::resolve::tests::cross_platform_prompt_composes_cleanly`
    reads `prompts/cross-platform.md`, which commit `41f9adeb8` deleted;
  - `claudine::composition::schema::tests::shipped_implement_plan_*` (2)
    fail on `has_skill(): skill name must be a basename`;
  - `shipped_prompt_route_drift` (3), `shipped_prompt_contract` (2),
    `shipped_prompts` (1), and
    `compose_caller_file_provenance::shipped_implement_router_refuses_an_archived_case`
    fail because the `prompts/_implement/implement-plan.md` body and hash
    differ from the Level 2 fixture, and `prompts/implement.md` uses a
    `^prompt` link the expression parser rejects.
- `just lint` in `claudine/`: exit 0. The only warning is the macOS linker's
  `__eh_frame` notice, which every build of this binary prints.
- `test_placement` (the 800-line production and 300-line inline-test
  budgets): passes.
- `just test` in `messenger/`: 634 passed, 2 skipped (both pre-existing).
  `just lint` in `messenger/`: exit 0. This phase changed no messenger source;
  these runs confirm that.

### Cross-OS evidence

- **Linux (real kernel, Docker `rust:1`, `linux/arm64`, kernel 6.18).**
  Worktree copied to `~/.cache/rb-linux/src` (the `os/macos.md` procedure;
  `rust:1` also needs `libasound2-dev` for `claudine-cli`, now recorded
  there). `cargo test`, because the image has no nextest:
  - `--bin claudine budget::`: 11/11;
  - `--test sequence_budget`: 10/10. This includes crash recovery killing
    the orphan's process group, and the shared fleet lock on Linux `flock`;
  - `sequence_cli` 30/30, `sequence_groups` 20/20, `sequence_schema` 7/7,
    and `wrap_watchdog_timeout` 6/6: existing sequence and timeout
    termination behavior is unchanged.
- **`build-linux`** (`just cross-check claudine-cli --os linux --test
  sequence_budget`): gave up after the 1800 s wait. The stale lock
  `reward-20260914-c3e60d0` has been held since 2026-09-14 and was not
  removed.
- **Native Windows.** `just check-windows` in `claudine/`
  (`x86_64-pc-windows-gnu`, `claudine` and `claudine-cli` with `--tests`)
  passes; neither warning is in budget code. `just cross-check … --os
  windows` failed: `W:` on `build-win-native` is full (`scp … Failure`,
  "No space left on device"). This is compile evidence only. The Windows
  paths never ran: the `kill_tree` `TerminateProcess` branch, the Job Object
  taking the worker down with a killed runner (the crash fixture accepts
  "no longer running" on Windows), and `File::try_lock` on two handles.
- **WSL2.** `just cross-check … --os wsl` failed with SSH connection resets to
  `build-win`, as in Phases 1–4.
- **OS risk review.**
  - The fake provider is compiled Rust, not a shell script, so the fixtures
    are portable.
  - The ledger and lock paths are joined per component.
  - Ledger JSON is LF-only.
  - Locking uses `std::fs::File::try_lock` (per handle on Windows, per open
    file description on Unix; both refuse a second holder).
  - Atomic writes reuse Claudine's `atomic_write`, which retries the
    transient Windows 5/32/2 errors.
  - Process identity comes from `sysinfo` start time on every OS.
  - Orphan termination: `kill(-pgid, SIGKILL)` on Unix, falling back to the
    PID, and `TerminateProcess` on Windows. A `setsid` descendant escapes on
    Unix, as documented.
  - The validation `shell:` step is `echo`, which exists on all OSes.

### Checkpoint notes

- "Relevant sequence and process-termination tests on macOS and each
  available cross-platform build host": no build host was available. macOS
  ran the full suite, Linux ran in Docker, and Windows was compile-checked
  (see above).
- "The Messenger fleet rejects missing limits before launching a worker":
  proven at Claudine's boundary. `budget init` without both positive limits
  is a usage error and writes no file, and `sequence --budget-ledger` on a
  missing ledger exits 1 with zero provider launches
  (`init_requires_both_positive_limits_and_a_sequence_refuses_a_missing_ledger`).
  Messenger's own `prepare` command (architecture `run_config`) does not exist
  until Phase 6; the Phase 6 agent is told to validate both limits there.


## Phase 6

### Starting point and rulings applied

- The spec's `human_review_items` (test hosts, remaining commands, layout)
  were still open. This phase proceeded on each item's recommendation: flat
  subcommands matching the shipped `recover` (item 2, option A) and the layout
  as built (item 3, option A). Lifecycle commands added: `prepare`,
  `check-run`, `runs`, `promote`, `reject`, `cleanup`. `check-run` and
  `reject` are additions beyond the architecture record's list (see below).
- Messenger never spawns Claudine or an agent: `prepare` writes the run and
  prints the `claudine budget init` / `claudine sequence --yolo
  --budget-ledger` commands (architecture: "Messenger reaches Claudine only
  through `just` recipes"). Package-area recipes stay with Phase 8 (plan
  Wave 21).

### Environment notes (in progress)

- `/Volumes/coding` ran out of space mid-build (`ld: errno=28`, 212 MiB
  free). Applied the repository policy (`just sweep`) to this worktree only:
  the capacity backstop removed 79.34 GiB from this worktree's `target/`;
  56 GiB free afterward. No other session's files were touched.
- GitNexus: `just gitnexus` gave up after 600 s waiting on another
  `gitnexus analyze` (pid 78755, the same holder seen in Phase 5); the process
  was left alone. `impact` returned `Target not found`/`UNKNOWN` for the
  research symbols, so callers of the edited `generate.rs` functions were
  confirmed by text search: `load_fleet` (CLI `validate`, publication tests),
  `generate` (CLI `generate`, tests), `check` (CLI), `Fleet::snapshot`
  (`generate`, `check` only). All are research-feature code; no delivery path.
  (The later `detect-changes --scope all` saw 10 tracked files and 2 README
  sections, risk "low"; it does not see the new untracked files and the index
  predates the research code, so it is **not** a clean check. Re-run after
  `just gitnexus` succeeds, before committing.)

### What landed

- **`messenger::research::refresh`** (new, feature `research`):
  - `config` — `RunLimits::new(max_seconds, max_invocations)`: both required
    and positive, no defaults (the Messenger-side check Phase 5 handed over).
  - `state` — the state area: `RunId` (`{date}-{8 hex}`), `RunStatus`,
    `Stage`, `run.json` (`messenger-research-run/1`, atomic replacement, LF),
    a read-only `LedgerView` of Claudine's `budget.json`, and `apply_ledger`
    (an exhausted ledger makes an active run `exhausted`, a recovered crash
    makes it `interrupted`).
  - `select` — refresh selection with reasons `missing`, `expired`, `forced`,
    `schema_invalid`, `prompt_changed`, `schema_changed`, `no_review_record`,
    `version_changed`; skips `current` or `open_run` with the reason.
  - `prepare` — creates the run, per-pass inputs (discovery gets
    identification + contract only; reconciliation gets curated sources,
    previous document, discovery outputs), schema copies beside the candidate,
    and `run.md` (five steps plus `review-check`; validation and the review
    check are `shell:` steps so their time is charged). `resume` reruns from
    the first incomplete stage (reconcile when validation failed), max two,
    same ledger, refused while the ledger is exhausted/suspended/interrupted/
    active.
  - `records` — pass outputs (`suggested-sources.json`, `source-checks.json`,
    `source-proposal.json`, `evidence-review.json`), their checks (cap,
    contributions, replacement at capacity, every curated source attempted),
    and `unsafe_content`, a library port of the corpus scanner, applied to
    every stored free-text field (bounded to 1000 characters).
  - `check` — `evaluate`/`check_run`: judges outputs, validates the candidate
    in Accepted scope at the accepted path, integrity rules (platform,
    `created`, chronology, removals in `changes`, `last_updated` only with a
    successful check, curated `retrieved` only with a successful check on that
    date), stale-baseline refusal, delta, and evidence-review coverage of every
    changed fact/source/gap/prose.
  - `approval` — `decide`: `Ineligible` / `Renewal` / `HumanRequired`.
  - `review` — durable `ReviewRecord` (`messenger-research-review/1`, sorted
    keys), local `RenewalRecord`, and `render_changelog`.
  - `promote` — `promote(run_ids…)` publishes one or more runs together via
    `generate_with`; `reject`.
  - `cleanup` — `plan` / `apply`.
- **`generate.rs`**: carries every published review record forward, accepts new
  ones (`generate_with`), renders `docs/research/CHANGELOG.md` from them, and
  validates them under the new rule code `SR-REVIEW` (documented in
  `_rules.md`). CHANGELOG and reviews are snapshot artifacts, so history is
  selected atomically with documents and catalog.
- **CLI** (`cli/src/research_lifecycle.rs`): `prepare`, `check-run`, `runs`,
  `promote`, `reject`, `cleanup`, with `--json` and `TerminalRenderable`
  output. Refusals exit 1, missing limits exit 2, environment failures exit 3.

### Decisions (new in this phase)

- **`promote` takes several run IDs.** Publication needs every active
  platform, and Phase 7 researches one platform at a time, so a one-run
  `promote` could never publish the first baseline. Each run is judged on its
  own; nothing is accepted unless the single publication succeeds. One run
  alone is refused with the missing platforms listed, and nothing is written.
- **`check-run` and `reject` are additional commands** beyond the
  architecture record's list: `check-run` is what the sequence's `shell:`
  steps call, and rejected runs need a recorded local state.
- **CHANGELOG is generated from review records** (architecture: "generated
  from approved review records"), so candidate, rejected, and renewal
  outcomes structurally cannot appear there.
- **"Researched under"** (prompt/schema fingerprints) is recorded in each
  review and renewal record; selection compares the newest one with the
  current contract. A generate-only snapshot has none, so its platforms read
  `no_review_record` (true of the fixture fleet).
- **Promotion never applies a curated-list proposal.** The proposal is kept
  in the review record; changing the roster stays a human edit, which is how
  "human approval for every curated-list change" is enforced.
- **A worker cannot write accepted research.** Candidates live in the state
  area; a direct edit of an accepted file makes `read_verified` refuse the
  snapshot (tested).
- **Unchanged renewal rule**: frontmatter equal after removing
  `last_updated`, `agent`, `model`, and `sources[*].retrieved`; same body
  hash; no curated change; same prompt and schema as the last record; no
  inaccessible check; every URL source rechecked on the date it records.
- **Shell-step root spelled with `/`** so a Windows path's backslashes are
  never tokenizer escapes (Windows accepts `C:/…`).

### Bugs found by tests

- `RunId::generate` used `xxh64_digest`'s `xxh64:` prefix as hex (unit test).
- `delta::compare` panics on a cross-platform candidate; `check` now skips
  the delta on a platform mismatch and reports it (lifecycle test).
- An unparseable candidate aborted `check-run`; it is now a validation
  finding.

### Requirement-to-test mapping (Phase 6)

| Requirement (plan / spec AC) | Test(s) |
|---|---|
| Selection reasons and auditable skips (AC 8 "skipped current items") | `research_refresh::selection_names_every_due_reason_and_skips_current_or_open_platforms`, `an_accepted_document_that_no_longer_validates_is_due`; CLI `prepare_requires_both_limits…` (dry-run JSON) |
| Missing limits rejected before any write (AC 33) | `refresh::config::tests::both_limits_are_required_and_positive`; CLI `prepare_requires_both_limits_before_writing_anything` (exit 2 for each missing/zero variant; no `runs/` dir; JSON refusal) |
| Discovery isolation; reconciliation inputs (AC 24) | `discovery_inputs_hold_no_previous_research_or_curated_sources` (also sequence shape, both limits and fleet lock in the printed commands, run.json round trip) |
| Initial migration uses legacy prose as input, not baseline (AC 26) | `initial_research_reconciles_legacy_prose_without_treating_it_as_a_baseline` |
| Outputs judged, not exit codes; missing/malformed/contradictory/timestamp-only output; integrity rules (AC 8, 25, 36) | `stages_are_judged_from_their_outputs_not_exit_codes`; CLI run test (exit 1 with nothing written) |
| Delta after validation, before promotion; same-URL evidence; unresolved stays unresolved; mechanical vs agent separate (AC 26) | `a_reviewed_change_publishes_with_its_review_record_and_changelog_entry`, `stages_are_judged…` (review coverage) |
| Approval policy (AC 29, 34) | `automatic_renewal_is_refused_unless_everything_substantive_is_unchanged_and_rechecked` (failed check, source added, prose changed, curated change), `a_verified_unchanged_renewal_is_accepted_without_a_changelog_entry` (prose preserved, dates advance), timestamp-only in `stages_are_judged…` |
| Initial baseline needs human approval and every platform (AC 28, 30) | `an_initial_publication_promotes_every_platform_together` |
| Partial refresh keeps failed platform resumable with real dates (AC 30) | `a_partial_refresh_publishes_successes_and_keeps_failed_platforms_resumable` |
| CHANGELOG vs review records; rejected/renewal never listed; no transcripts/host paths (AC 31, 36) | `a_reviewed_change…` (scan of every committed artifact), `a_verified_unchanged_renewal…`, `automatic_renewal_is_refused…` (4 rejections, 1 entry) |
| Suggestions never become evidence/curated; roster and mappings untouched (AC 24, 32) | `a_reviewed_change…` |
| Bounded recovery, no extra budget (AC 8, 33) | `recovery_is_bounded_and_never_adds_budget`, `an_exhausted_budget_stays_incomplete_and_needs_a_recorded_grant` |
| Interrupted promotion (AC 34) | `an_interrupted_promotion_recovers_to_one_consistent_snapshot_and_completes_on_retry` (all 8 fault points) |
| Retention (AC 35) | `cleanup_previews_exact_removals_and_protects_open_runs` (held ledger lock, unreadable record, resumability, exact apply); CLI `cleanup_previews_by_default_and_deletes_only_with_apply` |
| Candidates never overwrite accepted research | `prepared_runs_never_write_accepted_research_even_when_the_worker_misbehaves`; tree-unchanged asserts throughout |
| End-to-end through the binary | CLI `a_run_goes_from_preparation_to_promotion_through_the_binary` |
| Persisted round trips | run.json (discovery-inputs test), review record `to_bytes → parse → to_bytes` (reviewed-change test) |

The passive corpus test (`research_corpus.rs`) still covers every shipped
contract file, including the edited `_fleet.md` and `_rules.md` (SR-REVIEW
documented, as the corpus test requires for every `Rule`).

### Gates

- macOS: `just test` in `messenger/`: 658 passed, 2 skipped (both
  pre-existing). `just lint`: exit 0. `cargo clippy -p messenger
  --all-features --all-targets -- -D warnings`: exit 0. `cargo nextest run
  -p messenger --all-features`: 567 passed, 2 skipped. `cargo tree -e normal`
  for no-default, default, and `desktop`: no `darkmatter`, `biscuit-file`, or
  YAML crate.
- Manual: a scratch repository (removed afterwards) ran generate → prepare →
  fake outputs → `check-run` ×2 → `promote` with the real binary; the review
  record and CHANGELOG were read by hand (concise, sorted, no host paths).
  `claudine sequence --yolo --dry-run` on the generated `run.md` composed all
  6 steps and the discovery prompt.

### Cross-OS evidence

- **Linux (Docker `rust:1`, arm64, real kernel)** per `os/macos.md`:
  `research_refresh` 15/15, `--lib refresh` 6/6, `research_lifecycle_cli`
  3/3 (including the held-lock cleanup protection on Linux `flock`).
- **`build-linux`**: `just cross-check messenger --os linux` still waits on
  the stale `reward-20260914-c3e60d0` lock; stopped, lock not removed.
- **Native Windows**: `cargo check -p messenger --features research -p
  messenger-cli --tests --target x86_64-pc-windows-gnu`: exit 0, no warnings
  (compile evidence only; the 1.9 GB check target was deleted afterwards).
  Not run: `File::try_lock` against Claudine's held lock on Windows, and
  `remove_dir_all` of a run directory.
- **WSL2**: not attempted (same blocked host as Phases 1–5).

### Not done here (by plan)

- Package-area `just` recipes for refresh/publish/cleanup: Phase 8 Wave 21.
- The skill projection `platform-metadata.md`: Phase 8.

## Phase 7

### Outcome: blocked on required human input

No Phase 7 task is complete, and none is checked off in the plan. Every wave
(16–20) needs a live research run, and the checkpoint needs a person to
approve the first baseline. The specification makes both conditional on
things only a person can supply:

- "Before live research starts, require configured per-platform limits for
  elapsed time and agent invocations" and "No numeric time or invocation
  defaults are established by this specification; values must be supplied
  before a live run." (spec, execution budget)
- "A human maintainer must approve substantive changes … The initial baseline
  requires this approval." (spec, review)

The Phase 6 `human_review_items` asking for those limits and an approver's
name were still unanswered at the start of this phase: the spec had no
ruling, and the git history since Phase 6 has no decision. This session is
non-interactive, so they cannot be obtained here. Inventing limits, running
agents without them, or writing the platform documents by hand would each
skip the budget, the three-pass workflow, or the human approval the spec
requires. None was done. No live research ran, no budget was consumed, and
nothing was written to `messenger/.research-state/` or to accepted research.

### Readiness work done instead (nothing live, nothing persisted)

Goal: make sure that once limits and an approver are given, the first live
run is not lost to a tooling problem.

- **Read-only checks in the worktree** (binaries built from this worktree):
  - `messenger research runs`: no runs.
  - `messenger research prepare --dry-run`: all five platforms are due
    (`no accepted document`).
  - `messenger research validate`: the shipped legacy documents fail
    `SR-SCHEMA-BINDING`. Expected: they are reconciliation input only.
- **Rehearsal in a throwaway Git repository** under `/tmp`, holding a copy of
  `messenger/docs`. It was deleted afterwards, and the worktree has no state
  area. The limits used (60 s, 1 invocation) were placeholders for this
  rehearsal only; they are not proposed values.
  - `prepare <platform>` for all five platforms: exit 0. Each run has a
    non-empty `identification.md`, `_fleet.md`, `curated-sources.md` (4–6
    sources), `previous.md` (legacy prose), and per-pass prompts. No host
    paths appear in any input.
  - `claudine budget init …` exactly as printed, all five platforms: exit 0.
    Each ledger reads `stopped, 0/1 invocations, 0.0/60.0 s`.
  - `claudine sequence --yolo --dry-run run.md`, all five platforms: all six
    steps compose (discovery → reconcile → sources → validation → review →
    review-check). The `validation` shell step then correctly failed each
    run: outputs are missing, so the run is marked `failed`, which shows
    that stages are judged by their outputs, not by exit codes.

### Findings (recorded in the messenger skill's research-contract.md)

1. **The installed `claudine` is too old.** `~/.cargo/bin/claudine` has no
   `budget` subcommand, so the printed `claudine budget init` fails unless
   this worktree's `target/debug` comes first on PATH. The Phase 6 note
   already required this for `messenger`; it applies to `claudine` too.
2. **`run.md` names no agent.** Claudine's live sequence path refuses an
   unresolved agent when no terminal is attached (`AgentResolutionFailed`,
   `claudine/cli/src/commands/wrap/sequence/mod.rs`). With a terminal it
   opens a picker, and the wait is charged to the budget. Adding a provider
   flag fixes this: with `--claude`, the dry-run resolves to `Claude / opus`
   for every step. Which agent and model research the fleet is an operator
   choice the spec does not make, so it was added as a review item instead
   of being hard-coded.
3. **`sequence --dry-run` runs `shell:` steps.** It is not side-effect free
   for a prepared run: `check-run` marks the run `failed`. Rehearse only in a
   throwaway copy.

No code changed. Findings 1 and 3 are procedure. Finding 2 could become a
`prepare --agent` option, pending the ruling (see spec `human_review_items`).

### Requirement-to-test mapping (Phase 7)

This phase has no code or schema change, so no new tests were added. Phase 7
is covered by evidence, not by tests: each wave needs one live run, with
agent outputs, validation, delta and evidence review, and human approval.
None of this could be produced here. The tooling it depends on is already
covered by the Phase 6 suites (`research_refresh`,
`research_lifecycle_cli`), which pass. The rehearsal above exercised the real
shipped roster, contract, and prompts through the normal invocation path
(`prepare` → `budget init` → `sequence`).

### Gates (macOS)

- `just test` in `messenger/`: 658 passed, 2 skipped (both skips predate this
  phase).
- `just lint` in `messenger/`: exit 0.
- No cross-OS runs: nothing platform-sensitive changed.

## Phase 8

### Outcome: partial; the baseline-dependent work is still blocked

Phase 7 never produced a reviewed baseline, because no research limits,
approver, or agent choice have been supplied (spec `human_review_items`,
unchanged since Phase 7). Phase 8 therefore did everything that does not
need accepted research and left the rest unchecked:

| Plan item | State | Why |
|---|---|---|
| Generated Summary | **not done** | `catalog.json` and the summary are generated from accepted documents. None exist: the shipped documents are legacy prose that fails `SR-SCHEMA-BINDING`, and `generate` correctly refuses them. Hand-writing a catalog would fabricate research. |
| Skill Projection (`.claude/skills/messenger/platform-metadata.md`) | **not done** | Same reason: it is a projection of the accepted snapshot. Only the skill's routing documentation was updated (recipes and lifecycle). |
| Workflow Documentation | done | |
| Messenger Gates | done | |
| Claudine Gates | done, with pre-existing failures (below) | |
| Platform Gates | **not done** | `build-linux` locked, native-Windows host disk full, WSL2 SSH reset (below) |
| Change Analysis | done | |
| Checkpoint 1 (clean-fixture demonstration) | done | |
| Checkpoint 2 (acceptance criteria → evidence, unknowns as gaps) | **not done** | needs the accepted fleet |

### Workflow documentation and recipes

- `messenger/justfile`: `research-validate`, `research-generate`,
  `research-check` (`generate --check`), `research-report`, `research-runs`,
  `research-publish NAME RUN…` (`promote --approved-by`),
  `research-cleanup` (preview unless `--apply`), and
  `research-refresh SECONDS INVOCATIONS [prepare args] [-- sequence args]`.
  - The offline recipes use `cargo run -q -p messenger-cli --`, so they are
    portable. Variadic parameters are `*args` with no default: `*args=""`
    passed one empty argument through `"$@"`, which clap rejected.
  - `research-refresh` is the only recipe that runs agents. It builds both
    CLIs, puts `<target>/debug` first on PATH (via `cygpath -u` under Git Bash,
    where `C:\…` would split PATH), and runs `prepare --json` from the
    repository root. For each run, it then executes the printed
    `budget init` and `sequence` with the operator's `-- …` arguments
    inserted after `claudine sequence`. A failed platform does not stop the
    others; the recipe exits 1 at the end. It is bash 3.2-safe (`eval` +
    `jq @sh`, not `mapfile`, which macOS's `/bin/bash` lacks) and needs `jq`,
    as `just/devops.just` already does.
- `messenger/docs/user-guide.md` gains a "Provider Research" section:
  recipe table; authored vs accepted vs generated vs local files;
  validate/generate/drift; single, full-fleet, and forced refresh with the
  required limits and the agent flag; source-access rules; review,
  renewal, and publication; partial failure and every recovery path;
  cleanup.
- `messenger/README.md` (layout and recipe pointer) and `messenger/cli/README.md`
  (stopped sequences leave a `failed` run).
- Dependency docs: no crates or feature edges were added in this phase.
  `docs/dependencies.md` already documents the `research` feature's crates.
  Generated files already carry "do not edit" notices
  (`project::GENERATED_NOTICE`, `publish::REGION_BEGIN`).

### Defect found and fixed: a stopped sequence left its run stuck `active`

Found while exercising `just research-refresh 60 1 discord` with no agent
flag and no terminal (the Phase 7 launch trap). Claudine stopped with
`AgentResolutionFailed` and left the ledger `stopped` (`runs: 1`). Messenger's
run stayed `active` indefinitely: `reject`, `prepare --resume`, and `cleanup`
all refused it, and selection skipped Discord as having an open run. Only
deleting local state by hand would release it. The same happens after
Ctrl+C or a failed agent step: Claudine's documented resting state for
those is `stopped`, and `state::apply_ledger` mapped only `exhausted` and
`interrupted`.

Fix (`messenger/lib/src/research/refresh/`):

- `LedgerView.runs` (Claudine's count of opened budgeted runs) and
  `RunRecord.ledger_runs` (that count when the attempt was prepared: 0 at
  prepare, the ledger's value at resume; `#[serde(default)]`, so existing
  `run.json` files still load).
- `apply_ledger`: an `active` run whose ledger is `stopped` **and** whose
  `runs` exceeds `ledger_runs` becomes `failed`, keeping Claudine's stop
  reason. A ledger with no run for this attempt (freshly initialized,
  resumed but not relaunched, or Claudine rejected the arguments before
  opening a run) leaves the run `active`. Its printed sequence command is
  still the way forward, and a second `resume` cannot burn an attempt.
- `select`: judges open runs by the same effective status as `runs`,
  `resume`, and `reject`, instead of the last saved status.
- GitNexus impact (index refreshed with `just gitnexus` this phase):
  `apply_ledger` MEDIUM (callers: promote, reject, check-run, resume, runs
  listing); `RunRecord` (file-qualified) LOW; `LedgerView` LOW; `resume`
  UNKNOWN. Text search: its only caller is `cli/src/research_lifecycle.rs`.
  Unqualified `RunRecord` resolved to an unrelated TypeScript type and
  reported HIGH; discarded after text search.

### Requirement-to-test mapping (Phase 8)

| Behavior | Test / evidence |
|---|---|
| Stopped ledger after a launch → resumable; exact original input (`runs: 1`, stop reason `agent resolution failed … NoAgent`) | `research_refresh::a_sequence_that_stops_before_its_checks_leaves_a_resumable_or_rejectable_run` (new; failed before the fix with `WrongStatus { status: Active }`) |
| Initialized-but-unlaunched run stays active (negative: resume and reject refused, platform still open) | same test, first block |
| Resumed-but-not-relaunched run is not re-failed; a refused resume consumes no attempt | same test, third block |
| Ctrl+C after relaunch (`runs: 2`) → rejectable; decision persisted; platform selectable again | same test, last block |
| Selection uses the effective status | same test (`discord_is_open` at each step) |
| Accepted research untouched throughout | same test (`repo.tree()` before/after) |
| `apply_ledger` state mapping, `runs: 0` vs `runs: 1` boundary | `state::tests::only_an_active_run_takes_the_ledger_resting_state` (extended) |
| Real binary, real shipped roster: stuck run released | manual: `just research-refresh 60 1 discord` (no agent) → `messenger research reject` exit 0 → `prepare --dry-run` shows Discord due |
| Recipes | manual, real worktree: `research-validate` exit 1 (legacy documents), `--json` output; `research-check`/`research-report` exit 3 (no snapshot); `research-runs`, `research-cleanup` exit 0; `research-publish "Ken Snyder" no-such-run` exit 3 (name with a space passed intact); `research-refresh` failure path exit 1; `-- --claude --dry-run` reached `claudine sequence` (Claudine refused `--dry-run` with `--budget-ledger`, as documented); "nothing due" path exit 0. All local state was deleted afterwards. |

No parser, schema, template, or prompt changed, so the passive corpus
tests (`research_corpus`) and end-to-end CLI suites from earlier phases
still cover the shipped artifacts. They pass unchanged.

### Checkpoint 1 demonstration (clean fixture, real binaries)

Throwaway Git repository `/tmp/p8demo`: the shipped roster, schemas, and
fleet prompt plus `lib/tests/fixtures/research/lifecycle/fleet/`. Script:
`/tmp/p8demo.sh`.

1. `validate` → exit 0.
2. `generate` twice → exit 0 both times; the second reports "already
   current". The digest of every file under `messenger/docs` is identical
   (`cad91aa7bd1ed86a`).
3. `generate --check` → "No drift", exit 0.
4. Human `report --platform discord` renders; `report --json` parses
   (8 top-level keys) and contains 0 escape sequences.
5. Failed partial refresh: `prepare discord --force --max-seconds 600
   --max-invocations 8`; the printed `claudine budget init` ran for real.
   The ledger was then set by hand to what Claudine leaves after a failed
   first step (`stopped`, `runs: 1`, 1 invocation / 42 s used), because a
   real sequence needs an agent. `check-run --through validation` → run
   `failed`.
6. The selected snapshot is unchanged: `generate --check` clean, docs digest
   identical, `recover` → "Nothing to recover".
7. `prepare --resume` → stages from discovery; the only command is
   `claudine sequence` (no new `budget init`); the same ledger keeps
   1/8 invocations and 42/600 s used; `recovery_attempts` 1.

### Gates

- **macOS**, `just test` in `messenger/`: 659 passed, 2 skipped (both
  predate this feature). `just lint`: clean. `cargo clippy -p messenger
  --all-features --all-targets -D warnings`: clean (all chat providers).
  `cargo check -p messenger` (default features): builds without darkmatter,
  biscuit-file, biscuit-hash, or serde_path_to_error (`cargo tree`).
  Outside `src/research/`, the library mentions research only in the
  `#[cfg(feature = "research")]` module declaration. The CLI mentions it
  only in the `research` subcommand. Sends never read research artifacts.
- **macOS**, `just test` in `claudine/` (`--no-fail-fast`): 7319 passed,
  **10 failed**, 9 skipped. All 10 failures are shipped-prompt tests broken
  by `41f9adeb8 chore(prompts): reorganize prompt template layout`
  (2026-09-17, before this phase): `prompts/cross-platform.md` removed,
  `prompts/implement.md` now uses `link(^prompt/…)`, and the
  `shipped_implement_route` fixture drifted. None touches budget, sequence,
  cancellation, or restart code. 664 budget/sequence/cancel/restart/ledger
  tests pass. `just lint` in `claudine/`: clean.
- **Linux** (Docker `rust:1`, arm64): `research_refresh` 16/16,
  `research::refresh` unit tests 6/6, `research_lifecycle_cli` and
  `research_cli` 8/8 and 3/3.
- **Windows**: `cargo check -p messenger-cli --target
  x86_64-pc-windows-gnu` succeeds (compile evidence only).
  `just cross-check messenger --os windows` failed: "No space left on
  device" on `build-win-native`.
- **WSL2**: `just cross-check messenger --os wsl` failed: "Connection closed
  by 192.168.100.64 port 22".
- **build-linux**: `cross-check` still waits on the 2026-09-14 lock held by
  `reward-20260914-c3e60d0`; stopped.
- GitNexus `detect-changes --scope compare --base-ref 488b8b2e9`: 11 files,
  9 symbols, not partial or truncated, risk "critical". Every affected flow
  it lists belongs to other `RunRecord`/`resume` symbols (CI rollup,
  test-audit, terminal rendering). Messenger's `RunRecord` is used only in
  `messenger/lib/src/research/refresh/` and its test. No provider, `Message`,
  `Dispatch`, or `CapabilitySet` code changed.

### Notes

- **Commits made outside this session.** At 00:41, while this phase was in
  progress, `b2fb1d0bb` (the ledger fix) and `b3f69d2ba` (docs and recipes)
  were committed in Ken's name by a process outside this session. This
  session ran no git write command. All code, the recipes, and the README
  and user-guide changes are in those commits. The skill files, plan, log,
  and spec updates are uncommitted.
- The worktree also holds uncommitted Claudine spec work
  (`claudine/fixes/2026-09-17-remove-strict-mode/`) from another session; it
  was not touched here.
- A shell `cd` into `messenger/lib/src/research/…` hung twice for 120 s,
  apparently a directory-change hook on this host. Absolute paths avoided it.

## Implementation of Review Findings #1

> **started at:** 2026-09-18T01:45:22-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/messenger/features/2026-09-17-research-metadata-pipeline/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'The accepted five-platform baseline and its generated publications do not exist' at 01:45:45
        - the live three-pass research run, independent evidence review, and human approval cannot be performed in this non-interactive session (no agent/model choice, no per-platform budgets, no named approver; see Phase 7); that portion is deferred
        - the passive Level 1 corpus assertion recommended by the review is in scope and delegated to a subagent
        - added a passive Level 1 guard in `messenger/lib/tests/research_corpus.rs`:
                - `typed::published_baseline_findings(root)` returns `None` before `messenger/docs/research/publication.json` exists; once it exists, it reports every active roster document that is legacy prose (no `$schema`) or fails accepted-scope validation, plus snapshot drift from the library's own `generate::check`
                - `shipped_publication_is_accepted_and_drift_free` runs the guard on the real tree; today it passes vacuously and prints a `VACUOUS:` notice to stderr rather than skipping silently
                - three fixture-tree tests (built with a real `generate` over the `lifecycle/fleet` fixtures) prove the post-publication branch: a clean fleet passes, legacy prose fails, and a drifted or hand-edited catalog fails
                - `shipped_platform_documents_validate_or_delegate_to_the_fleet` now withdraws the legacy-prose allowance once a publication exists
                - the guard was confirmed to fire against the real tree by temporarily adding a placeholder `publication.json` (removed afterwards)
        - discovered: `research_publication.rs` and `research_refresh.rs` each keep their own `SHIPPED` list and `fleet_text` helper, and the lists already differ (only the refresh list includes `_rules.md`); a shared test helper is a candidate follow-up
        - gates: messenger `just test` 663 passed / 2 skipped; `just lint` clean
        - blocked (unchanged): the live five-platform research, evidence review, human approval, and first publication still require the maintainer's decisions listed in the review's `human_review_items`
- work completed for 'The accepted five-platform baseline and its generated publications do not exist' at 01:50:25 (passive guard only; the live baseline is deferred)
- starting the work on 'An empty approver name satisfies the human-approval gate' at 01:50:25
        - design: new `messenger/lib/src/research/refresh/input.rs` adds validated `Maintainer` and `DecisionReason` newtypes; each constructor trims surrounding whitespace and refuses blank input, and serde loads both through the same constructor (`try_from = "String"`), so the on-disk format is unchanged but a blank or invalid identity cannot be represented or loaded
                - `promote::Request::Human { by: Maintainer }` and `promote::reject(…, &Maintainer, &DecisionReason, …)` make the library boundary the enforcement point, and `Approval.by`, `Decision.by`, and `Decision.reason` use the same types
                - new `InputError { BlankMaintainer, BlankReason, ControlInMaintainer }` reaches callers as `RefreshError::Input`; the CLI exits 2 (`EXIT_USAGE`) with a message, or with a `{"refused": …}` object under `--json`, before anything is written
                - `reject --by` uses the same validation, and `--reason` must not be blank
        - follow-up raised during review of the subagent's work: maintainer names are interpolated into single-line CHANGELOG Markdown, so names containing control characters (newline, tab, CR, ESC, C1) are now rejected to prevent Markdown injection; rejection reasons may still span lines because they are stored only as JSON and never rendered
        - persisted review records and `run.json` files with a blank or multi-line name now fail to load (`ReviewRecord::parse` / `StateError::Corrupt`); none exist yet, since the feature is unreleased
        - tests: `input.rs` unit tests (`blank_inputs_are_refused_and_valid_ones_are_trimmed`, `deserialization_applies_the_same_validation`, `a_maintainer_name_holds_no_control_character_but_a_reason_may_span_lines`); library L1 `decisions_name_a_trimmed_maintainer_and_invalid_persisted_names_do_not_load`; real-CLI L1 `invalid_decision_inputs_exit_2_before_any_decision_or_publication` (asserts `messenger/docs/` and `run.json` are unchanged byte for byte)
        - docs updated: `messenger/docs/user-guide.md`, `messenger/cli/README.md`, and the "Decision inputs" bullet in `.claude/skills/messenger/research-contract.md`
        - discovered: U+2028/U+2029 are not control characters and remain allowed in names; CommonMark does not treat them as line breaks
        - GitNexus: `Decision`, `Approval`, and `refusal` LOW; library `promote`/`reject`/`Request`/`RefreshError` UNKNOWN, with callers confirmed by text search to be only the CLI lifecycle module and `research_refresh.rs`
        - gates: messenger `just test` 668 passed / 2 skipped; `just lint` clean
- work completed for 'An empty approver name satisfies the human-approval gate' at 02:03:25
- starting the work on 'Impossible calendar dates are accepted as research evidence dates' at 02:03:25
        - discovered (correction to the review's premise): platform documents are schema-validated by Darkmatter before the typed model runs, and its `date` type already refused `retrieved: 2026-09-31`; the paths that relied on `Date::parse` alone were `source-checks.json` `checked_on`, run/review/renewal records, `RunId::parse`, and the CLI `--today` (for example, `checked_on: 2026-09-31` passed the Reconcile stage)
        - fix at the single parsing boundary in `messenger/lib/src/research/model/common.rs`: `Date::parse` now checks real month lengths through a new `days_in_month(year, month)` helper using the proleptic-Gregorian leap-year rule; `from_unix_days` and `plus_days` construct dates arithmetically and cannot produce impossible days; no date crate was added
        - errors name the date: serde reports `invalid date \`X\`: expected a real YYYY-MM-DD calendar date`, and the CLI (`messenger/cli/src/research.rs`) exits 2
        - doc drift fixed: the `Date::parse` doc claimed range checks "(not leap years)" and now states the real rule and that serde, `--today`, and run IDs all go through it
        - tests (one shared table: Feb 29 in 2024/2000 valid; Feb 29 in 2026/1900, Feb 30/31, day 31 in Apr/Jun/Sep/Nov, day 0, month 0/13 invalid):
                - `dates_parse_only_real_calendar_days` and `dates_deserialize_only_real_calendar_days` (unit, `common.rs`)
                - `today_accepts_only_real_calendar_days` (real CLI, `research_cli.rs`)
                - `an_impossible_evidence_date_cannot_qualify_an_unchanged_renewal` (`research_refresh.rs`): an impossible check date with an impossible `retrieved` fails Reconcile, and a valid check date with an impossible `retrieved` fails Validation; renewal is refused and nothing is published in both cases; the first case was confirmed to go red with the fix neutered
        - follow-up (not fixed; outside this finding): `refresh_interval_days` has no schema maximum, so an interval of about 2.9 million days or more makes `plus_days` produce a five-digit year, which breaks `Date`'s fixed-width ordering; a schema maximum on the interval is the suggested guard
        - GitNexus: `Date` and `parse` UNKNOWN; callers confirmed by text search (serde deserializer, CLI `parse_date`, `RunId::parse`, `generate.rs`, tests)
        - gates: messenger `just test` 672 passed / 2 skipped; `just lint` clean
- work completed for 'Impossible calendar dates are accepted as research evidence dates' at 02:12:21
- starting the work on 'Corrupt active-run records are ignored when selecting new work' at 02:12:21
        - design: selection fails closed per platform through a new `Skip::UnreadableRun { path, error }` rather than failing all selection, so `prepare --dry-run` still reports every platform and unaffected platforms can still be prepared; `path` is the repository-relative run directory with `/` separators, and an unreadable record outranks a readable open run
                - `StateArea::list` now returns `(PlatformId, path, Result)`, taking the platform from the run's directory rather than the record, so a missing, truncated, or invalid `run.json` always blocks its platform; a record whose `platform_id`/`run_id` disagrees with its directory counts as corrupt
                - human output reads `blocked, <error>; repair or remove <path>`; `prepare --json` also warns on stderr so `just research-refresh` (which counts `.runs`) cannot report "No platform is due"
                - audit of `StateArea::list` consumers: `select` changed (it was the bug); CLI `runs` changed (error rows now carry their platform, so `runs --platform` no longer hides them); `cleanup::plan` unchanged (it already protected unreadable records)
        - concurrency: `prepare` and `prepare --resume` hold `messenger/.research-state/runs/prepare.lock` from selection through run creation, reusing the crate's `PublicationLock` (`File::try_lock`: `flock` on Unix, `LockFileEx` on Windows) widened to `pub(crate)`; the OS releases it when the holder exits, so a crash cannot wedge a platform; a contending caller gets `RefreshError::PrepareBusy { path }` (exit 3)
        - follow-up raised during review of the subagent's work: a failed run could be reopened with `prepare --resume` after a newer run had already been prepared, leaving two open runs; a new shared `select::blocking_runs(state, except)` now drives both selection and resume, and `resume` refuses under the lock with `RefreshError::OtherRunBlocks { run_id, path, blocker }` (exit 1)
        - tests (each confirmed red with its fix removed):
                - library: `an_unreadable_run_record_blocks_its_platform_and_names_its_path`, `concurrent_preparations_leave_exactly_one_open_run` (two threads behind a barrier), `a_held_prepare_lock_refuses_preparation_and_a_released_one_never_lingers`, `resuming_is_refused_while_another_run_for_the_platform_is_open_or_unreadable`
                - real CLI: `a_corrupt_active_run_record_blocks_prepare_and_names_its_path`, `prepare_is_refused_while_another_process_holds_the_prepare_lock` (deterministic: the test holds the real lock file), `resume_is_refused_while_another_run_for_the_platform_is_open`
        - docs updated: `list`, `select`, and `state` module docs; `PublicationLock` doc; `prepare`/`resume` `## Errors`; CLI README; user guide; messenger skill `research-contract.md`
        - GitNexus: `StateArea::list` **HIGH** before editing (3 direct callers, all audited above); `select`, `runs`, `cleanup::plan`, `try_acquire`, `load` LOW; `prepare`, `resume`, `Skip`, `render_selections` UNKNOWN, with callers confirmed by text search to be only the CLI and tests; `detect-changes` reports critical because it links `PublicationLock` to unrelated flows, apparently name matching, since only that type's visibility changed
        - cross-OS: `just cross-check messenger --os windows` failed before building (`No space left on device` on build-win's `W:` drive; nothing was deleted on the shared host); `--os linux` waited on a build-linux lock held since 2026-09-14 by `nightly-reward-spike` (branch `feat-nightly-perf`), left for its owner; `--os wsl` skipped because it shares the full volume; the lock primitive already has a unit test on `windows-latest` in CI, but the new cross-process lock test has no Windows evidence yet
        - discovered: an `active` run cannot be rejected (`WrongStatus`), so freeing a platform held by a prepared-but-unlaunched run requires launching it, failing it with `check-run`, then rejecting it; this may deserve a user-guide note or a spec decision
        - discovered: an external process committed the worktree mid-task (`feef52d55`, `94a94ee6f`, `7b51ee56c`, in Ken's name, not by this session); `feef52d55` captured part of this finding's in-progress changes (`state.rs`, `mod.rs`, CLI, docs) without `select.rs`, `prepare.rs`, `cleanup.rs`, or `fsutil.rs`, so that commit does not compile on its own; the worktree was restored to a complete, green state and the rest is uncommitted
        - gates: messenger `just test` 679 passed / 2 skipped; `just lint` clean
- work completed for 'Corrupt active-run records are ignored when selecting new work' at 02:44:42
- starting the work on 'Prepared runs embed absolute paths and interpolate the repository root into a shell command' at 02:44:42
        - Claudine execution discovered: `shell:` steps run verbatim through `sh -c` (Unix) or `cmd /D /C` with a raw tail (Windows) after template expansion; `tokenize_words_strict` feeds only preflight approval, not execution; there is no argument-vector form and no per-step working directory; a step runs in Claudine's working directory, which a live run switches to the Git root before agent steps
        - design (the root never enters the command): validation steps are now `messenger research check-run <run_id> --through <stage> --root .`, whose tail holds only `[A-Za-z0-9 .-]` and reads the same in `sh`, `cmd`, PowerShell, and Claudine's tokenizer; `yaml_single` and the backslash swap were deleted
        - pass prompts now build every path from `messenger/.research-state/runs/<platform>/<run_id>/…` via a new `run_dir()` and state that paths are relative to the repository root; `Path::display` of the state directory is gone
        - follow-up raised during review of the subagent's work: relative prompts would be wrong for an explicit `--root` below the Git top level (a regression from this fix), so `prepare` and `prepare --resume` now refuse such a root with `RefreshError::NotRepositoryTopLevel { root, top_level }` (exit 2) before writing anything; a root holding its own `.git` entry counts as top level, and otherwise paths are compared after `fs::canonicalize` (handles macOS `/private/var` and Windows `\\?\\` prefixes); non-Git roots stay allowed
                - reuses `sniff::filesystem::git::api::repo_root` (the call `resolve_root` already makes); the library's `research` feature now enables its existing optional `sniff` dependency, and `docs/dependencies.md` was updated
        - tests (each confirmed red with its fix removed):
                - `prepared_runs_name_no_host_path_and_their_checks_resolve_any_root` (real CLI): roots with a space, `'`, `"`, `$HOME`, backticks, `%PATH%`, non-ASCII, a newline, and all combined (`"` and newline are skipped on Windows, which forbids them in file names); asserts no file in the run directory contains the host prefix, asserts the exact step text, and executes the step through `sh -c` / `cmd /D /C` to prove `.` resolves to the original root
                - `prepare_refuses_a_root_below_the_git_top_level_and_accepts_the_top_level_or_no_repository` and `resume_refuses_a_root_that_became_a_subdirectory_of_a_git_work_tree` (real CLI)
        - drift detected and resolved: the Phase 7 claim "No host paths appear in any input" was inaccurate until this change; it now holds and is covered by the new test
        - docs updated: `prepare.rs` docs and `## Errors`, CLI README, user guide, CLI `--help` exit-status text, messenger skill `research-contract.md`, `docs/dependencies.md`
        - discovered: `check-run` inside the sequence always uses the real UTC date because `--today` has no environment fallback
        - Windows: `cargo check -p messenger-cli --tests --target x86_64-pc-windows-gnu` passes (compile evidence only); cross-check rigs were unavailable (see the previous finding), so `windows-latest` CI is the runtime proof
        - GitNexus: `write_sequence`, `write_inputs`, `prepared`, `prepare`, `resume` UNKNOWN (bare-name lookups matched unrelated `biscuit-terminal` symbols); callers confirmed by text search to be private to `prepare.rs`, the CLI, and tests; `refusal` LOW
        - gates: messenger `just test` 682 passed / 2 skipped; `just lint` clean
- work completed for 'Prepared runs embed absolute paths and interpolate the repository root into a shell command' at 03:02:08
- orchestrator verification at 03:02:08: messenger `just test` 682 passed / 2 skipped (81 slow), `just lint` clean for `messenger` and `messenger-cli`

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1h 17m. During this implementation all 5 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 1 was deferred (see reasons below):

- **deferred:** 'The accepted five-platform baseline and its generated publications do not exist'
        - the live three-pass research for Discord, Slack, Telegram, WhatsApp, and Signal, the independent evidence review, and the initial publication all depend on decisions only the maintainer can make: an AI research agent/model, per-platform elapsed-time and invocation limits (the spec sets no defaults), and a named human approver; this session is non-interactive, and inventing those values or hand-writing platform documents would bypass the budget, the three-pass workflow, and the approval gate the spec requires
        - the review's `human_review_items` entry lists exactly what the maintainer must supply
        - the part that did not need a human, the passive Level 1 guard, was implemented: once `messenger/docs/research/publication.json` exists, the corpus suite fails unless every active shipped document is accepted-scope valid and the generated snapshot is drift-free
- this deferral is not a performance-measurement deferral, so `deferred_perf_measurement` is not set
- follow-ups discovered during this cycle (not review findings):
        - `refresh_interval_days` needs a schema maximum to stop `plus_days` from producing five-digit years
        - an `active` run cannot be rejected, so freeing a platform held by a prepared-but-unlaunched run needs a documented path or a spec decision
        - `research_publication.rs` and `research_refresh.rs` keep diverging `SHIPPED` lists that could share a test helper
        - the new cross-process prepare-lock and hostile-root tests have compile evidence for Windows but no Windows runtime evidence yet; build-win's disk is full and build-linux holds a cross-check lock from 2026-09-14 (`nightly-reward-spike`)
- an external process committed part of this cycle mid-run (`7b51ee56c`, `feef52d55`, `94a94ee6f`); `feef52d55` does not compile on its own because it captured part of finding 4; the remaining changes are uncommitted in the worktree

The files changed in this cycle, in addition to the log:

- `messenger/lib/src/research/model/common.rs`
- `messenger/lib/src/research/refresh/{input.rs (new), mod.rs, promote.rs, review.rs, state.rs, select.rs, prepare.rs, cleanup.rs}`
- `messenger/lib/src/research/publish/fsutil.rs`, `messenger/lib/Cargo.toml`
- `messenger/cli/src/{research.rs, research_lifecycle.rs}`
- `messenger/lib/tests/{research_corpus.rs, research_refresh.rs}`, `messenger/cli/tests/{research_cli.rs, research_lifecycle_cli.rs}`
- `messenger/cli/README.md`, `messenger/docs/user-guide.md`, `.claude/skills/messenger/research-contract.md`, `docs/dependencies.md`

## Implementation of Review Findings #2

> **started at:** 2026-09-18T09:01:12-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/messenger/features/2026-09-17-research-metadata-pipeline/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'The accepted five-platform baseline and its generated publications still do not exist' at 09:01:30
        - the blockers recorded in review 1 and review 2 are unchanged: no AI research agent/model, no per-platform elapsed-time and launch limits, and no named human approver were supplied with this iteration's prompt, and this session is non-interactive
        - running the three research passes with invented budgets, or hand-writing platform documents and an approval record, would bypass the budget, three-pass, and approval gates the spec requires, so no attempt was made
        - the review marks this finding as requiring a human decision (`human_review: true`); its `human_review_items` entry lists what the maintainer must supply
        - deferred
- work completed for 'The accepted five-platform baseline and its generated publications still do not exist' at 09:01:30
- starting the work on 'Unbounded refresh intervals can produce invalid fixed-width dates' at 09:01:30
        - the spec states no interval semantics beyond "start with a configurable 30-day refresh interval", so the bound is a new, chosen ceiling: `MAX_REFRESH_INTERVAL_DAYS = 3660` (ten 366-day years) in `model/roster.rs`; a decade is far beyond any useful research cadence, and it keeps every `last_updated` through 9989-12-23 representable (9989-12-23 + 3660 = 9999-12-31; the review's "~9989" estimate was eight days generous, caught by a unit test)
        - `Date::plus_days` is replaced by `Date::checked_plus_days(days) -> Option<Date>`, `None` when the result passes 9999-12-31, so an out-of-range `Date` can no longer be constructed; `Date::from_unix_days` is unchanged (it takes the host clock, not authored input)
        - schema: `refresh_interval_days` gains `max(3660)` in `docs/platforms.schema.yaml` (roster default) and in `_types.yaml` `RosterPlatform` (per-platform override)
                - the `_types.yaml` edit changes the document-contract fingerprint (`xxh64:dcdb3d338874ed6d` → `xxh64:d766403489d161d9`); the two fixtures pinning it (`contract/overrides-valid.yaml`, `negative/semantic/sr-override--expired-override.yaml`) were updated to the value computed by `canonical::schema_fingerprint`; there is no published baseline yet, so no accepted document is forced to `schema_changed`, and there is no `just research-*` recipe that regenerates fixtures (none of the recipes produce shipped generated artifacts today)
        - semantic boundary uses the existing SR-ROSTER rule, no new rule ID: `check_roster` rejects a default or per-platform interval outside `1..=3660` (reachable only for a roster built in code, since the schema rejects it first), and `roster_coverage` reports `/last_updated` when `last_updated` plus the effective interval passes 9999-12-31 (covers late dates the interval ceiling alone cannot protect)
        - callers: `refresh::select` skips the `Expired`/`Current` computation when the date overflows (the same validation pass already marks the platform `schema_invalid`); `project::project` now returns `Result<Catalog, Vec<Diagnostic>>` and emits the same SR-ROSTER diagnostic instead of a five-digit year, and `Fleet::snapshot` maps it to `GenerateError::Refused` (unreachable from a clean fleet, but not a panic)
        - GitNexus impact: `plus_days` UNKNOWN (no resolved callers); grep confirmed the only callers were `select.rs` and `project.rs` plus unit tests; `check_roster` and `roster_coverage` report CRITICAL, but the processes listed (`tools/test-audit`, `homelab`) are index noise, and grep shows every real consumer inside the messenger `research` feature; `project_platform` resolved to a wrong symbol id (stale index); no crate outside `messenger/` uses `research::project`, `plus_days`, or the new constant; `detect-changes --scope all` = medium, and its two affected flows belong to other in-progress working-tree changes, not this one
        - tests added: `project::tests::checked_plus_days_stops_at_the_last_four_digit_year`, `project::tests::the_largest_interval_fits_every_last_updated_through_9989_12_23` (the old `plus_days_crosses_months_years_and_leap_days` was renamed `checked_plus_days_…`), `research_validation::refresh_intervals_past_the_ceiling_are_roster_findings`, `research_validation::a_refresh_date_past_9999_is_a_finding_not_a_catalog_date` (accepted 9989-12-23 projects `refresh_due: 9999-12-31`; 9989-12-24 is an SR-ROSTER finding and `project` refuses it), `research_refresh::a_refresh_date_past_9999_makes_the_document_invalid_not_expired`
        - fixtures added: `contract/roster-refresh-interval-max.yaml` (3660 accepted, schema and SR-ROSTER), `negative/schema/roster-refresh-interval-above-max.yaml` and `negative/schema/roster-platform-refresh-interval-above-max.yaml` (3661 rejected), `negative/semantic/sr-roster--refresh-due-after-9999.md`; `positive_roster_overrides_and_mappings_fixtures_are_clean` now also checks the new contract roster
        - red proof (each neutered in turn, then restored with a plain write for a fresh mtime): no 9999 guard in `checked_plus_days` → 5 tests red; no interval range check → the typed-interval test red; no `/last_updated` rule → the catalog, selection, and `identity_rules_reject_their_fixtures` tests red; no schema `max(3660)` → `schema_negative_fixtures_fail_at_their_declared_problem` red
        - docs updated: `_rules.md` SR-ROSTER row, the `platforms.schema.yaml` header and field comments, `lib/tests/fixtures/research/README.md`, `.claude/skills/messenger/research-contract.md`, and the `Date::parse`, `project`, and `Fleet::snapshot` docs; the messenger README, CLI README, and user guide never describe `refresh_interval_days` or its range, so they are unchanged
        - portability: pure date arithmetic and YAML, with no OS-specific code
        - gates (from `messenger/`): `just lint` passed (clippy + fmt, `desktop,research` features); `just test` passed, with 687 tests run, 687 passed, and 2 skipped
- work completed for 'Unbounded refresh intervals can produce invalid fixed-width dates' at 09:15:51
- orchestrator verification at 09:15:51: messenger `just test` 687 passed / 2 skipped, `just lint` clean for `messenger` and `messenger-cli`; the change is pure date arithmetic and schema text, so no cross-OS rig run was needed

### Successful Completion

The implementation of review cycle 2 has completed successfully in 15m. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 was fixed, 1 was deferred (see reasons below):

- **deferred:** 'The accepted five-platform baseline and its generated publications still do not exist'
        - the three research passes, the independent evidence review, and the initial publication require decisions only the maintainer can make: an AI research agent/model, per-platform elapsed-time and launch limits (the spec sets no defaults), and a named human approver
        - none of these was supplied for this iteration, and this session is non-interactive; inventing them or hand-writing platform documents would bypass the budget, three-pass, and approval gates the spec requires
        - the review's `human_review_items` entry lists exactly what the maintainer must supply
- this deferral is not a performance-measurement deferral, so `deferred_perf_measurement` is not set

The files changed in this cycle, in addition to the log and review file:

- `messenger/lib/src/research/{model/common.rs, model/roster.rs, project.rs, generate.rs, refresh/select.rs, validate/identity.rs, validate/mod.rs}`
- `messenger/docs/platforms.schema.yaml`, `messenger/docs/research/platforms/{_types.yaml, _rules.md}`
- `messenger/lib/tests/{research_validation.rs, research_refresh.rs, research_corpus.rs}`
- `messenger/lib/tests/fixtures/research/{README.md, contract/overrides-valid.yaml, contract/roster-refresh-interval-max.yaml (new), negative/schema/roster-refresh-interval-above-max.yaml (new), negative/schema/roster-platform-refresh-interval-above-max.yaml (new), negative/semantic/sr-roster--refresh-due-after-9999.md (new), negative/semantic/sr-override--expired-override.yaml}`
- `.claude/skills/messenger/research-contract.md`
