---
title: Sentrux Quality Review – darkmatter package area
date: 2026-05-04
package_area: darkmatter
packages: [darkmatter, darkmatter-cli]
metrics: [modularity, acyclicity, depth, equality, redundancy]
suggestions: 15
suggestions_critical: 2
suggestions_urgent: 5
---

# Sentrux Quality Review — `darkmatter`

**Note on methodology.** The Sentrux MCP scan/health/dsm tools are gated by user permission and could not be invoked in this non-interactive session, and the referenced `.sentrux/baseline.json` did not exist on disk for this package area. The findings below are derived from a structural read of the source tree (file sizes, module nesting, `use crate::*` graph, `use super::*` counts, deprecated re-exports, and entry-point fan-in) — i.e. the same signals Sentrux derives. Where exact metric numbers would normally be cited, this report uses the underlying observations.

## Aggregate Observations

- **132 source files** in `darkmatter/lib/src` totalling **~81 487 lines** (mean ≈ 617 lines/file).
- **6 source files** in `darkmatter/cli/src` totalling **~4 307 lines** (mean ≈ 718 lines/file).
- Top three files dominate ~19 % of the library: `markdown/output/terminal.rs` (8 458 LOC), `markdown/compose/mod.rs` (4 309 LOC), `markdown/cleanup.rs` (2 994 LOC).
- **148** `use crate::…` edges and **251** `use super::…` edges across the lib.
- File-level cycles exist where `markdown/types.rs` and `markdown/errors/mod.rs` mutually depend, and where `markdown/mod.rs` imports `compose::ComposeSource` while many files inside `compose/*` import `crate::markdown::Markdown`.
- A deprecated re-export wrapper exists at `markdown/delta/visual/mod.rs`, pointing at `crate::diff::visual::*`.

## `darkmatter`

### `critical`: God-file in terminal renderer

**Problem.** `darkmatter/lib/src/markdown/output/terminal.rs` is **8 458 lines** with **334 `fn` definitions** (29 free `pub fn`/`fn` at module scope plus a large embedded test module of 246 `#[test]` cases). A single file at this size is a Sentrux "god file" and dominates Equality (Gini) and Redundancy metrics; it concentrates state-machine, table-rendering, list-rendering, code-block, image-block, blockquote and inline rendering in one translation unit. It is also the highest-coupling node — virtually every change in `markdown/*` reaches it.

**Files touched.**

- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/src/markdown/output/mod.rs`

**Fix.** Decompose by concern into a `markdown/output/terminal/` directory with peer files. Each peer should expose a single `render_*` entry point and own its own `#[cfg(test)]` block, keeping helpers `pub(super)` so the public surface of `output::terminal` is unchanged.

```text
markdown/output/terminal/
├── mod.rs              // pub fn write_terminal entry, dispatch
├── state.rs            // RenderState, options, theme application
├── block/
│   ├── heading.rs
│   ├── paragraph.rs
│   ├── list.rs
│   ├── table.rs
│   ├── code_block.rs   // wraps shared output/code_block helpers
│   ├── blockquote.rs
│   ├── image.rs        // delegates to biscuit-terminal
│   └── rule.rs         // delegates to biscuit-terminal HorizontalRule
├── inline.rs
└── tests/              // split test module by block kind
```

```rust
// markdown/output/terminal/mod.rs
mod state;
mod block;
mod inline;
#[cfg(test)] mod tests;

pub use state::TerminalOptions;

pub fn write_terminal<W: io::Write>(
    out: &mut W,
    md: &Markdown,
    options: TerminalOptions,
) -> MarkdownResult<()> {
    let mut state = state::RenderState::new(options);
    block::render_document(out, md, &mut state)
}
```

Target: no peer file > 1 200 LOC; tests co-located with the block they cover.

### `critical`: God-file in compose pipeline orchestrator

**Problem.** `darkmatter/lib/src/markdown/compose/mod.rs` is **4 309 lines** with **164 `fn` definitions** and **128 `#[test]` cases**. It is the orchestrator for the seven-stage Inline-Pre + Transclusion + Inline-Post pipeline, but it also embeds large quantities of stage-level glue code (option resolution, source tracking, report assembly, replacement coordination) plus the bulk of pipeline tests. Skill docs treat compose as a published module; this size makes it the second-highest-priority Equality outlier and a Modularity drag because every sibling re-imports `ComposeSource`, `ComposeOptions`, `EffectiveStateBuilder`, `ComposeReport` from this single file.

**Files touched.**

- `darkmatter/lib/src/markdown/compose/mod.rs`
- All `markdown/compose/*/*.rs` files importing from the parent
- `darkmatter/lib/src/markdown/mod.rs` (re-exports ComposeSource)

**Fix.** Move type definitions out of `mod.rs` and leave it as a thin façade that wires phases together.

```text
markdown/compose/
├── mod.rs              // façade: pub use + compose() entry only
├── source.rs           // ComposeSource, SourceRange (move out of types.rs head)
├── options.rs          // ComposeOptions, ComposeOperation
├── report.rs           // ComposeReport, ComposeWarning
├── pipeline.rs         // run_pipeline(): inline_pre → transclusion → inline_post
└── stages/             // one module per pipeline stage
    ├── inline_pre.rs
    └── inline_post.rs
```

```rust
// markdown/compose/mod.rs
pub mod cache;
pub mod conditions;
pub mod context;
pub mod expression;
pub mod interpolation;
pub mod options;
pub mod page_blocks;
pub mod pipeline;
pub mod report;
pub mod shell_blocks;
pub mod shell_expansion;
pub mod source;
pub mod stages;
pub mod toc_linking;
pub mod transclusion;

pub use options::{ComposeOperation, ComposeOptions, TransclusionOptions};
pub use report::{ComposeReport, ComposeWarning};
pub use source::ComposeSource;
pub use pipeline::compose;
```

Target: `mod.rs` ≤ 200 LOC; per-stage files ≤ 1 000 LOC; tests live next to their stage.

### `urgent`: File-level cycle between `markdown::types` and `markdown::errors`

**Problem.** Acyclicity (Martin) failure: `markdown/types.rs` imports `crate::markdown::errors::blocks`, while `markdown/errors/mod.rs` imports `crate::markdown::MarkdownError` (which is defined in `markdown/types.rs` and re-exported through `markdown/mod.rs`). The two files form a 2-node cycle. Any change to either file forces a re-typecheck of the other and prevents the registry from being moved without touching the type root.

**Files touched.**

- `darkmatter/lib/src/markdown/types.rs:10`
- `darkmatter/lib/src/markdown/errors/mod.rs:18`
- `darkmatter/lib/src/markdown/errors/blocks.rs`

**Fix.** Break the cycle by inverting the direction: keep error rendering helpers in `errors::blocks`, but stop having `types.rs` reach back into `errors`. Either:

1. Move the small subset of `errors::blocks` helpers consumed by `types.rs` into `types.rs` (or a new `markdown/types/blocks.rs`), then let `errors/mod.rs` continue to import `MarkdownError` one-way.
2. Or expose a trait in `types.rs` (`trait BlockRender`) that errors implement — push the dependency from `types` onto the abstraction, with concrete `impl` blocks living in `errors`.

```rust
// markdown/types.rs — option (2)
pub trait MarkdownErrorBlock {
    fn render_block(&self) -> StatusBlock;
}

// markdown/errors/blocks.rs
impl MarkdownErrorBlock for MarkdownError { /* … */ }
```

After the fix, `cargo modules dependencies --no-uses --layout dot` should show `errors → types` only, with no back-edge.

### `urgent`: File-level cycle between `markdown::mod` and `markdown::compose::mod`

**Problem.** `markdown/mod.rs:73` does `use compose::ComposeSource`, while every file under `compose/` imports `crate::markdown::Markdown`. This is a parent ↔ child file-level cycle. Sentrux scores file-pair cycles independently, so even though Rust permits parent/child references, this counts against Acyclicity and pulls Modularity down (the cluster boundary is fuzzy).

**Files touched.**

- `darkmatter/lib/src/markdown/mod.rs`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- All consumers of `markdown::Markdown` inside `markdown/compose/*`

**Fix.** Move `ComposeSource` out of the `compose::mod.rs` re-export and into a dedicated `markdown/compose/source.rs` (this is also a sub-task of the compose god-file fix above), and keep the `markdown/mod.rs` import limited to that leaf:

```rust
// markdown/mod.rs
use compose::source::ComposeSource;
```

This converts the cycle into a directed edge `markdown::mod → compose::source`, breaking the back-edge from `compose::mod`. Then ensure `compose::source` does **not** import `crate::markdown::Markdown` — it only needs `Path`, `String`, and `serde_json::Value`.

### `urgent`: `markdown::errors` is a coupling hub

**Problem.** `markdown/errors/mod.rs` imports from **17** sibling modules (`compose::ShellBlockError`, `compose::ShellExpansionError`, `compose::TocLinkingError`, `compose::TransclusionError`, `compose::conditions::ConditionError`, `compose::context::merge::CtxMergeError`, `compose::page_blocks::PageBlockError`, `compose::transclusion::DeferredSetError`, `normalize::NormalizationError`, `reference::ReferenceError`, `reference::file_tree::FileTreeError`, `mermaid::MermaidThemeError`, `render::image_ref::ImageRefError`, `render::link::LinkError`, `render::stylesheet::StylesheetError`, `editor::EditorError`, `markdown::MarkdownError`). High inbound fan-in plus a "knows everyone" registry pattern depresses Newman modularity — the module belongs to no community.

**Files touched.**

- `darkmatter/lib/src/markdown/errors/mod.rs`

**Fix.** Replace the centralized `as_block_error` registry with a `BlockError` trait object indirection so each owning module registers its own mapping. Two viable shapes:

1. **Dyn dispatch via `dyn StdError`:** require every concrete error type to implement `BlockError`, then `as_block_error` becomes `(err as &dyn StdError).downcast_ref::<dyn BlockError>()` — `errors/mod.rs` no longer needs to know each variant.
2. **Inventory crate / `linkme` slice:** each error module submits its own `ErrorMapper` entry to a global slice; `errors/mod.rs` only iterates the slice.

```rust
// markdown/errors/mod.rs (after fix — no per-module imports)
pub fn as_block_error(err: &(dyn StdError + 'static)) -> Option<&dyn BlockError> {
    inventory::iter::<ErrorMapper>
        .into_iter()
        .find_map(|m| (m.map)(err))
}
```

```rust
// markdown/compose/transclusion/mod.rs
inventory::submit!(ErrorMapper { map: |e| e.downcast_ref::<TransclusionError>().map(|x| x as &dyn BlockError) });
```

After the change, `errors/mod.rs` should have ≤ 3 `use crate::*` imports.

### `urgent`: Equality outlier — `markdown/cleanup.rs`

**Problem.** `cleanup.rs` is **2 994 LOC** with **141 `fn`** and **95 `#[test]`** in one file. It is the third-largest contributor to the Gini outlier set and Sentrux will flag it as a god-file. Cleanup currently mixes formatting normalizers (whitespace, tables, list spacing, blank-line collapsing, code-fence repair, heading whitespace) which are independent rules.

**Files touched.**

- `darkmatter/lib/src/markdown/cleanup.rs`

**Fix.** Convert into a `markdown/cleanup/` directory with one rule per file, plus a `pipeline.rs` that sequences them. Each rule becomes a `pub(crate) fn apply(input: &mut String)` (or operates on `pulldown_cmark::Event` stream) and ships its own tests. The public API stays at `markdown::cleanup::clean_markdown`.

```text
markdown/cleanup/
├── mod.rs              // pub fn clean_markdown + ListSpacingMode
├── pipeline.rs         // ordered rule runner
└── rules/
    ├── whitespace.rs
    ├── list_spacing.rs
    ├── tables.rs
    ├── code_fences.rs
    ├── headings.rs
    └── blank_lines.rs
```

### `important`: Compose subdirectory has 30+ files but no clear cluster boundaries

**Problem.** `markdown/compose/` contains 7 sub-directories (`cache`, `context`, `expression`, `interpolation`, `page_blocks`, `shell_blocks`, `shell_expansion`, `toc_linking`, `transclusion`) plus 9 free files (`block_pairs.rs`, `conditions.rs`, `frontmatter_interpolation.rs`, `frontmatter_shell_expansion.rs`, `parse_utils.rs`, `perf.rs`, `replacement.rs`, `state.rs`, `types.rs`). The free files cross-cut the sub-directories — `parse_utils.rs` is imported by toc_linking, shell_expansion::parser, shell_blocks::parser; `state.rs` is imported by cache::hashing, shell_expansion::discovery, etc. Newman modularity drops because the cluster boundary inside `compose` is blurry.

**Files touched.**

- `darkmatter/lib/src/markdown/compose/{block_pairs,conditions,frontmatter_interpolation,frontmatter_shell_expansion,parse_utils,perf,replacement,state}.rs`

**Fix.** Group the free files by phase, matching the documented "Inline Pre / Transclusion / Inline Post" phases:

```text
markdown/compose/
├── pre/                // frontmatter_interpolation, frontmatter_shell_expansion,
│                       // replacement, page_blocks, interpolation, shell_expansion,
│                       // shell_blocks
├── transclusion/       // (existing)
├── post/               // cleanup pass + normalize hooks
├── shared/             // parse_utils, block_pairs, perf, state, conditions
└── mod.rs              // pipeline only
```

Even moving just `parse_utils.rs`, `block_pairs.rs`, `perf.rs`, `state.rs` into `compose/shared/` will materially raise the Modularity score because the sibling-import edges become "child-of-shared" instead of "everyone-imports-everyone".

### `important`: Equality outliers in `render/` (image_ref, stylesheet, link)

**Problem.** `render/image_ref.rs` (2 186 LOC), `render/stylesheet.rs` (2 016 LOC) and `render/link.rs` (1 913 LOC) are all in the Gini top-7. The `render` module is intended to be a thin set of hyperlink/image-ref helpers, but each file has grown into a sub-system. `image_ref.rs` is also pulled into `markdown/errors` (`ImageRefError`), so its weight propagates into the coupling hub.

**Files touched.**

- `darkmatter/lib/src/render/image_ref.rs`
- `darkmatter/lib/src/render/stylesheet.rs`
- `darkmatter/lib/src/render/link.rs`

**Fix.** For each, split the data type from the resolution / parsing / validation logic and move tests to dedicated peer files. Suggested layout per renderable:

```text
render/image_ref/
├── mod.rs
├── types.rs            // ImageRef, ImageRefError
├── parse.rs            // CommonMark image syntax parser
├── resolve.rs          // path/URL resolution against a base
└── tests.rs
```

Target: no file in `render/` > 800 LOC.

### `important`: Depth chain `markdown → compose → shell_expansion → executor`

**Problem.** Lakos depth: the `compose` subtree reaches 4 levels of nesting (`markdown/compose/shell_expansion/{discovery,executor,parser,policy,store,tokenize,types,alias}.rs` and `markdown/compose/transclusion/{code,conditions,engine,parser,resolver,types,wrappers}.rs`). Combined with workspace-level `darkmatter::markdown::compose::shell_expansion::executor::*`, callers see 5-segment paths. Each extra segment increases compile-unit churn and obscures public-API discoverability.

**Files touched.**

- `darkmatter/lib/src/markdown/compose/shell_expansion/*`
- `darkmatter/lib/src/markdown/compose/transclusion/*`

**Fix.** Pair this with the modularity reorganisation above — once `compose/{pre,post,shared,transclusion}` exists, collapse `shell_expansion` and `shell_blocks` under `compose/pre/`, and re-export only the public shell-policy types at `compose::shell` so external callers do not have to know the leaf path. Internal modules can remain deeply nested; what matters for the metric is the **public** path.

```rust
// markdown/compose/mod.rs
pub mod shell {
    pub use super::pre::shell_expansion::types::{ErrorHandling, ShellExpansionError};
    pub use super::pre::shell_blocks::types::ShellBlockError;
}
```

Existing `pub use compose::ShellExpansionError` re-exports stay valid; only internal paths shorten.

### `important`: Redundancy — deprecated re-export wrapper still in tree

**Problem.** `markdown/delta/visual/mod.rs` is a 6-line file whose entire purpose is `pub use crate::diff::visual::*;` with a `#[deprecated]` note. Sentrux's Kolmogorov / Redundancy metric flags any node that exists solely to forward to another node; this is a textbook example. It also adds an extra entry to the file graph (and one extra file under `markdown/delta/`) that contributes no information.

**Files touched.**

- `darkmatter/lib/src/markdown/delta/visual/mod.rs`
- `darkmatter/lib/src/markdown/delta/mod.rs` (the `pub mod visual` declaration)

**Fix.** Delete `markdown/delta/visual/mod.rs` and remove its `pub mod visual;` line from `markdown/delta/mod.rs`. The canonical location is already `darkmatter::diff::visual`, and the deprecation has been in place — confirm with `grep -rn 'darkmatter::markdown::delta::visual'` across the workspace before deleting; it should return zero hits.

```bash
rg -n 'darkmatter::markdown::delta::visual' --type rust
# expected: no matches
```

### `important`: Equality outlier — `markdown/compose/types.rs`

**Problem.** `compose/types.rs` is **2 261 LOC**. Sentrux flags type-only mega-files because they create a single dependency root that every sibling imports — coupling spikes and the file becomes a permanent merge-conflict zone.

**Files touched.**

- `darkmatter/lib/src/markdown/compose/types.rs`

**Fix.** Split by type family into `compose/types/{source,report,operation,warning,setter,error}.rs`, then re-export from `compose/types/mod.rs`. Every sibling continues to write `use crate::markdown::compose::types::ComposeWarning` — the public path is unchanged, but the internal graph splits the hub into 5–6 leaf nodes.

### `nice-to-have`: `markdown/inline/mod.rs` and `markdown/inline_html.rs` are co-located concerns

**Problem.** `inline_html.rs` (871 LOC) sits at the same level as `inline/mod.rs` (1 030 LOC) and the two repeatedly cross-import. Modularity could be improved by treating them as a single cluster.

**Files touched.**

- `darkmatter/lib/src/markdown/inline_html.rs`
- `darkmatter/lib/src/markdown/inline/mod.rs`

**Fix.** Move `inline_html.rs` under `markdown/inline/html.rs`. Update `markdown/mod.rs` to drop the `mod inline_html;` line.

### `nice-to-have`: `markdown::types::MarkdownResult` aliasing redundancy

**Problem.** `MarkdownResult` is re-exported via `markdown/mod.rs` and most consumers use `use crate::markdown::types::MarkdownResult` directly. Two import paths to the same type bloat Sentrux's import graph and create per-file inconsistency.

**Files touched.**

- `darkmatter/lib/src/markdown/types.rs`
- `darkmatter/lib/src/markdown/mod.rs`
- All `markdown/compose/**/*.rs` consumers

**Fix.** Pick the public path (`crate::markdown::MarkdownResult`) and replace the deeper `crate::markdown::types::MarkdownResult` imports with the public alias. A one-shot:

```bash
rg -l 'use crate::markdown::types::MarkdownResult' \
  | xargs sed -i '' 's|use crate::markdown::types::MarkdownResult|use crate::markdown::MarkdownResult|'
```

(Run inside `darkmatter/lib/`, then `cargo +nightly fmt && cargo check -p darkmatter`.)

## `darkmatter-cli`

### `urgent`: God-file in `cli::commands`

**Problem.** `darkmatter/cli/src/commands.rs` is **2 167 LOC** with **40 functions** — `run_subcommand`, `run_clean`, `run_render`, `run_compose`, `run_get`, `run_set`, `run_rm`, `run_edit`, `run_hash`, `run_validate`, `run_graph`, plus 11 internal `*_to_json` serializers and validation/format helpers. Single-file CLI command dispatchers are the most common Sentrux god-file pattern in CLI crates. The Equality (Gini) score for `darkmatter-cli` is dominated by this file (50 % of the crate's LOC).

**Files touched.**

- `darkmatter/cli/src/commands.rs`
- `darkmatter/cli/src/lib.rs` (re-exports)

**Fix.** Split into one file per subcommand under `cli/src/commands/`, with shared helpers in `commands/shared.rs`:

```text
cli/src/commands/
├── mod.rs              // pub use of run_* + run_subcommand dispatcher
├── shared.rs           // ResolvedTheme, ComposeAllowFlags, load_markdown,
│                       //   resolve_file_path, read_from_stdin, parse_compose_*
├── render.rs           // run_render
├── compose.rs          // run_compose, parse_compose_setter, parse_compose_positionals,
│                       //   print_shell_command_report, escape_table_cell,
│                       //   format_compose_perf_report
├── clean.rs            // run_clean, apply_cleanup, resolve_list_spacing
├── delta.rs            // delegates to lib delta + print_delta
├── toc.rs              // delegates to lib toc + print_toc_tree
├── hash.rs             // run_hash
├── frontmatter.rs      // run_get, run_set, run_rm, format_value, format_raw
├── edit.rs             // run_edit
├── validate.rs         // run_validate, print_validation_report_*,
│                       //   reference_kind_category_label, format_validation_issues
└── graph.rs            // run_graph + all *_to_json serializers,
│                       //   format_duration, format_metric_line
```

```rust
// cli/src/commands/mod.rs
mod clean;
mod compose;
mod delta;
mod edit;
mod frontmatter;
mod graph;
mod hash;
mod render;
mod shared;
mod toc;
mod validate;

pub use clean::run_clean;
pub use compose::run_compose;
pub use edit::run_edit;
pub use frontmatter::{run_get, run_rm, run_set};
pub use graph::run_graph;
pub use hash::run_hash;
pub use render::run_render;
pub use validate::run_validate;
pub use shared::{load_markdown, validate_subcommand_usage};

pub fn run_subcommand(command: CliCommand, cli: &Cli) -> Result<()> {
    match command {
        CliCommand::Clean(args)    => clean::run_clean(args, cli),
        CliCommand::Compose(args)  => compose::run_compose(args, cli),
        CliCommand::Delta(args)    => delta::run_delta(args, cli),
        CliCommand::Edit(args)     => edit::run_edit(&args.file),
        CliCommand::Get(args)      => frontmatter::run_get(args),
        CliCommand::Graph(args)    => graph::run_graph(args),
        CliCommand::Hash(args)     => hash::run_hash(args, cli),
        CliCommand::Rm(args)       => frontmatter::run_rm(args),
        CliCommand::Set(args)      => frontmatter::run_set(args),
        CliCommand::Toc(args)      => toc::run_toc(args, cli),
        CliCommand::Validate(args) => validate::run_validate(args.target),
    }
}
```

Target: no command file > 600 LOC; `commands/mod.rs` ≤ 80 LOC.

### `nice-to-have`: `cli::output` mixes artifact construction and rendering

**Problem.** `darkmatter/cli/src/output.rs` (657 LOC, 13 functions) holds three responsibilities: (1) building output artifacts (`html_artifact`, `json_artifact`, `markdown_artifact`), (2) emitting/showing artifacts (`emit_or_show_artifact`, `open_output_artifact`), and (3) rendering domain objects (`print_delta`, `print_toc_tree`, `render_terminal_output`). Sentrux Modularity will score this as a single file with three internal communities.

**Files touched.**

- `darkmatter/cli/src/output.rs`

**Fix.** Split into `cli/src/output/{mod,artifact,emit,print}.rs`. The public surface (`emit_or_show_artifact`, `print_delta`, `print_toc_tree`, `render_terminal_output`, `*_artifact`) is preserved via re-export from `output/mod.rs`.

## Summary

| Priority | Count |
|----------|-------|
| critical | 2 |
| urgent | 5 |
| important | 5 |
| nice-to-have | 3 |
| **total** | **15** |
