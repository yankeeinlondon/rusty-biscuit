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
//! - [`choose_one_file`] / [`choose_many_files`] — two-pane
//!   `SplitDirection::Auto` choosers with a live detail pane.

#![allow(dead_code)] // Phase 1 scaffolding consumed by Phases 2–4.

use std::io::{self, Write};
use std::path::Path;

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::RenderableTerminalContent;
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_tui::core::{SplitDirection, SplitPane};
use biscuit_tui::prelude::*;
use claudine::composition::FileDetail;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{Paragraph, StatefulWidget, Widget},
};

/// Render the detail block for a candidate file.
///
/// Layout per spec: `<badge> <name-or-path-line>`, blank line, blockquoted
/// description, `Schema:` header + unordered list, and an inline OSC8 path
/// link. Built from [`Compose`], [`BlockQuote`], [`UnorderedList`], and
/// [`Prose`] components rather than a hand-rolled markup string.
pub fn render_file_detail_prose(detail: &FileDetail) -> Compose {
    let parts: Vec<RenderableTerminalContent> = vec![
        RenderableTerminalContent::from(name_line_prose(detail)),
        RenderableTerminalContent::from("\n\n"),
        RenderableTerminalContent::from(BlockQuote::from(description_prose(detail))),
        RenderableTerminalContent::from("\n\nSchema:\n\n"),
        RenderableTerminalContent::from(schema_list(detail)),
    ];
    Compose::from(parts)
}

/// Render the full confirmation dialog including the `Use this file? (Y/n)`
/// trailer.
pub fn render_confirmation_dialog(detail: &FileDetail) -> Compose {
    let mut compose = render_file_detail_prose(detail);
    compose.add_text("\n\nUse this file? (Y/n)");
    compose
}

fn name_line_prose(detail: &FileDetail) -> Prose {
    let badge = badge_markup(&detail.badge);
    let abs_href = Prose::quoted_attr(&detail.path.display().to_string());
    let label = path_label(&detail.path);

    let body = if detail.has_custom_name {
        let name = Prose::escape_text(&detail.name);
        let escaped_label = Prose::escape_text(&label);
        format!(
            "{badge} <bold>{name}</bold> (<a href={abs_href}><dim><blue>{escaped_label}</blue></dim></a>)",
        )
    } else {
        let escaped_label = Prose::escape_text(&label);
        format!("{badge} <a href={abs_href}>{escaped_label}</a>")
    };

    Prose::new(body)
}

fn description_prose(detail: &FileDetail) -> Prose {
    let text = detail
        .description
        .as_deref()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or("no description");
    Prose::new(Prose::escape_text(text))
}

fn schema_list(detail: &FileDetail) -> UnorderedList {
    let mut list = UnorderedList::empty();
    if detail.schema_lines.is_empty() {
        list.add(Prose::new("<dim><i>no schema defined</i></dim>"));
    } else {
        for line in &detail.schema_lines {
            list.add(Prose::new(Prose::escape_text(line)));
        }
    }
    list
}

/// Drive a single-file confirmation dialog.
///
/// Returns `Ok(true)` for Y/Enter, `Ok(false)` for n/Esc. Ctrl+C is left
/// to the caller's SIGINT handler and will surface as an I/O error if
/// raw-mode reading is interrupted.
pub fn confirm_one_file(detail: &FileDetail) -> io::Result<bool> {
    let term = crate::log::terminal();
    let prose = render_confirmation_dialog(detail);
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
    let state = ChooseOneState::from_options(options);
    run_standalone(FileChooser, state, None)
}

/// Drive a two-pane multi-select chooser and return the selected file
/// details, or an error if the user cancelled.
pub fn choose_many_files(options: Vec<ChoiceOption<FileDetail>>) -> io::Result<Vec<FileDetail>> {
    let state = ChooseManyState::from_options(options);
    run_standalone(MultiFileChooser, state, None)
}

/// Wrapper widget that renders a [`ChooseOne`] list beside a live detail
/// pane.
#[derive(Clone)]
pub struct FileChooser;

impl StatefulWidget for FileChooser {
    type State = ChooseOneState<FileDetail>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (list_rect, detail_rect) = SplitPane::new()
            .with_direction(SplitDirection::Auto)
            .split(area);
        // Auto resolves to Vertical in tall terminals; the plan wants the
        // detail pane above the list when taller-than-wide.
        let (list_rect, detail_rect) = if area.width < area.height {
            (detail_rect, list_rect)
        } else {
            (list_rect, detail_rect)
        };
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
        let (list_rect, detail_rect) = SplitPane::new()
            .with_direction(SplitDirection::Auto)
            .split(area);
        // Auto resolves to Vertical in tall terminals; the plan wants the
        // detail pane above the list when taller-than-wide.
        let (list_rect, detail_rect) = if area.width < area.height {
            (detail_rect, list_rect)
        } else {
            (list_rect, detail_rect)
        };
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
    let prose = render_file_detail_prose(detail);
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
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_detail() -> FileDetail {
        FileDetail {
            badge: "COMPOSE".to_string(),
            name: "Review Plan".to_string(),
            path: PathBuf::from("/tmp/review-plan.md"),
            description: Some("A helpful prompt".to_string()),
            schema_lines: vec!["title: 'string(required)'".to_string()],
            has_custom_name: true,
        }
    }

    fn unnamed_detail() -> FileDetail {
        FileDetail {
            badge: "FILE".to_string(),
            name: "notes".to_string(),
            path: PathBuf::from("/tmp/notes.md"),
            description: None,
            schema_lines: Vec::new(),
            has_custom_name: false,
        }
    }

    #[test]
    fn detail_prose_includes_badge_name_and_description() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
        assert!(rendered.contains("Review Plan"));
        assert!(rendered.contains("A helpful prompt"));
        assert!(rendered.contains("Schema:"));
        assert!(rendered.contains("title:"));
    }

    #[test]
    fn detail_prose_falls_back_when_description_missing() {
        let detail = FileDetail {
            description: None,
            ..sample_detail()
        };
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
        assert!(rendered.contains("no description"));
    }

    #[test]
    fn detail_prose_falls_back_when_no_schema() {
        let detail = FileDetail {
            schema_lines: Vec::new(),
            ..sample_detail()
        };
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
        assert!(rendered.contains("no schema defined"));
    }

    #[test]
    fn detail_prose_renders_block_quote_border_for_description() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
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
    fn detail_prose_no_schema_renders_dim_italic_unordered_list() {
        let detail = unnamed_detail();
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("- no schema defined"));
        assert!(
            rendered.contains("\u{1b}[2m") || rendered.contains("\u{1b}[3m"),
            "dim or italic SGR must style the no-schema fallback: {rendered}"
        );
    }

    #[test]
    fn detail_prose_with_custom_name_renders_bold_name_and_parenthesized_path() {
        let detail = sample_detail();
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
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
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
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
        let rendered = render_file_detail_prose(&detail).render_optimistic(Some(80));
        assert!(
            !rendered.contains("Path:"),
            "detail block must not contain a separate 'Path:' line: {rendered}"
        );
    }

    #[test]
    fn confirmation_dialog_adds_trailer() {
        let detail = sample_detail();
        let rendered = render_confirmation_dialog(&detail).render_optimistic(Some(80));
        assert!(rendered.contains("Use this file? (Y/n)"));
    }
}
