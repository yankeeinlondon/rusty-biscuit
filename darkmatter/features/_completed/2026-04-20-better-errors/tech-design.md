---
prompt: |-
    Use the 'darkmatter' and 'biscuit-terminal' skills!

    Creating high quality errors that are "actionable" is a priority for Darkmatter and the 
    darkmatter/features/_unscheduled/better-errors/spec.md file describes at a high level what we're
    trying to achieve.

    Review the spec.md file and then review each of the documents in the darkmatter/features/_unscheduled/better-errors/ask directory. Each one is tackling a different type of error in Darkmatter and describing how a more contextual error using the BlockError style could be produced.

    Based on this context produce a detailed technical design which covers:

    1. the `BlockError` trait
    2. how we should approach the adoption of this trait in the Darkmatter library
last_updated: 2026-04-20
---
Now let me review the spec and ask documents.Now let me check the existing `StatusBlock` API in biscuit-terminal to make the design concrete.I have enough context now. Let me produce the detailed technical design.Now I have a complete picture of the surface area. Let me produce the replacement markdown body.

# Better Errors — Technical Design

This document specifies (1) the `BlockError` trait — a new terminal-rendered error abstraction owned by `biscuit-terminal` — and (2) the strategy for adopting it across the 16 error enums in the `darkmatter` library as documented in `errors.md` and the `ideas/*` proposals.

The target user experience for every improvement is the **Block Style Error** pattern defined in `spec.md`: a `Status` title line (bold-red `ErrorName:` + bold title) followed by a `StatusBlock` red-bordered body plus an optional hint. Neither the title nor the hint duplicates content from the body — each element carries distinct signal.

---

## 1. The `BlockError` Trait

### 1.1 Ownership & location

The trait lives in `biscuit-terminal` because:

- It is inherently a **rendering contract** (it produces `Renderable` output), not a domain contract.
- Multiple package areas beyond darkmatter (`homelab`, `claudine`, `queue`, `messenger`, etc.) will benefit from a uniform error presentation.
- It needs direct access to `Status`, `StatusBlock`, `Prose`, `Compose`, `Terminal`, `StatusState`, and `Color` — all of which are `biscuit-terminal` types.

Proposed module path: `biscuit_terminal::errors::block_error` with re-export through `biscuit_terminal::prelude`.

### 1.2 Design goals

1. **Ergonomic for implementers.** An error enum should be able to implement `BlockError` with roughly the same effort as `thiserror::Error` — variant-to-block conversion, not boilerplate.
2. **Composable.** The trait must compose cleanly with Rust's `std::error::Error` machinery so existing `?`/`thiserror` stacks remain intact.
3. **Terminal-aware.** Rendering must accept a `&Terminal` so wrapping, width, and colour mode are honoured.
4. **Useful without a terminal.** A degraded ANSI-width-80 fallback (`render_optimistic`) must exist for logs, piped output, and tests.
5. **Chain-preserving.** When an error wraps another `BlockError`, the outer renderer should be able to surface the inner block (or a compact causation tag) without losing context.
6. **Non-breaking adoption.** Existing `Display`/`thiserror` behaviour must keep working so the trait can be introduced incrementally.

### 1.3 Trait definition

```rust
// biscuit-terminal/lib/src/errors/block_error.rs

use std::error::Error as StdError;
use crate::{
    components::{
        renderable::Renderable,
        status::StatusState,
        status_block::StatusBlock,
    },
    terminal::Terminal,
};

/// An error that can render itself as a Block Style Error — a `Status` title
/// line, a `StatusBlock` body, and an optional hint — for terminal output.
///
/// Implement this trait alongside (not instead of) `std::error::Error`. The
/// `report_block_error` method is the single required method; every other
/// method has a default implementation that composes `StatusBlock` into a
/// rendered `String` via the provided `Terminal`.
pub trait BlockError: StdError {
    /// Build the `StatusBlock` for this error. Implementors own the
    /// header / body / hint text and the choice of severity.
    fn status_block(&self, term: &Terminal) -> StatusBlock;

    /// Preferred severity for this error. Override for warnings or non-fatal
    /// conditions; defaults to `StatusState::Error`.
    fn severity(&self) -> StatusState {
        StatusState::Error
    }

    /// Render this error to a terminal-ready string.
    ///
    /// The default implementation calls `status_block(term).render(term)`.
    /// Implementors that need to prepend a summary or append a cause chain
    /// should override this method and use `Compose` to stack renderables.
    fn report_block_error(&self, term: &Terminal) -> String {
        self.status_block(term).render(term)
    }

    /// Terminal-agnostic rendering for logs / non-TTY output. Defaults to
    /// an optimistic 80-column render with no capability detection.
    fn report_block_error_optimistic(&self, width: Option<u32>) -> String {
        self.status_block(&Terminal::new_optimistic(width.unwrap_or(80)))
            .render_optimistic(width)
    }

    /// If this error wraps another `BlockError`, return a reference to it.
    /// Used by `report_block_error` implementations that choose to surface
    /// the inner block under a `Caused by:` caption.
    ///
    /// Default: walks `std::error::Error::source` one step and downcasts
    /// via `Any` when possible (see `AnyBlockError` helper in §1.5).
    fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
        None
    }
}
```

Key points:

- `BlockError` requires `StdError`, so every existing `thiserror::Error` enum satisfies the supertrait for free.
- `status_block(&self, term: &Terminal)` returns a **configured but unrendered** `StatusBlock` so callers (including `report_block_error`) can layer their own behaviour (e.g. embed the block inside a `Compose` with a cause chain) before `render` is called.
- `report_block_error(&Terminal) -> String` matches the signature the spec requested.

### 1.4 Helper types in `biscuit-terminal`

Three small support types live next to the trait:

```rust
/// A builder that standardises the `<b>ErrorName:</b> <b>Title</b>` header
/// pattern so every `BlockError` renders the same shape without duplicating
/// Prose token strings across call sites.
pub struct ErrorHeader<'a> {
    pub error_name: &'a str,   // "TransclusionError"
    pub title: &'a str,        // "cycle detected"
}

impl<'a> ErrorHeader<'a> {
    pub fn into_prose(self) -> String {
        format!("<b>{}:</b> <b>{}</b>", self.error_name, self.title)
    }
}
```

```rust
/// Convenience extension on `StatusBlock` that takes an `ErrorHeader` directly
/// so implementors write the canonical pattern in one call.
pub trait StatusBlockExt {
    fn error_header(self, header: ErrorHeader<'_>) -> Self;
}
impl StatusBlockExt for StatusBlock {
    fn error_header(self, header: ErrorHeader<'_>) -> Self {
        self.header(header.into_prose())
    }
}
```

```rust
/// Compose a `BlockError` with one or more cause blocks derived from
/// `block_source()`. Used by wrapper errors (e.g. `MarkdownError`) whose
/// inner types also implement `BlockError`.
pub fn render_with_causes<E: BlockError + ?Sized>(err: &E, term: &Terminal) -> String;
```

`render_with_causes` walks `block_source()` up to a bounded depth (default 3), stacking each cause as an additional `StatusBlock` with a `Caused by:` caption rendered through `Compose`. This is the canonical implementation every "wrapper" error uses (see §2.4 _Wrapper Strategy_).

### 1.5 Interop with `std::error::Error`

To let a wrapper (e.g. `MarkdownError::Transclusion(TransclusionError)`) discover whether its `source` implements `BlockError`, we add a tiny internal helper:

```rust
pub(crate) fn as_block_error(err: &(dyn StdError + 'static))
    -> Option<&(dyn BlockError + 'static)>;
```

Because Rust trait objects can't be up-cast from `&dyn StdError` to `&dyn BlockError` directly, we publish a sibling macro:

```rust
/// Implements `fn as_block_error_ref(&self) -> &dyn BlockError` returning
/// `self`. Derive-friendly; applied via a `#[block_error]` proc-macro in
/// Phase 2 of the rollout (§2.1) to avoid boilerplate.
```

During Phase 1 rollout (§2) we supply a manual blanket impl in the crate where the wrapper lives — this is an adequate bridge until the optional proc-macro lands.

### 1.6 Rendering contract

Every `BlockError` implementation must:

1. Produce a **title line** matching `<b>ErrorName:</b> <b>Human-readable summary</b>`. The error-name half is rendered bold + red by `StatusState::Error`; the summary half is bold + default colour.
2. Put **descriptive content** in the body: never repeat the title there. Use `Prose` tokens (`<b>`, `<dim>`, `<red>`, `<cyan>`, `<yellow>`) for inline styling.
3. Put at most **one** high-signal hint in the `hint()` slot — actionable, imperative ("Run X", "Set Y", "Remove Z"), preferably with a command or directive example.
4. Prefer `StatusState::Error` unless the variant is informational (warning surfaces downgrade via `severity()`).
5. Never embed raw `\n\n` sequences for visual spacing — rely on `StatusBlock`'s layout (`Margin` / `WordWrap`); use `Compose` when stacking prose + code snippets in the body.
6. Be idempotent: calling `report_block_error` multiple times with the same terminal yields identical output (the trait has no interior mutability).

### 1.7 Blanket behaviour for free `std` errors

For `MarkdownError` variants that wrap bare `std` / third-party errors (`std::io::Error`, `reqwest::Error`, `serde_json::Error`) we do **not** implement `BlockError` on those foreign types. Instead, the owning variant extracts context at construction time (operation description, target path, URL, etc.) and implements `BlockError` locally. See §2.3 _Error-type enrichments_.

---

## 2. Adoption in the Darkmatter Library

The `errors.md` catalogue shows 16 error enums with roughly 100 variants. Not every variant warrants a block; but every error **enum** eventually gets a `BlockError` impl (even if a subset of variants delegate to a generic fallback block). This lets CLI and library consumers apply one rendering idiom everywhere.

### 2.1 Phased rollout

The work is structured as four sequential phases, each independently shippable:

| Phase                        | Goal                                                                                                                                                     | Packages touched            |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------|
| **P1 — Foundation**          | Ship `BlockError` trait + helpers + tests in `biscuit-terminal`. No darkmatter changes yet.                                                              | `biscuit-terminal`          |
| **P2 — CLI plumbing**        | `md` CLI's top-level error handler prefers `BlockError` rendering when available. Unchanged errors still fall back to `Display`.                         | `darkmatter/cli`            |
| **P3 — High-value variants** | Implement `BlockError` for the 9 priority variants listed in §2.5. Each variant lands as its own PR with a regression test.                              | `darkmatter/lib`            |
| **P4 — Coverage & polish**   | Implement `BlockError` for remaining enums (fallback-quality blocks where variants are thin). Wire library consumers (Claudine, reference CLI commands). | `darkmatter/lib`, consumers |

The **foundation phase** is mandatory before any darkmatter work starts. Phases P3 and P4 contain many independently landable tasks, so they are the right point to fan out parallel implementation.

### 2.2 Structural approach per enum

For each error enum the pattern is:

1. **Keep the existing `#[derive(thiserror::Error)]` attribute** and all `#[error("…")]` format strings. `Display` stays stable for log lines and `{err:?}` debug output.
2. **Enrich variants that need structured context** (see §2.3). These changes are additive to fields — new fields are appended and existing call sites updated in the same PR so public API breaks are contained.
3. **Add `impl BlockError for $Enum`** using a `match self { … }` that constructs and returns a `StatusBlock`. Each variant returns its own block.
4. **Expose a trait re-export** through `darkmatter::prelude` (or a new `darkmatter::errors` module) so downstream consumers can invoke `err.report_block_error(&term)` without importing `biscuit_terminal` directly.
5. **Add a test per variant** verifying that `status_block(&term_optimistic).render_optimistic(Some(80))` contains a non-empty title, body, and hint where applicable. Snapshot tests are appropriate; the fixture lives under `darkmatter/lib/tests/error_snapshots/`.

### 2.3 Error-type enrichments

The `ideas/*` documents call out where today's variants lack structured data. The enrichments are consolidated here so they can be planned as a single schema migration:

| Enum                          | Variant                             | New / expanded fields                                                                                                                                                                                                                 |
|-------------------------------|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `TransclusionError`           | `CycleDetected`                     | Keep `chain: Vec<(PathBuf, usize)>` with per-hop line numbers instead of `Vec<String>`.                                                                                                                                               |
| `TransclusionError`           | `InvalidReference`                  | Add `source_file: PathBuf`, `directive_kind: DirectiveKind` (enum).                                                                                                                                                                   |
| `DeferredSetError`            | `InvalidAssignment`                 | Add `source_file: PathBuf`; already carries `line`, `raw`, `reason`.                                                                                                                                                                  |
| `ConditionError`              | `Parse`                             | Add `span: Range<usize>` within `expr` so the renderer can point a caret.                                                                                                                                                             |
| `ShellExpansionError`         | `ExecutionFailed`                   | Already rich — no schema change; `BlockError` impl composes `Compose` with stdout/stderr.                                                                                                                                             |
| `TocLinkingError`             | `InvalidCleanupService`             | Add `valid_services: &'static [CleanupServiceDescriptor]` — a static table so the block can enumerate them with descriptions.                                                                                                         |
| `PageBlockError`              | `UnterminatedBlock`                 | Add `opening_text: String` (the raw `::block …` line) and `file_ends_at_line: usize`.                                                                                                                                                 |
| `CtxMergeError`               | `InvalidUserCtx`                    | Add `actual_kind: &'static str` (one of `"string"`, `"array"`, `"number"`, `"bool"`, `"null"`).                                                                                                                                       |
| `NormalizationError`          | `LevelOverflow`                     | Add `max_safe_target: HeadingLevel` (already computable via `StructureValidation::can_relevel_to`).                                                                                                                                   |
| `NormalizationError`          | `ValidationFailed`                  | Replace `String` with `Vec<StructureIssue>` (type already exists in `types.rs`).                                                                                                                                                      |
| `ReferenceError`              | `ParseDirective`                    | Add `source_file: PathBuf`, `directive_text: String`, `caret_col: Option<usize>`.                                                                                                                                                     |
| `StylesheetError`             | `PropertyValueTypeMismatch`         | No schema change; enrichment comes from static `CssProp::expected_kind()` / `examples_for_property()` helpers (new).                                                                                                                  |
| `LinkError` / `ImageRefError` | `MalformedMarkdown`                 | Extend to `MalformedMarkdown { message: String, input: Option<String>, caret: Option<usize> }`. Legacy `From<&str>` constructor preserved.                                                                                            |
| `ImageRefError`               | `InvalidReferrerPolicy` (and peers) | No schema change; `BlockError` impl runs a Levenshtein `did-you-mean` against `ReferrerPolicy` variants at render time.                                                                                                               |
| `MermaidThemeError`           | `InvalidJson`                       | Capture the offending JSON snippet + `serde_json::Error::line()/column()` at construction.                                                                                                                                            |
| `EditorError`                 | Multiple                            | Broader variant restructuring — see `ideas/editor.md`. Key additions: `NonZeroExit { code, editor, path }`, `Missing { path }`, `LaunchFailed { editor, full_command, source }`, and wrap `Io` with an `operation: &'static str` tag. |

All enrichments are **internal** to `darkmatter/lib`. The public API surface (enum constructors) accepts new fields, but most call sites are internal. External consumers see improved `Display` output and — if they opt in — `BlockError` rendering.

### 2.4 Wrapper strategy (`MarkdownError`)

`MarkdownError` is the top-level enum users see on the CLI. It has 15 variants, most of which delegate to sub-errors that also implement `BlockError`. The pattern for its implementation is:

```rust
impl BlockError for MarkdownError {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        match self {
            // Delegating variants: defer to the inner BlockError.
            MarkdownError::Transclusion(inner) => inner.status_block(term),
            MarkdownError::ShellExpansion(inner) => inner.status_block(term),
            MarkdownError::PageBlock(inner) => inner.status_block(term),
            MarkdownError::TocLinking(inner) => inner.status_block(term),
            MarkdownError::Reference(inner) => inner.status_block(term),
            MarkdownError::CtxMerge(inner) => inner.status_block(term),
            // Leaf variants: own block.
            MarkdownError::FrontmatterParse(yaml) => frontmatter_parse_block(yaml, term),
            MarkdownError::FileLoad { path, source } => file_load_block(path, source),
            MarkdownError::UrlFetch { url, origin, source } => url_fetch_block(url, origin, source),
            // …
        }
    }

    fn report_block_error(&self, term: &Terminal) -> String {
        render_with_causes(self, term)
    }

    fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
        match self {
            MarkdownError::Transclusion(inner) => Some(inner),
            MarkdownError::ShellExpansion(inner) => Some(inner),
            MarkdownError::PageBlock(inner) => Some(inner),
            MarkdownError::TocLinking(inner) => Some(inner),
            MarkdownError::Reference(inner) => Some(inner),
            MarkdownError::CtxMerge(inner) => Some(inner),
            _ => None,
        }
    }
}
```

Delegating variants **pass the sub-error's block through unchanged** — there is no need to add an outer "transclusion error occurred" wrapper block because the sub-error already names its own enum. This avoids the doubled `MarkdownError: Transclusion` / `TransclusionError: cycle detected` noise.

Leaf variants (`FrontmatterParse`, `FileLoad`, `UrlFetch`, `ThemeLoad`, `AstParse`, `InvalidLineRange`, `Serialization`, `Transform`, `FrontmatterMerge`) each get a dedicated helper function that returns a fully configured `StatusBlock`. Helpers live in a private module `darkmatter::markdown::errors::blocks` so they remain testable in isolation.

### 2.5 Priority variants for Phase 3

These 12 variants carry the most user-visible pain and give the largest UX uplift per unit of work. They should land first, one PR per bullet group, with snapshot tests:

1. `TransclusionError::CycleDetected` — chain visualisation with per-hop line numbers.
2. `TransclusionError::InvalidReference` — show directive + resolution attempt.
3. `ShellExpansionError::ExecutionFailed` — multi-part body (command, exit code, stderr, stdout).
4. `ShellExpansionError::CommandNotFound` — PATH hint.
5. `ShellExpansionError::ApprovalRequired` — whitelist/blacklist paths + `--approve-shell` CTA.
6. `PageBlockError::UnterminatedBlock` + `UnmatchedEnd` — opening directive echoed.
7. `ConditionError::Parse` — expression + caret + operator/function list.
8. `TocLinkingError::InvalidCleanupService` — enumerate valid services.
9. `ReferenceError::ParseDirective` — directive text + caret + syntax example.
10. `EditorError::NoEditorFound` + `LaunchFailed` — detection-step listing; decoded `io::ErrorKind`.
11. `FileTreeError::PathNotFound` + `NotAFile` — absolute path + invocation hint.
12. `MermaidThemeError::InvalidColor` + `InvalidJson` — accepted formats table / caret in JSON snippet.

### 2.6 CLI integration (`md` binary)

`md` owns its top-level error handler in `darkmatter/cli/src/main.rs`. After Phase 1 lands, the handler changes from:

```rust
eprintln!("{err:#}");
std::process::exit(1);
```

to:

```rust
let term = Terminal::new();
if let Some(block_err) = as_block_error(&err) {
    eprintln!("{}", block_err.report_block_error(&term));
} else {
    eprintln!("{err:#}");
}
std::process::exit(1);
```

For non-TTY (`stderr.is_terminal() == false`), the handler calls `report_block_error_optimistic(None)` to emit plain ANSI-wrapped text at a hard 80 columns — still readable in log files but without relying on live capability detection.

### 2.7 Library integration

Library consumers (`claudine`, `queue`, future monorepo tools) gain access to Block Style Error rendering by:

1. Importing `darkmatter::prelude::BlockError`.
2. Calling `err.report_block_error(&term)` instead of `format!("{err}")` in their own CLI handlers.

Because the trait requires only `StdError`, consumers can wrap darkmatter errors in their own `thiserror` enums without losing the block rendering: they add a trivial `impl BlockError for MyError` that delegates to `self.source().and_then(as_block_error)`.

### 2.8 Testing strategy

- **Per-variant snapshot tests** in `darkmatter/lib/tests/error_snapshots/` using `insta` (or a simple golden-file harness). Each test constructs a fixture error, renders with a fixed `Terminal::new_optimistic(80)`, and asserts against a checked-in `.snap` file.
- **Width tests** at 40 / 80 / 120 columns to confirm `StatusBlock`'s `WordWrap::WrapProse` behaves correctly at edge widths (no caret misalignment when wrapping).
- **ANSI-stripped assertions** for CI environments — compare rendered output after escape stripping to expected plain text.
- **Colour-mode tests** verifying that `StatusState::Error` resolves to `Tailwind::Red500` in dark mode and does not produce invisible text in light mode.
- **Cause-chain tests** for `MarkdownError` using `render_with_causes`, ensuring nested blocks indent consistently and the top-level title comes from the innermost leaf.
- **Regression tests**: existing `Display`-based tests must continue to pass — `thiserror`'s `#[error(...)]` output is unchanged.

### 2.9 Documentation

- Extend `biscuit-terminal/README.md` with a `BlockError` section mirroring the existing `StatusBlock` section and including one end-to-end example.
- Add a `darkmatter/docs/error-rendering.md` page with:

    - the trait contract,
    - the rendering contract rules from §1.6,
    - the canonical header/body/hint shape with before/after screenshots (or text snapshots),
    - an FAQ on how to add a new variant to an existing `BlockError` impl.

- Update the `darkmatter` and `biscuit-terminal` skills to reference the new trait and point at the docs.

### 2.10 Out of scope

- **Machine-readable error output (`--format json`)** — a separate feature. `BlockError` is strictly about terminal presentation.
- **Localisation** — all strings remain English. The trait is structured so that a future `fn fluent_key(&self) -> &'static str` could be added without breaking existing impls.
- **`miette` integration** — `BlockError` intentionally does _not_ adopt `miette::Diagnostic`. The two models overlap but differ; adopting both simultaneously would double the surface area of every error variant. If `miette` becomes desirable later, a `BlockError` impl can be generated from a `Diagnostic` impl (but not the other way around).

---

## 3. Summary

The `BlockError` trait, owned by `biscuit-terminal`, gives every error in the darkmatter library a uniform, terminal-aware rendering contract centred on `Status` + `StatusBlock`. It composes cleanly with `std::error::Error` and `thiserror`, requires only one method in the simple case (`status_block`), and provides opt-in cause-chain rendering for wrapper errors.

Adoption proceeds in four phases: ship the trait, wire it into the `md` CLI handler, implement the 12 priority variants, then round out the remaining enums. Schema enrichments are consolidated up front so variant constructors change once. Snapshot tests enforce per-variant output stability, and the existing `Display` messages remain untouched — callers that don't upgrade see no regressions.
