---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/fixes/2026-10-10-docs-audit-code-defects/spec.md"
plan: "tree-hugger/fixes/2026-10-10-docs-audit-code-defects/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - tree-hugger/lib/tests/l1/symbols_cache.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/queries/php/lint.scm
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/lint_diagnostics.rs
  - tree-hugger/lib/tests/l1/lint_severity_registry.rs
  - tree-hugger/lib/tests/l1/rule_registry_consistency.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/fixtures/lint/undefined_symbol.py
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/lint_severity_agreement.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_2:
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/release-notes.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/query-system.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/queries/drift.rs
  - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
docs_updated_during_phase_3:
  - tree-hugger/docs/library.md
  - tree-hugger/docs/queries.md
  - tree-hugger/docs/release-notes.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/query-system.md
source_files_during_phase_4:
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/command_feedback.rs
  - tree-hugger/cli/tests/changed/empty.rs
  - tree-hugger/cli/tests/symbols/filters.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/queries.md
  - tree-hugger/docs/release-notes.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/cli-architecture.md
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/query-system.md
packages:
  - tree-hugger
  - tree-hugger-cli
source_code:
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/queries/php/lint.scm
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/lint_diagnostics.rs
  - tree-hugger/lib/tests/l1/lint_severity_registry.rs
  - tree-hugger/lib/tests/l1/rule_registry_consistency.rs
  - tree-hugger/lib/tests/fixtures/lint/undefined_symbol.py
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/lint_severity_agreement.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/queries/drift.rs
  - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/command_feedback.rs
  - tree-hugger/cli/tests/changed/empty.rs
  - tree-hugger/cli/tests/symbols/filters.rs
documentation:
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/release-notes.md
  - tree-hugger/docs/library.md
  - tree-hugger/docs/queries.md
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/query-system.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/cli-architecture.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-10-10-docs-audit-code-defects (4 phases)

## Phase 1

Phase 1 is readiness only: one new test and the facts below. No production
code changed.

### Prove cache replay

- Added `symbols_cache::a_warm_hit_replays_the_stored_diagnostic_severity`
  (`lib/tests/l1/symbols_cache.rs`, already registered via `mod symbols_cache`).
  It writes `total = compute_total(items)` to a temp-dir `.py` file, analyzes
  it with `experimental_semantics(true)` and `Capability::Diagnostics` through
  a temp-dir `SymbolCache`, and asserts: cold run equals uncached; the entry
  stores the computed `undefined-symbol` severity; the warm run is a `Hit`
  serving it; after editing the stored severity to `Info`, the next `Hit`
  serves `Info`. So the cache replays stored severity and never restamps it.
- **Baseline at revision 50:** a temporary extra assertion
  `assert_eq!(computed, vec![json!("Error")])` failed with
  `left: [String("Error"), String("Error")]` — two `undefined-symbol`
  findings (`compute_total`, `items`), both `Error`, stored and replayed.
  Command: `cargo nextest run -E 'test(a_warm_hit_replays_the_stored_diagnostic_severity)'`
  from `lib/`. The temporary assertion was removed; the committed test holds
  no obsolete `Error` expectation (the plan forbids committing one). Phase 2
  adds the final `Warning` assertion and the revision-50 row.

### Confirmed seams (against this tree)

- `EXTRACTOR_REVISION = "50"` (`lib/src/file/mod.rs:26`).
- `severity_for_rule` (`lib/src/queries/mod.rs:366`, `pub` in `pub mod queries`;
  no crate-root re-export) has exactly six callers, all in
  `lib/src/file/tree_file.rs`: 1231 (generic `@diagnostic.<id>` loop), 1313
  `dead-code`, 1429 `undefined-symbol`, 1481 `unused-symbol`, 1539
  `unused-import`, 1616 `undefined-module`; imported at line 17.
  `invalid-syntax` is stamped with hard-coded `Error` near line 1694.
- `format_rule_message` (`queries/mod.rs:385`) has arms for `unused-variable`,
  `shadowed-variable`, `unreachable-code`, `deprecated-syntax` under "Legacy";
  it has **no** `undefined-variable` arm.
- The `check_undefined_symbols` comment still claims uppercase names are skipped
  (`tree_file.rs` ~1411); the code skips only `len()==1` and `_`-prefixed names.
- Registry API: `RuleRegistry::builtin()`, `get` (alias-aware, `Option`),
  `get_or_default` (unknown → `Warning`, **not** to be used), `has_rule`
  (alias-aware). **Departure from spec:** the spec's `is_known` does not
  exist; F7 uses `has_rule`. Severity lookup is `get(id).map(|r| r.default_severity)
  .unwrap_or(Info)`. No new registry method is needed.
- `RuleSelector::parse` never returns `None` for a rule name (only for a
  malformed `category:`), and `parse_selectors` (`cli/src/main.rs:671`) is called
  only from `Command::kind()` (lines 654–656), which `main` calls once (line 980);
  `AnalysisOptions::from_command` clones the vectors. **Correction recorded:**
  warn once in `Command::kind()`'s selector assembly, not in `parse_selectors`
  and not per consumer.
- `main` is `fn main() -> Result<(), TreeHuggerError>` (`cli/src/main.rs:887`),
  hence the `Debug` dump; `render::neutralized` is at `cli/src/render.rs:1384`.
- Removed-API consumers: `reexported_symbols` (`tree_file.rs:1093`) and the
  drift API (`lib/src/queries/drift.rs`, re-exported at `lib/src/lib.rs:21`) are
  used outside their own files only by `lib/tests/l1/phase6_neovim_query_reuse.rs`.
  A repo-wide `*.rs` grep found no consumer outside `tree-hugger/`.
- Provenance: overlay records use `"MIT"` at `provenance.rs:217`; Perl original
  uses `REPOSITORY_LICENSE` at line 82; vendored records `Apache-2.0` (line 103);
  the existing test asserts Perl at line 294.
- Cache-upgrade rows: `StaleRevision` in `cli/tests/symbols/cache_upgrade.rs:301`,
  computed keys via `computed_revision_key` (line 337).

### PHP sample parse (tree-sitter-php 0.24.2, `LANGUAGE_PHP`)

Sample (retain as Phase 2 regression input):

```php
<?php
exec('ls');
\exec('ls');
Foo\exec('ls');
namespace\exec('ls');
EXEC('ls');
eval('1');
```

| Call | `function:` node |
|------|------------------|
| `exec('ls')`, `EXEC('ls')`, `eval('1')` | `(name)` |
| `\exec('ls')` | `(qualified_name (name))` — anonymous `\` token, **no `prefix:` field** |
| `Foo\exec('ls')` | `(qualified_name prefix: (namespace_name (name)) (name))` |
| `namespace\exec('ls')` | `(relative_name (name))` |

Bare-global discriminator: `qualified_name` with no `prefix` field, e.g.
`(function_call_expression function: (qualified_name !prefix (name) @n))`
plus `#match? @n "(?i)^exec$"`. The parse was done once with a temporary test
that has been deleted.

### Other checks

- Manifest license keys: none in `Cargo.toml` (root), `tree-hugger/lib/Cargo.toml`,
  or `tree-hugger/cli/Cargo.toml`. No change implied.
- Test registration: lib `autotests = false`, single `[[test]] l1` at
  `tests/l1/main.rs` (new modules need `mod`); CLI uses default autotests, so
  `cli/tests/lint_severity_agreement.rs` will compile as its own binary.
  CLI `[package.metadata.ci.tests]` has no `source-inputs` key yet.
- F7 export-test grep: `--warn|--deny|--allow` has **no** matches in
  `cli/tests/symbols/export_controls.rs`, `export_ctags.rs`,
  `export_ctags_paths.rs`.
- Initial worktree: only the untracked empty `implementation-log.md`. The
  `2026-10-11-reexports-all-languages` feature is still `draft-spec`, queued.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ['tree-hugger', 'test-toolkit']`, `change_class: package`; gates required.
- `just test` (tree-hugger/): first run had one TIMEOUT,
  `tree-hugger-cli::symbols cache_robustness::every_kind_evidence_identities_name_retained_declarations_in_every_language`
  (30 s), with host load average ~160 from other work. It passed alone in 6.6 s
  and is a pre-existing slow test untouched by this phase. Rerun:
  **2076 tests run: 2076 passed**, exit 0.
- `just lint` (tree-hugger/): exit 0 for both packages.

### Requirement → test mapping

| Requirement | Test |
|-------------|------|
| Stored diagnostic severity is replayed on a hit (why the revision bump is needed) | `symbols_cache::a_warm_hit_replays_the_stored_diagnostic_severity` |
| Revision 50 stores `undefined-symbol` as `Error` | one-off temporary assertion on the same test (recorded above, removed) |

Input robustness matrix: not applicable (no reader changed).

## Phase 2

Change 1 (F1 + F2): diagnostic severity comes from the rule registry, the
never-emitted rules are gone, and PHP `exec`/`eval` are covered.

### Production changes

- `lib/src/file/tree_file.rs`: private `registry_severity(id)` =
  `RuleRegistry::builtin().get(id).map_or(Info, |r| r.default_severity)`
  replaces all six `severity_for_rule` calls; `invalid-syntax` now uses it
  too (was hard-coded `Error`; same value). New public
  `SEMANTIC_RULE_IDS` (the six Rust-emitted IDs) at the top of the module.
  The uppercase-skip comment in `check_undefined_symbols` was corrected
  (code wins: only 1-char and `_`-prefixed names are skipped).
- `lib/src/queries/mod.rs`: `severity_for_rule` and its `DiagnosticSeverity`
  import deleted (no crate-root re-export existed). Message arms for
  `unused-variable`, `shadowed-variable`, `deprecated-syntax` removed; the
  `unreachable-code` alias arm kept. `exec-call` message reworded to match
  the new title.
- `lib/src/rule_registry.rs`: the three never-emitted rules removed;
  `eval-call` 1.1.0 + Python language + Python example; `exec-call` 1.1.0,
  title "Dynamic code or command execution (exec)", PHP example
  `exec('ls -l');`, caveats naming what PHP does not flag.
- `lib/src/file/mod.rs`: `EXTRACTOR_REVISION` 50 → 51.
- `lib/queries/php/lint.scm`: `eval` uses `#match? "(?i)^eval$"`; new
  `exec-call` pattern.

### Departure: the Phase 1 PHP discriminator was wrong

Phase 1 recorded `(qualified_name !prefix (name) @n)` for `\exec`. It never
matches: in tree-sitter-php 0.24.2 `qualified_name.prefix` is a **required,
multiple** field whose types are the anonymous `\` token and
`namespace_name` (checked in the grammar's `node-types.json`). For `\exec`
the prefix is just the `\` token, which `to_sexp()` hides, so it looked
absent. The shipped pattern matches the callee text instead:
`function: [(name) (qualified_name)] @_fn` with
`#match? @_fn "(?i)^\\\\?exec$"` (regex `(?i)^\\?exec$`). That accepts
`exec`, `EXEC`, `\exec`, `\ExEc`, and rejects `Foo\exec`, `\Foo\exec`
(qualified, text differs) and `namespace\exec` (a `relative_name`, not in
the alternation). Recorded in skill `query-system.md`.

### Other departures

- **No commit.** The plan's "Validate first change" asks for a signed local
  commit; this session's instructions say not to commit or stage — the
  commit is made by a separate process. Task checked with that note.
- **Unknown-ID `Info` fallback** cannot be reached through a public API (no
  query can emit an unregistered ID and no query-override seam exists for
  diagnostics), so it is a unit test of the private helper
  (`tree_file::tests::registry_severity_falls_back_to_info_and_resolves_aliases`)
  rather than adding API for a test.
- **CLI revision-50 row** cannot observe severity: `--json-full symbols`
  shows none, and `hug lint` restamps severity from the registry
  (`apply_lint_policy`). The row proves the revision-50 entry is a miss
  through a renamed record; the library test carries the `Error` evidence.
- `docs/cli.md`'s lint example showed `[semantic] error [undefined-symbol]`;
  corrected to `warning` (and the real `'foo'` quoting). Docs already listed
  Python under `eval-call` (they described the query, not the registry).

### Requirement → test mapping

| Requirement | Test |
|---|---|
| F1: every registered rule's emitted severity = registry default, enabled or not | `lint_severity_registry::emitted_severity_equals_registry_default_for_every_rule` (+ `every_registered_rule_has_an_emitting_case` so no rule passes by never firing; asserts at least one disabled-by-default rule) |
| F1: shared fixture, library side `Warning` | `lint_severity_registry::undefined_symbol_in_the_shared_fixture_is_a_warning` (`include_str!("../fixtures/lint/undefined_symbol.py")`) |
| F1: library and `hug lint --experimental-semantics` agree | `cli/tests/lint_severity_agreement.rs::library_and_hug_lint_agree_on_undefined_symbol_severity` (fixture via `manifest_dir!().parent().join("lib/tests/fixtures/…")`, declared in CLI `source-inputs`) |
| F1: unregistered ID → `Info`; alias resolves | `tree_file::tests::registry_severity_falls_back_to_info_and_resolves_aliases` |
| F1: revision-50 entry never served (CLI) | `cache_upgrade::entry_from_extractor_revision_fifty_is_not_served_after_the_registry_severity_repair` |
| F1: revision-50 `Error` payload refused, recomputed `Warning`, next hit = uncached | `symbols_cache::a_revision_fifty_entry_never_replays_its_error_severity` (control proves the same payload is served under current fingerprints) |
| F1: warm hit serves `Warning` | `symbols_cache::a_warm_hit_replays_the_stored_diagnostic_severity` (now asserts `["Warning","Warning"]`) |
| F1/F2: message table names only registered rules | `rule_registry_consistency::message_table_names_only_registered_rules` (parses arms of `format_rule_message` via `include_str!`), `every_query_emitted_rule_has_a_message` |
| F2: registry ⇔ `lint.scm` captures ⇔ `SEMANTIC_RULE_IDS` | `rule_registry_consistency::registry_queries_and_semantic_rules_agree`, `semantic_rule_ids_are_the_rust_emitted_rules` |
| F2: guard fails on a one-sided addition | `rule_registry_consistency::a_one_sided_rule_addition_is_a_disagreement` (query-only, registry-only, language-only, semantic-list-only perturbations, test-local) |
| F2: removed rules gone; alias kept | `rule_registry_consistency::removed_rules_are_not_registered`; CLI `test_lint_list_rules_omits_rules_that_never_fire` (JSON and plain `--list-rules`) |
| F2: versions, Python language, exec title/example/caveats | `rule_registry_consistency::eval_and_exec_metadata` |
| F2: PHP `exec`, `EXEC`, `\exec`, `\ExEc`, `Exec` report | `lint_diagnostics::test_php_lint_exec_call_global_spellings` |
| F2: `Foo\exec`, `namespace\exec`, `\Foo\exec`, `shell_exec`, `system`, `passthru`, `proc_open`, `popen`, backticks, `executor`, `$obj->exec` don't | `lint_diagnostics::test_php_lint_exec_call_ignores_other_functions` |
| F2: Phase 1 grammar sample kept as regression input | `lint_diagnostics::test_php_lint_exec_call_grammar_sample` |
| F2: PHP `eval`/`EVAL`/`EvAl` report, `evaluate` doesn't | `lint_diagnostics::test_php_lint_eval_call_is_case_insensitive` |
| Python stays case-sensitive | `lint_diagnostics::test_python_lint_eval_exec_stay_case_sensitive` |
| `#match?` patterns compile | existing `query_compile::test_all_queries_compile_for_all_languages` |

All PHP fixtures assert `syntax_diagnostics()` is empty first, and assert
`Warning` severity on each finding.

**Fails-before evidence:** temporarily restoring `DiagnosticSeverity::Error`
at the `undefined-symbol` call site made
`emitted_severity_equals_registry_default_for_every_rule` and
`undefined_symbol_in_the_shared_fixture_is_a_warning` fail; restored. The
`\exec` positives failed against the Phase 1 `!prefix` pattern (that is how
the departure above was found). The CLI revision-50 row fails without the
bump through its `assert_ne!(old_key, current_key)`.

### CI test inputs

- CLI `[package.metadata.ci.tests] source-inputs =
  ["tree-hugger/lib/tests/fixtures/lint/undefined_symbol.py"]`.
- `python3 scripts/ci/affected_scope.py tree-hugger/lib/tests/fixtures/lint/undefined_symbol.py`
  → `packages: ['tree-hugger', 'tree-hugger-cli']`.
- `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests`
  (from `scripts/ci/`): 7 tests OK, so the declared path is spelled by an L1
  test.

### Gates

- Scope (after all edits): `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ['tree-hugger', 'tree-hugger-cli', 'test-toolkit']`,
  `change_class: package`; gates required.
- `just test` first run: 2 failures, `golden::json_symbols_over_multi_match_the_golden`
  and `golden::jsonl_…`, both only `extractor_version` `rev.50` → `rev.51`.
  Regenerated with `HUG_UPDATE_GOLDEN=1` for those two tests; diff verified
  to be exactly 188 lines of `rev.50` → `rev.51` per file. Rerun:
  **2096 tests run: 2096 passed**.
- `just lint`: exit 0 for `tree-hugger` and `tree-hugger-cli`.
- No `level2_`/`real_` markers were added; lib modules are registered in
  `tests/l1/main.rs`, the CLI file is an autotest binary.
- OS: macOS only. The changes add no path comparison, `cfg`, or process
  behavior beyond a `hug` spawn in a temp dir (same pattern as `cli.rs`),
  so cross-OS risk was judged low and left to CI; no cross-check was run.

Input robustness matrix: not applicable (no file-format reader changed).

## Phase 3

Change 2 (F3 + F5): remove unused public APIs. Done directly, without
subagents (two small, file-disjoint removals).

### Removals

- **F3:** deleted `TreeFile::reexported_symbols()` from
  `lib/src/file/tree_file.rs`. No import became unused. `ImportSymbol::is_reexport`
  and `import_is_reexport` are untouched.
- **F5:** deleted `lib/src/queries/drift.rs` (including
  `expected_upstream_revisions` and the module's 9 unit tests), `pub mod drift;`
  in `lib/src/queries/mod.rs`, and the `drift::{…}` re-export in `lib/src/lib.rs`.
  `QueryProvenance` revisions in `provenance.rs` are unchanged.
- `lib/tests/l1/phase6_neovim_query_reuse.rs`: removed the drift import, the 7
  "Upstream Drift Detection" tests, step 3 of `full_query_audit_workflow`;
  corrected the `//!` header and the integration banner/doc comment.
- Consumers: a manifest search shows `tree-hugger-cli` is the only crate that
  depends on `tree-hugger`; `claudine`'s `DriftReport`/`drift.rs` are its own,
  unrelated types and were not touched.
- `EXTRACTOR_REVISION` not bumped: no extracted record changed.
- Note: a `git rm --cached` used while deleting `drift.rs` briefly staged the
  deletion; it was unstaged immediately with `git restore --staged`. Nothing
  is staged.

### Tests

| Requirement | Test |
|---|---|
| F3: `pub use a::B; use c::D;` → only `B` has `is_reexport == true` (public `imported_symbols()`) | `tree_file::rust_pub_use_alone_is_a_reexport` (new) |
| F5: every query still has provenance with a nonempty upstream revision | `phase6_neovim_query_reuse::all_queries_compile_with_provenance` (strengthened: was `if let Some(prov)`, which passed on a missing record and did not check the revision; now requires a record for every language × {locals, lint, references, comments} and a nonempty `upstream_revision`), plus existing `provenance_fields_are_populated` |
| F5: compatibility checks still run | `full_query_audit_workflow` (provenance + compatibility steps), `no_unsupported_predicates_in_*`, `*_predicates_are_*` — all pass |
| F3/F5: removed APIs have no consumers; workspace builds | `just lint` / `just test` compile both packages with all targets |

The F3 test is a retention regression: it passes before and after the
removal by design (the removed method was never the source of the flag), so
there is no fails-before evidence for it. Targeted run:
`cargo nextest run --test l1 -E 'test(phase6) | test(rust_pub_use_alone_is_a_reexport) | test(rust_imports_report)'`
→ 21 passed.

### Docs and skill

- `docs/library.md`: replaced "`reexported_symbols()` exists but always returns
  an empty list" with "re-exports are detected for Rust `pub use` only" via
  `is_reexport`.
- `docs/queries.md`: replaced `### Drift` with `### Refreshing a vendored query`
  (recorded revision, `notes`, manual refresh, no automatic comparison);
  reflowed the following `QueryInventory` paragraph.
- `docs/release-notes.md`: new top section "Unused library APIs removed", one
  line per removed API group.
- Skill `api-reference.md`: dropped the method; added the Rust-only
  `is_reexport` statement. Skill `query-system.md`: removed "so drift checks
  skip it" and replaced "Upstream Drift Detection" with a short "Refreshing a
  Vendored Query" section.

### Removed-symbol search (acceptance)

`grep -rn "reexported_symbols\|check_query_drift\|check_all_drift\|summarize_drift\|expected_upstream_revisions\|DriftItem\|DriftReport" tree-hugger/lib tree-hugger/cli tree-hugger/docs .claude/skills/tree-hugger`
→ only `tree-hugger/docs/release-notes.md` lines 9–14 (the removed-API lines).
Also no case-insensitive `drift` left in `lib/src`, `cli/src`, the READMEs, or
`docs/` outside release notes.

### Gates

- `cargo check --workspace --all-targets` was started to catch removed-API
  consumers, but after >10 minutes still compiling the whole monorepo it was
  stopped (exit 144). Replaced by the manifest dependency search above (only
  `tree-hugger-cli` depends on the library), and the area gates compile both
  packages with all targets.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ['tree-hugger', 'tree-hugger-cli', 'test-toolkit']`,
  `change_class: package` (the uncommitted Phase 2 changes are still in the
  tree); gates required.
- `just test` (from `tree-hugger/`): **2081 tests run: 2081 passed** (2096 −
  7 drift L1 tests − 9 drift unit tests + 1 new).
- `just lint`: exit 0, no warnings, for `tree-hugger` and `tree-hugger-cli`.
- No commit: this session's instructions forbid committing; the plan's signed
  Change 2 commit is made by the separate commit process.
- OS: macOS only. The change removes code and adds a temp-dir parse test with
  no path comparison or `cfg`; no cross-check was run.

Input robustness matrix: not applicable (no reader changed).

## Phase 4

Change 3 (F4 + F6 + F7). Done directly, without subagents: the provenance
edit and the `main.rs` edits are small and file-disjoint.

### F4: query licenses

- `lib/src/queries/provenance.rs`: `overlay_query_provenance` now uses
  `REPOSITORY_LICENSE` (`AGPL-3.0-only`) instead of `"MIT"`. Perl's original
  `locals.scm` already used it; vendored records keep `Apache-2.0`.
- New unit test `queries::provenance::tests::every_record_carries_the_license_of_its_origin`
  partitions `all_query_provenance()` by `TranslationStatus::Original`
  (no hard-coded language/kind list), asserts both partitions are nonempty,
  every `Original` is the literal `"AGPL-3.0-only"`, and every other record is
  `nvim-treesitter` / `Apache-2.0` (so modified vendored queries keep upstream).
  **Fails before:** `Swift Comments is a Tree Hugger original: left "MIT"`.
- Manifest license key (Verify item 3): unchanged from Phase 1, none present.
- `EXTRACTOR_REVISION` not bumped: provenance is not part of a cached record.

### F6: readable command errors

- `cli/src/main.rs`: the old `main` body is now `run() -> Result<(), TreeHuggerError>`;
  `main` prints `error: {render::neutralized(&error.to_string())}` and exits 1.
  No `source()` chain. Clap usage errors still exit 2 (verified by the existing
  CLI suite); `process::exit` paths and broken-pipe handling are untouched
  (`report::imports_and_lint_end_quietly_on_a_closed_pipe` and friends pass).
- Reproduced before/after in a scratch dir (`mkdir .git`, `hug symbols 'missing/**'`
  etc.): before `Error: NoSourceFiles { … }`; after
  `error: No supported source files found in `…``, status 1.

### F7: unknown rule warnings

- `cli/src/main.rs`: `warn_unknown_rules(&CommandKind)` runs once in `run()`
  right after `cli.command.kind()` (the single selector assembly point), so
  `AnalysisOptions::from_command` cloning cannot double-print. It walks the
  already-parsed `deny`, `warn`, `allow` selectors, checks only
  `RuleSelector::Rule(name)` (so `all`/`ALL` and `category:` are exempt by
  construction), uses `RuleRegistry::builtin().has_rule` (alias-aware; the
  spec's `is_known` does not exist, as recorded in Phase 1), dedupes with a
  `HashSet` per run, and neutralizes the name. `RuleSelector::parse` and exit
  statuses are unchanged.
- Export-test grep (Verify item 4), rerun this phase:
  `grep -nE -- "--warn|--deny|--allow" cli/tests/symbols/export_controls.rs export_ctags.rs export_ctags_paths.rs`
  → no matches (exit 1). Their empty-stderr expectations stand.

### Tests changed because they asserted the defect

Three existing assertions matched the `Debug` variant name `NoSourceFiles`,
which only appeared because of F6. They now assert the `Display` form
(`starts_with("error: No supported source files found in `")`):
`cli/tests/changed/empty.rs::no_source_files_remains_an_error_without_changed`
and two assertions in `cli/tests/symbols/filters.rs`. Status assertions unchanged.
The god-files "does not exist" / "not a directory" tests in `cli/tests/cli.rs`
are unchanged and pass.

### Requirement → test mapping

New binary `cli/tests/command_feedback.rs` (default autotests, L1, no tier
marker; scratch dirs hold only a `.git` directory, no `git` spawn; reads no
repository file):

| Requirement | Test | Fails before fix |
|---|---|---|
| F4 every `Original` is `AGPL-3.0-only`, every vendored keeps `Apache-2.0` | `provenance::tests::every_record_carries_the_license_of_its_origin` | yes |
| F6 `symbols`, `refs`, `lint`, `imports`, `god-files` failures: status 1, starts `error: `, Display text (scan root named), no `{`/`Error:`, one line | `a_failing_command_prints_its_display_message_and_exits_1` (table of 5) | yes |
| F6 control character escaped `\u{1b}` in error (Unix only) | `a_control_character_in_a_failing_path_is_escaped` | yes |
| F7 `--warn unused-variable`, `--deny unwrap-cal`, `--allow no-such-rule`: exactly one warning each, status 0 | `an_unknown_rule_name_warns_once_and_keeps_the_exit_status` | yes |
| F7 repeated within and across all three flags → one line per distinct name | `a_repeated_unknown_rule_name_warns_once_per_run` | yes |
| F7 known rule, alias `unreachable-code`, `all`, `ALL`, `category:correctness` → no warning | `known_rules_aliases_and_special_selectors_do_not_warn` | no (control) |
| F7 control-bearing unknown name escaped | `an_unknown_rule_name_with_a_control_character_is_escaped` | yes |
| F7 lint findings still decide status beside an unknown name | `lint_findings_still_decide_the_status_beside_an_unknown_rule` | yes |

Fails-before evidence: `src/main.rs` was temporarily reverted with
`git checkout` (patch saved to `/tmp/ph4-main.patch`, re-applied with
`git apply`): 6 failed, the control passed.

Input robustness matrix: not applicable (no reader changed).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ['tree-hugger', 'tree-hugger-cli', 'test-toolkit']`,
  `change_class: package`; gates required.
- **Host trap:** the shell's `CDPATH` lists `/Volumes/coding/personal/rusty-biscuit`
  first, so a relative `cd tree-hugger` from this worktree can land in the main
  checkout. A first `just lint` showed `Checking tree-hugger (/Volumes/coding/personal/…)`
  and was discarded; all gates below were rerun after `cd` to the absolute
  worktree path (cargo output confirms worktree paths).
- First `just test` failed on `changed empty::no_source_files_remains_an_error_without_changed`
  (asserted the old `Debug` text; fixed as above, with `filters.rs`).
- `just test` (`/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger`):
  **2089 tests run: 2089 passed** (43 slow, all pre-existing; new tests each <0.4 s).
- `just lint`: exit 0, no warnings, both packages.
- `just test-l2`: **7 passed** (tmux, wezterm, kitty).
- OS evidence: macOS (all gates). Native Windows:
  `just cross-check tree-hugger-cli --os windows <six test names>` → **6 passed**
  (the Unix-only control-character path test is cfg'd out). Note: in archive mode
  `cross-check` rejects `--test`, and an `-E 'binary(…)'` filterset is broken by
  remote shell quoting; positional name filters work. Linux/WSL2 not run
  (assumed portable: plain stderr text, no path comparison or `cfg` beyond the
  Unix-only test).

### F1–F7 audit

- F1/F2 (Phase 2): revision-50 cache-upgrade row present
  (`cli/tests/symbols/cache_upgrade.rs`), shared-fixture `source-inputs`
  declared in `cli/Cargo.toml`, consistency guards pass in `just test`.
- F3/F5 (Phase 3): removed-symbol search over `tree-hugger/lib`, `cli`, `docs`,
  `.claude/skills/tree-hugger` → only `docs/release-notes.md`.
- F4, F6, F7: above.

### Departures

- **No commit.** The plan asks for Change 3's signed local commit; this
  session's instructions forbid committing or staging. Commits are made by the
  separate commit process.
- F7 uses `has_rule` (spec said `is_known`), as recorded in Phase 1.
- The plan's "Grep … before finishing F7" was done this phase as well as in
  Phase 1; result unchanged.

Implementation complete, ready for review.
