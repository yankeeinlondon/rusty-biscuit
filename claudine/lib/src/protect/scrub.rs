//! Capture-time redaction rules for harvested signal payloads.
//!
//! Co-located with the protect deny catalog because both are curated,
//! reviewable pattern sets — but these rules never block anything. They
//! redact string values in payloads that claudine is about to persist to
//! disk (the unmatched-event harvest under `~/.claudine/harvest/`, see
//! [`crate::signals::harvest`]).
//!
//! Scrubbing is capture-time only: the signal pipeline always observes the
//! original payload, and detection semantics are never affected.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

/// Replacement token for every redaction (matches the style of the
/// messaging module's `<redacted-webhook-url>` convention).
pub const SCRUB_REPLACEMENT: &str = "<redacted>";

/// One capture-time redaction rule: any regex match in a string value is
/// replaced with [`SCRUB_REPLACEMENT`].
#[derive(Debug, Clone)]
pub struct ScrubRule {
    /// Unique identifier for this rule within the scrub set.
    pub rule_id: &'static str,
    /// Regex pattern whose matches are redacted.
    pub pattern: &'static str,
}

/// The capture-time scrub catalog (v1): API-key/token shapes and email
/// addresses. Home-directory and key-name redaction are handled separately
/// (they are not expressible as static patterns): see [`scrub_text`] and
/// [`scrub_json_value`].
pub static SCRUB_CATALOG: &[ScrubRule] = &[
    ScrubRule {
        rule_id: "openai_anthropic_key",
        pattern: r"\bsk-[A-Za-z0-9_-]{16,}",
    },
    ScrubRule {
        rule_id: "aws_access_key_id",
        pattern: r"\bAKIA[0-9A-Z]{16}\b",
    },
    ScrubRule {
        rule_id: "github_token",
        pattern: r"\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})",
    },
    ScrubRule {
        rule_id: "slack_token",
        pattern: r"\bxox[baprs]-[A-Za-z0-9-]{10,}",
    },
    ScrubRule {
        rule_id: "bearer_token",
        pattern: r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{8,}",
    },
    ScrubRule {
        rule_id: "jwt",
        pattern: r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
    },
    ScrubRule {
        rule_id: "email",
        pattern: r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
    },
];

static COMPILED_SCRUB_RULES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    SCRUB_CATALOG
        .iter()
        .map(|rule| {
            Regex::new(rule.pattern)
                .unwrap_or_else(|error| panic!("invalid scrub regex {}: {error}", rule.rule_id))
        })
        .collect()
});

/// Key names whose STRING values are redacted wholesale, regardless of the
/// value's shape (an `Authorization` header value, a `session_token`, an
/// `OPENROUTER_API_KEY`, ...). Substring match so provider-prefixed and
/// suffixed spellings are covered.
static SENSITIVE_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(authorization|api[-_]?key|secret|token)").expect("sensitive-key regex")
});

/// Apply every scrub rule to one string, plus the home-directory rewrite
/// (the current user's absolute home prefix becomes `~`).
pub fn scrub_text(input: &str) -> String {
    let mut text = match home_prefix() {
        Some(home) if input.contains(home.as_str()) => input.replace(home.as_str(), "~"),
        _ => input.to_string(),
    };
    for regex in COMPILED_SCRUB_RULES.iter() {
        if regex.is_match(&text) {
            text = regex.replace_all(&text, SCRUB_REPLACEMENT).into_owned();
        }
    }
    text
}

/// The current user's home directory as a string, when resolvable.
/// Degenerate one-character homes (e.g. `/`) are ignored — replacing them
/// would mangle every path.
fn home_prefix() -> Option<String> {
    let home = dirs::home_dir()?;
    let home = home.to_str()?;
    (home.len() > 1).then(|| home.to_string())
}

/// Recursively scrub every string value in a JSON payload in place.
///
/// String values under a sensitive key name ([`SENSITIVE_KEY_RE`]) are
/// replaced wholesale; every other string runs through [`scrub_text`].
/// Non-string leaves (numbers, booleans, nulls) are never touched.
pub fn scrub_json_value(value: &mut Value) {
    match value {
        Value::String(text) => {
            let scrubbed = scrub_text(text);
            if scrubbed != *text {
                *text = scrubbed;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(scrub_json_value),
        Value::Object(map) => {
            for (key, entry) in map.iter_mut() {
                if entry.is_string() && SENSITIVE_KEY_RE.is_match(key) {
                    *entry = Value::String(SCRUB_REPLACEMENT.to_string());
                } else {
                    scrub_json_value(entry);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn scrub_rule_ids_are_unique_and_patterns_compile() {
        let mut seen = std::collections::HashSet::new();
        for rule in SCRUB_CATALOG.iter() {
            assert!(seen.insert(rule.rule_id), "duplicate: {}", rule.rule_id);
            Regex::new(rule.pattern)
                .unwrap_or_else(|e| panic!("invalid regex for {}: {e}", rule.rule_id));
        }
    }

    #[test]
    fn redacts_openai_anthropic_key() {
        let out = scrub_text("auth failed for sk-ant-api03-AbCdEf0123456789XyZ retry later");
        assert!(!out.contains("sk-ant"), "out: {out}");
        assert!(out.contains(SCRUB_REPLACEMENT));
    }

    #[test]
    fn redacts_aws_access_key_id() {
        let out = scrub_text("using AKIAIOSFODNN7EXAMPLE for s3");
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(out.contains(SCRUB_REPLACEMENT));
    }

    #[test]
    fn redacts_github_tokens() {
        let out = scrub_text("ghp_0123456789abcdefghij and github_pat_11ABCDEFG0_abcdefghij");
        assert!(!out.contains("ghp_"), "out: {out}");
        assert!(!out.contains("github_pat_"), "out: {out}");
    }

    #[test]
    fn redacts_slack_token() {
        let out = scrub_text("token xoxb-1234567890-abcdef rejected");
        assert!(!out.contains("xoxb-"), "out: {out}");
    }

    #[test]
    fn redacts_bearer_token() {
        let out = scrub_text("header was 'Bearer abc.DEF-123_xyz~9'");
        assert!(!out.contains("abc.DEF-123_xyz~9"), "out: {out}");
    }

    #[test]
    fn redacts_jwt_triplet() {
        let out = scrub_text(
            "jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpM expired",
        );
        assert!(!out.contains("eyJ"), "out: {out}");
    }

    #[test]
    fn redacts_email_address() {
        let out = scrub_text("contact ken@example.com for access");
        assert!(!out.contains("ken@example.com"));
        assert!(out.contains(SCRUB_REPLACEMENT));
    }

    #[test]
    fn rewrites_home_directory_prefix() {
        let home = dirs::home_dir().expect("home dir");
        let input = format!("read failed: {}/project/file.txt", home.display());
        let out = scrub_text(&input);
        assert!(!out.contains(home.to_str().unwrap()), "out: {out}");
        assert!(out.contains("~/project/file.txt"), "out: {out}");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        assert_eq!(scrub_text("rate limit exceeded"), "rate limit exceeded");
    }

    #[test]
    fn json_scrub_covers_nested_strings() {
        let mut payload = json!({
            "message": "key sk-ant-api03-AbCdEf0123456789XyZ leaked",
            "detail": { "inner": ["email me at a@b.co"] }
        });
        scrub_json_value(&mut payload);
        assert!(!payload.to_string().contains("sk-ant"));
        assert!(!payload.to_string().contains("a@b.co"));
    }

    #[test]
    fn json_scrub_redacts_sensitive_key_values() {
        let mut payload = json!({
            "Authorization": "some opaque value",
            "api_key": "plain",
            "API-KEY": "plain",
            "session_token": "opaque",
            "client_secret": "opaque",
            "message": "fine"
        });
        scrub_json_value(&mut payload);
        for key in [
            "Authorization",
            "api_key",
            "API-KEY",
            "session_token",
            "client_secret",
        ] {
            assert_eq!(
                payload[key],
                Value::String(SCRUB_REPLACEMENT.to_string()),
                "key {key} should be redacted by name"
            );
        }
        assert_eq!(payload["message"], "fine");
    }

    #[test]
    fn json_scrub_leaves_non_string_values_untouched() {
        let mut payload = json!({
            "max_tokens": 4096,
            "is_error": true,
            "retry_after": null,
            "utilization": 0.93
        });
        let before = payload.clone();
        scrub_json_value(&mut payload);
        assert_eq!(payload, before);
    }
}
