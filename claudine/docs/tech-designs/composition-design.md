# Composition Design

## Summary

This document turns the high-level feature brief in [composition.md](../features/composition.md) into an implementable design for Claudine composition workflows.

The design keeps the two product modes from the feature brief:

1. **Inline composition**
   - Read a Markdown document.
   - Compose the frontmatter `prompt` field through Darkmatter.
   - Run a non-interactive agent session.
   - Replace the document body with the agent output.
   - Update `last_updated` in frontmatter.
2. **Chained composition**
   - Read a Markdown document.
   - Compose the full document through Darkmatter.
   - Send the composed document to a non-interactive agent session.
   - Do not mutate the source file.

The main design change from the feature brief is consolidation:

- the existing wrapper-level `--prompt-file` behavior already covers explicit-agent chained composition, so this design does **not** introduce a second wrapper flag with the same meaning
- new work is focused on:
  - `--frontmatter-prompt` / `--fp` for inline composition on explicit wrapper commands
  - a new top-level `compose` command for agent-selected chained and inline composition
  - shared composition services below both entry points

## Review Of The Feature Brief

The current feature brief is directionally correct but leaves five implementation-critical gaps.

### 1. Wrapper-level chained composition already exists

Claudine already ships `--prompt-file` in the wrapper layer. It:

- resolves a Markdown file
- composes it through Darkmatter
- injects the composed body as the provider prompt
- exports residual frontmatter to child env vars

That is materially the same problem space as wrapper `--compose <file>`.

Adding `--compose` as a second wrapper switch would create overlapping behavior, documentation drift, and duplicate tests. The recommended design is:

- keep `--prompt-file` as the wrapper-level chained composition switch
- add `--frontmatter-prompt` for the inline file-mutation workflow
- reserve `compose` for the new top-level command

If product naming later insists on `--compose`, it should be a thin alias to `--prompt-file`, not a separate implementation.

### 2. File-reference semantics must be unified

The feature brief says composition should use `biscuit-file`'s `FileReference`, but the current wrapper prompt-file resolver uses custom semantics such as package-relative `./...`.

For composition v1, the canonical reference model is `FileReference`, and any bespoke Claudine resolver should be removed rather than maintained in parallel:

- relative paths
- absolute paths
- `@repo-or-home/path.md`
- `!package-root/path.md`
- `%` recursive search
- `vault:...`
- `{{ENV_VAR}}` interpolation

This keeps composition aligned with the monorepo's existing file-resolution story and avoids a second path mini-language. There should be exactly one file-reference implementation for composition in Claudine, and it should come from `biscuit-file`.

### 3. Agent selection needs a deterministic algorithm

The feature brief describes selection rules, but not an implementation-ready precedence model. This design makes that explicit and reuses:

- `Provider` parsing/fuzzy matching
- installed-provider detection from `sniff`
- persisted provider preference ordering from `settings.linking.preference`

### 4. Inline composition needs an atomic mutation contract

The brief says the body is replaced and `last_updated` is set, but it does not specify:

- how output is captured
- when the file is written
- whether partial provider output can leak into the file
- what happens on provider failure

This design requires full output capture and atomic writes so inline composition never corrupts the source document.

### 5. Composition should reuse shipped wrapper behavior instead of re-implementing it

The current wrapper stack already knows how to:

- map universal flags to provider-native flags
- build sanitized child environments
- inject prompts by args or stdin
- manage non-interactive defaults
- support MCP runtime composition

Composition should plug into that machinery rather than spawn ad hoc provider commands.

## Goals

1. Add inline composition for explicit wrapper invocations.
2. Add a top-level `compose` command that can select the agent automatically.
3. Reuse Darkmatter composition and `biscuit-file` file references.
4. Reuse Claudine's wrapper/provider execution logic.
5. Keep non-interactive behavior deterministic and safe for scripts.
6. Make inline mutations atomic and easy to test.

## Non-Goals

1. Replacing or renaming the existing `--prompt-file` feature.
2. Adding interactive multi-turn chat behavior to composition commands.
3. Changing Darkmatter composition semantics.
4. Translating inline composition into provider-specific prompt-file features.
5. Supporting non-Markdown source files.
6. Streaming partial inline results into the destination file.

## User-Facing CLI Design

### Explicit-Agent Wrapper Flow

#### Inline composition

```sh
claudine codex --frontmatter-prompt @claudine/docs/foo.md
claudine gemini --fp !docs/research.md
```

Behavior:

1. Resolve the file reference.
2. Load the Markdown document.
3. Require a string-valued `prompt` frontmatter property.
4. Compose that prompt through Darkmatter using the document frontmatter and source path as context.
5. Run the selected provider non-interactively.
6. Replace the document body with the provider output.
7. Set `last_updated` to the local date in `YYYY-MM-DD` format.
8. Persist the document atomically.

This mode treats the wrapper subcommand as the authoritative provider choice. The source document's `agent` frontmatter is ignored.

#### Chained composition

Explicit-agent chained composition continues to use the shipped wrapper switch:

```sh
claudine opencode --prompt-file @claudine/prompts/review.md --non-interactive
```

This is the explicit-agent equivalent of chained composition and does not mutate the file.

### Top-Level Compose Command

#### Chained composition

```sh
claudine compose @claudine/prompts/review.md
```

Default behavior:

1. Resolve the file reference.
2. Compose the full document through Darkmatter.
3. Select the provider using the composition agent-selection algorithm.
4. Run the provider non-interactively.
5. Print the provider output to stdout.

#### Inline composition

```sh
claudine compose inline @claudine/docs/research.md
```

Behavior is identical to wrapper `--frontmatter-prompt`, except agent selection comes from the source document and config rather than the wrapper subcommand.

#### Top-level compose flags

```text
claudine compose [inline] <file-ref> [--exclude <provider> ...] [--interactive]
```

- `--exclude <provider>` may be repeated and removes providers from automatic selection.
- `--interactive` forces a provider picker even when automatic selection could succeed. The computed default provider is preselected.

The top-level command intentionally keeps a narrow surface in v1. Advanced per-provider overrides remain available through explicit wrapper commands.

### Conflict Rules

#### Wrapper `--frontmatter-prompt`

This switch conflicts with:

- `--prompt-file`
- a provider-native prompt already present in passthrough args
- provider-native prompt files passed through directly

It is allowed with:

- `--model`
- `--system-prompt`
- `--sandbox`
- `--repo`
- `--mcp`
- `--use`

#### Top-level `compose`

The top-level command does not accept arbitrary passthrough provider args in v1. That constraint is deliberate: it keeps auto-selection deterministic and avoids inventing a second wrapper argument parser.

## Functional Semantics

### Inline Composition

Input:

- Markdown file with frontmatter
- required `prompt` property in frontmatter

Composition contract:

1. Parse the Markdown file into `Markdown`.
2. Read `frontmatter["prompt"]` as a string.
3. Build a temporary `Markdown` value:
   - frontmatter = original document frontmatter
   - content = `prompt` field text
4. Run `transform_with(TransformOptions::new().with_source_file(source_path))`.
5. Use the transformed body as the provider prompt.

Mutation contract:

- body becomes the provider stdout exactly as returned
- frontmatter is preserved, except `last_updated` is inserted or replaced
- `prompt` remains in frontmatter
- residual frontmatter is **not** exported to child env vars in this mode
- writes are atomic via temp-file + rename in the source directory
- the source file is only written after provider exit code `0`

### Chained Composition

Input:

- Markdown file with or without frontmatter

Composition contract:

1. Load the full document with Darkmatter `Markdown`.
2. Run `transform_with(TransformOptions::new().with_source_file(source_path))`.
3. Use `transformed.content()` as the provider prompt.

Mutation contract:

- no filesystem mutation
- residual frontmatter is **not** exported to child env vars in this mode
- stdout is emitted only after the provider succeeds

## Agent Selection Design

### Selection Scope

Agent selection only applies to the top-level `compose` command. Explicit wrapper commands already imply the provider.

### Candidate Set

Start from all installed providers that:

- map to a runnable wrapper profile
- are not excluded by `--exclude`

That means `RooCode` is automatically removed because it is not a standalone wrapped CLI.

### Precedence Order

1. If `--interactive` is set:
   - compute the default candidate using the remaining rules
   - show an interactive picker with only installed, non-excluded providers
2. Else if `AGENT` is set:
   - resolve it using provider fuzzy matching
   - if it matches one provider, use it
   - if it matches multiple providers, require interactive selection
   - if it matches none, ignore it and continue
3. Else if the source document has an `agent` frontmatter property:
   - string:
     - fuzzy match against providers
     - one match: use it
     - multiple matches: require interactive selection
     - zero matches: error
   - boolean `true` or string `interactive`:
     - require interactive selection
   - any other type:
     - error
4. Else:
   - use persisted provider preference ordering from:
     - repo config first
     - user config second
   - filter that ordering to installed, non-excluded runnable providers
   - try providers in ranked order until one succeeds

### Interactive Selection Rules

Interactive selection is allowed only when:

- stdin is a terminal
- stdout is a terminal

Otherwise any path that requires a picker fails with an actionable error.

### Provider Retry Rules

Retries are only used for preference-based automatic selection. Claudine does **not** retry after:

- an explicit `AGENT` override
- a source-file `agent` hint
- an interactive user selection

Rationale:

- retries make sense for fallback preferences
- retries are surprising when the user or source document explicitly chose a provider

## Architecture

### Responsibility Split

#### `claudine/lib`

Owns composition planning logic that does not require direct process spawning:

- file-reference resolution via `biscuit-file::FileReference`
- source document loading
- frontmatter validation
- prompt preparation
- agent-selection policy
- inline mutation planning
- composition error types

#### `claudine/cli`

Owns execution-time concerns:

- command-line parsing
- wrapper-profile prompt delivery
- child-process execution
- output capture
- atomic file writes
- dry-run/log presentation

This split is important because the provider wrapper implementation already lives in the CLI crate.

### New Library Module

Add `claudine/lib/src/composition/` with:

- `mod.rs`
- `error.rs`
- `resolve.rs`
- `source.rs`
- `prepare.rs`
- `select.rs`
- `types.rs`

#### Core types

```rust
pub enum CompositionMode {
    InlineFrontmatterPrompt,
    ChainedDocument,
}

pub struct CompositionRequest {
    pub mode: CompositionMode,
    pub file_ref: String,
    pub explicit_provider: Option<Provider>,
    pub excluded: BTreeSet<Provider>,
    pub force_interactive_selection: bool,
}

pub struct ResolvedCompositionSource {
    pub original_ref: String,
    pub resolved_path: PathBuf,
    pub markdown: Markdown,
}

pub struct PreparedPrompt {
    pub mode: CompositionMode,
    pub resolved_path: PathBuf,
    pub prompt: String,
    pub source_agent_hint: Option<serde_json::Value>,
}

pub enum SelectionReason {
    ExplicitProvider,
    EnvironmentOverride,
    FrontmatterHint,
    InteractiveChoice,
    PreferenceFallback,
}

pub struct SelectedProvider {
    pub provider: Provider,
    pub reason: SelectionReason,
}
```

### Wrapper Refactor

The current `wrap/prompt_file.rs` code should be split into:

- shared composition helpers moved into `claudine::composition`
- wrapper-specific prompt-file env export behavior left in CLI
- bespoke prompt-file path resolution removed entirely

Shared logic that moves out:

- Markdown-file validation after `FileReference` resolution
- file-reference resolution orchestration around `biscuit-file::FileReference`
- Darkmatter composition

Wrapper-specific logic that stays:

- frontmatter-to-env conversion for `--prompt-file`
- provider-native prompt conflict detection
- dry-run display

Wrapper-specific logic that must be deleted:

- `wrap/prompt_file.rs::resolve_prompt_file`
- `wrap/prompt_file.rs::resolve_bare_filename`
- `wrap/prompt_file.rs::search_repo_for_filename`
- `wrap/prompt_file.rs::walk_dir_recursive`

If current wrapper UX depends on behavior those functions provide, the correct fix is to extend `biscuit-file::FileReference` and then consume that behavior from Claudine. Claudine should not keep a second resolver implementation.

### File-Reference Migration Requirement

This is a hard migration rule for composition work:

1. New composition code must call `biscuit-file::FileReference`.
2. Existing Claudine composition-related custom resolvers must be removed.
3. If `FileReference` is missing behavior Claudine needs, add it to `biscuit-file` first.
4. Claudine should not preserve legacy composition resolver behavior by forking resolver logic locally.

### New CLI Command

Add `claudine/cli/src/commands/compose.rs` and wire it into `args.rs` as:

```rust
Compose(commands::compose::ComposeArgs)
```

Recommended shape:

```rust
pub struct ComposeArgs {
    pub interactive: bool,
    pub exclude: Vec<String>,
    pub command: Option<ComposeSubcommand>,
    pub file: String,
}

pub enum ComposeSubcommand {
    Inline { file: String },
}
```

Implementation detail:

- the parser should normalize to a single `CompositionMode` plus `file_ref`
- `claudine compose <file>` means chained mode
- `claudine compose inline <file>` means inline mode

### Output Capture Design

Inline composition requires full provider output capture. Chained composition also benefits from capture because it allows fallback retries without leaking partial failed output to stdout.

Add a second execution helper alongside `run_child`:

```rust
pub(crate) fn run_child_capture(...) -> Result<CapturedChildOutput>
```

```rust
pub(crate) struct CapturedChildOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}
```

Behavior:

- child stdout and stderr are piped
- configured noise filtering still applies
- stdin seeding still works
- no output is printed live

This helper becomes the execution path for:

- wrapper `--frontmatter-prompt`
- top-level `compose`

The existing `run_child` remains for ordinary wrappers.

### Inline File Persistence

Inline persistence flow:

1. Read and validate source file.
2. Prepare prompt.
3. Execute provider and capture stdout.
4. Reconstruct `Markdown` with:
   - original frontmatter
   - updated `last_updated`
   - captured stdout as body
5. Serialize via Darkmatter string output.
6. Write to `<file>.tmp` in the same directory.
7. `rename()` temp file over the original.

If serialization or rename fails, the original file remains untouched.

### Error Model

Add composition-specific errors in `claudine/lib/src/composition/error.rs` using `thiserror`.

Recommended variants:

- `InvalidReference`
- `FileNotFound`
- `NotMarkdown`
- `MarkdownLoad`
- `PromptPropertyMissing`
- `PromptPropertyWrongType`
- `ComposeFailed`
- `NoRunnableProviders`
- `AgentHintInvalid`
- `AgentHintAmbiguous`
- `InteractiveSelectionRequired`
- `ProviderLaunchFailed`
- `ProviderRunFailed`
- `AtomicWriteFailed`

### MCP And Other Wrapper Features

Composition runs reuse the wrapper stack, so the following remain available for explicit-agent inline composition:

- system prompt injection
- model selection
- repo shadow-home isolation
- MCP runtime composition
- sandbox mapping

The top-level `compose` command does not expose all of those flags in v1, but it should execute through the same wrapper planning code so future expansion is mechanical rather than architectural.

## Testing Plan

### Library Tests

- file-reference resolution through `FileReference`:
  - relative
  - absolute
  - `@`
  - `!`
  - `%`
  - interpolated env refs
- missing `prompt`
- non-string `prompt`
- inline prompt composition uses original frontmatter state
- chained composition uses full document body
- provider selection precedence
- exclusion filtering
- no runnable providers

### CLI Tests

- wrapper help includes `--frontmatter-prompt`
- `--frontmatter-prompt` conflicts with `--prompt-file`
- inline wrapper uses captured output and rewrites body
- inline wrapper updates `last_updated`
- source file is unchanged on non-zero exit
- `claudine compose <file>` selects provider from preferences
- `claudine compose inline <file>` respects `agent` frontmatter
- ambiguous `agent` hint errors in non-interactive sessions
- `--interactive` forces picker when tty is available

### Integration Tests

Use stub binaries on `PATH` the same way wrapper tests already do, so the suite can verify:

- prompt delivery shape
- captured stdout
- fallback to next preferred provider
- no partial stdout on failed first provider

## Rollout Plan

1. Extract shared composition helpers from wrapper prompt-file code into `claudine::composition`.
2. Add captured child execution support.
3. Implement wrapper `--frontmatter-prompt`.
4. Implement top-level `compose` chained mode.
5. Implement top-level `compose inline`.
6. Update docs:
   - [composition.md](../features/composition.md)
   - [prompt-file-design.md](../prompt-file-design.md)
   - CLI help/docs once the command ships

## Open Questions

1. Should wrapper `--prompt-file` eventually gain `--compose` as a documented alias, or should the product surface stay consolidated on `--prompt-file`?
2. Should top-level `compose` eventually expose a subset of universal wrapper flags such as `--model` and `--system-prompt`?
3. Should inline composition trim a single trailing newline from provider output before persistence, or preserve stdout byte-for-byte?
4. Should `last_updated` use system local time or repo-configured time if Claudine later gains explicit timezone settings?
