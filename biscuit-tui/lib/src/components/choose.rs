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

/// Layout direction for a choice list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    /// One item per row, stacked vertically (the default).
    #[default]
    Vertical,
    /// Items packed left-to-right, wrapping to new rows.
    Horizontal,
}

/// A keyboard shortcut that selects an option without moving the
/// active cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeySpec {
    /// `Ctrl+<char>` shortcut.
    Ctrl(char),
    /// `Alt+<char>` shortcut.
    Alt(char),
}

/// When (if ever) hotkey badges are rendered next to option labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HotkeyDisplayMode {
    /// Badges are never shown (the default).
    #[default]
    Hidden,
    /// Badges shown only while `Ctrl` is held.
    CtrlHeld,
    /// Badges shown only while `Alt` is held.
    AltHeld,
}

/// Background color used for the actively hovered option in a choice
/// list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActiveChoiceColor {
    /// Neutral grey (safe on both light and dark terminals).
    #[default]
    Grey,
    /// Green accent.
    Green,
    /// Yellow accent.
    Yellow,
    /// Red accent.
    Red,
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
    /// Optional keyboard shortcut that selects this option. `None`
    /// means the option has no hotkey at all — no badge, no keypress
    /// binding. Hotkeys are only created by explicit assignment
    /// (`with_hotkey()`, `[CTRL+x]` in CLI options, object-source
    /// `hotkey` field, or `--numeric-hot-keys`).
    pub hotkey: Option<HotkeySpec>,
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
            hotkey: None,
        }
    }

    /// Marks this option as disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Sets a keyboard shortcut for this option.
    pub fn with_hotkey(mut self, hotkey: HotkeySpec) -> Self {
        self.hotkey = Some(hotkey);
        self
    }

    /// Returns the keyboard shortcut bound to this option, if any.
    ///
    /// Returns the explicit `hotkey` set via [`with_hotkey`](Self::with_hotkey),
    /// or `None` when no hotkey was assigned. Disabled options never
    /// have a hotkey.
    pub fn effective_hotkey(&self) -> Option<HotkeySpec> {
        if self.disabled {
            return None;
        }
        self.hotkey
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
            hotkey: self.hotkey,
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
    /// When `true`, the option order is randomised when the state is
    /// built.
    pub shuffle_options: bool,
    /// When `true`, alphanumeric keystrokes open the inline fuzzy
    /// search prompt instead of jumping to a hotkey match.
    ///
    /// Library consumers keep the legacy hotkey shortcut by leaving
    /// this `false` (the default); CLI callers opt into the
    /// search-on-type behaviour by calling
    /// [`with_filter_enabled(true)`](ChoiceInput::with_filter_enabled).
    pub filter_enabled: bool,
    /// Layout direction for the option list.
    pub orientation: Orientation,
    /// Optional ordering applied to the option list before state
    /// construction.
    pub sort: Option<crate::core::SortOrder>,
    /// Background colour used for the actively hovered option.
    ///
    /// Defaults to [`ActiveChoiceColor::Grey`]. The renderer combines
    /// this colour with the detected terminal background to pick a
    /// foreground that meets the spec's contrast requirements (see
    /// [`crate::core::resolve_active_style`]).
    pub active_color: ActiveChoiceColor,
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
            shuffle_options: false,
            filter_enabled: false,
            orientation: Orientation::default(),
            sort: None,
            active_color: ActiveChoiceColor::default(),
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

    /// Sets `shuffle_options`.
    pub fn with_shuffle_options(mut self, shuffle: bool) -> Self {
        self.shuffle_options = shuffle;
        self
    }

    /// Sets `filter_enabled`.
    ///
    /// When enabled, typing an alphanumeric character on a hidden
    /// search prompt opens the inline fuzzy filter and seeds it with
    /// the typed character. When disabled (the default), alphanumeric
    /// keys fall through to the legacy hotkey-jump behaviour.
    ///
    /// CLI callers typically pass `true`; library consumers of the
    /// existing hotkey shortcut leave this `false`.
    pub fn with_filter_enabled(mut self, enabled: bool) -> Self {
        self.filter_enabled = enabled;
        self
    }

    /// Sets the layout `orientation`.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the `sort` order applied before state construction.
    pub fn with_sort(mut self, sort: crate::core::SortOrder) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Sets the [`ActiveChoiceColor`] used for the actively hovered
    /// option's background.
    pub fn with_active_color(mut self, color: ActiveChoiceColor) -> Self {
        self.active_color = color;
        self
    }

    /// Applies the configured [`crate::core::SortOrder`] (if any) to
    /// `self.options` in place.
    ///
    /// Library state constructors call this before building the hotkey
    /// map and `cached_labels` so that `ChoiceInput` is the single
    /// authority on option ordering. CLI callers used to invoke a
    /// duplicate `apply_sort` helper; that helper now delegates to the
    /// configured `with_sort` builder via this method.
    ///
    /// ## Notes
    ///
    /// - Sorts by `option.label` for `Asc`/`Desc`, reverses for
    ///   `Reverse`, and is a no-op for `Natural` or when `sort` is
    ///   `None`.
    /// - The standard library's `slice::sort_by` is stable, so options
    ///   with equal labels keep their relative input order.
    pub(crate) fn sort_options_in_place(&mut self) {
        if let Some(order) = self.sort {
            order.apply(&mut self.options);
        }
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
        assert!(option.hotkey.is_none());
    }

    #[test]
    fn choice_option_with_hotkey_sets_hotkey() {
        let option: ChoiceOption =
            ChoiceOption::new("r", "Red", "red").with_hotkey(HotkeySpec::Ctrl('r'));
        assert_eq!(option.hotkey, Some(HotkeySpec::Ctrl('r')));
    }

    #[test]
    fn effective_hotkey_returns_none_when_unassigned() {
        // An option with no explicit hotkey has no badge / no binding.
        let option: ChoiceOption = ChoiceOption::new("r", "Red", "red");
        assert_eq!(option.effective_hotkey(), None);
    }

    #[test]
    fn effective_hotkey_returns_explicit_assignment() {
        let option: ChoiceOption =
            ChoiceOption::new("r", "Red", "red").with_hotkey(HotkeySpec::Alt('x'));
        assert_eq!(option.effective_hotkey(), Some(HotkeySpec::Alt('x')));
    }

    #[test]
    fn effective_hotkey_ignores_disabled_options() {
        let option: ChoiceOption = ChoiceOption::new("r", "Red", "red")
            .with_hotkey(HotkeySpec::Ctrl('r'))
            .disabled();
        assert_eq!(option.effective_hotkey(), None);
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
    fn map_value_preserves_hotkey() {
        let option: ChoiceOption<String> =
            ChoiceOption::new("one", "One", "1".to_string()).with_hotkey(HotkeySpec::Alt('o'));
        let projected: ChoiceOption<u32> = option.map_value(|v| v.parse().unwrap());
        assert_eq!(projected.hotkey, Some(HotkeySpec::Alt('o')));
    }

    #[test]
    fn choice_input_new_defaults_to_single_mode() {
        let input: ChoiceInput = ChoiceInput::new("c", "Pick one");
        assert_eq!(input.selection_mode, SelectionMode::Single);
        assert!(input.options.is_empty());
        assert!(!input.required);
        assert!(input.min_selections.is_none());
        assert!(input.max_selections.is_none());
        assert!(!input.shuffle_options);
        assert!(!input.filter_enabled);
        assert_eq!(input.orientation, Orientation::Vertical);
        assert!(input.sort.is_none());
        assert_eq!(input.active_color, ActiveChoiceColor::Grey);
    }

    #[test]
    fn with_active_color_sets_the_color() {
        let input: ChoiceInput =
            ChoiceInput::new("c", "Pick one").with_active_color(ActiveChoiceColor::Green);
        assert_eq!(input.active_color, ActiveChoiceColor::Green);
    }

    #[test]
    fn with_filter_enabled_sets_the_flag() {
        let input: ChoiceInput = ChoiceInput::new("c", "Pick one").with_filter_enabled(true);
        assert!(input.filter_enabled);
        let input: ChoiceInput = ChoiceInput::new("c", "Pick one").with_filter_enabled(false);
        assert!(!input.filter_enabled);
    }

    #[test]
    fn builder_methods_chain() {
        let input: ChoiceInput = ChoiceInput::new("c", "Pick some")
            .with_selection_mode(SelectionMode::Multiple)
            .required()
            .with_min_selections(1)
            .with_max_selections(3)
            .with_help_text("Choose wisely")
            .with_shuffle_options(true)
            .with_orientation(Orientation::Horizontal)
            .with_sort(crate::core::SortOrder::Asc)
            .with_options(vec![ChoiceOption::new("r", "Red", "red")]);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
        assert!(input.required);
        assert_eq!(input.min_selections, Some(1));
        assert_eq!(input.max_selections, Some(3));
        assert_eq!(input.help_text.as_deref(), Some("Choose wisely"));
        assert_eq!(input.options.len(), 1);
        assert!(input.shuffle_options);
        assert_eq!(input.orientation, Orientation::Horizontal);
        assert_eq!(input.sort, Some(crate::core::SortOrder::Asc));
    }
}
