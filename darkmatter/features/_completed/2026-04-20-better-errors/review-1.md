---
reviewer: claude-opus-4-7
date: 2026-04-20
ready: true
feature: 2026-04-20-better-errors
scope:
  - biscuit-terminal/lib/src/errors/*
  - darkmatter/lib/src/markdown/errors/*
  - darkmatter/lib/src/**/*.rs (per-enum BlockError impls)
  - darkmatter/cli/src/main.rs
  - darkmatter/lib/tests/error_snapshots/*
  - darkmatter/docs/error-rendering.md
  - biscuit-terminal/README.md
---

# Better Errors — Review 1

## Summary

The feature is **functionally complete and ready to ship**. The `BlockError`
trait, helpers (`ErrorHeader`, `StatusBlockExt`, `render_with_causes`,
`as_block_error`), and a full adoption pass across **all 16 darkmatter error
enums (100+ variants)** have landed, the CLI handler prefers block rendering,
and snapshot coverage stands at **95 integration tests** plus the inline unit
tests.

All relevant test suites pass:

- `cargo test -p biscuit-terminal --lib` → 1181 passed, 2 ignored
- `cargo test -p darkmatter --test error_snapshots` → 95 passed
- `cargo test -p darkmatter-cli` → 160 passed

> One unrelated test failure exists on `main` in
> `darkmatter::markdown::reference::validate::tests::validate_magic_path_independent_of_cwd`
> (CWD-sensitive magic-path resolution). It has no connection to `BlockError`
> and should be tracked separately.

The review below covers (1) design gaps where the tech design specified richer
structure than shipped, (2) smaller correctness / ergonomics issues, and (3)
opportunities to tighten the rollout. None rise to the level of a ship blocker.

---

## 1. Design-vs-Implementation Gaps

The tech-design's §2.3 "Error-type enrichments" table is the single largest
area of drift. Most variants render useful blocks, but several of the
structured-context additions the design promised were not actually added to
the enum schemas, so the rendered blocks are "good" rather than "rich". These
are all follow-up work rather than regressions.

### 1.1 High-value enrichments not implemented

| Variant | Design expectation | Reality |
|---|---|---|
| `TransclusionError::CycleDetected` | `chain: Vec<(PathBuf, usize)>` with per-hop line numbers | Still `Vec<String>`; rendered chain has no line hops (`darkmatter/lib/src/markdown/compose/transclusion/types.rs:298`) |
| `TransclusionError::InvalidReference` | Add `source_file: PathBuf`, `directive_kind: DirectiveKind` | Still just `{reference, line}` (`…/transclusion/types.rs:283`) |
| `ConditionError::Parse` | Add `span: Range<usize>` so body can draw a caret | No `span`; no caret rendering (`…/compose/conditions.rs:14`) |
| `PageBlockError::UnterminatedBlock` | Add `opening_text: String` + `file_ends_at_line: usize` | Still just `{line}` — opening directive is never echoed (`…/page_blocks/types.rs:18`) |
| `ReferenceError::ParseDirective` | Add `source_file`, `directive_text`, `caret_col` | Still `{line, message}` (`…/reference/errors.rs:10`) |
| `NormalizationError::ValidationFailed` | Replace `String` with `Vec<StructureIssue>` | Still `String(msg)` (`…/normalize/types.rs:414`) |
| `LinkError` / `ImageRefError::MalformedMarkdown` | `{message, input: Option<String>, caret: Option<usize>}` | Remains `MalformedMarkdown(String)` — no caret or input echo |
| `MermaidThemeError::InvalidJson` | Capture offending JSON snippet at construction | Wraps `serde_json::Error` only; snippet not captured |
| `EditorError` restructuring | `NonZeroExit { code, editor, path }`, `Missing { path }`, `LaunchFailed { editor, full_command, source }`, `Io { operation, source }` | None of these enrichments shipped. `NonZeroExit(i32)`, `Missing` (unit), `LaunchFailed { editor, source }`, `Io(#[from] io::Error)` (`…/editor/mod.rs:45-65`) |

**Impact**: These make the difference between a *good* block (the user reads
the message) and a *great* block (the user immediately sees *where* to act).
Callers have to re-parse or grep to recover the context. Worth a follow-up
ticket per family (transclusion/condition/reference/editor) to append fields
and update call sites.

### 1.2 Partial enrichments (acceptable, but worth noting)

- `TocLinkingError::InvalidCleanupService` — design asked for a static
  `&'static [CleanupServiceDescriptor]` with descriptions; implementation
  builds the list at render time from `CleanupService::all()` with names only
  (no descriptions). Good enough for parity but loses the "descriptions"
  promise. (`…/toc_linking/types.rs:70`)
- `StylesheetError::PropertyValueTypeMismatch` — uses a local
  `example_for_kind(kind)` instead of the per-property
  `CssProp::expected_kind()` / `examples_for_property()` helpers from the
  design. Practically fine but less tailored ("use 12px" instead of "use e.g.
  `16px` for `font-size`"). (`…/render/stylesheet.rs:161`)
- `DeferredSetError::InvalidAssignment` — design said "already carries `line`";
  implementation has only `{raw, reason}`. Line is lost before this point and
  only the enclosing `TransclusionError::InvalidFrontmatterAssignment`
  preserves it. As a result `DeferredSetError`'s own rendering lacks a line
  reference — but users almost never see this error directly, so low-priority.
  (`…/transclusion/types.rs:96-113`)

---

## 2. Correctness / API Observations

### 2.1 `biscuit_terminal::errors::as_block_error` is a no-op stub

`biscuit-terminal/lib/src/errors/block_error.rs:188-192` always returns `None`
and is documented as such, but it is exported from `biscuit-terminal/prelude`
and wired into the CLI main handler:

```rust
let block = e.chain().find_map(|cause| {
    as_darkmatter_block_error(cause).or_else(|| as_terminal_block_error(cause))
});
```

Since the terminal stub is unconditionally `None`, the `.or_else(...)` branch
is dead. That is fine today but leaves an extensibility cliff: a downstream
library (e.g. `homelab`, `claudine`) cannot register its own `BlockError`
types into the terminal-local lookup — they must follow darkmatter's pattern
of a per-crate registry. Options:

1. Document explicitly that each crate owns a per-crate registry.
2. Swap the stub for a small inventory-style registry (e.g. `inventory` crate)
   so implementers can `inventory::submit! { as_block_error_fn(...) }` once
   per crate.
3. Remove the prelude export until it is useful.

### 2.2 `MarkdownError::block_source` always returns `None`

`darkmatter/lib/src/markdown/types.rs:113-120` explicitly returns `None` for
every variant, with a correct justification — delegating variants already
inline the inner block so re-surfacing under "Caused by:" would double-render.
But that means the `report_block_error` override:

```rust
fn report_block_error(&self, term: &Terminal) -> String {
    render_with_causes(self, term)
}
```

is effectively equivalent to the default `status_block(term).render(term)`. The
override is harmless dead code today. Either (a) drop it until there's a leaf
variant wrapping a `BlockError` cause, or (b) add a test demonstrating the
intended behaviour (currently only exercised via the inline
`WrapperError` test double in the biscuit-terminal module).

### 2.3 `status_block(&self, term: &Terminal)` signature is effectively unused

None of the 16 darkmatter impls inspect `term`. Every impl takes `_term` and
renders capability-agnostic Prose tokens. This is fine — `Prose` itself
handles capability adaptation at render time — but the trait API suggests a
richer capability check than actually happens. Not a regression; just an
observation for future trait evolution (a `status_block(&self) -> StatusBlock`
signature would be simpler and equally powerful, but it would be a breaking
change so leave it).

### 2.4 `strip_ansi` is duplicated three times

Handrolled copies live in:

- `biscuit-terminal/lib/src/errors/block_error.rs:282-295`
- `darkmatter/lib/src/markdown/errors/mod.rs:114-129`
- `darkmatter/lib/tests/error_snapshots/helpers.rs:7-21`

`biscuit_terminal::utils::escape_codes::strip_escape_codes` (already in the
prelude) does the same job more robustly (handles OSC / CSI / multi-byte
correctly). Swap the three duplicates for the canonical helper.

### 2.5 `as_block_error` registry drift risk

`darkmatter/lib/src/markdown/errors/mod.rs:54-106` manually downcasts 16 types
in sequence. A new enum added without a matching arm would silently fall back
to the `Display` chain. There is no compile-time check. Mitigation options:

1. Add a snapshot test that parses the source and asserts every `impl
   BlockError for X` has a matching `downcast_ref::<X>()` line.
2. Generate the table via a proc-macro or `inventory`-style registry.
3. Add a `#[should_panic]` or `compile_fail` sentinel.

Low urgency but the drift will bite in 6–12 months. Document in
`docs/error-rendering.md`'s "Adding a new error enum" section that the
downcast registry update is mandatory (currently listed but easy to miss).

### 2.6 `DeferredSetError::BlockError` is functionally unreachable via CLI

`DeferredSetError` values never reach the CLI top-level handler — they are
promoted to `TransclusionError::InvalidFrontmatterAssignment` /
`InvalidReassignedFrontmatterProperty` first (see
`…/transclusion/types.rs:324-336`). Keeping the `BlockError` impl is still
useful for library consumers who call the parser directly, but the CLI-level
downcast arm in `as_block_error` (`mod.rs:90`) is functionally dead code.

### 2.7 Snapshot coverage has two acknowledged gaps

`darkmatter/lib/tests/error_snapshots/markdown_error.rs:10-15, 90-92` skips
snapshots for `MarkdownError::FrontmatterParse` (`YamlParseError` needs a
dev-dep) and `MarkdownError::UrlFetch` (`reqwest::Error` cannot be cheaply
constructed). The note claims `errors/blocks.rs` has unit tests for these
paths — it does not. `blocks.rs` contains no `#[cfg(test)]` module at all.
Either:

- Add minimal unit tests in `blocks.rs` that exercise
  `frontmatter_parse_block`, `url_fetch_block`, and the six other helpers via
  synthetic inputs.
- Accept `serde_yaml_ng` as a dev-dep in `darkmatter/lib/Cargo.toml` so the
  snapshot can construct a `YamlParseError`.

The current note misrepresents coverage.

### 2.8 No end-to-end CLI rendering test

`darkmatter/cli/tests/` has no test that runs `md <broken-doc>` and asserts
the stderr output is a block-style error. The CLI handler logic
(`main.rs:67-87`) — TTY vs non-TTY, chain-walking, block discovery — is only
exercised manually. A single `assert_cmd` test feeding a broken document and
asserting stderr contains the expected `ErrorName` + hint would guard against
regressions.

---

## 3. Ergonomics & Performance Suggestions

### 3.1 Shared use-statements per impl

Every BlockError impl repeats the same three `use` statements inside the
function body:

```rust
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};
```

Across 16 impls that's 48 lines of identical boilerplate. Publish a
convenience module:

```rust
// biscuit-terminal/lib/src/errors/prelude.rs
pub use super::{BlockError, ErrorHeader, StatusBlockExt, render_with_causes};
pub use crate::components::status::StatusState;
pub use crate::components::status_block::StatusBlock;
pub use crate::terminal::Terminal;
```

So each impl becomes:

```rust
impl biscuit_terminal::errors::BlockError for X {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        use biscuit_terminal::errors::prelude::*;
        ...
    }
}
```

### 3.2 Static hint strings

Several impls build the same hint on every call (e.g. the `operator_hint` in
`ConditionError` — already lifted to a local const). Most variants use
`format!("…")` unconditionally even when the variant carries no dynamic
content. Promote those to `&'static str` via a `.hint_static(...)` or
`Cow<'static, str>`.

Ex. `ImageRefError::EmptySource.hint("Pass a non-empty URL…")` allocates a
new `String` each render. Minor; only matters in hot paths.

### 3.3 `truncate_output` complexity

`darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:473-497`
iterates `char_indices()` twice when `text.len() > MAX_BYTES`. Rewrite as:

```rust
let cut = text.char_indices()
    .take_while(|&(i, _)| i <= MAX_BYTES)
    .last()
    .map(|(i, c)| i + c.len_utf8())
    .unwrap_or(0);
```

which is a single pass. Not a hot path — shell-exec failures only — so
optional.

### 3.4 `StatusBlock::new(StatusState::Error)` could be `StatusBlock::error()`

Every impl re-opens the same pattern. A small constructor helper
(`StatusBlock::error()` / `StatusBlock::warning()`) would shave one token per
impl. In-scope for biscuit-terminal's API polish pass, out-of-scope for this
feature.

---

## 4. Documentation Review

- `darkmatter/docs/error-rendering.md` is thorough and accurate.
- `biscuit-terminal/README.md` §BlockError is clear and mirrors the trait
  surface.
- Neither doc calls out the **mandatory `as_block_error` registry update** as
  a gotcha for new enums. The "Adding a new error enum" section mentions it
  but doesn't flag the silent-degradation failure mode.
- The coverage table in `error-rendering.md:138-162` is correct against the
  actual impl count.

---

## 5. Follow-up Ticket Proposal

Ordered by impact:

1. **Schema enrichments (P1)** — `TransclusionError::CycleDetected` per-hop
   lines, `PageBlockError::UnterminatedBlock` opening text,
   `ReferenceError::ParseDirective` caret. These close the biggest UX-polish
   gaps from the tech-design.
2. **EditorError restructuring (P1)** — the design called for meaningful
   enum reshaping; none landed. Land as one focused PR.
3. **CLI end-to-end test (P2)** — `assert_cmd` test covering block-rendered
   stderr output on a broken document.
4. **`as_block_error` drift guard (P2)** — source-parse test or
   registry macro so every `impl BlockError for X` is linked to a downcast
   arm.
5. **Close snapshot gaps (P3)** — cover `FrontmatterParse` and `UrlFetch`
   paths with unit tests in `blocks.rs` (or add `serde_yaml_ng` dev-dep).
6. **Ergonomic polish (P3)** — `errors::prelude` module; collapse
   `strip_ansi` duplicates onto `strip_escape_codes`.
7. **Drop `MarkdownError::report_block_error` override (P3)** — dead code
   until a leaf variant has a `BlockError` cause.

None of these are ship blockers. The feature delivers on its primary goal —
every error darkmatter produces now renders as a Block Style Error with a
structured title, context body, and actionable hint — and the test coverage
is sufficient to protect against regressions on the currently shipped
behavior.

---

## 6. Verdict

**Ready for production.** Merge as-is and convert §5 into a follow-up
backlog. The remaining enrichments are polish; the spec's core contract —
terminal-aware, uniformly-rendered, actionable errors — is fully met.
