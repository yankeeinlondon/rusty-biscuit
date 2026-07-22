//! The system prompt render component.
//!
//! Provides header rendering, summary view with hyperlinks and token counts,
//! and body rendering (partial/full) inside an orange block quote.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use crate::system_prompt::{
    PreparedSystemPrompt, ResolvedSystemPrompt, SystemPromptMode, SystemPromptSource,
};

use super::formatting::{prompt_body_width, render_markdown_for_terminal, system_prompt_blockquote_styled};
use super::tokens::estimate_system_prompt_tokens;
use super::truncation::{truncate_front_back, truncate_head};
use super::{ReportMode, TruncationMode};

/// Nerd Font git icon used as the in-repo hyperlink label when the terminal
/// supports Nerd Fonts. Stands in for the repo root path.
const NERD_FONT_REPO_GLYPH: char = '\u{F02A2}';

/// Render the system-prompt header line. `action` is `appended` or `replaced`.
fn render_system_prompt_header(action: &str, term: &Terminal) -> String {
    Prose::new(format!(
        "\n<orange-500><b>■ System Prompt (<i>{action}</i>)</b></orange-500>"
    ))
    .render(term)
}

/// Resolve the visible label for a prompt-file hyperlink. Returns plain
/// text (no markup); callers apply styling and OSC8 wrapping.
///
/// The form depends on terminal capability and whether `absolute` lies
/// inside `base`: Nerd Font terminals with an in-base path get
/// [`NERD_FONT_REPO_GLYPH`] joined to the relative path; non-Nerd-Font
/// terminals with an in-base path get a `./`-prefixed relative path;
/// otherwise the absolute path is returned unchanged.
fn resolve_display_label(
    absolute: &Path,
    base: Option<&Path>,
    term: &Terminal,
) -> String {
    let rel = base.and_then(|b| absolute.strip_prefix(b).ok());
    match (rel, term.is_nerd_font) {
        (Some(rel), Some(true)) => format!("{NERD_FONT_REPO_GLYPH}/{}", rel.display()),
        (Some(rel), _) => format!("./{}", rel.display()),
        (None, _) => absolute.display().to_string(),
    }
}

/// Render the summary view for a system prompt as a single prose sentence
/// of the form: `The system prompt was **{action}**; the content was
/// _composed_ from <hyperlink>. {token-message}`.
///
/// `base_path` is the optional directory used to compute the relative path
/// displayed in the hyperlink label; when `None`, the absolute path is
/// shown verbatim.
fn render_system_prompt_summary(
    source: &SystemPromptSource,
    mode: SystemPromptMode,
    token_count: u64,
    base_path: Option<&Path>,
    term: &Terminal,
) -> String {
    let (action_phrase, token_message) = match mode {
        SystemPromptMode::Append => (
            "appended to",
            format!("The composed system prompt is roughly {token_count} tokens."),
        ),
        SystemPromptMode::Replace => (
            "replaced",
            format!("The replacement system prompt is roughly {token_count} tokens."),
        ),
    };

    let source_clause = match source {
        SystemPromptSource::StandardDiscovered { path, .. }
        | SystemPromptSource::ExplicitFile { path, .. }
        | SystemPromptSource::NonInteractiveFile { path, .. } => {
            let absolute: PathBuf = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let label = resolve_display_label(&absolute, base_path, term);
            let href = format!("file://{}", absolute.display());
            // Prose handles the OSC8 emission + plain-text fallback for us;
            // `<blue-400>` styles the visible label so a reader sees it as a
            // link even when OSC8 is unsupported.
            format!("the content was _composed_ from <blue-400>[{label}]({href})</blue-400>")
        }
        SystemPromptSource::BuiltInNonInteractive => {
            "the content was _composed_ from the <dim>built-in</dim> non-interactive prompt"
                .to_string()
        }
    };

    Prose::new(format!(
        "The system prompt was **{action_phrase}**; {source_clause}. {token_message}"
    ))
    .render(term)
}

/// Render the system-prompt body content as ANSI text (no `BlockQuote`
/// wrapper), ready to be concatenated with the rendered summary and
/// wrapped in a single [`system_prompt_blockquote_styled`].
///
/// `Summary` returns the empty string; `Partial` truncates per the
/// embedded `TruncationMode`; `Full` renders the full text.
fn render_system_prompt_body(
    text: &str,
    mode: ReportMode,
    term: &Terminal,
) -> String {
    let width = prompt_body_width(term);
    match mode {
        ReportMode::Summary => String::new(),
        ReportMode::Partial { truncation } => {
            // Render first, then truncate the rendered rows — see the note in
            // `agent::render_user_prompt_body`. Truncating Markdown source
            // by line can orphan indented content into a phantom code block.
            let rendered = render_markdown_for_terminal(text, term, width);
            match truncation {
                TruncationMode::FrontBack => truncate_front_back(&rendered, 20, 10),
                TruncationMode::Truncate => truncate_head(&rendered, 20),
            }
        }
        ReportMode::Full => render_markdown_for_terminal(text, term, width),
        ReportMode::Silent => String::new(),
    }
}

/// The system-prompt render component.
///
/// Suppression is decided at construction: [`SystemPrompt::from_mode`]
/// returns `None` when the report should not appear, so a constructed value
/// always produces output. Sink concerns (TTY detection, writer choice) stay
/// with the caller.
#[derive(Debug)]
pub struct SystemPrompt {
    resolved: ResolvedSystemPrompt,
    mode: ReportMode,
    base: Option<PathBuf>,
    layout: Layout,
}

impl SystemPrompt {
    /// Build the component, or `None` when the report is suppressed
    /// (`Silent` mode, or `ResolvedSystemPrompt::{None, Disabled}` in any
    /// mode below `Full`).
    pub fn from_mode(
        resolved: &ResolvedSystemPrompt,
        mode: ReportMode,
        base: Option<&Path>,
    ) -> Option<Self> {
        if matches!(mode, ReportMode::Silent) {
            return None;
        }
        if !matches!(resolved, ResolvedSystemPrompt::Ready(_))
            && !matches!(mode, ReportMode::Full)
        {
            return None;
        }

        Some(Self {
            resolved: resolved.clone(),
            mode,
            base: base.map(Path::to_path_buf),
            layout: Layout::default(),
        })
    }

    fn render_ready(&self, prepared: &PreparedSystemPrompt, term: &Terminal,
    ) -> String {
        let action = match prepared.mode {
            SystemPromptMode::Append => "appended",
            SystemPromptMode::Replace => "replaced",
        };

        // Header line is always emitted on its own (above the BlockQuote).
        let header = render_system_prompt_header(action, term);

        // Compose summary + body into one rendered string; both go into a
        // single orange BlockQuote so the bar runs continuously beneath the
        // icon.
        let mut body_parts: Vec<String> = Vec::new();

        let tokens = estimate_system_prompt_tokens(
            &prepared.composed_markdown,
            prepared
                .non_interactive_appendix
                .as_ref()
                .map(|a| a.composed_markdown.as_str()),
        );
        body_parts.push(render_system_prompt_summary(
            &prepared.source,
            prepared.mode,
            tokens,
            self.base.as_deref(),
            term,
        ));

        if !matches!(self.mode, ReportMode::Summary) {
            let body = render_system_prompt_body(
                &prepared.composed_markdown,
                self.mode,
                term,
            );
            if !body.is_empty() {
                body_parts.push(body);
            }
        }

        if body_parts.is_empty() {
            return header;
        }

        let combined = body_parts.join("\n\n");
        let quote = system_prompt_blockquote_styled(&combined).render(term);
        format!("{header}\n{quote}")
    }

    /// Render the `None`/`Disabled` placeholder report. Only reachable in
    /// `Full` mode — `from_mode` suppresses these variants below `Full`.
    fn render_empty(
        &self,
        action: &str,
        body_text: &str,
        term: &Terminal,
    ) -> String {
        let header = render_system_prompt_header(action, term);
        let body = render_markdown_for_terminal(body_text, term, prompt_body_width(term));
        let quote = system_prompt_blockquote_styled(&body).render(term);
        format!("{header}\n{quote}")
    }
}

impl TerminalRenderable for SystemPrompt {
    fn render(&self, term: &Terminal) -> String {
        match &self.resolved {
            ResolvedSystemPrompt::Ready(prepared) => self.render_ready(prepared, term),
            ResolvedSystemPrompt::None => {
                self.render_empty("none", "the system prompt has not been modified", term)
            }
            ResolvedSystemPrompt::Disabled { .. } => {
                self.render_empty("disabled", "the system prompt has been disabled", term)
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

#[cfg(test)]
mod tests;
