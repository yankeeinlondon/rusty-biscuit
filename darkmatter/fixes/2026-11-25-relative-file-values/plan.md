---
total-phases: 6
created: 2026-11-25
status: planned
phase: 1
agent: claude/opus
yolo: true
source_files_during_phase_1:
  - biscuit-file/cli/src/main.rs
  - biscuit-file/cli/tests/cli_tests.rs
  - biscuit-file/justfile
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/portable/diagnostics.rs
  - biscuit-file/lib/src/file_reference/portable/evaluate.rs
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/shadow.rs
  - biscuit-file/lib/src/file_reference/portable/strategy.rs
  - biscuit-file/lib/src/file_reference/portable/text/tests.rs
  - biscuit-file/lib/src/file_reference/portable/text.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/src/path_text.rs
  - biscuit-file/lib/tests/l1/file_tree.rs
  - biscuit-file/lib/tests/l1/glob_reference/boundary.rs
  - biscuit-file/lib/tests/l1/glob_reference/directory_case.rs
  - biscuit-file/lib/tests/l1/glob_reference/grammar.rs
  - biscuit-file/lib/tests/l1/glob_reference/literal.rs
  - biscuit-file/lib/tests/l1/glob_reference/unreadable.rs
  - biscuit-file/lib/tests/l1/launch_lookup.rs
  - biscuit-file/lib/tests/l1/magic_local_roots.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/tests/l1/portable_path/mod.rs
  - biscuit-file/lib/tests/l1/portable_path/originating_context.rs
  - biscuit-file/lib/tests/l1/portable_path/platform.rs
  - biscuit-file/lib/tests/l1/value_spelling.rs
  - claudine/cli/src/cli_utils.rs
  - claudine/cli/src/commands/agents.rs
  - claudine/cli/src/commands/compose/prep/tests.rs
  - claudine/cli/src/commands/config_tui/mod.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init/prompts.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/cli/src/commands/link_display.rs
  - claudine/cli/src/commands/mcp/init.rs
  - claudine/cli/src/commands/prep_target.rs
  - claudine/cli/src/commands/schema_interactive/mod.rs
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/setup.rs
  - claudine/cli/src/commands/signals.rs
  - claudine/cli/src/commands/skills.rs
  - claudine/cli/src/commands/slash_commands.rs
  - claudine/cli/src/commands/uninstall.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/target.rs
  - claudine/cli/src/commands/wrap/composition/timeouts.rs
  - claudine/cli/src/commands/wrap/env/package_context.rs
  - claudine/cli/src/commands/wrap/exec/watchdog/spawn.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/provider_extension_and_opencode.rs
  - claudine/cli/src/commands/wrap/overlay.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
  - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
  - claudine/cli/src/commands/wrap/session_report.rs
  - claudine/cli/src/commands/wrap/system_prompt.rs
  - claudine/cli/src/completion/autocomplete_ui.rs
  - claudine/cli/src/completion/composition/compose.rs
  - claudine/cli/src/completion/composition/setter_value.rs
  - claudine/cli/src/completion/operation_file.rs
  - claudine/cli/src/completion/schema_completion/candidates.rs
  - claudine/cli/src/completion/schema_completion/parity_tests.rs
  - claudine/cli/src/completion/schema_completion/tests.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/src/completion/setter_value.rs
  - claudine/cli/src/output/mod.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/tests/l1/compose_caller_file_provenance.rs
  - claudine/cli/tests/l1/compose_schema_cli.rs
  - claudine/cli/tests/l1/contamination_probes.rs
  - claudine/cli/tests/l1/ctx_launch_anchor.rs
  - claudine/cli/tests/l1/entry_point_parity.rs
  - claudine/cli/tests/l1/home_lookup_round_trip.rs
  - claudine/cli/tests/l1/inline_compose_cli.rs
  - claudine/cli/tests/l1/invalid_trigger_warnings.rs
  - claudine/cli/tests/l1/level1_provider_overlay_home.rs
  - claudine/cli/tests/l1/override_boundary_guard.rs
  - claudine/cli/tests/l1/setup_command.rs
  - claudine/cli/tests/l1/switch_catalog_guard.rs
  - claudine/cli/tests/level2/level2_file_resolution_capture.rs
  - claudine/cli/tests/prompts/pr_flow_rehearsal.rs
  - claudine/cli/tests/prompts/review_fix_rehearsal.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/main.rs
  - claudine/gen/src/report.rs
  - claudine/lib/src/composition/coordinator/commit.rs
  - claudine/lib/src/composition/coordinator/invocation.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/lifecycle.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/provider.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/file_detail.rs
  - claudine/lib/src/composition/guardrails.rs
  - claudine/lib/src/composition/launch_workspace.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - claudine/lib/src/composition/lifecycle/prep.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/resolve/tests.rs
  - claudine/lib/src/composition/resolve.rs
  - claudine/lib/src/composition/runtime_state.rs
  - claudine/lib/src/composition/schema/status_render.rs
  - claudine/lib/src/composition/schema/tests.rs
  - claudine/lib/src/composition/sequence/data.rs
  - claudine/lib/src/composition/sequence/declarations.rs
  - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
  - claudine/lib/src/composition/sequence/preflight/mod.rs
  - claudine/lib/src/composition/sequence/task/tests.rs
  - claudine/lib/src/error.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/resolve/tests.rs
  - claudine/lib/src/invocation_context/document_tests.rs
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/linking/compatibility/classify.rs
  - claudine/lib/src/linking/mod.rs
  - claudine/lib/src/linking/symlink.rs
  - claudine/lib/src/mcp/export.rs
  - claudine/lib/src/mcp/import/tests.rs
  - claudine/lib/src/mcp/import.rs
  - claudine/lib/src/mcp/validation.rs
  - claudine/lib/src/permissions/query.rs
  - claudine/lib/src/render/prompt/system/tests.rs
  - claudine/lib/src/render/prompt/system.rs
  - claudine/lib/src/stream/path_link.rs
  - claudine/lib/src/system_prompt/prepare/tests.rs
  - claudine/lib/tests/l1/installed_registration.rs
  - claudine/lib/tests/l1/invalid_trigger_findings.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/output/mod.rs
  - darkmatter/cli/src/render_style.rs
  - darkmatter/cli/tests/l1/compose_setters.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/cli/tests/l1/dynamic_schema_slots.rs
  - darkmatter/cli/tests/l1/entry_point_parity.rs
  - darkmatter/cli/tests/l1/load_data.rs
  - darkmatter/cli/tests/l1/output_envelope.rs
  - darkmatter/cli/tests/l1/render_style_files.rs
  - darkmatter/cli/tests/l1/render_style_wiring.rs
  - darkmatter/dmls/src/workspace/discover.rs
  - darkmatter/dmls/tests/l1/schema_slots_editor.rs
  - darkmatter/dmls/tests/l1/value_hovers.rs
  - darkmatter/dmls/tests/l1/workspace_file_list.rs
  - darkmatter/lib/src/config/migration_receipt.rs
  - darkmatter/lib/src/config/write.rs
  - darkmatter/lib/src/lifecycle/error.rs
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/src/lifecycle/preflight.rs
  - darkmatter/lib/src/markdown/compose/context/capture/invocation.rs
  - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture/repo.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/context/request.rs
  - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/expression/read_gate.rs
  - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/schema.rs
  - darkmatter/lib/src/markdown/reference/report.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/rewrite.rs
  - darkmatter/lib/src/markdown/schemas/schema_files.rs
  - darkmatter/lib/src/markdown/schemas/slots/approval_tests.rs
  - darkmatter/lib/src/markdown/schemas/slots/cache.rs
  - darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
  - darkmatter/lib/src/markdown/schemas/slots/session.rs
  - darkmatter/lib/src/markdown/schemas/slots/session_tests.rs
  - darkmatter/lib/src/markdown/schemas/slots/tests.rs
  - darkmatter/lib/tests/common/entry_point_parity/mod.rs
  - darkmatter/lib/tests/l1/compose_read_set.rs
  - darkmatter/lib/tests/l1/compose_setters.rs
  - darkmatter/lib/tests/l1/config_init.rs
  - darkmatter/lib/tests/l1/config_migration_receipt.rs
  - darkmatter/lib/tests/l1/context_functions.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_precedence.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_read_order.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_template_triggers.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slots.rs
  - darkmatter/lib/tests/l1/editor_observation.rs
  - darkmatter/lib/tests/l1/entry_point_parity.rs
  - darkmatter/lib/tests/l1/expression_recovery_data.rs
  - darkmatter/lib/tests/l1/expression_recovery_slots.rs
  - darkmatter/lib/tests/l1/file_predicates.rs
  - darkmatter/lib/tests/l1/file_tree_roots.rs
  - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
  - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
  - darkmatter/lib/tests/l1/glob_consumers.rs
  - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
  - darkmatter/lib/tests/l1/lifecycle_edit_action.rs
  - darkmatter/lib/tests/l1/link_interpolation_integration.rs
  - darkmatter/lib/tests/l1/load_data_destination.rs
  - darkmatter/lib/tests/l1/load_data_diagnostics.rs
  - darkmatter/lib/tests/l1/load_data_execution.rs
  - darkmatter/lib/tests/l1/load_data_options.rs
  - darkmatter/lib/tests/l1/load_data_options_slots.rs
  - darkmatter/lib/tests/l1/load_data_runtime.rs
  - darkmatter/lib/tests/l1/load_data_uncontracted.rs
  - darkmatter/lib/tests/l1/match_keyword_input_matrix.rs
  - darkmatter/lib/tests/l1/nested_composition.rs
  - darkmatter/lib/tests/l1/path_function_contracts.rs
  - darkmatter/lib/tests/l1/recursive_schema_rewrite.rs
  - darkmatter/lib/tests/l1/reference_report.rs
  - darkmatter/lib/tests/l1/request_context_builder.rs
  - darkmatter/lib/tests/l1/required_context.rs
  - darkmatter/lib/tests/l1/schema_roots.rs
  - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
  - darkmatter/lib/tests/l1/slot_permissions.rs
  - playa/cli/src/main.rs
  - simplified-schema/lib/src/file_match.rs
  - simplified-schema/lib/src/format.rs
  - simplified-schema/lib/src/resolve/bundle.rs
  - simplified-schema/lib/tests/l1/bundle_resolution.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/tests/l1/cli.rs
  - sniff/cli/tests/l1/snapshots.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/level2_god_files.rs
  - tree-hugger/cli/tests/symbols/grouping_acceptance.rs
  - tree-hugger/cli/tests/symbols/grouping_reports.rs
  - tree-hugger/lib/src/file/naming.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
docs_updated_during_phase_1:
  - biscuit-file/README.md
  - biscuit-file/cli/README.md
  - biscuit-file/docs/dependencies.md
  - biscuit-file/docs/topics/file-references.md
  - darkmatter/docs/topics/magic-paths.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
  - .claude/skills/biscuit-file/references/cli.md
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows-paths.md
packages:
  - biscuit-file
  - biscuit-file-cli
  - claudine
  - claudine-cli
  - claudine-gen
  - darkmatter
  - darkmatter-cli
  - darkmatter-lsp
  - playa-cli
  - simplified-schema
  - sniff
  - sniff-detection-cli
  - tree-hugger
  - tree-hugger-cli
  - worktree
---

# Plan: File values are relative by default

Source: [spec](./spec.md) (`2026-11-25-relative-file-values`).

## Summary and Definition of Done

Darkmatter binds a `file` value to a native absolute path. Today:

- the frontmatter, expressions, lifecycle state, and returned composition state
  all see `/home/me/repo/feat/x/spec.md`
- path functions (`dirname`, `join`, the index family, `find_files`) carry that
  path into generated text
- implicit relative references (`feat/x.md`) resolve beside the authoring
  document, then at the repository root

This fix changes three things across biscuit-file, simplified-schema,
Darkmatter (`md`, library, DMLS), and Claudine:

1. **Launch lookup.** An implicit relative path in composition, a command file
   argument, or completion is joined to the launch bases, in this order:
   1. the launch directory
   2. the package root
   3. the package area root
   4. the repository root

   This is an explicit opt-in in biscuit-file's resolver. Schema imports,
   configuration, permissions, link destinations, and other consumers keep
   their current contracts.
2. **One portable spelling.** Every bound `file` value (caller, authored,
   default, nested) and every path-function result is spelled the same way:
   - preferred: an implicit path from the launch repository root, guarded
     against shadows by one cached first-component check per nearer base
   - fallback: a declared `{{VAR}}` root, then `~/`, then a faithful absolute
     path

   The selected target and authored origin travel beside the value, never
   inside it, including through children, sequences, and lifecycle handoff.
3. **Renames and migration.** `absolute`/`relative` become
   `absolute_path`/`relative_path`, and the old names are removed. Each
   routine that strips a repository root to make display text is replaced by
   the shared spelling or kept with a recorded reason. Live documents,
   prompts, fixtures, docs, and skills are migrated.

**Done means all of the following hold, with evidence in the implementation log:**

- With the prompt launched from the repository root and
  `spec=feat/x/spec.md`, the spec's table holds exactly through `md compose`,
  `claudine compose`, and a `claudine` sequence task:
  - `{{ spec }}` is `feat/x/spec.md`
  - `{{ dirname(spec) }}` is `feat/x`
  - the index result is `feat/x/review-2.md`
  - `contains(spec, 'repo')` is `false`
  - `absolute_path(spec)` is native absolute
- Every bullet in the spec's **Verification** section has a named test, listed
  in the implementation log's verification map (Phase 6), and passes on Linux.
  Tests gated to Windows or Unix compile on every OS.
- `just test` and `just lint` pass in `biscuit-file/`, `simplified-schema/`,
  `darkmatter/`, and `claudine/`. `just test-l2` passes in `darkmatter/` and
  `claudine/` for the completion and chooser capture tests touched.
- `absolute(` and `relative(` no longer exist as expression functions anywhere:
  - not in the runtime, catalog, shipped artifact, docs, skills, prompts, or
    fixtures
  - historical feature and fix specs are untouched
  - calling either old name yields `dm.expression.unknown_function` in
    composition and DMLS
- `biscuit_file::to_portable_string`/`try_portable_string` no longer exist,
  renamed per Rule R2. `LaunchMagicScope` is renamed per Rule R3.
- The docs are updated and the caller skills are republished:
  - `biscuit-file/docs/topics/file-references.md` is the single description of
    the reference forms, the bases, the portable path, and the terms
  - Darkmatter's and Claudine's file-reference topics state only what they add
  - `use-darkmatter`, `use-dmls`, and `use-claudine` are republished with
    `just publish-skills`
- The implementation log ends with "implementation complete, ready for review".
  It lists two things for the review:
  - the passages edited in Claudine's design page (Rule R27)
  - the interpretation recorded in Rule R13

## Phase 1 — biscuit-file foundation

This phase changes only biscuit-file plus the mechanical renames that touch
every crate. Everything later builds on its API.

### Necessary Rules

These rules settle every open choice. An implementer follows them as written.

**Resolution (biscuit-file)**

- **R1: Launch lookup is chosen per context, defaults to authoring lookup, and
  never leaks into non-composition consumers.**
  - Add `pub enum ImplicitBases { Authoring, Launch }` in
    `biscuit-file/lib/src/file_reference/context.rs`, with `Authoring` as the
    default.
  - Add `FileResolutionContext::with_implicit_bases(ImplicitBases) -> Self`
    and `implicit_bases() -> ImplicitBases`.
  - The field is carried unchanged by every derivation: `for_source`,
    `for_cwd`, `for_trusted_external_*`, and `derive`.
  - Evidence: `simplified_schema::file_context::document_file_context`
    (`simplified-schema/lib/src/file_context.rs:62`) derives schema contexts
    from the request context with `for_cwd`. A flag that did not travel with
    derivations would split one composition into two rules, and one that
    travels must be reset explicitly by non-composition consumers (Rule R19).
  - Per-call variants of the dozen public resolve entry points
    (`FileReference::resolve_in_context`, `resolve_detailed*`,
    `candidate_plan*`, `complete_partial_in_context`,
    `resolve_target_in_context`, and the `GlobReference` listing methods in
    `biscuit-file/lib/src/file_reference/mod.rs` and `glob/mod.rs`) were
    rejected. They would double the API surface for one switch.
- **R2: Rename the separator helpers.**
  - `biscuit_file::to_portable_string` becomes `to_forward_slash_string`.
  - `biscuit_file::try_portable_string` becomes `try_forward_slash_string`.
  - The definitions are in `biscuit-file/lib/src/path_text.rs`. Update their
    doc comments to say they only rewrite separators.
  - The rename is mechanical across all ~227 Rust files that use them
    (`rg -l to_portable_string -t rust`), including sniff, tree-hugger,
    worktree, claudine, and darkmatter.
  - Historical specs under `*/features/*` and `*/fixes/*` are not edited.
- **R3: Rename `LaunchMagicScope` to `LaunchScope`.**
  - Rename the accessor and builder too: `launch_magic_scope()` becomes
    `launch_scope()` and `with_launch_magic_scope` becomes
    `with_launch_scope`. 73 occurrences, listed by
    `rg "LaunchMagicScope|launch_magic_scope"`.
  - Add `LaunchScope::bases(&self) -> Vec<PathBuf>`. It returns
    `[request_dir, package_root, package_area, repository_root]` in that
    order:
    - absent anchors are skipped
    - duplicates are removed by `PathIdentity`, keeping the first occurrence
    - with no `repository_root`, it returns `[request_dir]` alone
  - Evidence: the struct already captures exactly these four anchors
    (`context.rs:149-197`), and the spec's ordered bases are exactly these.
    The name "magic" stops being true once implicit lookup uses it.
- **R4: Where the launch-mode candidates come from.**
  - Under `ImplicitBases::Launch`, `implicit_relative_roots`
    (`resolve.rs:943`) returns `LaunchScope::bases()` and ignores the
    authoring `cwd`. Change its signature to take the
    `ResolutionContext`/mode.
  - Callers that inherit the change:
    - direct implicit resolution, through `build_anchoring_candidates`
      (`resolve.rs:1032`)
    - `%` searches and `GlobReference` roots, through
      `collect_anchoring_roots` and `reference_roots`
  - Completion's `completion_roots` (the `ImplicitRelative` arm, fed by
    `CompletionAnchors` in `complete_partial_in_context`, `resolve.rs:1591`)
    reads the same launch bases in launch mode.
  - Explicit `./`/`../`, `&`, `^`, `@`, `~`, `{{VAR}}` (after expansion, it
    is reclassified by the shared grammar as today), `vault:`, URL, and
    absolute forms are unchanged.
- **R5: Launch-relative lookup checks the launch tree's boundary.**
  - In launch mode, the lexical and symlink containment checks for
    implicit-relative candidates use the context's frozen `request_tree`
    (`context.rs` field `request_tree`) instead of the source `tree`.
  - Explicit relative forms keep the source tree.
  - A miss advances to the next base. A containment, permission, or I/O error
    keeps the resolver's existing stopping rule; `probe_candidate` is
    unchanged.
- **R6: Add a directory probe.**
  - Add `pub enum EntryKind { File, Directory }`.
  - Add
    `FileReference::resolve_detailed_as(&self, ctx, EntryKind) -> DetailedResolution`.
    `Directory` probes with `metadata().is_dir()` and returns a match only for
    a directory.
  - `resolve_detailed` stays `File`.
  - Evidence: `probe_candidate` (`resolve.rs:336`) uses `is_file()`, so a
    directory operand never matches and today always falls back to the first
    candidate. The spec requires a directory at a later base to be found.
- **R7: Plan a target that does not exist yet (ruling "A missing path is
  planned where its directory exists").**
  - Add `FileReference::plan_target(&self, ctx, EntryKind) -> Result<PlannedTarget, FileReferenceError>`.
    `PlannedTarget { path: PathBuf, provenance: RootProvenance, existed: bool }`.
  - The outcomes are, in order:
    1. the first base where the entry exists
    2. otherwise, for a launch-mode implicit reference, the first base where
       the parent directory exists
    3. otherwise, the launch directory
  - Non-implicit forms return their existing first candidate.
  - A recursive (`%`) reference returns its existing typed refusal. It has no
    single planned file.
  - The cost is one directory check per base, only on a miss.

**Spelling (biscuit-file)**

- **R8: Add the value spelling preference and strategy.**
  - Add `PortabilityPreference::LaunchRelative` in
    `portable/strategy.rs`, with `Display` `"LaunchRelative"`.
  - Add
    `PortabilityPreference::VALUE_STRATEGY = &[LaunchRelative, EnvRootedPath, HomeDir, AbsolutePath]`.
  - `DEFAULT_STRATEGY` is unchanged, so link normalization and other callers
    keep their default.
  - `LaunchRelative` writes the target's names below the launch repository
    root as implicit text (`Lead::Bare`, `portable/text.rs`).
  - It is `NotApplicable` when any of these holds:
    - the context has no launch repository, because the run started outside
      every repository
    - the target is outside the launch repository
    - the target equals the launch repository root
    - the target lies under a foreign checkout (Rule R10)
- **R9: How `LaunchRelative` is verified, using a shadow cache (ruling "One
  cached check before a short path is printed").**
  - Add `pub struct ShadowCache`. It is shareable (`Clone` over `Arc`) and
    holds a memo keyed by `(base, first component)` plus a probe count.
  - Add `pub trait ShadowProbe: Send + Sync { fn exists(&self, path: &Path) -> Result<bool, ProbeError>; }`
    and a default `FsShadowProbe` that uses `symlink_metadata`.
  - `ShadowCache::new(Arc<dyn ShadowProbe>)` and
    `ShadowCache::probe_count() -> usize`.
  - `PortablePath::with_shadow_cache(&ShadowCache)`. Without one, evaluation
    builds a private `FsShadowProbe` cache.
  - The nearer bases are the launch bases before the repository root.
  - For each nearer base, check `base/first_component` through the cache.
    Absent under every nearer base means accepted.
  - Where it is present, check the full path under that base:
    - for an existing target: `base/rest` exists
    - for a missing target: `base/parent(rest)` is a directory
  - Either hit is a shadow: the preference does not match and evaluation
    falls through to the fallbacks.
  - A probe error, including a denial reported by a `ShadowProbe`, makes the
    preference not match. It is recorded as an `Attempt` with
    `ProbeFailed`-style data, but it does **not** end evaluation, because the
    fallback spellings need no probe. Evidence: the spec's ruling requires
    the checks to respect read approvals; a check that cannot be made cannot
    prove the short path is safe.
  - The existing resolve-back verification is not run for `LaunchRelative`;
    the shadow rule is its verification. It still requires the target itself
    to exist, or to be planned (R7) when it is missing.
- **R10: Foreign checkouts are supplied by the caller.**
  - Add `PortablePath::with_foreign_checkouts(impl IntoIterator<Item = PathBuf>)`.
  - A target at or below one of those roots is `NotApplicable` for
    `LaunchRelative`. biscuit-file never discovers checkouts during spelling.
  - Darkmatter supplies the roots from `SourceRepositories` (Rule R18).
- **R11: Fall back to a native spelling when forward slashes would change the
  names.**
  - `render_reference` and `render_absolute` (`portable/text.rs`) first try
    the forward-slash text.
  - When it changes the names (`ChangesComponents`), they try the native text
    (`joined.to_str()` / `path.to_str()`) through the same re-parse and
    `check_grammar` checks, and return it when it reads back to the same
    names.
  - Evidence: on Unix `/tmp/a\b.md` is one name, and `try_portable_string`
    turns it into `/tmp/a/b.md`, so today `AbsolutePath` fails with
    `NoStrategyMatched`.
  - A non-Unicode name still yields `PortablePathError::UnrenderableTarget`.
    The Darkmatter error for it is Rule R23.

**Expression functions (Darkmatter)**

- **R12: Path function results.** Each of these spells its result with the
  value strategy (R8), even when its input was absolute:
  - `dirname`, `join`
  - `find_files`, `find_first_index`, `find_last_index`
  - `increment_file_index`, `decrement_file_index`
  - one-argument `link`/`code_link` labels

  Specifics:
  - `dirname` resolves the input file and spells its parent directory.
  - `join(dir, name)` resolves its first argument as a directory (R6), joins
    `name` to the selected directory, and spells the result independently
    (planned with R7 when it is missing).
  - The component functions keep their current semantics: `basename`,
    `parent_dir`, `ext`, `file_trailing`, `dir_leading`.
  - The `render_built_path` absolute-input branch (`fs.rs:431`) and the
    `dirname` absolute branch (`fs.rs:747`) are deleted.
  - Evidence: `darkmatter/lib/src/markdown/compose/expression/functions/fs.rs`
    and `path_projection.rs:142` `make_portable_relative_in_context`, which
    every one of these calls today.
- **R13: `absolute_path` on a missing file.**
  - For a missing non-recursive local file it returns the native absolute
    path of the R7 planned target.
  - The spec's "existing first-candidate behavior" is read as "returns a path
    without proving existence". Returning the literal first base instead
    would make `absolute_path(x)` and a lifecycle write of `x` name different
    files, which the spec forbids: "Preflight and execution must agree about
    the target".
  - A recursive miss and a URL keep their typed errors.
  - Record this interpretation in the implementation log for review.
- **R14: The old names are removed with no alias.**
  - In `functions/paths.rs`, the `absolute`/`relative` bindings become
    `absolute_path`/`relative_path`. The catalog keys in
    `darkmatter/schemas/partials/functions.yaml` (lines 595 and 844) are
    renamed in the same change.
  - New squashed aliases (`absolutepath`, `relativepath`) are added, because
    every other multi-word function has one (`functions/mod.rs` aliases such
    as `findfiles`). The retired names get no alias.
  - `dm.expression.unknown_function` for `absolute(`/`relative(` then comes
    for free: from `unknown_function_error` (`expression/mod.rs:950`) in
    composition, and from `unknown_function_calls`
    (`dmls/src/overlay/expressions.rs:1503`) in DMLS.
  - Regenerate the shipped artifacts with `just shipped-schemas` from the
    repository root, and the function docs with `just regen-expr-doc` in
    `darkmatter/`.
- **R15: Both arguments accept a directory.** `relative_path` and
  `absolute_path` accept a directory, such as a `dirname` result, by resolving
  with `EntryKind::Directory` when the file probe misses. Both keep their
  null behavior.
- **R16: Links.**
  - A one-argument `link(x)`/`code_link(x)` uses the R8 spelling as its label,
    with the existing `escape_link_text` or `code_span` formatting.
  - The destination stays `portable_destination` (`fs.rs:914`), which is
    absolute. Root-only `normalize_links` (`link_normalization.rs:128`, with
    `DEFAULT_STRATEGY`) still rewrites it, for example to `./plan.md`.
  - A two-argument call is unchanged.

**Darkmatter wiring**

- **R17: Every Darkmatter composition context opts in at one site.**
  - `ComposeRequest::prepare`/`with_context`
    (`darkmatter/lib/src/markdown/compose/context/request.rs`) set
    `ImplicitBases::Launch`.
  - `md` file arguments opened through `cli/src/io::open_argument` also use
    `Launch`.
  - Claudine's `build_file_resolution_context`
    (`claudine/lib/src/invocation_context.rs:2521`) sets `Launch`.
  - Evidence: these are the request builders named in the darkmatter skill
    ("Every entry point that resolves file references takes a
    `ComposeRequest`") and in Claudine's survey (`resolve.rs:66`/`:93`/`:133`
    all derive from it).
- **R18: A request-scoped speller.**
  - Add `darkmatter::markdown::compose::file_spelling::FileSpeller`, held by
    the `ComposeRequest` and shared by `Arc` with every derived child request.
    It owns:
    - one `ShadowCache`
    - a `ShadowProbe` implementation that passes each check through the
      existing read gate (`expression/read_gate.rs`), records it in the
      read set, and counts it as a new `work_counters` entry `Work::ShadowProbe`
    - the foreign checkout roots from `SourceRepositories`
  - It exposes:
    - `spell(target: &Path, entry: EntryKind) -> Result<String, MarkdownError>`
    - `spell_native(target) -> String`
  - `path_projection.rs`'s `project_in_context`,
    `make_portable_relative_in_context`, and `strip_prefix_any_spelling` are
    deleted. Every caller moves to `FileSpeller::spell`:
    - `fs.rs`
    - `schemas/rewrite.rs:628`
    - the caller projection in `schema_validation.rs`
  - Passive surfaces never construct a speller. Those are `md schema
    validate`, DMLS analysis, the reference check
    (`ComposeOptions::for_reference_check`), and the preflight walk.
- **R19: Consumers pinned to authoring lookup.** Each site below derives or
  receives a composition context and must call
  `.with_implicit_bases(ImplicitBases::Authoring)` before resolving:
  - schema imports, `$schema` selection, and schema-root selection:
    `simplified-schema/lib/src/resolve.rs` `resolve_reference_in_context`
    (:1374), `resolve/select.rs` `select_schema_file`/`select_schema_file_checked`
    (:130, :161), and `schema/source.rs`
  - Markdown link destinations: `darkmatter/lib/src/markdown/compose/link_resolve.rs`
    and `link_normalization.rs`
  - registered bundle members: `darkmatter/lib/src/config/registration.rs`
    and `registration/manifest.rs`
  - configuration and permission references:
    `darkmatter/lib/src/config/load/reference.rs`
  - style files: `darkmatter/lib/src/style/bespoke.rs`

  The sites that stay in launch mode are:
  - `simplified_schema::file_context::resolve_document_file_ref` and
    `document_file_context`. These serve the `darkmatter-file` format,
    `match(...)` globs, and expression functions, which are all composition
    values.
  - transclusion (`compose/transclusion/resolver.rs`)
  - data reads (`expression/data/load.rs`)
  - lifecycle targets (`lib/src/lifecycle/executor.rs`)

  A test proves an unchanged schema import, for example
  `Name@types.yaml` beside the document while a different `types.yaml` sits
  at the launch directory (Phase 6). It runs next to a guard test showing
  that `config::load` reads the same permissions file under both modes.

**File value binding (Darkmatter)**

- **R20: One binding record per file occurrence.**
  - Generalize `CallerFileProvenance` (`expression/resolve_ctx.rs:16`) into
    `FileBinding { occurrence: JsonPointer, raw: String, origin: FileResolutionContext, target: PathBuf, entry: EntryKind, mode: FileMode /* Eager | Lazy */, spelling: String, native: bool }`.
  - The records are stored in a `FileBindings` map, keyed by the existing
    occurrence pointers (`caller_occurrence`, `nested_occurrence`,
    `schema_validation.rs:2193`/`:1646`), inside `FrontmatterProvenance`
    (`compose/value_origin.rs`).
  - The installed frontmatter value is `spelling`.
  - `CallerProjection`'s separate `native` and `presentation` maps
    (`schema_validation.rs:280-327`) collapse into the spelling. The
    restore-native block in `run_with_registry` (`:803-826`) is deleted.
  - A plain string with the same text has no entry, so it never gains file
    identity.
  - `CallerInputRecord.raw` is kept for fresh preparation against another
    schema.
- **R21: When each kind of value is bound.** This preserves the pipeline order
  in `pipeline/mod.rs:414`:
  - **Caller inputs** are bound in `prepare_caller_projection`, before pass-1
    interpolation, as today.
  - **Document-authored values and defaults**, eager and lazy, are bound
    where `rewrite_eager_file_values` runs today
    (`schemas/rewrite.rs:87`/`:109`). That is after the effective schema
    and value are known, in both schema validation passes.
  - A lazy value is planned with R7.
  - An eager value must exist and satisfy `match(...)`.
  - A value already bound at the same occurrence with the same raw text is
    reused, so repeated preparation is stable.
  - An expression waiting on a pending schema-selected file keeps the
    existing deferral (`PendingDeferral`) and its existing failure.
  - The lazy-value exclusion in `is_eager_file_fragment` (`rewrite.rs:701`)
    is removed. Lazy and eager both bind; the test
    `lazy_file_value_not_rewritten_by_compose`
    (`schema_validation.rs:3936`) is rewritten to assert the new binding.
- **R22: Bound values are data, and `match(...)` judges the target.**
  - Each bound leaf is recorded in `DataPaths` (`value_origin.rs`), so a
    spelling like `{{NOTES}}/spec.md` is never re-scanned:
    - by pass-1 or pass-2 frontmatter interpolation
    - by repeated preparation
    - in a forwarded child (already data through `child_inherited_origin`,
      `transclusion/engine.rs:2024`)
    - by body rendering
  - A file operation on that text expands `{{NOTES}}` through the reference
    grammar with the captured request environment.
  - `match(...)` (`simplified-schema/lib/src/file_match.rs:647`
    `MatchKeyword::check`) judges the binding's `target` in its `origin`. The
    exact-text `SuppliedValues` lookup (`:657`) is replaced by a lookup of the
    occurrence's `FileBinding`.
  - Permissions, the read set, and typed errors use `target` and `origin`.
  - A portable spelling never grants access.
- **R23: Error when no text can name the target.**
  - Add `MarkdownError::UnrepresentableFileValue { reference: String, target: PathBuf, reason: SpellingReason /* NotUnicode | NoPortableSpelling */, source: SourceRef, property: Option<String> }`.
  - Its `md` diagnostic code is `dm.compose.unrepresentable_file`, added
    where `report_diagnostics.rs` maps `MarkdownError` (around :838-905) and
    to `COMPOSE_DIAGNOSTIC_CODES`.
  - Its Claudine lifecycle code is `composition.unrepresentable_file`, in
    `claudine/lib/src/diagnostics/registry.rs` `CODES`, with:
    - `category: Composition`
    - `disposition: Correctable`
    - `origin: Environment`
    - detail `["reference", "target", "reason", "source_path", "property", "event"]`
  - Add it to the `code` `suggest(...)` list in
    `darkmatter/schemas/partials/error.yaml:10`, which is what DMLS completes
    and checks `err.code` against, then `just shipped-schemas`.
  - Document one `failure` example that branches on
    `err.code == 'composition.unrepresentable_file'`, in
    `darkmatter/docs/topics/lifecycle/error-handling.md` and in Claudine's
    lifecycle topic.
  - No registered code fits. `composition.invalid_file_reference` is an
    authoring mistake (`Origin::Author`), and its `failure` vocabulary
    (`no_match` …) has no "exists but cannot be written" class.
- **R24: An explicit native result keeps its text.**
  - A leaf whose authored source is a whole-value `{{ absolute_path(…) }}`,
    detected on the parsed expression AST root, is recorded in a new
    `NativeLeaves` set beside `DataPaths`. A `file` binding at that leaf
    keeps the native text and sets `native: true`.
  - `NativeLeaves` is forwarded everywhere `DataPaths` is: to transclusion
    children, in Claudine sequence `with` and lifecycle `with:` overlays.
- **R25: Children receive bindings, not re-resolution.**
  - `inherited_child_options` (`transclusion/engine.rs:1952`) and the
    lifecycle `with:` overlay (`lifecycle/executor.rs:1569`, `:1641`, `:1702`)
    carry the parent's `FileBindings` for values passed intact.
  - In the child's schema preparation, an inherited data value whose
    occurrence has a binding with the same spelling adopts that binding:
    target, origin, mode, and native flag. Nothing is re-probed. This holds
    for a child in another checkout too.
  - Claudine's `PromptTaskRequest` (`claudine/lib/src/composition/sequence/task/mod.rs:209`)
    and `task_caller_records` (`claudine/cli/src/commands/wrap/sequence/task_run.rs:773`)
    forward intact file values as a **data** layer with their bindings. A
    task's own authored `with:` text stays authored.
  - This keeps an inline sequence and the same sequence in a referenced file
    equivalent, because both reach the same forwarding code.
  - Schema `isolated` still blocks a property, under the shared inheritance
    rule.

**DMLS and Claudine**

- **R26: DMLS context and diagnostics.**
  - DMLS keeps its request directory at the repository root
    (`dmls/docs/file-references.md`, "One context per repository"), and builds
    contexts with `ImplicitBases::Launch`. Its launch bases are therefore the
    repository root alone.
  - No host supplies a launch context today, so no protocol is added.
  - A `::file`/`::code`/`::toc-linking` target written as an implicit relative
    path that misses is still reported as `dm.transclusion.broken_path`,
    changed as follows:
    - severity drops from `WARNING` to `INFORMATION`
    - the message gains "(from the repository root; another launch directory
      may find it)"
    - `data` gains `"launch_dependent": true`
  - Explicit forms keep `WARNING` (`dmls/src/providers/dsl.rs:973`).
  - Completion:
    - `file(...)` value completion (`dmls/src/providers/frontmatter.rs:1383`)
      inserts the R8 spelling.
    - Directive-target completion (`dmls/src/providers/completion.rs:93`)
      inserts `./…` for a target in or below the document's folder, using the
      strategy `[SameDirRelative, ChildDir]` followed by `VALUE_STRATEGY`.
      Today's bare sibling insertion would now resolve from the launch bases.
  - Both `relativize` helpers (`completion.rs:202`, `frontmatter.rs:3712`) are
    deleted.
- **R27: Claudine's design page is edited under the spec's rulings.**
  - `claudine/docs/topics/file-referencing.md` asks for Ken's explicit
    approval before edits. The spec's decisions, which the author ruled on
    2026-11-25, are that approval for this content.
  - Rewrite its "The Implicit Relative Path" section (l.88-113) to state the
    launch order.
  - Correct the claim at l.144 that the CLI "immediately changes the directory
    to the repo root". Claudine switches only before spawning the provider
    (`cli/src/commands/wrap/exec/mod.rs:47`).
  - Do not edit the "Design-Intent Document" banner.
  - List every edited passage, with before and after text, in the
    implementation log for the review.
- **R28: Claudine completion and the chooser share one spelling.**
  - `CANDIDATE_STRATEGY`
    (`claudine/cli/src/completion/schema_completion/candidates.rs:314`) is
    deleted. `candidate_value` uses `VALUE_STRATEGY` with the launch context
    and the request's `ShadowCache`.
  - Bare `file` candidates are listed from every launch base in order:
    - `default_glob::default_markdown_candidates_filtered`, called once per
      base, with the existing depth and filter limits
    - deduplicated by target, first base wins
    - a target shadowed by a nearer base is not listed
  - The chooser's `path_label` (`commands/schema_interactive/mod.rs:748`) and
    the autocomplete window's `path_label` (`completion/autocomplete_ui.rs:357`)
    show the same spelling.
  - `resolve_file_value` (`schema_interactive/mod.rs:767`) and
    `supplied.rs:50-64` submit the spelling. Darkmatter re-binds the same
    target, because `match(...)` judges the target (R22).

**Inventory and migration**

- **R29: Routines that turn a path into text.**
  - **Replaced with `FileSpeller::spell` or `VALUE_STRATEGY`:**
    - Darkmatter `path_projection.rs` (all three helpers)
    - `cli/src/args/completion.rs:126` and `cli/src/args/path_completion.rs:708`
      (`md` argument completion)
    - DMLS `completion.rs:202` and `frontmatter.rs:3712` `relativize` (see R26)
    - Claudine `completion/operation_file.rs:400` `format_relative_insert`
    - Claudine `completion/composition/compose.rs:239` `format_relative_insert`
    - Claudine `completion/setter_value.rs:217` `format_relative`
    - Claudine `completion/composition/setter_value.rs:91`
    - Claudine `completion/schema_completion/candidates.rs:129`
    - the chooser and autocomplete `path_label`s (R28)
  - **Kept, each a report label or editor display with its own base.**
    Record each in the implementation log with this reason:
    - `reference/report.rs:353` `relative_display` (reference report, launch
      directory base)
    - `compose/util.rs:22` `abbreviate_path` (error and notice text)
    - DMLS `providers/reference_hover.rs:66` `display_path` (hover label)
    - DMLS `overlay/registry.rs:1134` `display_path` (log field)
    - DMLS `workspace/discover.rs:303`/`:364` (workspace index keys)
    - DMLS `wiki/logical_path.rs:59` (wiki logical names)
    - `file_links/discovery.rs:255` `display_path` (`::file-links` labels;
      the Markdown link base is the document)
    - context capture: `context/repository_scope.rs:127`,
      `context/current.rs:527`, `context/capture/docs.rs:152`. These are
      `ctx.*` values defined as repository-relative strings, not file values.
    - Claudine `lib/src/doctor/report.rs:166` `display_path` (diagnostic
      report shows exact paths)
    - Claudine `wrap/composition/timeouts.rs:178` (report label)
    - Claudine `wrap/composition/dry_run.rs:216` (dry-run report label)
    - Claudine `schema_completion/mod.rs:297` and
      `registration/inspect.rs:304`. These are not display text.
- **R30: How live documents migrate.**
  - The one authored neighbor among the live directives,
    `darkmatter/docs/lsp/features.md:17`, becomes `::file ./frontmatter.md`.
  - The five live repository-root-shaped directives keep their spelling.
    They resolve at the repository root base from any launch inside the
    repository:
    - `_fleet.md:48`/`:52`
    - `claudine/docs/getting-started/index.md:116`/`:120`
    - `claudine/docs/topics/schemas/schema-triggers.md:5`
  - The two intentionally missing fixture targets stay
    (`claudine/cli/tests/fixtures/lifecycle_traces/*`).
  - `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md:39`
    is a frozen corpus input and stays.
  - Fenced examples in docs that show a neighbor (`child.md`, `favorites.md`,
    `best_practices_for_abc.md`, …) are rewritten to `./name.md`.
  - The audit found 34 directive lines in 14 files, not the spec's 36 in 17.
    Record the difference in the log.
  - In prompts, replace each `dirname(X) + '/name'` and
    `parent_dir(X) + '/name'` with `join(dirname(X), 'name')`. Replace each
    `parent_dir(X) + '/' + basename(X)` with `X`. The sites are:
    - `prompts/_clarify/findings.md:17,19`
    - `_fix/fix-review.yaml:10,13`
    - `_implement/implement-feature.md:28,29,35,36,42,43,49`
    - `_implement/implement-plan.md:16,29,30,35`
    - `_implement/implement-review-findings-plan.md:21,22`
    - `_implement/review-findings-plan.md:16,17`
    - `_implement/implement-review.md:15,78,83`
    - `_implement/implement-suggestions.md:12,16,331`
    - `_reviews/review-implementation.md:24-27,37,38,41`
    - `_reviews/review-plan.md:13,14,20,21`
    - `clarify.md:27`
    - `complete.yaml:10,13`
    - `fix.yaml:19,20`
    - `implement.md:31,37,47,51`
    - `plan.md:19`
    - `review.md:33`
  - Three expressions are left as they are, because they are not a single
    sibling join: `implement-plan.md:39` `dirname(dirname(spec))`, `:41-43`,
    and `suggestion-review.md:16`.
  - In `prompts/feature.md:16`, `relative(i)` becomes `relative_path(i)`.
  - In `claudine/lib/tests/fixtures/sequence-schema/example-loops-review-loop.md:35`,
    `dirname(absolute(spec)) + '/review-' + …` becomes
    `join(dirname(spec), 'review-' + …)`.
  - Several prompt files carry uncommitted author edits (`git status`:
    `prompts/_implement/implement-plan.md`, `implement-suggestions.md`,
    `fix.yaml`, …). Edit on top of the working tree and never revert or stash
    them.
  - A shell command that interpolates a `file`-typed value switches to
    `{{ absolute_path(x) }}`, because `::shell` and `$( … )` run in the
    document's folder (`shell_expansion/executor.rs:81`
    `resolve_working_directory`). The audit of `prompts/` found none that
    interpolate a file value. The commands it found use `ctx.repo_root`,
    `area`, `base`, and `baseline-state`, which is a string from
    `frontmatter(...)`. Phase 5 re-runs the grep over fixtures and docs and
    converts every hit.

**Scope**

- **R31: No spike and no input robustness matrix.**
  - No spike is needed. The only runtime cost in the spec, the shadow
    checks, is settled by the ruling's design and proven by a probe-count
    test (Phase 6). It is not measured.
  - The input robustness matrix does not apply. No loader, deserializer, or
    configuration reader changes, and the `file` field validators accept the
    same shapes as before. Only the value bound after acceptance changes.

### Wave 1

- [x] **Mechanical renames**
  - Apply R2 (`to_forward_slash_string`, `try_forward_slash_string`) and R3
    (`LaunchScope`, `launch_scope`, `with_launch_scope`) across the workspace
    with `rg -l` and `sed`, then fix the doc comments on the renamed items.
  - Run this as a single task before anything else. It touches about 230
    files and would conflict with every other task.
  - Checkpoint: `cargo check --workspace --all-targets` succeeds.
    `rg "to_portable_string|try_portable_string|LaunchMagicScope|launch_magic_scope" -t rust`
    returns nothing outside `*/features/*` and `*/fixes/*`.

### Wave 2

- [x] **Launch lookup**
  - Implement R1, R3 `bases()`, R4, and R5 in `context.rs` and `resolve.rs`,
    including `CompletionAnchors` and `completion_roots`.
  - Add biscuit-file L1 tests under `biscuit-file/lib/tests/l1/`, one per
    rule:
    - the base order and deduplication when the launch directory equals the
      package or the repository root
    - missing package anchors
    - outside a repository, only the launch directory
    - a launch-directory shadow beats package and area shadows
    - `%` search and implicit glob roots
    - the `request_tree` boundary versus the source tree for `./`
    - a symlink escape keeps its typed failure
    - a miss advances, while an I/O error stops
    - `Authoring` mode is byte-for-byte unchanged (the existing suites
      pass unchanged)
- [x] **Directory probe and planning**
  - Implement R6 and R7, with tests:
    - a directory found at a later base
    - a nearer same-named file does not satisfy a directory probe
    - `plan_target` across the parent-directory ruling: from a package area,
      `darkmatter/features/x/review-3.md` is planned at the repository root,
      while `features/new.md` is planned in the area
    - no base has the directory, so the launch directory is used
    - a recursive refusal
  - Prerequisite: Launch lookup, in the same files. Run it in the same agent,
    after it.

### Wave 3

- [x] **Value spelling**
  - Implement R8 through R11 in `portable/strategy.rs`, `evaluate.rs`,
    `text.rs`, and a new `portable/shadow.rs`, and export the new items from
    `portable/mod.rs` and `lib.rs`.
  - Tests:
    - the spec's table rows at the biscuit-file level
    - the repository root directory falls back
    - outside every repository, the fallback chain is used
    - a foreign checkout falls back
    - a real shadow from `tools/` falls back to `{{WORKSPACE}}/feat/x.md`,
      then `~/`, then absolute
    - a missing target with a parent directory under a nearer base falls back
    - `probe_count()` is one per distinct first component per nearer base
      for 500 targets
    - a denying `ShadowProbe` falls through without ending evaluation
    - a Unix literal backslash keeps its native spelling
      (`#[cfg(unix)]`)
    - Windows native absolute output (`#[cfg(windows)]`)
    - a non-Unicode name gives `UnrenderableTarget` (`#[cfg(unix)]`)
- [x] **biscuit-file docs**
  - Rewrite `biscuit-file/docs/topics/file-references.md` as the single
    description of the reference forms, the bases (both modes and how to opt
    in), the portable path and `VALUE_STRATEGY`, and the terms table from the
    spec.
  - Replace the l.90/l.120/l.153/l.688 wording ("beside this document",
    "bare path").
  - Include a Mermaid flow of launch lookup and of the shadow check, and
    examples for each rule.
  - Update the `biscuit-file` repo skill (`.claude/skills/biscuit-file/`) for
    the renames and the new API.
  - Run in parallel with Value spelling.

### Phase 1 checkpoint

- [x] `just test` and `just lint` pass in `biscuit-file/`.
  `cargo check --workspace --all-targets` passes.

## Phase 2 — Darkmatter launch lookup and the shared speller

### Wave 4

- [ ] **Opt in and pin**
  - Implement R17 for Darkmatter (`ComposeRequest`, `open_argument`) and R19
    for every listed authoring site in simplified-schema and Darkmatter.
  - Update the source-scan guard in `darkmatter/lib/tests/l1/` that inventories
    context construction (see `.claude/skills/darkmatter/testing.md`) so a new
    `FileResolutionContext` consumer must name its `ImplicitBases`.
  - Tests:
    - an unchanged schema import with a launch-directory decoy
    - an unchanged permissions file
    - link destinations stay document-relative
    - from a subdirectory, `md compose` with an implicit `::file` finds the
      launch-directory file, the package-area file, then the repository-root
      file, in that order
- [ ] **FileSpeller**
  - Implement R18. Add `Work::ShadowProbe` to `darkmatter::work_counters`,
    and route every shadow check through the read gate and the read set.
  - Delete `path_projection.rs`'s helpers and move its callers.
  - Prerequisite: Phase 1. Run it in parallel with Opt in and pin; the files
    do not overlap.

### Wave 5

- [ ] **Path functions**
  - Implement R12 through R16 in `functions/fs.rs` and `functions/paths.rs`,
    the catalog `darkmatter/schemas/partials/functions.yaml` (signatures,
    descriptions, and examples for `absolute_path`/`relative_path`, plus every
    changed result example). `catalog/` names no function and does not
    change.
  - Run `just shipped-schemas` from the root, then `just regen-expr-doc`.
  - Update the contract tables:
    - `darkmatter/lib/tests/l1/path_function_contracts.rs`
    - `function_binding_registry.rs:491-494`
    - `fs_tests.rs`
  - Update the Rust test strings and doc comments listed by the audit:
    - `conditions.rs:1045,1048`
    - `frontmatter_interpolation.rs:2615-2642`
    - `fatality_characterization.rs:131,383`
    - `remote.rs:727-814`
    - `fs.rs` doc comments
    - `schemas/mod.rs:2818,2843`
    - `rewrite.rs:5,864`
  - Re-bless the golden fixtures with their switches, then diff and check
    every changed line is a rename or a spelling change:
    - `DMLS_UPDATE_GOLDEN=1` for `dmls/tests/fixtures/response_contracts/*.json`
    - `BLESS_VARIABLE` (`recursive_schema_golden.rs:469`) for
      `lib/tests/fixtures/recursive_schema_golden`
  - Tests:
    - directory lookup with the same directory name at a nearer and a later
      base
    - `join` builds a sibling of the selected directory
    - new index siblings and the no-family result
    - `find_files` order, skip warnings, and reusable results that open the
      listed files, including one shadowed under a nearer base
    - the probe count for a 500-file listing
    - both link label forms, Markdown punctuation, URLs, terminal hyperlink
      targets, and sibling-relative Markdown destinations
    - the retired names give `dm.expression.unknown_function`
  - Prerequisite: FileSpeller.

### Phase 2 checkpoint

- [ ] `just test` and `just lint` pass in `darkmatter/` and
  `simplified-schema/`.
  `just shipped-schemas --check`, run in `darkmatter/`, reports fresh
  artifacts.

## Phase 3 — Binding file values

### Wave 6

- [ ] **Binding records**
  - Implement R20 and R21 in `schema_validation.rs`, `schemas/rewrite.rs`,
    `expression/resolve_ctx.rs`, and `value_origin.rs`. The diagnostics that
    read `CallerFileProvenance` today (`owned_call_context`,
    `file_reference_diagnostic`) read `FileBinding`.
  - Tests through the `compose_with` public API:
    - implicit relative, absolute, and home-relative caller inputs bind to
      the same spelling, launched from the root and from a subdirectory
    - an authored file, a schema default, and nested lists and objects
    - a plain string matching a bound spelling stays a string
    - an ambiguous `file | string` stays as written
    - eager missing errors and `match(...)`
    - a lazy first candidate stays bound when missing
    - a later file exists, and a subsequent lifecycle step writes to the
      selected location
    - the recursive lazy refusal is unchanged
- [ ] **Data origin, match, native**
  - Implement R22 and R24 (`DataPaths`, `NativeLeaves`, `file_match.rs`).
  - Tests:
    - `{{NOTES}}/spec.md` survives both passes, repeated preparation, child
      forwarding, and body rendering
    - export the value, read it back with the same launch context, and get
      the original target
    - `absolute_path(spec)` assigned to a `file` field stays native
  - Prerequisite: Binding records, in the same files. Run it in the same
    agent, after it.

### Wave 7

- [ ] **Children and lifecycle**
  - Implement R25 in `transclusion/engine.rs` and `lifecycle/executor.rs`.
  - Lifecycle file targets (`edit`, `ensure_file`, `set_frontmatter`, write
    targets) resolve and plan with launch lookup and R7, and their preflight
    projection (`compose/preflight/lifecycle.rs`) calls the same function.
  - Tests:
    - transclusion and lifecycle forwarding into a second checkout and a
      nested checkout keep the target
    - an `isolated` field is not passed down
    - the child's authored `&` follows its own checkout
    - preflight and runtime agree on the target
- [ ] **Unrepresentable error**
  - Implement R23 on the Darkmatter side: the `MarkdownError` variant, the
    `dm.compose.unrepresentable_file` mapping, `error.yaml`
    `suggest(...)`, and `just shipped-schemas`.
  - Test: a non-Unicode filename (`#[cfg(unix)]`) yields the typed error,
    with the original reference, the target, and the authored location.
  - Run in parallel with Children and lifecycle.

### Phase 3 checkpoint

- [ ] `just test` and `just lint` pass in `darkmatter/`. The spec table holds
  through `md compose` from the repository root; check it with a CLI spawn
  test using `CliProcessFixture` and `git init`.

## Phase 4 — DMLS and Claudine

### Wave 8

- [ ] **DMLS**
  - Implement R26: launch-mode contexts, the `INFORMATION` severity for an
    implicit miss with `launch_dependent`, the completion insertions, and
    deleting the `relativize` helpers.
  - Hover and completion for `absolute_path`/`relative_path` come from the
    regenerated catalog. Confirm them with a response-contract test.
  - Tests: the implicit-miss diagnostic, directive and `file(...)`
    completion insertions, the retired-name diagnostic, and passive analysis
    that makes no shadow probe (`Work::ShadowProbe` stays 0).
- [ ] **Claudine composition**
  - Implement R17 in `invocation_context.rs` and R25's Claudine side
    (`PromptTaskRequest`, `task_caller_records`, preflight
    `wrap/sequence/mod.rs:149-164`, `phase1c.rs:217`).
  - Add `composition.unrepresentable_file` to
    `claudine/lib/src/diagnostics/registry.rs` with its detail projection and
    a conformance row in `claudine/lib/tests/l1/diagnostic_detail_conformance.rs`
    (R23).
  - Tests: `claudine compose` and a sequence task (inline and from a
    referenced file) bind the same spelling, and a prompt outside the
    repository resolves implicit references from the launch bases.
- [ ] **Claudine completion**
  - Implement R28 and the Claudine rows of R29 (`operation_file.rs`,
    `composition/compose.rs`, `setter_value.rs`,
    `composition/setter_value.rs`, `candidates.rs`).
  - Update the parity tests:
    - `cli/src/completion/schema_completion/parity_tests.rs`
    - `cli/src/commands/schema_interactive/tests.rs:327`
    - `cli/src/completion/autocomplete_ui.rs:587`
    - `cli/tests/l1/entry_point_parity.rs` (with the shared matrix in
      `darkmatter/lib/tests/common/entry_point_parity/mod.rs`)
    - `completion_resolution_round_trip.rs`
  - Update the L2 captures `level2_auto_complete_chooser.rs` and
    `level2_provided_partial_file_capture.rs`, keeping terminal focus away
    from the host. The capture tests already use the shared harness.
  - Test: from `darkmatter/`, a `spec` existing in the area and in a sibling
    area:
    - the area file is chosen
    - a file only at the repository root is still found
    - the area-relative spelling of a sibling-only file fails with `no-match`
    - insertion, label, submission, and prepared value agree
  - Prerequisite: Claudine composition (the shared launch context). Same
    agent, after it.

### Phase 4 checkpoint

- [ ] `just test` and `just lint` pass in `darkmatter/` (DMLS) and `claudine/`.
  `just test-l2` passes in `claudine/` for the touched captures.

## Phase 5 — Migration and documentation

### Wave 9

- [ ] **Document migration**
  - Apply R30 to the directives, fenced examples, prompts, and the fixture.
  - Rename every remaining `absolute(`/`relative(` expression use found by the
    audit:
    - `darkmatter/docs/topics/darkmatter-expressions.md:1543,1572,1666,1671-1675,1688,1769,1770`
    - `.claude/skills/darkmatter/compose.md:581-590`, where the "Path
      functions" table is rewritten to the new results
    - `prompts/_add/add-expressions.md:83`
  - Re-run the audit greps over `*/tests/fixtures/**` and `*/docs/**` for
    shell commands that interpolate a `file` value, and convert each hit to
    `absolute_path`.
  - Check that the migrated targets compose from a different launch
    directory. Run `md compose` on `darkmatter/docs/lsp/features.md` from the
    repository root and from `darkmatter/`.
- [ ] **Darkmatter and Claudine docs**
  - Cut `darkmatter/docs/topics/file-referencing.md` back to what Darkmatter
    adds: how a value is bound, the entry-point policy, and completion. Point
    to the biscuit-file page instead of restating the grammar. It must show:
    - a document neighbor (`./plan.md`)
    - a repository file reached from a package area
    - a prompt outside the repository
    - a shell handoff with `absolute_path`
  - Update the implicit-path wording the audit listed in:
    - `darkmatter-expressions.md:1555`
    - `errors/file-reference-failures.md` (the `no-match` row)
    - `inline/file-links.md`
    - `transclusion.md`
    - `dmls/docs/file-references.md` (the R26 severity rule)
    - `dmls/docs/diagnostics.md`
    - `darkmatter/lib/README.md:91`
  - Add the R23 `failure` example to
    `darkmatter/docs/topics/lifecycle/error-handling.md` and to
    `claudine/docs/topics/flow-control/lifecycle.md`.
  - Edit `claudine/docs/topics/file-referencing.md` per R27, and record the
    before and after passages in the log.
  - Update the repo skills `.claude/skills/darkmatter/` (`compose.md`,
    `requests.md`, `expressions.md`) and `.claude/skills/claudine/`.
  - Update the caller skill sources under `darkmatter/docs/use-skills/` and
    `claudine/docs/use-skills/`, including
    `use-darkmatter/authoring.md:170-214`, then run `just publish-skills` in
    `darkmatter/` and `claudine/`. Check the published text renders
    `{{ … }}` examples correctly.
  - No crate is added or removed, so `docs/dependencies.md` does not change.

### Phase 5 checkpoint

- [ ] `md validate refs` over each edited docs tree reports no new broken
  references. `just publish-skills` succeeds in both areas.

## Phase 6 — Verification matrix and final gates

### Wave 10

- [ ] **Verification map**
  - Write the spec's Verification section into the implementation log as a
    table: one row per bullet clause, with the test name and file that
    covers it.
  - For any clause without a test, add the test. Use the in-process public
    API (`compose_with`, Claudine's `prepare` service) where possible.
  - Spawn `md` or `claudine` only for wiring: per entry point, the control
    case plus the launch-from-subdirectory case, asserting the printed value
    and that nothing else launched.
  - The clauses most likely to be missing are:
    - a lifecycle write of a `+`-built path from a package area, including
      a path typed relative to the package area and one whose folder exists
      under no base
    - launching from outside every checkout
    - equivalence of inline and referenced sequences
    - passive validation that makes no checks
- [ ] **Final gates**
  - Run `just test`, `just test-l2`, and `just lint` in `biscuit-file/`,
    `simplified-schema/`, `darkmatter/`, and `claudine/`.
  - Run `cargo check --workspace --all-targets`.
  - Run `just ci-local --plan` and record the planned cells.
  - Set the spec's frontmatter `implemented: true` and
    `implemented-by: claude/opus`.
  - End the implementation log with "implementation complete, ready for
    review", listing R13's interpretation and R27's edited passages for the
    review.
  - Do not move the fix into `_completed` and do not commit unless told to.

### Phase 6 checkpoint

- [ ] Every gate above is green and every verification row names a passing test.
