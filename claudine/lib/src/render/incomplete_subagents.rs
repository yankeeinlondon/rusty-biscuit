use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, WordWrap};

use crate::stream::task_ledger::SubagentOutcome;

/// The full operator diagnostic for a run whose sub-agent work never finished.
///
/// The companion to the concise lifecycle headline, which is clamped to 240
/// characters and may name only the first few tasks. This component is under
/// no such budget: it enumerates **every** incomplete fact, so the terminal and
/// the machine record agree on the count even when the headline does not.
#[derive(Debug, Default)]
pub struct IncompleteSubagents {
    outcomes: Vec<SubagentOutcome>,
    layout: Layout,
}

impl IncompleteSubagents {
    pub fn new(outcomes: &[SubagentOutcome]) -> Self {
        Self {
            outcomes: outcomes.to_vec(),
            layout: Layout::default(),
        }
    }

    /// True when there is nothing to render, so callers can skip the section
    /// rather than emit an empty heading.
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }
}

impl TerminalRenderable for IncompleteSubagents {
    fn render(&self, term: &Terminal) -> String {
        if self.outcomes.is_empty() {
            return String::new();
        }
        let count = self.outcomes.len();
        let noun = if count == 1 { "task" } else { "tasks" };
        // `Layout::default()` ships `word_wrap: None`, so without this opt-in
        // the two-sentence heading reaches the pane as one logical line and the
        // *emulator* soft-wraps it mid-word. The list below already wraps on
        // word boundaries; the heading must read the same way.
        let heading = Prose::new(format!(
            "<b>{count} sub-agent {noun} did not complete.</b> The provider \
             exited normally, so this run is a failure despite its exit code."
        ))
        .with_word_wrap(WordWrap::default())
        .render(term);
        let items: Vec<String> = self
            .outcomes
            .iter()
            .map(SubagentOutcome::describe)
            .collect();
        let list = UnorderedList::from(items).with_bullet("• ").render(term);
        format!("{heading}\n{list}")
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
mod tests {
    use super::*;
    use crate::stream::task_ledger::TaskOutcome;
    use biscuit_terminal::discovery::detection::color::ColorDepth;
    use biscuit_terminal::utils::block_constraint::visible_width;

    /// A no-color terminal of a chosen width, so rendered rows can be measured
    /// and rejoined as plain text without stripping escapes first.
    fn plain_term(width: u32) -> Terminal {
        Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::None)
            .supports_unicode(true)
            .is_tty(false)
            .build()
    }

    fn stopped(name: &str) -> SubagentOutcome {
        SubagentOutcome {
            task_id: Some(format!("id-{name}")),
            name: Some(name.to_string()),
            outcome: TaskOutcome::Stopped,
            raw_status: Some("stopped".into()),
        }
    }

    #[test]
    fn an_empty_list_renders_nothing() {
        let term = Terminal::default();
        let component = IncompleteSubagents::new(&[]);
        assert!(component.is_empty());
        assert_eq!(component.render(&term), "");
    }

    #[test]
    fn every_incomplete_task_is_enumerated() {
        let term = Terminal::default();
        let outcomes: Vec<_> = (0..9).map(|i| stopped(&format!("agent-{i}"))).collect();
        let rendered = IncompleteSubagents::new(&outcomes).render(&term);
        assert!(rendered.contains("9 sub-agent tasks did not complete"), "{rendered}");
        for index in 0..9 {
            assert!(
                rendered.contains(&format!("agent-{index}")),
                "missing agent-{index} in:\n{rendered}"
            );
        }
    }

    #[test]
    fn a_single_task_reads_as_singular() {
        let term = Terminal::default();
        let outcomes = vec![stopped("lonely")];
        let rendered = IncompleteSubagents::new(&outcomes).render(&term);
        assert!(rendered.contains("1 sub-agent task did not complete"), "{rendered}");
    }

    /// The heading is two sentences and overruns any realistic pane, so it is
    /// the component's job to fold it. Left to `Layout::default()`'s
    /// `WordWrap::None` it reaches the terminal as one logical line and the
    /// emulator chops it mid-word — the defect a Level 2 capture caught.
    #[test]
    fn the_heading_wraps_on_word_boundaries() {
        const WIDTH: u32 = 70;
        let term = plain_term(WIDTH);
        let outcomes: Vec<_> = (0..7).map(|i| stopped(&format!("agent-{i}"))).collect();
        let rendered = IncompleteSubagents::new(&outcomes).render(&term);

        let heading_rows: Vec<&str> = rendered
            .lines()
            .take_while(|row| !row.trim_start().starts_with('\u{2022}'))
            .map(str::trim_end)
            .collect();

        assert!(
            heading_rows.len() > 1,
            "the heading is longer than {WIDTH} cells, so the component must fold it onto \
             more than one row.\nrendered:\n{rendered}"
        );
        for row in &heading_rows {
            assert!(
                visible_width(row) < WIDTH,
                "a heading row that fills the pane is the emulator's wrap, not the \
                 component's; row {row:?} used {} of {WIDTH} cells.\nrendered:\n{rendered}",
                visible_width(row)
            );
        }

        // Rejoining the rows with a single space reproduces the sentence exactly
        // only if every break fell between words: a mid-word chop would leave
        // the two halves separated (`so t` + `his run`).
        let rejoined = heading_rows
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            rejoined,
            "7 sub-agent tasks did not complete. The provider exited normally, so this run \
             is a failure despite its exit code.",
            "no word may be split across heading rows.\nrendered:\n{rendered}"
        );
    }

    #[test]
    fn an_anonymous_never_finished_task_still_gets_a_line() {
        let term = Terminal::default();
        let outcomes = vec![SubagentOutcome {
            task_id: None,
            name: None,
            outcome: TaskOutcome::Unfinished,
            raw_status: None,
        }];
        let rendered = IncompleteSubagents::new(&outcomes).render(&term);
        assert!(rendered.contains("unnamed task"), "{rendered}");
        assert!(rendered.contains("never finished"), "{rendered}");
    }
}
