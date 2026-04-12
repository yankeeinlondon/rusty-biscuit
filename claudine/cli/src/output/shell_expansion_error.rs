//! Rich rendering for [`ShellExpansionError`] surfaced during composition.
//!
//! Produces a `Status`-headed, `BlockQuote`-wrapped, darkmatter-highlighted
//! report that points the user directly at the offending directive in the
//! source file (body) or at the offending key in YAML frontmatter.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::Margin;
use claudine::composition::{CompositionError, ShellCommandOrigin, ShellExpansionError};
use color_eyre::eyre::{Report, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

use crate::log;

/// Sentinel message carried on a pre-rendered `Report` so the top-level
/// error handler knows to skip re-logging it.
pub(crate) const PRE_RENDERED_MARKER: &str = "__claudine_pre_rendered__";

/// Number of lines of context to show on either side of the offending line.
const CONTEXT_RADIUS: usize = 1;

/// Left margin for the BlockQuote content.
const LEFT_MARGIN: u32 = 2;

/// Right margin for the BlockQuote content.
const RIGHT_MARGIN: u32 = 2;

/// Border string used by the BlockQuote.
const BORDER: &str = "▌ ";

/// Pretty-print a composition-time shell expansion error.
///
/// Emits the report to the CLI's log stream using `biscuit-terminal` primitives
/// and `darkmatter`'s terminal renderer. The terminal is detected once and
/// used for both the header and the code block.
pub(crate) fn render(source_path: &Path, error: &ShellExpansionError) {
    let term = log::terminal();
    render_with_terminal(source_path, error, &term);
}

/// Map a [`CompositionError`] to a `color_eyre::Report`. Shell expansion
/// errors are rendered directly to stderr and the returned `Report` is
/// marked with [`PRE_RENDERED_MARKER`] so the top-level handler can
/// suppress its own error log. All other variants become a normal
/// `eyre!("{err}")`.
pub(crate) fn pretty_or_report(err: CompositionError) -> Report {
    match err {
        CompositionError::ShellExpansionFailed {
            source_path,
            error,
        } => {
            render(&source_path, error.as_ref());
            eyre!("{}", PRE_RENDERED_MARKER)
        }
        other => eyre!("{other}"),
    }
}

/// Returns `true` when a `color_eyre::Report` was produced by
/// [`pretty_or_report`] and its rich rendering has already been emitted.
pub(crate) fn is_pre_rendered(report: &Report) -> bool {
    report.to_string() == PRE_RENDERED_MARKER
}

/// Map a raw [`darkmatter::markdown::MarkdownError`] to a `color_eyre::Report`
/// for call sites that bypass the composition error layer (e.g. direct
/// `compose_with()` calls in the harness-prompt materializers). Shell
/// expansion errors are pre-rendered; other kinds are wrapped using the
/// supplied `context_label`.
pub(crate) fn pretty_markdown_report(
    source_path: &Path,
    context_label: &str,
    err: darkmatter::markdown::MarkdownError,
) -> Report {
    use darkmatter::markdown::MarkdownError;
    match err {
        MarkdownError::ShellExpansion(shell_err) => {
            render(source_path, &shell_err);
            eyre!("{}", PRE_RENDERED_MARKER)
        }
        other => eyre!("{context_label}: {other}"),
    }
}

fn render_with_terminal(source_path: &Path, error: &ShellExpansionError, term: &Terminal) {
    let report = ShellExpansionReport::build(source_path, error);

    log::message("");
    log::message(&report.header.render(term));
    log::message("");

    if let Some(ref quote) = report.body {
        log::message(&quote.render(term));
        log::message("");
    }

    if let Some(ref hint) = report.hint {
        log::message(&Prose::new(hint.clone()).render(term));
        log::message("");
    }
}

struct ShellExpansionReport {
    header: Status,
    body: Option<BlockQuote>,
    hint: Option<String>,
}

impl ShellExpansionReport {
    fn build(source_path: &Path, error: &ShellExpansionError) -> Self {
        let origin_kind = origin_kind(error);
        let reason = describe_error(error);
        let absolute = canonicalize_or_self(source_path);
        let relative = relative_to_cwd(&absolute);

        let header = build_header(&absolute, &relative, origin_kind, &reason);
        let body = build_body(source_path, error, origin_kind);
        let hint = build_hint(error);

        Self {
            header,
            body,
            hint,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OriginKind {
    Body,
    Frontmatter,
}

fn origin_kind(error: &ShellExpansionError) -> OriginKind {
    match origin_of(error) {
        Some(ShellCommandOrigin::Body { .. }) => OriginKind::Body,
        Some(ShellCommandOrigin::Frontmatter { .. }) => OriginKind::Frontmatter,
        None => OriginKind::Body,
    }
}

/// Extract the origin from any `ShellExpansionError` variant that carries one.
fn origin_of(error: &ShellExpansionError) -> Option<&ShellCommandOrigin> {
    match error {
        ShellExpansionError::ParseDirective { origin, .. }
        | ShellExpansionError::CommandNotFound { origin, .. }
        | ShellExpansionError::Blacklisted { origin, .. }
        | ShellExpansionError::ApprovalRequired { origin, .. }
        | ShellExpansionError::Denied { origin, .. }
        | ShellExpansionError::NotPreApproved { origin, .. }
        | ShellExpansionError::Timeout { origin, .. }
        | ShellExpansionError::ExecutionFailed { origin, .. } => Some(origin),
        ShellExpansionError::PolicyIo { .. } => None,
    }
}

/// Produce a short, human-readable reason that goes into the Status header.
fn describe_error(error: &ShellExpansionError) -> String {
    match error {
        ShellExpansionError::ParseDirective { message, .. } => {
            format!("parse error: {message}")
        }
        ShellExpansionError::CommandNotFound { command, .. } => {
            format!("command not found on PATH: {command}")
        }
        ShellExpansionError::Blacklisted {
            command, reason, ..
        } => {
            format!("command is blacklisted ({reason}): {command}")
        }
        ShellExpansionError::ApprovalRequired { command, .. } => {
            format!("command requires approval but no handler is available: {command}")
        }
        ShellExpansionError::Denied { command, .. } => {
            format!("command was denied during approval: {command}")
        }
        ShellExpansionError::NotPreApproved { command, .. } => {
            format!("command was not pre-approved: {command}")
        }
        ShellExpansionError::Timeout {
            command, timeout, ..
        } => {
            format!("command timed out after {timeout:?}: {command}")
        }
        ShellExpansionError::ExecutionFailed { command, code, .. } => {
            format!("command exited with code {code}: {command}")
        }
        ShellExpansionError::PolicyIo { path, source } => {
            format!("policy I/O error at {}: {source}", path.display())
        }
    }
}

/// Produce a Markdown-wrapped context window that darkmatter will render.
fn build_body(
    source_path: &Path,
    error: &ShellExpansionError,
    kind: OriginKind,
) -> Option<BlockQuote> {
    let source_text = std::fs::read_to_string(source_path).ok()?;

    match kind {
        OriginKind::Body => build_body_body_origin(&source_text, error),
        OriginKind::Frontmatter => build_frontmatter_body(&source_text, error),
    }
}

fn build_body_body_origin(
    source_text: &str,
    error: &ShellExpansionError,
) -> Option<BlockQuote> {
    let body_text = body_content(source_text);
    let body_lines: Vec<&str> = body_text.lines().collect();

    let target_index = find_body_directive_line(&body_lines, error)?;
    let (start, end) = context_window(target_index, body_lines.len(), CONTEXT_RADIUS);
    let window: Vec<&str> = body_lines[start..=end].to_vec();
    let fence = wrap_in_markdown_fence(&window, "md");

    Some(render_fence_into_block(&fence))
}

fn build_frontmatter_body(
    source_text: &str,
    error: &ShellExpansionError,
) -> Option<BlockQuote> {
    let frontmatter_text = frontmatter_content(source_text)?;
    let key = match origin_of(error) {
        Some(ShellCommandOrigin::Frontmatter { key }) => key.as_str(),
        _ => return None,
    };

    let yaml_snippet = extract_frontmatter_entry(frontmatter_text, key)
        .unwrap_or_else(|| frontmatter_text.to_string());
    let fence = wrap_in_markdown_fence_owned(&yaml_snippet, "yaml");

    Some(render_fence_into_block(&fence))
}

/// Render a synthesized `\`\`\`lang ... \`\`\`` block through darkmatter
/// and wrap the rendered output in a red-bordered BlockQuote.
fn render_fence_into_block(fence_source: &str) -> BlockQuote {
    let term = log::terminal();
    let content_width = (term.width() as u16)
        .saturating_sub(visible_width_of_border())
        .saturating_sub(LEFT_MARGIN as u16)
        .saturating_sub(RIGHT_MARGIN as u16);

    let mut opts = TerminalOptions::default();
    opts.max_width = Some(content_width);

    let rendered = match for_terminal(&Markdown::new(fence_source), opts) {
        Ok(text) => trim_trailing_blank_lines(&text),
        Err(_) => fence_source.to_string(),
    };

    let mut block = BlockQuote::new(RenderableContent::from(rendered), None::<&str>)
        .with_left_block_color(Color::Tailwind(Tailwind::Red500))
        .with_border(BORDER);
    block.layout_mut().left_margin = Margin::Chars(LEFT_MARGIN);
    block.layout_mut().right_margin = Margin::Chars(RIGHT_MARGIN);
    block
}

fn visible_width_of_border() -> u16 {
    biscuit_terminal::utils::block_constraint::visible_width(BORDER) as u16
}

fn trim_trailing_blank_lines(rendered: &str) -> String {
    let mut lines: Vec<&str> = rendered.lines().collect();
    while let Some(last) = lines.last() {
        if biscuit_terminal::discovery::eval::strip_ansi_codes(last)
            .trim()
            .is_empty()
        {
            lines.pop();
        } else {
            break;
        }
    }
    lines.join("\n")
}

/// Build the Status header with the "prompt referenced {file} provided an
/// invalid shell expansion command in the {body|frontmatter}" text.
fn build_header(
    absolute: &Path,
    relative: &Path,
    kind: OriginKind,
    reason: &str,
) -> Status {
    let rel_display = prose_escape(&relative.display().to_string());
    let abs_display = absolute.display().to_string();
    let kind_label = match kind {
        OriginKind::Body => "body",
        OriginKind::Frontmatter => "frontmatter",
    };
    let reason_escaped = prose_escape(reason);

    let markup = format!(
        "The prompt being referenced <a href=\"{abs}\"><blue>{rel}</blue></a> provided an invalid \
         shell expansion command in the <b>{kind}</b> of the prompt.\n\n\
         <dim>{reason}</dim>",
        abs = abs_display,
        rel = rel_display,
        kind = kind_label,
        reason = reason_escaped,
    );

    Status::from_prose(markup).state(StatusState::Failure)
}

/// Build a single hint line tailored to the error kind, when useful.
fn build_hint(error: &ShellExpansionError) -> Option<String> {
    match error {
        ShellExpansionError::CommandNotFound { command, .. } => Some(format!(
            "  <dim>Hint: check that <b>{}</b> is on your <blue>PATH</blue>, \
             or fix the directive — quoted strings with spaces are parsed as a \
             single executable name.</dim>",
            prose_escape(command)
        )),
        ShellExpansionError::Blacklisted { .. } => Some(
            "  <dim>Hint: remove or replace this command, or edit the whitelist \
             to allow it.</dim>"
                .to_string(),
        ),
        ShellExpansionError::ApprovalRequired { .. } => Some(
            "  <dim>Hint: run Claudine interactively to approve, or add the \
             command to your whitelist.</dim>"
                .to_string(),
        ),
        ShellExpansionError::Denied { .. } => Some(
            "  <dim>Hint: this command was denied during pre-flight approval.</dim>"
                .to_string(),
        ),
        ShellExpansionError::NotPreApproved { .. } => Some(
            "  <dim>Hint: this is a claudine pre-flight bug — the command was \
             discovered but not approved before execution.</dim>"
                .to_string(),
        ),
        ShellExpansionError::Timeout { .. } => Some(
            "  <dim>Hint: use a <blue>::timeout:<i>N</i></blue> suffix or the \
             <blue>--when-error</blue> directive flag to handle slow commands.</dim>"
                .to_string(),
        ),
        _ => None,
    }
}

/// Find the line within the markdown body that most likely hosts the
/// offending `::shell` directive. Uses the error's `command` field as a
/// substring search anchor.
fn find_body_directive_line(
    body_lines: &[&str],
    error: &ShellExpansionError,
) -> Option<usize> {
    let command = command_of(error)?;

    // Prefer lines that start with `::shell` and contain the command text
    // (either as-is or with quotes around it).
    for (index, line) in body_lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("::shell ") {
            continue;
        }
        if trimmed.contains(command) {
            return Some(index);
        }
    }

    // Fallback: any line that contains the full command string.
    for (index, line) in body_lines.iter().enumerate() {
        if line.contains(command) {
            return Some(index);
        }
    }

    None
}

/// Return a (start, end) inclusive window around `target`.
fn context_window(target: usize, len: usize, radius: usize) -> (usize, usize) {
    let start = target.saturating_sub(radius);
    let end = (target + radius).min(len.saturating_sub(1));
    (start, end)
}

fn wrap_in_markdown_fence(lines: &[&str], language: &str) -> String {
    let mut buf = String::with_capacity(64 + lines.iter().map(|l| l.len() + 1).sum::<usize>());
    buf.push_str("```");
    buf.push_str(language);
    buf.push('\n');
    for line in lines {
        buf.push_str(line);
        buf.push('\n');
    }
    buf.push_str("```\n");
    buf
}

fn wrap_in_markdown_fence_owned(content: &str, language: &str) -> String {
    let trimmed = content.trim_end_matches('\n');
    let mut buf = String::with_capacity(trimmed.len() + 16);
    buf.push_str("```");
    buf.push_str(language);
    buf.push('\n');
    buf.push_str(trimmed);
    buf.push('\n');
    buf.push_str("```\n");
    buf
}

/// Return the command text from an error variant that has one.
fn command_of(error: &ShellExpansionError) -> Option<&str> {
    match error {
        ShellExpansionError::CommandNotFound { command, .. }
        | ShellExpansionError::Blacklisted { command, .. }
        | ShellExpansionError::ApprovalRequired { command, .. }
        | ShellExpansionError::Denied { command, .. }
        | ShellExpansionError::NotPreApproved { command, .. }
        | ShellExpansionError::Timeout { command, .. }
        | ShellExpansionError::ExecutionFailed { command, .. } => Some(command.as_str()),
        ShellExpansionError::ParseDirective { .. } | ShellExpansionError::PolicyIo { .. } => None,
    }
}

/// Return the body content of a markdown file (everything after the closing
/// frontmatter `---`). If the file has no frontmatter, returns the whole
/// file unchanged.
fn body_content(source: &str) -> &str {
    if let Some(rest) = source.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        let after = &rest[end + 4..];
        return after.strip_prefix('\n').unwrap_or(after);
    }
    source
}

/// Return the raw YAML frontmatter text between the opening and closing
/// `---` markers, or `None` if the file has no frontmatter.
fn frontmatter_content(source: &str) -> Option<&str> {
    let rest = source.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Extract a single top-level key (plus its indented continuation lines)
/// from a YAML frontmatter string. Returns `None` when the key is not
/// found.
fn extract_frontmatter_entry(frontmatter: &str, key: &str) -> Option<String> {
    let mut iter = frontmatter.lines().enumerate();
    let key_prefix = format!("{key}:");
    let start = iter.find_map(|(index, line)| {
        if line.starts_with(&key_prefix) {
            Some(index)
        } else {
            None
        }
    })?;

    let lines: Vec<&str> = frontmatter.lines().collect();
    let mut end = start;
    for (offset, line) in lines[start + 1..].iter().enumerate() {
        if line.is_empty() || line.starts_with(|c: char| c.is_whitespace()) {
            end = start + 1 + offset;
        } else {
            break;
        }
    }

    Some(lines[start..=end].join("\n"))
}

/// Best-effort canonicalization. If the path can't be canonicalized we
/// return an absolute version if possible, otherwise the original.
fn canonicalize_or_self(path: &Path) -> PathBuf {
    if let Ok(resolved) = path.canonicalize() {
        return resolved;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if let Ok(cwd) = std::env::current_dir() {
        return cwd.join(path);
    }
    path.to_path_buf()
}

/// Compute a path relative to the current working directory. Falls back to
/// the absolute path if the path cannot be made relative.
fn relative_to_cwd(absolute: &Path) -> PathBuf {
    let Ok(cwd) = std::env::current_dir() else {
        return absolute.to_path_buf();
    };
    match absolute.strip_prefix(&cwd) {
        Ok(stripped) => stripped.to_path_buf(),
        Err(_) => absolute.to_path_buf(),
    }
}

/// Escape characters that would be interpreted as Prose markup.
fn prose_escape(input: &str) -> String {
    input.replace('<', "\\<").replace('>', "\\>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_content_strips_frontmatter() {
        let input = "---\nkey: value\n---\nbody line 1\nbody line 2\n";
        assert_eq!(body_content(input), "body line 1\nbody line 2\n");
    }

    #[test]
    fn body_content_returns_full_when_no_frontmatter() {
        let input = "# heading\n::shell echo hi\n";
        assert_eq!(body_content(input), input);
    }

    #[test]
    fn frontmatter_content_extracts_between_markers() {
        let input = "---\nname: world\nother: 42\n---\nbody\n";
        assert_eq!(frontmatter_content(input), Some("name: world\nother: 42"));
    }

    #[test]
    fn frontmatter_content_none_for_plain_markdown() {
        let input = "# heading\n";
        assert!(frontmatter_content(input).is_none());
    }

    #[test]
    fn extract_frontmatter_entry_single_line_value() {
        let fm = "name: world\ncmd: echo hi\nother: 42";
        assert_eq!(
            extract_frontmatter_entry(fm, "cmd").as_deref(),
            Some("cmd: echo hi"),
        );
    }

    #[test]
    fn extract_frontmatter_entry_multiline_value() {
        let fm = "doc: |\n  first line\n  second line\nother: 42";
        assert_eq!(
            extract_frontmatter_entry(fm, "doc").as_deref(),
            Some("doc: |\n  first line\n  second line"),
        );
    }

    #[test]
    fn extract_frontmatter_entry_missing_key() {
        let fm = "name: world";
        assert!(extract_frontmatter_entry(fm, "missing").is_none());
    }

    #[test]
    fn find_body_directive_line_by_command_substring() {
        let body = "line 0\n::shell echo hello\nline 2\n::shell just commit\nline 4\n";
        let body_lines: Vec<&str> = body.lines().collect();
        let error = ShellExpansionError::CommandNotFound {
            command: "just commit".to_string(),
            origin: ShellCommandOrigin::Body { line: 0 },
        };
        assert_eq!(find_body_directive_line(&body_lines, &error), Some(3));
    }

    #[test]
    fn find_body_directive_line_by_quoted_command() {
        let body = "line 0\n::shell \"just commit\"\nline 2\n";
        let body_lines: Vec<&str> = body.lines().collect();
        let error = ShellExpansionError::CommandNotFound {
            command: "just commit".to_string(),
            origin: ShellCommandOrigin::Body { line: 0 },
        };
        assert_eq!(find_body_directive_line(&body_lines, &error), Some(1));
    }

    #[test]
    fn context_window_midfile() {
        assert_eq!(context_window(5, 10, 1), (4, 6));
    }

    #[test]
    fn context_window_first_line() {
        assert_eq!(context_window(0, 10, 1), (0, 1));
    }

    #[test]
    fn context_window_last_line() {
        assert_eq!(context_window(9, 10, 1), (8, 9));
    }

    #[test]
    fn wrap_in_markdown_fence_adds_language_and_newlines() {
        let fenced = wrap_in_markdown_fence(&["a", "b"], "md");
        assert_eq!(fenced, "```md\na\nb\n```\n");
    }

    #[test]
    fn prose_escape_escapes_angle_brackets() {
        assert_eq!(prose_escape("A<b>c"), "A\\<b\\>c");
    }

    #[test]
    fn describe_command_not_found_is_human_readable() {
        let error = ShellExpansionError::CommandNotFound {
            command: "just commit".to_string(),
            origin: ShellCommandOrigin::Body { line: 69 },
        };
        let reason = describe_error(&error);
        assert!(reason.contains("just commit"));
        assert!(reason.contains("PATH"));
    }
}
