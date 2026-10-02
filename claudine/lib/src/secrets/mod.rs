//! Shared secret recognition.
//!
//! One catalog of credential shapes ([`SECRET_CATALOG`]) and one sensitive
//! key-name recognizer ([`is_sensitive_key_name`]) serve every consumer that
//! keeps secrets out of persisted or displayed text: capture-time scrubbing
//! ([`crate::protect::scrub`]), messaging error redaction, wrapper argument
//! and environment sanitization in the CLI, and steering audit records.
//!
//! Recognition is shared; replacement is not. Each consumer selects the rule
//! families it applies and keeps its own replacement token and extra privacy
//! policy (scrubbing, for example, also hides email addresses and home paths).
//! [`mask_secrets`] and [`Redactor`] are the steering policy: overlapping or
//! touching spans merge, and each merged span becomes [`MASK`]. Everything
//! outside a recognized span — prose, email addresses, paths — is kept.
//!
//! Recognition is heuristic and best effort. It does not guarantee that every
//! secret is found.
//!
//! Topic: `claudine/docs/topics/secret-recognition.md`.

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::sync::LazyLock;

use regex::{Captures, Regex};
use serde::{Serialize, Serializer};

#[cfg(test)]
mod tests;

/// The steering replacement for one merged secret span.
pub const MASK: &str = "****";

/// Smallest recognized secret value that [`Redactor`] also masks wherever the
/// same bytes reappear. Shorter values would mask ordinary words.
pub const MIN_KNOWN_SECRET_BYTES: usize = 6;

/// A group of rules that consumers select together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretFamily {
    /// Self-identifying credential tokens: API keys, GitHub and Slack tokens,
    /// bearer tokens, and JWTs.
    CredentialToken,
    /// Discord and Slack incoming-webhook URLs, whose path is a credential.
    WebhookUrl,
    /// Values that are secret because of their context: sensitive
    /// assignments and flags, authorization headers, URL passwords, and
    /// private-key blocks.
    Contextual,
}

/// One recognition rule.
///
/// A rule with a `key` capture group matches only when that key is a
/// sensitive name. The masked span is the first participating capture group
/// named `secret*` or `loose`; a `loose` value must also look like a
/// credential, so `token: expired` stays readable. Without such a group the
/// whole match is the span. Consumers that replace whole matches
/// ([`family_regexes`]) ignore the groups.
#[derive(Debug, Clone, Copy)]
pub struct SecretRule {
    pub id: &'static str,
    pub family: SecretFamily,
    pub pattern: &'static str,
}

/// The shared catalog, in the order consumers apply it sequentially.
pub static SECRET_CATALOG: &[SecretRule] = &[
    SecretRule {
        id: "openai_anthropic_key",
        family: SecretFamily::CredentialToken,
        pattern: r"\bsk-[A-Za-z0-9_-]{16,}",
    },
    SecretRule {
        id: "aws_access_key_id",
        family: SecretFamily::CredentialToken,
        pattern: r"\bAKIA[0-9A-Z]{16}\b",
    },
    SecretRule {
        id: "github_token",
        family: SecretFamily::CredentialToken,
        pattern: r"\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})",
    },
    SecretRule {
        id: "slack_token",
        family: SecretFamily::CredentialToken,
        pattern: r"\bxox[baprs]-[A-Za-z0-9-]{10,}",
    },
    SecretRule {
        id: "bearer_token",
        family: SecretFamily::CredentialToken,
        pattern: r"(?i)\bbearer\s+(?P<secret>[A-Za-z0-9._~+/=-]{8,})",
    },
    SecretRule {
        id: "jwt",
        family: SecretFamily::CredentialToken,
        pattern: r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
    },
    SecretRule {
        id: "discord_webhook_url",
        family: SecretFamily::WebhookUrl,
        pattern: r"https://(?:discord\.com|discordapp\.com)/api/webhooks/[0-9]+/[A-Za-z0-9._-]+",
    },
    SecretRule {
        id: "slack_webhook_url",
        family: SecretFamily::WebhookUrl,
        pattern: r"https://hooks\.slack\.com/services/[A-Za-z0-9/_-]+",
    },
    SecretRule {
        id: "sensitive_assignment",
        family: SecretFamily::Contextual,
        pattern: r#"(?P<key>[A-Za-z0-9_.-]+)["']?[ \t]*=[ \t]*(?:"(?P<secret>[^"\r\n]+)"|'(?P<secret_single>[^'\r\n]+)'|(?P<secret_bare>[^\s"',;&]+))"#,
    },
    SecretRule {
        id: "sensitive_field",
        family: SecretFamily::Contextual,
        pattern: r#"(?P<key>[A-Za-z0-9_.-]+)["']?[ \t]*:[ \t]*(?:"(?P<secret>[^"\r\n]+)"|'(?P<secret_single>[^'\r\n]+)'|(?P<loose>[^\s"',;&]+))"#,
    },
    SecretRule {
        id: "sensitive_flag",
        family: SecretFamily::Contextual,
        pattern: r#"(?P<key>--[A-Za-z][A-Za-z0-9-]*)[ \t]+(?P<loose>[^\s"',;&-][^\s"',;&]*)"#,
    },
    SecretRule {
        id: "authorization_header",
        family: SecretFamily::Contextual,
        pattern: r#"(?i)\b(?:proxy-)?authorization["']?[ \t]*[:=][ \t]*["']?(?:(?:basic|bearer|token|digest|negotiate)[ \t]+(?P<secret>[^\s"',;]+)|(?P<loose>[^\s"',;]+))"#,
    },
    SecretRule {
        id: "url_password",
        family: SecretFamily::Contextual,
        pattern: r"\b[A-Za-z][A-Za-z0-9+.-]*://[^\s/@:]+:(?P<secret>[^\s/@]+)@",
    },
    SecretRule {
        id: "private_key_block",
        family: SecretFamily::Contextual,
        pattern: r"(?s)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----(?P<secret>.+?)-----END [A-Z0-9 ]*PRIVATE KEY-----",
    },
];

/// Literal prefixes of the [`SecretFamily::CredentialToken`] shapes that
/// identify a whole value on their own. Wrapper argument sanitization masks
/// any argument starting with one, whatever its length.
pub const CREDENTIAL_TOKEN_PREFIXES: &[&str] = &[
    "sk-",
    "AKIA",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "xoxa-",
    "xoxb-",
    "xoxp-",
    "xoxr-",
    "xoxs-",
];

static COMPILED: LazyLock<Vec<(SecretRule, Regex)>> = LazyLock::new(|| {
    SECRET_CATALOG
        .iter()
        .map(|rule| {
            let regex = Regex::new(rule.pattern)
                .unwrap_or_else(|error| panic!("invalid secret regex {}: {error}", rule.id));
            (*rule, regex)
        })
        .collect()
});

/// Compiled rules of one family, in catalog order, for consumers that replace
/// whole matches with their own token.
pub fn family_regexes(family: SecretFamily) -> impl Iterator<Item = &'static Regex> {
    COMPILED
        .iter()
        .filter(move |(rule, _)| rule.family == family)
        .map(|(_, regex)| regex)
}

/// Whether a key, header, flag, or environment-variable name names a secret.
///
/// Case-insensitive; `-` and `_` are equivalent, so `x-api-key`, `API_KEY`,
/// and `apiKey` all match. `PUBLIC_KEY` does not.
pub fn is_sensitive_key_name(key: &str) -> bool {
    let key = key.to_ascii_uppercase().replace('-', "_");
    const CONTAINS: &[&str] = &[
        "API_KEY",
        "APIKEY",
        "AUTHORIZATION",
        "TOKEN",
        "PASSWORD",
        "SECRET",
        "PRIVATE_KEY",
        "CREDENTIAL",
        "ACCESS_KEY",
        "PASSPHRASE",
    ];
    const SUFFIXES: &[&str] = &["_AUTH", "_PAT", "_PWD", "_PEM"];
    CONTAINS.iter().any(|part| key.contains(part))
        || (key.ends_with("_KEY") && !key.contains("PUBLIC_KEY"))
        || SUFFIXES.iter().any(|suffix| key.ends_with(suffix))
}

/// Whether a whole value starts with a [`CREDENTIAL_TOKEN_PREFIXES`] entry.
pub fn has_credential_prefix(value: &str) -> bool {
    CREDENTIAL_TOKEN_PREFIXES
        .iter()
        .any(|prefix| value.starts_with(prefix))
}

/// Byte ranges of every recognized secret in `text`, sorted, with overlapping
/// and touching ranges merged. Every bound is a UTF-8 character boundary.
pub fn find_secret_spans(text: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    for (_, regex) in COMPILED.iter() {
        collect_rule_spans(regex, text, &mut spans);
    }
    merge(spans)
}

/// `text` with every recognized secret span replaced by [`MASK`]. Masking is
/// idempotent: masking the result again changes nothing.
pub fn mask_secrets(text: &str) -> Cow<'_, str> {
    replace_spans(text, find_secret_spans(text))
}

fn collect_rule_spans(regex: &Regex, text: &str, spans: &mut Vec<Range<usize>>) {
    let has_key = regex.capture_names().any(|name| name == Some("key"));
    let mut position = 0;
    while position < text.len() {
        let Some(captures) = regex.captures_at(text, position) else {
            break;
        };
        let whole = captures.get(0).expect("group 0 always participates");
        if has_key {
            let key = captures.name("key").expect("key group participates");
            if !is_sensitive_key_name(key.as_str()) {
                // Resume inside the rejected match: the value it consumed may
                // hold a sensitive key of its own (`https://h/?token=x`).
                position = next_char_boundary(text, key.end());
                continue;
            }
        }
        if let Some(span) = secret_span(regex, &captures) {
            spans.push(span);
        }
        position = if whole.end() > whole.start() {
            whole.end()
        } else {
            next_char_boundary(text, whole.end())
        };
    }
}

fn secret_span(regex: &Regex, captures: &Captures<'_>) -> Option<Range<usize>> {
    for name in regex.capture_names().flatten() {
        let Some(group) = captures.name(name) else {
            continue;
        };
        if name == "loose" {
            let value = trim_trailing_punctuation(group.as_str());
            return looks_like_credential(value).then(|| group.start()..group.start() + value.len());
        }
        if name == "secret_bare" {
            let value = trim_trailing_punctuation(group.as_str());
            return (!value.is_empty()).then(|| group.start()..group.start() + value.len());
        }
        if name.starts_with("secret") {
            return Some(group.range());
        }
    }
    let whole = captures.get(0).expect("group 0 always participates");
    Some(whole.range())
}

/// Sentence punctuation that follows an unquoted value in prose.
fn trim_trailing_punctuation(value: &str) -> &str {
    value.trim_end_matches(['.', '!', '?', ')', ':'])
}

/// A value after `key:` or `--flag ` is masked only when it looks like a
/// credential rather than a word: long, or containing a non-letter.
fn looks_like_credential(value: &str) -> bool {
    value.len() >= 16 || value.chars().any(|c| !c.is_alphabetic())
}

fn next_char_boundary(text: &str, from: usize) -> usize {
    let mut next = from + 1;
    while next < text.len() && !text.is_char_boundary(next) {
        next += 1;
    }
    next
}

fn merge(mut spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    spans.retain(|span| span.start < span.end);
    spans.sort_by_key(|span| (span.start, span.end));
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(spans.len());
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

fn replace_spans(text: &str, spans: Vec<Range<usize>>) -> Cow<'_, str> {
    if spans.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for span in spans {
        out.push_str(&text[cursor..span.start]);
        out.push_str(MASK);
        cursor = span.end;
    }
    out.push_str(&text[cursor..]);
    Cow::Owned(out)
}

/// Steering redaction for one message and everything that may echo it.
///
/// Built from the original message, it masks recognized spans and also every
/// later occurrence of a recognized value of at least
/// [`MIN_KNOWN_SECRET_BYTES`], so an error that echoes only `hunter2` is
/// masked once `password=hunter2` was recognized in the message. The values
/// stay in memory; `Debug` prints only their count.
#[derive(Clone, Default)]
pub struct Redactor {
    known: Vec<String>,
}

impl Redactor {
    /// Learns the recognized secret values of `message`.
    pub fn for_message(message: &str) -> Self {
        let mut known: Vec<String> = find_secret_spans(message)
            .into_iter()
            .map(|span| message[span].to_string())
            .filter(|value| value.len() >= MIN_KNOWN_SECRET_BYTES && value != MASK)
            .collect();
        known.sort();
        known.dedup();
        Self { known }
    }

    /// Also masks every occurrence of `values`, for secrets a caller
    /// identified by position rather than by shape (the value after
    /// `--password`). Values shorter than [`MIN_KNOWN_SECRET_BYTES`] are
    /// ignored, as for learned values.
    pub fn with_known_values(mut self, values: impl IntoIterator<Item = String>) -> Self {
        self.known.extend(
            values
                .into_iter()
                .filter(|value| value.len() >= MIN_KNOWN_SECRET_BYTES && value != MASK),
        );
        self.known.sort();
        self.known.dedup();
        self
    }

    /// `text` with recognized spans and known values masked.
    pub fn redact(&self, text: &str) -> RedactedText {
        let mut spans = find_secret_spans(text);
        for value in &self.known {
            spans.extend(text.match_indices(value.as_str()).map(|(start, found)| start..start + found.len()));
        }
        RedactedText(replace_spans(text, merge(spans)).into_owned())
    }
}

impl fmt::Debug for Redactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Redactor({} known values)", self.known.len())
    }
}

/// Text that has passed through a [`Redactor`]. It is the only text type
/// steering audit records, traces, and rendered errors accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedText(String);

impl RedactedText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RedactedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for RedactedText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}
