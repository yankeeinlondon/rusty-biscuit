---
phases: 7
created: 2026-06-05
start_phase: 1
---

# God Files Execution Plan

## Assumptions

- [ ] Treat the functional specification in `spec.md` as authoritative for thresholds, risk bands, output shape, and deferred future work.
- [ ] Save implementation under the existing `tree-hugger` lib/cli split and avoid changing unrelated package areas.
- [ ] Use `biscuit-file::FileReference` for file references and `biscuit-terminal::Prose` for terminal rendering, matching repo guidance.
- [ ] Keep v1 thresholds as constants, not CLI options.

## Phase 1 - Orient the Existing Surfaces

- [ ] Inspect `tree-hugger/lib/src/lib.rs`, `file/tree_file.rs`, `scanner` usage, language detection, query/comment extraction helpers, symbol record APIs, and CLI subcommand patterns.
- [ ] Confirm the canonical package names and workspace membership with `cargo metadata --no-deps --format-version 1` or `sniff repo`.
- [ ] Identify where `ProgrammingLanguage` is defined and where supported file extensions are mapped.
- [ ] Identify the existing JSON output pathway in `tree-hugger-cli` and whether public library structs already derive `Serialize`.
- [ ] Identify existing CLI tests and fixture conventions for pathless scans, explicit scan dirs, ignored fixture directories, and plain output.
- [ ] Validation checkpoint: document the exact files/modules to edit before coding and confirm no new architectural dependency is needed beyond the spec.

## Phase 2 - Add Library Types and Public API Skeleton

- [ ] Add `tree-hugger/lib/src/god_files/` with module boundaries for constants, models, scanning, SLOC metrics, analysis, and hints only if those boundaries remove real complexity.
- [ ] Define public constants: `MODERATE_MIN_SLOC`, `HIGH_MIN_SLOC`, `MAX_BLOCKS`, `MIN_BLOCK_SLOC`, and `MANY_MEMBERS_THRESHOLD`.
- [ ] Define `GodFiles` with `root: PathBuf`, `OnceCell<Vec<PathBuf>>` candidates cache, and `OnceCell<Vec<GodAnalysis>>` analysis cache.
- [ ] Define `GodAnalysis`, `RiskBand`, `SymbolBlock`, `ContainerCallout`, `MemberSummary`, `KindHistogram`, and `RefactorHint` with derives consistent with existing public types.
- [ ] Re-export the god-files public surface from `tree-hugger/lib/src/lib.rs`.
- [ ] Add `ProgrammingLanguage::is_programming_language() -> bool` returning `true` for all current programming language variants.
- [ ] Validation checkpoint: `cargo check -p tree-hugger-lib --color=never` reaches only expected incomplete-implementation errors or passes if skeleton methods are stubbed.

## Phase 3 - Implement Candidate Discovery

- [ ] Implement `GodFiles::new(dir: impl AsRef<Path>) -> Self` with no filesystem I/O.
- [ ] Implement `GodFiles::candidates(&self) -> &Vec<PathBuf>` using `OnceCell::get_or_init`.
- [ ] Reuse `scanner::collect_files(root, &[], &[], None)` or the closest existing library-compatible scanner helper; if CLI-only, extract the minimum shared scanner logic without changing CLI behavior.
- [ ] Filter discovered files to supported programming languages via `ProgrammingLanguage::is_programming_language()`.
- [ ] Count physical lines by reading bytes and using a newline-counting approach equivalent to `memchr::memchr_iter(b'\n', bytes)`.
- [ ] Keep files with at least `MODERATE_MIN_SLOC` physical lines and sort paths deterministically.
- [ ] Parallelizable: implement candidate counting with `rayon` independently from Phase 4 analysis internals once scanner integration is clear.
- [ ] Validation checkpoint: add or run a focused candidate discovery test proving no parse is required, sub-400 physical-line files are excluded, 400-line files are included, and candidate ordering is deterministic.

## Phase 4 - Implement Effective SLOC and File Analysis

- [ ] Implement an internal SLOC calculator that distinguishes physical, blank, comment-only, mixed code/comment, and effective SLOC lines.
- [ ] Reuse existing comment query infrastructure where available; add the smallest missing helper needed to mark comment-only lines by parse span.
- [ ] Implement `GodFiles::analysis(&self) -> &Vec<GodAnalysis>` so it populates candidates first, parses only candidates, analyzes in parallel, and caches the final vector.
- [ ] Re-filter parsed candidates by effective SLOC: drop files below 400, classify 400-999 as `Moderate`, and classify 1000+ as `High`.
- [ ] Preserve the unparseable-candidate fallback: emit an analysis using physical lines, empty blocks/signals, and a diagnostic note if the model needs one to represent this edge case.
- [ ] Sort final analyses with high risk first, then by descending effective SLOC within each band, then by path for deterministic ties.
- [ ] Validation checkpoint: add SLOC and band boundary tests for 399, 400, 999, and 1000 effective SLOC, including physical-high to effective-moderate demotion and effective-sub-400 drop.

## Phase 5 - Add Structural Signals, Blocks, and Refactor Hints

- [ ] Compute `SymbolBlock` values from existing `symbols()` or `symbol_records()` spans, using effective SLOC within each symbol span.
- [ ] Rank blocks by descending SLOC, apply `MIN_BLOCK_SLOC`, cap at `MAX_BLOCKS`, and compute `blocks_truncated`.
- [ ] Populate `doc_summary` from the first useful doc-comment line when existing doc extraction exposes it.
- [ ] Compute `top_level_symbol_count` from symbols with no container relationship.
- [ ] Populate `KindHistogram` from top-level symbols with deterministic `BTreeMap` ordering.
- [ ] Compute `max_nesting_depth` with a single tree walk or existing navigation metadata.
- [ ] Compute `import_fan_out` from `imported_symbols().len()`.
- [ ] Compute `todo_fixme_count` from comment text for `TODO`, `FIXME`, `HACK`, and `XXX`.
- [ ] Compute `comment_density` as comment lines divided by physical lines, clamped to `0.0..=1.0`.
- [ ] Add `ContainerCallout` for structural container symbols with more than `MANY_MEMBERS_THRESHOLD` members, excluding variables, fields, and parameters.
- [ ] Synthesize `RefactorHint` values for dominant symbol, many unrelated top-level symbols, deep nesting, high coupling, and low code density.
- [ ] Parallelizable: implement block ranking, histograms, nesting depth, coupling/debt counts, and hint synthesis as separate pure helpers with focused tests after Phase 4 exposes analysis inputs.
- [ ] Validation checkpoint: add unit tests for block ranking, truncation count, many-members threshold exclusion rules, histogram determinism, and each refactor hint trigger.

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
