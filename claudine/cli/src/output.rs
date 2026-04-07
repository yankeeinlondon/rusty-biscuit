use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges::{COMPOSE, INLINE_COMPOSE, INTERACTIVE, REPO_FLAG, SEQUENCE, VERBOSE, YOLO};
use claudine::events::Provider;
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
    log::message("");

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
        RenderableContent::from(rendered.trim_end().to_string()),
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
        return format!("{}...", &value[..117]);
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

pub(crate) fn opencode_non_interactive_model_hint() -> String {
    "<bold><blue>Info:</blue></bold> Opencode requires a model be specified when run in \
     non-interactive mode. You can specify with the --model switch or set either OPENCODE_MODEL \
     or MODEL environment variables."
        .to_string()
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

pub(crate) fn style_cli_switches(message: &str) -> String {
    let bytes = message.as_bytes();
    let mut i = 0usize;
    let mut last = 0usize;
    let mut out = String::with_capacity(message.len() + 32);

    while i < bytes.len() {
        let Some((start, end)) = next_switch_span(bytes, i) else {
            break;
        };
        out.push_str(&message[last..start]);
        out.push_str("<blue>");
        out.push_str(&message[start..end]);
        out.push_str("</blue>");
        i = end;
        last = end;
    }

    out.push_str(&message[last..]);
    out
}

fn next_switch_span(bytes: &[u8], start_at: usize) -> Option<(usize, usize)> {
    let mut i = start_at;
    while i < bytes.len() {
        if bytes[i] == b'-' && is_switch_boundary(bytes, i) {
            if i + 2 < bytes.len() && bytes[i + 1] == b'-' && is_switch_start(bytes[i + 2]) {
                let mut j = i + 3;
                while j < bytes.len() && is_switch_continue(bytes[j]) {
                    j += 1;
                }
                return Some((i, j));
            }
            if i + 1 < bytes.len() && is_switch_start(bytes[i + 1]) {
                let mut j = i + 2;
                while j < bytes.len() && is_switch_continue(bytes[j]) {
                    j += 1;
                }
                return Some((i, j));
            }
        }
        i += 1;
    }
    None
}

fn is_switch_boundary(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }

    !bytes[index - 1].is_ascii_alphanumeric()
}

fn is_switch_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

fn is_switch_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// Truncate the passthrough args string to fit within `max_chars` visible columns.
///
/// When the args exceed the available space, we cut 4 characters before the limit
/// and append `..."` so the total fits. If there's not even room for the ellipsis,
/// we return just `..."`.
fn truncate_args(args: &str, max_chars: usize) -> String {
    if args.len() <= max_chars {
        return args.to_string();
    }

    // We need 4 characters for the suffix: `..."`
    const SUFFIX: &str = "...\"";
    const SUFFIX_LEN: usize = 4;

    if max_chars <= SUFFIX_LEN {
        return SUFFIX.to_string();
    }

    let cut_at = max_chars - SUFFIX_LEN;

    // Find a safe char boundary (args is UTF-8)
    let truncated = if args.is_char_boundary(cut_at) {
        &args[..cut_at]
    } else {
        // Walk backwards to find a valid char boundary
        let mut pos = cut_at;
        while pos > 0 && !args.is_char_boundary(pos) {
            pos -= 1;
        }
        &args[..pos]
    };

    format!("{truncated}{SUFFIX}")
}

pub(crate) fn shell_escape(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }

    if arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '='))
    {
        return arg.to_string();
    }

    let escaped = arg
        .replace('\'', "'\\''")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("'{escaped}'")
}

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

/// Format an INFO-only line announcing the working directory used to launch the agent.
pub(crate) fn format_launch_directory(directory: &Path) -> String {
    Prose::new(format!(
        "- <dim>starting agent in</dim> <blue>{}</blue>",
        directory.display()
    ))
    .with_word_wrap(WordWrap::WrapProse(None, Some(2)))
    .render(&crate::log::terminal())
}

/// Render assistant terminal text through `Prose` so wrapping and styling are terminal-aware.
pub(crate) fn render_assistant_text(text: &str, term: &Terminal) -> String {
    Prose::new(text)
        .with_word_wrap(WordWrap::WrapProse(None, None))
        .render(term)
}

/// Render assistant text as full Markdown via darkmatter for rich terminal output.
///
/// Produces syntax-highlighted code blocks, formatted tables, bold/italic styling,
/// etc. Falls back to [`render_assistant_text`] (Prose word-wrap only) on error.
pub(crate) fn render_assistant_markdown(text: &str, term: &Terminal) -> String {
    render_assistant_markdown_with_options(text, term, None)
}

/// Render assistant text as Markdown with pre-built [`TerminalOptions`].
///
/// When `options` is `None`, a fresh default is created (incurs theme detection).
/// Pass a cached instance for hot paths like streaming.
pub(crate) fn render_assistant_markdown_with_options(
    text: &str,
    term: &Terminal,
    options: Option<&darkmatter::markdown::output::terminal::TerminalOptions>,
) -> String {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

    let owned;
    let opts = match options {
        Some(o) => o,
        None => {
            owned = TerminalOptions::default();
            &owned
        }
    };

    let md = Markdown::new(text.trim());
    match for_terminal(&md, opts.clone()) {
        Ok(rendered) => rendered,
        Err(_) => render_assistant_text(text, term),
    }
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

/// Try to reformat a raw API error line (e.g. `API Error: 529 {"type":"error",...}`)
/// into a human-readable message. Returns `None` if the line doesn't match.
pub(crate) fn try_format_api_error(line: &str, term: &Terminal) -> Option<String> {
    // Match pattern: "API Error: NNN {json}" or "API Error: NNN ..." at minimum
    let rest = line.strip_prefix("API Error: ")?;

    // Extract status code
    let (status_str, json_part) = rest.split_once(' ')?;
    let status: u16 = status_str.parse().ok()?;

    // Try to parse the JSON for a friendly message
    let friendly = if let Ok(obj) = serde_json::from_str::<serde_json::Value>(json_part) {
        let error_type = obj
            .get("error")
            .and_then(|e| e.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");
        let message = obj
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        let request_id = obj.get("request_id").and_then(|r| r.as_str());

        let mut parts = vec![format!(
            "<red><bold>API Error ({status}):</bold></red> {message}"
        )];

        // Add context for known error types
        match error_type {
            "overloaded_error" => {
                parts.push("<dim>The API is temporarily overloaded. This is usually transient — retrying the command may succeed.</dim>".to_string());
            }
            "api_error" if status == 500 => {
                parts.push("<dim>An internal server error occurred. This is usually transient — retrying the command may succeed.</dim>".to_string());
            }
            "rate_limit_error" => {
                parts.push(
                    "<dim>Rate limit exceeded. Wait a moment before retrying.</dim>".to_string(),
                );
            }
            _ => {}
        }

        if let Some(rid) = request_id {
            parts.push(format!("<dim>request: {rid}</dim>"));
        }

        parts.join("\n")
    } else {
        // Not valid JSON, just format the raw message
        format!(
            "<red><bold>API Error ({status}):</bold></red> {}",
            json_part.trim()
        )
    };

    Some(Prose::new(friendly).render(term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;

    /// Create a Terminal without real terminal probing (avoids TTY hangs in tests).
    fn test_terminal() -> Terminal {
        Terminal::new_optimistic(80)
    }

    #[test]
    fn try_format_api_error_parses_overloaded() {
        let term = test_terminal();
        let line = r#"API Error: 529 {"type":"error","error":{"type":"overloaded_error","message":"Overloaded. https://docs.claude.com/en/api/errors"},"request_id":"req_abc123"}"#;
        let result = try_format_api_error(line, &term).unwrap();
        assert!(result.contains("API Error (529)"));
        assert!(result.contains("Overloaded"));
        assert!(result.contains("transient"));
        assert!(result.contains("req_abc123"));
    }

    #[test]
    fn try_format_api_error_parses_500() {
        let term = test_terminal();
        let line = r#"API Error: 500 {"type":"error","error":{"type":"api_error","message":"Internal server error"},"request_id":"req_xyz"}"#;
        let result = try_format_api_error(line, &term).unwrap();
        assert!(result.contains("API Error (500)"));
        assert!(result.contains("Internal server error"));
    }

    #[test]
    fn try_format_api_error_returns_none_for_non_match() {
        let term = test_terminal();
        assert!(try_format_api_error("some random line", &term).is_none());
        assert!(try_format_api_error("Error: something", &term).is_none());
    }

    #[test]
    fn truncate_args_no_op_when_fits() {
        let args = "--flag 'short prompt'";
        assert_eq!(truncate_args(args, 50), args);
    }

    #[test]
    fn truncate_args_exact_fit() {
        let args = "--flag 'hello'";
        assert_eq!(truncate_args(args, args.len()), args);
    }

    #[test]
    fn truncate_args_truncates_with_suffix() {
        let args = "--dangerously-skip-permissions 'this is a very long prompt that goes on'";
        let result = truncate_args(args, 40);
        assert!(result.ends_with("...\""));
        assert_eq!(result.len(), 40);
    }

    #[test]
    fn truncate_args_tiny_budget() {
        let args = "'some long prompt'";
        let result = truncate_args(args, 4);
        assert_eq!(result, "...\"");
    }

    #[test]
    fn truncate_args_budget_smaller_than_suffix() {
        let args = "'some long prompt'";
        let result = truncate_args(args, 2);
        assert_eq!(result, "...\"");
    }

    #[test]
    fn shell_escape_replaces_newlines() {
        assert_eq!(shell_escape("Hi.\nHow are you?"), "'Hi.\\nHow are you?'");
    }

    #[test]
    fn shell_escape_replaces_tabs_and_carriage_returns() {
        assert_eq!(shell_escape("a\tb\rc"), "'a\\tb\\rc'");
    }

    #[test]
    fn format_launch_directory_mentions_directory() {
        let rendered = format_launch_directory(Path::new("/tmp/project"));
        assert!(rendered.contains("starting agent in"));
        assert!(rendered.contains("/tmp/project"));
    }

    #[test]
    fn truncate_args_respects_char_boundaries() {
        // Multi-byte chars: each é is 2 bytes
        let args = "ééééééééééé";
        let result = truncate_args(args, 10);
        assert!(result.ends_with("...\""));
        // Should not panic or produce invalid UTF-8
        assert!(result.is_char_boundary(0));
    }

    #[test]
    fn render_assistant_text_wraps_long_terminal_output() {
        let term = Terminal::new_optimistic(24);
        let rendered = render_assistant_text(
            "This is a long assistant sentence that should wrap cleanly in the terminal.",
            &term,
        );
        assert!(rendered.contains('\n'));
    }
}
