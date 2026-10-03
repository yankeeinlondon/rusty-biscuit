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

use crate::secrets::{self, SecretFamily};

/// Replacement token for every redaction (matches the style of the
/// messaging module's `<redacted-webhook-url>` convention).
pub const SCRUB_REPLACEMENT: &str = "<redacted>";

/// Email addresses: a scrub-only privacy rule applied after the shared
/// credential-token rules. Steering redaction deliberately keeps emails.
static EMAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("email scrub regex")
});

/// Rewrite the current user's absolute home prefix to `~`, then replace every
/// whole match of the shared credential-token rules ([`crate::secrets`]) and
/// the email rule, in that order, with [`SCRUB_REPLACEMENT`].
pub fn scrub_text(input: &str) -> String {
    let mut text = match home_prefix() {
        Some(home) if input.contains(home.as_str()) => input.replace(home.as_str(), "~"),
        _ => input.to_string(),
    };
    let rules = secrets::family_regexes(SecretFamily::CredentialToken).chain([&*EMAIL_RE]);
    for regex in rules {
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
/// String values under a sensitive key name
/// ([`secrets::is_sensitive_key_name`]) are replaced wholesale; every other
/// string runs through [`scrub_text`].
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
                if entry.is_string() && secrets::is_sensitive_key_name(key) {
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

    /// Sequential whole-match replacement is scrub's policy: a bearer header
    /// carrying an API key collapses to one replacement, where steering
    /// masking keeps the `Bearer` keyword.
    #[test]
    fn scrub_keeps_its_sequential_whole_match_policy() {
        assert_eq!(
            scrub_text("header Bearer sk-ant-api03-AbCdEf0123456789XyZ sent by a@b.co"),
            "header Bearer <redacted> sent by <redacted>"
        );
        assert_eq!(
            scrub_text("Authorization: Bearer abcdefgh1234"),
            "Authorization: <redacted>"
        );
        // Contextual rules are steering-only; scrub leaves assignments alone.
        assert_eq!(scrub_text("password=hunter2"), "password=hunter2");
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
            "apiKey": "opaque",
            "db_password": "opaque",
            "public_key": "ssh-ed25519 AAAA",
            "message": "fine"
        });
        scrub_json_value(&mut payload);
        assert_eq!(payload["public_key"], "ssh-ed25519 AAAA");
        for key in [
            "Authorization",
            "api_key",
            "API-KEY",
            "session_token",
            "client_secret",
            "apiKey",
            "db_password",
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
