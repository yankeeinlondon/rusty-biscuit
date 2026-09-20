//! Deterministic replay of sanitized fixtures.
//!
//! Error signatures are explicit conjunctions over envelope locators; there
//! is no substring or regex matching. A response that no known signature
//! matches stays unknown, and two unrelated matching signatures are
//! ambiguous rather than decided by array order.

use serde_json::Value;

use crate::research::model::{
    AnswerType, BodyFormat, Envelope, ErrorFixture, ErrorOutcome, ErrorRecord, MatchSignature,
    QuestionBinding, QuestionKind, State,
};

/// A sanitized response as a fixture describes it.
#[derive(Debug, Clone, Copy)]
pub struct Response<'a> {
    pub http_status: Option<u16>,
    /// `Name: value` lines.
    pub headers: &'a [String],
    pub body: Option<&'a str>,
    pub sdk_variant: Option<&'a str>,
}

impl<'a> From<&'a ErrorFixture> for Response<'a> {
    fn from(fixture: &'a ErrorFixture) -> Self {
        Self {
            http_status: fixture.http_status,
            headers: &fixture.headers,
            body: fixture.body.as_deref(),
            sdk_variant: fixture.sdk_variant.as_deref(),
        }
    }
}

/// How a response classifies against an envelope's executable signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    /// Exactly one most-specific signature matched.
    Matched(String),
    /// No executable signature matched; the classification stays unknown.
    Unknown,
    /// Several unrelated signatures matched (an SR-MATCH-OVERLAP defect).
    Ambiguous(Vec<String>),
}

/// Executable errors: known, with a `match` signature.
pub fn executable(error: &ErrorRecord) -> Option<&MatchSignature> {
    (error.knowledge.state == State::Known).then_some(error.signature.as_ref()).flatten()
}

/// Classifies `response` for `operation` against the executable signatures
/// of `errors` that use `envelope`.
pub fn classify(envelope: &Envelope, errors: &[ErrorRecord], operation: &str, response: Response<'_>) -> Classification {
    let parsed = parse_body(envelope.body_format, response.body);
    let matching: Vec<(&ErrorRecord, &MatchSignature)> = errors
        .iter()
        .filter(|error| error.envelope == envelope.id && error.operations.iter().any(|op| op == operation))
        .filter_map(|error| executable(error).map(|signature| (error, signature)))
        .filter(|(error, signature)| matches(envelope, error, signature, response, parsed.as_ref()))
        .collect();
    let most_specific: Vec<&str> = matching
        .iter()
        .filter(|(_, signature)| {
            !matching
                .iter()
                .any(|(_, other)| strictly_more_specific(other, signature))
        })
        .map(|(error, _)| error.id.as_str())
        .collect();
    match most_specific.as_slice() {
        [] => Classification::Unknown,
        [only] => Classification::Matched((*only).to_string()),
        many => Classification::Ambiguous(many.iter().map(|id| id.to_string()).collect()),
    }
}

/// `specific` holds every predicate of `general` plus at least one more.
pub fn strictly_more_specific(specific: &MatchSignature, general: &MatchSignature) -> bool {
    let specific = specific.predicates();
    let general = general.predicates();
    specific.len() > general.len() && general.iter().all(|predicate| specific.contains(predicate))
}

/// Two signatures could match one response: no shared predicate disagrees.
pub fn compatible(left: &MatchSignature, right: &MatchSignature) -> bool {
    let right = right.predicates();
    left.predicates().iter().all(|(name, value)| {
        right
            .iter()
            .filter(|(other, _)| other == name)
            .all(|(_, other_value)| other_value == value)
    })
}

fn parse_body(format: BodyFormat, body: Option<&str>) -> Option<Value> {
    match format {
        BodyFormat::Json | BodyFormat::JsonRpc => serde_json::from_str(body?).ok(),
        _ => None,
    }
}

fn matches(
    envelope: &Envelope,
    error: &ErrorRecord,
    signature: &MatchSignature,
    response: Response<'_>,
    body: Option<&Value>,
) -> bool {
    let read = |locator: &str| read_locator(locator, response, body);
    let any_equals = |locator: Option<&str>, expected: &str| {
        locator.is_some_and(|locator| read(locator).iter().any(|value| value == expected))
    };

    if let Some(status) = signature.http_status
        && response.http_status != Some(status)
    {
        return false;
    }
    let code = |expected: &str| {
        any_equals(envelope.code_locator.as_deref(), expected)
            || (error.outcome == ErrorOutcome::Warning && any_equals(envelope.warnings_locator.as_deref(), expected))
    };
    if let Some(number) = signature.native_code_number
        && !code(&number.to_string())
    {
        return false;
    }
    if let Some(text) = &signature.native_code_string
        && !code(text)
    {
        return false;
    }
    if let Some(subcode) = &signature.native_subcode
        && !any_equals(envelope.subcode_locator.as_deref(), subcode)
    {
        return false;
    }
    if let Some(locator) = &signature.discriminator_locator {
        let values = read(locator);
        let satisfied = match &signature.discriminator_equals {
            Some(expected) => values.iter().any(|value| value == expected),
            None => !values.is_empty(),
        };
        if !satisfied {
            return false;
        }
    }
    if let Some(token) = &signature.exact_text_token {
        let locator = match envelope.body_format {
            BodyFormat::PlainText => Some("body_text"),
            _ => envelope.message_locator.as_deref(),
        };
        if !any_equals(locator, token) {
            return false;
        }
    }
    if let Some(variant) = &signature.sdk_error_variant
        && response.sdk_variant != Some(variant.as_str())
    {
        return false;
    }
    true
}

/// Scalar texts at a locator. Arrays contribute each scalar element.
pub(crate) fn read_locator(locator: &str, response: Response<'_>, body: Option<&Value>) -> Vec<String> {
    if locator == "http_status" {
        return response.http_status.map(|status| vec![status.to_string()]).unwrap_or_default();
    }
    if locator == "body_text" {
        return response.body.map(|body| vec![body.trim().to_string()]).unwrap_or_default();
    }
    if let Some(name) = locator.strip_prefix("header:") {
        return response
            .headers
            .iter()
            .filter_map(|line| line.split_once(':'))
            .filter(|(header, _)| header.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim().to_string())
            .collect();
    }
    if let Some(path) = locator.strip_prefix("sdk_variant:") {
        return response
            .sdk_variant
            .filter(|variant| *variant == path)
            .map(|variant| vec![variant.to_string()])
            .unwrap_or_default();
    }
    match body.and_then(|body| body.pointer(locator)) {
        Some(Value::Array(items)) => items.iter().filter_map(scalar_text).collect(),
        Some(value) => scalar_text(value).into_iter().collect(),
        None => Vec::new(),
    }
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The outcome of replaying an answer payload.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionReplay {
    /// The canonical answer (JSON boolean, string, or string array).
    Answer(Value),
    /// The payload has no answer at the binding's locator.
    NoAnswer,
    /// The binding returns no answer or declares no locator.
    NotReplayable,
}

/// Extracts the canonical answer a question binding would read from a
/// sanitized payload.
pub fn replay_interaction(question: &QuestionBinding, payload: &str) -> InteractionReplay {
    let Some(locator) = question.answer_locator.as_deref() else {
        return InteractionReplay::NotReplayable;
    };
    if question.answer_type == AnswerType::None {
        return InteractionReplay::NotReplayable;
    }
    let Ok(payload) = serde_json::from_str::<Value>(payload) else {
        return InteractionReplay::NoAnswer;
    };
    let Some(value) = payload.pointer(locator) else {
        return InteractionReplay::NoAnswer;
    };
    let answer = match (question.kind, question.answer_type, value) {
        (QuestionKind::Confirmation, AnswerType::Boolean, Value::Bool(flag)) => Some(Value::Bool(*flag)),
        (QuestionKind::Confirmation, AnswerType::Boolean, Value::String(text)) => question
            .confirmation_mapping
            .as_ref()
            .and_then(|mapping| {
                if *text == mapping.affirmative {
                    Some(true)
                } else if *text == mapping.negative {
                    Some(false)
                } else {
                    None
                }
            })
            .map(Value::Bool),
        (_, AnswerType::OptionIdArray, Value::Array(items)) => items
            .iter()
            .map(scalar_text)
            .collect::<Option<Vec<_>>>()
            .map(|ids| Value::Array(ids.into_iter().map(Value::String).collect())),
        (_, AnswerType::OptionId | AnswerType::String, value) => scalar_text(value).map(Value::String),
        _ => None,
    };
    answer.map_or(InteractionReplay::NoAnswer, InteractionReplay::Answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signature(status: Option<u16>, code: Option<&str>) -> MatchSignature {
        MatchSignature {
            http_status: status,
            native_code_string: code.map(str::to_string),
            ..MatchSignature::default()
        }
    }

    #[test]
    fn a_signature_with_an_added_predicate_is_strictly_more_specific() {
        let general = signature(Some(400), None);
        let specific = signature(Some(400), Some("bad"));
        assert!(strictly_more_specific(&specific, &general));
        assert!(!strictly_more_specific(&general, &specific));
        assert!(!strictly_more_specific(&general, &general), "equal signatures are not ordered");
    }

    #[test]
    fn signatures_with_disagreeing_predicates_are_incompatible() {
        assert!(!compatible(&signature(Some(400), Some("a")), &signature(Some(400), Some("b"))));
        assert!(compatible(&signature(Some(400), None), &signature(None, Some("b"))));
    }

    #[test]
    fn locators_read_headers_status_plain_text_and_json_arrays() {
        let headers = vec!["Retry-After: 3".to_string()];
        let response = Response {
            http_status: Some(429),
            headers: &headers,
            body: Some(" invalid_token \n"),
            sdk_variant: None,
        };
        assert_eq!(read_locator("header:retry-after", response, None), vec!["3"]);
        assert_eq!(read_locator("http_status", response, None), vec!["429"]);
        assert_eq!(read_locator("body_text", response, None), vec!["invalid_token"]);
        let body: Value = serde_json::json!({"ok": false, "warnings": ["a", 2]});
        assert_eq!(read_locator("/ok", response, Some(&body)), vec!["false"]);
        assert_eq!(read_locator("/warnings", response, Some(&body)), vec!["a", "2"]);
        assert!(read_locator("/missing", response, Some(&body)).is_empty());
    }
}
