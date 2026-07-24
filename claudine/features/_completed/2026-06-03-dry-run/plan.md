---
phases: 7
created: 2026-06-03
start_phase: 1
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/tests/argv_normalization.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/harness/shell.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - claudine-cli
source_files_during_phase_6:
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine-cli
source_files_during_phase_7:
  - claudine/cli/tests/level2_dry_run_pty.rs
  - claudine/cli/tests/level2_dry_run_metadata_capture.rs
  - claudine/cli/Cargo.toml
docs_updated_during_phase_7:
  - claudine/docs/topics/composition.md
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/timeline.md
  - claudine/features/2026-06-03-dry-run/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/timeline.md
packages_during_phase_7:
  - claudine-cli
packages:
  - claudine
  - claudine-cli
---

# Implementation Plan — Working `--dry-run` for Claudine Composition Commands

Converts [`spec.md`](spec.md) into an ordered, dependency-aware execution plan.

## Goal

Make `--dry-run` a real, useful gate for `claudine compose`, `claudine inline-compose`,
and `claudine sequence`: run composition through provider/model resolution, then
emit the composed body to **stdout** and the finalized frontmatter + a metadata
table to **stderr** before selected-executable validation or launch wiring. The
selected provider need not be installed. Suitable for CI gating and rehearsal.

## Current State (grounding)

- The `--dry-run` flag already exists on `SharedComposeArgs`
  (`claudine/cli/src/commands/compose.rs:156-158`) and is threaded into
  `CompositionExecutionRequest.dry_run` for all three commands.
- The **current** behavior is unrelated to the spec: at the dry-run seam
  (`claudine/cli/src/commands/wrap/composition/mod.rs:1313-1344`) it calls
  `crate::output::log_dry_run(...)` which prints the *provider launch command*
  (binary, argv, env, MCP, system prompt). **This is the feature to replace.**
- Composition, shell expansion (real side effects), and shell approval all complete
  **before** the request is built, in the CLI handler:
  `resolve_shell_approvals` (`compose.rs:465-473`) → `prepare_direct_with_schema`
  (`compose.rs:508/614`). At the seam, `request.prepared.effective_frontmatter`
  (merged + interpolated + shell-expanded JSON `Value`) and `request.prepared.prompt`
  (composed body) are fully materialized.
- Provider/agent/model are resolved eagerly into `request.resolved_target`
  (`ResolvedExecutionTarget { provider, model, .. }`, `composition/types.rs:296`).
- stdout/stderr split helpers exist: `crate::log::data()` → stdout,
  `crate::log::message()` → stderr (`claudine/cli/src/log.rs:105-124`).
- Non-TTY unapproved-command handling already exists in
  `claudine/lib/src/composition/preflight.rs:114-130` (generic message, no `--dry-run`
  framing).
- Harness documents run an additional harness-command shell approval + writability
  pre-check block **after** the current seam
  (`composition/mod.rs:1414-1470`).
- The provider is actually spawned in `run_child`
  (`claudine/cli/src/commands/wrap/exec/spawn.rs:128`), reached via the harness loop
  (`harness_orch.rs`) — always downstream of the seam.
- `sequence` already threads `dry_run` per step (`wrap/sequence.rs:443`) and toggles a
  perf accumulator (`wrap/sequence.rs:81`); divider/concatenation behavior is absent.
- Compose/inline run through `run_loop_with_overrides` (`compose.rs:498`); a dry-run
  must render once and **not** iterate the loop.

## Design Decisions (assumptions — non-interactive session, no clarification available)

These resolve genuine ambiguities in the spec. Each is the least-surprising default;
flagged so a reviewer can override.

1. **Dry-run exit seam.** Move the dry-run early-return from its current location
   (`composition/mod.rs:1313`, *before* `switch_process_cwd` and harness preflight) to
   **after the harness preflight shell-approval + writability block**
   (`composition/mod.rs:~1470`), but before the harness loop / `run_child`. Rationale:
   the spec lists "Harness pre-checks run normally" and requires harness-command shell
   approvals to participate in the non-TTY gate. `switch_process_cwd` is retained for
   path fidelity (claudine is the shell's child; its CWD change does not leak to the
   user's shell). For **non-harness** documents this block is a no-op, so the effective
   seam is unchanged.

2. **inline-compose body semantics.** In normal mode inline-compose sends the composed
   `prompt` to the provider and the provider's *response* replaces the body + rewrites
   the file. In dry-run the provider never launches, so there is no generated content.
   Therefore dry-run **stdout = the composed `prompt`** (what would be sent), and the
   **file is never mutated** (`last_updated` untouched, no write-back). This matches
   "composed document body = the data product of the command."

3. **`Area` detection.** "Inside a monorepo" = `launch_workspace.package_area` is present
   and non-empty (same source as the `ctx.area` runtime variable). When absent, the
   `Area` row is omitted.

4. **Loop interaction.** When `--dry-run` is set, compose/inline bypass the iteration
   engine entirely (single composition + single render). No multi-iteration dry-run.

5. **`log_dry_run` retirement.** The old provider-command dump is removed from the
   composition path. (Confirm no other caller depends on it before deleting; if shared
   with raw provider wrappers, keep it there and only swap the composition call site.)

## Validation Conventions

- Builds/tests are **targeted**: `cargo build -p claudine-cli`, `cargo test -p claudine-cli`
  / `-p claudine`. Never root `cargo build`. Prefer the area `just` recipes
  (`just test`, `just lint`) per the **rust-testing** skill.
- L1 unit + L2 integration via `assert_cmd`/`predicates`; TTY approval-prompt behavior
  needs the L2 tmux harness (see **biscuit-test-harness** / `rust-testing` skill).
- Never run `cargo fmt` unless explicitly told.

---

## Phase 1 — Dry-run render core (metadata extraction + frontmatter/table rendering)

Foundation used by every command path. No behavior wired yet — pure, unit-testable
helpers in a new module (e.g. `claudine/cli/src/commands/wrap/composition/dry_run.rs`).

- [ ] Define a `DryRunRender` (or similar) struct capturing everything the renderer
      needs: composed body (`String`), `effective_frontmatter` (`serde_json::Value`),
      `name` / `description` (from frontmatter), resolved agent (`Option<Provider>`),
      resolved model (`Option<String>`), `yolo: bool`, `area: Option<String>`, and the
      resolved document path (for the OSC8 link).
- [ ] Implement a builder that extracts these fields from `CompositionExecutionRequest`
      (`prepared.effective_frontmatter`, `prepared.prompt`, `resolved_target`, `yolo`,
      `prep_launch_workspace.package_area`, `prepared.resolved_path`). Document where
      each field originates.
- [ ] Implement `render_frontmatter(&Value) -> String`: convert the JSON `Value` to YAML
      **using the `biscuit-file` skill** (`Json` → `Yaml`), then syntax-highlight as YAML
      using darkmatter's highlighter (per **darkmatter** skill; syntect/two-face). Output
      targets stderr.
- [ ] Implement `render_metadata_table(&DryRunRender, &Terminal) -> String` using
      `table_utils::base_table` + `biscuit_terminal::components::prose::Prose`
      (**biscuit-terminal** skill). Rows, in order:
      - **Document**: frontmatter `name` if set, else relative path to the doc; rendered
        in blue as an **OSC8 hyperlink** to the document (Prose OS8 support).
      - **Description**: frontmatter `description` in italic + dim, only if present.
      - **Agent**: resolved agent name, else `<i><yellow>interactive</yellow></i>`.
      - **Model**: resolved model, else `<i><dim>default</dim></i>`.
      - **YOLO**: `<green>true</green>` / `<red>false</red>`.
      - **Area**: only when `area` is `Some` (Decision 3).
- [ ] Unit tests (L1): table omits Description/Area when absent; agent/model fallbacks;
      YOLO true/false; name-vs-path Document selection; YAML conversion round-trips a
      representative `Value`. Assert on **semantic** content, not byte-equal SGR
      (per L2 SGR-collapsing guidance). Use `Terminal::default()` for width/capabilities.

**Validation checkpoint:** `cargo test -p claudine-cli dry_run` passes; `cargo build -p claudine-cli` clean.

---

## Phase 2 — Compose single-document dry-run path

Depends on **Phase 1**. Wires the new renderer into the compose path and removes the old
behavior.

- [x] In `compose.rs:run_compose_inner`, when `shared.dry_run` is set, **bypass the loop
      engine** (`run_loop_with_overrides`): run a single `prepare_direct_with_schema` +
      single `execute_composition_request_inner` (Decision 4). Keep schema pre-validation
      and shell preflight on the normal path.
- [x] In `composition/mod.rs`, relocate the `if request.dry_run { ... }` early-return from
      `:1313` to **after** the harness preflight block (`~:1470`), before harness-loop /
      spawn (Decision 1). Retain `switch_process_cwd`.
- [x] Replace the `crate::output::log_dry_run(...)` call with the new flow:
      build `DryRunRender`, then:
      - composed body → **stdout** via `crate::log::data` / `data_raw`.
      - highlighted frontmatter → **stderr** via `crate::log::message`.
      - metadata table → **stderr** via `crate::log::message` (after frontmatter).
      Return `SingleCompositionOutcome { exit_code: 0, .. }`.
- [x] **`--quiet` / `--silent` have no effect in dry-run**: ensure the dry-run render path
      ignores `request.quiet` / `request.silent` (always renders full output). Confirm the
      pre-flight status lines / receipt banner that *precede* the seam do not pollute
      stdout (they go to stderr already) — body must be the only thing on stdout.
- [x] Retire `output::log_dry_run` from the composition path; remove the function only if
      no other caller remains (Decision 5 — grep first). *Kept the function: still used by
      the raw provider wrappers at `wrap/mod.rs:911`; only the composition call site was
      swapped.*

**Validation checkpoint (observable):**
- `claudine compose --dry-run fixture.md > body.md` → `body.md` contains *only* the
  composed body; frontmatter + table appear on the terminal (stderr).
- `claudine compose --dry-run fixture.md` does **not** spawn any provider (assert via a
  mock binary that writes a sentinel file — sentinel must be absent).
- `--dry-run --quiet` and `--dry-run --silent` produce identical full output to `--dry-run`.

---

## Phase 3 — Shell-approval gating: TTY parity + non-TTY dry-run failure

Depends on **Phase 1/2** for the dry-run flag plumbing; logic is mostly in the lib
preflight + handler call sites. **Parallelizable with Phase 4** (different files).

- [x] Confirm TTY behavior is unchanged: in `--dry-run` with a TTY, unapproved shell
      commands trigger the *same* interactive approval prompt as normal mode (the
      approval handler is installed independent of `dry_run`). No code change expected —
      add a regression test (Phase 7) rather than new logic. *Verified: the dry-run flag
      only changes the **no-handler** branch message; when a handler is present (TTY) the
      prompt path is untouched.*
- [x] Make the non-TTY unapproved-command path **dry-run-aware**. Today
      `preflight.rs:114-130` emits a generic "requires approval but no approval handler"
      message. Thread a `dry_run` signal so that when `dry_run` is set and no approval
      handler is available, the surfaced error message is exactly:
      `Cannot dry-run: shell command 'X' requires interactive approval. Run with --yolo to auto-approve, or pre-approve the command in your configuration.`
      *Chosen path: threaded a `dry_run: bool` field into `ShellApprovalOptions` (not the
      `resolve_shell_approvals` signature). Rationale — nearly every construction already
      uses `..Default::default()`, so only the `Default` impl + one explicit test
      construction needed touching (fewest signatures). The no-handler branch in
      `preflight.rs` emits the spec message when `approval_options.dry_run`.*
- [x] Ensure the error is written to **stderr** and the process exits **non-zero**
      (composition errors already bubble to `main.rs`; verify exit code is non-zero).
      *Covered by `compose_dry_run_non_tty_unapproved_shell_emits_gate_error` (asserts
      `.failure()` + message on stderr).*
- [x] `--yolo` bypass: confirm that `--dry-run --yolo` auto-approves and proceeds to
      render (no failure). Pre-approved commands (config/whitelist) likewise pass.
      *Implemented via a new `YoloApprovalHandler` (always `AllowOnce`) installed by
      `apply_composition_shell_overrides` when `yolo` is set; blacklist still applies.
      Wired at the compose/inline template preflight and the harness preflight seam.
      Covered by `compose_dry_run_yolo_bypasses_shell_gate`.*

**Validation checkpoint (observable):**
- Non-TTY (piped) `claudine compose --dry-run doc-with-unapproved-shell.md` exits
  non-zero and stderr contains the exact spec message naming the command.
- `claudine compose --dry-run --yolo doc-with-unapproved-shell.md` renders successfully.

---

## Phase 4 — inline-compose dry-run semantics

Depends on **Phase 1/2** (reuses the render core + seam). **Parallelizable with Phase 3.**

- [x] In `run_inline_compose_inner`, apply the same loop-bypass + single-render path when
      `shared.dry_run`. *Added the `!shared.dry_run &&` guard on the
      `run_loop_with_overrides` call, mirroring `run_compose_inner`.*
- [x] Guarantee **no file write-back** in dry-run: the inline closure/write-back
      (`CompositionClosurePlan`) must be suppressed so the source file (body and
      `last_updated`) is untouched (Decision 2). Verify the write happens only on the
      post-provider path that dry-run never reaches. *Confirmed: the dry-run early-return
      (`composition/mod.rs:1455`) returns before the `else` branch
      (`composition/mod.rs:1687+`) that performs the inline write-back; no code change was
      needed beyond the loop bypass. Verified by a byte-identical file assertion.*
- [x] Confirm the inline-specific writability pre-check (`composition/mod.rs:1433-1469`)
      still runs before the dry-run exit (it is part of "harness pre-checks run normally"
      and should fail dry-run if the file is not writable — keep parity with normal mode).
      *Confirmed: the non-harness inline writability check (`composition/mod.rs:1429-1445`)
      sits above the dry-run seam at `:1455`, so it runs under `--dry-run`.*
- [x] stdout = composed `prompt`; stderr = frontmatter + metadata table (Agent row shows
      the provider that *would* generate the body). *Inherited from the shared seam
      (`DryRunRender::from_request` → `crate::log::data` for the body, `crate::log::message`
      for frontmatter + table); no inline-specific code needed.*

**Validation checkpoint (observable):**
- `claudine inline-compose --dry-run doc.md` leaves `doc.md` byte-identical (hash before
  == hash after) and prints the composed prompt to stdout.

---

## Phase 5 — sequence dry-run (dividers, concatenation, fail-fast)

Depends on **Phase 1** (render core) and the compose dry-run path (**Phase 2**), since
each step flows through `execute_composition_request_inner`.

- [x] In `wrap/sequence.rs:execute_sequence`, when `shared.dry_run`, drive each step
      through the per-step dry-run render (each already gets `dry_run: true` at `:443`).
      Each document: frontmatter + metadata table → **stderr**; composed body → **stdout**.
      *No code change needed: each step's `execute_composition_request_inner` already hits
      the shared post-preflight dry-run seam (`composition/mod.rs:1455`) which writes
      body → stdout (`crate::log::data`) and frontmatter + table → stderr.*
- [x] Write a section divider to **stderr** between documents:
      `=== Document N of M ===` (e.g. before each document's metadata block, matching the
      spec example `=== Document 2 of 3 ===`). Decide and document whether the divider
      precedes every doc or only appears *between* docs; spec wording "between each
      document" → place before docs 2..M (confirm with a test).
      *Implemented in the Phase 2 execution loop: when `shared.dry_run && step_index > 0`
      a dim `=== Document {N} of {M} ===` divider (N = `step_index + 1`, M = `total_steps`)
      is emitted to stderr via `crate::log::message` **unconditionally** (quiet/silent must
      not suppress dry-run output). No divider precedes the first document. Covered by
      `sequence_dry_run_concatenates_bodies_with_dividers`.*
- [x] **stdout** accumulates all composed bodies in sequence order (concatenation is the
      natural result of each step writing its body to stdout — verify ordering and that no
      step interleaves stderr content into stdout).
      *Verified: each step writes its body to stdout via `crate::log::data`; the dividers,
      frontmatter, and table all go to stderr. `sequence_dry_run_concatenates_bodies_with_dividers`
      asserts the body marker appears exactly once per step on stdout.*
- [x] **Fail-fast on composition error**: if any document fails during composition, render
      the error to **stderr** and stop the sequence immediately (same as normal mode — this
      already holds via `FAIL_FAST`/error propagation; verify it triggers for a mid-sequence
      schema/missing-file error and exits non-zero).
      *Verified via `sequence_dry_run_fail_fast_on_composition_error`: a `$schema`-required
      property the frontmatter does not satisfy aborts during Phase 1c (per-step compose),
      surfaces an aggregated missing-properties report on stderr, exits non-zero, and never
      launches a provider. (Note: composition happens up front in Phase 1c before any Phase 2
      render, so the abort precedes — rather than follows — any per-document render.)*
- [x] `--quiet` / `--silent` have no effect on sequence dry-run output (same invariant as
      Phase 2).
      *The dry-run render seam ignores `request.quiet` / `request.silent`, and the divider is
      emitted unconditionally. Covered by `sequence_dry_run_quiet_and_silent_are_no_op`.*

**Validation checkpoint (observable):**
- `claudine sequence --dry-run multi.md > bodies.md` → `bodies.md` holds all bodies in
  order; stderr shows `=== Document N of M ===` dividers + per-doc frontmatter/table.
- A 3-doc sequence whose 2nd doc has a schema error stops after doc 1's render, prints the
  error to stderr, exits non-zero (doc 3 never rendered).

---

## Phase 6 — Cross-cutting hardening (errors → stderr, exit codes, quiet/silent matrix)

Depends on Phases 2–5. Consolidation + invariant verification across all paths.

- [x] Audit all dry-run error surfaces (schema validation failure, missing file,
      `ShellCommandDenied`, harness plan parse failure, writability failure) → confirm each
      renders to **stderr** and exits **non-zero** for compose, inline-compose, and
      sequence. Adjust any path that leaks an error to stdout.
      *Audit result: no source change needed. The dry-run seam
      (`composition/mod.rs:1455`) emits the body via `crate::log::data` (stdout) and the
      frontmatter + table via `crate::log::message` (stderr); every `?`-propagated error
      reaches `render_top_level_error` (`main.rs:179`), which writes only to stderr via
      `log::message` / `log::error`. Body bytes are the sole stdout writes on the dry-run
      path. Consolidation tests added: `compose_dry_run_missing_file_errors_to_stderr_with_clean_stdout`
      and `inline_compose_dry_run_schema_error_to_stderr_with_clean_stdout` (each asserts
      non-zero exit, empty stdout, named error on stderr; the inline case also asserts the
      source file is untouched). Sequence + non-TTY `ShellCommandDenied` surfaces were
      already covered in Phases 3/5.*
- [x] Verify the data/status discipline end-to-end: **stdout carries body bytes only**;
      every status/banner/preflight line and the frontmatter+table go to **stderr**
      (consistent with the repo "verbose vs debug" / "CLI output sections" conventions).
      *Covered by `compose_dry_run_body_only_on_stdout_metadata_on_stderr`: stdout contains
      the composed body and none of the metadata-table labels / frontmatter keys; stderr
      carries the highlighted frontmatter plus the Document/Agent/Model/YOLO rows.*
- [x] Build the `--quiet` × `--silent` × `--dry-run` matrix for all three commands and
      assert dry-run output is invariant under quiet/silent.
      *Added `compose_dry_run_quiet_and_silent_are_no_op` and
      `inline_compose_dry_run_quiet_and_silent_are_no_op` (both iterate `--quiet` + `--silent`);
      `sequence_dry_run_quiet_and_silent_are_no_op` from Phase 5 completes the matrix.*
- [x] Run `just lint` for the claudine area; resolve clippy/doc warnings introduced by the
      new module. Update any `///`/`//!` docs touched (authoring-discipline rule).
      *`just lint` (claudine + claudine-cli) is clean; no source/doc changes were required —
      Phase 6 added only integration tests.*

**Validation checkpoint:** `just test` + `just lint` for the claudine area are green.
*`just test` → 1457 claudine-cli tests pass (library tier green first); `just lint` clean.*

---

## Phase 7 — Tests & documentation

Depends on all prior phases. Closes every acceptance criterion with an automated test and
updates docs.

- [x] **L1 unit tests** (render core already covered in Phase 1) — add metadata builder
      tests against synthetic `CompositionExecutionRequest`s. *Deviation (Rule 2 — Simplicity):
      `CompositionExecutionRequest` has ~50 fields with no `Default` impl or test builder, and
      nested non-`Default` members (`PreparedComposition`, `EffectiveSelectionHints`,
      `CompositionClosurePlan`, `LifecycleConfig`, `SystemPromptArgs`). A synthetic-request
      builder would be heavy and brittle for what is a trivial field-extraction function.
      `DryRunRender::from_request` is instead validated **end-to-end** by the L2 tests (Area
      from a real monorepo CWD, agent/model/yolo/name/description all flow through it), and the
      render logic itself is fully unit-tested in `dry_run.rs`.*
- [x] **L2 integration tests** (`assert_cmd` + `predicates`, mock provider binary), mapping
      one test (or assertion group) per acceptance criterion (all added in Phases 2–6; see the
      spec Acceptance Criteria for the test-name map):
      - compose dry-run: full pipeline, no provider launch (sentinel-absent), body→stdout,
        frontmatter+table→stderr.
      - `--quiet` / `--silent` no-op under dry-run (compose + sequence).
      - shell commands executed for real; their output appears interpolated in the body.
      - non-TTY unapproved-shell exact error + non-zero exit; `--yolo` bypass.
      - schema failure / missing file → stderr + non-zero.
      - metadata table contains Document (OSC8), Description, Agent, Model, YOLO, Area
        (Area present only inside a monorepo fixture).
      - sequence: all docs rendered, dividers on stderr, bodies concatenated in order,
        mid-sequence failure stops immediately.
      - inline-compose dry-run: file unchanged (hash equality), prompt → stdout.
- [x] **L2 TTY test**: interactive approval prompt appears under `--dry-run` exactly as in
      normal mode. *Implemented as a PTY test (`expectrl`), matching the established
      composition-prompt L2 pattern in `level2_schema_prompt_pty.rs` (this repo uses `expectrl`
      PTY sessions, not a tmux harness, for interactive composition tests). New file
      `cli/tests/level2_dry_run_pty.rs` →
      `level2_pty_dry_run_shell_approval_prompt_appears_and_allows`: gated with
      `require_level!(Level::L2, pty_available(), …)`, drives an unapproved `::shell` command
      under `compose --dry-run`, waits for the `Shell Approval Required` marker (proving the
      handler fires identically to normal mode — `dry_run` only changes the no-handler branch),
      sends "Allow once", and asserts the command ran for real (its output in the body) while
      the provider stub never launched. Semantic substring assertions, ANSI-stripped.*
      *Review-1 follow-up: added `level2_pty_dry_run_approval_prompt_matches_normal_mode` to the
      same file — it captures the rendered approval-prompt region for one shared fixture in both
      normal and `--dry-run` mode and asserts the two are byte-identical (ANSI-stripped), proving
      "exactly as in normal mode" by direct comparison rather than a single-mode existence check.*
- [x] Update docs (drift maintenance):
      - `claudine/docs/topics/composition.md` — added a `## Dry Run` section (pipeline scope,
        stdout/stderr split, metadata-table rows, non-TTY gate, sequence behavior). *No
        frontmatter on this file, so no hash to regenerate.*
      - `.claude/skills/claudine/cli-reference.md` — added a `### --dry-run` subsection under
        Composition Commands documenting the real semantics. *This file has no `hash:`
        frontmatter (only `SKILL.md` does, and `SKILL.md` was not edited), so no hash
        regeneration was required.*
      - `.claude/skills/claudine/timeline.md` — added a `## 2026-06` entry.
- [x] Mark all spec **Acceptance Criteria** checkboxes covered by a passing test; note any
      criterion deferred or altered by a Design Decision. *All 14 criteria checked with their
      mapped test names.* *Review-1 follow-up: the metadata-table styling/OSC8 criterion now has
      real terminal-emulator capture coverage in `cli/tests/level2_dry_run_metadata_capture.rs`
      (new `biscuit-test-harness` dev-dependency). `level2_dry_run_metadata_table_renders_styled_in_tmux`
      drives `compose --dry-run` inside a tmux pane and asserts the blue Document, italic+dim
      Description, and red `false` YOLO SGR from `frame.raw`; `level2_dry_run_document_cell_renders_osc8_link_in_wezterm`
      asserts the Document cell emits a real OSC8 `file://` hyperlink through WezTerm's capture
      path. Both run with `FORCE_COLOR=1` so styling is emitted via the optimistic terminal
      regardless of host detection, and both are skip-clean when their backend is absent.*

**Validation checkpoint (final):** every spec acceptance criterion has a corresponding
green test; `just test` + `just lint` for the claudine area pass; manual smoke of the four
observable scenarios (compose redirect, inline no-mutation, sequence concatenation, non-TTY
gate) confirmed.

---

## Dependency & Parallelism Summary

```
Phase 1 (render core) ──┬─> Phase 2 (compose) ──┬─> Phase 5 (sequence) ─┐
                        │                       │                       ├─> Phase 6 ─> Phase 7
                        ├─> Phase 3 (shell gate) ┘ (|| Phase 4)          │
                        └─> Phase 4 (inline)  ───────────────────────────┘
```

- **Phase 1** is the critical foundation; start here.
- **Phase 3** and **Phase 4** are parallelizable (distinct files: lib preflight + handler
  call sites vs inline handler + closure suppression).
- **Phase 5** needs Phases 1 + 2.
- **Phase 6** consolidates after 2–5; **Phase 7** is last.

## Risk Notes

- **Seam relocation (Decision 1)** is the highest-risk change — moving the early-return past
  `switch_process_cwd` and the harness preflight. Verify harness fixtures still behave
  (writability + harness-command approval) and that no provider spawn occurs. If relocation
  proves invasive, fall back to the original seam (`:1313`) for non-harness docs and only
  run the harness preflight separately for harness docs.
- **Loop bypass (Decision 4)**: confirm `run_loop_with_overrides` is not relied upon for
  side effects the dry-run still needs; the single-render path must reproduce the
  per-iteration `prepare` semantics for the (single) render.
- **`log_dry_run` removal**: grep all callers before deleting; raw provider wrappers may
  share it.
