//! Shared types for [`ChooseOne`](super::choose_one::ChooseOne) and
//! [`ChooseMany`](super::choose_many::ChooseMany).
//!
//! Both choice components share the same option schema, selection
//! mode enum, and typed value projection helper. Keeping them here
//! avoids a circular dependency between the two component modules.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::components::choose::{ChoiceInput, ChoiceOption, SelectionMode};
//!
//! let input: ChoiceInput = ChoiceInput::new("colour", "Pick a colour")
//!     .with_options(vec![
//!         ChoiceOption::new("red", "Red", "red"),
//!         ChoiceOption::new("green", "Green", "green"),
//!     ])
//!     .required();
//! assert_eq!(input.options.len(), 2);
//! assert_eq!(input.selection_mode, SelectionMode::Single);
//! ```

/// How many options a choice component allows the user to pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionMode {
    /// Exactly one option may be selected at a time.
    Single,
    /// Zero or more options may be selected.
    Multiple,
}

/// A single option rendered by `ChooseOne` or `ChooseMany`.
///
/// `V` is the typed value associated with the option; defaults to
/// `String`. Use [`ChoiceOption::map_value`] to project an option
/// into a different value type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption<V = String> {
    /// Stable identifier for the option (emitted in JSON output).
    pub id: String,
    /// Human-readable label rendered next to the selection indicator.
    pub label: String,
    /// Typed value returned from `state.value()` when the option is
    /// selected.
    pub value: V,
    /// When `true`, the option is rendered dimmed and cannot be
    /// selected.
    pub disabled: bool,
}

impl<V> ChoiceOption<V> {
    /// Convenience constructor.
    ///
    /// Accepts any value that converts into `V` via `Into`, so call
    /// sites using the default `V = String` can pass `&str` literals
    /// without an explicit `.to_string()`.
    pub fn new<Id, Label, Value>(id: Id, label: Label, value: Value) -> Self
    where
        Id: Into<String>,
        Label: Into<String>,
        Value: Into<V>,
    {
        Self {
            id: id.into(),
            label: label.into(),
            value: value.into(),
            disabled: false,
        }
    }

    /// Marks this option as disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Projects the option's value into a new type `U`.
    ///
    /// Useful for CLI callers that start with `ChoiceOption<String>`
    /// and want to convert into a strongly-typed domain value.
    pub fn map_value<U>(self, f: impl FnOnce(V) -> U) -> ChoiceOption<U> {
        ChoiceOption {
            id: self.id,
            label: self.label,
            value: f(self.value),
            disabled: self.disabled,
        }
    }
}

/// Configuration for a choice component (both `ChooseOne` and
/// `ChooseMany` consume this shape).
///
/// The CLI always operates with `V = String`; library consumers with
/// a typed `V` can build an input from strings via the
/// [`choice_builders`](crate::helpers::choice_builders) helpers and
/// then project per-option via [`ChoiceOption::map_value`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceInput<V = String> {
    /// Stable identifier for the input (used by [`InputTable`] when
    /// embedding).
    ///
    /// [`InputTable`]: crate::components
    pub id: String,
    /// Prompt text shown to the user. The component itself does not
    /// render it — callers typically attach it as a [`Label`].
    ///
    /// [`Label`]: crate::core::Label
    pub prompt: String,
    /// Optional help text surfaced below the prompt.
    pub help_text: Option<String>,
    /// Whether a single or multiple options may be selected.
    pub selection_mode: SelectionMode,
    /// The list of options displayed to the user.
    pub options: Vec<ChoiceOption<V>>,
    /// When `true`, submitting with no selection fails submit-time
    /// validation.
    pub required: bool,
    /// Minimum selections (only honoured by `ChooseMany`).
    pub min_selections: Option<usize>,
    /// Maximum selections (only honoured by `ChooseMany`).
    pub max_selections: Option<usize>,
}

impl<V> ChoiceInput<V> {
    /// Creates a new `Single`-mode input with no options.
    pub fn new(id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            help_text: None,
            selection_mode: SelectionMode::Single,
            options: Vec::new(),
            required: false,
            min_selections: None,
            max_selections: None,
        }
    }

    /// Replaces the options list.
    pub fn with_options(mut self, options: Vec<ChoiceOption<V>>) -> Self {
        self.options = options;
        self
    }

    /// Sets the selection mode.
    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Sets `help_text`.
    pub fn with_help_text(mut self, text: impl Into<String>) -> Self {
        self.help_text = Some(text.into());
        self
    }

    /// Marks the input as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Sets `min_selections` (only meaningful for `Multiple` mode).
    pub fn with_min_selections(mut self, min: usize) -> Self {
        self.min_selections = Some(min);
        self
    }

    /// Sets `max_selections` (only meaningful for `Multiple` mode).
    pub fn with_max_selections(mut self, max: usize) -> Self {
        self.max_selections = Some(max);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_option_new_defaults_disabled_to_false() {
        let option: ChoiceOption = ChoiceOption::new("r", "Red", "red");
        assert_eq!(option.id, "r");
        assert_eq!(option.label, "Red");
        assert_eq!(option.value, "red");
        assert!(!option.disabled);
    }

    #[test]
    fn choice_option_disabled_builder_sets_flag() {
        let option: ChoiceOption = ChoiceOption::new("r", "Red", "red").disabled();
        assert!(option.disabled);
    }

    #[test]
    fn map_value_projects_value_type() {
        let option: ChoiceOption<String> = ChoiceOption::new("one", "One", "1".to_string());
        let projected: ChoiceOption<u32> = option.map_value(|v| v.parse().unwrap());
        assert_eq!(projected.id, "one");
        assert_eq!(projected.label, "One");
        assert_eq!(projected.value, 1);
        assert!(!projected.disabled);
    }

    #[test]
    fn map_value_preserves_disabled_flag() {
        let option: ChoiceOption<String> =
            ChoiceOption::new("one", "One", "1".to_string()).disabled();
        let projected: ChoiceOption<u32> = option.map_value(|v| v.parse().unwrap());
        assert!(projected.disabled);
    }

    #[test]
    fn choice_input_new_defaults_to_single_mode() {
        let input: ChoiceInput = ChoiceInput::new("c", "Pick one");
        assert_eq!(input.selection_mode, SelectionMode::Single);
        assert!(input.options.is_empty());
        assert!(!input.required);
        assert!(input.min_selections.is_none());
        assert!(input.max_selections.is_none());
    }

    #[test]
    fn builder_methods_chain() {
        let input: ChoiceInput = ChoiceInput::new("c", "Pick some")
            .with_selection_mode(SelectionMode::Multiple)
            .required()
            .with_min_selections(1)
            .with_max_selections(3)
            .with_help_text("Choose wisely")
            .with_options(vec![ChoiceOption::new("r", "Red", "red")]);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
        assert!(input.required);
        assert_eq!(input.min_selections, Some(1));
        assert_eq!(input.max_selections, Some(3));
        assert_eq!(input.help_text.as_deref(), Some("Choose wisely"));
        assert_eq!(input.options.len(), 1);
    }
}
