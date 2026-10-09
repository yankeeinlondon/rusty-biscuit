//! The serialized normalized policy and the policy identity.
//!
//! A normalized policy is JSON of the form
//! `{"grammar_version":1,"entries":[{"rule":"ValidFor(3mo)","action":"refresh"}]}`.
//! It holds entries only; evidence stays outside it.

use std::fmt;

use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::diagnostic::{Diagnostic, DiagnosticCode, Invalid, Location};
use crate::grammar;
use crate::model::{Baseline, GRAMMAR_VERSION, Policy, PolicyEntry, Rule};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalizedPolicy {
    grammar_version: u32,
    #[serde(deserialize_with = "entries_naming_the_element")]
    entries: Vec<NormalizedEntry>,
}

/// Deserializes the entry list, prefixing any element error with its
/// position so the diagnostic names the element.
fn entries_naming_the_element<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<NormalizedEntry>, D::Error> {
    struct Entries;

    impl<'de> Visitor<'de> for Entries {
        type Value = Vec<NormalizedEntry>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a list of {rule, action} entries")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut entries = Vec::new();
            loop {
                let index = entries.len();
                match seq.next_element::<NormalizedEntry>() {
                    Ok(Some(entry)) => entries.push(entry),
                    Ok(None) => return Ok(entries),
                    Err(error) => {
                        return Err(de::Error::custom(format!("entry {}: {error}", index + 1)));
                    }
                }
            }
        }
    }

    deserializer.deserialize_seq(Entries)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalizedEntry {
    rule: String,
    action: String,
}

fn malformed(message: impl Into<String>) -> Invalid {
    Invalid::new(vec![Diagnostic::new(
        DiagnosticCode::MalformedSerialization,
        Location::Policy,
        message,
    )])
}

impl Policy {
    /// Serializes the normalized policy as compact JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        let normalized = NormalizedPolicy {
            grammar_version: GRAMMAR_VERSION,
            entries: self
                .entries()
                .iter()
                .map(|entry| NormalizedEntry {
                    rule: entry.rule.to_string(),
                    action: entry.action.as_str().to_string(),
                })
                .collect(),
        };
        serde_json::to_string(&normalized).expect("the normalized policy always serializes")
    }

    /// Reads a policy written by [`Policy::to_json`].
    ///
    /// ## Errors
    ///
    /// Returns [`Invalid`] for any other shape: a missing, `null`, or
    /// wrong-typed field, an unknown or duplicated field, trailing content,
    /// an empty entry list, an invalid rule or action, or a grammar version
    /// newer than [`GRAMMAR_VERSION`].
    pub fn from_json(json: &str) -> Result<Self, Invalid> {
        let normalized: NormalizedPolicy =
            serde_json::from_str(json).map_err(|error| malformed(error.to_string()))?;
        if normalized.grammar_version > GRAMMAR_VERSION {
            return Err(Invalid::new(vec![Diagnostic::new(
                DiagnosticCode::UnsupportedGrammarVersion,
                Location::Policy,
                format!(
                    "grammar version {} is newer than this library reads ({GRAMMAR_VERSION})",
                    normalized.grammar_version
                ),
            )]));
        }
        if normalized.grammar_version == 0 {
            return Err(malformed("grammar version 0 does not exist"));
        }
        let declaration = serde_json::Value::Array(
            normalized
                .entries
                .into_iter()
                .map(|entry| serde_json::json!({ "rule": entry.rule, "action": entry.action }))
                .collect(),
        );
        grammar::parse_declaration(&declaration)
    }

    /// A stable digest of the policy's rules and actions:
    /// `xxh64:` followed by 16 lowercase hex digits.
    ///
    /// Inline baseline dates are excluded, so renewal never changes it; an
    /// `@name` reference counts by name. Entry order is excluded, because it
    /// never changes a verdict. The grammar version is included.
    #[must_use]
    pub fn identity(&self) -> String {
        let mut entries: Vec<String> = self
            .entries()
            .iter()
            .map(|entry| {
                serde_json::to_string(&IdentityEntry::from(entry))
                    .expect("identity entries always serialize")
            })
            .collect();
        entries.sort();
        let canonical = format!(
            "{{\"grammar_version\":{GRAMMAR_VERSION},\"entries\":[{}]}}",
            entries.join(",")
        );
        format!(
            "xxh64:{:016x}",
            biscuit_hash::xx_hash_bytes(canonical.as_bytes())
        )
    }
}

/// One entry as hashed: the rule name, its parameters with any inline date
/// replaced by the marker `inline`, and the action. A `FileChanged` path
/// counts as authored; its stored fingerprint is evidence and never does.
#[derive(Serialize)]
struct IdentityEntry {
    rule: &'static str,
    parameters: Vec<String>,
    action: &'static str,
}

impl From<&PolicyEntry> for IdentityEntry {
    fn from(entry: &PolicyEntry) -> Self {
        let parameters = match &entry.rule {
            Rule::Evergreen | Rule::TimeSensitive => Vec::new(),
            Rule::ValidFor { duration, baseline } => {
                let baseline = match baseline {
                    Baseline::Inline(_) => "inline".to_string(),
                    Baseline::Reference(name) => format!("@{name}"),
                    Baseline::Defaulted => "default".to_string(),
                };
                vec![duration.to_string(), baseline]
            }
            Rule::ValidUntil { deadline } => vec![match deadline {
                crate::model::Deadline::Inline(date) => date.to_string(),
                crate::model::Deadline::Reference(name) => format!("@{name}"),
            }],
            Rule::FileChanged { path, property } => vec![path.clone(), format!("@{property}")],
        };
        Self {
            rule: entry.rule.name(),
            parameters,
            action: entry.action.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn policy(entries: serde_json::Value) -> Policy {
        Policy::from_declaration(&entries).unwrap()
    }

    fn identity(entries: serde_json::Value) -> String {
        policy(entries).identity()
    }

    #[test]
    fn identity_has_the_xxh64_shape() {
        let id = identity(json!(["ValidFor(3mo)"]));
        assert!(id.starts_with("xxh64:") && id.len() == 22, "{id}");
        assert!(id[6..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    }

    #[test]
    fn identity_ignores_inline_baselines_and_order() {
        assert_eq!(
            identity(json!(["ValidFor(3mo, 2026-09-28)"])),
            identity(json!(["ValidFor(3mo, 2026-12-29)"]))
        );
        assert_eq!(
            identity(json!(["ValidFor(3mo)", {"rule": "ValidUntil(2027-01-01)", "action": "archive"}])),
            identity(json!([{"rule": "ValidUntil(2027-01-01)", "action": "archive"}, "ValidFor(3mo)"]))
        );
        // The compact form and an explicit `refresh` are the same entry.
        assert_eq!(
            identity(json!(["ValidFor(3mo)"])),
            identity(json!([{"rule": "ValidFor(3mo)", "action": "refresh"}]))
        );
    }

    #[test]
    fn identity_changes_with_every_policy_field() {
        let base = identity(json!(["ValidFor(3mo, @last_updated)"]));
        for changed in [
            json!(["ValidFor(6mo, @last_updated)"]),
            json!(["ValidFor(3wk, @last_updated)"]),
            json!(["ValidFor(3mo, @reviewed)"]),
            json!(["ValidFor(3mo)"]),
            json!(["ValidFor(3mo, 2026-09-28)"]),
            json!([{"rule": "ValidFor(3mo, @last_updated)", "action": "archive"}]),
            json!(["ValidUntil(@last_updated)"]),
            json!(["TimeSensitive"]),
        ] {
            assert_ne!(identity(changed.clone()), base, "{changed}");
        }
        assert_ne!(
            identity(json!(["ValidUntil(2027-01-01)"])),
            identity(json!(["ValidUntil(2027-01-02)"]))
        );
        assert_ne!(
            identity(json!(["ValidUntil(2027-01-01)"])),
            identity(json!(["ValidUntil(2027-01-01)", "ValidUntil(2027-01-01)"]))
        );
    }

    #[test]
    fn json_round_trips() {
        let original = policy(json!([
            "ValidFor(3mo, 2026-09-28)",
            {"rule": "ValidUntil(@deadline)", "action": "remove"},
        ]));
        let json = original.to_json();
        assert_eq!(
            json,
            r#"{"grammar_version":1,"entries":[{"rule":"ValidFor(3mo, 2026-09-28)","action":"refresh"},{"rule":"ValidUntil(@deadline)","action":"remove"}]}"#
        );
        let reread = Policy::from_json(&json).unwrap();
        assert_eq!(reread, original);
        assert_eq!(reread.to_json(), json);
    }
}
