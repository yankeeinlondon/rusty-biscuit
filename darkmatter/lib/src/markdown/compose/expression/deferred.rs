//! Recognizing a value whose content is supplied later by a `{{ … }}`
//! template, the one rule passive validation and lifecycle playback share.
//!
//! A deferred value is text in which the existing template scanner
//! ([`ExpressionFinder::find_all_plain`]) finds at least one span and every
//! span parses. Nothing is evaluated. An unclosed `{{` is not a span under the
//! scanner, so such text is an ordinary literal; a span that fails to parse is
//! a malformed template, which is never deferred and is judged as written.

use serde_json::Value;

use super::lexer::ExpressionFinder;
use super::parser::parse;
use crate::markdown::coercion::PathSegment;

/// Whether `text` is a template whose value exists only once it is rendered.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::is_deferred_template;
///
/// assert!(is_deferred_template("{{ fm.count }}"));
/// assert!(is_deferred_template("n={{ fm.count }}"));
/// assert!(!is_deferred_template("{{ 1 + }}"));
/// assert!(!is_deferred_template("{{ fm.count"));
/// assert!(!is_deferred_template("3"));
/// ```
pub fn is_deferred_template(text: &str) -> bool {
    let spans = ExpressionFinder::find_all_plain(text);
    !spans.is_empty() && spans.iter().all(|span| parse(&span.expression).is_ok())
}

/// Whether `text` holds a `{{ … }}` span that fails to parse: a malformed
/// template, which no rendering can settle. A caller that sets a template's
/// problem aside for a later rendering pass judges this one as written.
///
/// ```
/// use darkmatter::markdown::compose::expression::is_malformed_template;
///
/// assert!(is_malformed_template("{{ 1 + }}"));
/// assert!(is_malformed_template("{{ fm.count }} and {{ 1 + }}"));
/// assert!(!is_malformed_template("{{ fm.count }}"));
/// assert!(!is_malformed_template("{{ fm.count"));
/// ```
pub fn is_malformed_template(text: &str) -> bool {
    ExpressionFinder::find_all_plain(text).iter().any(|span| parse(&span.expression).is_err())
}

/// [`is_deferred_template`] for a JSON value: only a string can be deferred.
pub fn is_deferred_value(value: &Value) -> bool {
    value.as_str().is_some_and(is_deferred_template)
}

/// The deferral predicate for [`crate::markdown::coercion::BindOptions`]:
/// a deferred value at any path is treated as matching the shape declared
/// there, while its literal siblings are still bound.
pub fn defer_templates(_path: &[PathSegment], value: &Value) -> bool {
    is_deferred_value(value)
}
