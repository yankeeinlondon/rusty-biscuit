# Messenger Debug & Tracing Review

**Date:** 2025-04-03
**Scope:** `messenger/lib` (library) and `messenger/cli` (CLI binary)
**Measured against:** Monorepo tracing conventions (see `docs/tracing.md`)

---

## Executive Summary

The messenger package has **zero operational tracing**. Both crates declare tracing dependencies but neither emits a single span or event. The CLI initializes a subscriber with `tracing_subscriber::fmt::init()` but has no filter configuration, no `--debug` flag, no `RUST_LOG` support, and no tracing macros anywhere. The library declares `tracing = "0.1"` as a dependency but never imports or uses it.

This means the entire send pipeline — config loading, route resolution, secret resolution, provider registration, message validation, capability normalization, HTTP transport, and receipt storage — is completely opaque at runtime.

---

## Current State

### Library (`messenger/lib`)

| Item | Status |
|------|--------|
| `tracing = "0.1"` in Cargo.toml | Declared but unused |
| `use tracing::*` imports | None (0 of 31 source files) |
| `#[instrument]` attributes | None |
| `trace!`/`debug!`/`info!`/`warn!` calls | None |
| `error!` calls | None (correct — errors propagate via `Result`) |

### CLI (`messenger/cli`)

| Item | Status |
|------|--------|
| `tracing-subscriber = "0.3"` in Cargo.toml | Declared |
| `tracing` crate in Cargo.toml | **Missing** — macros are unavailable |
| Subscriber initialization | `tracing_subscriber::fmt::init()` (bare, no filter) |
| `--debug` flag | **Missing** |
| `RUST_LOG` support | **Missing** (subscriber ignores env) |
| `--verbose` / `-v` flag | **Missing** |
| `#[instrument]` attributes | None |
| Tracing macros | None (0 of 4 source files) |

---

## Recommendations

### 1. CLI: Add `--debug` Flag and `RUST_LOG` Support

The CLI needs a `--debug` flag and `RUST_LOG` environment variable support following the monorepo convention of separating `--verbose` (user-facing styled output) from `--debug` (developer-facing tracing output).

**In `Cargo.toml`:**
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**In `Cli` struct (`main.rs`):**
```rust
/// Enable debug tracing output (1=info, 2=debug, 3+=trace).
#[arg(long, action = ArgAction::Count, global = true)]
debug: u8,
```

**Replace `tracing_subscriber::fmt::init()` with:**
```rust
fn init_tracing(debug_level: u8) {
    let env_log = std::env::var("RUST_LOG").ok();
    if debug_level == 0 && env_log.is_none() {
        return; // no subscriber = zero overhead
    }

    let default_filter = match debug_level {
        1 => "messenger=info,messenger_cli=info",
        2 => "messenger=debug,messenger_cli=debug",
        _ => "messenger=trace,messenger_cli=trace",
    };

    let filter = env_log
        .as_deref()
        .unwrap_or(default_filter)
        .parse::<tracing_subscriber::EnvFilter>()
        .unwrap_or_else(|_| default_filter.parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .compact()
        .with_target(true)
        .init();
}
```

### 2. Library: Instrument the Send Pipeline

The core send flow is the highest-value tracing target. These functions form the critical path and should get `#[instrument]` spans with structured fields.

**`provider/mod.rs` — `Messenger::send()`:**
```rust
#[tracing::instrument(skip_all, fields(target = ?dispatch.target))]
pub async fn send(&self, dispatch: Dispatch, message: &Message) -> Result<SendReceipt, MessengerError> {
    // ...
}
```

**`provider/mod.rs` — `Messenger::plan_send()`:**
```rust
#[tracing::instrument(skip_all, fields(provider))]
pub fn plan_send(&self, dispatch: Dispatch, message: &Message) -> Result<SendPlan, MessengerError> {
    validate_message(message)?;
    let provider_kind = target_provider_kind(&dispatch.target);
    tracing::Span::current().record("provider", tracing::field::display(&provider_kind));
    // ...
}
```

**`provider/mod.rs` — `Messenger::send_planned()`:**
```rust
#[tracing::instrument(skip_all, fields(provider = %plan.provider))]
pub async fn send_planned(&self, plan: SendPlan) -> Result<SendReceipt, MessengerError> {
    // ...
}
```

**`provider/mod.rs` — `Messenger::send_many()`:**
```rust
#[tracing::instrument(skip_all, fields(count = dispatches.len()))]
pub async fn send_many(&self, dispatches: Vec<Dispatch>, message: &Message) -> Vec<Result<SendReceipt, MessengerError>> {
    tracing::info!(count = dispatches.len(), "starting fan-out send");
    // ...
}
```

### 3. Library: Instrument Each Provider's `send_prepared()`

Each provider adapter makes an HTTP call — the most common failure point. These need tracing for observability.

**Pattern for all 5 providers:**
```rust
#[tracing::instrument(skip_all, fields(provider = "slack", channel))]
async fn send_prepared(&self, dispatch: &Dispatch, message: &PreparedMessage) -> Result<SendReceipt, MessengerError> {
    let channel_id = /* extract */;
    tracing::Span::current().record("channel", &channel_id);

    // Before HTTP call:
    tracing::debug!("sending message");

    // After successful response:
    tracing::debug!(raw_id = %receipt.raw_id, "message sent");

    // On rate limit:
    tracing::warn!(retry_after_ms = ?retry_after, "rate limited");
}
```

**Specific provider trace points:**

| Provider | Key trace points |
|----------|-----------------|
| **Slack** | `debug!` before `chat.postMessage`, `warn!` on 429 rate limit, `debug!` on auth error classification |
| **Discord** | `debug!` before `create_message`, `trace!` for attachment count and types |
| **Signal** | `debug!` before RPC call, `warn!` on JSON-RPC error codes |
| **Telegram** | `debug!` before `sendMessage`/`sendLocation`, `warn!` on rate limit |
| **WhatsApp** | `debug!` before API call, `trace!` for template vs text path selection |

### 4. Library: Instrument Validation and Normalization

The `validate.rs` module makes silent decisions (dropping features, issuing warnings) that are invisible without tracing.

**`normalize_dispatch()`:**
```rust
#[tracing::instrument(skip_all, fields(provider = %provider, mode = ?dispatch.options.compatibility))]
pub fn normalize_dispatch(
    dispatch: &Dispatch,
    message: &Message,
    capabilities: &CapabilitySet,
    provider: ProviderKind,
) -> Result<NormalizedDispatch, MessengerError> {
    // On each feature drop:
    tracing::debug!(feature = "attachments", "dropping unsupported feature");

    // On strict mode rejection:
    tracing::debug!(feature = "attachments", "strict mode: rejecting unsupported feature");
}
```

### 5. CLI: Instrument Config and Receipt Operations

These are file I/O operations that fail silently or confusingly without tracing.

**`config.rs` — `Config::load()` / `Config::load_from_path()`:**
```rust
// In load_from_path:
tracing::debug!(path = %path.display(), "loading config");
// On file not found (returning default):
tracing::debug!(path = %path.display(), "config file not found, using defaults");
```

**`receipt_store.rs` — `save_receipt()`:**
```rust
tracing::debug!(path = %path.display(), provider = %receipt.provider, "saving receipt");
```

**`receipt_store.rs` — `load_message_ref()`:**
```rust
tracing::debug!(spec = %spec, "loading message ref");
// On successful parse from each format:
tracing::trace!("parsed as StoredReceipt");
tracing::trace!("parsed as SendReceipt");
tracing::trace!("parsed as MessageRef");
```

**`main.rs` — `resolve_route()`:**
```rust
// On ad-hoc route:
tracing::debug!(provider = %provider, channel = %channel, "using ad-hoc route");
// On named route:
tracing::debug!(route = %route_name, "using named route");
// On default route:
tracing::debug!(route = %default_name, "using default route");
```

**`main.rs` — `resolve_secret()`:**
```rust
// On direct value (don't log the value!):
tracing::trace!("using direct config value");
// On env var lookup:
tracing::trace!(env = %env_name, "resolving secret from environment");
```

### 6. Library: Instrument Markdown Rendering

The markdown parse-and-render path is a non-trivial transform that's hard to debug without traces.

**`markdown/mod.rs` — `render_for_provider()`:**
```rust
#[tracing::instrument(skip_all, fields(provider = %provider))]
pub fn render_for_provider(body: &MessageBody, provider: ProviderKind) -> String {
    tracing::trace!(input_len = body_text.len(), "rendering markdown");
    // ...
}
```

---

## Level Discipline Summary

Following monorepo conventions:

| Level | Use in messenger |
|-------|-----------------|
| **`error!`** | Never in library. The CLI error handler in `main()` is the only place errors surface. |
| **`warn!`** | Rate limits, auth failures, compatibility feature drops in strict mode, config file parse issues. |
| **`info!`** | Phase transitions: "starting send", "send complete", "fan-out send". Reserved for high-level operation boundaries. |
| **`debug!`** | Decisions and resolved values: route resolution, provider selection, secret source (env vs direct), HTTP request/response, receipt save path. |
| **`trace!`** | Per-item details: individual attachment processing, markdown AST node rendering, JSON parse attempts in `load_message_ref`. |

---

## Anti-Patterns to Avoid

1. **Do not log secrets.** Skip `bot_token`, `access_token`, `SecretString` fields in all `#[instrument]` attributes. Use `skip_all` and manually record safe fields.
2. **Do not use `error!` in library code.** Return `Result` and let the CLI handle display.
3. **Do not use format strings where structured fields work.** Prefer `debug!(provider = %p, channel = %c)` over `debug!("sending to {} on {}", p, c)`.
4. **Do not instrument hot paths.** The markdown parser's per-character loop should not be instrumented; the outer `render_for_provider()` is sufficient.
5. **Do not conflate `--verbose` with `--debug`.** If `--verbose` is added later for richer user-facing output, it must be a separate flag controlling styled `biscuit-terminal` output, not tracing filter levels.

---

## Implementation Priority

1. **CLI subscriber setup** — Add `--debug` flag, `RUST_LOG` support, stderr output (`main.rs`)
2. **Send pipeline spans** — `send()`, `plan_send()`, `send_planned()`, `send_many()` (`provider/mod.rs`)
3. **Provider `send_prepared()` traces** — All 5 providers, especially HTTP call and response handling
4. **Validation traces** — `normalize_dispatch()` feature-drop decisions (`validate.rs`)
5. **CLI operational traces** — Config load, route resolution, secret resolution, receipt store (`main.rs`, `config.rs`, `receipt_store.rs`)
6. **Markdown rendering span** — `render_for_provider()` entry point (`markdown/mod.rs`)

---

## Estimated Scope

- **Library:** ~30-40 trace points across 8 files (provider/mod.rs, validate.rs, 5 provider files, markdown/mod.rs)
- **CLI:** ~15-20 trace points across 4 files (main.rs, config.rs, receipt_store.rs, setup.rs) plus subscriber setup
- **No behavioral changes** — tracing is zero-cost when no subscriber is active
