---
kind: plan
feature: improved-performance
spec: spec.md
total_phases: 7
created: 2026-10-08
phase: 1
agent: claude/opus
yolo: false
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/dead_code.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/src/ignore_directives.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - tree-hugger/lib/src/analysis/declaration_names.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/syntax_markers.rs
  - tree-hugger/lib/src/dead_code.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/navigation.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/static_flags/cpp.rs
  - tree-hugger/lib/src/file/extract/static_flags/php.rs
  - tree-hugger/lib/src/file/extract/static_flags/python.rs
  - tree-hugger/lib/src/file/extract/static_flags/rust.rs
  - tree-hugger/lib/src/file/extract/static_flags/scala.rs
  - tree-hugger/lib/src/file/extract/visibility/c.rs
  - tree-hugger/lib/src/file/extract/visibility/cpp.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/extract/visibility/lua.rs
  - tree-hugger/lib/src/file/extract/visibility/rust.rs
  - tree-hugger/lib/src/file/extract/visibility/swift.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/parents.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/relations/lexical.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_5:
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/source_file.rs
docs_updated_during_phase_5:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_6:
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/file_identity.rs
  - tree-hugger/cli/src/changed.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/spelling.rs
  - tree-hugger/cli/tests/changed/ownership.rs
docs_updated_during_phase_6:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_7:
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
docs_updated_during_phase_7:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/tree-hugger/SKILL.md
source_code:
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/dead_code.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/src/ignore_directives.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/lib/src/analysis/declaration_names.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/syntax_markers.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/navigation.rs
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/static_flags/cpp.rs
  - tree-hugger/lib/src/file/extract/static_flags/php.rs
  - tree-hugger/lib/src/file/extract/static_flags/python.rs
  - tree-hugger/lib/src/file/extract/static_flags/rust.rs
  - tree-hugger/lib/src/file/extract/static_flags/scala.rs
  - tree-hugger/lib/src/file/extract/visibility/c.rs
  - tree-hugger/lib/src/file/extract/visibility/cpp.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/extract/visibility/lua.rs
  - tree-hugger/lib/src/file/extract/visibility/rust.rs
  - tree-hugger/lib/src/file/extract/visibility/swift.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/parents.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/relations/lexical.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/lib/tests/l1/file_identity.rs
  - tree-hugger/cli/src/changed.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/spelling.rs
  - tree-hugger/cli/tests/changed/ownership.rs
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
documentation:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - .claude/skills/tree-hugger/SKILL.md
  - tree-hugger/docs/references.md
  - tree-hugger/docs/cache-design.md
completed_phase: 7
implemented: true
---

# Plan: Tree Hugger Performance

## Summary and Definition of Done

This plan carries out [the spec](./spec.md). Finding IDs (P1, C1, L3, …)
refer to the spec's sections; the spec holds the evidence, locations and
risks, and this plan does not repeat them.

There are seven phases:

| Phase | What | Lands as | Builds | CI runs |
|---|---|---|---|---|
| 1 | Baseline and spikes that need no build | log only | 0 | 0 |
| 2 | Byte-identical fixes already backed by evidence | PR 1 | dev builds | 1 |
| 3 | One spike session (switch-gated arms); author rulings | log only, branch never merged | 3 | 0 |
| 4 | Navigation and Rust sharing | PR 2 | dev builds | 1 |
| 5 | Output pipeline and command-level work | PR 3 | dev builds | 1 |
| 6 | Native path identity | PR 4 (`ci:all-os`) | dev builds | 1 (all OSes) |
| 7 | Policy changes the author approved | PR 5+ (one per ruling group) | dev builds | 1 each |

That is five or more pull requests, one more than the spec's "about four".
Path identity is split out so that only that small PR pays for
`ci:all-os`, as the spec's evidence budget allows.

Success looks like:

- [x] Every phase follows the spec's
  [Evidence budget](./spec.md#evidence-budget-builds-and-ci-cycles):
  - while iterating, compare `hug` output and run targeted tests;
  - run the full `just test` + `just lint` once per PR;
  - run `just test-l2` only in Phase 5 (rendered output) and in any Phase 7
    change that touches the terminal.
- [x] Every PR before Phase 7 leaves public output byte-identical. The proof
  is the [output diff protocol](#output-diff-protocol), run from the same
  absolute root.
- [x] The implementation log records every spike result: command, switch
  arm, build profile, revision, worker count, cache state, load average, and
  the time and work counts.
- [ ] Phase 3 ends with the author's rulings on the spec's Open Questions.
  Phase 7 implements only what was approved.
  - **Not met:** no ruling has been recorded (the spec's `human_review` is
    still open). Phase 7 built only the ungated O4 free part and deferred
    every ruling-gated item (implementation log, Phase 7).
- [x] Docs, the skill and dependency catalogs are updated in the PR that
  changes the behavior they describe.
- [x] The feature directory is left active for author review. The agent
  never moves it to `_completed`.

### Output diff protocol

This is the iteration-time equivalence check. It needs only the release
binary, not a test build.

1. Build `target/profiling/hug` at the base revision and save the binary as
   `hug-base` in the scratchpad.
2. Use one fixed scratch `TMPDIR` per arm, and run every command from the
   **same absolute directory** (IDs hash absolute paths). The corpus:
   - `tree-hugger/` itself;
   - `tree-hugger/lib/tests/fixtures/symbols/multi/**`, selected explicitly
     with `--file`, because ordinary scans skip `fixtures/`;
   - the measured p99 files: `schematic/definitions/src/openai/types.rs` and
     `biscuit-terminal/lib/src/components/filesystem/mod.rs`.
3. Run each of these with both binaries, cold (`--no-cache`), then warm
   twice, then once more after a wide run:
   - `--json-full symbols`
   - `--json symbols`
   - `symbols --plain`
   - `classes --plain`
   - `imports`
   - `lint --json`
   - `refs <name>` for one Rust and one TS name
4. `cmp` each pair of outputs. Only `--verbose` work counts may differ, and
   only where a task says so.

## Necessary Rules

These are decided here with the spec's recommendations as defaults. The
author may overrule any of them before the phase that depends on it.

1. **Spike switches** (Phase 3) are read once from `HUG_SPIKE` (a
   comma-separated list) into a `LazyLock` static. They live only on the
   spike branch and are never merged.
2. **Quiet host.** Run the scaling spikes (L2's thread sweep, P11) only when
   `uptime` shows a 1-minute load average below the core count (16). Every
   timing records the load average. Do not set any other threshold.
3. **oxlint.** If `oxlint` is not installed (the dev host warns "external
   lint adapter 'oxlint' is unavailable"), Phase 1 records the per-file
   discovery cost instead of spawn counts. Installing oxlint is the author's
   call.
4. **L11 in Phase 2 is the correctness half only:** a buffered writer, and a
   closed pipe ends the command with exit 0 (`hug imports … | head -1`
   currently panics, exit 101, reproduced on 2026-10-08). Per-file
   streaming of `lint`/`imports` moves to Phase 5.
5. **No public schema bumps** in Phases 2–6. A change that would alter
   records stops the phase for a ruling.
6. **Do not use `cargo fmt`.** Do not commit unless the author asks.

---

## Phase 1: Baseline and No-Build Spikes

Uses the existing `target/profiling/hug`. If the tree has moved, rebuild it
once (`cargo build --profile profiling -p tree-hugger-cli`). Nothing in this
phase changes code.

### Tasks

- [x] **Create `implementation-log.md`** with a results table: date,
  revision, finding, arm, command, threads, cache state, load average, wall
  time, user time, counts.
- [x] **Baseline record.** For each, record the load average and warm or
  cold state:
  - cold (empty `TMPDIR`) and warm `hug symbols --count` over the monorepo;
  - warm `--plain`;
  - first and second `--json`;
  - `--json-full` on the two p99 files, warm and `--no-cache`;
  - single-file `hug symbols --count tree-hugger/lib/src/lib.rs`, from the
    repo root and from outside it.

  These are the comparison points for Phases 2–6.
- [x] **O4 candidate sizing.**
  1. Run `--json-full symbols` on `scripts/ci/test_affected_scope.py` and on
     a generated Python class with 300 methods that use `self` (write it to
     the scratchpad).
  2. With a short `python3 -I` script, count occurrences, the sum of
     `candidates.len()`, and the output bytes.
- [x] **L5 cost.**
  - If oxlint is available: count spawns for `hug lint 'some-ts-dir/**'`
    (`sudo execsnoop`, or `fs_usage`), plus wall time.
  - Otherwise: `sample` a `hug lint` run over a JS/TS-heavy glob and record
    the time spent in `OxlintAdapter` discovery.
- [x] **L6 TTY sample.** In the author's real terminal (not a forged
  `TERM_PROGRAM`), run `hyperfine -N` on `hug symbols <file>` against
  `hug symbols --count <file>` with a warm cache; the difference is renderer
  plus terminal detection. Also record the `defaults` spawns under iTerm2,
  if that is the terminal in use.
  - Done as far as an agent can (pseudo-TTY and piped runs). The terminal
    is WezTerm, not iTerm2; the real-terminal sample is deferred to the
    author (see the implementation log).
- [x] **L2 symptom check, only under rule 2's load condition.** Warm and cold
  `darkmatter/` `hug symbols --count` with `RAYON_NUM_THREADS=1,4,8,16`
  (`hyperfine -w 1 -r 3`).
  - Speed-up under 8× at 16 threads with low load means the symptom
    reproduces.
  - Otherwise L2 and P11 are marked "not reproduced" and drop out of
    Phase 5.

**Checkpoint 1:** the implementation log holds the baseline, the O4/L5/L6
numbers and the L2 verdict, or "deferred: host busy". No code has changed.

---

## Phase 2: Byte-Identical Fixes with Evidence in Hand (PR 1)

The spec's evidence already supports every task here, and every task keeps
output byte-identical. Each Wave 1 group owns distinct files, so the groups
can run in parallel.

### Wave 1 (parallel)

#### 2A — Cache validation (`lib/src/cache/stored_evidence.rs`)

- [x] **C1.** In `unbound_names_have_a_binding` (the `taken` loop at
  `:876-900`, verified):
  - test `checked.contains(text)` before any span test;
  - replace `taken.iter().any(..)` with a start-sorted, merged `Vec<(u32,
    u32)>` and an advancing cursor (readings are produced in source order);
  - return readings without a `Vec` per identifier;
  - guard the second char-level pass (`:904-932`) with a `memchr` probe for
    the bytes it looks for (`r#` for Rust, `\u` for ECMA).

  The accept and refuse results must stay identical.
- [x] **C2.** Replace the `made` closure (`:657-663`, verified: two linear
  scans of `scoped` per edge) with a `HashSet` keyed by
  `(owner, target, name_identity::key(language, name), kind)`, built once.
  Fix the per-record `contains` at `:674`/`:686` and the per-edge `find` in
  `summaries_in_first_occurrence_order` (`:768-776`) the same way, keeping
  first-occurrence order.
- [x] **C7.**
  - Build `LineStarts` once in `StoredEntry::validate` and pass it to both
    checks (`:325`, `:433`).
  - In `import_inventory` (`:127-130`, verified to hash through
    `biscuit_hash::blake3_hash`), serialize a borrowed view instead of
    cloning into `StoredImport`. Keep `biscuit_hash`.
  - Add a unit test pinning the digest of a fixed import list, before and
    after.
- **Verify:** `just test cache_robustness symbols_cache stored_evidence
  cache_upgrade symbols_cache_imports cache_lexical`. Then the protocol's warm
  and wide-to-narrow comparisons on the p99 files: `--verbose` must show
  `Hit`, and the time must drop below the `--no-cache` time.

#### 2B — Query pipeline and native diagnostics (`file/tree_file.rs`, `file/tree_file/definitions.rs`, `analysis/mod.rs`, `dead_code.rs`, `rule_registry.rs`)

- [x] **P4.**
  - In `capture_definitions` (`definitions.rs:194` onward), keep the
    `local.definition.import` capture nodes it already sees and skips
    (`:236`).
  - Factor `imported_symbols`' body (`tree_file.rs:170`) into
    `imports_from_nodes(nodes)`. `imported_symbols()` stays public and calls
    it after its own capture run.
  - `run_query` (`analysis/mod.rs:97`) takes the imports from the
    definitions result instead of a second query run.
  - `definitions()` and `definitions_with_scopes()` return the import nodes
    alongside, through a small struct, not by widening the tuple further.
  - Keep sorting and deduplicating by `(start_byte, end_byte)` exactly as
    today (`tree_file.rs:236-237`).
  - No counter changes: `imported_symbols` increments no counter (verified),
    and `definition_captures` stays at 1. `QueryKind::Imports` and
    `RECORD_QUERIES` stay, so the fingerprint is unchanged.
- [x] **P9.** Add `is_exported: bool` to `ExtractedSymbol`
  (`definitions.rs:81`), computed once where definitions are materialized.
  Read it in `exports_from` (`tree_file.rs:1057`), `records_from`
  (`:1870`), `exported_symbols` (`:1044`) and `local_symbols` (`:1085`).
  `is_exported_definition` (`classify.rs:1138`) stays the one decision.
- [x] **L8, library part.** Add `RuleRegistry::builtin() -> &'static Self`
  backed by a `LazyLock`. `diagnostic_metadata` (`tree_file.rs:151-160`)
  uses it instead of `RuleRegistry::new()` per diagnostic. `new()` and
  `get_or_default` keep their signatures.
- [x] **L9.**
  - `build_source_context_from_range` (`tree_file.rs:1606-1631`, verified
    `lines().nth`): use a line-start index built once per diagnostics run
    (`OnceLock<Vec<usize>>` on `TreeFile`, split on `\n` only), keyed by
    **line number**. The text slice drops one trailing `\r`, which is
    exactly `str::lines()`. A line past the end gives `""`, as today.
  - `dead_code.rs:313` and `tree_file.rs:1311`: replace the
    `named_child(index)` loops with `named_children(&mut cursor)`.
- [x] **L10.** In `apply_ignore_directives` (`tree_file.rs:1157`), return
  early when `diagnostics.is_empty()`, or when the source does not contain
  `"tree-hugger-ignore"`. Both prefixes (`ignore_directives.rs:21,24`) are
  case-sensitive `strip_prefix` matches that contain that literal, so the
  early return is exact.
- **Verify:**
  - `just test tree_file lint_diagnostics symbols_query symbols_cache
    symbols_coverage ignore_directives dead_code`.
  - A new unit test comparing the line-index context to `lines().nth` on LF,
    CRLF, a missing final newline, an empty file and out-of-range lines.
  - The protocol (`--json-full`, `lint --json`, `imports`).

#### 2C — Occurrences and declaration climbs (`analysis/occurrences.rs`, `extract/declaration.rs`, `extract/qualified_name.rs`)

- [x] **O5.** Compute `written_path` once per occurrence (verified at
  `occurrences.rs:555` and `:634`) and pass `Option<&str>` into
  `resolve_supported`.
- [x] **P1(a).**
  - `is_scope_boundary` (`declaration.rs:133`): move `node.parent().is_none()`
    to the end of the `||` chain. The function is pure, so reordering keeps
    the result.
  - `resolve_declaration_node` (`:161-174`): compute `node.parent()` once
    per step.
  - `Builder::prefix` (`qualified_name.rs:196-197`): give `Builder` the root
    node ID, and test `node.id() != root_id` instead of
    `node.parent().is_some()`.
  - **Not** the `fill_metadata` scope reuse (`definitions.rs:460`). It runs
    per *candidate*, not only for the winner, so reusing the winner's scope
    is wrong for other candidates. It moves to Phase 4 together with the
    parent index.
- **Verify:** `just test symbols_occurrences symbols_qualified_names
  symbols_scope symbols_relations symbols_owner_identity`. Then the protocol
  (`--json-full`, `refs`).

#### 2D — CLI lint and output correctness (`cli/src/main.rs`)

- [x] **L8, CLI part.** Replace `RuleRegistry::new()` at `main.rs:1542`,
  `:1600`, `:1760` and `:1831` with `RuleRegistry::builtin()`. Before
  removing the second `apply_lint_policy` pass (`render_summary`), prove it
  idempotent with a unit test (apply once vs twice over every rule ×
  `--deny`/`--warn`/`--allow`/`--strict`). If it is not idempotent, keep
  both passes and record that in the log.
- [x] **L11, correctness half.** Route the `imports` and `lint` text and
  JSON output through one `BufWriter<StdoutLock>` passed as `&mut dyn Write`
  to `print_file_header` (`:1717`), `render_summary` (`:1742`) and
  `render_imports` (`:2104`), plus the `run` sites (`:1148-1233`). Map
  `BrokenPipe` to exit 0 by reusing `declarations::write_status` semantics.
  Output order and content stay unchanged.
- **Verify:**
  - `just test report filters` plus a new closed-pipe test for `imports`
    and `lint`, using `Scratch::process` (the pattern already in
    `cli/tests/symbols/report.rs`).
  - `hug imports 'tree-hugger/**' | head -1` exits 0.

### Wave 2 (after Wave 1)

- [x] **Docs.**
  - `docs/cache-design.md`: performance notes only if they mention the old
    algorithms. The robustness tables are unchanged.
  - `docs/symbol-reports.md` / the CLI README: the `imports`/`lint` closed
    pipe now exits 0.
  - The tree-hugger skill: one line on `RuleRegistry::builtin()`.
- [x] **Full verification, once:** `just test` and `just lint` in
  `tree-hugger/`, and the whole output diff protocol against `hug-base`.
- [x] **Timing sample:** re-run Phase 1's baseline commands and log them
  next to the baseline.

**Checkpoint 2:** PR 1 is ready, with a byte-identical output diff and green
tests. The log shows:
- warm wide-entry hits on `openai/types.rs` faster than `--no-cache`;
- the cold-scan change;
- `imports | head` exits 0.

Stop for review before pushing.

---

## Phase 3: Spike Session and Author Rulings

The spike branch starts from the tip of Phase 2. It carries the
`HUG_SPIKE` switches (rule 1) and is never merged. Each arm is the smallest
code that answers its question.

### Wave 1: build the spike binary (one `profiling` build)

- [x] `p1b`: a lazy per-file parent memo behind one helper
  (`ancestors_of`). It is used by the top callers in the spec's parent
  table: `scope_around`, `Lexical::build`, `is_exported_definition`,
  `Builder::prefix`, `symbol_scope`, `find_symbol_container`,
  `refine_symbol_kind`. Under the arm, `debug_assert_eq!` against
  `Node::parent()`.
- [x] `p2-prefilter`: skip `rust_pattern_non_bindings` when the file has no
  glob `use`, and no bare captured name equals a const, static, unit struct,
  const generic or import name, or `None`/`Self`. Add a `--verbose` counter
  of skipped and run walks.
- [x] `p3-share`: carry `impl_self_types`' map and `walk().impls` from
  `rust_impl_owners` into `link_relations`. Add a temporary `Lexical::build`
  counter.
- [x] `p5-gate`: skip the `records_from` metadata block for `Local` records.
  This is for measurement only; its output is knowingly different.
- [x] `p11-nocount`: `count_shared_context_steps` becomes a no-op.
- [x] `window`: the sliding-window `in_file_order`, with W = `4 × threads`.
- [x] `flags-on-miss`: add Flags to every cache-enabled miss.

### Wave 2: background builds (start while Wave 3 runs)

- [x] **B1 + B2 combined:**
  `CARGO_PROFILE_PROFILING_LTO=fat CARGO_PROFILE_PROFILING_CODEGEN_UNITS=1`
  plus a `mimalloc` `#[global_allocator]` in the CLI binary (spike branch
  only), built into a separate `--target-dir` so it doesn't evict the main
  build.
- [x] **C4 decode bench:** add a `cache_decode` criterion group to
  `lib/benches/tree_file.rs`. It times `serde_json::from_slice::<StoredEntry>`
  on real `--json-full` entries copied from the scratch cache, against a
  stripped stored form: no per-record `file_path`, provenance once per
  entry, spans as arrays. Run it with `cargo bench -p tree-hugger --bench
  tree_file -- cache_decode`.

### Wave 3: measure (one session, rule 2 for scaling arms)

Log every arm against the no-switch run of the same binary. Use
`hyperfine -w 1 -r 3` on `darkmatter/` or `tree-hugger/`, plus one
whole-monorepo confirmation of each promising arm.

- [x] P1: `p1b` on vs off, cold `symbols --count`. Re-sample and record the
  `ts_node_parent` share. Note that `fill_metadata`'s per-candidate scope
  remains, for Phase 4.
- [x] P2: skip rate over the monorepo's Rust files, and cold time on vs off.
- [x] P3: `Lexical::build` count (expected to halve on Rust), and cold Rust
  time on vs off.
- [x] P5: the gain bound from `p5-gate` on cold default `symbols --count`.
- [x] O-findings: a References profile with `sample` on `--json-full` and
  `refs` for the three files the spec names, with and without `p1b`.
  Record O1's export and excluded-context climbs separately.
- [x] C3: after Phase 2, a narrow warm `--count` over entries widened by
  `--json-full`, against narrow-only entries.
- [x] C5: the cold miss cost of the same `--json` query with
  `flags-on-miss` on vs off, so that output and serialization are identical.
  Then a listing followed by `--json` on vs off.
- [x] L2 / P11 (only if Phase 1 said "reproduced"): `window`, `p11-nocount`
  and both together, each with a `RAYON_NUM_THREADS` sweep.
- [x] B1 + B2: cold and warm `--count` and warm `--plain`. Isolate the two
  only if the combined gain is clearly above noise.
- [x] C4: decode-time ratio and reconstruction cost.

### Wave 4: rulings

- [x] Write a one-page summary in the implementation log. For each Open
  Question, give the numbers and the spec's recommendation.
- [ ] **The author rules** on C5, C3/C6, C4/P15, P5, O4, L5, L3, L6, C9 and
  B1/B2. Record the rulings in the log. The spec stays as decided; any
  departure is recorded in the log and the docs.
  - Open: awaits the author. The questions, with options and a
    recommendation each, are in the spec's `human_review_items`.

**Checkpoint 3:** every spike row is logged and every ruling recorded.
The spike branch is deleted, or kept local for reference. No CI run has
happened.

---

## Phase 4: Navigation and Rust Sharing (PR 2)

Build on Phase 3's numbers. Items marked *conditional* are done only if
their spike showed a gain.

### Wave 1

- [x] **P1.** Implement the parent lookup that won in Phase 3: (b), the
  lazy memo, or (c), the full index from one cursor pass. Owned by the
  per-file facts (`FileFacts` / the occurrence `Walk`); indexes never
  outlive their tree.
  - Move the top `parent()` callers onto it.
  - Bring in the `fill_metadata` scope reuse: compute it once per
    *candidate* through the index, not borrowed from the winner.
  - Pin the index with a test asserting `index.parent(n) == n.parent()` for
    every node of every `symbols/` fixture, including anonymous, aliased
    and missing nodes.
  - Done as (c). The index is installed per thread for the duration of
    `run_query` (`file/parents.rs`), not threaded through signatures;
    see the implementation log.
- [x] **O2.** Do `ScopeTree::with_rules`' parents and depth with one
  span-sorted stack sweep (as `smallest_enclosing` does), and move
  `scope_around` and `Regions::around` onto the P1 index.
  - Done through the P1 index instead of a span sweep, which cannot prove
    ancestry for zero-width or equal-range nodes (implementation log).

### Wave 2 (parallel, after Wave 1)

- [x] **O1.** Excluded-context and `export_statement` lookups use a
  nesting-aware index recorded during `Walk::run`. That index keeps node
  identity, never bare byte intervals, per the spec's O1 risk notes.
- [x] **O3.** Handle `with_statement` in `visit_ecma`. Swift
  `swift_element_labels` uses the cursor.
- [x] **P3.** Share the impl/self-type decisions and the `Lexical` build
  between `rust_impl_owners` and `link_relations`. Also skip
  `Declarations` in `link_relations` when nothing can resolve (P14).
  - The `Lexical` walk is shared; the self-type decisions are not (the
    linker needs its own lookup site for the trait). P14's early return is
    done.
- [x] **P2** *(conditional)*:
  - Above ~80% skip rate: the prefilter.
  - Otherwise: share the walk between definitions and bind (P2(a)).
  - Done as P2(a): Phase 3 measured a 27% skip rate.
- [x] **P6, P7, P10, P14 remainder, O6, O8.** Small byte-identical cleanups
  per the spec. P7 keeps IDs pinned with an equality test against the
  current `stable_symbol_key`/`symbol_id_from_stable_key`.
  - Done: P6, P7, P10, part of P14 and O8. Not done, with reasons in the
    log: O6 and the rest of P14/O8.
- [x] **P8** *(conditional)*: memoize comment/attribute runs only if the
  `prev_sibling` share is still visible after P1.
  - Not done: Phase 3 measured the sibling share at 1.3% after `p1b`.
- [x] **P12, O7** *(conditional, on the References profile)*.
  - Not done: the Phase 3 References profile showed no hashing or
    `rust_impl_type` frames at a visible share.

### Wave 3

- [x] **Verify:**
  - targeted tests: `just test symbols_occurrences symbols_relations
    symbols_owner_identity symbols_shared_context symbols_relation_work
    symbols_query symbols_cache symbols_flags symbols_metrics
    symbols_field_lists`;
  - the full output diff protocol;
  - then `just test` and `just lint` once.

  `symbols_shared_context` and `symbols_relation_work` assert linear
  `shared_context_steps`. Where the work count changes on purpose, update
  those expectations, and record each change in the log.
- [x] **Docs:** the tree-hugger skill's architecture bullets (the parent
  index, the shared Rust facts). Topic docs are unchanged, since there is no
  behavior change.
- [x] **Timing sample** against Phase 2's numbers.

**Checkpoint 4:** PR 2 has a byte-identical diff and green tests. The cold
monorepo `symbols --count` is logged. Stop for review.

---

## Phase 5: Output Pipeline and Command-Level Work (PR 3)

### Wave 1 (parallel)

- [x] **L1.**
  - For the default single-file listing unit, render on the workers into
    `Vec<u8>` in `work`. Capture terminal capabilities and width once on
    the main thread; workers never probe the TTY.
  - Keep cross-file units on the current path.
  - Stop the per-line `Terminal` clone in `Renderer::term_at`.
  - The biscuit-terminal literal-row fast path is a separate item: only if
    a narrow API that preserves escaping and width can be added (spec L1).
  - Done for the terminal and Prose layouts; Markdown keeps the delivery
    path (its layout carries blank-line state between units). The
    biscuit-terminal fast path was not added: warm `--plain` already fell
    2.35 → 0.72 s (implementation log).
- [x] **L4.** Make `hug refs` two-phase:
  - phase 1: every kind and scope, no Occurrences;
  - decide ambiguity before writing anything;
  - phase 2: Occurrences on the one matching `SourceFile` snapshot.

  Add no text prefilter.
  - Done. A scan of exactly one file asks for Occurrences in phase 1 and
    reuses that result, so its cost and `--verbose` totals are unchanged.
- [x] **L7, L14, L15.** Add the run-scoped package-root memo and move JSON
  trimming and sorting into `work`. Key stats by enum. Use the run-scoped
  `is_test_file` manifest memo, which never enters records or cache
  entries. Consume reused prelude results instead of cloning them.
  - Done, except the `is_test_file` memo and the root-identity precompute:
    both are now worker-side costs measured below 1% (implementation log).
    JSON entries and lines are also serialized on the worker.
- [x] **L11, streaming half.** Render `lint`/`imports` text per file in the
  delivery closure, deciding clean-file suppression from `files.len() > 1`.
  Adapter warnings keep their position.
  - An unavailable adapter is warned once, right before the first file it
    affected, after flushing earlier output.
- [x] **L12.** `god-files`: one read per candidate, a metadata-size
  pre-reject, and a borrowed source.

### Wave 2 (conditional on Phase 1's L2 verdict and Phase 3's numbers)

- [x] **L2.** The sliding window, with the cancellation, error and flush
  rules from the spec. New tests cover one worker, a slow head, empty
  input, a failed write and a closed pipe.
  - The window advances only past a delivery that flushed, so a failed
    write still stops within one window (`scan::tests`; the closed-pipe
    bounds in `symbols/report.rs` are unchanged).
- [x] **P11.** Padding or thread-local counters, per Phase 3's result,
  preserving the `work_counters()` contract (spec P11 risk).
  - Dropped, as Phase 3 decided: `p11-nocount` changed nothing at any
    thread count. The atomics stay.

### Wave 3

- [x] **Verify:**
  - targeted tests: `just test golden export reports outline refs
    filters report grouping changed`;
  - the output diff protocol;
  - `just test-l2` (L1 changes rendering);
  - then `just test` and `just lint`.
- [x] **Docs:** `docs/symbol-reports.md`, if broken-pipe or streaming bounds
  changed wording ("chunk" becomes "window"); `docs/references.md` for the
  two-phase `refs` (`--verbose` totals); the CLI README; the skill.
- [x] **Timing sample:** warm `--plain`, `refs`, `imports | head`.

**Checkpoint 5:** PR 3 has a byte-identical diff, green L1 and L2, and a
log entry. Stop for review.

---

## Phase 6: Native Path Identity (PR 4, `ci:all-os`)

Load the `os` skill first. This phase alters how native path spellings are
reached, though never the spellings themselves.

- [x] **C8.** Add a crate-private identity-preserving `SourceFile`
  constructor used only for `collect_files` output. Public constructors
  still canonicalize.
- [x] **L13.** Add an identity-only membership check for scanner output in
  `changed.rs:60`. The ownership checks are unchanged.
- [x] **L3.** Add a conservative literal-positional fast path, with fallback
  to the walker whenever equivalence is uncertain (spec L3 rules). Pin it
  with focused cases in the filter and spelling suites: anchored literal,
  bare name, ignored parent, fixture, hidden file, symlink, multiple
  patterns, no match. Each case compares the fast path with the walker.
- [x] **Verify:** `just test filters spelling changed cache_lookup`, the
  protocol, then `just test` and `just lint`. Push with the `ci:all-os`
  label, once.
  - Done locally: protocol byte-identical, `just test` 2063 passed,
    `just lint` clean, and the targeted suites green on Linux, native
    Windows and WSL2 rigs. The push with `ci:all-os` belongs to the
    separate commit step. The Windows run found two pre-existing
    `changed/ownership.rs` failures (`/` in symlink targets), fixed here.
- [x] **Timing sample:** single-file in-repo latency against Phase 1's
  70–103 ms.
  - 66.9 → 9.2 ms (implementation log).

**Checkpoint 6:** PR 4 is green on every OS. Stop for review.

---

## Phase 7: Approved Policy Changes (PR 5+)

Only the rulings Phase 3 recorded as approved. Each group is its own small
PR. Each needs the docs, the `cache_robustness` cells and the projection
tests the spec lists for it.

- [x] **C5** *(if approved, Flags only)*: cache-enabled misses also compute
  Flags; `--no-cache` stays selective; projection back to the requested
  capabilities. Update `docs/cache-design.md`'s outcome examples.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log.
- [x] **C4** *(if approved, compact private JSON)*: a private stored form
  with exact public reconstruction. Old entries are rejected cleanly as a
  private format change, not a public schema bump. The `cache_robustness`
  and `cache_upgrade` tooling moves to the new form. **P15** follows as a
  private range, if approved.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log.
- [x] **P5** *(if approved)*: kind-and-scope pair coverage.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log.
- [x] **O4** *(free part always; contract part only if approved)*.
  - Free part done: `Resolver::new` stable-sorts each `by_name` bucket
    once, the rule-less path passes the bucket as a slice (no clone, no
    per-occurrence sort), and `name_only` keeps the first-wins dedup.
    Output byte-identical (protocol, Python corpus added).
  - Contract part **deferred**: no author ruling recorded.
- [x] **L5** *(per the batch-metadata ruling)*: batched oxlint.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log. oxlint is also not installed on the dev host, so the batching could not be measured.
- [x] **L6** *(if approved)*: a deterministic unstyled constructor in
  biscuit-terminal.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log.
- [x] **C9** *(if approved)*: `hug cache clear`.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log.
- [x] **B1/B2** *(if approved)*: a scoped profile or build invocation, and a
  mimalloc dependency in the CLI only, with the dependency catalogs
  updated.
  - **Deferred.** No author ruling is recorded (the spec's `human_review_items` are still open), so nothing was built; deferred in the log.
- [x] **C3** *(only if the author overrules the recommendation to keep full
  validation)*.
  - **Not built.** The author has not overruled the recommendation, so full
    validation stays.

Verification per PR: targeted tests for the touched area, the protocol
(where output is meant to stay identical), then the full `just test` and
`just lint`. Any terminal change also runs `just test-l2`.

**Checkpoint 7:** each approved item is merged or explicitly deferred in
the log. Implementation is complete and ready for review.

---

## Confidence Notes

**Verified against the code on 2026-10-08 for this plan:**
- C1's `taken` loop and C2's `made` closure.
- C7's `biscuit_hash` use.
- P4's unconditional `imported_symbols` call (`analysis/mod.rs:97`), and
  that it increments no counter.
- P9's call sites (`tree_file.rs:1044`, `:1057`, `:1085`, `:1870`).
- O5's two `written_path` calls.
- L8's `RuleRegistry::new()` sites (lib `tree_file.rs:154`; CLI
  `main.rs:1542`, `:1600`, `:1760`, `:1831`).
- L9's `lines().nth` and both `named_child(index)` loops.
- L10's prefix constants.
- P1(a)'s three sites, and that `fill_metadata`'s scope runs per candidate
  (hence its deferral).
- The `imports` closed-pipe panic (exit 101, reproduced).
- That `just test` forwards name filters to nextest.

**Remaining uncertainty:**
- **L8:** whether `apply_lint_policy` is idempotent. The task tests it
  before removing the second pass.
- **P4:** whether `definitions_with_scopes` can return import nodes without
  disturbing the record/definition index alignment. Keep the nodes in a
  separate field; never interleave them.
- **Phase 2 timing gains** are expected from the spec's profiles, not yet
  measured after the change. The Wave 2 sample records them.
- **Phases 4–7** depend on Phase 3's numbers and rulings. Their task lists
  are complete for the recommended answers. A different ruling changes only
  the matching Phase 7 item.
