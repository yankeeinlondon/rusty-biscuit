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
message_to_agent: |-
  Phase 1 landed the library contract and also fixed every Claudine test that
  the contract change alone broke (see decisions_for_review). Both areas' full
  L1 suites are green at the end of Phase 1 (darkmatter 16748, claudine 12170)
  and both lint clean. Things to know for Phase 2:
  - `mutation_target` no longer takes the raw text; it returns a lexically
    clean path that has already passed `ensure_within(mutation_root, …)`.
    Frontmatter verbs now hand the engine that absolute path.
  - The darkmatter executor test module has a `Workspace` fixture
    (`<tmp>/ws/prompts/doc.md`, root `<tmp>/ws`, `<tmp>/outside`) and a
    13-row `WRITERS` table; Phase 3's contract/reference tests can extend it.
  - In the Claudine CLI fixtures `child_cwd` already equals the launch
    repository top (`package_context.rs` uses `detect_repo_root(launch_cwd)`),
    so the Phase 2 rewiring should not flip those fixtures; the sequence route
    (`base_dir()`) and task route (`origin_dir`) are where behavior can change.
  - Something outside this session staged the whole working tree (including
    earlier uncommitted absent-equals-null work) during Phase 1. Nothing was
    unstaged; do not assume the index reflects only one phase.
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

