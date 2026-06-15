---
created: 2026-06-14
reviewed: true
status: completed
area: darkmatter
source_review: darkmatter/reviews/2026-06-14-summary-and-suggest/review.md
suggestions: 9
---

# Review Follow-up: Rendering and Compose Maintenance

## Problem Statement

The 2026-06-14 Darkmatter review identified nine follow-up items after a large
two-week push across rendering, composition, schema validation, hashing, remote
references, file-link directives, and disclosure blocks. None of the items
requires a public behavior change. The theme is maintenance: remove accidental
artifacts, turn debug-only coverage into real assertions, reduce drift-prone
metadata, and create clearer boundaries before the next feature adds more
weight to already large modules.

This fix batches the suggestions into a small number of deliberate changes. The
goal is to lower maintenance risk without changing CLI contracts, rendering
output, compose semantics, or style-frontmatter behavior.

## Goals

1. Remove checked-in editor artifacts from the Darkmatter package area.
2. Replace or delete assertion-free debug tests and debug-only dumps related to
   frontmatter interpolation.
3. Keep CLI command handling split by command family, with `commands.rs` acting
   as the dispatcher and shared helper home rather than the owner of every
   command implementation.
4. Centralize compose operation metadata so operation count, index, phase,
   default order, display labels, and perf mapping cannot silently diverge.
5. Align compose phase documentation with the live four-phase pipeline:
   Inline Pre, Transclusion, Inline Post, and Finalization.
6. Use one Markdown serialization helper for `md code-block --output markdown`
   and `md code-block --output markdown --show`.
7. Use one highlight-range parser for fenced code metadata and CLI
   `--highlight`, while preserving clear CLI-facing errors.
8. Resolve themes only for output formats that need them.
9. Establish extraction guardrails for the render fold and compose module so
   the next feature adds a boundary instead of another large in-file section.

## Non-Goals

- No public CLI flag, subcommand, or output-format change.
- No compose operation reorder.
- No rendering output change except where tests reveal accidental divergence
  between duplicate code paths.
- No speculative split of `render_tree/fold.rs` or `compose/mod.rs` without a
  concrete feature or bug fix that naturally creates the boundary.
- No broad comment cleanup outside the symbols touched by this fix.

## Current State

The source review was written against a snapshot where
`darkmatter/cli/src/commands.rs` still owned most command implementations. The
current worktree already contains extracted command modules:

- `darkmatter/cli/src/commands/compose.rs`
- `darkmatter/cli/src/commands/hash.rs`
- `darkmatter/cli/src/commands/frontmatter.rs`
- `darkmatter/cli/src/commands/code_block.rs`
- `darkmatter/cli/src/commands/schema/*`

The fix should treat that extraction as the target shape and validate it rather
than redoing it. Remaining work still exists around duplicated code-block
Markdown serialization, CLI highlight parser drift, early theme resolution in
rendering, compose metadata drift, and god-file guardrails.

## Design

### Design Decisions From Review

- Treat this as a behavior-preserving maintenance spec. If implementation
  discovers a user-visible behavior change is required, stop and update this
  spec or open a follow-up instead of folding the behavior change into the
  cleanup.
- The compose operation descriptor table is the source of truth for
  `ComposeOperation` metadata only. It should not force schema validation,
  effective-state build, or transclusion sub-step perf metrics to masquerade as
  `ComposeOperation` variants.
- `md code-block --output markdown` should emit safe Markdown, not merely the
  shortest text that works for ordinary code. The serialization helper owns
  fence selection and must choose a fence longer than any same-character fence
  run in the code body.
- Lazy theme resolution applies to both render and compose commands. Any path
  that returns raw Markdown or JSON should not construct theme state.

### 1. Artifact Cleanup

Delete these files if present:

- `darkmatter/cli/src/commands.rs.orig`
- `darkmatter/lib/src/markdown/compose/shell_expansion/store.rs.bak`

They are not package sources and should not be referenced by builds, tests, or
documentation. Verification is a direct path check plus `rg` to ensure no live
code or docs mention them.

### 2. Debug-Only Frontmatter Coverage

Search the compose module and frontmatter interpolation module for debug-only
test behavior:

- env-gated `DM_DEBUG` output
- `eprintln!` or `dbg!` diagnostics in tests
- tests that only print composed state and do not assert behavior

If the repro still describes an active interpolation bug, convert it into a
regression test with explicit assertions on frontmatter values, warnings, and
error behavior. If the bug is already covered by the interpolation error
handling fix, delete the repro.

The production code may keep structured `tracing::debug!` instrumentation where
it carries runtime diagnostics, but tests should not depend on ad hoc terminal
output.

### 3. CLI Command Module Boundaries

Keep `darkmatter/cli/src/commands.rs` responsible for:

- top-level `CliCommand` dispatch
- shared command helpers used by multiple modules
- small single-purpose commands whose extraction would add indirection

Command families with substantial implementation should live under
`darkmatter/cli/src/commands/`:

- `compose.rs`: compose execution, compose argument classification, remote
  read config, validation allow flags, shell/perf reports
- `hash.rs`: `md hash`
- `frontmatter.rs`: get/set/rm/edit frontmatter commands
- `code_block.rs`: `md code-block`
- `schema/*`: existing schema validate/detect/about layout

The dispatcher should import these modules and call their public `run_*`
entrypoints. Shared helpers that are only needed by one module should move into
that module. Shared helpers needed by several modules should remain
`pub(crate)` in `commands.rs` or move to a deliberately named helper module if
the helper group becomes coherent.

Success criteria:

- `commands.rs` remains mostly dispatch and shared helpers.
- No command implementation is duplicated between `commands.rs` and a module.
- The extracted modules do not form circular conceptual dependencies.

### 4. Compose Operation Descriptor Table

Introduce a single descriptor table in
`darkmatter/lib/src/markdown/compose/types.rs` as the source of truth for
operation metadata. This table covers the operation enum, stable operation
indices, phase membership, default enablement, display labels, and operation
level perf mapping.

Suggested shape:

```rust
pub struct ComposeOperationDescriptor {
    pub operation: ComposeOperation,
    pub index: usize,
    pub phase: ComposePhase,
    pub default_enabled: bool,
    pub label: &'static str,
    pub perf_kind: Option<perf::PerfMetricKind>,
}
```

The exact visibility and `perf` dependency direction should follow the existing
module boundaries. If importing `perf::PerfMetricKind` into `types.rs` creates
an undesirable dependency, use a small local enum or provide perf mapping beside
the table through an exhaustive `match` generated from the same ordering. The
important invariant is that adding an operation requires touching one obvious
metadata definition, not several independent lists.

Do not expand `ComposeOperation` just to make this table line up with every
perf metric. `SchemaValidation`, `EffectiveStateBuild`,
`TransclusionParse`, `TransclusionPrepare`, `TransclusionResolve`, and
`TransclusionApply` are perf/report stages, not individually toggled compose
operations. Keep those metrics in `perf.rs` unless a separate feature turns one
of them into a real user-toggleable operation.

Derived APIs should include:

- `ComposeOperation::COUNT`
- `ComposeOperation::index()`
- `ComposeOperation::phase()`
- `ComposeOperation::default_order()`
- display or label helpers used by reports
- perf mapping used by the runner

Guardrails:

- Preserve stable operation indices unless a breaking change is explicitly
  approved. `ComposeOperationSet` depends on fixed indices.
- Keep `ComposeOperation::COUNT` derived from the descriptor table length, or
  make the invariant test fail if a manually maintained count drifts.
- Add a test that verifies descriptor indices are contiguous, unique, and match
  `ComposeOperation::COUNT`.
- Add a test that verifies `default_order()` is descriptor order filtered by
  `default_enabled`.
- Add an exhaustive mapping test or compile-time match so every operation either
  maps to a perf metric or is explicitly documented as having no operation-level
  metric.

### 5. Compose Phase Documentation

Treat the code as ground truth: the pipeline has four phases, with
`Finalization` as the root-only phase after Inline Post.

Update all relevant docs and comments touched by this fix:

- `darkmatter/lib/src/markdown/compose/mod.rs` module docs
- the `run_compose_pipeline_internal` phase comment
- `darkmatter/docs/topics/context-variables.md` or compose topic docs if they
  list the phases
- `.claude/skills/darkmatter/SKILL.md` if the compose pipeline summary drifts

The documentation should mention schema validation as a pre-operation stage
that runs after frontmatter interpolation and before frontmatter shell
expansion, because it is not represented as a `ComposeOperation` variant.
Also update stale comments that still say operations are grouped into three
phases; `ComposePhase::Finalization` is a first-class phase even though it only
runs on the root document.

### 6. CodeBlock Markdown Serialization Helper

Create one helper for `md code-block --output markdown` serialization. The
helper should be close to the CLI code-block implementation unless the library
already has an appropriate public renderer.

Suggested private helper:

```rust
fn code_block_markdown(block: &CodeBlock) -> String
```

It should return a complete fenced block with a trailing newline. Both stdout
and `--show` should use this helper, and stdout should print the exact helper
string without adding a second newline. The helper owns:

- fence info assembly
- title quoting and escaping
- `line-numbering=true`
- `highlight=...`
- safe fence selection when the code body contains backtick fences
- code body placement
- trailing newline policy

Add a focused CLI module test for at least:

- language only
- quoted title containing a double quote
- line numbering
- highlight ranges
- empty language with metadata
- code containing triple backticks
- stdout and `--show` artifact content using the same helper output

### 7. Shared Highlight Parser

Move highlight-range parsing into the library surface that already owns code
fence metadata, likely near `darkmatter::markdown::dsl::HighlightSpec` and
`parse_code_info`.

Suggested API:

```rust
pub fn parse_highlight_spec(raw: &str) -> Result<HighlightSpec, HighlightSpecParseError>
```

Requirements:

- `parse_code_info` uses the helper for `highlight=...`.
- `md code-block --highlight` uses the helper and maps the structured error to
  the existing clear CLI message.
- The parser accepts the current grammar exactly: comma-separated line numbers
  and inclusive ranges such as `1,4-6`.
- Empty comma segments keep the current behavior and are ignored.
- Invalid ranges remain fatal on the CLI path.
- Existing fenced-code behavior is preserved, including how malformed metadata
  is surfaced by render preflight.
- The shared error type should retain enough detail to reconstruct the current
  `MarkdownError::InvalidLineRange(...)` messages for fenced-code parsing while
  still allowing the CLI to add its `Invalid --highlight range: ...` context.
- The CLI-local parser and its drift-prone documentation should be deleted
  after the shared helper is wired.

Tests should cover the helper directly and at least one CLI-facing invalid
range path.

### 8. Lazy Theme Resolution

Avoid resolving themes for output formats that do not need prose or code
themes.

For `run_render`:

- `OutputFormat::Markdown` should not call `ResolvedTheme::from_cli`.
- non-TTY `OutputFormat::Auto` should not call `ResolvedTheme::from_cli`.
- `OutputFormat::Json` should not call `ResolvedTheme::from_cli`.
- terminal rendering, `MarkdownPlus`, and `Html` should keep using one
  resolved theme derived from the same `Terminal` color mode.

For `run_compose`:

- `OutputFormat::Auto` and `OutputFormat::Markdown` should not call
  `ResolvedTheme::from_cli`.
- `OutputFormat::Json` should not call `ResolvedTheme::from_cli`.
- `OutputFormat::MarkdownPlus` and `OutputFormat::Html` should resolve the
  theme only inside those match arms.
- Compose output must keep the current `markdown-plus` behavior: composed
  disclosure blocks route through the MarkdownPlus fold and emit
  `<details>/<summary>` inline HTML.

This is a small performance and side-effect containment change. It should not
alter rendered output for terminal, MarkdownPlus, or HTML. Be careful not to
reintroduce dual-source color mode drift: any output arm that needs both a page
theme and a code-block theme should derive them from one `ResolvedTheme` value.

### 9. God-File Guardrails

Do not split `darkmatter/lib/src/markdown/render_tree/fold.rs` or
`darkmatter/lib/src/markdown/compose/mod.rs` merely to reduce line count. Split
only when a concrete change introduces a natural boundary.

Adopt these extraction rules:

- A new block extension should normally add or extend a dedicated module under
  `markdown/render_tree/` instead of adding a large section to `fold.rs`.
- A new compose stage should come with a stage module and descriptor metadata
  rather than another large match section in `compose/mod.rs`.
- Large in-file test suites should move to a sibling `tests` module when the
  production code around them is already being changed.
- Extraction must preserve public APIs and snapshots unless the feature spec
  explicitly says otherwise.

This item is mostly process, but the descriptor table and CLI command module
shape make the next extraction easier.

## Implementation Sequence

1. Remove backup/orig artifacts and verify no references remain.
2. Resolve debug-only frontmatter tests: assert or delete.
3. Finish or validate CLI command module extraction.
4. Add CodeBlock Markdown serialization helper and tests.
5. Add shared highlight parser and wire both parse paths through it.
6. Move render and compose theme resolution into output arms that need it.
7. Add compose operation descriptor table and invariants.
8. Update compose phase docs and the Darkmatter skill if needed.
9. Record god-file guardrails in the relevant design or maintenance docs if a
   nearby doc already exists; avoid creating broad process docs just for this.

The order keeps behavior-preserving cleanup early and leaves the descriptor
table for later because it has the largest internal blast radius.

## Verification Plan

Run the smallest checks that cover the touched surfaces:

```sh
cargo test -p darkmatter --lib markdown::compose
cargo test -p darkmatter --lib markdown::dsl
cargo test -p darkmatter-cli code_block
cargo test -p darkmatter-cli render
cargo test -p darkmatter-cli compose
```

If test names differ, use package-local `rg` to find the nearest module tests
and run those exact filters. For the final pass, prefer the package-area recipe:

```sh
cd darkmatter
just test
```

Do not run `cargo fmt` unless explicitly requested.

## Acceptance Criteria

- The two editor artifact files are absent.
- No assertion-free debug repro remains in frontmatter interpolation tests.
- CLI command implementations are split by command family, with no duplicated
  code-block Markdown serialization.
- `parse_code_info` and CLI `--highlight` share one highlight parser.
- Markdown, JSON, and non-TTY auto render paths do not resolve themes.
- Compose Markdown/Auto and JSON paths do not resolve themes.
- Compose operation metadata has one descriptor source plus invariant tests.
- Compose docs consistently describe four phases and root-only finalization.
- Existing CLI behavior, compose output, and render snapshots remain stable.
