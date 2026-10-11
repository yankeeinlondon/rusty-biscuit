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
packages:
  - tree-hugger
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
