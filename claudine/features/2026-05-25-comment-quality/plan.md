---
phases: 6
created: 2026-05-25
start_phase: 1
packages:
  - claudine
  - darkmatter
source_files_during_phase_1:
  - CLAUDE.md
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/mcp/mod.rs
  - claudine/lib/src/protect/mod.rs
  - claudine/lib/src/stream/mod.rs
  - claudine/lib/src/system_prompt/mod.rs
  - claudine/lib/src/prompt_reporting/formatting.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
  - claudine/lib/src/prompt_reporting/user_prompt.rs
  - claudine/cli/tests/wrap_commands.rs
  - claudine/cli/tests/snapshots/wrap_commands__wrapper_reports_removed_sensitive_env_names.snap
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/style/parse.rs
docs_updated_during_phase_1:
  - CLAUDE.md
docs_created_during_phase_1:
  - docs/comment-quality.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - scripts/check-comments.sh
  - justfile
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/prompt_reporting/formatting.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
  - claudine/lib/src/prompt_reporting/user_prompt.rs
  - claudine/lib/src/prompt_reporting/frontmatter.rs
  - claudine/lib/src/prompt_reporting/precedence.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/stream/reporting.rs
  - claudine/lib/src/config/claudine_config.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - claudine
source_files_during_phase_4:
  - claudine/lib/src/actions/bash_executor.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/loop_actions.rs
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/harness/timeout.rs
  - claudine/lib/src/mcp/catalog.rs
  - claudine/lib/src/mcp/state.rs
  - claudine/lib/src/permissions/engine.rs
  - claudine/lib/src/permissions/native.rs
  - claudine/lib/src/permissions/query.rs
  - claudine/lib/src/provider/path_template.rs
  - claudine/lib/src/reporting/mod.rs
  - claudine/lib/src/stream/semantic.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - claudine
source_files_during_phase_5:
  - claudine/cli/src/perf.rs
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/completion/mod.rs
  - claudine/cli/src/completion/engine.rs
  - claudine/cli/src/commands/config_tui/tabs/services.rs
  - claudine/cli/src/commands/wrap/exec/wiring.rs
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/context.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - claudine
source_files_during_phase_6:
  - claudine/remote-signal/core/src/identity.rs
  - claudine/remote-signal/core/src/envelope.rs
  - claudine/remote-signal/daemon/src/quic.rs
  - claudine/remote-signal/daemon/src/sync.rs
  - claudine/remote-signal/daemon/src/server.rs
  - claudine/remote-signal/daemon/src/session_log.rs
  - claudine/remote-signal/daemon/src/projection.rs
  - claudine/remote-signal/daemon/src/storage.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine
---

# Plan: Comment Quality Rubric and Cleanup

This plan implements the comment quality rubric and performs a cleanup pass on the `claudine` package area as specified in `features/2026-05-25-comment-quality/spec.md`.

## Phase 1: Policy and Reference Documentation

Establish the authoritative rules and provide reference examples for the new comment quality standards.

- [ ] **Task 1.1: Update `CLAUDE.md`**
    - Add "Comment Quality" section immediately after "Rustdoc Convention".
    - Include the 8 anti-patterns and 5 positive criteria as a concise list.
    - Add the authoring-discipline rule.
    - Link to `docs/comment-quality.md`.
- [ ] **Task 1.2: Create `docs/comment-quality.md`**
    - Expand each anti-pattern and positive criterion with real before/after examples from the codebase.
    - Cite file paths for examples to calibrate judgment.
- [ ] **Task 1.3: Seed Topic Doc Links**
    - Update module-level (`//!`) docs in `claudine/lib/src/` for high-traffic areas (composition, mcp, protect, system_prompt, stream) to link to their topic docs in `claudine/docs/topics/`.

## Phase 2: Heuristic Tooling

Create a low-friction script to help identify suspicious comment patterns.

- [x] **Task 2.1: Implement `scripts/check-comments.sh`**
    - Implement checks for:
        - Long docblocks (>15 lines) on small functions (<10 lines).
        - Literal `## Arguments` headers.
        - Heavy-setup doc examples (>20 lines).
        - Redundant accessor docs (sequential short `///` on simple `pub fn` getters).
    - Ensure output is parseable (file:line:category).
- [x] **Task 2.2: Add `just` recipe**
    - Add `check-comments` to the root `justfile` to invoke the script.
- [x] **Task 2.3: Initial Baseline Run**
    - Run `just check-comments` on the current state of `claudine/` to identify cleanup targets.
    - Baseline: 52 findings (4 `arguments-block`, 1 `heavy-example`, 9 `long-doc-short-fn`, 38 `redundant-accessor`).

## Phase 3: Claudine Library Cleanup (High-Density Targets)

Perform the first pass of manual cleanup on modules identified as having high concentration of anti-patterns.

- [x] **Task 3.1: Cleanup `prompt_reporting`**
    - Target: `claudine/lib/src/prompt_reporting/*`.
    - Focus: HOW-narration, format/color narration, stale comments.
- [x] **Task 3.2: Cleanup `dispatch`**
    - Target: `claudine/lib/src/dispatch/template.rs` and `loader.rs`.
    - Focus: Redundant accessor docs, tautological examples.
- [x] **Task 3.3: Cleanup `stream`**
    - Target: `claudine/lib/src/stream/reporting.rs` and `badges.rs`.
    - Focus: Section-marker `//` comments, HOW-narration.
- [x] **Task 3.4: Cleanup `config`**
    - Target: `claudine/lib/src/config/claudine_config.rs`.
    - Focus: Redundant docs, field doc duplication in `## Arguments`.

## Phase 4: Claudine Library Cleanup (Remaining Sweep)

Complete the cleanup of the library crate.

- [x] **Task 4.1: Sweep Remaining `claudine/lib/src/`**
    - Reviewed every finding from `bash scripts/check-comments.sh claudine/lib/src/`.
    - Removed 16 redundant accessor docs across `composition/`, `dispatch/`,
      `harness/`, `mcp/`, `permissions/`, `reporting/`, and `stream/`.
    - Trimmed 3 long docblocks in `actions/bash_executor.rs`,
      `harness/timeout.rs`, and `provider/path_template.rs`, keeping the
      contract / hidden-coupling content and dropping format-narrating
      examples.
- [x] **Task 4.2: Add `#[allow(missing_docs)]` where necessary**
    - Not required — no `missing_docs` lint is enabled on the `claudine`
      library crate, so removing redundant accessor docs needs no allow.
- [x] **Task 4.3: Verify Library Build and Docs**
    - `cargo test -p claudine --lib` → 2302 passed.
    - `just lint` (claudine) → clean for both `claudine` and `claudine-cli`.
    - `cargo doc -p claudine --no-deps` warning count unchanged (21 → 21);
      none of the new warnings were introduced by this phase (verified by
      stashing and re-running).

## Phase 5: Claudine CLI Cleanup

Apply the rubric to the CLI crate.

- [x] **Task 5.1: Cleanup CLI Core**
    - Target: `claudine/cli/src/main.rs`, `perf.rs`.
    - Removed one redundant accessor doc in `perf.rs`. `main.rs` had no rubric
      violations in scope.
- [x] **Task 5.2: Cleanup CLI Modules**
    - Target: `claudine/cli/src/argv/`, `completion/`, `commands/`.
    - Removed redundant accessor docs in `commands/wrap/exec/timeouts.rs`.
    - Trimmed HOW-narration summary on `completion/mod.rs::maybe_complete`,
      preserving the hidden-coupling Notes.
    - Removed all banner-style `// -----` section-marker comments across
      `completion/engine.rs`, `commands/config_tui/tabs/services.rs`,
      `commands/wrap/exec/wiring.rs`, `commands/wrap/profile/mod.rs`, and
      `commands/context.rs`. One stale banner in `profile/mod.rs` also
      labelled a re-export that no longer sat above it (anti-pattern 8).
- [x] **Task 5.3: Verify CLI Build and Docs**
    - `bash scripts/check-comments.sh claudine/cli/src/` → empty.
    - `just lint` (from `claudine/`) → clean.
    - `cargo test -p claudine-cli` → passed.
    - `cargo doc -p claudine-cli --no-deps` → warning count unchanged from
      baseline (only `///` content was preserved; deletions were `//` banners).

## Phase 6: Final Validation

Ensure the codebase meets the acceptance criteria and the tooling reports zero findings.

- [x] **Task 6.1: Final Heuristic Check**
    - `bash scripts/check-comments.sh claudine/` → zero findings.
    - Cleared 11 outstanding `redundant-accessor` flags in `claudine/remote-signal/`
      (core/identity.rs, core/envelope.rs, daemon/quic.rs, daemon/sync.rs,
      daemon/server.rs, daemon/session_log.rs, daemon/storage.rs). Removed
      one-line docs whose semantics were already conveyed by the function
      name and return type.
    - `claudine/remote-signal/daemon/src/projection.rs::Projection::path`
      kept its doc but was rewritten so the `:memory:` sentinel return shape
      (criterion C — semantics of complex return shape) stays visible. The
      single-line phrasing also takes it off the heuristic's redundant-accessor
      list.
- [x] **Task 6.2: Documentation Verification**
    - `CLAUDE.md` "Comment Quality" section is present and links to
      `docs/comment-quality.md`; the link target exists.
- [x] **Task 6.3: Behavior Check**
    - `just lint` (claudine area) → clean for `claudine` and `claudine-cli`.
    - `just test` (claudine area) → passing.
    - `cargo nextest run -p remote-signal-core -p remote-signal-daemon -p remote-signal-client`
      → 117 tests passed (1 leaky, pre-existing).
    - Only doc comments and `///` lines were removed/rewritten; no
      functional code was touched.
