use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::renderable::RenderableContent;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use claudine::events::Provider;

use crate::log;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentErrorCategory {
    Configuration,
    AgentNative,
    ApiRemote,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SuggestionStyle {
    BareList,
    DidYouMean,
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

        let mut block = BlockQuote::new(RenderableContent::from(compose), None::<&str>)
            .with_left_block_color(border_color)
            .with_border("▌ ");
        block.layout_mut().left_margin = biscuit_terminal::utils::layout::Margin::Chars(2);
        block.layout_mut().right_margin = biscuit_terminal::utils::layout::Margin::Chars(2);

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

    if let Some(category) = classify_native_cli_error(exit_code, stderr_text, model_source) {
        return category;
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

#[allow(clippy::type_complexity)]
fn classify_native_cli_error(
    exit_code: i32,
    stderr: &str,
    model_source: Option<&crate::commands::wrap::profile::OpenCodeModelSource>,
) -> Option<(
    AgentErrorCategory,
    String,
    Option<String>,
    Option<String>,
    Option<Vec<String>>,
    Option<String>,
)> {
    let lower = stderr.to_lowercase();

    if lower.contains("providermodelnotfounderror")
        || lower.contains("model not found")
        || lower.contains("invalid model")
    {
        let suggestions = parse_model_suggestions(stderr);
        let location = model_source.map(|s| s.location_string().to_string());
        let loc = location.as_deref().unwrap_or("the command line");
        return Some((
            AgentErrorCategory::AgentNative,
            format!(
                "Invalid model specified in {loc}! Running <yellow>opencode models</yellow> will give you\n\
                 a list of all valid models. Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim>\n\
                 for direct providers like Google or Anthropic but take the form\n\
                 <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim> for aggregators like OpenRouter."
            ),
            None,
            None,
            suggestions,
            location,
        ));
    }

    if lower.contains("unrecognized argument")
        || lower.contains("unknown flag")
        || lower.contains("unknown option")
        || lower.contains("unexpected argument")
        || lower.contains("invalid argument")
    {
        let flag = extract_unknown_flag(stderr);
        return Some((
            AgentErrorCategory::AgentNative,
            format!("The provider did not recognize a flag{flag}"),
            Some(stderr.lines().next().unwrap_or("").to_string()),
            Some(
                "Use `--` before provider flags to prevent Claudine from intercepting them."
                    .to_string(),
            ),
            None,
            None,
        ));
    }

    if lower.contains("missing required argument")
        || lower.contains("required argument")
        || lower.contains("the following required arguments were not provided")
    {
        return Some((
            AgentErrorCategory::AgentNative,
            "A required argument was missing from the command.".to_string(),
            Some(stderr.lines().next().unwrap_or("").to_string()),
            None,
            None,
            None,
        ));
    }

    if lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("not authorized")
        || lower.contains("authentication")
    {
        return Some((
            AgentErrorCategory::Configuration,
            "An authentication or permission error occurred.".to_string(),
            Some(stderr.lines().next().unwrap_or("").to_string()),
            Some("Check API keys and provider authentication configuration.".to_string()),
            None,
            None,
        ));
    }

    if lower.contains("no such file") || lower.contains("not found") && exit_code == 127 {
        return Some((
            AgentErrorCategory::Configuration,
            "A required file or command was not found.".to_string(),
            Some(stderr.lines().next().unwrap_or("").to_string()),
            None,
            None,
            None,
        ));
    }

    None
}

fn extract_unknown_flag(stderr: &str) -> String {
    for line in stderr.lines() {
        for candidate in line.split_whitespace() {
            if candidate.starts_with('-') && !candidate.contains("error") {
                return format!(" (`{candidate}`)");
            }
        }
    }
    String::new()
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

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
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
        assert!(report
            .summary
            .contains("Invalid model specified in the --model CLI switch"));
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
        assert!(report
            .summary
            .contains("the OPENCODE_MODEL environment variable"));
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
        assert!(report.summary.contains("<blue>~/.config/opencode/config.json</blue>"));
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
        assert!(report
            .summary
            .contains("Invalid model specified in the --model CLI switch"));
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
    fn model_not_found_without_source_or_suggestions_still_classifies() {
        let stderr = "ProviderModelNotFoundError: model xyz not found";
        let report = AgentErrorReport::from_exit_code(Provider::OpenCode, 1, Some(stderr));
        assert_eq!(report.category, AgentErrorCategory::AgentNative);
        assert!(report.summary.contains("Invalid model specified in the command line"));
        assert!(report.suggestions.is_none());
        assert_eq!(report.location, None);
    }
}
