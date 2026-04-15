---
title: OpenCode Model Resolution & AgentError Consistency
date: 2026-04-15
status: design
area: claudine
---

# OpenCode Model Resolution & AgentError Consistency

## Background

Claudine currently assumes that non-interactive OpenCode sessions must receive a model from Claudine — via the `--model` CLI switch or the `OPENCODE_MODEL` (or deprecated `MODEL`) environment variable. When neither is present, Claudine bails before launch with a configuration error.

This assumption is wrong. OpenCode's `run` entrypoint will happily use the `model` property configured in `~/.config/opencode/config.json` when no flag is passed. Claudine's hard-fail prevents a legitimate workflow.

Additionally, Claudine's current failure mode double-reports when OpenCode exits with an error: a redundant `OpenCode exited with error code 1` line sits above an `Agent Error` BlockQuote, and neither surface distinguishes between "Claudine never had a model to pass" and "OpenCode rejected the model we gave it".

This design fixes the precedence logic, removes the deprecated `MODEL` read path, adds a launch-time Status log showing where the resolved model came from, and upgrades the `AgentError` surface to render two distinct, suggestion-aware error variants.

## Goals

- Respect OpenCode's built-in config-file default as a valid model source
- Establish a clear precedence ladder (CLI switch → `OPENCODE_MODEL` → config file → error) for model resolution
- Stop reading the deprecated `MODEL` environment variable (writing it for logging stays untouched)
- Surface the model source at launch via a `Status` log line so the user always knows what is going to run
- Render pre-flight ("No Model Provided") and post-flight ("Invalid Model Specified") errors as styled `AgentError` BlockQuotes with no redundant text above
- Parse OpenCode's `ProviderModelNotFoundError` suggestions and surface them in the error

## Non-Goals

- Validating the model string before launch (no `opencode models` subprocess call)
- Changing behavior for any provider other than OpenCode
- Changing behavior for interactive OpenCode sessions (the resolver runs in the non-interactive path only)
- Touching the `MODEL=` env-var write path (kept for cross-provider log consistency)
- Introducing interactive model selection or fuzzy matching

## Precedence

When Claudine resolves the model for a non-interactive OpenCode launch:

1. **`--model` / `-m` CLI switch** (highest)
2. **`OPENCODE_MODEL` environment variable**
3. **`model` field in `~/.config/opencode/config.json`**
4. **None** → raise "No Model Provided" AgentError and exit

The legacy `MODEL` environment variable is no longer read.

## Architecture

### New type: `OpenCodeModelSource`

```rust
enum OpenCodeModelSource {
    CliSwitch(String),
    OpenCodeModelEnv(String),
    ConfigDefault(String),
}
```

Carries both the resolved model name and the origin, so both the launch Status log and any post-flight error can describe the source.

### Resolver

```rust
fn resolve_opencode_model(cli_model: Option<&str>)
    -> Result<OpenCodeModelSource, NoModelProvided>
```

Walks the precedence list and returns the first hit. Reading the OpenCode config file:

- Path: `~/.config/opencode/config.json` (resolved via `dirs`-style home lookup, not via `OPENCODE_CONFIG_CONTENT` or env overrides — we want the on-disk default only)
- Parses as `serde_json::Value`, reads `.model`
- Missing file, unreadable file, malformed JSON, missing field, non-string value, or empty string are all treated as "no default" (Claudine does not warn about a malformed file — OpenCode itself will surface that later if relevant)

### Profile changes (`claudine/cli/src/commands/wrap/profile.rs`)

- Delete `apply_non_interactive_defaults` (lines 1455–1464)
- Replace `validate_non_interactive_requirements` (lines 1466–1476) with a call to the resolver
- New branch in the OpenCode launch path:
  - `CliSwitch(m)` / `OpenCodeModelEnv(m)` → push `--model m` onto args, `MODEL=m` onto env (existing write, unchanged)
  - `ConfigDefault(m)` → **do not** push `--model` (let OpenCode read its own config), but still push `MODEL=m` onto env for log consistency
- The resolved `OpenCodeModelSource` is stored on the run context so post-exit classification can read it

### Status log

Emitted once at launch, after preflight, before the provider is spawned, to stderr via the standard Claudine `Status` surface. Rendered with the `Status` struct from `biscuit-terminal`.

Templates (Prose markup):

| Source | Message |
|---|---|
| `CliSwitch` | `<dim><i>using the </i><yellow>{model}</yellow><i> based on the CLI switch override used by caller</i></dim>` |
| `OpenCodeModelEnv` | `<dim><i>using the </i><yellow>{model}</yellow><i> based on the OPENCODE_MODEL environment variable</i></dim>` |
| `ConfigDefault` | `<dim><i>using the </i><b>{model}</b><i> because this is the default configured in <blue>~/.config/opencode/config.json</blue></i></dim>` |

### AgentError extensions (`claudine/cli/src/output/error_report.rs`)

Add optional fields to `AgentErrorReport`:

```rust
pub(crate) struct AgentErrorReport {
    // existing fields…
    pub(crate) suggestions: Option<Vec<String>>,
    pub(crate) location: Option<String>,
}
```

`render()` is extended to:

- Emit a blank line and `Did you mean:` followed by an `UnorderedList` of `<yellow>{s}</yellow>` items when `suggestions` is present
- Use `location` to template `"Invalid model specified in {location}!"` for the invalid-model variant

Two new constructors:

- `AgentErrorReport::no_model_provided(provider: Provider) -> Self` — pre-flight, exit 1, no stderr
- `AgentErrorReport::invalid_model(provider, exit_code, location, suggestions) -> Self` — post-flight

### Invalid-model detection

New branch in `classify_native_cli_error`: if stderr contains `ProviderModelNotFoundError`, classify as `AgentNative` with the invalid-model summary. Suggestions parsed out of the stderr payload via a regex that captures the `suggestions: [ "a", "b", … ]` array.

The classifier also takes the `OpenCodeModelSource` (threaded through from the launch context) so it can produce the correct `location` string:

| Source | Location |
|---|---|
| `CliSwitch` | `the --model CLI switch` |
| `OpenCodeModelEnv` | `the OPENCODE_MODEL environment variable` |
| `ConfigDefault` | `the config file ~/.config/opencode/config.json` |

## Error Message Templates

### No Model Provided (pre-flight)

- Label line: `Agent Error` red-bold, `(OpenCode, exit 1)` dim
- Body:

  > No model specified! OpenCode by default does not specify a model but you can change this behavior by adding a `<yellow>model</yellow>` property to the `<blue>~/.config/opencode/config.json</blue>` file. You can override/set the default model with any of the following methods:

- UnorderedList:
  - `set <yellow>OPENCODE_MODEL</yellow> to a valid model name`
  - `use the CLI switch <yellow>--model <model></yellow>`

- Blank line

- Footer:

  > Running `<yellow>opencode models</yellow>` will give you a list of all valid models. Model names follow the format `<dim>[provider]</dim>/<dim>[model]</dim>` for direct providers like Google or Anthropic but take the form `<dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim>` for aggregators like OpenRouter.

No text is rendered above the BlockQuote. Claudine exits with code 1 without spawning OpenCode.

### Invalid Model Specified (post-flight)

- Label line: `Agent Error` red-bold, `(OpenCode, exit <n>)` dim
- Body:

  > Invalid model specified in `{location}`! Running `<yellow>opencode models</yellow>` will give you a list of all valid models. Model names follow the format `<dim>[provider]</dim>/<dim>[model]</dim>` for direct providers like Google or Anthropic but take the form `<dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim>` for aggregators like OpenRouter.

- If OpenCode returned suggestions:
  - Blank line
  - `Did you mean:`
  - UnorderedList of `<yellow>{suggestion}</yellow>` items

No text above the BlockQuote. No redundant `OpenCode exited with error code N` line.

## Data Flow

```
launch non-interactive OpenCode
  │
  ├── resolve_opencode_model(cli_model)
  │     ├── Ok(CliSwitch(m))          → push --model m, push MODEL=m env
  │     ├── Ok(OpenCodeModelEnv(m))   → push --model m, push MODEL=m env
  │     ├── Ok(ConfigDefault(m))      → push MODEL=m env only
  │     └── Err(NoModelProvided)      → render AgentError, exit 1
  │
  ├── emit launch Status log (per source template above)
  │
  ├── spawn OpenCode
  │
  └── on exit:
        ├── if stderr contains "ProviderModelNotFoundError":
        │     ├── parse suggestions from stderr
        │     ├── translate OpenCodeModelSource → location string
        │     └── render AgentError "Invalid Model Specified"
        │
        └── else: existing classify_exit path (unchanged)
```

## Testing

### Unit tests

- `resolve_opencode_model`:
  - Returns `CliSwitch` when cli_model is Some
  - Returns `OpenCodeModelEnv` when OPENCODE_MODEL is set and cli_model is None
  - Returns `ConfigDefault` when config.json has a valid `model`
  - Returns `Err(NoModelProvided)` when none of the above
  - CLI beats `OPENCODE_MODEL` when both are set (precedence)
  - CLI beats config-file default when both are set (precedence)
  - `OPENCODE_MODEL` beats config-file default when both are set (precedence)
  - `MODEL` env variable is ignored entirely (set only MODEL, assert `NoModelProvided`)
  - Malformed config.json → `NoModelProvided`
  - Missing config.json → `NoModelProvided`
  - Empty-string `model` → `NoModelProvided`
- Use `serial_test` for env-var tests, `tempfile` + HOME override for config-file tests

### Classifier tests

- `ProviderModelNotFoundError` in stderr → invalid-model category
- Suggestions parsed correctly from the OpenCode payload format
- Absent suggestions → `suggestions: None`

### Integration tests (`claudine/cli/tests/wrap_commands.rs`)

- No CLI model, no env, no config → exits 1 with rendered "No Model Provided" BlockQuote, no text above it
- CLI model present → proceeds past resolver (smoke)
- Snapshot assertions via existing fixture infrastructure where viable

## Files Touched

- `claudine/cli/src/commands/wrap/profile.rs` — resolver, Status emission, remove `MODEL` read, remove deprecated helpers, carry `OpenCodeModelSource` forward
- `claudine/cli/src/output/error_report.rs` — extend struct, add constructors, extend renderer, detect `ProviderModelNotFoundError`
- `claudine/cli/src/commands/wrap/mod.rs` — wire the resolver result into run context / classifier invocation
- Tests: new unit tests in profile.rs and error_report.rs modules, new integration cases in `wrap_commands.rs`
- (New, if needed) small helper for `~/.config/opencode/config.json` reading — may live in `profile.rs` initially

## Rollback

This change is self-contained to the OpenCode profile and the shared error renderer. Reverting the feature commit restores prior behavior with no schema or config migrations needed.
