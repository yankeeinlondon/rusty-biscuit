pub(crate) mod api_errors;
pub(crate) mod assistant;
pub(crate) mod error_report;
pub(crate) mod error_walker;
pub(crate) mod switches;

pub(crate) use api_errors::try_format_api_error;
pub(crate) use assistant::emit_final_message;
pub(crate) use switches::{style_cli_switches, truncate_args};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
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
use crate::commands::wrap::profile::{WrapperProfile, profile_for_provider};
use crate::log;

/// Context for compose/inline-compose mode display in the header.
#[allow(dead_code)]
pub(crate) enum ComposeDisplay {
    /// Chained composition (`claudine compose`).
    Compose,
    /// Inline composition (`claudine inline-compose`).
    InlineCompose,
}

#[allow(dead_code)]
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

fn render_operation_badge(operation: &str, term: &Terminal) -> String {
    let open = "<bg-green-900><green-100><bold>";
    let close = "</bold></green-100></bg-green-900>";

    // Each segment owns the background because closing nested emphasis resets it.
    Prose::new(format!(
        "{open} Op({close}{open}<dim><i>{operation}</i></dim>{close}{open}) {close}"
    ))
    .render(term)
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
        header_parts.push(render_operation_badge(op, term));
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

    let rendered = header_parts.join(" ");
    let rendered = if matches!(
        term.color_depth,
        biscuit_terminal::discovery::detection::ColorDepth::None
    ) {
        strip_ansi_codes(&rendered)
    } else {
        rendered
    };
    log::message(&format!("\n{rendered}\n"));
}

/// Render the composed prompt as a BlockQuote after environment details.
///
/// The prompt is rendered as Markdown via Darkmatter (for proper wrapping,
/// bold, links, code, etc.) and then wrapped in a green-bordered BlockQuote.
///
/// In verbose mode the entire prompt is shown. Otherwise the first 10 lines
/// are rendered with a truncation notice.
pub(crate) fn log_compose_prompt(
    prompt: &str,
    verbose: bool,
    silent: bool,
    _quiet: bool,
    term: &Terminal,
) {
    use claudine::render::{AgentPrompt, resolve_agent_prompt_report_mode};

    if silent {
        return;
    }

    let mode = resolve_agent_prompt_report_mode(silent, verbose, prompt.lines().count());
    if let Some(report) = AgentPrompt::from_mode(prompt, mode) {
        log::message(&report.render(term));
    }
}

#[allow(dead_code)]
pub(crate) fn log_system_prompt(
    effective_sp: &claudine::system_prompt::ResolvedSystemPrompt,
    verbose: bool,
    silent: bool,
    quiet: bool,
    term: &Terminal,
) {
    log_system_prompt_with_scope(effective_sp, verbose, silent, quiet, None, term)
}

pub(crate) fn log_system_prompt_with_scope(
    effective_sp: &claudine::system_prompt::ResolvedSystemPrompt,
    verbose: bool,
    silent: bool,
    quiet: bool,
    scope: Option<&Path>,
    term: &Terminal,
) {
    use claudine::render::{
        ReportMode, SystemPrompt, parse_frontmatter_verbosity,
        resolve_system_prompt_report_mode,
    };
    use claudine::system_prompt::check_and_record;

    if silent {
        return;
    }

    let env_verbosity = std::env::var("CLAUDINE_SYSTEM_PROMPT")
        .ok()
        .as_deref()
        .and_then(ReportMode::parse);

    let (line_count, frontmatter_verbosity, unchanged) = match effective_sp {
        claudine::system_prompt::ResolvedSystemPrompt::Ready(prepared) => {
            let frontmatter = parse_frontmatter_verbosity(&prepared.raw_text);
            let unchanged = scope
                .map(|s| {
                    check_and_record(
                        s,
                        &prepared.composed_markdown,
                        prepared
                            .non_interactive_appendix
                            .as_ref()
                            .map(|a| a.composed_markdown.as_str()),
                    )
                })
                .unwrap_or(false);
            (
                prepared.composed_markdown.lines().count(),
                frontmatter,
                unchanged,
            )
        }
        _ => (0, None, false),
    };

    let mode = resolve_system_prompt_report_mode(
        silent,
        quiet,
        verbose,
        env_verbosity,
        line_count,
        frontmatter_verbosity,
        unchanged,
    );

    if let Some(report) = SystemPrompt::from_mode(effective_sp, mode, scope) {
        log::message(&report.render(term));
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

        let mut items: Vec<RenderableTerminalContent> = Vec::new();
        for removed in &env_plan.removed {
            items.push(RenderableTerminalContent::from(Prose::new(format!(
                "<red><strikethrough>{removed}</strikethrough></red>"
            ))));
        }

        for included in &env_plan.included {
            items.push(RenderableTerminalContent::from(Prose::new(format!(
                "<orange>{included}</orange>"
            ))));
        }

        for (key, value) in &env_plan.added {
            items.push(RenderableTerminalContent::from(Prose::new(format!(
                "<green>{key}</green><dim>={}</dim>",
                summarize_value(key, value)
            ))));
        }

        if items.is_empty() {
            items.push(RenderableTerminalContent::from(Prose::new(
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
            biscuit_file::to_portable_string(child_cwd)
        ))
        .render(term),
    );

    // Full command line
    let cmd_parts: Vec<String> = std::iter::once(biscuit_file::to_portable_string(binary_path))
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
    let mut items: Vec<RenderableTerminalContent> = Vec::new();
    for removed in &env_plan.removed {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<red><strikethrough>{removed}</strikethrough></red>"
        ))));
    }
    for included in &env_plan.included {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<orange>{included}</orange>"
        ))));
    }
    for (key, value) in &env_plan.added {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>{key}</green><dim>={value}</dim>"
        ))));
    }
    if items.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(
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
        let items: Vec<RenderableTerminalContent> = lines
            .iter()
            .map(|l| RenderableTerminalContent::from(Prose::new(format!("<dim>{l}</dim>"))))
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
            biscuit_file::to_portable_string(path)
        )
    } else {
        String::new()
    };

    Prose::new(format!(
        "- <blue><bold>Info:</bold></blue> the {} was used; this constrains skills, commands, and subagent definitions to those in the repo.{}",
        *claudine::badges::REPO_FLAG,
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

    let mut items: Vec<RenderableTerminalContent> = Vec::new();
    if mcp_runtime.servers.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(
            "<dim>no active MCP servers</dim>",
        )));
    } else {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>servers</green><dim>={}</dim>",
            mcp_runtime.servers.join(", ")
        ))));
    }

    if !mcp_runtime.default_servers.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>defaults</green><dim>={}</dim>",
            mcp_runtime.default_servers.join(", ")
        ))));
    }
    if !mcp_runtime.explicit_servers.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>use</green><dim>={}</dim>",
            mcp_runtime.explicit_servers.join(", ")
        ))));
    }
    if !mcp_runtime.tag_servers.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>tag_servers</green><dim>={}</dim>",
            mcp_runtime.tag_servers.join(", ")
        ))));
    }

    if !mcp_runtime.resolved_tags.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>tags</green><dim>={}</dim>",
            mcp_runtime.resolved_tags.join(", ")
        ))));
    }
    if !mcp_runtime.missing_tags.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<orange>missing_tags</orange><dim>={}</dim>",
            mcp_runtime.missing_tags.join(", ")
        ))));
    }
    if !mcp_runtime.ambiguous_tags.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<orange>ambiguous_tags</orange><dim>={}</dim>",
            mcp_runtime.ambiguous_tags.join(", ")
        ))));
    }
    if let Some(cleaned_prompt) = &mcp_runtime.cleaned_prompt {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>cleaned_prompt</green><dim>={}</dim>",
            shell_escape(cleaned_prompt)
        ))));
    }
    if !mcp_runtime.env_vars_set.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>env</green><dim>={}</dim>",
            mcp_runtime.env_vars_set.join(", ")
        ))));
    }
    if !mcp_runtime.extra_args.is_empty() {
        items.push(RenderableTerminalContent::from(Prose::new(format!(
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
        items.push(RenderableTerminalContent::from(Prose::new(format!(
            "<green>files</green><dim>={}</dim>",
            mcp_runtime
                .temp_files
                .iter()
                .map(|path| biscuit_file::to_portable_string(path))
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

/// Process-scoped flag set by the loop's `SIGINT` handler when the user
/// presses Ctrl+C. Rendering surfaces consult this to relabel any
/// post-interrupt agent error as a user-action block instead of a red
/// `Agent Error`.
static USER_INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Mark that a user interrupt was observed in this process.
///
/// Also raises the lib-side flag in [`claudine::interrupt`] so blocking
/// post-execute work (lifecycle messenger sends, TTS playback, sound
/// effects) short-circuits on the first Ctrl+C instead of after several.
///
/// # Signal-handler safety
///
/// This function is called from the `SIGINT` handler installed by
/// `commands::wrap::interrupt`. It must remain async-signal-safe: it
/// performs only atomic stores and calls [`claudine::interrupt::mark_interrupted`],
/// which is also a pure atomic store. No `OnceLock`, `Mutex`, allocation,
/// or non-reentrant libc calls are introduced on the store path.
pub(crate) fn mark_user_interrupted() {
    USER_INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst);
    claudine::interrupt::mark_interrupted();
}

/// Returns `true` once a Ctrl+C has been observed in this process.
pub(crate) fn user_interrupt_observed() -> bool {
    USER_INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Nesting depth of the active child-process wait loops.
///
/// A child wait loop (`wait_with_signal_*` in `wrap::exec`) installs its own
/// `SIGINT` handler that drives the child-targeted `SIGINT → SIGTERM →
/// SIGKILL` escalation ladder. While that ladder is in charge, the
/// compose-scoped Ctrl+C guard must **not** abruptly `_exit` the wrapper out
/// from under it. Outside that window — prep, between loop iterations, and
/// post-execute lifecycle side effects (TTS, sound) — there is no ladder, so a
/// repeated press must be able to force-exit a wedged synchronous call.
///
/// A counter (not a bool) keeps the flag correct if wait loops ever nest.
static WAIT_LOOP_DEPTH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Returns `true` while at least one child-process wait loop is blocking with
/// its own `SIGINT` handler installed. Read from the compose guard's signal
/// handler — a single atomic load, async-signal-safe.
pub(crate) fn wait_loop_active() -> bool {
    WAIT_LOOP_DEPTH.load(std::sync::atomic::Ordering::SeqCst) > 0
}

/// RAII guard that marks a child wait loop active for its lifetime.
///
/// Construct it at the top of each `wait_with_signal_*` function; the
/// decrement on `Drop` fires on every exit path (including `?` early returns),
/// so the flag can never get stuck "active" after the wait returns.
pub(crate) struct WaitLoopActiveGuard;

impl WaitLoopActiveGuard {
    pub(crate) fn new() -> Self {
        WAIT_LOOP_DEPTH.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self
    }
}

impl Drop for WaitLoopActiveGuard {
    fn drop(&mut self) {
        WAIT_LOOP_DEPTH.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Reset the user-interrupt flag. Used only by tests that need a clean
/// observable state across assertions.
///
/// Clears both the CLI-local flag set by [`mark_user_interrupted`] **and**
/// the lib-side flag in [`claudine::interrupt`] so subsequent in-process
/// tests that observe lifecycle side effects through `claudine::interrupt`
/// see a clean slate.
#[cfg(test)]
pub(crate) fn clear_user_interrupt_for_tests() {
    USER_INTERRUPTED.store(false, std::sync::atomic::Ordering::SeqCst);
    claudine::interrupt::clear_for_tests();
}

/// Format an INFO-only line announcing the working directory used to launch the agent.
pub(crate) fn format_launch_directory(directory: &Path) -> String {
    Prose::new(format!(
        "- <dim>starting agent in</dim> <blue>{}</blue>",
        biscuit_file::to_portable_string(directory)
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

/// Render the one-line execution header for a composition run.
///
/// Shared by the up-front emit in `compose` / `inline-compose` (which
/// resolves the agent eagerly so the line appears immediately) and the
/// in-pipeline emit for callers that did not pre-render it.
///
/// Returns `false` without emitting when `provider` has no wrapper
/// profile, so the caller leaves the header to the executor rather than
/// silently dropping it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_execution_header(
    provider: Provider,
    yolo: bool,
    session_interactive: bool,
    detail_requested: bool,
    repo: bool,
    is_inline: bool,
    sequence: bool,
    operation: Option<&str>,
    file_ref: &str,
    package_context: Option<claudine::composition::PackageContext>,
    term: &Terminal,
) -> bool {
    let Some(profile) = profile_for_provider(provider) else {
        return false;
    };
    let compose_display = if is_inline {
        ComposeDisplay::InlineCompose
    } else {
        ComposeDisplay::Compose
    };
    let header_env_plan = EnvPlan {
        package_context,
        ..Default::default()
    };
    log_wrapper_header(
        profile,
        yolo,
        !session_interactive,
        session_interactive,
        detail_requested,
        repo,
        Some(&compose_display),
        sequence,
        operation,
        None, // no inline prompt text for compose
        Some(file_ref),
        &header_env_plan,
        term,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_loop_active_guard_tracks_nesting_depth() {
        // Outside any wait loop the compose Ctrl+C guard is free to force-exit.
        assert!(!wait_loop_active(), "must start inactive");

        {
            let _outer = WaitLoopActiveGuard::new();
            assert!(wait_loop_active(), "active while a guard is held");

            {
                let _inner = WaitLoopActiveGuard::new();
                assert!(wait_loop_active(), "still active while nested");
            }
            // Inner drop must not clear the flag while the outer guard lives —
            // a bool flag would; the depth counter must not.
            assert!(
                wait_loop_active(),
                "must remain active until the outermost guard drops"
            );
        }

        assert!(
            !wait_loop_active(),
            "must be inactive again once all guards drop"
        );
    }

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

    #[test]
    fn operation_badge_reapplies_background_before_closing_parenthesis() {
        let rendered = render_operation_badge("commit", &Terminal::new_optimistic(80));
        let closing_parenthesis = rendered
            .find(')')
            .expect("operation badge should contain a closing parenthesis");
        let before_closing_parenthesis = &rendered[..closing_parenthesis];
        let last_background = before_closing_parenthesis
            .rfind("\x1b[48;2;")
            .expect("operation badge should render a true-color background");
        let last_reset = before_closing_parenthesis
            .rfind("\x1b[0m")
            .expect("nested operation styling should emit a reset");

        assert!(
            last_background > last_reset,
            "badge background must be active for the closing parenthesis: {rendered:?}"
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
