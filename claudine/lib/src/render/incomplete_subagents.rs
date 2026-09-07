use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

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
        let heading = Prose::new(format!(
            "<b>{count} sub-agent {noun} did not complete.</b> The provider \
             exited normally, so this run is a failure despite its exit code."
        ))
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
