---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-edit-action/spec.md"
plan: "darkmatter/features/2026-11-14-edit-action/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - sniff/lib/src/os/display.rs
  - sniff/lib/src/os/mod.rs
  - sniff/lib/src/programs/enums/editor_traits.rs
  - sniff/lib/src/programs/enums/mod.rs
  - sniff/lib/src/programs/mod.rs
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/src/lib.rs
  - tools/test-toolkit/src/fake_editor.rs
  - tools/test-toolkit/src/bin/test-toolkit-fake-editor.rs
  - tools/test-toolkit/tests/fake_editor.rs
  - darkmatter/cli/Cargo.toml
  - darkmatter/cli/tests/bin/fake_editor/main.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/tests/bin/fake_editor/main.rs
  - Cargo.lock
docs_updated_during_phase_1:
  - sniff/docs/sniff-library-architecture.md
  - tools/test-toolkit/README.md
  - docs/dependencies.md
  - darkmatter/docs/dependencies.md
  - claudine/docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/programs.md
  - .claude/skills/darkmatter/testing.md
packages:
  - sniff
  - test-toolkit
  - darkmatter-cli
  - claudine-cli
  - darkmatter
  - claudine
  - darkmatter-lsp
decisions_for_review:
  - |-
    `Editor::identify` treats its whole argument as one program path and compares only its final component; it does not split on whitespace. The plan (R7) said it "takes the first whitespace-separated token's path", but also asked for a test with "a path with spaces in the directory", which cannot be recognized if the input is split on spaces. Splitting the `$EDITOR` command line is left to the caller (the Phase 2 resolver, which already takes the first whitespace token). Alternative: split inside `identify`, which makes `identify("code --wait")` work but makes `/opt/My Editors/bin/subl` unrecognizable. Redoing it means changing one line in `identify` and two test assertions.
  - |-
    The test toolkit got its own copy of the fake editor binary (`test-toolkit-fake-editor`, ungated) so its tests can launch the real program, plus a Unix-only `libc` dependency (already in the graph through `biscuit-test-harness`) for ignoring `SIGTERM`. The plan named only the Darkmatter and Claudine binaries. Alternative: test the body in-process only, which cannot prove the spawn-child or ignore-SIGTERM behaviors. Reversing means deleting one `[[bin]]` and the integration test file.
  - |-
    Fake editor details the plan left open: `FAKE_EDITOR_REPLACE` names a content file (like `FAKE_EDITOR_WRITE`) that is saved through a temporary file and a rename; `FAKE_EDITOR_READ_LINE` runs before `FAKE_EDITOR_STDIO` so the report can show the line read; the stdio report is a small JSON file read back with `read_stdio_report`; a held or spawned process exits on its own after 120 seconds so a failed test cannot leak it; a spawned child reports readiness over a private pipe before its PID is written, so a test never races its `SIGTERM` setup. Each is a local change in `tools/test-toolkit/src/fake_editor.rs` if a reviewer prefers otherwise.
  - |-
    On Windows the editor's Job Object is created with kill-on-close, as planned, but a successful edit clears that limit before closing the Job instead of letting the close kill whatever is left. The plan said "Job object (kill-on-close)" without saying what happens after a normal exit. With plain kill-on-close, closing a file in VS Code would also kill a VS Code window that its waiting client had started, because that window is in the Job. Alternatives: no kill-on-close at all (simpler, but an editor survives if the host crashes mid-edit), or keep plain kill-on-close (kills a GUI editor's window after every successful edit). Reversing it means removing `ProcessTree::release` in `darkmatter/lib/src/process_tree.rs` and its one call in `editor/launch.rs`.
  - |-
    Resolution's order and filtering are unit-tested in-process over a private core (`resolve_editor_from`) that takes the two variables, a program lookup, and a probe lookup as inputs, and the CLI tests cover only cases where a variable decides the outcome. The plan asked for environment-variable tests plus portable stubs; tests that reach the built-in probe list through the real environment would depend on which editors the test host has installed, so they cannot be deterministic. Alternative: control `PATH` in the CLI tests so the probes see only installed fakes (much more fixture code, and `PATH` lookups differ per OS). Reversing means adding such a fixture; the core stays either way.
  - |-
    The plan asked for a "test hook creating the file between the existence check and the call" to prove a concurrently created file is opened, not replaced. The service has no separate existence check: it makes one exclusive create, so the race window the hook would target does not exist. The evidence instead is a two-thread race on the exclusive create (exactly one creator wins) plus a service test where the name is already held by another writer's bytes (opened unchanged; refused under `create_new`). Alternative: add a test-only seam inside `create_new_guarded`, which would test a seam rather than a real window. Nothing to redo unless a reviewer wants the seam.
  - |-
    The fake editor gained one variable, `FAKE_EDITOR_PAUSE` (create a marker, wait until the test removes it, then exit normally), which the plan did not list. Without it no test can change the target while the editor is "open" and still see a normal exit, so the post-exit directory and containment checks could not be tested. Alternative: leave those two checks untested. Reversing means deleting the variable from `tools/test-toolkit/src/fake_editor.rs` and the one test that uses it.
  - |-
    The plan's `EditError` list had no cancellation variant, so a cancellation (before the editor starts or while it is open) is reported as `EditError::Editor(EditorError::Cancelled { path })`, with `EditError::is_cancelled()` for callers. A file deleted by the editor is `EditError::Editor(EditorError::Missing)`, the existing "edited file was deleted" error, rather than a new variant. Alternative: dedicated `EditError::Cancelled` and `EditError::Deleted` variants (clearer matching, two more names). Reversing touches `editor/service.rs` and the matching assertions in `cli/tests/l1/editor_service.rs`; Phase 4 maps these to `ActionFailure`, so decide before then.
  - |-
    `edit`'s literal-boolean errors are located at the flag's own key (`initialize[0].else[0].edit.gui`), and a long-form `file` that is a number, a boolean, or blank at `…edit.file`; the short form's operand error stays at `…edit`. The plan asked for exactly this location, but the nearest precedent, `break.code`, reports at the action (`…break`) and names the key only in the message. Alternative: report every `edit` error at the action for consistency with `break` (coarser DMLS squiggles). Reversing means changing two `property:` expressions (`parse.rs` flag check, `action_shape.rs` long-form `edit_file` call) and the expected paths in `darkmatter/lib/tests/l1/lifecycle_edit_action.rs`.
  - |-
    Two schema-agreement tests compare the parser's verbs with the shipped schemas, which do not type `edit` until the schema work later in the plan: `darkmatter/lib/tests/l1/lifecycle_schema_parity.rs::the_open_arm_rejects_every_md_verb_and_admits_every_unprovided_verb` and `claudine/lib/tests/l1/claudine_schemas.rs::hand_verbs_equal_the_parser_inventory`. Each now exempts `LIFECYCLE_ONLY_VERBS` and also asserts the verb is still untyped, so the test fails (with a message saying to drop the exemption) as soon as a schema types `edit`; the exemption cannot outlive the gap. The plan expected these checks to be updated now without saying how. Alternative: write the `edit` schema types now and regenerate the shipped artifacts, moving part of the schema phase forward. Until the schemas land, `md schema validate` and DMLS accept any `edit` value through the md open arm and reject it in Claudine's closed action types.
  - |-
    The schema projection's collision check now treats a lifecycle-only verb like a control verb: a catalog function or effect named `edit` fails generation ("a lifecycle-only verb"). The plan only asked for a test that no descriptor is named `edit`; this makes the generator refuse it too. Reversing means removing one branch in `schema_projection.rs::hand_owner` and one row of `lifecycle_schema_generation.rs`.
  - |-
    On a failed `edit`, `md compose` prints "`<property>` could not edit the file: <reason>", adding " (<target>)" only when the reason does not already name the target. The plan (R19) said "`<property>` could not edit <target>: <reason>". Every lifecycle failure message is clamped to one 240-character line, and most reasons name the path again (some also name the editor by full path), so with the plan's order the reason itself was routinely cut off, in the terminal line and in the JSON diagnostic. Alternatives: keep the plan's order and accept truncated reasons; or raise or bypass the shared clamp for `edit` (affects text-to-speech and webhook consumers of the same messages). Reversing means changing one arm of `failure_message` in `darkmatter/lib/src/lifecycle/md_host.rs` and the expected prefix in `darkmatter/cli/tests/l1/compose_edit_action.rs`.
  - |-
    Under `md compose`, the "prompt stored outside the launch directory" acceptance case is tested with the prompt in a directory above the launch directory, and a prompt in a sibling library directory is pinned as refused. `md compose` confines writes to the prompt's own directory (Claudine confines them to the launch workspace), so a sibling prompt editing `{{ ctx.cwd }}/…` fails containment before anything is created. The spec's example targets Claudine's prompt libraries; it did not say which root `md` should use. Alternative: change `md compose`'s mutation root to the launch directory, which also changes where `ensure_file` and the other file side effects may write for every `md` document. Reversing means changing the root in `ComposeHost::new` (`darkmatter/cli/src/commands/compose/lifecycle.rs`) and flipping the sibling assertion in `a_prompt_outside_the_launch_directory_edits_the_absolute_target_inside_its_root`.
  - |-
    Until Claudine's host wiring lands (the next phase), Claudine reports an `edit` failure as a plain `edit` action failure with the same "could not edit" text, not as a typed `CompositionError`. Claudine's failure match is exhaustive, so the new failure kind needed an arm to compile, and its host supplies no editor yet, so the only failure it can see is "this host cannot run `edit`". Alternative: add `CompositionError::LifecycleEditFailed` now, which is the next phase's listed task. Nothing to redo: the next phase replaces the arm in `claudine/lib/src/composition/lifecycle/executor.rs::classify_run_failure`.
  - |-
    `EditError` gained a `NoLocalTarget { reference }` variant for a reference that resolves to no local path a file could be created at (the side-effect resolver's existing "has no local mutation target" case). The plan's list had no variant for it, and flattening it into a string would have dropped the typed cause the spec asks to keep. Alternative: reuse `Reference(FileReferenceError)` with a synthesized error, which misstates the cause. Reversing touches `editor/service.rs` and `executor.rs::resolve_edit_target`.
  - |-
    A cancellation noticed before the editor starts is reported as "the run was cancelled, so `edit` did not open an editor", distinct from "the run was cancelled while an `edit` was open" (the plan's wording, used when the editor was open). Both are the same unsuppressible failure kind. Alternative: one message for both, which hides whether the editor ever started. Reversing is a one-line change in `executor.rs::dispatch_edit` and its unit test.
  - |-
    The race acceptance case ("a test hook creates the target between the existence check and the edit") is driven by an authored `ensure_file` placed between the `when: file_exists(fqn)` check and the `edit` in `initialize`, rather than a test-only seam in the executor or service. It reproduces the window exactly as a concurrent writer would, through the normal invocation path. Alternative: a test-only hook in the service, which tests a seam instead of real behavior. Nothing to redo unless a reviewer wants the seam.
  - |-
    `darkmatter/justfile`'s `test-l2` now builds the CLI with `terminal-tests,test-fixtures`, matching the features CI already enables for the L2 cell, so the L2 `edit` test can launch the fake editor binary. Alternative: gate the L2 `edit` tests behind `test-fixtures` (they would silently not run locally), or make the `level2` test target require `test-fixtures` (changes every L2 test's build contract). Reversing is one line in the justfile.
  - |-
    The seven Claudine process tests that need `edit` outside a skipping `initialize` (start/success/finalize ordering, container stacks, a prompt task, both parallel-member cases, the sibling-failure cancellation) are written now but wrapped in `pending_until_the_schema_knows_edit`: each passes only while its body fails on Claudine's schema refusal of `edit`, and fails, asking for the wrapper's removal, once the schemas accept it. The plan put these tests in this phase and the Claudine schema entries in the next, but Claudine validates every document against those schemas before `start` and before any sequence task runs, so the tests cannot pass until then. Each body was proven against a temporary schema entry that was then removed. Alternative: move Claudine's `edit` schema entries (and the shipped-artifact regeneration they require) forward into this phase. Redoing: delete the wrapper calls in `claudine/cli/tests/l1/lifecycle_edit_action.rs` when the schemas land (the tests force this).
  - |-
    No timeout cancels an open editor in Claudine, so the spec's example (`timeout: 30m` on a document whose `initialize` edits) does not stop the edit. A document's `timeout:` and `--timeout` bound only the provider session and a task's `timeout:` only its shell commands; no deadline covers lifecycle stacks. I did not add one: making the run timeout cover lifecycle events changes what every Claudine timeout means (when the clock starts, whether `shell` actions and `initialize` count), which is a design choice beyond this feature. Cancellation is proven with an interrupt and a fail-fast sibling. Alternatives: start a watchdog at run start that sets a cancellation cause when the document `timeout:` expires during an event (covers `edit` and every other action), or a per-action deadline for `edit` only. Redoing: a new deadline-to-cancellation bridge in Claudine's run guard plus the plan's timeout test.
  - |-
    A sequence whose root `initialize` fails exits with status 1 and prints nothing, for every action (`edit`, `ensure_file`, `prep`), so an `edit` failure there is silent. This predates the feature: `claudine/cli/src/commands/wrap/sequence/mod.rs`'s `Finished { failed }` arm returns the code without rendering the recorded error. I left it unfixed because how a root-container failure should render (status block, warning line, or a returned typed error) is a choice the sequence owners should make, and it affects every action. Alternative: render `root.runner.primary_error()` there as a warning line. Redoing: one arm plus a CLI test.
  - |-
    `composition.lifecycle_edit_failed` is cataloged with origin `environment` and disposition `correctable`: most causes are the host (no editor, no terminal or display, an editor that failed or cannot wait), though a refused target (outside the workspace, a URL, `create_new` on an existing file) is the author's. The typed cause is stored as `Box<EditError>`, not the plan's `Arc`, because Claudine's errors are not cloned. Alternative: origin `author`, or split the code by cause. Redoing: one registry field (and the `err.origin` a lifecycle handler sees).
  - |-
    Two plan items were proved with the closest current grammar: task `setup:`/`teardown:` and `execution: serial` groups were removed from Claudine's grammar and every group is concurrent, so the serial-stack ordering test uses an inline `sequence:` container's `start`/`finalize` stacks around its children. And "structured output keeps one valid result" has no Claudine counterpart: `--output json` sets the provider's output format and Claudine prints no envelope for an `initialize` failure, so the test asserts stdout stays empty and the refusal reaches stderr. Nothing to redo unless a reviewer wants a Claudine-level JSON result for lifecycle failures.
  - |-
    Schema validation now applies the lifecycle parser's own rules to `edit`'s target and flags (`BoundOperands::report_refused_targets`, using `edit_file_problem` and `edit_flag_problem`, which the parser now calls too). The plan expected the schema types alone to agree with the parser, but the shared conversion engine reads `edit: 5` and `edit: [1]` as text and `gui: "yes"` as `true`, so `md schema validate` and DMLS reported documents valid that `md compose` refuses ("`edit.file` must be a string, got number"). Alternatives: accept the disagreement and document it; add a schema-language marker that turns conversion off for one value (a change to the simplified-schema crate and every consumer of conversion); or apply the parser to every hand-written verb (wider behavior change: `say: 5` and similar are accepted today). Reversing means removing the `edit` branch and `report_refused_targets` in `darkmatter/lib/src/markdown/schemas/bound_operands.rs`, its one call in `schemas/mod.rs`, and the number, boolean, and quoted-flag rows of `lib/tests/l1/lifecycle_edit_schema_parity.rs`.
  - |-
    `edit`'s schema arms and its `create_new`/`gui` flags are marked `non-null`, which plan rule R24 did not list. Without it an optional property accepts `null`, so the schema admitted `edit: null`, `gui: null`, and `create_new: null`, all of which the parser refuses (`null` must not read as the absent default). Alternative: leave `null` admitted by the schema and rely on the parser. Reversing means deleting the three `non-null` markers in `darkmatter/schemas/partials/lifecycle.yaml`, regenerating the shipped artifacts and goldens, and dropping the null rows of the parity test.
  - |-
    `edit` is typed in every Claudine place that accepts an expression action, which is more than the sites the plan listed (beside `prep` and `ensure_file`): also `cancelled-bundle` and both loop arms (`loop-bundle`, `loop-single`). Claudine's parser accepts `edit` in all of them, and its schemas promise to hold exactly the parser's verbs. In `cancelled` (and after any cancellation) the action is refused at run time, so typing it there only lets an author write it. Alternative: leave it out of `cancelled-bundle`, making the schema refuse what the parser accepts. Reversing is one line in `claudine/schemas/action.yaml` plus regeneration.
  - |-
    `edit-long` carries `operands(edit-long)` like `edit-short`, although R24's prose said the types carry no `operands(...)` marker. The marker is how schema validation finds a parser-owned verb's value at all (malformed `{{ … }}` spans and the parser rules above); R24's own `edit-short` definition already had it. Nothing to redo unless a reviewer wants templates in `edit`'s long form left unscanned.
  - |-
    In DMLS, a quoted `gui: "yes"` shows two diagnostics: the parser's located one at `gui` and the schema's union summary ("not valid under any of the schemas") on the enclosing `edit`, because the editor's passive validation does not convert the value first. `md schema validate` shows only the located one. I left the summary: dropping a union failure at the action could hide an unrelated problem in the same action (an unknown key beside the bad flag). Alternative: suppress the action-level union summary whenever a parser rule located a problem inside it. Redoing is a filter in `report_refused_targets` and a tighter assertion in `darkmatter/dmls/tests/l1/edit_action_editor.rs`.
message_to_agent: |-
  Phase 6 landed the schemas and the parity tests. What Phase 7 should know:
  - Schema descriptions an author sees on hover: `edit-short` "open this file in the editor and wait for it to close; a missing file is created"; `edit-long` adds that `create_new` refuses an existing file and `gui` chooses a graphical (true) or terminal (false) editor. Keep docs consistent with them (`darkmatter/schemas/partials/lifecycle.yaml`).
  - Schema validation (`md schema validate`, DMLS) now refuses exactly what the parser refuses for `edit`, including `edit: 5`, `gui: "yes"`, and `null` flags, through `edit_file_problem`/`edit_flag_problem` in `darkmatter/lib/src/lifecycle/action_shape.rs` (see the decisions list). The darkmatter skill's `schema.md` already describes this; `lifecycle.md` and the user docs do not yet.
  - Claudine types `edit` in its expression category, all three bundles (including `cancelled-bundle`), and both loop arms.
  - The Claudine `pending_until_the_schema_knows_edit` wrapper and both Phase 3 exemptions are gone; Claudine's `bundle_values_are_the_category_short_forms` now counts 21 hand long forms.
  - Disk: this worktree's `target/debug/incremental.kache-auto` grew to 64 GB and filled the volume mid-phase (`scripts/sweep.sh`'s incremental pass does not match that directory name); I deleted it. Check `df -h /Volumes/coding` before the closing full gates.
source_files_during_phase_2:
  - darkmatter/lib/src/process_tree.rs
  - darkmatter/lib/src/lib.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
  - darkmatter/lib/src/effects/mod.rs
  - darkmatter/lib/src/effects/fs_write.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/editor/resolve.rs
  - darkmatter/lib/src/editor/launch.rs
  - darkmatter/lib/src/editor/surface.rs
  - darkmatter/lib/src/editor/service.rs
  - darkmatter/lib/tests/l1/context_construction_guard.rs
  - darkmatter/lib/tests/l1/error_snapshots/editor.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/editor_support.rs
  - darkmatter/cli/tests/l1/editor_resolution.rs
  - darkmatter/cli/tests/l1/editor_launcher.rs
  - darkmatter/cli/tests/l1/editor_service.rs
  - tools/test-toolkit/src/fake_editor.rs
  - tools/test-toolkit/tests/fake_editor.rs
docs_updated_during_phase_2:
  - darkmatter/docs/cli/edit.md
  - tools/test-toolkit/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/testing.md
  - .claude/skills/darkmatter/library-surfaces.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_3:
  - darkmatter/lib/src/lifecycle/actions.rs
  - darkmatter/lib/src/lifecycle/signatures.rs
  - darkmatter/lib/src/lifecycle/mod.rs
  - darkmatter/lib/src/lifecycle/action_shape.rs
  - darkmatter/lib/src/lifecycle/parse.rs
  - darkmatter/lib/src/lifecycle/validate.rs
  - darkmatter/lib/src/lifecycle/operands.rs
  - darkmatter/lib/src/lifecycle/source_map.rs
  - darkmatter/lib/src/lifecycle/schema_projection.rs
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/tests/l1/lifecycle_edit_action.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
  - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
  - claudine/lib/src/composition/sequence/task/mod.rs
  - claudine/lib/tests/l1/claudine_schemas.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/lifecycle.md
source_files_during_phase_4:
  - claudine/lib/src/composition/lifecycle/executor.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/compose_edit_action.rs
  - darkmatter/cli/tests/level2/main.rs
  - darkmatter/cli/tests/level2/level2_edit_action.rs
  - darkmatter/dmls/tests/l1/no_side_effects.rs
  - darkmatter/justfile
  - darkmatter/lib/src/editor/service.rs
  - darkmatter/lib/src/lifecycle/cancellation/tests.rs
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/src/lifecycle/executor/host.rs
  - darkmatter/lib/src/lifecycle/executor/tests.rs
  - darkmatter/lib/src/lifecycle/md_host.rs
  - darkmatter/lib/src/lifecycle/md_host/tests.rs
  - darkmatter/lib/tests/l1/lifecycle_event_binding.rs
  - darkmatter/lib/tests/l1/load_data_runtime.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/darkmatter/testing.md
source_files_during_phase_5:
  - claudine/lib/src/composition/lifecycle/editor.rs
  - claudine/lib/src/composition/lifecycle/editor/tests.rs
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/diagnostics/registry.rs
  - claudine/lib/src/composition/sequence/task/group.rs
  - claudine/lib/src/composition/sequence/task/tests.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/tests/l1/lifecycle_edit_action.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/l1/error_guards.rs
  - claudine/cli/tests/level2/level2_edit_action.rs
  - claudine/cli/tests/level2/main.rs
  - claudine/justfile
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/architecture.md
  - .claude/skills/darkmatter/lifecycle.md
source_files_during_phase_6:
  - darkmatter/schemas/partials/lifecycle.yaml
  - darkmatter/schemas/partials/md-lifecycle.yaml
  - claudine/schemas/action.yaml
  - claudine/schemas/lifecycle.yaml
  - darkmatter/lib/generated/shipped.json
  - claudine/lib/generated/shipped.json
  - darkmatter/lib/src/lifecycle/action_shape.rs
  - darkmatter/lib/src/lifecycle/mod.rs
  - darkmatter/lib/src/lifecycle/parse.rs
  - darkmatter/lib/src/markdown/schemas/bound_operands.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs
  - darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json
  - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json
  - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
  - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
  - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
  - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
  - darkmatter/cli/tests/l1/compose_edit_action.rs
  - darkmatter/dmls/tests/l1/edit_action_editor.rs
  - darkmatter/dmls/tests/l1/main.rs
  - claudine/lib/tests/l1/claudine_schemas.rs
  - claudine/cli/tests/l1/lifecycle_edit_action.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/schema.md
source_files_during_phase_7:
  - darkmatter/lib/src/editor/launch.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/editor/resolve.rs
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/src/process_tree.rs
  - tools/test-toolkit/src/fake_editor.rs
docs_updated_during_phase_7:
  - darkmatter/README.md
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/cli/edit.md
  - darkmatter/docs/topics/caching.md
  - darkmatter/docs/topics/lifecycle/index.md
  - darkmatter/docs/topics/lifecycle/error-handling.md
  - darkmatter/docs/topics/loading-data.md
  - darkmatter/docs/topics/side-effects.md
  - claudine/docs/getting-started/index.md
  - claudine/docs/topics/flow-control/lifecycle.md
  - claudine/docs/topics/flow-control/sequences.md
  - claudine/docs/topics/schemas.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
  - ~/.claude/skills/use-darkmatter/authoring.md (user scope, outside the repository)
  - ~/.claude/skills/use-claudine/lifecycle.md (user scope, outside the repository)
---

# Implementation Log for 2026-11-14-edit-action (7 phases)

## Phase 1

### What was built

- **Sniff editor facts** (R5–R7): `sniff/lib/src/programs/enums/editor_traits.rs` adds
  `EditorSurface` (`terminal`/`gui`/`both`, serde and `Display` snake_case),
  `EditorTraits`, and the `EDITOR_TRAITS` table indexed like `EDITOR_INFO`;
  `Editor::{traits, surface, wait_args, can_block, identify}`. Re-exported from
  `sniff::programs`. `ProgramInfo` and every serialized program shape are unchanged
  (`program_serialization` tests pass unchanged within the full sniff run).
- **Sniff display session** (R8): `sniff/lib/src/os/display.rs` with `DisplaySession`,
  `detect_display_session()`, and the pure `display_session_from(os, runtime, var)`.
  Not in `OsInfo` or any cache.
- **Fake editor** (R21): `tools/test-toolkit/src/fake_editor.rs` (`main`,
  `read_stdio_report`, `read_pids`, the variable-name constants, `SLEEP_CEILING`), thin
  bins `darkmatter-fake-editor` (darkmatter-cli) and `claudine-fake-editor`
  (claudine-cli), both `required-features = ["test-fixtures"]`, plus the toolkit's own
  `test-toolkit-fake-editor`. `rg darkmatter-argv-recorder` found no justfile, CI
  manifest, or archive list naming the recorder, so only Cargo manifests and the
  dependency docs needed the new names.
- **Docs and skills**: a new "Editor Launch Facts and the Display Session" section in
  `sniff/docs/sniff-library-architecture.md` (the existing page `rg -l Editor sniff/docs`
  found), a new item in the toolkit README, dependency notes, Sniff skill
  (`SKILL.md`, `programs.md`), and a "Launching the user's editor" section in the
  darkmatter skill's `testing.md`. Darkmatter's own `wait_args_for_editor` table is still
  present; Phase 2 removes it, so the docs say only that Sniff is the source of these
  facts.

### Requirement-to-test mapping

All are L1, F1 unless noted (pure in-process logic at Sniff's public API; the fake
editor crosses a process boundary, so its tests spawn the real binary: F3 at L1).

| Requirement | Test(s) |
|---|---|
| R5 table covers every variant | `sniff programs::enums::editor_traits::tests::traits_table_covers_every_editor` |
| R6 surface + wait args per variant (wildcard-free match) | `...::every_editor_has_its_documented_surface_and_wait_args` |
| R5/R6 `can_block` | `...::non_blocking_gui_editors_cannot_block`, `...::terminal_dual_and_waiting_gui_editors_can_block` |
| R7 identify by path, spaces in directory | `...::identify_reads_the_final_path_component` |
| R7 launcher-extension stripping only (`code.cmd`, `nvim.appimage`) | `...::identify_strips_windows_launcher_extensions_only` |
| R7 `code-insiders`, `helix` | `...::identify_uses_extra_identifying_names` |
| R7 case by platform (`Code.exe`, `NVIM`) | `...::identify_compares_case_by_platform` (both branches proved: macOS here, Windows via cross-check) |
| R7 unknown wrapper → `None` | `...::identify_does_not_recognize_unknown_commands` |
| `EditorSurface` wire form | `...::editor_surface_serializes_in_snake_case` |
| R8 macOS/Windows, Linux X11/Wayland/empty/neither/mixed, WSL1/WSL2 with and without vars, other OS | `sniff os::display::tests::*` (9 tests) |
| R8 `detect_display_session` does not panic | `os::display::tests::detecting_the_process_display_session_does_not_panic` |
| R21 no variables = no-op | `test-toolkit::fake_editor no_variables_exits_zero_and_leaves_the_target_untouched` |
| R21 RECORD (argv_recorder format, repeated runs) | `record_appends_one_argv_record_per_run` |
| R21 WRITE / REPLACE / DELETE / EXIT | `write_copies_content_into_the_target_in_place`, `replace_saves_through_a_rename_and_leaves_no_temporary`, `delete_removes_the_target`, `exit_sets_the_exit_code` |
| R21 fixture failure is loud (exit 2) | `an_unreadable_content_file_fails_loudly_with_exit_two` |
| R21 OUT_TEXT / ERR_TEXT | `out_and_err_text_reach_their_streams` |
| R21 STDIO (non-terminal streams, piped stdin) and report round trip | `stdio_reports_non_terminal_streams_and_piped_input`, `fake_editor::tests::stdio_reports_round_trip`, `a_malformed_stdio_report_is_invalid_data` |
| R21 READ_LINE | `read_line_writes_one_stdin_line_into_the_target` |
| R21 HOLD | `hold_creates_its_marker_and_stays_running_until_killed` |
| R21 SPAWN_CHILD + PIDS | `spawn_child_records_a_live_child_pid` |
| R21 IGNORE_TERM (Unix), editor and child | `ignore_term_keeps_a_held_editor_running_after_sigterm`, `ignore_term_reaches_a_spawned_child` |

The terminal-dependent STDIO cases (a real TTY) are not testable without a terminal and
are left to the Phase 2/4 L2 tests that use the fixture, per the plan.

The input-robustness matrix does not apply to Phase 1: nothing here reads a file
format or configuration (the fake editor reads only environment variables set by
tests).

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine-cli, darkmatter-cli, sniff, test-toolkit, claudine, darkmatter]`,
  `change_class: package`, so the gates were run.
- `sniff`: `just test` 3590 passed, none ≥ 5 s; `just lint` clean.
- `darkmatter`: `just test` 16483 passed, 10 skipped, none ≥ 5 s; `just lint` clean.
- `test-toolkit` has no `test`/`lint` recipes in its own justfile, so the root
  `just test test-toolkit` (498 passed) and `cargo clippy -p test-toolkit --all-targets -- -D warnings`
  (clean) were used. `just check-tier-coverage test-toolkit`: nothing stranded.
- `claudine-cli`: the new bin builds and passes clippy with `-D warnings`
  (`--features test-fixtures --bin claudine-fake-editor`); the consolidated layout gate
  (`test_placement`, 12 tests) passes. The rest of Claudine was not run: Claudine changed
  only a `[[bin]]` entry and docs, and the Sniff change is additive (no glob import of
  `sniff::programs` or `sniff::os` exists outside Sniff, and no other crate defines the
  new names).
- **Pre-existing slow tests (test-toolkit, not touched here).** In the package run with
  the host at load ≈ 40 on 16 cores: `archive_path_guard every_allowlist_entry_still_names_a_live_site`
  7.5 s (alone 3.6 s), `archive_path_guard no_archive_executed_target_bakes_in_a_producer_path`
  9.1 s (alone 4.8 s), `kache_init_contracts ensure_is_idempotent_on_a_second_run` 5.2 s
  (alone 2.5 s), `kache_host_contracts report_resolves_the_worktree_base_by_the_wt_contract`
  6.1 s (alone 3.1 s): cleared under the loaded-host 10 s allowance by re-measuring alone.
  `ci_workflow_contracts the_shipped_planner_emits_plans_the_guards_reader_accepts` 15.0 s
  (alone 2.9 s) and `ci_workflow_contracts the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`
  15.5 s (alone 0.17 s) exceed 10 s in the package run: they are fast alone and compete
  for a shared resource (most likely the shipped planner both run). They are not in this
  phase's gate (darkmatter) and were not touched; recorded here for follow-up.
- **Windows cross-check** (`just cross-check <pkg> --os windows <filters>`): the first run
  found `spawn_child_records_a_live_child_pid` timing out, because on Windows the spawned
  child inherits the editor's standard handles and the test captured the editor's output
  with `.output()`. Fixed by running that test with null streams and documenting the
  caveat in the module notes; rerun passed. All 14 fake-editor integration tests (minus
  the two Unix-only ones) and both unit tests pass on Windows; the 20 Sniff tests pass on
  Windows, proving the case-insensitive `identify` branch.

## Phase 2

### What was built

- **Shared process tree** (R10): `darkmatter/lib/src/process_tree.rs` holds `ProcessTree`
  (moved unchanged from the login-shell launcher, which now imports it; its four tests
  stay green), plus `terminate_gracefully(grace, root)` (Unix `SIGTERM` to the group,
  poll the root, unconditional `SIGKILL` sweep, reap; Windows `TerminateJobObject`, reap)
  and `release()` (Windows clears kill-on-close before closing; see decisions).
- **Resolver** (R9): `editor/resolve.rs` with `SurfaceRequest`, `RejectReason`,
  `ResolvedEditor { program, args, surface, display }`, `resolve_editor`, and
  `resolve_editor_command` (now a wrapper). The order and filtering live in a private core
  `resolve_editor_from` over injected variables and lookups; probes run lazily (one
  `InstalledEditors` scan, only when reached). A probe now launches the path Sniff found
  (an alternate binary name previously launched the primary name). `wait_args_for_editor`,
  `build_editor_command_details`, and their 11 tests are deleted; Sniff's Phase 1 tests
  own those facts. `EditorError` gained `ConfiguredEditorCannotWait`, `NoMatchingEditor`,
  and `Cancelled`, each with a `status_block` arm. The context-construction guard's
  `std::env::var` allowance moved from `editor/mod.rs` to `editor/resolve.rs` (same count).
- **Launcher** (R10): `editor/launch.rs` with `LaunchPlan`, `LaunchOptions` (`PLAIN` is
  `md edit`/`--edit`: host process group, `Child::wait`), `LaunchIo`, `LaunchStdin`,
  `LaunchOutput`, `EDITOR_TERMINATE_GRACE` (2 s). `own_tree: true`: `process_group(0)` on
  Unix, a Job Object right after spawn on Windows (the escape window is commented), a
  20 ms `try_wait` loop polling `cancelled`. A terminal editor (terminal, both, or unknown
  surface) with inherited stdin gets the Unix foreground handoff only when stdin is a
  terminal whose foreground group is the host's: the child blocks `SIGTTOU` and calls
  `tcsetpgrp` in `pre_exec`; a `Drop` guard restores the host group and `termios`
  (`TCSADRAIN`) with `SIGTTOU` ignored, after the root is reaped, on every exit path.
  `ToHostStderr` duplicates the host's stderr handle (`try_clone_to_owned`, std only).
- **File preparation** (R13): `effects::fs_write::create_new_guarded` → `(cleaned path,
  CreateOutcome::{Created, Existing})`; one `create_new` open, handle dropped before
  returning. `fs_write` became `pub(crate)` for the editor module.
- **Surface and probe** (R12, R16): `editor/surface.rs` with the thread-local
  `deny_terminal_in_this_thread()` → `TerminalDenyScope` (not `Send`, restores the
  previous value), `terminal_denied()`, the process-wide `InteractiveSurface` claim, and
  `EnvironmentProbe::process()`.
- **Service** (R11, R12, R15): `editor/service.rs` with `EditorService`, `EditRequest`,
  `EditOutcome`, `EditError`, `EditorHostFacts`, `EditorOutput`, `ProcessEditorService`.
  Order: cancelled → resolve → eligibility (+ terminal claim held until exit) →
  containment → create (`attempted = true` from here) → `canonicalize_simplified` →
  launch (`own_tree: true`) → containment and regular-file recheck. Everything is
  re-exported from `darkmatter::editor`; every pre-existing public path is unchanged.
- **Docs and skills**: `darkmatter/docs/cli/edit.md` "How the editor is chosen" now
  describes skipping non-blocking editors, wait flags by program name (including a full
  path), no doubled flag, and the no-space-in-program-path limit. Darkmatter skill:
  a "Launching the user's editor" section in `library-surfaces.md`, the new fake-editor
  variable in `testing.md`. `os` skill: two symptom rows (below).
- **Fake editor**: one additive variable, `FAKE_EDITOR_PAUSE` (module docs, README, one
  toolkit test).

### Findings along the way

- **Fixture binary truncated by the test helper.** The first version of
  `install_fake_editor` hard-linked the fixture and fell back to `fs::copy`. Reinstalling a
  name that already existed (a hard link) made `copy` write through the link, truncating
  `target/debug/darkmatter-fake-editor` itself to 0 bytes; every later launch then ran an
  empty file as a shell script, exiting 0 and doing nothing. Symptoms were "passes alone,
  fails in a batch". Fixed by never copying over an existing install (comment at the
  site), and the binary was rebuilt.
- **`$EDITOR` is split on whitespace** (pre-existing, kept by R9): a program path that
  contains a space never resolves. Documented in `docs/cli/edit.md` and in the test
  helper; not changed here.
- **Windows: `CREATE_NEW` on an existing directory is "access denied"**, not "already
  exists" (os error 5). `create_new_guarded` now classifies any create error on a name an
  entry already holds (`symlink_metadata`) as `Existing`. Added to the `os` skill.
- **`CARGO_TARGET_TMPDIR` does not exist in archive runs** (cross-check, WSL2, CI): it
  is baked in at compile time. The helper uses the system temp directory. Added to the
  `os` skill.
- Windows forbids `|` and `*` in file names; the argv test uses `;$()&^%'` and a
  backtick instead.

### Requirement-to-test mapping

Boundaries: resolution reads the process environment and `PATH` (F1 in-process over the
pure core; F3 through the real environment and a real program for wiring); the launcher
and service cross a process boundary and the filesystem, so they spawn the real fake
editor binary (F3, L1). No real terminal is used: eligibility takes an injected probe,
and the handoff only engages on a real foreground TTY (Phase 4's L2 test covers it).

| Requirement | Test(s) |
|---|---|
| R9 `EDITOR` precedence, args kept | `darkmatter editor::resolve::tests::editor_takes_precedence_over_visual_and_keeps_its_arguments`; `darkmatter-cli l1 editor_resolution::editor_wins_over_visual_and_its_arguments_reach_the_editor` (argv incl. `;`, `$`, `&`, Unicode) |
| blank / unset / missing `EDITOR` → `VISUAL` | `resolve::tests::{a_blank_editor_falls_to_visual, an_unset_editor_falls_to_visual, a_missing_editor_falls_to_a_visual_that_exists}`; `editor_resolution::a_blank_or_missing_editor_falls_to_visual` |
| missing configured editor refuses, probes never run | `resolve::tests::a_configured_editor_that_is_missing_refuses_without_probing`; `editor_resolution::a_missing_configured_editor_refuses_rather_than_probing` |
| non-blocking `geany`/`lapce` skipped, error says cannot wait, never probed | `resolve::tests::{a_configured_non_blocking_editor_is_skipped_and_named, a_non_blocking_editor_falls_to_the_next_variable, the_first_rejected_variable_names_the_error, probes_never_select_a_non_blocking_editor, only_non_blocking_editors_installed_is_no_editor_found}`; `editor_resolution::a_configured_non_blocking_editor_is_refused_as_unable_to_wait` |
| `gui` true/false/omitted, `EDITOR=nvim VISUAL=code` | `resolve::tests::{gui_true_skips_a_terminal_editor_for_a_gui_visual, gui_false_keeps_a_terminal_editor, gui_omitted_keeps_ordinary_precedence}`; `editor_resolution::gui_true_skips_a_terminal_editor_for_a_gui_visual_and_gui_false_keeps_it` |
| `EDITOR=emacs gui:false` → terminal probe | `resolve::tests::gui_false_with_a_dual_surface_editor_probes_for_a_terminal_editor` |
| `Both`/unknown never match explicit `gui` | `resolve::tests::an_explicit_surface_never_matches_a_dual_or_unknown_editor` |
| probes run although a variable is set (explicit `gui`) | `resolve::tests::an_explicit_surface_probes_although_a_variable_is_set` |
| typed `NoMatchingEditor` lists every candidate + reason | `resolve::tests::no_matching_editor_lists_every_candidate_with_its_reason`; `darkmatter l1 error_snapshots::editor::no_matching_editor_lists_each_candidate_and_reason` |
| `EDITOR="code --wait"` not doubled; `/abs/path/code` gets `--wait` before the path | `resolve::tests::{a_configured_wait_flag_is_not_doubled, an_absolute_path_to_a_known_gui_editor_gets_its_wait_flag, kate_gets_block}`; `editor_resolution::{a_configured_wait_flag_is_not_doubled, a_gui_editor_named_by_absolute_path_gets_its_wait_flag_before_the_path}` (leading-dash file name) |
| unknown command trusted; probe launches found path | `resolve::tests::{an_unknown_command_is_trusted_without_gui, a_probe_launches_the_path_it_was_found_at}` |
| `md edit`/`--edit` unchanged | `editor_resolution::{md_edit_surfaces_a_non_zero_exit, wrapper_edit_text_returns_the_trimmed_buffer}`; existing `darkmatter editor::tests::*` (Unix); Claudine `prompt_source`/`edit` tests (31) |
| new `EditorError` status blocks | `error_snapshots::editor::{configured_editor_cannot_wait_names_the_variable, no_matching_editor_lists_each_candidate_and_reason, cancelled_names_the_path_and_keeps_saved_bytes}` |
| R10 exact argv (spaces, metacharacters, Unicode, leading-dash directory) | `darkmatter-cli l1 editor_launcher::the_path_is_one_argument_after_the_editor_arguments` |
| nonzero exit, missing binary | `editor_launcher::a_non_zero_exit_and_a_missing_program_are_typed_failures` |
| stdout/stderr routing (`Inherit`, `ToHostStderr`, `Discard`) | `editor_launcher::output_goes_where_the_launch_io_says` (re-runs the test binary as the host via `route_helper`) |
| cancellation terminates and reaps the tree (grandchild dead), saved bytes kept | `editor_launcher::cancellation_terminates_the_whole_tree_and_reaps_it` (Unix group; Windows Job, proved by cross-check) |
| `SIGTERM` ignored → `SIGKILL` after grace, elapsed in [grace, grace + 5 s) | `editor_launcher::an_editor_ignoring_sigterm_is_killed_after_the_grace_period` (Unix; 2.07 s) |
| GUI client: only the client is owned | `editor_launcher::cancelling_a_gui_client_leaves_an_already_running_editor_alone` |
| `own_tree: false` keeps the host group; `own_tree: true` leads its own; a successful owned launch leaves its leftovers running | `editor_launcher::a_plain_launch_keeps_the_host_process_group` (Unix) |
| `ProcessTree` move byte-identical | existing `markdown::compose::shell_expansion::launcher::tests::*` (4) |
| R13 creation under missing parents; `Existing` for file, dir, symlink, dangling symlink; race = exactly one `Created`; escaping symlink refused before creation; outside root creates nothing | `darkmatter effects::fs_write::tests::{create_new_makes_an_empty_file_under_missing_parents, create_new_leaves_an_existing_file_untouched, create_new_reports_a_directory_as_existing, create_new_does_not_follow_a_symlink_even_a_dangling_one, racing_creators_see_exactly_one_created, create_new_refuses_a_symlink_that_escapes_the_root_before_creating, create_new_outside_the_root_is_refused_and_creates_nothing}` |
| R16 deny scope nested/per-thread; terminal claim exclusive | `darkmatter editor::surface::tests::{denial_is_scoped_nested_and_per_thread, the_interactive_surface_is_exclusive_until_released}` |
| R12 eligibility table: terminal/unknown/`Both`-without-`gui` × structured, stdin reserved, stdin/stdout not tty, Markdown stderr not tty, background, parallel member; refusals create nothing, `attempted == false`; Claudine-style terminal output needs no tty stderr and no display | `darkmatter-cli l1 editor_service::a_terminal_editor_needs_an_interactive_terminal_the_host_can_lend` |
| GUI: no display refused; allowed under structured output, no tty, parallel member; stdin null | `editor_service::a_gui_editor_needs_a_display_but_no_terminal` |
| missing parents created, editor gets the absolute native path | `editor_service::an_eligible_terminal_editor_creates_missing_parents_and_succeeds` |
| `create_new` occupied by file/dir/symlink/dangling symlink: never launched, bytes unchanged | `editor_service::create_new_refuses_an_occupied_name_without_launching` |
| directory target before launch | `editor_service::a_directory_target_is_not_a_regular_file` |
| outside the mutation root refused before creation | `editor_service::a_target_outside_the_mutation_root_is_refused_before_creation` |
| byte-for-byte preservation (trailing whitespace, CRLF, non-UTF-8) | `editor_service::an_existing_regular_file_is_opened_with_its_bytes_preserved` |
| atomic-save replacement succeeds | `editor_service::an_atomic_save_that_replaces_the_file_is_success` |
| deleted after exit fails | `editor_service::a_file_deleted_by_the_editor_fails_after_exit` |
| directory after exit; containment recheck after exit (escaping symlink, Unix) | `editor_service::the_target_is_rechecked_after_the_editor_exits` (uses `FAKE_EDITOR_PAUSE`) |
| concurrently created name opened, not replaced | `editor_service::a_name_created_concurrently_is_opened_not_replaced` + the `fs_write` race test (see decisions) |
| `attempted == true` after a failed launch; created file kept | `editor_service::a_failed_launch_still_counts_as_attempted` |
| refused once cancelled, nothing created | `editor_service::an_edit_is_refused_once_the_run_is_cancelled` |
| process-wide surface guard → `SurfaceBusy` | `editor_service::a_second_terminal_editor_waits_for_no_one_and_is_refused` |
| `FAKE_EDITOR_PAUSE` contract | `test-toolkit fake_editor pause_waits_for_its_marker_to_be_removed_then_exits_normally` |

Unicode paths are covered in the argv test and the missing-parents service test. Not
covered here, by design: the real-TTY foreground handoff and termios restore (needs a
terminal; Phase 4's L2 test), and Windows Job termination of a `SIGTERM`-ignoring tree
(the Unix-only grace test has no Windows analogue; the Windows cancellation test proves
job termination). The input-robustness matrix does not apply: Phase 2 reads no file
format or configuration.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [darkmatter, darkmatter-cli, test-toolkit]`, `change_class: package`; gates run.
- `darkmatter`: `just lint` clean (after factoring one `type` alias clippy asked for);
  `just test` 16539 passed, 10 skipped, 0 slow (final run, after every edit);
  `just slow-tests fresh` exit 0; `just check-tier-coverage darkmatter`: nothing stranded.
  An earlier full run (host load 58→138 on 16 cores, not from this session's work) had 4
  tests at 5.1–6.4 s: `trigger_spelling_guard::no_active_file_spells_a_renamed_trigger_form`
  (alone 1.8 s) and three `claudine_schema_bundle` tests (alone 2.0–2.6 s); none touched
  here, cleared under the loaded-host rule, and the final run had none.
- `test-toolkit`: root `just test test-toolkit` 499 passed; one pre-existing slow test,
  `archive_path_guard no_archive_executed_target_bakes_in_a_producer_path` 5.4 s at load
  ≈ 38, 4.86 s alone (cleared; Phase 1 recorded the same test). `cargo clippy -p
  test-toolkit --all-targets -- -D warnings` clean.
- Dependents: `claudine-cli` `-E 'test(/prompt_source|edit/)'` 31 passed (the plan's
  checkpoint); Claudine's library builds against the unchanged public paths.
- Cross-OS (process trees, Job Objects, handle duplication, termios are real OS risk):
  `just cross-check darkmatter-cli --os windows editor_` — attempt 1 found the
  `CARGO_TARGET_TMPDIR` problem, attempt 2 (after the fixes above) 30/30 passed;
  `just cross-check darkmatter --os windows editor fs_write shell_expansion::launcher context_construction_guard`
  50/50 passed; `just cross-check darkmatter-cli --os linux editor_` 32/32 passed.
  (`-E 'test(...)'` breaks the remote quoting; positional substring filters work.)

## Phase 3

### What was built

- **Typed action and registration** (R1, R2): `EditAction { file: Expr, create_new: bool,
  gui: Option<bool> }` and `LifecycleActionKind::Edit` in `lifecycle/actions.rs`;
  `LIFECYCLE_ONLY_VERBS = ["edit"]` and `is_lifecycle_only_verb` in `signatures.rs`, read by
  `is_known_lifecycle_verb` and `all_lifecycle_verbs` (so bare-verb parsing, did-you-mean, and
  Claudine's loop-key check see it). `action_type` → `Expression`, `action_verb` → `"edit"`;
  `runs_a_shell` stays false, so `initialize` admits it (both registries declare
  `Expression` for `initialize`, confirmed in `registry.rs` and Claudine's registry).
- **Parsing** (R3): short form and one-element list through a new `edit` branch in
  `build_non_control_positional_action` (exact arity 1, before the side-effect and
  expression-function fallbacks); long form in `build_action_from_params` (`file` required,
  unknown keys through `reject_extra_params`); `parse_long_form` refuses a non-boolean
  `create_new`/`gui` (including `null`, `"true"`, `"{{ true }}"`) before
  `action_value_to_expr`. A shared `edit_file` check refuses number, boolean, and blank
  literals; a non-blank literal is kept byte-for-byte. `positional_operand_field("edit")`
  is `file`. Bare `edit` now reports arity, not "unknown verb". `with:` hits the existing
  `LifecycleWithUnsupported`; ordering needed no change (`canonical_rank` third bucket).
- **Passive surfaces** (R4): `walk_action` visits `…edit.file` only; `template_operands`
  covers `edit` (long form: `file` only, never `create_new`/`gui`); the source map records
  long-form `edit` parameters by name; `action_writes` returns nothing (wildcard). The
  schema projection's `hand_owner` refuses a catalog verb named like a lifecycle-only verb.
- **Execution placeholder**: `execute_action_inner` refuses `edit` with an unsuppressible
  evaluation failure ("this host cannot run `edit`") until Phase 4's dispatch replaces it,
  so no execution behavior is introduced. Claudine's `describe_action` got the R22 arm
  (needed to compile). Every other `LifecycleActionKind` walker (preflight candidates,
  shell walk, shell templates, Claudine `prepare.rs`, task templates, container preflight)
  uses a wildcard and treats `edit` as inert; each was read to confirm.

### Departures from the plan

- The plan said to update "verb-inventory assertions in `lifecycle_generated_operands.rs`
  and the catalog-equality tests under `catalog/tests.rs`". Neither reads
  `all_lifecycle_verbs` (they cover schema templates and the function catalog), and both
  stayed green unchanged. The checks that did break were the two parser-vs-schema
  agreement tests; they got self-expiring exemptions (see `decisions_for_review`).
- The parse tests live in one L1 integration file through the public API rather than
  split across `lib/src/lifecycle/tests/` and `lib/tests/l1/`.

### Requirement-to-test mapping

All in `darkmatter/lib/tests/l1/lifecycle_edit_action.rs` unless noted (24 tests).

| Requirement | Test(s) |
|---|---|
| Control row: motivating fixture → `EditAction { create_new: true, gui: None }`, `{{ fqn }}` kept as an expression | `the_motivating_workflow_parses_to_an_exclusive_edit_in_the_else_branch` |
| Short form, list-of-one equals scalar, defaults | `the_short_forms_open_the_operand_with_default_flags` |
| `edit: fqn` is the literal `fqn` | `a_bare_word_operand_is_a_literal_file_name_not_a_variable` |
| Whitespace preserved in non-blank path; mixed template kept | `a_nonblank_literal_keeps_its_whitespace`, `a_mixed_template_operand_stays_a_template` |
| Flag control rows, absent defaults, inner `no_error` | `the_long_form_reads_each_literal_boolean` |
| `file`: absent / null / wrong type / object | `file_absent_is_refused`, `file_null_is_refused`, `file_of_the_wrong_type_is_refused` |
| List element wrong (one, every), two strings → arity | `a_list_element_of_the_wrong_type_is_refused` |
| Empty: `""`, `"   "`, `[""]`, long-form blank, `[]` | `an_empty_operand_is_refused` |
| Bare `edit` is arity, not unknown verb | `a_bare_edit_is_an_arity_error_not_an_unknown_verb` |
| Object short operand | `a_direct_object_short_operand_is_refused` |
| `create_new`/`gui` shared validator: null, strings, `{{ true }}`, number, array, object, located at the key | `a_flag_that_is_not_a_literal_boolean_is_refused_at_its_key` |
| Unknown key, `with:` | `unknown_and_with_keys_are_refused` |
| Dictionary order: after `info` and `shell`, before `ensure_file` (by name) and `stop` | `a_compact_dictionary_runs_edit_after_communication_and_shell_before_flow_control` |
| Sibling `no_error`; inner+sibling duplicate; non-boolean `no_error` | `a_sibling_no_error_applies_and_a_duplicate_is_refused` |
| `Expression` type, not a shell; accepted in `initialize`, every md event, and inside `then`/`else` | `edit_is_an_expression_action_admitted_in_initialize_and_every_md_event` |
| `LIFECYCLE_ONLY_VERBS`, `is_known_lifecycle_verb`, `all_lifecycle_verbs` | `edit_is_the_one_lifecycle_only_verb_and_a_known_verb` |
| No `edit` expression function or side effect | `no_expression_function_or_side_effect_is_named_like_a_lifecycle_only_verb`; generator refusal in `lifecycle_schema_generation.rs::a_new_collision_with_a_hand_verb_or_reserved_key_fails_the_projection` (`edit` row) |
| Did-you-mean `edti` → `edit` | `a_misspelled_edit_suggests_edit` |
| Both branches walked, event-time value deferred (an unknown function in `file` is never run) | `the_call_check_walks_the_file_operand_in_both_branches_without_evaluating_it` |
| A proven-bad call in an untaken branch's `file` rejected at `…edit.file` | `a_proven_bad_call_in_an_untaken_edit_branch_is_rejected_at_its_operand` |
| Schema span coverage reads `file`, not the flags | `schema_template_operands_cover_file_but_not_the_boolean_flags` |

Every error assertion checks the variant, `source_path`, the located `property`, and the
rendered message. The input loader is the existing frontmatter loader (R25), so no
document-shape tests were added. Smell grep over the new code (`serde(default)`,
`unwrap_or_default()`, `filter_map(.. as_str)`, `.ok()`): none on a load-bearing field;
`gui: Option<bool>` is `None` only when the key is absent, because `null` is refused first.
Spawned-CLI wiring cases and `md schema validate`/DMLS agreement are later phases (the
schemas do not type `edit` yet).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-cli, test-toolkit, claudine-cli]`,
  `change_class: package` (the working tree still holds Phase 2's uncommitted files); gates run.
- `darkmatter`: `just lint` clean; `just test` 16563 passed, 10 skipped, none at or above
  5 s; `just check-tier-coverage darkmatter`: nothing stranded.
- Dependent `claudine` (over 200 tests, so filtered): `-E 'test(claudine_schemas) |
  test(loop) | test(lifecycle) | test(side_effect)'` 1914 run, one failure
  (`claudine_schemas::hand_verbs_equal_the_parser_inventory`, fixed as above, then passes);
  `-E 'test(sequence::task) | test(side_effect) | test(claudine_schemas::)'` 277 passed;
  `cargo clippy -p claudine --all-targets -- -D warnings` clean. That first run reported
  six `claudine_schemas_matrix` tests at 5.6–6.8 s while host load was ≈ 80 (the run
  itself); alone, `yaml_trigger_combinator::control` 1.45 s and
  `yaml_action_initialize_bundle::wrong_type_whole` 1.32 s (load-sensitive, untouched here).
  `claudine-cli` compiles (`cargo check --all-targets`).
- Cross-OS: not run. Phase 3 is pure parsing over in-memory JSON with no path, process,
  or OS-specific code.

## Phase 4

### What was built

- **Host seam** (R18, R19): `LifecycleHost::editor() -> Option<&dyn EditorService>`
  (default `None`) and `LifecycleRunFailure::EditFailed { source_path, property, target,
  error: Box<EditError> }` in `lifecycle/executor/host.rs`. `EditError` gained
  `NoLocalTarget` (see decisions).
- **Executor** (R14, R17): `execute_action_inner`'s placeholder arm is replaced by
  `dispatch_edit`. Order: evaluate `file` (`evaluate_operand`); a non-string or blank value
  is an unsuppressible evaluation failure ("`edit` needs a string path, got number",
  "`edit` needs a non-blank path"); resolve the target; refuse if a cancellation cause
  exists (`cancellation::cause_with`, evaluation failure); `host.editor()` or
  `EditFailed(Unsupported)`; call the service with `cancelled = cause_with(..).is_some()`;
  `host.side_effect_attempted()` once when `attempted`; a cancelled outcome, or a cause
  that appeared while the editor was open, is the evaluation failure "the run was
  cancelled while an `edit` was open"; every other failure is a dispatch failure through
  `host.failure(EditFailed{..})`, located at the action (`…edit`). `resolve_effect_path`
  and the new `resolve_edit_target` share one private helper, `mutation_target`, so the
  two cannot drift; `resolve_edit_target` refuses a URL before resolving anything.
  `dispatch_task_side_effect_inner` still rejects `Edit` through its wildcard (R22).
- **`md compose` host**: `MdStackContext::editor` / `MdStackHost::editor`, filled per
  event by the CLI with `ProcessEditorService::new(EditorHostFacts { output: Structured`
  under a data output, else `MarkdownDocument`, `stdin_reserved` when the document came
  from stdin `})`; `LifecycleCompose`/`ComposeHost` carry `stdin_is_input`.
  `failure_message` renders `EditFailed` (see decisions for the wording).
- **Claudine compile arm**: `classify_run_failure` maps `EditFailed` to a plain `edit`
  action failure until Phase 5 adds `CompositionError::LifecycleEditFailed`.
- **Recipe**: `darkmatter/justfile`'s `test-l2` builds the CLI with
  `terminal-tests,test-fixtures`, as CI's `[package.metadata.ci.tests] features` already
  does, so the L2 tier can launch `darkmatter-fake-editor`.
- **Skills**: `darkmatter/lifecycle.md` (the executor/host contract, md's mutation root,
  the message clamp) and `darkmatter/testing.md` (how the `md compose` and L2 editor
  tests are built).
- **Docs**: none in this phase; the canonical lifecycle docs for `edit` are Phase 7's
  task. Until then `docs/` does not describe `edit`.

### Findings along the way

- **`md`'s failure line is clamped to 240 characters** (`concise_message`). R19's
  "`{property}` could not edit {target}: {error}" put two absolute paths ahead of the
  reason, and most `EditError` texts name the path again, so the reason itself was cut
  off (in the terminal line and in the JSON diagnostic alike). Fixed by putting the
  reason first (decisions).
- **`md compose`'s mutation root is the prompt's own directory**, while Claudine's is the
  launch workspace. A prompt in a sibling library therefore cannot `edit` a
  `{{ ctx.cwd }}` path under `md` (containment refuses it before anything is created).
  The spec's "prompt outside the launch directory" acceptance case is met under `md` with
  the prompt in an ancestor directory, and the sibling case is pinned as a refusal
  (decisions).
- **`error:` in `composed` goes straight to `finalize` under `md`**; only a preparation
  failure (here a missing `::file`) reaches `failed`. The ordering test uses one run for
  `composed`/`finalize` and one for `failed`.
- **`PENDIN` flips on its own.** Comparing `stty -g` before and after `md` differed only
  in `PENDIN` (`lflag` `0x20000000`), which the kernel sets while typed input is pending;
  the L2 test compares `stty -a` without the `pendin` token.
- **The local `just test-l2` never built the fake editor** (it lacked `test-fixtures`,
  which CI enables), so an L2 test that launches it could only fail locally.

### Requirement-to-test mapping

Boundaries: the executor contract is F1 in-process against a recording fake
`EditorService` (`darkmatter/lib/src/lifecycle/executor/tests.rs`, L1); the `md compose`
host crosses the process, filesystem, and stream boundaries, so it is F3: the real `md`
binary launching the real `darkmatter-fake-editor` (`darkmatter/cli/tests/l1/compose_edit_action.rs`,
L1, argv recorder + directory checks, never mocked dispatch); the terminal handoff needs a
real terminal, F3 at L2 (`darkmatter/cli/tests/level2/level2_edit_action.rs`, tmux, no focus).
Unit-test names below are under `darkmatter lifecycle::executor::tests::`, process tests
under `darkmatter-cli::l1 compose_edit_action::`, L2 under `darkmatter-cli::level2 level2_edit_action::`.

| Requirement | Test(s) |
|---|---|
| R17 `cause_with` sees a cause inside `CleanupScope`, `requested_with` does not | `darkmatter lifecycle::cancellation::tests::inside_cleanup_a_cause_is_still_visible_to_cause_with_but_not_requested_with` |
| R14 interpolated absolute path, flags passed through, next action runs, attempted → memo forgotten | `edit_opens_the_interpolated_absolute_path_with_its_flags_then_continues` |
| R14 missing bare relative target under the mutation root; refusal before preparation keeps the memo | `a_missing_bare_relative_target_is_created_under_the_mutation_root` |
| R18 host without an editor → `EditFailed(Unsupported)` located at `…edit`, no continuation; `no_error` continues | `a_host_without_an_editor_fails_the_action_as_unsupported_and_no_error_continues` |
| R19 editor failure is a located dispatch failure; `no_error` suppresses; attempted still forgets reads | `an_editor_failure_is_a_located_dispatch_failure_that_no_error_suppresses` |
| R14 non-string (number, boolean, null, array) and blank (`""`, `"   "`) values: unsuppressible evaluation failure, editor never called | `a_non_string_or_blank_value_is_an_unsuppressible_evaluation_failure` |
| R14 URL refused without reaching the editor | `a_url_is_refused_without_reaching_the_editor`; process: `editor_failures_name_the_action_property_and_target_and_stop_the_stack` (`url` row) |
| R17 cancelled while open → evaluation failure, `no_error` does not continue | `an_edit_cancelled_while_open_never_continues_even_with_no_error` |
| R17 cancellation during a successful exit still stops | `a_cancellation_during_a_successful_editor_exit_still_stops_the_stack` |
| R17 cleanup events edit on a healthy run; refused after an interrupt or a worker cause | `cleanup_events_edit_on_a_healthy_run_and_refuse_after_a_cancellation` |
| Compact dictionary: communication before `edit`, failure ends the stack | `a_compact_dictionary_runs_communication_before_edit` |
| AC1 motivating workflow: absolute computed path, `--wait`, exact bytes (CRLF, tab, trailing spaces), success line, `skip` writes nothing; existing spec → warn branch, no launch | `the_motivating_workflow_creates_and_opens_the_spec_then_skips` |
| AC1 prompt outside the launch directory; sibling library refused before creation | `a_prompt_outside_the_launch_directory_edits_the_absolute_target_inside_its_root` |
| AC1 race: `create_new` fails without launch, bytes unchanged; `no_error` continues; without `create_new` the existing file is opened | `a_name_taken_after_the_check_fails_create_new_and_opens_without_it` |
| AC1 `create_new` target held by a directory or a (dangling) symlink | `a_create_new_name_held_by_a_directory_or_symlink_fails_without_launch` |
| AC7 ordering in `composed`, `failed`, `finalize`, `then`/`else`, compact dictionary | `edit_runs_in_order_in_composed_failed_and_finalize` (initialize: the motivating test) |
| AC7 post-edit reads see changed data after a failed edit absorbed by `no_error`, and deleted data | `a_read_after_the_edit_sees_changed_and_deleted_data` |
| AC5 non-zero exit, deleted target, directory target, outside the root, no editor, URL: located message naming property and target, no continuation; launched only where expected | `editor_failures_name_the_action_property_and_target_and_stop_the_stack` |
| AC3/AC5 symlink escape refused, no parent created through it | `a_symlink_escaping_the_root_is_refused_before_anything_is_created` (Unix) |
| AC5 noninteractive terminal editor refused before creation; stdin document reserves stdin | `a_terminal_editor_without_an_interactive_terminal_is_refused_before_creation` |
| AC9 Markdown mode: editor output on stderr, never in the document; GUI stdin null | `a_gui_editors_output_never_reaches_the_composed_document` |
| AC9 structured output: terminal editor refused into one envelope, empty stderr; `no_error` → success envelope; GUI editor runs silenced | `structured_output_refuses_a_terminal_editor_and_silences_a_gui_editor` |
| AC6 `md compose --shell`, `md schema validate`, untaken branch: no launch, no creation; bad call in untaken branch still rejected at `…edit.file` | `passive_surfaces_and_untaken_branches_never_launch_or_create` |
| AC6 library composition | `library_composition_never_runs_an_edit` |
| AC6 DMLS analysis | `darkmatter-lsp l1 no_side_effects::expression_validation_runs_no_actions_shells_file_effects_or_lazy_providers` (now includes an `edit` with `create_new`; the workspace listing is unchanged) |
| AC7/AC9 terminal handoff: editor reads a typed line, all three streams are terminals, next action and document follow, terminal settings restored, shell takes input | `level2_edit_action_a_terminal_editor_reads_the_pane_then_the_stack_continues` |
| AC9/AC10 interrupt while open: editor tree reaped (spawned child dead), run reported cancelled, `no_error` does not resume, created file kept, settings restored | `level2_edit_action_an_interrupt_reaps_the_open_editor_and_returns_the_terminal` |

Launch failure of a resolved program is covered at the launcher by Phase 2
(`editor_launcher::a_non_zero_exit_and_a_missing_program_are_typed_failures`); through `md`
the "no editor" row covers a configured program that does not exist. The input-robustness
matrix does not apply: this phase adds no reader of a file format or configuration (the
operand's shapes were Phase 3's matrix; R14's runtime string/blank check is covered above).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-cli, darkmatter-lsp, test-toolkit, claudine-cli]`,
  `change_class: package`; gates run.
- `darkmatter`: `just lint` clean (plus `cargo clippy -p darkmatter-cli --all-targets
  --features test-fixtures,terminal-tests -- -D warnings` clean); `just test` 16587 passed,
  10 skipped, "none at or above 5.0s" (host load ≈ 11 on 16 cores); `just slow-tests fresh`
  exit 0; `just check-tier-coverage darkmatter`: nothing stranded. The race test was moved
  from `composed` to `initialize` after that run's build; re-run alone: 13/13 passed.
- `just test-l2 edit_action`: 2 passed (tmux, no focus).
- Dependents: `claudine` (over 200 tests, filtered) `-E 'test(lifecycle) | test(side_effect) |
  test(classify) | test(claudine_schemas::)'` 1153 passed; `cargo clippy -p claudine
  --all-targets -- -D warnings` clean. `claudine-cli` `-E 'test(/prompt_source|edit|lifecycle_failure/)'`
  30 passed.
- Cross-OS (process spawns, paths, `code.exe` identification, containment):
  `just cross-check darkmatter-cli --os windows compose_edit_action` 12/12 passed (the
  symlink-escape test is Unix-only); `--os linux` 13/13 passed. The L2 tests are Unix-only
  by design.

## Phase 5

### What was built

- **Host seam** (R18, R20): `claudine/lib/src/composition/lifecycle/editor.rs` (new, `pub mod editor`).
  `StackExecutionContext`'s `LifecycleHost::editor` returns `Some(&EDITOR_SERVICE)`, a zero-sized
  `ClaudineEditorService` that builds a `ProcessEditorService` at each edit (`service()` →
  `service_with(EnvironmentProbe::process())`), so the probe sees the streams as they are when the
  editor would open. `EditorHostFacts { output: Structured` when the process-wide
  `set_structured_output(true)` flag is set, else `Terminal`; `stdin_reserved: false }`.
- **Output mode** (R20): set in `claudine/cli/src/commands/wrap/mod.rs::run_provider_wrapper_inner`
  (first thing, from `--output`; an unparsable value is still reported where the flag is applied) and in
  `claudine/cli/src/commands/compose/prep.rs::run_composition_inner` (from `SharedComposeArgs::output`,
  which `compose` and `inline-compose` share).
- **Failure routing** (R19): `CompositionError::LifecycleEditFailed { source_path, property, target,
  error: Box<EditError> }` (`#[source]` keeps the typed cause), Display "`<property>` could not edit the
  file: <reason>[ (<target>)] (<source>)" — reason first, target only when the reason does not name it,
  matching `md`. Code `composition.lifecycle_edit_failed` (registry entry: Composition / Correctable /
  Environment, detail `source_path`, `property`, `target`, `reason`), its `code()` and `detail()` arms in
  `error/render/mod.rs`, and a corpus entry in `claudine/cli/tests/l1/error_guards.rs` (the
  scan-backed guard requires one per returnable code). `classify_run_failure`'s temporary Phase 4 arm is
  replaced by the typed mapping. The status block uses the generic catch-all, as `LifecyclePrepFailed` does.
- **Parallel denial** (R16): each `group.rs` member thread enters
  `darkmatter::editor::deny_terminal_in_this_thread()` beside its cancellation and prep scopes.
- **Task refusal** (R22): already present from Phase 3 (`describe_action`); this phase added its tests.
- **Fake editor** (R21): `claudine-fake-editor` was wired in Phase 1. `claudine/justfile`'s `test-l2`
  now builds `claudine-cli` with `terminal-tests,test-fixtures` (CI's declared features already include
  both), so the L2 tier can launch it — the same fix Phase 4 made for Darkmatter.
- **Skills**: `.claude/skills/claudine/architecture.md` (Execution bullet) and a pointer in `.claude/skills/darkmatter/lifecycle.md` record the host wiring,
  the corpus requirement, and the timeout gap.

### Findings along the way

- **Claudine's schemas reject `edit` until Phase 6.** `claudine compose` schema-validates the document
  after `initialize` and before `start`; `claudine sequence` validates prompt tasks and containers before
  running them. Today an `edit` is reachable only in an `initialize` that ends in `skip` or a failure.
  Seven L1 tests therefore wrap their bodies in `pending_until_the_schema_knows_edit`, which passes only
  while the body fails on that schema refusal and fails (asking for its own removal) once the schemas
  accept `edit`. The test agent verified all seven bodies pass against a temporary schema entry, then
  restored `claudine/schemas/` byte-for-byte (decisions).
- **No Claudine timeout bounds a lifecycle stack.** A document's `timeout:`/`--timeout` bound only the
  provider session and a task's `timeout:` only its shell commands; `claudine compose --timeout 1s` with
  a held editor stays blocked. The spec's "a sequence `timeout` cancels an open editor" therefore has no
  Claudine trigger; cancellation is proven with an interrupt and a fail-fast sibling instead (decisions).
- **A failing sequence-root `initialize` exits 1 silently** (pre-existing; reproduced with
  `ensure_file: "../outside.md"`, also seen with `prep`). `wrap/sequence/mod.rs`'s `Finished { failed }`
  arm returns the exit code without rendering `root.runner.primary_error()`. An `edit` failure there is
  therefore silent too. Not fixed in this phase: it affects every action, predates this feature, and the
  rendering route for a root-container failure needs a design choice (recorded for review).
- **A sequence root value such as `fqn: "{{ ctx.cwd }}/x.md"` is not interpolated when the root
  `initialize` reads it** (it stays literal), whereas `compose` expands it. The sequence test spells
  `{{ ctx.cwd }}` at the use site. Pre-existing; noted only.
- **Task `setup:`/`teardown:` and `execution: serial` are removed grammar**, and groups are always
  concurrent, so the plan's "serial-group stack" and "task setup/teardown" orderings were proved with an
  inline `sequence:` container's `start`/`finalize`, the serial equivalent.
- **Disk full.** `/Volumes/coding` hit 0 bytes free mid-phase (the APFS container is shared; another
  worktree's `target/` holds 790 GB). `scripts/sweep.sh` on this worktree only (repository policy)
  trimmed its `target/` 127 → 102 GB and freed ~18 GB. Nothing outside this worktree was touched.

### Requirement-to-test mapping

Boundaries: the host seam and failure classification are F1 in-process through Claudine's real
`StackExecutionContext` (`claudine` lib unit tests, L1); the CLI wiring (output-format flag, group
threads, sequences, provider skipping) crosses process, filesystem, and stream boundaries, so it is F3:
the real `claudine` binary launching the real `claudine-fake-editor` and `claudine-fake-goose`
(`claudine-cli::l1 lifecycle_edit_action::`); the terminal handoff is F3 at L2 in a detached tmux pane
(`claudine-cli::level2 level2_edit_action::`, Unix only). "(pending)" marks a body wrapped in
`pending_until_the_schema_knows_edit` until Phase 6.

| Requirement | Test(s) |
|---|---|
| R20 output mode from the flag; stdin never reserved; injected probe used | `claudine composition::lifecycle::editor::tests::the_output_mode_follows_the_invocation_and_stdin_is_never_reserved` |
| No display → GUI editor refused before creation (injected probe) | `…editor::tests::a_host_without_a_display_refuses_a_gui_editor_before_creating_the_file` |
| R19 typed `LifecycleEditFailed`, code, located property, detail, reason-first message; `no_error` absorbs | `claudine composition::lifecycle::executor::tests::action_dispatch::an_edit_failure_is_a_typed_lifecycle_edit_failure_that_no_error_absorbs` |
| R18 Claudine supplies a real service (never `Unsupported`); nothing created before an editor resolves | `…action_dispatch::claudine_stacks_supply_an_editor_service` |
| R22 `side_effect: edit` refused, nothing created (in-process / CLI) | `claudine composition::sequence::task::tests::side_effect_tasks::an_edit_side_effect_is_refused_without_opening_or_creating_anything`; `lifecycle_edit_action::a_side_effect_task_containing_edit_is_refused_without_launching_or_creating` |
| Catalog completeness | `claudine diagnostics::registry::tests::catalog_covers_the_ratified_count`; `claudine-cli::l1 error_guards::production_sources_pass_every_scan_backed_guard` |
| AC1 motivating workflow via `compose`: absolute path, `--wait`, exact bytes, success line, `skip` → provider never launched; existing spec → warn, no launch | `lifecycle_edit_action::compose_motivating_workflow_creates_and_opens_the_spec_then_skips_the_provider` |
| AC1 prompt kept outside the launch directory edits inside the launch workspace | `lifecycle_edit_action::compose_edits_inside_the_launch_workspace_from_a_prompt_kept_elsewhere` |
| AC1 via `sequence` (root initialize / prompt task) | `lifecycle_edit_action::sequence_root_initialize_runs_the_motivating_workflow_before_any_task`; `…::sequence_prompt_task_runs_the_motivating_workflow_and_launches_no_provider` (pending) |
| AC7 ordering: initialize incl. `then`; `start`/`success`/`finalize`; container `start`/`finalize` around children | `…::initialize_runs_each_edit_in_order_with_the_actions_around_it`; `…::compose_runs_edit_in_order_in_start_success_and_finalize` (pending); `…::a_sequence_container_runs_edit_in_its_start_and_finalize_around_its_children` (pending) |
| R16 parallel member: terminal editor refused before creation; GUI editor runs | `…::a_parallel_member_refuses_a_terminal_editor_before_creating_anything` (pending); `…::a_parallel_member_runs_a_gui_editor` (pending) |
| AC9 structured output refuses a terminal editor, stdout untouched; `no_error` continues; GUI output off stdout | `…::structured_output_refuses_a_terminal_editor_and_no_error_continues`; `…::structured_output_runs_a_gui_editor_and_keeps_its_output_off_stdout` |
| AC10 interrupt reaps the editor tree, keeps saved bytes and the created file, next action skipped despite `no_error`, `cancelled`/`finalize` run, exit 130; SIGTERM-ignoring editor escalates | `…::an_interrupt_terminates_a_held_editor_tree_and_keeps_what_it_saved`; `…::an_interrupt_escalates_to_kill_for_an_editor_that_ignores_term` (Unix) |
| AC10 sibling failure in a fail-fast group cancels a held GUI editor | `…::a_failing_sibling_cancels_a_held_editor_in_a_fail_fast_group` (pending, Unix) |
| Non-blocking configured editor refused (lifecycle `edit`, wrapper `--edit`) | `…::an_edit_refuses_a_configured_editor_that_cannot_wait`; `…::wrapper_edit_refuses_a_configured_editor_that_cannot_wait` |
| AC7/AC9 terminal handoff: typed line read, three terminal streams, next action follows, `stty` restored | `level2_edit_action_a_terminal_editor_reads_the_pane_then_the_stack_continues` |
| AC10 interrupt while a terminal editor is open: tree reaped, `no_error` does not resume, file kept, terminal restored | `level2_edit_action_an_interrupt_reaps_the_open_editor_and_returns_the_terminal` |

Not covered: a sequence `timeout:` cancelling an open edit (no such Claudine timeout exists; see
findings). The input-robustness matrix does not apply: no file-format or configuration reader changed.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli, darkmatter-lsp, test-toolkit]`,
  `change_class: package` (Phase 4's changes are still uncommitted in this tree, hence the Darkmatter
  packages); gates run.
- `claudine`: `just lint` clean after adding the error-guard corpus entry (its first run failed on
  `error_guards::production_sources_pass_every_scan_backed_guard`, which requires a corpus entry for the
  new code). `just test`: 12070 passed, 15 skipped; 2 recorded at or above 5 s on a loaded host
  (load ≈ 74 on 16 cores), both outside this phase's code and cleared by re-measuring alone:
  `claudine_schemas::shared_payload_properties_have_identical_types` 5.65 s → 2.23 s alone,
  `container_schema_recursion::deep_container_deferral_does_not_hide_literal_or_shape_errors_in_either_phase`
  5.60 s → 2.33 s alone. `just test-l2 edit_action`: 2 passed (tmux, detached, no focus).
- `darkmatter` (no source change this phase): `just lint` clean; `just test` 16587 passed, 10 skipped;
  2 recorded at or above 5 s at load ≈ 128, cleared alone:
  `expression_type_scope::every_guard_side_admits_every_value_the_evaluator_sends_there::scalar_unions`
  6.80 s → 1.70 s, `glob_implementation_guard::file_references_have_one_glob_implementation` 5.93 s → 1.33 s.
- Dependents: no package outside the `claudine` area depends on `claudine`/`claudine-cli`; `claudine-gen`
  ran inside `claudine`'s `just test`.
- Cross-OS (process spawns, editor identification by file name, paths):
  `just cross-check claudine-cli --os windows lifecycle_edit_action` 13/13 passed (the four
  interrupt/PTY tests are Unix-only by design).

## Phase 6

### What was built

- **Schema definition (R24).** `darkmatter/schemas/partials/lifecycle.yaml` gains hand-written `edit-short` (a non-blank string, or a one-item list of one) and `edit-long` (`file` required and non-blank; `create_new`, `gui`, `no_error` booleans), outside every generated region, with hover descriptions, and references `edit` in `expression`, `bundle`, and `initialize-bundle`. `md-lifecycle.yaml` adds `edit` to `md-expression`, `md-bundle`, `md-initialize-bundle`, and the `md-unknown-action` exclusion list. Claudine's `action.yaml` types it in `expression` and all three bundles (`bundle`, `initialize-bundle`, `cancelled-bundle`); `lifecycle.yaml` in `loop-bundle` and `loop-single`.
- **`non-null` arms and flags** (a departure from R24, see decisions): without it the schema admitted `edit: null`, `gui: null`, and `create_new: null`.
- **Schema validation applies the parser's own `edit` rules** (a departure, see decisions). The conversion engine read `edit: 5`/`edit: [1]` as text and `gui: "yes"` as `true`; `md compose` refused all three while `md schema validate` reported them valid (reproduced with the built `md` before the fix). The rules now live once, in `lifecycle/action_shape.rs::{edit_file_problem, edit_flag_problem}`; the parser (`edit_file`, `parse.rs`'s flag check) and `markdown/schemas/bound_operands.rs::report_refused_targets` both call them, so the schema problem carries the parser's message at the operand's own pointer.
- **Generator and projection.** `schema_projection.rs` already treated `LIFECYCLE_ONLY_VERBS` as hand-owned (Phase 3's `hand_owner` branch); `just regen-lifecycle-schema` left every generated region unchanged, and the generator example needed no edit.
- **Artifacts.** Root `just shipped-schemas` regenerated Darkmatter's and Claudine's `shipped.json`; `just regen-lifecycle-artifact` (claudine) produced no byte change to `globals.lifecycle.json`, as expected; a second `just shipped-schemas` reported "every shipped artifact is current" and the bytes were compared equal. `PREPARATION_REVISION` not bumped (no preparation code changed).
- **Goldens.** The recursive-schema goldens that embed the live baseline (`darkmatter-schemas-partials-fm`, `darkmatter-schemas-darkmatter-schema-body`, `effective-baseline-*`, `embedded-darkmatter-base`) were re-blessed with the README's command; the diffs are only the new `edit` properties and `edit` in the patterns' exclusion lists.
- **Exemptions removed.** `lifecycle_schema_parity.rs` (md open arm) and `claudine_schemas.rs::hand_verbs_equal_the_parser_inventory` no longer exempt `LIFECYCLE_ONLY_VERBS`; the parity test now also asserts `edit` is among md's provided verbs. `claudine_schemas.rs::bundle_values_are_the_category_short_forms` counts 21 hand long forms (was 20). The six `pending_until_the_schema_knows_edit` call sites in `claudine/cli/tests/l1/lifecycle_edit_action.rs` (the log said seven tests; there were six call sites) are unwrapped, and the helper, `SCHEMA_ORACLE`, and the module doc's "Pending contracts" section are deleted; all 17 tests in the file pass unwrapped.

### Requirement-to-test mapping

| Requirement | Test | Boundary / rung |
|---|---|---|
| The same malformed shapes are refused by schema validation and the runtime parser, at the action | `darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs::every_refused_cell_fails_schema_validation_at_the_action_and_the_parser` (28 shapes × `initialize` and an untaken `composed` `else` branch) | library public API (F2) |
| Valid shapes and the unedited motivating workflow are accepted by both | `…::every_accepted_cell_passes_schema_validation_and_the_parser`, `…::the_motivating_workflow_passes_schema_validation_and_the_parser` | F2 |
| `md schema validate` and `md compose` wiring: non-boolean `gui` and `edit: []` refused at the authored line/column and property, control validates clean, no editor launch, nothing created | `darkmatter/cli/tests/l1/compose_edit_action.rs::schema_validate_and_compose_refuse_malformed_operands_alike_without_launch` | spawned CLI (F3) |
| DMLS completion offers `edit` in `initialize` and `composed` (md) / `start` (Claudine); hover shows the schema description | `darkmatter/dmls/tests/l1/edit_action_editor.rs::{md,claudine}_completion_offers_edit_…_and_hover_describes_it` | LSP session (F3) |
| DMLS diagnoses malformed `gui` (untaken branch) and blank `file` (taken branch) at the authored location, md and Claudine (host artifact loaded) | `…::{md,claudine}_malformed_operands_are_diagnosed_where_written_in_both_branches` | F3 |
| Control: the motivating workflow has no schema diagnostic in either host | `…::{md,claudine}_motivating_workflow_raises_no_schema_diagnostic` | F3 |
| No analysis path launches an editor or writes a file | every `edit_action_editor.rs` session runs with `EDITOR`/`VISUAL` naming a recorder script and compares the repository tree before and after | F3 |
| Shipped artifacts and host profile are fresh | `darkmatter/lib/tests/l1/shipped_artifacts.rs`, `claudine/lib/tests/l1/shipped_artifacts.rs`, `claudine/lib/tests/l1/host_catalog.rs` | F2 |
| Claudine's schemas accept `edit` everywhere its parser does | the unwrapped `claudine/cli/tests/l1/lifecycle_edit_action.rs` tests; `claudine_schemas.rs::{hand_verbs_equal_the_parser_inventory, bundle_values_are_the_category_short_forms}` | F3 / F2 |

Input robustness: no new loader (R25). The two validators (`file`; the shared `create_new`/`gui` boolean) are walked through every applicable field shape in the parity matrix: absent, null, wrong type whole field, wrong type one element, every element, empty/blank, object; each flag has a control (accepted cells) and invalid shapes. Smell grep over the new and changed code (`serde(default)`, `unwrap_or_default()`, `filter_map(.. as_str())`, `.ok()`) found nothing that parses a load-bearing field; the `unwrap_or_default` in `edit_action_editor.rs` reads a test response, and `bound_operands.rs` uses `if let Ok` only to skip a value whose template fails to parse, which `report_malformed` already reports.

### Storage incident

Mid-phase the APFS container hit 134 MiB free and commands could not start. This worktree's `target/debug/incremental.kache-auto` held 64 GB; `scripts/sweep.sh` (run at phase start, cleaning 8.4 GB) does not cap it because its incremental pass does not match that directory name. I deleted that directory (this worktree's own rebuildable cache); free space went to 64 GB.

### Gates and verification (macOS host)

- `cd darkmatter && just lint`: green (one `needless_borrows_for_generic_args` in the new CLI test fixed first).
- `cd darkmatter && just test`: 16,597 passed, 10 skipped; "L1 run recorded: … none at or above 5.0s"; `just slow-tests fresh` confirms `target/slow-tests/current.md` describes this tree (0 slow).
- `cd claudine && just lint`: green. `cd claudine && just test`: 12,070 passed, 15 skipped, none at or above 5.0 s. (Three `claudine_schemas` tests took 5.5–6.5 s in an earlier targeted run while the host's load average was about 25; they were under 5 s in the full run on a quiet host.)
- Dependent package: `messenger`'s research tests validate documents through `DarkmatterSchemas` (`cargo nextest run -p messenger --features desktop,research -E 'binary(/research/)'`): 166 passed. No other dependent package reads lifecycle schemas or schema validation.
- Root `just shipped-schemas` run twice: the second run reported every artifact current and the bytes compared equal; `claudine/schemas/globals.lifecycle.json` unchanged by `just regen-lifecycle-artifact`.
- Cross-OS: not run on the rigs. The change is schema text, validation logic with no path or process handling, and tests; the one OS-specific piece is the DMLS test's never-launched recorder script (`.cmd` on Windows, `/bin/sh` elsewhere), which CI's Windows and WSL2 legs will compile and run.
- `scripts/ci/affected_scope.py` over this phase's files selects claudine, claudine-cli, darkmatter, darkmatter-cli, darkmatter-lsp, test-toolkit, and simplified-schema (`change_class: package`). The last two only for tests that read the shipped schema chain: `simplified-schema` `shipped_corpus`/`shipped_artifacts`/`scanner_corpus` (104 passed) and `test-toolkit` `shipped_schemas_coordinator_contracts` (5 passed).
- No pre-existing failures were seen.

## Phase 7

### What was built

- **Docs (Wave 14).** Darkmatter's canonical lifecycle page (`docs/topics/lifecycle/index.md`) documents `edit`: both forms, `create_new`, `gui`, the host requirements, file creation, blocking, failures, cancellation (including GUI editors that stay open and the `gui: false` guarantee), and how it differs from wrapper `--edit`, `stop`, and initialize-time `skip`. It includes a Mermaid diagram of the edit lifecycle (resolve → eligibility → create → launch → wait or cancel → recheck). `error-handling.md` covers the failure classes; `side-effects.md`, `caching.md`, `loading-data.md`, `cli/compose.md`, `cli/edit.md`, and the README cross-link to it. Claudine's `flow-control/lifecycle.md`, `flow-control/sequences.md` (parallel-group and structured-output behavior), `schemas.md` (authoring reference), and `getting-started/index.md` describe it from Claudine's side. No doc names this feature directory.
- **Skills.** The repo skills `darkmatter/lifecycle.md` (`LifecycleHost::editor`, `EditorService`, `LIFECYCLE_ONLY_VERBS`), `claudine/SKILL.md` (verb list), and `os/SKILL.md` plus `os/windows.md` (Job-object assignment window, shared-console Ctrl-C, `.cmd` shim launch through `find_program`). The user-scope `use-darkmatter` and `use-claudine` skills live outside the repository and were updated in place.
- **Drift pass (Wave 15).** Comment-only corrections in `editor/{launch,mod,resolve}.rs`, `process_tree.rs`, `lifecycle/executor.rs`, and `fake_editor.rs`, with one behavior change: the hint on `EditorError::Io` (`editor/mod.rs`) named only the temp-file location, but that variant now also reports failures to contain or wait for the editor's process, so the hint names both. That string is user-visible and belongs in its own commit, apart from the comment-only changes.
- **Smell grep** over the new files found nothing beyond what Phase 6 recorded.

### Acceptance criteria to tests

Earlier phases' tables give full file paths; the names below are the anchor tests for each criterion.

| AC | Tests (host proven on) |
|---|---|
| AC1 motivating workflow, outside-prompt, race, occupied `create_new` | md: `compose_edit_action::{the_motivating_workflow_creates_and_opens_the_spec_then_skips, a_prompt_outside_the_launch_directory_edits_the_absolute_target_inside_its_root, a_name_taken_after_the_check_fails_create_new_and_opens_without_it, a_create_new_name_held_by_a_directory_or_symlink_fails_without_launch}`; Claudine: `lifecycle_edit_action::{compose_motivating_workflow_creates_and_opens_the_spec_then_skips_the_provider…, compose_edits_inside_the_launch_workspace_from_a_prompt_kept_elsewhere, sequence_root_initialize_runs_the_motivating_workflow_before_any_task, sequence_prompt_task_runs_the_motivating_workflow_and_launches_no_provider}` (macOS) |
| AC2 forms, literal/interpolated paths, located diagnostics, bare `fqn` literal | `darkmatter l1 lifecycle_edit_action::*` (Phase 3 table), `lifecycle_edit_schema_parity::*`, `edit_action_editor::{md,claudine}_malformed_operands_are_diagnosed_where_written_in_both_branches` (macOS) |
| AC3 fake editor, path forms, byte preservation, creation, containment, URL refusal | `test-toolkit fake_editor::*`; `editor_launcher::the_path_is_one_argument_after_the_editor_arguments`; `editor_service::{an_existing_regular_file_is_opened_with_its_bytes_preserved, an_eligible_terminal_editor_creates_missing_parents_and_succeeds, a_name_created_concurrently_is_opened_not_replaced, a_target_outside_the_mutation_root_is_refused_before_creation}`; `fs_write::tests` race; `compose_edit_action::a_symlink_escaping_the_root_is_refused_before_anything_is_created` (Unix); `executor tests::a_url_is_refused_without_reaching_the_editor` (macOS) |
| AC4 shared precedence, Sniff surfaces and wait flags, `gui` resolution, non-blocking never selected | `sniff editor_traits::tests::*`; `darkmatter editor::resolve::tests::*`; `editor_resolution::*`; Claudine `lifecycle_edit_action::{an_edit_refuses_a_configured_editor_that_cannot_wait, wrapper_edit_refuses_a_configured_editor_that_cannot_wait}` (macOS) |
| AC5 defined failures, `no_error` continuation, nothing created on refusal, post-exit recheck, atomic save | `editor_service::*` (Phase 2 table); `compose_edit_action::{editor_failures_name_the_action_property_and_target_and_stop_the_stack, a_terminal_editor_without_an_interactive_terminal_is_refused_before_creation}`; `executor tests::an_editor_failure_is_a_located_dispatch_failure_that_no_error_suppresses` (macOS) |
| AC6 passive surfaces and untaken branches never launch or create | `compose_edit_action::{passive_surfaces_and_untaken_branches_never_launch_or_create, library_composition_never_runs_an_edit}`; `darkmatter-lsp no_side_effects::expression_validation_runs_no_actions_shells_file_effects_or_lazy_providers`; every `edit_action_editor.rs` session (recorder `EDITOR`, tree compared) (macOS) |
| AC7 ordering in every event and container, parallel members, no-display refusal, terminal handoff, post-edit reads | `compose_edit_action::{edit_runs_in_order_in_composed_failed_and_finalize, a_read_after_the_edit_sees_changed_and_deleted_data}`; Claudine `lifecycle_edit_action::{initialize_runs_each_edit_in_order_with_the_actions_around_it, compose_runs_edit_in_order_in_start_success_and_finalize, a_parallel_member_refuses_a_terminal_editor_before_creating_anything, a_parallel_member_runs_a_gui_editor}`; `claudine editor::tests::a_host_without_a_display_refuses_a_gui_editor_before_creating_the_file`; L2 `level2_edit_action_a_terminal_editor_reads_the_pane_then_the_stack_continues` (md and Claudine) (macOS) |
| AC8 all four OSes, Test Toolkit, package recipes, no focus | Gates below on macOS; Linux, native Windows, and WSL2 by CI only (see Cross-OS) |
| AC9 Markdown-mode isolation, structured output, terminal restoration | `compose_edit_action::{a_gui_editors_output_never_reaches_the_composed_document, structured_output_refuses_a_terminal_editor_and_silences_a_gui_editor}`; Claudine `lifecycle_edit_action::{structured_output_refuses_a_terminal_editor_and_no_error_continues, structured_output_runs_a_gui_editor_and_keeps_its_output_off_stdout}`; both L2 tests (restoration) (macOS) |
| AC10 tree termination, SIGTERM escalation, sibling cancellation, kept bytes, GUI client only | `editor_launcher::{cancellation_terminates_the_whole_tree_and_reaps_it, an_editor_ignoring_sigterm_is_killed_after_the_grace_period, cancelling_a_gui_client_leaves_an_already_running_editor_alone}`; Claudine `lifecycle_edit_action::{an_interrupt_terminates_a_held_editor_tree_and_keeps_what_…, a_failing_sibling_cancels_a_held_editor_in_a_fail_fast_group}`; L2 `level2_edit_action_an_interrupt_reaps_the_open_editor_and_returns_the_terminal` (macOS). **Sequence timeout not covered**, see departures |
| AC11 runtime, `md schema validate`, and DMLS agree; artifact freshness | `lifecycle_edit_schema_parity::*`; `compose_edit_action::schema_validate_and_compose_refuse_malformed_operands_alike_without_launch`; `edit_action_editor::*`; `shipped_artifacts.rs` (both), `host_catalog.rs` (macOS) |

### Departures from the spec

Each is recorded with alternatives in `decisions_for_review` above; the ones that change what the spec promised:

- **AC10 sequence timeout.** No Claudine timeout cancels an open editor: document `timeout:`/`--timeout` bound only the provider session, and a task's `timeout:` only its shell commands. Cancellation is proven with an interrupt and a fail-fast sibling instead. **Accepted by the author (2026-11-14)**; documented in `claudine/docs/topics/flow-control/lifecycle.md`.
- **AC9 structured output in Claudine.** Claudine prints no result envelope for an `initialize` failure, so "one valid result" is asserted as empty stdout with the refusal on stderr.
- **`md compose` mutation root** is the prompt's own directory, so a prompt in a sibling library cannot edit `{{ ctx.cwd }}/…` (Claudine confines writes to the launch workspace, matching the spec's example).
- **Schema validation applies the parser's `edit` rules** (`edit: 5`, `gui: "yes"`, `null` flags are refused by `md schema validate` and DMLS, not only at run time), and `edit`'s arms and flags are `non-null`.
- **Failure message order** is "`<property>` could not edit the file: <reason>" (target appended only when the reason omits it), so the 240-character clamp does not cut off the reason.

### Gates and verification (macOS host)

- `just lint`: green in `sniff/`, `darkmatter/`, `claudine/`. `tools/test-toolkit/` has no `lint` or `test` recipe; `cargo clippy -p test-toolkit --all-targets --all-features -- -D warnings` is green and root `just test test-toolkit` passed 499, 4 skipped.
- `just test`: `sniff/` 3,590 passed, 33 skipped (the first run failed `sniff-detection-cli::l1 cli::test_recent_commits_hash_period_walks_back_to_the_hash`, which then passed three isolated runs and the full rerun; no `sniff` source changed after Phase 1; load average was about 27). `darkmatter/` 16,597 passed, 10 skipped. Four DMLS tests crossed 5 s in that run while the host was deleting an 837 GB worktree; rerun on their own they took 1.2–1.7 s. `claudine/` 12,070 passed, 15 skipped, none at or above 5 s.
- `just test-l2`: `darkmatter/` and `claudine/` green, including both `level2_edit_action` tests in each; no window took focus.
- Root `just shipped-schemas`: reported every shipped artifact current (Phase 6 had already regenerated them). One run is sufficient: the script regenerates the whole chain in dependency order and publishes all or nothing, and `shipped_artifacts::shipped_artifact_is_fresh` in each `just test` is the standing byte-for-byte check. The plan's "run twice" step was redundant with that test and was not repeated as evidence (author ruling, 2026-11-14).
- `just ci-local --plan` schedules L1 and lint for every package this feature touches on Linux, macOS, native Windows, and WSL2, plus L2 for `claudine-cli`/`darkmatter-cli` on the hosts that have an L2 backend. The plan is branch-wide and also lists the untracked root `.darkmatter.yaml` (Darkmatter's own migration from `.darkmatter-shell-whitelist`, not part of this feature).
- Cross-OS: Linux, native Windows, and WSL2 are proven only by CI. The Windows paths (Job object with kill-on-close cleared after success, `.cmd` launcher shims, recorder scripts) were never run on a Windows host during this feature.
- Storage: the first Phase 7 gate attempt failed with ENOSPC; this run started at 66 GB free and the author freed more by deleting another worktree.

### Status

Implementation complete, ready for review. Not moved to `_completed`, and not committed.
