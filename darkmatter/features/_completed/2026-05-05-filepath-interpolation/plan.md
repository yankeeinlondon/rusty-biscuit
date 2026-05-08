---
phases: 5
created: 2025-01-24
start_phase: 1
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/types.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - darkmatter
---

# Filepath Interpolation — Execution Plan

## Overview

Implement filepath interpolation for the Darkmatter compose pipeline per the functional specification. This adds a new **Finalization** phase (root-only) and two new operations: **Link Resolve** (Inline-Pre) and **Link Normalization** (Finalization).

---

## Phase 1: Foundation — Type System & Configuration

**Goal:** Add the new phase, operations, and configuration fields so downstream phases can compile.

**Dependency:** None. This phase gates all others.

| Step | Task | Observable Completion |
|------|------|----------------------|
| 1.1 | Add `Finalization` variant to `ComposePhase` enum in `types.rs` | `ComposePhase` has 4 variants |
| 1.2 | Add `LinkResolve` and `LinkNormalization` variants to `ComposeOperation` enum | `ComposeOperation` has 15 variants |
| 1.3 | Update `ComposeOperation::COUNT` from `13` to `15` | Constant reflects new total |
| 1.4 | Update `ComposeOperation::index()` with discriminant indices `13` and `14` for new ops | Indices are contiguous and unique |
| 1.5 | Update `ComposeOperation::phase()` — map `LinkResolve` → `InlinePre`, `LinkNormalization` → `Finalization` | Match arms cover all variants |
| 1.6 | Update `ComposeOperation::default_order()` — insert `LinkResolve` at end of Inline-Pre block, `LinkNormalization` as sole Finalization op | Order matches spec (Link Resolve before transclusion, Link Normalization after all other phases) |
| 1.7 | Update `Display` impl for `ComposePhase` to include `"finalization"` | `format!("{}", ComposePhase::Finalization)` yields `"finalization"` |
| 1.8 | Add `env_path_whitelist: Vec<String>` to `ComposeOptions` | Field exists with default empty vec |
| 1.9 | Add builder method `with_env_path_whitelist(paths: Vec<String>) -> Self` on `ComposeOptions` | Method chains and sets field |
| 1.10 | Add default known whitelist entries (`PROJECT_ROOT`, `DOCS_BASE`) as fallback when user config is empty | Hardcoded defaults applied when field is empty |

**Checkpoint:** `cargo check` passes in `darkmatter/lib` with no errors.

---

## Phase 2: Link Resolve — Inline-Pre Implementation

**Goal:** Convert all local links in a document to absolute paths during the Inline-Pre stage.

**Dependency:** Phase 1 complete.

| Step | Task | Observable Completion |
|------|------|----------------------|
| 2.1 | Create `darkmatter/lib/src/markdown/compose/link_resolve.rs` | File exists |
| 2.2 | Implement `link_resolve(markdown: &mut Markdown, options: &ComposeOptions) -> ComposeResult<()>` | Function signature compiles |
| 2.3 | Extract all references from markdown content using existing `reference/` subsystem (local.rs + html.rs) | Returns `Vec<ReferenceRecord>` with accurate byte spans |
| 2.4 | Filter to link-like references only: `ReferenceKind::Hyperlink`, `ReferenceKind::Image`, and HTML equivalents (`<a href>`, `<img src>`, `<video src>`, `<audio src>`, `<source src>`, `<iframe src>`, `<link href>`) | Filter logic covers all spec'd syntaxes |
| 2.5 | For each filtered reference with `ReferenceTarget::LocalPath`, resolve to absolute path via `biscuit_file::FileReference::new(raw).resolve()` (reuse resolver.rs patterns) | Absolute `PathBuf` returned for each |
| 2.6 | Replace original link text in `markdown.content` with absolute path using byte spans from `ReferenceRecord` | Content string updated in-place |
| 2.7 | Add `ComposeOperation::LinkResolve` handler in `run_inline_pre_operation()` dispatch in `mod.rs` | Match arm calls `link_resolve()` |
| 2.8 | Add unit tests for link_resolve: markdown link, markdown image, HTML link, HTML image | Tests pass, asserting absolute path output |

**Checkpoint:** `cargo test link_resolve` passes.

---

## Phase 3: Link Normalization — Finalization Implementation

**Goal:** Convert absolute paths back to portable forms in the new Finalization stage, running only on the root document.

**Dependency:** Phase 1 and 2 complete.

| Step | Task | Observable Completion |
|------|------|----------------------|
| 3.1 | Create `darkmatter/lib/src/markdown/compose/link_normalization.rs` | File exists |
| 3.2 | Implement `find_git_repo_root(from: &Path) -> Option<PathBuf>` helper | Returns `Some(repo_root)` when inside a git repo, `None` otherwise |
| 3.3 | Implement `compute_relative_path(from: &Path, to: &Path) -> PathBuf` helper | Returns correct relative path (e.g. `../../assets/img.png`) |
| 3.4 | Implement `normalize_links(markdown: &mut Markdown, options: &ComposeOptions) -> ComposeResult<()>` | Function signature compiles |
| 3.5 | Extract absolute path references from final composed content (reuse reference extraction) | Identifies all absolute path links |
| 3.6 | **Same-repo rule:** If absolute path is inside the same git repo as the base document, replace with relative path between the two documents | Relative path substituted |
| 3.7 | **Home-dir rule:** Else if path is under `dirs::home_dir()`, replace prefix with `~/` | `~` alias substituted |
| 3.8 | **ENV-var rule:** Else collect whitelisted ENV vars that are prefixes of the path, select the one with the longest path, and substitute `${VAR}/rest` | ENV abstraction substituted |
| 3.9 | Emit warning to STDERR via `biscuit_terminal::Status` when ENV-var rule is used: *"the path {absolute-filepath} was found to be an offset of the {ENV} environment variable and will use this abstraction."* | Warning message matches spec exactly |
| 3.10 | Add `Finalization` phase execution in `run_compose_pipeline_internal()` in `mod.rs` — loop over operations where `phase() == ComposePhase::Finalization`, but **only when `runtime.depth == 0`** (root document) | Runs after InlinePost, only on root |
| 3.11 | Add `ComposeOperation::LinkNormalization` handler in the Finalization execution block | Calls `normalize_links()` |

**Checkpoint:** `cargo test link_normalization` passes.

---

## Phase 4: Integration & Testing

**Goal:** Wire the full pipeline and validate end-to-end behavior with no regressions.

**Dependency:** Phases 1–3 complete.

| Step | Task | Observable Completion |
|------|------|----------------------|
| 4.1 | Create integration test: compose a markdown document with a relative image link, verify final output has a relative path (same repo) | Test passes |
| 4.2 | Create integration test: compose a markdown document with a home-dir link, verify `~` alias in output | Test passes |
| 4.3 | Create integration test: compose with ENV-var whitelisted path, verify `${VAR}` substitution and warning captured | Test passes |
| 4.4 | Create integration test: child document (transcluded) has links — verify Link Resolve runs on child, but Link Normalization does NOT run on child (only root) | Test passes |
| 4.5 | Run full existing test suite: `cargo test` in `darkmatter/lib` | Zero regressions |
| 4.6 | Run `cargo clippy` in `darkmatter/lib` | Zero warnings |

**Parallelizable:** Steps 4.1–4.4 can be written in parallel by different agents once Phase 3 is complete.

**Checkpoint:** CI green (tests + clippy).

---

## Phase 5: Documentation

**Goal:** Update docs to reflect the new phase and operations.

**Dependency:** Phase 4 complete (or parallel with 4 if docs are draft-only until validation).

| Step | Task | Observable Completion |
|------|------|----------------------|
| 5.1 | Update `darkmatter/docs/darkmatter-compose-pipeline.md` — document the new **Finalization** phase and its root-only behavior | Markdown file updated |
| 5.2 | Create `darkmatter/docs/operations/link-normalization.md` — describe Link Normalization logic (same-repo, home-dir, ENV-var rules) | File created with examples |
| 5.3 | Update `darkmatter/docs/operations/link-resolve.md` (or create if missing) — describe Link Resolve operation | File created/updated |
| 5.4 | Update any README or feature index that lists pipeline operations | Mentions of 13 operations updated to 15 |

**Checkpoint:** Documentation accurately describes implemented behavior.

---

## Cross-Cutting Concerns

- **Logging:** All ENV-var substitutions emit the specified warning via `biscuit_terminal::Status`.
- **Security:** Strict whitelist for ENV vars — no ambient environment leakage. Default whitelist is minimal (`PROJECT_ROOT`, `DOCS_BASE`).
- **Monorepo:** Repo boundary is the git root, not the workspace member. This ensures relative links work across workspace members.
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/tests/link_interpolation_integration.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
