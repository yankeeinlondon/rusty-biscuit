# Compose Refactor Review

Following the implementation of the Claudine compose refactor, here is a technical review focused on architectural alignment, functionality gaps, and potential improvements.

## Summary of Successes

The refactor successfully addresses the primary goals of the specification:

- **Canonical CLI Surface**: The CLI has been reduced to `claudine compose` and `claudine inline-compose`, with retired entry points (like `compose inline` and provider-local `--compose`) removed.
- **Shared Execution Pipeline**: Both commands now route through `claudine/cli/src/commands/wrap/composition.rs`, which provides a unified "wrapper-grade" execution path.
- **Effective Frontmatter**: The key architectural fix of using composed (Darkmatter-processed) frontmatter as the source of truth for provider selection and harness detection is correctly implemented.
- **Deterministic Inline Closure**: `inline-compose` now performs a deterministic rewrite in the library (`claudine/lib/src/composition/closure.rs`), preserving original frontmatter and updating only managed fields (`last_updated`).
- **Provider Selection**: Precedence rules match the specification, including support for explicit flags, effective frontmatter hints, and interactive chooser fallbacks.

## Functional Gaps & Regression Risks

### 1. Harness Regression in Plain Wrapper Path
A significant regression was observed: the plain wrapper path (`claudine <agent> -- prompt`) in `claudine/cli/src/commands/wrap/mod.rs` no longer calls `run_harness_loop`. While composition now has full harness support, standard prompt passthroughs have lost it.

- **Impact**: Users who rely on `CLAUDE.md` or similar for repository-wide validations/handlers during regular wrapped sessions will find them inactive.
- **Recommendation**: Restore harness detection to `run_provider_wrapper_inner` or unify the two entry points further into a shared execution request model that always checks for harness properties.

### 2. Redundant Document Composition
In `claudine/cli/src/commands/wrap/mod.rs`, `materialize_harness_prompt` re-composes the document via Darkmatter.

- **Observation**: For top-level composition, the document is already composed during the "Prepare" stage in `claudine/cli/src/commands/compose.rs`. Re-composing it in the harness loop is redundant for the first attempt.
- **Recommendation**: Pass the already-prepared `PreparedComposition` into `run_harness_loop` to seed the first attempt, only re-composing for subsequent attempts if handlers (like `redirect`) change the source.

### 3. Missing Harness Integration Tests
While unit tests for the composition library are strong, there is a gap in integration testing for harness behavior within the new composition commands.

- **Gap**: There are no CLI integration tests verifying that `pre_checks`, `post_checks`, or `handlers` work correctly when triggered via `claudine compose` or `claudine inline-compose`.
- **Recommendation**: Add integration tests that exercise complex harness scenarios (e.g., a failing `pre_check` blocking execution, or a `retry` handler triggered by a `post_check` failure) to confirm the "wrapper-grade" claim.

## Technical Improvements & Ergonomics

### 1. Explicit Interactivity Capability Check
The tech design noted that `inline-compose -i` is provider-gated. The current implementation in `composition.rs` checks `profile.supports_interactive_inline_closure()`.

- **Refinement**: Consider making this capability more visible in `claudine providers` or providing a more detailed error message when it fails, explaining *why* a specific provider cannot support inline interactive closure (e.g., "lacks assistant message capture").

### 2. Managed Fields Extensibility
Currently, `last_updated` is the only managed field. The library provides `default_managed_fields()`.

- **Suggestion**: Ensure that future managed fields (like `claudine_session_id` for resume) can be easily added to the `InlineClosurePlan` without breaking the deterministic rewrite logic.

### 3. Metadata for Resume
The spec requires capturing enough metadata for future `claudine resume` workflows.

- **Verification**: While `LiveStreamSink` dispatches events for logging, ensure that the `CompositionExecutionRequest` context (like the original `file_ref`) is included in the dispatched metadata so a resume session knows which file it belongs to.

## Conclusion

The refactor provides a much cleaner and more predictable composition model. However, the apparent removal of harness support from the standard wrapper path is a major change that should be either reversed or explicitly documented as a breaking change. Strengthening integration coverage for the harness-in-composition path will ensure that the unified pipeline remains robust as new features are added.
