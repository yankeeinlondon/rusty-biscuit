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
///
/// Container literals recurse rather than qualifying outright, for the same
/// reason: `[Bb]` is a regex character class that also parses as an array
/// literal of bare variables, so it must keep falling through to the regex
/// branch. `[tool_name == 'Bash']` does qualify, because an element uses a
/// condition-grade feature. Object keys are authored text, never expressions,
/// so only the values are inspected.
fn expression_uses_known_features(expr: &Expr) -> bool {
    match expr {
        Expr::Variable(_) => false,
        Expr::Paren(inner) => expression_uses_known_features(inner),
        Expr::ArrayLiteral(elements) => elements.iter().any(expression_uses_known_features),
        Expr::ObjectLiteral(entries) => {
            entries.iter().any(|(_, value)| expression_uses_known_features(value))
        }
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
mod tests {
    use super::*;
    use crate::events::*;
    use crate::provider::Provider;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tracing_test::traced_test;

    fn tool_meta(event: AgenticEvent, tool_name: Option<&str>) -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event,
            timestamp: Utc::now(),
            session_id: None,
            cwd: None,
            tool_name: tool_name.map(String::from),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            agent_pid: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn notification_meta(ntype: Option<&str>) -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::Notification,
            timestamp: Utc::now(),
            session_id: None,
            cwd: None,
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: ntype.map(String::from),
            notification_message: None,
            agent_pid: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn tool_meta_with_branch(
        event: AgenticEvent,
        tool_name: Option<&str>,
        branch: Option<&str>,
        is_dirty: bool,
    ) -> EventMeta {
        let mut meta = tool_meta(event, tool_name);
        meta.env.git = Some(GitContext {
            repo_root: PathBuf::from("/tmp/repo"),
            branch: branch.map(String::from),
            is_dirty,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            head_sha: None,
            head_message: None,
            user_name: None,
            user_email: None,
            remote_name: None,
            remote_url: None,
            hosting_provider: None,
            repo_name: None,
            repo_org: None,
        });
        meta
    }

    #[test]
    fn no_matcher_returns_true() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(matches_with_pattern(None, &meta));
    }

    #[test]
    fn regex_matches_tool_name() {
        let meta_bash = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        let meta_edit = tool_meta(AgenticEvent::AfterTool, Some("Edit"));
        let meta_read = tool_meta(AgenticEvent::BeforeTool, Some("Read"));

        assert!(matches_with_pattern(Some("Bash|Edit"), &meta_bash));
        assert!(matches_with_pattern(Some("Bash|Edit"), &meta_edit));
        assert!(!matches_with_pattern(Some("Bash|Edit"), &meta_read));
    }

    #[test]
    fn regex_matches_tool_error() {
        let meta = tool_meta(AgenticEvent::ToolError, Some("Bash"));
        assert!(matches_with_pattern(Some("Bash"), &meta));
    }

    #[test]
    fn regex_matches_notification_type() {
        let meta_match = notification_meta(Some("permission_prompt"));
        let meta_nomatch = notification_meta(Some("info"));

        assert!(matches_with_pattern(
            Some("permission_prompt|ToolPermission"),
            &meta_match
        ));
        assert!(!matches_with_pattern(
            Some("permission_prompt|ToolPermission"),
            &meta_nomatch
        ));
    }

    #[test]
    fn tool_event_with_no_tool_name_returns_false() {
        let meta = tool_meta(AgenticEvent::BeforeTool, None);
        assert!(!matches_with_pattern(Some("Bash"), &meta));
    }

    #[test]
    fn notification_with_no_type_returns_false() {
        let meta = notification_meta(None);
        assert!(!matches_with_pattern(Some("info"), &meta));
    }

    #[test]
    fn regex_character_class_still_compiles_as_a_regex() {
        // `[Bb]ash` and `[abc]` are regex character classes that also parse
        // as array literals of bare variables. Container literals recurse
        // instead of qualifying outright, so these keep their legacy regex
        // semantics; an unconditional `true` arm would hijack them into
        // expression matchers.
        assert!(matches!(
            RuntimeMatcher::compile("[abc]"),
            Some(RuntimeMatcher::Regex(_))
        ));
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("bash"));
        assert!(matches_with_pattern(Some("[Bb]ash"), &meta));
    }

    #[test]
    fn array_literal_with_condition_grade_element_compiles_as_expression() {
        // An element that uses a condition-grade feature makes the whole
        // container literal condition-grade; a no-op `false` arm would send
        // this to the regex branch instead.
        assert!(matches!(
            RuntimeMatcher::compile("[tool_name == 'Bash']"),
            Some(RuntimeMatcher::Expression { .. })
        ));
        assert!(matches!(
            RuntimeMatcher::compile("{ hit: tool_name == 'Bash' }"),
            Some(RuntimeMatcher::Expression { .. })
        ));
    }

    #[test]
    fn invalid_matcher_returns_false() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        // A pattern that is neither a valid expression nor a valid regex
        // compiles to None, which causes matches_with_pattern to return
        // false (binding skipped).
        assert!(!matches_with_pattern(Some("[invalid(regex"), &meta));
    }

    #[test]
    fn non_tool_event_with_regex_matcher_returns_true() {
        let meta = tool_meta(AgenticEvent::SessionStart, None);
        assert!(matches_with_pattern(Some("anything"), &meta));
    }

    #[test]
    fn matches_with_pattern_function() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(matches_with_pattern(Some("Bash"), &meta));
        assert!(!matches_with_pattern(Some("Read"), &meta));
        assert!(matches_with_pattern(None, &meta));
    }

    // -----------------------------------------------------------------
    // Expression-mode matcher tests
    // -----------------------------------------------------------------

    #[test]
    fn expression_matches_tool_and_branch() {
        let meta =
            tool_meta_with_branch(AgenticEvent::BeforeTool, Some("Bash"), Some("main"), false);

        assert!(matches_with_pattern(
            Some("tool_name == 'Bash' && git.branch == 'main'"),
            &meta,
        ));
    }

    #[test]
    fn expression_provider_and_not_dirty() {
        let meta =
            tool_meta_with_branch(AgenticEvent::BeforeTool, Some("Bash"), Some("main"), false);

        assert!(matches_with_pattern(
            Some("provider == 'claude' && !git.is_dirty"),
            &meta,
        ));
    }

    #[test]
    fn expression_fails_when_branch_does_not_match() {
        let meta = tool_meta_with_branch(
            AgenticEvent::BeforeTool,
            Some("Bash"),
            Some("feature/foo"),
            false,
        );

        assert!(!matches_with_pattern(
            Some("tool_name == 'Bash' && git.branch == 'main'"),
            &meta,
        ));
    }

    #[test]
    fn expression_returns_false_for_missing_field() {
        // tool_name is missing but the expression references it.
        let meta = tool_meta(AgenticEvent::SessionStart, None);

        assert!(!matches_with_pattern(Some("tool_name == 'Bash'"), &meta,));
    }

    #[test]
    fn expression_compiles_with_helper_function() {
        let mut meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        meta.tool_input = Some(serde_json::json!({"command": "echo hi"}));

        let matcher =
            RuntimeMatcher::compile("tool_name == 'Bash' && length(tool_input.command) > 0")
                .expect("compile should succeed");
        assert!(matches!(matcher, RuntimeMatcher::Expression { .. }));
        assert!(matches(Some(&matcher), &meta));
    }

    #[test]
    fn compile_prefers_regex_for_bare_word() {
        // `Bash` parses as a bare variable; we prefer regex semantics so
        // legacy `tool_name`-style matchers keep working.
        let matcher =
            RuntimeMatcher::compile("Bash|Edit").expect("compile should succeed for regex");
        assert!(matches!(matcher, RuntimeMatcher::Regex(_)));
    }

    #[test]
    fn compile_returns_none_for_invalid_input() {
        // Neither valid expression nor valid regex.
        assert!(RuntimeMatcher::compile("[invalid(regex").is_none());
    }

    #[test]
    fn compile_returns_none_for_empty_string() {
        assert!(RuntimeMatcher::compile("").is_none());
        assert!(RuntimeMatcher::compile("   ").is_none());
    }

    #[traced_test]
    #[test]
    fn compile_many_aggregates_one_warning_for_invalid_matchers() {
        let bindings = &[
            (AgenticEvent::BeforeTool, "[invalid(regex"),
            (AgenticEvent::AfterTool, "(also[bad"),
        ];
        let results = compile_many(bindings);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(_, m)| m.is_none()));

        logs_assert(|logs| {
            let matching: Vec<_> = logs
                .iter()
                .filter(|l| l.contains("listed bindings will fire unconditionally"))
                .collect();
            assert_eq!(
                matching.len(),
                1,
                "expected exactly one aggregated warning, got: {:?}",
                logs
            );
            let warning = matching[0];
            assert!(warning.contains("before_tool"));
            assert!(warning.contains("after_tool"));
            assert!(warning.contains("[invalid(regex"));
            assert!(warning.contains("(also[bad"));
            Ok(())
        });
    }

    #[traced_test]
    #[test]
    fn compile_many_emits_no_warning_for_valid_matchers() {
        let bindings = &[
            (AgenticEvent::BeforeTool, "Bash|Edit"),
            (AgenticEvent::AfterTool, "tool_name == 'Bash'"),
        ];
        let results = compile_many(bindings);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(_, m)| m.is_some()));

        logs_assert(|logs| {
            let matching: Vec<_> = logs
                .iter()
                .filter(|l| l.contains("listed bindings will fire unconditionally"))
                .collect();
            assert!(
                matching.is_empty(),
                "expected no warnings, got: {:?}",
                logs
            );
            Ok(())
        });
    }

    #[traced_test]
    #[test]
    fn compile_many_skips_empty_matchers_in_warning() {
        let bindings = &[
            (AgenticEvent::BeforeTool, ""),
            (AgenticEvent::AfterTool, "[invalid(regex"),
        ];
        compile_many(bindings);

        logs_assert(|logs| {
            let matching: Vec<_> = logs
                .iter()
                .filter(|l| l.contains("listed bindings will fire unconditionally"))
                .collect();
            assert_eq!(matching.len(), 1, "expected one warning, got: {:?}", logs);
            let warning = matching[0];
            assert!(!warning.contains("before_tool"));
            assert!(warning.contains("after_tool"));
            Ok(())
        });
    }
}
