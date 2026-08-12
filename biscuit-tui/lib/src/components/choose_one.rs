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
//! use biscuit_tui::prelude::*;
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

    /// Returns the option at the current highlight (the "active" row),
    /// if any.
    ///
    /// Mirrors [`hover`](Self::hover) and applies no filtering of its
    /// own: it returns the option **as-is**, including a `disabled`
    /// one if the highlight rests on it (navigation already governs
    /// where the highlight may land). `None` means there is no active
    /// row — e.g. an empty option list.
    ///
    /// Distinct from [`selected_value`](Self::selected_value): the
    /// active row is the highlighted one, not the submitted
    /// selection. This is the read entry point for the master/detail
    /// pattern — a detail pane can derive its content from the active
    /// option each frame.
    pub fn active_option(&self) -> Option<&ChoiceOption<V>> {
        self.hover().and_then(|i| self.options().get(i))
    }

    /// Returns the value of the option at the current highlight, if
    /// any.
    ///
    /// Sugar over [`active_option`](Self::active_option); inherits its
    /// disabled-passthrough and `None`-means-no-active-row semantics.
    pub fn active_value(&self) -> Option<&V> {
        self.active_option().map(|o| &o.value)
    }

    /// Looks up a description for the active option in a caller-
    /// supplied `id -> description` map.
    ///
    /// Sugar over [`active_option`](Self::active_option): performs the
    /// `active_option().id -> map` lookup. The map is caller-owned —
    /// this does **not** add a description field to [`ChoiceOption`],
    /// which stays unchanged. `None` means either there is no active
    /// row, or the active option's `id` has no entry in `map`.
    ///
    /// The returned reference borrows from `map`, not `self`, so it
    /// can outlive a transient borrow of the state (e.g. across
    /// independent re-renders in a master/detail layout).
    pub fn active_description<'a>(
        &self,
        map: &'a HashMap<String, String>,
    ) -> Option<&'a str> {
        self.active_option()
            .and_then(|o| map.get(&o.id).map(String::as_str))
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

        // Ctrl/Alt hotkeys select and submit. Modifier matching uses
        // `.contains(...)` so a benign extra bit (e.g. SHIFT on an
        // uppercase chord) does not suppress an otherwise valid hotkey.
        // WHY CONTROL|ALT falls through: on some layouts AltGr-style
        // chords report CONTROL|ALT and must not be hijacked as a
        // hotkey, so when both are present neither map matches.
        let modifiers = event.modifiers;
        if modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT) {
            if let KeyCode::Char(c) = event.code
                && let Some(&idx) = state.ctrl_hotkeys.get(&c.to_ascii_lowercase())
            {
                select_at(state, idx);
                return EventOutcome::Submitted;
            }
        } else if modifiers.contains(KeyModifiers::ALT)
            && !modifiers.contains(KeyModifiers::CONTROL)
            && let KeyCode::Char(c) = event.code
            && let Some(&idx) = state.alt_hotkeys.get(&c.to_ascii_lowercase())
        {
            select_at(state, idx);
            return EventOutcome::Submitted;
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
mod tests;
