use biscuit_terminal::components::block_quote::BlockQuote;
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

#[derive(Debug, Clone)]
pub(crate) struct AgentErrorReport {
    pub(crate) provider: Provider,
    pub(crate) exit_code: i32,
    pub(crate) category: AgentErrorCategory,
    pub(crate) summary: String,
    pub(crate) detail: Option<String>,
    pub(crate) hint: Option<String>,
    pub(crate) suggestions: Option<Vec<String>>,
    pub(crate) location: Option<String>,
}

impl AgentErrorReport {
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
            detail,
            hint,
            suggestions,
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
                 change this behavior by adding a model property to ~/.config/opencode/config.json.\n\
                 You can override/set the default model with any of the following methods:\n\n\
                 \x20\x20• set OPENCODE_MODEL to a valid model name\n\
                 \x20\x20• use the CLI switch --model <model>\n\n\
                 Running `opencode models` will give you a list of all valid models."
            ),
            detail: None,
            hint: None,
            suggestions: None,
            location: None,
        }
    }

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
                "Invalid model specified in {location}! Running `opencode models` will give you\n\
                 a list of all valid models."
            ),
            detail: None,
            hint: None,
            suggestions: Some(suggestions),
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

        let mut parts = vec![format!(
            "<red><bold>{label}</bold></red> <dim>({provider_name}, exit {})</dim>",
            self.exit_code
        )];
        parts.push(self.summary.clone());

        if let Some(ref detail) = self.detail {
            parts.push(format!("<dim>{detail}</dim>"));
        }
        if let Some(ref hint) = self.hint {
            parts.push(format!("<blue>{hint}</blue>"));
        }

        let content = parts.join("\n");
        let rendered = Prose::new(content).render(term);

        let mut block = BlockQuote::new(RenderableContent::from(rendered), None::<&str>)
            .with_left_block_color(border_color)
            .with_border("▌ ");
        block.layout_mut().left_margin = biscuit_terminal::utils::layout::Margin::Chars(2);
        block.layout_mut().right_margin = biscuit_terminal::utils::layout::Margin::Chars(2);

        log::message("");
        log::message(&block.render(term));

        if let Some(ref suggestions) = self.suggestions {
            if !suggestions.is_empty() {
                let suggestion_items: Vec<String> = suggestions
                    .iter()
                    .map(|s| format!("<yellow>{s}</yellow>"))
                    .collect();
                let list = UnorderedList::new(suggestion_items);
                let header =
                    Status::from_prose("Did you mean:".to_string()).state(StatusState::Warning);
                log::message(&header.render(term));
                log::message(&list.render(term));
            }
        }

        log::message("");
    }
}

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
        if suggestions.is_some() || location.is_some() {
            let loc = location.as_deref().unwrap_or("the command line");
            return Some((
                AgentErrorCategory::AgentNative,
                format!(
                    "Invalid model specified in {loc}! Running `opencode models` will give you\n\
                     a list of all valid models."
                ),
                None,
                None,
                suggestions,
                location,
            ));
        }
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

    if items.is_empty() { None } else { Some(items) }
}
