use std::path::Path;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges::{NON_INTERACTIVE, YOLO};

use crate::commands::wrap::profile::WrapperProfile;
use crate::commands::wrap::env::EnvPlan;
use crate::log;

pub(crate) fn log_wrapper_summary(
    profile: &dyn WrapperProfile,
    yolo_requested: bool,
    non_interactive_requested: bool,
    child_args: &[String],
    env_plan: &EnvPlan,
    term: &Terminal,
    verbose: u8,
) {
    // Header always includes: Claudine ▸ ProviderName [badges]
    let mut header_parts: Vec<String> = vec![
        Prose::new(format!(
            "<blue><bold>Claudine</bold></blue> <dim>\u{25b8}</dim> <bold>{}</bold>",
            profile.provider()
        ))
        .fallback_render(term),
    ];

    if yolo_requested {
        header_parts.push(YOLO.to_string());
    }

    if non_interactive_requested {
        header_parts.push(NON_INTERACTIVE.to_string());
    }

    if let Some(package_name) = package_name_display(env_plan) {
        header_parts.push(
            Prose::new(format!(
                "<green><bold>PACKAGE_NAME:</bold> {package_name}</green>"
            ))
            .fallback_render(term),
        );
    }

    let remaining = format_passthrough_args(child_args);
    if !remaining.is_empty() {
        header_parts.push(Prose::new(format!("<dim>{remaining}</dim>")).fallback_render(term));
    }

    log::message(&format!("\n{}", header_parts.join(" ")));

    // Verbose level 0: header only (no env details)
    // Verbose level 1+: header + environment changes
    if verbose > 0 || !env_plan.added.is_empty() || !env_plan.removed.is_empty() || !env_plan.included.is_empty() {
        log::message(
            &Prose::new("<bold>Environment Variables:</bold>").fallback_render(term),
        );

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

        let rendered = UnorderedList::from(items)
            .with_bullet("• ")
            .fallback_render(term);
        log::message(&rendered);
    }
}

pub(crate) fn log_dry_run(
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    env_plan: &EnvPlan,
    child_cwd: &Path,
    term: &Terminal,
) {
    log::message(
        &Prose::new(format!(
            "\n<blue><bold>Claudine</bold></blue> <dim>\u{25b8}</dim> <bold>{}</bold> <dim>[DRY RUN]</dim>",
            profile.provider()
        ))
        .fallback_render(term),
    );

    // Working directory
    log::message(
        &Prose::new(format!(
            "<bold>Working directory:</bold> <dim>{}</dim>",
            child_cwd.display()
        ))
        .fallback_render(term),
    );

    // Full command line
    let cmd_parts: Vec<String> = std::iter::once(binary_path.display().to_string())
        .chain(child_args.iter().map(|a| shell_escape(a)))
        .collect();
    log::message(
        &Prose::new(format!("<bold>Command:</bold> <dim>{}</dim>", cmd_parts.join(" ")))
            .fallback_render(term),
    );

    // Environment changes
    log::message(
        &Prose::new("<bold>Environment Changes:</bold>").fallback_render(term),
    );
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
    let rendered = UnorderedList::from(items)
        .with_bullet("• ")
        .fallback_render(term);
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
        .fallback_render(term),
    )
}

pub(crate) fn post_env_message(message: &str, term: &Terminal) -> String {
    let styled = style_cli_switches(message);
    Prose::new(format!("- {styled}"))
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
        .fallback_render(term)
}

pub(crate) fn post_env_warning_message(message: &str, term: &Terminal) -> String {
    let styled = style_cli_switches(message);
    Prose::new(format!(
        "- <orange><bold>Warning:</bold></orange> {styled}"
    ))
    .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
    .fallback_render(term)
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
