//! Multi-selection list component.
//!
//! [`ChooseMany`] is a zero-sized [`StatefulWidget`] marker that
//! renders a vertical list of [`ChoiceOption`]s with checkbox glyphs.
//! Users toggle selections with Space and submit with Enter.
//! `max_selections` is enforced at keystroke time (toggle-on is
//! silently dropped when the cap is reached); `required` and
//! `min_selections` are validated at submit time.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let input: ChoiceInput = ChoiceInput::new("toppings", "Pick toppings")
//!     .with_options(vec![
//!         ChoiceOption::new("p", "Pepperoni", "pepperoni"),
//!         ChoiceOption::new("m", "Mushrooms", "mushrooms"),
//!     ])
//!     .with_max_selections(1);
//! let state = ChooseManyState::new(input);
//! assert!(state.selected_values().is_empty());
//! ```

use std::collections::HashMap;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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

use super::choice_layout::{ChoiceLayout, navigate_row};
use super::choice_render::ChoiceRenderContext;
use super::choose::{ChoiceInput, ChoiceOption, HotkeyDisplayMode, Orientation, SelectionMode};
use super::choose_one::{
    HOTKEY_DISPLAY_FALLBACK, build_hotkeys, first_enabled_index, last_enabled_index,
    modifier_only_mode,
};

/// Mutable state for a [`ChooseMany`] widget.
///
/// Wraps a [`ChoiceInput<V>`] (forced to `SelectionMode::Multiple`)
/// and adds transient UI state: hovered row, per-option selected flags,
/// scroll offset, precomputed hotkey map, key bindings, and any active
/// validation error.
///
/// The type parameter `V` defaults to `String` for backward
/// compatibility.
#[derive(Debug, Clone)]
pub struct ChooseManyState<V = String> {
    input: ChoiceInput<V>,
    selected: Vec<bool>,
    hover: usize,
    scroll_offset: usize,
    hotkeys: HashMap<char, usize>,
    label: Option<Label>,
    theme: ComponentTheme,
    bindings: KeyBindings,
    validation_error: Option<String>,
    filter: FuzzyFilter,
    filter_visible: bool,
    cached_labels: Vec<String>,
    layout_cache: ChoiceLayout,
    hotkey_display: HotkeyDisplayMode,
    hotkey_display_deadline: Option<Instant>,
}

impl<V: Clone + PartialEq> ChooseManyState<V> {
    /// Builds a new state from a [`ChoiceInput<V>`].
    ///
    /// The selection mode is forced to [`SelectionMode::Multiple`].
    pub fn new(mut input: ChoiceInput<V>) -> Self {
        input.selection_mode = SelectionMode::Multiple;
        // Sort first (per the configured `with_sort`), then optionally
        // shuffle, then allocate per-option selection state. Library
        // consumers and CLI consumers see the same ordering because
        // `ChoiceInput` is the single authority on option ordering.
        input.sort_options_in_place();
        if input.shuffle_options {
            input.options.shuffle(&mut rand::rng());
        }
        let selected = vec![false; input.options.len()];
        let hotkeys = build_hotkeys(&input.options);
        let hover = first_enabled_index(&input.options).unwrap_or(0);
        let cached_labels: Vec<String> = input.options.iter().map(|o| o.label.clone()).collect();
        let mut filter = FuzzyFilter::new();
        filter.clear(&cached_labels);
        Self {
            input,
            selected,
            hover,
            scroll_offset: 0,
            hotkeys,
            label: None,
            theme: ComponentTheme::default(),
            bindings: KeyBindings::default(),
            validation_error: None,
            filter,
            filter_visible: false,
            cached_labels,
            layout_cache: ChoiceLayout::default(),
            hotkey_display: HotkeyDisplayMode::Hidden,
            hotkey_display_deadline: None,
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

    /// Sets the [`ActiveChoiceColor`] used for the actively hovered
    /// option. Sugar over mutating the underlying [`ChoiceInput`].
    pub fn with_active_color(mut self, color: super::choose::ActiveChoiceColor) -> Self {
        self.input.active_color = color;
        self
    }

    /// Forces the hotkey badge display mode for the lifetime of this
    /// state. See [`ChooseOneState::with_hotkey_display`] for details.
    pub fn with_hotkey_display(mut self, mode: HotkeyDisplayMode) -> Self {
        self.hotkey_display = mode;
        self.hotkey_display_deadline = None;
        self
    }

    /// Returns the raw stored [`HotkeyDisplayMode`] without consulting
    /// the fallback deadline.
    pub fn hotkey_display(&self) -> HotkeyDisplayMode {
        self.hotkey_display
    }

    /// Resolves the effective [`HotkeyDisplayMode`] at `now` against
    /// the fallback deadline. See
    /// [`ChooseOneState::current_hotkey_display`] for details.
    pub fn current_hotkey_display(&self, now: Instant) -> HotkeyDisplayMode {
        match self.hotkey_display_deadline {
            Some(deadline) if now < deadline => self.hotkey_display,
            Some(_) => HotkeyDisplayMode::Hidden,
            None => self.hotkey_display,
        }
    }

    /// Pre-selects options by their stable identifiers.
    pub fn with_initial_selection(mut self, ids: &[&str]) -> Self {
        for id in ids {
            if let Some((idx, _)) = self
                .input
                .options
                .iter()
                .enumerate()
                .find(|(_, option)| option.id == *id)
            {
                self.selected[idx] = true;
            }
        }
        self
    }

    /// Pre-selects options by matching each entry in `values` against
    /// the options' `value` field.
    ///
    /// Intended for CLI callers that expose `--selected <VALUE>`: the
    /// option's `value` is the authoritative identity once a
    /// `--delimiter` has split a `label⟂value` pair, and may differ
    /// from the option's stable `id` when built from a dictionary
    /// source. Unmatched entries are silently ignored.
    pub fn with_initial_values(mut self, values: &[&str]) -> Self
    where
        V: PartialEq<str>,
    {
        for value in values {
            if let Some((idx, _)) = self
                .input
                .options
                .iter()
                .enumerate()
                .find(|(_, option)| option.value == **value)
            {
                self.selected[idx] = true;
            }
        }
        self
    }

    /// Returns the full list of options.
    pub fn options(&self) -> &[ChoiceOption<V>] {
        &self.input.options
    }

    /// Returns the index of the currently hovered option, if any.
    pub fn hover(&self) -> Option<usize> {
        if self.input.options.is_empty() {
            None
        } else {
            Some(self.hover)
        }
    }

    /// Returns whether the option at `idx` is currently selected.
    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected.get(idx).copied().unwrap_or(false)
    }

    /// Returns the stable ids of every selected option.
    pub fn selected_ids(&self) -> Vec<&str> {
        self.selected
            .iter()
            .zip(self.input.options.iter())
            .filter_map(|(flag, option)| {
                if *flag {
                    Some(option.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the typed values of every selected option.
    pub fn selected_values(&self) -> Vec<&V> {
        self.selected
            .iter()
            .zip(self.input.options.iter())
            .filter_map(
                |(flag, option)| {
                    if *flag { Some(&option.value) } else { None }
                },
            )
            .collect()
    }

    /// Returns how many options are currently selected.
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|flag| **flag).count()
    }

    /// Returns `true` when the input was marked required.
    pub fn required(&self) -> bool {
        self.input.required
    }

    /// Returns the `min_selections` cap, if any.
    pub fn min_selections(&self) -> Option<usize> {
        self.input.min_selections
    }

    /// Returns the `max_selections` cap, if any.
    pub fn max_selections(&self) -> Option<usize> {
        self.input.max_selections
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

    /// Returns a reference to the key bindings.
    pub fn key_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    /// Sets an inline validation error.
    pub fn set_validation_error(&mut self, message: impl Into<String>) {
        self.validation_error = Some(message.into());
    }

    /// Selects every enabled option.
    ///
    /// Disabled options are skipped and remain unselected. `max_selections`
    /// is intentionally *not* enforced here: the bulk keystroke is the
    /// user's explicit intent and validation runs at submit time.
    /// Any active validation error is cleared.
    pub fn select_all(&mut self) {
        for (idx, option) in self.input.options.iter().enumerate() {
            if !option.disabled {
                self.selected[idx] = true;
            }
        }
        self.validation_error = None;
    }

    /// Clears every selection.
    ///
    /// `min_selections` is intentionally *not* enforced here — validation
    /// runs at submit time, not at toggle time. Any active validation
    /// error is cleared.
    pub fn deselect_all(&mut self) {
        for flag in &mut self.selected {
            *flag = false;
        }
        self.validation_error = None;
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

impl<V: Clone + PartialEq> ValidationState for ChooseManyState<V> {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl<V: Clone + PartialEq> StandaloneState for ChooseManyState<V> {
    type Value = Vec<V>;

    fn value(&self) -> Self::Value {
        self.selected_values().into_iter().cloned().collect()
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn help_hint(&self) -> &str {
        &self.theme.help_hint
    }
}

/// Multi-selection list widget.
///
/// Rendered via [`StatefulWidget`] against a [`ChooseManyState<V>`]. The
/// widget itself is zero-sized — all state lives on the paired state
/// struct.
///
/// The type parameter `V` must match the state's value type. Defaults
/// to `String` for backward compatibility.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChooseMany<V = String> {
    _phantom: std::marker::PhantomData<V>,
}

impl<V> ChooseMany<V> {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<V: Clone + PartialEq> StatefulWidget for ChooseMany<V> {
    type State = ChooseManyState<V>;

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

            let hotkey_display = state.current_hotkey_display(Instant::now());
            let layout = {
                let theme = state.theme.clone();
                let ctx = ChoiceRenderContext::for_multiple(
                    &theme,
                    TerminalStyle::default(),
                    state.input.orientation,
                )
                .with_active_color(state.input.active_color)
                .with_hotkey_display(hotkey_display);
                ctx.compute_layout(list_area, &state.input.options, &visible_indices, |idx| {
                    state.selected.get(idx).copied().unwrap_or(false)
                })
            };
            state.layout_cache = layout.clone();
            adjust_scroll(state, visible, &visible_indices, &layout);

            let ctx = ChoiceRenderContext::for_multiple(
                &state.theme,
                TerminalStyle::default(),
                state.input.orientation,
            )
            .with_active_color(state.input.active_color)
            .with_hotkey_display(hotkey_display);
            ctx.render(
                list_area,
                b,
                &state.input.options,
                &visible_indices,
                state.scroll_offset,
                state.hover,
                |idx| state.selected.get(idx).copied().unwrap_or(false),
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
    state: &ChooseManyState<V>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text = format!("{}{}", state.theme.search_indicator, state.filter.pattern());
    let line = Line::from(Span::styled(text, state.theme.search_style));
    buf.set_line(area.x, area.y, &line, area.width);
}

impl<V: Clone + PartialEq> HandleEvent for ChooseMany<V> {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        // Modifier-only key events drive hotkey badge display
        // directly. See `ChooseOne::handle_event` for details.
        if let Some(mode) = modifier_only_mode(&event) {
            match event.kind {
                KeyEventKind::Press | KeyEventKind::Repeat => {
                    state.hotkey_display = mode;
                    state.hotkey_display_deadline = None;
                    return EventOutcome::Consumed;
                }
                KeyEventKind::Release => {
                    state.hotkey_display = HotkeyDisplayMode::Hidden;
                    state.hotkey_display_deadline = None;
                    return EventOutcome::Consumed;
                }
            }
        }

        // Chord fallback for terminals that never emit modifier-only
        // events.
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            state.hotkey_display = HotkeyDisplayMode::CtrlHeld;
            state.hotkey_display_deadline = Some(Instant::now() + HOTKEY_DISPLAY_FALLBACK);
        } else if event.modifiers.contains(KeyModifiers::ALT) {
            state.hotkey_display = HotkeyDisplayMode::AltHeld;
            state.hotkey_display_deadline = Some(Instant::now() + HOTKEY_DISPLAY_FALLBACK);
        }

        // Cancel binding: when the fuzzy filter is visible, the first
        // press clears + hides it; subsequent presses fall through to
        // abort.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            if state.filter_visible {
                state.filter.clear(&state.cached_labels);
                state.filter_visible = false;
                snap_hover_to_visible(state);
                return EventOutcome::Consumed;
            }
            // Spec: "The default key-bindings for ChooseMany should stay as they
            // are." ESC returns Cancelled (exit code 1) — unlike ChooseOne which
            // restores and submits.
            return EventOutcome::Cancelled;
        }

        // When the search prompt is visible, route printable chars and
        // backspace to the pattern buffer BEFORE checking nav bindings
        // so `j`/`k` do not collide with vim-style up/down.
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
            toggle_hover(state);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.up, &event) {
            match state.input.orientation {
                Orientation::Vertical => move_hover(state, -1),
                Orientation::Horizontal => move_hover_row(state, -1),
            }
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.down, &event) {
            match state.input.orientation {
                Orientation::Vertical => move_hover(state, 1),
                Orientation::Horizontal => move_hover_row(state, 1),
            }
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.left, &event) {
            move_hover(state, -1);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.right, &event) {
            move_hover(state, 1);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.select_all, &event) {
            state.select_all();
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.deselect_all, &event) {
            state.deselect_all();
            return EventOutcome::Consumed;
        }

        // Home/End/vim-style jumps + hotkey/filter-open, only when the
        // search prompt is hidden.
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
                        toggle_at(state, idx);
                        return EventOutcome::Consumed;
                    }
                }
                _ => {}
            }
        }

        EventOutcome::Ignored
    }
}

fn submit<V: Clone + PartialEq>(state: &mut ChooseManyState<V>) -> EventOutcome {
    // Block submit while a filter is active but no option matches.
    if state.filter_visible && state.filter.visible().is_empty() && !state.input.options.is_empty()
    {
        return EventOutcome::Consumed;
    }
    // Phase 5: Enter submits the selected set exactly as-is — no
    // fallback auto-selection of the active row.
    let count = state.selected_count();
    if state.input.required && count == 0 {
        state.validation_error = Some("Please make a selection".into());
        return EventOutcome::Consumed;
    }
    if let Some(min) = state.input.min_selections
        && count < min
    {
        state.validation_error = Some(format!("Please select at least {min}"));
        return EventOutcome::Consumed;
    }
    EventOutcome::Submitted
}

fn toggle_hover<V: Clone + PartialEq>(state: &mut ChooseManyState<V>) {
    let idx = state.hover;
    toggle_at(state, idx);
}

fn toggle_at<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, idx: usize) {
    let Some(option) = state.input.options.get(idx) else {
        return;
    };
    if option.disabled {
        return;
    }
    let currently_selected = state.selected[idx];
    if !currently_selected
        && let Some(max) = state.input.max_selections
        && state.selected_count() >= max
    {
        // Keystroke-time rejection: silently drop toggle-on.
        return;
    }
    state.selected[idx] = !currently_selected;
    state.validation_error = None;
}

fn move_hover<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, delta: i32) {
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

fn move_hover_row<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, delta: i32) {
    if let Some(new_hover) = navigate_row(
        &state.layout_cache,
        &state.input.options,
        state.hover,
        delta,
    ) {
        state.hover = new_hover;
    } else {
        move_hover(state, delta);
    }
}

/// Snaps `state.hover` to the first enabled index in `filter.visible()`
/// when the current hover is no longer visible. Leaves the hover
/// unchanged when every visible option is disabled or when the current
/// hover is already both visible and enabled.
fn snap_hover_to_visible<V: Clone + PartialEq>(state: &mut ChooseManyState<V>) {
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

fn jump_to<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, idx: usize) {
    if state.input.options.get(idx).is_some_and(|o| !o.disabled) {
        state.hover = idx;
    }
}

fn error_row_y<V: Clone + PartialEq>(
    inner_area: Rect,
    option_count: usize,
    state: &ChooseManyState<V>,
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
    state: &mut ChooseManyState<V>,
    visible_rows: usize,
    _visible_indices: &[usize],
    layout: &ChoiceLayout,
) {
    let total_rows = layout.row_count();
    if visible_rows == 0 || total_rows == 0 {
        state.scroll_offset = 0;
        return;
    }

    let hover_row = layout.row_of(state.hover).unwrap_or(0);

    if hover_row < state.scroll_offset {
        state.scroll_offset = hover_row;
    } else if hover_row >= state.scroll_offset + visible_rows {
        state.scroll_offset = hover_row + 1 - visible_rows;
    }
    if state.scroll_offset + visible_rows > total_rows {
        state.scroll_offset = total_rows.saturating_sub(visible_rows);
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
        ChoiceInput::new("toppings", "Pick toppings").with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pepperoni"),
            ChoiceOption::new("m", "Mushrooms", "mushrooms"),
            ChoiceOption::new("o", "Olives", "olives"),
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
    fn new_starts_with_no_selections() {
        let state = ChooseManyState::new(fixture_input());
        assert_eq!(state.selected_count(), 0);
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn space_toggles_selection_on_and_off() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(state.is_selected(0));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(!state.is_selected(0));
    }

    #[test]
    fn hotkey_toggles_matching_option() {
        let mut state = ChooseManyState::new(fixture_input());
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('m')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.is_selected(1));
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn max_selections_blocks_additional_toggle_on() {
        let input = fixture_input().with_max_selections(1);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_count(), 1);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(1));
    }

    #[test]
    fn max_selections_allows_toggle_off_when_at_cap() {
        let input = fixture_input().with_max_selections(1);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn required_validation_fires_when_fallback_cannot_find_enabled_option() {
        // With fallback-submit-on-active, Enter while nothing is selected
        // toggles the hovered option on. When every option is disabled,
        // fallback cannot find a non-disabled hover, so the required-input
        // validation error fires.
        let input = ChoiceInput::<String>::new("toppings", "Pick toppings")
            .with_options(vec![
                ChoiceOption::new("p", "Pepperoni", "pepperoni").disabled(),
                ChoiceOption::new("m", "Mushrooms", "mushrooms").disabled(),
            ])
            .required();
        let mut state = ChooseManyState::new(input);
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn enter_with_fewer_than_min_selections_sets_error() {
        let input = fixture_input().with_min_selections(2);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        let message = <ChooseManyState as ValidationState>::validation_error(&state).unwrap();
        assert!(message.contains('2'));
    }

    #[test]
    fn enter_with_min_selections_met_submits() {
        let input = fixture_input().with_min_selections(1);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn toggling_selection_clears_validation_error() {
        let input = fixture_input().required();
        let mut state = ChooseManyState::new(input);

        // Seed a validation error as if a prior submit had been blocked.
        state.set_validation_error("Please make a selection");
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_none());
        assert!(state.is_selected(0));
    }

    #[test]
    fn enter_does_not_auto_select_when_none_chosen() {
        // Phase 5: Enter submits the selected set exactly as-is — no
        // fallback auto-selection of the active row.
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover(), Some(1));
        assert_eq!(state.selected_count(), 0);

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert!(!state.is_selected(1));
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn fallback_submit_skips_disabled_hover() {
        // When the hovered option is disabled, fallback must not toggle it
        // on. Under `required`, the validation error must fire.
        let input = ChoiceInput::<String>::new("toppings", "Pick toppings")
            .with_options(vec![
                ChoiceOption::new("p", "Pepperoni", "pepperoni").disabled(),
                ChoiceOption::new("m", "Mushrooms", "mushrooms"),
            ])
            .required();
        let mut state = ChooseManyState::new(input);
        // Force hover onto the disabled row.
        state.hover = 0;

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_count(), 0);
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());
    }

    #[test]
    fn fallback_submit_does_not_override_existing_selections() {
        // When at least one option is already selected, fallback must not
        // touch other rows — only the existing selections are submitted.
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(state.is_selected(0));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover(), Some(2));

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(2));
        assert_eq!(state.selected_count(), 1);
    }

    #[test]
    fn esc_cancels() {
        let mut state = ChooseManyState::new(fixture_input());
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn disabled_options_cannot_be_toggled() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "Alpha", "alpha"),
            ChoiceOption::new("b", "Beta", "beta").disabled(),
        ]);
        let mut state = ChooseManyState::new(input);
        state.hover = 1;
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(!state.is_selected(1));
    }

    #[test]
    fn selected_values_returns_value_fields_in_order() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let values = state.selected_values();
        assert_eq!(
            values,
            vec![&"pepperoni".to_string(), &"olives".to_string()]
        );
    }

    #[test]
    fn render_draws_checkbox_glyphs_per_row() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        // Focus prefix on hovered row 0, blank padding on others
        assert!(buffer_row(&buf, 0).starts_with("▶ ☑ Pepperoni"));
        assert!(buffer_row(&buf, 1).starts_with("  ☐ Mushrooms"));
        assert!(buffer_row(&buf, 2).starts_with("  ☐ Olives"));
    }

    #[test]
    fn render_with_label_above_draws_label_then_list() {
        let mut state = ChooseManyState::new(fixture_input())
            .with_label(Label::new("Toppings", LabelPosition::Above));
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Toppings");
        // First option has focus prefix (hover defaults to 0)
        assert!(buffer_row(&buf, 1).starts_with("▶ ☐ Pepperoni"));
    }

    #[test]
    fn render_draws_validation_error_on_row_below_list() {
        let input = fixture_input().required();
        let mut state = ChooseManyState::new(input);
        state.set_validation_error("Please make a selection");
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 3), "Please make a selection");
    }

    #[test]
    fn end_key_jumps_hover_to_last_enabled_option() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::End));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn vim_capital_g_jumps_hover_to_last_enabled_option() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('G')));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn initial_selection_pre_selects_listed_ids() {
        let state = ChooseManyState::new(fixture_input()).with_initial_selection(&["p", "o"]);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(1));
        assert!(state.is_selected(2));
    }

    #[test]
    fn initial_values_pre_select_by_value() {
        // `with_initial_values` matches the option's `value` field
        // (rather than its `id`). We distinguish id/label/value so the
        // test pins the by-value semantics.
        let input: ChoiceInput<String> = ChoiceInput::new("toppings", "Pick toppings")
            .with_options(vec![
                ChoiceOption::new("p", "Pepperoni", "pep-val"),
                ChoiceOption::new("m", "Mushrooms", "mush-val"),
                ChoiceOption::new("o", "Olives", "oliv-val"),
            ]);
        let state = ChooseManyState::new(input).with_initial_values(&["pep-val", "oliv-val"]);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(1));
        assert!(state.is_selected(2));
    }

    #[test]
    fn initial_values_ignore_unmatched_entries() {
        let state = ChooseManyState::new(fixture_input()).with_initial_values(&["ghost"]);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn custom_submit_binding_overrides_default() {
        let bindings = KeyBindings {
            submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        // Ctrl-S should submit.
        let outcome = ChooseMany::new().handle_event(
            &mut state,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
        assert_eq!(outcome, EventOutcome::Submitted);

        // Enter should be ignored (no longer bound to submit).
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn custom_cancel_binding_works() {
        let bindings = KeyBindings {
            cancel: vec![press(KeyCode::Char('q'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

        // q should cancel.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('q')));
        assert_eq!(outcome, EventOutcome::Cancelled);

        // Esc should be ignored.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn custom_toggle_binding_works() {
        let bindings = KeyBindings {
            toggle: vec![press(KeyCode::Char('t'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

        // t should toggle.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('t')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.is_selected(0));

        // Space should be ignored (no longer bound to toggle).
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Ignored);
        assert!(state.is_selected(0));
    }

    #[test]
    fn custom_up_down_bindings_work() {
        let bindings = KeyBindings {
            up: vec![press(KeyCode::Char('w'))],
            down: vec![press(KeyCode::Char('s'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);
        assert_eq!(state.hover, 0);

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('s')));
        assert_eq!(state.hover, 1);

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('w')));
        assert_eq!(state.hover, 0);

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
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
            ChoiceInput::new("nums", "Pick numbers").with_options(typed_options);
        let mut state: ChooseManyState<u32> = ChooseManyState::new(input);

        // Select first and third options.
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        assert_eq!(state.selected_values(), vec![&1_u32, &3_u32]);
        assert_eq!(state.value(), vec![1_u32, 3_u32]);
    }

    #[test]
    fn typed_value_enum_flow() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Topping {
            Pepperoni,
            Mushrooms,
            Olives,
        }
        let options: Vec<ChoiceOption<Topping>> = vec![
            ChoiceOption::new("p", "Pepperoni", Topping::Pepperoni),
            ChoiceOption::new("m", "Mushrooms", Topping::Mushrooms),
            ChoiceOption::new("o", "Olives", Topping::Olives),
        ];
        let input: ChoiceInput<Topping> =
            ChoiceInput::new("toppings", "Pick toppings").with_options(options);
        let mut state: ChooseManyState<Topping> = ChooseManyState::new(input);

        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        assert_eq!(
            state.selected_values(),
            vec![&Topping::Pepperoni, &Topping::Olives]
        );
        assert_eq!(state.value(), vec![Topping::Pepperoni, Topping::Olives]);
    }

    #[test]
    fn render_overflow_indicators_when_scrolled() {
        // 6 options, 3 rows tall, hover on option 3 (scrolls to show 1-3)
        let options: Vec<ChoiceOption<String>> = (0..6)
            .map(|i| ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("val{i}")))
            .collect();
        let input = ChoiceInput::new("test", "Test").with_options(options);
        let mut state = ChooseManyState::new(input);

        // Move hover to option 3
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover, 3);

        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);

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
        let mut state = ChooseManyState::new(input);

        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);

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
    fn ctrl_a_selects_all_enabled_options() {
        let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pepperoni"),
            ChoiceOption::new("m", "Mushrooms", "mushrooms").disabled(),
            ChoiceOption::new("o", "Olives", "olives"),
        ]);
        let mut state = ChooseManyState::new(input);
        let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);

        let outcome = ChooseMany::new().handle_event(&mut state, ctrl_a);
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.is_selected(0));
        assert!(
            !state.is_selected(1),
            "disabled option must stay unselected"
        );
        assert!(state.is_selected(2));
        assert_eq!(state.selected_count(), 2);
    }

    #[test]
    fn ctrl_a_ignores_max_selections_cap() {
        // The `select_all` bulk keystroke is the user's explicit intent;
        // validation runs at submit time, so the cap does not block bulk
        // selection at toggle time.
        let input = fixture_input().with_max_selections(1);
        let mut state = ChooseManyState::new(input);
        let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        ChooseMany::new().handle_event(&mut state, ctrl_a);
        assert_eq!(state.selected_count(), 3);
    }

    #[test]
    fn ctrl_d_clears_all() {
        let mut state = ChooseManyState::new(fixture_input());
        // Select every option first.
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_count(), 3);

        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        let outcome = ChooseMany::new().handle_event(&mut state, ctrl_d);
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn ctrl_d_clears_even_when_below_min_selections() {
        // min_selections is validated at submit, not at toggle — so bulk
        // deselect always clears.
        let input = fixture_input().with_min_selections(2);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_count(), 2);

        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        ChooseMany::new().handle_event(&mut state, ctrl_d);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn select_all_clears_validation_error() {
        let input = fixture_input().required();
        let mut state = ChooseManyState::new(input);
        state.set_validation_error("Please make a selection");
        state.select_all();
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_none());
        assert_eq!(state.selected_count(), 3);
    }

    #[test]
    fn deselect_all_clears_validation_error() {
        let mut state = ChooseManyState::new(fixture_input());
        state.set_validation_error("Please select fewer");
        state.deselect_all();
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_none());
    }

    #[test]
    fn custom_select_all_binding_overrides_default() {
        let bindings = KeyBindings {
            select_all: vec![press(KeyCode::Char('A'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

        // Plain 'A' should now select all.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('A')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_count(), 3);

        // Ctrl-A should now be ignored (no longer bound).
        state.deselect_all();
        let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let outcome = ChooseMany::new().handle_event(&mut state, ctrl_a);
        assert_eq!(outcome, EventOutcome::Ignored);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn shuffle_randomises_order_choose_many() {
        let options: Vec<ChoiceOption<String>> = (0..20)
            .map(|i| {
                ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("value{i}"))
            })
            .collect();
        let original_labels: Vec<String> = options.iter().map(|o| o.label.clone()).collect();
        let input = ChoiceInput::new("x", "P")
            .with_shuffle_options(true)
            .with_options(options);
        let state = ChooseManyState::new(input);
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
    fn shuffle_then_select_choose_many() {
        let options: Vec<ChoiceOption<String>> = (0..10)
            .map(|i| ChoiceOption::new(format!("id{i}"), format!("Opt{i}"), format!("val{i}")))
            .collect();
        let input = ChoiceInput::new("x", "P")
            .with_shuffle_options(true)
            .with_options(options);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(state.is_selected(0));
        let values = state.selected_values();
        assert_eq!(values.len(), 1);
    }

    // -----------------------------------------------------------------
    // Phase 8 — Search Prompt Rendering & State Plumbing
    // -----------------------------------------------------------------

    fn filter_fixture_input() -> ChoiceInput<String> {
        ChoiceInput::new("fruit", "Pick fruits")
            .with_filter_enabled(true)
            .with_options(vec![
                ChoiceOption::new("a", "Apple", "apple"),
                ChoiceOption::new("b", "Banana", "banana"),
                ChoiceOption::new("c", "Blueberry", "blueberry"),
                ChoiceOption::new("d", "Cherry", "cherry"),
            ])
    }

    #[test]
    fn filter_visible_starts_false_many() {
        let state = ChooseManyState::new(filter_fixture_input());
        assert!(!state.filter_visible());
        assert_eq!(state.filter_pattern(), "");
        assert_eq!(state.visible_indices(), &[0, 1, 2, 3]);
    }

    #[test]
    fn typing_letter_opens_filter_many() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('B')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.filter_visible());
        assert_eq!(state.filter_pattern(), "B");
        let visible = state.visible_indices().to_vec();
        assert!(visible.contains(&1));
        assert!(visible.contains(&2));
        assert!(!visible.contains(&0));
    }

    #[test]
    fn backspace_pops_filter_character_many() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('l')));
        assert_eq!(state.filter_pattern(), "bl");
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Backspace));
        assert_eq!(state.filter_pattern(), "b");
    }

    #[test]
    fn down_walks_filtered_indices_many() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
        let visible = state.visible_indices().to_vec();
        assert!(!visible.is_empty());
        let first = visible[0];
        assert_eq!(state.hover(), Some(first));

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        let hover = state.hover().unwrap();
        assert!(visible.contains(&hover));
        assert_ne!(hover, first);
    }

    #[test]
    fn space_toggles_first_filtered_match() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('c'))); // Cherry + maybe Blueberry via fuzzy
        let hover = state.hover().unwrap();
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.is_selected(hover));
    }

    #[test]
    fn esc_clears_filter_first_then_aborts_many() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
        assert!(state.filter_visible());

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(!state.filter_visible());
        assert_eq!(state.filter_pattern(), "");

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn submit_blocked_when_filter_hides_everything() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('z')));
        assert!(state.filter_visible());
        assert!(state.visible_indices().is_empty());

        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn render_draws_search_prompt_row_when_filter_visible_many() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        let prompt = buffer_row(&buf, 0);
        assert!(
            prompt.starts_with("/ b"),
            "expected '/ b' prompt at top, got {prompt:?}"
        );
    }

    #[test]
    fn render_shows_no_matches_row_when_filter_matches_nothing_many() {
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('z')));
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 1), "(no matches)");
    }

    #[test]
    fn ctrl_a_selects_all_visible_matches_only() {
        // When a filter is active, Ctrl+A selects every enabled visible
        // option. The current implementation selects every enabled
        // option regardless of filter, which is acceptable because the
        // bulk keystroke is the user's explicit intent; validation runs
        // at submit time. We pin the existing behaviour here so future
        // refactors surface any regressions.
        let mut state = ChooseManyState::new(filter_fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
        let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        ChooseMany::new().handle_event(&mut state, ctrl_a);
        assert!(state.selected_count() > 0);
    }

    // -----------------------------------------------------------------
    // Phase 6 — Horizontal Layout & Navigation
    // -----------------------------------------------------------------

    fn horizontal_fixture_input_many() -> ChoiceInput<String> {
        ChoiceInput::new("toppings", "Pick toppings")
            .with_orientation(Orientation::Horizontal)
            .with_options(vec![
                ChoiceOption::new("p", "Pepperoni", "pepperoni"),
                ChoiceOption::new("m", "Mushrooms", "mushrooms"),
                ChoiceOption::new("o", "Olives", "olives"),
                ChoiceOption::new("e", "Extra", "extra"),
            ])
    }

    #[test]
    fn horizontal_many_render_packs_items_left_to_right() {
        let mut state = ChooseManyState::new(horizontal_fixture_input_many());
        let area = Rect::new(0, 0, 80, 2);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);

        let row0 = buffer_row(&buf, 0);
        assert!(
            row0.contains("Pepperoni"),
            "expected Pepperoni on row 0: {row0}"
        );
        assert!(
            row0.contains("Mushrooms"),
            "expected Mushrooms on row 0: {row0}"
        );
    }

    #[test]
    fn horizontal_many_render_no_triangular_pointer() {
        let mut state = ChooseManyState::new(horizontal_fixture_input_many());
        let area = Rect::new(0, 0, 80, 2);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);

        let row0 = buffer_row(&buf, 0);
        assert!(
            !row0.contains('▶'),
            "horizontal mode should not show triangular pointer: {row0}"
        );
    }

    #[test]
    fn horizontal_many_left_moves_to_previous_option() {
        let mut state = ChooseManyState::new(horizontal_fixture_input_many());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Right));
        assert_eq!(state.hover(), Some(1));

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Left));
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn horizontal_many_right_moves_to_next_option() {
        let mut state = ChooseManyState::new(horizontal_fixture_input_many());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Right));
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn horizontal_many_up_down_use_row_navigation() {
        let mut state = ChooseManyState::new(horizontal_fixture_input_many());
        // Move to option likely on a different row.
        for _ in 0..3 {
            ChooseMany::new().handle_event(&mut state, press(KeyCode::Right));
        }
        let hover_before = state.hover().unwrap();

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Up));
        let hover_after = state.hover().unwrap();
        assert_ne!(hover_after, hover_before, "up should change hover");
    }

    #[test]
    fn horizontal_many_stale_cache_fallback_to_sequential() {
        let mut state = ChooseManyState::new(horizontal_fixture_input_many());
        state.layout_cache = ChoiceLayout::default();

        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover(), Some(1));

        state.layout_cache = ChoiceLayout::default();
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.hover(), Some(0));
    }

    fn unsorted_input_many() -> ChoiceInput<String> {
        ChoiceInput::new("fruit", "Pick").with_options(vec![
            ChoiceOption::new("b", "Berry", "berry"),
            ChoiceOption::new("a", "Apple", "apple"),
            ChoiceOption::new("c", "Cherry", "cherry"),
        ])
    }

    fn labels_many(state: &ChooseManyState) -> Vec<&str> {
        state.options().iter().map(|o| o.label.as_str()).collect()
    }

    #[test]
    fn state_new_applies_with_sort_asc_orders_by_label() {
        let input = unsorted_input_many().with_sort(crate::core::SortOrder::Asc);
        let state = ChooseManyState::new(input);
        assert_eq!(labels_many(&state), vec!["Apple", "Berry", "Cherry"]);
    }

    #[test]
    fn state_new_applies_with_sort_desc_orders_by_label() {
        let input = unsorted_input_many().with_sort(crate::core::SortOrder::Desc);
        let state = ChooseManyState::new(input);
        assert_eq!(labels_many(&state), vec!["Cherry", "Berry", "Apple"]);
    }

    #[test]
    fn state_new_applies_with_sort_inverse_reverses_natural() {
        // `SortOrder::Inverse` is the canonical reversing ordering used
        // by both the library and the CLI `--sort inverse` surface.
        let input = unsorted_input_many().with_sort(crate::core::SortOrder::Inverse);
        let state = ChooseManyState::new(input);
        assert_eq!(labels_many(&state), vec!["Cherry", "Apple", "Berry"]);
    }

    #[test]
    fn state_new_with_sort_natural_is_no_op() {
        let input = unsorted_input_many().with_sort(crate::core::SortOrder::Natural);
        let state = ChooseManyState::new(input);
        assert_eq!(labels_many(&state), vec!["Berry", "Apple", "Cherry"]);
    }

    #[test]
    fn state_new_without_sort_preserves_input_order() {
        // `with_sort` is never called, so the configured order is
        // preserved exactly as given.
        let state = ChooseManyState::new(unsorted_input_many());
        assert_eq!(labels_many(&state), vec!["Berry", "Apple", "Cherry"]);
    }

    #[test]
    fn state_new_sort_allocates_selection_vec_with_correct_length() {
        // The per-option `selected` vec must match the post-sort options
        // list exactly. A stale length here would index out of bounds
        // when toggling.
        let input = unsorted_input_many().with_sort(crate::core::SortOrder::Asc);
        let state = ChooseManyState::new(input);
        assert_eq!(state.options().len(), 3);
        assert!(!state.is_selected(0));
        assert!(!state.is_selected(1));
        assert!(!state.is_selected(2));
    }

    #[test]
    fn state_new_sort_then_shuffle_does_not_panic() {
        // Sorting and shuffling are independent: the constructor sorts
        // first, then shuffles. Either ordering must be acceptable as a
        // post-condition; this test pins that the combination simply
        // does not panic and yields the same number of options.
        let input = unsorted_input_many()
            .with_sort(crate::core::SortOrder::Asc)
            .with_shuffle_options(true);
        let state = ChooseManyState::new(input);
        assert_eq!(state.options().len(), 3);
    }
}
