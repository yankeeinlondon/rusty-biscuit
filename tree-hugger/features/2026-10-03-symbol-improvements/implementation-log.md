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
