# Error Rendering

Darkmatter exposes every public error enum through the [`BlockError`] trait from
[`biscuit-terminal`]. A `BlockError` renders as a **Block Style Error**: a
`Status` title line over a red-bordered `StatusBlock` body with an optional
actionable hint.

This page describes:

- the trait contract implementations must satisfy,
- how the `md` CLI discovers a `BlockError` on an arbitrary `std::error::Error`,
- how wrapper errors compose with their inner blocks,
- how to add a new variant (or a new error enum) without breaking the pattern.

[`BlockError`]: https://docs.rs/biscuit-terminal/latest/biscuit_terminal/errors/trait.BlockError.html
[`biscuit-terminal`]: ../../biscuit-terminal/README.md

## Trait contract

Every implementation owns three pieces of text:

1. **Title line** — `<b>ErrorName:</b> <b>Human-readable summary</b>`. The
   error-name half is rendered bold + red by `StatusState::Error`; the summary
   half is bold + default colour. Built via `ErrorHeader::new("MyError",
   "thing failed")` combined with `StatusBlockExt::error_header`.
2. **Body** — descriptive content using `Prose` tokens (`<dim>`, `<b>`,
   `<cyan>`, `<yellow>`, `<red>`). The body **never** repeats the title;
   instead, it supplies the *specific* context (paths, commands, line numbers,
   stderr, caret lines, …) a reader needs to act on the error.
3. **Hint** — at most **one** imperative sentence that tells the user what to
   try next ("Run X", "Set Y", "Remove Z"). Include a command or directive
   example wherever the fix is a command.

## Rendering contract rules

- Do not embed raw `\n\n` sequences for visual spacing; the `StatusBlock`
  layout owns margin/wrapping. Use `Compose` when stacking multiple pieces of
  content.
- Use `StatusState::Error` unless the variant is genuinely informational; use
  `StatusState::Warning` for warnings so the border colour matches severity.
- Keep `report_block_error` idempotent: multiple calls with the same terminal
  must produce identical output.
- When a wrapper enum has a delegating variant that already implements
  `BlockError`, surface the inner block directly from `status_block` instead
  of rendering an outer "wrapper failed" block. This avoids doubled
  `FooError: something` / `InnerError: cause` noise in the terminal.

## Canonical shape

```text
FooError: human-readable summary
┃ <dim>Context:</dim> <cyan>./docs/root.md</cyan>
┃ <dim>Line:</dim> 42
┃
┃ <dim>Hint:</dim> Run `md graph ./docs/root.md` to inspect the graph.
```

## CLI discovery

`md` (in `darkmatter/cli/src/main.rs`) uses
`darkmatter::markdown::errors::as_block_error` to decide whether a caught
`std::error::Error` should be rendered as a block. The helper performs runtime
downcasting against every concrete `BlockError`-implementing type in the
library:

```rust
use darkmatter::markdown::errors::as_block_error;
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();
let dyn_err: &(dyn std::error::Error + 'static) = &err;

if let Some(block_err) = as_block_error(dyn_err) {
    eprintln!("{}", block_err.report_block_error(&term));
} else {
    eprintln!("{err:#}");
}
```

When the process is not attached to a TTY, `md` calls
`report_block_error_optimistic(None)` so output remains readable in log files
without depending on live capability detection.

## Adding a new variant

1. Add the variant to the `#[derive(thiserror::Error)]` enum. Keep the
   `#[error("...")]` format string so `Display` output remains stable.
2. Enrich the variant with any structured context (line numbers, paths, exit
   codes, …). New fields are additive — update constructors in the same PR.
3. Extend the `impl BlockError for FooError` match arm:
   - Use `StatusBlock::new(self.severity())` as the starting point.
   - Call `.error_header(ErrorHeader::new("FooError", "concise summary"))`.
   - Build the body via `Prose` tokens describing the variant-specific state.
   - Append a `.hint("…")` with the imperative next step.
4. Add a fixture to `darkmatter/lib/tests/error_snapshots/<enum>.rs` asserting
   the rendered optimistic output contains the tokens you expect.

## Adding a new error enum

> **Required:** After adding a new `BlockError` implementation, you **must**
> add a corresponding downcast arm to
> [`as_block_error`](../lib/src/markdown/errors/mod.rs) in
> `darkmatter/lib/src/markdown/errors/mod.rs`. Without it the CLI silently
> falls back to the generic `Display` chain and the new enum's rich block
> rendering is never shown. The drift-guard test in
> `darkmatter/lib/tests/as_block_error_registry.rs` will fail CI if you
> forget.

1. Implement `BlockError` in the file that owns the enum (or in a sibling
   `errors/blocks.rs` helper module if the impl would be large).
2. Add the concrete type to `as_block_error` in
   `darkmatter/lib/src/markdown/errors/mod.rs` so the CLI discovers it.
3. Create a new module under `darkmatter/lib/tests/error_snapshots/` covering
   each variant, and register it in `main.rs`.
4. Re-export the enum through `darkmatter::prelude` if downstream consumers
   need to invoke `err.report_block_error(&term)` directly.

## FAQ

### Where do helper functions for leaf variants live?

Inside `darkmatter/lib/src/markdown/errors/blocks.rs`. Each helper returns a
fully configured `StatusBlock` for a single variant so the top-level
`BlockError` impl stays a clean `match`.

### When should I override `report_block_error`?

Only when you need to prepend a summary or append a cause chain. Wrappers
whose inner variants also implement `BlockError` should call
`render_with_causes(self, term)` and override `block_source` to return the
inner error. Delegating variants (the inner block *is* the outer block) should
instead return `None` from `block_source` and surface the inner block from
`status_block` directly.

### What about foreign errors (`std::io::Error`, `reqwest::Error`, …)?

Do **not** implement `BlockError` on foreign types. The variant that wraps
them extracts context at construction (path, URL, operation name, …) and
renders its own block via a helper in `errors/blocks.rs`.

### Is cause-chain rendering automatic?

Only when `report_block_error` is overridden to call `render_with_causes`.
Today only `MarkdownError` does this, and its leaf variants all return `None`
from `block_source` so no double-rendering occurs.

## Coverage

Phase 4 of the "Better Errors" initiative closes the last coverage gaps. All
16 error enums (100+ variants) implement `BlockError`:

| Enum                  | Variants | Notes                                          |
|-----------------------|:--------:|------------------------------------------------|
| `MarkdownError`       | 15       | Wrapper; delegates to inner blocks.            |
| `TransclusionError`   | 18       | Leaf blocks for directive + resource failures. |
| `DeferredSetError`    | 2        | Parser-recorded set-override errors.           |
| `ConditionError`      | 2        | Shared `when=` parser/evaluator.               |
| `ShellExpansionError` | 9        | Rich blocks for exec + policy failures.        |
| `TocLinkingError`     | 6        | Enumerates valid cleanup services.             |
| `PageBlockError`      | 4        | Points at opening line for unterminated blocks.|
| `CtxMergeError`       | 1        | Calls out `--allow-override` CTA.              |
| `NormalizationError`  | 2        | Computes deepest-safe target for overflow.     |
| `ReferenceError`      | 7        | Delegates to inner `MarkdownError` blocks.     |
| `FileTreeError`       | 4        | Absolute-path resolution + invocation hint.    |
| `StylesheetError`     | 9        | Examples per `CssValueKind`.                   |
| `LinkError`           | 7        | Lists valid target values.                     |
| `ImageRefError`       | 10       | Levenshtein "did you mean?" for policies.      |
| `MermaidThemeError`   | 2        | Accepted colour formats hint.                  |
| `EditorError`         | 5        | Lists probed editor binaries.                  |

Snapshot coverage lives in `darkmatter/lib/tests/error_snapshots/`.
