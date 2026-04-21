---
phases: 7
starting_phase: 2
start_phase: 2
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/parse_utils.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/src/markdown/reference/errors.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/tests/error_snapshots/page_block.rs
  - darkmatter/lib/tests/error_snapshots/reference.rs
  - darkmatter/lib/tests/error_snapshots/transclusion.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-04-20-better-errors/review-plan-1.md
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/tests/error_snapshots/editor.rs
docs_updated_during_phase_3:
  - darkmatter/features/2026-04-20-better-errors/review-plan-1.md
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
packages:
  - darkmatter
---
# Better Errors — Review 1 Remediation Plan

> Source review: `darkmatter/features/2026-04-20-better-errors/review-1.md`
> Scope: bounded to the `darkmatter` and `biscuit-terminal` package areas.
> The review explicitly touches `biscuit-terminal` (§2.4 `strip_ansi`, §3.1
> `errors::prelude`, §2.1 `as_block_error` stub), so both packages are in-scope.

## Summary

Review 1 confirms the Better Errors feature is production-ready and maps out a
follow-up backlog of design-drift, correctness, and polish items. None are
ship-blockers, but together they close the gap between "good" and "great"
block rendering (richer enrichments per variant), prevent silent registry
drift, remove duplicated helpers, and add a missing end-to-end CLI test.

This plan organises every recommendation in the review into **7 phases**,
ordered so upstream dependencies (biscuit-terminal polish, schema enrichments,
helper consolidation) land before downstream work (CLI test, docs tightening).
Each phase is independently executable by a rust-developer subagent.

### Phase Map

| # | Phase                                                      | Priority |
|:-:|------------------------------------------------------------|:--------:|
| 1 | biscuit-terminal polish (`errors::prelude` + `strip_ansi`) |  P3/P3   |
| 2 | Schema enrichments — transclusion / page-block / reference |    P1    |
| 3 | EditorError restructuring                                  |    P1    |
| 4 | Remaining schema enrichments + partial-enrichment uplifts  |   P1/P2  |
| 5 | Registry drift guard + dead-code trim                      |    P2    |
| 6 | Snapshot coverage gap closure (FrontmatterParse, UrlFetch) |    P3    |
| 7 | CLI end-to-end test + documentation tightening             |   P2/P3  |

### Global conventions for every phase

- **Workspace commands.** Run targeted `cargo` commands scoped to the crates
  touched. Never `cargo build` at the repo root.
- **Rustdoc.** No `# H1` headings inside `///`. Use `##` sections (`Examples`,
  `Returns`, `Errors`, `Panics`, `Safety`, `Notes`) in the canonical order
  from the root `CLAUDE.md`.
- **Lint gate.** Every phase must end with **zero** `cargo clippy` warnings
  across the darkmatter package area (lib, cli, and tests). The review does
  not relax `-D warnings`.
- **Tests.** Each phase adds or updates tests — snapshot tests under
  `darkmatter/lib/tests/error_snapshots/` where a new variant or enrichment
  changes rendered output, unit tests otherwise.
- **No commits from subagents.** Implementers run tests and clippy locally;
  the orchestrator performs the git commit.
- **No drive-by refactors.** Touch only what a given recommendation names.

---

## Phase 1 — biscuit-terminal ergonomic polish

**Addresses review items:** §2.4, §3.1.

### Goal

Collapse the three hand-rolled `strip_ansi` copies onto the canonical
`biscuit_terminal::utils::escape_codes::strip_escape_codes`, and add a tiny
`biscuit_terminal::errors::prelude` module so each `BlockError` impl stops
repeating the same three `use` lines.

### Scope

- `biscuit-terminal/lib/src/errors/prelude.rs` (new)
- `biscuit-terminal/lib/src/errors/mod.rs` (re-export the new prelude)
- `biscuit-terminal/lib/src/errors/block_error.rs` (replace inline
  `strip_ansi` test helper with `strip_escape_codes`)
- `darkmatter/lib/src/markdown/errors/mod.rs` (replace inline `strip_ansi`
  helper with `strip_escape_codes`)
- `darkmatter/lib/tests/error_snapshots/helpers.rs` (replace inline
  `strip_ansi` helper with `strip_escape_codes`)

### Tasks

1. **Create `biscuit-terminal/lib/src/errors/prelude.rs`.**
   - Re-export only the names implementers need inside a `status_block` body:
     ```rust
     pub use super::{BlockError, ErrorHeader, StatusBlockExt, render_with_causes};
     pub use crate::components::status::StatusState;
     pub use crate::components::status_block::StatusBlock;
     pub use crate::terminal::Terminal;
     ```
   - Add a rustdoc summary describing when to use this prelude vs the
     crate-level `biscuit_terminal::prelude`. Honour the rustdoc rules (no
     H1, use `##` sections).
2. **Wire the prelude into `errors/mod.rs`.** Add `pub mod prelude;` and
   `pub use prelude::*` is **not** desired — the prelude is opt-in by import,
   not re-exported into `errors::*`. Update the top-of-file rustdoc to
   mention the prelude.
3. **Replace `strip_ansi` in `biscuit-terminal/lib/src/errors/block_error.rs`.**
   - Delete the `fn strip_ansi` inside the `tests` module (lines 281-295).
   - Import `crate::utils::escape_codes::strip_escape_codes` and update every
     call site inside the module to use it (renaming local `rendered`/`plain`
     variables is unnecessary).
4. **Replace `strip_ansi` in `darkmatter/lib/src/markdown/errors/mod.rs`.**
   - Delete the `fn strip_ansi` inside the `tests` module (lines 114-129).
   - Call `biscuit_terminal::utils::escape_codes::strip_escape_codes` from
     the `render` helper. Prefer importing through the existing
     `biscuit_terminal::prelude` re-export.
5. **Replace `strip_ansi` in `darkmatter/lib/tests/error_snapshots/helpers.rs`.**
   - Keep the public `strip_ansi` function name (downstream modules call it)
     but make the body delegate to `strip_escape_codes`, or rename the
     function and update all call sites in the snapshot modules
     (`ctx_merge.rs`, `deferred_set.rs`, …, `transclusion.rs`) accordingly.
     Delegation is the smaller change; prefer it.
6. **(Optional, within phase scope)** Do NOT migrate existing
   `BlockError` impls to use the new `errors::prelude`. Leave that migration
   for a future ergonomic pass — the review explicitly recommends **creating**
   the module; actually re-importing every impl is out-of-scope because it
   touches 16 files.

### Test coverage

- Existing tests in `biscuit-terminal` and `darkmatter` that call the old
  `strip_ansi` helpers must continue to pass — no new assertions needed.
- Add a single new doc-test (or unit test) on the new `errors::prelude`
  module importing it and building a trivial `StatusBlock` to guarantee the
  re-exports compile.

### Lint gate

`cargo clippy` across `biscuit-terminal` and `darkmatter` (lib + cli +
tests) must emit zero warnings.

### Verification commands

```bash
cargo test -p biscuit-terminal --lib
cargo test -p darkmatter --lib
cargo test -p darkmatter --test error_snapshots
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
cargo clippy -p darkmatter --all-targets -- -D warnings
```

---

## Phase 2 — High-value schema enrichments (Transclusion, PageBlock, Reference)

**Addresses review items:** §1.1 rows for `TransclusionError::CycleDetected`,
`TransclusionError::InvalidReference`, `PageBlockError::UnterminatedBlock`,
and `ReferenceError::ParseDirective`. Review §5 item 1 (P1).

### Goal

Promote four "good" blocks to "great" by adding the structured fields the
tech-design promised. After this phase users see per-hop cycle line numbers,
echoed opening-directive text, and caret-style directive locations.

### Scope

- `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/parser.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/mod.rs` (only if the
  `CycleDetected` construction site needs the new shape)
- `darkmatter/lib/src/markdown/compose/page_blocks/types.rs`
- `darkmatter/lib/src/markdown/compose/page_blocks/parser.rs`
- `darkmatter/lib/src/markdown/reference/errors.rs`
- `darkmatter/lib/src/markdown/reference/mod.rs` (the one
  `ReferenceError::ParseDirective` construction site at line 96)
- `darkmatter/lib/tests/error_snapshots/transclusion.rs`
- `darkmatter/lib/tests/error_snapshots/page_block.rs`
- `darkmatter/lib/tests/error_snapshots/reference.rs`

### Tasks

1. **`TransclusionError::CycleDetected` — per-hop line numbers.**
   - Change the variant field from `chain: Vec<String>` to
     `chain: Vec<(std::path::PathBuf, usize)>` (tech-design §2.3).
   - Update the `TransclusionRuntime::enter` construction site in
     `compose/transclusion/types.rs` (around line 208) so the cycle chain is
     built as `(PathBuf, line)` tuples. The `DependencyNode` currently only
     tracks `id: String`; extend it with the resolved path and the line at
     which the transclusion directive appeared, or thread that context in
     through the caller. Keep the semantics identical — the set of nodes
     treated as a cycle must not change.
   - Update the `#[error(...)]` format string to render the chain in a form
     stable for log output, e.g. `"Transclusion cycle detected: a.md:3 -> b.md:7 -> a.md"`.
   - Update the `BlockError` arm in `types.rs` (around line 407) so the
     body renders each hop as
     `  <index>. <cyan>{path}</cyan> <dim>:line {N}</dim>`. Keep the hint.
   - Update every test construction site (notably
     `darkmatter/lib/src/markdown/errors/mod.rs` lines 146 & 161, snapshot
     tests under `error_snapshots/transclusion.rs` and
     `error_snapshots/markdown_error.rs`, and any other call sites surfaced
     by `rg 'CycleDetected \{'`).
2. **`TransclusionError::InvalidReference` — source context fields.**
   - Extend the variant to
     `InvalidReference { reference: String, line: usize, source_file: std::path::PathBuf, directive_kind: DirectiveKind }`.
     The `DirectiveKind` enum already exists (used by transclusion parsing);
     re-export or declare an alias next to the variant if needed.
   - Update the `#[error(...)]` format string to include the directive kind
     and source file.
   - Propagate the new fields at every construction site (search with
     `rg 'InvalidReference\s*\{'` and `rg 'InvalidReference\s*\('`). If the
     current caller does not know the source file, walk one level up to
     supply it; for parser paths where only the reference string is
     available, `source_file: PathBuf::new()` is **not** acceptable — push
     the context through.
   - Update the `BlockError` body to surface the directive kind and source
     file, preserving the existing hint.
3. **`PageBlockError::UnterminatedBlock` — echo the opening line.**
   - Extend the variant to
     `UnterminatedBlock { line: usize, opening_text: String, file_ends_at_line: usize }`.
   - Update the construction site in
     `darkmatter/lib/src/markdown/compose/page_blocks/parser.rs:94` to
     capture the raw opening directive text from the parsed entry (`entry.0`
     or equivalent; confirm the field by re-reading the parser) and the
     line at which the file ended.
   - Update the `BlockError` arm in
     `compose/page_blocks/types.rs:47` to render both the opened-at line
     and the echoed opening text in the body, and to include the
     file-ends-at line, keeping the hint.
   - Update the inline test in
     `darkmatter/lib/src/markdown/errors/mod.rs:203` and the snapshot in
     `error_snapshots/page_block.rs`.
4. **`ReferenceError::ParseDirective` — directive + caret.**
   - Extend the variant to
     `ParseDirective { line: usize, message: String, source_file: std::path::PathBuf, directive_text: String, caret_col: Option<usize> }`.
   - Update the construction site in
     `darkmatter/lib/src/markdown/reference/mod.rs:96` so each of the
     new fields is populated from the parser's `ParseError` output (add a
     helper that turns the tokenizer's internal column info into a
     `caret_col`).
   - Update the `BlockError` arm in `reference/errors.rs:53` to render:
     - `<dim>Source:</dim> <cyan>{source_file}</cyan>`
     - `<dim>Line:</dim> {line}`
     - `<dim>Directive:</dim>` followed on the next line by
       `  {directive_text}` and — only when `caret_col` is `Some` — a
       caret line `  {spaces}^` beneath it.
   - Update the inline test in
     `darkmatter/lib/src/markdown/errors/mod.rs:251` and the snapshot in
     `error_snapshots/reference.rs`.

### Test coverage

- Each enrichment must add **one new assertion** per snapshot file that
  checks the newly-surfaced token (line number, echoed text, caret).
- Every existing snapshot that constructs one of the enriched variants must
  be updated to pass the new fields (compiler will fail any missed site).
- Ensure `cargo test -p darkmatter --test error_snapshots` passes with the
  enriched assertions.

### Lint gate

Zero `cargo clippy` warnings across `darkmatter` (lib + cli + tests).

### Verification commands

```bash
cargo test -p darkmatter --lib
cargo test -p darkmatter --test error_snapshots
cargo test -p darkmatter-cli
cargo clippy -p darkmatter --all-targets -- -D warnings
```

---

## Phase 3 — `EditorError` restructuring

**Addresses review item:** §1.1 `EditorError` row (last row of table).
Review §5 item 2 (P1).

### Goal

Land the variant reshape the tech-design called for:
`NonZeroExit { code, editor, path }`, `Missing { path }`,
`LaunchFailed { editor, full_command, source }`, and
`Io { operation, source }`. Current shape in `editor/mod.rs:45-65` has none
of these fields.

### Scope

- `darkmatter/lib/src/editor/mod.rs`
- Every downstream caller of `EditorError` constructors:
  - `darkmatter/lib/src/editor/mod.rs` (tests + `launch_editor_on_path`)
  - Any `claudine` callers surfaced by
    `rg 'EditorError::(NonZeroExit|Missing|LaunchFailed|Io)'`
- `darkmatter/lib/tests/error_snapshots/editor.rs`

> The review's `claudine/features/...` grep hits are documentation, not
> source. If any live claudine source files construct these variants, they
> must be updated in the same change; otherwise this stays darkmatter-only.
> Confirm via `rg --type rust 'EditorError::' claudine/` during execution.

### Tasks

1. **Restructure the enum.**
   - `NonZeroExit(i32)` → `NonZeroExit { code: i32, editor: String, path: std::path::PathBuf }`.
   - `Missing` (unit) → `Missing { path: std::path::PathBuf }`.
   - `LaunchFailed { editor, source }` → `LaunchFailed { editor: String, full_command: String, source: std::io::Error }`.
   - `Io(#[from] std::io::Error)` → `Io { operation: &'static str, source: std::io::Error }`.
     Because this breaks the `#[from]` conversion, drop the `#[from]`
     attribute and construct `EditorError::Io { operation, source }`
     explicitly at every internal call site.
   - Update every `#[error("...")]` format string to render the new fields.
2. **Update construction sites.**
   - `launch_editor_on_path` (line 175):
     `EditorError::LaunchFailed { editor: editor_cmd.clone(), full_command: /* built Command as "bin args..." */, source }`.
     Build `full_command` from the same `bin`, `parts`, and
     `wait_args_for_editor` values used to assemble the `Command`; do not
     include the final `path` argument so the string stays stable across
     runs (or include it — callers decide, document the choice in the
     rustdoc).
   - Line 181: `EditorError::NonZeroExit { code: status.code().unwrap_or(-1), editor: editor_cmd, path: path.to_path_buf() }`.
   - Line 198: `EditorError::Missing { path: path.to_path_buf() }`.
   - All `?` sites that previously relied on `#[from] std::io::Error` must
     construct `EditorError::Io { operation: "<noun-phrase>", source: e }`.
     Operation strings: `"read temp file"`, `"create temp file"`,
     `"persist temp file"`, etc. — specific to each call site.
   - Update the `resolve_editor_command` path if needed.
3. **Update `BlockError` impl** (lines 76-117) to render the new fields:
   - `NonZeroExit` body shows `<dim>Editor:</dim> {editor}`,
     `<dim>Path:</dim> {path}`, and `<dim>Exit code:</dim> {code}`.
   - `Missing` body shows `<dim>Path:</dim> {path}`.
   - `LaunchFailed` body shows editor, full-command, kind, and source.
   - `Io` body shows `<dim>Operation:</dim> {operation}` plus the existing
     kind/source content.
4. **Fix test fixtures.**
   - In-module tests (lines 418 & 435) and snapshot tests in
     `error_snapshots/editor.rs` must be updated to the new shape. Every
     assertion should gain a check for at least one new field (path,
     operation, or full-command) so we lock the new rendering in place.
5. **Check downstream consumers.** Run `rg 'EditorError::' claudine/` and
   update any source (not documentation) that constructs these variants.

### Test coverage

- Update `error_snapshots/editor.rs` fixtures to pass new fields.
- Add at least one new `assert_contains_all` line per variant asserting the
  newly-surfaced token.
- Confirm `cargo test -p darkmatter --lib --all-targets` passes (the
  `editor::mod::tests` module has `#[cfg(test)]` tests that use these
  constructors).

### Lint gate

Zero `cargo clippy` warnings across `darkmatter` (and `claudine` if any
source changes land there).

### Verification commands

```bash
cargo test -p darkmatter --lib
cargo test -p darkmatter --test error_snapshots
cargo test -p claudine --all-targets   # only if claudine sources were touched
cargo clippy -p darkmatter --all-targets -- -D warnings
```

---

## Phase 4 — Remaining schema enrichments and partial-enrichment uplifts

**Addresses review items:** remaining §1.1 rows (`ConditionError::Parse`,
`NormalizationError::ValidationFailed`, `LinkError`/`ImageRefError::MalformedMarkdown`,
`MermaidThemeError::InvalidJson`) plus §1.2 rows (`TocLinkingError::InvalidCleanupService`
descriptions, `StylesheetError::PropertyValueTypeMismatch` per-property examples,
`DeferredSetError::InvalidAssignment` line). Review §5 is silent on a single
priority for these — grouped here because they are independent and small.

### Goal

Close every remaining schema-drift recommendation. After this phase all
"reality" rows in the review's §1.1 and §1.2 tables match the tech-design.

### Scope

- `darkmatter/lib/src/markdown/compose/conditions.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs` (consumer
  of `ConditionError::Parse`)
- `darkmatter/lib/src/markdown/normalize/types.rs`
- `darkmatter/lib/src/render/link.rs`
- `darkmatter/lib/src/render/image_ref.rs`
- `darkmatter/lib/src/mermaid/theme.rs`
- `darkmatter/lib/src/markdown/compose/toc_linking/types.rs`
- `darkmatter/lib/src/render/stylesheet.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/types.rs` (DeferredSetError)
- `darkmatter/lib/tests/error_snapshots/{condition,normalization,link,image_ref,mermaid_theme,toc_linking,stylesheet,deferred_set}.rs`

### Tasks

1. **`ConditionError::Parse` — caret span.**
   - Add `span: std::ops::Range<usize>` to the variant.
   - Update the construction site in `compose/conditions.rs:68` and
     `compose/transclusion/conditions.rs:22` to extract the span from the
     parser's `ParseError`. When the underlying parser does not yield a
     span, use `span: 0..expr.len()` and note this fallback in a rustdoc
     note on the variant.
   - Update the `BlockError` arm in `compose/conditions.rs:43` to render
     the expression with a caret line underneath pointing at `span.start`,
     using the same `  ^` pattern as Phase 2 §4.
   - Update inline tests (`markdown/errors/mod.rs:212`) and the snapshot in
     `error_snapshots/condition.rs`.
2. **`NormalizationError::ValidationFailed(String)` →
   `Vec<StructureIssue>`.**
   - `StructureIssue` already exists in `normalize/types.rs` (review
     confirms). Change the variant and update the `#[error("...")]` format
     string to summarise the count (e.g. `"Validation failed: {N} issue(s)"`).
   - Update the `BlockError` arm to render each issue as a bulleted line.
   - Update every construction site (`rg 'ValidationFailed\('`).
   - Snapshot test updated accordingly.
3. **`LinkError::MalformedMarkdown` and `ImageRefError::MalformedMarkdown`
   — caret + input echo.**
   - Extend both variants to
     `MalformedMarkdown { message: String, input: Option<String>, caret: Option<usize> }`.
   - Preserve source compatibility via a `From<&str>` (or a named
     constructor `malformed(message: &str) -> Self` that fills `input` and
     `caret` with `None`).
   - Update every construction site (both types).
   - Update `BlockError` impls to render `input` + caret when present.
   - Update snapshot fixtures to cover both `Some(input)` and `None` cases.
4. **`MermaidThemeError::InvalidJson` — capture snippet + position.**
   - Extend the variant to
     `InvalidJson { snippet: String, line: usize, column: usize, source: serde_json::Error }`
     (or similar). `serde_json::Error` exposes `line()` and `column()`; use
     them. The snippet is the offending substring — callers usually have
     the full JSON buffer, so add a helper that extracts a windowed
     snippet (up to 200 chars around the error site).
   - Update every construction site in `mermaid/theme.rs` and any other
     caller surfaced by `rg 'MermaidThemeError::InvalidJson'`.
   - Update the `BlockError` arm and snapshot tests.
5. **`TocLinkingError::InvalidCleanupService` — descriptions.**
   - Add a `CleanupServiceDescriptor` struct (or tuple) with `name` and
     `description` next to `CleanupService::all()`. Implementation choice:
     keep `CleanupService::all()` as the source of names and add a sibling
     `CleanupService::describe()` returning `&'static str` per variant so
     the block can enumerate `name — description` pairs.
   - Update the `BlockError` arm in `toc_linking/types.rs:60` to include the
     description column.
   - Update the snapshot to assert at least one description token appears.
6. **`StylesheetError::PropertyValueTypeMismatch` — per-property examples.**
   - Add `CssProp::expected_kind()` (if missing) and
     `examples_for_property(prop: &CssProp) -> &'static str` helpers
     alongside the existing `example_for_kind`.
   - Switch the `BlockError` arm that currently calls `example_for_kind`
     (review cites `render/stylesheet.rs:161`) to use the per-property
     helper, falling back to the kind-based helper for `CssProp::Custom`.
   - Update the snapshot to assert a property-specific example token
     (e.g. `16px` when `font-size` is the property in the fixture).
7. **`DeferredSetError::InvalidAssignment` — carry line.**
   - Review §1.2 notes users rarely hit `DeferredSetError` directly. Land
     this as a small additive change: extend the variant fields to
     `{ raw: String, reason: String, line: usize }` (the line is
     currently lost before this level per the review's own note) and update
     the single upstream site that promotes this error to
     `TransclusionError::InvalidFrontmatterAssignment` to thread the line
     through.
   - Update the `BlockError` arm in
     `compose/transclusion/types.rs:96-113` to render the line.
   - Update the snapshot in `error_snapshots/deferred_set.rs`.

### Test coverage

- One new assertion per enriched variant added to its snapshot file.
- All pre-existing snapshot tests must still pass — compilation errors at
  construction sites will surface missed updates.

### Lint gate

Zero `cargo clippy` warnings across `darkmatter` lib + cli + tests.

### Verification commands

```bash
cargo test -p darkmatter --lib
cargo test -p darkmatter --test error_snapshots
cargo clippy -p darkmatter --all-targets -- -D warnings
```

---

## Phase 5 — Registry drift guard and dead-code trim

**Addresses review items:** §2.1 (`as_terminal_block_error` stub), §2.2
(`MarkdownError::block_source`/`report_block_error` override), §2.3
(`status_block` term parameter — observation only), §2.5 (`as_block_error`
registry drift), §2.6 (`DeferredSetError` unreachable via CLI). Review §5
items 4 (P2) and 7 (P3).

### Goal

Stop registry drift before it bites, and cut genuinely dead code.

Per the review, §2.3 is an observation only ("not a regression; just an
observation for future trait evolution") — it is **not** actionable in this
plan and is deferred to a future trait evolution. It is recorded here so the
orchestrator can confirm every review item was triaged.

### Scope

- `darkmatter/lib/src/markdown/errors/mod.rs`
- `darkmatter/lib/src/markdown/types.rs` (the `report_block_error` override)
- `darkmatter/cli/src/main.rs` (the `as_terminal_block_error` fallback call)
- `biscuit-terminal/lib/src/errors/block_error.rs` (decision on the stub)
- `darkmatter/lib/tests/` (new drift-guard test file)

### Tasks

1. **Drift guard test for the darkmatter downcast registry (§2.5).**
   - Create `darkmatter/lib/tests/as_block_error_registry.rs` (new
     integration-test binary, or add to the existing
     `error_snapshots` binary — pick the former to keep it isolated).
   - Implement a compile-time or runtime assertion that every
     `impl BlockError for X` in `darkmatter/lib/src` has a matching arm in
     `as_block_error`. Preferred approach: parse the crate's source files
     at test time with a simple regex walker, collect every type name `X`
     from the pattern `impl[^{]*BlockError[^{]*for (?P<name>[A-Z][A-Za-z0-9_]*)`,
     and assert each name appears in `as_block_error`'s body (read
     `darkmatter/lib/src/markdown/errors/mod.rs` as a string).
   - Add a helpful failure message pointing at the missing type and the
     file `darkmatter/lib/src/markdown/errors/mod.rs` so future authors
     know where to add the downcast arm.
2. **Trim `MarkdownError::report_block_error` override (§2.2).**
   - Since every leaf variant currently returns `None` from `block_source`,
     the override is a no-op. Delete it — the default
     `status_block(term).render(term)` is equivalent — **only if** every
     call site in the tree still produces the same rendered output.
   - Run the snapshot tests and the inline test
     `markdown_error_delegates_transclusion_block_without_caused_by`
     afterwards to confirm nothing changed.
   - Leave `block_source` as-is (it still documents the delegation
     intent).
3. **Decide the fate of `biscuit_terminal::errors::as_block_error` stub
   (§2.1).**
   - Pick option 1 from the review (document explicit per-crate registry
     pattern) to keep the surface area stable today.
   - Update the rustdoc on `as_block_error` (currently lines 178-192) to
     say plainly: "This function exists as a single detection seam for
     callers; each crate that implements `BlockError` must expose its own
     downcast registry (e.g. `darkmatter::markdown::errors::as_block_error`).
     This helper returns `None` unconditionally so that a downstream
     `.or_else(...)` fallback remains a valid extension point."
   - Keep the prelude export (the CLI chains it via `.or_else(...)`).
   - **Do not** introduce the `inventory` crate (option 2) in this phase —
     it widens dependency surface without a current consumer.
4. **Annotate the CLI `as_terminal_block_error` fallback.**
   - Add a one-line comment at `darkmatter/cli/src/main.rs:72-76` pointing
     at the docs so the dead-today-but-intentional call is obvious.
5. **Dead `DeferredSetError` downcast arm (§2.6).**
   - Leave the arm in place but add a comment in
     `markdown/errors/mod.rs` (near the `DeferredSetError` arm, currently
     line 90) explaining that it is kept for library consumers that call
     the parser directly — the CLI path cannot reach it because the
     wrapping promotion in `transclusion/types.rs:324-336` always runs
     first. No code change; docs only.

### Test coverage

- New `as_block_error_registry` drift-guard test must fail when a
  hypothetical new `impl BlockError for DummyError` is added without a
  matching arm. Verify by temporarily adding a dummy impl locally, running
  the test, confirming failure, removing the dummy.
- Existing snapshot tests must continue to pass after removing the
  `MarkdownError::report_block_error` override.

### Lint gate

Zero `cargo clippy` warnings across `biscuit-terminal` and `darkmatter`.

### Verification commands

```bash
cargo test -p darkmatter --lib
cargo test -p darkmatter --test error_snapshots
cargo test -p darkmatter --test as_block_error_registry
cargo test -p biscuit-terminal --lib
cargo test -p darkmatter-cli
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
cargo clippy -p darkmatter --all-targets -- -D warnings
```

---

## Phase 6 — Snapshot coverage gap closure

**Addresses review item:** §2.7. Review §5 item 5 (P3).

### Goal

Back the coverage claim in `error_snapshots/markdown_error.rs:10-15,90-92`
by adding actual unit tests in `darkmatter/lib/src/markdown/errors/blocks.rs`
for the currently uncovered leaf renderers (`frontmatter_parse_block`,
`url_fetch_block`, plus the other six helpers for completeness). The review
accepts either the unit-test approach or adding `serde_yaml_ng` as a
dev-dep — unit tests are smaller and keep the dev-dep surface narrow.

### Scope

- `darkmatter/lib/src/markdown/errors/blocks.rs`
- `darkmatter/lib/tests/error_snapshots/markdown_error.rs` (update the NOTE
  comments to point at the new unit tests)

### Tasks

1. **Add a `#[cfg(test)] mod tests` to `blocks.rs`.**
   - `frontmatter_parse_block`: construct a minimal `YamlParseError` by
     parsing invalid YAML inside the test using `biscuit_file::YamlParseError`'s
     public constructor — if one exists. If not, add a small helper that
     uses whatever constructor `biscuit_file` exposes; if none is
     ergonomic, mark this single helper as covered by a smoke test that
     instantiates a dummy type implementing the same `Display` shape and
     invokes the helper with a trait-object cast. Document the chosen path
     in a rustdoc `## Notes` section.
   - `url_fetch_block`: `reqwest::Error` is effectively unconstructable
     without firing a request. Make the test perform a short-circuited
     `reqwest::Client::builder().build()` call only if that path can
     produce a cheap builder error (e.g. invalid TLS config). If not,
     document the limitation with a `/// ## Notes` block and add a smoke
     test using a test-double that implements `std::error::Error` and the
     shape the helper inspects. (The review explicitly accepts this
     compromise.)
   - Cover the other six helpers (`file_load_block`, `frontmatter_merge_block`,
     `theme_load_block`, `ast_parse_block`, `invalid_line_range_block`,
     `serialization_block`, `transform_block`) with one assertion each —
     each helper takes easy-to-construct inputs.
2. **Update the NOTE comments in
   `error_snapshots/markdown_error.rs:10-15,90-92`** to point readers at
   the new unit tests in `blocks.rs`.

### Test coverage

- At least one unit test per helper function in `blocks.rs`.
- Each test runs the helper at 80 columns via a local
  `Terminal::new_optimistic(80)` and asserts the resulting block, after
  `strip_escape_codes`, contains the expected header tokens.

### Lint gate

Zero `cargo clippy` warnings across `darkmatter`.

### Verification commands

```bash
cargo test -p darkmatter --lib markdown::errors::blocks
cargo test -p darkmatter --test error_snapshots
cargo clippy -p darkmatter --all-targets -- -D warnings
```

---

## Phase 7 — CLI end-to-end test + documentation tightening

**Addresses review items:** §2.8 (no end-to-end CLI test) and §4
(documentation gotcha). Review §5 items 3 (P2) and — partially — 6 (P3,
remaining polish beyond Phase 1).

### Goal

Guard the CLI block rendering path end-to-end and add a clear gotcha note
in both docs about the mandatory `as_block_error` registry update.

### Scope

- `darkmatter/cli/tests/cli.rs` (new test cases)
- `darkmatter/cli/tests/fixtures/` (new fixture files if needed; create if
  absent)
- `darkmatter/docs/error-rendering.md`
- `biscuit-terminal/README.md`

### Tasks

1. **CLI end-to-end test.**
   - Add a test to `darkmatter/cli/tests/cli.rs` that writes a markdown
     document containing a transclusion cycle (or any other clearly broken
     input) to a tempfile via the existing `md_file` helper, runs
     `md <path>`, and asserts:
     - the exit code is non-zero,
     - `stderr` contains the error type name (e.g. `TransclusionError`),
     - `stderr` contains the human-readable summary (e.g. `cycle detected`),
     - `stderr` contains a hint-tagged token from the rendered block.
   - Add a **second** test running the same invocation with stdin/stdout
     piped (non-TTY) and assert the output is still readable plain text
     (optimistic render).
   - Use the existing `md_cmd()` helper so no new harness is required.
2. **Docs gotcha for registry drift (§4).**
   - In `darkmatter/docs/error-rendering.md` under "Adding a new error
     enum", add a bold **Required** callout that updating `as_block_error`
     in `markdown/errors/mod.rs` is mandatory; without it the new enum
     silently falls back to the `Display` chain. Reference the new Phase 5
     drift-guard test as the backstop.
   - In `biscuit-terminal/README.md` under the `BlockError` section (after
     the helpers list), add a short paragraph noting that each crate is
     responsible for its own downcast registry and link to darkmatter's
     example in `darkmatter/lib/src/markdown/errors/mod.rs`.
3. **Verify the coverage table in
   `darkmatter/docs/error-rendering.md:138-162`** still matches reality
   after Phases 2-4 (the review says it was correct at time of review; the
   enrichments change field counts but not variant counts). If variant
   counts shift, update the table.

### Test coverage

- `darkmatter-cli` test suite grows by at least two cases.
- Docs changes are prose-only; covered by passing existing doctests (if
  any).

### Lint gate

Zero `cargo clippy` warnings across `darkmatter` and `biscuit-terminal`.

### Verification commands

```bash
cargo test -p darkmatter-cli
cargo test -p biscuit-terminal --doc
cargo test -p darkmatter --doc
cargo clippy -p darkmatter --all-targets -- -D warnings
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
```

---

## Recommendation → Phase Mapping

Every actionable item in the review is listed below with its target phase.
Observation-only items are marked and carried to Phase 5 for triage.

| Review §  | Recommendation                                                      | Phase |
|-----------|---------------------------------------------------------------------|:-----:|
| §1.1      | `TransclusionError::CycleDetected` per-hop lines                    |   2   |
| §1.1      | `TransclusionError::InvalidReference` source context                |   2   |
| §1.1      | `ConditionError::Parse` span/caret                                  |   4   |
| §1.1      | `PageBlockError::UnterminatedBlock` opening text + EOF line         |   2   |
| §1.1      | `ReferenceError::ParseDirective` caret + source                     |   2   |
| §1.1      | `NormalizationError::ValidationFailed` → `Vec<StructureIssue>`      |   4   |
| §1.1      | `LinkError` / `ImageRefError::MalformedMarkdown` caret + input      |   4   |
| §1.1      | `MermaidThemeError::InvalidJson` snippet capture                    |   4   |
| §1.1      | `EditorError` restructuring                                         |   3   |
| §1.2      | `TocLinkingError::InvalidCleanupService` descriptions               |   4   |
| §1.2      | `StylesheetError::PropertyValueTypeMismatch` per-property examples  |   4   |
| §1.2      | `DeferredSetError::InvalidAssignment` line field                    |   4   |
| §2.1      | `biscuit_terminal::errors::as_block_error` stub — documentation     |   5   |
| §2.2      | `MarkdownError::block_source`/`report_block_error` dead override    |   5   |
| §2.3      | `status_block(&self, term)` observation (no action)                 |   5   |
| §2.4      | Collapse three `strip_ansi` duplicates                              |   1   |
| §2.5      | `as_block_error` registry drift guard                               |   5   |
| §2.6      | `DeferredSetError` CLI-unreachable — documentation                  |   5   |
| §2.7      | Snapshot coverage gaps — `FrontmatterParse`, `UrlFetch` + 6 others  |   6   |
| §2.8      | End-to-end CLI rendering test                                       |   7   |
| §3.1      | `biscuit_terminal::errors::prelude` convenience module              |   1   |
| §3.2      | Static hint strings via `Cow<'static, str>`                         |  DEFER|
| §3.3      | `truncate_output` single-pass rewrite                               |  DEFER|
| §3.4      | `StatusBlock::error()` / `::warning()` constructor helpers          |  DEFER|
| §4        | Docs gotcha for `as_block_error` registry update                    |   7   |
| §4        | Coverage-table sanity check after Phases 2-4                        |   7   |

### Deferred / merged items

- **§3.2 static hint strings** — deferred. The review itself marks it
  "Minor; only matters in hot paths"; no hot error path exists. Revisit if
  `darkmatter`'s error-render benchmarks ever flag it.
- **§3.3 `truncate_output` rewrite** — deferred. Review labels it
  "optional" and the code path only fires on shell-exec failures where
  allocation cost is irrelevant compared to the subprocess already run.
- **§3.4 `StatusBlock::error()` / `StatusBlock::warning()`** — deferred.
  The review itself says "In-scope for biscuit-terminal's API polish pass,
  out-of-scope for this feature." Left to a future biscuit-terminal API
  sweep.
- **§2.3 trait-signature simplification** — deferred as explicitly
  out-of-scope per the review ("it would be a breaking change so leave
  it"). Recorded in Phase 5 for traceability.

No recommendations were merged into unrelated phases — each action maps to
exactly one phase, and observation-only items are flagged rather than
silently dropped.

### Coverage checklist

- [x] Every §1.1 row mapped.
- [x] Every §1.2 row mapped.
- [x] Every §2.x row mapped (including observation-only §2.3).
- [x] Every §3.x row mapped (implemented or deferred with justification).
- [x] Every §4 doc comment mapped.
- [x] Every §5 P1/P2/P3 follow-up ticket covered.
- [x] Scope is bounded to `biscuit-terminal` and `darkmatter` package
      areas; no unrelated crates touched (the `claudine` check in Phase 3
      is gated on actual source references surfaced during execution).
