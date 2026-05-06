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
use super::choice_layout::navigate_row;
use super::choice_render::ChoiceRenderContext;
use super::choice_state::{
    ChoiceCommonState, HOTKEY_DISPLAY_FALLBACK, first_enabled_index, impl_choice_common_builders,
    last_enabled_index, modifier_only_mode, sticky_toggle_mode,
};

use super::choose::{ChoiceInput, ChoiceOption, HotkeyDisplayMode, Orientation, SelectionMode};

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
    common: ChoiceCommonState<V>,
    selected: Option<usize>,
    initial_selected: Option<usize>,
}

impl<V> Deref for ChooseOneState<V> {
    type Target = ChoiceCommonState<V>;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<V> DerefMut for ChooseOneState<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

impl<V: Clone + PartialEq> ChooseOneState<V> {
    /// Builds a new state from a [`ChoiceInput<V>`].
    ///
    /// The selection mode is forced to [`SelectionMode::Single`]
    /// regardless of what `input` specifies — `ChooseOne` is a
    /// single-select component.
    pub fn new(mut input: ChoiceInput<V>) -> Self {
        input.selection_mode = SelectionMode::Single;
        // Sort first (per the configured `with_sort`), then optionally
        // shuffle, then build the hotkey map and cached labels. Library
        // consumers and CLI consumers see the same ordering because
        // `ChoiceInput` is the single authority on option ordering.
        input.sort_options_in_place();
        if input.shuffle_options {
            input.options.shuffle(&mut rand::rng());
        }
        Self {
            common: ChoiceCommonState::new(input),
            selected: None,
            initial_selected: None,
        }
    }

    /// Shortcut constructor from a bare vec of options.
    pub fn from_options(options: Vec<ChoiceOption<V>>) -> Self {
        Self::new(ChoiceInput::new("", "").with_options(options))
    }

    impl_choice_common_builders!(ChooseOneState);

    /// Returns the raw stored [`HotkeyDisplayMode`] without consulting
    /// the fallback deadline. Tests use this to verify modifier-only
    /// transitions; the renderer uses
    /// [`current_hotkey_display`](Self::current_hotkey_display) so the
    /// deadline can decay it back to [`HotkeyDisplayMode::Hidden`].
    pub fn hotkey_display(&self) -> HotkeyDisplayMode {
        self.hotkey_display
    }

    /// Resolves the effective [`HotkeyDisplayMode`] at `now` against
    /// the fallback deadline.
    ///
    /// When `hotkey_display` is non-`Hidden` and no deadline is set
    /// (modifier-only events were observed), the stored mode is
    /// returned verbatim. When a deadline is set, the mode is returned
    /// only while `now` precedes it; otherwise the resolver collapses
    /// to [`HotkeyDisplayMode::Hidden`].
    pub fn current_hotkey_display(&self, now: Instant) -> HotkeyDisplayMode {
        self.common.current_hotkey_display(now)
    }

    /// Returns the current `Ctrl+Space` / `Alt+Space` toggle state.
    ///
    /// `None` means neither toggle is active and badge visibility is
    /// governed by the dynamic modifier-press / chord-deadline path.
    /// `Some(CtrlHeld)` and `Some(AltHeld)` mean the user has pinned the
    /// corresponding emphasis mode via the portable chord toggle.
    pub fn hotkey_display_sticky(&self) -> Option<HotkeyDisplayMode> {
        self.hotkey_display_sticky
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

    /// Returns the effective Ctrl hotkey map keyed by lowercase char.
    pub fn ctrl_hotkeys(&self) -> &HashMap<char, usize> {
        &self.ctrl_hotkeys
    }

    /// Returns the effective Alt hotkey map keyed by lowercase char.
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
            let hotkey_display = state.current_hotkey_display(Instant::now());
            let layout = {
                let theme = state.theme.clone();
                let ctx = ChoiceRenderContext::for_single(
                    &theme,
                    state.terminal_style,
                    state.input.orientation,
                )
                .with_active_color(state.input.active_color)
                .with_hotkey_display(hotkey_display);
                ctx.compute_layout(list_area, &state.input.options, &visible_indices, |idx| {
                    Some(idx) == state.selected
                })
            };
            state.layout_cache = layout.clone();
            let scroll_visible = {
                let theme = state.theme.clone();
                let ctx = ChoiceRenderContext::for_single(
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
            let selected = state.selected;
            let validation_error = common.validation_error.clone();
            let ctx = ChoiceRenderContext::for_single(
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
                |idx| Some(idx) == selected,
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
        if is_ctrl_c(&event) {
            return EventOutcome::Cancelled;
        }

        // Modifier-only key events: terminals that emit explicit
        // KeyCode::Modifier press/release events drive
        // `hotkey_display` directly, bypassing the chord-fallback
        // deadline path. We only consume the event if the modifier
        // matched something; otherwise it falls through.
        //
        // When `with_hotkey_display` has installed a lifetime-long
        // override, we still consume modifier-only events so they do
        // not leak to other handlers, but we do not mutate badge state.
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
                    // Clear ALL badge state on bare-modifier release:
                    // not just the dynamic `hotkey_display` but the
                    // sticky `Ctrl+Space` / `Alt+Space` toggle too.
                    // Without this, a user who holds Ctrl, presses
                    // Space, then releases Ctrl would see the badges
                    // remain visible forever — sticky was set during
                    // the chord and would otherwise outlive the
                    // release event.
                    state.hotkey_display = HotkeyDisplayMode::Hidden;
                    state.hotkey_display_deadline = None;
                    state.hotkey_display_sticky = None;
                    return EventOutcome::Consumed;
                }
            }
        }

        // Portable badge-visibility chord: `Ctrl+Space` and
        // `Alt+Space` mirror "hold to show" UX without depending on
        // the terminal's bare-modifier reporting. Press sets the
        // sticky display mode; Release clears it. The CLI lifetime
        // override (`--hotkey-badges always` / `never` / etc.)
        // suppresses this behaviour so the public flag remains the
        // source of truth when set.
        //
        // Release events for these chords arrive only on terminals
        // that emit kitty-protocol release events; on legacy paths
        // the badges remain visible after Press until another event
        // (e.g. another modifier or chord) supersedes the sticky
        // display.
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

        // Chord fallback: any key event carrying Ctrl or Alt arms a
        // brief deadline so badges become visible on terminals that
        // never emit modifier-only events. A forced override mode
        // (set via `with_hotkey_display`) suppresses these writes so
        // the override cannot be flipped to the other modifier or
        // armed with a transient deadline.
        if state.hotkey_display_override.is_none() {
            if event.modifiers.contains(KeyModifiers::CONTROL) {
                state.hotkey_display = HotkeyDisplayMode::CtrlHeld;
                state.hotkey_display_deadline = Some(Instant::now() + HOTKEY_DISPLAY_FALLBACK);
            } else if event.modifiers.contains(KeyModifiers::ALT) {
                state.hotkey_display = HotkeyDisplayMode::AltHeld;
                state.hotkey_display_deadline = Some(Instant::now() + HOTKEY_DISPLAY_FALLBACK);
            }
        }

        // Cancel binding: when a fuzzy filter is active, the first
        // press clears it; subsequent presses restore the initial
        // selection and submit.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            if state.filter_visible && state.filter.is_active() {
                let cached_labels = state.cached_labels.clone();
                state.filter.clear(&cached_labels);
                state.filter_visible = false;
                snap_hover_to_visible(state);
                return EventOutcome::Consumed;
            }
            if state.filter_visible {
                state.filter_visible = false;
                let cached_labels = state.cached_labels.clone();
                state.filter.clear(&cached_labels);
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
            select_hover(state);
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

fn move_hover_row<V: Clone + PartialEq>(state: &mut ChooseOneState<V>, delta: i32) {
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

fn is_ctrl_c(event: &KeyEvent) -> bool {
    event.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(event.code, KeyCode::Char(c) if c.eq_ignore_ascii_case(&'c'))
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
    use super::super::choose::HotkeySpec;
    use super::*;
    use crate::core::{Label, LabelPosition, NerdFontStatus, TerminalBackground};
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::style::Color;
    use std::time::Duration;

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
    fn typing_letter_without_filter_does_not_jump_or_select() {
        // The "first-letter quick-jump" navigation feature was
        // removed — pressing a literal letter when no filter is
        // active is now Ignored. The user must use arrow keys (or
        // an explicit Ctrl/Alt hotkey) for navigation.
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, press(KeyCode::Char('b')));
        assert_eq!(outcome, EventOutcome::Ignored);
        assert!(state.selected_index().is_none());
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
    fn ctrl_space_is_consumed_as_sticky_toggle() {
        // `Ctrl+Space` (and `Alt+Space`) are reserved as the portable
        // badge-visibility toggle. They are *not* ignored — the
        // component consumes them and updates the sticky mode.
        let mut state = ChooseOneState::new(fixture_input());
        let event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        let outcome = ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
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
    fn ctrl_c_cancels_before_hotkey_dispatch() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "Apple", "apple").with_hotkey(HotkeySpec::Ctrl('a')),
            ChoiceOption::new("c", "Cherry", "cherry").with_hotkey(HotkeySpec::Ctrl('c')),
        ]);
        let mut state = ChooseOneState::new(input);
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let outcome = ChooseOne::new().handle_event(&mut state, event);

        assert_eq!(outcome, EventOutcome::Cancelled);
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn plain_options_have_no_ctrl_hotkeys() {
        // Auto-derivation from the first label character is gone; an
        // option without an explicit `with_hotkey()` call gets no
        // hotkey. Pressing `Ctrl+r` on a plain "Red" option does
        // nothing.
        let state = ChooseOneState::new(fixture_input());
        assert!(state.ctrl_hotkeys().is_empty());
        assert!(state.alt_hotkeys().is_empty());
    }

    #[test]
    fn explicit_ctrl_hotkey_skips_unbound_options_and_submits() {
        // Plain options no longer auto-derive hotkeys from their first
        // letter. Only the explicitly-bound `Green` (Ctrl+G) responds;
        // pressing Ctrl+R on plain `Red` does nothing.
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("r", "Red", "red"),
            ChoiceOption::new("g", "Green", "green").with_hotkey(HotkeySpec::Ctrl('g')),
            ChoiceOption::new("b", "Blue", "blue"),
        ]);
        let mut state = ChooseOneState::new(input);
        let event = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        let outcome = ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_value(), Some(&"green".to_string()));
    }

    #[test]
    fn explicit_alt_hotkey_overrides_default_ctrl_hotkey() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("r", "Red", "red").with_hotkey(HotkeySpec::Alt('x')),
        ]);
        let mut state = ChooseOneState::new(input);
        assert!(state.ctrl_hotkeys().get(&'r').is_none());
        assert_eq!(state.alt_hotkeys().get(&'x'), Some(&0));

        let ctrl = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(
            ChooseOne::new().handle_event(&mut state, ctrl),
            EventOutcome::Ignored
        );
        let alt = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(
            ChooseOne::new().handle_event(&mut state, alt),
            EventOutcome::Submitted
        );
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn disabled_options_drop_their_explicit_hotkey() {
        // A disabled option that *does* carry an explicit hotkey is
        // not dispatched and its hotkey is not registered.
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("r", "Red", "red")
                .with_hotkey(HotkeySpec::Ctrl('r'))
                .disabled(),
            ChoiceOption::new("g", "Green", "green").with_hotkey(HotkeySpec::Ctrl('g')),
        ]);
        let state = ChooseOneState::new(input);
        assert!(state.ctrl_hotkeys().get(&'r').is_none());
        assert_eq!(state.ctrl_hotkeys().get(&'g'), Some(&1));
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
        let mut state =
            ChooseOneState::new(fixture_input()).with_terminal_style(TerminalStyle::default());
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
    fn choose_one_render_uses_nerd_font_terminal_style() {
        let mut state = ChooseOneState::new(fixture_input())
            .with_initial_selection("r")
            .with_terminal_style(TerminalStyle {
                nerd_font: NerdFontStatus::Likely,
                ..TerminalStyle::default()
            });
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        let selected_row = buffer_row(&buf, 0);
        let unselected_row = buffer_row(&buf, 1);
        assert!(
            selected_row.contains('\u{f043e}'),
            "expected Nerd Font selected radio glyph in {selected_row:?}"
        );
        assert!(
            unselected_row.contains('\u{f4aa}'),
            "expected Nerd Font unselected radio glyph in {unselected_row:?}"
        );
    }

    #[test]
    fn choose_one_render_uses_light_background_active_foreground() {
        let mut state = ChooseOneState::new(fixture_input()).with_terminal_style(TerminalStyle {
            background: TerminalBackground::Light,
            ..TerminalStyle::default()
        });
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        assert_eq!(buf[(1, 0)].style().fg, Some(Color::Black));
        assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(252)));
    }

    #[test]
    fn render_with_label_above_draws_label_then_list() {
        let mut state = ChooseOneState::new(fixture_input())
            .with_terminal_style(TerminalStyle::default())
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
    fn typing_letter_with_filter_disabled_is_ignored() {
        // First-letter quick-jump was removed — pressing a literal
        // letter when filtering is disabled is simply ignored. No
        // navigation, no selection.
        let input: ChoiceInput<String> = ChoiceInput::new("x", "p")
            .with_filter_enabled(false)
            .with_options(vec![
                ChoiceOption::new("a", "Apple", "apple"),
                ChoiceOption::new("b", "Banana", "banana"),
            ]);
        let mut state = ChooseOneState::new(input);
        let outcome =
            ChooseOne::<String>::new().handle_event(&mut state, press(KeyCode::Char('b')));
        assert_eq!(outcome, EventOutcome::Ignored);
        assert!(!state.filter_visible());
        assert!(state.selected_index().is_none());
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

    // -----------------------------------------------------------------
    // Phase 6 — Horizontal Layout & Navigation
    // -----------------------------------------------------------------

    fn horizontal_fixture_input() -> ChoiceInput<String> {
        ChoiceInput::new("color", "Pick a color")
            .with_orientation(Orientation::Horizontal)
            .with_options(vec![
                ChoiceOption::new("r", "Red", "red"),
                ChoiceOption::new("g", "Green", "green"),
                ChoiceOption::new("b", "Blue", "blue"),
                ChoiceOption::new("y", "Yellow", "yellow"),
            ])
    }

    #[test]
    fn horizontal_render_packs_items_left_to_right() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        // Wide area: all 4 items should fit on one row.
        let area = Rect::new(0, 0, 80, 2);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        // All options should be on row 0.
        let row0 = buffer_row(&buf, 0);
        assert!(row0.contains("Red"), "expected Red on row 0: {row0}");
        assert!(row0.contains("Green"), "expected Green on row 0: {row0}");
        assert!(row0.contains("Blue"), "expected Blue on row 0: {row0}");
        assert!(row0.contains("Yellow"), "expected Yellow on row 0: {row0}");
    }

    #[test]
    fn horizontal_render_no_triangular_pointer() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        let area = Rect::new(0, 0, 80, 2);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        // No ▶ pointer in horizontal mode.
        let row0 = buffer_row(&buf, 0);
        assert!(
            !row0.contains('▶'),
            "horizontal mode should not show triangular pointer: {row0}"
        );
    }

    #[test]
    fn horizontal_render_wraps_to_new_rows() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        // Narrow area: items should wrap to multiple rows.
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseOne::new().render(area, &mut buf, &mut state);

        // At least one option should be on row 1 due to wrapping.
        let row1 = buffer_row(&buf, 1);
        assert!(
            row1.contains("Green") || row1.contains("Blue") || row1.contains("Yellow"),
            "expected wrapped options on row 1: {row1}"
        );
    }

    #[test]
    fn choose_one_horizontal_badges_short_viewport_does_not_draw_past_area() {
        let input: ChoiceInput<String> = ChoiceInput::new("color", "Pick a color")
            .with_orientation(Orientation::Horizontal)
            .with_options(vec![
                ChoiceOption::new("a", "Alpha", "alpha").with_hotkey(HotkeySpec::Ctrl('a')),
                ChoiceOption::new("b", "Bravo", "bravo").with_hotkey(HotkeySpec::Ctrl('b')),
                ChoiceOption::new("c", "Charlie", "charlie").with_hotkey(HotkeySpec::Ctrl('c')),
            ]);
        let mut state = ChooseOneState::new(input).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        let area = Rect::new(0, 0, 12, 3);
        let mut buf = Buffer::empty(area);

        ChooseOne::new().render(area, &mut buf, &mut state);

        assert!(state.layout_cache.row_count() >= 3);
        assert_eq!(state.scroll_offset, 0);
        assert!(buffer_row(&buf, 0).contains("Alpha"));
        assert!(buffer_row(&buf, 1).contains("^A"));
        for y in 0..area.height {
            let row = buffer_row(&buf, y);
            assert!(
                !row.contains("Bravo") && !row.contains("Charlie"),
                "short viewport must not render hidden logical rows on y={y}: {row:?}"
            );
        }
    }

    #[test]
    fn horizontal_left_moves_to_previous_option() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        // Move to option 1.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Right));
        assert_eq!(state.hover(), Some(1));

        // Left moves back to option 0.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Left));
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn horizontal_right_moves_to_next_option() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Right));
        assert_eq!(state.hover(), Some(1));

        ChooseOne::new().handle_event(&mut state, press(KeyCode::Right));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn horizontal_up_moves_to_closest_column_above() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        // Navigate to an option likely on row 1 after wrapping.
        for _ in 0..3 {
            ChooseOne::new().handle_event(&mut state, press(KeyCode::Right));
        }
        let hover_before = state.hover().unwrap();

        // Up should move to the closest column in the row above.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Up));
        let hover_after = state.hover().unwrap();
        assert_ne!(hover_after, hover_before, "up should change hover");
    }

    #[test]
    fn horizontal_down_moves_to_closest_column_below() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        // Start on row 0, move down to row 1.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        let hover_after = state.hover().unwrap();
        assert_ne!(hover_after, 0, "down should change hover from 0");
    }

    #[test]
    fn horizontal_stale_cache_fallback_to_sequential() {
        let mut state = ChooseOneState::new(horizontal_fixture_input());
        // Clear layout cache to simulate stale state.
        state.layout_cache = ChoiceLayout::default();

        // Up/Down should fallback to sequential movement when cache is empty.
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover(), Some(1));

        state.layout_cache = ChoiceLayout::default();
        ChooseOne::new().handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.hover(), Some(0));
    }

    fn unsorted_input() -> ChoiceInput<String> {
        ChoiceInput::new("fruit", "Pick").with_options(vec![
            ChoiceOption::new("b", "Berry", "berry"),
            ChoiceOption::new("a", "Apple", "apple"),
            ChoiceOption::new("c", "Cherry", "cherry"),
        ])
    }

    fn labels(state: &ChooseOneState) -> Vec<&str> {
        state.options().iter().map(|o| o.label.as_str()).collect()
    }

    #[test]
    fn state_new_applies_with_sort_asc_orders_by_label() {
        let input = unsorted_input().with_sort(crate::core::SortOrder::Asc);
        let state = ChooseOneState::new(input);
        assert_eq!(labels(&state), vec!["Apple", "Berry", "Cherry"]);
    }

    #[test]
    fn state_new_applies_with_sort_desc_orders_by_label() {
        let input = unsorted_input().with_sort(crate::core::SortOrder::Desc);
        let state = ChooseOneState::new(input);
        assert_eq!(labels(&state), vec!["Cherry", "Berry", "Apple"]);
    }

    #[test]
    fn state_new_applies_with_sort_inverse_reverses_natural() {
        // `SortOrder::Inverse` is the canonical reversing ordering used
        // by both the library and the CLI `--sort inverse` surface.
        let input = unsorted_input().with_sort(crate::core::SortOrder::Inverse);
        let state = ChooseOneState::new(input);
        assert_eq!(labels(&state), vec!["Cherry", "Apple", "Berry"]);
    }

    #[test]
    fn state_new_with_sort_natural_is_no_op() {
        let input = unsorted_input().with_sort(crate::core::SortOrder::Natural);
        let state = ChooseOneState::new(input);
        assert_eq!(labels(&state), vec!["Berry", "Apple", "Cherry"]);
    }

    #[test]
    fn state_new_without_sort_preserves_input_order() {
        // `with_sort` is never called, so the configured order is
        // preserved exactly as given.
        let state = ChooseOneState::new(unsorted_input());
        assert_eq!(labels(&state), vec!["Berry", "Apple", "Cherry"]);
    }

    #[test]
    fn state_new_sort_then_shuffle_does_not_panic() {
        // Sorting and shuffling are independent: the constructor sorts
        // first, then shuffles. Either ordering must be acceptable as a
        // post-condition; this test pins that the combination simply
        // does not panic and yields the same number of options.
        let input = unsorted_input()
            .with_sort(crate::core::SortOrder::Asc)
            .with_shuffle_options(true);
        let state = ChooseOneState::new(input);
        assert_eq!(state.options().len(), 3);
    }

    // --- Phase 6: hotkey badge display state ---------------------------

    fn ctrl_press(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn alt_press(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
    }

    fn modifier_press(mod_key: crossterm::event::ModifierKeyCode) -> KeyEvent {
        KeyEvent::new(KeyCode::Modifier(mod_key), KeyModifiers::NONE)
    }

    fn modifier_release(mod_key: crossterm::event::ModifierKeyCode) -> KeyEvent {
        let mut event = KeyEvent::new(KeyCode::Modifier(mod_key), KeyModifiers::NONE);
        event.kind = KeyEventKind::Release;
        event
    }

    #[test]
    fn hotkey_display_initially_hidden() {
        let state = ChooseOneState::new(fixture_input());
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
    }

    #[test]
    fn hotkey_display_transitions_to_ctrl_held_on_ctrl_modifier_press() {
        use crossterm::event::ModifierKeyCode;
        let mut state = ChooseOneState::new(fixture_input());
        let outcome =
            ChooseOne::new().handle_event(&mut state, modifier_press(ModifierKeyCode::LeftControl));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
    }

    #[test]
    fn hotkey_display_returns_to_hidden_on_ctrl_modifier_release() {
        use crossterm::event::ModifierKeyCode;
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, modifier_press(ModifierKeyCode::LeftControl));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
        ChooseOne::new().handle_event(&mut state, modifier_release(ModifierKeyCode::LeftControl));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
    }

    #[test]
    fn hotkey_display_alt_modifier_transition_is_symmetric() {
        use crossterm::event::ModifierKeyCode;
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, modifier_press(ModifierKeyCode::LeftAlt));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::AltHeld);
        ChooseOne::new().handle_event(&mut state, modifier_release(ModifierKeyCode::LeftAlt));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
    }

    #[test]
    fn hotkey_display_briefly_visible_after_ctrl_chord_via_deadline() {
        // Even on terminals that never emit modifier-only events, a
        // Ctrl chord must arm the deadline so badges become discoverable.
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne::new().handle_event(&mut state, ctrl_press('z'));
        // The chord itself is unmapped (no Ctrl+Z hotkey is set on
        // any option), but the modifier still arms the deadline.
        assert!(outcome == EventOutcome::Ignored || outcome == EventOutcome::Consumed);
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
        // Resolved at "now": still inside the fallback window.
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::CtrlHeld
        );
        // Resolved well past the fallback window: collapses to Hidden.
        assert_eq!(
            state.current_hotkey_display(
                Instant::now() + HOTKEY_DISPLAY_FALLBACK + Duration::from_millis(50)
            ),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn hotkey_display_alt_chord_arms_deadline() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, alt_press('q'));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::AltHeld);
    }

    #[test]
    fn with_hotkey_display_forces_mode_and_clears_deadline() {
        let state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
        // No deadline set → resolver returns the forced mode at any time.
        assert_eq!(
            state.current_hotkey_display(Instant::now() + Duration::from_secs(60)),
            HotkeyDisplayMode::CtrlHeld
        );
    }

    #[test]
    fn current_hotkey_display_with_no_deadline_returns_stored_mode() {
        let state = ChooseOneState::new(fixture_input());
        // Default is Hidden / None → Hidden at any time.
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn with_hotkey_display_hidden_survives_ctrl_modifier_press() {
        use crossterm::event::ModifierKeyCode;
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
        ChooseOne::new().handle_event(&mut state, modifier_press(ModifierKeyCode::LeftControl));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
        assert!(state.hotkey_display_deadline.is_none());
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn with_hotkey_display_hidden_survives_alt_modifier_press() {
        use crossterm::event::ModifierKeyCode;
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
        ChooseOne::new().handle_event(&mut state, modifier_press(ModifierKeyCode::LeftAlt));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
        assert!(state.hotkey_display_deadline.is_none());
    }

    #[test]
    fn with_hotkey_display_hidden_survives_chord_fallback() {
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
        ChooseOne::new().handle_event(&mut state, ctrl_press('a'));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
        assert!(state.hotkey_display_deadline.is_none());
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn with_hotkey_display_ctrl_held_survives_modifier_release() {
        use crossterm::event::ModifierKeyCode;
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        ChooseOne::new().handle_event(&mut state, modifier_release(ModifierKeyCode::LeftControl));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
        assert!(state.hotkey_display_deadline.is_none());
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::CtrlHeld
        );
    }

    #[test]
    fn with_hotkey_display_alt_held_survives_modifier_release() {
        use crossterm::event::ModifierKeyCode;
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::AltHeld);
        ChooseOne::new().handle_event(&mut state, modifier_release(ModifierKeyCode::LeftAlt));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::AltHeld);
        assert!(state.hotkey_display_deadline.is_none());
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::AltHeld
        );
    }

    #[test]
    fn with_hotkey_display_ctrl_held_not_overwritten_by_alt_event() {
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        ChooseOne::new().handle_event(&mut state, alt_press('q'));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
        assert!(state.hotkey_display_deadline.is_none());
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::CtrlHeld
        );
    }

    #[test]
    fn with_hotkey_display_alt_held_not_overwritten_by_ctrl_event() {
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::AltHeld);
        ChooseOne::new().handle_event(&mut state, ctrl_press('q'));
        assert_eq!(state.hotkey_display(), HotkeyDisplayMode::AltHeld);
        assert!(state.hotkey_display_deadline.is_none());
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::AltHeld
        );
    }

    // --- Sticky `Ctrl+Space` / `Alt+Space` toggle ----------------------------
    //
    // The sticky toggle is the portable fallback for badge visibility.
    // It MUST work on every terminal regardless of whether bare modifier
    // press/release events are emitted, so the user always has a way to
    // surface the spec'd hotkey-emphasis UX.

    fn space_with_modifier(modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(' '), modifiers)
    }

    #[test]
    fn ctrl_space_toggles_sticky_ctrl_held() {
        let mut state = ChooseOneState::new(fixture_input());
        assert_eq!(state.hotkey_display_sticky(), None);
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::CONTROL));
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::CtrlHeld
        );
    }

    #[test]
    fn ctrl_space_legacy_nul_encoding_also_toggles_sticky_ctrl_held() {
        // The dominant encoding for `Ctrl+Space` outside the kitty
        // keyboard protocol is ASCII NUL (`\0`) — the binary receives
        // `KeyCode::Char('\0')` with the CONTROL modifier set. The
        // toggle MUST fire on this form too. A previous version of
        // the matcher only accepted `Char(' ') + CONTROL` and silently
        // dropped the NUL form, so users on non-kitty paths could press
        // `Ctrl+Space` and see no response despite the bytes reaching
        // the binary.
        let mut state = ChooseOneState::new(fixture_input());
        let event = KeyEvent::new(KeyCode::Char('\0'), KeyModifiers::CONTROL);
        ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
    }

    #[test]
    fn bare_nul_byte_without_control_modifier_does_not_toggle() {
        // The NUL-byte form is only honoured when CONTROL is set —
        // otherwise a stray `\0` from some other byte source could
        // accidentally toggle the badge mode.
        let mut state = ChooseOneState::new(fixture_input());
        let event = KeyEvent::new(KeyCode::Char('\0'), KeyModifiers::NONE);
        ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(state.hotkey_display_sticky(), None);
    }

    #[test]
    fn ctrl_slash_does_not_toggle_sticky() {
        // `Ctrl+/` was briefly added as a macOS-friendly alternative
        // chord, but never asked for. It must not toggle anything —
        // only `Ctrl+Space` and `Alt+Space` are accepted.
        let mut state = ChooseOneState::new(fixture_input());
        let event = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL);
        ChooseOne::new().handle_event(&mut state, event);
        assert_eq!(state.hotkey_display_sticky(), None);
    }

    #[test]
    fn alt_space_toggles_sticky_alt_held() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::ALT));
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::AltHeld)
        );
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::AltHeld
        );
    }

    #[test]
    fn sticky_toggle_release_clears() {
        // Press of Ctrl+Space sets the sticky display; the matching
        // Release event clears it. This mirrors the "hold modifier
        // to show badges" UX — a tap of Ctrl+Space briefly shows
        // the Ctrl-emphasis state.
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::CONTROL));
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
        let release = KeyEvent {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        };
        ChooseOne::new().handle_event(&mut state, release);
        assert_eq!(state.hotkey_display_sticky(), None);
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn sticky_toggle_legacy_nul_release_clears() {
        // Same as the previous test but with the legacy NUL encoding
        // for Ctrl+Space release (the form a non-kitty terminal would
        // emit if it emitted release events at all).
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(
            &mut state,
            KeyEvent::new(KeyCode::Char('\0'), KeyModifiers::CONTROL),
        );
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
        let release = KeyEvent {
            code: KeyCode::Char('\0'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        };
        ChooseOne::new().handle_event(&mut state, release);
        assert_eq!(state.hotkey_display_sticky(), None);
    }

    #[test]
    fn sticky_toggle_other_chord_switches_mode() {
        // `Ctrl+Space` then `Alt+Space` switches emphasis instead of
        // clearing — the user moves directly between the two states.
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::CONTROL));
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::ALT));
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::AltHeld)
        );
    }

    #[test]
    fn sticky_toggle_suppressed_when_override_active() {
        // The CLI lifetime override (`--hotkey-badges always` / `never`)
        // is the source of truth — the sticky toggle MUST NOT mutate
        // visibility while the override is in effect.
        let mut state =
            ChooseOneState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::CONTROL));
        assert_eq!(state.hotkey_display_sticky(), None);
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn modifier_release_clears_sticky_toggle() {
        // User flow: hold Ctrl (bare press) → press Space (chord) →
        // release Ctrl. Without this clear, sticky would remain set
        // and badges would stay visible forever. With it, release
        // restores hidden state cleanly.
        use crossterm::event::ModifierKeyCode;
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne::new().handle_event(&mut state, modifier_press(ModifierKeyCode::LeftControl));
        ChooseOne::new().handle_event(&mut state, space_with_modifier(KeyModifiers::CONTROL));
        assert_eq!(
            state.hotkey_display_sticky(),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
        ChooseOne::new().handle_event(&mut state, modifier_release(ModifierKeyCode::LeftControl));
        assert_eq!(state.hotkey_display_sticky(), None);
        assert_eq!(
            state.current_hotkey_display(Instant::now()),
            HotkeyDisplayMode::Hidden
        );
    }

    #[test]
    fn lone_release_without_prior_press_is_a_safe_clear() {
        // Receiving an unmatched Release of `Ctrl+Space` (e.g. when
        // the prompt opens just as the user is releasing a chord they
        // pressed earlier) clears the sticky mode. Since None → None
        // is a no-op, this is benign.
        let mut state = ChooseOneState::new(fixture_input());
        let release = KeyEvent {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        };
        ChooseOne::new().handle_event(&mut state, release);
        assert_eq!(state.hotkey_display_sticky(), None);
    }
}
