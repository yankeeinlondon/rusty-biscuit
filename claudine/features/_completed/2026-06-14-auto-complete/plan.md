---
agent: "open_code/zai-coding-plan/glm-5.2"
phases: 5
created: "2026-06-25"
start_phase: 1
yolo: "true"
source_files_during_phase_2:
  - "claudine/cli/src/completion/schema_completion.rs"
  - "claudine/cli/tests/compose_schema_cli.rs"
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - "claudine/cli/src/completion/operation_file.rs"
  - "claudine/cli/src/completion/mod.rs"
  - "claudine/cli/src/commands/compose/prep.rs"
  - "claudine/cli/src/commands/sequence.rs"
  - "claudine/lib/src/composition/error.rs"
  - "claudine/cli/tests/wrap_compose_validation.rs"
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - "claudine/lib/src/composition/error.rs"
  - "claudine/lib/src/composition/schema_validation.rs"
  - "claudine/cli/src/commands/schema_interactive.rs"
  - "claudine/cli/src/commands/wrap/sequence/tests.rs"
  - "claudine/cli/src/completion/schema_completion.rs"
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - "claudine/cli/tests/level2_auto_complete_chooser.rs"
  - "claudine/cli/tests/completion_contract.rs"
  - "claudine/cli/tests/completion_perf.rs"
  - "claudine/cli/src/completion/autocomplete_ui.rs"
docs_updated_during_phase_5:
  - "claudine/docs/topics/shell-completions.md"
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - ".claude/skills/claudine/cli-reference.md"
packages:
  - "claudine"
  - "claudine-cli"
---

# Execution Plan — Shell Completions and Autocomplete

Converts `spec.md` (status: ready for planning and implementation) into a
high-confidence, dependency-ordered execution plan.

## Orientation (what already exists vs. what is new)

The `claudine __complete` engine, the bounded walker, the per-mode
composition positional completer, the `ScopeSet` resolver, and the
schema-aware `name=`/`property=` setters are **already shipped** and are
described by the spec as the preserved contract. The genuinely new work is
narrow:

- **TAB path** (Shell Completion Extensions):
  - (a) bare `file`/`file[]` schema property falls back to the **default
    glob** instead of emitting zero candidates (today
    `cli/src/completion/schema_completion.rs` `file_candidates` returns
    `Vec::new()` for an empty pattern list).
  - (b) `,`-continuation for `file[]` setters (TAB-only).
- **ENTER path** (Runtime Autocomplete), two surfaces:
  - the `compose|inline-compose|sequence <file>` positional argument, and
  - the missing-frontmatter-property prompt (`file` / `file[]`).

Both ENTER surfaces reuse the bounded walker with the `*query*` substring
filter **pushed into the walk** so `MAX_CANDIDATES = 500` counts
query-matching files, share one badge/name/description/schema **detail
block**, and share one two-pane `SplitPane` chooser + one `Prose`
confirmation dialog presentation.

## Cross-cutting decisions

- **No `cargo fmt`.** Match surrounding style by hand (`main` is the
  formatting authority — see AGENTS.md).
- **US English** for all symbols and docs.
- **TerminalRenderable contract.** All confirmation / chooser-detail /
  prompt copy flows through `Prose` / `BlockQuote` / `UnorderedList` and
  the relevant `biscuit-tui` widgets — never ad-hoc ANSI strings.
- **`just test` / `just test-l2` / `just lint`** in the `claudine/`
  package area are the validation commands (nextest under the hood).
- Badges `COMPOSE`, `INLINE_COMPOSE`, `SEQUENCE` already exist in
  `lib/src/badges.rs` and are reused verbatim.

## Parallelization map

- Phase 1 is the dependency root for Phases 2–4.
- **Phase 2 (TAB) is independent of Phases 3 & 4 (ENTER)** once Phase 1
  lands — they can proceed in parallel branches.
- Phase 3 and Phase 4 both consume the Phase 1 presentation layer; they
  can also proceed in parallel, but share the chooser/dialog components so
  coordinate on that module.
- Phase 5 is strictly last.

---

## Phase 1 — Shared foundations

Builds the shared primitives both the TAB and ENTER paths depend on. None
of this is user-visible on its own; it is verified by unit tests and by
being consumed in later phases.

**Parallelizable within the phase:** tasks 1.1, 1.2, 1.3, and 1.6 are
independent of each other; 1.4 depends on 1.3; 1.5 is independent.

- [x] **1.1 Walker query-predicate variant.** Add
  `walk_scope_filtered(scope, budget, predicate)` to
  `cli/src/completion/walker.rs` that threads a filename/path substring
  predicate into the existing `ignore::WalkBuilder` loop so the
  `MAX_CANDIDATES` cap counts only predicate-matching entries (early-abort
  on overflow). Refactor `walk_scope` to call the new core so both paths
  route through one entry point (acceptance: "shared bounded walker"). Add
  unit tests asserting (a) the cap counts matches not raw discoveries, (b)
  early-abort reports "more than cap", (c) zero-budget / nonexistent-root
  parity with `walk_scope`.

- [x] **1.2 Default-glob file candidate gatherer.** New module (e.g.
  `cli/src/completion/default_glob.rs`) exposing a function that returns
  markdown candidates under the effective repo root (or CWD tree when no
  repo) honoring `.gitignore` / `.git/info/exclude` / global gitignore,
  excluding every **prompt** directory surfaced by `ScopeSet` (so
  compose-associated prompts never leak into frontmatter `file` values),
  excluding `_`-prefix files/dirs, and respecting `MAX_CANDIDATES`. This is
  the shared backend for the TAB bare-file fallback (2.1) and the ENTER
  `file`/`file[]` property choosers (4.3). Unit-test the prompt-dir
  exclusion and `_`-prefix exclusion explicitly.

- [x] **1.3 Detail-block data model + extractor.** New type (e.g.
  `FileDetail { badge, name, path, description, schema_lines }`) plus an
  extractor that reads `name` / `description` / `$schema` from (a)
  Markdown frontmatter via `darkmatter::Markdown` and (b) **YAML sequence
  files** via top-level keys (badge `SEQUENCE`, treating the YAML doc as
  "frontmatter without a body" per spec §158). Fallbacks: "no
  description", "no schema defined". Lives in the `claudine` lib
  (composition area) so both the operation-file autocomplete and the
  property chooser consume it. Unit-test both Markdown and YAML extraction
  paths including every fallback.

- [x] **1.4 Shared presentation components.** In the CLI layer, build:
  (a) a `Prose`-rendered **confirmation dialog** renderer
  (`<badge> <name>` line, blank line, `BlockQuote` description, `Schema:`
  block via `UnorderedList`, trailing `Use this file? (Y/n)`); (b) a
  **two-pane chooser scaffold** that takes a `SplitPane` with
  `SplitDirection::Auto`, renders a `ChooseOne`/`ChooseMany` list in the
  first rect, and derives the detail rect each frame from
  `active_option()`/`hover()` → the 1.3 detail block → `Prose` →
  `ansi-to-tui` → ratatui `Paragraph` (the bridge documented in
  `biscuit-tui/docs/components/split_pane.md`). Confirm `ansi-to-tui` is
  already a dependency before adding it. Depends on 1.3.

- [x] **1.5 Autocomplete error variants.** Add typed
  `CompositionError` variants for the three ENTER failure modes: no
  matches, over-cap ("narrow your query"), and non-TTY/disabled. Keep them
  distinct so the CLI error walker can render each precisely (acceptance:
  "Autocomplete failure modes … all three are observable"). Wire the
  existing error-rendering path so the over-cap message names the query.

- [x] **1.6 Phase-1 checkpoint validation.** Run `just test` and
  `just lint` in the `claudine/` area. Confirm the new modules compile
  under the workspace and that no existing completion test regressed.

---

## Phase 2 — Shell Completion Extensions (TAB path)

Implements the two TAB-only additions in the spec. **Independent of
Phases 3 & 4** — can be developed on a parallel branch once Phase 1 lands.

- [x] **2.1 Bare-file default-glob fallback.** In
  `cli/src/completion/schema_completion.rs` `file_candidates`, when the
  property's `match(...)` pattern list is empty (bare `file`/`file[]`),
  delegate to the 1.2 default-glob gatherer instead of returning
  `Vec::new()`. Render candidates with the existing `name='relpath'`
  contract and single-quote normalization. Update the
  `property_value_returns_empty_for_file_without_match` test to assert
  the new fallback behavior, and add a positive test that a bare `file`
  property surfaces repo markdown (excluding prompt dirs).

- [x] **2.2 `,`-continuation for `file[]`.** In the setter-value dispatch
  (`engine.rs::run_setter_value` and/or `schema_completion`), when the
  committed value partial for a `file[]` property already contains a
  top-level `,`, split the typed comma-list (trim whitespace, honor
  shell-quoting), exclude those files, and re-open the glob for the next
  single file. TAB-only by construction (the ENTER path never parses
  commas — see spec §119). Add tests in `completion_setter.rs` /
  `compose_schema_cli.rs` covering: first-file completion, trailing-`,`
  re-open, exclusion of already-named files, and a literal-comma filename
  edge case (documented as unsupported for the exclusion set).

- [x] **2.3 Phase-2 checkpoint validation.** Run the
  `completion_*` integration tests via `just test` (and
  `completion_perf` ignored test stays green). Verify via
  `claudine __complete` subprocess tests in `tests/common/completion.rs`
  that a bare `file` schema property now yields default-glob candidates
  and that `file[]` comma-continuation round-trips.

---

## Phase 3 — Runtime Autocomplete: operation file (ENTER path)

The `compose|inline-compose|sequence <file>` positional autocomplete. The
ENTER path reuses the shipped bounded walker verbatim **except** the
`*query*` filter is pushed into the walk.

- [x] **3.1 Hook point + gates.** In the compose/inline-compose/sequence
  runtime, after `resolve_composition_source` (`lib/src/composition/
  resolve.rs`) returns `FileNotFound` (FileReference failed), gate on an
  interactive session: stdin **and** stderr are TTYs (mirror
  `InteractiveSchemaOptions::allowed()` semantics, but note autocomplete
  does **not** consult `prompt_for_missing` or `--silent` per spec §175 —
  only the TTY check applies for the operation-file path). Non-TTY → emit
  the 1.5 non-TTY error and stop. Keep the change surgical: only the
  FileNotFound branch gains autocomplete; every other resolve error path
  is untouched.

- [x] **3.2 Query walk + cap enforcement.** Resolve the same `ScopeSet`
  the TAB path uses (`scopes::resolve_compose_scopes` for the mode) and
  walk it through the 1.1 filtered walker with the user's typed token as
  the `*query*` substring predicate. Enforce: zero query-matches → 1.5
  "no matches" error; query-match count exceeds `MAX_CANDIDATES` → 1.5
  "narrow your query" error (never silent truncation; early-abort reports
  "more than 500"). One `sniff` invocation per run (reuse `ScopeContext`).

- [x] **3.3 Single-match confirmation dialog.** Exactly one query-match →
  render the 1.4 confirmation dialog (no `SplitPane`, no chooser) using
  the 1.3 detail block for that file. `Y`/Enter proceeds with the
  resolved path; `n`/Esc aborts to a typed cancel (non-fatal). Verify the
  `Use this file? (Y/n)` trailer is present and that the path renders as
  an OSC8 link.

- [x] **3.4 Multi-match two-pane chooser.** Two or more query-matches →
  drive `ChooseOne` (single-select — the file argument is ONE file) in a
  `SplitPane` with `SplitDirection::Auto`. The detail pane recomputes from
  `ChooseOneState::active_option()` each frame. Submitting returns the
  selected path to the runtime; Esc aborts. Honor the operation's
  frontmatter contract (compose/inline-compose/sequence filtering already
  lives in `frontmatter::valid_for_mode` — apply it to the candidate set).

- [x] **3.5 YAML sequence candidate detail.** Ensure 3.3/3.4 populate the
  detail block for `.yaml`/`.yml` sequence files from top-level keys
  (badge `SEQUENCE`) via the 1.3 extractor, identical fallbacks to the
  Markdown path (spec §158).

- [x] **3.6 Phase-3 checkpoint validation.** `just test` covering: single
  match → dialog path, multiple matches → chooser path, zero matches →
  error, over-cap → "narrow your query" error, non-TTY → error. Assert
  the FileReference-first happy path is unchanged (existing compose CLI
  tests in `cli/tests/compose_schema_cli.rs` stay green).

---

## Phase 4 — Runtime Autocomplete: frontmatter properties (ENTER path)

Improves the missing-property prompt and adds `file`/`file[]` interactive
collection. **Preserves the existing prompt gate** (TTY × `--silent` ×
`prompt_for_missing`) per spec §182 and acceptance "Prompt gates
preserved".

- [x] **4.1 New `File` interactive shape.** Extend
  `InteractiveShape` (`lib/src/composition/error.rs`) with a `File`
  variant carrying array-ness and the schema's `match(...)` patterns
  (empty patterns ⇒ default glob). Update the schema-validation layer
  (`schema_validation.rs`) to map a `file` property to the single-select
  shape and `file[]` to the multi-select shape, replacing the current
  `Text { format: File }` mapping for file-typed properties. Add unit
  tests at the validation layer for both mappings.

- [x] **4.2 Inline missing-property prompt.** Rework
  `prompt_for_property` in `cli/src/commands/schema_interactive.rs` so it
  no longer consumes the full screen via `run_standalone`; render only
  the space needed. Add the per-type intro statements (string / number /
  boolean / file) exactly as worded in spec §187–195, reinforce min/max
  constraints for string/number, and render the property's schema
  description below the input in dim italics when present. Preserve the
  existing enum/boolean/number/text behaviors (including number
  parse-and-retry).

- [x] **4.3 `file`/`file[]` choosers.** For a `file` property use
  `ChooseOne`; for `file[]` use `ChooseMany`. When more than one candidate
  exists, reuse the 1.4 two-pane `SplitPane`/`SplitDirection::Auto`
  layout with the detail pane derived from `active_option()` (single) or
  `hover()` (multi) each frame. Candidates come from the schema
  `match(...)` globs when present, else the 1.2 default-glob gatherer.
  Resolve the selected value(s) through `FileReference` before returning
  them as JSON (string / array-of-string).

- [x] **4.4 Preserve gates + cancellation.** Confirm the prompt only
  fires when `InteractiveSchemaOptions::allowed()` is true (stdin+stderr
  TTY, `--silent` off, `prompt_for_missing` true) and that user cancel
  still bubbles back as the original `MissingProperties` error so the CLI
  shows the non-TTY remediation block. Add a non-TTY regression test
  asserting no prompt is driven.

- [x] **4.5 Phase-4 checkpoint validation.** `just test` covering each
  interactive shape, the file/file[] chooser paths, min/max reinforcement
  rendering, and the gate-preservation behavior. Verify existing
  `compose_schema_cli.rs` / `sequence_schema.rs` interactive-collection
  tests still pass.

---

## Phase 5 — Integration testing, performance, and drift

Strictly last. Exercises the full feature across real terminals and
locks in the non-functional acceptance criteria.

- [x] **5.1 Terminal-harness coverage.** Using the
  `biscuit-test-harness`, add L2 tests asserting the complete interaction
  matrix and one L3 smoke test for the OS-to-WezTerm Enter path. Assert the
  type-driven chooser (`ChooseOne` for `file`/single-file-arg, `ChooseMany`
  for `file[]`), and the `SplitPane` `SplitDirection::Auto` layout (detail
  right when wider-than-tall, detail above when taller-than-wide). Covers
  acceptance "Type-driven chooser + layout".

- [x] **5.2 Latency assertion.** Extend/reuse the `completion_perf.rs`
  fixture to add an autocomplete (ENTER-path) latency scenario and assert
  p95 stays within the same ~100 ms-class budget as completion (acceptance
  "Latency"). Keep the test `#[ignore]`d per the existing harness
  convention; document the invocation command in the module doc comment.

- [x] **5.3 Contract regression sweep.** Add/extend tests asserting:
  `claudine __complete` still drives dynamic completion; `claudine
  completions <shell>` remains the bootstrap install command; selected
  `@` magic candidates still insert concrete paths **without** the `@`
  sigil; bare `file`/`file[]` resolves to the default glob; comma-
  continuation is TAB-only; YAML `sequence` candidates populate the detail
  block from top-level keys (acceptance criteria sweep).

- [x] **5.4 Drift maintenance.** Update `claudine/docs/topics/
  shell-completions.md` (autocomplete ENTER path, bare-file fallback,
  comma-continuation), the `claudine` skill (`cli-reference.md` /
  `architecture.md` as needed), and `AGENTS.md` only if a workspace-
  layout or repo-wide convention changed (likely no change here). Keep
  edits behavior-scoped per the AGENTS.md comment-quality rules.

- [x] **5.5 Final validation.** Run `just test`, `just test-l2`, `just
  lint`, and `just doctest` in the `claudine/` package area. Confirm
  cross-platform considerations (the walker, FileReference, biscuit-tui,
  and the terminal harness are already cross-platform; no new platform-
  specific code should be introduced). Report any platform gaps
  discovered.

---

## Definition of Done (mirrors spec §200–213)

- Shared bounded walker; both paths route through one entry point;
  ENTER path pushes the query filter into the walk.
- Current completion contract preserved (`__complete`, `completions
  <shell>`, `@`-sigil stripping).
- No `files.prompts.default_glob` config key introduced.
- p95 autocomplete latency within the ~100 ms-class budget.
- All three ENTER failure modes observable (no matches / over-cap
  "narrow your query" / non-TTY).
- Type-driven chooser (`ChooseOne` vs `ChooseMany`) + `SplitPane`
  `SplitDirection::Auto` and interaction behavior verified via L2; one L3
  smoke test verifies OS Enter delivery through WezTerm.
- Bare `file`/`file[]` resolves to the default glob.
- Two presentations: single match → lightweight `Prose` dialog ending in
  `Use this file? (Y/n)`; multiple matches → two-pane chooser+detail.
- YAML `sequence` contract: top-level `sequence` key; `kind: sequence`
  alone rejected; detail block from top-level keys.
- Prompt gates preserved (TTY × `--silent` × `prompt_for_missing`).
- Terminal-rendering contract: all copy via `TerminalRenderable`
  components and `biscuit-tui` widgets.
