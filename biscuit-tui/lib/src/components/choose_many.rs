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
//! use biscuit_tui::prelude::*;
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

use std::ops::{Deref, DerefMut};
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
    ComponentTheme, EventOutcome, HandleEvent, KeyBindings, Label, StandaloneState, TerminalStyle,
    ValidationState, render_with_label,
};

use super::choice_layout::ChoiceLayout;
use super::choice_render::ChoiceRenderContext;
use super::choice_state::{
    ChoiceCommonState, HOTKEY_DISPLAY_FALLBACK, first_enabled_index, impl_choice_common_builders,
    last_enabled_index, modifier_only_mode, sticky_toggle_mode,
};
use super::choose::{ChoiceInput, ChoiceOption, HotkeyDisplayMode, Orientation, SelectionMode};

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
    common: ChoiceCommonState<V>,
    selected: Vec<bool>,
}

impl<V> Deref for ChooseManyState<V> {
    type Target = ChoiceCommonState<V>;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<V> DerefMut for ChooseManyState<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
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
        Self {
            common: ChoiceCommonState::new(input),
            selected,
        }
    }

    /// Shortcut constructor from a bare vec of options.
    pub fn from_options(options: Vec<ChoiceOption<V>>) -> Self {
        Self::new(ChoiceInput::new("", "").with_options(options))
    }

    impl_choice_common_builders!(ChooseManyState);

    /// Returns the raw stored [`HotkeyDisplayMode`] without consulting
    /// the fallback deadline.
    pub fn hotkey_display(&self) -> HotkeyDisplayMode {
        self.hotkey_display
    }

    /// Resolves the effective [`HotkeyDisplayMode`] at `now` against
    /// the fallback deadline.
    pub fn current_hotkey_display(&self, now: Instant) -> HotkeyDisplayMode {
        self.common.current_hotkey_display(now)
    }

    /// Returns the current `Ctrl+Space` / `Alt+Space` toggle state.
    pub fn hotkey_display_sticky(&self) -> Option<HotkeyDisplayMode> {
        self.hotkey_display_sticky
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
        for (idx, option) in self.common.input.options.iter().enumerate() {
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
            let hotkey_display = state.current_hotkey_display(Instant::now());
            let layout = {
                let theme = state.theme.clone();
                let ctx = ChoiceRenderContext::for_multiple(
                    &theme,
                    state.terminal_style,
                    state.input.orientation,
                )
                .with_active_color(state.input.active_color)
                .with_hotkey_display(hotkey_display);
                ctx.compute_layout(list_area, &state.input.options, &visible_indices, |idx| {
                    state.selected.get(idx).copied().unwrap_or(false)
                })
            };
            state.layout_cache = layout.clone();
            let scroll_visible = {
                let theme = state.theme.clone();
                let ctx = ChoiceRenderContext::for_multiple(
                    &theme,
                    state.terminal_style,
                    state.input.orientation,
                )
                .with_active_color(state.input.active_color)
                .with_hotkey_display(hotkey_display);
                ctx.visible_logical_rows(body_rows)
            };
            adjust_scroll(state, scroll_visible, &visible_indices, &layout);

            let common = &mut state.common;
            let selected = &state.selected;
            let validation_error = common.validation_error.clone();
            let ctx = ChoiceRenderContext::for_multiple(
                &common.theme,
                common.terminal_style,
                common.input.orientation,
            )
            .with_active_color(common.input.active_color)
            .with_hotkey_display(hotkey_display);
            ctx.render(
                list_area,
                b,
                &common.input.options,
                &visible_indices,
                common.scroll_offset,
                common.hover,
                |idx| selected.get(idx).copied().unwrap_or(false),
                Some(&mut common.filter),
                validation_error.as_deref(),
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
        // directly. See `ChooseOne::handle_event` for details. When a
        // lifetime-long override is in effect, modifier-only events
        // are still consumed but never mutate badge state.
        if let Some(mode) = modifier_only_mode(&event) {
            if state.hotkey_display_override.is_some() {
                return EventOutcome::Consumed;
            }
            match event.kind {
                KeyEventKind::Press | KeyEventKind::Repeat => {
                    state.hotkey_display = mode;
                    state.hotkey_display_deadline = None;
                    return EventOutcome::Consumed;
                }
                KeyEventKind::Release => {
                    // See `ChooseOne::handle_event` — release clears
                    // dynamic, deadline, AND sticky display state so
                    // the badges don't outlive the modifier hold.
                    state.hotkey_display = HotkeyDisplayMode::Hidden;
                    state.hotkey_display_deadline = None;
                    state.hotkey_display_sticky = None;
                    return EventOutcome::Consumed;
                }
            }
        }

        // Portable badge-visibility chord: `Ctrl+Space` / `Alt+Space`.
        // Press sets the sticky display, Release clears it. See
        // `ChooseOne::handle_event` for the full rationale — the
        // semantics here are identical.
        if let Some(toggled) = sticky_toggle_mode(&event)
            && state.hotkey_display_override.is_none()
        {
            match event.kind {
                KeyEventKind::Press | KeyEventKind::Repeat => {
                    state.hotkey_display_sticky = Some(toggled);
                }
                KeyEventKind::Release => {
                    state.hotkey_display_sticky = None;
                }
            }
            return EventOutcome::Consumed;
        }

        // Chord fallback for terminals that never emit modifier-only
        // events. A forced override mode (set via
        // `with_hotkey_display`) suppresses these writes so the
        // override cannot be flipped to the other modifier or armed
        // with a transient deadline.
        if state.hotkey_display_override.is_none() {
            if event.modifiers.contains(KeyModifiers::CONTROL) {
                state.hotkey_display = HotkeyDisplayMode::CtrlHeld;
                state.hotkey_display_deadline = Some(Instant::now() + HOTKEY_DISPLAY_FALLBACK);
            } else if event.modifiers.contains(KeyModifiers::ALT) {
                state.hotkey_display = HotkeyDisplayMode::AltHeld;
                state.hotkey_display_deadline = Some(Instant::now() + HOTKEY_DISPLAY_FALLBACK);
            }
        }

        // Cancel binding: when the fuzzy filter is visible, the first
        // press clears + hides it; subsequent presses fall through to
        // abort.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            if state.filter_visible {
                let cached_labels = state.cached_labels.clone();
                state.filter.clear(&cached_labels);
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
                    let cached_labels = state.cached_labels.clone();
                    state.filter.pop_char(&cached_labels);
                    snap_hover_to_visible(state);
                    state.validation_error = None;
                    return EventOutcome::Consumed;
                }
                KeyCode::Char(c) if c != ' ' => {
                    let cached_labels = state.cached_labels.clone();
                    state.filter.push_char(c, &cached_labels);
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

        // Ctrl/Alt hotkeys toggle the matching option. Modifier matching
        // uses `.contains(...)` so a benign extra bit (e.g. SHIFT on an
        // uppercase chord) does not suppress an otherwise valid hotkey.
        // WHY CONTROL|ALT falls through: on some layouts AltGr-style
        // chords report CONTROL|ALT and must not be hijacked as a
        // hotkey, so when both are present neither map matches.
        let modifiers = event.modifiers;
        if modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT) {
            if let KeyCode::Char(c) = event.code
                && let Some(&idx) = state.ctrl_hotkeys.get(&c.to_ascii_lowercase())
            {
                jump_to(state, idx);
                toggle_at(state, idx);
                return EventOutcome::Consumed;
            }
        } else if modifiers.contains(KeyModifiers::ALT)
            && !modifiers.contains(KeyModifiers::CONTROL)
            && let KeyCode::Char(c) = event.code
            && let Some(&idx) = state.alt_hotkeys.get(&c.to_ascii_lowercase())
        {
            jump_to(state, idx);
            toggle_at(state, idx);
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
                KeyCode::Char(c) if state.input.filter_enabled && c.is_alphanumeric() => {
                    let cached_labels = state.cached_labels.clone();
                    state.filter.clear(&cached_labels);
                    state.filter.push_char(c, &cached_labels);
                    state.filter_visible = true;
                    snap_hover_to_visible(state);
                    state.validation_error = None;
                    return EventOutcome::Consumed;
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
    if let Some(new_hover) =
        state
            .layout_cache
            .navigate_row(&state.input.options, state.hover, delta)
    {
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
mod tests;
