use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::components::renderable::Renderable;
use crate::discovery::detection::ColorDepth;
use crate::terminal::Terminal;
use crate::utils::styling::{FontWeight, Style, Stylist};
use crate::utils::{
    color::{BasicColor, TermColor},
    layout::Layout,
};

/// box outline shape for an _open_ **TODO**
const NERD_CHECKBOX_OPEN: &str = "\u{f0131}";

const NERD_CHECKBOX_COMPLETED: &str = "\u{f4a7}";
/// a badged checkbox nerd icon representing a _blocked_ **TODO**
const NERD_CHECKBOX_BLOCKED: &str = "\u{f0117}";
/// intermediate nerd icon representing a _in progress_ **TODO**
const NERD_CHECKBOX_IN_PROGRESS: &str = "\u{f0856}";
/// an box off nerd icon representing a _cancelled_ **TODO**
const NERD_CHECKBOX_CANCELLED: &str = "\u{f12ed}";

// const NERD_CHECKBOX_FILLED: &'static str = "\u{f012e}";

/// fallback representation for an _open_ **TODO**
pub static FB_CHECKBOX_OPEN: &str = "[ ]";
/// In-progress fallback with color
pub static FB_CHECKBOX_IN_PROGRESS: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::Green.fg("⏺")));
/// Completed fallback with color
pub static FB_CHECKBOX_COMPLETED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::Green.fg("✔")));
/// Cancelled fallback with color
pub static FB_CHECKBOX_CANCELLED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::BrightRed.fg("-")));
/// Blocked fallback with color
pub static FB_CHECKBOX_BLOCKED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::BrightRed.fg("⏺")));

/// No-color fallback representations for terminals without color support
pub static FB_CHECKBOX_IN_PROGRESS_NOCOLOR: &str = "[>]";
pub static FB_CHECKBOX_COMPLETED_NOCOLOR: &str = "[x]";
pub static FB_CHECKBOX_CANCELLED_NOCOLOR: &str = "[-]";
pub static FB_CHECKBOX_BLOCKED_NOCOLOR: &str = "[!]";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum TodoState {
    Open,
    InProgress,
    Completed,
    Cancelled,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TodoStateRep {
    pub nerd: &'static str,
    pub fallback: &'static str,
}

/// **TODO_CHAR_LOOKUP** provides a way to lookup what string representation to use for each
/// _state_ of a TODO. This lookup provides a representation for both a nerd font and the
/// fallback representation.
pub static TODO_CHAR_LOOKUP: LazyLock<HashMap<TodoState, TodoStateRep>> = LazyLock::new(|| {
    let mut m = HashMap::with_capacity(5);

    m.insert(
        TodoState::Open,
        TodoStateRep {
            nerd: NERD_CHECKBOX_OPEN,
            fallback: FB_CHECKBOX_OPEN,
        },
    );
    m.insert(
        TodoState::InProgress,
        TodoStateRep {
            nerd: NERD_CHECKBOX_IN_PROGRESS,
            fallback: &FB_CHECKBOX_IN_PROGRESS,
        },
    );
    m.insert(
        TodoState::Completed,
        TodoStateRep {
            nerd: NERD_CHECKBOX_COMPLETED,
            fallback: &FB_CHECKBOX_COMPLETED,
        },
    );
    m.insert(
        TodoState::Cancelled,
        TodoStateRep {
            nerd: NERD_CHECKBOX_CANCELLED,
            fallback: &FB_CHECKBOX_CANCELLED,
        },
    );
    m.insert(
        TodoState::Blocked,
        TodoStateRep {
            nerd: NERD_CHECKBOX_BLOCKED,
            fallback: &FB_CHECKBOX_BLOCKED,
        },
    );

    m
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    state: TodoState,
    description: String,
    created: DateTime<Utc>,
    last_updated: DateTime<Utc>,
    #[serde(skip)]
    layout: Layout,
}

impl PartialEq for Todo {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.description == other.description
            && self.created == other.created
            && self.last_updated == other.last_updated
    }
}

impl Eq for Todo {}

impl std::hash::Hash for Todo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.hash(state);
        self.description.hash(state);
        self.created.hash(state);
        self.last_updated.hash(state);
    }
}

impl Default for Todo {
    fn default() -> Self {
        Self {
            state: TodoState::Open,
            description: "".to_string(),
            created: Utc::now(),
            last_updated: Utc::now(),
            layout: Layout::default(),
        }
    }
}

impl From<&Todo> for Todo {
    /// allows the `state` and `description` of a Todo reference to
    /// be cloned into a new Todo which has `created` and `last_updated`
    /// set to now.
    fn from(value: &Todo) -> Self {
        Todo {
            state: value.state.clone(),
            description: value.description.clone(),
            ..Todo::default()
        }
    }
}

impl Todo {
    /// Create a new TODO item with just a description, the TODO's state
    /// will start as being "open".
    pub fn new<T: Into<String>>(desc: T) -> Todo {
        Todo {
            description: desc.into(),
            ..Todo::default()
        }
    }

    /// Reports the Todo item to the terminal. Using a nerd font representation
    /// if the terminal has detected that the font is a nerd font. Otherwise it
    /// uses basic characters which should be in all font variants.
    ///
    /// When the terminal does not support colors (`ColorDepth::None`), plain
    /// ASCII fallbacks are used without any ANSI escape codes.
    fn to_terminal(&self, term: &Terminal) -> String {
        let todo_icon = TODO_CHAR_LOOKUP
            .get(&self.state)
            .unwrap_or(&TODO_CHAR_LOOKUP[&TodoState::Open]);

        // Check if terminal supports colors
        let has_color = term.color_depth != ColorDepth::None;

        // Get the appropriate fallback icon based on color support
        let fallback_icon = if has_color {
            todo_icon.fallback
        } else {
            match self.state {
                TodoState::Open => FB_CHECKBOX_OPEN,
                TodoState::InProgress => FB_CHECKBOX_IN_PROGRESS_NOCOLOR,
                TodoState::Completed => FB_CHECKBOX_COMPLETED_NOCOLOR,
                TodoState::Cancelled => FB_CHECKBOX_CANCELLED_NOCOLOR,
                TodoState::Blocked => FB_CHECKBOX_BLOCKED_NOCOLOR,
            }
        };

        match self.state {
            TodoState::Cancelled => match term.is_nerd_font {
                Some(true) if has_color => {
                    FontWeight::Dim.wrap(format!("{} {}", todo_icon.nerd, self.description))
                }
                Some(true) => {
                    // Nerd font but no color - just use the icon without dim styling
                    format!("{} {}", todo_icon.nerd, self.description)
                }
                _ if has_color => FontWeight::Dim.wrap(format!(
                    "{} {}",
                    fallback_icon,
                    Style::Strikethrough.term_wrap(&self.description, term)
                )),
                _ => {
                    // No color - plain text with no styling
                    format!("{} {}", fallback_icon, self.description)
                }
            },
            _ => match term.is_nerd_font {
                Some(true) => {
                    format!("{} {}", todo_icon.nerd, self.description)
                }
                _ => {
                    format!("{} {}", fallback_icon, self.description)
                }
            },
        }
    }
}

impl Renderable for Todo {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        let content = self.to_terminal(&term);
        self.layout.apply_layout(&content, width)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let width = term.width();
        let content = self.to_terminal(term);
        self.layout.apply_layout(&content, width)
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

    /// Create a terminal with no color support for testing
    fn no_color_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::None)
            .build();
        // Ensure no nerd font support for consistent fallback output
        term.is_nerd_font = Some(false);
        term
    }

    /// Create a terminal with color support for testing
    fn color_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::TrueColor)
            .build();
        // Ensure no nerd font support for consistent fallback output
        term.is_nerd_font = Some(false);
        term
    }

    /// Helper to create Todo with a specific state
    fn todo_with_state(desc: &str, state: TodoState) -> Todo {
        Todo {
            description: desc.to_string(),
            state,
            ..Todo::default()
        }
    }

    #[test]
    fn test_no_color_open_todo() {
        let term = no_color_terminal();
        let todo = Todo::new("Buy groceries");
        let result = todo.to_terminal(&term);
        // Should use plain ASCII fallback "[ ]" without color codes
        assert_eq!(result, "[ ] Buy groceries");
    }

    #[test]
    fn test_no_color_completed_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Done task", TodoState::Completed);
        let result = todo.to_terminal(&term);
        // Should use plain "[x]" without color codes
        assert_eq!(result, "[x] Done task");
    }

    #[test]
    fn test_no_color_in_progress_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Working on it", TodoState::InProgress);
        let result = todo.to_terminal(&term);
        // Should use plain "[>]" without color codes
        assert_eq!(result, "[>] Working on it");
    }

    #[test]
    fn test_no_color_cancelled_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Dropped task", TodoState::Cancelled);
        let result = todo.to_terminal(&term);
        // Should use plain "[-]" without color codes or strikethrough
        assert_eq!(result, "[-] Dropped task");
    }

    #[test]
    fn test_no_color_blocked_todo() {
        let term = no_color_terminal();
        let todo = todo_with_state("Waiting on dependency", TodoState::Blocked);
        let result = todo.to_terminal(&term);
        // Should use plain "[!]" without color codes
        assert_eq!(result, "[!] Waiting on dependency");
    }

    #[test]
    fn test_color_completed_todo_has_ansi() {
        let term = color_terminal();
        let todo = todo_with_state("Done task", TodoState::Completed);
        let result = todo.to_terminal(&term);
        // With color support, should contain ANSI escape codes
        assert!(
            result.contains('\x1b'),
            "Expected ANSI codes in colored output: {:?}",
            result
        );
    }
}
