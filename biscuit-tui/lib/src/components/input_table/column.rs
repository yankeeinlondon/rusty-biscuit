//! Column schema definitions for [`InputTable`](super::InputTable).
//!
//! An [`InputTableColumn`] describes both the *type* of cell that
//! occupies that column for every row and the per-column
//! configuration that seeds each cell's initial state. Rows in
//! [`InputTableState`](super::InputTableState) are constructed by
//! pairing the column schema with an optional per-cell initial value
//! supplied by the caller.

use crate::components::choose::ChoiceInput;

/// Per-column configuration for a [`BooleanSwitch`] cell.
///
/// [`BooleanSwitch`]: crate::components::BooleanSwitch
#[derive(Debug, Clone)]
pub struct BooleanSwitchConfig {
    /// Initial value for every cell in this column (rows may
    /// override).
    pub initial: bool,
    /// Caption painted on the "on" side of the switch track.
    pub on_label: String,
    /// Caption painted on the "off" side of the switch track.
    pub off_label: String,
}

impl Default for BooleanSwitchConfig {
    fn default() -> Self {
        Self {
            initial: false,
            on_label: "ON".into(),
            off_label: "OFF".into(),
        }
    }
}

/// Per-column configuration for a [`TextInput`] cell.
///
/// [`TextInput`]: crate::components::TextInput
#[derive(Debug, Clone, Default)]
pub struct TextInputConfig {
    /// Default initial buffer contents (rows may override).
    pub initial: String,
    /// Optional keystroke-time max-length cap.
    pub max_length: Option<usize>,
}

/// Per-column configuration for a [`TextAreaInput`] cell.
///
/// [`TextAreaInput`]: crate::components::TextAreaInput
#[derive(Debug, Clone)]
pub struct TextAreaInputConfig {
    /// Preferred editor width in terminal cells.
    pub preferred_width: u16,
    /// Preferred editor height in terminal cells.
    pub preferred_height: u16,
    /// Default initial buffer contents (rows may override).
    pub initial: Vec<String>,
    /// Whether a scrollbar overlay renders when content overflows.
    pub scrollbar: bool,
}

impl Default for TextAreaInputConfig {
    fn default() -> Self {
        Self {
            preferred_width: 40,
            preferred_height: 3,
            initial: Vec::new(),
            scrollbar: false,
        }
    }
}

/// Column schema — both the cell *type* and its seed configuration.
///
/// Each variant carries a unique `id` that identifies the column for
/// value extraction. For [`ChooseOne`](Self::ChooseOne) and
/// [`ChooseMany`](Self::ChooseMany), the id comes from the
/// [`ChoiceInput::id`] field.
///
/// ## Variants
///
/// - [`InputTableColumn::StaticText`] — a display-only column whose
///   string is painted identically for every row.
/// - [`InputTableColumn::BooleanSwitch`] — every cell renders a
///   [`BooleanSwitch`](crate::components::BooleanSwitch) seeded from
///   [`BooleanSwitchConfig`].
/// - [`InputTableColumn::TextInput`] — every cell renders a
///   [`TextInput`](crate::components::TextInput).
/// - [`InputTableColumn::TextAreaInput`] — every cell renders a
///   [`TextAreaInput`](crate::components::TextAreaInput).
/// - [`InputTableColumn::ChooseOne`] — every cell renders a
///   [`ChooseOne`](crate::components::ChooseOne) driven by the
///   supplied [`ChoiceInput`].
/// - [`InputTableColumn::ChooseMany`] — every cell renders a
///   [`ChooseMany`](crate::components::ChooseMany) driven by the
///   supplied [`ChoiceInput`].
#[derive(Debug, Clone)]
pub enum InputTableColumn {
    /// Display-only column. The string is rendered identically for
    /// every row.
    StaticText {
        /// Column identifier for value extraction.
        id: String,
        /// Text rendered in every cell of this column.
        text: String,
    },
    /// Boolean toggle-switch column.
    BooleanSwitch {
        /// Column identifier for value extraction.
        id: String,
        /// Cell configuration.
        config: BooleanSwitchConfig,
    },
    /// Single-line text input column.
    TextInput {
        /// Column identifier for value extraction.
        id: String,
        /// Cell configuration.
        config: TextInputConfig,
    },
    /// Multi-line text editor column.
    TextAreaInput {
        /// Column identifier for value extraction.
        id: String,
        /// Cell configuration.
        config: TextAreaInputConfig,
    },
    /// Single-select choice column.
    ///
    /// Table choice columns always operate on `ChoiceInput<String>`,
    /// unlike standalone [`ChooseOne<V>`](crate::components::ChooseOne)
    /// which is generic over the value type. This is an intentional v1
    /// limitation to keep the table's cell state management simpler.
    ///
    /// Consumers who need typed values can map via
    /// [`ChoiceOption::map_value`](crate::components::choose::ChoiceOption::map_value)
    /// on individual options before passing them to the table, or project
    /// the extracted `String` values after reading the table's results.
    ChooseOne(ChoiceInput<String>),
    /// Multi-select choice column.
    ///
    /// Table choice columns always operate on `ChoiceInput<String>`,
    /// unlike standalone [`ChooseMany<V>`](crate::components::ChooseMany)
    /// which is generic over the value type. This is an intentional v1
    /// limitation to keep the table's cell state management simpler.
    ///
    /// Consumers who need typed values can map via
    /// [`ChoiceOption::map_value`](crate::components::choose::ChoiceOption::map_value)
    /// on individual options before passing them to the table, or project
    /// the extracted `String` values after reading the table's results.
    ChooseMany(ChoiceInput<String>),
}

impl InputTableColumn {
    /// Returns the column's unique identifier.
    pub fn id(&self) -> &str {
        match self {
            Self::StaticText { id, .. }
            | Self::BooleanSwitch { id, .. }
            | Self::TextInput { id, .. }
            | Self::TextAreaInput { id, .. } => id,
            Self::ChooseOne(input) | Self::ChooseMany(input) => &input.id,
        }
    }

    /// Returns `true` when cells in this column accept keyboard focus.
    pub fn is_focusable(&self) -> bool {
        !matches!(self, Self::StaticText { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::choose::{ChoiceOption, SelectionMode};

    #[test]
    fn static_text_column_is_not_focusable() {
        let column = InputTableColumn::StaticText {
            id: "row".into(),
            text: "Name".into(),
        };
        assert!(!column.is_focusable());
        assert_eq!(column.id(), "row");
    }

    #[test]
    fn boolean_switch_column_is_focusable() {
        let column = InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        };
        assert!(column.is_focusable());
        assert_eq!(column.id(), "active");
    }

    #[test]
    fn text_input_column_default_is_empty() {
        let config = TextInputConfig::default();
        assert_eq!(config.initial, "");
        assert!(config.max_length.is_none());
    }

    #[test]
    fn text_area_input_default_size_is_modest() {
        let config = TextAreaInputConfig::default();
        assert_eq!(config.preferred_width, 40);
        assert_eq!(config.preferred_height, 3);
        assert!(config.initial.is_empty());
        assert!(!config.scrollbar);
    }

    #[test]
    fn boolean_switch_config_default_labels_are_on_off() {
        let config = BooleanSwitchConfig::default();
        assert_eq!(config.on_label, "ON");
        assert_eq!(config.off_label, "OFF");
        assert!(!config.initial);
    }

    #[test]
    fn choose_one_column_carries_input() {
        let input = ChoiceInput::new("c", "p").with_options(vec![ChoiceOption::new("a", "A", "a")]);
        let column = InputTableColumn::ChooseOne(input);
        assert!(column.is_focusable());
    }

    #[test]
    fn choose_many_column_selection_mode_is_not_required_here() {
        // The column constructor accepts any ChoiceInput; ChooseManyState
        // forces selection mode on its own when the cell is built.
        let input: ChoiceInput =
            ChoiceInput::new("c", "p").with_selection_mode(SelectionMode::Single);
        let column = InputTableColumn::ChooseMany(input);
        assert!(column.is_focusable());
    }
}
