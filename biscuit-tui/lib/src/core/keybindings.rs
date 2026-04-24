//! Configurable key bindings shared by every input component.
//!
//! [`KeyBindings`] is a plain struct of `Vec<KeyEvent>` per logical
//! action. Components match incoming `KeyEvent`s against the binding
//! lists and fall through to `EventOutcome::Ignored` when no binding
//! matches. The [`Default`] impl provides vim-compatible directional
//! bindings plus the conventional submit/cancel keys.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Configurable key bindings for component navigation, toggle,
/// submit, and cancel actions.
///
/// ## Notes
///
/// Each action is represented by a `Vec<KeyEvent>` so that multiple
/// keys (e.g. `Up` and `k`) can bind to the same action. Components
/// iterate the list and match against the incoming event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBindings {
    /// Moves focus/hover upward (e.g. previous option).
    pub up: Vec<KeyEvent>,

    /// Moves focus/hover downward (e.g. next option).
    pub down: Vec<KeyEvent>,

    /// Moves focus/hover leftward (e.g. previous cell).
    pub left: Vec<KeyEvent>,

    /// Moves focus/hover rightward (e.g. next cell).
    pub right: Vec<KeyEvent>,

    /// Toggles the current selection (e.g. `ChooseMany` item toggle).
    pub toggle: Vec<KeyEvent>,

    /// Submits the current value.
    pub submit: Vec<KeyEvent>,

    /// Cancels the current interaction.
    pub cancel: Vec<KeyEvent>,

    /// Selects every enabled option (multi-select only).
    pub select_all: Vec<KeyEvent>,

    /// Clears every selection (multi-select only).
    pub deselect_all: Vec<KeyEvent>,
}

impl KeyBindings {
    /// Returns `true` when `event` matches any binding in `list`.
    ///
    /// Callers typically use this as:
    ///
    /// ```ignore
    /// if KeyBindings::matches(&bindings.submit, &key) {
    ///     return EventOutcome::Submitted;
    /// }
    /// ```
    pub fn matches(list: &[KeyEvent], event: &KeyEvent) -> bool {
        list.iter().any(|bound| bound == event)
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            up: vec![plain(KeyCode::Up), plain(KeyCode::Char('k'))],
            down: vec![plain(KeyCode::Down), plain(KeyCode::Char('j'))],
            left: vec![plain(KeyCode::Left), plain(KeyCode::Char('h'))],
            right: vec![plain(KeyCode::Right), plain(KeyCode::Char('l'))],
            toggle: vec![plain(KeyCode::Char(' '))],
            submit: vec![plain(KeyCode::Enter)],
            cancel: vec![plain(KeyCode::Esc)],
            select_all: vec![ctrl(KeyCode::Char('a'))],
            deselect_all: vec![ctrl(KeyCode::Char('d'))],
        }
    }
}

/// A `KeyEvent` with no modifiers — a convenience for default bindings.
const fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// A `KeyEvent` with `Ctrl` held — a convenience for default bindings.
const fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn defaults_bind_arrows_and_vim_keys() {
        let bindings = KeyBindings::default();
        assert!(KeyBindings::matches(&bindings.up, &key(KeyCode::Up)));
        assert!(KeyBindings::matches(&bindings.up, &key(KeyCode::Char('k'))));
        assert!(KeyBindings::matches(&bindings.down, &key(KeyCode::Down)));
        assert!(KeyBindings::matches(
            &bindings.down,
            &key(KeyCode::Char('j'))
        ));
        assert!(KeyBindings::matches(&bindings.left, &key(KeyCode::Left)));
        assert!(KeyBindings::matches(
            &bindings.left,
            &key(KeyCode::Char('h'))
        ));
        assert!(KeyBindings::matches(&bindings.right, &key(KeyCode::Right)));
        assert!(KeyBindings::matches(
            &bindings.right,
            &key(KeyCode::Char('l'))
        ));
    }

    #[test]
    fn defaults_bind_toggle_submit_and_cancel() {
        let bindings = KeyBindings::default();
        assert!(KeyBindings::matches(
            &bindings.toggle,
            &key(KeyCode::Char(' '))
        ));
        assert!(KeyBindings::matches(&bindings.submit, &key(KeyCode::Enter)));
        assert!(KeyBindings::matches(&bindings.cancel, &key(KeyCode::Esc)));
    }

    #[test]
    fn defaults_bind_select_all_and_deselect_all_to_ctrl_a_and_ctrl_d() {
        let bindings = KeyBindings::default();
        let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(KeyBindings::matches(&bindings.select_all, &ctrl_a));
        assert!(KeyBindings::matches(&bindings.deselect_all, &ctrl_d));
        // Plain 'a' or 'd' must not match (they are filter/search input).
        assert!(!KeyBindings::matches(
            &bindings.select_all,
            &key(KeyCode::Char('a'))
        ));
        assert!(!KeyBindings::matches(
            &bindings.deselect_all,
            &key(KeyCode::Char('d'))
        ));
    }

    #[test]
    fn matches_is_false_for_unbound_keys() {
        let bindings = KeyBindings::default();
        assert!(!KeyBindings::matches(
            &bindings.submit,
            &key(KeyCode::Char('z'))
        ));
    }

    #[test]
    fn custom_bindings_override_defaults() {
        let bindings = KeyBindings {
            submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
            ..KeyBindings::default()
        };
        let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(KeyBindings::matches(&bindings.submit, &ctrl_s));
        assert!(!KeyBindings::matches(
            &bindings.submit,
            &key(KeyCode::Enter)
        ));
    }
}
