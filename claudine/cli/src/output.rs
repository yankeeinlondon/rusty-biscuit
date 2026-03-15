use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges::{COMPOSE, INLINE_COMPOSE, NON_INTERACTIVE, REPO_FLAG, YOLO};
use std::path::Path;

use crate::commands::wrap::McpRuntimeInfo;
use crate::commands::wrap::env::EnvPlan;
use crate::commands::wrap::profile::WrapperProfile;
use crate::commands::wrap::prompt_file::PromptFileDryRunInfo;
use crate::log;

/// Context for compose/inline-compose mode display in the header.
pub(crate) enum ComposeDisplay {
    /// Chained composition (`--compose`).
    Compose,
    /// Inline frontmatter-prompt composition (`--frontmatter-prompt`).
    InlineCompose,
}

/// Print the one-line header: `Claudine ▸ Provider [badges] args`
#[allow(clippy::too_many_arguments)]
pub(crate) fn log_wrapper_header(
    profile: &dyn WrapperProfile,
    yolo_requested: bool,
    non_interactive_requested: bool,
    repo_requested: bool,
    compose_display: Option<&ComposeDisplay>,
    operation: Option<&str>,
    child_args: &[String],
    prompt_summary: Option<&str>,
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

    if non_interactive_requested {
        header_parts.push(NON_INTERACTIVE.to_string());
    }

    match compose_display {
        Some(ComposeDisplay::Compose) => header_parts.push(COMPOSE.to_string()),
        Some(ComposeDisplay::InlineCompose) => header_parts.push(INLINE_COMPOSE.to_string()),
        None => {}
    }

    if repo_requested {
        header_parts.push(REPO_FLAG.to_string());
    }

    if let Some(op) = operation {
        header_parts.push(
            Prose::new(format!("<green><bold>OP:</bold> {op}</green>")).render(term),
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

    // Build the trailing args display: passthrough args + optional prompt summary
    let mut remaining = format_passthrough_args(child_args);
    if let Some(prompt) = prompt_summary {
        let escaped = shell_escape(prompt);
        if remaining.is_empty() {
            remaining = escaped;
        } else {
            remaining = format!("{remaining} {escaped}");
        }
    }

    if !remaining.is_empty() {
        // Measure the prefix (everything before the passthrough args) plus the
        // joining space so we know how many columns are left for the args.
        let prefix = header_parts.join(" ");
        let prefix_width = visible_width(&prefix) as usize;
        // +1 for the space between prefix and args
        let used = prefix_width + 1;

        let term_width = term.width() as usize;
        // Allow spilling onto a second line, but no further.
        let max_width = term_width.saturating_mul(2);
        let available = max_width.saturating_sub(used);

        let truncated = truncate_args(&remaining, available);
        header_parts.push(Prose::new(format!("<dim>{truncated}</dim>")).render(term));
    }

    log::message(&format!("\n{}", header_parts.join(" ")));
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
    prompt_file_info: Option<&PromptFileDryRunInfo>,
    child_cwd: &Path,
    term: &Terminal,
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

    if let Some(pf) = prompt_file_info {
        log_prompt_file_dry_run(term, pf);
    }
}

fn log_prompt_file_dry_run(term: &Terminal, info: &PromptFileDryRunInfo) {
    log::message(&Prose::new("<bold>Prompt File:</bold>").render(term));

    let mut items: Vec<RenderableContent> = Vec::new();
    items.push(RenderableContent::from(Prose::new(format!(
        "<green>path</green><dim>={} (from {})</dim>",
        info.resolved_path.display(),
        info.original
    ))));
    items.push(RenderableContent::from(Prose::new(format!(
        "<green>delivery</green><dim>={}</dim>",
        info.delivery_method
    ))));
    if !info.env_names.is_empty() {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>env_vars</green><dim>={}</dim>",
            info.env_names.join(", ")
        ))));
    }

    let rendered = UnorderedList::from(items).with_bullet("• ").render(term);
    log::message(&rendered);
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

pub(crate) fn format_passthrough_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ")
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

    format!("'{}'", arg.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn truncate_args_respects_char_boundaries() {
        // Multi-byte chars: each é is 2 bytes
        let args = "ééééééééééé";
        let result = truncate_args(args, 10);
        assert!(result.ends_with("...\""));
        // Should not panic or produce invalid UTF-8
        assert!(result.is_char_boundary(0));
    }
}
