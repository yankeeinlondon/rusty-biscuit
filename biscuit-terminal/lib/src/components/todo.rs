use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};

use crate::components::renderable::Renderable;
use crate::terminal::Terminal;
use crate::utils::styling::{FontWeight, Stylist};
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

#[derive(Eq, Hash, PartialEq)]
pub enum TodoState {
    Open,
    InProgress,
    Completed,
    Cancelled,
    Blocked,
}

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

impl Todo {
    /// Create a new TODO item with just a description, the TODO's state
    /// will start as being "open".
    fn new<T: Into<String>>(desc: T) -> Todo {
        Todo {
            description: desc.into(),
            ..Todo::default()
        }
    }

    /// Reports the Todo item to the terminal. Using a nerd font representation
    /// if the terminal has detected that the font is a nerd font. Otherwise it
    /// uses basic characters which should be in all font variants.
    pub fn to_terminal(self, term: Terminal) -> String {
        let todo_icon = TODO_CHAR_LOOKUP.get(&self.state).unwrap_or_else(|| {
            // Default to Open state if state not found
            &TODO_CHAR_LOOKUP[&TodoState::Open]
        });

        match self.state {
            TodoState::Cancelled => {
                FontWeight::Dim.wrap(format!("{} {}", todo_icon, self.description))
            }
            _ => format!("{} {}", todo_icon, self.description),
        }
    }
}

impl Renderable for Todo {
    /// When we render _prior_ to having a concrete terminal and it's capabilities to
    /// work with we will assume that the font is NOT a nerd font and conservatively
    /// just use normal character strings to render a todo.
    fn render(self, layout: Option<&Layout>) -> String {
        match layout {
            Some(layout) => {
                todo!()
            }
            _ => todo!(),
        }
    }
    fn fallback_render(self, term: &Terminal, layout: Option<&Layout>) -> String {
        match layout {
            Some(layout) => {
                todo!()
            }
            _ => todo!(),
        }
    }
}
