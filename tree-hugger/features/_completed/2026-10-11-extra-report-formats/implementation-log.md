---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-11-extra-report-formats/spec.md"
plan: "tree-hugger/features/2026-10-11-extra-report-formats/plan.md"
implemented_by: "codex/gpt-6-luna"
started_phase: "1"
source_files_during_phase_1:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/output.rs
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/export.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/export_plumbing.rs
docs_updated_during_phase_2:
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/export.rs
  - tree-hugger/cli/src/tsv.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/symbols/export_tsv.rs
docs_updated_during_phase_3:
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/markdown.rs
  - tree-hugger/cli/src/output.rs
  - tree-hugger/cli/src/prose.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/export_prose_markdown.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/dependencies.md
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - tree-hugger/cli/src/ctags.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/export_ctags.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
docs_updated_during_phase_5:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
packages:
  - tree-hugger-cli
source_files_during_phase_6:
  - tree-hugger/cli/tests/symbols/golden.rs
docs_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
---

# Implementation Log for 2026-10-11-extra-report-formats (6 phases)

## Phase 1

- Added `tsv`, `markdown`, `prose`, and `ctags` format arguments and a separate
  export output mode carrying signature and global plain settings. Resolved the
  Phase 1 option-combination rules, including `--tree`, `--comments`, and
  explicit `--signatures` handling. Until writer plumbing lands, invoking an
  export parses normally and exits with an argument error identifying the
  format as not implemented.
- Mapped resolver behavior to `output::tests`: mode conflicts and stats
  conflicts, tree eligibility, explicit comments/signatures restrictions,
  signature defaults, global `--plain`, and format names in command help.
- Tags reader spike: selected headless Neovim (`nvim --headless -u NONE -i
  NONE`). Its `taglist()` resolved duplicate names from an unsorted format-2
  tags file and returned numeric addresses, a path containing spaces, a
  Unicode name, `kind:`/`class:` extension fields, and decoded `\\t`, `\\n`,
  and `\\r` in names, filenames, and extension values. No editor window was
  opened. No tags-reader crate was present in `Cargo.lock`; no dependency was
  added. The recommended control policy for Phase 5 is to encode these three
  escapes and skip/warn on other controls. This does not claim Universal
  Ctags compatibility.
- Proceeded on the spec's recommended documented extended-subset tags design;
  the author confirmation required by the plan remains outstanding before
  Phase 5. No ctags writer or editor-integration claim is included in Phase 1.
- Targeted gate: `just test output::tests` passed (8 tests). A CLI smoke run of
  `cargo run --quiet --package tree-hugger-cli --bin hug -- functions --format
  tsv` parsed the new value and emitted the planned temporary
  “not implemented yet” diagnostic (exit 2).
- New targeted tests: `output::tests::new_formats_reject_every_other_output_mode_and_stats`,
  `output::tests::new_format_tree_and_explicit_flag_rules`,
  `output::tests::new_formats_accept_plain_and_signatures_where_supported`,
  and `output::tests::help_lists_all_output_formats`. Existing mode-matrix,
  single-mode, and tree tests were extended.
- Scope check from the repository root:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  reported `packages: ["tree-hugger-cli", "test-toolkit"]` and
  `change_class: "package"`; therefore neither required gate was skipped.
- `just lint` passed. Two default-concurrency `just test` attempts hit the
  30-second timeout on unchanged cache-robustness cases under host load; each
  affected case passed when isolated. The complete `just test --test-threads 4`
  run passed all 1,942 tests (11 slow, 0 skipped). No test failures remain.

## Phase 2

- Added the shared export dispatch to `ReportFormat`, preserving the existing
  `Sink::settle` failure reporting, exit status, broken-pipe handling, cache
  stderr reporting, and bounded `in_file_order` chunks. Export capabilities
  request containment only for outline layouts and ctags ownership. Each
  writer currently settles and flattens its records but emits no rows; the
  format-specific phases add the concrete encoders.
- Added a class flattening helper that orders containers by `--sort`, retains
  each selected container's displayed methods/constructors, and deduplicates
  records by ID. It does not convert summary-only fields into declaration
  rows. Added the TSV cell encoder, which handles backslash/tab/CR/LF before
  reusing `render::neutralized` for other controls.
- Requirement-to-test mapping: duplicate member ID, field omission, and class
  ordering are covered by `export::tests::class_flattening_deduplicates_members_omits_summary_fields_and_sorts`;
  TSV escape ordering by `export::tests::tsv_escape_precedes_control_neutralization`;
  outside-root fallback and unchanged record ID by
  `export::tests::outside_root_projection_keeps_fallback_and_record_identity`;
  four-format dispatch, failed-file stderr/exit status, empty-selection
  success, and no-source discovery by
  `export_plumbing::export_dispatch_settles_file_errors_and_empty_selections`;
  broken-pipe versus other write failures by
  `declarations::export_write_tests::export_write_errors_preserve_broken_pipe_and_other_failure_statuses`;
  and conditional containment by
  `declarations::export_write_tests::export_capabilities_add_only_requested_containment`.
- Targeted checks: `cargo check --manifest-path
  tree-hugger/cli/Cargo.toml --bin hug --color never` passed;
  `cargo test --manifest-path tree-hugger/cli/Cargo.toml --bin hug
  export::tests --color never` passed (4 tests); the initial ordering
  expectation failed and was corrected to match the existing grouped classes
  presentation; `cargo test --manifest-path
  tree-hugger/cli/Cargo.toml --test symbols export_plumbing --color never`
  passed after the no-source assertion was added; the final targeted command
  `cargo test --manifest-path tree-hugger/cli/Cargo.toml --bin hug
  declarations::export_write_tests --color never` passed both dispatch policy
  tests.
- CI scope calculation:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  reported `packages: ["tree-hugger-cli", "test-toolkit"]` and
  `change_class: "package"`; gates were required. `just check-tier-coverage
  tree-hugger-cli` reported no stranded tests.
- Required gates after all changes: `just test --test-threads 4` passed 1,949
  tests (11 slow, 0 skipped); `just lint` passed for `tree-hugger` and
  `tree-hugger-cli`. The final containment-policy unit test was included in
  this full run. During implementation, the initial class-order expectation
  failed and was corrected to the existing container-grouped presentation;
  the no-source assertion was narrowed to the actual discovery diagnostic.
  No pre-existing failures remain and no gates were skipped.

## Phase 3

- Added the TSV writer and connected it to the shared export stream. It emits
  the header once, including for empty selections, writes eight encoded cells
  per selected record, uses the declaration start point, preserves the
  existing portable file path and qualified name, and shares the table
  signature formatter. Class reports use the Phase 2 flattening helper and
  retain its member selection, static/instance filtering, ordering, and
  summary-field omission.
- Requirement-to-test mapping: the exact header, empty-selection behavior,
  eight-cell rows, empty signature cells, and `--comments` exit 2 are covered
  by `export_tsv::tsv_header_empty_row_shapes_and_comment_error`; location,
  kind/name/path/qualified-name parity with JSONL and short/full/none
  signatures by `export_tsv::tsv_matches_jsonl_record_fields_and_signature_modes`;
  path ordering, silent file grouping, and ANSI-containing filenames by
  `export_tsv::tsv_encodes_cells_once_and_keeps_file_order_and_grouping_silent`;
  classes static/instance and field behavior by
  `export_tsv::classes_tsv_flattens_only_selected_static_or_instance_members`;
  and one-pass reversible encoding of slash, tab, CR, LF, C1, ANSI, Unicode,
  quotes, and spaces by `export::tests::tsv_cell_round_trips_controls_and_escape_looking_text_in_one_pass`.
- The JSONL schema currently serializes `SymbolKindV2` in PascalCase
  (`Function`), while this export follows the specified snake_case spelling
  (`function`) used by `--kind`; the parity test normalizes that existing JSON
  enum spelling before comparing. No library-wide JSON schema change was made.
- Targeted checks passed: `cargo test --manifest-path
  tree-hugger/cli/Cargo.toml --bin hug export::tests --color never` (5 tests),
  `cargo test --manifest-path tree-hugger/cli/Cargo.toml --test symbols
  export_tsv --color never` (4 tests), and the TSV writer test (1 test). Early
  focused runs caught and corrected test assumptions about `--group-by-file`,
  JSONL's nested `record`, and source span placement; no implementation failure
  remains.
- Tier coverage passed: `just check-tier-coverage tree-hugger-cli` reported
  zero stranded tests. The required scope command
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
  python3 scripts/ci/affected_scope.py` reported `packages:
  ["tree-hugger-cli", "test-toolkit"]` and `change_class: "package"`, so
  package gates were required.
- Required gates passed after the final test update: `just test --test-threads 4`
  in `tree-hugger/` passed all 1,955 tests (10 slow, 0 skipped); `just lint` passed for both
  packages in the area. The first lint run found one collapsible conditional
  in export dispatch; it was simplified and the lint rerun passed. No gates
  were skipped and no failures remain.

## Phase 4

- Added CLI-owned Prose and Markdown writers and routed both formats through
  the existing export settlement/selection path. Prose uses shared declaration
  markup, the source-location URL encoder, quoted link attributes, and optional
  literal doc lines. Markdown escapes source-derived text before writing list
  items, keeps comments beneath their declaration, emits readable unlinked
  locations, and handles the empty selection.
- Added the already-locked `pulldown-cmark` crate as a CLI dev dependency.
  Documented the test-only dependency in `tree-hugger/docs/dependencies.md`.
  Requirement-to-test mapping so far: public `--format prose` and
  `--format markdown` invocation plus UTF-8/ANSI/CR checks are in
  `export_prose_markdown::prose_and_markdown_are_public_formats`; parser event
  checks for source comment links, HTML, and code spans are in
  `export_prose_markdown::markdown_source_comment_syntax_is_not_promoted_to_markup`.
- Focused command passed:
  `cargo test --manifest-path tree-hugger/cli/Cargo.toml --test symbols
  export_prose_markdown --color never` (3 tests). `just lint` passed.
  `just test --test-threads 4` passed all 1,958 tests (10 slow, 0 skipped);
  `just lint` passed for the tree-hugger area.
- Scope command:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
  python3 scripts/ci/affected_scope.py` reported `packages:
  ["tree-hugger-cli", "test-toolkit", "sniff"]` and
  `change_class: "package"`; tree-hugger area gates were required.
- `just check-tier-coverage tree-hugger-cli` passed from the repository root
  with zero stranded tests. Running it from `tree-hugger/` failed because that
  area does not define the recipe; the root recipe is the supported one.
- Phase 4 remains incomplete: the writers do not yet share `ListingStream`
  grouping behavior, preserve `--tree` outline structure, emit semantic
  Markdown emphasis/headings, or cover all hostile source constructs, URL
  variants, plain-output equivalence, and cache/worker/width stability. The
  corresponding Phase 4 plan tasks remain unchecked. Do not proceed to Phase 5
  until those gaps are completed and tested. No terminal rendering behavior
  was intentionally changed.

## Phase 5

- Implemented ctags in `cli/src/ctags.rs` and connected it to the shared
  export stream. Output starts with unsorted format-2 headers (including for
  empty selections), emits numeric source lines and relative `/` paths, keeps
  duplicate names in per-file report order, and includes recorded container
  metadata without name-equality inference. Synthetic and pathless records are
  omitted; class summaries do not manufacture field entries.
- Adopted the documented extended subset: Hug kind words, numeric addresses,
  and recorded `class:` values. Every supported language is listed in an
  exhaustive language match; every `SymbolKindV2` is matched exhaustively.
  Generic `type` and `unknown` stay generic. For controls in name, container,
  or path, the entry is skipped with a file/line warning and contributes a
  report error, making the command exit 1 while retaining valid entries.
  Scope calculation (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`) reported `packages: ["tree-hugger-cli", "test-toolkit", "sniff"]` and `change_class: "package"`; package gates are required.
- Requirement-to-test mapping: format headers, duplicate names, numeric lines,
  directory spaces, Unicode, per-file report order despite `--sort name`, and
  recorded container data are covered by
  `export_ctags::ctags_emits_numeric_unsorted_navigation_records_and_preserves_duplicates`;
  unsafe-path warning, skip, retained safe tag and exit status by
  `export_ctags::ctags_skips_control_names_with_warning_and_failure_status`; tab,
  CR, LF, and C1 rejection for each of name/container/path is also pinned by
  `ctags::tests::name_container_and_path_controls_are_rejected_without_emitting_partial_tags`;
  empty selection, `--plain` byte equivalence, explicit option rejection and
  default acceptance by
  `export_ctags::ctags_headers_survive_empty_selection_and_plain_is_identical`;
  editor navigation using Neovim headless `taglist()` by
  `level2_declarations::level2_ctags_reader_resolves_duplicates_and_numeric_lines_headlessly`.
  The latter uses the L2 `require_level!` gate and headless Neovim lookup to
  verify duplicate entries, numeric addresses, the `class:` value, spaced
  relative filenames, and Unicode. Neovim never opens a window.
- The focused L1 command
  `cargo test --manifest-path tree-hugger/cli/Cargo.toml --test symbols
  export_ctags --color never` passed (3 tests). The focused reader check and
  complete `just test-l2` each passed on macOS (the latter: 7 passed, 794
  filtered). The initial L2 test compile found incorrect `TempDir` environment
  values; changing them to `.path()` fixed the fixture. An
  initial run exposed a line-number off-by-one; the ctags writer now uses the
  record's already one-based line directly. No implementation failures remain.
- Reader policy and mapping are documented in `cli/README.md` and
  `docs/symbol-reports.md`; the tree-hugger skill quick reference now lists
  ctags. Final gates passed: `just test --test-threads 4` ran all 1,962 L1
  tests (1,962 passed, 11 slow, 0 skipped), `just test-l2` ran 7 tests (7
  passed, 794 filtered), and `just lint` passed for both packages. The first
  default-concurrency `just test` run timed out two existing broad cache
  robustness tests after 30 seconds (1,704 passed); both passed in the full
  four-thread run. No failures remain; no gate was skipped. Comment drift review: no existing renderer/helper comments claimed
  that ctags or export output was terminal-only, so none required correction.
- Phase 5's direct implementation request authorized proceeding despite the
  earlier plan's pending author ruling. Phase 4's remaining tasks are still
  open and are called out in the spec message for the next agent.

## Phase 6

- Added `symbols.tsv`, `symbols.markdown`, `symbols.prose`, and `symbols.ctags`
  goldens over the shared 16-language `multi/` scratch fixture. The harness
  checks raw output for ESC, C1 OSC, and CR before normalization. Golden
  review confirmed all four preserve their documented record/path order and
  format-specific headers or markup. Prose's local file URLs need URL-root
  normalization because macOS canonicalizes `/var` temp paths as
  `/private/var`; the golden helper now replaces each fixture URL root without
  stripping markup.
- Added `golden::export_formats_are_stable_across_workers_cache_and_terminal_width`.
  It compares each format across one/four workers, cold/warm cache, cached and
  uncached runs, and `COLUMNS=20/200`. The test passes on macOS.
- Requirement-to-test map: four 16-language output contracts and raw-byte
  safety are asserted by `golden::{tsv,markdown,prose,ctags}_symbols_over_multi_match_the_golden`;
  worker/cache/width invariance is asserted by
  `golden::export_formats_are_stable_across_workers_cache_and_terminal_width`.
  Earlier format-specific tests remain the concrete checks for TSV cells and
  escaping (`export_tsv`), CommonMark hostile comments and Prose plain output
  (`export_prose_markdown`), and tags lookup/navigation (`export_ctags` plus
  `level2_ctags_reader_resolves_duplicates_and_numeric_lines_headlessly`).
- The tree-hugger skill quick reference now lists all four export formats.
  README and Symbol Reports already contain the format rules and a Mermaid
  report-mode flow; no help/completion change was needed because the CLI's
  existing help and completion generation derive format values from the
  argument enum.
- Final `just lint` passed. Final `just test` passed all 1,968 L1 tests (36
  slow, zero skipped). An earlier run caught the Prose golden's random URL
  root; the streaming normalizer fix passed both the focused test and final
  full run.
- Required scope calculation reported `packages: ["tree-hugger-cli",
  "test-toolkit", "sniff"]`, `change_class: "package"`; therefore gates
  were required. Linux cross-check initially found the Prose golden failed
  because its URL normalizer reused byte offsets after replacement. A
  streaming normalizer fixed the defect; the focused Linux Prose golden and
  full Linux cross-check then passed (801 passed, 7 skipped). Native Windows and WSL2 are left to
  the scheduled CI environments. This phase changed no shared terminal
  rendering, so the conditional shared-terminal L2 rerun was not needed.
- Phase 6 remains incomplete: Phase 4's shared `ListingStream` integration,
  outline/class/grouped Markdown and Prose layouts, broader hostile-source
  coverage and corrected Prose/plain comparisons remain unchecked in the
  plan. The Phase 6 cross-cutting work-counter assertion is also outstanding.
  These gaps prevent setting `implemented: true` and the final completion
  frontmatter until resolved.

### Phase 6 verification completion

- Added `declarations::export_write_tests::exports_do_no_extra_analysis_beyond_equivalent_listings`.
  It exercises real uncached analysis for all four declaration commands and
  all four exports, including accepted outline requests, using existing
  process-wide counters. Work equals the equivalent declaration-only or
  containment listing: one parse and definition capture, no reference
  captures, occurrence resolution, diagnostics or complexity walks, and no
  Semantic pass. Required capabilities are checked as well. The plan's
  abbreviated outline-only containment wording now includes the recorded
  ownership needed by ctags and classes, already permitted by Phase 2.
- Export goldens no longer use terminal snapshot normalization, which could
  hide whitespace changes. They normalize only changing file-URL roots and
  reject raw ESC, C1 CSI/OSC and CR before normalization.
- Golden review exposed a TSV location defect: `TextPoint` is already
  one-based, but the writer added one to both coordinates. Removed that
  conversion, corrected the JSON Lines cross-check and added an independent
  first-source-byte assertion. Reviewed all 94 TSV golden rows: only the two
  position columns changed, each by minus one; the header, other cells and
  ordering are unchanged. This restores the specified contract without any
  record schema, cache key or extractor revision change.
- Added a native file-URL encoding/round-trip test that runs on every host,
  covering spaces, Unicode, parentheses and portable URL separators. On
  Windows it exercises the drive URI shape with a native Windows path.
- The native Windows rig exposed two filesystem fixture failures before
  writer execution: TAB and ESC filenames are rejected with error 123.
  Replaced those filesystem controls with C1 U+009B, which is valid in native
  filenames and still exercises writer neutralization/rejection. Direct
  writer tests retain TAB, CR, LF and ESC coverage; ctags now also explicitly
  covers ESC and U+009B in names, containers and paths. No platform skip was
  added. Recorded the filename constraint in the OS skill.
- The topic page was missing TSV/Markdown/Prose rules despite its earlier
  checklist marking documentation complete. Added the escaping/field rules,
  option table, local Prose links, class export limits and named reader;
  README links to that documentation. Corrected the stale terminal-only
  `--tree` statement and the declaration module's incomplete output-mode
  list. The docs explicitly mark outstanding nested/grouped/class-section
  Markdown and Prose behavior as planned rather than claiming it exists.
  Help and completions already derive all six values from the argument enum.
- Validation: macOS full `just test --test-threads 4` passed 1,969 tests
  (zero skips); subsequent focused tests over the final source passed 42,
  including all affected export/golden/URL tests. Final `just lint` passed.
  Linux full CLI archive validation passed 802 tests (7 tier exclusions),
  and final focused archive validation passed 39. Native Windows final
  focused archive validation passed 39, including the repaired fixtures,
  raw-byte goldens, stability, work counters and Windows URL round trip.
  The first Windows run's two invalid-filename failures are resolved.
  WSL2 focused archive validation passed 39, with the builder's target
  directory hidden during execution. Exclusions in the focused runs are
  intentional test-name filtering, not skipped selected tests. All four
  environments passed the final affected checks. The WSL runner correctly
  published no receipt for the outgoing HEAD because it tested the patched
  worktree; this is behavioral evidence for this implementation, not a
  passing receipt for an unchanged HEAD.
- Only `BUILD_LINUX` was inherited by this session. At the user's instruction
  to use the available local rigs, verified the SSH configuration's OS roles
  and connectivity, then supplied `BUILD_WIN=build-win-native` and
  `BUILD_WSL=build-win` to the cross-check runner. Updated the OS skill so an
  unset destination is not treated as proof that a user-confirmed rig is
  unavailable. Windows and WSL builds run sequentially on their shared host.
- Phase 4 remains outstanding. This verification completes Phase 6's work
  without closing its prerequisite: the overall spec status remains unchanged,
  and the active directory stays in place for author review. No lifecycle
  completion or Git commit was performed.
