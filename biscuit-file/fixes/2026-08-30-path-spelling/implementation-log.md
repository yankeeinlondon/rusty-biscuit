---
spec: /Volumes/coding/wt/rusty-biscuit/fix-path-spelling/biscuit-file/fixes/2026-08-30-path-spelling/spec.md
plan: biscuit-file/fixes/2026-08-30-path-spelling/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity/tests.rs
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/tests/l1/magic_local_roots.rs
  - biscuit-file/lib/tests/l1/repository_scope_catalog.rs
  - biscuit-file/lib/tests/l1/finalized_reference_resolution.rs
docs_updated_during_phase_2:
  - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_3:
  - darkmatter/cli/tests/common/path_lookup_guard.rs
  - darkmatter/cli/tests/common/source_scan.rs
  - darkmatter/cli/tests/l1/path_lookup_guard.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/lib/tests/l1/path_lookup_guard.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/Cargo.toml
  - biscuit-file/lib/src/path_text.rs
  - biscuit-file/lib/tests/l1/path_lookup_guard.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/cli/tests/cli_tests.rs
  - biscuit-file/cli/Cargo.toml
  - claudine/lib/tests/l1/path_lookup_guard.rs
  - claudine/lib/tests/l1/main.rs
  - claudine/lib/Cargo.toml
  - claudine/cli/tests/l1/path_lookup_guard.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/Cargo.toml
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/os/windows.md
source_files_during_phase_4:
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/lib/tests/l1/resolution_context.rs
  - biscuit-file/cli/Cargo.toml
  - biscuit-file/cli/tests/cli_tests.rs
  - Cargo.lock
  - claudine/lib/Cargo.toml
  - claudine/lib/src/config/antigravity.rs
  - claudine/lib/src/config/backup.rs
  - claudine/lib/src/config/claude.rs
  - claudine/lib/src/config/codex.rs
  - claudine/lib/src/config/gemini.rs
  - claudine/lib/src/config/goose.rs
  - claudine/lib/src/config/kilo.rs
  - claudine/lib/src/config/kimicode.rs
  - claudine/lib/src/config/mod.rs
  - claudine/lib/src/config/opencode.rs
  - claudine/lib/src/config/pi.rs
  - claudine/lib/src/config/qwen.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/linking/paths.rs
  - claudine/lib/src/mcp/import.rs
  - claudine/lib/src/mcp/types.rs
  - claudine/lib/src/messaging/resolve.rs
  - claudine/lib/src/model_catalog/cache.rs
  - claudine/lib/src/permissions/context.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/scrub.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/protect/service/tests.rs
  - claudine/lib/src/provider/claude/behavior.rs
  - claudine/lib/src/provider/codex/behavior.rs
  - claudine/lib/src/provider/gemini/behavior.rs
  - claudine/lib/src/provider/opencode/behavior.rs
  - claudine/lib/src/reporting/paths.rs
  - claudine/lib/src/system_prompt/resolve/tests.rs
  - claudine/lib/tests/l1/context_construction_guard.rs
  - claudine/lib/tests/l1/path_lookup_guard.rs
  - claudine/cli/src/commands/uninstall.rs
  - claudine/cli/src/commands/wrap/composition/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/stream_capture.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/profile/gemini.rs
  - claudine/cli/src/commands/wrap/profile/opencode.rs
  - claudine/cli/src/completion/scopes/tests.rs
  - claudine/cli/tests/l1/context_construction_guard.rs
  - claudine/cli/tests/l1/path_lookup_guard.rs
  - claudine/cli/tests/l1/home_lookup_round_trip.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/tests/level2/level2_provider_overlay_capture.rs
  - darkmatter/lib/src/markdown/compose/context/request.rs
  - darkmatter/lib/tests/l1/context_construction_guard.rs
  - darkmatter/lib/tests/l1/nested_composition.rs
  - darkmatter/cli/tests/common/path_lookup_guard.rs
docs_updated_during_phase_4:
  - biscuit-file/docs/topics/file-references.md
  - biscuit-file/docs/dependencies.md
  - claudine/docs/topics/repo-isolation.md
  - claudine/docs/topics/system-prompt.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/os/windows.md
  - .claude/skills/os/macos.md
source_files_during_phase_5:
  - biscuit-file/lib/src/file_reference/glob/roots.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/tests/l1/path_lookup_guard.rs
  - claudine/cli/src/commands/compose/interrupt.rs
  - claudine/cli/src/completion/autocomplete_ui.rs
  - claudine/cli/tests/l1/path_lookup_guard.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/schema/status_render.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/mcp/state.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/service/tests.rs
  - claudine/lib/src/provider_overlay/tests.rs
  - claudine/lib/src/provider_overlay/write_back.rs
  - claudine/lib/src/render/prompt/system.rs
  - claudine/lib/src/render/prompt/system/tests.rs
  - claudine/lib/tests/l1/path_lookup_guard.rs
  - darkmatter/cli/tests/common/path_lookup_guard.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
  - darkmatter/lib/tests/l1/path_lookup_guard.rs
  - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
  - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
  - darkmatter/lib/tests/level2/level2_render_tree_terminal/file_links.rs
docs_updated_during_phase_5:
  - biscuit-file/docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/os/macos.md
source_files_during_phase_6:
  - biscuit-file/lib/tests/l1/implicit_relative.rs
  - claudine/lib/src/provider_overlay/selector.rs
  - claudine/lib/src/provider_overlay/tests.rs
  - claudine/cli/tests/common/completion.rs
  - claudine/cli/tests/l1/completion_compose.rs
  - claudine/cli/tests/l1/compose_prompt_tiers.rs
  - claudine/cli/tests/l1/level1_provider_overlay_home.rs
  - claudine/cli/tests/l1/mcp_cli.rs
  - claudine/cli/tests/l1/propagated_context_fixtures.rs
  - claudine/cli/tests/level2/level2_provider_overlay_capture.rs
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/tests/l1/link_interpolation_integration.rs
  - Cargo.lock
docs_updated_during_phase_6:
  - biscuit-file/docs/topics/file-references.md
  - biscuit-file/README.md
  - claudine/docs/topics/file-referencing.md
  - darkmatter/docs/topics/compose-requests.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/os/windows.md
packages:
  - biscuit-file
  - biscuit-file-cli
  - claudine
  - claudine-cli
  - darkmatter
  - darkmatter-cli
source_code:
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity/tests.rs
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/tests/l1/magic_local_roots.rs
  - biscuit-file/lib/tests/l1/repository_scope_catalog.rs
  - biscuit-file/lib/tests/l1/finalized_reference_resolution.rs
  - darkmatter/cli/tests/common/path_lookup_guard.rs
  - darkmatter/cli/tests/common/source_scan.rs
  - darkmatter/cli/tests/l1/path_lookup_guard.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/lib/tests/l1/path_lookup_guard.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/Cargo.toml
  - biscuit-file/lib/src/path_text.rs
  - biscuit-file/lib/tests/l1/path_lookup_guard.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/cli/tests/cli_tests.rs
  - biscuit-file/cli/Cargo.toml
  - claudine/lib/tests/l1/path_lookup_guard.rs
  - claudine/lib/tests/l1/main.rs
  - claudine/lib/Cargo.toml
  - claudine/cli/tests/l1/path_lookup_guard.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/cli/Cargo.toml
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/tests/l1/resolution_context.rs
  - Cargo.lock
  - claudine/lib/src/config/antigravity.rs
  - claudine/lib/src/config/backup.rs
  - claudine/lib/src/config/claude.rs
  - claudine/lib/src/config/codex.rs
  - claudine/lib/src/config/gemini.rs
  - claudine/lib/src/config/goose.rs
  - claudine/lib/src/config/kilo.rs
  - claudine/lib/src/config/kimicode.rs
  - claudine/lib/src/config/mod.rs
  - claudine/lib/src/config/opencode.rs
  - claudine/lib/src/config/pi.rs
  - claudine/lib/src/config/qwen.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/linking/paths.rs
  - claudine/lib/src/mcp/import.rs
  - claudine/lib/src/mcp/types.rs
  - claudine/lib/src/messaging/resolve.rs
  - claudine/lib/src/model_catalog/cache.rs
  - claudine/lib/src/permissions/context.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/scrub.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/protect/service/tests.rs
  - claudine/lib/src/provider/claude/behavior.rs
  - claudine/lib/src/provider/codex/behavior.rs
  - claudine/lib/src/provider/gemini/behavior.rs
  - claudine/lib/src/provider/opencode/behavior.rs
  - claudine/lib/src/reporting/paths.rs
  - claudine/lib/src/system_prompt/resolve/tests.rs
  - claudine/lib/tests/l1/context_construction_guard.rs
  - claudine/cli/src/commands/uninstall.rs
  - claudine/cli/src/commands/wrap/composition/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/stream_capture.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/profile/gemini.rs
  - claudine/cli/src/commands/wrap/profile/opencode.rs
  - claudine/cli/src/completion/scopes/tests.rs
  - claudine/cli/tests/l1/context_construction_guard.rs
  - claudine/cli/tests/l1/home_lookup_round_trip.rs
  - claudine/cli/tests/level2/level2_provider_overlay_capture.rs
  - darkmatter/lib/src/markdown/compose/context/request.rs
  - darkmatter/lib/tests/l1/context_construction_guard.rs
  - darkmatter/lib/tests/l1/nested_composition.rs
  - biscuit-file/lib/src/file_reference/glob/roots.rs
  - claudine/cli/src/commands/compose/interrupt.rs
  - claudine/cli/src/completion/autocomplete_ui.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/schema/status_render.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/mcp/state.rs
  - claudine/lib/src/provider_overlay/tests.rs
  - claudine/lib/src/provider_overlay/write_back.rs
  - claudine/lib/src/render/prompt/system.rs
  - claudine/lib/src/render/prompt/system/tests.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
  - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
  - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
  - darkmatter/lib/tests/level2/level2_render_tree_terminal/file_links.rs
  - biscuit-file/lib/tests/l1/implicit_relative.rs
  - claudine/lib/src/provider_overlay/selector.rs
  - claudine/cli/tests/common/completion.rs
  - claudine/cli/tests/l1/completion_compose.rs
  - claudine/cli/tests/l1/compose_prompt_tiers.rs
  - claudine/cli/tests/l1/level1_provider_overlay_home.rs
  - claudine/cli/tests/l1/mcp_cli.rs
  - claudine/cli/tests/l1/propagated_context_fixtures.rs
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/tests/l1/link_interpolation_integration.rs
documentation:
  - biscuit-file/docs/topics/file-references.md
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/os/windows.md
  - biscuit-file/docs/dependencies.md
  - claudine/docs/topics/repo-isolation.md
  - claudine/docs/topics/system-prompt.md
  - .claude/skills/os/macos.md
  - biscuit-file/README.md
  - claudine/docs/topics/file-referencing.md
  - darkmatter/docs/topics/compose-requests.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/SKILL.md
completed_phase: 6
implemented: true
---

# Implementation Log for 2026-08-30-path-spelling (6 phases)

## Phase 1

Phase 1 changes no source code. It records the rulings, the baseline, the
dependency facts, and the canonicalization and home-lookup inventories that
Phases 3, 4, and 5 consume.

### Rulings

The plan is `yolo: true`; no author objection was received, so every assumed
default stands.

| Ruling | Outcome |
| --- | --- |
| R1 home lookup policy (gates Phase 4) | **Option 1 accepted.** `biscuit_file::home_dir()` becomes `std::env::home_dir()` filtered to absolute paths. The spec's "Consistent home lookup" section already describes this option, so no spec revision is needed before Phase 4. |
| R2 remediation scope (gates Phase 3) | **Option 1 accepted.** Five packages only (`biscuit-file`, `claudine`, `claudine-cli`, `darkmatter`, `darkmatter-cli`). Findings elsewhere are listed below under "Separate work". No new dependencies in other areas. |
| R3 process-capture boundary | Accepted: the only direct home lookup allowed is the one inside `biscuit_file::home_dir()`. Claudine gets no direct `dirs::home_dir` / `std::env::home_dir` call. `#[cfg(test)]` code is excluded by the scanner, not excepted. |
| R4 Claudine reaches `home_dir` | **Confirmed** (see "Dependency facts"). |
| R5 captured homes in Claudine | Accepted: request-scoped code takes the injected home; ambient convenience sites call one small Claudine wrapper that delegates to `biscuit_file::home_dir()`. |
| R6 wider measurement | No performance spike; no workspace-wide remediation (R2 option 1). |
| R7 scanner implementation | **Decided: no `syn`-based scanner.** See "Dependency facts". Reuse the blanking approach of `darkmatter/cli/tests/common/source_scan.rs`. |

### Dependency facts (R4, R7)

- **R4.** `claudine/lib/Cargo.toml` declares
  `biscuit-file = { path = "../../biscuit-file/lib", features = ["yaml", "json5"] }`
  and `claudine/cli/Cargo.toml` declares `biscuit-file = { path = "../../biscuit-file/lib" }`.
  Neither sets `default-features = false`, and biscuit-file's `default`
  includes `file-reference`, so both Claudine packages already compile
  `biscuit_file::home_dir()`. The CLI already calls it
  (`claudine/cli/src/commands/steer/service.rs:46`). The Claudine **library**
  has no call today; it gets `file-reference` only through defaults. If a later
  change sets `default-features = false` there, the Phase 4 wrapper stops
  compiling, so Phase 4 should add `"file-reference"` explicitly to the
  library's feature list rather than rely on defaults (a manifest tweak, not a
  new crate).
- **R7.** `syn` (2.0, `full`/`visit`) is a dev-dependency of `claudine-cli`
  only. It is absent from `claudine`, `darkmatter`, `darkmatter-cli`, and
  `biscuit-file` manifests. Because the plan places the guard in
  `claudine/cli/tests/l1/main.rs`, `syn` *is* technically available there, but
  the guard must also be reusable as a source fixture and must blank
  `#[cfg(test)]` bodies and follow `mod name;` declarations, which
  `darkmatter/cli/tests/common/source_scan.rs` already does (`sanitize`,
  `production_sources`, `ident_offsets`, `line_at`). Ruling: build on
  `source_scan.rs`; add no crate.
- `dunce` is a normal dependency of `biscuit-file` and a dev-dependency of
  `claudine`. `dirs = "6"` is a direct dependency of `claudine`,
  `claudine-cli`, `darkmatter` (lib), and biscuit-file (optional, under
  `file-reference`). Removing the last `dirs::home_dir` from Claudine does not
  remove `dirs` there: `config_dir`/`cache_dir`/`data_dir` uses remain
  (Phase 4's XDG audit). `docs/dependencies.md` therefore needs no change.

### Existing guard that overlaps Phases 3 and 4 (important)

An ambient-state source guard already exists:
`darkmatter/cli/tests/common/context_guard.rs`, used by every
`context_construction_guard.rs` (`claudine/{lib,cli,gen}/tests/l1/`,
`darkmatter/{lib,cli,dmls}/tests/l1/`, `messenger/{lib,cli}/tests/`). Its
`AmbientState` gate already counts `std::env::home_dir`, `dirs::home_dir`,
`home::home_dir`, and biscuit-file's `home_dir`, per file, with an exact
count and a reason. Consequences:

- Every Phase 4 replacement of `dirs::home_dir` with the Claudine wrapper or
  `biscuit_file::home_dir` changes those allowlists
  (`claudine/lib/tests/l1/context_construction_guard.rs`,
  `claudine/cli/tests/l1/context_construction_guard.rs`,
  `darkmatter/cli/tests/l1/context_construction_guard.rs`). Phase 4 must update
  them in the same change or the L1 suite goes red.
- The Phase 3 "home-lookup rule" overlaps this gate. Phase 3 should consider
  expressing the new rule as a tightening of the existing gate (or calling into
  `context_guard.rs`) rather than a second, independent scanner for the same
  identifiers.

### Baseline (before any edit)

Recorded on macOS (darwin), worktree `fix/path-spelling` at `92c562dcc`.

| Area | `just test` | `just lint` |
| --- | --- | --- |
| biscuit-file | pass (1065 run, 1065 passed, 0 skipped) | pass |
| claudine | pass (8104 run, 8104 passed, 9 skipped) | pass |
| darkmatter | **fail, pre-existing** (8885 run, 8883 passed, 2 failed, 12 skipped; `just test --no-fail-fast`) | pass |

**Pre-existing darkmatter failures (not caused by this fix).** Both are
documentation-contract tests that read `.claude/skills/claudine/`, broken by
the branch's HEAD commit `92c562dcc` ("split the claudine skill into CLI
command, module map, and research index pages"), which moved text out of
`SKILL.md` into `cli-commands.md`:

- `darkmatter::l1 current_root_documentation_contract::every_required_page_explains_the_binding_time_model`
  — `.claude/skills/claudine/SKILL.md` is missing six required `ctx`/`current`
  binding-time sentences.
- `darkmatter::l1 current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects`
  — new occurrence in `.claude/skills/claudine/cli-commands.md:66`; stale
  allowlist entry for `.claude/skills/claudine/SKILL.md` (expects 2, has 0).

They are left unchanged in Phase 1 (no source or skill edits in this phase).
They will keep the darkmatter area red, so the plan's later "`just test`
passes in darkmatter" checkpoints cannot be met until they are fixed. The
smallest fix is to point the two tests' expectations at
`.claude/skills/claudine/cli-commands.md` (or restore the sentences to
`SKILL.md`); Phase 6 edits the Claudine skill anyway and is the natural place.

### Canonicalization audit (production source, five packages)

**Method.** A throwaway scanner (not committed) reused the
`source_scan.rs` rules: comments and string literals blanked; every
`#[cfg(test)]` / `#[cfg(all(test, ..))]` item blanked; every file declared by a
`#[cfg(test)] mod name;` dropped. It walked `biscuit-file/{lib,cli}/src`,
`claudine/{lib,cli}/src`, `darkmatter/{lib,cli}/src`. A raw
`grep -rn canonicalize` over the same roots gives **322** lines; the scanner
keeps **120** production hits, 78 of them the bare identifier
`canonicalize`. Every file the scanner dropped was checked by hand: 18 non-test
files contain the word only in comments or in `#[cfg(test)]` modules
(`file_reference/context.rs`, `path_identity.rs`, `argv/rule2_canonicalize.rs`,
`prep_context.rs`, `preflight/shape.rs`, `linking/paths.rs`, `provider/acp.rs`,
`stream/path_link.rs`, `darkmatter/cli/src/args/completion.rs`,
`cleanup/lists.rs`, `context/options.rs`, `context/repository_scope.rs`,
`context/request.rs`, `compose/util.rs`, `language_grammar.rs`,
`schemas/format.rs`, `simplified/convert.rs`, `triggers/matcher.rs`); the rest
are `tests.rs` / `tests/` modules.

**Destination key.** *private* — both operands get the same
canonicalization and only a bool, a dedupe-set entry, or a relative suffix
leaves the function. *onward* — the canonical path (or its text) is returned,
stored, rendered, hashed into a key another producer also writes, or put into
an error. Actions: **convert** to `biscuit_file::canonicalize_simplified`;
**except** = guard exception with the stated invariant; **ok** = already the
helper; **ignore** = not a filesystem call (scanner must not flag).

#### biscuit-file

| File:line | Enclosing item | Form | Destination | Action |
| --- | --- | --- | --- | --- |
| `lib/src/path_text.rs:62` | `canonicalize_simplified` | `dunce::canonicalize` | the helper | **allowed** (the one direct call) |
| `lib/src/file_reference/resolve.rs:1415` | `validate_containment` | `dunce::canonicalize` | private (root vs candidate containment) | convert (identical function; removes a second direct `dunce` call) |
| `lib/src/file_reference/resolve.rs:1433` | `validate_containment` | `dunce::canonicalize` | private; error carries the path | convert (same reason) |
| `lib/src/file_reference/glob/roots.rs:294` | `spelled_as_stored` | `std::fs::canonicalize` | private (only `file_name()` compared) | convert (prefix cannot matter; avoids an exception) or except "only the final component is compared" |
| `lib/src/file_reference/glob/list.rs:42,119`, `glob/roots.rs:384` | `escaping_target`, `walk`, `canonical_prefix` | helper | — | ok |

#### claudine (library)

| File:line | Enclosing item | Form | Destination | Action |
| --- | --- | --- | --- | --- |
| `composition/error/render/mod.rs:175` | `render_file_link` | method `path.canonicalize()` | onward (file URL in rendered error) | convert |
| `composition/schema/status_render.rs:11` | `schema_status_report_prose` | method | onward (file URL) | convert |
| `render/prompt/system.rs:88` | `render_system_prompt_summary` | method | onward (label + URL) | convert |
| `composition/lifecycle/control.rs:259,264` | `proxy_handoff_allowed` | `std::fs::canonicalize` ×2 | private (bool, both sides same) | except |
| `composition/resolve.rs:495,496` | `with_prompt_magic_roots` | imported `fs::canonicalize` ×2 | private (bool "local root is home") | except |
| `dispatch/loader.rs:428,430` | `load_claudine_config` | method ×2 | private (bool "same file") | except |
| `invocation_context.rs:2259` | `canonical_key` | `std::fs::canonicalize` | private (doc: keys only compared with keys) | except; Phase 5 must confirm every producer is `canonical_key` |
| `linking/hashing.rs:17` | `hash_skill_dir` | imported `fs::canonicalize` | private (walk root; only relative paths enter the hash) | except |
| `mcp/state.rs:218` | `canonical_repo_path` | imported `fs::canonicalize` | **onward, persisted** (lossy string key in MCP state on disk) | convert; note: on Windows this changes persisted keys from `\\?\C:\…` to `C:\…` — Phase 5 must check whether existing state files need a read-side fallback |
| `protect/path.rs:141,154` | `canonicalize_existing_ancestor` (pub) | method ×2 | onward (pub fn; feeds `canonical_comparison` string match against home-based patterns and `protect/service.rs:153` write-path evaluation) | convert |
| `provider_overlay/write_back.rs:194` | `WriteBack::record` | imported `fs::canonicalize` | onward (stored in `Entry`) | convert |
| `composition/sequence/preflight/mod.rs:1251`, `system_prompt/context.rs:157` | `canonical`, `canonical_or_self` | helper | — | ok |
| `permissions/{backend,engine}.rs`, `permissions/providers/{claude,codex,gemini,goose,kimi,opencode,qwen}.rs` | trait `PermissionBackend::canonicalize` | method name collision | not filesystem (policy canonicalization) | ignore; the scanner must treat `backend.canonicalize(ctx, &native)` (2 args) and `async fn canonicalize(` as non-fs, or list them as reviewed unresolved-receiver candidates |
| `permissions/query.rs:931–1097` | `canonicalize_lossy` | local lexical helper | no I/O | ignore (scanner matches whole identifier `canonicalize` only) |
| `protect/path.rs:75,244,251`, `protect/service.rs:153` | `canonicalize_native_spelling`, `canonicalize_existing_ancestor` callers | local helpers | follow their bodies | covered by the `protect/path.rs:141,154` rows |

#### claudine-cli

| File:line | Enclosing item | Form | Destination | Action |
| --- | --- | --- | --- | --- |
| `commands/compose/interrupt.rs:428` | `format_user_interrupt_message` | method | onward (file URL) | convert |
| `completion/autocomplete_ui.rs:358` | `path_label` | method | onward (display label) | convert |
| `completion/autocomplete_ui.rs:367` | `file_href` | method | onward (URL) | convert |
| `commands/wrap/provider_overlay.rs:386` | `copy_entry` | imported `fs::canonicalize` | private (cycle `visited` set) | except |
| `commands/wrap/runaway_guard.rs:292,294` | `resolve_guard_inputs` | method ×2 | private (bool "same file") | except |
| `completion/composition/compose.rs:144` | `gather_repo_dirs` | `std::fs::canonicalize` | private (dedupe set) | except |
| `completion/composition/compose.rs:189` | `render_entry_word` | `std::fs::canonicalize` | private (dedupe set) | except |
| `completion/composition/setter_value.rs:70` | `gather_committed_root` | `std::fs::canonicalize` | private (dedupe set) | except |
| `completion/setter_value.rs:159` | `gather_value_candidates` | `std::fs::canonicalize` | private (dedupe set) | except |
| `completion/operation_file.rs:295` | `gather_candidates` | `std::fs::canonicalize` | private (dedupe set) | except |
| `completion/schema_completion/mod.rs:118` | `in_repository_spelling` | method ×2 | private (only the relative suffix is rejoined to the authored root) | except |
| `argv/mod.rs:46,51` | `mod rule2_canonicalize` | module name | not a call | ignore |
| `commands/wrap/env/package_context.rs:273`, `completion/schema_completion/candidates.rs:210` | `canonical_or_self`, `match_glob_files` | helper | — | ok |

#### darkmatter (library)

| File:line | Enclosing item | Form | Destination | Action |
| --- | --- | --- | --- | --- |
| `markdown/compose/link_resolve.rs:210` | `resolve_absolute` | `std::fs::canonicalize` | onward (returned link target) | convert |
| `markdown/compose/file_links/discovery.rs:86` | local `fn canonicalize` (used at :39,40,110,119,153,199,254,255) | `std::fs::canonicalize` | onward (`display_path`, `component_root` rendered) | convert the helper body |
| `markdown/compose/context/report.rs:689` | `from_schema_advisory` | `std::fs::canonicalize` | onward (report path) | convert |
| `markdown/mod.rs:226,249,280,1060` | `source_context_for_errors`, `full_source_context_for_errors`, `loaded_source_context_for_errors`, `try_from` | method | onward (`SourceContext` absolute path in errors) | convert |
| `markdown/compose/transclusion/resolver.rs:178` | `resolve_file_reference` | `std::fs::canonicalize` | onward (`LocalTarget.canonical`; compared with document identities that `markdown/mod.rs:1071` already builds with the helper — mixed producers) | convert |
| `markdown/compose/cache/hashing.rs:22` | `compose_cache_key` | `std::fs::canonicalize` | key shared with other producers (`source_id`) | convert together with the next two rows |
| `markdown/compose/pipeline/mod.rs:148` | `run_compose_pipeline_internal` | `std::fs::canonicalize` | onward (preflight node key string) | convert |
| `markdown/compose/preflight/collect.rs:315` | `collect_recursive` | `std::fs::canonicalize` | onward (preflight source key) | convert |
| `markdown/compose/preflight/mod.rs:157` | `canonical_key` | `std::fs::canonicalize` | key compared with the strings above | convert (keep all preflight key producers on one spelling) |
| `markdown/schemas/resolve.rs:1247` | `canonical_path` | method | onward (`referenced_files`, shown in `TriggerPayloadCycle` errors) | convert |
| `markdown/schemas/triggers/assemble.rs:397` | local `fn canonicalize` (used at :308) | method | onward (compared with `referenced_files`; trigger-discovery seam) | convert together with `schemas/resolve.rs:1247` |
| `markdown/compose/expression/path_projection.rs:93,97` | `strip_prefix_any_spelling` | `std::fs::canonicalize` ×2 | private (relative suffix only) but compares a raw `abs` against a canonical root | convert (removes the mixed-spelling comparison) |
| `markdown/compose/link_normalization.rs:66,75` | `in_context_spelling` | `std::fs::canonicalize` ×2 | private (both sides canonical; result rebuilt on the authored anchor) | except |
| `markdown/compose/shell_expansion/mod.rs:510` | `cache_key` | `std::fs::canonicalize` | private (cache key; only producer; comment: mismatch costs a re-run only) | except |
| `effects/fs_write.rs:112` | `canonicalize_with_missing_tail` (used by `normalize_within`) | `std::fs::canonicalize` | private (both sides; returns the lexical `cleaned` path) | except |
| `markdown/output/terminal.rs:549,550` | `render_image` | method ×2 | private (traversal check; values only in a `tracing` warning) | except |
| `style/descriptor.rs:238` | `pub fn canonicalize(raw_path: &str)` | style-name normalizer | not filesystem | ignore (the spec's named lookalike) |
| `markdown/compose/context/capture/document.rs:66`, `expression/resolve_ctx.rs:422,423`, `markdown/mod.rs:1071`, `reference/file_tree/mod.rs:75,89`, `reference/graph.rs:974`, `reference/snapshot.rs:22`, `schemas/roots.rs:303` | various | helper | — | ok |

#### darkmatter-cli

All four hits (`commands/frontmatter.rs:305`, `commands/schema/{detect.rs:101,triggers.rs:24,validate.rs:156}`) already use the helper: **ok**.

**Totals.** convert: 29 call lines (biscuit-file 3, claudine 7, claudine-cli 3,
darkmatter 16). except: 24 call lines in 16 items (claudine 8/5, claudine-cli
10/7, darkmatter 6/4). ignore: the permissions trait family,
`style::descriptor::canonicalize`, the `rule2_canonicalize` module, and the
lexical `canonicalize_*` helpers.

**Scanner requirements discovered.** Whole-identifier match (`canonicalize`,
not `canonicalize_lossy`); method form `x.canonicalize()` with zero
arguments is filesystem, `backend.canonicalize(ctx, &native)` and
`async fn canonicalize(` are not; local wrapper functions named
`canonicalize` (`discovery.rs`, `assemble.rs`) contain the real call, so the
exception or conversion lives on their body.

### Home-lookup inventory (production source, input to Phase 4)

`dirs::home_dir` / `std::env::home_dir` / `home::home_dir`, same scanner:
claudine **35** call sites in 27 files, claudine-cli **8** in 7 files,
darkmatter 1 (`markdown/compose/context/request.rs`, `RequestSnapshot::from_process`),
biscuit-file 1 (`file_reference/context.rs`, the helper itself).

Sites the plan's Phase 4 task lists do **not** name, found by the scan:

- `claudine/lib/src/linking/paths.rs:55` (`new`, falls back to `"."`)
- `claudine/lib/src/messaging/resolve.rs:161` (`resolve_image_path`)
- `claudine/lib/src/config/pi.rs` is named; `protect/path.rs` has **3** sites
  (`:61`, `:110`, `:115`), and `mcp/import.rs` has **4** (`:263,291,307,323`,
  each falling back to `"."` when no home).
- `claudine/lib/src/dispatch/loader.rs` is the one `std::env::home_dir` site.

Several Claudine sites fall back to `PathBuf::from(".")` when no home is
found (`linking/paths.rs`, `mcp/import.rs`). That is existing behavior; Phase 4
should preserve or deliberately change it, not change it by accident.

### Separate work (outside the five packages; unchanged per R2)

Production files containing a `canonicalize(` call outside the five packages
(raw grep of `*/src/`, test modules excluded by path; not classified):

- `biscuit-terminal`: `cli/src/commands/image.rs`, `lib/src/components/{filesystem/mod.rs,image_options.rs,prose/styles.rs,terminal_image/mod.rs}`, `lib/src/render_tree/render.rs`, `lib/src/terminal.rs`
- `darkmatter/dmls`: `src/workspace/discover.rs`; `darkmatter/dmls/zed-dmls-cli/src/lib.rs` (both in the Darkmatter area but not among the five packages)
- `messenger`: `lib/src/research/load.rs`, `lib/src/research/refresh/prepare.rs`
- `research`: `lib/src/link/creation.rs`
- `sniff`: `cli/src/commands/mod.rs`, `cli/src/output/filesystem/mod.rs`, `lib/src/filesystem/{docs.rs,mod.rs,git/remote_refresh.rs,git/status.rs,git/types.rs,git/worktree.rs,repo/detection.rs}`, `lib/src/process.rs`, `lib/src/services/mod.rs`
- `tools/test-toolkit`: `src/archive_guard.rs`, `src/bin/leak-sweep.rs`
- `unchained-ai`: `lib/src/models/identity.rs`
- `worktree`: `cli/src/commands/refresh_worker.rs`, `lib/src/{cache.rs,copy_record.rs,fast_forward.rs,git.rs,include/rules.rs,remove/included.rs,worktree.rs}`
- a spike, not a package: `content-policy/features/2026-09-28-content-policy/spikes/filechanged-paths/src/main.rs`

### Phase 1 validation checkpoint

- Log exists with rulings, dependency facts, baseline, audit table, home
  inventory, and separate-work list.
- The audit table covers every production hit the scanner reported for the
  five packages; every dropped raw-grep file was confirmed to be a comment or
  `#[cfg(test)]` occurrence.
- No source, doc, or skill file changed in this phase.

## Phase 2

Phase 2 makes the Windows lexical rules testable on every host through
production code, and pins candidate ordering and provenance. All work is in
`biscuit-file/lib`.

### Pre-existing coverage (not duplicated)

`path_identity/tests.rs` already covered, through `from_windows_text` and
`windows::parse`: safe verbatim drive vs legacy (`verbatim_drive_equals_its_legacy_spelling`),
verbatim literal `/`, `.`, `..` (`verbatim_dot_segments_are_literal_names`,
`verbatim_paths_split_only_on_backslash`, `a_literal_verbatim_parent_name_is_kept_in_a_route`),
legacy vs verbatim UNC (`unc_spellings_of_one_share_are_equal`), different
legacy shares/drives as roots (`different_drives_and_shares_are_separate_roots`),
`C:\a` vs `C:a` (`drive_absolute_and_drive_relative_differ`), drive-letter case
folding with names kept (`drive_letters_are_case_insensitive_and_names_are_not`),
whole-component containment (`prefix_matches_whole_components_only`), long
verbatim inside a short root, and non-Unicode names on both grammars
(`unpaired_surrogates_stay_distinct` at the `windows::parse` level, which keeps
raw UTF-16 units, plus `non_unicode_names_stay_distinct_on_unix` and the
Windows-host `host_identity_keeps_unpaired_surrogates_distinct_on_windows`).
The "Non-Unicode names" task therefore needed no `&[u16]` entry point:
`windows::parse` *is* the production parser and already takes `&[u16]`, and
`from_windows_text` stays `&str`.

### Extraction (production change)

Candidate dedupe (`resolve.rs::dedupe_candidates`) keyed on
`normalize_components(path)`, and the `@` chain (`build_magic_chain`) on the
already-normalized `PathBuf`. Both are host-grammar `Path` comparisons, so no
Windows rule could reach them on macOS or Linux. New
`portable::path_identity::first_seen_by_identity(items, identity)` (crate
internal) keeps the first item per `PathIdentity`, in order; both sites now
call it with `PathIdentity::new(path)`. Public API and host-native
interpretation are unchanged; tests drive the same function with
`from_windows_text` identities, and the existing Windows-only
`host_identity_matches_the_portable_parser_on_windows` pins
`PathIdentity::new` to that parser on a Windows host.

**Deliberate behavior change (Windows only).** `PathIdentity` equality is a
superset of the old `normalize_components` key: in addition to everything the
old key equated, it equates (a) an unreducible verbatim drive path (too long,
which `dunce` declines to reduce) with its legacy spelling, and (b) a
verbatim UNC path with its legacy UNC spelling (`dunce` only reduces disk
paths). Such pairs used to produce two candidates/roots for one directory; they
now produce one, and the first keeps its provenance and its spelling. On Unix
the two keys agree exactly. Known limitation inherited from `PathIdentity`
(documented there, not introduced here): a verbatim name that changes meaning
without its prefix (a reserved device name, trailing dot or space) is treated
as equal to its legacy spelling. Docs updated (see below).

### Requirement-to-test mapping

| Spec requirement | Test (all hosts unless marked) |
| --- | --- |
| Safe verbatim vs legacy: same identity; first occurrence + provenance kept | `verbatim_and_legacy_duplicates_keep_the_first_occurrence_in_either_order` (both orders, distinct tags, spelling of survivor asserted) |
| Ordinary mixed separators equal | `mixed_separators_on_an_ordinary_path_share_one_identity`; ordering in `ordering_folds_only_the_windows_equalities` |
| Verbatim `/`, `.`, `..` literal | pre-existing tests above; ordering in `verbatim_literal_dot_segments_are_not_duplicates_of_collapsed_paths` |
| Legacy vs verbatim UNC equal; other server/share distinct | `a_verbatim_share_is_distinct_from_another_server_or_share`; ordering in `unc_verbatim_and_legacy_duplicates_keep_the_first_occurrence` |
| `C:` vs `D:`, `C:repo` vs `C:\repo` distinct | `other_drives_and_drive_relative_spellings_are_distinct_identities`; ordering in `ordering_folds_only_the_windows_equalities` |
| `C:\Repo` vs `C:\repo` distinct; drive case folds | pre-existing; ordering in `ordering_folds_only_the_windows_equalities` |
| `C:\repo` vs `C:\repo-old\x.md` outside | `a_sibling_with_the_boundary_as_text_prefix_is_outside` (legacy, verbatim, lowercase-slash spellings) |
| Short/long names not unified | `short_and_long_names_are_not_lexically_unified`, `short_and_long_spellings_are_both_kept`; **Windows:** `a_real_short_name_alias_is_one_directory_but_two_identities` |
| Non-Unicode names distinct | pre-existing; ordering in `ordering_keeps_distinct_non_unicode_names_on_unix` |
| Long verbatim duplicate | `a_long_verbatim_duplicate_collapses_onto_its_legacy_spelling` |
| Emitted spelling pinned apart from identity | **Windows:** `magic_local_roots::a_safe_verbatim_root_is_emitted_legacy_and_absorbs_its_duplicate` (prefix removed), `an_unreducible_verbatim_root_keeps_its_prefix_and_absorbs_its_duplicate` (prefix kept); portable tests assert the survivor keeps its input spelling |
| Catalog: reducible verbatim root rejected as unnormalized (explicit error) | `repository_scope_catalog::constructor_names_the_unnormalized_root_it_rejects` (exact error variant + path, all hosts, dot segments); **Windows:** `constructor_rejects_a_reducible_verbatim_root_as_unnormalized` |
| Catalog: containment against an already-valid catalog | `containment_in_a_valid_catalog_uses_whole_components`; **Windows:** `containment_accepts_verbatim_and_mixed_spellings_of_a_document` |
| Canonical symlink/junction escape checks kept | untouched; `repository_containment_rejects_an_external_junction` got a comment saying why it needs Windows |

All new tests live in already-declared targets (the lib unit tests and the
`l1` binary, `tests/l1/main.rs` already declares `magic_local_roots` and
`repository_scope_catalog`), carry no tier marker, and read no repository
file. `just check-tier-coverage biscuit-file`: stranded (0).

### Native-only tests

No real 8.3-alias test existed. Added
`a_real_short_name_alias_is_one_directory_but_two_identities` (Windows): it
asks `cmd`'s `%~sI` for the short spelling of a fresh `Long Directory Name`
directory, and when the volume does not generate 8.3 names (short spelling
equals long) it prints why and returns instead of assuming generation. Note
the rust-testing rule that an L1 skip reads as a pass: on a volume without 8.3
names this test proves nothing, and only a host with generation enabled
(GitHub's `windows-latest`, whose `TEMP` is `RUNNER~1`) supplies that
evidence. The existing junction test and the `link_dir` helpers (symlink on
Unix, junction on Windows) are unchanged.

### Mutation check

1. `first_seen_by_identity` changed to last-wins: 9 tests failed, 5 new
   (`a_long_verbatim_duplicate…`, `ordering_keeps_distinct_non_unicode…`,
   `ordering_folds_only…`, `unc_verbatim_and_legacy…`, `verbatim_and_legacy…`)
   and 4 pre-existing `magic_local_roots` tests (`launch_equals_home_user_override…`,
   `magic_search_roots_exposes…`, `recursive_magic_walks…`,
   `repository_equals_home_user_override…`).
2. Drive-letter folding removed from `windows::drive_root`: 3 failed
   (`ordering_folds_only_the_windows_equalities` and two pre-existing).

Both edits restored from a backup copy; `git diff` of `path_identity.rs`
shows only the new function.

### Results

- macOS: `just test` 1079 passed (baseline 1065; +14 portable tests);
  `just lint` passes.
- Windows target compile: `just cross-check biscuit-file --os windows` built
  and archived every test binary on build-win-native, then the consume step
  stopped at the storage preflight (47.8 GiB free on the target volume, 50
  GiB floor; the automatic sweep freed 0). Per the storage policy the floor
  was not overridden and no other session's artifacts were deleted.
  Local `cargo clippy -p biscuit-file --tests --target x86_64-pc-windows-gnu`
  is clean except a **pre-existing** `unused variable: ctx` in
  `repository_containment_rejects_an_external_junction` (left as-is; not this
  phase's change).
- **Evidence gap:** the six Windows-gated tests added this phase
  (`a_real_short_name_alias…`, both `magic_local_roots` Windows tests, both
  Windows catalog tests) have compiled but not run. They need a native Windows
  run once build-win-native has headroom (`just windows-sweep-status`), or the
  `windows-latest` cell on push to `main` / the `ci:all-os` label. Linux and
  WSL2 were not cross-checked: the new portable tests use no OS facility
  beyond Unix byte paths, which macOS already exercises.

### Docs and skills

- `biscuit-file/docs/topics/file-references.md`: `@` roots and candidate plans
  are deduplicated by `PathIdentity`, keeping first provenance and spelling;
  added the unreducible-verbatim example.
- `.claude/skills/biscuit-file/references/file-references.md`: same rule,
  naming `first_seen_by_identity`.

## Phase 3

Phase 3 adds the canonicalization/home-lookup source guard and the
`canonicalize_simplified` contract tests. No production behavior changed:
every current direct call is listed as an exception, so the guard is green on
the tree as it stands and red on any new call.

### What was built

- **Engine:** `darkmatter/cli/tests/common/path_lookup_guard.rs`, next to the
  `source_scan.rs` / `context_guard.rs` family it builds on (R7: blanking
  scanner, no `syn`, no new crate). Two rules: `Rule::Canonicalize`
  (`std::fs::` / `tokio::fs::` / `fs::` / `Path::` / `PathBuf::` /
  `dunce::canonicalize`, the zero-argument `.canonicalize()` method) and
  `Rule::HomeLookup` (`dirs::home_dir`, `std::env::home_dir`,
  `home::home_dir`; `biscuit_file::home_dir` is the approved capture point and
  is never reported). Each is recognized qualified, through an imported module
  or module alias, imported bare, imported under an alias, and as a function
  reference.
- **Scope discovery:** `src/` plus the directory of every `[lib]` / `[[bin]]`
  root the manifest declares (a `[[bin]]` under `tests/` is a test fixture and
  skipped), plus the build script. A missing manifest, declared root, or `src/`
  fails, as does an empty scan. `source_scan.rs` gained `production_file` (one
  file, test items blanked) for the build script.
- **Exception model:** keyed by rule + repository-relative path + enclosing
  item (`Type::method`, `outer::inner`) + operation, with exact count, a
  `Kind`, and a reason. `Kind::Invariant` (private comparison, or the helper's
  own call), `Kind::Temporary` (scheduled for migration; Phases 4 and 5 delete
  these), `Kind::Reviewed` (an unresolved candidate judged not to be the
  guarded function). New call, changed count, stale entry, duplicate, empty
  reason, and a kind that does not fit the site all fail with `path:line`.
  Line numbers are never part of identity.
- **Unresolved candidates:** an unknown qualifier (`descriptor::canonicalize`)
  or a bare call with no import and no same-file definition is reported for
  review rather than ignored. A method with arguments
  (`backend.canonicalize(ctx, &native)`) is not `Path::canonicalize`, which
  takes none. Limits (macro expansion, `include!`, re-exports under a new name)
  are documented in the engine's module comment.

### Deviation from the plan: one guard per package, not one cross-package guard

The plan suggested registering a single guard in `claudine/cli/tests/l1`
scanning all five packages. That guard would read the other packages' whole
`src/` trees, which `source-inputs` cannot declare (entries must be files, and
the index deliberately ignores directory scans of other packages' source; see
`docs/cicd/test-inputs.md`). A change adding `std::fs::canonicalize` to
`darkmatter/lib/src` would then schedule darkmatter but not claudine-cli, and
the guard would not run until the nightly. Instead each package runs the
shared engine over **its own** source in its existing L1 target, exactly like
`context_construction_guard.rs`:

| Package | Target (existing) | Rules | Exceptions |
| --- | --- | --- | --- |
| biscuit-file | `lib/tests/l1/path_lookup_guard.rs` | canonicalize | 1 invariant (the helper), 2 temporary entries (3 calls) |
| biscuit-file-cli | `cli/tests/cli_tests.rs` | canonicalize | none |
| claudine | `lib/tests/l1/path_lookup_guard.rs` | canonicalize, home | 5 invariant (8 calls), 6 temporary (7 calls); 34 temporary home entries (35 calls) |
| claudine-cli | `cli/tests/l1/path_lookup_guard.rs` | canonicalize, home | 8 invariant (10 calls), 3 temporary; 8 temporary home entries |
| darkmatter | `lib/tests/l1/path_lookup_guard.rs` | canonicalize | 4 invariant (6 calls), 15 temporary (16 calls) |
| darkmatter-cli | `cli/tests/l1/path_lookup_guard.rs` (+ 16 engine self-tests) | canonicalize | none |

biscuit-file-cli is included because the spec asks for "library and CLI" of
each area; it has no call today. No new test binary was added. Every guard
uses `biscuit_test_harness::manifest_dir!()` (not `env!("CARGO_MANIFEST_DIR")`),
so it resolves in an archived (WSL2) run, and spells the engine and
`source_scan.rs` with `include_str!`. Each non-owning package declares both in
`[package.metadata.ci.tests] source-inputs` (biscuit-file-cli gained the
table). Verified: `affected_scope.py --resolved-plan` for a change to the
engine alone schedules exactly one narrowed `ubuntu-latest` L1 cell per
package (`test(=path_lookup_guard::production_source_…)`) plus darkmatter-cli's
own suite; `test_affected_scope.RealWorkspaceTestInputTests` passes (6 tests).

The seeded exceptions match the Phase 1 audit row for row: the engine's first
scan reported exactly the audit's convert/except rows (with Phase 2's line
shifts in `resolve.rs`) and all 43 Claudine home sites, and no permissions
trait call, `canonicalize_lossy`, `rule2_canonicalize`, or Darkmatter
`style::descriptor::canonicalize` (a definition with no production caller).

### Home-lookup rule vs the existing ambient-state gate

The Phase 1 log suggested tightening `context_guard.rs`'s ambient gate instead
of a second scanner. Not done: that gate counts per file and identifier,
cannot follow an import alias (`use dirs::home_dir as h; h()`), and has to keep
counting `biscuit_file::home_dir` reads (they are ambient for its purpose). The
home rule shares the canonicalize rule's import/alias resolution instead.
Consequence for Phase 4: each migrated site must be removed from **both**
lists, the `Kind::Temporary` home entry here and the `dirs::home_dir`
allowance in `context_construction_guard.rs` (replaced by a
`biscuit_file::home_dir` allowance where the read remains).

### Requirement-to-test mapping

| Spec requirement | Test |
| --- | --- |
| Helper: absolute, usable for I/O, accepted by `try_portable_string`, equal to the real canonical target (`/var` vs `/private/var`) | `path_text::tests::canonicalize_simplified_yields_a_usable_portable_canonical_path` (also runs under `--no-default-features` via `just test-minimal`) |
| Helper resolves to the real target, not the authored spelling | `canonicalize_simplified_resolves_a_symlink_to_its_target` (Unix) |
| Missing path is an I/O error | `canonicalize_simplified_reports_a_missing_path_as_an_io_error` (`NotFound`) |
| Windows: safe verbatim and legacy disk spellings agree, prefix dropped | `canonicalize_simplified_agrees_for_verbatim_and_legacy_disk_spellings` (Windows) |
| Windows: real 8.3 alias reaches the same target (skip when generation is off) | `canonicalize_simplified_reaches_one_target_through_a_short_name_alias` (Windows) |
| Refeeding an unsimplifiable result is the grammar's explicit error, not a panic | `an_unsimplifiable_result_fed_back_as_a_reference_is_an_explicit_error` (all hosts with the textual shapes; on Windows also a real past-`MAX_PATH` canonical result, asserted unportable first) |
| Guard: new call fails with `file:line` | `a_new_call_fails_with_its_file_and_line`; scratch edit below |
| Guard: alias / imported / module alias / method / function reference | `every_spelling_of_a_filesystem_canonicalize_is_found` |
| Guard: allowed call passes; moved line still passes | `an_excepted_call_passes_and_survives_a_moved_line` |
| Guard: deleted exception fails; stale exception fails | `a_deleted_exception_or_a_deleted_call_fails` |
| Guard: count change, empty reason, duplicate entry | `a_changed_count_fails`, `an_exception_without_a_reason_or_a_duplicate_fails` |
| Guard: inline/separate test modules, `cfg(all(test, ..))`, comments, strings, raw strings excluded | `test_code_comments_and_strings_are_not_calls` |
| Guard: target-specific branches scanned | `a_platform_branch_is_scanned` |
| Guard: style-name normalizer not mistaken; unresolved candidates reported and accepted only as `Reviewed` | `a_style_name_normalizer_is_not_filesystem_canonicalization` |
| Guard: permission-backend method with arguments ignored | `a_method_with_arguments_is_not_path_canonicalize` |
| Guard: enclosing item identity | `the_enclosing_item_names_the_impl_type` |
| Guard: manifest-declared roots and build script scanned; `tests/` bins skipped | `a_manifest_declared_root_and_the_build_script_are_scanned` |
| Guard: missing root / manifest / wrong package dir / empty scan fail | `a_missing_root_or_an_empty_scan_fails` |
| Home rule: every direct spelling incl. alias and function reference; helper, accessor, variable not reported | `every_spelling_of_a_direct_home_lookup_is_found`, `a_home_lookup_imported_from_the_helper_is_allowed`, `a_rule_the_package_does_not_take_is_not_reported` |
| Guard runs on the real tree of each package | six `production_source_…` tests (table above) |

All engine self-tests are in `darkmatter-cli::l1 path_lookup_guard::*`; none
carries a tier marker. **Scratch-edit check:** appending a
`std::fs::canonicalize` call to `biscuit-file/lib/src/file_reference/glob/roots.rs`
failed `biscuit-file::l1 path_lookup_guard::…` with
`new canonicalize call `std::fs::canonicalize` in scratch_probe at
biscuit-file/lib/src/file_reference/glob/roots.rs:398` and a paste-ready
exception; the file was restored from a copy and `git diff` of `lib/src` shows
only `path_text.rs`.

### Results (macOS)

| Area | `just test` | `just lint` |
| --- | --- | --- |
| biscuit-file | pass, 1085 run (baseline 1079; +4 path_text Unix tests, +2 guards); `test-minimal` 9 passed | pass |
| claudine | pass, 8106 run (baseline 8104; +2 guards), 9 skipped | pass |
| darkmatter | **2 failed, 1 timed out (none caused by this phase)**, 8900 passed of 8903 | pass |

- darkmatter's 2 failures are the pre-existing Phase 1 baseline failures
  (`current_root_documentation_contract`, `current_root_migration_guard`).
- The timeout is `darkmatter-cli::l1 entry_point_parity::md_entry_points_agree_on_every_reference`
  (30 s limit) under full-suite load; it passes alone in 21.6 s and reads
  nothing this phase changed.
- Windows clippy (`--target x86_64-pc-windows-gnu`, biscuit-file and
  biscuit-file-cli tests): clean except the pre-existing `unused variable: ctx`
  in `finalized_reference_resolution.rs`.
- Tests that read the two edited skill files
  (`schema_roots::no_doc_or_skill_names_the_singular_schema_variable`,
  `ci_workflow_contracts::the_ci_documentation_states_the_implemented_behavior`)
  pass.

### Evidence gaps

- **Native Windows:** `just cross-check biscuit-file --os windows` built and
  archived on build-win-native (MSVC compile of every new Windows-gated test),
  then stopped at the consume step's storage preflight again (47.6 GiB free,
  50 GiB floor, sweep freed 0). Not overridden, per the storage policy. The
  three new Windows helper tests plus Phase 2's six have compiled but **not
  run**. They need build-win-native headroom or the `windows-latest` cell
  (push to `main` or the `ci:all-os` label).
- Linux and WSL2 were not cross-checked: the guard and helper tests use only
  portable `std` path handling (the engine joins `/`-separated keys itself and
  tolerates CRLF), and the PR's `ubuntu-latest` cells run them.

### Out-of-scope findings

- `just ci-local --plan` schedules test-toolkit's archive-path guard for the
  `biscuit-file/cli/tests/cli_tests.rs` change. That guard **fails on this
  branch before this phase**: 8 `context_construction_guard.rs` files
  (claudine lib/cli/gen, darkmatter lib/cli/dmls, messenger lib/cli) use
  `env!("CARGO_MANIFEST_DIR")`. They arrived with earlier branch commits (for
  example `a8065bb87`), not `main`. The new guards use `manifest_dir!()` and
  are not among the sites. Left unchanged (outside this fix); the fix is the
  same one-line swap in each file.

## Phase 4

Phase 4 makes `biscuit_file::home_dir()` the one home reader and routes
Claudine and Darkmatter through it. The guard's temporary home-lookup
exceptions are gone: no direct `dirs::home_dir` / `std::env::home_dir`
remains in Claudine production source.

### What changed

- **Shared helper.** `biscuit_file::home_dir()` is now
  `std::env::home_dir().filter(|h| h.is_absolute())` (`HOME` on POSIX,
  `USERPROFILE` on native Windows, platform fallback when unset). Signature,
  non-Unicode support, and the `file-reference` gate are unchanged. No
  canonicalization, existence check, or rebasing. biscuit-file no longer uses
  `dirs`, so the optional dependency and its `file-reference` feature entry
  were removed (`biscuit-file/docs/dependencies.md` updated; `dirs` stays in the
  workspace through Claudine CLI, Darkmatter, and others, so the root
  `docs/dependencies.md` is unchanged).
- **Darkmatter.** `RequestSnapshot::from_process` calls
  `biscuit_file::home_dir()`; its "two readers" doc comment is rewritten
  (drift resolved). The ambient-state allowlist now names
  `biscuit_file::home_dir`.
- **Claudine library.** All 35 production sites (config, backups, provider
  behaviors, MCP import/types, linking, messaging, model cache, reporting,
  permissions, protect, `dispatch/loader.rs`) call `biscuit_file::home_dir()`
  qualified. Fallbacks (`unwrap_or_default`, `"."`, `"~"`, error) are
  preserved exactly. `claudine` no longer uses `dirs` and the dependency was
  removed; `"file-reference"` is now listed explicitly in its biscuit-file
  features (R4).
- **Protect uses one captured home.** `SensitivePathChecker` already captured a
  home, but `is_sensitive` expanded `~` through a second, ambient
  `dirs::home_dir()` inside `normalize_path`, and `ProtectService` did too.
  `normalize_path` now takes the home (`normalize_path(path, home)`), the
  checker exposes `home_dir()`, and both the checker and the service expand
  `~` against the captured value. `PolicyContext::new` and
  `SensitivePathChecker::new` document that the environment is an input, not
  proof of a trusted boundary.
- **Claudine CLI.** The 8 sites (`uninstall`, `wrap/*`) call the shared
  reader. `commands/init/mod.rs` is `#[cfg(test)]`-only and was left alone.
- **Guards.** Every `Kind::Temporary` HomeLookup entry was deleted from
  `claudine/{lib,cli}/tests/l1/path_lookup_guard.rs` (and the now-unused
  `home` constructor). The `context_construction_guard.rs` allowances in
  both packages were swapped to `biscuit_file::home_dir`; `protect/path.rs`
  dropped from 3 reads to 1.

### Deviations from the plan

- **No Claudine wrapper (R5).** Ambient sites call `biscuit_file::home_dir()`
  directly instead of a Claudine wrapper. A wrapper would hide each read from
  the ambient-state gate (`context_construction_guard.rs`), which counts reads
  per file. It would also add a second name for the same policy. The R5 goal,
  one policy, is met.
- **CLI wrap sites read at their own point.** The plan asked to capture once
  per request and thread it. The wrap sites (`live_semantic_sink`, `timeouts`,
  `stream_capture`, `profile/{gemini,opencode}`) each read the shared reader
  where they need it. Claudine never sets `HOME`/`USERPROFILE` in its own
  process (checked: its only `set_var`s are `NO_COLOR`/`FORCE_COLOR`), so
  every read in one process returns the same value. Threading a home through
  the wrap pipeline is a wider refactor with no behavioral gain today. The
  request-scoped policy paths (protect, permissions) do capture once, as
  above.

### XDG / OS-folder audit

- `wrap/harness_orch/loop_control/requeue.rs`: `dirs::config_dir()` first,
  home as last resort, overridden by `CLAUDINE_RENDEZVOUS_FALLBACK_DIR`. This
  is an intentional OS-folder policy and was kept. On native Windows
  `%APPDATA%` comes from the known-folder API, so a fixture must set the
  override variable. The doc comment claimed the fallback was
  `~/.claudine/rendezvous/`, but the code writes `<home>/claudine/rendezvous/`
  (no dot). Drift was resolved by fixing the comment to match the code. The
  function is `#[allow(dead_code)]` today.
- `wrap/profile/opencode.rs`: `XDG_CONFIG_HOME`, else `<home>/.config`. This
  is OpenCode's own policy and is unchanged.
- Claudine's home-based config flows (`.claudine/config.json`, backups, logs,
  cache, MCP, provider configs) use no OS folder. The round-trip tests prove
  the primary write lands in the fixture. The Claudine CLI fixture already
  points `APPDATA`/`LOCALAPPDATA` at the fixture home and removes
  `XDG_CONFIG_HOME`.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| `~` follows `HOME` on POSIX and `USERPROFILE` on Windows; `HOME` alone does not relocate Windows home; home used as spelled (macOS `/var` kept) | `biscuit-file-cli::cli_tests home_reference_follows_the_platform_home_variable` (`bf ref ~/cfg.toml` child process, conflicting variables) |
| Relative home is no home, with no fallback to cwd or the real profile | `biscuit-file-cli::cli_tests a_relative_home_is_no_home_and_does_not_fall_back` (`relhome`, `./relhome`, `.`: exit 2, `MissingHomeContext`, nothing on stdout) |
| Claudine config load and save land under the child's home and reload from there | `claudine-cli::l1 home_lookup_round_trip::config_is_saved_under_the_child_home_and_reloaded_from_it` (headless create, save, reload, change, reload) |
| Conflicting overrides pin the platform variable; the other home and the default fixture home stay untouched | `home_lookup_round_trip::conflicting_home_variables_resolve_to_the_platform_variable` |
| Relative home: neither used nor replaced by the real profile | `home_lookup_round_trip::a_relative_home_is_no_home_and_does_not_reach_the_real_profile` |
| Policy uses one captured home, including `~` expansion (injected home) | `claudine protect::path::tests::a_checker_expands_tilde_against_its_captured_home_only` (fails on the old code, which expanded `~` against the ambient home), `tilde_path_is_expanded_against_the_given_home` (with and without a home) |
| Explicit injection / cleared home | existing: `PolicyContext::with_home_dir` tests (goose, kimi), `FileResolutionContext::without_home_dir` and Darkmatter `RequestSnapshot::new` (no home) tests |
| Guard: no direct home lookup in Claudine | `claudine{,-cli}::l1 path_lookup_guard::production_source_…` (no HomeLookup exception left) |

All environment changes are made on child processes (`assert_cmd`
`.env`). No test mutates this process's environment. The new tests carry no
tier marker and sit in declared targets (`cli_tests.rs`; `tests/l1/main.rs`
declares `home_lookup_round_trip`). `test_placement` passes.

Not covered, with the reason:

- **Missing home (variable unset).** std falls back to the passwd entry or
  the OS profile, which is the real user's home. A hermetic test cannot
  produce "no home" this way without touching the real profile, so the
  relative-home cases stand in for "no home".
- **Captured home unchanged after the parent environment changes.** This can
  only be shown by mutating a live process's environment, which the rules
  forbid. The captured-value test above shows that policy reads the captured
  home rather than re-reading.

**Mutation check.** `home_dir()` was temporarily changed to read only
`USERPROFILE`. Both conflicting-variable tests failed (`bf` and Claudine), and
the others passed. The file was restored from a copy, and the diff shows only
the intended change. Note: on macOS and Linux the old `dirs`-based helper also
honors `HOME`, so these tests catch the original defect only on native
Windows.

### Surfaced behavior (not changed)

With no usable home, Claudine's `user_config_path()` falls back to the
literal relative path `~/.claudine/config.json`. The headless init then
writes a directory named `~` under the working directory.
`a_relative_home_is_no_home_and_does_not_reach_the_real_profile` pins this
so any change to it is deliberate. Before this phase, a relative `HOME` was
used as a relative directory instead. Both behaviors are poor. A clean fix
(fail with "home directory is not available") changes `user_config_path()`
to return a `Result` for about 8 callers. That is outside this phase and is
recorded for follow-up.

### Docs and skills (drift)

- `biscuit-file/docs/topics/file-references.md`: the Home section explains
  where the home comes from, with examples (Windows `USERPROFILE`, `HOME`
  alone not enough, relative home means none, environment is not a trust
  boundary). The "cross-platform home" wording elsewhere is corrected. The
  full Mermaid treatment is Phase 6.
- `biscuit-file/docs/dependencies.md`: the `dirs` entry is removed, and the
  `dunce` note no longer names `dirs`.
- `claudine/docs/topics/repo-isolation.md`: removed the false claim that
  Claudine's home "resolves through the known-folder profile and ignores
  `USERPROFILE`".
- `claudine/docs/topics/system-prompt.md`: the snippet now uses the shared
  reader.
- `claudine/cli/tests/level2/level2_provider_overlay_capture.rs` module doc:
  removed the same stale `dirs::home_dir` claim.
- `biscuit-file/lib/src/file_reference/resolve.rs` (`simplify_root` comment)
  and `biscuit-file/lib/tests/l1/resolution_context.rs` (Windows home test
  comment) no longer name `dirs`.
- Skills: `.claude/skills/biscuit-file/SKILL.md` (new "Need the user's home?"
  rule), `references/file-references.md`, `.claude/skills/os/windows.md`
  (trap 2), and `.claude/skills/os/macos.md`.
- `darkmatter/lib/tests/l1/nested_composition.rs` built its expected `~` link
  with `dirs::home_dir()`. It now uses `biscuit_file::home_dir()`, the reader
  the request uses.

### Results (macOS)

| Area | `just test` | `just lint` |
| --- | --- | --- |
| biscuit-file | pass, 1087 run (Phase 3: 1085; +2 `bf` home tests) | pass |
| claudine | pass, 8110 run (Phase 3: 8106; +3 round-trip, +1 protect), 9 skipped | pass |
| darkmatter | 8900 passed of 8903: the 2 **pre-existing** Phase 1 baseline failures (`current_root_documentation_contract`, `current_root_migration_guard`) plus `entry_point_parity::md_entry_points_agree_on_every_reference` timing out at 30 s under full-suite load (passes alone in 29.1 s; reads nothing this phase changed) | pass |

- Windows target compile: `cargo clippy -p biscuit-file -p biscuit-file-cli
  --tests --target x86_64-pc-windows-gnu` is clean except the pre-existing
  `unused variable: ctx`. `cargo check -p claudine -p claudine-cli --tests
  --target x86_64-pc-windows-gnu` is clean.
- **Evidence gap, native Windows.** `just cross-check biscuit-file --os
  windows` stopped again at build-win-native's storage preflight (47.1 GiB
  free, 50 GiB floor). The floor was not overridden. Native Windows is where
  this phase changes behavior (`USERPROFILE` is now honored), so the
  `cfg!(windows)` branches of the two conflicting-variable tests have
  compiled but **not run**. They need build-win-native headroom or the
  `windows-latest` cell (push to `main` or the `ci:all-os` label). Linux and
  WSL2 behave like macOS for `HOME` and are covered by the PR's
  `ubuntu-latest` cells.

## Phase 5

Phase 5 converts every canonicalization the Phase 1 audit marked "passed
onward" to `canonicalize_simplified`, removes the guard's last
`Kind::Temporary` entries, and removes the `Temporary` kind from the engine.
All six `path_lookup_guard` tests are green with only `Invariant` (private
comparison) and `Reviewed` entries.

**Behavior scope.** On macOS and Linux `canonicalize_simplified` *is*
`std::fs::canonicalize` (`dunce::canonicalize` only differs on Windows), so
nothing in this phase changes behavior on POSIX hosts. On native Windows,
each converted site stops emitting a `\\?\` verbatim spelling wherever the
legacy spelling names the same file.

### Conversions (production)

| Package | Site (enclosing item) | Was | Where the result goes |
| --- | --- | --- | --- |
| biscuit-file | `resolve.rs` `validate_containment` ×2 | `dunce::canonicalize` | containment; root/candidate path in the escape error |
| biscuit-file | `glob/roots.rs` `spelled_as_stored` | `std::fs::canonicalize` | final component compared (prefix irrelevant; converted to drop an exception) |
| claudine | `composition/error/render/mod.rs` `render_file_link` | `.canonicalize()` | file URL |
| claudine | `composition/schema/status_render.rs` `schema_status_report_prose` | `.canonicalize()` | file URL |
| claudine | `render/prompt/system.rs` `render_system_prompt_summary` | `.canonicalize()` | label and file URL |
| claudine | `mcp/state.rs` `canonical_repo_path` | `fs::canonicalize` | repository key persisted in MCP state |
| claudine | `protect/path.rs` `canonicalize_existing_ancestor` ×2 | `.canonicalize()` | allow-path match, block message (`matched_text`, `target_path`) |
| claudine | `provider_overlay/write_back.rs` `WriteBack::record` | `fs::canonicalize` | stored `Entry.source` |
| claudine | `invocation_context.rs` `canonical_key` | `std::fs::canonicalize` | **changed from the Phase 1 plan (except → convert)**, see below |
| claudine-cli | `compose/interrupt.rs` `format_user_interrupt_message` | `.canonicalize()` | file URL |
| claudine-cli | `completion/autocomplete_ui.rs` `path_label`, `file_href` | `.canonicalize()` | display label, URL |
| darkmatter | `link_resolve.rs` `resolve_absolute` | `std::fs::canonicalize` | returned link target |
| darkmatter | `file_links/discovery.rs` local `canonicalize` | `std::fs::canonicalize` | `display_path`, `component_root` |
| darkmatter | `context/report.rs` `from_schema_advisory` | `std::fs::canonicalize` | compose-warning path |
| darkmatter | `markdown/mod.rs` ×4 (`source_context_for_errors`, `full_…`, `loaded_…`, `try_from`) | `.canonicalize()` | `SourceContext` path in diagnostics |
| darkmatter | `transclusion/resolver.rs` `resolve_file_reference` | `std::fs::canonicalize` | `LocalTarget.canonical` |
| darkmatter | `cache/hashing.rs` `compose_cache_key`, `pipeline/mod.rs`, `preflight/collect.rs`, `preflight/mod.rs` `canonical_key` | `std::fs::canonicalize` | compose cache / preflight keys (all producers converted together) |
| darkmatter | `schemas/resolve.rs` `canonical_path`, `schemas/triggers/assemble.rs` local `canonicalize` | `.canonicalize()` | `referenced_files` and the trigger-cycle comparison (PR #66 seam) |
| darkmatter | `expression/path_projection.rs` `strip_prefix_any_spelling` ×2 | `std::fs::canonicalize` | relative suffix; removes the mixed-spelling comparison |

Error handling and call counts are unchanged at every site (each call is a
one-for-one replacement; `.ok()`, `?`, `unwrap_or_else` fallbacks kept).
No site gained an existence requirement.

**Deviation: `invocation_context::canonical_key` converted, not excepted.**
Phase 1 listed it as a private key ("keys only compared with keys"), pending
confirmation that every producer is `canonical_key`. Every producer is, but
`canonical_key` falls back to the *authored* path when canonicalization
fails, so with the raw call one function produced two spelling families on
Windows (verbatim on success, legacy on fallback) that were then compared.
That fails the spec's rule for keys ("private only when every producer
follows the same spelling contract"), so it converts; its doc comment was
rewritten to state that reason (the old one described the verbatim prefix
as something to keep out of projections, which is now moot).

**MCP state persisted keys (Phase 1 open item).** No read-side fallback was
needed: `resolve_repo_key` already re-canonicalizes every stored key when the
exact and canonical lookups miss, so a key a Windows host wrote earlier as
`\\?\C:\…` resolves for `C:\…` and is reused rather than duplicated. A test
pins this (below).

### Guard and engine

- `Kind::Temporary` entries deleted: biscuit-file 2 (3 calls), claudine 6
  (7 calls) plus `canonical_key`, claudine-cli 3, darkmatter 15 (16 calls).
  The per-file `onward` helpers and their now-unused reason constants went
  with them.
- `Kind::Temporary` removed from `darkmatter/cli/tests/common/path_lookup_guard.rs`
  (and from the kind-mismatch diagnostic). It existed only to stage this
  migration; without it a new direct call can only be accepted as a stated
  private-comparison invariant.
- **Consolidation review.** Remaining entries: biscuit-file 1 (the helper),
  claudine 4 items / 7 calls, claudine-cli 8 items / 10 calls, darkmatter 4
  items / 6 calls. No duplicates; every reason names a concrete invariant
  (same-file verdict, dedupe/cycle membership, relative suffix rejoined to
  the authored root, walk root not hashed, single-producer cache key,
  containment verdict). `linking/hashing.rs`'s walk root is canonical only
  when the skill dir is a symlink, which does not matter because only
  relative paths enter the hash.
- **Mutation check.** Restoring `fs::canonicalize` in
  `McpProviderStateStore::canonical_repo_path` failed
  `claudine::l1 path_lookup_guard::…` with `new canonicalize call
  \`fs::canonicalize\` in McpProviderStateStore::canonical_repo_path at
  claudine/lib/src/mcp/state.rs:218` and a paste-ready `Kind::Invariant`
  entry; the file was restored from a copy and re-checked.

### Collateral test expectations

Tests whose expectation came from raw `fs::canonicalize` while the value
under test now comes from the helper were switched to the helper (on Windows
they would otherwise encode the verbatim spelling as correct). Comparisons
that canonicalize both sides, `to_portable_string` expectations already
outside a converted seam, and fixture *inputs* were left alone.

- claudine: `render/prompt/system/tests.rs` (the `base` passed to the summary
  is stripped against the now-simplified absolute path, so a verbatim base
  would change the label on Windows; also the expected `href`s),
  `provider_overlay/tests.rs` (fault targets match `Entry.source`),
  `protect/path.rs` `canonicalize_existing_ancestor_resolves_symlink_parent`.
- darkmatter: `transclusion/resolver.rs` and `link_resolve.rs` inline tests,
  `compose/tests/frontmatter.rs`, `tests/l1/suggest_constraint_phase4.rs`
  (`referenced_files`, `imports`, `examples`),
  `tests/l1/unknown_identifier_warning.rs`,
  `darkmatter-cli tests/l1/compose_transclusion.rs` (a negative "no absolute
  path" assertion that would be vacuous on Windows with a verbatim needle),
  `tests/level2/level2_render_tree_terminal/file_links.rs` (`component_root`).

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| No unlisted direct canonicalize in the five packages; no temporary entries | six `path_lookup_guard::production_source_…` tests (biscuit-file, biscuit-file-cli, claudine, claudine-cli, darkmatter, darkmatter-cli) |
| Guard red on a reintroduced call, with `file:line` | mutation check above; engine self-tests in `darkmatter-cli::l1 path_lookup_guard::*` (Phase 3) |
| Protection reports and matches in the shared spelling; an authored absolute allow path still suppresses | **new** `claudine protect::service::tests::an_existing_sensitive_file_is_reported_and_allowed_in_the_shared_canonical_spelling` (all hosts; macOS exercises `/var` → `/private/var`; on Windows it is the verbatim-leak regression) |
| MCP state: new keys in helper spelling; an earlier raw-spelled key resolves, is reused (one key), and survives save/load/save | **new** `claudine mcp::state::tests::a_repo_key_persisted_in_the_raw_canonical_spelling_still_resolves` (all hosts; on POSIX the two spellings coincide, so the compat half is only load-bearing on Windows) |
| Converted outputs equal the helper's spelling at public seams | the collateral tests above (each now fails on Windows if its site reverts to the raw call) |
| PR #66 scenarios stay covered | `darkmatter-cli::l1 schema_triggers::*` (pass), claudine `sequence*` and `propagated_context`, biscuit-file `precedence_flip` (pass in the full runs) |

No new test file or target: both new tests are inline `#[cfg(test)]` tests in
the claudine lib unit binary, no tier marker, selected by `just test`.

### Results (macOS)

| Area | `just test` | `just lint` |
| --- | --- | --- |
| biscuit-file | pass, 1087 run, 1087 passed | pass |
| claudine | 8110 run: 8104 passed, 6 timed out at 30 s, 9 skipped; the 6 (`direct_composition_runs_shell_in_configured_working_directory`, `refresh_blocking_does_not_panic`, two `runaway::detector` tests, `non_repository_session_runs_shell_in_launch_cwd`, `claudine_entry_points_agree_on_every_reference`) all pass when rerun (host load average was 163 from other sessions); the 2 new tests were added after this run and pass in targeted runs (protect 129/129, mcp::state 8/8) | pass |
| darkmatter | 8903 run: 8899 passed, 2 failed, 2 timed out; the 2 failures are the **pre-existing** Phase 1 baseline (`current_root_documentation_contract`, `current_root_migration_guard`); `darkmatter_entry_points_agree_on_every_reference` passes on rerun; `md_entry_points_agree_on_every_reference` times out under nextest and passes under `cargo test` in 29.5 s (Phase 4: 29.1 s; this phase cannot change its macOS runtime) | pass |

- Targeted reruns after the collateral edits: darkmatter/darkmatter-cli 143
  tests (resolver, link_resolve, frontmatter, suggest_constraint_phase4,
  unknown_identifier_warning, compose_transclusion, guards, schema_triggers)
  pass; claudine 259 (system prompt, provider_overlay, protect,
  invocation_context, mcp::state) pass.
- Windows target: `cargo check --tests --target x86_64-pc-windows-gnu` for all
  six packages: 0 errors. Windows clippy on the changed files reports only
  pre-existing lints (`result_large_err` at unchanged signatures) after the
  two new-test hits (`cloned_ref_to_slice_refs`) were fixed.

### Out-of-scope findings ("Separate work")

No new canonicalize hits outside the five packages beyond the Phase 1 list.
The `darkmatter/dmls` and `zed-dmls-cli` hits listed there remain
unconverted and unguarded (they are not among the five packages).

### Docs, skills, comment drift

- **Drift fixed:** `biscuit-file/docs/dependencies.md` said `dunce` is used
  at "two boundaries" and omitted `canonicalize_simplified`; it now lists the
  helper as the crate's only `dunce::canonicalize` call.
- **Comment drift fixed (code is right, comment was stale):**
  `claudine invocation_context::canonical_key` (described the verbatim prefix
  as a projection hazard; now states why the key must share the fallback's
  spelling family); `darkmatter path_projection::strip_prefix_any_spelling`
  (listed "Windows verbatim prefixes" as a spelling a canonical candidate
  carries, which the helper no longer produces; now "a symlinked ancestor").
- The `biscuit-file` skill's canonicalization bullet still describes the guard
  correctly (it never mentioned temporary entries); no skill change. Topic
  pages and the Mermaid decision diagram are Phase 6.

### Evidence gaps

- **Native Windows (the only platform where this phase changes behavior).**
  `just cross-check claudine --os windows mcp::state protect::
  render::prompt::system provider_overlay path_lookup_guard` built and
  archived on build-win-native (MSVC compile of every claudine test target,
  including both new tests, succeeded), then stopped at the consume step's
  storage preflight: 47.5 GiB free, 50 GiB floor, sweep freed 0. The floor
  was not overridden. The converted seams, the switched collateral
  expectations, and the two new tests have therefore compiled for Windows but
  **not run** there. They need build-win-native headroom or the
  `windows-latest` cell (push to `main`, or the `ci:all-os` label).
  (A first attempt with a parenthesized `-E` filterset failed in the remote
  shell before building; plain name filters work.)
- Linux and WSL2 were not cross-checked: the helper equals
  `std::fs::canonicalize` there, so this phase is a no-op on both, and the
  PR's `ubuntu-latest` cells run the guards.

Status: Phase 5 implementation complete, ready for review.

### Incident: edits redirected into the main checkout (resolved)

The host `CDPATH` (already documented in `.claude/skills/os/macos.md`) sent two
relative `cd darkmatter/lib/src` / `cd claudine/lib/src` edit commands into
`/Users/ken/coding/personal/rusty-biscuit`. Two files there received this
phase's edits (`claudine/lib/src/invocation_context.rs`,
`darkmatter/lib/src/markdown/compose/link_resolve.rs`). Both diffs were
confirmed to contain only those edits and were reverted with
`git checkout --`; the main checkout's `git status` is clean. The edits were
then re-applied in the worktree with absolute paths. The skill gained one
sentence noting that the trap redirects edits as well as test runs.

## Phase 6

Phase 6 brings the docs, READMEs, and skills in line with the code from Phases
2-5, runs a comment-drift pass over the changed symbols, and does the final
verification. The drift pass turned up stale Windows test gates, which were
re-enabled (see "Re-enabled Windows tests"). That is the only change in this
phase that alters test behavior.

### Docs

- `biscuit-file/docs/topics/file-references.md`
  - **Home section:** a Mermaid flow for `home_dir()` (OS → `HOME` /
    `USERPROFILE` → profile fallback → absolute check → home or
    `MissingHomeContext`), why `dirs::home_dir()` is not used, and an example
    of capturing the home once per request.
  - **New "Canonicalizing Paths" section:** what the verbatim `\\?\` form
    breaks, `canonicalize_simplified` with an example, the
    "leaves the comparison → helper" rule with a private-comparison example,
    the cache-key rule (every producer, including fallbacks), a Mermaid
    decision flow, and what the `path_lookup_guard` source guard enforces.
    No feature or fix directory is named.
- `darkmatter/docs/topics/compose-requests.md`: `from_process` reads
  `biscuit_file::home_dir()`; a relative home is no home. The examples were
  checked against the built `md`: with a fixture `HOME`, `::file ~/x.md`
  transcludes the file, and with `HOME=relative` it fails with
  `missing-context`.
- `claudine/docs/topics/file-referencing.md`: the home behind `~` is the
  same one Claudine uses for its own files (`HOME`, `USERPROFILE` on Windows,
  relative → none), with a `USERPROFILE` round-trip example and a link to
  biscuit-file's Home section.
- `biscuit-file/README.md`: new "Canonical Paths and the Home Directory"
  section. The Claudine and Darkmatter READMEs do not describe home lookup or
  canonicalization, so they are unchanged. No dependency changed in a way
  `docs/dependencies.md` records (see the `dirs` note below), so it is
  unchanged.

### Skills

- `.claude/skills/biscuit-file/references/api.md`: new "Canonicalization and
  home" subsection (signatures, rule, `home_dir` contract).
- `.claude/skills/biscuit-file/references/architecture.md`: **drift fixed.**
  The `file-reference` feature row listed `dirs`, which Phase 4 removed; it
  now lists `gix`, `walkdir`, `globset`, `url`, matching `Cargo.toml`.
- `.claude/skills/claudine/SKILL.md`: home and canonicalization rules,
  the guards, and "do not gate fixture-home tests `cfg(unix)`".
- `.claude/skills/darkmatter/SKILL.md`: the same rules for Darkmatter.
- `.claude/skills/os/windows.md` trap 2: **drift fixed.** It still said
  "`biscuit_file::home_dir()` itself still asks the known folder" and
  "Claudine's provider overlay still resolves through the known folder".
  Both stopped being true in Phase 4. It now states the current behavior, says
  the stale gates were re-enabled, and warns that a helper setting only `HOME`
  still reaches the real profile on Windows.

### Comment drift (code is right, comments were stale)

| Site | Stale claim | Resolution |
| --- | --- | --- |
| `claudine/lib/src/provider_overlay/selector.rs` `OVERLAY_DIR_ENV` | exists "because native Windows ... ignores `USERPROFILE`" | rewritten: it keeps overlays out of the home without moving the home |
| `claudine/lib/src/provider_overlay/tests.rs` (override test doc) | same claim | rewritten |
| `claudine/cli/tests/l1/level1_provider_overlay_home.rs` | "the known-folder home ignores its fixture `USERPROFILE`" | removed the reason; the explicit roots stay as their own coverage |
| `claudine/cli/tests/level2/level2_provider_overlay_capture.rs` | "The known-folder profile would otherwise supply both roots" | rewritten (comment only; L2 test unchanged) |
| `biscuit-file/lib/tests/l1/implicit_relative.rs` ×2 | Windows "known-folder API", "shared cross-platform provider" | now: Windows reads `USERPROFILE`, never `HOME`; `home_dir()` falls back to the platform profile lookup |
| Darkmatter `RequestSnapshot::from_process` | (the known one from the plan) | already fixed in Phase 4; re-checked, accurate |

`claudine/cli/src/commands/init/mod.rs` still calls `dirs::home_dir()`, but
only inside `#[cfg(test)]` code, which the guard excludes by design (R3). It
is unchanged. The `requeue.rs` and `shell_expansion/probe.rs` known-folder
comments are about `%APPDATA%` and the Documents folder, not the home, and
are still correct.

### Re-enabled Windows tests (deviation: test behavior, not only docs)

Before Phase 4 these tests were turned off on Windows with the reason "fixture
home cannot replace the known folder". Since Phase 4, every Claudine home read
honors `USERPROFILE`, and `CliProcessFixture` sets both `HOME` and
`USERPROFILE`. The stated reason is therefore false, and leaving tests off
for a false reason is debt. Gates with an independent reason were kept.

| File | Change |
| --- | --- |
| `claudine/cli/tests/l1/compose_prompt_tiers.rs` | 7 `cfg(not(windows))` gates and the `rendered_search_roots` helper gate removed; module doc rewritten; the symlink test keeps `cfg(unix)` for its Unix symlink API only |
| `claudine/cli/tests/l1/completion_compose.rs` | `compose_path_shaped_magic_offers_user_tier_from_plain_repo_under_home` ungated |
| `claudine/cli/tests/common/completion.rs` | `run_complete_with_home` now sets `USERPROFILE` as well as `HOME`. Without it the re-enabled test, and every `run_complete` caller, would read the real profile on Windows |
| `claudine/cli/tests/l1/propagated_context_fixtures.rs` | `isolated_fixture_can_opt_in_to_user_prompt_discovery` ungated; its doc ("inertness is the specified native behavior") contradicted R1 and was deleted. The provider stub is already cross-platform (`.cmd` on Windows) |
| `claudine/cli/tests/l1/mcp_cli.rs` | the 9 tests marked "requires Unix HOME isolation" are ungated, along with their seed helpers and imports. Tests using `#!/bin/sh` provider stubs keep `cfg(unix)` (an independent reason). The repo-key fixture now uses `canonicalize_simplified`, the spelling production keys use since Phase 5 |

### Darkmatter test expectations and the `dirs` dependency

Three Darkmatter tests built their expected home with `dirs::home_dir()`
while the code under test uses `biscuit_file::home_dir()`:
`link_normalization.rs` `a_target_under_home_is_home_rooted`,
`path_projection.rs` `home_aliased_when_under_home`, and
`tests/l1/link_interpolation_integration.rs` `test_home_dir_interpolation`.
They now use the same reader, as Phase 4 did for `nested_composition.rs`.
Those were the last uses of `dirs` in the Darkmatter library, so `dirs = "6"`
was removed from `darkmatter/lib/Cargo.toml` (`Cargo.lock` loses one edge).
`dirs` stays in the workspace (claudine-cli, zed-dmls-cli). The root
`docs/dependencies.md` does not list per-crate edges for darkmatter lib, and
`darkmatter/docs/dependencies.md` never listed `dirs` for the library, so
neither doc changes.

Side finding, unchanged: `test_home_dir_interpolation` writes
`integration_test_home.txt` into the real home directory. It should use a
captured fixture home. Recorded under "Separate work".

### Requirement-to-test mapping (Phase 6)

| Requirement | Evidence |
| --- | --- |
| Docs describe the home policy and the canonicalization rule | doc review above; Darkmatter examples checked against `target/debug/md` (fixture home transcludes; relative home → `missing-context`) |
| Docs and skills that tests read still satisfy those tests | test-toolkit (460/461; its skill-reading units pass), and claudine and darkmatter `just test` (their doc and skill contract tests) |
| Re-enabled tests still pass where they ran before | claudine `just test` on macOS: 8112/8112 |
| Re-enabled tests compile for Windows | `cargo check --tests --target x86_64-pc-windows-gnu` (claudine-cli, darkmatter, biscuit-file): 0 errors; MSVC build and archive on build-win-native succeeded (below) |
| Darkmatter expectations use the production reader | the three tests pass in darkmatter `just test` |
| No new test target or tier marker | none added. Re-enabled tests live in existing `tests/l1/` modules declared in `main.rs`; `test_layout` / `test_placement` pass |

### Results (macOS)

| Area | `just test` | `just lint` |
| --- | --- | --- |
| biscuit-file | pass, 1087 run, 1087 passed | pass |
| claudine | pass, 8112 run, 8112 passed, 9 skipped | pass |
| darkmatter | 8903 run, 8900 passed. The 2 **pre-existing** failures (`current_root_documentation_contract`, `current_root_migration_guard`) are unchanged: they report the same missing `.claude/skills/claudine/` content as in Phase 1, and the paragraph added here does not affect them. `md_entry_points_agree_on_every_reference` timed out at 30 s, as in Phases 4 and 5 | pass (including the zed-dmls wasm check) |
| test-toolkit (narrowed cell) | 461 run, 460 passed. The **pre-existing** `archive_path_guard` failure lists the same 8 `context_construction_guard.rs` sites as Phase 3 (claudine lib/cli/gen, darkmatter lib/cli/dmls, messenger lib/cli use `env!("CARGO_MANIFEST_DIR")`) | — |

`just test-l2` was not run. The only L2 edit this phase is a comment in
`level2_provider_overlay_capture.rs`, which `just lint` compiles.

`just ci-local --plan`: 117 cells marked `execute ci`. The branch is 1429
files ahead of `origin/main`, so almost every cell comes from earlier branch
work. This phase adds narrowed cells for test inputs it touched: test-toolkit
L1 (the skill files) and biscuit-terminal-cli / zed-dmls-cli test-input cells
caused by earlier branch changes. Per the CLAUDE.md duration table, more than
30 cells means about 60-100 min for the pull request run.

### Evidence gaps

- **Native Windows.** `just cross-check claudine-cli --os windows
  compose_prompt_tiers mcp_cli completion_compose propagated_context_fixtures
  level1_provider_overlay_home` compiled every re-enabled test with MSVC on
  build-win-native and archived it (build `c183d58ee4fb2b2f`). The consume
  step then stopped at the storage preflight: **33.6 GiB free** (Phase 5:
  47.5), 50 GiB floor, automatic sweep freed 0. The floor was not overridden.
  The re-enabled tests, and every Windows-only behavior from Phases 2-5, have
  compiled but **not run** on Windows. If one fails on the `windows-latest`
  cell (push to `main`, or the `ci:all-os` label), fix it forward; do not
  re-gate it for the home reason.
- **WSL2 / Linux:** not cross-checked. Nothing in this phase is OS-specific
  on Linux, and the PR's `ubuntu-latest` cells run every changed test.

### Separate work (outside this fix, unchanged)

- `darkmatter/lib/tests/l1/link_interpolation_integration.rs`
  `test_home_dir_interpolation` writes into the real home directory.
- The 8 `env!("CARGO_MANIFEST_DIR")` sites the test-toolkit archive-path
  guard rejects (Phase 3 finding). This keeps the `test-toolkit` lint cell
  red on this branch; the fix is a one-line `manifest_dir!()` swap per file.
- The two pre-existing darkmatter `current_root_*` failures, which need
  `.claude/skills/claudine/` content from another workstream.
- Claudine `user_config_path()` falls back to a literal relative
  `~/.claudine/config.json` when there is no home (Phase 4 finding).
- `darkmatter/dmls` and `zed-dmls-cli` canonicalize hits (Phase 1 list).

### Incident: Phase 6 edits committed mid-phase

At 13:18 a separate commit process (author identity) committed the working
tree while this phase was running. Its commits (`7458c3a7a` through
`e1ddcd346`, with Phase 5 messages) therefore include most Phase 6 edits:
docs, skills, README, comment fixes, the re-enabled tests, and the `dirs`
removal. They were verified intact at HEAD. Only the plan, log, and spec
frontmatter updates written after that point are uncommitted. A reviewer
reading those commits will find Phase 6 changes under Phase 5 subjects.

### Rulings, deviations, and mutation checks (whole fix)

- Rulings: R1-R7 as in Phase 1 (all defaults accepted, `yolo: true`).
- Deviations from the spec or plan, with docs corrected and the spec left as
  is: one guard per package instead of one cross-package guard (Phase 3); no
  Claudine home wrapper, and CLI wrap sites read the shared reader at their
  own point (Phase 4); `invocation_context::canonical_key` converted rather
  than excepted (Phase 5); stale Windows test gates re-enabled and Darkmatter's
  unused `dirs` dependency removed (Phase 6).
- Mutation checks: last-wins dedupe and removed drive-letter folding (Phase
  2); a reintroduced raw call fails the guard with `file:line` (Phases 3 and 5);
  `home_dir()` reading only `USERPROFILE` fails both conflicting-variable
  tests (Phase 4). Each mutated file was restored from a copy and the restore
  checked with `git diff`.

Status: implementation complete, ready for review.
