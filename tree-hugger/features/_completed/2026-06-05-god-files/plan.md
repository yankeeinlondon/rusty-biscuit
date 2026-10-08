---
phases: 7
created: 2026-06-05
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/god_files/mod.rs
  - tree-hugger/lib/src/god_files/constants.rs
  - tree-hugger/lib/src/god_files/types.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/Cargo.toml
docs_updated_during_phase_2:
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - tree-hugger
source_files_during_phase_3:
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/cli/src/scanner.rs
docs_updated_during_phase_3:
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_4:
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/tree_file.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
packages_during_phase_4:
  - tree-hugger
source_files_during_phase_5:
  - tree-hugger/lib/src/god_files/analysis.rs
docs_updated_during_phase_5:
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - tree-hugger
source_files_during_phase_6:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/cli.rs
docs_updated_during_phase_6:
  - tree-hugger/README.md
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
packages_during_phase_6:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_7:
  - tree-hugger/lib/src/god_files/analysis.rs
docs_updated_during_phase_7:
  - docs/dependencies.md
  - tree-hugger/features/2026-06-05-god-files/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_7:
  - tree-hugger
  - tree-hugger-cli
packages:
  - tree-hugger
  - tree-hugger-cli
---

# God Files Execution Plan

## Assumptions

- [x] Treat the functional specification in `spec.md` as authoritative for thresholds, risk bands, output shape, and deferred future work.
- [x] Save implementation under the existing `tree-hugger` lib/cli split and avoid changing unrelated package areas.
- [x] Use `biscuit-file::FileReference` for file references and `biscuit-terminal::Prose` for terminal rendering, matching repo guidance.
- [x] Keep v1 thresholds as constants, not CLI options.

## Phase 1 - Orient the Existing Surfaces

- [x] Inspect `tree-hugger/lib/src/lib.rs`, `file/tree_file.rs`, `scanner` usage, language detection, query/comment extraction helpers, symbol record APIs, and CLI subcommand patterns.
- [x] Confirm the canonical package names and workspace membership with `cargo metadata --no-deps --format-version 1` or `sniff repo`.
- [x] Identify where `ProgrammingLanguage` is defined and where supported file extensions are mapped.
- [x] Identify the existing JSON output pathway in `tree-hugger-cli` and whether public library structs already derive `Serialize`.
- [x] Identify existing CLI tests and fixture conventions for pathless scans, explicit scan dirs, ignored fixture directories, and plain output.
- [x] Validation checkpoint: document the exact files/modules to edit before coding and confirm no new architectural dependency is needed beyond the spec.

### Phase 1 Findings

**Canonical packages:** `tree-hugger` (lib, `tree_hugger` crate name) and `tree-hugger-cli` (CLI, `hug` binary). Both are workspace members.

**Files/modules to edit in subsequent phases:**

| Phase | File(s) | Purpose |
|-------|---------|---------|
| 2 | `tree-hugger/lib/src/god_files/mod.rs` (new) | Module root for constants, types, scanning, SLOC, analysis, hints |
| 2 | `tree-hugger/lib/src/god_files/constants.rs` (new) | `MODERATE_MIN_SLOC`, `HIGH_MIN_SLOC`, `MAX_BLOCKS`, `MIN_BLOCK_SLOC`, `MANY_MEMBERS_THRESHOLD` |
| 2 | `tree-hugger/lib/src/god_files/types.rs` (new) | `GodFiles`, `GodAnalysis`, `RiskBand`, `SymbolBlock`, `ContainerCallout`, `MemberSummary`, `KindHistogram`, `RefactorHint` |
| 2 | `tree-hugger/lib/src/god_files/analysis.rs` (new) | `candidates()`, `analysis()`, SLOC calculator, block ranking, hint synthesis |
| 2 | `tree-hugger/lib/src/lib.rs` | Re-export god-files public surface; add `ProgrammingLanguage::is_programming_language()` |
| 2 | `tree-hugger/lib/Cargo.toml` | Add `rayon` and `memchr` dependencies |
| 3–5 | `tree-hugger/lib/src/god_files/` (new tests) | Unit tests for SLOC, banding, block ranking, hint triggers |
| 6 | `tree-hugger/cli/src/main.rs` | Add `GodFilesArgs` and `Command::GodFiles` variant |
| 6 | `tree-hugger/cli/src/main.rs` | Handler: construct `GodFiles`, call `.analysis()`, render with `Prose` or JSON |
| 6 | `tree-hugger/cli/Cargo.toml` | Add `biscuit-terminal` dependency for `Prose` rendering |
| 6 | `tree-hugger/cli/tests/cli.rs` | Integration tests for `god-files` subcommand |

**Key existing APIs identified:**

- `TreeFile::new(path)` — parses a file, returns `TreeHuggerError` on failure.
- `TreeFile::symbols()` → `Vec<SymbolInfo>` with `name`, `kind`, `range`, `container_name`, `doc_comment`.
- `TreeFile::symbol_records()` → `Vec<SymbolRecord>` with v2 `source.body_span` for SLOC within spans.
- `TreeFile::imported_symbols()` → `Vec<ImportSymbol>` for fan-out count.
- `scanner::collect_files(root, inputs, excludes, language)` — CLI-only; uses `ignore::WalkBuilder`, honors `.gitignore`, excludes `**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**` by default.
- `ProgrammingLanguage` — defined in `shared/symbol.rs`; 16 variants; `from_path()` maps extensions; no `is_programming_language()` yet.
- `QueryKind::Comments` — exists; per-language `comments.scm` queries in `lib/queries/<lang>/comments.scm` for all 16 languages.
- `doc_comments.rs` — has `extract_doc_comment()`, `collect_doc_comments()`, `is_doc_comment_node()`, `clean_doc_comment()`.
- JSON output — CLI uses `serde_json::to_string_pretty` on `JsonOutput`, `ClassSummary`, etc.; all public library types derive `Serialize`/`Deserialize`.
- `biscuit-file::FileReference` — already a dependency of `tree-hugger-lib`; derives `Debug, Clone` but **not `Serialize`**. For JSON output we will need to either derive `Serialize` on `FileReference` or store a `PathBuf` alongside it.
- `biscuit-terminal::Prose` — available in `biscuit-terminal/lib`; not yet a dependency of `tree-hugger-cli`.

**Dependencies to add:**
- `rayon` (parallelism) — not currently in `tree-hugger` or `tree-hugger-cli`.
- `memchr` (fast newline counting) — available transitively but should be explicit.
- `biscuit-terminal` — for `Prose` rendering in CLI (Phase 6).

**No new architectural dependency is needed beyond the spec.** All required capabilities (scanner, parser, symbol extraction, comment queries, JSON serialization) already exist in the tree-hugger lib/cli split.

## Phase 2 - Add Library Types and Public API Skeleton

- [ ] Add `tree-hugger/lib/src/god_files/` with module boundaries for constants, models, scanning, SLOC metrics, analysis, and hints only if those boundaries remove real complexity.
- [ ] Define public constants: `MODERATE_MIN_SLOC`, `HIGH_MIN_SLOC`, `MAX_BLOCKS`, `MIN_BLOCK_SLOC`, and `MANY_MEMBERS_THRESHOLD`.
- [ ] Define `GodFiles` with `root: PathBuf`, `OnceCell<Vec<PathBuf>>` candidates cache, and `OnceCell<Vec<GodAnalysis>>` analysis cache.
- [ ] Define `GodAnalysis`, `RiskBand`, `SymbolBlock`, `ContainerCallout`, `MemberSummary`, `KindHistogram`, and `RefactorHint` with derives consistent with existing public types.
- [ ] Re-export the god-files public surface from `tree-hugger/lib/src/lib.rs`.
- [ ] Add `ProgrammingLanguage::is_programming_language() -> bool` returning `true` for all current programming language variants.
- [ ] Validation checkpoint: `cargo check -p tree-hugger-lib --color=never` reaches only expected incomplete-implementation errors or passes if skeleton methods are stubbed.

## Phase 3 - Implement Candidate Discovery

- [x] Implement `GodFiles::new(dir: impl AsRef<Path>) -> Self` with no filesystem I/O.
- [x] Implement `GodFiles::candidates(&self) -> &Vec<PathBuf>` using `OnceCell::get_or_init`.
- [x] Reuse `scanner::collect_files(root, &[], &[], None)` or the closest existing library-compatible scanner helper; if CLI-only, extract the minimum shared scanner logic without changing CLI behavior.
- [x] Filter discovered files to supported programming languages via `ProgrammingLanguage::is_programming_language()`.
- [x] Count physical lines by reading bytes and using a newline-counting approach equivalent to `memchr::memchr_iter(b'\n', bytes)`.
- [x] Keep files with at least `MODERATE_MIN_SLOC` physical lines and sort paths deterministically.
- [x] Parallelizable: implement candidate counting with `rayon` independently from Phase 4 analysis internals once scanner integration is clear.
- [x] Validation checkpoint: add or run a focused candidate discovery test proving no parse is required, sub-400 physical-line files are excluded, 400-line files are included, and candidate ordering is deterministic.

## Phase 4 - Implement Effective SLOC and File Analysis

- [ ] Implement an internal SLOC calculator that distinguishes physical, blank, comment-only, mixed code/comment, and effective SLOC lines.
- [ ] Reuse existing comment query infrastructure where available; add the smallest missing helper needed to mark comment-only lines by parse span.
- [ ] Implement `GodFiles::analysis(&self) -> &Vec<GodAnalysis>` so it populates candidates first, parses only candidates, analyzes in parallel, and caches the final vector.
- [ ] Re-filter parsed candidates by effective SLOC: drop files below 400, classify 400-999 as `Moderate`, and classify 1000+ as `High`.
- [ ] Preserve the unparseable-candidate fallback: emit an analysis using physical lines, empty blocks/signals, and a diagnostic note if the model needs one to represent this edge case.
- [ ] Sort final analyses with high risk first, then by descending effective SLOC within each band, then by path for deterministic ties.
- [ ] Validation checkpoint: add SLOC and band boundary tests for 399, 400, 999, and 1000 effective SLOC, including physical-high to effective-moderate demotion and effective-sub-400 drop.

## Phase 5 - Add Structural Signals, Blocks, and Refactor Hints

- [x] Compute `SymbolBlock` values from existing `symbols()` or `symbol_records()` spans, using effective SLOC within each symbol span.
- [x] Rank blocks by descending SLOC, apply `MIN_BLOCK_SLOC`, cap at `MAX_BLOCKS`, and compute `blocks_truncated`.
- [x] Populate `doc_summary` from the first useful doc-comment line when existing doc extraction exposes it.
- [x] Compute `top_level_symbol_count` from symbols with no container relationship.
- [x] Populate `KindHistogram` from top-level symbols with deterministic `BTreeMap` ordering.
- [x] Compute `max_nesting_depth` with a single tree walk or existing navigation metadata.
- [x] Compute `import_fan_out` from `imported_symbols().len()`.
- [x] Compute `todo_fixme_count` from comment text for `TODO`, `FIXME`, `HACK`, and `XXX`.
- [x] Compute `comment_density` as comment lines divided by physical lines, clamped to `0.0..=1.0`.
- [x] Add `ContainerCallout` for structural container symbols with more than `MANY_MEMBERS_THRESHOLD` members, excluding variables, fields, and parameters.
- [x] Synthesize `RefactorHint` values for dominant symbol, many unrelated top-level symbols, deep nesting, high coupling, and low code density.
- [x] Parallelizable: implement block ranking, histograms, nesting depth, coupling/debt counts, and hint synthesis as separate pure helpers with focused tests after Phase 4 exposes analysis inputs.
- [x] Validation checkpoint: add unit tests for block ranking, truncation count, many-members threshold exclusion rules, histogram determinism, and each refactor hint trigger.

## Phase 6 - Add CLI Subcommand, Rendering, and JSON Output

- [ ] Add `hug god-files [DIR] [--high-risk]` to the existing clap command structure.
- [ ] Default `DIR` to the current working directory using the existing CLI error/reporting style.
- [ ] Implement the handler by constructing `GodFiles::new(dir)` and calling `.analysis()`.
- [ ] Honor `--high-risk` by suppressing the moderate section body while keeping the heading counts for both risk bands.
- [ ] Honor `--json` by serializing the `Vec<GodAnalysis>` shape used by the library.
- [ ] Honor `--plain` through the existing terminal rendering capability path rather than hand-emitting escape codes.
- [ ] Render the report heading, risk sections, file links, block lists, signals line, truncation notes, many-member sub-lists, and refactor hints with `biscuit-terminal::Prose`.
- [ ] Ensure empty/no-candidate scans print `0` moderate and `0` high counts, omit both sections, and exit successfully.
- [ ] Parallelizable: implement JSON output tests independently from Prose styling assertions once the handler returns structured analyses.
- [ ] Validation checkpoint: add CLI integration tests for grouping/order, `--high-risk`, `--json`, default current directory behavior, empty scans, and `--plain`.

## Phase 7 - Final Verification, Documentation, and Drift Pass

- [ ] Review all new and changed rustdoc and inline comments against `docs/comment-quality.md`; delete or fix comments that restate implementation.
- [ ] Update `tree-hugger/README.md` or CLI docs only if the new public command changes documented behavior.
- [ ] Update `tree-hugger/docs/dependencies.md` and root dependency docs if any crate is added or removed.
- [ ] Update `.claude/skills/tree-hugger/SKILL.md` only if the public workflow or architecture changed enough for future agents to need it.
- [ ] Run `cargo test -p tree-hugger-lib --color=never`.
- [ ] Run the focused CLI integration test target for `tree-hugger-cli`.
- [ ] Run `cargo check -p tree-hugger-cli --color=never` if CLI tests do not already compile the command.
- [ ] Run a manual smoke command on a temporary fixture tree with one moderate file, one high file, one sub-400 file, and one high-physical/moderate-effective file.
- [ ] Validation checkpoint: confirm all acceptance criteria from `spec.md` are either implemented, tested, or explicitly documented as deferred future work.
