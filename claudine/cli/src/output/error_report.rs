use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::StatusBlock;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use claudine::provider::Provider;
use claudine::stream::semantic::SemanticErrorKind;

use crate::log;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentErrorCategory {
    Configuration,
    AgentNative,
    ApiRemote,
    Interrupted,
}

impl From<SemanticErrorKind> for AgentErrorCategory {
    fn from(kind: SemanticErrorKind) -> Self {
        match kind {
            SemanticErrorKind::Configuration => AgentErrorCategory::Configuration,
            SemanticErrorKind::AgentNative => AgentErrorCategory::AgentNative,
            SemanticErrorKind::ApiRemote => AgentErrorCategory::ApiRemote,
            SemanticErrorKind::Interrupted => AgentErrorCategory::Interrupted,
            // Unknown maps to AgentNative since it has the broadest "something
            // went wrong with the agent" framing without overclaiming a
            // specific upstream cause.
            SemanticErrorKind::Unknown => AgentErrorCategory::AgentNative,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SuggestionStyle {
    BareList,
    DidYouMean,
}

/// Typed cause distilled from a provider's native (non-stream) process exit —
/// `exit_code` plus the captured stdout/stderr tails.
///
/// This is deliberately **separate** from the structured-stream
/// `stream/providers/vocabulary.rs` classification: that table classifies
/// semantic stream error *events*, whereas these are process-level argv/exit
/// rejections that only the wrapper observes. The two must not be conflated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeCliCause {
    /// The provider rejected its argv during argument parsing (unknown flag,
    /// unexpected/invalid argument). Carries the offending flag when one could
    /// be extracted from the diagnostic.
    ArgumentRejected { flag: Option<String> },
    /// A required argument was missing from the command.
    MissingArgument,
    /// The provider could not resolve the requested model.
    ModelNotFound { suggestions: Option<Vec<String>> },
    /// An authentication or permission failure.
    AuthOrPermission,
    /// A required file or command was not found (typically exit 127).
    FileNotFound,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentErrorReport {
    pub(crate) provider: Provider,
    pub(crate) exit_code: i32,
    pub(crate) category: AgentErrorCategory,
    pub(crate) summary: String,
    pub(crate) body_list: Option<Vec<String>>,
    pub(crate) footer: Option<String>,
    pub(crate) detail: Option<String>,
    pub(crate) hint: Option<String>,
    pub(crate) suggestions: Option<Vec<String>>,
    pub(crate) suggestion_style: SuggestionStyle,
    #[allow(dead_code)]
    pub(crate) location: Option<String>,
}

impl AgentErrorReport {
    #[allow(dead_code)]
    pub(crate) fn from_exit_code(provider: Provider, exit_code: i32, stderr: Option<&str>) -> Self {
        Self::from_exit_code_with_source(provider, exit_code, stderr, None)
    }

    pub(crate) fn from_exit_code_with_source(
        provider: Provider,
        exit_code: i32,
        stderr: Option<&str>,
        model_source: Option<&crate::commands::wrap::profile::OpenCodeModelSource>,
    ) -> Self {
        let (category, summary, detail, hint, suggestions, location) =
            classify_exit(provider, exit_code, stderr, model_source);
        Self {
            provider,
            exit_code,
            category,
            summary,
            body_list: None,
            footer: None,
            detail,
            hint,
            suggestions,
            suggestion_style: SuggestionStyle::DidYouMean,
            location,
        }
    }

    /// Build a report for a provider that exited non-zero after Claudine
    /// forwarded a provider-argument tail, correlating the failure with that
    /// tail **only** when the native classifier attributes it to argument
    /// rejection. For every other cause (auth, missing binary, model, or an
    /// unclassified exit) this defers to [`Self::from_exit_code_with_source`]
    /// so a stronger classification is never overwritten and an unrelated
    /// failure is never misattributed to the forwarded arguments.
    ///
    /// `forwarded_switch_names` must already be redacted and reduced to switch
    /// names (no values); `explicit` selects opaque-tail wording. `stderr` is
    /// the provider's captured diagnostic.
    // Wired into the composition failure path once the harness loop surfaces
    // the terminal attempt's stderr tail (the composition-correlation
    // follow-up); the wrapper path already renders via the shared typed
    // classifier below.
    #[allow(dead_code)]
    pub(crate) fn correlated_with_forwarded_tail(
        provider: Provider,
        exit_code: i32,
        stderr: Option<&str>,
        model_source: Option<&crate::commands::wrap::profile::OpenCodeModelSource>,
        forwarded_switch_names: &[String],
        explicit: bool,
    ) -> Self {
        let stderr_text = stderr.unwrap_or("");
        let is_arg_rejection = matches!(
            classify_native_cli_cause(exit_code, stderr_text),
            Some(NativeCliCause::ArgumentRejected { .. })
        );
        if exit_code == 0 || forwarded_switch_names.is_empty() || !is_arg_rejection {
            return Self::from_exit_code_with_source(provider, exit_code, stderr, model_source);
        }

        let provider_name = crate::output::capitalize_provider(provider);
        let tail_desc = if explicit {
            "the opaque argument tail forwarded after `--`".to_string()
        } else {
            format!("the forwarded argument(s): {}", forwarded_switch_names.join(" "))
        };
        Self {
            provider,
            exit_code,
            category: AgentErrorCategory::AgentNative,
            summary: format!(
                "{provider_name} rejected its arguments during startup — this was \
                 likely caused by {tail_desc}, which Claudine forwarded to {provider_name} \
                 without recognizing."
            ),
            body_list: None,
            footer: None,
            detail: Some(stderr_text.lines().next().unwrap_or("").to_string()),
            hint: Some(
                "If this is a valid provider switch Claudine doesn't yet know about, \
                 the run still forwarded it; check the provider's own usage above."
                    .to_string(),
            ),
            suggestions: None,
            suggestion_style: SuggestionStyle::DidYouMean,
            location: None,
        }
    }

    pub(crate) fn no_model_provided(provider: Provider) -> Self {
        let provider_name = crate::output::capitalize_provider(provider);
        Self {
            provider,
            exit_code: 1,
            category: AgentErrorCategory::Configuration,
            summary: format!(
                "No model specified! {provider_name} by default does not specify a model but you can\n\
                 change this behavior by adding a <yellow>model</yellow> property to the <blue>~/.config/opencode/config.json</blue> file.\n\
                 You can override/set the default model with any of the following methods:"
            ),
            body_list: Some(vec![
                "set <yellow>OPENCODE_MODEL</yellow> to a valid model name".to_string(),
                "use the CLI switch <yellow>--model <model></yellow>".to_string(),
            ]),
            footer: Some(
                "Running <yellow>opencode models</yellow> will give you a list of all valid models.\n\
                 Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim> for direct providers\n\
                 like Google or Anthropic but take the form <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim>\n\
                 for aggregators like OpenRouter."
                    .to_string(),
            ),
            detail: None,
            hint: None,
            suggestions: None,
            suggestion_style: SuggestionStyle::BareList,
            location: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invalid_model(
        provider: Provider,
        exit_code: i32,
        location: String,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            provider,
            exit_code,
            category: AgentErrorCategory::AgentNative,
            summary: format!(
                "Invalid model specified in {location}! Running <yellow>opencode models</yellow> will give you\n\
                 a list of all valid models. Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim>\n\
                 for direct providers like Google or Anthropic but take the form\n\
                 <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim> for aggregators like OpenRouter."
            ),
            body_list: None,
            footer: None,
            detail: None,
            hint: None,
            suggestions: Some(suggestions),
            suggestion_style: SuggestionStyle::DidYouMean,
            location: Some(location),
        }
    }

    pub(crate) fn render(&self, term: &Terminal) {
        let border_color = match self.category {
            AgentErrorCategory::Configuration => Color::Tailwind(Tailwind::Orange700),
            AgentErrorCategory::AgentNative => Color::Tailwind(Tailwind::Red700),
            AgentErrorCategory::ApiRemote => Color::Tailwind(Tailwind::Red700),
            AgentErrorCategory::Interrupted => Color::Tailwind(Tailwind::Yellow700),
        };

        let label = match self.category {
            AgentErrorCategory::Configuration => "Configuration Error",
            AgentErrorCategory::AgentNative => "Agent Error",
            AgentErrorCategory::ApiRemote => "API Error",
            AgentErrorCategory::Interrupted => "Interrupted",
        };

        let provider_name = crate::output::capitalize_provider(self.provider);

        let mut compose = Compose::default();

        compose.add_prose(Prose::new(format!(
            "<red><bold>{label}</bold></red> <dim>({provider_name}, exit {})</dim>\n{}",
            self.exit_code, self.summary,
        )));

        if let Some(ref items) = self.body_list {
            compose.add_unordered_list(UnorderedList::new(items.clone()));
        }

        if let Some(ref footer) = self.footer {
            compose.add_prose(Prose::new(format!("\n{footer}")));
        }

        if let Some(ref detail) = self.detail {
            compose.add_prose(Prose::new(format!("\n<dim>{detail}</dim>")));
        }

        if let Some(ref hint) = self.hint {
            compose.add_prose(Prose::new(format!("\n<blue>{hint}</blue>")));
        }

        if let Some(ref suggestions) = self.suggestions
            && !suggestions.is_empty()
        {
            let suggestion_items: Vec<String> = suggestions
                .iter()
                .map(|s| format!("<yellow>{s}</yellow>"))
                .collect();
            let list = UnorderedList::new(suggestion_items);
            match self.suggestion_style {
                SuggestionStyle::DidYouMean => {
                    let header =
                        Status::from_prose("Did you mean:".to_string()).state(StatusState::Warning);
                    compose.add_text("\n");
                    compose.add_prose(Prose::new(header.render(term)));
                    compose.add_unordered_list(list);
                }
                SuggestionStyle::BareList => {
                    compose.add_text("\n");
                    compose.add_unordered_list(list);
                }
            }
        }

        let block = StatusBlock::new(StatusState::Error)
            .body(Prose::new(compose.render(term)))
            .border_color(border_color)
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(2)));

        log::message("");
        log::message(&block.render(term));
        log::message("");
    }
}

#[allow(clippy::type_complexity)]
fn classify_exit(
    provider: Provider,
    exit_code: i32,
    stderr: Option<&str>,
    model_source: Option<&crate::commands::wrap::profile::OpenCodeModelSource>,
) -> (
    AgentErrorCategory,
    String,
    Option<String>,
    Option<String>,
    Option<Vec<String>>,
    Option<String>,
) {
    let stderr_text = stderr.unwrap_or("");
    let provider_name = crate::output::capitalize_provider(provider);

    if exit_code == 130 || exit_code == 143 {
        return (
            AgentErrorCategory::Interrupted,
            format!("{provider_name} was interrupted by the user"),
            None,
            None,
            None,
            None,
        );
    }

    if let Some(cause) = classify_native_cli_cause(exit_code, stderr_text) {
        return native_cause_report(&cause, provider, stderr_text, model_source);
    }

    if stderr_text.contains("API Error:") {
        let message = extract_first_api_error_message(stderr_text);
        return (
            AgentErrorCategory::ApiRemote,
            message,
            Some("This error came from the provider's API layer.".to_string()),
            Some("Check API key, rate limits, and service status.".to_string()),
            None,
            None,
        );
    }

    (
        AgentErrorCategory::AgentNative,
        format!("{provider_name} exited with error code {exit_code}"),
        stderr_text.lines().next().map(|l| l.to_string()),
        None,
        None,
        None,
    )
}

/// Classify a provider's native process exit into a typed [`NativeCliCause`].
///
/// Pure and side-effect free so it can be unit-tested against positive and
/// collision fixtures. Signatures are kept deliberately narrow; an uncertain
/// exit returns `None` and falls through to the generic provider-error report
/// rather than risk a misattribution.
pub(crate) fn classify_native_cli_cause(exit_code: i32, stderr: &str) -> Option<NativeCliCause> {
    let lower = stderr.to_lowercase();

    if lower.contains("providermodelnotfounderror")
        || lower.contains("model not found")
        || lower.contains("invalid model")
    {
        return Some(NativeCliCause::ModelNotFound {
            suggestions: parse_model_suggestions(stderr),
        });
    }

    if lower.contains("unrecognized argument")
        || lower.contains("unknown flag")
        || lower.contains("unknown option")
        || lower.contains("unexpected argument")
        || lower.contains("invalid argument")
    {
        return Some(NativeCliCause::ArgumentRejected {
            flag: extract_unknown_flag(stderr),
        });
    }

    if lower.contains("missing required argument")
        || lower.contains("required argument")
        || lower.contains("the following required arguments were not provided")
    {
        return Some(NativeCliCause::MissingArgument);
    }

    if lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("not authorized")
        || lower.contains("authentication")
    {
        return Some(NativeCliCause::AuthOrPermission);
    }

    if lower.contains("no such file") || lower.contains("not found") && exit_code == 127 {
        return Some(NativeCliCause::FileNotFound);
    }

    None
}

/// Map a typed [`NativeCliCause`] to the report tuple `classify_exit` returns.
#[allow(clippy::type_complexity)]
fn native_cause_report(
    cause: &NativeCliCause,
    provider: Provider,
    stderr: &str,
    model_source: Option<&crate::commands::wrap::profile::OpenCodeModelSource>,
) -> (
    AgentErrorCategory,
    String,
    Option<String>,
    Option<String>,
    Option<Vec<String>>,
    Option<String>,
) {
    let first_line = || stderr.lines().next().unwrap_or("").to_string();
    match cause {
        NativeCliCause::ModelNotFound { suggestions } => {
            let location = model_source.map(|s| s.location_string().to_string());
            let loc = location.as_deref().unwrap_or("the command line");
            (
                AgentErrorCategory::AgentNative,
                format!(
                    "Invalid model specified in {loc}! Running <yellow>opencode models</yellow> will give you\n\
                     a list of all valid models. Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim>\n\
                     for direct providers like Google or Anthropic but take the form\n\
                     <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim> for aggregators like OpenRouter."
                ),
                None,
                None,
                suggestions.clone(),
                location,
            )
        }
        NativeCliCause::ArgumentRejected { flag } => {
            let suffix = flag
                .as_deref()
                .map(|f| format!(" (`{f}`)"))
                .unwrap_or_default();
            (
                AgentErrorCategory::AgentNative,
                format!(
                    "{} did not recognize a flag{suffix}",
                    crate::output::capitalize_provider(provider)
                ),
                Some(first_line()),
                None,
                None,
                None,
            )
        }
        NativeCliCause::MissingArgument => (
            AgentErrorCategory::AgentNative,
            "A required argument was missing from the command.".to_string(),
            Some(first_line()),
            None,
            None,
            None,
        ),
        NativeCliCause::AuthOrPermission => (
            AgentErrorCategory::Configuration,
            "An authentication or permission error occurred.".to_string(),
            Some(first_line()),
            Some("Check API keys and provider authentication configuration.".to_string()),
            None,
            None,
        ),
        NativeCliCause::FileNotFound => (
            AgentErrorCategory::Configuration,
            "A required file or command was not found.".to_string(),
            Some(first_line()),
            None,
            None,
            None,
        ),
    }
}

/// Extract the offending flag token from a native argument-rejection
/// diagnostic, trimming the surrounding quotes/punctuation providers wrap it
/// in (`'--foo'`, `"--foo"`, `--foo,`). Returns `None` when none is found.
fn extract_unknown_flag(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        for candidate in line.split_whitespace() {
            let trimmed = candidate.trim_matches(|c: char| {
                c == '\'' || c == '"' || c == ',' || c == '.' || c == '`'
            });
            if trimmed.starts_with('-') && trimmed.len() > 1 && !trimmed.contains("error") {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_first_api_error_message(stderr: &str) -> String {
    for line in stderr.lines() {
        if line.contains("API Error:") {
            return line.to_string();
        }
    }
    "An API error occurred.".to_string()
}

fn parse_model_suggestions(stderr: &str) -> Option<Vec<String>> {
    let lower = stderr.to_lowercase();
    let start = lower.find("suggestions:")?;
    let bracket_start = lower[start..].find('[')?;
    let bracket_end = lower[start + bracket_start..].find(']')?;
    let inner = &stderr[start + bracket_start + 1..start + bracket_start + bracket_end];

    let mut items = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    for ch in inner.chars() {
        if ch == '"' {
            if in_quote {
                if !current.is_empty() {
                    items.push(current.clone());
                    current.clear();
                }
                in_quote = false;
            } else {
                in_quote = true;
            }
        } else if in_quote {
            current.push(ch);
        }
    }

    if items.is_empty() { None } else { Some(items) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::wrap::profile::OpenCodeModelSource;

    fn opencode_report(
        exit_code: i32,
        stderr: &str,
        model_source: Option<&OpenCodeModelSource>,
    ) -> AgentErrorReport {
        AgentErrorReport::from_exit_code_with_source(
            Provider::OpenCode,
            exit_code,
            Some(stderr),
            model_source,
        )
    }

    #[test]
    fn classify_provider_model_not_found_error() {
        let stderr = "Error: ProviderModelNotFoundError: model xyz not found\nsuggestions: [\"abc/one\", \"abc/two\"]";
        let source = OpenCodeModelSource::CliSwitch("xyz".to_string());
        let report = opencode_report(1, stderr, Some(&source));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(
            report
                .summary
                .contains("Invalid model specified in the --model CLI switch")
        );
        assert!(report.suggestions.is_some());
        assert_eq!(report.suggestions.as_ref().unwrap().len(), 2);
        assert_eq!(report.suggestions.as_ref().unwrap()[0], "abc/one");
        assert_eq!(report.suggestions.as_ref().unwrap()[1], "abc/two");
        assert_eq!(report.location.as_deref(), Some("the --model CLI switch"));
    }

    #[test]
    fn classify_model_not_found_lowercased() {
        let stderr = "model not found: invalid model name";
        let source = OpenCodeModelSource::OpenCodeModelEnv("bad".to_string());
        let report = opencode_report(1, stderr, Some(&source));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(
            report
                .summary
                .contains("the OPENCODE_MODEL environment variable")
        );
        assert_eq!(
            report.location.as_deref(),
            Some("the OPENCODE_MODEL environment variable")
        );
    }

    #[test]
    fn classify_invalid_model_text() {
        let stderr = "invalid model specified";
        let source = OpenCodeModelSource::ConfigDefault("bad".to_string());
        let report = opencode_report(1, stderr, Some(&source));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(report.summary.contains("the config file"));
        assert_eq!(
            report.location.as_deref(),
            Some("the config file ~/.config/opencode/config.json")
        );
    }

    #[test]
    fn suggestions_parsed_from_stderr_payload() {
        let result = parse_model_suggestions(
            "some output\nSuggestions: [\"provider/a\", \"provider/b\", \"provider/c\"]\nmore",
        );
        assert_eq!(
            result,
            Some(vec![
                "provider/a".to_string(),
                "provider/b".to_string(),
                "provider/c".to_string(),
            ])
        );
    }

    #[test]
    fn suggestions_none_when_absent() {
        assert_eq!(parse_model_suggestions("no suggestions here"), None);
    }

    #[test]
    fn suggestions_none_when_empty_array() {
        assert_eq!(parse_model_suggestions("suggestions: []"), None);
    }

    #[test]
    fn no_model_provided_report_has_expected_content() {
        let report = AgentErrorReport::no_model_provided(Provider::OpenCode);
        assert_eq!(report.exit_code, 1);
        assert_eq!(report.category, AgentErrorCategory::Configuration);
        assert!(report.summary.contains("No model specified"));
        assert!(report.summary.contains("<yellow>model</yellow>"));
        assert!(
            report
                .summary
                .contains("<blue>~/.config/opencode/config.json</blue>")
        );
        assert!(report.body_list.is_some());
        let body_list = report.body_list.as_ref().unwrap();
        assert!(body_list.iter().any(|s| s.contains("OPENCODE_MODEL")));
        assert!(body_list.iter().any(|s| s.contains("--model")));
        assert!(report.footer.is_some());
        let footer = report.footer.as_ref().unwrap();
        assert!(footer.contains("<yellow>opencode models</yellow>"));
        assert!(footer.contains("<dim>[provider]</dim>"));
        assert!(footer.contains("<dim>[aggregator]</dim>"));
        assert!(report.suggestions.is_none());
        assert_eq!(report.suggestion_style, SuggestionStyle::BareList);
        assert!(report.location.is_none());
    }

    #[test]
    fn invalid_model_report_has_suggestions_and_location() {
        let report = AgentErrorReport::invalid_model(
            Provider::OpenCode,
            1,
            "the --model CLI switch".to_string(),
            vec!["suggestion/a".to_string(), "suggestion/b".to_string()],
        );
        assert_eq!(report.exit_code, 1);
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(
            report
                .summary
                .contains("Invalid model specified in the --model CLI switch")
        );
        assert!(report.summary.contains("<yellow>opencode models</yellow>"));
        assert!(report.summary.contains("<dim>[provider]</dim>"));
        assert!(report.summary.contains("<dim>[aggregator]</dim>"));
        assert_eq!(report.suggestions.as_ref().unwrap().len(), 2);
        assert_eq!(report.location.as_deref(), Some("the --model CLI switch"));
    }

    #[test]
    fn interrupted_exit_code_classified_correctly() {
        let report = AgentErrorReport::from_exit_code(Provider::OpenCode, 130, None);
        assert_eq!(report.category, AgentErrorCategory::Interrupted);
    }

    #[test]
    fn unknown_flag_classified_correctly() {
        let stderr = "error: unexpected argument '--foo' found";
        let report = AgentErrorReport::from_exit_code(Provider::Claude, 1, Some(stderr));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(report.summary.contains("did not recognize a flag"));
        assert!(report.summary.contains("--foo"), "flag name should be named: {}", report.summary);
    }

    #[test]
    fn native_cause_classifies_argument_rejection() {
        let cause = classify_native_cli_cause(2, "error: unexpected argument '--nope' found");
        assert_eq!(
            cause,
            Some(NativeCliCause::ArgumentRejected {
                flag: Some("--nope".to_string())
            })
        );
    }

    #[test]
    fn native_cause_does_not_misclassify_auth_as_argument() {
        // Collision fixture: an auth failure must not be read as arg rejection.
        let cause = classify_native_cli_cause(1, "Error: authentication failed: invalid api key");
        assert_eq!(cause, Some(NativeCliCause::AuthOrPermission));
    }

    #[test]
    fn native_cause_none_for_unclassified_exit() {
        assert_eq!(classify_native_cli_cause(1, "some unrelated crash output"), None);
    }

    #[test]
    fn correlated_report_names_forwarded_switch_on_arg_rejection() {
        let report = AgentErrorReport::correlated_with_forwarded_tail(
            Provider::Codex,
            2,
            Some("error: unexpected argument '--badflag' found"),
            None,
            &["--badflag".to_string()],
            false,
        );
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(report.summary.contains("--badflag"), "summary: {}", report.summary);
        assert!(report.summary.contains("likely caused by"));
    }

    #[test]
    fn correlated_report_defers_when_not_arg_rejection() {
        // An auth failure with a forwarded tail must NOT be attributed to it.
        let report = AgentErrorReport::correlated_with_forwarded_tail(
            Provider::Codex,
            1,
            Some("Error: authentication failed"),
            None,
            &["--badflag".to_string()],
            false,
        );
        assert_eq!(report.category, AgentErrorCategory::Configuration);
        assert!(!report.summary.contains("likely caused by"));
    }

    #[test]
    fn correlated_report_defers_when_no_forwarded_tail() {
        let report = AgentErrorReport::correlated_with_forwarded_tail(
            Provider::Codex,
            2,
            Some("error: unexpected argument '--x' found"),
            None,
            &[],
            false,
        );
        assert!(!report.summary.contains("likely caused by"));
    }

    #[test]
    fn model_not_found_without_source_uses_default_location() {
        let stderr = "ProviderModelNotFoundError: nope\nsuggestions: [\"a\"]";
        let report = AgentErrorReport::from_exit_code(Provider::OpenCode, 1, Some(stderr));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(report.summary.contains("the command line"));
        assert_eq!(report.suggestions.as_ref().unwrap()[0], "a");
    }

    #[test]
    fn semantic_error_kind_maps_to_agent_error_category() {
        assert_eq!(
            AgentErrorCategory::from(SemanticErrorKind::Configuration),
            AgentErrorCategory::Configuration
        );
        assert_eq!(
            AgentErrorCategory::from(SemanticErrorKind::AgentNative),
            AgentErrorCategory::AgentNative
        );
        assert_eq!(
            AgentErrorCategory::from(SemanticErrorKind::ApiRemote),
            AgentErrorCategory::ApiRemote
        );
        assert_eq!(
            AgentErrorCategory::from(SemanticErrorKind::Interrupted),
            AgentErrorCategory::Interrupted
        );
        // Unknown maps to AgentNative so the surface stays consistent with
        // the existing four-category reporting scheme.
        assert_eq!(
            AgentErrorCategory::from(SemanticErrorKind::Unknown),
            AgentErrorCategory::AgentNative
        );
    }

    #[test]
    fn model_not_found_without_source_or_suggestions_still_classifies() {
        let stderr = "ProviderModelNotFoundError: model xyz not found";
        let report = AgentErrorReport::from_exit_code(Provider::OpenCode, 1, Some(stderr));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(
            report
                .summary
                .contains("Invalid model specified in the command line")
        );
        assert!(report.suggestions.is_none());
        assert_eq!(report.location, None);
    }
}
