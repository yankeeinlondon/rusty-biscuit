pub(crate) mod api_errors;
pub(crate) mod assistant;
pub(crate) mod error_report;
pub(crate) mod error_walker;
pub(crate) mod switches;

pub(crate) use api_errors::try_format_api_error;
pub(crate) use assistant::{render_assistant_markdown, render_assistant_markdown_with_options};
pub(crate) use switches::{style_cli_switches, truncate_args};

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::discovery::eval::strip_ansi_codes;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges::{COMPOSE, INLINE_COMPOSE, INTERACTIVE, REPO_FLAG, SEQUENCE, VERBOSE, YOLO};
use claudine::provider::Provider;
use std::path::Path;

use crate::commands::wrap::McpRuntimeInfo;
use crate::commands::wrap::env::EnvPlan;
use crate::commands::wrap::profile::WrapperProfile;
use crate::log;

/// Context for compose/inline-compose mode display in the header.
#[allow(dead_code)]
pub(crate) enum ComposeDisplay {
    /// Chained composition (`claudine compose`).
    Compose,
    /// Inline composition (`claudine inline-compose`).
    InlineCompose,
}

fn trim_trailing_blank_rendered_lines(rendered: &str) -> String {
    let mut lines: Vec<&str> = rendered.lines().collect();
    while let Some(last) = lines.last() {
        if strip_ansi_codes(last).trim().is_empty() {
            lines.pop();
        } else {
            break;
        }
    }
    lines.join("\n")
}

/// Print the one-line header: `Claudine ▸ Provider [badges] prompt`
#[allow(clippy::too_many_arguments)]
pub(crate) fn log_wrapper_header(
    profile: &dyn WrapperProfile,
    yolo_requested: bool,
    non_interactive: bool,
    _interactive_override: bool,
    detail_requested: bool,
    repo_requested: bool,
    compose_display: Option<&ComposeDisplay>,
    sequence: bool,
    operation: Option<&str>,
    prompt_display: Option<&str>,
    compose_source_hint: Option<&str>,
    env_plan: &EnvPlan,
    term: &Terminal,
) {
    let mut header_parts: Vec<String> = vec![
        Prose::new(format!(
            "<blue><bold>Claudine</bold></blue> <dim>\u{25b8}</dim> <bold>{}</bold>",
            profile.provider()
        ))
        .render(term),
    ];

    if yolo_requested {
        header_parts.push(YOLO.to_string());
    }

    // Non-interactive is the norm when a prompt is given — no badge needed.
    // Show Interactive badge when session is interactive (explicit -i override
    // or no prompt at all).
    if !non_interactive {
        header_parts.push(INTERACTIVE.to_string());
    }

    if detail_requested {
        header_parts.push(VERBOSE.to_string());
    }

    match compose_display {
        Some(ComposeDisplay::Compose) => header_parts.push(COMPOSE.to_string()),
        Some(ComposeDisplay::InlineCompose) => header_parts.push(INLINE_COMPOSE.to_string()),
        None => {}
    }

    if sequence {
        header_parts.push(SEQUENCE.to_string());
    }

    if repo_requested {
        header_parts.push(REPO_FLAG.to_string());
    }

    if let Some(op) = operation {
        header_parts.push(
            Prose::new(format!(
                "<bg-green-900><green-100><bold> Op(<dim><i>{op}</i></dim>) </bold></green-100></bg-green-900>"
            ))
            .render(term),
        );
    }

    if let Some(package_name) = package_name_display(env_plan) {
        header_parts.push(
            Prose::new(format!(
                "<green><bold>PACKAGE_NAME:</bold> {package_name}</green>"
            ))
            .render(term),
        );
    }

    // For compose-based prompts, show the source file instead of the prompt text.
    // For static string prompts, show truncated prompt text as before.
    if let Some(filename) = compose_source_hint {
        let prose_safe = filename.replace('<', "\\<");
        header_parts.push(
            Prose::new(format!(
                "<dim><i>prompt sourced from <blue>{prose_safe}</blue></i></dim>"
            ))
            .render(term),
        );
    } else if let Some(prompt) = prompt_display {
        let flattened = prompt.replace('\n', "\\n").replace('\r', "\\r");
        let escaped = shell_escape(&flattened);
        let prefix = header_parts.join(" ");
        let prefix_width = visible_width(&prefix) as usize;
        let used = prefix_width + 1;
        let term_width = term.width() as usize;
        let available = term_width.saturating_sub(used);
        let truncated = truncate_args(&escaped, available);
        let prose_safe = truncated.replace('<', "\\<");
        header_parts.push(Prose::new(format!("<dim>{prose_safe}</dim>")).render(term));
    }

    log::message(&format!("\n{}\n", header_parts.join(" ")));
}

/// Render the composed prompt as a BlockQuote after environment details.
///
/// The prompt is rendered as Markdown via Darkmatter (for proper wrapping,
/// bold, links, code, etc.) and then wrapped in a green-bordered BlockQuote.
///
/// In verbose mode the entire prompt is shown. Otherwise the first 10 lines
/// are rendered with a truncation notice.
pub(crate) fn log_compose_prompt(prompt: &str, verbose: bool, term: &Terminal) {
    use biscuit_terminal::utils::color::{Color, Tailwind};
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

    log::message(&Prose::new("<bold>Agent Prompt:</bold>").render(term));

    let display_text = if verbose {
        prompt.to_string()
    } else {
        let lines: Vec<&str> = prompt.lines().collect();
        lines
            .iter()
            .take(10)
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Render prompt as Markdown through Darkmatter, constraining width to
    // account for the block quote border ("▌ " = 2 visible cols) and
    // margins (2 cols each side).
    let left_margin: u16 = 2;
    let right_margin: u16 = 2;
    let border_width: u16 = 2;
    let content_width = (term.width() as u16)
        .saturating_sub(border_width)
        .saturating_sub(left_margin)
        .saturating_sub(right_margin);
    let mut opts = TerminalOptions::default();
    opts.max_width = Some(content_width);
    let rendered = match for_terminal(&Markdown::new(display_text.trim()), opts) {
        Ok(r) => r,
        Err(_) => display_text.clone(),
    };

    // Wrap in a BlockQuote with green border, left margin, and no additional
    // word wrapping (Darkmatter already wrapped to the correct width).
    let mut block = BlockQuote::new(
        RenderableContent::from(trim_trailing_blank_rendered_lines(&rendered)),
        None::<&str>,
    )
    .with_left_block_color(Color::Tailwind(Tailwind::Green700))
    .with_border("▌ ");
    block.layout_mut().left_margin =
        biscuit_terminal::utils::layout::Margin::Chars(left_margin as u32);
    block.layout_mut().right_margin =
        biscuit_terminal::utils::layout::Margin::Chars(right_margin as u32);
    log::message(&block.render(term));

    if !verbose && prompt.lines().count() > 10 {
        log::message(""); // blank line between block quote and bullet
        log::message(
            &Prose::new(
                "- <dim>remaining prompt truncated for brevity, use <blue>--verbose</blue> to show entire prompt</dim>",
            )
            .with_word_wrap(WordWrap::WrapProse(None, Some(2)))
            .render(term),
        );
    }
}

pub(crate) fn log_system_prompt(
    effective_sp: &claudine::system_prompt::EffectiveSystemPrompt,
    verbose: bool,
    silent: bool,
    quiet: bool,
    term: &Terminal,
) {
    use biscuit_terminal::utils::color::{Color, Tailwind};
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

    if silent {
        return;
    }

    match effective_sp {
        claudine::system_prompt::EffectiveSystemPrompt::None => {
            if verbose && !quiet {
                let mut block = BlockQuote::new(
                    RenderableContent::from("the system prompt has not been modified".to_string()),
                    None::<&str>,
                )
                .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
                .with_border("▌ ");
                block.layout_mut().left_margin = biscuit_terminal::utils::layout::Margin::Chars(2);
                block.layout_mut().right_margin = biscuit_terminal::utils::layout::Margin::Chars(2);
                log::message(&block.render(term));
            }
        }
        claudine::system_prompt::EffectiveSystemPrompt::Disabled { source: _ } => {
            if verbose && !quiet {
                let mut block = BlockQuote::new(
                    RenderableContent::from("the system prompt has been disabled".to_string()),
                    None::<&str>,
                )
                .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
                .with_border("▌ ");
                block.layout_mut().left_margin = biscuit_terminal::utils::layout::Margin::Chars(2);
                block.layout_mut().right_margin = biscuit_terminal::utils::layout::Margin::Chars(2);
                log::message(&block.render(term));
            }
        }
        claudine::system_prompt::EffectiveSystemPrompt::Ready(prepared) => {
            let variant_label = match prepared.mode {
                claudine::system_prompt::SystemPromptMode::Append => "appended",
                claudine::system_prompt::SystemPromptMode::Replace => "replaced",
            };
            log::message(
                &Prose::new(format!(
                    "<bold>System Prompt(<dim><i>{variant_label}</i></dim>):</bold>"
                ))
                .render(term),
            );

            let full_text = &prepared.composed_markdown;

            let left_margin: u16 = 2;
            let right_margin: u16 = 2;
            let border_width: u16 = 2;
            let content_width = (term.width() as u16)
                .saturating_sub(border_width)
                .saturating_sub(left_margin)
                .saturating_sub(right_margin);
            let mut opts = TerminalOptions::default();
            opts.max_width = Some(content_width);
            let rendered = match for_terminal(&Markdown::new(full_text.trim()), opts) {
                Ok(r) => r,
                Err(_) => full_text.clone(),
            };

            let mut block = BlockQuote::new(
                RenderableContent::from(trim_trailing_blank_rendered_lines(&rendered)),
                None::<&str>,
            )
            .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
            .with_border("▌ ");
            block.layout_mut().left_margin =
                biscuit_terminal::utils::layout::Margin::Chars(left_margin as u32);
            block.layout_mut().right_margin =
                biscuit_terminal::utils::layout::Margin::Chars(right_margin as u32);
            log::message(&block.render(term));
        }
    }
}

/// Print environment variable details (removed, included, added).
pub(crate) fn log_wrapper_env_details(
    env_plan: &EnvPlan,
    mcp_runtime: Option<&McpRuntimeInfo>,
    term: &Terminal,
    verbose: u8,
) {
    if verbose > 0
        || !env_plan.added.is_empty()
        || !env_plan.removed.is_empty()
        || !env_plan.included.is_empty()
    {
        log::message(&Prose::new("<bold>Environment Variables:</bold>").render(term));

        let mut items: Vec<RenderableContent> = Vec::new();
        for removed in &env_plan.removed {
            items.push(RenderableContent::from(Prose::new(format!(
                "<red><strikethrough>{removed}</strikethrough></red>"
            ))));
        }

        for included in &env_plan.included {
            items.push(RenderableContent::from(Prose::new(format!(
                "<orange>{included}</orange>"
            ))));
        }

        for (key, value) in &env_plan.added {
            items.push(RenderableContent::from(Prose::new(format!(
                "<green>{key}</green><dim>={}</dim>",
                summarize_value(key, value)
            ))));
        }

        if items.is_empty() {
            items.push(RenderableContent::from(Prose::new(
                "<dim>no environment changes</dim>",
            )));
        }

        let rendered = UnorderedList::from(items).with_bullet("• ").render(term);
        log::message(&rendered);
    }

    if let Some(mcp_runtime) = mcp_runtime {
        log_mcp_runtime(term, mcp_runtime);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn log_dry_run(
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    repo_requested: bool,
    env_plan: &EnvPlan,
    mcp_runtime: Option<&McpRuntimeInfo>,
    child_cwd: &Path,
    term: &Terminal,
    sp_lines: Option<&[String]>,
) {
    let mut header = format!(
        "\n<blue><bold>Claudine</bold></blue> <dim>\u{25b8}</dim> <bold>{}</bold> <dim>[DRY RUN]</dim>",
        profile.provider()
    );
    if repo_requested {
        header.push_str(&format!(" {}", &*REPO_FLAG.to_string()));
    }
    log::message(&Prose::new(header).render(term));

    // Working directory
    log::message(
        &Prose::new(format!(
            "<bold>Working directory:</bold> <dim>{}</dim>",
            child_cwd.display()
        ))
        .render(term),
    );

    // Full command line
    let cmd_parts: Vec<String> = std::iter::once(binary_path.display().to_string())
        .chain(child_args.iter().map(|a| shell_escape(a)))
        .collect();
    log::message(
        &Prose::new(format!(
            "<bold>Command:</bold> <dim>{}</dim>",
            cmd_parts.join(" ")
        ))
        .render(term),
    );

    // Environment changes
    log::message(&Prose::new("<bold>Environment Changes:</bold>").render(term));
    let mut items: Vec<RenderableContent> = Vec::new();
    for removed in &env_plan.removed {
        items.push(RenderableContent::from(Prose::new(format!(
            "<red><strikethrough>{removed}</strikethrough></red>"
        ))));
    }
    for included in &env_plan.included {
        items.push(RenderableContent::from(Prose::new(format!(
            "<orange>{included}</orange>"
        ))));
    }
    for (key, value) in &env_plan.added {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>{key}</green><dim>={value}</dim>"
        ))));
    }
    if items.is_empty() {
        items.push(RenderableContent::from(Prose::new(
            "<dim>no environment changes</dim>",
        )));
    }
    let rendered = UnorderedList::from(items).with_bullet("• ").render(term);
    log::message(&rendered);

    if let Some(mcp_runtime) = mcp_runtime {
        log_mcp_runtime(term, mcp_runtime);
    }

    if let Some(lines) = sp_lines {
        log::message(&Prose::new("<bold>System prompt:</bold>").render(term));
        let items: Vec<RenderableContent> = lines
            .iter()
            .map(|l| RenderableContent::from(Prose::new(format!("<dim>{l}</dim>"))))
            .collect();
        let rendered = UnorderedList::from(items).with_bullet("• ").render(term);
        log::message(&rendered);
    }
}

pub(crate) fn summarize_value(key: &str, value: &str) -> String {
    if key == "AGENT_PARAMS" && value.len() > 120 {
        let mut end = 117;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        return format!("{}...", &value[..end]);
    }
    value.to_string()
}

pub(crate) fn package_name_display(env_plan: &EnvPlan) -> Option<String> {
    let package_context = env_plan.package_context.as_ref()?;
    let package = package_context.package.as_deref()?;

    Some(format!(
        "{package} <dim>(area: {})</dim>",
        package_context.package_area
    ))
}

pub(crate) fn repo_flag_info_message(
    term: &Terminal,
    shadow_home: Option<&std::path::Path>,
) -> String {
    let shadow_msg = if let Some(path) = shadow_home {
        format!(
            " A shadow HOME has been created at <blue>{}</blue> to preserve authentication.",
            path.display()
        )
    } else {
        String::new()
    };

    Prose::new(format!(
        "- <blue><bold>Info:</bold></blue> the {} was used; this constrains skills, commands, and subagent definitions to those in the repo.{}",
        &*claudine::badges::REPO_FLAG,
        shadow_msg
    ))
    .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
    .render(term)
}

pub(crate) fn removed_env_info_message(removed_env: &[String], term: &Terminal) -> Option<String> {
    if removed_env.is_empty() {
        return None;
    }
    Some(
        Prose::new(
            "- <blue><bold>Info:</bold></blue> potentially dangerous ENV variables were removed; \
             if you need one of these to be included use the <blue>--include \
             <dim>\\<ENV\\></dim></blue> CLI switch",
        )
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
        .render(term),
    )
}

fn log_mcp_runtime(term: &Terminal, mcp_runtime: &McpRuntimeInfo) {
    log::message(&Prose::new("<bold>MCP:</bold>").render(term));

    let mut items: Vec<RenderableContent> = Vec::new();
    if mcp_runtime.servers.is_empty() {
        items.push(RenderableContent::from(Prose::new(
            "<dim>no active MCP servers</dim>",
        )));
    } else {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>servers</green><dim>={}</dim>",
            mcp_runtime.servers.join(", ")
        ))));
    }

    if !mcp_runtime.default_servers.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>defaults</green><dim>={}</dim>",
            mcp_runtime.default_servers.join(", ")
        ))));
    }
    if !mcp_runtime.explicit_servers.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>use</green><dim>={}</dim>",
            mcp_runtime.explicit_servers.join(", ")
        ))));
    }
    if !mcp_runtime.tag_servers.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>tag_servers</green><dim>={}</dim>",
            mcp_runtime.tag_servers.join(", ")
        ))));
    }

    if !mcp_runtime.resolved_tags.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>tags</green><dim>={}</dim>",
            mcp_runtime.resolved_tags.join(", ")
        ))));
    }
    if !mcp_runtime.missing_tags.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<orange>missing_tags</orange><dim>={}</dim>",
            mcp_runtime.missing_tags.join(", ")
        ))));
    }
    if !mcp_runtime.ambiguous_tags.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<orange>ambiguous_tags</orange><dim>={}</dim>",
            mcp_runtime.ambiguous_tags.join(", ")
        ))));
    }
    if let Some(cleaned_prompt) = &mcp_runtime.cleaned_prompt {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>cleaned_prompt</green><dim>={}</dim>",
            shell_escape(cleaned_prompt)
        ))));
    }
    if !mcp_runtime.env_vars_set.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>env</green><dim>={}</dim>",
            mcp_runtime.env_vars_set.join(", ")
        ))));
    }
    if !mcp_runtime.extra_args.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>extra_args</green><dim>={}</dim>",
            mcp_runtime
                .extra_args
                .iter()
                .map(|arg| shell_escape(arg))
                .collect::<Vec<_>>()
                .join(" ")
        ))));
    }
    if !mcp_runtime.temp_files.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>files</green><dim>={}</dim>",
            mcp_runtime
                .temp_files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))));
    }

    let rendered = UnorderedList::from(items).with_bullet("• ").render(term);
    log::message(&rendered);
}

pub(crate) fn post_env_message(message: &str, term: &Terminal) -> String {
    let styled = style_cli_switches(message);
    Prose::new(format!("- {styled}"))
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
        .render(term)
}

pub(crate) fn post_env_warning_message(message: &str, term: &Terminal) -> String {
    let styled = style_cli_switches(message);
    Prose::new(format!("- <orange><bold>Warning:</bold></orange> {styled}"))
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
        .render(term)
}

pub(crate) use biscuit_terminal::utils::text::shell_escape_for_display as shell_escape;

/// Format the session start line for non-interactive prompts.
///
/// Produces: `- *Claude* session ID abc123def4`
pub(crate) fn format_session_start(
    provider: Provider,
    session_id: &str,
    model: Option<&str>,
) -> String {
    let name = capitalize_provider(provider);
    let short_id = if session_id.len() > 12 {
        &session_id[..12]
    } else {
        session_id
    };
    let model_part = if let Some(m) = model {
        format!(" \u{00b7} {m}")
    } else {
        String::new()
    };
    Prose::new(format!(
        "- <i>{name}</i><dim> session ID </dim>{short_id}<dim>{model_part}</dim>"
    ))
    .with_word_wrap(WordWrap::WrapProse(None, Some(2)))
    .render(&crate::log::terminal())
}

/// Format a terminal-error `Status` line announcing that the user aborted
/// the wrapped session with Ctrl+C.
///
/// Rendered with the circular failure theme so the icon matches Claudine's
/// other terminal-error surfaces. Intended for stderr in the wrap command's
/// `ProcessTermination::Interrupted` short-circuit.
#[allow(deprecated)]
pub(crate) fn format_user_interrupt_status() -> String {
    Status::new("User terminated non-interactive session with CTRL+C")
        .state(StatusState::Failure)
        .theme(StatusTheme::Circular)
        .render(&crate::log::terminal())
}

/// Format an INFO-only line announcing the working directory used to launch the agent.
pub(crate) fn format_launch_directory(directory: &Path) -> String {
    Prose::new(format!(
        "- <dim>starting agent in</dim> <blue>{}</blue>",
        directory.display()
    ))
    .with_word_wrap(WordWrap::WrapProse(None, Some(2)))
    .render(&crate::log::terminal())
}

/// Format a validation check line (success).
pub(crate) fn check_ok(message: &str, term: &Terminal) -> String {
    Prose::new(format!("<green-500>\u{2713}</green-500> {message}")).render(term)
}

/// Format a validation check line (failure).
pub(crate) fn check_fail(message: &str, term: &Terminal) -> String {
    Prose::new(format!("<red-500>\u{2a2f}</red-500> {message}")).render(term)
}

/// Format a frontmatter-prompt validation check line (success).
///
/// Thin wrapper kept for backwards compatibility.
pub(crate) fn fm_check_ok(message: &str, term: &Terminal) -> String {
    check_ok(message, term)
}

/// Format a frontmatter-prompt validation check line (failure).
///
/// Thin wrapper kept for backwards compatibility.
pub(crate) fn fm_check_fail(message: &str, term: &Terminal) -> String {
    check_fail(message, term)
}

pub(crate) fn capitalize_provider(provider: Provider) -> String {
    let s = format!("{provider}");
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_launch_directory_mentions_directory() {
        let rendered = format_launch_directory(Path::new("/tmp/project"));
        assert!(rendered.contains("starting agent in"));
        assert!(rendered.contains("/tmp/project"));
    }

    #[test]
    fn trim_trailing_blank_rendered_lines_removes_plain_and_ansi_blank_lines() {
        let rendered = "first\nsecond\n\x1b[32m\x1b[0m\n\n";
        assert_eq!(
            trim_trailing_blank_rendered_lines(rendered),
            "first\nsecond"
        );
    }

    mod interrupt_status_tests {
        use super::super::format_user_interrupt_status;
        use biscuit_terminal::prelude::strip_escape_codes;

        #[test]
        fn format_user_interrupt_status_renders_failure_icon_and_message() {
            let rendered = format_user_interrupt_status();
            let plain = strip_escape_codes(&rendered);
            assert!(
                plain.contains("User terminated non-interactive session with CTRL+C"),
                "missing expected interrupt message: {plain:?}"
            );
            assert!(
                !plain.contains('\u{2192}') && !plain.contains('\u{2190}'),
                "interrupt status must not reuse tool-call arrows: {plain:?}"
            );
            assert!(
                !plain.trim().is_empty(),
                "rendered status line must not be empty"
            );
        }
    }
}
