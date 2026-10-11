# Testing Tree Hugger

Fixture layout, cache-upgrade rules, goldens and the real-terminal (L2)
harness for `tree-hugger` and `tree-hugger-cli`. Run `just test` and
`just test-l2` from `tree-hugger/`.

## Rules for query and extraction changes

When modifying queries or symbol extraction:

1. Every language with type constructs must have type distinction tests
2. All typed languages need `types.*` fixture files
3. Bug fixes require regression tests
4. Kind, visibility and static expectations live against the per-language
   fixtures in `lib/tests/fixtures/symbols/<query_name>.<ext>` (one primary
   file per language, plus JS/TS script, CommonJS and module-by-extension
   extras), read through `lib/tests/l1/symbols_fixtures.rs`;
   `symbols_coverage` fails when a language has no primary fixture or a
   fixture stops parsing cleanly
5. Run `just test` in `tree-hugger/` to verify (the library package is
   `tree-hugger`, not `tree-hugger-lib`)
6. CLI rendering checks over those fixtures live in the `cli/tests/symbols/`
   binary: `Scratch` (in its `main.rs`) copies fixtures (`include_str!`) into a
   scratch Git repo and runs `hug` with isolated `HOME`/config and
   `TMPDIR`/`TMP`/`TEMP`, which is where the persistent cache lives. `hug`
   will not scan files under the repo's own `fixtures/` directories, so CLI
   checks always need that copy. The cache-upgrade tests hard-code the
   schema-2 literal key and the analyzer key formula for each earlier
   extractor revision (`revision_key`, with that revision's record
   schema: `3.0` through revision 4, `4.0` from revision 5); keep them
   unchanged. That formula was retired with revision 6 by the computed
   `SymbolCache` key, which cannot be hard-coded (it digests the grammar and
   query text): a later bump adds a `StaleRevision` with `OldKey::Computed`,
   whose `computed_revision_key` rebuilds the key from the public
   `FingerprintInputs` with only `tree_hugger_version` changed, after
   asserting the same formula reproduces the current key (one `StaleRevision`
   row per earlier revision in
   `cli/tests/symbols/cache_upgrade.rs`; the current value is
   `EXTRACTOR_REVISION` in `lib/src/file/mod.rs`). Revisions 17 (schema 4.0, no
   `metrics.lines`), 26 (schema 4.2, no `metrics.cyclomatic`) and 30 (schema
   4.3, no `occurrences`/`reference_coverage` keys) cannot be servable stale
   payloads, so `assert_schema_bump_entry_is_not_served` checks each by hand (a
   `SchemaBump` row: what the old shape lacked and how to recognize the current
   one): rejected under both keys; a later schema bump adds a row there
   (revision 51, schema 5.1, lacks `reexports`, `imports[].reexported` and the
   `reexports` capability). Revision 52 stored the placeholder report; its row
   is a `StaleRevision` (Python `from m import a`, probe: entry count).
   The re-export report's matrix lives in `cache_robustness`
   (`reexport_report_fields_are_served_only_when_consistent` and its limited,
   not-applicable, widening and duplicate-key siblings; JSON paths under
   `reexports.entries[n]`), and the consumers in `tests/l1/reexports_consumers.rs`. Public-surface equivalence
   (direct = complete index = selective = cache cold/hit/narrowed/widened for all
   16 fixtures; extraction reads no forwarding target; one shared definitions pass;
   widening extracts once, a hit extracts nothing) is `tests/l1/reexports_equivalence.rs`,
   and the other-scan-root hit is `cache_lookup::json_full_reexports_agree_fresh_hit_and_from_another_scan_root`.
   Behavior docs: `docs/reexports.md`.
   `cache_robustness` pins which malformed
   shapes of the entry fields and record columns (including nested generics,
   relation targets and attribute/modifier members) are served or
   recomputed (its `WrongType` cell writes `"123"` for a numeric field) (tables
   in `docs/cache-design/robustness.md`); a schema change that adds a default or makes a
   field optional must update that row. The cache tests probe through
   `hug --json-full` (the complete index), since `--json` reports only the
   command's records. `Scratch` pins `RAYON_NUM_THREADS=2` so concurrent
   test processes do not each start a worker per core (tests that compare
   thread counts set it explicitly); `Scratch::process` gives a raw
   `std::process::Command` for pipe tests. `--verbose`'s stats line is how a
   CLI test proves work counts (`parsed N files, … 0 diagnostics runs`)
7. `lib/tests/fixtures/symbols/multi/` is a 16-language tree (one file per
   language, some under `nested/`, `Widget` declared in two files) for
   multi-file, ordering, parallel and golden tests;
   `symbols_coverage::multi_tree_has_one_clean_file_per_language` guards it.
   `golden::plain_symbols_over_multi_match_the_golden` (CLI `symbols` binary)
   pins `hug --plain symbols` over it against
   `cli/tests/fixtures/golden/symbols.plain`, and
   `golden::json_symbols_over_multi_match_the_golden` pins `--json symbols`
   against `symbols.json` (keys sorted, `root_dir` and every 64-hex record ID
   normalized, since IDs hash the absolute path); regenerate with
   `HUG_UPDATE_GOLDEN=1` only for a deliberate output change. The four export
   goldens reject raw ESC, C1 CSI/OSC and CR before comparison and normalize
   only changing file-URL roots, preserving whitespace. The export stability
   test compares worker counts, cache states and terminal widths.
   `declarations::export_write_tests::exports_do_no_extra_analysis_beyond_equivalent_listings`
   compares process-wide work counters for every command and export against
   the equivalent listing; containment is allowed for outlines and recorded
   ownership (ctags and classes), with no references, diagnostics, semantic
   pass or complexity walks. `order.rs`
   builds its oracle from the JSON records (add a root `Cargo.toml` to a
   scratch tree, or every manifest-less directory is its own package and the
   listing splits). Grammar limits
   found while writing fixtures: C# rejects an attribute between modifiers,
   and tree-sitter-zsh rejects a function body brace on the line after the
   header
8. `lib/tests/fixtures/symbols/test_files/` holds one file per test-file
   name rule (Go `*_test.go`, Python `test_*.py`/`*_test.py`/`conftest.py`,
   JS/TS `*.test.*`/`*.spec.*`, Rust under a package `tests/`), listed in
   `symbols_fixtures::TEST_FILE_FIXTURES` and guarded by
   `symbols_coverage::every_test_file_rule_has_a_clean_fixture`. The Rust
   one is a test in place only because it sits under this crate's `tests/`;
   assert a non-test reading from a scratch-package copy.
   Every report matrix is filled (`symbols_scope`, `symbols_qualified_names`,
   `symbols_containment`, `symbols_async`, `symbols_flags`, `symbols_metrics`,
   `symbols_kind_coverage`, `symbols_selection_filters`) and
   `symbols_coverage::every_language_has_one_row_in_each_report_matrix`
   requires one row per language in each

## Real-terminal (L2) tests

The CLI's real-terminal tests are behind the internal `terminal-tests`
feature: `level2_god_files` (the `god-files` report) and
`level2_declarations` (the `functions`/`types`/`symbols`/`classes` renderer
over `cli/tests/fixtures/declarations/`, run without `CLICOLOR_FORCE` so `hug`
detects the pane's TTY; the styling scenarios pin `COLUMNS=10000`, while
`check_wrapping`/`check_table` assert `hug`'s own wrapping at the pane width
(`Pane::cols`) and at an injected `COLUMNS`; `check_outline`, `check_stats`,
`check_signature_modes` and `check_classes_table` compare `--tree`, `--stats`,
`--signatures full|none` and `classes --format table` row for row and need an
80-column pane). `check_stats` checks complete overall/Unassigned sections
for the mixed-language fixture and complete sections for a temporary named
workspace at both ordinary width and `COLUMNS=20`. It pins duplicate names
at distinct paths, bold literal Unicode/markup/control text, zero counters,
Unassigned last, and absence of an unselected catalog package.
Every styled view a feature adds needs a check here, called
from all three backend entry points. Ordinary local L1 omits their harness and
targets;
`just test-l2` and CI enable them. A new `level2_*` target needs a `[[test]]`
entry with `required-features = ["terminal-tests"]`. Both targets read panes
through `cli/tests/support/terminal_report.rs` (`#[path]`-included): every
command runs between unique `printf`-printed begin/end markers, the capture
is the pane's whole history (tmux `capture-pane -S - -J`, WezTerm
`capture_scrollback`, Kitty `--extent=all`), and `Report::scope` keeps only
the rows strictly between the markers, failing on a missing, repeated or
reversed marker. A shared pane keeps earlier reports, other tests and a
colored prompt, so never assert on a whole capture. Style checks name one
uniquely matching row and read the cell's effective SGR state replayed from
the capture's start (tmux re-emits only changed attributes, across rows),
not the escapes next to the text; WezTerm's split soft wraps are rejoined
only for whole-line checks (`has_logical_row`, which measures rows without
trailing spaces: WezTerm pads a wrapped line's last piece to the pane width).
The helpers are
L1-tested on synthetic captures in the `symbols` binary (`terminal_scope`).
