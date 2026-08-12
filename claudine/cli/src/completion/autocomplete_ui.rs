//! Shared presentation components for ENTER-path autocomplete.
//!
//! Phase 1 of the `2026-06-14-auto-complete` feature. Provides:
//!
//! - [`render_file_detail_prose`] — the badge/name/description/schema/path
//!   detail block consumed by both the confirmation dialog and the chooser
//!   detail pane.
//! - [`render_confirmation_dialog`] — the full `Use this file? (Y/n)` dialog
//!   as a [`Prose`] value.
//! - [`confirm_one_file`] — drives the confirmation dialog and returns
//!   `true` for Y/Enter, `false` for n/Esc.
//! - [`choose_one_file`] / [`choose_many_files`] — two-pane choosers with a
//!   live detail pane, split horizontally (detail right) in wide terminals
//!   and vertically (detail above) in narrow ones per [`stacks_vertically`].

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;
use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::RenderableTerminalContent;
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_tui::core::{SplitDirection, SplitPane, SplitRatio};
use biscuit_tui::prelude::*;
use claudine::composition::FileDetail;
use darkmatter::markdown::CodeBlock;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{Paragraph, StatefulWidget, Widget},
};
use url::Url;

/// Render the detail block for a candidate file.
///
/// Layout per spec: `<badge> <name-or-path-line>`, blank line, blockquoted
/// description, a `Schema:` header, and the `$schema` block rendered as a
/// syntax-highlighted YAML code block, plus an inline OSC8 path link when the
/// absolute path can be represented as a file URL. A declined URL renders as
/// plain path text. Built from [`Compose`], [`BlockQuote`], [`CodeBlock`], and
/// [`Prose`] components rather than a hand-rolled markup string.
///
/// The `$schema` code block is pre-rendered against `term` (rather than left
/// as a lazy component) so darkmatter's syntax highlighter runs — the Compose
/// tree path has no code renderer wired and would otherwise emit a plain,
/// unhighlighted fence. Callers must therefore render the returned [`Compose`]
/// at the same width `term` carries so the code block's wrapping stays aligned.
pub fn render_file_detail_prose(detail: &FileDetail, term: &Terminal) -> Compose {
    let parts: Vec<RenderableTerminalContent> = vec![
        RenderableTerminalContent::from(name_line_prose(detail)),
        RenderableTerminalContent::from("\n\n"),
        RenderableTerminalContent::from(BlockQuote::from(description_prose(detail))),
        RenderableTerminalContent::from("\n\nSchema:\n\n"),
        schema_content(detail, term),
    ];
    Compose::from(parts)
}

/// Render the full confirmation dialog including the `Use this file? (Y/n)`
/// trailer.
pub fn render_confirmation_dialog(detail: &FileDetail, term: &Terminal) -> Compose {
    let mut compose = render_file_detail_prose(detail, term);
    compose.add_text("\n\nUse this file? (Y/n)");
    compose
}

fn name_line_prose(detail: &FileDetail) -> Prose {
    let badge = badge_markup(&detail.badge);
    let label = path_label(&detail.path);
    let escaped_label = Prose::escape_text(&label);
    let linked_label = file_href(&detail.path).map_or_else(
        || escaped_label.clone(),
        |href| format!("<a href={}>{escaped_label}</a>", Prose::quoted_attr(&href)),
    );

    let body = if detail.has_custom_name {
        let name = Prose::escape_text(&detail.name);
        format!("{badge} <bold>{name}</bold> (<dim><blue>{linked_label}</blue></dim>)",)
    } else {
        format!("{badge} {linked_label}")
    };

    Prose::new(body)
}

fn description_prose(detail: &FileDetail) -> Prose {
    let text = detail
        .description
        .as_deref()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or("no description");
    // Render the authored description through Prose's markdown subset so
    // inline emphasis (`_feature_` → italic) renders; escaping it here would
    // print the markup literally, which is Bug 1.
    Prose::new(text)
}

/// Build the schema section: a syntax-highlighted YAML code block of the
/// `$schema` properties, or the dim-italic placeholder when none are declared.
///
/// The code block is pre-rendered against `term` and returned as a verbatim
/// string part — see [`render_file_detail_prose`] for why the Compose tree
/// path cannot highlight it lazily.
fn schema_content(detail: &FileDetail, term: &Terminal) -> RenderableTerminalContent {
    if detail.schema_lines.is_empty() {
        RenderableTerminalContent::from(Prose::new("<dim><i>no schema defined</i></dim>"))
    } else {
        let yaml = detail.schema_lines.join("\n");
        RenderableTerminalContent::from(CodeBlock::yaml(yaml).render(term))
    }
}

/// Drive a single-file confirmation dialog.
///
/// Returns `Ok(true)` for Y/Enter, `Ok(false)` for n/Esc. Ctrl+C is left
/// to the caller's SIGINT handler and will surface as an I/O error if
/// raw-mode reading is interrupted.
pub fn confirm_one_file(detail: &FileDetail) -> io::Result<bool> {
    let term = crate::log::terminal();
    let prose = render_confirmation_dialog(detail, &term);
    eprintln!("{}", prose.render(&term));
    io::stderr().flush()?;

    crossterm::terminal::enable_raw_mode()?;
    let result = read_confirm_key();
    // Always attempt cleanup; preserve the original error if one occurred.
    let _ = crossterm::terminal::disable_raw_mode();
    result
}

fn read_confirm_key() -> io::Result<bool> {
    loop {
        if let Event::Key(key) = crossterm::event::read()? {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                _ => {}
            }
        }
    }
}

/// Drive a two-pane single-select chooser and return the selected file
/// detail, or `None` if the user cancelled.
pub fn choose_one_file(options: Vec<ChoiceOption<FileDetail>>) -> io::Result<Option<FileDetail>> {
    let height = Some(chooser_height(&options));
    let state = ChooseOneState::from_options(options);
    run_standalone(FileChooser, state, height)
}

/// Drive a two-pane multi-select chooser and return the selected file
/// details, or an error if the user cancelled.
pub fn choose_many_files(options: Vec<ChoiceOption<FileDetail>>) -> io::Result<Vec<FileDetail>> {
    let height = Some(chooser_height(&options));
    let state = ChooseManyState::from_options(options);
    run_standalone(MultiFileChooser, state, height)
}

/// Inline viewport height for the two-pane file chooser.
///
/// Passing `None` to [`run_standalone`] runs the prompt fullscreen (alternate
/// screen), which is what made the chooser consume the whole terminal. Instead
/// the chooser claims only the rows it needs: one per option for the list pane,
/// floored by a detail-pane reservation so the side-by-side detail pane
/// (badge/name, description, and the `$schema` YAML code block) is not
/// vertically clipped, and bounded by [`MAX_LIST_ROWS`] so a large candidate
/// set scrolls rather than filling the screen. One extra row hosts the
/// help-hint overlay. `HeightSpec::Cells` further clamps the result to the live
/// terminal height.
///
/// The detail floor is content-aware: the YAML code block adds one row per
/// `$schema` property plus a fixed chrome allowance (header, fences, name,
/// blank, and blockquoted description), so a schema with more properties
/// reserves more rows instead of clipping the code block — the concrete Bug 3
/// failure mode.
fn chooser_height(options: &[ChoiceOption<FileDetail>]) -> HeightSpec {
    let list = list_rows(options);
    let detail = detail_rows(options);
    // Side-by-side panes share rows, so a horizontal split reserves only the
    // taller of the two; a stacked (vertical) split draws the detail pane
    // above the list, so it needs the SUM or the lower pane clips. The
    // direction is decided from the terminal width — the same signal the
    // render split uses — so the reservation always matches the layout.
    let content_rows = if stacks_vertically(terminal_width()) {
        list.saturating_add(detail)
    } else {
        list.max(detail)
    };
    HeightSpec::Cells(content_rows.saturating_add(1))
}

/// Absolute floor that keeps a minimal detail pane readable.
const MIN_DETAIL_ROWS: u16 = 12;
/// Rows the detail pane needs beyond the schema lines themselves: name,
/// blank, blockquoted description (up to two wrapped lines), `Schema:`
/// header, blank, the code block's top/bottom fences and header row, plus the
/// bottom help-hint row.
const DETAIL_CHROME_ROWS: u16 = 11;
/// Ceiling on the list pane before it scrolls.
const MAX_LIST_ROWS: u16 = 20;
/// Minimum columns each pane needs to stay readable in a side-by-side split.
/// A terminal narrower than two of these can't show two useful panes at once,
/// so the chooser stacks the detail pane above the list instead.
const MIN_PANE_WIDTH: u16 = 30;

/// List-pane row count: one per candidate, capped by [`MAX_LIST_ROWS`] so a
/// large set scrolls rather than filling the screen.
fn list_rows(options: &[ChoiceOption<FileDetail>]) -> u16 {
    options.len().min(MAX_LIST_ROWS as usize) as u16
}

/// Detail-pane row count: the widest candidate's `$schema` code block plus a
/// fixed chrome allowance, floored by [`MIN_DETAIL_ROWS`] so a schemaless
/// file still renders a readable pane. Content-aware so a larger schema
/// reserves more rows instead of clipping the code block.
fn detail_rows(options: &[ChoiceOption<FileDetail>]) -> u16 {
    let max_schema_lines = options
        .iter()
        .map(|option| option.value.schema_lines.len())
        .max()
        .unwrap_or(0)
        .min(u16::MAX as usize) as u16;
    max_schema_lines
        .saturating_add(DETAIL_CHROME_ROWS)
        .max(MIN_DETAIL_ROWS)
}

/// Whether the chooser stacks the detail pane above the list (vertical split)
/// rather than beside it (horizontal split) at `term_width`.
///
/// This is the single source of truth shared by [`chooser_height`]'s row
/// reservation and the render-time [`split_panes`], so the rows claimed and
/// the layout drawn never disagree. The decision keys off the terminal
/// *window* width (a real constraint), not the inline viewport's aspect —
/// the viewport is deliberately only a dozen-odd rows tall, so its own aspect
/// would always read as "wide" and force horizontal.
fn stacks_vertically(term_width: u16) -> bool {
    term_width < MIN_PANE_WIDTH.saturating_mul(2)
}

/// Live terminal width in columns, or a wide fallback (which prefers the
/// side-by-side layout) when the size is unavailable — e.g. not a TTY, where
/// the chooser does not run interactively anyway.
fn terminal_width() -> u16 {
    crossterm::terminal::size()
        .map(|(cols, _)| cols)
        .unwrap_or(u16::MAX)
}

/// Split the inline chooser area into `(list_rect, detail_rect)`.
///
/// A terminal wide enough for two readable panes splits horizontally (list
/// left, detail right); a narrow terminal stacks the detail pane on top of
/// the list. When stacked, the list keeps its full height on the bottom (so
/// every candidate stays visible rather than scrolling) and the detail pane
/// takes the remaining rows above it — [`chooser_height`] reserved the sum of
/// both, so the detail pane still gets its content height. The direction keys
/// off `area.width`, which for an inline viewport is the live terminal width,
/// so it agrees with that reservation.
fn split_panes(area: Rect, list_rows: u16) -> (Rect, Rect) {
    if stacks_vertically(area.width) {
        let (detail_rect, list_rect) = SplitPane::new()
            .with_direction(SplitDirection::Vertical)
            .with_ratio(SplitRatio::second_fixed(list_rows))
            .split(area);
        (list_rect, detail_rect)
    } else {
        SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .split(area)
    }
}

/// Wrapper widget that renders a [`ChooseOne`] list beside a live detail
/// pane.
#[derive(Clone)]
pub struct FileChooser;

impl StatefulWidget for FileChooser {
    type State = ChooseOneState<FileDetail>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (list_rect, detail_rect) = split_panes(area, list_rows(state.options()));
        ratatui::widgets::StatefulWidget::render(ChooseOne::new(), list_rect, buf, state);
        render_detail_pane(detail_rect, state.active_option().map(|o| &o.value), buf);
    }
}

impl HandleEvent for FileChooser {
    fn handle_event(
        &self,
        state: &mut ChooseOneState<FileDetail>,
        event: crossterm::event::KeyEvent,
    ) -> EventOutcome {
        ChooseOne::new().handle_event(state, event)
    }
}

/// Wrapper widget that renders a [`ChooseMany`] list beside a live detail
/// pane.
#[derive(Clone)]
pub struct MultiFileChooser;

impl StatefulWidget for MultiFileChooser {
    type State = ChooseManyState<FileDetail>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (list_rect, detail_rect) = split_panes(area, list_rows(state.options()));
        ratatui::widgets::StatefulWidget::render(ChooseMany::new(), list_rect, buf, state);
        let detail = state
            .hover()
            .and_then(|idx| state.options().get(idx))
            .map(|o| &o.value);
        render_detail_pane(detail_rect, detail, buf);
    }
}

impl HandleEvent for MultiFileChooser {
    fn handle_event(
        &self,
        state: &mut ChooseManyState<FileDetail>,
        event: crossterm::event::KeyEvent,
    ) -> EventOutcome {
        ChooseMany::new().handle_event(state, event)
    }
}

fn render_detail_pane(area: Rect, detail: Option<&FileDetail>, buf: &mut Buffer) {
    let Some(detail) = detail else { return };
    // Pre-render the schema code block at the pane width so its highlighting
    // and wrapping match the width the whole block is rendered at below.
    let term = Terminal::new_optimistic(area.width as u32);
    let prose = render_file_detail_prose(detail, &term);
    let ansi = prose.render_optimistic(Some(area.width as u32));
    let text: Text = ansi_to_tui::IntoText::into_text(&ansi).unwrap_or_default();
    let paragraph = Paragraph::new(text);
    Widget::render(paragraph, area, buf);
}

fn badge_markup(badge: &str) -> String {
    match badge {
        "COMPOSE" => "<bg-cyan-900><bold><cyan-100> Compose </cyan-100></bold></bg-cyan-900>".to_string(),
        "INLINE_COMPOSE" => "<bg-cyan-900><bold><cyan-100> Inline Compose </cyan-100></bold></bg-cyan-900>".to_string(),
        "SEQUENCE" => "<bg-yellow-900><bold><yellow-100> Sequence </yellow-100></bold></bg-yellow-900>".to_string(),
        _ => format!("<bold> {} </bold>", Prose::escape_text(badge)),
    }
}

fn path_label(path: &Path) -> String {
    path.canonicalize()
        .map(|p| to_portable_string(&p))
        .unwrap_or_else(|_| to_portable_string(path))
}

fn file_href(path: &Path) -> Option<String> {
    let absolute: PathBuf = if path.is_absolute() {
        path.to_path_buf()
    } else {
        path.canonicalize().ok()?
    };
    Url::from_file_path(absolute)
        .ok()
        .map(|url| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Optimistic terminal at a fixed width for deterministic rendering. The
    /// same width must be handed to [`render_file_detail_prose`] and the
    /// outer `render`, since the schema code block is pre-rendered against it.
    fn term(width: u32) -> Terminal {
        Terminal::new_optimistic(width)
    }

    fn sample_detail() -> FileDetail {
        FileDetail {
            badge: "COMPOSE".to_string(),
            name: "Review Plan".to_string(),
            path: std::env::temp_dir().join("review-plan.md"),
            description: Some("A helpful prompt".to_string()),
            schema_lines: vec!["title: 'string(required)'".to_string()],
            has_custom_name: true,
        }
    }

    fn unnamed_detail() -> FileDetail {
        FileDetail {
            badge: "FILE".to_string(),
            name: "notes".to_string(),
            path: std::env::temp_dir().join("notes.md"),
            description: None,
            schema_lines: Vec::new(),
            has_custom_name: false,
        }
    }

    /// The exact `$schema` block authored in the repo `plan.md`, faithfully
    /// serialized to YAML lines (spec → design → plan, in order). Bug 3
    /// dropped `spec` and mangled `match(`; this fixture pins the fix.
    fn plan_schema_detail() -> FileDetail {
        FileDetail {
            badge: "COMPOSE".to_string(),
            name: "Plan".to_string(),
            path: std::env::temp_dir().join("plan.md"),
            description: Some("Write an implementation _plan_".to_string()),
            schema_lines: vec![
                "spec: file(required;match(**/*spec*.md);eager)".to_string(),
                "design: file(match(**/*design*.md))".to_string(),
                "plan: file".to_string(),
            ],
            has_custom_name: true,
        }
    }

    #[test]
    fn detail_prose_includes_badge_name_and_description() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        let stripped = strip_ansi(&rendered);
        assert!(rendered.contains("Review Plan"));
        assert!(rendered.contains("A helpful prompt"));
        assert!(rendered.contains("Schema:"));
        // Highlighting inserts SGR between tokens, so assert on stripped text.
        assert!(stripped.contains("title:"));
    }

    #[test]
    fn detail_prose_falls_back_when_description_missing() {
        let detail = FileDetail {
            description: None,
            ..sample_detail()
        };
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(rendered.contains("no description"));
    }

    #[test]
    fn detail_prose_falls_back_when_no_schema() {
        let detail = FileDetail {
            schema_lines: Vec::new(),
            ..sample_detail()
        };
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(rendered.contains("no schema defined"));
    }

    #[test]
    fn detail_prose_renders_block_quote_border_for_description() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(rendered.contains('│'), "block quote border must be present: {rendered}");
        assert!(rendered.contains("A helpful prompt"));
    }

    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut in_escape = false;
        for ch in s.chars() {
            if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else if ch == '\u{1b}' {
                in_escape = true;
            } else {
                result.push(ch);
            }
        }
        result
    }

    #[test]
    fn detail_prose_no_schema_renders_dim_italic_placeholder() {
        let detail = unnamed_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        let stripped = strip_ansi(&rendered);
        // The placeholder is a plain dim-italic line now, no `- ` list bullet.
        assert!(stripped.contains("no schema defined"));
        assert!(
            rendered.contains("\u{1b}[2m") || rendered.contains("\u{1b}[3m"),
            "dim or italic SGR must style the no-schema fallback: {rendered}"
        );
    }

    /// Bug 3: the `$schema` block must render as a YAML code block carrying
    /// **all** properties, in authored order, un-mangled — `spec` is not
    /// dropped and `match(**/*spec*.md)` is not corrupted — even in a narrow
    /// detail pane.
    #[test]
    fn detail_prose_schema_code_block_is_faithful_and_ordered() {
        let detail = plan_schema_detail();
        let rendered = render_file_detail_prose(&detail, &term(60)).render_optimistic(Some(60));
        let stripped = strip_ansi(&rendered);

        let spec_idx = stripped.find("spec:").expect("spec property present");
        let design_idx = stripped.find("design:").expect("design property present");
        let plan_idx = stripped.find("plan:").expect("plan property present");
        assert!(
            spec_idx < design_idx && design_idx < plan_idx,
            "schema properties must appear in authored order: {stripped}"
        );
        assert!(
            stripped.contains("match(**/*spec*.md)"),
            "match() glob must render un-mangled: {stripped}"
        );
    }

    /// Bug 1: an authored `_emphasis_` description renders as italic, not as
    /// the literal underscore markup.
    #[test]
    fn detail_prose_description_renders_markdown_emphasis() {
        let detail = plan_schema_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        let stripped = strip_ansi(&rendered);
        assert!(
            rendered.contains("\u{1b}[3m"),
            "italic SGR must style the emphasized description: {rendered}"
        );
        assert!(
            !stripped.contains("_plan_"),
            "underscore markup must not leak literally: {stripped}"
        );
    }

    #[test]
    fn detail_prose_with_custom_name_renders_bold_name_and_parenthesized_path() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(
            rendered.contains("\u{1b}[1m"),
            "custom name must be rendered in bold SGR: {rendered}"
        );
        assert!(
            rendered.contains('(') && rendered.contains(')'),
            "path must appear inside parentheses: {rendered}"
        );
        assert!(
            rendered.contains("\u{1b}]8;"),
            "path must be emitted as an OSC8 hyperlink: {rendered}"
        );
    }

    #[test]
    fn detail_prose_without_custom_name_renders_path_as_osc8_link() {
        let detail = unnamed_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(
            rendered.contains("\u{1b}]8;"),
            "path must be emitted as an OSC8 hyperlink: {rendered}"
        );
        assert!(
            !rendered.contains("Path:"),
            "path must be rendered inline, not on a separate 'Path:' line: {rendered}"
        );
    }

    #[test]
    fn detail_prose_does_not_emit_separate_path_line() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(
            !rendered.contains("Path:"),
            "detail block must not contain a separate 'Path:' line: {rendered}"
        );
    }

    #[test]
    fn confirmation_dialog_adds_trailer() {
        let detail = sample_detail();
        let rendered =
            render_confirmation_dialog(&detail, &term(80)).render_optimistic(Some(80));
        assert!(rendered.contains("Use this file? (Y/n)"));
    }

    #[test]
    fn path_label_portably_renders_windows_shaped_segments() {
        assert_eq!(
            path_label(&PathBuf::from("docs").join(r"nested\plan.md")),
            "docs/nested/plan.md"
        );
    }

    #[test]
    fn relative_hyperlink_target_declines_to_plain_text() {
        let detail = FileDetail {
            path: PathBuf::from(r"relative\plan.md"),
            ..unnamed_detail()
        };
        let rendered = render_file_detail_prose(&detail, &term(80)).render_optimistic(Some(80));
        assert!(!rendered.contains("\u{1b}]8;"), "{rendered}");
        assert!(strip_ansi(&rendered).contains("relative/plan.md"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_hyperlink_encodes_reserved_characters() {
        assert_eq!(
            file_href(Path::new(r"C:\Program Files\claudine#100%.md")).as_deref(),
            Some("file:///C:/Program%20Files/claudine%23100%25.md")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_unc_hyperlink_uses_authority_and_encodes_path() {
        assert_eq!(
            file_href(Path::new(r"\\server\share\plan #%.md")).as_deref(),
            Some("file://server/share/plan%20%23%25.md")
        );
    }
}
