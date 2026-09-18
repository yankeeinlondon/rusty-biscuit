//! Shared scalars, identifiers, knowledge, evidence, and conditions.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// Declares a closed string enum whose serialized spellings are exactly the
/// schema's enum literals, with `ALL` in declaration order and `as_str`.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize,
        )]
        $vis enum $name {
            $($(#[$vmeta])* #[serde(rename = $text)] $variant),+
        }

        impl $name {
            /// Every value, in schema declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The schema spelling.
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}
pub(crate) use string_enum;

/// A calendar date in `YYYY-MM-DD` form.
///
/// Ordering is chronological because the representation is fixed-width.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Date(String);

impl Date {
    /// Parses `YYYY-MM-DD`, checking month and day ranges (not leap years).
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let digits = |range: std::ops::Range<usize>| {
            bytes[range.clone()].iter().all(u8::is_ascii_digit)
                && text[range].parse::<u32>().is_ok()
        };
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        if !(digits(0..4) && digits(5..7) && digits(8..10)) {
            return None;
        }
        let month: u32 = text[5..7].parse().ok()?;
        let day: u32 = text[8..10].parse().ok()?;
        ((1..=12).contains(&month) && (1..=31).contains(&day)).then(|| Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Date::parse(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid date `{text}`, expected YYYY-MM-DD")))
    }
}

string_enum! {
    /// The five researched chat platforms; roster keys and document stems.
    pub enum PlatformId {
        Discord => "discord",
        Slack => "slack",
        Telegram => "telegram",
        WhatsApp => "whatsapp",
        Signal => "signal",
    }
}

string_enum! {
    /// Messenger adapter identities: exactly `ProviderKind::as_str()` for the
    /// seven chat adapters, already persisted in receipts and route config.
    pub enum AdapterId {
        Discord => "discord",
        DiscordWebhook => "discord-webhook",
        Slack => "slack",
        SlackWebhook => "slack-webhook",
        Telegram => "telegram",
        WhatsApp => "whatsapp",
        Signal => "signal",
    }
}

impl AdapterId {
    /// The platform an adapter delivers to.
    pub fn platform(self) -> PlatformId {
        match self {
            AdapterId::Discord | AdapterId::DiscordWebhook => PlatformId::Discord,
            AdapterId::Slack | AdapterId::SlackWebhook => PlatformId::Slack,
            AdapterId::Telegram => PlatformId::Telegram,
            AdapterId::WhatsApp => PlatformId::WhatsApp,
            AdapterId::Signal => PlatformId::Signal,
        }
    }
}

string_enum! {
    /// Knowledge state of one fact.
    pub enum State {
        Known => "known",
        Unknown => "unknown",
        Conflicting => "conflicting",
        NotApplicable => "not_applicable",
    }
}

string_enum! {
    pub enum Confidence {
        High => "high",
        Medium => "medium",
        Low => "low",
    }
}

string_enum! {
    /// A capability value; always wrapped in a knowledge state.
    pub enum Support {
        Supported => "supported",
        Conditional => "conditional",
        Unsupported => "unsupported",
    }
}

string_enum! {
    pub enum Fidelity {
        Exact => "exact",
        Approximate => "approximate",
        Unmapped => "unmapped",
    }
}

string_enum! {
    pub enum YesNo {
        Yes => "yes",
        No => "no",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum SourceKind {
        OfficialDocs => "official_docs",
        SourceCode => "source_code",
        SdkValidator => "sdk_validator",
        ObservedFixture => "observed_fixture",
        Secondary => "secondary",
    }
}

string_enum! {
    /// Measurement unit of a bound. Only some units resolve how to count.
    pub enum Unit {
        Utf8Bytes => "utf8_bytes",
        UnicodeScalars => "unicode_scalars",
        Utf16CodeUnits => "utf16_code_units",
        GraphemeClusters => "grapheme_clusters",
        Items => "items",
        UnspecifiedCharacters => "unspecified_characters",
        Bytes => "bytes",
        Unknown => "unknown",
    }
}

impl Unit {
    /// Units that measure text content (as opposed to items or raw bytes).
    pub fn is_text(self) -> bool {
        matches!(
            self,
            Unit::Utf8Bytes
                | Unit::UnicodeScalars
                | Unit::Utf16CodeUnits
                | Unit::GraphemeClusters
                | Unit::UnspecifiedCharacters
        )
    }

    /// Whether the unit states how to count; `unspecified_characters` and
    /// `unknown` do not.
    pub fn is_resolved(self) -> bool {
        !matches!(self, Unit::UnspecifiedCharacters | Unit::Unknown)
    }
}

string_enum! {
    /// Stage at which a bound is measured.
    pub enum Stage {
        FieldValue => "field_value",
        ParsedText => "parsed_text",
        SerializedPayload => "serialized_payload",
        Unknown => "unknown",
    }
}

string_enum! {
    /// Research categories. The same list keys the per-interface coverage
    /// matrix and scopes implementation assessments.
    pub enum Category {
        Versions => "versions",
        Constraints => "constraints",
        Formatting => "formatting",
        TextBindings => "text_bindings",
        Images => "images",
        Attachments => "attachments",
        Addressing => "addressing",
        Receipts => "receipts",
        Attribution => "attribution",
        Location => "location",
        Expression => "expression",
        Interactivity => "interactivity",
        DeliveryControls => "delivery_controls",
        Eligibility => "eligibility",
        RateLimits => "rate_limits",
        Errors => "errors",
    }
}

string_enum! {
    /// Provider-neutral semantic surfaces.
    pub enum Surface {
        Body => "body",
        Summary => "summary",
        Caption => "caption",
        AttachmentDescription => "attachment_description",
        AltText => "alt_text",
        Filename => "filename",
        NotificationFallback => "notification_fallback",
        RichTitle => "rich_title",
        RichDescription => "rich_description",
        RichFieldName => "rich_field_name",
        RichFieldValue => "rich_field_value",
        RichFooterText => "rich_footer_text",
        RichAuthorName => "rich_author_name",
        RichObjects => "rich_objects",
        RichFields => "rich_fields",
        BlockText => "block_text",
        Blocks => "blocks",
        Attachments => "attachments",
        Attachment => "attachment",
        MediaGroup => "media_group",
        RequestPayload => "request_payload",
        TemplateHeader => "template_header",
        TemplateBody => "template_body",
        TemplateFooter => "template_footer",
        TemplateParameter => "template_parameter",
        InteractiveHeader => "interactive_header",
        InteractiveBody => "interactive_body",
        InteractiveFooter => "interactive_footer",
        ButtonLabel => "button_label",
        ListTitle => "list_title",
        ListRowTitle => "list_row_title",
        ListRowDescription => "list_row_description",
        PollQuestion => "poll_question",
        PollOption => "poll_option",
        OptionLabel => "option_label",
        OptionValue => "option_value",
        CallbackData => "callback_data",
        LocationLabel => "location_label",
        LocationAddress => "location_address",
        SenderDisplayName => "sender_display_name",
    }
}

impl Surface {
    /// Collection surfaces: members and subjects of count bounds, measured in
    /// `items` rather than a text unit.
    pub fn is_collection(self) -> bool {
        matches!(
            self,
            Surface::RichObjects
                | Surface::RichFields
                | Surface::Blocks
                | Surface::Attachments
                | Surface::MediaGroup
        )
    }
}

/// One competing value of a `conflicting` record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub statement: String,
    pub value: Option<f64>,
    pub evidence: Vec<String>,
}

/// The knowledge block every fact carries.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Knowledge {
    pub state: State,
    pub evidence: Vec<String>,
    pub confidence: Option<Confidence>,
    /// The gap record for `unknown` and `conflicting` states.
    pub gap: Option<String>,
    pub explanation: Option<String>,
    #[serde(default)]
    pub claims: Vec<Claim>,
}

/// An evidence source, cited by ID from facts in the same document.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    pub url: Option<String>,
    /// A repository-relative evidence path.
    pub location: Option<String>,
    pub locator: Option<String>,
    /// Observation date, never a release date.
    pub retrieved: Option<Date>,
    pub revision: Option<String>,
    pub note: Option<String>,
}

string_enum! {
    pub enum ConditionKind {
        MessageForm => "message_form",
        AccountTier => "account_tier",
        ApiVersion => "api_version",
        SdkVersion => "sdk_version",
        BridgeVersion => "bridge_version",
        HostingMode => "hosting_mode",
        MediaKind => "media_kind",
        RecipientClient => "recipient_client",
        ConversationType => "conversation_type",
        PermissionGrant => "permission_grant",
        ReleaseAge => "release_age",
    }
}

/// A bounded, typed applicability condition.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub kind: ConditionKind,
    pub equals: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub one_of: Option<Vec<String>>,
    /// Names the gap when no operand can express the condition; such a
    /// condition makes its record non-executable.
    pub gap: Option<String>,
}

/// The operand form a condition resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand<'a> {
    Equals(&'a str),
    Range { min: Option<&'a str>, max: Option<&'a str> },
    OneOf(&'a [String]),
}

impl Condition {
    /// The single operand form, or `None` when the condition has no operand
    /// or mixes forms (both are SR-CONDITION findings unless a gap is named).
    pub fn operand(&self) -> Option<Operand<'_>> {
        let equals = self.equals.is_some();
        let range = self.min.is_some() || self.max.is_some();
        let one_of = self.one_of.is_some();
        match (equals, range, one_of) {
            (true, false, false) => self.equals.as_deref().map(Operand::Equals),
            (false, true, false) => Some(Operand::Range {
                min: self.min.as_deref(),
                max: self.max.as_deref(),
            }),
            (false, false, true) => self.one_of.as_deref().map(Operand::OneOf),
            _ => None,
        }
    }

    /// Number of operand forms present.
    pub fn operand_forms(&self) -> usize {
        usize::from(self.equals.is_some())
            + usize::from(self.min.is_some() || self.max.is_some())
            + usize::from(self.one_of.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_parse_only_the_fixed_iso_form() {
        assert!(Date::parse("2026-09-17").is_some());
        for invalid in ["2026-9-17", "2026-13-01", "2026-00-10", "2026-01-32", "26-09-17", "2026/09/17", "2026-09-17T00:00"] {
            assert!(Date::parse(invalid).is_none(), "{invalid}");
        }
        assert!(Date::parse("2026-09-17") < Date::parse("2027-01-01"));
    }

    #[test]
    fn conditions_resolve_to_exactly_one_operand_form() {
        let condition = |equals: Option<&str>, min: Option<&str>, one_of: Option<Vec<String>>| Condition {
            kind: ConditionKind::ApiVersion,
            equals: equals.map(str::to_string),
            min: min.map(str::to_string),
            max: None,
            one_of,
            gap: None,
        };
        assert_eq!(condition(Some("10"), None, None).operand(), Some(Operand::Equals("10")));
        assert!(matches!(condition(None, Some("9"), None).operand(), Some(Operand::Range { .. })));
        assert_eq!(condition(None, None, None).operand(), None);
        assert_eq!(condition(Some("10"), Some("9"), None).operand(), None);
        assert_eq!(condition(Some("10"), Some("9"), Some(vec![])).operand_forms(), 3);
    }
}
