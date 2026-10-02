---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/biscuit-file/features/2026-09-30-glob-reference/spec.md"
plan: "biscuit-file/features/2026-09-30-glob-reference/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - biscuit-file
source_files_during_phase_1:
    - biscuit-file/lib/Cargo.toml
    - Cargo.lock
docs_updated_during_phase_1:
    - biscuit-file/docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
---

# Implementation Log for 2026-09-30-glob-reference (8 phases)

Standing sections (later phases append to them): [Spikes](#spikes),
[Changed Outputs](#changed-outputs), [Departures from Spec](#departures-from-spec),
[Windows Evidence](#windows-evidence), [Handed-off Reads](#handed-off-reads).
Phase entries follow, then the [baseline inventory](#appendix-baseline-inventory).

## Spikes

### Spike 1: `globset` semantic probe

`globset 0.4.18` (the workspace's locked version), throwaway crate in
`/tmp/gr-spike1` (not committed), macOS. "lit-sep" is
`GlobBuilder::new(p).literal_separator(true)` with case-sensitive matching;
"default" is `Glob::new(p)`, which `FileMatchGlobs::compile` and the trigger
matcher use today.

| Question | Pattern → path | lit-sep | default |
|---|---|---|---|
| `*` vs `/` | `*.md` → `docs/a.md`; `docs/*.md` → `docs/a/b.md` | no; no | **yes; yes** |
| `**/` matches zero directories | `**/intro.md` → `intro.md`; `docs/**/x.md` → `docs/x.md` | yes; yes | yes; yes |
| `[id]` is a class | `[id].md` → `[id].md` / `i.md` / `d.md` | no / yes / yes | same |
| `{a,b}` alternation | `{a,b}.md` → `a.md`, `b.md` / `c.md` | yes / no | same |
| escaping a literal `[id].md` | `\[id\].md` and `globset::escape("[id].md")` = `[[]id[]].md` → `[id].md` / `i.md` | yes / no (both) | same |
| leading `!` | `!x.md` → `!x.md` / `x.md` | literal `!` (yes / no) | same |

Extra observations, all from the same run:

- Matching is case-sensitive by default (`*.MD` does not match `a.md`).
- `*` and `**` match dot-files and dot-directories (`*.md` → `.x.md`,
  `**` → `.hidden/x.md`). There is no hidden-file rule.
- `?` crosses `/` with the defaults but not with `literal_separator(true)`.
- **Windows trap.** `globset`'s `backslash_escape` defaults to `true` on Unix
  and `false` on Windows, so `\[id\].md` means different things per OS.
  `globset::escape` emits character classes (`[[]`, `[]]`, `[*]`, `[?]`),
  which read the same on every OS. Consequences for Phase 2:
  `GlobReference::escape` delegates to `globset::escape`, and the builder sets
  `backslash_escape` explicitly so a pattern never depends on the host OS.

Outcome for R4 and the `%` rewrite: `**/x` matches a top-level `x`, so the
`%` → `**/<payload>` rewrite needs no zero-depth alternative, and
criterion 15 (`{pkg}/intro.md` matched by `^**/intro.md`) holds as written.
R4 (`literal_separator(true)`, case-sensitive) stands. It **differs** from
`FileMatchGlobs::compile` and `triggers::matcher::compile_globs` today, which
use the defaults, so `*` crosses `/` there now. `find_files()` and
`::file-links` already use `literal_separator(true)`. No existing test
asserts that `*` crosses `/` in `match()` or `$path` (searched below), so
R4's "existing-test results decide" clause does not overturn it. One
shipped artifact changes; see [Changed Outputs](#changed-outputs).

### Spike 2: unfiltered-walk cost

Release build of the same throwaway crate: `walkdir` from the repository
root of this worktree, `follow_links(false)`, no filters, matching
`**/*.md` (`literal_separator(true)`) on every file. macOS dev host, one
sample per configuration (two runs where shown).

| Configuration | Entries visited | `**/*.md` matches | Wall clock |
|---|---|---|---|
| warm `target/` present (cold, then warm cache) | 376,673 (355,233 under `target/`) | 5,272 | 1.14 s, then 0.62 s |
| `target/` absent (simulated by pruning `target` entries) | 21,440 | 5,272 | 79 ms, then 62 ms |

Below the plan's ~2 s threshold, so R7 needs no extra `list_files` note.
`target/` makes up 94% of the entries and about 90% of the time. The
`find_files()` docs should still say that a `&**/…` walk from the repository
root visits build output, because the spec forbids content filters (Phase 3
docs). No other OS was measured (R12).

## Changed Outputs

Recorded as they are found; Phase 8 consolidates them.

- **`file(match(...))` and `$path` `*` stop crossing `/` (R4).** Shipped
  prompt `prompts/_reviews/review-implementation.md` (and its DMLS corpus copy
  `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-implementation.md`)
  declares `template: file(match(prompts/*.md,.claudine/prompts/*.md))`.
  Today it also offers `prompts/_add/…`, `prompts/_reviews/…`, and the other
  nested partials. After Phase 2 it offers only top-level prompts. This is
  criterion 10's intended behavior, not a regression. No other shipped
  `match()` or `$path` pattern has a single `*` after a `/`.
- **All-negation lists (R6).** Today the two consumers disagree:
  `FileMatchGlobs::is_match` admits every path no negative rejects, while
  `triggers::matcher::path_matches` admits nothing. R6 replaces both with
  `GlobReferenceError::NoPositivePattern`. No existing test, shipped schema,
  or shipped prompt uses an all-negation list (searched `match("!…`, `$path`
  lists, and every `"!` literal under `darkmatter/` and `claudine/`). The
  only all-negative-like case is Claudine's unrelated `linking::filter`, which
  is not a glob reference. So the ruling stands and departs from no
  existing behavior that a test pins.

## Departures from Spec

- **R2: optional first-root list mode not built (deliberate omission).** The
  spec calls it optional and no Darkmatter consumer uses it (Rule 2: no
  speculative features).
- **R3: warning code spelled `dm.glob.skipped_symlink`, not
  `compose.glob.skipped_symlink`.** Every existing compose warning code (and the
  DMLS diagnostic codes) uses the `dm.<family>.<code>` form (`dm.expression.unknown_identifier`,
  `dm.expression.parse_failure`, `dm.remote_cache.io_failure`,
  `dm.schema.missing_simplified_envelope`, `dm.transclusion.broken_path`).
  No `compose.` prefix exists. The closest spelling is `dm.glob.skipped_symlink`,
  one code shared by `find_files()` and `::file-links`.

## Windows Evidence

None in Phase 1 (no code paths changed). The `backslash_escape` default
(Spike 1) is the Windows risk that Phase 2's tests must cover.

## Handed-off Reads

The `2026-09-30-file-refs-use-magic` ambient-state guard still carries the
two entries this feature must delete (Decision 22):

- `darkmatter/lib/src/markdown/compose/file_links/discovery.rs`:
  `resolve_boundary` reads the current directory.
- `darkmatter/lib/src/markdown/schemas/file_match.rs`: `admits` falls back to
  `std::env::current_dir()` when the validator has no `base_dir`.

Both carry the reason "handed off to 2026-09-30-glob-reference" and are
still present at the start of this feature.

## Phase 1

### Readiness check

- `git status` at start: only this log file, untracked and empty. The plan
  expected two modified test files
  (`darkmatter/dmls/tests/l1/repository_contexts.rs`,
  `darkmatter/lib/tests/l1/entry_point_parity.rs`). Neither is modified: both
  were committed in the branch's recent
  `2026-09-30-file-refs-use-magic` review iterations (`fd5521d2c`,
  `d91d9e8ae`, and parents). No unexpected files. Nothing to report.
- `2026-09-30-file-refs-use-magic` is at Phase 8 close: its implementation log
  ends with the Phase 8 final checkpoint and hand-off, which names this
  feature as next. Its later review-to-implement iterations (2–4) are
  committed (`b4190fc09`, `fd5521d2c`), and the Claudine port it was blocked on
  landed in `042b1025d`.
- **Baseline, biscuit-file `just test`:** 1017 tests run, 1017 passed, 0
  skipped.
- **Baseline, darkmatter `just test --no-fail-fast`:** 8842 tests run, 8841
  passed, **1 failed**, 12 skipped. (The default fail-fast run stopped after
  6420 tests on the same failure.)
  - Pre-existing failure:
    `darkmatter::l1 error_snapshots::link::unrecognized_format_mentions_html_and_markdown`
    (`darkmatter/lib/tests/l1/error_snapshots/link.rs:21`). The snapshot
    expects `` Markdown `[text](href)` link `` and the renderer now emits
    `` Markdown `text` link ``: an inline link inside a code span is being
    rewritten by prose rendering. It is unrelated to glob references, file
    resolution, or schemas, and this feature does not touch that path. Left
    as is. The `.snap.new` insta wrote was deleted so the tree stays clean.

### Rulings check

Each ruling's "the implementer first confirms…" clause was checked against
the code, not deferred:

- R1, R2, R5, R7, R8, R10, R11, and R12 need no Phase 1 evidence.
- R3: convention read; see [Departures](#departures-from-spec).
- R4: confirmed by Spike 1 and the code read above; stands.
- R6: searched for all-negation lists; none, so the ruling stands.
- R9: no Rust source reads a `SCHEMA_DIR` or `SCHEMAS_DIR` environment
  variable today (the only hit is `triggers/discovery.rs`'s directory-name
  constant `SCHEMAS_DIR_NAME = "schemas"`). The reader is Phase 4's.

### Dependency note

- `biscuit-file/lib/Cargo.toml`: `globset = { version = "0.4", optional = true }`
  added and enabled by the `file-reference` feature (`dep:globset`). Same
  `0.4` requirement as `darkmatter/lib/Cargo.toml`; `Cargo.lock` gains one
  line (biscuit-file's dependency list) and no new package.
- `cargo check -p biscuit-file` passes with default features and with
  `--no-default-features`.
- `biscuit-file/docs/dependencies.md` documents the dependency as
  **planned** (no code uses it until Phase 2) and records the
  `backslash_escape` rule. The root `docs/dependencies.md` lists only
  per-crate summaries for biscuit-file and already lists `globset`, so it is
  unchanged.

### Tests

Phase 1 changes no behavior, so it adds no tests. The two spikes are
throwaway probes, as the plan directs, and were not committed. The baselines
above are the regression evidence, and they ran after the dependency was
added to `Cargo.toml`.

### Gates at the end of Phase 1 (macOS)

| Area | `just test` | `just lint` |
|---|---|---|
| biscuit-file | 1017 passed, 0 skipped | pass |
| darkmatter (lib, cli, dmls, zed-dmls; `--no-fail-fast`) | 8841 passed, 1 failed (the pre-existing snapshot above), 12 skipped | pass |

No cross-OS run: Phase 1 adds an unused optional dependency and changes no
code path. The `darkmatter` skill needed no update.

## Appendix: Baseline Inventory

Call sites by `git grep -n -w` over `*.rs`, at the start of Phase 1. This
list drives Phase 8's changed-outputs record.

### `FileMatchGlobs`, `admits_path`, `admits`, `file_match_admits`

- `darkmatter/lib/src/markdown/schemas/file_match.rs`: definitions,
  `match_keyword_factory` / `MatchKeyword::check` (validation), private
  `admits` (reads `std::env::current_dir()`, handed off), and unit tests
  `admits_path` (`fixes/**/spec.md`) and `file_match_admits` (partial,
  `features/x/spec.md` rejected, `fixes/x/spec.md` accepted) at lines ~351–370.
- `claudine/cli/src/completion/schema_completion/candidates.rs:328`:
  re-exports `FileMatchGlobs as MatchGlobs` for Claudine's candidate walk.
- `claudine/lib/src/composition/schema/supplied.rs:5,334`:
  `file_match_admits` and `root_union_match_patterns` for caller-supplied
  values (`admits_arm_files`).
- Other `admits` hits (`claudine/lib/src/signals/version.rs`,
  `sniff/lib/src/filesystem/repo/nested.rs`, and others) are unrelated
  functions with the same name.

### `find_files_fn`

- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2443`:
  definition, with its own prefix splitter and a `GlobBuilder` using
  `literal_separator(true)` (~2485).
- `darkmatter/lib/src/markdown/compose/expression/functions/paths.rs:7`:
  binding (`find_files`, alias `findfiles`).
- Tests: `darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs` (8
  functions); `claudine/cli/tests/l1/shipped_prompt_contract.rs` (a shipped
  prompt's `find_files` use).

### `resolve_boundary` (`::file-links`)

- `darkmatter/lib/src/markdown/compose/file_links/discovery.rs:35,76`:
  only caller and definition (reads the current directory, handed off).
  `GlobBuilder::new(..).literal_separator(true)` at ~105.
- Tests: unit tests in `file_links/discovery.rs` and
  `compose/tests/transclusion.rs`; `darkmatter/lib/tests/l1/reference_integration.rs`;
  `darkmatter/lib/tests/level2/level2_render_tree_terminal.rs`;
  `darkmatter/dmls/tests/l1/lsp_session.rs`.

### `schema_roots`

- Definition: `darkmatter/lib/src/markdown/schemas/triggers/discovery.rs:109`
  (`schema_roots(document_dir, boundary)`, the ancestor walk), re-exported
  from `schemas/triggers/mod.rs:36` and `schemas/mod.rs:137`.
- Production caller: `triggers::scan` (`discovery.rs:315`), called from
  `schemas/mod.rs:425`. Roots then reach bare-name `$schema` lookup through
  `triggers/assemble.rs:209,224` →
  `resolve::resolve_yaml_schema_with_roots`, and `schemas/mod.rs:508` →
  `resolve::resolve_schema_with_roots`.
- Tests: `discovery.rs` unit tests (lines ~446–528); `resolve.rs` tests
  `bare_name_import_resolves_via_schema_roots` (3363) and
  `bare_name_example_resolves_via_schema_roots` (3436), plus the
  `resolve_yaml_schema_with_roots` cases at ~3057–3195; `schemas/mod.rs:3450`
  (`triggers::scan` direct).

### Trigger glob matcher (`$path`)

- `darkmatter/lib/src/markdown/schemas/triggers/matcher.rs`:
  `compile_globs` (~432, default `Glob::new`, bare-basename `**/` expansion,
  thread-local cache) and `path_matches` (~410, all-negation admits nothing).
- Grammar and lint: `triggers/grammar.rs` (tests at ~658 and ~745, list
  `**/*.md`, `!**/_*.md`), `triggers/lint.rs`, `schemas/errors.rs`,
  `schemas/about.rs`.

### `resolve_recursive_core` (`%`)

- `biscuit-file/lib/src/file_reference/resolve.rs:129` (only caller, from
  the shared resolve core when `parsed.recursive`) and `:525` (definition).
- Tests naming recursive references: `biscuit-file/lib/tests/l1/`
  `detailed_resolution.rs`, `file_tree.rs`, `finalized_reference_resolution.rs`,
  `implicit_relative.rs`, `magic_local_roots.rs`, `precedence_flip.rs`,
  `reference_grammar.rs`, `completion_round_trip.rs`, and
  `portable_path/{configuration,excess_parent,inputs,originating_context}.rs`;
  unit tests in `file_reference/{mod,parse,resolve}.rs`;
  `darkmatter/lib/tests/l1/reference_integration.rs`.

### Other glob-crate users (for Phase 6's guard allowlist)

Production `globset` use outside the four consumers above:
`darkmatter/dmls/src/overlay/schema.rs` (`Glob::new`, ~1084),
`darkmatter/dmls/src/workspace/discover.rs` (~98), and
`darkmatter/lib/src/markdown/compose/toc_linking/filter.rs` (~64). Outside
Darkmatter (`sniff`, `research`, `biscuit-terminal`), uses are out of this
guard's scope.
