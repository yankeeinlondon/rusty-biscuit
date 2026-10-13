---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-03-symbol-improvements/spec.md
plan: tree-hugger/features/2026-10-03-symbol-improvements/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
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
  - tree-hugger/lib/tests/fixtures/symbols/multi/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/multi/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/multi/c.c
  - tree-hugger/lib/tests/fixtures/symbols/multi/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/multi/java.java
  - tree-hugger/lib/tests/fixtures/symbols/multi/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/multi/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/multi/nested/go.go
  - tree-hugger/lib/tests/fixtures/symbols/multi/nested/python.py
  - tree-hugger/lib/tests/fixtures/symbols/multi/perl.pl
  - tree-hugger/lib/tests/fixtures/symbols/multi/php.php
  - tree-hugger/lib/tests/fixtures/symbols/multi/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/multi/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/multi/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/multi/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/multi/zsh.zsh
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/header/mod.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_header.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_declarator_wrappers.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_field_lists.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/shared/schema_v2/kinds.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
docs_updated_during_phase_3:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/lib/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_4:
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/shared/diagnostic_metadata.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_5:
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/classes.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/order.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/classes.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/symbols/prelude.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
docs_updated_during_phase_5:
  - tree-hugger/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/refactor.md
  - docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - tree-hugger/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
docs_created_during_phase_6:
  - tree-hugger/docs/symbol-reports.md
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_code:
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
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
  - tree-hugger/lib/tests/fixtures/symbols/multi/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/multi/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/multi/c.c
  - tree-hugger/lib/tests/fixtures/symbols/multi/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/multi/java.java
  - tree-hugger/lib/tests/fixtures/symbols/multi/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/multi/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/multi/nested/go.go
  - tree-hugger/lib/tests/fixtures/symbols/multi/nested/python.py
  - tree-hugger/lib/tests/fixtures/symbols/multi/perl.pl
  - tree-hugger/lib/tests/fixtures/symbols/multi/php.php
  - tree-hugger/lib/tests/fixtures/symbols/multi/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/multi/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/multi/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/multi/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/multi/zsh.zsh
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/header/mod.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_header.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_declarator_wrappers.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_field_lists.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/shared/schema_v2/kinds.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/shared/diagnostic_metadata.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/classes.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/order.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/symbols/classes.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/symbols/prelude.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
documentation:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/README.md
  - tree-hugger/docs/refactor.md
  - docs/dependencies.md
  - tree-hugger/docs/symbol-reports.md
completed_phase: 6
implemented: true
---

# Implementation Log for 2026-10-03-symbol-improvements (6 phases)

## Phase 1

### Baseline (measured before any code change)

- **Tree:** commit `60e3b841e5fbc65fb139e05688f0092fcbb07c8d` (`feat/symbols`), only
  the plan/spec/log files modified; no source changes.
- **Build:** `cargo build --release -p tree-hugger-cli` (workspace `release`
  profile). A copy of the binary was kept at `/tmp/th-baseline/hug-pre`
  (volatile; rebuild from the commit above if it is gone).
- **Host:** Apple M4 Max, 16 cores, macOS. **Load average was 38–45 during the
  timing runs** (other sessions compiling), so wall-clock times below are
  noisy. RSS was stable across runs.
- **Cache state:** `--no-cache` runs bypass the persistent cache; the warm
  `functions` run followed a priming `hug functions`, with the user's normal
  `TMPDIR` cache.

#### Peak RSS (the required preparatory measurement)

Run from the repository root:
`/usr/bin/time -l hug symbols --no-cache > /dev/null`

| Run | Elapsed | Maximum resident set size |
|---|---|---|
| 1 | 275.3 s | 3,478,781,952 B (3.48 GB) |
| 2 | 238.1 s | 3,481,255,936 B (3.48 GB) |
| 3 | 184.3 s | 3,486,515,200 B (3.49 GB) |

**Baseline peak RSS: 3.48 GB** (max of the three: 3,486,515,200 B).

#### Inputs in `tree-hugger/`

The spec's baseline was taken over 66 files; `tree-hugger/` now scans **108
files** (pathless scan: fixtures and snapshots skipped), so the absolute
numbers differ from the spec's table. The after-sample must use these same
inputs.

| Command (run in `tree-hugger/`) | Result |
|---|---|
| `hug --json functions` | 39,165,347 B (39.2 MB): 1,580 selected functions plus all 11,029 full v2 records in `symbol_indexes` |
| `hug --json symbols` | 49,980,073 B (50.0 MB) |
| `hug symbols` | 11,041 lines |

#### Timing (hyperfine, `-N --warmup 1 --runs 5`, loaded host)

| Command (run in `tree-hugger/`) | Mean ± σ | Min | Max |
|---|---|---|---|
| `hug functions --no-cache` | 3.523 s ± 0.374 | 3.131 | 3.917 |
| `hug imports --no-cache` | 3.190 s ± 1.758 | 1.703 | 5.802 |
| `hug lint --no-cache` | 6.812 s ± 1.584 | 5.609 | 9.343 |
| `hug functions` (warm) | 4.222 s ± 0.803 | 3.008 | 5.183 |

A warm run slower than a cold one shows the load noise. **Recommendation for
the Phase 6 sample:** rebuild the pre-change binary from the commit above and
time it back-to-back with the post-change binary in one hyperfine invocation,
so both see the same load; compare ratios, not these absolute numbers.

A first scripted attempt produced bogus 0.01–0.05 s timings: zsh does not
word-split an unquoted `$c`, so `hug "functions --no-cache"` failed fast. Those
numbers were discarded.

### Pre-change goldens

- Captured with the pre-change binary (`/tmp/th-baseline/hug-pre`) over a
  scratch Git copy of `lib/tests/fixtures/symbols/multi/`, isolated `HOME` and
  `TMPDIR`, `NO_COLOR=1`, `--no-cache`. Two runs were byte-identical.
- `cli/tests/fixtures/golden/symbols.plain`: `hug --plain symbols` (129 lines,
  no absolute paths). It is now **pinned by an L1 test**,
  `tree-hugger-cli::symbols golden::plain_symbols_over_multi_match_the_golden`,
  which compares after `strip_ansi_codes` + `redact_snapshot_text`, so phases
  2–4 get a regression guard on today's layout. Regenerate with
  `HUG_UPDATE_GOLDEN=1` only for a deliberate difference, and log each one.
  A mutation check (renaming one golden line) made the test fail with the
  expected/actual line, then the golden was restored byte-for-byte.
- `features/2026-10-03-symbol-improvements/baseline/symbols.pre-change.json`:
  today's `hug --json symbols` (pretty-printed, 511 KB) with the scratch root
  replaced by `<ROOT>` and the 121 `created_at_epoch_ms`/`updated_at_epoch_ms`
  pairs zeroed. A reference for record content only; it is not the Phase 5
  `symbols.json` golden (the envelope changes) and no test reads it.
- Today's layout, for the Phase 5 renderer: a flat list sorted by name
  (case-insensitive), files in a nested directory (`nested/`) under their own
  heading. Two **pre-existing label defects** are visible and expected to change
  deliberately in later phases: Python `MAX_SIZE = 10` renders as
  `= MAX_SIZE` (text-guessed binding token), and Go
  `func (w *Widget) Grow(amount int)` renders the return as `(amount int)`.

### Fixtures

- **`lib/tests/fixtures/symbols/multi/`** (new): 16 files, one per language;
  `nested/go.go` and `nested/python.py` sit in a nested directory; `Widget` is a
  type in both `rust.rs` and `nested/go.go` (the `--sort name` tie-break case).
  Guarded by `symbols_coverage::multi_tree_has_one_clean_file_per_language`
  (one cleanly parsing file per language, a nested file, `Widget` in exactly
  two files). Helpers `multi_fixture_dir()` / `multi_fixtures()` live in
  `symbols_fixtures.rs`. `Scratch::new` (CLI tests) now creates parent
  directories so a nested fixture path can be written.
- **Per-language fixtures extended**: each primary fixture gained a trailing
  "Declaration boundaries" section (names start with `boundary`/`Boundary`, or
  the Java/C# class `DeclarationBoundaries`, so no existing matrix row moved).
  No existing test expectation changed; the full L1 suite (726 tests) passed without edits to any existing test.

| Language | Large initializer | Multi-line header | In-node annotation | Annotation between modifiers | Keyword comment |
|---|---|---|---|---|---|
| Rust | `BOUNDARY_TABLE` | `boundary_multi_line_header` | `#[derive]` on `BoundaryAttributed` (sibling `attribute_item`, outside the node) | n/a | yes |
| Go | `boundaryTable` | `boundaryMultiLineHeader` | n/a (no annotations) | n/a | yes |
| Swift | `boundaryTable` | `boundaryMultiLineHeader` | `@available` on `boundaryAnnotated` | n/a | yes |
| Scala | `boundaryTable` | `boundaryMultiLineHeader` | `@deprecated` on `boundaryAnnotated` | n/a | yes |
| TypeScript | `BOUNDARY_TABLE` (+ arrow `boundaryArrow` for trailing `=>`) | `boundaryMultiLineHeader` | `@boundarySealed` class, `@boundaryLogged` method | n/a | yes |
| JavaScript | same as TypeScript | same | same | n/a | yes |
| Java | `DeclarationBoundaries.BOUNDARY_TABLE` | `boundaryMultiLineHeader` | `@Deprecated boundaryAnnotated` | `public @Deprecated static boundaryAnnotationBetweenModifiers` | yes |
| C# | `DeclarationBoundaries.BoundaryTable` | `BoundaryMultiLineHeader` | `[Obsolete] BoundaryAnnotated` | **n/a: C# requires attributes before modifiers; `public [Obsolete] static void M()` is a syntax error in tree-sitter-c-sharp (verified)** | yes |
| C | `boundary_table` | `boundary_multi_line_header` | `[[maybe_unused]] boundary_annotated` | n/a | yes |
| C++ | `boundaryTable` | `boundaryMultiLineHeader` | `[[nodiscard]] boundaryAnnotated` | n/a | yes |
| PHP | `BOUNDARY_TABLE` | `boundary_multi_line_header` | `#[Deprecated] boundary_annotated` | n/a | yes |
| Python | `BOUNDARY_TABLE` | `boundary_multi_line_header` | `@boundary_marker boundary_decorated` (`decorated_definition` wrapper) | n/a | yes |
| Perl | `%boundary_table` | `boundary_multi_line_header` (brace on next line) | not added (sub attributes untested in grammar) | n/a | yes |
| Lua | `boundaryTable` | `Widget.boundaryMultiLineHeader` | n/a | n/a | yes |
| Bash | `BOUNDARY_TABLE` array | `boundary_multi_line_header ()` + brace on next line | n/a | n/a | yes |
| Zsh | `BOUNDARY_TABLE` array | **not representable: tree-sitter-zsh rejects a body brace on the line after the header (`name()` then `{`), which real zsh accepts**; `boundary_header` keeps a one-line header | n/a | n/a | yes |

All new declarations are extracted by the current code (checked with the
pre-change binary); Python's `BOUNDARY_TABLE` shows the same `= NAME` label
defect as `multi/`.

### Recon notes

- **Pipeline** (`lib/src/analysis/mod.rs`): `analyze_file` always runs
  `parse_pass` first, then each requested pass in the given order (no planner).
  `parse_pass` calls `symbol_records()`, `imported_symbols()`,
  `exported_symbols()` and **`try_diagnostics()` unconditionally** (line ~101),
  and stamps `SchemaVersion::V3_0`. `AnalysisPass` (`shared/schema_v2/types.rs`)
  has Parse/Bind/Semantic/Docs.
- **CLI scan** (`cli/src/main.rs`, `analyze_tree_file` ~1634): every file is
  opened with `TreeFile::with_language` (a full parse) **before** any cache
  lookup, and every hit (in-process, persistent, legacy in-memory) goes through
  `hydrate_diagnostics`, which runs `try_diagnostics()` again. So a warm
  symbol command still parses and runs diagnostics on every file, which
  explains why warm costs about as much as cold. The cache key's
  `query_fingerprint` is a hand-written string carrying `EXTRACTOR_REVISION`;
  `grammar_fingerprint` is just the language's query name.
- **Classes path**: `CommandKind::Classes` (~1037) loops files sequentially,
  calling `analyze_tree_file` then `extract_class_summaries`.
- **Callers to remove/replace later:**
  - `symbol_presentation` (`main.rs:603`): called from `render_symbol_line`
    (`:2271`) and `render_symbols` (`:2324`).
  - `binding_token_for_symbol` (`main.rs:645`): called only from
    `symbol_presentation` (`:625`).
  - `SOURCE_LINE_CACHE` (`main.rs:543`): used only by `source_line_for_symbol`
    (`:760`), itself called from `symbol_presentation` (`:606`) and
    `binding_token_for_symbol` (`:646`).
  - `InMemorySymbolCache` (`lib/src/cache/mod.rs:536`): CLI import `main.rs:20`,
    constructed `:1021`, parameter of `analyze_tree_file` `:1637`; library unit
    test `cache/mod.rs:672`; `lib/tests/l1/cache_tests.rs` (several tests).
  - `InProcessCache` (`lib/src/cache/mod.rs:218`): CLI `main.rs:20`, `:1022`,
    `:1638`; library unit test `cache/mod.rs:733`; `lib/tests/l1/cache_tests.rs`.
- **`identity.module_path`** is read in `lib/src/analysis/mod.rs`,
  `analysis/tests.rs`, `builtins.rs`, `file/tree_file.rs`,
  `god_files/analysis.rs` (2), `package/tree_package.rs` (3) and
  `shared/schema_v2/{facets,mod}.rs`; Phase 2 should confirm which of the
  `tree_package`/`god_files`/`builtins` hits are the record field rather than
  an unrelated local of the same name.

### Rulings

| # | Ruling | Status | Evidence |
|---|---|---|---|
| 1 | Chunks of `4 × rayon::current_num_threads()`, each serialized in file order | **Accepted.** Note the baseline peak RSS of 3.48 GB with today's sequential scan; the chunk bound matters most for the RSS goal. | — |
| 2 | Which flags are "symbol selectors" that `--json-full` rejects | **Accepted, with the concrete list below.** | clap definitions in `main.rs:60–200` |
| 3 | `classes --json` `members` hold the flag-filtered set | **Accepted.** | — |
| 4 | Marker for synthetic prelude records | **Accepted: add an additive field inside the 4.0 bump.** No existing marker: prelude builds v1 `SymbolInfo` directly (`prelude.rs::symbol_from_prelude_export`), and v2 records only have the free-form `extensions` map and `ProvenanceFacet { extractor, extractor_version, parse_pass, timestamps }`. Recommended placement: `provenance.synthetic: bool`, serialized only when `true`. | `facets.rs:316`, `prelude.rs:298` |
| 5 | `Unknown` for `is_recursive` | **Confirmed, no new type needed.** `SemanticFacet::is_recursive: Option<TriState>` and `TriState { Yes, No, Unknown }` already exist (`facets.rs:281–299`). Today `semantic_pass` sets `Yes`/`No` from a name heuristic (`analysis/mod.rs:219–225`); v1 `kinds.rs:177` is a plain `bool`. | `facets.rs`, `analysis/mod.rs` |
| 6 | Coverage encoding: `covered_kinds` / `completed_capabilities`, explicit "all", malformed = miss | **Accepted.** | — |
| 7 | Adapters read `container_name`; `--group-by-module` groups by directory | **Accepted and confirmed.** `module_key_for_symbol` (`main.rs:2127`) groups by the display path's parent directory and never reads `identity.module_path`. | `main.rs:2127` |
| 8 | Wider measurement | Not scheduled (author decision). | — |

**Ruling 2, concrete selector list.** Rejected with `--json-full` (they narrow
records):

- positional `FILTER` tokens that are **not** file-like (symbol-name filters:
  fuzzy, `*glob*`, `name!`), on `functions`/`types`/`symbols`/`imports` and
  `classes`;
- `--exclude-symbols <GLOB>` (`ScanOptions`);
- `--exported`, `--prelude` (`CommonArgs` and `ClassArgs`);
- `classes` only: `--name/-n`, `--static-only`, `--instance-only`.

Allowed (they select files or change display): file-like positional `FILTER`
tokens (paths, globs, extensions), `--exclude-files`, `--language`,
`--no-cache`, `--comments`, `--group-by-file`, `--group-by-module`, and the
`--sort` that replaces `--sort-by-kind`/`--sort-by-module`. Because positional
filters mix file and symbol tokens, the rejection must use the same
file-vs-symbol classification the scan already applies, not "any positional
argument".

### Requirement → test mapping (Phase 1)

Phase 1 changes no library or CLI behavior; its tests are guards for later
phases.

| Requirement | Test |
|---|---|
| `multi/` holds one cleanly parsing file per language, a nested file, a shared type name | `tree-hugger::l1 symbols_coverage::multi_tree_has_one_clean_file_per_language` (new) |
| Today's `hug --plain symbols` layout over `multi/` is pinned | `tree-hugger-cli::symbols golden::plain_symbols_over_multi_match_the_golden` (new; mutation-checked) |
| Extended per-language fixtures still parse cleanly and keep every existing expectation | existing `symbols_coverage::every_symbols_fixture_parses_without_syntax_errors` and every `symbols_*` matrix (unchanged, all pass) |

The Input Robustness Matrix does not apply to Phase 1 (no parser/reader
change).

### Gates run

- `just test` (tree-hugger + tree-hugger-cli, L1): **726/726 passed**,
  including the two new tests (also run targeted).
- `just lint` (clippy `--all-targets -D warnings`, both packages): clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `just cross-check tree-hugger-cli --os linux golden`: **pass**.
- `just cross-check tree-hugger-cli --os windows golden`: **not run, host
  unreachable** (`ssh: connect to host … port 2222: No route to host`). The
  golden's Windows exposure is path rendering (`nested/go.go`); the CLI's
  `display_path` already emits slash-separated paths and the comparison
  normalizes CRLF, so the risk is low; Windows CI on `main` will cover it.
- `just test-l2` not run: Phase 1 adds no L2 test and changes no code the L2
  `god-files` test exercises.

### Incidents

- In this shell `cd` is a zoxide alias: `cd tree-hugger` from inside
  `tree-hugger/` jumped to a different checkout
  (`/Volumes/coding/personal/rusty-biscuit/tree-hugger`) and one `just lint`
  ran there (read-only clippy; nothing written). Lint was rerun in the
  worktree with absolute paths. Every other command was verified to have run
  in the worktree.

## Phase 2

Record foundation: schema 4.0, `identity.keyword`, `source.signature_text`.
Implemented by claude/opus on 2026-10-04.

### Wave 1: schema 4.0

- `SchemaVersion::V4_0` added and emitted on records, file indexes, the CLI
  envelope and the CLI's analyzer fingerprint; `V3_0` stays as a documented,
  no-longer-emitted constant.
- `ProvenanceFacet` lost `created_at_epoch_ms` / `updated_at_epoch_ms`; the
  three `updated_at` stamps in `analysis/mod.rs` and the adapter's `now` are
  gone. **Also removed `SymbolSnapshot::created_at_epoch_ms`** (the cache
  entry's own timestamp) and the then-unused public `now_epoch_ms()`: the
  plan's success criterion is "no timestamps anywhere" and spec A9 wants
  identical cache entries across runs. No caller outside tree-hugger used
  either (grepped the monorepo; `schematic`'s `module_path` is unrelated).
- `identity.module_path` → `identity.container_name` (doc comment added).
  Real record-field readers renamed: `analysis/mod.rs` (bind pass),
  `tree_file.rs` (ordinal key), `god_files/analysis.rs` (top-level filter and
  its comments), both adapters, tests. `builtins.rs` (`module_path` is a Rust
  macro name) and `tree_package.rs` (a local variable) were not the field and
  are untouched.
- `identity.keyword: Option<String>` with
  `skip_serializing_if = "Option::is_none"`, matching the neighboring
  optional identity fields (no `#[serde(default)]`; serde already reads an
  absent `Option` as `None`).
- `EXTRACTOR_REVISION` 4 → 5. Cache-upgrade rule from the skill: the existing
  literal-key and revision 1–3 cases are unchanged; added
  `cache_upgrade::entry_from_extractor_revision_four_is_not_served_after_the_repairs`
  (schema-3.0 key at revision 4; probe = `module_function`'s keyword, `def`
  repaired vs. absent stale; control proves the stale payload is servable
  under the current key).
- **Ruling 4 revised:** records already have `visibility.is_synthetic: bool`
  (`facets.rs`, `VisibilityFacet`), which Phase 1's recon missed. A second
  marker (`provenance.synthetic`) would duplicate it, so none was added;
  Phase 5's synthetic prelude records should set `visibility.is_synthetic`.
- **Not changed:** `identity.qualified_name` is already populated today by
  `From<SymbolInfo> for SymbolRecord` as `container::name` (216 occurrences in
  the Phase 1 baseline JSON). The spec forbids *newly* populating it here; the
  existing value is left as is for `2026-10-03-symbol-filters-and-reports`
  to own.

### Wave 2: keyword and signature text (all 16 languages)

**Departure from the plan's wave shape:** the plan split Wave 2 into four
parallel per-language-group subagents behind a shared stub helper. Dumping
the declaration nodes of every fixture record first (a throwaway in-crate
test, since removed) showed that the signature boundaries are the same few
node-level rules in every grammar (body / `value`-like fields, the first
`=`/`:=` in declarator children, terminator, leading annotations), with only
the keyword placement varying. One shared engine plus one per-language table
was smaller and consistent where four copies would have drifted, so it was
implemented directly rather than delegated.

New module `lib/src/file/extract/header/`:

- `keyword.rs`: `declaration_keyword(declaration, language)`, a
  `(language, declaration node kind) → KeywordSite` table. Sites: a direct
  anonymous child from a token list; the token inside a named wrapper child
  (Perl `scope`, Swift `value_binding_pattern`); the token inside a wrapper
  under a field (Swift protocol property name pattern); a field token
  (JS/TS `lexical_declaration.kind`, Swift `declaration_kind`); a marker node
  (PHP `var_modifier` is a named leaf with no token → `var`); or a token of the
  wrapping statement (Go `const_spec`/`var_spec`/`type_spec`, Lua
  `local` assignment, shell `declaration_command`). Only anonymous tokens are
  read, and the token's *kind* is reported (PHP keywords are
  case-insensitive in source).
- `mod.rs`: `signature_text(declaration, name_bytes, language, source)`.
  Start = first child that is not a comment or a leading annotation (inside a
  `modifiers` list, only annotations before the first modifier are skipped).
  End = earliest of: the first child with a cut field (`body`, `value`,
  `accessors`, `computed_value`, `default_value`) or cut kind
  (`protocol_property_requirements`, `arrow_expression_clause`), moved back
  over a preceding `=`/`=>`/`:`/`:=` token; `body_node_for_declaration`; the
  first `=`/`:=` in the declaration or its declarator children; a Go struct
  field list / interface brace; a final `;` (Lua `end`). `None` for a C/C++
  type with an inline body, a grouped Go `type (…)`, or when the declaration
  contains the name but the header does not (`int a = 1, b = 2;` for `b`).
- Wired into `TreeFile::symbol_records()` next to `declaration_text`, from the
  same `declaration` node in the same loop.

Decisions taken while implementing (all within the spec's wording):

| Decision | Choice | Why |
|---|---|---|
| Keyword is one token or a phrase | One token: Scala `case class` → `class`, C++ `enum class` → `enum`, Swift `class func` → `func`, Lua `local function` → `function` | The token that names the construct; a fixed shape is easier to filter on |
| Keyword of a Go spec / Lua or shell `local` | Reported from the wrapping statement | It is the declaring keyword in the grammar; `declaration_text` does not include it, which the docs state |
| Type-alias header | Excludes the aliased type (`pub type Alias`, `using SizeAlias`, `type ExportedAlias`); a C `typedef` keeps its full text (name is last) | The aliased type is the alias's initializer and already lives in `kind_data` |
| Macro header | `#define NAME` / `#define NAME(x)` | The replacement text is the macro's initializer |
| JS/TS arrow function | `(value: number): number`, keyword `None` | The declaration node is the `arrow_function` (fixed by the prerequisite fix); its header has no name and no keyword |
| Keyword table scope | Only declaration kinds the extractor captures | Probing found generators, class/function expressions, Scala `package`/`type`/`given`/enum cases, Swift `init`/`deinit`/`subscript`/`typealias`, C++ `concept`/variadic/default type params, Java `@interface`/`module`, C# `delegate`, PHP enum `case`/`namespace`, Python `type`, TS `namespace` produce no record today; their entries were removed rather than kept untested |

### Wave 3: matrices and guards

New `lib/tests/l1/symbols_header.rs` (declared in `tests/l1/main.rs`):

- `KEYWORD_MATRIX` (125 rows, every language; includes locals: Rust `let`,
  JS/TS `const`, Swift `let`, Perl `my`/`state`, Lua/Bash/Zsh `local`, and
  `None` rows) → `keyword_matrix_holds_for_every_language`.
- `SIGNATURE_MATRIX`: every language × every `Shape` (`FunctionHeader`,
  `Initializer`, `MultiLineHeader`, `LeadingAnnotation`,
  `AnnotationBetweenModifiers`, `BodyIntroducer`, `Terminator`) is either a
  fixture record with its exact header or an explicit `NotApplicable(reason)`
  / `Unsupported(reason)`. Zsh `MultiLineHeader` is the one `Unsupported`
  cell (grammar rejects the next-line brace); C# `AnnotationBetweenModifiers`
  is `NotApplicable` (attributes must precede modifiers).
  → `signature_matrix_holds_for_every_language`.
- `symbols_coverage::every_language_has_matrix_rows` now also fails on a
  language with no keyword row or with a `Shape` missing from the signature
  matrix.
- Passive corpus: `every_fixture_signature_is_a_contiguous_part_of_its_declaration_text`
  (all 21 fixtures, >500 records: non-empty, trimmed, substring of
  `declaration_text`, no trailing `;`/`=`).
- `headers_that_cannot_be_isolated_are_none` (C typedef/struct-with-body,
  C/Java/TS/Swift multi-declarators, Python chained assignment, grouped Go
  types; plus the positive `int first` control).
- `a_keyword_inside_a_comment_never_becomes_the_keyword` (comments *inside*
  the declaration node naming other keywords, in Rust, Go, C, TS, PHP, Bash);
  the fixtures' "comment naming declaration keywords" lines are covered by
  the `BOUNDARY_TABLE` / `boundaryTable` keyword rows.
- `the_documented_comparison_rows_hold`: the spec's six comparison rows (plus
  the C# property) as complete sources, asserting all three fields.
- `less_common_declarations_report_their_keyword`: 13 inline constructs not in
  fixtures (`macro_rules!`, Swift `actor`, TS `let` and overload signature,
  C# `event`, C++ `typename` parameter, function-like `#define`, grouped Go
  `var`/`type`, Lua multi-name `local`, Bash `declare`/`export`, Zsh
  `typeset`).

Mutation checks (each reverted, verified with `cmp`): emptying the
initializer-token list failed 11 `Initializer` rows; dropping direct-child
annotation skipping failed the JS, PHP, C, C++, C# and Scala
`LeadingAnnotation` rows.

### CLI and cache tests

- `wiring::json_records_carry_keyword_signature_text_and_container_name`:
  real `hug --json symbols` over the TypeScript fixture; keyword, header,
  `container_name`, `declaration_text` keeping the initializer; output
  contains no `"module_path"`, `created_at_epoch_ms`, `updated_at_epoch_ms`.
- `wiring::json_symbols_carry_visibility_and_schema_4_0` (renamed from
  `…_3_0`): envelope and index carry `{4, 0}`.
- `golden::json_symbols_over_multi_are_identical_across_runs`: two
  `hug --json symbols --no-cache` runs over a scratch copy of `multi/` are
  byte-identical (checkpoint 2's determinism item; the per-record
  timestamps made this impossible before).
- `golden::plain_symbols_over_multi_match_the_golden` still passes unchanged:
  **Phase 2 changes no plain output**, so the golden was not regenerated.
- `cache_robustness`: two new columns, `identity.keyword` and
  `source.signature_text`, with the same outcome row as `declaration_text`
  (absent/`null`/`""` served as no value / empty; every other malformed shape
  recomputed). Its timestamp-stripping `normalized()` helper was deleted:
  records have no timestamps, so cached and uncached output are now compared
  byte for byte (a stronger check). `docs/cache-design.md`'s table gained the
  two rows ("twelve fields").

Input Robustness Matrix: applies only through the persisted cache snapshot,
the one reader of the new fields. Both new fields are `Option<String>` with no
`#[serde(default)]`; their full row (absent, null, wrong type, one/every wrong
element, empty, duplicate key, trailing garbage) is walked by
`cache_robustness` from a real entry written by `hug`, with the control cell
proving the unedited entry is served. Absent and null both read as `None`
here by the same schema rule `declaration_text` already follows; the plan's
Phase 4 matrix (coverage fields) is where absent-vs-null must differ.

### Requirement → test mapping (Phase 2)

| Requirement | Test(s) |
|---|---|
| Schema 4.0 on records, indexes, envelope | `tree_file::emits_symbol_records_v2`, `tree_file::emits_symbol_index_v2_with_completed_passes`, `symbols_adapter::serialized_record_round_trips_the_new_fields`, `wiring::json_symbols_carry_visibility_and_schema_4_0` |
| No timestamps; deterministic records | `golden::json_symbols_over_multi_are_identical_across_runs`, `wiring::json_records_carry_keyword_signature_text_and_container_name`, `cache_robustness` (byte-equal cached vs. uncached) |
| `module_path` → `container_name`; v1 adapters keep working | every `symbols_*` matrix (they look records up by `container_name`), `symbols_adapter::*`, wiring test |
| Revision-4 entries rejected, not migrated | `cache_upgrade::entry_from_extractor_revision_four_is_not_served_after_the_repairs` |
| Keyword from the node, `None` where none, all 16 languages | `symbols_header::keyword_matrix_holds_for_every_language`, `…less_common_declarations_report_their_keyword` |
| Keyword in a comment never used | `symbols_header::a_keyword_inside_a_comment_never_becomes_the_keyword`, keyword-comment rows in the matrix |
| Signature contracts per shape, N/A explicit, unsupported labeled | `symbols_header::signature_matrix_holds_for_every_language` + coverage guard |
| Contiguous sub-range of `declaration_text` | `symbols_header::every_fixture_signature_is_a_contiguous_part_of_its_declaration_text`, `…the_documented_comparison_rows_hold` |
| Unisolable → `None`, never the body | `symbols_header::headers_that_cannot_be_isolated_are_none` |
| Coverage guard fails on a missing matrix row | `symbols_coverage::every_language_has_matrix_rows` |
| New fields robust in the persisted cache | `cache_robustness::persisted_symbol_fields_are_served_or_recomputed_per_shape` |
| Plain output unchanged | `golden::plain_symbols_over_multi_match_the_golden` |

### Gates run

- `just test` (tree-hugger + tree-hugger-cli L1): **736/736 passed** (726 at
  Phase 1 close + 7 `symbols_header` + 3 CLI: `cache_upgrade` revision 4,
  `wiring` keyword/schema, `golden` determinism).
- `just lint`: clean (two `type_complexity` findings in the new tests fixed by
  replacing tuples with the `Inline` / `Documented` structs; one
  `needless_return` fixed with the `WrappedInField` keyword site).
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `just cross-check tree-hugger --os linux symbols_header`: pass (7/7).
- `just cross-check tree-hugger-cli --os linux wiring`: pass (24/24).
- `just cross-check tree-hugger --os windows symbols_header`: **not run to
  completion.** The rig was reachable this time, but its Cargo storage
  preflight refused: volume `B:` had 16.4 GiB free of 299.9 GiB against the
  required 50 GiB, and the automatic sweep freed 0 GiB. The guard was not
  overridden and nothing was deleted on the shared host. Phase 2 adds no path,
  process or OS-specific code, so the exposure is low; Windows CI on `main`
  runs these tests.
- Line endings: the repo's `.gitattributes` forces `eol=lf`, so the
  multi-line `\n` expectations hold on a Windows checkout.
- `just test-l2` not run: Phase 2 adds no L2 test and does not touch the
  `god-files` terminal path the L2 test exercises.

## Phase 3

Selective analysis in the library. Every Phase 3 task and validation
checkpoint 3 is complete. `EXTRACTOR_REVISION` is now `"6"`.

### What was built

- **`AnalysisPass::Diagnostics` and one planner** (`analysis/mod.rs`).
  `parse_pass` no longer calls `try_diagnostics`. `analyze_file(passes)`,
  `TreeFile::symbol_index_with_passes`, `TreeFile::symbol_index_v2` and the
  new `TreeFile::query` all go through `run_query`, which builds a `PassPlan`
  from the query's (prerequisite-closed) capabilities and runs each pass once
  in the order parse → bind → semantic → docs → diagnostics.
  `completed_passes` is the plan, so it lists only what ran (previously it
  listed passes in request order; now it is always canonical order).
  `symbol_index_v2()` = `SymbolQuery::complete_index()` and still includes
  diagnostics; `symbol_index_with_passes(&[Parse, Bind, Semantic, Docs])` no
  longer does (documented on the method).
- **`Capability`, `SymbolQuery`, `SymbolQueryResult`** (`file/query.rs`).
  Kinds are an `Option<BTreeSet<SymbolKindV2>>` (`None` = all; `SymbolKindV2`
  now derives `Ord` for a canonical order); capabilities are closed under
  prerequisites (References ⇒ Containment, Semantics ⇒ References, all ⇒
  Declarations). The result is `{ index: FileSymbolIndex (selected records
  only), completed_capabilities }`. `Containment` without `References` runs
  `link_containers` only (no reference query) and does not claim the Bind
  pass.
- **`SourceFile`** (`file/source_file.rs`): `read`, `read_with_language`,
  `hash`, `language`, `source`, `parse(self)`, `query(self, &q)`.
  `TreeFile::with_language` is now `SourceFile::read_with_language(..)?.parse()`
  (same error variants in the same order: read → detect → grammar → parse);
  `TreeFile::from_parts` is crate-private.
- **One definition capture per analysis.** Definition capture moved from
  `tree_file.rs` into `file/tree_file/definitions.rs`
  (`TreeFile::definitions(&MetadataSelection)`, `ExtractedSymbol`,
  `align_field_metadata`, `resolve_container_kind`). Records
  (`records_from`), exports (`exports_from`), the reference filter
  (`references_from`) and semantic lint (`lint_diagnostics_from`,
  `diagnostics_from`) take that one result. The public v1 methods each
  capture once too: `try_diagnostics()` and `try_lint_diagnostics()` went
  from 3 locals-query runs to 1, `symbol_index_v2()` from 6 to 1.
- **Lazy metadata after de-duplication.** Captures are grouped by
  `(start, end, name)` keeping the winner (highest priority, first of equal)
  and the later equal-priority captures; metadata is read from the winner and
  from a later capture only for a missing field. The old merge's quirk is
  kept on purpose: while no signature is known, a later capture's signature,
  visibility and `static` flags replace the current ones together. Container
  lookup now runs only for winners. No vendored query was edited.
- **Kind push-down.** `MetadataSelection::for_kinds` skips visibility,
  `static` flags, signature, type metadata, doc comment, header texts,
  keyword, type-alias target and implemented trait for unselected kinds;
  they keep name, kind, span, container, ID and export status, are bound, and
  are dropped after binding. A selected owner kind (type or class, not enum)
  widens the selection by Field/Property/Constant, so `align_field_metadata`
  keeps their access and `static` flags (the spec's `classes` rule,
  generalized to any type selection).
- **Indexed binding.** `NameIndex` (`HashMap<String, Vec<usize>>`, buckets in
  record order) replaces the full scans in `find_reference_target` (first of
  the bucket), and also in `find_container_symbol` and `find_export_target`
  (same filter over the bucket instead of every record; same choice, since
  bucket order is record order). `push_unique_ref` became `RefSet`: resolved
  by ID, unresolved by (name, qualified name).
- **Honest `is_recursive`**: `Some(Unknown)` for every function and method
  when the semantic pass runs, `None` otherwise.
- **Work counters** (`analysis/counters.rs`, re-exported as
  `tree_hugger::{work_counters, WorkCounters}`): process-wide atomics for
  lint-diagnostics runs (counted in `lint_diagnostics` and
  `lint_diagnostics_from`, which every fallible path reaches), parser
  constructions (`SourceFile::parse`) and definition captures
  (`TreeFile::definitions`).

### Decisions and departures

| Decision | Choice | Why |
|---|---|---|
| Where imports belong | Always computed with `Declarations`; no separate capability | Keeps one meaning per pass (`Parse` always carried imports), and the spec lists exactly six capabilities. Phase 6's perf sample decides whether an imports capability is worth adding |
| Where to count "diagnostics passes" | Every lint-diagnostics run, not only the pipeline pass | Lets a test see that `lint`, `try_diagnostics()` and `symbol_index_v2()` still run diagnostics |
| `push_unique_ref` dedupe | Strict spec rule (ID for resolved refs; name + qualified name for unresolved) | Old code collapsed every pair of ID-less refs (`None == None`), so an owner kept one import reference; it also merged resolved refs with different IDs but the same name |
| Name index scope | Also used for container and export targets | Same scans, same choice, no extra structure |
| Cached CLI entry | `symbol_cache_index` drops `Diagnostics` from `completed_passes`; `hydrate_diagnostics` adds it back with the fresh list | Without it the entry claimed a pass it does not hold (it never stores diagnostics), which Phase 4's coverage rule forbids. Cold and warm JSON are still identical |
| v1 adapter and `is_recursive` | Nothing to map | v1 `SymbolInfo` has no recursion field; the adapter drops `semantics` entirely. The schema doc now says so instead of "v1 maps it to `false`" |
| CLI | Still calls `symbol_index_v2()` | The CLI moves to queries in Phase 5; Phase 3 changed only the cache-coverage honesty above |

Drift corrected on the way: `lib/README.md` and the skill's
`api-reference.md` imported from `tree_hugger_lib`; the crate is
`tree_hugger`. Fixed in those two files (the skill's `diagnostics.md` still
says `tree_hugger_lib`, untouched).

### Equivalence evidence (before writing the new tests)

The Phase 2 `hug` binary (built 16:18, before the Phase 2 log closed) was
copied before rebuilding. Both binaries ran `hug --json symbols --no-cache`
over every symbol fixture (21 files plus the 16 `multi/` files, in a scratch
Git repo). Apart from `provenance.extractor_version`, the only differences
were the intended ones:

- every index's `completed_passes` gained `Diagnostics`
- 273 function/method records: `is_recursive` `No`→`Unknown` (272) and
  `Yes`→`Unknown` (1)

Every other record field, relation, import, export and diagnostic matched
byte for byte. The fixtures have no owner with two ID-less references; the
regression case below was confirmed against the Phase 2 binary: `run` in
`two_imports.py` reported references `[path]` and `is_recursive: No` before,
`[path, argv]` and `Unknown` after.

### Tests added

- `lib/tests/l1/symbols_query.rs` (declared in `tests/l1/main.rs`, L1):
  - `a_symbol_query_over_multi_runs_no_diagnostics_and_one_capture_per_file`
    (0 diagnostics runs, 16 parser constructions, 16 captures for 16 files;
    `[Parse]`, `[Declarations]`, only selected kinds)
  - `the_complete_index_and_lint_still_run_diagnostics_from_one_capture`
    (`symbol_index_v2`: 1 diagnostics run, 1 capture, `dead-code` +
    `unused-import`, equal to `try_diagnostics()`; `lint_diagnostics()` runs
    them; four passes without `Diagnostics` → empty list, no pass entry)
  - `passes_run_once_after_their_prerequisites` (six request rows incl.
    duplicates and out-of-order; capability closure)
  - `containment_alone_links_members_without_binding_references`
  - `source_file_parses_the_bytes_it_hashed` (file edited between read and
    parse; hash and records follow the read bytes)
  - `source_file_and_tree_file_report_the_same_errors` (missing → `Io
    NotFound`, unknown extension → `UnsupportedLanguage`, non-UTF-8 → `Io
    InvalidData`, for both constructors; forced language)
  - `narrow_queries_equal_the_filtered_records_of_a_full_query` (every
    fixture × 6 kind sets × 3 capability sets, whole serialized records;
    counts selected methods with an unselected parent and classes-set fields
    with access/`static`, so the check is not vacuous)
  - `distinct_unresolved_references_are_all_kept` (the regression input)
  - `is_recursive_is_unknown_when_semantics_run_and_absent_otherwise`
- `lib/src/analysis/tests.rs` (unit): `name_index_selects_the_same_targets_as_a_full_scan`
  keeps the three previous full scans as reference implementations and
  compares every reference name, symbol name, container and export over every
  fixture (1,261 / 427 / 45 lookups);
  `name_index_keeps_the_first_of_equal_candidates`;
  `ref_set_keeps_distinct_unresolved_references_and_dedupes_resolved_ones_by_id`.
  `pipeline_marks_completed_passes` now asserts the exact ordered list.
- `cli/tests/symbols/cache_upgrade.rs`:
  `entry_from_extractor_revision_five_is_not_served_after_the_repairs`
  (probe `module_function`'s `is_recursive`: stale `No`, repaired
  `Unknown`). `revision_key` now takes the revision's record schema; the four
  older cases pass `"3.0"`, so their keys are unchanged.
- `cli/tests/symbols/cache_robustness.rs`:
  `the_cache_entry_claims_only_the_passes_it_holds`.

Mutation checks (each reverted, verified with `cmp`): dropping the
Field/Property/Constant widening failed the narrow-vs-full test; collapsing
unresolved `RefSet` keys failed `distinct_unresolved_references_are_all_kept`;
always computing references and diagnostics failed the three counter tests;
removing the CLI's `retain` failed the cache-pass test.

Input Robustness Matrix: not applicable. Phase 3 adds no reader of a file
format. The only persisted change is which passes a cache entry lists, and
`cache_robustness` already walks the snapshot reader.

### Requirement → test mapping (Phase 3)

| Requirement | Test(s) |
|---|---|
| Symbol listing runs no diagnostics; one capture and one parse per file | `symbols_query::a_symbol_query_over_multi_runs_no_diagnostics_and_one_capture_per_file` |
| `symbol_index_v2()` and lint still run diagnostics | `symbols_query::the_complete_index_and_lint_still_run_diagnostics_from_one_capture`, all existing `lint` CLI tests |
| Planner dedupes and orders prerequisites; `completed_passes` = work done | `symbols_query::passes_run_once_after_their_prerequisites`, `analysis::tests::pipeline_marks_completed_passes` |
| Capabilities distinguish empty from not computed | `symbols_query::containment_alone_links_members_without_binding_references`, `…is_recursive_is_unknown_…` |
| `SourceFile` never rereads; error contract kept | `symbols_query::source_file_parses_the_bytes_it_hashed`, `…report_the_same_errors` |
| Lazy metadata output identical | whole-output comparison above; every existing `symbols_*` matrix (750 L1 green); `golden::plain_symbols_over_multi_match_the_golden` |
| Kind push-down keeps containers, resolution and field access | `symbols_query::narrow_queries_equal_the_filtered_records_of_a_full_query` |
| Name index picks the same targets | `analysis::tests::name_index_selects_the_same_targets_as_a_full_scan`, `…keeps_the_first_of_equal_candidates` |
| Distinct unresolved references kept | `symbols_query::distinct_unresolved_references_are_all_kept`, `analysis::tests::ref_set_…` |
| `is_recursive` `Unknown` / `None` | `symbols_query::is_recursive_is_unknown_when_semantics_run_and_absent_otherwise`, `cache_upgrade::entry_from_extractor_revision_five_…` |
| Cache entry coverage matches contents | `cache_robustness::the_cache_entry_claims_only_the_passes_it_holds` |

### Gates run

- `just test` (tree-hugger + tree-hugger-cli L1): **750/750 passed** (736 at
  Phase 2 close + 9 `symbols_query` + 3 analysis unit tests + 1
  `cache_upgrade` + 1 `cache_robustness`).
- `just lint`: clean (one `field_reassign_with_default` in `semantic_pass`
  fixed with a struct literal; the now-unused `revision_key` wrapper folded
  into one function).
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `just cross-check tree-hugger --os linux symbols_query`: pass (9/9).
- `just cross-check tree-hugger --os windows symbols_query`: **not run**. The
  Cargo storage preflight on build-win-native refused again: the automatic
  reclaim did not restore the required headroom. The guard was not overridden
  and nothing was deleted on the shared host. Phase 3 adds no path, process or
  console code. The one OS-sensitive assertion is the `io::ErrorKind` of a
  missing (`NotFound`) or non-UTF-8 (`InvalidData`) file, which std reports
  the same way on Windows. Windows CI on `main` runs these tests.
- `just test-l2` not run: Phase 3 adds no L2 test and does not touch the
  `god-files` terminal path.
- No pre-existing failures.

## Phase 4

Persistent cache with complete keys. Entries below are written as the work
lands.

### Wave 1 and the CLI wiring

- **Fingerprint** (`lib/src/cache/symbol_cache.rs`, `FingerprintInputs`):
  a computed `PassFingerprint` per (language, grammar): schema `4.0`,
  `<crate version>+rev.<EXTRACTOR_REVISION>`, the grammar's own identity
  (`Language::name`, declared version when present, ABI, node-kind, field and
  parse-state counts — read from the compiled grammar, nothing hand-written),
  the grammar ID, and the effective text of the `locals`, `imports` and
  `references` queries (inherited and overlay text resolved by the same
  `resolve_query_text` the analysis compiles). Query text is injectable
  (`FingerprintInputs::with_query_text`, `SymbolCache::with_query_text`).
  Diagnostics have their own fingerprint (lint + comments query text, a
  digest of the built-in rule metadata, `experimental_semantics`); it is
  stored inside the entry, not in the key, so rules and lint settings affect
  only the diagnostics pass (spec A7).
- **Key** = `canonical_digest(["symbols", path with `/` separators, source
  xxHash, forced language or "detected", records fingerprint digest])`. Kinds
  and capabilities are not inputs. `canonical_digest` (new, `cache/mod.rs`)
  length-prefixes each field before BLAKE3, and `PassFingerprint::digest` now
  uses it (its old `format!` joined schema minor and crate version with no
  separator). All digests go through `biscuit-hash`, already a normal
  dependency of `tree-hugger`; no dependency was added, so no dependency doc
  changed.
- **Entry** (Ruling 6): `schema_version`, `fingerprint`, `file` (path exactly
  as analyzed, since records repeat it), `language`, `source_hash`,
  `covered_kinds` (`"all"` or a non-empty strictly ascending kind list),
  `completed_capabilities` (non-empty, strictly ascending, closed under
  prerequisites), `diagnostics_fingerprint` (required, nullable), `records`,
  `imports`, `exports`, `diagnostics`. Read with `deny_unknown_fields` and no
  defaults; after decoding, schema/fingerprint/file/language/hash must match
  and contents must match coverage (diagnostics only with the capability,
  every record's kind covered).
- **Atomic write**: `PersistentCache::put` writes `<key>.<pid>.<n>.tmp` in
  the shard directory and renames it over the entry; on failure the temp
  file is removed. `PersistentCache::get_bytes` (new) separates "no entry"
  from an I/O error.
- **CLI**: `analyze_file` reads a `SourceFile` once, then
  `SymbolCache::query(source, &SymbolQuery::complete_index())`; `--no-cache`
  calls `SourceFile::query` (no reads, no writes). `InMemorySymbolCache`,
  `InProcessCache` and the stderr `cache:` stats line (which counted only
  in-process hits) are gone from the scan; the library types remain. The old
  `AnalysisOptions::fingerprint` went with them: deny/warn/allow/strict are
  applied after analysis, and `experimental_semantics` is in the diagnostics
  fingerprint. Before this change every warm run parsed each file (the
  `TreeFile` was built before the lookup) and recomputed diagnostics.

### Defect found: stored diagnostics never round-tripped

`DiagnosticMetadata` omits `is_enabled_by_default` and
`requires_experimental_semantics` when `false` but required both when
reading. Nothing read a stored diagnostic before, so it was latent; once
entries stored diagnostics, every file with a diagnostic from a
default-off rule produced an unreadable entry and was silently recomputed.
`golden::json_symbols_over_multi_are_identical_across_runs` stayed green
because a recompute gives the same output. Fixed with `#[serde(default)]` on
both fields (the serializer itself defines absent as `false`). The
per-language cache test asserts the warm lookup is a `Hit`, not only equal
output, so this class of failure is caught.

### Wave 2: lookup, merge and write-back

- `SymbolCache::query(SourceFile, &SymbolQuery) -> CachedQueryResult
  { result, lookup, write_error }`. `lookup` is `Hit`,
  `WidenedCapabilities { parsed }`, `WidenedKinds` or `Miss(CacheMiss)`;
  `CacheMiss` is `Absent`, `Unreadable`, `Malformed(serde message)`,
  `Mismatched(field)` or `Inconsistent(what)`.
- **Narrowing** (`lib/src/analysis/coverage.rs`, `project`): every result,
  hit or computed, is reduced to exactly the request: unselected kinds
  dropped; containment links, reference edges, export targets, `semantics`,
  `docs.parsed` and diagnostics cleared when not requested; each record's
  `provenance.parse_pass` recomputed (Docs > Semantic for functions/methods >
  Bind > Parse). This is what makes a wide entry serve a narrow command with
  `--no-cache`-identical output. It relies on two facts checked over every
  fixture: `visibility.is_exported` is the same before and after binding,
  and `docs.comments` is never changed by the docs pass when a raw doc
  exists.
- **Record-only widening** (`add_record_passes`): docs, semantics (when
  references are stored) and containment (when every kind is stored) run
  over the stored records with no parse. References and diagnostics parse
  and re-run the query for the union, definitions capture included, because
  binding and semantic lint consume the live definitions (tree nodes cannot
  be cached).
- A query selecting no kinds is computed but not stored (an entry must name
  at least one kind).
- `SourceFile` gained `forced_language()` (a key input) and
  `experimental_semantics(bool)` (carried into the parsed `TreeFile`, and
  into the diagnostics fingerprint).

### Decisions and departures

| Decision | Choice | Why |
|---|---|---|
| "Run only the missing passes" | Literal for docs, semantics and all-kind containment; references and diagnostics re-parse and recapture | Binding needs every kind as a candidate owner and the reference query's live tree; semantic lint needs the definitions' syntax nodes. Neither is cacheable. The lookup reports which happened (`parsed`) |
| Diagnostics in the entry | Stored, with a diagnostics fingerprint inside the entry | Spec A7: rules and settings affect only the passes that use them. Keying on them would split one file's entry per lint setting |
| Empty `records` | Served when coverage is valid (a file with no records of the covered kinds) | The reader cannot tell it from an edited entry without a checksum; entries are written atomically, and truncation is malformed JSON. Recorded in the docs table |
| Unordered or repeated coverage lists | Miss | The writer always emits strictly ascending lists; a reader that normalized would accept shapes no writer produces |
| `PassFingerprint::digest` encoding | Changed to `canonical_digest` | Spec: unambiguous canonical encoding. No stored data uses the old digest |
| Entry `file` | Exact path string, validated; key uses `/` separators | The records repeat the path, so an entry for another spelling would print the wrong path |
| CLI request | Still the complete index for every command | Phase 5 switches commands to per-command queries; with diagnostics stored, a warm complete-index run already parses nothing |
| CLI work-counter visibility | Not added | The counters are proven in-process (library tests). Phase 5's `--verbose` is the place to report hits/parses |
| Stale-revision test for revision 6 | Not added | The old key formula is retired; the computed key cannot be hard-coded. The revision is pinned as a fingerprint input in `symbols_cache` |
| Windows rename races | Concurrency tests accept a failed write, never a torn read | `MoveFileExW` can refuse a rename racing another onto the same file; the cache reports it as `write_error` by contract |

`docs/cache-design.md` was a pre-implementation draft (SQLite, L1–L4 layers,
`--cache-stats`) describing behavior that never existed; it was rewritten as
the current record: usage, key, fingerprint tables, entry format, a lookup
flow diagram, atomic writes, both robustness tables, and what is not cached.

### Tests added

- `lib/tests/l1/symbols_cache.rs` (L1, declared in `tests/l1/main.rs`):
  - `every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run` — the
    "Cache correctness" per-language row (21 primary fixtures + 16 `multi/`):
    cold `Miss(Absent)`, warm `Hit` with all work counters zero, both equal
    to uncached
  - `projected_results_equal_uncached_queries` /
    `projected_results_over_multi_equal_uncached_queries` — a complete entry
    narrowed to each capability alone, all capabilities, and Method-only at
    both ends equals the uncached query; non-vacuity counts of stripped
    references and parsed docs
  - `widening_capabilities_runs_only_the_missing_passes` — six rows (docs,
    semantics, all-kind containment: no parse; narrow containment,
    references, diagnostics: one parse, one capture, diagnostics runs as
    expected); one entry recording the union; then first, second and union
    queries are hits
  - `widening_kinds_reextracts_once_for_the_union` — one parse, one capture;
    `covered_kinds` is the sorted union; three later queries hit
  - `each_key_input_and_stored_identity_decides_a_miss` — edited locals
    text, forced language, source edit (new key → `Absent`); extractor
    revision in the fingerprint; edited lint text and experimental semantics
    keep the key, serve listings, recompute diagnostics; stored schema,
    fingerprint, source hash, file and language each `Mismatched`
  - `a_corrupt_entry_is_a_miss_and_is_replaced` (truncated, empty, not JSON)
  - `absent_and_null_entry_fields_report_different_reasons` (seven fields)
  - `covered_kind_lists_are_served_only_in_canonical_form` (list shapes
    from a narrow entry, plus a record of an uncovered kind)
  - `concurrent_writers_leave_a_readable_entry_whose_coverage_matches`
    (six threads, five rounds, independent handles)
  - `a_disabled_store_neither_reads_nor_writes`
- `lib/tests/l1/cache_tests.rs`: atomic replace with no temp file left;
  failed put removes its temp file; concurrent puts never expose a partial
  entry; `canonical_digest` separation.
- `cli/tests/symbols/cache_robustness.rs`: matrix extended with seven entry
  columns (`covered_kinds`, `completed_capabilities`, `schema_version`,
  `fingerprint`, `source_hash`, `diagnostics_fingerprint`, `records`) ×
  nine shapes, from a real `hug` entry; `the_cache_entry_claims_only_the_
  passes_it_holds` replaced by `the_cache_entry_records_the_coverage_it_holds`
  (entries now store diagnostics with their capability and fingerprint).
- `cli/tests/symbols/cache_lookup.rs` (new, declared in `main.rs`):
  `concurrent_processes_leave_one_readable_entry` (six concurrent `hug`
  processes, mixed commands and lint settings) and
  `cache_hits_and_cache_failures_print_nothing` (cold, warm and
  unwritable-cache runs: empty stderr, uncached output; `--no-cache` writes
  nothing).
- `cache_upgrade.rs`: adapted to the entry layout (`file`, `records`); the
  stale payload keeps the entry's current `schema_version`, since the reader
  now checks it.

Mutation checks (each reverted): keeping references, members, semantics,
parsed docs or export IDs in the projection, or stamping `Bind` instead of
`Semantic`, failed the projection tests; allowing narrow containment from
records failed the widening test; ignoring the diagnostics fingerprint failed
the key test; accepting empty, unordered or repeated kind lists, or records
of uncovered kinds, failed the kind-list test; a plain `fs::write` instead of
temp + rename failed the concurrent put test (3/3 runs); flipping two entry
matrix cells failed exactly those cells.

### Input Robustness Matrix (cache entry reader)

Outcomes are asserted through the public CLI result (served vs. equal to
`--no-cache`) by `cache_robustness::persisted_symbol_fields_are_served_or_recomputed_per_shape`,
with a control row (the unedited entry is served). The library adds the
reason text (absent vs null) and the list form of `covered_kinds`.

| Shape | `covered_kinds` | `completed_capabilities` | `schema_version` | `fingerprint` | `source_hash` | `diagnostics_fingerprint` | `records` |
|---|---|---|---|---|---|---|---|
| control | served | served | served | served | served | served | served |
| absent | miss | miss | miss | miss | miss | miss | miss |
| null | miss | miss | miss | miss | miss | miss (inconsistent) | miss |
| wrong type | miss | miss | miss | miss | miss | miss | miss |
| one wrong element | miss | miss | miss | miss | miss | miss | miss |
| every element wrong | miss | miss | miss | miss | miss | miss | miss |
| empty | miss (`[]`) | miss | miss (`{}`) | miss | miss | diagnostics recomputed (miss) | served, no records |
| duplicate key | miss | miss | miss | miss | miss | miss | miss |
| trailing content | miss | miss | miss | miss | miss | miss | miss |

Smell grep over `cache/symbol_cache.rs`, `analysis/coverage.rs`,
`file/source_file.rs`: no `#[serde(default)]`, `unwrap_or_default()` or
`.ok()` on a load-bearing parse; the one `filter_map` reads the rule
registry's own IDs. `diagnostics_fingerprint` is `Option<String>` read
through a required-but-nullable deserializer, so absent and null differ. The
one `#[serde(default)]` added (`DiagnosticMetadata`'s two flags) matches the
serializer's own encoding of `false` as absent.

### Requirement → test mapping (Phase 4)

| Requirement | Test(s) |
|---|---|
| Complete warm hit: no parse, no analysis | `symbols_cache::every_fixture_is_a_complete_warm_hit_…` (counters all zero) |
| Warm == `--no-cache` per language | same; `cache_lookup::cache_hits_and_cache_failures_print_nothing`; `golden::json_symbols_over_multi_are_identical_across_runs` |
| Kinds/passes not in the key; narrow↔broad equal uncached | `symbols_cache::projected_results_*`, `widening_*` |
| Capability widening: one entry, only missing passes | `widening_capabilities_runs_only_the_missing_passes` |
| Kind widening: re-extracted once; third narrow run hits | `widening_kinds_reextracts_once_for_the_union` |
| Query text, extractor revision, language override, schema → miss | `each_key_input_and_stored_identity_decides_a_miss`, `cache_upgrade::*` |
| Rules affect only diagnostics | `each_key_input_…` (lint text, experimental semantics) |
| Corrupt/incompatible = miss, never empty | `a_corrupt_entry_…`, `absent_and_null_…`, `cache_robustness::*` |
| Atomic write, last rename wins, readable survivor | `cache_tests::concurrent_persistent_puts_…`, `symbols_cache::concurrent_writers_…`, `cache_lookup::concurrent_processes_…` |
| `--no-cache` bypasses reads and writes | `a_disabled_store_neither_reads_nor_writes`, `cache_lookup::cache_hits_…` |
| Quiet cache failures | `cache_lookup::cache_hits_and_cache_failures_print_nothing` |

### Gates run

- `just test` (tree-hugger + tree-hugger-cli L1): **767/767 passed** (750 at
  Phase 3 close + 11 `symbols_cache` + 4 `cache_tests` + 2 `cache_lookup`;
  one `cache_robustness` test replaced). The projection test was split in
  two after hitting nextest's 30 s timeout under full-suite load.
- `just lint`: clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `just cross-check tree-hugger --os linux` (629/629) and
  `just cross-check tree-hugger-cli --os linux`: pass.
- `just cross-check tree-hugger --os windows`: **not run**. The storage
  preflight on build-win-native refused again (15.9 GiB free, 50 GiB
  required, automatic reclaim insufficient); not overridden, nothing deleted.
  Windows-relevant behavior: `std::fs::rename` replaces an existing entry on
  Windows (`MoveFileExW` with replace) and std opens files with
  `FILE_SHARE_DELETE`, so readers do not block it; a rename racing another
  rename may fail, which the cache reports as `write_error` and the tests
  accept. Windows CI on `main` runs these tests.
- `just test-l2` not run: no L2 test added; the `god-files` terminal path is
  untouched.
- No pre-existing failures.

## Phase 5

CLI on records: per-command queries, the versioned JSON report, `--sort`,
`--verbose`, the parallel scan, prelude reuse and one renderer. Entries below
are written as the work lands.

### Waves 1–3 (code)

- **Library:** `SymbolQueryResult` gained `selected_kinds` and
  `narrowed(&SymbolQuery) -> Option<_>`, which projects a wide result to a
  narrower query through the same `coverage::project` the cache uses and
  refuses (`None`) a query the result does not cover. Prelude reuse needs it
  under `--no-cache`, where there is no cache entry to narrow.
- **New CLI modules** (`main.rs` only wires): `order.rs` (`SortOrder`, the
  D7 kind order, `portable_path_cmp`, tie-breakers), `report.rs` (streamed
  envelope, `ReportError`, path relativization), `render.rs` (the one
  renderer), `classes.rs` (container summaries), `scan.rs` (`Analyzer`,
  `in_file_order`, `CacheStats`), `declarations.rs` (the four declaration
  commands end to end). `prelude.rs` was rewritten on records.
- **Removed from `main.rs`:** `render_symbols`, `render_symbol_line`,
  `symbol_presentation`, `binding_token_for_symbol`, `SOURCE_LINE_CACHE`,
  `source_line_for_symbol`, the v1 `ClassSummary` and every
  `render_*`/`format_*` helper they used, `analyze_file`, `SymbolFilter`,
  `SymbolRenderOptions`, `--sort-by-kind`, `--sort-by-module`;
  `scanner::apply_symbol_filters` (no caller left).

### Behavior as built

- **Queries** (`Declarations::query`): `functions` = Function/Method/
  Constructor, `types` = Type/Class/Interface/Enum/Trait/TypeAlias,
  `symbols` = every kind, all with Declarations only; `classes` = Class/Type/
  Interface/Trait + Method/Constructor with Containment; `--json-full` =
  `complete_index()`; `lint` = Declarations + Diagnostics; `imports` =
  Declarations (import records come with it).
- **Parallel scan** (`scan::in_file_order`): rayon `par_iter` over chunks of
  `4 × current_num_threads()` (Ruling 1), each chunk delivered in file order
  before the next starts. `lint` and `imports` use the same driver. Cache
  stats come back with each result (no thread-locals); work counts are the
  library's process-wide atomics, read once at the end.
- **File order:** files are sorted by their scan-root-relative portable path,
  compared component by component (`a/b.rs` before `a-c.rs`), for JSON and
  terminal alike.
- **JSON** (`--json`): compact, written with `serde_json::to_writer` to a
  locked `BufWriter`, envelope head first, one entry per file with a match,
  `errors` last. `root_dir` is the resolved scan root (`pkg_root`, the
  directory files are collected from); terminal paths stay relative to the
  Git root as before.
- **Errors:** a file that fails to read or analyze is written to stderr as
  `error: <message>` in file order, becomes one `errors` entry, and the run
  exits `1`. Before, the first failing file aborted the whole run with a
  `Debug`-printed error; `lint` and `imports` now also continue past a failed
  file (stderr, exit `1`), their JSON shape unchanged.
- **Output failures:** a broken pipe ends the run with status `0` and no
  message; any other stdout write error prints `error: cannot write output`
  and exits `1`.
- **`--verbose`:** `cache: <path>: <reason>` for every non-`Absent` miss,
  `cache: <path>: write failed: <error>` for a failed write, then one line
  `cache: H hits, P partial hits, M misses[, R reused in this run]; parsed N
  files, C definition captures, D diagnostics runs` (`cache: disabled; …`
  under `--no-cache`). Adding the work counts to the stats line is how CLI
  tests prove "zero diagnostics", "no duplicate parse" and "a warm hit parses
  nothing" end to end.
- **`--json-full` rejection** (Ruling 2, as recorded in Phase 1): symbol-name
  positional filters (classified by the scan's own file/symbol classifier),
  `--exclude-symbols`, `--exported`, `--prelude`, and for `classes` `--name`,
  `--static-only`, `--instance-only`; also `--json-full` on `lint`/`imports`.
  All exit `2` through clap's `ArgumentConflict`. `--sort` and file selectors
  are allowed.
- **Classes** (Ruling 3): `members` holds the flag-filtered (displayed)
  methods and constructors, in source order, keyed by container ID.
  `--sort` orders containers within each file; members keep source order.
- **Prelude** (Ruling 4): synthetic records use the existing
  `visibility.is_synthetic` (plus `is_reexported`, `is_exported`); no schema
  field was added. They take the alias name, the target's kind, keyword, doc
  comment and declaration location (`use` location when unresolved, kind
  `Unknown`, or a text-inferred kind), and an ID from
  `stable_symbol_key(language, prelude file, kind, "prelude-export::<name>",
  ordinal)`. Target lookup uses the every-kind index (exported records first,
  all records when the module exports none), as before.
- **Renderer:** `Prose` lines inside `UnorderedList` sections (bullet `- `,
  doc comments as hanging-indent continuation lines), indented 2 (listing) or
  6 (class members). biscuit-terminal's list re-wraps items at the terminal
  width even when the item's `Prose` says `WordWrap::None` (found with a
  probe); the renderer pins `Terminal::fixed_width` to 1,000,000 so today's
  unwrapped layout holds. Plain output is rendered on a no-color terminal and
  stripped of escapes; OSC 8 links are emitted only when the terminal
  reports link support.

### Decisions and departures

| Decision | Choice | Why |
|---|---|---|
| Label source | `identity.keyword` for Type, Variable, Field, Constant (the kinds the old renderer guessed from source text); the kind label for every other kind | Keeps the layout (`public method`, `function`) while deleting the source-line guess. Using the keyword everywhere would print `fn`/`def`/`func` for every function |
| Relation targets | Not re-sorted | They are already built in source / first-seen order from vectors; hash maps are lookups only. Byte-identity tests (threads, cold/warm, two runs) pin determinism; re-sorting would lose the first-seen meaning |
| `docs.comments` in default `--json` | Dropped | The `Capability` docs assign attached comments to the docs pass, which no listing requests, and in a declarations-only record the one comment repeats `raw_doc` twice (~200 KB of the functions report). `--json-full` keeps them |
| `identity.stable_key` path | Rewritten scan-root-relative in both reports; ID unchanged | Spec D1: every machine-readable path is portable and scan-root-relative. The key no longer re-derives the ID (which hashes the analyzed absolute path) |
| Record IDs | Unchanged, still derived from the absolute path | The spec forbids altering IDs. Consequence: IDs differ between checkouts at different paths; the JSON golden numbers them (`<id-N>`). Recorded as a follow-up below |
| JSON formatting | Compact (one line) | The spec names `serde_json::to_writer`; pretty printing would cost ~40% more bytes against the 3 MB target. The golden is a canonical pretty rendering of the parsed report |
| Parallel `lint`/`imports` | Same driver as the declaration commands | Plan: "par_iter per file"; their report shape is untouched |
| `--sort` on `classes` | Orders containers per file; members keep source order | The spec defines `--sort` over declarations; class sections are a presentation of one container |
| Group-by-file group order | Portable path order (component-wise), not `BTreeMap` string order | One path order everywhere; equal for every name without `-`/`.` before a separator |
| Library change | `SymbolQueryResult.selected_kinds` + `narrowed` | Needed for prelude reuse under `--no-cache`; checked (refuses an uncovered query) rather than trusted |

### Output differences (deliberate; goldens regenerated)

`symbols.plain` was regenerated once. The set of lines is unchanged except
for the labels, and the order is now file, then source position (96 lines
moved). Label changes, all from `identity.keyword`:

| Before | After |
|---|---|
| `internal type Meter` (Swift) | `internal struct Meter` |
| `module-private variable M`, `default_size` (Lua) | `module-private local …` |
| `variable $queue`, `$size`, `$default_size` (Perl) | `my …` |
| `type point` ×3 (C) | `struct point` |
| `= MAX_SIZE` (Python; the Phase 1 label defect) | `variable MAX_SIZE` |

The Go `Grow(amount int) (amount int)` return defect noted in Phase 1 is a
record signature issue and is unchanged. `symbols.json` is new.

### JSON size

| Input | `hug --json functions` |
|---|---|
| Phase 1 baseline tree (commit `60e3b841`, exported read-only to `/tmp`; 94 files with functions, 1,580 functions) | **2,722,600 B (2.72 MB)**, was 39.2 MB |
| Today's `tree-hugger/` (113 files, 1,784 functions) | 3,104,400 B (3.10 MB) |

The target is met on the baseline input it was set on; today's tree is 13%
larger in functions (≈1.74 KB per function). The largest remaining parts of
a record are kind data (parameters), spans (declaration, name, body),
`type_info` views that repeat the return type, and the always-serialized
visibility booleans; trimming those would change the record schema or drop
data the spec keeps, so it is left for a deliberate decision.

### Tests added or rewritten

New (all L1, in the `tree-hugger-cli` `symbols` binary unless noted; every
file is declared in `tests/symbols/main.rs`):

- `tree-hugger::l1 symbols_query::a_wide_result_narrows_to_exactly_the_uncached_narrow_query`
  (every fixture × functions/types/classes × two capability sets; refusals)
- `golden::json_symbols_over_multi_match_the_golden`,
  `json_symbols_over_multi_are_identical_with_one_thread`,
  `json_full_over_multi_is_identical_with_one_thread`,
  `json_symbols_over_multi_are_identical_cold_and_warm`
- `report.rs`: envelope and record round trip; kind sets; empty selection;
  failed file (`--json`, `--json-full`, terminal); failure order with 1 and
  8 workers; `--json-full` indexes; `--json-full` rejections (10 cases);
  broken pipe
- `order.rs`: each `--sort` against a test-side oracle; the `Widget` tie;
  JSON file order with per-file sort (`--json` and `--json-full`); sort
  within file and directory groups; argument errors (8 cases)
- `classes.rs`: interface/trait containers and `members`; static/instance
  sections and both flags; container fields equal to the complete index
  without Field/Property/Constant selected
- `prelude.rs`: synthetic alias at its declaration (own ID), unresolved
  re-export, `PRELUDE` names, target outside the scan, one parse per file
  (uncached and cached)
- `cache_lookup.rs`: `verbose_reports_cache_use_and_cache_failures`,
  `symbol_commands_run_no_diagnostics`, `lint_and_json_full_run_diagnostics`,
  `complete_warm_hits_parse_nothing`
- `order.rs` unit test in the binary: `portable_paths_order_directory_by_directory`

Rewritten for the new shape (pre-authorized): seven `cli.rs` JSON tests
(parsed-JSON assertions on records instead of v1 fields and pretty-print
substrings); `wiring.rs` JSON tests (the "legacy symbols match the indexes"
test became `json_records_match_the_complete_index`, `--json` vs
`--json-full` by ID); `cache_robustness.rs` and `cache_upgrade.rs` probe
through `--json-full` and read `indexes` (same cells, same outcomes);
`concurrent_processes_leave_one_readable_entry` now asserts the survivor's
records are of kinds it covers (narrow commands write narrow entries, so
"all" is no longer guaranteed).

Mutation checks (each reverted): scanning without prelude reuse failed
`prelude_and_scan_parse_each_file_once`; exiting `1` on a broken pipe failed
`a_closed_stdout_ends_the_run_quietly`; reversing file order failed the
order, failure-order, failed-file and JSON golden tests.

### Requirement → test mapping (Phase 5)

| Requirement | Test(s) |
|---|---|
| Per-command kind sets, no diagnostics in listings | `report::functions_and_types_select_exactly_their_kind_sets`, `cache_lookup::symbol_commands_run_no_diagnostics` |
| `lint`/`--json-full` still run diagnostics | `cache_lookup::lint_and_json_full_run_diagnostics`, `report::json_full_reports_complete_indexes` |
| Complete warm hit parses nothing | `cache_lookup::complete_warm_hits_parse_nothing` |
| Envelope, `errors` last, round trip, no `declaration_text` | `report::the_json_envelope_…`, `wiring::json_records_carry_keyword_…` |
| Empty selection `files: []` | `report::an_empty_selection_is_an_empty_files_array` |
| Failed file: listed good files, one error, stderr, exit 1 | `report::a_failed_file_is_reported_…` |
| Deterministic parallel error order | `report::failures_are_reported_in_file_order_with_any_thread_count` |
| `--json-full` indexes and rejections | `report::json_full_reports_complete_indexes`, `report::json_full_rejects_…` |
| Classes `members` and sections | `classes::*`, `cli.rs` classes tests |
| Parallel == single thread; cold == warm | `golden::json_*_identical_with_one_thread`, `golden::json_symbols_over_multi_are_identical_cold_and_warm` |
| Goldens | `golden::plain_symbols_over_multi_match_the_golden`, `golden::json_symbols_over_multi_match_the_golden` |
| Orders, tie-breakers, groups, JSON file order | `order::*` |
| Removed/misused `--sort` flags are argument errors | `order::removed_and_misplaced_sort_flags_are_argument_errors` |
| Broken pipe exits quietly | `report::a_closed_stdout_ends_the_run_quietly` |
| Quiet stderr / `--verbose` | `cache_lookup::cache_hits_and_cache_failures_print_nothing`, `cache_lookup::verbose_reports_cache_use_and_cache_failures` |
| Prelude aliases, unresolved, env names, outside targets, no duplicate parse | `prelude::*` |
| Wide → narrow result equals uncached | `symbols_query::a_wide_result_narrows_…` (library) |
| Portable paths (os skill) | envelope test (`/`-separated, relative), failure-order test (native separators normalized), goldens |

The Input Robustness Matrix does not apply: Phase 5 adds no reader of a
persisted or configuration format (the cache reader is unchanged; its matrix
test was ported to the new probe with identical cells).

A stdout write error other than a broken pipe (for example `/dev/full`) is
handled (`error: cannot write output`, exit 1) but not tested: no portable
way to produce it.

### Gates run

- `just test` (tree-hugger + tree-hugger-cli L1): **797/797 passed**.
- `just lint`: clean.
- `just test-l2`: 3/3 passed (the Kitty case returns at once, as when Kitty
  is unavailable).
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `just cross-check tree-hugger-cli --os linux`: 167/167 passed.
- `just cross-check tree-hugger-cli --os windows`: **not run**: storage
  preflight on the Windows host refused again (15.6 GiB free, 50 GiB
  required; automatic sweep freed nothing); not overridden, nothing deleted.
  Compile evidence only: `cargo check -p tree-hugger-cli --tests --target
  x86_64-pc-windows-gnu` passes. Windows-sensitive points: JSON paths go
  through `to_portable_string`, file ordering compares portable strings, the
  failure-order test normalizes `\` in stderr. Windows CI on `main` covers
  behavior.
- WSL not run (follows Linux code paths; nightly CI covers it).
- No pre-existing failures.

### Incidents

- Two test-only edits were silently skipped because `cd` (a zoxide alias in
  this shell) failed inside a `&&` chain; caught by re-checking, re-applied
  with absolute paths.
- First full-suite run: three new tests timed out at 30 s. Each `hug` started
  one rayon worker per core inside 16 concurrent test processes; `Scratch`
  now pins `RAYON_NUM_THREADS=2`, and the two longest tests were split.

### Follow-ups for Phase 6 and later

- Record IDs hash the analyzed absolute path, so they differ between
  checkouts; a future `naming context` or ID scheme should decide whether
  IDs become root-independent.
- Phase 6 writes `docs/symbol-reports.md`; this phase updated the READMEs,
  cache design, schema design and dependency docs only where they had
  become wrong.

## Phase 6

Documentation, performance sample, acceptance. Implemented by claude/opus on
2026-10-04. Entries are written as the work lands.

### Performance sample

- **Tree:** `b76a33baa` (`feat/symbols`, clean; Phase 5 closed). **Build:**
  `cargo build --release -p tree-hugger-cli` (workspace `release` profile),
  copied to `/tmp/th-baseline/hug-post`. **Baseline binary:** the Phase 1
  release build of `60e3b841` kept at `/tmp/th-baseline/hug-pre`.
- **Host:** the Phase 1 host (Apple M4 Max, 16 cores, macOS). Load average
  24 → 14 during the timing run (lower than Phase 1's 38–45).
- **Method:** as Phase 1 recommended, both binaries timed back-to-back in one
  hyperfine invocation (`-N --warmup 1 --runs 5`) in `tree-hugger/`, so both
  see the same load and the same inputs (today's tree: 131 files). Each binary
  had its own `TMPDIR` cache; the warm runs follow hyperfine's warmup run.
  `-i` was passed, so every command was re-run once by hand: all exit `0`,
  `pre` and `post` print the same line counts (`functions` 1,853, `imports`
  773, `lint` 8), and the `lint` and `imports` outputs are byte-identical
  between the binaries.

| Command (in `tree-hugger/`) | Pre (`60e3b841`) | Post | Ratio | Target | Met |
|---|---|---|---|---|---|
| `hug functions --no-cache` | 1.859 s ± 0.015 | 0.123 s ± 0.004 | 6.6% | ≤ 40% | yes |
| `hug functions` (warm) | 1.078 s ± 0.004 | 0.044 s ± 0.001 | 4.1% | ≤ 15% | yes |
| `hug lint --no-cache` | 1.883 s ± 0.050 | 0.241 s ± 0.050 | 12.8% | no slower | yes |
| `hug imports --no-cache` | 1.852 s ± 0.020 | 0.100 s ± 0.004 | 5.4% | (reference) | — |

The post numbers include the parallel scan. To separate parallelism from the
removed work, the post binary was also timed with `RAYON_NUM_THREADS=1`:
`functions --no-cache` 0.435 s (23% of pre), `lint --no-cache` 0.782 s (42%),
warm `functions` 0.055 s (5.1%). Every target holds on one thread too. User
CPU tells the same story: `functions --no-cache` uses 0.56 s of CPU across
workers against 1.82 s before.

**Work evidence** (`hug --verbose`, post binary, `tree-hugger/`):

| Run | Stats line |
|---|---|
| `functions --no-cache` | `cache: disabled; parsed 131 files, 131 definition captures, 0 diagnostics runs` |
| `lint --no-cache` | `cache: disabled; parsed 131 files, 131 definition captures, 131 diagnostics runs` |
| `functions` (warm) | `cache: 131 hits, 0 partial hits, 0 misses; parsed 0 files, 0 definition captures, 0 diagnostics runs` |

**Peak RSS** (repository root, `/usr/bin/time -l hug symbols --no-cache >
/dev/null`, post binary, one run): **1,950,662,656 B (1.95 GB)**, elapsed
8.51 s (23.55 s user), against the Phase 1 baseline of 3.48 GB (184–275 s).
Target "no higher than baseline": met, 56% of baseline. The baseline was not
re-measured (the plan compares against the recorded figure).

**JSON size** (`hug --json functions --no-cache`):

| Input | Pre | Post | Target |
|---|---|---|---|
| Phase 1 baseline tree (`git archive 60e3b841 tree-hugger` into `/tmp/th-perf/base`, `git init` so the scan has a root) | 38,688,247 B | **2,722,595 B (2.72 MB)** | < 3 MB: met |
| Today's `tree-hugger/` (131 files) | 46,774,604 B | 3,197,216 B (3.20 MB) | over by 0.20 MB |

The target was set on the baseline input and is met there. Today's tree has
more functions (the phases added code and tests); the remaining cost per
record is the one recorded in Phase 5 (kind data, three spans, `type_info`
views repeating the return type, always-serialized visibility booleans).
Trimming it is a schema decision, not a Phase 6 task. `hug --json symbols` on
today's tree is 15,564,617 B (was 50.0 MB on the smaller baseline tree);
`hug symbols` prints 13,114 lines.

### Documentation (Wave 1)

- **`tree-hugger/docs/symbol-reports.md` (new).** Commands and kind sets
  (with what `classes` displays), narrowing filters and `--prelude`, a
  Mermaid query → cache → passes → order → output diagram, terminal labels,
  `--sort` orders, tie-breakers, path order, grouping, JSON file order, the
  JSON envelope with a real (abridged) sample, field table, what default
  reports omit, `--json-full` and its rejections, failed files and the exit
  status table, `--verbose`. Samples were produced with the release binary
  over a scratch repo, not written by hand. No feature/fix is linked or named.
  `cli/src/order.rs` already pointed at this page.
- **READMEs.** `tree-hugger/README.md`: link to the new page; a "Release
  Notes: record-native symbol reports" section (speed figures from the
  sample, new default order with a before/after listing, new JSON shape with
  before/after envelope, `--json-full`, keyword labels, failed files, quiet
  stderr); its "Running Tests" section still said `cargo test -p
  tree-hugger-lib` (wrong package name, not nextest) and now lists `just
  test` / `just test-l2` / `just lint`. `cli/README.md`: link to the new
  page (it already documented `--sort`, `--verbose`, `--json-full`, the
  envelope and the removed flags).
- **CLI help and completions:** already current (Phase 5). `hug functions
  --help` documents `--sort` and its tie-breakers, `hug --help` documents
  `--verbose`, `--sort-by-kind` is rejected. Completions are clap's dynamic
  completions, generated from the same definitions, so they need no edit.
- **Skill.** `SKILL.md`: the new page in "Detailed Documentation" and in the
  CLI declaration-reports bullet. `api-reference.md`: a "Caches" section
  (`SymbolCache` is the only CLI cache; `InProcessCache`,
  `InMemorySymbolCache`, `SymbolSnapshot`, `AnalyzerFingerprint` are
  library-only), `is_recursive` (`Some(Unknown)` / `None`), and the "JSON
  Output Types" section no longer claims `PackageSummary` is every CLI
  `--json` (only `imports`/`lint`). `SourceFile`, `SymbolQuery`, `keyword`,
  `signature_text` and schema 4.0 were already documented in earlier phases.
- **Dependencies:** no change needed. `docs/dependencies.md` already records
  `rayon` in `tree-hugger-cli` (Phase 5) and `biscuit-hash` as a CLI
  dev-dependency; `biscuit-hash` was already a normal `tree-hugger`
  dependency before this feature (checked against `60e3b841`'s
  `lib/Cargo.toml`). The area has no `docs/dependencies.md`.

### Drift pass

A read-only subagent compared `cache-design.md`, `symbol-schema-design.md`
and `symbol-kinds-and-visibility.md` with the code; every finding was
re-checked against the cited source before editing. In each case the code
was taken as correct.

| Doc | Drift | Resolution |
|---|---|---|
| cache-design | "`lint` and `--json-full` add diagnostics" | `--json-full` asks for every capability |
| cache-design | `--verbose` section omitted `cache: disabled;`, `reused in this run` and that an absent entry is not listed | Added |
| cache-design | Key "a digest of four things" (code also hashes a fixed `symbols` tag) | Says so |
| symbol-schema-design | Design principle "parse -> bind -> infer -> doc parse" | The real pass graph |
| symbol-schema-design | `Parameter(ParameterData)` | `ParameterSymbolData` with its fields |
| symbol-schema-design | `ParsedDocs` listed six fields that do not exist; `DocsFacet.parsed` not optional | Real fields; the others marked **planned** (today they land in `tags`) |
| symbol-schema-design | `Attribute { name, arguments, raw_text, span }` | `{ name, value }` |
| symbol-schema-design | `FileSymbolIndex` missing `schema_version`, `completed_passes` | Added |
| symbol-schema-design | Import/export records "should" use `SymbolRef` | They do |
| symbol-schema-design | Stable key ends in the declaration start byte; "BLAKE3 recommended" | Ends in the sibling ordinal (same kind, container, name), kind's variant name; BLAKE3 is what is used; example added |
| symbol-schema-design | 4.0 example showed `rev.5` | `rev.6` |
| symbol-kinds-and-visibility | Diagram drew `SymbolInfo → hug output` | `SymbolRecord → hug output` (the CLI renders records since Phase 5) |
| symbol-kinds-and-visibility | No statement of which kinds `hug` labels with the keyword | New "Labels in `hug` output" subsection |

Most schema-design drifts predate this feature; they were fixed because
`docs/` is the current record. Checked and clean: no doc names a feature/fix
or links into `features/`/`fixes/`; `EXTRACTOR_REVISION` (6) matches
`cache-design.md`; schema 4.0, `container_name`, `keyword`,
`signature_text`, no timestamps, the Capability table and `is_recursive`
match. `docs/symbol-schema.md` mentions `module_path`, but it is a generic
design essay, not this library's record schema; left alone.
`docs/prelude-semantics.md` is likewise a research note, so the new page
describes `--prelude` itself instead of linking it. No test reads any of
the edited docs (grep of both packages' tests and sources).

### Acceptance sweep (spec "Testing" and "Focused cases")

| Spec case | Test(s) (L1 unless marked) |
|---|---|
| Keyword matrix, every language; a keyword in a comment never labels | `tree-hugger::l1 symbols_header::keyword_matrix_holds_for_every_language`, `a_keyword_inside_a_comment_never_becomes_the_keyword`, `less_common_declarations_report_their_keyword` |
| Signature matrix; contiguous sub-range; exclusions; `None` not the body | `symbols_header::signature_matrix_holds_for_every_language`, `every_fixture_signature_is_a_contiguous_part_of_its_declaration_text`, `headers_that_cannot_be_isolated_are_none`, `the_documented_comparison_rows_hold` |
| Coverage guard (fixture + matrix row per language; `multi/`) | `symbols_coverage::every_language_has_matrix_rows`, `every_language_has_a_primary_symbols_fixture`, `every_symbols_fixture_parses_without_syntax_errors`, `multi_tree_has_one_clean_file_per_language` |
| Cache correctness per language (warm = `--no-cache`) | `symbols_cache::every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run` |
| Goldens (`symbols.plain`, `symbols.json`) | `tree-hugger-cli::symbols golden::plain_symbols_over_multi_match_the_golden`, `golden::json_symbols_over_multi_match_the_golden` |
| `functions`/`types` kind sets; `classes` `symbols` + `members` | `report::functions_and_types_select_exactly_their_kind_sets`, `classes::classes_summarize_classes_types_interfaces_and_traits` |
| JSON envelope, round trip, no `declaration_text`; `--json-full` indexes and selector rejection; empty selection | `report::the_json_envelope_holds_only_selected_records_and_ends_with_errors`, `report::json_full_reports_complete_indexes`, `report::json_full_rejects_symbol_selectors_and_other_commands`, `report::an_empty_selection_is_an_empty_files_array` |
| JSON with a failed file (`--json`, `--json-full`, stderr, exit) | `report::a_failed_file_is_reported_last_on_stderr_and_in_the_exit_status` |
| Performance counters: zero diagnostics over `multi/`, one capture per file, warm hit zero parses, `lint`/`symbol_index_v2()` still diagnose | `symbols_query::a_symbol_query_over_multi_runs_no_diagnostics_and_one_capture_per_file`, `symbols_query::the_complete_index_and_lint_still_run_diagnostics_from_one_capture`, `cache_lookup::symbol_commands_run_no_diagnostics`, `cache_lookup::complete_warm_hits_parse_nothing`, `cache_lookup::lint_and_json_full_run_diagnostics` |
| Parallel = single thread | `golden::json_symbols_over_multi_are_identical_with_one_thread`, `golden::json_full_over_multi_is_identical_with_one_thread` |
| Two runs identical, cold then warm | `golden::json_symbols_over_multi_are_identical_cold_and_warm`, `golden::json_symbols_over_multi_are_identical_across_runs` |
| Cache: query text, revision, language, schema each a miss; kinds/capabilities change coverage not key; widening runs only missing work; third narrow run a hit; corrupt and concurrent writes | `symbols_cache::each_key_input_and_stored_identity_decides_a_miss`, `projected_results_equal_uncached_queries`, `projected_results_over_multi_equal_uncached_queries`, `widening_capabilities_runs_only_the_missing_passes`, `widening_kinds_reextracts_once_for_the_union`, `a_corrupt_entry_is_a_miss_and_is_replaced`, `concurrent_writers_leave_a_readable_entry_whose_coverage_matches`; CLI `cache_lookup::concurrent_processes_leave_one_readable_entry`, `cache_robustness::*` |
| Binding: index = full scan; distinct unresolved refs kept; `is_recursive` `Unknown`/`None` | `tree-hugger` unit `analysis::tests::name_index_selects_the_same_targets_as_a_full_scan`, `name_index_keeps_the_first_of_equal_candidates`; `symbols_query::distinct_unresolved_references_are_all_kept`, `is_recursive_is_unknown_when_semantics_run_and_absent_otherwise`. The spec's "v1 adapter maps both to `false`" has no subject: v1 `SymbolInfo` has no recursion field (Phase 3 decision) |
| Prelude: aliases, unresolved, env names, outside targets, no duplicate parse | `prelude::prelude_exports_are_synthetic_records_at_their_declarations`, `prelude_environment_names_select_declarations`, `a_target_outside_the_scan_is_resolved_but_not_listed`, `prelude_and_scan_parse_each_file_once` |
| Order: each `--sort`, tie-breakers, groups, JSON file order, argument errors | `order::each_sort_order_lists_declarations_by_its_keys_and_tie_breakers`, `sort_name_keeps_same_named_types_together_in_path_order`, `sort_orders_declarations_within_file_and_directory_groups`, `json_keeps_file_order_and_sorts_within_each_file`, `removed_and_misplaced_sort_flags_are_argument_errors` |
| Classes: interface/trait containers, static/instance sections, field flags without member kinds | `classes::*` (three tests) |
| Output failures: broken pipe, file errors, deterministic parallel error order | `report::a_closed_stdout_ends_the_run_quietly`, `report::failures_are_reported_in_file_order_with_any_thread_count` |
| Quiet stderr; `--verbose` stats and failure messages | `cache_lookup::cache_hits_and_cache_failures_print_nothing`, `cache_lookup::verbose_reports_cache_use_and_cache_failures` |
| Performance sample | this section, above |

Every case maps to a named test that exists and passed in the gate run
below. Phase 6 adds no code and so no new test; the Input Robustness Matrix
does not apply (no reader changed).

### Gates run

- `just test` (tree-hugger + tree-hugger-cli L1): **797/797 passed**
  (58 s).
- `just lint`: exit 0, no warnings.
- `just test-l2`: 3/3 passed (tmux, WezTerm; Kitty returns at once as when
  unavailable).
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `just ci-local --plan`: only `tree-hugger` and `tree-hugger-cli` cells,
  plus two narrowed test-input cells (`sniff` for `Cargo.lock`,
  `test-toolkit` for `docs/dependencies.md`), both from earlier phases'
  changes. No surprise scope.
- Cross-OS: not run in this phase. Phase 6 changes documentation only; the
  Phase 5 evidence stands (Linux cross-check 167/167; Windows compile-only,
  host storage preflight refused; WSL via nightly CI).
- No pre-existing failures.

### Incidents

- The verification loop for the hyperfine runs first failed with `unexpected
  argument '--no-cache'`: zsh does not word-split an unquoted `$c` (the same
  trap Phase 1 recorded). Hyperfine itself splits its command strings, so its
  timings were valid; the hand-run re-check used `${=c}`.
- `cd tree-hugger` from inside `tree-hugger/` again jumped (zoxide) to
  `/Volumes/coding/personal/rusty-biscuit/tree-hugger`; two read-only greps
  ran there. Caught by a missing file; everything after used absolute paths.

## Hand-off: final state

**Implementation complete, ready for review.** All six phases are
implemented; the spec directory stays where it is (the author moves it to
`_completed` after review), and nothing was committed in this phase.

### Rulings 1–7, as finally applied

| # | Ruling | Outcome |
|---|---|---|
| 1 | Chunks of `4 × current_num_threads()`, delivered in file order | Built (`scan::in_file_order`). Peak RSS of whole-repo `hug symbols` fell from 3.48 GB to 1.95 GB |
| 2 | Symbol selectors rejected by `--json-full` | Built with the Phase 1 list (name filters classified by the scan's own classifier, `--exclude-symbols`, `--exported`, `--prelude`, `classes` `--name`/`--static-only`/`--instance-only`); `lint`/`imports` reject `--json-full` too |
| 3 | `classes --json` `members` hold the flag-filtered set | Built |
| 4 | Marker for synthetic prelude records | **Departure:** no new field. The existing `visibility.is_synthetic` (with `is_reexported`) carries it, so the schema stayed at 4.0 without an additive field |
| 5 | `is_recursive` `Unknown` | Built: `Some(Unknown)` whenever semantics run, `None` otherwise. v1 has no field, so there is nothing to map |
| 6 | Coverage encoding (`covered_kinds` `"all"` or ascending list, `completed_capabilities`, malformed = miss) | Built; unordered or repeated lists are a miss too |
| 7 | Adapters read `container_name`; `--group-by-module` groups by directory | Built as ruled |

### Departures from the spec (all recorded in their phase sections)

- Prelude marker reuses `visibility.is_synthetic` (Ruling 4, above).
- "Run only the missing passes" is literal for docs, semantics and
  containment; references and diagnostics re-parse on a partial hit, since
  their inputs (live tree, syntax nodes) are not cacheable (Phase 4).
- Imports are always part of `Declarations`; no imports capability. Phase 3
  left the question to this sample: `imports --no-cache` now takes 0.10 s
  (5.4% of before) against 0.12 s for `functions`, so a separate capability
  would save almost nothing. Not added.
- Default `--json` drops `docs.comments` (attached comments belong to the
  docs pass no listing requests) and rewrites `identity.stable_key`'s path
  segment scan-root-relative; IDs are unchanged (Phase 5).
- The JSON size target is met on the input it was set on (2.72 MB) but not
  on today's larger tree (3.20 MB); the remaining per-record cost is listed
  above and needs a schema decision.
- The spec's "v1 adapter maps `is_recursive` to `false`" does not apply:
  v1 has no such field.

### Follow-ups

1. **v1 removal audit** (spec Open Question "Removing v1 from the library").
   Remaining v1 users seen in this work: `cli/src/prelude.rs` builds the
   synthetic record through `SymbolRecord::from(SymbolInfo { … })` and
   resolves exports with v1 `SymbolInfo`/`SymbolKind`; `cli/src/render.rs`
   converts `SymbolKindV2` to v1 `SymbolKind` for labels and colors;
   `imports` and `lint` still emit the v1 `PackageSummary`/`FileSummary`
   JSON; `TreeFile::symbols()` and the v1↔v2 adapter remain public, and the
   in-process caches (`InProcessCache`, `InMemorySymbolCache`,
   `SymbolSnapshot`, `AnalyzerFingerprint`) have no CLI caller. An audit
   should list every public v1 entry point with its callers before deciding.
2. **Root-independent record IDs.** IDs hash the analyzed absolute path, so
   the same file has different IDs in two checkouts (Phase 5).
3. **Record size.** Trimming kind data, spans, repeated `type_info` views
   and always-serialized visibility booleans would bring today's
   `--json functions` under 3 MB; it is a schema change.
4. **Windows behavior evidence.** Only compile evidence exists locally
   (host storage preflight); the first push to `main` runs Windows CI.
5. Pre-existing, noticed only: the Go `Grow(amount int) (amount int)` return
   in the signature (Phase 1), and `skills/tree-hugger/diagnostics.md` still
   importing from `tree_hugger_lib` (Phase 3).
