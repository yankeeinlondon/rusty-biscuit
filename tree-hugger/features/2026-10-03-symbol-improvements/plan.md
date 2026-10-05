---
kind: plan
feature: symbol-improvements
spec: spec.md
total_phases: 6
created: 2026-10-04
phase: 6
agent: claude/sonnet
yolo: true
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
packages:
  - tree-hugger
  - tree-hugger-cli
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

# Plan: Fast, deterministic symbol reports on v2 records

## Summary and definition of done

### The work

The `hug` symbol commands (`functions`, `types`, `symbols`, `classes`) do the
full lint/semantic pipeline, sequentially, then guess labels from source text
and emit oversized, non-deterministic JSON. This feature makes them
record-native, selective, cached by complete key, parallel and deterministic.

The work has five strands, in dependency order:

1. **Record foundation (library).** Schema 4.0: drop timestamps, rename
   `module_path` → `container_name`, add `identity.keyword`, fill
   `source.signature_text` for 16 languages.
2. **Selective analysis (library).** `AnalysisPass::Diagnostics` split out,
   `SourceFile`, `SymbolQuery` + capabilities, one definition-capture per file,
   lazy metadata, kind push-down, indexed bind, honest `is_recursive`.
3. **Cache (library + CLI).** Source hash from bytes read once, computed
   fingerprint, coverage-bearing single entry per file, atomic write-back,
   partial-hit merge.
4. **CLI.** Consume records directly, versioned JSON envelope with trailing
   `errors`, `--sort`, `--verbose`, rayon parallel scan with ordered bounded
   output, prelude reuse, one CLI-local renderer.
5. **Docs, perf sample, acceptance.**

### Success looks like

- [x] `just test`, `just test-l2` and `just lint` pass in `tree-hugger/`.
- [x] A symbol command over `multi/` records zero diagnostics-pass executions;
      a complete warm hit records zero parser constructions; `lint` and
      `TreeFile::symbol_index_v2()` still run diagnostics.
- [x] `--json` is byte-identical between parallel and single-thread runs, and
      between cold and warm runs; no timestamps anywhere.
- [x] `hug --json functions` on `tree-hugger/` is under 3 MB. (2.72 MB on the
      Phase 1 baseline input; today's larger tree is 3.20 MB, recorded with its
      remaining cost in the Phase 6 log.)
- [x] Performance sample recorded in the implementation log: `functions
      --no-cache` ≤ 40% of 1.07 s, warm `functions` ≤ 15% of 0.64 s, `lint
      --no-cache` no slower, peak RSS of `hug symbols` no higher than the
      pre-change baseline. A missed goal is recorded with remaining cost.
- [x] Every doc listed in the spec's "Documentation to update" is updated.
- [x] Agent terminal state is "implementation complete, ready for review"; no
      move to `_completed`, no `just complete`, no commit unless asked.

### Conventions for all tasks

- US English. Never run `cargo fmt`. Tests via `just test` (nextest), never
  `cargo test`. Subagents must be told the whole session tree is
  **non-interactive**.
- Any behavior change includes a pass over the symbol's `///`/`//!` docs and
  inline comments (repo comment-quality rule).
- Bump `EXTRACTOR_REVISION` with any change to extracted records.
- New CLI code goes in new modules (e.g. `cli/src/report.rs`, `render.rs`,
  `order.rs`) rather than growing `cli/src/main.rs` (3,516 lines), so CLI
  tasks in one wave do not collide on one file.
- The v1 library API and adapters stay working; removing v1 is out of scope.

---

## Phase 1: Baseline, rulings, and test scaffolding

Goal: record what must be measured before any code changes, settle rulings,
and build the fixtures and guards the later phases assert against.

### Necessary Rules

The spec is marked finalized with no open rulings, and it chose no spikes. The
points below are gaps found while planning. Each has a **proposed ruling** the
implementer applies unless the author overrides; none blocks starting.

1. **Bounded concurrency size.** Spec says "chunks"/"bounded outstanding
   results" without a number. *Proposed:* process files in chunks of
   `4 × rayon::current_num_threads()`; serialize each chunk in file order before
   starting the next. Revisit only if the peak-RSS goal is missed.
2. **Definition of "symbol selector" for `--json-full` rejection.** *Proposed:*
   every flag today that narrows records rather than files: `--name/-n`,
   `--exported`-style filters, `--static-only`, `--instance-only` (read the
   actual clap definitions in Task 1.3 and list them in the log). File
   selectors (paths, globs, `--no-cache`, git selection) stay allowed.
3. **`classes --json` `members` under `--static-only/--instance-only`.**
   *Proposed:* `members` holds only the members the command displays, i.e. the
   flag-filtered set, matching the spec's "members the command displays".
4. **Synthetic prelude records.** *Proposed:* mark with an existing
   provenance/flag field if one exists; otherwise add an additive
   `synthetic: bool` inside the 4.0 bump (no extra version change). Confirm in
   Task 1.3.
5. **`Unknown` for `is_recursive`.** *Proposed:* confirm a tri-state type with
   an `Unknown` variant already exists in `semantics`; if not, adding one is
   part of the 4.0 bump.
6. **Cache entry coverage encoding.** *Proposed:* store `covered_kinds`
   (sorted set; "all" encoded explicitly, not as an empty list) and
   `completed_capabilities` (sorted set) inside the entry. Absent, null,
   wrong-typed or empty coverage is a **miss** (see the robustness matrix in
   Phase 4), never an empty result and never "all".
7. **Container-rename fallout.** `identity.module_path` is used by v1 adapters
   and `--group-by-module` today. *Proposed:* adapters read
   `container_name`; `--group-by-module` keeps grouping by file directory
   (spec D7), unrelated to the field.
8. **Wider measurement (for the author, not scheduled).** The spec limits the
   perf sample to one host and the baseline commands. If the author wants
   Linux/Windows RSS or repeated runs, that is a separate decision; this plan
   does not schedule it.

### Tasks (Wave 1: sequential prerequisites)

- [x] **Measure RSS baseline** *(must precede every code change)*
    - Build `hug` in release from the pre-change tree.
    - From the repo root run `/usr/bin/time -l hug symbols --no-cache >
      /dev/null`; record command, build profile, cache state and "maximum
      resident set size".
    - Also record the existing baselines you will re-measure later
      (`functions --no-cache`, warm `functions`, `lint --no-cache`,
      `--json functions` byte size) so the after-sample uses the same inputs.
    - Write all of it into a new `implementation-log.md` beside this plan.
- [x] **Capture pre-change goldens**
    - Run plain `hug --plain symbols` over a scratch copy of the (new) `multi/`
      tree *after* Task 1.2, using the current binary, and keep the output as
      the reference for "keeps today's visible layout" (Phase 5 renderer).
      Store as `cli/tests/fixtures/golden/symbols.plain` candidate; it is
      regenerated only for deliberate differences (default order changes to
      file/source, labels now come from keywords) and each difference is
      logged.

### Tasks (Wave 2: parallel, after Wave 1 baseline)

- [x] **Multi-language fixtures**
    - Create `lib/tests/fixtures/symbols/multi/` with one small file per
      `ProgrammingLanguage::all()` (16 languages), enough for multi-file,
      ordering, parallel and golden tests; include one same-named type in two
      files (for `--sort name` tie-break tests) and one nested directory.
    - Add `strip_ansi_codes` + path-redaction use through the existing
      `corpus` helpers.
- [x] **Extend per-language fixtures**
    - For each `lib/tests/fixtures/symbols/<language>.<ext>`, where the
      language has the construct, add: large/multi-line initializer;
      multi-line declaration header; annotated/decorated declaration with the
      annotation inside the node; annotation between modifiers (Java, C#); a
      comment containing a declaration keyword.
    - Re-run existing `symbols_*` tests; expectations that change only because
      of new fixture content are updated, not weakened.
- [x] **Recon notes**
    - Read `analysis/mod.rs` (`analyze_file`, `parse_pass`), `cache/mod.rs`,
      `file/tree_file.rs`, `cli/src/main.rs` scan/render/classes paths, and
      `cli/src/prelude.rs`; log the answers to Necessary Rules 2, 4 and 5 and
      the list of callers of `symbol_presentation`,
      `binding_token_for_symbol`, `SOURCE_LINE_CACHE`, `InMemorySymbolCache`,
      `InProcessCache`.

### Validation checkpoint 1

- [x] RSS baseline and other baselines are in `implementation-log.md`.
- [x] `just test` green with the new fixtures; coverage guard still passes.
- [x] Rulings 1–7 recorded as accepted or overridden in the log.

---

## Phase 2: Record foundation (schema 4.0, keyword, signature text)

Goal: records carry everything the renderer needs, deterministically, with
schema 4.0. Depends on Phase 1 fixtures.

### Wave 1 (sequential: shared schema surface)

- [x] **Schema 4.0 bump**
    - Add `SchemaVersion::V4_0`; move records, file indexes, CLI envelope
      constants and analyzer fingerprints to it.
    - Remove `created_at_epoch_ms` / `updated_at_epoch_ms` from provenance
      (extractor identity, version and pass remain).
    - Rename `identity.module_path` → `identity.container_name` (beside
      `container_kind`); `qualified_name` stays unpopulated.
    - Add `identity.keyword: Option<String>` (serde: skip when `None` only if
      that matches existing facet conventions; check).
    - Update the v1↔v2 adapters to the renamed field; v1 API unchanged.
    - Bump `EXTRACTOR_REVISION`; cached entries of schema 3.0 are rejected, not
      migrated. Update the cache-upgrade tests per the skill's rule (keep the
      old literal-key tests unchanged, add a `StaleRevision` case).
    - Fix every compile error across lib, cli, and tests (`schema_version:
      V3_0` sites in `analysis`, `cache`, `god_files`, `schema_v2`).
    - Rewrite tests that assert timestamps, `module_path` or schema 3.0
      (pre-authorized).

### Wave 2 (parallel by language group; each task touches only its readers/queries/fixtures)

Split the 16 languages into groups and dispatch one subagent per group. Each
fills **both** `keyword` and `signature_text` from the declaration node in the
same extraction pass (`signature_text` a contiguous sub-range of
`declaration_text`), and adds the per-language matrix rows.

- [x] **Keyword + signature: Rust, Go, Swift, Scala**
- [x] **Keyword + signature: TypeScript, JavaScript, Java, C#**
- [x] **Keyword + signature: C, C++, PHP, Python**
- [x] **Keyword + signature: Perl, Lua, Bash, Zsh**

Common requirements for each group:

- `keyword` comes from the syntax node, never from text; `None` where the
  construct has no keyword (Java method).
- `signature_text`: outer whitespace stripped, internal whitespace and newlines
  preserved; excludes initializers, leading in-node annotations/decorators,
  doc comments, trailing `=`/`=>`; annotation between modifiers kept;
  unsupported grammar → `None`, never the body.
- Cover the spec's six table rows where the language applies; assert
  not-applicable explicitly; label unsupported grammar separately.
- Shared helpers (e.g. a `header_range` helper in `file/extract/`) are created
  in Wave 1 of this phase by the first group to need them; to avoid conflicts
  the **first task in the wave to start commits the helper first** or the
  orchestrator lands a stub helper before dispatch. (Orchestrator: land the
  stub as the last Wave 1 item.)

### Wave 3 (after Wave 2)

- [x] **Matrix tests and coverage guard**
    - Table-driven `keyword` and `signature_text` matrices, one row per
      language, naming the language on failure.
    - Extend the coverage guard to fail on a missing row in each matrix.
    - "Cache correctness" matrix row is added in Phase 4.
- [x] **Locals-capture and keyword-in-comment cases**: a keyword inside a
      comment never yields a label.

### Validation checkpoint 2

- [x] `just test` and `just lint` green; `hug --json symbols` shows no
      timestamps, `container_name`, `keyword`, `signature_text`.
- [x] Two consecutive runs of `hug --json symbols --no-cache` over `multi/` are
      byte-identical (determinism of records alone).
- [x] `docs/symbol-kinds-and-visibility.md` and `docs/symbol-schema-design.md`
      updated for the contracts and the 4.0 bump (drift rule: same change).

---

## Phase 3: Selective analysis in the library

Goal: the library plans only the passes and kinds a request needs. Depends on
Phase 2 (stable record shape). Cache coverage in Phase 4 builds on the
completeness contract defined here.

### Wave 1 (sequential: defines the contract the rest consume)

- [x] **Pass planner and `AnalysisPass::Diagnostics`**
    - Split diagnostics out of `parse_pass` into `AnalysisPass::Diagnostics`.
    - Planner dedupes and orders prerequisites: parse → bind → semantic;
      parse → docs; diagnostics request their own dependencies.
    - `completed_passes` records only work performed.
    - `TreeFile::symbol_index_v2()` keeps running diagnostics; `lint` requests
      diagnostics including syntax, experimental semantics and external
      adapters.
    - Add a test-visible pass counter (atomics, not thread-locals) for
      diagnostics-pass executions, parser constructions, and
      definition-capture extractions.
- [x] **`Capability`, `SymbolQuery`, result type**
    - `SymbolQuery::new().kinds(..).capabilities(..)`; library default selects
      all kinds. Capabilities: declarations, containment, references,
      semantics, structured docs, diagnostics.
    - Result: records, file/language identity, `completed_passes`,
      `completed_capabilities` so "empty" ≠ "not computed".
    - Selection never removes prerequisite data (e.g. excluded parents kept for
      containment).

### Wave 2 (parallel: disjoint files)

- [x] **`SourceFile`**
    - `SourceFile::read(path)` / `.parse()` / `.query(&q)`; owns bytes,
      detected-or-forced language and source identity; shares the planning path
      with `TreeFile::query`. Existing constructors become wrappers and keep
      the existing error contract. Never reread between hash and parse.
- [x] **Single definition-capture per file**
    - Capture definitions once; derive records, locals and exports from it;
      native semantic diagnostics reuse the same captures instead of calling
      `symbols()` / `exported_symbols()` again.
- [x] **Lazy metadata after de-duplication**
    - Keep today's duplicate rule and winner; extract `signature`,
      `type_metadata`, `doc_comment` from the winner, falling back to later
      duplicates in existing order only for missing fields. Edit vendored
      queries only where dedup requires it, updating provenance notes.
- [x] **Indexed binding**
    - `HashMap<&str, Vec<usize>>` per file for `find_reference_target`; same
      candidate choice as before. `push_unique_ref` uses a per-owner set
      (resolved: by ID; unresolved: by name+qualifier).
    - `semantics.is_recursive` = `Unknown` whenever semantics run, `None`
      otherwise; v1 adapter maps both to `false`; doc comment says "not known
      to be recursive".

### Wave 3 (after Wave 2)

- [x] **Kind push-down**
    - Excluded kinds skip metadata extraction (and binding when safe) but keep
      identity, span and container data (they can shadow or own selected
      symbols). Selecting only methods must not change reference resolution or
      lose containers. `classes` additionally keeps Field/Property/Constant
      visibility and `static` data for `align_field_metadata`.

### Tests (parallel with Wave 3, separate test files)

- [x] Counter tests: symbol query over `multi/` → zero diagnostics passes; one
      definition capture per file cold; `symbol_index_v2` and `lint` still run
      diagnostics.
- [x] Binding equivalence tests: new index selects the same targets as the
      previous full scan (keep a reference scan in the test); `push_unique_ref`
      keeps distinct unresolved references with missing IDs; `is_recursive`
      states.
- [x] Narrow-vs-full equivalence: for every language, records for a narrow query
      equal the filtered records of a full query (identity/span/container).

### Validation checkpoint 3

- [x] `just test`, `just lint` green; v1 API tests untouched or only changed for
      the container rename.
- [x] `docs/symbol-schema-design.md` documents explicit pass dependencies,
      `is_recursive`, retained v1 adapters.

---

## Phase 4: Persistent cache with complete keys

Goal: one persistent, coverage-bearing, atomically written entry per file.
Depends on Phase 3 (`SymbolQuery`, `SourceFile`, capabilities).

### Input Robustness Matrix (cache entry reader)

This phase adds a reader of a persisted format with new load-bearing fields.
Load-bearing fields: `covered_kinds`, `completed_capabilities`, `schema_version`,
the embedded `fingerprint`, `source_hash`, and `records`/`index`. Each cell's
outcome below is asserted through the public CLI/library result (a served hit
vs. a recompute), by **one test per format (JSON entry)** that starts from a
real cache entry written by the tool, with a control row proving the unedited
entry is a complete hit.

| Shape | `covered_kinds` / `completed_capabilities` | `schema_version` / `fingerprint` / `source_hash` | `records` |
|---|---|---|---|
| absent | miss (recompute) | miss | miss |
| explicit null | miss, not conflated with absent in the reported reason | miss | miss |
| wrong type, whole field | miss | miss | miss |
| wrong type, one element | miss (no filtering of the bad element) | n/a | miss |
| wrong type, every element | miss (not read as empty coverage) | n/a | miss |
| empty (`[]`/`{}`) | miss; empty coverage never means "all" | n/a | valid only if coverage says so; otherwise miss |
| duplicate key | miss | miss | miss |
| trailing/invalid content | miss | miss | miss |

Smells to grep before closing: `#[serde(default)]` on these fields,
`Option<T>` where absent vs null differ, `filter_map(.. as_str())`,
`unwrap_or_default()`, `.ok()` on a load-bearing parse. The existing
`cache_robustness` test (ten snapshot fields) is extended, not duplicated, and
its table in `docs/cache-design.md` updated.

### Wave 1 (sequential)

- [x] **Fingerprint and key**
    - Compute the fingerprint from: schema, grammar version/identity, actual
      effective query text (inherited queries and overlays) via an
      **injectable** query-text input, relevant config, `EXTRACTOR_REVISION`.
      Rules/external-tool versions only for passes using them.
    - Key = normalized file identity + source hash (separate from
      `PassFingerprint`) + forced language + fingerprint. Kinds and passes are
      **not** key inputs.
    - Use `biscuit-hash` for every digest with an unambiguous canonical
      encoding; add it as a normal dependency to the tree-hugger packages that
      need it; update dependency docs.
- [x] **Entry format and atomic write**
    - Entry records covered kinds and completed capabilities (Ruling 6).
    - `PersistentCache::put`: temp file in the same directory, then rename;
      last rename wins; corrupt/incompatible = miss (never empty success);
      `--no-cache` bypasses reads and writes.

### Wave 2 (parallel, after Wave 1)

- [x] **Lookup/merge logic**
    - Three outcomes: complete hit (no parse, no analysis); missing
      capabilities only (run only those passes, write merged entry); missing
      kinds (re-extract once for the union, write wider entry). Full-index
      requests cannot reuse an entry lacking relations, docs or diagnostics.
    - Wired through `SourceFile::query` so one hash read feeds lookup and parse.
- [x] **Remove in-process layers from the CLI scan path**
    - Drop `InMemorySymbolCache` / `InProcessCache` from the single-visit scan
      (keep the library APIs). Prelude keeps a request-local result map
      (Phase 5).
- [x] **Cache tests**
    - Edited query text, extractor revision, language override, schema each → a
      miss. Narrow→broad and broad→narrow match uncached results. After a
      capability-widening run: one entry, only missing passes ran; after
      kind-widening: re-extracted once; a third narrow run is a complete hit.
    - Corrupt entry, concurrent writes (threads and processes): every surviving
      entry is readable and its coverage matches contents.
    - Robustness matrix test above.
    - Add the "Cache correctness" per-language row (warm == `--no-cache`).

### Validation checkpoint 4

- [x] Complete warm hit: zero parser constructions (counter).
- [x] `docs/cache-design.md` documents the single layer, computed fingerprint,
      partial-hit write-back and the updated robustness table.
- [x] `just test`, `just lint` green.

---

## Phase 5: CLI — records, report, order, parallelism, renderer

Goal: the CLI consumes records directly and produces deterministic, sized
output. Depends on Phases 2–4. New code lives in new modules; `main.rs` only
wires.

### Wave 1 (sequential)

- [x] **Argument surface**
    - Add `rayon` to tree-hugger-cli (pre-authorized).
    - Global `--verbose` on `Cli`.
    - `--sort file|name|kind` on only `functions`, `types`, `symbols`,
      `classes` (not in shared `ScanOptions`/`CommonArgs`); remove
      `--sort-by-kind` and `--sort-by-module`; repeated `--sort`,
      `--sort container`, and `--sort` on `lint`/`imports` are argument errors.
    - Update shell completions and help text.
- [x] **Per-command queries**
    - Map each command to its fixed kind set and capabilities (functions,
      types, classes with containment, symbols all kinds); `lint` requests
      diagnostics.

### Wave 2 (parallel: disjoint new modules)

- [x] **Order module**
    - Keys per spec D7: file → start → end → kind (spec's kind order) → ID;
      `name` is case-sensitive written name; relation targets sorted
      deterministically. JSON: file entries always in portable path order;
      `--sort` orders only within a file's `symbols`. Groups
      (`--group-by-file`, `--group-by-module`) are independent of `--sort`.
- [x] **Report/JSON module**
    - Envelope `{report_schema_version: 1, schema_version: {4,0}, root_dir,
      files:[{file, language, completed_passes, completed_capabilities,
      symbols}], errors:[{file, kind, message}]}`; `errors` always last and
      always present; file entries with no matches omitted; empty selection =
      `files: []`.
    - `classes` adds `members` keyed by container ID; fields stay in
      `kind_data`.
    - `--json-full`: same envelope with `indexes`; rejects symbol selectors
      (Ruling 2); requests every capability including diagnostics.
    - Default records omit `declaration_text` and unrequested relations,
      semantics and blobs; portable scan-root-relative paths without altering
      record IDs; `imports` and `lint` keep their current JSON shapes.
    - Stream with `serde_json::to_writer` to locked buffered stdout; broken pipe
      exits quietly; write error stops work.
- [x] **Renderer**
    - One CLI-local renderer built on `biscuit-terminal` `Prose` and
      `UnorderedList`, fed from records (keyword, signature, visibility,
      container); keeps today's layout, compact signatures, OSC 8 hyperlinks and
      `--plain`; single locked `BufWriter`.
    - Delete `render_symbols`, `render_symbol_line`, `symbol_presentation`,
      `binding_token_for_symbol`, and `SOURCE_LINE_CACHE`.
    - Classes: Interface/Trait containers summarized; static/instance sections
      and `--static-only`/`--instance-only` behavior preserved.

### Wave 3 (after Wave 2)

- [x] **Parallel scan driver**
    - `par_iter` per file in chunks (Ruling 1), results collected in file order;
      per-worker parser/state; counters returned with each result or atomics;
      failures reported in file order, each also on stderr, nonzero exit.
    - `--verbose` gates the cache-stats line (persistent hits / partial hits /
      misses) and recoverable cache-failure messages; analysis errors,
      lint adapter warnings and argument errors always print.
- [x] **Prelude reuse**
    - Two-phase: analyze prelude files, discover targets, then scan; reuse a
      request-local map of complete indexes; analyze out-of-scan targets on
      demand without emitting them; preserve aliases and locations, unresolved
      public re-exports and `PRELUDE` env names; synthetic records marked and
      not borrowing the target ID; membership decided before filters, using an
      unrestricted index.

### Tests (L1; parallel authors, separate files under `cli/tests/`)

- [x] Goldens: `cli/tests/fixtures/golden/symbols.plain` and `symbols.json` over
      `multi/`, compared after `strip_ansi_codes` and path redaction.
- [x] Envelope, failed-file (unreadable/unparsable scratch file: good files
      listed, one `errors` entry, stderr, nonzero), `--json-full` rejection,
      class report `members`.
- [x] Parallel == single-thread (`RAYON_NUM_THREADS=1`) byte-identical; cold
      then warm byte-identical.
- [x] Order tests (all sort values, groups, JSON file-order invariance, argument
      errors) and classes tests.
- [x] Output failures: broken pipe, file errors, deterministic parallel error
      order; no terminal focus.
- [x] Quiet stderr: warm run and failed-cache-write run print nothing without
      `--verbose`; with it, print stats line and failure message.
- [x] Prelude tests: aliases, unresolved re-exports, env names, targets outside
      the scan, no duplicate parse (counter).
- [x] Path-rendering tests follow the `os` skill (load it before touching
      path-comparison code).

### Validation checkpoint 5

- [x] `just test`, `just test-l2`, `just lint` green.
- [x] `hug --json functions` in `tree-hugger/` is under 3 MB. (2.72 MB on the
      Phase 1 baseline input, 1,580 functions; today's larger tree, 1,784
      functions, is 3.10 MB. See the Phase 5 log.)
- [x] Removed flags/`--sort` misuse yield argument errors.

---

## Phase 6: Documentation, performance sample, acceptance

Depends on Phase 5.

### Wave 1 (parallel docs; independent files)

- [x] **`docs/symbol-reports.md` (new)**: commands and kind sets (what `classes`
      displays), JSON envelope with sample, trailing `errors` and exit status,
      `--json-full`, ordering and tie-breakers, grouping, `--verbose`, Mermaid
      query → passes → render diagram. Written for a reader new to the repo; no
      links to features/fixes.
- [x] **`README.md`, CLI help, completions**: `--sort`, `--verbose`, removed
      flags, new JSON shape and default order; before/after examples and
      release notes.
- [x] **Skill**: `.claude/skills/tree-hugger/SKILL.md` and `api-reference.md`
      for `SourceFile`, `SymbolQuery`, `keyword`, `signature_text`, schema 4.0,
      removal of the in-process caches from the CLI path.
- [x] **Dependencies**: `docs/dependencies.md` and area dependency docs (`rayon`
      in tree-hugger-cli, `biscuit-hash` where now normal).
- [x] **Drift pass**: confirm earlier-phase docs (`symbol-kinds-and-visibility`,
      `symbol-schema-design`, `cache-design`) match the final behavior.

### Wave 2 (sequential, after docs)

- [x] **Performance sample** on the baseline host with release builds and the
      same inputs as Phase 1: elapsed time for the four commands, JSON byte
      size, peak RSS (`/usr/bin/time -l hug symbols --no-cache > /dev/null`) vs.
      the recorded baseline; record build and cache state. No extra hosts,
      repetitions or corpora. A miss is recorded with its remaining cost.
- [x] **Acceptance sweep**: walk the spec's Testing section and "Focused cases"
      list, ticking each against a named test; run `just test`, `just test-l2`,
      `just lint`; verify `just ci-local --plan` shows no surprise scope.
- [x] **Hand-off**: write final state into `implementation-log.md` (departures
      from spec, decisions taken on Rulings 1–7, follow-up: v1 removal audit).
      Set spec status to "implemented-ready for review" only if the repo's
      convention allows; do not move to `_completed`.

### Validation checkpoint 6 (final)

- [x] Every "Success looks like" box above is checked.
- [x] Implementation log is complete; no commit made unless the author asks.
