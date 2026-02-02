use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::components::renderable::{Renderable, RenderableWrapper};
use crate::terminal::Terminal;
use crate::utils::styling::{FontWeight, Style, Stylist};
use crate::utils::{
    color::{BasicColor, TermColor},
    layout::Layout,
};

/// box outline shape for an _open_ **TODO**
const NERD_CHECKBOX_OPEN: &'static str = "\u{f0131}";

const NERD_CHECKBOX_COMPLETED: &'static str = "\u{f4a7}";
/// a badged checkbox nerd icon representing a _blocked_ **TODO**
const NERD_CHECKBOX_BLOCKED: &'static str = "\u{f0117}";
/// intermediate nerd icon representing a _in progress_ **TODO**
const NERD_CHECKBOX_IN_PROGRESS: &'static str = "\u{f0856}";
/// an box off nerd icon representing a _cancelled_ **TODO**
const NERD_CHECKBOX_CANCELLED: &'static str = "\u{f12ed}";

const NERD_CHECKBOX_FILLED: &'static str = "\u{f012e}";

/// fallback representation for an _open_ **TODO**
pub static FB_CHECKBOX_OPEN: &str = "[ ]";
pub static FB_CHECKBOX_IN_PROGRESS: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::Green.fg("⏺")));
pub static FB_CHECKBOX_COMPLETED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::Green.fg("✔")));
pub static FB_CHECKBOX_CANCELLED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::BrightRed.fg("-")));
pub static FB_CHECKBOX_BLOCKED: LazyLock<String> =
    LazyLock::new(|| format!("[{}]", BasicColor::BrightRed.fg("⏺")));

#[derive(Debug, Clone,  PartialEq, Eq, Serialize, Deserialize, Hash )]
pub enum TodoState {
    Open,
    InProgress,
    Completed,
    Cancelled,
    Blocked,
}

#[derive(Debug, Clone,  PartialEq, Eq, Serialize, Deserialize, Hash )]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash )]
pub struct Todo {
    state: TodoState,
    description: String,
    created: DateTime<Utc>,
    last_updated: DateTime<Utc>,
}


impl Default for Todo {
    fn default() -> Self {
        Self {
            state: TodoState::Open,
            description: "".to_string(),
            created: Utc::now(),
            last_updated: Utc::now(),
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
    fn to_terminal(&self, term: &Terminal) -> String {
        let todo_icon = TODO_CHAR_LOOKUP
            .get(&self.state)
            .unwrap_or(&TODO_CHAR_LOOKUP[&TodoState::Open]);

        match self.state {
            TodoState::Cancelled => match term.is_nerd_font {
                Some(true) => {
                    FontWeight::Dim.wrap(format!("{} {}", todo_icon.nerd, self.description))
                }
                _ => FontWeight::Dim.wrap(format!(
                    "{} {}",
                    todo_icon.fallback,
                    Style::Strikethrough.term_wrap(&self.description, term)
                )),
            },
            _ => match term.is_nerd_font {
                Some(true) => {
                    format!("{} {}", todo_icon.nerd, self.description)
                }
                _ => {
                    format!("{} {}", todo_icon.fallback, self.description)
                }
            },
        }
    }
}

impl Renderable for Todo {
    fn render(&self, layout: Option<&Layout>) -> String {
        let term = Terminal::new();

        match layout {
            Some(layout) => {
                layout.render(self.to_terminal(&term))
            },
            _ => self.to_terminal(&term)
        }
    }

    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String {
        match layout {
            Some(layout) => {
                layout.render(self.to_terminal(term))
            },
            _ => self.to_terminal(term)
        }
    }
}
