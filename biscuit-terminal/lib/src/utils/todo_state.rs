use std::{collections::HashMap, sync::LazyLock};

use crate::terminal::Terminal;

pub struct TodoChar {
    pub nerd: char,
    pub fallback: char
}

pub static TODO_CHAR_LOOKUP: LazyLock<HashMap<TodoState, TodoChar>> = LazyLock::new(|| {
    let mut m = HashMap::with_capacity(5);

    m.insert(TodoState::Open, TodoChar { nerd: '\u{2610}', fallback: '\u{f096}' });
    m.insert(TodoState::InProgress, TodoChar { nerd: '\u{2610}', fallback: '\u{f096}' });
    m.insert(TodoState::Completed, TodoChar { nerd: '\u{2610}', fallback: '\u{f096}' });
    m.insert(TodoState::Cancelled, TodoChar { nerd: '\u{2610}', fallback: '\u{f096}' });
    m.insert(TodoState::Blocked, TodoChar { nerd: '\u{2610}', fallback: '\u{f096}' });

    m
});


#[derive(Eq, Hash, PartialEq)]
pub enum TodoState {
    Open,
    InProgress,
    Completed,
    Cancelled,
    Blocked
}

/// The **Todo** struct represents a TODO action item with
/// a description and state.
pub struct Todo {
    pub state: TodoState,
    pub description: String,
}

impl From<&str> for Todo {
    fn from(value: &str) -> Self {
        Todo {
            state: TodoState::Open,
            description: value.to_owned().to_string()
        }
    }
}

impl From<&String> for Todo {
    fn from(value: &String) -> Self {
        Todo {
            state: TodoState::Open,
            description: value.clone()
        }
    }
}

impl From<String> for Todo {
    fn from(value: String) -> Self {
        Todo {
            state: TodoState::Open,
            description: value
        }
    }
}

impl Todo {
    pub fn new<T:Into<String>>(description: T, state: Option<TodoState>) -> Self {
        match state {
            Some(s) => Todo { description: description.into(), state: s },
            _ => Todo { description: description.into(), state: TodoState::Open }
        }
    }

    /// Reports the Todo item to the terminal. Using a nerd font representation
    /// if the terminal has detected that the font is a nerd font. Otherwise it
    /// uses basic characters which should be in all font variants.
    pub fn to_terminal(self, term: Terminal) {
        let todo_char = TODO_CHAR_LOOKUP.get(&self.state).unwrap_or_else(|| {
            // Default to Open state if state not found
            &TODO_CHAR_LOOKUP[&TodoState::Open]
        });

        let char = match term.is_nerd_font {
            Some(true) => todo_char.nerd,
            _ => todo_char.fallback,
        };

        println!("{} {}", char, self.description);
    }

    /// While it's unusual to prepare output for the browser in the **Biscuit Terminal**
    /// it's just
    pub fn to_browser(self) {
        todo!()
    }
}
