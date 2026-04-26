# Implementation Plan: Review 1 Findings

> **Feature:** New Message & Notification Platforms  
> **Source Review:** [review-1.md](review-1.md)  
> **Date:** 2026-04-23  
> **Goal:** Address all 16 findings with high test coverage, zero lint warnings, and all tests passing.

---

## Overview

This plan groups the 16 review findings into **4 phases** ordered by dependency and risk. Each phase has clear deliverables, specific file paths, line numbers, and test requirements. A rust-developer subagent can execute phases sequentially.

| Phase | Theme | Findings | Est. Effort |
|---|---|---|---|
| **Phase 1** | Core Correctness & Refactoring | #1, #2, #3, #4 | Medium |
| **Phase 2** | Test Coverage Expansion | #5, #6, #7, #8 | Medium |
| **Phase 3** | UX & Ergonomics | #9, #10, #11, #12 | Medium-Large |
| **Phase 4** | Security, Documentation & Polish | #13, #14, #15, #16 | Small |

**Total estimated effort:** Medium-Large (roughly 1–2 days of focused work).

---

## Phase 1: Core Correctness & Refactoring

**Objective:** Fix the two correctness issues, eliminate code duplication in webhook provider registration, and add a property test that guards redaction coverage.

**Dependencies:** None. This phase is foundational.

### 1.1 Fix Desktop Notification Title Duplication (Finding #1)

**File:** `claudine/lib/src/messaging/send.rs`  
**Lines:** ~235

**Current:**
```rust
let message = Message::text(title.to_string()).title(title.to_string());
```

**Change to:**
```rust
let message = Message {
    title: Some(title.to_string()),
    body: None,
    attachments: Vec::new(),
    location: None,
    metadata: std::collections::BTreeMap::new(),
};
```

**Rationale:** The messenger desktop provider reads `title` and `body` separately. Setting both causes the text to appear twice in the OS notification. A title-only message is explicitly supported by the messenger validation layer (`validate_message_for_provider` short-circuits empty-body for `ProviderKind::Desktop`).

**Test:** Add a unit test in the `#[cfg(test)]` block of `send.rs`:

```rust
#[test]
fn execute_notification_message_is_title_only() {
    let title = "Deployment Successful";
    let message = build_notification_message(title);
    assert_eq!(message.title.as_deref(), Some(title));
    assert!(message.body.is_none());
    assert!(message.attachments.is_empty());
}
```

> If `build_notification_message` is not extracted as a standalone helper, inline the assertion inside a test that calls `execute_notification` directly and inspects the `Message` sent to the `Messenger`. Alternatively, extract `build_notification_message` as a pure function to make it trivially testable.

---

### 1.2 Guard `execute_notification` Against Missing Tokio Runtime (Finding #2)

**File:** `claudine/lib/src/messaging/send.rs`  
**Lines:** 214–226

**Current:** unconditionally calls `tokio::spawn`.

**Change to:**
```rust
pub fn execute_notification(title: &str) {
    let title = title.trim();
    if title.is_empty() {
        return;
    }

    let handle = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => {
            tracing::warn!("Cannot execute desktop notification: no Tokio runtime active");
            return;
        }
    };

    let title = title.to_string();
    handle.spawn(async move {
        // ... existing build/send logic ...
    });
}
```

**Rationale:** Matches the defensive pattern already used in `say_blocking` at `lifecycle.rs:634–651`. Prevents a latent panic if a synchronous caller ever invokes this function.

**Test:** Add a unit test that calls `execute_notification` from a synchronous context and asserts it returns without panicking and without spawning:

```rust
#[test]
fn execute_notification_no_panic_without_runtime() {
    // Must not panic when called outside a Tokio runtime
    execute_notification("test title");
}
```

Also update the existing `execute_notification_blank_is_noop` test to remove its comment about the runtime constraint (the constraint is now closed).

---

### 1.3 Extract Shared Webhook Provider Registration Helper (Finding #3)

**File:** `claudine/lib/src/messaging/send.rs`  
**Lines:** 544–568 (`send_payload`) and 157–200 (`test_webhook_connection`)

**Action:** Extract a private helper that both code paths call.

```rust
fn register_webhook_provider(
    messenger: &mut messenger::Messenger,
    config: &MessagingRouteConfig,
) -> Result<(messenger::Target, messenger::ProviderKind), String> {
    match config {
        MessagingRouteConfig::DiscordWebhook { webhook_url, webhook_url_env } => {
            let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)
                .map_err(|e| format!("Discord webhook URL unresolved: {e}"))?;
            let provider = messenger::provider::DiscordWebhookProvider::try_new(
                url.expose_secret(),
            ).map_err(|e| redact_webhook_urls(&e))?;
            let target = messenger::Target::discord_webhook();
            let kind = messenger::ProviderKind::DiscordWebhook;
            messenger.register_provider(kind, Box::new(provider));
            Ok((target, kind))
        }
        MessagingRouteConfig::SlackWebhook { webhook_url, webhook_url_env } => {
            let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)
                .map_err(|e| format!("Slack webhook URL unresolved: {e}"))?;
            let provider = messenger::provider::SlackWebhookProvider::try_new(
                url.expose_secret(),
            ).map_err(|e| redact_webhook_urls(&e))?;
            let target = messenger::Target::slack_webhook();
            let kind = messenger::ProviderKind::SlackWebhook;
            messenger.register_provider(kind, Box::new(provider));
            Ok((target, kind))
        }
        _ => Err("Not a webhook config".to_string()),
    }
}
```

**Rationale:** Guarantees that the test-connection path and the production send path use identical provider construction and redaction logic. Reduces the future maintenance burden when provider constructors change.

**Test:** The existing tests for `send_payload` and `test_webhook_connection` already cover behavior. After the refactor, run them to verify no regression. No new test file needed, but ensure:
- `cargo test -p claudine-lib messaging::send` passes
- `cargo test -p claudine-cli config_tui` passes

---

### 1.4 Assert Redaction Covers All Valid Webhook URLs (Finding #4)

**File:** `claudine/lib/src/messaging/config.rs` (validators) and `claudine/lib/src/messaging/send.rs` (redactor)  
**Lines:** `config.rs:47–53`, `send.rs:350–358`

**Action:** Add a property-style test in the `#[cfg(test)]` block of `send.rs` (or in a new `tests/redaction.rs` if preferred):

```rust
#[test]
fn redactor_covers_every_valid_webhook_url() {
    let samples = [
        // Discord
        "https://discord.com/api/webhooks/123456789/abcDEF123.-_",
        "https://discordapp.com/api/webhooks/987654321/XYZ789",
        // Slack
        "https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX",
        "https://hooks.slack.com/services/T123/B456/abc123DEF",
    ];

    for url in &samples {
        let is_valid = validate_discord_webhook_url(url) || validate_slack_webhook_url(url);
        assert!(is_valid, "sample URL should be valid: {url}");

        let redacted = redact_webhook_urls(&format!("prefix {url} suffix"));
        assert!(
            !redacted.contains(url),
            "redaction failed on valid URL: {url}"
        );
    }
}
```

**Rationale:** Documents the invariant that the redactor regex is a superset of the validator regex. Prevents a future validator change from silently narrowing redaction coverage.

---

### Phase 1 Exit Criteria
- [ ] `execute_notification` produces title-only messages.
- [ ] `execute_notification` does not panic outside a Tokio runtime.
- [ ] `send_payload` and `test_webhook_connection` both call `register_webhook_provider`.
- [ ] `redactor_covers_every_valid_webhook_url` test passes.
- [ ] All existing tests in `cargo test -p claudine-lib messaging` pass.
- [ ] `cargo clippy -p claudine-lib --lib -- -D warnings` is clean.

---

## Phase 2: Test Coverage Expansion

**Objective:** Close the four identified test coverage gaps, including integration-level wiring tests and render-level snapshot tests.

**Dependencies:** Phase 1 (specifically 1.3, the extracted helper, makes 2.2 easier).

### 2.1 Add `DefaultLifecycleEmitter` Integration Test (Finding #5)

**File:** `claudine/lib/src/composition/lifecycle.rs` (tests)  
**Lines:** Add in the `#[cfg(test)]` block near existing `RecordingEmitter` tests.

**Action:** Add a `#[tokio::test]` that constructs the real default emitter and calls `emit_notification`:

```rust
#[tokio::test]
async fn default_lifecycle_emitter_emit_notification_does_not_panic() {
    let emitter = DefaultLifecycleEmitter;
    // Fire-and-forget: should return immediately without panic
    emitter.emit_notification("test notification title");
    // Give the spawned task a moment to start
    tokio::task::yield_now().await;
}
```

**Rationale:** All 38 lifecycle tests use `RecordingEmitter`. This test is the only regression fence ensuring the default emitter actually calls `execute_notification` and does not crash in a Tokio runtime.

---

### 2.2 Add `HookAction::Message` Through Webhook Route Test (Finding #6)

**File:** `claudine/lib/src/messaging/send.rs` (tests)  
**Lines:** Add in the `#[cfg(test)]` block.

**Action:** Add a test that wires `HookAction::Message` to a `RuntimeMessagingSettings` whose active config is a webhook variant, then calls `build_payload` (or the full `execute_message` flow if mocking the HTTP layer):

```rust
#[test]
fn hook_action_message_with_webhook_route_builds_correct_payload() {
    let settings = RuntimeMessagingSettings {
        active_config: Some("deploys".to_string()),
        configs: {
            let mut map = HashMap::new();
            map.insert(
                "deploys".to_string(),
                MessagingRouteConfig::DiscordWebhook {
                    webhook_url: Some("https://discord.com/api/webhooks/123/abc".to_string()),
                    webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
                },
            );
            map
        },
    };

    let hook = HookAction::Message { content: "Hello webhook".to_string() };
    let payload = build_payload(&hook, &settings).expect("payload should build");

    assert_eq!(payload.target, messenger::Target::discord_webhook());
    assert_eq!(payload.provider, messenger::ProviderKind::DiscordWebhook);
    assert_eq!(payload.message.body.as_deref(), Some("Hello webhook"));
}
```

Repeat for `SlackWebhook`. If `build_payload` is not public, test through `execute_message` with a mocked or stubbed `Messenger`.

**Rationale:** The spec states `HookAction::Message` is a trigger. Every piece is tested in isolation, but no test wires the full path from hook → build → payload for webhook variants.

---

### 2.3 Add Ratatui Render Snapshot for Masked URLs (Finding #7)

**File:** `claudine/cli/src/commands/config_tui/tabs/messenger.rs` (tests)  
**Lines:** Add in the module's `#[cfg(test)]` block or in `tests/config_tui_snapshots.rs`.

**Action:** Render the messenger tab into a `ratatui::backend::TestBackend` and assert the buffer does not contain a raw webhook URL.

```rust
#[test]
fn messenger_render_does_not_expose_raw_webhook_url() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = MessengerTabState {
        // ... populate with a config that has an inline webhook URL ...
    };

    terminal.draw(|f| render_messenger_tab(f, &mut state)).unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer.content.iter().map(|c| c.symbol()).collect::<String>();

    assert!(
        !text.contains("https://discord.com/api/webhooks/"),
        "raw webhook URL must not appear in rendered TUI buffer"
    );
    assert!(
        !text.contains("https://hooks.slack.com/services/"),
        "raw slack webhook URL must not appear in rendered TUI buffer"
    );
}
```

**Rationale:** Model-level tests validate the *logic* of masking, but a ratatui render test enforces the *invariant* that no raw URL ever reaches the terminal buffer. This is the only test that catches a regression where the render function accidentally uses the wrong field.

---

### 2.4 Make Send-Error Redaction Test Deterministic (Finding #8)

**File:** `claudine/lib/src/messaging/send.rs` (tests)  
**Lines:** Locate `test_webhook_connection_redacts_url_in_send_error`.

**Action:** Replace the timing-dependent 2-second network timeout with a deterministic stub or a direct redactor test against known error strings.

**Option A (preferred — direct redactor test):**
```rust
#[test]
fn redact_webhook_urls_in_error_strings() {
    let url = "https://discord.com/api/webhooks/123/abc";
    let error = format!("reqwest error: error sending request for url ({url}): connection failed");
    let redacted = redact_webhook_urls(&error);
    assert!(!redacted.contains(url), "URL must be redacted in error: {redacted}");
    assert!(redacted.contains("***REDACTED***"), "redaction marker should be present");
}
```

**Option B (if the test must go through `test_webhook_connection`):** Stub the `Messenger` or the provider so it returns an error immediately with the URL in the message, then assert the returned `String` is redacted.

**Rationale:** The current test depends on network timeout timing. A deterministic test is faster, reliable in CI, and explicitly documents the redaction behavior against the exact error strings `reqwest` produces.

---

### Phase 2 Exit Criteria
- [ ] `default_lifecycle_emitter_emit_notification_does_not_panic` passes.
- [ ] `hook_action_message_with_webhook_route_builds_correct_payload` (and Slack variant) passes.
- [ ] `messenger_render_does_not_expose_raw_webhook_url` passes.
- [ ] `test_webhook_connection_redacts_url_in_send_error` is deterministic and passes.
- [ ] All tests in `cargo test -p claudine-lib` and `cargo test -p claudine-cli` pass.
- [ ] `cargo clippy -p claudine --lib -- -D warnings` and `cargo clippy -p claudine-cli --bins -- -D warnings` are clean.

---

## Phase 3: UX & Ergonomics

**Objective:** Improve the TUI test-connection flow, hotkey discoverability, failure hints, and modal navigation.

**Dependencies:** Phase 1 (refactored helper may be useful if test connection needs to share logic). Phase 2 tests should be in place before modifying TUI behavior.

### 3.1 Make TUI `T: Test` Non-Blocking (Finding #9)

**File:** `claudine/cli/src/commands/config_tui/tabs/messenger.rs`  
**Lines:** 779–788

**Current:** `tokio::task::block_in_place(|| Handle::current().block_on(...))` with a 5-second timeout freezes the TUI event loop.

**Change to:**
1. On `T` keypress, immediately set `test_status = Some("Testing…")`.
2. Spawn the test in a background Tokio task, using a oneshot channel or shared `Arc<Mutex<Option<TestResult>>>`.
3. On each event-loop iteration, poll for completion with `try_recv` (or check the shared cell) and update `test_status` with the result.
4. If the user presses `Esc` or navigates away, drop the pending test gracefully.

**Rationale:** A frozen event loop is a poor user experience. Even a fast failure blocks redraw and cursor blink.

**Test:** The existing TUI tests should still pass. Add a test that simulates the `T` keypress and asserts `test_status` transitions through `"Testing…"` to a final result.

---

### 3.2 Update Outer Hotkey Strip (Finding #10)

**File:** `claudine/cli/src/commands/config_tui/mod.rs`  
**Lines:** 282–284  
**Also:** `claudine/cli/src/commands/config_tui/tabs/messenger.rs` lines 279–282 (for reference)

**Action:** Review the outer hotkey strip. The current strip shows `Tab Focus, Enter Activate, S Select`. Add `A: Add` if it is missing (pre-existing gap). For `T: Test`, document the intentional omission from the outer strip because it is only meaningful inside the webhook input modal. If the modal is open, the modal's own strip already shows `T: Test`.

If the team prefers consistency, append `, T Test` to the outer strip but gray it out or document that it is context-sensitive.

**Rationale:** Discoverability. Users should see all available actions.

**Test:** Update any snapshot tests of the hotkey strip. Add an assertion that `A: Add` appears.

---

### 3.3 Add Discord Rate-Limit (429) to `failure_hint` (Finding #11)

**File:** `claudine/lib/src/messaging/send.rs`  
**Lines:** 312–340

**Action:** Add one more match arm or condition:

```rust
if error_lower.contains("429")
    || error_lower.contains("rate limit")
    || error_lower.contains("retry_after")
{
    return Some(
        "Discord rate limit hit. Wait a few seconds before retrying, or reduce message frequency."
            .to_string(),
    );
}
```

Place it before the generic 401/403/404 arms so it takes precedence.

**Rationale:** Discord webhooks routinely return 429 when workflows burst messages. A targeted hint keeps the UX useful.

**Test:** Add a unit test in `send.rs` tests:

```rust
#[test]
fn failure_hint_discord_rate_limit() {
    let hint = failure_hint(&"Discord returned 429: rate limited, retry_after: 5000".to_string(), &MessagingRouteConfig::DiscordWebhook { .. });
    assert!(hint.unwrap().contains("rate limit"));
}
```

---

### 3.4 Add "Back" / "Edit Previous Field" to Modal (Finding #12)

**File:** `claudine/cli/src/commands/config_tui/tabs/messenger.rs`  
**Lines:** Modal input flow (various lines in the `MessengerInput` modal handling)

**Action:** Add a `Backspace` or `B` hotkey (or `Shift+Tab`) that decrements the modal step counter and re-populates the input buffer with the previously committed value. This allows users to correct a mistyped config name or webhook URL without pressing `Esc` and restarting.

**Rationale:** The webhook input flow collects a secret up front. If the user mistypes the config name, they currently must restart.

**Test:** Add a reducer/state-machine test:

```rust
#[test]
fn modal_back_navigation_returns_to_previous_field() {
    let mut state = MessengerInputState::new();
    state.advance_to_name();
    state.set_buffer("deploys");
    state.advance_to_url(); // moves to URL field
    state.press_back(); // new action
    assert_eq!(state.current_field(), Field::Name);
    assert_eq!(state.buffer(), "deploys");
}
```

---

### Phase 3 Exit Criteria
- [ ] TUI test connection is non-blocking and shows `Testing…` immediately.
- [ ] Outer hotkey strip includes `A: Add` and documents `T: Test` context.
- [ ] `failure_hint` returns a rate-limit message for 429 errors.
- [ ] Modal supports stepping back to the previous field.
- [ ] All `cargo test -p claudine-cli config_tui` tests pass.
- [ ] `cargo clippy -p claudine-cli --bins -- -D warnings` is clean.

---

## Phase 4: Security, Documentation & Polish

**Objective:** Document validator limitations, expand escape coverage, record naming drift, and update the skill.

**Dependencies:** None. These are documentation and low-risk code changes.

### 4.1 Mark Validators as Conservative (Finding #13)

**File:** `claudine/lib/src/messaging/config.rs`  
**Lines:** Near the `DISCORD_WEBHOOK_REGEX` and `SLACK_WEBHOOK_REGEX` definitions (~47–53).

**Action:** Add doc comments:

```rust
/// Conservative validator for Discord webhook URLs.
///
/// This regex is intentionally stricter than the actual Discord token charset
/// (e.g., it does not include `/`). The authoritative validation happens in
/// `messenger::provider::DiscordWebhookProvider::try_new` at send time.
pub static DISCORD_WEBHOOK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https://(discord\.com|discordapp\.com)/api/webhooks/[0-9]+/[A-Za-z0-9._-]+$")
        .expect("regex is valid")
});

/// Conservative validator for Slack webhook URLs.
///
/// Slack may introduce new token formats in the future. This regex is early
/// TUI feedback only; the authoritative check is in
/// `messenger::provider::SlackWebhookProvider::try_new`.
pub static SLACK_WEBHOOK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https://hooks\.slack\.com/services/[A-Z0-9]+/[A-Z0-9]+/[A-Za-z0-9]+$")
        .expect("regex is valid")
});
```

**Rationale:** Prevents future contributors from assuming the regex is an exact upstream match.

---

### 4.2 Expand `prose_escape` or Document Limitations (Finding #14)

**File:** `claudine/lib/src/messaging/send.rs`  
**Lines:** 368–370

**Action:** Either expand the escape set or add a doc comment explaining the limitation.

**Option A (expand):**
```rust
fn prose_escape(input: &str) -> String {
    input
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace("{{", "\\{{")
        .replace("}}", "\\}}")
        .replace("**", "\\*\\*")
}
```

**Option B (document):**
```rust
/// Escapes `<` and `>` to prevent accidental Prose markup injection.
///
/// Note: This is not a full HTML/markdown sanitizer. Error strings from
/// reqwest/serde rarely contain `{{` or `**`, but if the source of error
/// text changes, this escape set may need to expand.
fn prose_escape(input: &str) -> String {
    input.replace('<', "\\<").replace('>', "\\>")
}
```

**Rationale:** The current escape set is incomplete. At minimum, the limitation should be documented so future maintainers know where to add more escapes.

---

### 4.3 Document Naming Drift in `tech-design.md` (Finding #15)

**File:** `claudine/features/2026-04-23-new-message-platforms/tech-design.md`  
**Lines:** Implementation plan section (~324–361).

**Action:** Add a note:

```markdown
> **Implementation Note:** The validation function was implemented as
> `validate_provider_config` (free function) rather than
> `ClaudineMessengerConfig::validate` (method). The behavior is identical;
> this naming drift is recorded here for future consolidation.
```

---

### 4.4 Update `claudine` Skill with Redaction Guarantees (Finding #16)

**File:** `claudine/.claude/skills/claudine/SKILL.md`  
**Lines:** Under the "Config TUI Messenger Tab" section (or create it if absent).

**Action:** Append:

```markdown
### Messenger Webhook Redaction Invariants

- Inline webhook URLs are never rendered raw in the TUI. They appear as `webhook: ********`.
- Secret input buffers are masked (bullets/asterisks) during modal entry.
- All error messages from webhook sends run through `redact_webhook_urls` before display.
- The test-connection failure status also redacts URLs.
```

---

### Phase 4 Exit Criteria
- [ ] Validator regexes have `conservative` doc comments.
- [ ] `prose_escape` is expanded or documented.
- [ ] `tech-design.md` notes the `validate_provider_config` naming drift.
- [ ] `claudine/.claude/skills/claudine/SKILL.md` enumerates redaction invariants.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy` is clean for both `claudine-lib` and `claudine-cli`.

---

## Cross-Phase Regression Checklist

After all phases are complete, run the full test matrix:

```bash
# Unit + integration tests
cargo test -p claudine-lib messaging
cargo test -p claudine-lib composition::lifecycle
cargo test -p claudine-lib dispatch::loader
cargo test -p claudine-cli config_tui

# Messenger provider tests
cargo test -p messenger-lib discord_webhook
cargo test -p messenger-lib slack_webhook
cargo test -p messenger-lib desktop

# Lint
cargo fmt --check
cargo clippy -p claudine-lib --lib -- -D warnings
cargo clippy -p claudine-cli --bins -- -D warnings
```

Expected test counts (baseline from review + new tests):
- `claudine-lib messaging`: 71 baseline + ~8 new = ~79
- `claudine-lib composition::lifecycle`: 38 baseline + ~2 new = ~40
- `claudine-cli config_tui`: 49 baseline + ~4 new = ~53

---

## Priority Summary (Reproduced from Review)

| # | Finding | Phase | Priority | Effort |
|---|---------|-------|----------|--------|
| 1 | Desktop notification duplicates title in body | 1 | Medium | Small |
| 2 | `execute_notification` panics outside tokio runtime | 1 | Low | Small |
| 3 | Webhook provider registration duplicated | 1 | Medium | Small |
| 4 | Redaction-vs-validator coverage not asserted | 1 | Low | Small |
| 5 | No `DefaultLifecycleEmitter` integration test | 2 | Medium | Small |
| 6 | No `HookAction::Message` + webhook test | 2 | Medium | Small |
| 7 | No ratatui render snapshot for masked URL | 2 | Low | Medium |
| 8 | Send-error redaction test is timing-dependent | 2 | Low | Medium |
| 9 | TUI `T: Test` blocks the event loop | 3 | Medium | Medium |
| 10 | Outer hotkey strip does not surface `T: Test` | 3 | Low | Trivial |
| 11 | Discord rate-limit (429) not in `failure_hint` | 3 | Low | Trivial |
| 12 | Modal cannot edit a committed field | 3 | Low | Medium |
| 13 | Validator regexes may drift from upstream | 4 | Low | Trivial (doc-only) |
| 14 | `prose_escape` is incomplete | 4 | Low | Small |
| 15 | Tech-design vs impl naming drift | 4 | Info | Trivial |
| 16 | Skill doc could state redaction guarantees | 4 | Low | Trivial |

---

## Notes for the Implementing Subagent

1. **Start with Phase 1.** The refactoring in 1.3 makes Phase 2 tests easier to write.
2. **Run the full test suite after every file change.** Do not wait until the end.
3. **For TUI changes (Phase 3),** use `ratatui::backend::TestBackend` for deterministic assertions. Avoid tests that depend on real network calls.
4. **For documentation (Phase 4),** verify the skill file path exists before editing. If `claudine/.claude/skills/claudine/SKILL.md` does not exist, note it and skip finding #16.
5. **If a finding cannot be addressed exactly as specified** (e.g., a function is not public, a module lacks a test block), document the alternative approach taken in a comment near the change.
6. **Do not commit.** The plan is the deliverable; the implementing agent will make the changes.
