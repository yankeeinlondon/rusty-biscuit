# Claudine Type Safety, DRY & Test Coverage Review

**Date:** 2026-04-03
**Scope:** `claudine/lib/` and `claudine/cli/`
**Focus:** Type safety improvements, DRY violations, test coverage gaps

---

## 1. Type Safety Improvements

### 1.1 [HIGH] `CompositionExecutionRequest.output` is `Option<String>` for known formats

**File:** `lib/src/composition/types.rs:126`

```rust
/// Set the output format (json, text, stream).
pub output: Option<String>,
```

The comment constrains this to three values but the type allows any string. A typo like `"jsno"` compiles and only fails at runtime.

**Recommendation:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Text,
    Stream,
}

pub output: Option<OutputFormat>,
```

**Ergonomic cost:** Minimal. Callers already pass string literals.

---

### 1.2 [HIGH] `ProtectCallContext.outcome` is `String` but should use `ProtectOutcome`

**File:** `lib/src/actions/hook_response.rs:52-62`

```rust
pub struct ProtectCallContext {
    /// Protect outcome label (snake_case string).
    pub outcome: String,
    pub reason: String,
    #[serde(default)]
    pub short_circuited: bool,
}
```

The codebase already has a strongly-typed `ProtectOutcome` enum (`Allow`, `StopCurrent`, `AdvisoryOnly`, etc.). Using `String` means typos pass silently and downstream consumers must match on raw strings.

**Recommendation:** Store `ProtectOutcome` directly with a custom `Serialize` impl that writes the snake_case tag, or introduce a lightweight serialization-friendly newtype.

**Ergonomic cost:** None. This is an internal struct, not a public API parameter.

---

### 1.3 [HIGH] `ClaudineError` uses `String` for `provider` where `Provider` enum exists

**Files:**

- `lib/src/error.rs:74-79` -- `ConfigCreationNotSupported { provider: String }`
- `lib/src/error.rs:149-156` -- `McpProviderNotSupported { provider: String }`

The `Provider` enum already exists and implements `Display`. Other error variants (e.g., `PolicyBackendUnavailable`) correctly use `Provider`. This inconsistency means callers cannot programmatically match on the provider.

**Recommendation:** Change `provider: String` to `provider: Provider`.

**Ergonomic cost:** None. `Provider` already implements `Display` so the error message is unchanged.

---

### 1.4 [MEDIUM] `CompositionExecutionRequest.system_prompt` is ambiguous `String`

**File:** `lib/src/composition/types.rs:128`

```rust
/// Set or append a system prompt (string or file path).
pub system_prompt: Option<String>,
```

The doc says it can be either a raw string or a file path. Runtime detection (checking if it looks like a path) is error-prone.

**Recommendation:**

```rust
pub enum SystemPromptInput {
    Inline(String),
    File(PathBuf),
}
pub system_prompt: Option<SystemPromptInput>,
```

**Ergonomic cost:** Minor. Callers must be explicit about intent.

---

### 1.5 [MEDIUM] CLI `--provider` args accept raw `String` instead of typed `Provider`

**Files:**

- `cli/src/commands/handle.rs:23`
- `cli/src/commands/sync.rs:24`
- `cli/src/commands/logs.rs:42`
- `cli/src/commands/hooks.rs:30`

All four have `pub provider: Option<String>`. The `Provider` enum already has `parse_cli_name()` and `fuzzy_match_cli_name()`. Deferring parsing to the body of each command wastes the type system.

**Recommendation:** Use a typed clap `value_parser` that produces `Option<Provider>` directly. The validation already happens via `provider_value_parser()` at the clap level.

**Ergonomic cost:** None. Clap error messages would actually improve.

---

### 1.6 [MEDIUM] CLI `--date`/`--from`/`--to` args use `Option<String>` instead of `Option<NaiveDate>`

**File:** `cli/src/commands/logs.rs:30-38`

Invalid dates fail with a generic error instead of a proper clap validation message.

**Recommendation:** Use a custom clap `value_parser` backed by `NaiveDate::parse_from_str`.

**Ergonomic cost:** None. Better UX.

---

### 1.7 [MEDIUM] MCP `ExportArgs` uses raw `String` for `provider` and `scope`

**File:** `cli/src/commands/mcp.rs:153-157`

```rust
pub struct ExportArgs {
    pub provider: String,
    #[arg(long, default_value = "user")]
    pub scope: String,
}
```

Both are validated post-parse via manual string matching. Use typed enums with clap `value_parser`.

---

### 1.8 [LOW] `ToolName` newtype would centralize MCP parsing logic

**File:** `lib/src/events/event_meta.rs:36`

Tool names like `"mcp__filesystem__read_file"` appear across all 8 adapters, each reimplementing `name.starts_with("mcp__")` and `name.splitn(3, "__")`.

**Recommendation:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolName(pub String);

impl ToolName {
    pub fn is_mcp_tool(&self) -> bool { self.0.starts_with("mcp__") }
    pub fn mcp_components(&self) -> Option<(&str, &str)> { ... }
}
```

**Ergonomic cost:** Minor. Call sites change from `&str` to `&ToolName`.

---

### 1.9 [LOW] `ResourceScope::RepoMasked` is a dead variant

**File:** `lib/src/linking/model.rs:8-16`

`RepoMasked` is never constructed or matched anywhere. Additionally, `matches_scope_style` in `linking/mod.rs` treats `User` and `Repo` identically (both arms return the same expression), making the match a no-op.

**Recommendation:** Remove the dead variant or implement the intended semantics. If both scope arms genuinely use the same check, simplify the function.

---

### 1.10 [LOW] `ProviderOverrideArgs` uses one bool per provider

**File:** `cli/src/commands/compose.rs:20-49`

Adding a new provider requires touching 3 places: struct field, clap arg, `resolve()` method. A single `--provider <SLUG>` flag would be self-maintaining.

---

## 2. DRY Violations

### 2.1 [CRITICAL] Skills / Slash Commands / Agents rendering -- ~1700 lines of near-identical code

**Files:**

- `cli/src/commands/skills.rs`
- `cli/src/commands/slash_commands.rs`
- `cli/src/commands/agents.rs`

These three files implement the same ~10 functions with only the resource type differing:

| Function | skills.rs | slash_commands.rs | agents.rs |
|----------|-----------|-------------------|-----------|
| `repo_canonical_needs_init` | ~136 | ~125 | ~122 |
| `render_canonical_providers` | ~153 | ~141 | ~138 |
| `render_detail` | ~202 | ~189 | ~186 |
| `render_verbose` | ~233 | ~243 | ~233 |
| `render_normal` | ~251 | ~260 | ~251 |
| `render_exceptions` | ~284 | ~292 | ~283 |
| `render_fix_summary` | ~483 | ~458 | ~464 |
| `build_provider_header` | ~502 | ~476 | ~482 |
| `render_footer` | ~527 | ~500 | ~506 |
| `scope_badge` | ~585 | ~557 | ~563 |

**Recommendation:** Introduce a `LinkableResourceDisplay` trait that abstracts over `SkillInfo`/`CommandInfo`/`AgentInfo`, their scopes, exception types, and `LinkableResource` variants. Implement shared rendering once as generic functions.

**Estimated recovery:** ~800-1000 lines.

---

### 2.2 [HIGH] `str_field` helper duplicated in all 8 adapter files

**Files:**

- `adapters/claude.rs:251-253`
- `adapters/codex.rs:301-303`
- `adapters/gemini.rs:249-251`
- `adapters/goose.rs:150-152`
- `adapters/opencode.rs:362-364`
- `adapters/qwen.rs:228-230`
- `adapters/roo.rs:242-244`
- `adapters/kimicode.rs:211-213`

```rust
fn str_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}
```

**Recommendation:** Move to `adapters/mod.rs` as `pub(crate) fn str_field(...)`.

---

### 2.3 [HIGH] `tool_input_path` duplicated in 6 adapter files

**Files:**

- `adapters/claude.rs:255-266`
- `adapters/gemini.rs:253-264`
- `adapters/opencode.rs:349-360`
- `adapters/qwen.rs:215-226`
- `adapters/roo.rs:229-240`
- `adapters/kimicode.rs:198-209`

All are functionally identical (only the function name differs). They extract `"file_path"` / `"path"` / `"file"` from `tool_input`.

**Recommendation:** Single `pub(crate) fn extract_tool_input_path(meta: &EventMeta) -> Option<String>` in `adapters/mod.rs`.

---

### 2.4 [HIGH] "Preserve completion scan intent" block copy-pasted in 7 adapters

**Files:** `claude.rs`, `codex.rs`, `gemini.rs`, `opencode.rs`, `qwen.rs`, `roo.rs`, `kimicode.rs`

```rust
if obs.intents.iter().any(|i| matches!(i, ProtectIntent::CompletionOutputScan)) {
    intents.push(ProtectIntent::CompletionOutputScan);
}
obs.intents = intents;
```

**Recommendation:** Extract helper:

```rust
fn replace_intents_preserving_completion(obs: &mut ProtectObservation, new_intents: Vec<ProtectIntent>) {
    let has_completion = obs.intents.iter().any(|i| matches!(i, ProtectIntent::CompletionOutputScan));
    let mut final_intents = new_intents;
    if has_completion {
        final_intents.push(ProtectIntent::CompletionOutputScan);
    }
    obs.intents = final_intents;
}
```

---

### 2.5 [HIGH] `ComposeArgs` vs `InlineComposeArgs` share 20+ identical fields

**File:** `cli/src/commands/compose.rs`

`ComposeArgs` (lines 75-154) and `InlineComposeArgs` (lines 158-237) duplicate all common flags. The `run_compose_inner` and `run_inline_compose_inner` functions also duplicate `CompositionExecutionRequest` construction.

**Recommendation:** Extract a `SharedComposeArgs` struct and use `#[command(flatten)]`.

**Estimated recovery:** ~80 lines.

---

### 2.6 [HIGH] `parse_provider` function triplicated

**Files:**

- `cli/src/commands/handle.rs:84-95`
- `cli/src/commands/sync.rs:471-482`
- `cli/src/commands/mcp.rs:646` (uses `Provider::fuzzy_match_cli_name` directly with manual error)

All three are identical.

**Recommendation:** Single `parse_provider(name: &str) -> Result<Provider>` in a shared `cli_utils` module.

---

### 2.7 [MEDIUM] `event_name_pascal` function triplicated

**Files:**

- `cli/src/commands/sync.rs:63`
- `cli/src/commands/hooks.rs:75`
- `cli/src/commands/actions.rs:41`

```rust
fn event_name_pascal(slug: &str) -> String {
    AgenticEvent::from_slug(slug)
        .map(|event| event.as_pascal_case().to_string())
        .unwrap_or_else(|| slug.to_string())
}
```

**Recommendation:** Move to shared utility or add as inherent method on `AgenticEvent`.

---

### 2.8 [MEDIUM] Degraded-advisory `map_protect_outcome` duplicated in 3 non-blocking adapters

**Files:** `adapters/goose.rs:98-124`, `adapters/roo.rs:181-207`, `adapters/codex.rs:212-238`

All three have nearly identical implementations that extract a reason from `ProtectOutcome`, prepend an advisory message if `decision.degraded`, and return `HookResponse { decision: Continue, .. }`.

**Recommendation:** Add a default method on `ProviderAdapter` for non-blocking providers.

---

### 2.9 [MEDIUM] `EventMeta` construction boilerplate repeated in all 8 adapters

Every adapter constructs `EventMeta` with the same field ordering, `HashMap::new()`, and `EnvironmentContext::default()`.

**Recommendation:** Provide `EventMeta::new(provider, event)` that sets all fields to their defaults.

---

### 2.10 [MEDIUM] Provider list constant duplicated

**Files:**

- `lib/src/events/provider.rs:65-74` -- `PROVIDERS_DISPLAY_ORDER`
- `lib/src/linking/capabilities.rs:431-440` -- `ALL_PROVIDERS`

Both define the same 8-provider array in the same order.

**Recommendation:** Use `PROVIDERS_DISPLAY_ORDER` everywhere.

---

### 2.11 [MEDIUM] Full `Provider::*` list repeated in test blocks

Multiple test functions iterate over all 8 providers with:

```rust
for provider in [
    Provider::Claude, Provider::Codex, Provider::Gemini,
    Provider::Goose, Provider::KimiCode, Provider::OpenCode,
    Provider::QwenCode, Provider::RooCode,
] {
```

**Recommendation:** Use `PROVIDERS_DISPLAY_ORDER`.

---

### 2.12 [LOW] `bool_indicator` duplicated

**Files:** `cli/src/commands/providers.rs:12`, `cli/src/commands/hooks.rs:67`

**Recommendation:** Move to shared `output.rs` or `cli_utils`.

---

### 2.13 [LOW] `base_table` helper duplicated

**Files:** `cli/src/commands/logs.rs:1050`, `cli/src/commands/mcp.rs:1206`

**Recommendation:** Move to shared `table_utils.rs`.

---

### 2.14 [LOW] Wrapper command dispatch in `main.rs` -- 7 identical match arms

**File:** `cli/src/main.rs:49-68`

Seven match arms that only differ by `Provider` variant. A macro or mapping from `Commands` variant to `Provider` could collapse this.

---

## 3. Test Coverage Gaps

### 3.1 [CRITICAL] Protect evaluation engine -- zero direct unit tests

**File:** `lib/src/services/protect/evaluate.rs` (~821 lines)

The core protect evaluation engine with snapshot resolution, finding evaluation, and redaction planning has **zero direct unit tests**. The 30 tests in `services/protect/mod.rs` provide integration coverage but don't isolate the complex evaluation logic.

**Related zero-test files in the same subsystem:**

| File | Risk | Description |
|------|------|-------------|
| `services/protect/redact.rs` | **Security** | Sensitive data redaction logic |
| `services/protect/service.rs` (~335 lines) | **Security** | Service orchestrator |
| `services/protect/downgrade.rs` | **Correctness** | Capability downgrade logic |
| `services/protect/explain.rs` | **UX** | Explanation generation |
| `services/protect/intent.rs` | **Correctness** | Intent-to-query mapping (48 lines, pure mapping) |

**Priority:** HIGH. This is security-critical code.

---

### 3.2 [HIGH] Reporting ingestion engine -- zero tests

**File:** `lib/src/reporting/ingest.rs` (~716 lines)

SQLite ingestion engine with zero tests. Data integrity risk.

**Related zero-test files:**

| File | Description |
|------|-------------|
| `reporting/queries.rs` | Query logic |
| `reporting/paths.rs` | Path resolution |

---

### 3.3 [HIGH] Permission policy modules -- zero tests

| File | Description |
|------|-------------|
| `permissions/explain.rs` | Policy explanation generation |
| `permissions/change.rs` | Policy change types |
| `permissions/context.rs` | Policy context |

---

### 3.4 [MEDIUM] Adapter coverage varies widely

| Adapter | Tests | Assessment |
|---------|-------|------------|
| `opencode.rs` | 11 | Well-covered |
| `codex.rs` | 9 | Well-covered |
| `claude.rs` | 6 | Adequate |
| `kimicode.rs` | 7 | Adequate |
| `qwen.rs` | 7 | Adequate |
| `gemini.rs` | 4 | Sparse |
| `goose.rs` | 3 | **Minimal** |
| `roo.rs` | 2 | **Minimal** |

---

### 3.5 [MEDIUM] Config parsing for newer providers is minimally tested

| Config File | Tests | Assessment |
|-------------|-------|------------|
| `config/goose.rs` | 3 | Minimal |
| `config/kimicode.rs` | 2 | Minimal |
| `config/roo.rs` | 2 | Minimal |

---

### 3.6 [MEDIUM] MCP validation -- only 2 tests

**File:** `lib/src/mcp/validation.rs`

Validation is security-relevant (validating MCP server configurations).

---

### 3.7 [LOW] `wrap/mod.rs` is ~2900 lines but only tested via integration

The core wrapper harness loop, stream summarization, inline closures, retry logic, and pre/post checks are all in one monolithic file. The 20 inline tests and integration tests cover it, but the file size makes targeted testing difficult.

---

### 3.8 Test Infrastructure Note

- **No lib-level integration tests.** All library testing is via inline `#[cfg(test)]` modules. There is no `claudine/lib/tests/` directory.
- **CLI commands are integration-tested** via `cli/tests/` (~111 tests) rather than unit-tested inline.
- **Well-tested modules:** Composition, Linking (best-tested), Harness, Messaging, System Prompt, Dispatch, Stream.

---

## 4. Summary of Recommendations by Priority

### Immediate (Type Safety -- low effort, high impact)

| # | Issue | Effort | Impact |
|---|-------|--------|--------|
| 1.1 | `OutputFormat` enum for composition output | Small | Prevents invalid format strings |
| 1.2 | Typed `ProtectOutcome` in `ProtectCallContext` | Small | Prevents invalid outcome strings |
| 1.3 | `Provider` enum in error variants | Small | Enables programmatic error matching |

### Short-term (DRY -- moderate effort, significant line recovery)

| # | Issue | Effort | Lines Saved |
|---|-------|--------|-------------|
| 2.1 | Skills/Commands/Agents generic trait | Large | ~800-1000 |
| 2.2-2.4 | Shared adapter helpers | Small | ~60-80 |
| 2.5 | Shared compose args | Medium | ~80 |
| 2.6 | Shared `parse_provider` | Small | ~20 |
| 2.7 | Shared `event_name_pascal` | Small | ~10 |

### Medium-term (Test Coverage)

| # | Issue | Priority |
|---|-------|----------|
| 3.1 | Protect evaluate/redact/service unit tests | CRITICAL |
| 3.2 | Reporting ingest unit tests | HIGH |
| 3.3 | Permission policy explain/change/context tests | HIGH |
| 3.4 | Goose and Roo adapter tests | MEDIUM |
| 3.5 | Goose/KimiCode/Roo config tests | MEDIUM |
