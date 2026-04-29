# Compose Refactor

Read [drift.md](./drift.md) first. This spec replaces the older, looser description and defines the intended end state for Claudine composition.

## Problem

Composition currently exists behind too many partially overlapping entrypoints:

- `claudine compose <file-ref>`
- `claudine compose inline <file-ref>`
- `claudine compose-inline <file-ref>`
- `claudine <agent> --compose <file-ref>`
- `claudine <agent> --frontmatter-prompt <file-ref>`
- `claudine <agent> --prompt-file <file-ref>`

Those entrypoints no longer behave the same way. The drift is not cosmetic; it is architectural:

- top-level `compose` uses a smaller execution stack than wrapper composition
- wrapper composition has harness, MCP, streaming, retry/resume/redirect, and richer reporting
- chained `--compose` currently decides harness activation from raw frontmatter instead of the effective composed frontmatter
- the user-facing docs still describe several of these paths as equivalent when they are not

The result is a surface area with too much baggage, too much drift, and no single composition contract.

## Goals

1. Reduce composition to a small, explicit CLI surface.
2. Make direct composition and inline composition first-class top-level commands.
3. Route all composition through one shared execution pipeline.
4. Ensure composition gets the same wrapper features regardless of how provider selection happens.
5. Remove ambiguous or redundant composition-related wrapper switches.
6. Define provider-selection behavior, interactivity, validations, handlers, and mutation rules precisely enough to implement and test.

## Non-Goals

1. This refactor does not redesign Darkmatter composition itself.
2. This refactor does not redesign existing freshness or content policy features for inline documents.
3. This refactor should capture the metadata needed for future generic resume UX, but a new standalone `claudine resume` command is not required to land this work.
4. This refactor does not require adding structured stream support to providers that do not already expose an appropriate machine-readable mode.

## Terms

### Direct Composition

Direct composition means:

1. Resolve a Markdown file reference.
2. Compose the document with Darkmatter.
3. Use the composed document content as the prompt sent to a provider.
4. Do not mutate the source file as part of normal completion.

### Inline Composition

Inline composition means:

1. Resolve a Markdown file reference.
2. Read the `prompt` frontmatter property from that file.
3. Compose that `prompt` value with Darkmatter.
4. Send the composed prompt to a provider with instructions to update the same file's body.
5. Rebuild the file from the preserved frontmatter plus Claudine-managed metadata updates plus the new body.

### Provider Selection

Provider selection answers "which provider will run this composition?"

It is separate from prompt-session interactivity. Choosing a provider interactively is not the same thing as running the provider in interactive chat mode.

### Prompt-Session Interactivity

Prompt-session interactivity answers "does the provider run as an interactive session or as a non-interactive prompt?"

This is controlled by `--interactive` / `-i` on the composition commands. Provider selection may still require user input even when the resulting provider session is non-interactive.

## Canonical CLI Surface

After this refactor, the composition surface is reduced to two canonical commands:

1. `claudine compose <file-ref>`
2. `claudine inline-compose <file-ref>`

### Explicit Provider Selection

Both commands support eager provider selection but default to interactive user choice on the Agent used at runtime.

- `--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, and similar

### Retired Composition Entry Points

The following signatures are retired by this refactor:

- `claudine <agent> --compose <file-ref>`
- `claudine <agent> --frontmatter-prompt <file-ref>`
- `claudine <agent> --prompt-file <file-ref>`
- `claudine compose inline <file-ref>`
- `claudine compose-inline <file-ref>`

There are no active users, so this refactor does not need a deprecation period. Remove the old paths rather than keeping aliases that preserve drift.

## Core Architectural Requirement

Top-level composition must stop owning its own reduced execution path.

Instead:

1. The top-level command resolves the composition mode and provider-selection intent.
2. It builds a shared composition execution request.
3. That request is executed by the same wrapper-grade pipeline used for provider execution.

This is the most important requirement in the refactor.

If `compose` and `inline-compose` do not ultimately run through the same wrapper-grade pipeline, the drift will reappear.

## Shared Composition Pipeline

Both `compose` and `inline-compose` must use the same high-level phases:

1. Resolve
2. Prepare
3. Select Provider
4. Launch
5. Closure

### 1. Resolve

Resolve the user-supplied file reference using Claudine's existing file-reference behavior.

Both modes must fail before provider launch when:

- the reference cannot be resolved
- the target is not a valid Markdown document

### 2. Prepare

Preparation produces the effective prompt input plus the effective frontmatter used by validations and handlers.

#### Direct Composition Prepare Rules

Direct composition:

- composes the entire document
- uses the composed document body as the provider prompt
- preserves the effective composed frontmatter for later harness detection

#### Inline Composition Prepare Rules

Inline composition:

- requires the source document to contain a `prompt` frontmatter property
- composes that `prompt` value
- builds the provider prompt from the composed `prompt` plus Claudine's inline-update instructions
- preserves the effective composed frontmatter for later harness detection
- captures pre-run state needed for closure validation and file rewrite

#### Effective Frontmatter Rule

Harness detection and harness parsing must always use the effective composed frontmatter, not raw source frontmatter.

That rule applies equally to:

- direct composition
- inline composition

This explicitly fixes one of the drift issues documented in [drift.md](./drift.md).

### 3. Select Provider

Provider selection precedence is:

1. Explicit provider from CLI (`--agent` or provider convenience flag)
2. Single installed provider, if there is only one
3. `agent` frontmatter hint from the source document
4. Favorite provider from config
5. Interactive chooser when a TTY is available
6. Error

Additional rules:

- `--exclude <provider>` applies only when provider selection is not explicit.
- If the frontmatter `agent` hint resolves to exactly one installed provider, use it.
- If the frontmatter `agent` hint is `true` or `interactive`, use the interactive chooser when a TTY is available.
- If the frontmatter `agent` hint is ambiguous and a TTY is available, open the chooser narrowed to valid matches.
- If the frontmatter `agent` hint is ambiguous and no TTY is available, return an error.
- If the hinted or favorite provider is not installed, continue through the precedence order rather than silently inventing a different rule.

### Provider Failure Semantics

This refactor intentionally removes "automatic rerun on another provider after provider execution fails" as a core behavior.

Reason:

- once a provider has launched, the prompt may already have caused side effects
- silently replaying the same request against another provider is not a safe default

So:

- provider selection fallback may happen before provider launch
- automatic provider retry must not happen after a provider session has already started unless a future feature explicitly opts into that behavior

This replaces the older, ambiguous contract.

### 4. Launch

Once the provider is selected, both composition modes must inherit the normal wrapper-grade behavior for that provider, including when supported:

- environment planning and sanitization
- MCP composition and tag handling
- structured streaming
- captured metadata such as session ID, model, tokens, and similar details
- harness execution
- handler-driven retry, resume, redirect, or deviate flows
- wrapper diagnostics and summaries

Top-level composition is allowed to choose the provider later. It is not allowed to use a weaker execution engine.

### 5. Closure

Closure rules differ by mode.

#### Direct Composition Closure

Direct composition has no file-mutation closure step.

It still participates in:

- post-checks
- timeout handling
- handler resolution
- final reporting

#### Inline Composition Closure

Inline composition must validate and rewrite the target file deterministically.

Required behavior:

1. Claudine captures the original frontmatter and original body before provider launch.
2. Claudine captures the provider result.
3. Claudine derives the replacement body from that result.
4. Claudine rewrites the file itself.

The agent is not trusted to mutate the file directly.

Required inline closure checks:

- the resulting body must not be empty
- the resulting body must be materially different from the original body
- the file rewrite must preserve original frontmatter values except for Claudine-managed fields

Claudine-managed fields are:

- `last_updated`
- any existing internal bookkeeping fields already owned by Claudine for inline documents

If the provider response tries to mutate frontmatter, Claudine ignores those mutations and rewrites the document using the preserved frontmatter plus managed-field updates.

This is clearer and safer than allowing partial agent-authored frontmatter edits and then trying to repair them afterward.

## Interactivity Rules

Both canonical composition commands default to non-interactive provider execution.

`--interactive` / `-i` changes only the provider session mode. It does not disable the shared composition pipeline.

Implications:

- `claudine compose <file>` without `-i` runs as a non-interactive prompt
- `claudine inline-compose <file>` without `-i` runs as a non-interactive prompt
- `claudine compose -i <file>` still performs composition first, then starts the provider interactively with the composed prompt as the first prompt
- `claudine inline-compose -i <file>` still performs inline prompt preparation first, then starts the provider interactively with that prepared prompt as the first prompt

## Structured Output and Metadata

For non-interactive provider execution, Claudine should prefer structured output paths whenever the selected provider supports them.

That structured path should be used to capture:

- session ID
- model
- token counts
- other provider metadata already supported by the wrapper system

Rendered user-facing output should continue to flow through Claudine's terminal rendering path where supported.

This requirement applies to composition because composition now uses the same wrapper-grade launch path. It is not a demand to invent new structured modes for providers that do not already support them.

## Validations, Timeouts, and Handlers

Validations and handlers are available to both composition modes through the shared harness pipeline.

### Built-In Prep Validations

Built-in validations should be expressed in the same conceptual stage model as user-defined validations.

Required built-in prep validations:

- file resolves to a Markdown document
- inline composition requires `prompt`
- inline composition requires that the target file is writable by the current filesystem and by the provider sandbox model when that can be determined

### Built-In Closure Validations

Required built-in closure validations:

- inline composition produced a non-empty body
- inline composition actually changed the body

### Harness Source of Truth

The harness source of truth is the effective composed frontmatter, not the raw file on disk before composition.

That means:

- `pre_checks`
- `post_checks`
- `timeout`
- `handle`
- `handle_*`

must all be read from the effective composed state.

## Resume Metadata

This refactor must preserve enough metadata from non-interactive composition runs to support future follow-up and resume workflows.

Required captured data:

- provider
- session ID, when the provider exposes one
- timestamp
- enough local execution context to present a useful recent-session list later

Creating a standalone `claudine resume` subcommand is a follow-on feature, not a blocker for this refactor. The refactor should not broaden scope more than necessary.

## Documentation Changes Required by This Refactor

Implementation is not complete until the docs stop describing the old drifted behavior.

At minimum update:

- [claudine/README.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/README.md)
- [claudine/docs/topics/composition.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md)

Those docs must describe:

- the new canonical commands
- the retirement of wrapper composition switches
- the difference between provider selection and provider-session interactivity
- the fact that composition now uses one shared execution pipeline

## Acceptance Criteria

The refactor is complete when all of the following are true:

1. Only `claudine compose <file-ref>` and `claudine inline-compose <file-ref>` remain as public composition entrypoints.
2. Top-level composition no longer owns a reduced execution implementation.
3. Both composition modes inherit wrapper features such as harness, MCP composition, streaming, and reporting.
4. Harness detection for composition uses effective composed frontmatter.
5. Inline composition rewrites the document deterministically and preserves frontmatter except for Claudine-managed fields.
6. Provider selection behavior is deterministic and documented.
7. Automatic cross-provider rerun after provider failure is not part of the default behavior.
8. Documentation and tests reflect the new contract.

## Test Requirements

Add or update integration coverage for at least the following:

1. `claudine compose <file-ref>` exercises the same harness-capable execution stack as the former wrapper compose path.
2. `claudine inline-compose <file-ref>` exercises the same harness-capable execution stack as the former wrapper inline path.
3. Effective composed frontmatter can enable harness behavior even when the raw source frontmatter would not.
4. `--interactive` starts an interactive provider session after composition, not instead of composition.
5. Inline composition preserves frontmatter while updating body and managed metadata.
6. Explicit provider selection bypasses interactive selection.
7. Ambiguous provider hints fail cleanly without a TTY.

## Assumption Called Out Explicitly

This spec assumes `--prompt-file` is intentionally being removed as part of composition simplification rather than being preserved as a separate non-composition prompt feature.

If that assumption is wrong, that decision should be corrected now in the spec rather than preserved as an implementation-side surprise.
