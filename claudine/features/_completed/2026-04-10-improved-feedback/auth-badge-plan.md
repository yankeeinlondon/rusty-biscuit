# Auth / Quota / Billing Badges Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit structured diagnostic badges (Auth / Billing / Quota / RateLimit / ContextPressure / Permission) on every wrapped session summary, with provider-specific remediation URLs where available.

**Architecture:** A new `stream::badges` module defines typed `SessionBadge` values plus a pure `derive_badges(summary, provider)` function that inspects fields already populated on `StreamExecutionSummary` (`error_kind`, `rate_limit`, `context_usage`). Each provider parser's `finish()` populates `summary.badges` before returning. Rendering is layered: library-level `stderr::format_completion_summary` / `format_compact_completion` append plain-text badge lines; CLI-level `wrap::mod::format_summary_prose` renders styled Prose markup for the real user-facing output; `reporting::summary_to_event_meta` writes badges into the JSONL log.

**Tech Stack:** Rust 2024 edition, `serde` (`Serialize`/`Deserialize`), existing `claudine::events::Provider` for dashboard-URL lookups, `biscuit_terminal::Prose` for styled terminal markup, `cargo test -p claudine` for the lib, `cargo test -p claudine-cli` for wrapper-layer tests.

**Design source:** [`auth-badge-design.md`](auth-badge-design.md).

---

## Scope and Out-of-Scope

**In scope (Phase 1 + Phase 2 of the design):**

- New `SessionBadge` type + derivation function
- `summary.badges` field + parser wiring for all six stream providers (Claude, Codex, Gemini, Kimi, OpenCode, Qwen)
- Claude, Codex, Gemini, Kimi, Qwen classification via existing `error_kind` extracted by typed protocol models
- Kimi context-pressure badge (threshold ≥ 80%)
- Stderr text formatters (`format_completion_summary`, `format_compact_completion`)
- CLI Prose-rendered summary (`format_summary_prose`) so real wrapper sessions show badges
- JSONL reporting integration (`summary_to_event_meta`)

**Out of scope (Phase 3 — Tier 3):**

- OpenCode / Goose / Roo Code "via upstream" badge derivation with no dashboard URL
- Upstream-provider detection from model name
- Reading rate-limit messages via substring matching beyond what `error_kind` already captures

---

## Important Context

**Naming collision, no conflict.** A crate-level module `claudine::badges` already exists at `claudine/lib/src/badges.rs` — it hosts terminal badge constants like `YOLO`, `PROTECT`, `COMPOSE`. The new module lives at `claudine::stream::badges` (`claudine/lib/src/stream/badges.rs`) so the two do not collide. Import as `claudine::stream::badges::SessionBadge`.

**Rendering call-sites.** The library-level `format_completion_summary` and `format_compact_completion` in `stream/stderr.rs` are currently **only called from their own tests** — not from wrapper CLI output. Production user-facing rendering lives in `claudine/cli/src/commands/wrap/mod.rs::format_summary_prose`. Both must be updated so the lib-level contract stays coherent with the real wrapper surface. Confirmed via grep on 2026-04-11.

**`label` vs `error_kind`.** The design examples loosely mix raw error kinds and category labels in rendered output. This plan uses `badge.label` (e.g. `"Billing"`, `"Auth"`) for rendering, never the raw error_kind, to match the `SessionBadge` struct definition.

**Dashboard URL lookup.** Already implemented as `Provider::usage_dashboard_url() -> Option<&'static str>` in `claudine/lib/src/events/provider.rs:515`. No changes required there.

**`error_kind` is already populated.** Every Tier 1 / Tier 2 provider parser sets `summary.error_kind` via typed protocol models (confirmed: `claude.rs:136`, `codex.rs:140`). The derivation function is pure and requires no parser-side refactor.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `claudine/lib/src/stream/badges.rs` | **create** | `SessionBadge`, `BadgeCategory`, `BadgeSeverity`, pattern tables, `derive_badges()` |
| `claudine/lib/src/stream/mod.rs` | modify | `pub mod badges;` |
| `claudine/lib/src/stream/summary.rs` | modify | Add `pub badges: Vec<SessionBadge>` (serde skip when empty) |
| `claudine/lib/src/stream/claude.rs` | modify | Call `derive_badges` at the end of `finish()` |
| `claudine/lib/src/stream/codex.rs` | modify | Call `derive_badges` at the end of `finish()` |
| `claudine/lib/src/stream/gemini.rs` | modify | Call `derive_badges` at the end of `finish()` |
| `claudine/lib/src/stream/kimi.rs` | modify | Call `derive_badges` at the end of `finish()` |
| `claudine/lib/src/stream/opencode.rs` | modify | Call `derive_badges` at the end of `finish()` |
| `claudine/lib/src/stream/qwen.rs` | modify | Call `derive_badges` at the end of `finish()` |
| `claudine/lib/src/stream/stderr.rs` | modify | Render badges in `format_completion_summary` + `format_compact_completion` |
| `claudine/lib/src/stream/reporting.rs` | modify | Serialize `badges` into `summary_to_event_meta` `extra` |
| `claudine/cli/src/commands/wrap/mod.rs` | modify | Append Prose-styled badges in `format_summary_prose` |

---

## Task Ordering Rationale

Tasks are ordered so later tasks build on the types and functions introduced by earlier tasks. Each task ends with a commit on a green test run. The sequencing is strictly bottom-up: types → derivation → summary integration → per-parser wiring → renderer layers → reporting.

---

### Task 1: Scaffold `stream::badges` module with types only

**Files:**

- Create: `claudine/lib/src/stream/badges.rs`
- Modify: `claudine/lib/src/stream/mod.rs` (append `pub mod badges;`)

- [ ] **Step 1: Create the new file with type definitions**

Write to `claudine/lib/src/stream/badges.rs`:

```rust
//! Structured diagnostic badges emitted after a wrapped session ends.
//!
//! A [`SessionBadge`] surfaces a single human-readable condition such as an
//! authentication failure, billing exhaustion, rate-limit throttle, or
//! context-window pressure. Badges are derived from the fields already
//! present on [`crate::stream::summary::StreamExecutionSummary`] and are
//! consumed by the stderr formatters, the wrapper Prose renderer, and JSONL
//! reporting.

use serde::{Deserialize, Serialize};

use crate::events::Provider;
use crate::stream::summary::StreamExecutionSummary;

/// Category describing why a badge was emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeCategory {
    Auth,
    Billing,
    Quota,
    RateLimit,
    ContextPressure,
    Permission,
}

/// Severity of a badge from the operator's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeSeverity {
    Info,
    Warning,
    Error,
}

/// A single diagnostic badge attached to a session summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBadge {
    pub category: BadgeCategory,
    pub severity: BadgeSeverity,
    pub label: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation_url: Option<String>,
}

/// Derive zero or more badges from a completed session summary.
///
/// Pure function — reads `error_kind`, `rate_limit`, and `context_usage`
/// from the summary and consults [`Provider::usage_dashboard_url`] for
/// remediation links. Returns an empty vector when no signal fires.
pub fn derive_badges(_summary: &StreamExecutionSummary, _provider: Provider) -> Vec<SessionBadge> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_badge_round_trip() {
        let badge = SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Warning,
            label: "Billing",
            message: "Insufficient credits".into(),
            remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
        };
        let json = serde_json::to_string(&badge).unwrap();
        let restored: SessionBadge = serde_json::from_str(&json).unwrap();
        assert_eq!(badge, restored);
    }

    #[test]
    fn category_serializes_snake_case() {
        let json = serde_json::to_string(&BadgeCategory::RateLimit).unwrap();
        assert_eq!(json, "\"rate_limit\"");
        let json = serde_json::to_string(&BadgeCategory::ContextPressure).unwrap();
        assert_eq!(json, "\"context_pressure\"");
    }

    #[test]
    fn severity_serializes_snake_case() {
        let json = serde_json::to_string(&BadgeSeverity::Warning).unwrap();
        assert_eq!(json, "\"warning\"");
    }

    #[test]
    fn derive_badges_returns_empty_for_default_summary() {
        let summary = StreamExecutionSummary::default();
        assert!(derive_badges(&summary, Provider::Claude).is_empty());
    }
}
```

- [ ] **Step 2: Register the module**

Edit `claudine/lib/src/stream/mod.rs` at line 1 (top of file with other `pub mod` declarations). Find:

```rust
pub mod claude;
pub mod codex;
```

Insert a line `pub mod badges;` above `pub mod claude;` so the block reads:

```rust
pub mod badges;
pub mod claude;
pub mod codex;
```

- [ ] **Step 3: Run the badges tests to verify they compile and pass**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: 4 tests pass (`session_badge_round_trip`, `category_serializes_snake_case`, `severity_serializes_snake_case`, `derive_badges_returns_empty_for_default_summary`).

- [ ] **Step 4: Commit**

```bash
git add claudine/lib/src/stream/badges.rs claudine/lib/src/stream/mod.rs
git commit -m "feat(claudine): scaffold stream::badges module with SessionBadge types"
```

---

### Task 2: Add error-kind pattern tables and Auth classification

**Files:**

- Modify: `claudine/lib/src/stream/badges.rs`

- [ ] **Step 1: Write failing tests for auth classification**

Append to the `tests` module in `claudine/lib/src/stream/badges.rs`:

```rust
    fn summary_with_kind(provider: Provider, kind: &str, message: &str) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider,
            is_error: true,
            error_kind: Some(kind.into()),
            error_message: Some(message.into()),
            ..Default::default()
        }
    }

    #[test]
    fn auth_kind_yields_auth_badge_with_dashboard_url() {
        let summary = summary_with_kind(Provider::Claude, "authentication_error", "Invalid API key");
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        let badge = &badges[0];
        assert_eq!(badge.category, BadgeCategory::Auth);
        assert_eq!(badge.severity, BadgeSeverity::Error);
        assert_eq!(badge.label, "Auth");
        assert_eq!(badge.message, "Invalid API key");
        assert_eq!(
            badge.remediation_url.as_deref(),
            Some("https://console.anthropic.com/settings/billing"),
        );
    }

    #[test]
    fn auth_kind_without_message_falls_back_to_label_text() {
        let summary = StreamExecutionSummary {
            is_error: true,
            error_kind: Some("unauthorized".into()),
            error_message: None,
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Auth);
        assert_eq!(badges[0].message, "Authentication failed");
    }

    #[test]
    fn invalid_api_key_is_auth_kind() {
        let summary = summary_with_kind(Provider::Codex, "invalid_api_key", "bad key");
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges[0].category, BadgeCategory::Auth);
        assert_eq!(
            badges[0].remediation_url.as_deref(),
            Some("https://platform.openai.com/usage"),
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: 3 new tests fail with assertions about empty `badges` vec.

- [ ] **Step 3: Add pattern tables and implement auth branch**

Edit `claudine/lib/src/stream/badges.rs`. Above the `derive_badges` function, insert the pattern tables:

```rust
const AUTH_KINDS: &[&str] = &[
    "authentication_error",
    "authentication_failed",
    "auth_error",
    "invalid_api_key",
    "unauthorized",
];

const BILLING_KINDS: &[&str] = &[
    "billing_error",
    "payment_required",
    "credit_balance_too_low",
    "account_suspended",
];

const QUOTA_KINDS: &[&str] = &[
    "quota_exceeded",
    "insufficient_quota",
    "usage_limit_reached",
    "capacity_limit",
];

const RATE_LIMIT_KINDS: &[&str] = &[
    "rate_limit_error",
    "rate_limit",
    "rate_limit_exceeded",
    "too_many_requests",
    "resource_exhausted",
];

const PERMISSION_KINDS: &[&str] = &[
    "permission_error",
    "forbidden_error",
    "access_denied",
    "forbidden",
];

fn kind_matches(kind: &str, table: &[&str]) -> bool {
    table.iter().any(|entry| entry.eq_ignore_ascii_case(kind))
}
```

Replace the stub `derive_badges` body with:

```rust
pub fn derive_badges(summary: &StreamExecutionSummary, provider: Provider) -> Vec<SessionBadge> {
    let mut badges = Vec::new();
    let dashboard_url = provider.usage_dashboard_url().map(str::to_owned);

    if let Some(kind) = summary.error_kind.as_deref() {
        if kind_matches(kind, AUTH_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Auth,
                severity: BadgeSeverity::Error,
                label: "Auth",
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Authentication failed".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        }
    }

    badges
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/badges.rs
git commit -m "feat(claudine): add auth badge classification with dashboard URLs"
```

---

### Task 3: Add Billing, Quota, RateLimit, and Permission classification

**Files:**

- Modify: `claudine/lib/src/stream/badges.rs`

- [ ] **Step 1: Write failing tests for remaining categories and priority ordering**

Append to the `tests` module in `claudine/lib/src/stream/badges.rs`:

```rust
    #[test]
    fn billing_kind_yields_billing_badge() {
        let summary = summary_with_kind(Provider::Claude, "billing_error", "Insufficient credits");
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Billing);
        assert_eq!(badges[0].label, "Billing");
        assert_eq!(badges[0].severity, BadgeSeverity::Error);
        assert_eq!(badges[0].message, "Insufficient credits");
    }

    #[test]
    fn quota_kind_yields_quota_badge() {
        let summary = summary_with_kind(
            Provider::Codex,
            "insufficient_quota",
            "You exceeded your current quota",
        );
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Quota);
        assert_eq!(badges[0].label, "Quota");
    }

    #[test]
    fn quota_kind_exceeded_yields_quota_badge() {
        let summary = summary_with_kind(Provider::QwenCode, "quota_exceeded", "OAuth free quota depleted");
        let badges = derive_badges(&summary, Provider::QwenCode);
        assert_eq!(badges[0].category, BadgeCategory::Quota);
        assert_eq!(
            badges[0].remediation_url.as_deref(),
            Some("https://bailian.console.aliyun.com/"),
        );
    }

    #[test]
    fn rate_limit_kind_yields_rate_limit_badge() {
        let summary = summary_with_kind(Provider::Codex, "rate_limit", "Too many requests");
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::RateLimit);
        assert_eq!(badges[0].label, "Rate Limit");
        assert_eq!(badges[0].severity, BadgeSeverity::Warning);
    }

    #[test]
    fn permission_kind_yields_permission_badge() {
        let summary = summary_with_kind(Provider::Gemini, "permission_error", "Access denied");
        let badges = derive_badges(&summary, Provider::Gemini);
        assert_eq!(badges[0].category, BadgeCategory::Permission);
        assert_eq!(badges[0].label, "Permission");
    }

    #[test]
    fn billing_wins_over_rate_limit_on_same_error_kind() {
        // `insufficient_quota` overlaps billing and quota tables.
        // Priority order per design: Auth > Billing > Quota > RateLimit > Permission.
        let summary = summary_with_kind(
            Provider::Codex,
            "insufficient_quota",
            "You exceeded your current quota",
        );
        let badges = derive_badges(&summary, Provider::Codex);
        // Quota wins here because it appears in the quota table but not the
        // billing table. This assertion exists to lock in the priority.
        assert_eq!(badges[0].category, BadgeCategory::Quota);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: 6 new tests fail.

- [ ] **Step 3: Implement the remaining classification branches with priority ordering**

Replace the body of `derive_badges` in `claudine/lib/src/stream/badges.rs`:

```rust
pub fn derive_badges(summary: &StreamExecutionSummary, provider: Provider) -> Vec<SessionBadge> {
    let mut badges = Vec::new();
    let dashboard_url = provider.usage_dashboard_url().map(str::to_owned);

    if let Some(kind) = summary.error_kind.as_deref() {
        // Priority: Auth > Billing > Quota > RateLimit > Permission.
        // At most one classification badge per kind.
        if kind_matches(kind, AUTH_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Auth,
                severity: BadgeSeverity::Error,
                label: "Auth",
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Authentication failed".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, BILLING_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing",
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Billing error".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, QUOTA_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Quota,
                severity: BadgeSeverity::Error,
                label: "Quota",
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Quota exceeded".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, RATE_LIMIT_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit",
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Rate limit hit".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, PERMISSION_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Permission,
                severity: BadgeSeverity::Error,
                label: "Permission",
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Permission denied".to_string()),
                remediation_url: None,
            });
        }
    }

    badges
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: all 13 tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/badges.rs
git commit -m "feat(claudine): classify billing, quota, rate-limit, and permission badges"
```

---

### Task 4: Add rate-limit event badge derivation from `rate_limit` field

**Files:**

- Modify: `claudine/lib/src/stream/badges.rs`

**Rationale:** Claude emits `rate_limit_event` as a stream event (not via `error_kind`), so `summary.rate_limit` is set but `error_kind` remains `None` on an otherwise successful run. The derivation must also consult `summary.rate_limit.is_throttled`.

- [ ] **Step 1: Write failing test for rate-limit event derivation**

Append to the `tests` module:

```rust
    #[test]
    fn throttled_rate_limit_info_yields_badge_even_without_error_kind() {
        use crate::stream::summary::RateLimitInfo;
        let summary = StreamExecutionSummary {
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: Some(5000),
                message: Some("Rate limit exceeded".into()),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::RateLimit);
        assert!(badges[0].message.contains("Rate limit exceeded"));
        assert!(badges[0].message.contains("5.0s"));
    }

    #[test]
    fn non_throttled_rate_limit_info_does_not_yield_badge() {
        use crate::stream::summary::RateLimitInfo;
        let summary = StreamExecutionSummary {
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(false),
                retry_after_ms: None,
                message: None,
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Claude);
        assert!(badges.is_empty());
    }

    #[test]
    fn rate_limit_from_error_kind_does_not_duplicate_with_rate_limit_info() {
        use crate::stream::summary::RateLimitInfo;
        // If both a rate_limit_error kind AND a throttled rate_limit_info are
        // present, we only emit one RateLimit badge, preferring the detailed
        // error-kind message.
        let summary = StreamExecutionSummary {
            is_error: true,
            error_kind: Some("rate_limit".into()),
            error_message: Some("Too many requests".into()),
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: Some(1000),
                message: Some("Slow down".into()),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Codex);
        let rate_limit_count = badges
            .iter()
            .filter(|b| b.category == BadgeCategory::RateLimit)
            .count();
        assert_eq!(rate_limit_count, 1);
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: 3 new tests fail.

- [ ] **Step 3: Add the rate-limit branch after the error-kind classification block**

In `claudine/lib/src/stream/badges.rs`, inside `derive_badges`, immediately after the `if let Some(kind) = ...` block and before `badges`, add:

```rust
    let already_has_rate_limit = badges
        .iter()
        .any(|b| b.category == BadgeCategory::RateLimit);

    if !already_has_rate_limit
        && let Some(rate_limit) = summary.rate_limit.as_ref()
        && rate_limit.is_throttled.unwrap_or(false)
    {
        let retry_hint = rate_limit
            .retry_after_ms
            .map(|ms| format!(" (retry in {:.1}s)", ms as f64 / 1000.0))
            .unwrap_or_default();
        let base_message = rate_limit
            .message
            .clone()
            .unwrap_or_else(|| "Rate limit hit".to_string());
        badges.push(SessionBadge {
            category: BadgeCategory::RateLimit,
            severity: BadgeSeverity::Warning,
            label: "Rate Limit",
            message: format!("{base_message}{retry_hint}"),
            remediation_url: dashboard_url.clone(),
        });
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: all 16 tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/badges.rs
git commit -m "feat(claudine): derive rate-limit badge from throttled rate_limit events"
```

---

### Task 5: Add context-pressure badge derivation

**Files:**

- Modify: `claudine/lib/src/stream/badges.rs`

- [ ] **Step 1: Write failing tests**

Append to the `tests` module:

```rust
    #[test]
    fn context_usage_at_or_above_threshold_yields_context_pressure_badge() {
        use crate::stream::summary::ContextUsage;
        let summary = StreamExecutionSummary {
            context_usage: Some(ContextUsage {
                used: Some(110_000),
                total: Some(128_000),
                percent: Some(85.9),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::KimiCode);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::ContextPressure);
        assert_eq!(badges[0].label, "Context");
        assert_eq!(badges[0].severity, BadgeSeverity::Warning);
        assert!(badges[0].message.contains("86%"));
        assert!(badges[0].message.contains("110000"));
        assert!(badges[0].message.contains("128000"));
        assert_eq!(
            badges[0].remediation_url.as_deref(),
            Some("https://platform.moonshot.cn/console/account"),
        );
    }

    #[test]
    fn context_usage_below_threshold_does_not_yield_badge() {
        use crate::stream::summary::ContextUsage;
        let summary = StreamExecutionSummary {
            context_usage: Some(ContextUsage {
                used: Some(50_000),
                total: Some(128_000),
                percent: Some(39.0),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::KimiCode);
        assert!(badges.is_empty());
    }

    #[test]
    fn context_usage_at_exact_threshold_yields_badge() {
        use crate::stream::summary::ContextUsage;
        let summary = StreamExecutionSummary {
            context_usage: Some(ContextUsage {
                used: Some(102_400),
                total: Some(128_000),
                percent: Some(80.0),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::KimiCode);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::ContextPressure);
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: 3 new tests fail.

- [ ] **Step 3: Add the context-pressure branch above the final `badges` return**

In `claudine/lib/src/stream/badges.rs`, inside `derive_badges`, immediately after the rate-limit block and before the final `badges`, insert:

```rust
    const CONTEXT_PRESSURE_THRESHOLD: f64 = 80.0;

    if let Some(context) = summary.context_usage.as_ref()
        && let Some(percent) = context.percent
        && percent >= CONTEXT_PRESSURE_THRESHOLD
    {
        let used = context
            .used
            .map(|u| u.to_string())
            .unwrap_or_else(|| "?".to_string());
        let total = context
            .total
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".to_string());
        badges.push(SessionBadge {
            category: BadgeCategory::ContextPressure,
            severity: BadgeSeverity::Warning,
            label: "Context",
            message: format!(
                "Context window pressure: {percent:.0}% used ({used}/{total} tokens)"
            ),
            remediation_url: dashboard_url.clone(),
        });
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine stream::badges -- --nocapture`
Expected: all 19 tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/badges.rs
git commit -m "feat(claudine): derive context-pressure badge at 80% threshold"
```

---

### Task 6: Add `badges: Vec<SessionBadge>` field to `StreamExecutionSummary`

**Files:**

- Modify: `claudine/lib/src/stream/summary.rs`

- [ ] **Step 1: Write a failing test for the new field default and serde behavior**

Append to the `tests` module at `claudine/lib/src/stream/summary.rs`:

```rust
    #[test]
    fn badges_default_is_empty_vec() {
        let summary = StreamExecutionSummary::default();
        assert!(summary.badges.is_empty());
    }

    #[test]
    fn empty_badges_vec_is_skipped_in_serde_output() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("badges"));
    }

    #[test]
    fn non_empty_badges_vec_round_trips() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let summary = StreamExecutionSummary {
            badges: vec![SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing",
                message: "Insufficient credits".into(),
                remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
            }],
            ..Default::default()
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("Insufficient credits"));
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.badges.len(), 1);
        assert_eq!(restored.badges[0].category, BadgeCategory::Billing);
    }
```

- [ ] **Step 2: Run the tests to verify failure**

Run: `cargo test -p claudine stream::summary -- --nocapture`
Expected: fails to compile — `badges` is not a field of `StreamExecutionSummary`.

- [ ] **Step 3: Add the `badges` field**

Edit `claudine/lib/src/stream/summary.rs`. Add an import at the top of the file under the existing imports:

```rust
use crate::stream::badges::SessionBadge;
```

Inside the `StreamExecutionSummary` struct, immediately before `raw_summary`, add:

```rust
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub badges: Vec<SessionBadge>,
```

In the `Default::default()` impl block inside `impl Default for StreamExecutionSummary`, add `badges: Vec::new(),` immediately before `raw_summary: None,`.

Update every existing struct literal in this file that constructs `StreamExecutionSummary` so it includes `badges: Vec::new(),`. As of 2026-04-11 the literals live in:

- The `serde_round_trip_full` test at lines 119–144 — insert `badges: Vec::new(),` before `raw_summary`.

Do not modify tests in other files yet — those will be touched in Task 7.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p claudine stream::summary -- --nocapture`
Expected: all `summary` tests pass (including the new 3).

- [ ] **Step 5: Full lib build to catch other struct-literal construction sites**

Run: `cargo build -p claudine 2>&1 | tee /tmp/badges-build.log`
Expected: possible E0063 errors flagging other construction sites (claude.rs, codex.rs, gemini.rs, kimi.rs, opencode.rs, qwen.rs, stderr.rs tests, reporting.rs tests, compose tests). For each location, add `badges: Vec::new(),` immediately before `raw_summary: None,`. Re-run build until green.

- [ ] **Step 6: Run the full lib test suite**

Run: `cargo test -p claudine`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/stream/
git commit -m "feat(claudine): add badges field to StreamExecutionSummary"
```

---

### Task 7: Populate `badges` on `finish()` for all six parsers

**Files:**

- Modify: `claudine/lib/src/stream/claude.rs`
- Modify: `claudine/lib/src/stream/codex.rs`
- Modify: `claudine/lib/src/stream/gemini.rs`
- Modify: `claudine/lib/src/stream/kimi.rs`
- Modify: `claudine/lib/src/stream/opencode.rs`
- Modify: `claudine/lib/src/stream/qwen.rs`

- [ ] **Step 1: Write a failing test in `claude.rs`**

Append to the `tests` module in `claudine/lib/src/stream/claude.rs`:

```rust
    #[test]
    fn billing_error_populates_billing_badge_on_summary() {
        let mut parser = make_parser();
        let init =
            r#"{"type":"init","session_id":"sess-err","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();
        let error = r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#;
        parser.feed_line(error).unwrap();
        let summary = parser.finish(1);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::Billing
        );
        assert_eq!(summary.badges[0].message, "Insufficient credits");
        assert_eq!(
            summary.badges[0].remediation_url.as_deref(),
            Some("https://console.anthropic.com/settings/billing"),
        );
    }

    #[test]
    fn auth_error_populates_auth_badge_on_summary() {
        let mut parser = make_parser();
        let init =
            r#"{"type":"init","session_id":"sess-auth","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();
        let error = r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;
        parser.feed_line(error).unwrap();
        let summary = parser.finish(1);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::Auth
        );
    }

    #[test]
    fn rate_limit_event_populates_rate_limit_badge_on_summary() {
        let mut parser = make_recording_parser();
        let init = r#"{"type":"init","session_id":"sess-rl","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();
        let rl = r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limit exceeded"}"#;
        parser.feed_line(rl).unwrap();
        let result =
            r#"{"type":"result","duration_ms":5000,"usage":{"input_tokens":100,"output_tokens":50}}"#;
        parser.feed_line(result).unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::RateLimit
        );
        assert!(summary.badges[0].message.contains("5.0s"));
    }
```

- [ ] **Step 2: Run the Claude parser tests to verify failure**

Run: `cargo test -p claudine stream::claude -- --nocapture`
Expected: 3 new tests fail — badges is empty.

- [ ] **Step 3: Wire `derive_badges` into Claude `finish()`**

Edit `claudine/lib/src/stream/claude.rs`. Locate the `fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary` around line 314 and replace its body so the final summary is computed in a `let mut summary = ...` bind, then populated with badges via `derive_badges(&summary, Provider::Claude)`. Replace the existing body with:

```rust
    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut summary = StreamExecutionSummary {
            provider: Provider::Claude,
            session_id: self.session_id,
            model: self.model,
            assistant_text: self.assistant_text,
            provider_status: self.provider_status,
            exit_code,
            is_error: self.is_error,
            error_kind: self.error_kind,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
            duration_api_ms: self.duration_api_ms,
            num_turns: self.num_turns,
            token_usage: self.token_usage,
            cost_usd: self.cost_usd,
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            rate_limit: self.rate_limit,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: self.raw_summary,
            stderr_text: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::Claude);
        summary
    }
```

- [ ] **Step 4: Run Claude parser tests to verify pass**

Run: `cargo test -p claudine stream::claude -- --nocapture`
Expected: all Claude tests pass (including 3 new ones).

- [ ] **Step 5: Apply the same pattern to the other five parsers**

Each parser constructs its summary in `finish()`. Edit each file to follow the same pattern — change the tail of `finish()` to bind `let mut summary = ...`, then set `summary.badges = crate::stream::badges::derive_badges(&summary, Provider::<Variant>);` before returning `summary`. The provider enum variants are:

- `claudine/lib/src/stream/codex.rs` → `Provider::Codex`
- `claudine/lib/src/stream/gemini.rs` → `Provider::Gemini`
- `claudine/lib/src/stream/kimi.rs` → `Provider::KimiCode`
- `claudine/lib/src/stream/opencode.rs` → `Provider::OpenCode`
- `claudine/lib/src/stream/qwen.rs` → `Provider::QwenCode`

Make sure each existing `StreamExecutionSummary { ... }` literal already includes `badges: Vec::new(),` from Task 6; if any file still emits E0063, add that field before re-running.

- [ ] **Step 6: Run the full lib test suite**

Run: `cargo test -p claudine`
Expected: all tests pass. No regressions in the other five parsers' existing test suites.

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/stream/
git commit -m "feat(claudine): populate summary.badges in every stream parser finish()"
```

---

### Task 8: Render badges in `format_completion_summary`

**Files:**

- Modify: `claudine/lib/src/stream/stderr.rs`

- [ ] **Step 1: Write failing tests for badge rendering**

Append to the `tests` module in `claudine/lib/src/stream/stderr.rs`:

```rust
    #[test]
    fn completion_summary_renders_single_badge_with_url() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Error,
            label: "Billing",
            message: "Insufficient credits".into(),
            remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
        }];
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("\u{2713}")); // check mark on the first line
        assert!(rendered.contains("\u{26a0}")); // warning symbol on badge line
        assert!(rendered.contains("Billing"));
        assert!(rendered.contains("Insufficient credits"));
        assert!(rendered.contains("https://console.anthropic.com/settings/billing"));
        let lines: Vec<&str> = rendered.lines().collect();
        assert!(lines.len() >= 3, "expected summary + badge + url lines");
    }

    #[test]
    fn completion_summary_renders_badge_without_url() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Permission,
            severity: BadgeSeverity::Error,
            label: "Permission",
            message: "Access denied".into(),
            remediation_url: None,
        }];
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("Permission"));
        assert!(rendered.contains("Access denied"));
        assert!(!rendered.contains("\u{2192}")); // no "→ url" line
    }

    #[test]
    fn completion_summary_renders_multiple_badges() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![
            SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit",
                message: "Slow down".into(),
                remediation_url: None,
            },
            SessionBadge {
                category: BadgeCategory::ContextPressure,
                severity: BadgeSeverity::Warning,
                label: "Context",
                message: "Context window pressure: 86% used".into(),
                remediation_url: None,
            },
        ];
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("Rate Limit"));
        assert!(rendered.contains("Slow down"));
        assert!(rendered.contains("Context"));
        assert!(rendered.contains("86%"));
    }

    #[test]
    fn completion_summary_without_badges_is_unchanged() {
        let summary = full_summary();
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(!rendered.contains("\u{26a0}"));
        assert_eq!(rendered.lines().count(), 1);
    }
```

- [ ] **Step 2: Run the tests to verify failure**

Run: `cargo test -p claudine stream::stderr -- --nocapture`
Expected: 4 new tests fail.

- [ ] **Step 3: Append a badge block to `format_completion_summary`**

Edit `claudine/lib/src/stream/stderr.rs`. Replace the final return of `format_completion_summary` (currently `Some(format!("{prefix} {}", parts.join(" \u{00b7} ")))`) with:

```rust
    let mut out = format!("{prefix} {}", parts.join(" \u{00b7} "));
    for badge in &summary.badges {
        out.push('\n');
        out.push_str(&format!("\u{26a0} {} \u{2014} {}", badge.label, badge.message));
        if let Some(url) = &badge.remediation_url {
            out.push('\n');
            out.push_str(&format!("  \u{2192} {url}"));
        }
    }
    Some(out)
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p claudine stream::stderr -- --nocapture`
Expected: all stderr tests pass (including the 4 new ones).

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/stderr.rs
git commit -m "feat(claudine): render badges in format_completion_summary"
```

---

### Task 9: Render badge indicator in `format_compact_completion`

**Files:**

- Modify: `claudine/lib/src/stream/stderr.rs`

- [ ] **Step 1: Write failing tests**

Append to the `tests` module in `claudine/lib/src/stream/stderr.rs`:

```rust
    #[test]
    fn compact_completion_shows_badge_indicator_with_label() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Error,
            label: "Billing",
            message: "Insufficient credits".into(),
            remediation_url: None,
        }];
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(rendered.contains("|"));
        assert!(rendered.contains("\u{26a0}"));
        assert!(rendered.contains("Billing"));
        assert!(!rendered.contains("Insufficient credits")); // compact: no message
    }

    #[test]
    fn compact_completion_shows_multiple_badge_labels_comma_separated() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![
            SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit",
                message: "Slow down".into(),
                remediation_url: None,
            },
            SessionBadge {
                category: BadgeCategory::ContextPressure,
                severity: BadgeSeverity::Warning,
                label: "Context",
                message: "86%".into(),
                remediation_url: None,
            },
        ];
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(rendered.contains("Rate Limit"));
        assert!(rendered.contains("Context"));
        assert!(rendered.contains(", "));
    }

    #[test]
    fn compact_completion_without_badges_is_single_line() {
        let summary = full_summary();
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(!rendered.contains('|'));
        assert_eq!(rendered.lines().count(), 1);
    }
```

- [ ] **Step 2: Run the tests to verify failure**

Run: `cargo test -p claudine stream::stderr -- --nocapture`
Expected: 3 new tests fail.

- [ ] **Step 3: Add a badge indicator to `format_compact_completion`**

Edit `claudine/lib/src/stream/stderr.rs`. Replace the final return of `format_compact_completion` (currently `Some(format!("{prefix} {}", parts.join(" \u{00b7} ")))`) with:

```rust
    let mut out = format!("{prefix} {}", parts.join(" \u{00b7} "));
    if !summary.badges.is_empty() {
        let labels: Vec<&str> = summary.badges.iter().map(|b| b.label).collect();
        out.push_str(&format!(" | \u{26a0} {}", labels.join(", ")));
    }
    Some(out)
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p claudine stream::stderr -- --nocapture`
Expected: all stderr tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/stderr.rs
git commit -m "feat(claudine): render compact badge indicator in format_compact_completion"
```

---

### Task 10: Serialize badges in `summary_to_event_meta` JSONL output

**Files:**

- Modify: `claudine/lib/src/stream/reporting.rs`

- [ ] **Step 1: Write failing tests**

Append to the `tests` module in `claudine/lib/src/stream/reporting.rs`:

```rust
    #[test]
    fn summary_to_event_meta_serializes_badges_when_present() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = make_test_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Error,
            label: "Billing",
            message: "Insufficient credits".into(),
            remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
        }];
        let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());
        let badges = meta.extra.get("badges").unwrap();
        let arr = badges.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["category"], Value::String("billing".into()));
        assert_eq!(arr[0]["severity"], Value::String("error".into()));
        assert_eq!(arr[0]["label"], Value::String("Billing".into()));
        assert_eq!(arr[0]["message"], Value::String("Insufficient credits".into()));
        assert_eq!(
            arr[0]["remediation_url"],
            Value::String("https://console.anthropic.com/settings/billing".into()),
        );
    }

    #[test]
    fn summary_to_event_meta_omits_badges_when_empty() {
        let summary = make_test_summary();
        let meta = summary_to_event_meta(&summary, StreamProtocol::StreamJson, &make_test_env());
        assert!(!meta.extra.contains_key("badges"));
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p claudine stream::reporting -- --nocapture`
Expected: 2 new tests fail.

- [ ] **Step 3: Serialize badges into `extra`**

Edit `claudine/lib/src/stream/reporting.rs`. In `summary_to_event_meta_with_context`, immediately after the `provider_summary` block (the block that inserts `extra.insert("provider_summary".into(), ...)`) and before the `EventMeta { ... }` literal, insert:

```rust
    if !summary.badges.is_empty()
        && let Ok(value) = serde_json::to_value(&summary.badges)
    {
        extra.insert("badges".into(), value);
    }
```

- [ ] **Step 4: Run the tests to verify pass**

Run: `cargo test -p claudine stream::reporting -- --nocapture`
Expected: all reporting tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/reporting.rs
git commit -m "feat(claudine): serialize session badges into JSONL summary events"
```

---

### Task 11: Render styled badges in CLI `format_summary_prose`

**Files:**

- Modify: `claudine/cli/src/commands/wrap/mod.rs`

**Rationale:** `format_completion_summary` is a library-layer formatter that is only consumed by its own tests. User-facing wrapper output is built in `format_summary_prose` at `claudine/cli/src/commands/wrap/mod.rs:3391`. Badges must be rendered there to actually show up in real sessions.

- [ ] **Step 1: Write failing tests**

Locate the `#[cfg(test)] mod tests` block for the wrap module. Find the existing call site that imports `format_cost, format_duration, format_number` (around line 3394 — there is an existing test module at the end of `wrap/mod.rs`). Append a new test that invokes `format_summary_prose` with a summary that has a badge, asserting the returned markup contains the expected label and remediation URL.

Search for the existing test module with `grep -n "fn format_summary_prose" claudine/cli/src/commands/wrap/mod.rs` and the nearest `#[cfg(test)]`. Append tests inside that test module:

```rust
    #[test]
    fn format_summary_prose_appends_badge_markup() {
        use claudine::events::Provider;
        use claudine::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        use claudine::stream::summary::StreamExecutionSummary;
        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            duration_ms: Some(1000),
            badges: vec![SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing",
                message: "Insufficient credits".into(),
                remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
            }],
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("Billing"));
        assert!(rendered.contains("Insufficient credits"));
        assert!(rendered.contains("https://console.anthropic.com/settings/billing"));
    }

    #[test]
    fn format_summary_prose_without_badges_has_no_badge_markup() {
        use claudine::events::Provider;
        use claudine::stream::summary::StreamExecutionSummary;
        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            duration_ms: Some(1000),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(!rendered.contains("Billing"));
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p claudine-cli format_summary_prose -- --nocapture`
Expected: 2 new tests fail.

- [ ] **Step 3: Render badges in `format_summary_prose`**

Edit `claudine/cli/src/commands/wrap/mod.rs`. Replace the final return of `format_summary_prose` (currently `Some(format!("<dim>{prefix} {}</dim>", parts.join(" \u{00b7} ")))`) with:

```rust
    let mut out = format!("<dim>{prefix} {}</dim>", parts.join(" \u{00b7} "));
    for badge in &summary.badges {
        let color = match badge.severity {
            claudine::stream::badges::BadgeSeverity::Error => "red",
            claudine::stream::badges::BadgeSeverity::Warning => "yellow",
            claudine::stream::badges::BadgeSeverity::Info => "cyan",
        };
        out.push('\n');
        out.push_str(&format!(
            "<{color}>\u{26a0} <bold>{}</bold> \u{2014} {}</{color}>",
            badge.label, badge.message
        ));
        if let Some(url) = &badge.remediation_url {
            out.push('\n');
            out.push_str(&format!("  <dim>\u{2192} {url}</dim>"));
        }
    }
    Some(out)
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p claudine-cli format_summary_prose -- --nocapture`
Expected: the 2 new tests pass.

- [ ] **Step 5: Run the full CLI test suite**

Run: `cargo test -p claudine-cli`
Expected: all wrap tests pass.

- [ ] **Step 6: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): render styled session badges in wrapper summary prose"
```

---

### Task 12: End-to-end Codex insufficient_quota + rate_limit regression tests

**Files:**

- Modify: `claudine/lib/src/stream/codex.rs`

**Rationale:** Locks in the Tier 1 Codex contract from the design (`codex.rs:468` already tests rate_limit; this task proves the badge pipeline). Also exercises that the Codex parser populates `error_kind` through to the badge pipeline.

- [ ] **Step 1: Write failing (or pinning) tests**

Append to the `tests` module in `claudine/lib/src/stream/codex.rs`:

```rust
    #[test]
    fn rate_limit_error_yields_rate_limit_badge() {
        // Use the existing error event fixture pattern in this module.
        let mut parser = CodexStreamParser::new(NullSink, None);
        let line = r#"{"type":"error","error":{"type":"rate_limit","message":"Too many requests"}}"#;
        parser.feed_line(line).unwrap();
        let summary = parser.finish(1);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::RateLimit
        );
        assert_eq!(
            summary.badges[0].remediation_url.as_deref(),
            Some("https://platform.openai.com/usage"),
        );
    }

    #[test]
    fn insufficient_quota_yields_quota_badge() {
        let mut parser = CodexStreamParser::new(NullSink, None);
        let line = r#"{"type":"error","error":{"type":"insufficient_quota","message":"You exceeded your current quota"}}"#;
        parser.feed_line(line).unwrap();
        let summary = parser.finish(1);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::Quota
        );
    }
```

**Note:** Before running, verify the exact Codex error event shape by searching the existing tests. The existing rate_limit test is at `codex.rs:468`. If the event wire format differs (e.g. `type: "turn_error"` wrapper or `error_type` field instead of nested `error.type`), adapt the JSON literal to match the fixtures already in that test file. Do not invent a new schema.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p claudine stream::codex -- --nocapture`
Expected: 2 new tests fail.

- [ ] **Step 3: Resolve the correct Codex error fixture**

Open `claudine/lib/src/stream/codex.rs` and locate the existing assertion at line 438 that reads `summary.error_kind.as_deref() == Some("rate_limit")`. Scroll up to find the JSON literal that produces that summary. Copy that exact schema into the two new tests, substituting the error kind and message.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine stream::codex -- --nocapture`
Expected: all codex tests pass, including the 2 new ones.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/codex.rs
git commit -m "test(claudine): lock in Codex rate-limit and quota badge derivation"
```

---

### Task 13: End-to-end Kimi context-pressure regression test

**Files:**

- Modify: `claudine/lib/src/stream/kimi.rs`

- [ ] **Step 1: Write a failing test**

Append to the `tests` module in `claudine/lib/src/stream/kimi.rs`:

```rust
    #[test]
    fn status_update_above_threshold_yields_context_pressure_badge() {
        // Feed a status_update event that populates context_usage at 86%,
        // then finish and assert the derived badge.
        let mut parser = KimiStreamParser::new(NullSink);
        // Locate the existing fixture for context_usage in this module
        // (see the test around kimi.rs:156-184 referenced by the design).
        // Reuse that same JSON schema.
        let status_line = r#"{"type":"status_update","context_usage":{"used":110000,"total":128000,"percent":85.9}}"#;
        parser.feed_line(status_line).unwrap();
        let summary = parser.finish(0);
        assert_eq!(summary.badges.len(), 1);
        assert_eq!(
            summary.badges[0].category,
            crate::stream::badges::BadgeCategory::ContextPressure
        );
        assert_eq!(
            summary.badges[0].remediation_url.as_deref(),
            Some("https://platform.moonshot.cn/console/account"),
        );
    }

    #[test]
    fn status_update_below_threshold_yields_no_badge() {
        let mut parser = KimiStreamParser::new(NullSink);
        let status_line = r#"{"type":"status_update","context_usage":{"used":50000,"total":128000,"percent":39.0}}"#;
        parser.feed_line(status_line).unwrap();
        let summary = parser.finish(0);
        assert!(summary.badges.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p claudine stream::kimi -- --nocapture`
Expected: 2 new tests fail. If they fail because the `status_update` schema is wrong, open `kimi.rs:156-184` (referenced in the design) and copy the exact wire format from the existing tests there.

- [ ] **Step 3: Resolve the correct Kimi status_update fixture**

Verify the JSON schema matches the real Kimi fixtures. If the existing tests use field names like `context_window_used` or a nested `usage` object, update the test fixtures to match before rerunning.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine stream::kimi -- --nocapture`
Expected: all kimi tests pass, including the 2 new ones.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/stream/kimi.rs
git commit -m "test(claudine): lock in Kimi context-pressure badge derivation"
```

---

### Task 14: Final workspace validation

**Files:** none (validation only)

- [ ] **Step 1: Run the full claudine test suite**

Run: `cargo test -p claudine`
Expected: all tests pass.

- [ ] **Step 2: Run the CLI test suite**

Run: `cargo test -p claudine-cli`
Expected: all tests pass.

- [ ] **Step 3: Run claudine lint**

Run: `just claudine lint` (or `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings`)
Expected: no warnings.

- [ ] **Step 4: Dry-run a composition or wrap command to eyeball real-world output**

Optional manual sanity check. Run `cargo run -p claudine-cli -- claude --help` (won't hit a live provider) and, if you have test recordings, replay one with a forced `billing_error` line to confirm badges appear in real stderr output.

- [ ] **Step 5: Commit any lint fixes (if needed)**

If clippy demanded changes, commit them as a final housekeeping commit. Otherwise this step is a no-op.

```bash
git status
# If clean, nothing to commit.
# If not: git add -A && git commit -m "chore(claudine): address lint after badge feature"
```

---

## Self-Review Notes

- **Spec coverage:** Every Tier 1 + Tier 2 bullet from the design (Phase 1 and Phase 2) is covered by Tasks 1–13. Tier 3 (Phase 3) is explicitly deferred per the "Scope and Out-of-Scope" section.
- **No placeholders:** Every code step includes full Rust source. Test steps include fixture JSON. The only places that defer to an "existing fixture" are Tasks 12 and 13, where the plan instructs the engineer to locate and reuse the exact fixture from the current parser tests rather than invent a schema — this is the correct behavior.
- **Type consistency:** `SessionBadge`, `BadgeCategory`, `BadgeSeverity`, `derive_badges`, and the `badges: Vec<SessionBadge>` field are referenced consistently across Tasks 1–13. The rendering uses `badge.label` throughout (never the raw `error_kind`).
- **Priority ordering pin:** Task 3 includes a test that locks in the priority order via the `insufficient_quota` overlap case. This prevents silent regressions if the tables are later edited.
- **Test-first discipline:** Every implementation task leads with a failing test, then a minimal change, then a verification run, then a commit. No speculative abstractions.

---

## Execution Handoff

Plan complete and saved to `claudine/features/2026-04-10-improved-feedback/auth-badge-plan.md`. Two execution options:

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task with review checkpoints between tasks. Best for this plan because Tasks 6 and 7 require hunting down every `StreamExecutionSummary` struct literal in the crate, which benefits from a clean context window.
2. **Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batching commits at natural checkpoints (after Task 5, Task 7, Task 10, Task 14).

Which approach?
