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
packages:
  - tree-hugger
  - tree-hugger-cli
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
