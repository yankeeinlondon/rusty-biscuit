//! Event binding matchers.
//!
//! A matcher decides whether the actions configured for an event should
//! actually run for a given [`EventMeta`]. Two matcher modes are supported:
//!
//! - [`RuntimeMatcher::Regex`] — the historical behavior where a regex is
//!   tested against `tool_name` (for tool events) or `notification_type`
//!   (for `Notification` events). All other events with a regex matcher
//!   pass through unconditionally so that bindings without a meaningful
//!   matchable field continue to fire.
//! - [`RuntimeMatcher::Expression`] — a Darkmatter condition expression
//!   evaluated against the full [`EventMeta`] via
//!   [`EventMetaExpressionLookup`]. Expressions can combine multiple fields
//!   (`tool_name == 'Bash' && git.branch == 'main'`) and use the same
//!   operators and helper functions as dispatch templates and hook `when`.
//!
//! Compilation policy at config-load time: a matcher string is first
//! attempted as a Darkmatter condition; if that fails, it is compiled as a
//! regex; if both fail it is dropped with a warning so the binding fires
//! unconditionally rather than silently disappearing.

use darkmatter::markdown::compose::expression::{Expr, ParseMode, Parser, evaluate, is_truthy};
use regex::Regex;
use tracing::warn;

use crate::dispatch::expression::EventMetaExpressionLookup;
use crate::events::{AgenticEvent, EventMeta};

/// Compiled matcher attached to a runtime event binding.
#[derive(Debug, Clone)]
pub enum RuntimeMatcher {
    /// Legacy single-field regex matcher.
    Regex(Regex),
    /// Darkmatter condition expression evaluated against full [`EventMeta`].
    Expression {
        /// Original source string, retained for diagnostics.
        source: String,
        /// Pre-parsed expression AST.
        expr: Expr,
    },
}

impl RuntimeMatcher {
    /// Compile a raw matcher string from configuration.
    ///
    /// The string is parsed as a Darkmatter condition first; on parse
    /// failure it is compiled as a regex. If neither succeeds, returns
    /// `None` silently. At config-load time callers should use
    /// [`compile_many`] so invalid matchers are reported in one
    /// aggregated warning rather than one log line per binding.
    ///
    /// ## Notes
    ///
    /// The end-to-end production behaviour for invalid input is pinned by
    /// `dispatch::loader::tests::invalid_matcher_in_config_compiles_to_unconditional_binding`.
    /// Note that the test-only helper `matches_with_pattern` returns
    /// `false` for inputs that compile to `None`, which is the *opposite*
    /// of how [`matches()`] treats `None` matchers at runtime. Do not
    /// "align" the helper with the runtime function without first
    /// updating the dispatch sites that depend on the current contract.
    pub fn compile(source: &str) -> Option<Self> {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Ok(mut parser) = Parser::with_mode(trimmed, ParseMode::Condition)
            && let Ok(expr) = parser.parse()
            && expression_uses_known_features(&expr)
        {
            return Some(Self::Expression {
                source: trimmed.to_string(),
                expr,
            });
        }
        // A bare variable like `Bash` would parse but is intentionally
        // treated as a regex below so legacy `tool_name`-style matchers
        // keep their original semantics.

        Regex::new(trimmed).ok().map(Self::Regex)
    }
}

/// Compile matchers for multiple event bindings at load time.
///
/// Invalid matchers compile to `None` so the binding fires
/// unconditionally. A single aggregated warning names every binding
/// whose matcher failed, replacing the previous per-binding warnings.
pub fn compile_many(bindings: &[(AgenticEvent, &str)]) -> Vec<(AgenticEvent, Option<RuntimeMatcher>)> {
    let mut results = Vec::with_capacity(bindings.len());
    let mut failed = Vec::new();

    for (event, source) in bindings {
        let trimmed = source.trim();
        let compiled = RuntimeMatcher::compile(source);
        if compiled.is_none() && !trimmed.is_empty() {
            failed.push(format!("{} ({})", event, trimmed));
        }
        results.push((*event, compiled));
    }

    if !failed.is_empty() {
        warn!(
            bindings = %failed.join(", "),
            "Matchers are neither valid Darkmatter conditions nor valid regexes; listed bindings will fire unconditionally"
        );
    }

    results
}

/// Whether the parsed expression uses condition-grade features (operators,
/// comparisons, fallbacks, helper functions, literals). A bare variable
/// reference such as `Bash` deliberately does not qualify so that simple
/// regex-style matchers continue to compile as regexes.
fn expression_uses_known_features(expr: &Expr) -> bool {
    match expr {
        Expr::Variable(_) => false,
        Expr::Paren(inner) => expression_uses_known_features(inner),
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => true,
        Expr::UnaryNot(_)
        | Expr::UnaryMinus(_)
        | Expr::Binary { .. }
        | Expr::Index { .. }
        | Expr::MemberAccess { .. }
        | Expr::Fallback { .. }
        | Expr::Ternary { .. }
        | Expr::Comparison { .. }
        | Expr::FunctionCall { .. } => true,
    }
}

/// Evaluate a runtime matcher (or absence thereof) against an event.
///
/// Returns `true` when the matcher is absent, when a regex matcher matches
/// the event's matchable field, or when an expression matcher evaluates to
/// a truthy value. An expression that fails to evaluate is logged and
/// treated as a non-match so a broken matcher cannot silently allow
/// arbitrary actions through.
pub fn matches(matcher: Option<&RuntimeMatcher>, meta: &EventMeta) -> bool {
    let Some(matcher) = matcher else {
        return true;
    };

    match matcher {
        RuntimeMatcher::Regex(regex) => regex_matches(regex, meta),
        RuntimeMatcher::Expression { source, expr } => {
            let lookup = EventMetaExpressionLookup::new(meta);
            match evaluate(expr, &lookup) {
                Ok(value) => is_truthy(&value),
                Err(error) => {
                    warn!(
                        matcher = %source,
                        %error,
                        "Failed to evaluate expression matcher; treating as non-match"
                    );
                    false
                }
            }
        }
    }
}

fn regex_matches(matcher: &Regex, meta: &EventMeta) -> bool {
    match meta.event {
        AgenticEvent::BeforeTool | AgenticEvent::AfterTool | AgenticEvent::ToolError => {
            match &meta.tool_name {
                Some(name) => matcher.is_match(name),
                None => false,
            }
        }
        AgenticEvent::Notification => match &meta.notification_type {
            Some(ntype) => matcher.is_match(ntype),
            None => false,
        },
        _ => true,
    }
}

/// Check if an event matches using an explicit pattern string.
///
/// Convenience wrapper that re-runs the load-time compilation policy for a
/// raw matcher string. Prefer [`matches`] in runtime code so compilation is
/// performed once at config-load time.
#[cfg(test)]
pub fn matches_with_pattern(pattern: Option<&str>, meta: &EventMeta) -> bool {
    let Some(pattern) = pattern else {
        return true;
    };

    match RuntimeMatcher::compile(pattern) {
        Some(matcher) => matches(Some(&matcher), meta),
        None => false,
    }
}

#[cfg(test)]
mod tests;
