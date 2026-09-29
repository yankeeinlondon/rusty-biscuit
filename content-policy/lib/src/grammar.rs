//! Declaration grammar: compact rule strings, long-form entries, and the
//! policy value that holds them.

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;

use crate::diagnostic::{Diagnostic, DiagnosticCode, EntryField, Invalid, Location};
use crate::model::{
    Action, Baseline, Deadline, Duration, DurationUnit, Policy, PolicyEntry, Rule,
};
use crate::time;

const RULE_NAMES: [&str; 4] = ["Evergreen", "TimeSensitive", "ValidFor", "ValidUntil"];

/// A rule-level failure; the caller decides its [`Location`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleError {
    pub code: DiagnosticCode,
    pub message: String,
}

impl RuleError {
    fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn at(self, location: Location) -> Diagnostic {
        Diagnostic::new(self.code, location, self.message)
    }
}

/// How a string in a date position reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DateText {
    Date(NaiveDate),
    /// A real date followed by a time part, such as `2026-09-28T10:00:00Z`.
    Timestamp,
    NotADate,
}

fn has_date_shape(bytes: &[u8]) -> bool {
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

pub(crate) fn classify_date(text: &str) -> DateText {
    let bytes = text.as_bytes();
    let parse = |slice: &str| NaiveDate::parse_from_str(slice, "%Y-%m-%d").ok();
    if has_date_shape(bytes) {
        return parse(text).map_or(DateText::NotADate, DateText::Date);
    }
    if bytes.len() > 10
        && has_date_shape(&bytes[..10])
        && matches!(bytes[10], b'T' | b't' | b' ')
        && parse(&text[..10]).is_some()
    {
        return DateText::Timestamp;
    }
    DateText::NotADate
}

/// The diagnostic for a string that is not a usable date, or `None` for a date.
pub(crate) fn date_error(text: &str) -> Result<NaiveDate, RuleError> {
    match classify_date(text) {
        DateText::Date(date) => Ok(date),
        DateText::Timestamp => Err(RuleError::new(
            DiagnosticCode::DatesOnly,
            format!("`{text}` is a timestamp; only `YYYY-MM-DD` dates are supported"),
        )),
        DateText::NotADate => Err(RuleError::new(
            DiagnosticCode::InvalidDate,
            format!("`{text}` is not a `YYYY-MM-DD` calendar date"),
        )),
    }
}

fn parse_duration(text: &str) -> Result<Duration, RuleError> {
    let invalid = || {
        RuleError::new(
            DiagnosticCode::InvalidDuration,
            format!(
                "`{text}` is not a duration; write a positive whole number and one unit: \
                 `d`, `wk`, `mo`, or `yr` (for example `3mo`)"
            ),
        )
    };
    let digits_end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, unit) = text.split_at(digits_end);
    if digits.is_empty() || digits.starts_with('0') {
        return Err(invalid());
    }
    let count: u32 = digits.parse().map_err(|_| invalid())?;
    let unit = match unit {
        "d" => DurationUnit::Days,
        "wk" => DurationUnit::Weeks,
        "mo" => DurationUnit::Months,
        "yr" => DurationUnit::Years,
        _ => return Err(invalid()),
    };
    Duration::new(count, unit).ok_or_else(invalid)
}

/// Returns `true` for a name an `@name` reference may select.
pub(crate) fn is_property_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_reference(text: &str) -> Result<String, RuleError> {
    let name = text.strip_prefix('@').unwrap_or(text);
    if name.contains('.') {
        return Err(RuleError::new(
            DiagnosticCode::NestedReference,
            format!(
                "`{text}` is a nested path; a reference selects one top-level property, \
                 such as `@last_updated`"
            ),
        ));
    }
    if !is_property_name(name) {
        return Err(RuleError::new(
            DiagnosticCode::InvalidReference,
            format!("`{text}` is not a property reference such as `@last_updated`"),
        ));
    }
    Ok(name.to_string())
}

fn unknown_rule(name: &str) -> RuleError {
    let message = if let Some(known) = RULE_NAMES
        .iter()
        .find(|known| known.eq_ignore_ascii_case(name))
    {
        format!("unknown rule `{name}`; rule names are case-sensitive, write `{known}`")
    } else if name == "Duration" {
        "unknown rule `Duration`; write `ValidFor` instead, such as `ValidFor(3mo)`".to_string()
    } else {
        format!(
            "unknown rule `{name}`; expected one of {}",
            RULE_NAMES.map(|known| format!("`{known}`")).join(", ")
        )
    };
    RuleError::new(DiagnosticCode::UnknownRule, message)
}

fn arguments_error(message: impl Into<String>) -> RuleError {
    RuleError::new(DiagnosticCode::InvalidArguments, message)
}

/// Parses one compact rule string, such as `ValidFor(3mo, @last_updated)`.
pub(crate) fn parse_rule(text: &str) -> Result<Rule, RuleError> {
    if text.is_empty() {
        return Err(RuleError::new(
            DiagnosticCode::InvalidRuleSyntax,
            "the rule is empty",
        ));
    }
    if text.trim() != text {
        return Err(RuleError::new(
            DiagnosticCode::InvalidRuleSyntax,
            format!("`{text}` has leading or trailing whitespace"),
        ));
    }
    let opens = text.matches('(').count();
    let closes = text.matches(')').count();
    if opens != closes {
        return Err(RuleError::new(
            DiagnosticCode::UnbalancedParentheses,
            format!("`{text}` has unbalanced parentheses"),
        ));
    }
    let name_end = text
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(text.len());
    let (name, rest) = text.split_at(name_end);
    if name.is_empty() {
        return Err(RuleError::new(
            DiagnosticCode::InvalidRuleSyntax,
            format!("`{text}` is not a rule such as `ValidFor(3mo)`"),
        ));
    }
    let arguments = if rest.is_empty() {
        None
    } else {
        let inner = rest
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'))
            .filter(|inner| !inner.contains(['(', ')']))
            .ok_or_else(|| {
                RuleError::new(
                    DiagnosticCode::InvalidRuleSyntax,
                    format!("`{text}` is not `Name` or `Name(arguments)`"),
                )
            })?;
        Some(inner.split(',').map(|arg| arg.trim_matches(' ')).collect::<Vec<_>>())
    };
    if !RULE_NAMES.contains(&name) {
        return Err(unknown_rule(name));
    }
    if let Some(args) = &arguments
        && args.iter().any(|arg| arg.is_empty())
    {
        return Err(arguments_error(format!("`{text}` has an empty argument")));
    }
    match (name, arguments.as_deref()) {
        ("Evergreen", None) => Ok(Rule::Evergreen),
        ("TimeSensitive", None) => Ok(Rule::TimeSensitive),
        ("Evergreen" | "TimeSensitive", Some(_)) => {
            Err(arguments_error(format!("`{name}` takes no arguments")))
        }
        ("ValidFor", Some([duration])) => Ok(Rule::ValidFor {
            duration: parse_duration(duration)?,
            baseline: Baseline::Defaulted,
        }),
        ("ValidFor", Some([duration, baseline])) => {
            let duration = parse_duration(duration)?;
            let baseline = if baseline.starts_with('@') {
                Baseline::Reference(parse_reference(baseline)?)
            } else {
                let date = date_error(baseline)?;
                if time::add_duration(date, duration).is_none() {
                    return Err(out_of_range(text));
                }
                Baseline::Inline(date)
            };
            Ok(Rule::ValidFor { duration, baseline })
        }
        ("ValidFor", _) => Err(arguments_error(
            "`ValidFor` takes a duration and an optional baseline: `ValidFor(3mo)`, \
             `ValidFor(3mo, @last_updated)`, or `ValidFor(3mo, 2026-09-28)`",
        )),
        ("ValidUntil", Some([deadline])) => {
            let deadline = if deadline.starts_with('@') {
                Deadline::Reference(parse_reference(deadline)?)
            } else {
                Deadline::Inline(date_error(deadline)?)
            };
            Ok(Rule::ValidUntil { deadline })
        }
        ("ValidUntil", _) => Err(arguments_error(
            "`ValidUntil` takes one deadline: `ValidUntil(2027-01-01)` or `ValidUntil(@name)`",
        )),
        _ => unreachable!("every known rule name is matched above"),
    }
}

pub(crate) fn out_of_range(text: &str) -> RuleError {
    RuleError::new(
        DiagnosticCode::DateOutOfRange,
        format!("`{text}` computes a date outside the supported calendar range"),
    )
}

fn paren_balance(value: &Value) -> Option<isize> {
    let text = value.as_str()?;
    let count = |c| isize::try_from(text.matches(c).count()).unwrap_or(isize::MAX);
    Some(count('(') - count(')'))
}

/// A rule split at its comma by a flow list leaves an unclosed half followed
/// by an unopened half; name both and show the block-list form.
fn comma_split_message(items: &[Value], index: usize, text: &str) -> String {
    let joined = match paren_balance(&items[index]) {
        Some(balance) if balance > 0 => items
            .get(index + 1)
            .filter(|next| paren_balance(next).is_some_and(|b| b < 0))
            .and_then(Value::as_str)
            .map(|next| format!("{text}, {next}")),
        Some(balance) if balance < 0 && index > 0 => items
            .get(index - 1)
            .filter(|previous| paren_balance(previous).is_some_and(|b| b > 0))
            .and_then(Value::as_str)
            .map(|previous| format!("{previous}, {text}")),
        _ => None,
    };
    match joined {
        Some(rule) => format!(
            "`{text}` has unbalanced parentheses: a one-line `[...]` list splits items at \
             every comma, so `{rule}` became two entries; write the policy as a block list \
             instead:\n  - {rule}"
        ),
        None => format!(
            "`{text}` has unbalanced parentheses; if it came from a one-line `[...]` list, \
             the list split a rule at its comma; write the policy as a block list instead, \
             one `- Rule(...)` line per entry"
        ),
    }
}

pub(crate) fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "a mapping",
    }
}

fn parse_long_form(
    index: usize,
    map: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PolicyEntry> {
    let entry = Location::Entry { index };
    let field = |field| Location::EntryField { index, field };
    let before = diagnostics.len();
    for key in map.keys().filter(|key| !matches!(key.as_str(), "rule" | "action")) {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::UnknownField,
            entry.clone(),
            format!("unknown key `{key}`; a long-form entry has only `rule` and `action`"),
        ));
    }
    let rule = match map.get("rule") {
        None => {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::MissingRule,
                entry.clone(),
                "a `{rule, action}` entry needs a `rule`",
            ));
            None
        }
        Some(Value::String(text)) => match parse_rule(text) {
            Ok(rule) => Some(rule),
            Err(error) => {
                diagnostics.push(error.at(field(EntryField::Rule)));
                None
            }
        },
        Some(other) => {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::WrongType,
                field(EntryField::Rule),
                format!("`rule` must be a rule string, found {}", type_name(other)),
            ));
            None
        }
    };
    let action = match map.get("action") {
        None => {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::MissingAction,
                entry,
                "a `{rule, action}` entry needs an `action` (`refresh`, `archive`, or \
                 `remove`); write the rule as a plain list item to use the default `refresh`",
            ));
            None
        }
        Some(Value::String(text)) => match Action::parse(text) {
            Some(action) => Some(action),
            None => {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::UnknownAction,
                    field(EntryField::Action),
                    format!(
                        "unknown action `{text}`; expected `refresh`, `archive`, or `remove` \
                         (case-sensitive)"
                    ),
                ));
                None
            }
        },
        Some(other) => {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::WrongType,
                field(EntryField::Action),
                format!("`action` must be a string, found {}", type_name(other)),
            ));
            None
        }
    };
    match (rule, action) {
        (Some(rule), Some(action)) if diagnostics.len() == before => {
            Some(PolicyEntry::with_action(rule, action))
        }
        _ => None,
    }
}

fn parse_entry(
    items: &[Value],
    index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PolicyEntry> {
    let location = Location::Entry { index };
    match &items[index] {
        Value::String(text) => match parse_rule(text) {
            Ok(rule) => Some(PolicyEntry::new(rule)),
            Err(mut error) => {
                if error.code == DiagnosticCode::UnbalancedParentheses {
                    error.message = comma_split_message(items, index, text);
                }
                diagnostics.push(error.at(location));
                None
            }
        },
        Value::Object(map) => parse_long_form(index, map, diagnostics),
        Value::Null => {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::NullEntry,
                location,
                "the entry has no value",
            ));
            None
        }
        other => {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::WrongType,
                location,
                format!(
                    "an entry must be a rule string or a `{{rule, action}}` mapping, found {}",
                    type_name(other)
                ),
            ));
            None
        }
    }
}

fn evergreen_combined(index: usize) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::EvergreenCombined,
        Location::Entry { index },
        "`Evergreen` must be the only entry; it says the document never expires, which \
         contradicts every other rule",
    )
}

fn empty_policy() -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::EmptyPolicy,
        Location::Policy,
        "an empty policy has no meaning; to declare that the document never expires, write:\n  \
         - Evergreen",
    )
}

/// Checks the list-level rules shared by every constructor: non-empty, and
/// `Evergreen` alone.
pub(crate) fn policy_shape_diagnostics(entries: &[PolicyEntry]) -> Vec<Diagnostic> {
    if entries.is_empty() {
        return vec![empty_policy()];
    }
    if entries.len() == 1 {
        return Vec::new();
    }
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.rule == Rule::Evergreen)
        .map(|(index, _)| evergreen_combined(index))
        .collect()
}

pub(crate) fn parse_declaration(value: &Value) -> Result<Policy, Invalid> {
    let items = match value {
        Value::Array(items) => items,
        Value::Null => {
            return Err(Invalid::new(vec![Diagnostic::new(
                DiagnosticCode::NullPolicy,
                Location::Policy,
                "the policy has no value; list at least one rule, such as `- ValidFor(6mo)`, \
                 or `- Evergreen` for a document that never expires",
            )]));
        }
        Value::String(text) => {
            return Err(Invalid::new(vec![Diagnostic::new(
                DiagnosticCode::NotAList,
                Location::Policy,
                format!("the policy must be a list; write it as a block list:\n  - {text}"),
            )]));
        }
        other => {
            return Err(Invalid::new(vec![Diagnostic::new(
                DiagnosticCode::WrongType,
                Location::Policy,
                format!("the policy must be a list of rules, found {}", type_name(other)),
            )]));
        }
    };
    if items.is_empty() {
        return Err(Invalid::new(vec![empty_policy()]));
    }
    let mut diagnostics = Vec::new();
    let mut entries = Vec::with_capacity(items.len());
    for index in 0..items.len() {
        if let Some(entry) = parse_entry(items, index, &mut diagnostics) {
            if entry.rule == Rule::Evergreen && items.len() > 1 {
                diagnostics.push(evergreen_combined(index));
            }
            entries.push(entry);
        }
    }
    if diagnostics.is_empty() {
        Ok(Policy::from_valid_entries(entries))
    } else {
        diagnostics.sort_by_key(|diagnostic| location_order(&diagnostic.location));
        Err(Invalid::new(diagnostics))
    }
}

fn location_order(location: &Location) -> usize {
    match location {
        Location::Entry { index } | Location::EntryField { index, .. } => *index,
        Location::Property { entry, .. } => entry.unwrap_or(0),
        Location::Policy | Location::Frontmatter => 0,
    }
}

pub(crate) fn parse_policy_text(text: &str) -> Result<Policy, Invalid> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Invalid::new(vec![empty_policy()]));
    }
    if text.starts_with('[') {
        let value = biscuit_file::Yaml::from_str(text)
            .map_err(|error| error.to_string())
            .and_then(|yaml| Value::deserialize(yaml.value().clone()).map_err(|e| e.to_string()))
            .map_err(|error| {
                Invalid::new(vec![Diagnostic::new(
                    DiagnosticCode::InvalidRuleSyntax,
                    Location::Policy,
                    format!("`{text}` is not a YAML flow list: {error}"),
                )])
            })?;
        return parse_declaration(&value);
    }
    parse_rule(text)
        .map(|rule| Policy::from_valid_entries(vec![PolicyEntry::new(rule)]))
        .map_err(|error| Invalid::new(vec![error.at(Location::Entry { index: 0 })]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn code(text: &str) -> DiagnosticCode {
        parse_rule(text).expect_err(text).code
    }

    #[test]
    fn compact_rules_parse_and_display_canonically() {
        for (text, canonical) in [
            ("Evergreen", "Evergreen"),
            ("TimeSensitive", "TimeSensitive"),
            ("ValidFor(3mo)", "ValidFor(3mo)"),
            ("ValidFor(10d, @last_updated)", "ValidFor(10d, @last_updated)"),
            ("ValidFor(2wk,2026-09-28)", "ValidFor(2wk, 2026-09-28)"),
            ("ValidFor( 1yr , @reviewed-on )", "ValidFor(1yr, @reviewed-on)"),
            ("ValidUntil(2027-01-01)", "ValidUntil(2027-01-01)"),
            ("ValidUntil(@retire_on)", "ValidUntil(@retire_on)"),
        ] {
            let rule = parse_rule(text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
            assert_eq!(rule.to_string(), canonical, "{text}");
            assert_eq!(parse_rule(canonical).unwrap(), rule, "{canonical} round trip");
        }
        assert_eq!(
            parse_rule("ValidFor(3mo, 2026-09-28)").unwrap(),
            Rule::ValidFor {
                duration: Duration::new(3, DurationUnit::Months).unwrap(),
                baseline: Baseline::Inline(date("2026-09-28")),
            }
        );
    }

    #[test]
    fn rule_syntax_errors_have_targeted_codes() {
        use DiagnosticCode::*;
        for (text, expected) in [
            ("", InvalidRuleSyntax),
            (" ValidFor(3mo)", InvalidRuleSyntax),
            ("ValidFor(3mo) x", InvalidRuleSyntax),
            ("ValidFor(3mo))", UnbalancedParentheses),
            ("ValidFor(3mo", UnbalancedParentheses),
            ("2026-09-28)", UnbalancedParentheses),
            ("(3mo)", InvalidRuleSyntax),
            ("ValidFor((3mo))", InvalidRuleSyntax),
            ("Duration(3mo)", UnknownRule),
            ("validfor(3mo)", UnknownRule),
            ("FileChanged(src/a.rs, @fp)", UnknownRule),
            ("Evergreen()", InvalidArguments),
            ("Evergreen(1d)", InvalidArguments),
            ("ValidFor", InvalidArguments),
            ("ValidFor(3mo, @a, @b)", InvalidArguments),
            ("ValidFor(3mo,)", InvalidArguments),
            ("ValidUntil(2027-01-01, @x)", InvalidArguments),
            ("ValidFor(0mo)", InvalidDuration),
            ("ValidFor(03mo)", InvalidDuration),
            ("ValidFor(3)", InvalidDuration),
            ("ValidFor(3 mo)", InvalidDuration),
            ("ValidFor(3months)", InvalidDuration),
            ("ValidFor(3w)", InvalidDuration),
            ("ValidFor(3MO)", InvalidDuration),
            ("ValidFor(-3mo)", InvalidDuration),
            ("ValidFor(99999999999d)", InvalidDuration),
            ("ValidFor(3mo, 2026-02-30)", InvalidDate),
            ("ValidFor(3mo, Sept 28)", InvalidDate),
            ("ValidFor(3mo, 2026-09-28T10:00:00Z)", DatesOnly),
            ("ValidUntil(2026-9-28)", InvalidDate),
            ("ValidFor(3mo, @review.last_checked)", NestedReference),
            ("ValidFor(3mo, @)", InvalidReference),
            ("ValidFor(3mo, @9lives)", InvalidReference),
            ("ValidFor(4000000000yr, 2026-01-01)", DateOutOfRange),
        ] {
            assert_eq!(code(text), expected, "{text}");
        }
    }

    #[test]
    fn unknown_rule_messages_point_at_the_fix() {
        assert!(parse_rule("Duration(3mo)").unwrap_err().message.contains("ValidFor"));
        assert!(
            parse_rule("validfor(3mo)")
                .unwrap_err()
                .message
                .contains("case-sensitive, write `ValidFor`")
        );
    }

    #[test]
    fn date_classification() {
        assert_eq!(classify_date("2026-09-28"), DateText::Date(date("2026-09-28")));
        assert_eq!(classify_date("2024-02-29"), DateText::Date(date("2024-02-29")));
        assert_eq!(classify_date("2026-02-29"), DateText::NotADate);
        assert_eq!(classify_date("2026-09-28T10:00:00Z"), DateText::Timestamp);
        assert_eq!(classify_date("2026-09-28 10:00"), DateText::Timestamp);
        assert_eq!(classify_date("2026-09-28x"), DateText::NotADate);
        assert_eq!(classify_date("+2026-09-28"), DateText::NotADate);
        assert_eq!(classify_date(""), DateText::NotADate);
    }

    #[test]
    fn policy_text_accepts_one_rule_or_a_flow_list() {
        let single = Policy::from_text("TimeSensitive").unwrap();
        assert_eq!(single.entries(), [PolicyEntry::new(Rule::TimeSensitive)]);
        let list = Policy::from_text(
            r#"["ValidFor(3mo)", {rule: "ValidUntil(2027-01-01)", action: archive}]"#,
        )
        .unwrap();
        assert_eq!(list.entries().len(), 2);
        assert_eq!(list.entries()[1].action, Action::Archive);
        assert!(Policy::from_text("").unwrap_err().has(DiagnosticCode::EmptyPolicy));
        assert!(Policy::from_text("[").is_err());
        assert!(Policy::from_text("[]").unwrap_err().has(DiagnosticCode::EmptyPolicy));
        assert!(
            Policy::from_text("[ValidFor(3mo, 2026-09-28)]")
                .unwrap_err()
                .has(DiagnosticCode::UnbalancedParentheses)
        );
    }

    #[test]
    fn policy_new_enforces_shape() {
        assert!(Policy::new(vec![]).unwrap_err().has(DiagnosticCode::EmptyPolicy));
        let combined = Policy::new(vec![
            PolicyEntry::new(Rule::Evergreen),
            PolicyEntry::new(Rule::TimeSensitive),
        ]);
        assert!(combined.unwrap_err().has(DiagnosticCode::EvergreenCombined));
        assert!(Policy::new(vec![PolicyEntry::new(Rule::Evergreen)]).is_ok());
    }
}
