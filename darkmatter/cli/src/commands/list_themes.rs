//! `md list-themes`: the themes `style.page.theme` and
//! `style.page.code.theme` accept, in their canonical order.
//!
//! A data output is a bare array of `{ name, description }`; the formatted
//! outputs list each theme and say how to select one.

use crate::args::Cli;
use crate::output::command::{CommandRun, Completed, DataShape};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::command::{CommandValue, prose_document};
use serde::Serialize;

/// One theme as `md list-themes` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeEntry {
    /// The canonical (kebab-case) name a style value takes.
    pub name: String,
    /// The dark variant's description.
    pub description: String,
}

/// Every theme, in [`ThemePair::all`]'s order.
pub fn theme_entries() -> Vec<ThemeEntry> {
    ThemePair::all()
        .iter()
        .map(|theme| ThemeEntry { name: theme.kebab_name().to_string(), description: theme.description(ColorMode::Dark).to_string() })
        .collect()
}

/// How a reader selects a theme, in every formatted output.
const GUIDANCE: &str = "Set the prose theme with `style.page.theme=NAME` and the code theme with `style.page.code.theme=NAME`.";

/// Runs `md list-themes`.
///
/// ## Errors
///
/// A usage error from `--width`; [`crate::output::report::ReportedExit`]
/// when the result cannot be written.
pub(crate) fn run_list_themes(output: OutputTarget, cli: &Cli) -> Result<()> {
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Bare)?;
    let entries = theme_entries();
    let prose = prose_document(&themes_markdown(&entries));
    let terminal_entries = entries.clone();
    let value = CommandValue::new(entries, prose)
        .with_terminal(move |options| themes_terminal(&terminal_entries, &Terminal::from_output_options(options)));
    run.finish(Ok(Completed::success(value)))
}

fn themes_markdown(entries: &[ThemeEntry]) -> String {
    let mut out = String::from("Available themes:\n\n");
    for entry in entries {
        out.push_str(&format!("- `{}`: {}\n", entry.name, entry.description));
    }
    out.push('\n');
    out.push_str(GUIDANCE);
    out.push('\n');
    out
}

fn themes_terminal(entries: &[ThemeEntry], terminal: &Terminal) -> String {
    let mut list = UnorderedList::empty();
    for entry in entries {
        let item = format!("<b>{}</b>: {}", Prose::escape_text(&entry.name), Prose::escape_text(&entry.description));
        list.add(Prose::new(item).with_line_breaks(LineBreaks::Hard));
    }
    let heading = Prose::new("Available themes:").render(terminal);
    let guidance = Prose::new(Prose::escape_text_outside_code_spans(GUIDANCE)).render(terminal);
    format!("{heading}\n\n{}\n\n{guidance}", list.render(terminal).trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entries_follow_the_canonical_theme_order() {
        let names: Vec<String> = theme_entries().into_iter().map(|entry| entry.name).collect();
        let expected: Vec<String> = ThemePair::all().iter().map(|theme| theme.kebab_name().to_string()).collect();
        assert_eq!(names, expected);
        assert!(theme_entries().iter().all(|entry| !entry.description.is_empty()));
    }

    #[test]
    fn the_markdown_names_each_theme_and_the_setters_that_select_one() {
        let markdown = themes_markdown(&theme_entries());
        for theme in ThemePair::all() {
            assert!(markdown.contains(&format!("`{}`", theme.kebab_name())), "{markdown}");
        }
        assert!(markdown.contains("style.page.theme=NAME") && markdown.contains("style.page.code.theme=NAME"), "{markdown}");
    }
}
