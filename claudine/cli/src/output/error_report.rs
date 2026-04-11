use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::renderable::RenderableContent;
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
}

impl AgentErrorReport {
    pub(crate) fn from_exit_code(provider: Provider, exit_code: i32, stderr: Option<&str>) -> Self {
        let (category, summary, detail, hint) = classify_exit(provider, exit_code, stderr);
        Self {
            provider,
            exit_code,
            category,
            summary,
            detail,
            hint,
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
        log::message("");
    }
}

fn classify_exit(
    provider: Provider,
    exit_code: i32,
    stderr: Option<&str>,
) -> (AgentErrorCategory, String, Option<String>, Option<String>) {
    let stderr_text = stderr.unwrap_or("");
    let provider_name = crate::output::capitalize_provider(provider);

    if exit_code == 130 || exit_code == 143 {
        return (
            AgentErrorCategory::Interrupted,
            format!("{provider_name} was interrupted by the user"),
            None,
            None,
        );
    }

    if let Some(category) = classify_native_cli_error(exit_code, stderr_text) {
        return category;
    }

    if stderr_text.contains("API Error:") {
        let message = extract_first_api_error_message(stderr_text);
        return (
            AgentErrorCategory::ApiRemote,
            message,
            Some("This error came from the provider's API layer.".to_string()),
            Some("Check API key, rate limits, and service status.".to_string()),
        );
    }

    (
        AgentErrorCategory::AgentNative,
        format!("{provider_name} exited with error code {exit_code}"),
        stderr_text.lines().next().map(|l| l.to_string()),
        None,
    )
}

fn classify_native_cli_error(
    exit_code: i32,
    stderr: &str,
) -> Option<(AgentErrorCategory, String, Option<String>, Option<String>)> {
    let lower = stderr.to_lowercase();

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
        ));
    }

    if lower.contains("no such file") || lower.contains("not found") && exit_code == 127 {
        return Some((
            AgentErrorCategory::Configuration,
            "A required file or command was not found.".to_string(),
            Some(stderr.lines().next().unwrap_or("").to_string()),
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
