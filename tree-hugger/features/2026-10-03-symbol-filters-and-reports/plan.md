---
kind: plan
feature: symbol-filters-and-reports
spec: spec.md
total_phases: 7
created: 2026-10-05
phase: 6
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
  - tree-hugger/lib/src/file/extract/scope_dump.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/fixtures/symbols/test_files/go/point_test.go
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/test_point.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/point_test.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/conftest.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/javascript/point.test.js
  - tree-hugger/lib/tests/fixtures/symbols/test_files/javascript/point.spec.js
  - tree-hugger/lib/tests/fixtures/symbols/test_files/typescript/point.test.ts
  - tree-hugger/lib/tests/fixtures/symbols/test_files/typescript/point.spec.ts
  - tree-hugger/lib/tests/fixtures/symbols/test_files/rust/tests/point.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-03-symbol-filters-and-reports/scope-dump.md
skills_files_updated_during_phase_1:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/kinds.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/scope_dump.rs
  - tree-hugger/lib/src/file/extract/static_flags/mod.rs
  - tree-hugger/lib/src/file/extract/visibility/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_async.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_containment.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_header.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_qualified_names.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/l1/symbols_scope.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/fixtures/symbols/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/go.go
  - tree-hugger/lib/tests/fixtures/symbols/java.java
  - tree-hugger/lib/tests/fixtures/symbols/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/python.py
  - tree-hugger/lib/tests/fixtures/symbols/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/zsh.zsh
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_kind_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/fixtures/symbols/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/c.c
  - tree-hugger/lib/tests/fixtures/symbols/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/go.go
  - tree-hugger/lib/tests/fixtures/symbols/java.java
  - tree-hugger/lib/tests/fixtures/symbols/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/perl.pl
  - tree-hugger/lib/tests/fixtures/symbols/php.php
  - tree-hugger/lib/tests/fixtures/symbols/python.py
  - tree-hugger/lib/tests/fixtures/symbols/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/zsh.zsh
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
docs_updated_during_phase_3:
  - tree-hugger/features/_unscheduled/deferred-kind-precision/spec.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_4:
  - tree-hugger/lib/src/file/pattern.rs
  - tree-hugger/lib/src/file/naming.rs
  - tree-hugger/lib/src/file/filter.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_qualified_names.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/l1/symbols_query_filters.rs
  - tree-hugger/lib/tests/l1/symbols_selection_filters.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
docs_updated_during_phase_4:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/cli/src/selection.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/classes.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/prelude.rs
  - tree-hugger/cli/tests/symbols/classes.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
docs_updated_during_phase_5:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_6:
  - tree-hugger/cli/src/output.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/support/terminal_report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.table
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols-signatures-full.plain
docs_updated_during_phase_6:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/README.md
  - tree-hugger/cli/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_7:
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/src/selection.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/naming.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
docs_updated_during_phase_7:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/features/_unscheduled/symbol-type-relations/spec.md
  - tree-hugger/features/_unscheduled/stats-package-grouping/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/tree-hugger/SKILL.md
packages:
  - tree-hugger
  - tree-hugger-cli
source_code:
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/fixtures/symbols/test_files/go/point_test.go
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/test_point.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/point_test.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/conftest.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/javascript/point.test.js
  - tree-hugger/lib/tests/fixtures/symbols/test_files/javascript/point.spec.js
  - tree-hugger/lib/tests/fixtures/symbols/test_files/typescript/point.test.ts
  - tree-hugger/lib/tests/fixtures/symbols/test_files/typescript/point.spec.ts
  - tree-hugger/lib/tests/fixtures/symbols/test_files/rust/tests/point.rs
  - tree-hugger/lib/src/shared/schema_v2/kinds.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/static_flags/mod.rs
  - tree-hugger/lib/src/file/extract/visibility/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_async.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_containment.rs
  - tree-hugger/lib/tests/l1/symbols_header.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_qualified_names.rs
  - tree-hugger/lib/tests/l1/symbols_scope.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/fixtures/symbols/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/go.go
  - tree-hugger/lib/tests/fixtures/symbols/java.java
  - tree-hugger/lib/tests/fixtures/symbols/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/python.py
  - tree-hugger/lib/tests/fixtures/symbols/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/zsh.zsh
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_kind_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/fixtures/symbols/c.c
  - tree-hugger/lib/tests/fixtures/symbols/perl.pl
  - tree-hugger/lib/tests/fixtures/symbols/php.php
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/lib/src/file/pattern.rs
  - tree-hugger/lib/src/file/naming.rs
  - tree-hugger/lib/src/file/filter.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_query_filters.rs
  - tree-hugger/lib/tests/l1/symbols_selection_filters.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/cli/src/selection.rs
  - tree-hugger/cli/src/classes.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/prelude.rs
  - tree-hugger/cli/tests/symbols/classes.rs
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/src/output.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/support/terminal_report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.table
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols-signatures-full.plain
documentation:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
  - tree-hugger/features/_unscheduled/deferred-kind-precision/spec.md
  - tree-hugger/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/features/2026-10-03-symbol-filters-and-reports/scope-dump.md
  - tree-hugger/features/_unscheduled/symbol-type-relations/spec.md
  - tree-hugger/features/_unscheduled/stats-package-grouping/spec.md
completed_phase: 7
implemented: true
---

# Plan: precise, filterable symbol reports

## Summary and definition of success

The spec extends the v2 `SymbolRecord` pipeline built by
`2026-10-03-symbol-improvements` with new per-declaration facts, a shared
record-filter path and new report shapes. The work falls into four layers,
which fix the phase order:

1. **Prerequisite gate and scope dump.** Confirm the prerequisite features are
   really in the tree, then dump every declaration's proposed scope across the
   16 fixtures for review before the classifier is written.
2. **Library metadata.** `scope`, the `async` modifier and in-file qualified
   names are always computed (changing IDs, so `EXTRACTOR_REVISION` is
   bumped). `is_test`, `is_deprecated`, `parameters` and `has_docs` are
   capabilities computed on request and stored in the capability-aware cache.
3. **Query and filters.** `SymbolQuery` predicates, a post-lookup qualified
   name prefix (not a cache-key input), one library filter, smart-case name
   matching, then the CLI flags wired onto that single path.
4. **Reports.** JSON Lines, `--tree`, `--signatures`, `--format plain|table`,
   `--count`, `--stats` (terminal and JSON) and the output-mode rules, all
   through the one `biscuit-terminal` renderer. Then docs, release notes and
   the performance sample.

**Success looks like** (mirrors the spec's definition of done):

- `just test`, `just test-l2` and `just lint` pass in `tree-hugger/`.
- Every per-language matrix has a row for all 16 languages, enforced by the
  extended coverage guard.
- One CLI golden per new format/view over the `multi/` tree, plus the focused
  cases; `--count` is a single integer line.
- `hug symbols` on `tree-hugger/` hides locals by default (about 2,550 lines
  from 8,593); `--all-scopes` restores them.
- `--json`/`--jsonl` always carry computed `is_test`/`is_deprecated`, and the
  test/deprecation capability is in `completed_capabilities`.
- A symbol's ID and cache entry are identical when `hug` runs from two scan
  roots over one file; only the displayed `qualified_name` prefix differs.
- `EXTRACTOR_REVISION` is bumped and `cache_upgrade` covers the previous
  revision.
- The performance sample from `2026-10-03-symbol-improvements` is re-run once
  after landing and recorded in the implementation log.
- All docs listed in the spec are updated; release notes carry before/after
  examples. The terminal state is "implementation complete, ready for review":
  no move to `_completed`, no `just complete`, no commit unless asked.

**Parallelism convention.** A *wave* is a set of tasks that touch disjoint
files and can run as concurrent subagents. Per-language extractor work is
split into language groups (A: Rust, Go, Python, Bash, Zsh, Perl; B: TypeScript,
JavaScript, Lua, PHP; C: Java, C#, Scala, Swift; D: C, C++), each owning only
`lib/src/file/extract/**` files for its languages and the matching fixture
and matrix rows. Shared dispatchers, the schema, the coverage guard and the
cache are single-owner tasks run in their own wave. Every subagent brief must
repeat that the session tree is non-interactive and that `cargo fmt` and
`git commit` are forbidden.

**Skills to load:** `tree-hugger`, `rust-testing` (before any test work),
`biscuit-terminal` (renderer), `biscuit-file` (`FileReference`), `os` (path
and width tests), `nextest`, `clap`.

---

## Phase 1: Gate, rulings and scope dump

Nothing in later phases is written until the prerequisites are confirmed, the
rulings below are settled, and the scope dump is reviewed.

### Necessary Rules

Rulings the spec leaves implicit. Each carries the plan's default; the author
may override any of them before the phase it gates.

1. **Prerequisites are "landed" only when the tree proves it.** The spec says
   not to start until `SymbolQuery`, the capability-aware cache and the single
   renderer exist. `git status` shows the symbol-improvements work still
   uncommitted in this worktree. *Default:* proceed in this worktree on top of
   that working state, gated by task 1.1 (symbols and tests green); do not
   start if 1.1 fails. Also confirm `2026-10-03-visibility-and-kind` is
   present (its kind/visibility/`static`/container fixes).
2. **Record schema version.** The new record fields are "additive on top of
   schema 4.0", but the cache reader requires exactly `SchemaVersion::V4_0`.
   *Default:* keep `SchemaVersion::V4_0` and rely on the
   `EXTRACTOR_REVISION` bump to invalidate old entries; new fields get
   serde shapes under the matrix below. *Alternative:* bump to 4.1, which
   also touches every schema literal and `cache_upgrade`. Author decides if
   the bump is wanted.
3. **Capability names.** *Default:* add `Capability::Flags` (is_test,
   is_deprecated) and `Capability::Metrics` (parameters, has_docs), the
   latter carrying the requested metric set so `--min-params` alone does not
   compute `has_docs`. `has_docs` uses the existing attached-doc detection,
   not the structured docs pass. `completed_capabilities` names both.
4. **Rust `tests/` rule scope.** "Any file under a package's `tests/`
   directory" means: the file's path relative to its package root
   (`find_package_root`) starts with a `tests` component. Without a package
   root, no `tests/` rule applies (the file-name rules still do). The result
   depends only on the path, which is already a cache-key input.
5. **Stored `None` metrics.** Inside a computed `metrics` facet an
   unsupported metric is serialized as explicit `null`; an absent key inside
   a computed facet is malformed. An absent `metrics` facet means "not
   requested". Keeps `None` (unsupported) distinct from not computed.
6. **Light records keep `scope`.** Declarations excluded by a kind selection
   still carry `scope` (spec B2), so the light-record shape and its cache
   projection gain that field.
7. **Smart-case on `--in` and `--exclude-symbols`.** Spec C5 applies smart
   case to inclusion and exclusion patterns; `--in` follows the same name
   rules (C4). `--returns`/`--param-type`/`--file` stay case-sensitive.
8. **`--exact` and trailing `!`.** Both force whole-string literal matching
   with smart-case; `*` is literal under it. Both forms are applied to
   `--name`, positional names and `--in`.
9. **Wider performance measurement.** The spec asks for one re-run of the
   baseline sample on one host. Anything broader (more hosts, languages,
   repetitions) is *not* planned; the author may request it separately.
10. **No spike.** The spec states none is warranted. The scope dump (task 1.2)
    is the first implementation task, not a spike, and is not repeated.

### Tasks

- [x] **1.1 Prerequisite check** (sequential, first)
    - Run `just test` and `just lint` in `tree-hugger/` and confirm green.
    - Confirm presence of `SymbolQuery`, `Capability`, `SymbolCache`,
      `render::Renderer`, `docs/symbol-reports.md`, `EXTRACTOR_REVISION == "5"`
      (or later, record the actual value) and the visibility-and-kind
      fixes. Record findings in `implementation-log.md` (create it in this
      feature directory).
- [x] **1.2 Scope dump**
    - Write a test-only or `examples/` dumper that prints, for each of the 16
      fixtures in `lib/tests/fixtures/symbols/`, every declaration with its
      current kind, container and *proposed* scope.
    - Apply the spec rules: nearest semantic declaration decides; parameters
      Local only where extracted; Rust `mod_item`/C++ `namespace_definition`/
      C# `namespace_declaration` bodies Item; `impl_item`/`trait_item` bodies
      Member; Bash/Zsh nested functions Local; members of a type inside a
      function are Member.
    - Start from `is_local_declaration` (`extract/declaration.rs`) and
      `is_member_owner_kind` (`extract/classify.rs`); the `declaration_list`
      ambiguity must be resolved by parent kind.
    - Save the dump to the feature directory as `scope-dump.md`.
- [x] **1.3 Dump review checkpoint**
    - Review `scope-dump.md` for every language (including Lua `M.f`/`M:m`,
      Go receivers, Python decorated defs, C/C++ wrapper nodes, Scala
      `object`/`given`, JS object literals and class expressions). Record
      corrections in the implementation log. The classifier in Phase 2 starts
      only after this checkpoint.
- [x] **1.4 Fixture and matrix skeleton** (parallel with 1.3)
    - Extend the coverage guard (`lib/tests/l1/symbols_coverage.rs`) with
      empty-but-required matrix tables: Scope, Flags, Qualified names,
      Metrics, Kind coverage, Selection filters, Containment. Each fails
      closed on a missing language row, so later waves fill rows rather than
      add tests.
    - Add the test-file fixtures (Go `*_test.go`; Python `test_*.py`,
      `*_test.py`, `conftest.py`; JS/TS `*.test.*`, `*.spec.*`; a Rust file
      under a package `tests/` directory), each with a test and a non-test
      helper. Rust non-test assertions use an isolated Test Toolkit copy
      (the in-place fixtures live under `lib/tests/` and so read as tests).

**Validation checkpoint 1:** prerequisite log written, rulings 1-5 confirmed
or overridden, scope dump reviewed, guard fails only on the unfilled matrix
rows.

---

## Phase 2: Always-computed metadata (scope, `async`, qualified names)

Changes record shape and IDs, so `EXTRACTOR_REVISION` is bumped once, here.

### Wave 1 (single owner, shared types)

- [x] **2.1 Schema additions**
    - Add `SymbolScope { Item, Member, Local }` and `SymbolRecord::scope`;
      fill the `async` entry of the existing `Modifier` facet; add
      `visibility.is_test` (default false, Phase 3 fills it); add the optional
      `metrics { parameters: Option<u32>, has_docs: Option<bool> }` facet.
      Follow ruling 2 and ruling 5 for serde shapes.
    - Keep the attribute, generic and relation facets empty.
    - Update the v1↔v2 adapter (`shared/schema_v2/mod.rs`) and its
      round-trip test.
- [x] **2.2 Scope classifier core**
    - Implement one scope classifier next to `classify.rs`, decided by the
      parent node kind as the dump established; expose it on
      `DeclarationContext` (add a field there rather than rescanning).
- [x] **2.3 In-file qualified-name builder core**
    - Containers joined with the language's native separator (`::` Rust/C++,
      `.` Java/C#/Python/TS/JS/Go/Scala/Swift/PHP-per-language as the spec
      table gives), plus explicit package/namespace, inline Rust `mod` blocks.
      `stable_symbol_key` consumes the in-file name plus normalized file
      identity.
    - Bump `EXTRACTOR_REVISION`; add the stale-revision case to
      `cli/tests/symbols/cache_upgrade.rs` (keep earlier cases unchanged).

### Wave 2 (parallel by language group A-D)

- [x] **2.4 Per-language extraction (group A)** Rust, Go, Python, Bash, Zsh, Perl
- [x] **2.5 Per-language extraction (group B)** TypeScript, JavaScript, Lua, PHP
- [x] **2.6 Per-language extraction (group C)** Java, C#, Scala, Swift
- [x] **2.7 Per-language extraction (group D)** C, C++
    - Each group: wire the scope classifier cases, `async` modifier (only
      where the language has async syntax), and qualified-name construction;
      extend its fixtures (async declaration, same-named methods on two
      types, explicit namespace/package, Rust inline module) and fill its
      rows in the Scope, Qualified-names and Containment matrices, with
      positive and negative cases and "unsupported extraction" labeled apart
      from "language has no construct".
    - Edit vendored `locals.scm` only where needed; update
      `vendored_locals_changes` provenance notes in the same change.

### Wave 3 (sequential cleanup)

- [x] **2.8 Rewrite superseded expectations**
    - Existing tests and goldens asserting `container::name` for `.`
      languages, and any ID literals derived from it, are rewritten (spec
      pre-authorizes this). Regenerate goldens with `HUG_UPDATE_GOLDEN=1`
      only after reviewing the diff is limited to those changes.
- [x] **2.9 Cache wiring and robustness**
    - Store scope and the in-file name in cache entries; project them through
      `analysis::coverage::project`. Extend `cache_robustness.rs` per the
      matrix below.
    - Assert `CacheLookup::Hit` in a warm run from a different scan root.

### Input robustness matrix (cache entry reader)

New load-bearing record fields the cache reader decodes. Each cell names the
outcome; tests walk every cell from one real cached entry (one edit per cell,
plus an unedited control row proving a `Hit`) through `hug --json-full`, never
through the reader's return value. Add the columns to the existing
`cli/tests/symbols/cache_robustness.rs` tables.

| Shape | `scope` | `modifiers[].async` | `visibility.is_test` / `is_deprecated` | `metrics` facet | `metrics.parameters` / `has_docs` | in-file `qualified_name` |
|---|---|---|---|---|---|---|
| absent | Malformed (recompute) | n/a (modifier list simply lacks the entry) | Malformed | not requested; entry served only if capability not claimed | Malformed (facet computed) | Malformed |
| explicit null | Malformed | Malformed | Malformed | Malformed | `None` (unsupported) | Malformed |
| wrong type, whole field | Malformed | Malformed | Malformed | Malformed | Malformed | Malformed |
| wrong type, one element | n/a | Malformed | n/a | n/a | n/a | n/a |
| wrong type, every element | n/a | Malformed | n/a | n/a | n/a | n/a |
| empty | unknown variant: Malformed | `[]` is valid (no modifiers) | n/a | `{}` Malformed | n/a | empty string Malformed |
| duplicate key | Malformed | Malformed | Malformed | Malformed | Malformed | Malformed |
| trailing/invalid content | Malformed (whole entry) | same | same | same | same | same |

Cross-checks: an entry whose `completed_capabilities` omit `Flags` or
`Metrics` but whose records carry a true flag or a `metrics` facet is
`Inconsistent`. Code smells to grep before declaring the matrix done:
`#[serde(default)]` and `Option<` on these fields,
`unwrap_or_default()`, `.ok()` on any decode of them.

**Validation checkpoint 2:** `just test` green; Scope, Qualified-names and
Containment rows exist for all 16 languages; the same file scanned from two
roots yields one ID and one cache entry; `cache_upgrade` covers the previous
revision.

---

## Phase 3: Capability-computed metadata (flags and metrics)

### Wave 1 (single owner)

- [x] **3.1 Capability plumbing**
    - Add `Capability::Flags` and `Capability::Metrics` (ruling 3) to the
      planner (`analysis/mod.rs`), the cache coverage rules (partial hit,
      `completed_capabilities`) and `analysis::coverage::project`; extend the
      `projected_results_*` tests for both.
    - A flag-only request must not run diagnostics, references or the
      structured docs pass.
- [x] **3.2 Attribute reader core**
    - Shared helper that reads a declaration's attributes/annotations/
      decorators/doc tags into full syntactic name plus raw argument text,
      without evaluating arguments, resolving imports or expanding macros.
      It populates nothing in the `Attribute` facet.

### Wave 2 (parallel by language group A-D)

- [x] **3.3 Group A** (Rust, Go, Python, Bash, Zsh, Perl)
- [x] **3.4 Group B** (TypeScript, JavaScript, Lua, PHP)
- [x] **3.5 Group C** (Java, C#, Scala, Swift)
- [x] **3.6 Group D** (C, C++)
    - `is_deprecated` from the spec's marker table only (Rust `#[deprecated]`,
      Java `@Deprecated` or `@deprecated` tag, C# `[Obsolete]`, JS/TS/PHP
      `@deprecated` in the attached doc block, Go `Deprecated:` paragraph,
      Swift `@available(*, deprecated…)`, Scala `@deprecated`, C++
      `[[deprecated]]`, Python `@deprecated` incl. qualified
      `typing_extensions.deprecated` and `warnings.deprecated`); ordinary
      comments never set it.
    - `is_test`: file-name rules (Go, Python, JS/TS, Rust `tests/` per ruling
      4), test attributes (Rust `#[test]`/`#[x::test]`, Java `@Test` incl.
      qualified, C# `[Test]`/`[Fact]`/`[TestMethod]`), everything inside Rust
      `#[cfg(test)]`; `cfg(any(test, …))` does not set it. No Python
      `TestCase` base-class rule; no synthesis from JS `describe`/`it`.
    - `parameters` (excluding a separate receiver; one variadic counts once;
      known empty list → `Some(0)`; `None` where parameters are not
      extracted: Perl, Bash, Zsh and elsewhere per the spec's 13-language
      list) and `has_docs` (`None` for Lua, Perl, Bash, Zsh).
    - Fixtures: deprecated symbol plus ordinary-comment decoy, a test and a
      similarly named non-test (`test_helper`, `latest.rs`), documented and
      undocumented functions, functions with zero/one/several parameters
      including receiver and variadic. Fill the Flags and Metrics matrix rows
      and the "Kind coverage" row labeling `FEAT` constructs as unsupported
      extraction.

### Wave 3 (sequential)

- [x] **3.7 JSON always requests flags**
    - `--json` and `--jsonl` always request `Flags`, so `false` means
      "computed and not set"; assert the capability appears in
      `completed_capabilities`. (JSON Lines emission itself lands in Phase 6;
      here wire the request for `--json` and leave the JSONL hook in the
      shared request builder.)
- [x] **3.8 Cache robustness rows for the new capability fields** per the
      Phase 2 matrix; add the `Inconsistent` cross-check tests.

**Validation checkpoint 3:** Flags and Metrics matrices complete for all 16
languages; an `--undocumented`-style library query over a mixed `.rs`/`.sh`
tree lists only Rust; a warm run after a flag-only widening reports a partial
hit, not a miss.

---

## Phase 4: `SymbolQuery` predicates and the one library filter

### Wave 1 (parallelizable; disjoint files)

- [x] **4.1 Query predicates**
    - Extend `SymbolQuery` with `.scopes()`, `.visibility()` (repeatable, exact
      level; Unknown matches only Unknown), `.exclude_tests()`/`.tests_only()`,
      `.undocumented()`, `.deprecated()`, `.is_async()`, `.returns()`,
      `.param_type()`, `.min_params()`, container (`--in`) and name/file
      selectors. Library defaults select all kinds and scopes.
    - The query carries the scan root and the optional naming context as
      post-lookup inputs, excluded from the cache key.
- [x] **4.2 Smart-case matcher**
    - Extend the current wildcard matcher (`wildcard_match` in
      `cli/src/scanner.rs`, or move it into the library if the library filter
      needs it) per spec C5 and rulings 7-8: all-lowercase insensitive, any
      uppercase sensitive, `--exact`/trailing `!` literal, `*` only wildcard,
      no new dependency. Case-fold pattern and candidate consistently without
      slicing UTF-8; non-ASCII regression cases (e.g. `Ünï`, `ß`, Turkish
      dotted İ).
- [x] **4.3 Post-lookup qualified-name prefix**
    - After cache lookup, prepend the naming-context prefix, or the portable
      scan-root-relative path plus `#` (`/` separators, no lowercasing);
      explicit-namespace names get no prefix. IDs untouched. Library-only
      naming context tests (`crate::shared::symbol::SymbolInfo::new`,
      `package.module.Class.method`).

### Wave 2 (sequential)

- [x] **4.4 Library record filter**
    - One filter function applying all predicates to records; shape
      predicates select function-like declarations only (including
      constructors); missing type text never matches; `--param-type` needs a
      non-receiver parameter; `has_docs == None` never matches undocumented.
    - Predicate pushdown: kind intersection and the capabilities each
      predicate needs go into the planner so a filter never forces work it
      does not need (metrics never force diagnostics).
- [x] **4.5 Selection-filters matrix**
    - One test per language row: each predicate returns exactly the expected
      symbol set for that language's fixture; "matches nothing" asserted where
      the construct is n/a.

**Validation checkpoint 4:** Selection-filters row complete for all 16
languages; non-ASCII matcher tests green; `just lint` clean.

---

## Phase 5: CLI filters on one path

### Wave 1 (single owner: clap surface)

- [x] **5.1 Flags**
    - Add `--kind` (repeatable, snake_case `SymbolKindV2` names; unknown is an
      argument error; intersects the command set, never widens),
      `--scope` (repeatable) and `--all-scopes` (conflicts with `--scope`),
      `--visibility` (repeatable), `--in`, `--exclude-tests`/`--tests-only`
      (mutually exclusive), `--undocumented`, `--deprecated`, `--async`,
      `--returns`, `--param-type`, `--min-params`, `--file`, `--name`,
      `--exact`.
    - `hug symbols` defaults to `item` and `member`; other commands start with
      all scopes. `--exported` stays an independent test of `is_exported`.
      Make `classes --static-only` and `--instance-only` conflict at parse
      time.
    - Verify shell completions (`hug completions`) include the new flags.

### Wave 2 (parallel; disjoint modules)

- [x] **5.2 One filter path for declaration commands**
    - Replace the per-command kind-and-filter logic in `summarize_file` and
      the filtering at `cli/src/main.rs:1056-1084` with the shared library
      query/filter from Phase 4; records are filtered once before
      presentation.
- [x] **5.3 `classes` semantics**
    - Kind, scope, name, visibility, test, docs and deprecation filters select
      containers; `--in` matches the container's own name; async, return/param
      type, parameter count and static/instance select displayed members. A
      selected container may have zero members. `--json` stores a `members`
      map keyed by container ID.
- [x] **5.4 `imports` selectors**
    - Name and file filters; `--name` matches alias or original name
      (`--name HashMap` and `--name Map` both find `use … HashMap as Map`).
      Positional tokens stay file-only.
- [x] **5.5 Explicit file/name selectors**
    - `--file`/`--name` bypass `is_file_filter_token`; file and name lists
      intersect, values within a list are alternatives, exclusions win.
      File references go through `biscuit_file::FileReference`; `.gitignore`
      and default fixture exclusions stay, explicit targets re-included.
      `--name` accepts `#`, `::`, `.` qualified patterns anchored at start
      unless leading `*`.

### Wave 3 (sequential)

- [x] **5.6 Rewrite superseded CLI tests**
    - Case-sensitive matching expectations, locals shown by default, old
      `classes` filtering, the static/instance flag combination.
- [x] **5.7 Focused CLI cases**
    - Filter intersections and conflicts; exact vs smart-case including
      non-ASCII; `--file`/`--name` vs positional; file-qualified and
      namespace `--name` including the leading `*`; class/member counts;
      `--kind type_alias`, unknown `--kind`; `imports --name`; `--undocumented`
      excluding `None`; empty selections succeed; analysis errors still fail
      on stderr; `hug symbols` locals default and `--all-scopes`; warm run
      from a different scan root reusing the entry with a different prefix.

**Validation checkpoint 5:** `just test` and `just lint` green;
`hug symbols` on `tree-hugger/` is about 2,550 lines and `--all-scopes`
restores the 8,593-line baseline set.

---

## Phase 6: Reports

### Wave 1 (parallel; disjoint report shapes)

- [x] **6.1 JSON Lines** (`--jsonl`)
    - One envelope per selected record: `report_schema_version`,
      `schema_version`, `file`, `language`, `completed_passes`,
      `completed_capabilities`, `record`; class lines add `members` (a list).
      No source blobs. Files in portable path order, `--sort` orders records
      within each file, bounded buffering via `scan::in_file_order`; never
      worker order. Broken pipe exits quietly; read/parse failures go to
      stderr with nonzero exit even after partial output; empty selection is
      zero lines. Always requests `Flags` (Phase 3.7 hook).
- [x] **6.2 Outline view** (`--tree`)
    - Nest members under containers by container ID (out-of-body Rust impl
      and Go receivers included), repeated impls grouped, marked contextual
      heading for an excluded/outside-file parent (not counted), overloads in
      source order. Combination table: `--sort` orders containers and members
      separately, `--group-by-file` no-op, `--group-by-module` adds a
      directory heading, `--format plain` accepted, `--format table` and
      `--json`/`--json-full`/`--jsonl`/`--count`/`--stats` argument errors.
- [x] **6.3 Signature modes** (`--signatures full|short|none`)
    - `full` = `signature_text`; `short` default (field lists truncated past 4
      as `{ a, b, c, d, … +3 }`); `none` names only with location/container.
- [x] **6.4 Count and stats model**
    - `--count`: one decimal integer plus newline. Stats aggregation by kind,
      visibility, language, doc coverage (`documented` over known-docs
      `total`, `unavailable` separately, null `ratio` when `total` is 0),
      synthetic exports reported apart; classes count containers only;
      `imports --stats` is an argument error.

### Wave 2 (sequential; shared renderer)

- [x] **6.5 Renderer changes**
    - `UnorderedList`/`Prose` wrapping at terminal width with hanging indent
      for long signatures (inject dimensions for L1 tests); `--format plain`
      and `--plain`, redirected output and machine formats disable ANSI and
      OSC 8; escape signature text as literal component content so control
      sequences or markup cannot act. `--format table`: biscuit-terminal
      `Table` with Location (`file:line`, OSC 8 when interactive), Kind,
      Visibility, Name, Signature (wraps in column). No change to the
      `biscuit-terminal` package.
- [x] **6.6 Stats rendering**
    - Terminal: three `Table`s (kind, visibility, language) plus one
      doc-coverage line including `unavailable`; `--format table` identical,
      `--format plain` unstyled. JSON: `{report_schema_version, root_dir,
      stats:{by_kind,by_visibility,by_language,doc_coverage,
      synthetic_exports}, errors}` with trailing `errors`, no record
      `schema_version`, nonzero exit when `errors` is non-empty.
- [x] **6.7 Output-mode validation**
    - Exactly one mode among terminal, `--json`, `--json-full`, `--jsonl`,
      `--count`, `--stats`, `--format`, except `--stats` composing with
      `--json`, default/`--format table`, or `--format plain`; `--stats`
      with `--sort`, `--signatures`, `--tree`, `--json-full`, `--jsonl` or
      `--count` is an argument error. Table-driven test over the mode pairs.

### Wave 3 (sequential)

- [x] **6.8 CLI goldens** (over the 16-language `multi/` tree, after
      `strip_ansi_codes` and path redaction): `--format table`,
      `--format plain`, `--jsonl`, `--tree`, `--stats`, `--stats --json`,
      `--signatures full`; `--count` asserted as one integer line.
- [x] **6.9 Focused report cases**
    - JSONL parsing/round-trip, envelope fields, class `members`, failure
      after partial output; `--jsonl --sort name|kind` over several files;
      `--stats --json` with a failing file; `doc_coverage.unavailable` from a
      Lua/Bash file and null `ratio` on zero matches; `--stats` with each
      filter and the accepted/rejected combinations; `--tree` combinations;
      width wrapping with injected dimensions; escaped control sequences;
      `--json`/`--jsonl` carrying computed flags without a filter; stale
      `EXTRACTOR_REVISION` entry.
- [x] **6.10 L2 terminal test** (parallel with 6.9)
    - Extend `level2_declarations` for table and wrapped rendering via the
      shared harness (`cli/tests/support/terminal_report.rs`); windows must
      not take focus; scope rows between markers, never a whole capture.

**Validation checkpoint 6:** `just test` and `just test-l2` green; goldens
reviewed for OS-independence (`/` separators, no width dependence outside
injected-width tests).

---

## Phase 7: Docs, release notes, performance sample and close

### Wave 1 (parallel documentation tasks; disjoint files)

- [x] **7.1 `docs/symbol-kinds-and-visibility.md`**: scope (parameters only
      where extracted; Bash/Zsh nested functions Local), `async`, test rules
      and the out-of-line `cfg(test)` limit, deprecation table, qualified-name
      spelling per language, `parameters`/`has_docs`, unsupported extraction.
- [x] **7.2 `docs/symbol-reports.md`**: every filter with an example, each
      format and view with a short sample, `--tree` combinations (Mermaid
      where a flow is clearer), the output-mode rule and `--stats` exception,
      JSONL ordering and both envelopes, the two `members` shapes. Written
      for a reader new to the repository; no link to this feature or spec.
- [x] **7.3 `docs/symbol-schema-design.md` and `docs/cache-design.md`**:
      additive fields, empty facets, library-only naming context, post-lookup
      prefix and why neither naming context nor scan root is a key input, the
      ID change, always-computed vs capability facts, metric absence
      semantics, and the new robustness-table rows.
- [x] **7.4 `README.md`, CLI help text, completions, `.claude/skills/tree-hugger/SKILL.md`
      and `api-reference.md`**: new flags, facets, `SymbolQuery` predicates.
- [x] **7.5 Release notes** with before/after examples for smart-case matching
      and locals hidden by default in `hug symbols`.
- [x] **7.6 Unscheduled entries**: fix text under `features/_unscheduled/`
      that this feature's decisions made stale (for example the `lines`
      metric now belonging to `symbol-type-relations`). Check
      `docs/dependencies.md` only if a dependency declaration changed.
- [x] **7.7 Comment drift pass**: every behavior-changing symbol gets its
      `///`/`//!` and inline comments reviewed; report any drift found and how
      it was resolved.

### Wave 2 (sequential)

- [x] **7.8 Performance sample**
    - Re-run the symbol-improvements performance sample once: same host,
      commands and inputs, release build; record elapsed time, serialized
      bytes, peak RSS, build and cache state in `implementation-log.md`. No
      hard gate; record any regression with its cause. Nothing wider (ruling
      9).
- [x] **7.9 Final validation**
    - `just test`, `just test-l2`, `just lint` in `tree-hugger/`; run the
      `os` skill's checks for path and width tests; verify `git status` shows
      no stray files. Do not run `cargo fmt`, do not commit, do not move the
      feature to `_completed`.

**Validation checkpoint 7 (done):** every item in the spec's definition of
done is checked; the implementation log records departures from the spec (the
spec itself is left unchanged); terminal state is "implementation complete,
ready for review".
