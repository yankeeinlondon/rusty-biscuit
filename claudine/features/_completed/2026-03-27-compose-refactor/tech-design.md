# Compose Refactor Tech Design

This document turns the compose refactor spec into an implementation-ready design for Claudine's CLI, composition library, and wrapper pipeline.

Primary inputs:

- `claudine/features/2026-03-26-compose-refactor/spec.md`
- `claudine/features/2026-03-26-compose-refactor/drift.md`
- current top-level composition code in `claudine/cli/src/commands/compose.rs`
- current wrapper execution flow in `claudine/cli/src/commands/wrap/mod.rs`
- current composition helpers in `claudine/lib/src/composition/`

The core design decision is simple: composition stops being a second-class executor. `compose` and `inline-compose` become thin command entrypoints that feed one wrapper-grade composition pipeline.

## Summary

The refactor has four major parts:

1. reduce the public CLI surface to `claudine compose <file-ref>` and `claudine inline-compose <file-ref>`
2. remove composition switches from provider wrapper subcommands
3. replace the current reduced top-level compose runner with a wrapper-grade composition executor
4. preserve effective composed frontmatter through the full pipeline so provider selection, harness parsing, validations, handlers, MCP, streaming, and closure all use the same composed state

The result is:

- one composition contract
- one provider-selection contract
- one launch path
- deterministic inline file rewriting owned by Claudine

## Goals

1. Match the spec's canonical CLI and behavior exactly.
2. Remove the current drift between `claudine compose` and wrapper composition.
3. Keep composition-specific logic in `claudine::composition` and provider execution logic in the wrapper layer.
4. Reuse existing wrapper capabilities rather than reimplementing them.
5. Make the inline rewrite path deterministic, testable, and independent of provider-authored filesystem mutation.

## Non-Goals

1. Redesigning Darkmatter composition semantics.
2. Redesigning the harness model introduced by the validations feature.
3. Adding generic structured stream support to providers that do not already expose machine-readable output.
4. Adding a standalone `claudine resume` command in this refactor.

## Explicit Assumption

This design follows the spec's explicit assumption: `--prompt-file` is removed as part of the composition simplification instead of being preserved as a separate public prompt-loading feature.

That means this refactor removes all of the following public entrypoints:

- `claudine <agent> --compose <file-ref>`
- `claudine <agent> --frontmatter-prompt <file-ref>`
- `claudine <agent> --prompt-file <file-ref>`
- `claudine compose inline <file-ref>`
- `claudine compose-inline <file-ref>`

If product direction changes on `--prompt-file`, that should be corrected at the spec level before implementation starts. It should not be silently preserved in code.

## Current Baseline

Today the implementation is split in two incompatible ways:

1. `claudine/cli/src/commands/compose.rs` resolves, prepares, selects, and launches providers through a reduced runner (`run_provider_composition`) that bypasses wrapper-grade behavior.
2. `claudine/cli/src/commands/wrap/mod.rs` has a richer composition path behind `--compose` and `--frontmatter-prompt`, but it still has internal drift because harness detection for inline/chained composition uses raw source frontmatter instead of the effective composed frontmatter.

Current concrete problems:

- top-level compose hardcodes fallback preferences in `load_provider_preferences()`
- top-level compose still considers `AGENT` env as a composition selection source, which the new spec does not
- top-level compose retries other providers only on launch/setup errors, not normal provider failures
- wrapper compose and inline composition still rely on the provider to mutate the file in practice, then repair frontmatter afterward
- composition logic is spread across `compose.rs`, `wrap/mod.rs`, `wrap/prompt_file.rs`, and `claudine::composition` without one shared request model

## Target Architecture

The target architecture is a five-stage pipeline shared by both canonical commands:

1. Resolve
2. Prepare
3. Select Provider
4. Launch
5. Closure

Stages 1-3 live primarily in `claudine::composition`.

Stages 4-5 live in a new wrapper-side composition executor that reuses the normal wrapper machinery for:

- env planning and sanitization
- MCP composition and `#tag` handling
- structured streaming
- harness parsing and execution
- retry/resume/redirect/deviate handling
- reporting and terminal output

## Recommended Module Layout

### Library

Keep composition-specific state in `claudine/lib/src/composition/` and expand it:

```txt
claudine/lib/src/composition/
├── mod.rs
├── error.rs
├── prepare.rs
├── resolve.rs
├── select.rs
├── closure.rs        # new
└── types.rs
```

Recommended responsibilities:

- `resolve.rs`
  - resolve file references
  - validate Markdown-ness
  - load source text and parsed Markdown
- `prepare.rs`
  - produce prompt text
  - produce effective composed frontmatter
  - capture inline pre-run state for closure
- `select.rs`
  - deterministic provider selection
  - config-backed favorite provider resolution
  - interactive chooser narrowing rules
- `closure.rs`
  - inline response extraction
  - deterministic file reconstruction
  - managed-field updates
- `types.rs`
  - shared request/result structs

### CLI / Wrapper

Split composition execution out of the already-large wrapper module:

```txt
claudine/cli/src/commands/
├── compose.rs
└── wrap/
   ├── mod.rs
   ├── composition.rs   # new
   ├── env.rs
   ├── exec.rs
   ├── profile.rs
   └── repo_home.rs
```

`wrap/composition.rs` should own wrapper-grade composition orchestration and call into existing wrapper helpers instead of duplicating them.

## Public CLI Design

After the refactor, the public composition commands are:

1. `claudine compose <file-ref>`
2. `claudine inline-compose <file-ref>`

### CLI argument shape

`claudine/cli/src/args.rs` should expose:

- `Compose(commands::compose::ComposeArgs)`
- `InlineCompose(commands::compose::InlineComposeArgs)` with `#[command(name = "inline-compose")]`

Remove:

- `ComposeInline`
- `#[command(name = "compose-inline")]`
- `ComposeSubcommand::Inline`

### Shared composition args

Both commands should accept the same shared selection/session flags:

- `--interactive` / `-i`
- `--exclude <provider>` repeatable
- explicit provider convenience flags such as `--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`
- any existing output flags we intentionally keep, such as `--silent`, if they still fit the wrapper-grade path

Recommended parsing shape:

- define a shared `ProviderOverrideArgs`
- use a clap `ArgGroup` so only one explicit provider flag can be set
- normalize explicit flags into `Option<Provider>`

### Important semantic change

`--interactive` now means only:

- run the chosen provider session interactively

It does not mean:

- force provider selection to be interactive

Interactive provider choice is now only a selection fallback when the precedence rules require it and a TTY is available.

## Core Data Model

### Prepared composition

Replace the current `PreparedPrompt` with a richer prepared state:

```rust
pub struct PreparedComposition {
    pub mode: CompositionMode,
    pub resolved_path: PathBuf,
    pub prompt: String,
    pub effective_frontmatter: serde_json::Value,
    pub effective_agent_hint: Option<serde_json::Value>,
    pub closure: CompositionClosurePlan,
}
```

`effective_frontmatter` is the key fix. It becomes the source of truth for:

- harness detection
- harness parsing
- `agent` selection hint
- built-in prep/closure validation planning

### Closure plan

```rust
pub enum CompositionClosurePlan {
    Direct,
    Inline(InlineClosurePlan),
}

pub struct InlineClosurePlan {
    pub original_document_text: String,
    pub original_frontmatter_hash: u64,
    pub original_body_hash: u64,
    pub managed_fields: BTreeSet<String>,
}
```

Managed fields initially include:

- `last_updated`
- any existing Claudine-owned inline bookkeeping fields already recognized by the product

### Composition request

The top-level commands should build a wrapper-ready request instead of launching directly:

```rust
pub struct CompositionExecutionRequest {
    pub mode: CompositionMode,
    pub file_ref: String,
    pub prepared: PreparedComposition,
    pub explicit_provider: Option<Provider>,
    pub excluded: BTreeSet<Provider>,
    pub session_interactive: bool,
    pub silent: bool,
}
```

The selected provider does not need to be stored in the initial request if selection is performed in the wrapper executor, but the request must carry all information needed to apply the selection rules deterministically.

## Resolve Stage

The resolve stage remains in `claudine::composition::resolve_composition_source(...)`, but it must become the canonical resolver for both commands.

Required behavior:

1. resolve file reference using existing Claudine file-reference rules
2. fail if the file cannot be resolved
3. fail if the target is not Markdown
4. preserve both original text and parsed Markdown

No provider-specific behavior should happen in this stage.

## Prepare Stage

### Direct composition

Direct composition should:

1. compose the entire Markdown document through Darkmatter
2. use the composed document body as the provider prompt
3. persist the composed frontmatter as `effective_frontmatter`

### Inline composition

Inline composition should:

1. require a string `prompt` property in frontmatter
2. create a temporary Markdown document using the source frontmatter plus the `prompt` value as body
3. compose that temporary document through Darkmatter
4. use the composed body plus inline-update instructions as the provider prompt
5. persist the composed frontmatter as `effective_frontmatter`
6. capture pre-run state in `InlineClosurePlan`

### Inline prompt contract change

The inline prompt must stop telling the provider to mutate the file directly.

Instead, the prompt contract becomes:

- return the replacement Markdown body
- do not emit frontmatter
- do not edit the source file

This is a meaningful change from the current implementation and is required by the spec's deterministic closure rules.

### Guardrails

The current guardrails loaded by `load_or_create_guardrails(...)` should be updated to reflect the new contract:

- no file mutation by the provider
- output body content only
- frontmatter is Claudine-owned

## Provider Selection Design

Provider selection moves to the exact precedence required by the spec:

1. explicit provider from CLI
2. single installed provider
3. `agent` hint from effective composed frontmatter
4. favorite provider from config
5. interactive chooser when a TTY is available
6. error

### What changes from current behavior

1. `AGENT` env is removed from composition selection precedence.
2. automatic provider retry after provider launch is removed.
3. `--exclude` only matters when provider selection is not explicit.
4. the `agent` hint is read from effective composed frontmatter, not raw source frontmatter.

### Config-backed favorite provider

Replace `load_provider_preferences()` in `claudine/cli/src/commands/compose.rs` with shared config loading that reads:

- repo config first when in a repo: `<repo>/.claudine/config.json`
- user config otherwise: `~/.claudine/config.json`

Source of truth:

- `HookerConfig.settings.linking.preference`

This logic should move into shared library code so `compose` does not own a CLI-local config reader.

### Agent hint behavior

Recommended exact behavior:

- if the composed `agent` hint matches exactly one installed provider, use it
- if it is `true` or `"interactive"`, request interactive chooser
- if it matches multiple installed providers, open a narrowed chooser when TTY is available
- if it matches multiple installed providers and no TTY exists, error
- if it matches known providers but none are installed, continue to favorite/chooser/error
- if it is an invalid type or does not match any known provider name at all, return a validation error

This preserves the spec's "continue when hinted provider is not installed" rule without silently accepting malformed hints.

## Wrapper-Grade Launch Design

Introduce a new wrapper-side entrypoint:

```rust
pub(crate) fn execute_composition_request(
    request: CompositionExecutionRequest,
    verbose: u8,
) -> Result<i32>
```

This function should:

1. resolve installed providers
2. select a provider
3. acquire the wrapper profile for that provider
4. build the same env/MCP/stream/harness plan used by normal wrapper execution
5. launch through the normal structured or captured output path
6. return a rich execution result for closure and final reporting

### Critical reuse boundary

The composition executor may choose the provider later than normal wrapper commands, but once provider selection is done it must use the same machinery already used by wrapper execution:

- `env::build_child_env(...)`
- MCP runtime composition
- `WrapperProfile` prompt delivery and mode configuration
- structured parsing in `exec.rs`
- harness loop and handler resolution
- summary/reporting flow

It must not have its own smaller `run_provider_composition(...)` equivalent.

## Harness Integration

Harness activation should use `prepared.effective_frontmatter` for both direct and inline modes.

That fixes the current bug in `claudine/cli/src/commands/wrap/mod.rs`, where inline/chained composition still derive harness enablement from `source.markdown.frontmatter()` instead of the composed state.

### Built-in validations

Built-in composition checks should be normalized into the same conceptual model as harness validations.

Prep-time built-ins:

- resolved file is Markdown
- inline mode requires `prompt`
- inline target is writable by the filesystem
- inline target is writable under provider sandbox constraints when determinable

Closure built-ins:

- inline result body is non-empty
- inline result body differs materially from the original body

Recommended implementation:

- prepend built-in rules to parsed harness rules
- keep them marked as system-owned so rendering can explain them clearly

## Inline Closure Design

Inline closure is the most important behavioral change.

### Source of replacement body

The replacement body must come from the provider result captured by Claudine, not from whatever happens to be on disk after the provider exits.

Preferred extraction order:

1. structured summary assistant text
2. provider-specific captured last-message path
3. parsed captured stdout when that path is already trusted for the provider

### Rewrite algorithm

Given:

- `InlineClosurePlan`
- captured replacement body
- original document text

Claudine should:

1. trim and validate the replacement body
2. reject empty body
3. compare with original body and reject unchanged output
4. reconstruct the Markdown document using original frontmatter text
5. update only managed fields
6. write atomically to the same file

### Frontmatter preservation

The provider never gets authority over frontmatter.

If the provider response includes frontmatter-looking content:

- treat it as invalid inline output and strip or reject according to the response parser
- do not merge it into source frontmatter

The rewrite path should preserve frontmatter formatting as much as possible by reusing the existing text-preserving rewrite helpers now in `wrap/mod.rs`, moved into `claudine::composition::closure`.

### Managed field update

`last_updated` should continue to be set to the local current date in `YYYY-MM-DD` format during a successful inline rewrite.

## Interactive Session Behavior

The spec requires that composition still happens before interactive launch.

That means:

- `compose -i` prepares the prompt first, then starts the chosen provider interactively with that prompt as the first prompt
- `inline-compose -i` prepares the inline prompt first, then starts the chosen provider interactively with that prompt as the first prompt

### Capability boundary for inline interactive

Inline closure still needs a captured final assistant body. This is only possible when the selected provider exposes a retrievable final assistant message for interactive runs.

Therefore:

- direct composition can always run interactively
- inline interactive composition requires a wrapper capability that can recover the final assistant body after the session ends
- if the selected provider cannot expose that output, Claudine should fail before launch with a clear error

This is consistent with the spec's rule not to invent new structured modes for providers that do not already support them.

## Error Handling

Update `claudine/lib/src/composition/error.rs` to match the new contract.

Recommended changes:

- remove environment-selection-specific wording from interactive selection errors
- add an error for unsupported `inline-compose --interactive` provider capability
- add an error for invalid inline response shape when body extraction fails
- keep selection, compose, file, and atomic-write errors library-owned

Also remove:

- `ProviderRunFailed` as a selection fallback trigger

Provider failures after launch should terminate the chosen run. They should not silently reroute to another provider.

## Documentation Changes

The implementation is not complete until docs reflect the new contract.

At minimum update:

- `claudine/README.md`
- `claudine/docs/topics/composition.md`
- CLI help text in `claudine/cli/src/args.rs` and any compose help output

Docs must explain:

- the two canonical commands
- the retirement of wrapper composition switches
- the difference between provider selection and provider-session interactivity
- that composition now uses the wrapper-grade execution pipeline
- that inline composition returns replacement body content and Claudine rewrites the file

## Test Plan

### Integration coverage

Add or update CLI integration tests for:

1. `claudine compose <file-ref>` using wrapper-grade execution behavior
2. `claudine inline-compose <file-ref>` using wrapper-grade execution behavior
3. effective composed frontmatter enabling harness behavior that raw source frontmatter would miss
4. `--interactive` preserving composition-first behavior
5. inline deterministic rewrite preserving frontmatter and updating managed fields
6. explicit provider flags bypassing chooser logic
7. ambiguous agent hints failing without TTY
8. hinted-but-uninstalled provider continuing to config/chooser/error instead of failing immediately
9. no automatic cross-provider retry after a launched provider exits non-zero

### Unit coverage

Add or update unit tests for:

1. `prepare_inline_prompt` returning effective composed frontmatter
2. `prepare_chained_prompt` returning effective composed frontmatter
3. provider selection precedence and exclusion rules
4. inline closure body extraction and validation
5. text-preserving managed-field rewrite helpers

## Implementation Sequence

Recommended landing order:

1. expand `claudine::composition` types so prepare returns effective frontmatter and closure state
2. update selection logic to the new precedence rules and shared config lookup
3. add `wrap/composition.rs` and move composition launch into wrapper-grade execution
4. switch top-level `compose.rs` to a thin request builder
5. remove wrapper flags `--compose`, `--frontmatter-prompt`, and `--prompt-file`
6. remove `compose inline` and `compose-inline`
7. move inline rewrite helpers out of `wrap/mod.rs` into `composition::closure`
8. update docs and tests

This order keeps the pipeline functional while progressively deleting the drifted entrypoints.

## Risks and Watchpoints

1. `wrap/mod.rs` is already large. Do not add another large branch there; extract `wrap/composition.rs`.
2. Inline interactive behavior depends on provider output-capture capabilities and needs an explicit early capability check.
3. Prompt-file removal can touch help text, shell completions, and tests in more places than the compose code itself.
4. Effective frontmatter must be threaded carefully through harness and MCP paths so a later fallback to raw source state does not reintroduce drift.

## Final Design Decision

The single most important invariant for the implementation is:

Once a composition request has been prepared, every downstream decision must operate on the same prepared state.

That means:

- the same prompt text goes to the provider
- the same effective frontmatter drives provider selection and harness behavior
- the same wrapper executor handles launch and reporting
- the same captured provider result drives inline closure

If any one of those steps falls back to a second codepath or to raw source state, the drift returns.
