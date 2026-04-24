---
ready: false
reviewer: Gemini CLI
date: 2026-04-21
---

# Feature Review: Contextual Errors (Review 1)

This review evaluates the implementation of the "Contextual Errors" feature against the provided [specification](./spec.md) and [execution plan](./plan.md). While the infrastructure in `darkmatter` (specifically `as_block_error`) is in place, the corresponding integration in `claudine` is missing or incomplete across all planned phases.

## 1. Gaps in Functionality

### Phase 1: Preservation of Typed Errors (Library)
The library-level error types have not been refactored to carry structured `MarkdownError` metadata.
- **`claudine/lib/src/error.rs`**: `ClaudineError::SystemPromptComposition` still carries a `String` (L231). It was expected to become `SystemPromptComposition(#[from] MarkdownError)`.
- **`claudine/lib/src/composition/error.rs`**: `CompositionError::ComposeFailed` (L39) and `PreFlightDiscoveryFailed` (L129) are still `String` payloads.
- **Metadata Destruction**: Call sites in `prepare.rs` (L37) and `preflight.rs` (L58) still use `.map_err(|e| ... e.to_string())`, which destroys structured metadata as identified in the spec's "Current State" section.

### Phase 2: Centralized CLI Rendering
The centralized error handling logic designed to replace specialized renderers is absent.
- **Cause-Chain Walker**: `claudine/cli/src/main.rs` still uses default `color_eyre` installation and does not implement the cause-chain walker that calls `as_block_error`.
- **Specialized Renderer**: `claudine/cli/src/output/shell_expansion_error.rs` has not been retired. It still contains the `PRE_RENDERED_MARKER` and duplicate rendering logic that Phase 2 was intended to consolidate.
- **Top-level Integration**: Wrapper commands and composition subcommands still use the `pretty_or_report` pattern which suppresses rich rendering at the top level.

### Phase 3: Test Coverage
No new tests were found that verify the three headline failure paths:
- System prompt composition failures.
- Transclusion cycle failures.
- Shell expansion failures via the centralized walker.

## 2. Incomplete or Broken Implementation

- **Ergonomics**: The goal of using `?` for error propagation in the library has not been achieved because the `From` implementations are missing.
- **Redundancy**: The codebase now contains both the `darkmatter::as_block_error` infrastructure and the legacy `claudine` specialized renderers, increasing maintenance surface without delivering the benefit of the new system.

## 3. Ergonomics and Performance

- **Performance**: No significant performance concerns were noted, but the current "stringified" errors are slightly less efficient than carrying the typed error and only rendering on failure.
- **Ergonomics**: The current implementation is significantly less ergonomic for developers than the proposed `#[from]` propagation.

## 4. Suggestions for Finalization

1. **Implement Phase 1**: Update the enums in `claudine/lib` to carry `MarkdownError` and remove all `.to_string()` calls on error boundaries.
2. **Implement Phase 2**: Add the `as_block_error` walker to `main.rs` and delete `shell_expansion_error.rs`.
3. **Add Tests**: Create the snapshot tests described in Phase 3 of the plan to prove that deep error metadata is preserved and rendered correctly.

## Conclusion

The feature is **NOT ready for production**. The current state of the codebase does not reflect the changes described in the specification.
