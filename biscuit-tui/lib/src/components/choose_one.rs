//! Single-selection list component.
//!
//! [`ChooseOne`] is a zero-sized [`StatefulWidget`] marker that renders
//! a vertical list of [`ChoiceOption`]s. State tracks the currently
//! hovered option, the selected option (if any), a scroll viewport,
//! hotkey mappings, and any submit-time validation error.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let input: ChoiceInput = ChoiceInput::new("colour", "Pick a colour")
//!     .with_options(vec![
//!         ChoiceOption::new("r", "Red", "red"),
//!         ChoiceOption::new("g", "Green", "green"),
//!     ])
//!     .required();
//! let state = ChooseOneState::new(input);
//! assert!(state.selected_index().is_none());
//! ```

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::seq::SliceRandom;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::StatefulWidget,
};

use crate::core::{
    ComponentTheme, EventOutcome, FuzzyFilter, HandleEvent, KeyBindings, Label, StandaloneState,
    TerminalStyle, ValidationState, render_with_label,
};

use super::choice_render::ChoiceRenderContext;

use super::choose::{ChoiceInput, ChoiceOption, HotkeySpec, SelectionMode};

/// Mutable state for a [`ChooseOne`] widget.
///
/// Wraps a [`ChoiceInput<V>`] and adds transient UI state: the
/// hovered row, the selected row (if any), scroll offset, precomputed
/// hotkey map, key bindings, and any active submit-time validation error.
///
/// The type parameter `V` defaults to `String` for backward
/// compatibility.
#[derive(Debug, Clone)]
pub struct ChooseOneState<V = String> {
    input: ChoiceInput<V>,
    hover: usize,
    selected: Option<usize>,
    initial_selected: Option<usize>,
    scroll_offset: usize,
    hotkeys: HashMap<char, usize>,
    ctrl_hotkeys: HashMap<char, usize>,
    alt_hotkeys: HashMap<char, usize>,
    label: Option<Label>,
    theme: ComponentTheme,
    bindings: KeyBindings,
    validation_error: Option<String>,
    filter: FuzzyFilter,
    filter_visible: bool,
    cached_labels: Vec<String>,
}

impl<V: Clone + PartialEq> ChooseOneState<V> {
    /// Builds a new state from a [`ChoiceInput<V>`].
    ///
    /// The selection mode is forced to [`SelectionMode::Single`]
    /// regardless of what `input` specifies — `ChooseOne` is a
    /// single-select component.
    pub fn new(mut input: ChoiceInput<V>) -> Self {
        input.selection_mode = SelectionMode::Single;
        if input.shuffle_options {
            input.options.shuffle(&mut rand::rng());
        }
        let hotkeys = build_hotkeys(&input.options);
        let (ctrl_hotkeys, alt_hotkeys) = build_explicit_hotkeys(&input.options);
        let hover = first_enabled_index(&input.options).unwrap_or(0);
        let cached_labels: Vec<String> = input.options.iter().map(|o| o.label.clone()).collect();
        let mut filter = FuzzyFilter::new();
        filter.clear(&cached_labels);
        Self {
            input,
            hover,
            selected: None,
            initial_selected: None,
            scroll_offset: 0,
            hotkeys,
            ctrl_hotkeys,
            alt_hotkeys,
            label: None,
            theme: ComponentTheme::default(),
            bindings: KeyBindings::default(),
            validation_error: None,
            filter,
            filter_visible: false,
            cached_labels,
        }
    }

    /// Shortcut constructor from a bare vec of options.
    pub fn from_options(options: Vec<ChoiceOption<V>>) -> Self {
        Self::new(ChoiceInput::new("", "").with_options(options))
    }

    /// Attaches a label rendered at the configured position.
    pub fn with_label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Replaces the active theme.
    pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Replaces the key bindings.
    pub fn with_key_bindings(mut self, bindings: KeyBindings) -> Self {
        self.bindings = bindings;
        self
    }

    /// Pre-selects an option by its stable identifier.
    pub fn with_initial_selection(mut self, id: &str) -> Self {
        if let Some((idx, _)) = self
            .input
            .options
            .iter()
            .enumerate()
            .find(|(_, option)| option.id == id)
        {
            self.selected = Some(idx);
            self.initial_selected = Some(idx);
            self.hover = idx;
        }
        self
    }

    /// Pre-selects an option by matching its `value` against `value`.
    ///
    /// Intended for CLI callers that expose `--selected <VALUE>`: the
    /// option's `value` is the authoritative identity once a
    /// `--delimiter` has split a `label⟂value` pair, and may differ
    /// from the option's stable `id` when built from a dictionary
    /// source.
    ///
    /// When no option matches, the state is returned unchanged.
    pub fn with_initial_value(mut self, value: &str) -> Self
    where
        V: PartialEq<str>,
    {
        if let Some((idx, _)) = self
            .input
            .options
            .iter()
            .enumerate()
            .find(|(_, option)| option.value == *value)
        {
            self.selected = Some(idx);
            self.initial_selected = Some(idx);
            self.hover = idx;
        }
        self
    }

    /// Returns the full list of options.
    pub fn options(&self) -> &[ChoiceOption<V>] {
        &self.input.options
    }

    /// Returns the index of the currently hovered option, if the list
    /// is non-empty.
    pub fn hover(&self) -> Option<usize> {
        if self.input.options.is_empty() {
            None
        } else {
            Some(self.hover)
        }
    }

    /// Returns the index of the currently selected option, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// Returns the stable `id` of the selected option, if any.
    pub fn selected_id(&self) -> Option<&str> {
        self.selected
            .and_then(|idx| self.input.options.get(idx))
            .map(|option| option.id.as_str())
    }

    /// Returns the typed `value` of the selected option, if any.
    pub fn selected_value(&self) -> Option<&V> {
        self.selected
            .and_then(|idx| self.input.options.get(idx))
            .map(|option| &option.value)
    }

    /// Returns `true` when the input was marked required.
    pub fn required(&self) -> bool {
        self.input.required
    }

    /// Returns a reference to the attached label, if any.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns a reference to the component theme.
    pub fn theme(&self) -> &ComponentTheme {
        &self.theme
    }

    /// Returns the precomputed hotkey map keyed by lowercase char.
    pub fn hotkeys(&self) -> &HashMap<char, usize> {
        &self.hotkeys
    }

    /// Returns the explicit Ctrl hotkey map keyed by lowercase char.
    pub fn ctrl_hotkeys(&self) -> &HashMap<char, usize> {
        &self.ctrl_hotkeys
    }

    /// Returns the explicit Alt hotkey map keyed by lowercase char.
    pub fn alt_hotkeys(&self) -> &HashMap<char, usize> {
        &self.alt_hotkeys
    }

    /// Returns a reference to the key bindings.
    pub fn key_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    /// Sets an inline validation error.
    pub fn set_validation_error(&mut self, message: impl Into<String>) {
        self.validation_error = Some(message.into());
    }

    /// Returns whether the inline fuzzy search prompt row is currently
    /// rendered above the list.
    pub fn filter_visible(&self) -> bool {
        self.filter_visible
    }

    /// Returns the live fuzzy filter pattern buffer.
    pub fn filter_pattern(&self) -> &str {
        self.filter.pattern()
    }

    /// Returns the indices (into [`options`](Self::options)) that
    /// currently pass the fuzzy filter.
    ///
    /// When no filter is active the returned slice contains every
    /// index in source order.
    pub fn visible_indices(&self) -> &[usize] {
        self.filter.visible()
    }
}

impl<V: Clone + PartialEq> ValidationState for ChooseOneState<V> {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl<V: Clone + PartialEq> StandaloneState for ChooseOneState<V> {
    type Value = Option<V>;

    fn value(&self) -> Self::Value {
        self.selected_value().cloned()
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn help_hint(&self) -> &str {
        &self.theme.help_hint
    }
}

/// Single-selection list widget.
///
/// Rendered via [`StatefulWidget`] against a [`ChooseOneState<V>`]. The
/// widget itself is zero-sized — all state lives on the paired state
/// struct.
///
/// The type parameter `V` must match the state's value type. Defaults
/// to `String` for backward compatibility.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChooseOne<V = String> {
    _phantom: std::marker::PhantomData<V>,
}

impl<V> ChooseOne<V> {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<V: Clone + PartialEq> StatefulWidget for ChooseOne<V> {
    type State = ChooseOneState<V>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let label = state.label.clone();
        let label_style = state.theme.label_style;

        let inner_area = render_with_label(area, buf, label.as_ref(), label_style, |rect, b| {
            let list_area = if state.filter_visible && rect.height > 0 {
                draw_search_prompt(Rect::new(rect.x, rect.y, rect.width, 1), b, state);
                Rect::new(rect.x, rect.y + 1, rect.width, rect.height - 1)
            } else {
                rect
            };

            let visible_indices: Vec<usize> = state.filter.visible().to_vec();
            let body_rows = if state.validation_error.is_some() && list_area.height > 1 {
                list_area.height - 1
            } else {
                list_area.height
            };
            let visible = body_rows as usize;
            adjust_scroll(state, visible, &visible_indices);

            let ctx = ChoiceRenderContext::for_single(
                &state.theme,
                TerminalStyle::default(),
                state.input.orientation,
            );
            ctx.render(
                list_area,
                b,
                &state.input.options,
                &visible_indices,
                state.scroll_offset,
                state.hover,
                |idx| Some(idx) == state.selected,
                Some(&mut state.filter),
                state.validation_error.as_deref(),
            );
        });

        if let Some(message) = state.validation_error.as_deref()
            && let Some(error_row) = error_row_y(inner_area, state.input.options.len(), state)
        {
            let error_line = Line::from(Span::styled(message.to_string(), state.theme.error_style));
            buf.set_line(inner_area.x, error_row, &error_line, inner_area.width);
        }
    }
}

fn draw_search_prompt<V: Clone + PartialEq>(
    area: Rect,
    buf: &mut Buffer,
    state: &ChooseOneState<V>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text = format!("{}{}", state.theme.search_indicator, state.filter.pattern());
    let line = Line::from(Span::styled(text, state.theme.search_style));
    buf.set_line(area.x, area.y, &line, area.width);
}

impl<V: Clone + PartialEq> HandleEvent for ChooseOne<V> {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        // Cancel binding: when a fuzzy filter is active, the first
        // press clears it; subsequent presses restore the initial
        // selection and submit.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            if state.filter_visible && state.filter.is_active() {
                state.filter.clear(&state.cached_labels);
                state.filter_visible = false;
                snap_hover_to_visible(state);
                return EventOutcome::Consumed;
            }
            if state.filter_visible {
                state.filter_visible = false;
                state.filter.clear(&state.cached_labels);
                snap_hover_to_visible(state);
                return EventOutcome::Consumed;
            }
            // Phase 5: Esc restores the initial selection and submits.
            state.selected = state.initial_selected;
            return EventOutcome::Submitted;
        }

        // When the search prompt is showing, route printable chars and
        // backspace to the pattern buffer BEFORE checking nav bindings
        // (otherwise `j`/`k` would collide with vim-style up/down).
        if state.filter_visible && event.modifiers == KeyModifiers::NONE {
            match event.code {
                KeyCode::Backspace => {
                    state.filter.pop_char(&state.cached_labels);
                    snap_hover_to_visible(state);
                    state.validation_error = None;
                    return EventOutcome::Consumed;
                }
                KeyCode::Char(c) if c != ' ' => {
                    state.filter.push_char(c, &state.cached_labels);
                    snap_hover_to_visible(state);
                    state.validation_error = None;
                    return EventOutcome::Consumed;
                }
                _ => {}
            }
        }

        if KeyBindings::matches(&state.bindings.submit, &event) {
            return submit(state);
        }
        if KeyBindings::matches(&state.bindings.toggle, &event) {
            select_hover(state);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.up, &event) {
            move_hover(state, -1);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.down, &event) {
            move_hover(state, 1);
            return EventOutcome::Consumed;
        }

        // Ctrl/Alt hotkeys select and submit (Phase 5).
        match event.modifiers {
            KeyModifiers::CONTROL => {
                if let KeyCode::Char(c) = event.code
                    && let Some(&idx) = state.ctrl_hotkeys.get(&c.to_ascii_lowercase())
                {
                    select_at(state, idx);
                    return EventOutcome::Submitted;
                }
            }
            KeyModifiers::ALT => {
                if let KeyCode::Char(c) = event.code
                    && let Some(&idx) = state.alt_hotkeys.get(&c.to_ascii_lowercase())
                {
                    select_at(state, idx);
                    return EventOutcome::Submitted;
                }
            }
            _ => {}
        }

        // Home/End/vim-style jumps, hotkey or filter-open, only when
        // the search prompt is hidden.
        if !state.filter_visible && event.modifiers.is_empty() {
            match event.code {
                KeyCode::Home | KeyCode::Char('g') => {
                    jump_to(
                        state,
                        first_enabled_index(&state.input.options).unwrap_or(0),
                    );
                    return EventOutcome::Consumed;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    jump_to(
                        state,
                        last_enabled_index(&state.input.options)
                            .unwrap_or(state.input.options.len().saturating_sub(1)),
                    );
                    return EventOutcome::Consumed;
                }
                KeyCode::Char(c) => {
                    if state.input.filter_enabled && c.is_alphanumeric() {
                        state.filter.clear(&state.cached_labels);
                        state.filter.push_char(c, &state.cached_labels);
                        state.filter_visible = true;
                        snap_hover_to_visible(state);
                        state.validation_error = None;
                        return EventOutcome::Consumed;
                    }
                    if let Some(&idx) = state.hotkeys.get(&c.to_ascii_lowercase()) {
                        jump_to(state, idx);
                        select_at(state, idx);
                        return EventOutcome::Consumed;
                    }
                }
                _ => {}
            }
        }

        EventOutcome::Ignored
    }
}

fn submit<V: Clone + PartialEq>(state: &mut ChooseOneState<V>) -> EventOutcome {
    // Block submit while a filter is active but no option matches.
    if state.filter_visible && state.filter.visible().is_empty() && !state.input.options.is_empty()
    {
        return EventOutcome::Consumed;
    }
    // Phase 5: Enter always selects the active enabled item.
    if let Some(idx) = state.hover()
        && !state.input.options[idx].disabled
        && state.filter.visible().contains(&idx)
    {
        state.selected = Some(idx);
    }
    if state.selected.is_none() && state.input.required {
        state.validation_error = Some("Please make a selection".into());
        return EventOutcome::Consumed;
    }
    EventOutcome::Submitted
}

fn select_hover<V: Clone + PartialEq>(state: &mut ChooseOneState<V>) {
    let idx = state.hover;
    select_at(state, idx);
}

fn select_at<V: Clone + PartialEq>(state: &mut ChooseOneState<V>, idx: usize) {
    if let Some(option) = state.input.options.get(idx)
        && !option.disabled
    {
        state.selected = Some(idx);
        state.validation_error = None;
    }
}

fn move_hover<V: Clone + PartialEq>(state: &mut ChooseOneState<V>, delta: i32) {
    if state.input.options.is_empty() {
        return;
    }
    let visible = state.filter.visible();
    if visible.is_empty() {
        return;
    }
    let len = visible.len() as i32;
    let start = visible
        .iter()
        .position(|&i| i == state.hover)
        .map(|p| p as i32)
        .unwrap_or(0);
    let mut pos = start;
    for _ in 0..visible.len() {
        pos += delta;
        if pos < 0 {
            pos += len;
        } else if pos >= len {
            pos -= len;
        }
        let candidate = visible[pos as usize];
        if !state.input.options[candidate].disabled {
            state.hover = candidate;
            return;
        }
    }
}

/// Snaps `state.hover` to the first enabled index in `filter.visible()`
/// when the current hover is no longer visible. Leaves the hover
/// unchanged when every visible option is disabled or when the current
/// hover is already both visible and enabled.
fn snap_hover_to_visible<V: Clone + PartialEq>(state: &mut ChooseOneState<V>) {
    let visible = state.filter.visible();
    if visible.is_empty() {
        return;
    }
    let current_ok = visible.contains(&state.hover)
        && state
            .input
            .options
            .get(state.hover)
            .is_some_and(|o| !o.disabled);
    if current_ok {
        return;
    }
    for &idx in visible {
        if !state.input.options[idx].disabled {
            state.hover = idx;
            return;
        }
    }
}

fn jump_to<V: Clone + PartialEq>(state: &mut ChooseOneState<V>, idx: usize) {
    if state.input.options.get(idx).is_some_and(|o| !o.disabled) {
        state.hover = idx;
    }
}

pub(super) fn first_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> {
    options.iter().position(|option| !option.disabled)
}

pub(super) fn last_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> {
    options.iter().rposition(|option| !option.disabled)
}

pub(super) fn build_hotkeys<V>(options: &[ChoiceOption<V>]) -> HashMap<char, usize> {
    let mut map = HashMap::new();
    for (idx, option) in options.iter().enumerate() {
        if option.disabled {
            continue;
        }
        if let Some(first) = option.label.chars().next() {
            let lower = first.to_ascii_lowercase();
            map.entry(lower).or_insert(idx);
        }
    }
    map
}

/// Builds separate maps for explicit Ctrl and Alt hotkeys declared on
/// [`ChoiceOption::hotkey`].
pub(super) fn build_explicit_hotkeys<V>(
    options: &[ChoiceOption<V>],
) -> (HashMap<char, usize>, HashMap<char, usize>) {
    let mut ctrl = HashMap::new();
    let mut alt = HashMap::new();
    for (idx, option) in options.iter().enumerate() {
        if option.disabled {
            continue;
        }
        if let Some(hotkey) = option.hotkey {
            match hotkey {
                HotkeySpec::Ctrl(c) => {
                    ctrl.entry(c.to_ascii_lowercase()).or_insert(idx);
                }
                HotkeySpec::Alt(c) => {
                    alt.entry(c.to_ascii_lowercase()).or_insert(idx);
                }
            }
        }
    }
    (ctrl, alt)
}

fn error_row_y<V: Clone + PartialEq>(
    inner_area: Rect,
    option_count: usize,
    state: &ChooseOneState<V>,
) -> Option<u16> {
    if inner_area.height == 0 {
        return None;
    }
    let search_rows: u16 = if state.filter_visible { 1 } else { 0 };
    let available_for_list = inner_area.height.saturating_sub(search_rows);
    let content_rows = if state.filter_visible && state.filter.visible().is_empty() {
        1
    } else {
        option_count.min(available_for_list as usize) as u16
    };
    let candidate = inner_area.y + search_rows + content_rows;
    if candidate < inner_area.y + inner_area.height {
        Some(candidate)
    } else {
        None
    }
}

fn adjust_scroll<V: Clone + PartialEq>(
    state: &mut ChooseOneState<V>,
    visible: usize,
    visible_indices: &[usize],
) {
    let visible_len = visible_indices.len();
    if visible == 0 || visible_len == 0 {
        state.scroll_offset = 0;
        return;
    }
    // Translate `state.hover` into its position inside the filtered list
    // so scroll tracking works whether or not a filter is active.
    let hover_pos = visible_indices
        .iter()
        .position(|&i| i == state.hover)
        .unwrap_or(0);

    if hover_pos < state.scroll_offset {
        state.scroll_offset = hover_pos;
    } else if hover_pos >= state.scroll_offset + visible {
        state.scroll_offset = hover_pos + 1 - visible;
    }
    if state.scroll_offset + visible > visible_len {
        state.scroll_offset = visible_len.saturating_sub(visible);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Label, LabelPosition};
    use crossterm::event::{KeyCode, KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn fixture_input() -> ChoiceInput<String> {
        ChoiceInput::new("colour", "Pick a colour").with_options(vec![
            ChoiceOption::new("r", "Red", "red"),
            ChoiceOption::new("g", "Green", "green"),
            ChoiceOption::new("b", "Blue", "blue"),
        ])
    }

    fn buffer_row(buf: &Buffer, y: u16) -> String {
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row.trim_end().to_string()
    }

    #[test]
    fn new_starts_with_hover_on_first_enabled_option() {
        let state = ChooseOneState::new(fixture_input());
        assert_eq!(state.hover(), Some(0));
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn hover_skips_disabled_options() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "A", "a").disabled(),
            ChoiceOption::new("b", "B", "b"),
        ]);
        let state = ChooseOneState::new(input);
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn down_arrow_moves_hover_forward() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn up_arrow_wraps_around() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn vim_j_and_k_navigate() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('j')));
        assert_eq!(state.hover(), Some(1));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('k')));
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn space_selects_hovered_option() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_value(), Some(&"green".to_string()));
    }

    #[test]
    fn hotkey_selects_target_option_directly() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_index(), Some(2));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn hotkey_is_case_insensitive() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('R')));
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn space_on_disabled_option_is_ignored() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "A", "a"),
            ChoiceOption::new("b", "B", "b").disabled(),
        ]);
        let mut state = ChooseOneState::new(input);
        state.hover = 1;
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn required_validation_fires_when_fallback_cannot_find_enabled_option() {
        // With fallback-submit-on-active, Enter without an explicit selection
        // promotes the hovered option to selected. When every option is
        // disabled, fallback cannot find a non-disabled hover, so the
        // required-input validation error fires.
        let input = ChoiceInput::<String>::new("colour", "Pick a colour")
            .with_options(vec![
                ChoiceOption::new("r", "Red", "red").disabled(),
                ChoiceOption::new("g", "Green", "green").disabled(),
            ])
            .required();
        let mut state = ChooseOneState::new(input);
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(
            <ChooseOneState as ValidationState>::validation_error(&state)
                .unwrap()
                .contains("selection"),
        );
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn enter_after_selection_submits_even_when_required() {
        let input = fixture_input().required();
        let mut state = ChooseOneState::new(input);
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn selecting_option_clears_required_validation_error() {
        let input = fixture_input().required();
        let mut state = ChooseOneState::new(input);

        // Seed a validation error as if a prior submit had been blocked.
        state.set_validation_error("Please make a selection");
        assert!(<ChooseOneState as ValidationState>::validation_error(&state).is_some());

        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(<ChooseOneState as ValidationState>::validation_error(&state).is_none());
        assert_eq!(state.selected_value(), Some(&"red".to_string()));
    }

    #[test]
    fn enter_without_selection_not_required_submits() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn fallback_submit_promotes_hover() {
        // Enter with no explicit selection should promote the currently
        // hovered option to `selected` and submit.
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover(), Some(1));
        assert!(state.selected_index().is_none());

        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_value(), Some(&"green".to_string()));
    }

    #[test]
    fn fallback_submit_skips_disabled_hover() {
        // If the hovered option is disabled, fallback must not promote it.
        // Under `required`, the validation error must fire.
        let input = ChoiceInput::<String>::new("colour", "Pick a colour")
            .with_options(vec![
                ChoiceOption::new("r", "Red", "red").disabled(),
                ChoiceOption::new("g", "Green", "green"),
            ])
            .required();
        let mut state = ChooseOneState::new(input);
        // Force hover onto the disabled row (bypassing the navigation
        // filter that normally skips disabled options).
        state.hover = 0;

        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.selected_index().is_none());
        assert!(<ChooseOneState as ValidationState>::validation_error(&state).is_some());
    }

    #[test]
    fn fallback_submit_promotes_hover_even_when_not_required() {
        // When not required, fallback still promotes the hovered option so
        // the caller receives a value rather than `None`.
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_value(), Some(&"red".to_string()));
    }

    #[test]
    fn esc_restores_initial_and_submits() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn esc_after_navigation_restores_initial_selection() {
        let input = fixture_input();
        let mut state = ChooseOneState::new(input).with_initial_selection("r");
        assert_eq!(state.selected_index(), Some(0));

        // Navigate away and change selection.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_index(), Some(1));

        // Esc restores the initial selection and submits.
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn esc_after_space_restores_initial_selection() {
        let input = fixture_input();
        let mut state = ChooseOneState::new(input).with_initial_selection("g");
        assert_eq!(state.selected_index(), Some(1));

        // Space selects the hovered option (which is also index 1 initially).
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_index(), Some(1));

        // Navigate and Space again to change selection.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_index(), Some(2));

        // Esc restores the initial selection and submits.
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn modifier_chords_are_ignored() {
        let mut state = ChooseOneState::new(fixture_input());
        let event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        let outcome = ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn initial_selection_pins_hover_and_selected() {
        let state = ChooseOneState::new(fixture_input()).with_initial_selection("g");
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn initial_value_pre_selects() {
        // `with_initial_value` matches the option's `value` field
        // (rather than its `id`). The fixture above uses the same
        // string for id/label/value, so we build a fresh fixture that
        // distinguishes them to pin the behaviour.
        let input: ChoiceInput<String> =
            ChoiceInput::new("colour", "Pick a colour").with_options(vec![
                ChoiceOption::new("r", "Red", "red-value"),
                ChoiceOption::new("g", "Green", "green-value"),
                ChoiceOption::new("b", "Blue", "blue-value"),
            ]);
        let state = ChooseOneState::new(input).with_initial_value("green-value");
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.hover(), Some(1));
        assert_eq!(state.selected_value(), Some(&"green-value".to_string()));
    }

    #[test]
    fn initial_value_with_no_match_leaves_state_unchanged() {
        let state = ChooseOneState::new(fixture_input()).with_initial_value("nope");
        assert!(state.selected_index().is_none());
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn hotkeys_map_lowercased_first_chars() {
        let state = ChooseOneState::new(fixture_input());
        assert_eq!(state.hotkeys().get(&'r'), Some(&0));
        assert_eq!(state.hotkeys().get(&'g'), Some(&1));
        assert_eq!(state.hotkeys().get(&'b'), Some(&2));
    }

    #[test]
    fn hotkey_duplicates_go_to_first_occurrence() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a1", "Apple", "apple"),
            ChoiceOption::new("a2", "Avocado", "avocado"),
        ]);
        let state = ChooseOneState::new(input);
        assert_eq!(state.hotkeys().get(&'a'), Some(&0));
    }

    #[test]
    fn explicit_ctrl_hotkey_selects_and_submits() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "Apple", "apple").with_hotkey(HotkeySpec::Ctrl('a')),
            ChoiceOption::new("b", "Banana", "banana"),
        ]);
        let mut state = ChooseOneState::new(input);
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let outcome = ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn explicit_alt_hotkey_selects_and_submits() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "Apple", "apple"),
            ChoiceOption::new("b", "Banana", "banana").with_hotkey(HotkeySpec::Alt('b')),
        ]);
        let mut state = ChooseOneState::new(input);
        let event = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
        let outcome = ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn explicit_hotkey_is_case_insensitive() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "Apple", "apple").with_hotkey(HotkeySpec::Ctrl('A')),
        ]);
        let mut state = ChooseOneState::new(input);
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let outcome = ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn render_draws_indicator_and_label_per_row() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);
        // Focus prefix (▶ + space) on hovered row 0, blank padding on others
        assert!(buffer_row(&buf, 0).starts_with("▶ ● Red"));
        assert!(buffer_row(&buf, 1).starts_with("  ○ Green"));
        assert!(buffer_row(&buf, 2).starts_with("  ○ Blue"));
    }

    #[test]
    fn render_with_label_above_draws_label_then_list() {
        let mut state = ChooseOneState::new(fixture_input())
            .with_label(Label::new("Colour", LabelPosition::Above));
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Colour");
        // First option has focus prefix (hover defaults to 0)
        assert!(buffer_row(&buf, 1).starts_with("▶ ○ Red"));
    }

    #[test]
    fn render_draws_validation_error_on_row_below_list() {
        let input = fixture_input().required();
        let mut state = ChooseOneState::new(input);
        state.set_validation_error("Please make a selection");
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 3), "Please make a selection");
    }

    #[test]
    fn scroll_offset_tracks_hover_past_visible_window() {
        let options: Vec<ChoiceOption<String>> = (0..6)
            .map(|i| {
                ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("value{i}"))
            })
            .collect();
        let mut state = ChooseOneState::new(ChoiceInput::new("x", "P").with_options(options));
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        for _ in 0..4 {
            ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        }
        ChooseOne::new().render(area, &mut buf, &mut state);
        assert!(state.scroll_offset > 0);
        assert!(buffer_row(&buf, 2).contains("Option 4"));
    }

    #[test]
    fn custom_submit_binding_overrides_default() {
        let bindings = KeyBindings {
            submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
            ..KeyBindings::default()
        };
        let mut state = ChooseOneState::new(fixture_input()).with_key_bindings(bindings);
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        // Ctrl-S should submit.
        let outcome = ChooseOne::new().handle_event(
            &mut state,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
        assert_eq!(outcome, EventOutcome::Submitted);

        // Enter should be ignored (no longer bound to submit).
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn custom_cancel_binding_restores_and_submits() {
        let bindings = KeyBindings {
            cancel: vec![press(KeyCode::Char('q'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseOneState::new(fixture_input()).with_key_bindings(bindings);

        // q should restore initial selection and submit.
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('q')));
        assert_eq!(outcome, EventOutcome::Submitted);

        // Esc should be ignored (no longer bound to cancel).
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn custom_up_down_bindings_work() {
        let bindings = KeyBindings {
            up: vec![press(KeyCode::Char('w'))],
            down: vec![press(KeyCode::Char('s'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseOneState::new(fixture_input()).with_key_bindings(bindings);
        assert_eq!(state.hover, 0);

        // s should move down.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('s')));
        assert_eq!(state.hover, 1);

        // w should move up.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('w')));
        assert_eq!(state.hover, 0);

        // Arrow keys should be ignored (no longer bound).
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover, 0);
    }

    #[test]
    fn typed_value_u32_flow() {
        // Build from strings, project to u32.
        let options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("one", "One", "1"),
            ChoiceOption::new("two", "Two", "2"),
            ChoiceOption::new("three", "Three", "3"),
        ];
        let typed_options: Vec<ChoiceOption<u32>> = options
            .into_iter()
            .map(|opt| opt.map_value(|v| v.parse::<u32>().unwrap()))
            .collect();
        let input: ChoiceInput<u32> =
            ChoiceInput::new("num", "Pick a number").with_options(typed_options);
        let mut state: ChooseOneState<u32> = ChooseOneState::new(input);

        // Select the second option (Two -> 2).
        ChooseOne::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseOne::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        assert_eq!(state.selected_value(), Some(&2_u32));
        assert_eq!(state.value(), Some(2_u32));
    }

    #[test]
    fn typed_value_with_map_value() {
        // Use the ChoiceOption::map_value helper.
        let option: ChoiceOption<String> = ChoiceOption::new("x", "X", "42");
        let mapped: ChoiceOption<i32> = option.map_value(|v| v.parse().unwrap());
        assert_eq!(mapped.value, 42);
        assert_eq!(mapped.id, "x");
        assert_eq!(mapped.label, "X");
    }

    #[test]
    fn typed_value_enum_flow() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        let options: Vec<ChoiceOption<Color>> = vec![
            ChoiceOption::new("r", "Red", Color::Red),
            ChoiceOption::new("g", "Green", Color::Green),
            ChoiceOption::new("b", "Blue", Color::Blue),
        ];
        let input: ChoiceInput<Color> =
            ChoiceInput::new("color", "Pick a color").with_options(options);
        let mut state: ChooseOneState<Color> = ChooseOneState::new(input);

        ChooseOne::<Color>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_value(), Some(&Color::Red));
        assert_eq!(state.value(), Some(Color::Red));
    }

    #[test]
    fn render_overflow_indicators_when_scrolled() {
        // 6 options, 3 rows tall, hover on option 3 (scrolls to show 1-3)
        let options: Vec<ChoiceOption<String>> = (0..6)
            .map(|i| ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("val{i}")))
            .collect();
        let input = ChoiceInput::new("test", "Test").with_options(options);
        let mut state = ChooseOneState::new(input);

        // Move hover to option 3
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover, 3);

        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        // scroll_offset will be 1 (hover=3, visible=3, offset = 3+1-3 = 1), showing options 1,2,3
        // Top row should contain overflow up indicator (option 0 is hidden)
        let top_row = buffer_row(&buf, 0);
        assert!(
            top_row.contains("▲"),
            "Expected ▲ in top row, got: {top_row}"
        );

        // Bottom row should contain overflow down indicator (options 4,5 are hidden)
        let bottom_row = buffer_row(&buf, 2);
        assert!(
            bottom_row.contains("▼"),
            "Expected ▼ in bottom row, got: {bottom_row}"
        );

        // Bottom row (option 3, which is hovered) should have focus prefix
        let bottom_visible_row = buffer_row(&buf, 2);
        assert!(
            bottom_visible_row.starts_with("▶ "),
            "Expected focus prefix on hovered row: {bottom_visible_row}"
        );
        assert!(
            bottom_visible_row.contains("Option 3"),
            "Expected hovered option 3 in bottom visible row: {bottom_visible_row}"
        );
    }

    #[test]
    fn render_disabled_row_with_dim_style() {
        let mut options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("a", "Active", "active".to_string()),
            ChoiceOption::new("d", "Disabled", "disabled".to_string()),
        ];
        options[1].disabled = true;
        let input = ChoiceInput::new("test", "Test").with_options(options);
        let mut state = ChooseOneState::new(input);

        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        // Row 1 is disabled; check that the label span has disabled style
        let y = 1;
        let mut found_disabled = false;
        for x in 0..area.width {
            let cell = &buf[(area.x + x, area.y + y)];
            if cell.symbol() == "D" {
                // First char of "Disabled"
                assert!(
                    cell.style().fg == Some(ratatui::style::Color::DarkGray)
                        || cell
                            .style()
                            .add_modifier
                            .contains(ratatui::style::Modifier::DIM),
                    "Disabled row label should have DarkGray or DIM style"
                );
                found_disabled = true;
                break;
            }
        }
        assert!(
            found_disabled,
            "Did not find disabled label 'Disabled' in buffer"
        );
    }

    #[test]
    fn shuffle_false_preserves_order() {
        let input: ChoiceInput<String> = ChoiceInput::new("x", "P")
            .with_shuffle_options(false)
            .with_options(vec![
                ChoiceOption::new("a", "Alpha", "alpha"),
                ChoiceOption::new("b", "Beta", "beta"),
                ChoiceOption::new("c", "Charlie", "charlie"),
            ]);
        let state = ChooseOneState::new(input);
        let labels: Vec<&str> = state.options().iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Alpha", "Beta", "Charlie"]);
    }

    #[test]
    fn shuffle_randomises_order_choose_one() {
        let options: Vec<ChoiceOption<String>> = (0..20)
            .map(|i| {
                ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("value{i}"))
            })
            .collect();
        let original_labels: Vec<String> = options.iter().map(|o| o.label.clone()).collect();
        let input = ChoiceInput::new("x", "P")
            .with_shuffle_options(true)
            .with_options(options);
        let state = ChooseOneState::new(input);
        let shuffled_labels: Vec<&str> = state.options().iter().map(|o| o.label.as_str()).collect();
        let same_set: std::collections::HashSet<&str> = shuffled_labels.iter().copied().collect();
        let original_set: std::collections::HashSet<&str> =
            original_labels.iter().map(|s| s.as_str()).collect();
        assert_eq!(same_set, original_set);
        assert_ne!(
            shuffled_labels,
            original_labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            "With 20 options it is astronomically unlikely the order is unchanged"
        );
    }

    #[test]
    fn shuffle_then_select_choose_one() {
        let options: Vec<ChoiceOption<String>> = (0..10)
            .map(|i| ChoiceOption::new(format!("id{i}"), format!("Opt{i}"), format!("val{i}")))
            .collect();
        let input = ChoiceInput::new("x", "P")
            .with_shuffle_options(true)
            .with_options(options);
        let mut state = ChooseOneState::new(input);
        let idx = state.hover().unwrap();
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_index(), Some(idx));
        assert!(state.selected_value().is_some());
    }

    #[test]
    fn shuffle_preserves_hotkey_mapping() {
        let input: ChoiceInput<String> = ChoiceInput::new("x", "P")
            .with_shuffle_options(true)
            .with_options(vec![
                ChoiceOption::new("a", "Alpha", "alpha"),
                ChoiceOption::new("b", "Beta", "beta"),
                ChoiceOption::new("c", "Charlie", "charlie"),
            ]);
        let state = ChooseOneState::new(input);
        let mut matched = 0;
        for (idx, option) in state.options().iter().enumerate() {
            let first = option.label.chars().next().unwrap().to_ascii_lowercase();
            if let Some(&mapped_idx) = state.hotkeys().get(&first)
                && mapped_idx == idx
            {
                matched += 1;
            }
        }
        assert!(matched > 0, "At least one hotkey should map to its option");
    }

    // -----------------------------------------------------------------
    // Phase 8 — Search Prompt Rendering & State Plumbing
    // -----------------------------------------------------------------

    fn filter_fixture_input() -> ChoiceInput<String> {
        ChoiceInput::new("fruit", "Pick a fruit")
            .with_filter_enabled(true)
            .with_options(vec![
                ChoiceOption::new("a", "Apple", "apple"),
                ChoiceOption::new("b", "Banana", "banana"),
                ChoiceOption::new("c", "Blueberry", "blueberry"),
                ChoiceOption::new("d", "Cherry", "cherry"),
            ])
    }

    #[test]
    fn filter_visible_starts_false() {
        let state = ChooseOneState::new(filter_fixture_input());
        assert!(!state.filter_visible());
        assert_eq!(state.filter_pattern(), "");
        assert_eq!(state.visible_indices(), &[0, 1, 2, 3]);
    }

    #[test]
    fn typing_letter_opens_filter() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('B')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.filter_visible());
        assert_eq!(state.filter_pattern(), "B");
        let visible = state.visible_indices().to_vec();
        assert!(visible.contains(&1), "Banana should match 'B'");
        assert!(visible.contains(&2), "Blueberry should match 'B'");
        assert!(!visible.contains(&0), "Apple should not match 'B'");
    }

    #[test]
    fn typing_letter_with_filter_disabled_falls_back_to_hotkey() {
        let input: ChoiceInput<String> = ChoiceInput::new("x", "p")
            .with_filter_enabled(false)
            .with_options(vec![
                ChoiceOption::new("a", "Apple", "apple"),
                ChoiceOption::new("b", "Banana", "banana"),
            ]);
        let mut state = ChooseOneState::new(input);
        ChooseOne::<String>::new().handle_event(&mut state, press(KeyCode::Char('b')));
        assert!(!state.filter_visible());
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn backspace_pops_filter_character() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('l')));
        assert_eq!(state.filter_pattern(), "bl");
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Backspace));
        assert_eq!(state.filter_pattern(), "b");
    }

    #[test]
    fn down_walks_filtered_indices() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        // After filtering on "b", visible contains Banana and Blueberry.
        let visible = state.visible_indices().to_vec();
        assert!(!visible.is_empty());
        let first = visible[0];
        assert_eq!(state.hover(), Some(first));

        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        // Hover must now be on another visible index, not some non-visible one.
        let hover = state.hover().unwrap();
        assert!(visible.contains(&hover), "hover {hover} must be visible");
        assert_ne!(hover, first);
    }

    #[test]
    fn esc_clears_filter_first_then_submits() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        assert!(state.filter_visible());

        // First Esc: clears + hides filter, stays consumed.
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(!state.filter_visible());
        assert_eq!(state.filter_pattern(), "");

        // Second Esc: restores initial selection and submits.
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn submit_blocked_when_filter_hides_everything() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('z')));
        assert!(state.filter_visible());
        assert!(state.visible_indices().is_empty());

        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn render_draws_search_prompt_row_when_filter_visible() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);
        let prompt = buffer_row(&buf, 0);
        assert!(
            prompt.starts_with("/ b"),
            "expected '/ b' prompt at top, got {prompt:?}"
        );
    }

    #[test]
    fn render_shows_no_matches_row_when_filter_matches_nothing() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('z')));
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);
        // Row 0 is the search prompt; row 1 is the "(no matches)" row.
        assert_eq!(buffer_row(&buf, 1), "(no matches)");
    }

    #[test]
    fn esc_with_empty_filter_still_hides_prompt_then_restores() {
        // Pattern is active, Esc clears + hides it, second Esc restores + submits.
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Backspace));
        assert!(state.filter_visible());
        assert_eq!(state.filter_pattern(), "");

        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(!state.filter_visible());
    }

    #[test]
    fn filter_open_hover_snaps_to_first_match() {
        // Hover starts on Apple (idx 0). Type 'b' → visible = [Banana, Blueberry].
        // Hover must snap to one of the visible indices.
        let mut state = ChooseOneState::new(filter_fixture_input());
        assert_eq!(state.hover(), Some(0));
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        let hover = state.hover().unwrap();
        let visible = state.visible_indices().to_vec();
        assert!(
            visible.contains(&hover),
            "hover {hover} must be in visible {visible:?}"
        );
        assert_ne!(hover, 0);
    }

    #[test]
    fn submit_via_enter_after_filter_selects_hovered_match() {
        let mut state = ChooseOneState::new(filter_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        // Fallback submit on hover — hover is on first visible match.
        let hovered = state.hover().unwrap();
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(hovered));
    }

}
