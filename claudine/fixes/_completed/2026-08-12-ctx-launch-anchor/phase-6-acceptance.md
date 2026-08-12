# Phase 6 acceptance evidence

## Requirement-to-test mapping

| Criterion | Observable behavior | Public route or canonical owner | Targeted regression evidence |
|---|---|---|---|
| AC1 | Repository-root and package-local documents retain the launch area through direct and repeated loop execution, including lifecycle interpolation and `when:`. | Real `claudine compose --codex` process with a provider stub. | `cli_uses_launch_context_across_launch_source_matrix`; `cli_loop_reuses_launch_context_for_root_and_package_prompt_copies` |
| AC2 | A document in the opposing package area reports the launch area in its body, quoted whole-value frontmatter, preflight-expanded bytes, lifecycle output, and `when:`. | Real `compose` and `inline-compose` processes. | `cli_uses_launch_context_across_launch_source_matrix`; `inline_cli_uses_launch_context_across_launch_source_matrix` |
| AC3 | An external source does not replace launch repository facts, and an in-repository source cannot invent repository facts for an outside launch. | Real `compose` and `inline-compose` processes. | `cli_uses_launch_context_across_launch_source_matrix`; `inline_cli_uses_launch_context_across_launch_source_matrix`; `cli_keeps_launch_repository_facts_absent_when_source_is_in_repo` |
| AC4 | Capture occurs through the real CLI owner rather than a hand-built executor snapshot. | Spawned `claudine` binary, provider executable, filesystem, and Git repositories. | All four `ctx_launch_anchor_baseline` CLI matrix tests and all three `sequence_ctx_launch_anchor` tests |
| AC5 | One target-adjusted snapshot is reused within an epoch; stabilized reread adds only missing groups; re-entry starts one fresh epoch. | Direct epoch owner, harness reread owner, and proxy/retry/resume entry owner. | `document_epoch_reuses_one_target_adjusted_launch_snapshot`; `stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity`; `proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs` |
| AC6 | Resolved `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` reach direct/loop, re-entry, JIT, and serial/parallel sequence tasks. | Canonical direct owner, harness re-entry owner, JIT preflight, and real sequence CLI. | `document_epoch_reuses_one_target_adjusted_launch_snapshot`; `proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs`; `template_preflight_combines_launch_facts_with_the_selected_target`; `serial_and_parallel_prompt_tasks_keep_target_identity_and_launch_facts` |
| AC7 | Sequence graph/task/group/prompt work uses launch facts; pre-selection target identity is rejected for native command forms and quoted/folded YAML scalar forms. | Public graph owner and real `claudine sequence --dry-run`. | `root_referenced_task_and_group_shells_share_one_launch_context`; `graph_preselection_rejects_each_target_identity_root`; `external_sequence_uses_launch_facts_and_source_relative_schema_and_file_reads`; `graph_preflight_rejects_all_target_identity_roots_across_yaml_scalar_forms`; `serial_and_parallel_prompt_tasks_keep_target_identity_and_launch_facts` |
| AC8 | Lifecycle consumes prepared state; unchanged loop work stays in the epoch; proxy, retry, and resume retain launch facts and target identity. | Lifecycle request seam, loop preparation owner, and harness re-entry owner. | `lifecycle_context_consumes_prepared_snapshot_and_accounts_for_a_missing_one`; `document_epoch_reuses_one_target_adjusted_launch_snapshot`; `proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs`; `rebuild_records_an_invocation_backed_missing_prepared_context` |
| AC9 | Relocating primary/appendix system prompts and overlays does not move launch-facing values. | Normal system-prompt session path and passthrough overlay owner. | `normal_session_composes_the_shipped_root_system_prompt_from_launch_context`; `primary_prompt_relocation_keeps_launch_context_and_source_local_files`; `appendix_relocation_keeps_launch_context_and_source_local_files`; `passthrough_seed_relocation_keeps_launch_ctx_and_source_file_resolution` |
| AC10 | Eager schema file values, `$schema`, transclusion, and overlay/system-prompt files stay source-relative in launch/source conflicts. | Real compose/sequence CLI plus canonical system-prompt and overlay paths. | `cli_keeps_eager_schema_files_source_relative_and_ctx_launch_relative`; `external_sequence_uses_launch_facts_and_source_relative_schema_and_file_reads`; the three relocation tests named for AC9; `task_stack_keeps_source_resolution_but_uses_the_launch_repository_context` |
| AC11 | Repeated projection does not repeat ambient CWD, HOME, environment, Git, topology, or host work. | Invocation owner and every epoch owner named by AC5. | `launch_capture_ignores_same_opposing_and_external_document_sources`; `launch_capture_reports_no_repository_for_an_outside_launch`; `repeated_launch_capture_reuses_retained_evidence_after_ambient_state_changes`; `launch_context_extension_projects_only_missing_groups_and_preserves_overrides`; all AC5 counter tests |
| AC12 | A new production direct capture owner fails with a normalized location; every allowed site carries a compatibility, documentation, or live-event reason. | Production source inventory over `claudine/lib/src` and `claudine/cli/src`. | `prepared_context_capture_owners_hold_the_line`; `capture_owner_guard_reports_a_forbidden_normalized_source_location` |
| AC13 | Component-aware paths handle macOS temporary-root aliases, symlinked roots, Windows drive/UNC key shapes, and platform separators. | Real CLI fixtures plus invocation path owners. | `path_containment_is_component_bounded`; `repository_keys_preserve_windows_drive_and_unc_shapes`; `symlinked_launch_ancestor_reuses_one_observation_and_authored_root`; `symlinked_launch_ancestor_still_separates_nested_repositories`; the CLI matrices above |
| AC14 | Scoped package build, L1, L2, and lint gates pass without focusing a terminal window. | Canonical area `just` recipes. | Recorded under Final gates below. |

The exact original regression expressions, `{{ ctx.area }}` and
`{{ ctx.repo_root }}`, appear in body, quoted whole-value frontmatter,
preflight shell input, lifecycle interpolation, and `when:` conditions. Missing
launch repository values are covered by the inverse CLI case. Target-identity
rejection covers double-quoted, single-quoted, flow-array, and folded YAML
values. Malformed and invalid-input behavior remains covered by the broader
Claudine suites.

## Artifact and persistence coverage

`shipped_system_prompt_corpus_parses_and_scans_requirements` passively parses
and requirement-scans every shipped `system-prompt.md` artifact.
`normal_session_composes_the_shipped_root_system_prompt_from_launch_context`
uses the real shipped root prompt through normal session discovery and
preparation. Phase 6 changes no persisted value, so a repeated
read/write/read persistence test is not applicable.

## Targeted results

- `cargo nextest run -p claudine-cli --test ctx_launch_anchor_baseline --test sequence_ctx_launch_anchor`: 8 passed after the AC1 loop case was added.
- Targeted `claudine-cli` owner/inventory filter: 10 passed.
- Targeted `claudine` owner/system-prompt/preflight filter: 10 passed.
- Targeted macOS symlink and Windows path-shape filter: 4 passed.
- Total targeted evidence: 32 passed, 0 failed, 0 skipped.

Phase 6 added one test:
`cli_loop_reuses_launch_context_for_root_and_package_prompt_copies`. It closes
the real-CLI loop half of AC1 and asserts two provider iterations plus body,
frontmatter, preflight, lifecycle, and `when:` outputs for both source
locations.

## Capture-owner and blast-radius review

The inventory found only the two prepared-context owners:
`InvocationContext::capture_launch_context` and the public-library
`derive_compose_context` fallback. The four direct-capture exceptions are
content-only compatibility, document compatibility, the documentation-only
`claudine context --values` report, and live `current.ctx.*`; each reason is
specific and none is a canonical CLI route.

GitNexus reported high aggregate change risk because the work spans compose,
sequence, system-prompt, overlay, and Darkmatter context paths. The eight
direct callers of the retained `runtime_evidence` seam are within Claudine's
library/CLI routes and tests. Changed-flow detection identified compose,
sequence, and passthrough-overlay execution flows. `sniff` dependency mapping
therefore selects the `claudine` and `darkmatter` package areas for scoped
build/test/lint gates; the Darkmatter APIs added here are additive.

## Final gates

- `just build` passed for all five Claudine packages and all three Darkmatter
  packages.
- `just test` passed 6,652 L1 tests across all five Claudine packages and 7,538
  L1 tests across all three Darkmatter packages, with no failures. The
  canonical L1 tier filter skipped 253 Claudine tests (0 catalog, 0 library,
  4 contract, 245 CLI, and 4 generator) and 185 Darkmatter tests (113 library,
  69 CLI, and 3 DMLS); these belong to explicitly non-L1 name tiers rather
  than representing runtime skips.
- `just test-l2` passed 230 Claudine CLI tests and three Claudine generator
  tests. The repository harness kept its tmux, WezTerm, and Apple Terminal
  fixtures under background test control; 2,557 non-L2 tests were skipped by
  the tier filter.
- `just lint` passed for all five Claudine packages and all three Darkmatter
  packages, including formatting checks, Clippy, structural guards, and the
  lifecycle documentation-facet guard.
- The original root-copy and package-copy CLI reproduction passed for body,
  quoted frontmatter, preflight command bytes, lifecycle interpolation,
  `when:`, and two loop iterations.
- `git diff --check` passed. The final diff review found the existing
  `prompts/_implement/implement-plan.md` workspace edit outside this plan's
  recorded files and left it untouched.

Validation ran on macOS. Windows drive and UNC representations are covered by
targeted path tests, and fixtures use component-aware `Path`/`PathBuf`
construction; Windows and Linux jobs were not available in this local session.
The only non-failing diagnostic was the macOS linker's existing compact-unwind
fallback warning for a large `__eh_frame` section during the Claudine L1 gate.
