# Plan: OpenCode Model Resolution & AgentError Consistency

## Phase 1: Foundation — New Types and Resolver

### 1.1 Add `OpenCodeModelSource` enum

**File**: `claudine/cli/src/commands/wrap/profile.rs`

```rust
pub(crate) enum OpenCodeModelSource {
    CliSwitch(String),
    OpenCodeModelEnv(String),
    ConfigDefault(String),
}
```

### 1.2 Implement `resolve_opencode_model` function

**File**: `claudine/cli/src/commands/wrap/profile.rs`

```
Precedence:
1. CLI switch (--model) → CliSwitch
2. OPENCODE_MODEL env var → OpenCodeModelEnv
3. ~/.config/opencode/config.json "model" field → ConfigDefault
4. None → Err(NoModelProvided)

Config file handling:
- Use dirs-style home lookup (~/.config/opencode/config.json)
- Parse as serde_json::Value, read .model
- Missing file, unreadable, malformed JSON, missing field, non-string, or empty string → NoModelProvided (no warning)
```

### 1.3 Add `NoModelProvided` error type

**File**: `claudine/cli/src/commands/wrap/profile.rs`

```rust
pub(crate) struct NoModelProvided;
```

---

## Phase 2: Profile Changes (`profile.rs`)

### 2.1 Remove deprecated code

- Delete `apply_non_interactive_defaults` (lines 1455–1464)
- Remove `validate_non_interactive_requirements` (lines 1466–1476) — replace with resolver call

### 2.2 Update OpenCode launch path

**Branch on `OpenCodeModelSource` variant:**

| Variant | `--model` arg | `MODEL=` env |
|---|---|---|
| `CliSwitch(m)` | Push `--model m` | Push `MODEL=m` |
| `OpenCodeModelEnv(m)` | Push `--model m` | Push `MODEL=m` |
| `ConfigDefault(m)` | **Do not push** | Push `MODEL=m` (log consistency) |

### 2.3 Thread source through run context

Store `OpenCodeModelSource` on the run context so post-exit classifier can read it.

---

## Phase 3: Status Log at Launch

**Emitted after preflight, before spawn, via `Status` struct from `biscuit-terminal`**

| Source | Template |
|---|---|
| `CliSwitch` | `<dim><i>using the </i><yellow>{model}</yellow><i> based on the CLI switch override used by caller</i></dim>` |
| `OpenCodeModelEnv` | `<dim><i>using the </i><yellow>{model}</yellow><i> based on the OPENCODE_MODEL environment variable</i></dim>` |
| `ConfigDefault` | `<dim><i>using the </i><b>{model}</b><i> because this is the default configured in <blue>~/.config/opencode/config.json</blue></i></dim>` |

---

## Phase 4: AgentError Extensions

### 4.1 Extend `AgentErrorReport`

**File**: `claudine/cli/src/output/error_report.rs`

```rust
pub(crate) struct AgentErrorReport {
    // existing fields…
    pub(crate) suggestions: Option<Vec<String>>,
    pub(crate) location: Option<String>,
}
```

### 4.2 Add new constructors

```rust
AgentErrorReport::no_model_provided(provider: Provider) -> Self
AgentErrorReport::invalid_model(provider, exit_code, location, suggestions) -> Self
```

### 4.3 Extend `render()`

- Emit `Did you mean:` + `UnorderedList` of `<yellow>{s}</yellow>` when `suggestions` present
- Use `location` to template `"Invalid model specified in {location}!"`

### 4.4 Update `classify_native_cli_error`

- Detect `ProviderModelNotFoundError` in stderr
- Parse suggestions via regex capturing `suggestions: [ "a", "b", … ]`
- Take `OpenCodeModelSource` to produce location string:

| Source | Location string |
|---|---|
| `CliSwitch` | `the --model CLI switch` |
| `OpenCodeModelEnv` | `the OPENCODE_MODEL environment variable` |
| `ConfigDefault` | `the config file ~/.config/opencode/config.json` |

---

## Phase 5: Error Message Templates

### "No Model Provided" (pre-flight, exit 1)

```
Agent Error (OpenCode, exit 1)

> No model specified! OpenCode by default does not specify a model but you can
  change this behavior by adding a model property to ~/.config/opencode/config.json.
  You can override/set the default model with any of the following methods:

  • set OPENCODE_MODEL to a valid model name
  • use the CLI switch --model <model>

Running `opencode models` will give you a list of all valid models. Model names
follow the format [provider]/[model] for direct providers...
```

No text above BlockQuote. Claudine exits 1 without spawning OpenCode.

### "Invalid Model Specified" (post-flight)

```
Agent Error (OpenCode, exit <n>)

> Invalid model specified in {location}! Running `opencode models` will give you
  a list of all valid models. Model names follow the format...

Did you mean:
  • <yellow>suggestion1</yellow>
  • <yellow>suggestion2</yellow>
```

No redundant `OpenCode exited with error code N` line.

---

## Phase 6: Wiring

**File**: `claudine/cli/src/commands/wrap/mod.rs`

- Wire resolver result into run context
- Pass `OpenCodeModelSource` to classifier invocation

---

## Phase 7: Testing

### Unit tests (`profile.rs`)

- `resolve_opencode_model`:
  - `CliSwitch` when cli_model is Some
  - `OpenCodeModelEnv` when OPENCODE_MODEL set, cli_model is None
  - `ConfigDefault` when config.json has valid `model`
  - `Err(NoModelProvided)` when none available
  - Precedence: CLI > OPENCODE_MODEL > config
  - `MODEL` env var ignored entirely
  - Malformed config.json → `NoModelProvided`
  - Missing config.json → `NoModelProvided`
  - Empty-string `model` → `NoModelProvided`
- Use `serial_test` for env-var tests, `tempfile` + HOME override for config-file tests

### Classifier tests (`error_report.rs`)

- `ProviderModelNotFoundError` in stderr → invalid-model category
- Suggestions parsed correctly from OpenCode payload format
- Absent suggestions → `suggestions: None`

### Integration tests (`claudine/cli/tests/wrap_commands.rs`)

- No CLI model, no env, no config → exits 1 with rendered "No Model Provided" BlockQuote, no text above it
- CLI model present → proceeds past resolver (smoke)
- Snapshot assertions via existing fixture infrastructure

---

## Files to Modify

| File | Changes |
|---|---|
| `claudine/cli/src/commands/wrap/profile.rs` | `OpenCodeModelSource`, resolver, Status emission, remove `MODEL` read, remove `apply_non_interactive_defaults`, carry source forward |
| `claudine/cli/src/output/error_report.rs` | Extend `AgentErrorReport`, add constructors, extend renderer, detect `ProviderModelNotFoundError` |
| `claudine/cli/src/commands/wrap/mod.rs` | Wire resolver result into run context / classifier |
| `claudine/cli/tests/wrap_commands.rs` | Integration tests |

---

## Rollback

Self-contained to OpenCode profile and shared error renderer. Reverting the feature commit restores prior behavior with no schema or config migrations needed.
