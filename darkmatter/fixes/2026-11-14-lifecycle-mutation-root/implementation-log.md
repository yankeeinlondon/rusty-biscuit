---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/fixes/2026-11-14-lifecycle-mutation-root/spec.md"
plan: "darkmatter/fixes/2026-11-14-lifecycle-mutation-root/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/src/lifecycle/executor/tests.rs
  - darkmatter/lib/src/effects/verbs.rs
  - darkmatter/lib/src/effects/fs_write.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs
  - claudine/lib/src/composition/sequence/task/tests.rs
  - claudine/cli/tests/l1/compose_initialize_acceptance.rs
  - claudine/cli/tests/l1/lifecycle_prep.rs
  - claudine/cli/tests/l1/lifecycle_mutation_root.rs
  - claudine/cli/tests/l1/lifecycle_edit_action.rs
  - claudine/cli/tests/l1/compose_schema_cli.rs
  - claudine/cli/tests/l1/compose_caller_file_provenance.rs
  - claudine/cli/tests/l1/with_file_provenance.rs
  - claudine/cli/tests/l1/ctx_launch_anchor.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/lifecycle.md
packages:
  - darkmatter
  - claudine
  - claudine-cli
  - darkmatter-cli
  - sniff
decisions_for_review:
  - |-
    Claudine's existing tests that encoded the old "create a missing relative
    target at the mutation root" behavior were corrected in Phase 1 rather
    than left red until the Phase 3 sweep the plan schedules (R10.3). The
    library change alone flips their outcome, and each phase is committed
    separately, so leaving them would commit a red dependent suite. Only test
    expectations changed (markers are now read beside the authoring document,
    one foreign-prompt edit now writes through `{{ ctx.cwd }}`), and every fix
    was checked to hold under both the current host roots and the Phase 2
    launch-scope root. Alternative: defer to Phase 3 as planned, at the cost
    of a red Claudine suite between phases. Reverting means restoring those
    test files; no production code depends on it.
  - |-
    `lifecycle_mutation_root.rs`'s first test was rewritten now (plan R10.2)
    as `a_relative_write_from_a_prompt_in_another_repo_is_refused_and_creates_nothing`.
    The CLI's status line is clamped to one line, so the test asserts the
    refusal text, the target name, and the `(root:` label rather than the
    caller root's full path; the full target and root text are pinned by the
    darkmatter library tests. Alternative: assert the root through a
    structured (JSON) output, if the Claudine route exposes one; Phase 3 can
    strengthen it.
  - |-
    An unmatched recursive (`%`) reference gets its own internal error
    variant with the message "recursive file reference `…` matched no file,
    and a search directory is never a new filename" for side-effect verbs;
    for `edit` it maps to the existing `EditError::NoLocalTarget` ("has no
    local target") rather than a new `EditError` variant. The plan said "the
    existing NoLocalTarget channel with a message naming the reference". A
    dedicated `EditError` variant would give `edit` the same wording at the
    cost of a public enum change.
  - |-
    A relative escape such as `../../outside/escape.md` is refused by the
    file-reference contract's own tree rule ("relative reference … leaves
    file tree …") before the boundary check runs, so its message differs
    from the "refusing to write outside the mutation root" one an absolute
    outside target gets. Both refuse before anything is created; the tests
    assert each message. Unifying them would mean catching the reference
    error and re-wording it, which the plan's R4 (keep the contract's
    errors, do not reimplement) argues against.
  - |-
    The new `md compose` refusal test checks the exact target but only a
    prefix of the root, because the lifecycle failure message (in the
    terminal summary and in the `--output json` diagnostic alike) is clamped
    to 240 characters, and macOS temp-dir paths push the target-plus-root
    text past that. With ordinary project paths the full root shows. The
    plan did not address this. Alternatives: give the data-output
    diagnostic the unclamped message (a product change to the shared
    diagnostic path, outside this fix), or shorten fixture paths (not
    possible for system temp dirs). If a reviewer wants the full root in
    every diagnostic, the change is in how `md` builds its lifecycle
    failure diagnostic, and the test would then assert the whole message.
  - |-
    Sniff's repository discovery was changed (outside the two areas this
    fix names) so a `.git` file that points at no repository is an error
    rather than skipped. Without it, `md compose` launched in a directory
    with a broken `.git` file inside a larger repository used that larger
    repository as the write boundary, which the spec forbids ("do not turn
    a discovery failure into permission to write under a guessed root").
    The rule matches git, which refuses such a directory. Alternatives: fix
    only the lifecycle hosts (they cannot see what gix skipped), or leave it
    and file it. Every sniff consumer now gets an error where it used to
    get the enclosing repository; sniff, darkmatter, claudine and worktree
    suites pass. Reverting means restoring `trusted_discover` in
    `sniff/lib/src/filesystem/git/open.rs` and dropping its two tests.
  - |-
    Claudine treats a failed launch-repository discovery as "no repository":
    lifecycle writes are then bounded by the launch directory and nothing is
    reported. The spec says the failure must be reported; the plan assumed
    `InvocationContext::capture` already propagates it, but it does not.
    This was not fixed in Phase 3 because the fix is a design choice: fail
    only lifecycle file actions at the seven engine constructions, or fail
    every Claudine run at capture. The current behavior never widens the
    boundary (a test pins that). Filed as the draft follow-up
    `2026-11-15-launch-discovery-failure-reporting`. If the reviewer wants
    it in this fix, the work is a shared helper plus a check at each of the
    seven hosts, and tightening that test to assert the reported error.
  - |-
    A referenced sequence or group now runs its event, child, and loop-gate
    stacks in a file-resolution context derived for its own document
    (keeping the launch scope), not the caller's. The spec says "a
    referenced sequence or group uses its own document" but the code passed
    the caller's context, which made every lifecycle write from a shared
    library's sequence fail as an unsuppressible evaluation error. If
    derivation fails, the caller's context is used as before. The
    alternative was to derive the context inside Darkmatter's resolver from
    `source_path` alone, which would lose Claudine's launch-scope and trust
    rules.
  - |-
    The plan's corrupt-repository fixture (a `.git` directory with a garbage
    `HEAD`) is not a repository at all to git, gix, or sniff, so it cannot
    test a discovery failure. The `md` test adds `objects/` and `refs/` so
    it is a repository with a bad `HEAD`, and the broken-gitfile case is
    tested separately. Claudine's version uses a corrupt `.git/config`.
message_to_agent: |-
  Phase 3 added the verification suites and fixed two production defects
  they found; Phase 4 (docs, skills, publish) should know:
  - Claudine: referenced sequences/groups now run their stacks in their own
    document's file-resolution context (`TaskExecution::authoring_context`
    in `claudine/lib/src/composition/sequence/task/mod.rs`), keeping the
    launch scope. The Claudine lifecycle docs should state that a referenced
    container's relative targets resolve beside its own file, and an inline
    group/task beside its containing document.
  - sniff: `trusted_discover` now fails on a `.git` file that points at no
    repository instead of skipping it (git parity). Already documented in
    `sniff/docs/sniff-library-architecture.md` and the sniff repo skill.
  - Claudine still treats a failed launch-repository discovery as "no
    repository" (narrows the boundary to the launch dir, reports nothing).
    Not fixed; filed as `2026-11-15-launch-discovery-failure-reporting` and
    listed in decisions_for_review. Docs must not claim Claudine reports it.
  - `md compose` stdin uses the launch directory as the authoring base;
    Claudine has no stdin or text-input route (`inline-compose` takes a
    file), so R9 for Claudine is covered by a document at the launch dir.
  - An env placeholder reaches a lifecycle operand as `"{{{NAME}}}/x.md"`
    (renders the literal `{{NAME}}`), worth a line in the lifecycle docs.
  - Host load from other sessions was very high (30-250); 19 passive
    Claudine schema tests measured ~5.0 s even alone. They are older and
    untouched by this fix; re-measure on a quiet host at the final gate.
source_files_during_phase_2:
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/tests/l1/compose_lifecycle.rs
  - darkmatter/cli/tests/l1/compose_edit_action.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/wrap/composition/pipeline.rs
  - claudine/cli/src/commands/wrap/composition/staged_boot.rs
  - claudine/cli/src/commands/wrap/sequence/mod.rs
  - claudine/cli/src/commands/wrap/sequence/task_run.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/tests/l1/lifecycle_mutation_root.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/lifecycle.md
source_files_during_phase_3:
  - darkmatter/lib/src/lifecycle/executor/tests.rs
  - darkmatter/lib/tests/l1/lifecycle_passive_compose.rs
  - darkmatter/cli/tests/l1/compose_lifecycle.rs
  - darkmatter/cli/tests/common/fixture.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/sequence/task/mod.rs
  - claudine/lib/src/composition/sequence/task/sequence.rs
  - claudine/lib/src/composition/sequence/task/group.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/common/mutation_paths.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/lifecycle_mutation_root.rs
  - claudine/cli/tests/l1/lifecycle_mutation_lifetime.rs
  - claudine/cli/tests/l1/compose_initialize_staged_boot.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/tests/l1/integration.rs
docs_updated_during_phase_3:
  - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_3:
  - claudine/fixes/2026-11-15-launch-discovery-failure-reporting/spec.md
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/sniff/remote-and-repository.md
---

# Implementation Log for 2026-11-14-lifecycle-mutation-root (4 phases)

## Phase 1

- Started Phase 1 (Darkmatter library: one resolver and one boundary check).
- **Resolver (R4/R5/R6)** — `darkmatter/lib/src/lifecycle/executor.rs`:
  `mutation_target` now resolves by contract (`resolve_in_context` on the
  document context), refuses an unmatched recursive reference
  (`MutationTargetError::UnmatchedRecursive`), otherwise takes the first
  `candidate_plan` entry for every kind (the implicit-relative
  `mutation_root().join(raw)` branch is deleted), then runs
  `fs_write::ensure_within` and returns its cleaned path
  (`MutationTargetError::Outside` → dispatch failure via `error_or_action`,
  or `EditError::Containment` for `edit`). The seven frontmatter verbs
  resolve operand 0 once through `path(0)` (`FRONTMATTER_VERBS`), pass that
  absolute target to the engine, and `mirror_frontmatter_mutation` receives
  the same resolved `&Path`. `targets_document` compares
  `PathIdentity` of `canonicalize_with_missing_tail` (made `pub(crate)` in
  `fs_write.rs`); `lexical_normalize` is deleted. Doc comments on
  `mutation_target`, `targets_document`, and the error enum were rewritten.
- **Engine read guard (R7)** — `EffectEngine::load` runs `ensure_within`
  before `read_to_string`; unit test
  `effects::verbs::tests::frontmatter_verbs_refuse_an_outside_target_before_reading_it`
  proves a missing outside file fails with `OutsideMutationRoot`, not `Io`.
- **Library tests** (`darkmatter/lib/src/lifecycle/executor/tests.rs`), all
  through `StackExecutor::execute_event` with the fake host and the
  filesystem as the observable result:

  | Requirement | Test |
  |---|---|
  | R10.1 retitled: missing bare relative created beside the document | `a_missing_bare_relative_target_is_created_beside_the_document` |
  | creation beside a subdirectory document, never root-joined | `a_missing_bare_relative_target_in_a_subdirectory_document_lands_beside_it_not_at_the_root` |
  | existing root-level `notes.md` is the second candidate; `./` pins the document dir | `an_existing_root_level_file_is_the_contracts_second_candidate_while_dot_slash_pins_the_document_dir` |
  | a created file is what later references find | `a_file_created_beside_the_document_is_found_there_by_the_next_reference` |
  | nested missing parents (implicit and `./`) | `a_nested_missing_target_creates_its_parents_beside_the_document` |
  | unmatched `%` refused for side effect and `edit`, nothing created | `an_unmatched_recursive_reference_is_refused_and_creates_nothing` |
  | URL refused as not local, nothing written | `a_url_target_is_refused_as_not_local_without_writing` (plus existing `a_url_is_refused_without_reaching_the_editor`) |
  | outside root refused naming target and root, no parent, no editor | `a_target_outside_the_root_is_refused_naming_target_and_root_before_any_parent_exists` |
  | in-root `../` succeeds; sibling text prefix refused | `an_in_root_parent_reference_and_a_sibling_text_prefix_are_judged_by_components` |
  | symlink escape refused (unix; Windows counterpart is Phase 3) | `a_symlink_inside_the_root_cannot_redirect_a_write_outside_it` |
  | frontmatter write to active doc visible to the next action | `a_frontmatter_write_to_the_active_document_is_visible_to_the_next_action` |
  | write to another file leaves live state alone | `a_frontmatter_write_to_another_file_leaves_the_active_documents_state_alone` |
  | mirror identity across equivalent spellings | `the_active_document_is_recognized_through_an_equivalent_spelling` |
  | every writer (7 frontmatter, both `ensure_file` forms, `ensure_dir`, both appends, `edit`) lands beside the document | `every_writer_acts_on_the_target_beside_the_document` |
  | every writer refuses outside, creates/writes nothing, no editor | `every_writer_refuses_an_outside_target_and_touches_nothing` |
  | guard: the table names every frontmatter verb | `the_writer_table_covers_every_frontmatter_verb` |
  | containment failure honors `no_error` | `a_refused_target_honors_no_error_and_the_stack_continues` |

  Fidelity: F1, in-process at the crate's public executor API; L1. No
  input-robustness matrix applies (no parser/loader changed). CLI and
  host-wiring end-to-end cases belong to Phases 2–3.
- **Dependent package (Claudine)** — the contract change alone broke 20
  Claudine tests that encoded root-joined creation or relative source
  paths. Fixed in this phase (test expectations only, no production code):
  the claudine lib executor tests now pass an absolute source inside the
  mutation root (`action_dispatch`, `mutation_visibility` — whose negative
  control now also asserts the write landed so it is not vacuous —
  `proxy_with_evaluation`); the task fixture test uses
  `mutating_its_directory()`; the CLI fixtures read markers beside the
  authoring document (`compose_initialize_acceptance`, `lifecycle_prep`,
  `compose_schema_cli`, `compose_caller_file_provenance`,
  `with_file_provenance`, `ctx_launch_anchor`), the foreign-prompt `edit`
  writes through `{{ ctx.cwd }}` (`lifecycle_edit_action`), and
  `lifecycle_mutation_root.rs`'s first test is rewritten per R10.2 to a
  refusal that creates nothing anywhere. The CLI-fixture sweep was delegated
  to one foreground subagent with scoped nextest runs.
- **Skill** — `.claude/skills/darkmatter/lifecycle.md`: the `mutation_target`
  sentence now states the contract-then-boundary selection. The host-root
  sentence (`md`'s root is the prompt directory) is still true until Phase 2
  and is left for Phase 2/4.
- **Gates** (macOS, run by the orchestrator with no subagent running):
  - `darkmatter/`: `just test` — 16748 passed, 0 failed, none at or above
    5.0 s; `just lint` — clean.
  - `claudine/`: `just test` — 12170 passed, 0 failed, none at or above
    5.0 s; `just lint` — clean.
  - No other package uses the lifecycle executor or `EffectEngine` (searched).
  - Darkmatter L2 edit-action suite not run: the plan schedules it for
    Phase 4, and Phase 1 changes no terminal behavior.
  - One mistaken run: a `cargo nextest --workspace` invocation briefly ran
    unrelated packages (biscuit-speaks "real" tests, one biscuit-file json5
    failure) before a 590 s timeout stopped it; those results are not
    evidence for this change and were superseded by the scoped `just test`
    runs above.
  - Cross-OS: the change reuses existing cross-platform helpers
    (`ensure_within`, `canonicalize_with_missing_tail`, `PathIdentity`) and
    adds no `#[cfg]` code beyond one `#[cfg(unix)]` symlink test; no
    cross-check run in this phase.
- **Index note** — during this phase something outside the session staged
  the entire working tree (76 files, including earlier uncommitted
  absent-equals-null work). This session staged nothing and unstaged
  nothing.


## Phase 2

- Started Phase 2 (Host wiring: the same boundary everywhere). The five
  tasks were small, separate edits, so they were done inline with no
  subagents.
- **Wiring** — every lifecycle `EffectEngine` construction now takes its
  root from the request's `FileResolutionContext::launch_magic_scope().local_root()`:
  - `md compose` (`darkmatter/cli/src/commands/compose/lifecycle.rs`):
    replaced the document-directory selection; the module docs now state
    the boundary.
  - Claudine compose loop route and staged single boot (`compose/prep.rs`,
    via `loop_prepare_options` / `options.file_resolution_context`); the
    `staged_boot.rs` comment now names the launch-scope root.
  - Claudine wrapper pipeline (`wrap/composition/pipeline.rs`), through
    `request.prepared.input_layers.file_resolution_context`.
  - Sequence container (`wrap/sequence/mod.rs`): was `source_context.base_dir()`.
  - Task stack and step guard (`wrap/sequence/task_run.rs`): were
    `task.origin_dir`. The comment that called the authoring directory the
    root now says it is the resolution base only.
  - Loop harness (`wrap/harness_orch/loop_control.rs`): was `run.child_cwd`.
  - The plan's checkpoint sweep (`rg "launch_workspace.child_cwd|source_context.base_dir\(\)|origin_dir|env_plan.child_cwd|source_path.*parent"`
    filtered to engine constructions) finds nothing. A search for every
    `EffectEngine::builder()` in the two areas' production code lists only
    these eight sites.
  - Failure reporting for capture errors is unchanged (R3): every site reads
    a scope that was already captured.
- **Behavior changes and their tests** (each new test was run against the
  old code and failed there, then passed with the fix):

  | Changed behavior | Boundary / tier | Test |
  |---|---|---|
  | `md`: a prompt in `prompts/` writes `../root-note.md` into the launch dir (was refused) | CLI process, F3, L1 | `compose_lifecycle::a_prompt_in_a_subdirectory_writes_to_the_launch_root_through_a_parent_reference` |
  | `md`: a prompt outside the launch scope cannot write beside itself (was allowed); exit 1, no document, nothing created; message names the exact target and starts the launch root | CLI process (`--output json`), F3, L1 | `compose_lifecycle::a_prompt_outside_the_launch_scope_cannot_write_beside_itself` |
  | `md`: launched from a repository subdirectory, the boundary is the working-tree top, not the launch dir or the prompt dir | CLI process + `git init`, F3, L1 | `compose_lifecycle::a_launch_from_a_repository_subdirectory_bounds_writes_by_the_working_tree_top` |
  | `md`: a sibling-library prompt editing a `{{ ctx.cwd }}` file now succeeds and opens that same file (the old test expected a refusal) | CLI process + fake editor, F3, L1 | `compose_edit_action::a_prompt_outside_the_launch_directory_edits_the_absolute_target_inside_its_root` (rewritten) |
  | Claudine sequence container `initialize` and a side-effect task in `sequences/` both write `../events.log` at the repository top (were refused) | CLI process, F3, L1 | `lifecycle_mutation_root::a_sequence_in_a_subdirectory_writes_to_the_launch_root_from_initialize_and_a_task` |
  | Claudine: a foreign-library sequence cannot write beside itself, from its container `initialize` or from a task alone (both were allowed) | CLI process, F3, L1 | `lifecycle_mutation_root::a_foreign_sequence_cannot_write_beside_itself_from_initialize_or_a_task` |

  The task route was also checked on its own: with only `task_run.rs`
  reverted, the task-only half of the foreign-sequence test failed.
  Unchanged in practice: Claudine's compose, staged, wrapper, and loop
  routes, where `child_cwd` already equaled the launch repository top. The
  existing `lifecycle_mutation_root` compose tests and the Claudine suite
  cover them. The step guard has no test of its own (it evaluates a
  condition with shell disabled). No input-robustness matrix applies: no
  parser or loader changed.
- **Findings, not changed (outside this phase)**:
  - The lifecycle failure message is clamped to 240 characters
    (`concise_message`), and that includes the `md compose --output json`
    diagnostic. With macOS temp-dir paths, the root in a containment
    refusal is cut off, so the test checks a prefix (decisions_for_review).
  - A sequence container event that fails with no `failure:` stack exits 1
    with no output. The foreign-sequence test adds a `failure:` stack that
    prints `err.msg` (the Claudine name; `failed:` is rejected as
    Darkmatter's name).
  - Running Claudine CLI tests without `--features test-fixtures` fails
    `document_environment_routes` (7) and the drain-budget test. The
    failures were the same with this phase's Claudine changes reverted,
    and `just test` passes the feature.
- **Skill** — `.claude/skills/darkmatter/lifecycle.md`: replaced the old
  host-root sentence ("`md`'s mutation root is the prompt's own directory")
  with the one rule for every host. The `docs/` pages and caller skills are
  Phase 4 work in the plan, and until then they still describe the old
  roots.
- **Gates** (macOS, nothing else running):
  - `darkmatter/`: `just test` gave 16751 passed, 0 failed, "none at or above
    5.0s". `just lint` was clean. `just check-tier-coverage darkmatter`
    showed 0 stranded.
  - `claudine/` (changed directly; it is also the only dependent of
    `darkmatter-cli`): `just test` gave 12172 passed, 0 failed, none at or
    above 5.0 s. `just lint` exited 0 (only the macOS linker's
    `__eh_frame` notice). `just check-tier-coverage claudine` showed 0
    stranded.
  - Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
    `packages` = `claudine-cli`, `darkmatter-cli`, `test-toolkit`,
    `darkmatter`; `change_class` = `package`. The gates were therefore
    required, and they ran.
  - Cross-OS (Windows, `just cross-check --os windows`): `darkmatter-cli
    compose_lifecycle` 54 passed, `darkmatter-cli compose_edit_action` 13
    passed, `claudine-cli lifecycle_mutation_root` 4 passed. A first
    attempt with an `-E 'test(...)'` filter failed on shell quoting inside
    the recipe and ran nothing; positional filters were used instead.

## Phase 3

- Started Phase 3 (verification suites). Three foreground subagents ran in
  parallel, each scoped to its own tests: Darkmatter library and CLI tests,
  Claudine caller/source and inline tests (`lifecycle_mutation_root.rs`),
  and Claudine lifetime/wiring tests (new `lifecycle_mutation_lifetime.rs`).
  The orchestrator then fixed what they found and ran the gates.
- **Requirement-to-test mapping** (all L1; library tests are F1 in-process
  through `StackExecutor::execute_event`, CLI tests are F3 binary spawns):
  | Spec Verification bullet | Test(s) |
  |---|---|
  | Relative targets: the spec's example table (library) | `lifecycle::executor::tests::the_specs_example_table_selects_each_target_by_the_file_reference_contract` |
  | Relative targets: the spec's example table (`md`, git repo) | `compose_lifecycle::the_specs_example_resolves_each_target_by_the_file_reference_contract` |
  | Relative targets: stdin | `compose_lifecycle::stdin_input_resolves_relative_targets_against_the_launch_directory` |
  | Relative targets: inline input | `lifecycle_mutation_root::inline_compose_creates_bare_and_nested_relative_targets_beside_a_document_at_the_launch_directory`; `lifecycle_mutation_lifetime::inline_compose_events_write_beside_the_document` (Unix) |
  | Reference semantics: prefixes inside the root, missing targets at the prefix's first candidate | `prefixed_targets_inside_the_root_resolve_by_their_prefix_and_write` |
  | Reference semantics: prefixes outside the root (`vault:`, `@` through home or a magic path, `~`) | `prefixed_targets_resolving_outside_the_root_are_refused_and_create_nothing` |
  | Reference semantics: `&`/`^` are not write grants | `repository_prefixes_are_not_write_grants_when_the_root_is_narrower_than_the_repository` |
  | Reference semantics: environment expansion to an absolute path | `an_environment_expanded_absolute_target_is_judged_as_that_absolute_path` (the operand `"{{{NAME}}}/x.md"` renders the literal `{{NAME}}/x.md`, which biscuit-file expands) |
  | Reference semantics: unmatched `%`, URL | Phase 1 tests (unchanged) |
  | Boundary enforcement: Windows link counterpart | `a_junction_or_directory_symlink_inside_the_root_cannot_redirect_a_write_outside_it` (`#[cfg(windows)]`; junction via `mklink /J`, plus `symlink_dir` that fails rather than passes without privilege) |
  | Boundary enforcement: `../`, sibling prefix, unix symlink | Phase 1 tests (unchanged) |
  | Boundary selection: outside a repository | `compose_lifecycle::outside_any_repository_the_launch_directory_is_the_root` |
  | Boundary selection: monorepo package | `compose_lifecycle::a_launch_from_a_monorepo_package_is_bounded_by_the_repository_top_not_the_package` (asserts `ctx.current_package` so it really is a package launch) |
  | Boundary selection: linked worktree | `compose_lifecycle::a_launch_inside_a_linked_worktree_is_bounded_by_that_worktree_never_the_main_checkout` |
  | Boundary selection: discovery failure reported | `compose_lifecycle::a_corrupt_repository_at_launch_fails_before_any_file_action`, `compose_lifecycle::a_broken_gitfile_at_launch_fails_rather_than_bounding_writes_by_the_enclosing_repository`, `sniff integration::test_a_broken_gitfile_is_a_discovery_failure_not_the_enclosing_repository`; Claudine: `lifecycle_mutation_root::a_failed_launch_discovery_never_widens_the_boundary_to_the_repository_top` (see the Claudine finding below) |
  | Caller and source differ | existing `lifecycle_mutation_root` compose tests (absolute in-root write succeeds, relative write beside the foreign prompt refused); new `outside_a_repository_the_launch_directory_bounds_a_prompt_from_inside_one` |
  | Lifetime: `prep`/`proxy` | `lifecycle_mutation_lifetime::prep_switches_the_base_to_the_target_and_back_while_the_boundary_stays`, `proxy_hops_move_the_base_with_the_active_document_but_never_the_boundary` |
  | Lifetime: retry and resume | `a_retried_attempt_…`, `a_resumed_attempt_…` (one macro) |
  | Lifetime: loop progression, loop events | `every_pass_and_gate_of_a_document_loop_writes_beside_the_document`, `every_pass_of_a_sequence_loop_keeps_each_documents_base` |
  | Host wiring: nested sequences/groups | `referenced_containers_write_beside_themselves_and_inline_ones_beside_their_container`, `a_foreign_referenced_sequence_is_bounded_by_the_callers_root` |
  | Lifetime: parallel members | `parallel_members_resolve_relative_targets_against_their_own_documents` |
  | Host wiring: staged init and the wrapper pipeline | `compose_staged_initialize_and_pipeline_events_write_beside_the_document`; root sequence and task: Phase 2 tests |
  | Passive surfaces | `lifecycle_passive_compose` (extended with `append_line`, `edit`, `set_frontmatter`) and `schema_validating_a_document_with_lifecycle_writers_dispatches_none_of_them` |
  Shared path-expectation helpers: `claudine/cli/tests/common/mutation_paths.rs`
  (`CallerTree`, `assert_written_beside`, `assert_refused_outside_root`, …)
  and `compose_lifecycle.rs`'s `assert_writes`. Darkmatter's CLI fixture gained
  `add_worktree`. Not covered: `kind: task` adapters, and the step guard (it
  evaluates a condition with shell disabled and cannot write). No input
  robustness matrix applies: no parser or loader changed.
- **Production bug found and fixed (Claudine)**: a referenced sequence or
  group's event, child, and loop-gate stacks switched `source_path` to the
  nested document but kept the caller's `file_resolution_context`. The
  Phase 1 resolver then failed with "repository root … does not contain the
  resolution source …" (as an evaluation error that `no_error` cannot
  suppress) for a nested document in a shared library, or reached through a
  symlinked spelling (macOS `/tmp`). Fix: `TaskExecution::authoring_context`
  (`claudine/lib/src/composition/sequence/task/mod.rs`) derives the
  document's own context via `InvocationContext::derive_source`, which keeps
  the launch scope, so the boundary is unchanged; `sequence.rs` and
  `group.rs` use it for their stacks, while their rereads keep the caller's
  context. It returns a `Cow` because Claudine's context-construction guard
  forbids `Option<FileResolutionContext>`, and the first full run failed
  that guard. Reverting the group half fails the nested test.
- **Production bug found and fixed (sniff, the widening case)**: gix's upward
  search skips a `.git` *file* that points at no repository and keeps
  walking, so `md compose` launched in `outer/inner` (with `inner/.git`
  reading `gitdir: /nonexistent`) used the enclosing repository `outer/` as
  the mutation root and wrote `outer/escaped.md`. Git itself refuses this
  directory. `sniff/lib/src/filesystem/git/open.rs::trusted_discover` now
  rejects a discovery that walked past a `.git` file below the repository it
  found (or found none), as a `SniffError::Git("discover", …)`. Both sides
  are canonicalized once per discovery. A `.git` *directory* that is not a
  repository is still skipped, as git skips it. Tests: the sniff integration
  test above (broken gitfile, garbage gitfile, valid separate-git-dir
  gitfile, invalid `.git` directory, no repository) and the `md` CLI test.
- **Finding not fixed (Claudine, the narrowing case)**: `InvocationContext`
  records a launch-repository discovery error (`RepositoryEntry::failed`)
  and continues, so the launch scope has no repository root and lifecycle
  writes are bounded by the launch directory. The plan's R3 assumed
  `capture` propagates the error; it does not. The fallback only narrows the
  boundary (the test above pins that it never widens), but the spec says a
  discovery failure must be reported. Recorded in `decisions_for_review`.
- **Plan fixture correction**: a `.git` directory holding only a garbage
  `HEAD` is "no repository" to git, gix, and sniff alike. The Darkmatter
  corrupt-repository test adds `objects/` and `refs/` so it is a repository
  with a bad `HEAD`; `md` then fails before any file action.
  The Claudine finding is filed as the draft follow-up
  `2026-11-15-launch-discovery-failure-reporting` (Claudine fixes).
- **Windows-only test defects fixed**: `cross-check --os windows` of the
  executor tests showed two older assertions that could not pass on Windows:
  `a_host_without_an_editor_fails_the_action_as_unsupported_and_no_error_continues`
  (the fake host labels a failure with its `Debug` form, which escapes
  backslashes) and `a_target_outside_the_root_is_refused_naming_target_and_root_before_any_parent_exists`
  (its expected path joined `deep/new/file.md`, leaving mixed separators).
  Both assertions now build the spelling the message uses. Production was
  correct.
- **Skills and docs**: `.claude/skills/darkmatter/lifecycle.md` (a host
  running a stack for another document must pass that document's own
  context); `.claude/skills/sniff/remote-and-repository.md` and
  `sniff/docs/sniff-library-architecture.md` (`GitRequest` section: a broken
  `.git` file is a discovery error, not the enclosing repository). The
  Darkmatter and Claudine lifecycle `docs/` rewrites remain Phase 4.
- **Gates** (macOS; host load from other sessions ran 30–250 throughout):
  - `darkmatter/`: `just test` gave 16764 passed, 0 failed, none at or above
    5.0 s (load ~11–17). After it, only `executor/tests.rs` changed (the two
    Windows assertions); its 52 tests pass. `just lint` was clean;
    `just check-tier-coverage darkmatter` showed 0 stranded.
  - `claudine/`: the first `just test` failed one test,
    `context_construction_guard::production_source_builds_contexts_only_through_the_builder`
    (the new `Option<FileResolutionContext>`), fixed with the `Cow`. The
    second `just test` gave 12186 passed, 0 failed, with 182 tests at
    5.0–9.2 s while the load average was 120–180; none reached 10 s. Rerun
    serially (`-j 1`): every `claudine-cli` slow test, including all the new
    ones, came in under 5 s. 19 `claudine` passive schema tests
    (`claudine_schemas*`, `claudine_schema_shipped_corpus`) stayed at about
    5 s; measured alone at load 250 they took 4.9–5.0 s. The local timing
    history already records them as slow on earlier commits (`200a476e`),
    and they exercise no code this phase changed, so they are recorded as
    load-sensitive, not cleared at a quiet 5 s. `just lint` was clean;
    0 stranded.
  - `sniff/` (changed directly): `just test` gave 3594 passed, none slow;
    `just lint` was clean; 0 stranded.
  - Dependent `worktree` (most broken-gitfile fixtures in the repository):
    `just test` gave 1506 passed. A first run failed
    `live_remote::tests::the_deadline_kills_the_http_transport_and_closes_its_connection`
    at load 98, and it passed alone in 0.63 s (load-sensitive, unrelated).
    Its `git_graph` slow tests (12–15 s in the package run) were timed with
    the sniff change reverted and restored: 3.1–4.2 s either way, and they
    were recorded at up to 19.8 s on earlier commits, so they are existing
    and unaffected. Darkmatter's own gitfile fixture
    (`git_context_integration`) passed in `just test`.
  - Cross-OS (Windows, `just cross-check`): `sniff broken_gitfile` 1 passed;
    `darkmatter lifecycle::executor::tests` failed 2 on the first attempt
    (fixed above), then 52 passed, including the new junction test;
    `darkmatter-cli compose_lifecycle` 61 passed;
    `claudine-cli lifecycle_mutation` 17 passed (the inline-compose
    lifetime test is Unix-only).
  - Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
    `packages` = `claudine`, `claudine-cli`, `darkmatter`, `darkmatter-cli`,
    `sniff`, `test-toolkit`; `change_class` = `package`. The gates were
    required, and they ran.
