---
spec: "/home/build/wt/rusty-biscuit/feat-fix-sequence/claudine/features/2026-11-22-better-errors-and-dry-run/spec.md"
plan: "claudine/features/2026-11-22-better-errors-and-dry-run/plan.md"
implemented-by: "claude/opus"
started-phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/findings/mod.rs
    - darkmatter/lib/src/findings/trail.rs
    - darkmatter/lib/src/findings/sink.rs
    - darkmatter/lib/src/findings/report.rs
    - darkmatter/lib/src/findings/reference.rs
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/cli/src/output/mod.rs
    - darkmatter/lib/tests/l1/findings.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/diagnostic_projection.rs
    - darkmatter/lib/tests/l1/report_diagnostics.rs
    - darkmatter/lib/tests/l1/shared_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/document_check/mod.rs
    - claudine/cli/src/commands/document_check/outcome.rs
    - claudine/cli/src/commands/document_check/selection.rs
    - claudine/cli/src/commands/document_check/preview.rs
    - claudine/cli/src/commands/document_check/tests.rs
    - claudine/cli/tests/l1/dry_run_check.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/context_construction_guard.rs
    - claudine/cli/tests/l1/switch_catalog_guard.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/fixtures/check/fleet-run1/renderers/_fleet.yaml
    - claudine/cli/tests/fixtures/check/fleet-run1/renderers/_find.md
    - claudine/cli/tests/fixtures/check/fleet-run2/renderers/_fleet.yaml
    - claudine/cli/tests/fixtures/check/fleet-run2/renderers/_find.md
    - claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run1_reports_every_problem_and_previews_the_composable_task.snap
    - claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run2_human.snap
    - claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run2_json.snap
docs_updated_during_phase_1:
    - claudine/docs/cli/sequence.md
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/claudine/SKILL.md
    - darkmatter/docs/use-skills/use-darkmatter/library.md
packages:
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
    - claudine
    - claudine-cli
decisions-for-review:
    - |-
      The passive check is built but NOT yet `--dry-run`. Switching the dispatch made 285 existing L1 tests fail, 208 of them
      for real reasons: the old preview runs the library preflight (dynamic and file-referenced sequence sources, list
      classification, groups, task files, catalogs, preflight errors), and this phase's walker reports those as not checked,
      so a dry run would have said "fine" where a real run fails. `main.rs` routes a document dry run to
      `commands/document_check` only when `document_check::enabled()`, today the `test-fixtures` seam
      `CLAUDINE_TEST_PASSIVE_DRY_RUN` (`1` human, `json` JSON). `--dry-run` and `--dry-run-shell` behave as before.
      Alternatives: ship the switch and migrate about 40 test files now, or keep the old preview as a fallback for what the
      walker cannot reach. Redo: once the walker reuses `Loader::walk_plan`/preflight (plan Rule 9), delete `enabled()`,
      route every document dry run there, add `--json`, refuse `--dry-run-shell`, and migrate the old preview's L1
      suites (`sequence_sources_cli`, `sequence_errors_cli`, `entry_point_parity`, `argv_normalization`,
      `compose_prompt_tiers`, `wrap_sequence_composition`, `level1_dry_run_pty`, `wrap_compose_preflight`, …).
    - |-
      `--json` is not declared. Declaring it on `SharedComposeArgs` (spec) took a provider's own `--json` out of the caller
      tail after the file (`provider_tail_launch::a_resume_carries_the_tail_exactly_once` failed). Shipping the spec's flag
      is a behavior change for callers who forward `--json` to a provider; it needs either acceptance of that, or a different
      name. Redo: add the flag with the switch and update that test.
    - |-
      The sequence walker reads the root's authored `sequence` list directly (string and mapping items, executables,
      `with`, `agent`, `model`, `prompt`) instead of `normalize.rs`/the preflight loader. It does not walk nested groups,
      task files, nested or referenced sequences, or command-backed lists (each is an `info` not-checked row). Redo:
      Rule 9's shared walker; keep the finding codes and messages.
    - |-
      The provider chain is written out in `document_check/selection.rs` (flag, `agent=` setter, `with.agent`, prompt
      frontmatter `agent`, root `agent`) rather than recorded inside `wrap/sequence/providers.rs::select_target` and
      `wrap/composition/target.rs` (Rule 10). List-valued `agent`, `preferred_agent`, and the review rules are not
      modeled. Not installed = error; unresolved = error on a non-terminal stderr, warning on a terminal. Redo: record
      attempts at the selector's decision points and render them.
    - |-
      The model requirement is a presence check over `-m`, the provider's model variables, `MODEL`, `with.model`, the
      prompt's `model`, and the profile's configured default, instead of reusing `resolve_model_and_validate`'s pure
      precedence. Redo with Rule 10.
    - |-
      The dry-run guard sits in Darkmatter's `prepare_resolved` and `prepare_site` (after the built-in blacklist, before
      approval) and again in `execute_prepared_outcome`. It refuses with `ShellExpansionError::SkippedInDryRun`, so a
      preview depending on any command is `not checked` as a whole. Rule 6's deferred-value propagation (skip only the
      dependent parts) and the remote/ask/read-approval passive policy are not built. Remote reads in a dry run follow
      `allow_remote_transclusion` plus the `network` permission, as a real compose does.
    - |-
      The preview composes `Markdown::compose_with` directly (`document_check::preview::compose_passively`, added to the
      `compose_with` allowlist) with `with_dry_run()` and `CacheAccessMode::Off`. Caller setters, `--set`, and a task's
      `with:` are inserted into the document's frontmatter. The canonical `prepare_*` path was not used because its staged
      initialize and approval are effects. Alternative: a `dry_run` field on `PrepareOptions` applied in
      `canonical_compose_options`. Redo if the canonical preparer gains a passive mode.
    - |-
      The three new Darkmatter codes live in `darkmatter::findings::FINDING_CODES`, not in `COMPOSE_DIAGNOSTIC_CODES` (the
      `md --deny` catalog), because `md compose` does not emit them yet. Claudine's dry-run codes (`composition.*`,
      `internal.bug`) are in a test-only `CHECK_CODES` list, not in `diagnostics/registry.rs`, which holds error codes with
      lifecycle facets. Redo when Rule 13 wires `md compose`, and if these become lifecycle errors.
    - |-
      `Diagnostic` gained `hint`, `notes` (used for the provider chain lines), and `trail`. `ReferenceDetails` gained `base`,
      `resolved`, `searched`, and `suggestions`. All are omitted when empty, so existing JSON is byte-identical. The legacy
      `iteration_path`/`included_via` are not yet serialized from `trail` (Rule 2). `RelatedLocation` reuses the existing
      `DiagnosticRelated`. The reference renderer of `md validate refs` was not extracted (Rule 4); `FindingReport` has its
      own renderer.
    - |-
      `missing_document`'s hint names the path relative to the launch directory (`DocumentConstruct::launch_dir`). For a
      sequence step the host overrides `base` with the sequence file, as the spec's example shows.
message-to-agent: |-
  Phase 1 is only partly delivered. The building blocks are landed and tested: the Darkmatter `findings` module (record,
  trail, sink, report, missing-document builder), `ComposeOptions::with_dry_run()` with its shell guard, and Claudine's
  `commands/document_check/` passive check, which reproduces the spec's fleet Run 1/Run 2 reports. The public switch is
  held back behind `document_check::enabled()` (see the first decision). Still open in Phase 1, in this order:
  (1) Recover Graph: run the check over the library's preflight loader and `discover_sequence_inputs`, with per-item
  recovery and `TaskSite`, and add the inputs check (today it is never reported for a prompt task).
  (2) Record provider attempts in the real selectors and reuse `resolve_model_and_validate`'s precedence.
  (3) Make the check `--dry-run`: remove `enabled()`, add `--json`, refuse `--dry-run-shell`, delete the task-runner
  preview for the three document commands, and migrate the old preview's L1 suites (list in the first decision; about
  285 tests). Then restore the docs rewrite. The rejected first version of the Claudine docs (`docs/cli/sequence.md`,
  `docs/topics/composition.md`, `docs/topics/flow-control/*`, the use-claudine skill) was reverted; `docs/cli/sequence.md`
  carries a "Passive check (planned)" section to expand.
  (4) Rule 6 deferred values and passive remote/read policy in Darkmatter; Rule 7 excluded-block empty causes and both
  `prepare.rs` empty checks.
  (5) Rule 8 `CompositionError::CheckFailed` and the real-run report; Rule 13 `md compose` collection; Rule 4 renderer
  extraction with `md validate refs`/`md graph` baselines; Rule 2 legacy-field projection from `trail`;
  Rule 5 suggestion-search move into biscuit-file. Rules 14–16 belong to Phases 2–3.
  None of the plan's Phase 1 checkboxes is ticked, because no task is finished as written.
---

# Implementation Log for 2026-11-22-better-errors-and-dry-run (3 phases)

## Phase 1

- 2026-11-26: started Phase 1. Read the spec and plan; surveyed the
  Darkmatter `Diagnostic` record, Claudine's compose/sequence argument
  model, and the dry-run path.
- 2026-11-26: **Build Findings (partial).** Added `darkmatter::findings`
  (`findings/{mod,trail,sink,report,reference}.rs`): tagged `Trail`/
  `TrailFrame`, `FindingSink` with opaque `SubjectId`/`FindingId`,
  equivalence dedup and `not_checked`, `FindingReport` with counts and a
  biscuit-terminal human renderer, `EmptyBodyCause`, and the
  `missing_document` builder over biscuit-file's `DetailedResolution` with a
  shared `ListingMemo` and up to three suggestions. `Diagnostic` keeps its
  public path and gains `hint`, `notes`, `trail`, `actionable`/`projected`
  constructors and setters; `ReferenceDetails` gains `base`, `resolved`,
  `searched`, `suggestions`. New fields are omitted when empty, so existing
  JSON is unchanged.
- 2026-11-26: **Guard Composition (partial).** `ComposeOptions::with_dry_run()`
  (part of the options identity). Darkmatter refuses every document command
  under it with `ShellExpansionError::SkippedInDryRun`, after the built-in
  blacklist and before approval (`prepare_resolved`, `prepare_site`), and
  again before the cache and spawn (`execute_prepared_outcome`).
- 2026-11-26: **Integrate Claudine Checks (partial).** New
  `claudine/cli/src/commands/document_check/` (mod, outcome, selection,
  preview, tests). Added `ExecutableField::noun`. Dispatched from
  `main.rs::dispatch` before tracing, the Darkmatter config load, the wrapper,
  and `ensure_config_exists`, but only under `document_check::enabled()`.
- 2026-11-26: switched `--dry-run` to the check first; the full CLI suite
  then failed 285 tests (208 still failing with provider findings
  downgraded). These are real coverage gaps of the walker, so I held the
  switch back. I restored `--dry-run-shell`, the task-runner hook, the old
  `sequence_dry_run.rs`, and the Claudine docs and skills I had rewritten.
  The new tests moved to `cli/tests/l1/dry_run_check.rs` behind the seam.
  See `decisions-for-review`.
- 2026-11-26: **Isolate Startup (not done).** `--json` is not declared (it
  shadowed a provider's `--json` in the caller tail). The passive config
  projection (Rule 11) was not built; the check reads configuration only
  through `InvocationContext` without `with_darkmatter_config`, and
  `dry_run_check::a_dry_run_creates_no_configuration` proves nothing is
  created.
- 2026-11-26: **Prove Passive Reports (partial).** Fixtures
  `cli/tests/fixtures/check/fleet-run1/` and `fleet-run2/`; insta snapshots
  of the human report and the JSON envelope reproduce the spec's Run 1 and
  Run 2 (2 errors, 2 warnings, 3 info; previewed 1 of 2). Also covered:
  continuation past a failed task, an unknown provider, commands and side
  effects never run, no configuration created, provider not on PATH,
  OpenCode's model requirement, the inline file left intact, a missing root
  as a finding, and `--dry-run-shell` refused by the check. Darkmatter
  `lib/tests/l1/findings.rs` (16 tests) covers the record, trail, sink,
  renderer, reference builder, and the guard with and without pre-approval.
- 2026-11-26: guards updated with reasons: `compose_with` allowlist,
  context-construction allowance for the seam's `var_os`, the ownership-site
  list, and the error-collapse allowlist. The module was split to stay under
  the 800-line placement limit.
- 2026-11-26: gates. Claudine `just test`: 13089 passed, 16 skipped
  (the CLI suite alone: 5403 passed). `just lint` is clean. Dependents,
  serially: darkmatter lib, full, 12903 passed; darkmatter-cli, scoped to
  output/json/diagnostic/validate/graph/compose, 1507 passed;
  darkmatter-lsp, scoped to diagnostics, 187 passed. biscuit-file was not
  changed. Cross-OS: no Windows run. The new code uses `/`-normalized display
  paths and `EXE_SUFFIX` fakes, and the switch is not live.
- 2026-11-26: `just check-tier-coverage`: Stranded (0). A first attempt
  failed with "listing failed" for 49 cells because the disk was full (0 B
  free). `just sweep` freed 71 GiB of incremental caches, and the retry
  passed. `just publish-skills` (darkmatter) failed the same way and then
  succeeded.
- 2026-11-26: docs: `claudine/docs/cli/sequence.md` gains a "Passive check
  (planned)" section. Darkmatter `docs/cli/data-contracts.md` documents the
  new record fields and finding codes, and `docs/inline/preflight-checks.md`
  documents dry runs. The `use-darkmatter` caller skill (`library.md`) gains
  `with_dry_run`, and the repo `claudine` skill describes the held-back check.
