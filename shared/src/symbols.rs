
/// represents a TODO "state" with both a character which can
/// be universal used, and a "nerd font" variant.
pub struct TodoState(&'static str, &'static str);

impl TodoState {
  pub fn get_universal(&self) -> &'static str {
    self.0
  }
  pub fn get_nerd_font(&self) -> &'static str {
    self.1
  }
}


/// Represents "states" of a TODO item which can be represented
/// well in the terminal (no progress, in process, completed, cancelled)
pub enum Todo {
  NoProgress,
  InProgress,
  Completed,
  Cancelled
}

impl Todo {
  pub fn get_icon_state(&self) -> TodoState {
    match *self {
      Todo::NoProgress => TodoState("\u{2610}", "\u{f096}"),
      Todo::InProgress => TodoState("\u{25A3}", "\u{f0c8}"),
      Todo::Completed => TodoState("\u{2611}","\u{f046}"),
      Todo::Cancelled => TodoState("\u{2612}","\u{f057}")
    }
  }
}

