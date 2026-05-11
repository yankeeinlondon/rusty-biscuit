# Compose Drift Review

Reviewed: 2026-03-27 11:05:18 PDT

## Intended Contract

The current feature spec says these two entrypoints were meant to perform largely the same task:

- `claudine compose <file-ref> ...`
- `claudine <agent> --compose <file-ref> ...`

The only intended difference was provider selection timing:

- `claudine compose` defers provider choice until runtime
- `claudine <agent> --compose` fixes the provider in the CLI invocation

That contract is also reflected in `claudine/README.md`, which currently says both syntaxes provide the same functionality.

## Current State

That contract no longer holds.

The two commands still share the same composition primitives:

- `claudine::composition::resolve_composition_source(...)`
- `claudine::composition::prepare_chained_prompt(...)`

After that, they diverge into materially different execution paths.

## Execution Path Comparison

### `claudine compose <file-ref>`

The top-level compose command currently:

1. Resolves the Markdown source
2. Prepares the chained prompt
3. Selects a provider using:
   - explicit interactive selection if requested
   - `AGENT`
   - frontmatter `agent`
   - fallback preference order
4. Builds a minimal provider argv/env
5. Forces non-interactive mode
6. Injects provider-specific captured-output flags
7. Executes the child directly with `run_child_capture()`
8. Prints captured output or rewrites the file for inline mode

Primary code:

- `claudine/cli/src/commands/compose.rs`

Important properties of this path:

- provider selection is unique to this path
- execution is always forced into non-interactive mode
- no full wrapper pipeline
- no MCP composition
- no wrapper header / reporting flow
- no harness detection
- no structured streaming path
- no handler-driven retry / resume / redirect loop

### `claudine <agent> --compose <file-ref>`

The wrapper-based compose path currently:

1. Enters the provider wrapper directly from the chosen subcommand
2. Resolves the Markdown source
3. Prepares the chained prompt
4. Injects the composed prompt through `apply_prompt_body(...)`
5. Preserves `--interactive` semantics, so the composed document can be the first prompt in an interactive session
6. Performs final argument validation
7. Builds the full child environment through wrapper env planning
8. Detects harness properties
9. Enables structured streaming when supported and appropriate
10. Runs the harness loop when validations / handlers are present
11. Supports retry / resume / redirect / deviate recovery behavior
12. Supports MCP composition and tag handling elsewhere in the wrapper flow

Primary code:

- `claudine/cli/src/commands/wrap/mod.rs`

Important properties of this path:

- full wrapper feature set
- supports interactive first-prompt compose sessions
- supports structured stream parsing
- supports harness-driven recovery
- participates in wrapper diagnostics and summary rendering

## Drift Findings

### 1. The two public entrypoints are no longer functionally equivalent

This is the main drift.

`claudine compose` does not delegate into the wrapper execution path. Instead it reimplements a smaller execution path in `run_provider_composition(...)`.

That means the top-level command is missing major behavior already available to `claudine <agent> --compose`, including:

- interactive composed first-prompt sessions
- harness activation
- retry / resume / redirect handlers
- structured stream execution
- MCP session composition
- wrapper-level execution reporting

This directly contradicts the current user-facing documentation that says the two syntaxes provide the same functionality.

### 2. Top-level compose still hardcodes preference order

`claudine compose` currently uses:

- Claude
- Codex
- Gemini

from `load_provider_preferences()`.

The docs describe this as using a favorite agent from config with fallback behavior. That wiring does not exist yet in the current implementation.

### 3. Top-level compose fallback retry does not trigger on normal provider failure

`execute_composition(...)` only falls through to the next provider when `run_provider_composition(...)` returns `Err`.

But `run_provider_composition(...)` returns `Ok(exit_code)` even when the provider exits non-zero.

Result:

- provider launch/setup failures can trigger fallback
- normal provider runtime failures do not

So the documented behavior of trying the next preferred provider after failure is not actually true for the common failure mode.

### 4. Wrapper chained compose harness detection reads raw frontmatter, not composed frontmatter

The wrapper compose path prepares the provider prompt from the composed document first.

Later, harness activation for chained compose is decided from:

- `source.markdown.frontmatter()`

rather than from the effective composed frontmatter returned by Darkmatter.

The issue is not about any one specific directive such as `imports`. The core problem is a state mismatch:

- the provider prompt is built from composed state
- harness activation is decided from raw source state

Result:

- if composition-time frontmatter transforms, substitutions, or overlays produce `pre_checks`, `post_checks`, `timeout`, `handle`, or `handle_*`
- those keys are not visible to the initial harness activation check
- the run can skip harness behavior that the effective composed document requested

`--prompt-file` does not have this specific problem because it preserves the composed frontmatter and uses that for harness detection. `--compose` currently does not.

This is narrower than the top-level path split, but it is still real drift inside the wrapper implementation itself.

## Test Signal

### What is covered

The current CLI integration coverage is concentrated in:

- `claudine/cli/tests/wrap_commands.rs`

That file exercises wrapper-side composition behavior, including:

- `codex_compose_response_validation_uses_captured_legacy_output`
- prompt-file handler retries
- redirect behavior
- inline frontmatter-prompt recovery

### What is not covered

I did not find integration coverage that invokes the top-level public entrypoint:

- `claudine compose ...`

So the current drift is not surprising:

- the wrapper path is actively exercised
- the top-level compose path appears to be largely unexercised at the CLI integration level

## Comparison Summary

### Shared behavior

- resolve file reference
- compose the document into a prompt
- deliver prompt to a provider

### `claudine compose` only

- provider auto-selection logic
- exclude list handling
- preference fallback logic
- interactive provider chooser

### `claudine <agent> --compose` only

- explicit provider binding from CLI
- interactive first-prompt compose sessions
- harness parsing and execution
- structured stream path
- MCP integration
- wrapper summary / diagnostics flow
- retry / resume / redirect recovery behavior

## Practical Conclusion

Today, `claudine compose <file-ref>` is not just a provider-selection veneer over the wrapper compose path.

It is a separate execution implementation with less capability.

If the product goal remains:

- same behavior
- different provider-selection model

then `claudine compose` should likely become a thin selector + delegator into the wrapper compose path rather than maintaining a second execution stack.

## Verification Notes

Reviewed against:

- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/lib/src/composition/prepare.rs`
- `claudine/lib/src/composition/select.rs`
- `claudine/README.md`
- `claudine/docs/topics/composition.md`
- `claudine/cli/tests/wrap_commands.rs`

Local test run:

- `cd claudine && just test`
- library tests passed
- CLI tests currently fail on `codex_prompt_file_retry_speaks_handler_say_message`
- that failure appears unrelated to the compose drift described here
