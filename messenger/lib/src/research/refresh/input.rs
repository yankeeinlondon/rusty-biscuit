//! Operator-supplied decision inputs: who decided, and why.
//!
//! A decision names an accountable maintainer, so neither value may be blank.
//! Both types trim surrounding whitespace and reject what is left empty, and
//! both deserialize through the same constructor: a persisted run record or
//! review record holding an invalid value fails to load rather than standing
//! as a decision nobody made.

use serde::{Deserialize, Serialize};

/// An invalid decision input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    #[error("the maintainer name is empty; name the person accountable for this decision")]
    BlankMaintainer,
    #[error("the maintainer name contains a control character such as a newline or tab; give a single-line name")]
    ControlInMaintainer,
    #[error("the decision reason is empty; say why")]
    BlankReason,
}

/// The maintainer accountable for a decision: non-blank, trimmed, and free of
/// control characters, because the CHANGELOG renders it inside a single-line
/// Markdown list item where a newline would inject content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Maintainer(String);

impl Maintainer {
    /// ## Errors
    ///
    /// [`InputError::BlankMaintainer`] when `name` is empty or only whitespace;
    /// [`InputError::ControlInMaintainer`] when the trimmed name holds a
    /// control character.
    pub fn new(name: &str) -> Result<Self, InputError> {
        let name = non_blank(name).ok_or(InputError::BlankMaintainer)?;
        if name.chars().any(char::is_control) {
            return Err(InputError::ControlInMaintainer);
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a decision was made: non-blank and trimmed. Interior newlines are
/// allowed: the reason is persisted only as JSON in the local run record and
/// is never interpolated into single-line Markdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DecisionReason(String);

impl DecisionReason {
    /// ## Errors
    ///
    /// [`InputError::BlankReason`] when `text` is empty or only whitespace.
    pub fn new(text: &str) -> Result<Self, InputError> {
        non_blank(text).map(Self).ok_or(InputError::BlankReason)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn non_blank(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

macro_rules! string_conversions {
    ($type:ty) => {
        impl TryFrom<String> for $type {
            type Error = InputError;
            fn try_from(text: String) -> Result<Self, InputError> {
                Self::new(&text)
            }
        }

        impl From<$type> for String {
            fn from(value: $type) -> String {
                value.0
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_conversions!(Maintainer);
string_conversions!(DecisionReason);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_inputs_are_refused_and_valid_ones_are_trimmed() {
        for blank in ["", "   ", "\t\n "] {
            assert_eq!(Maintainer::new(blank), Err(InputError::BlankMaintainer), "{blank:?}");
            assert_eq!(DecisionReason::new(blank), Err(InputError::BlankReason), "{blank:?}");
        }
        assert_eq!(Maintainer::new("  Ken Snyder \n").expect("valid").as_str(), "Ken Snyder");
        assert_eq!(DecisionReason::new(" wrong agent ").expect("valid").as_str(), "wrong agent");
    }

    #[test]
    fn a_maintainer_name_holds_no_control_character_but_a_reason_may_span_lines() {
        for name in ["Ken\n- approved by mallory", "Ken\tSnyder", "Ken\rSnyder", "Ken\u{1b}[31mSnyder", "Ken\u{85}Snyder"] {
            assert_eq!(Maintainer::new(name), Err(InputError::ControlInMaintainer), "{name:?}");
        }
        assert_eq!(Maintainer::new("\tKen Snyder\n").expect("surrounding control characters are trimmed").as_str(), "Ken Snyder");
        assert_eq!(DecisionReason::new("duplicate run\nsee run 2").expect("multi-line").as_str(), "duplicate run\nsee run 2");
        let error = serde_json::from_str::<Maintainer>(r#""Ken\n- approved by mallory""#).expect_err("control character");
        assert!(error.to_string().contains("control character"), "{error}");
    }

    #[test]
    fn deserialization_applies_the_same_validation() {
        assert!(serde_json::from_str::<Maintainer>(r#""  ""#).expect_err("blank").to_string().contains("maintainer name is empty"));
        assert!(serde_json::from_str::<DecisionReason>(r#""""#).expect_err("blank").to_string().contains("decision reason is empty"));
        let maintainer: Maintainer = serde_json::from_str(r#"" Ken ""#).expect("valid");
        assert_eq!(serde_json::to_string(&maintainer).expect("serialize"), r#""Ken""#);
    }
}
