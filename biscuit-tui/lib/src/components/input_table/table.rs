//! Grid container widget that owns a 2D matrix of [`CellState`]s.
//!
//! [`InputTable`] is a zero-sized [`StatefulWidget`] marker; the full
//! per-cell matrix, focus coordinates, theme, submit-key binding and
//! any aggregated validation error live on [`InputTableState`].
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//! use tui_chrome::components::input_table::{
//!     BooleanSwitchConfig, InputTable, InputTableColumn, InputTableState,
//!     TextInputConfig,
//! };
//!
//! let columns = vec![
//!     InputTableColumn::StaticText("Row".into()),
//!     InputTableColumn::TextInput(TextInputConfig::default()),
//!     InputTableColumn::BooleanSwitch(BooleanSwitchConfig::default()),
//! ];
//! let state = InputTableState::new(columns, 2);
//! let _widget = InputTable::new();
//! let _ = state; // built successfully
//! ```

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::StatefulWidget,
};

use crate::components::{
    BooleanSwitch, ChooseMany, ChooseOne, TextAreaInput, TextInput,
};
use crate::core::{ComponentTheme, EventOutcome, HandleEvent, StandaloneState, ValidationState};

use super::cell::CellState;
use super::column::InputTableColumn;

/// Default submit key — `Ctrl-S`.
const DEFAULT_SUBMIT_CODE: KeyCode = KeyCode::Char('s');
const DEFAULT_SUBMIT_MODIFIERS: KeyModifiers = KeyModifiers::CONTROL;

/// Mutable state for an [`InputTable`] widget.
///
/// Holds the column schema, the cell matrix, focus coordinates,
/// theme, submit-key binding, and any aggregated validation error.
#[derive(Debug, Clone)]
pub struct InputTableState {
    columns: Vec<InputTableColumn>,
    rows: Vec<Vec<CellState>>,
    focus_row: usize,
    focus_col: usize,
    theme: ComponentTheme,
    submit_code: KeyCode,
    submit_modifiers: KeyModifiers,
    validation_error: Option<String>,
}

impl InputTableState {
    /// Creates a new state with `row_count` rows, each seeded from
    /// the column configuration payloads.
    ///
    /// The initial focus lands on the first focusable cell (columns
    /// of type [`InputTableColumn::StaticText`] are skipped). When no
    /// column is focusable, focus defaults to `(0, 0)`.
    pub fn new(columns: Vec<InputTableColumn>, row_count: usize) -> Self {
        let rows: Vec<Vec<CellState>> = (0..row_count)
            .map(|_| columns.iter().map(CellState::from_column).collect())
            .collect();
        let (focus_row, focus_col) = first_focusable_cell(&rows).unwrap_or((0, 0));
        Self {
            columns,
            rows,
            focus_row,
            focus_col,
            theme: ComponentTheme::default(),
            submit_code: DEFAULT_SUBMIT_CODE,
            submit_modifiers: DEFAULT_SUBMIT_MODIFIERS,
            validation_error: None,
        }
    }

    /// Replaces the active theme.
    pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Overrides the submit key.
    pub fn with_submit_key(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.submit_code = code;
        self.submit_modifiers = modifiers;
        self
    }

    /// Replaces the initial value of a specific cell.
    ///
    /// Silently no-ops when `row` or `col` is out of range.
    pub fn set_cell_initial(&mut self, row: usize, col: usize, value: &str) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(col)) {
            cell.set_initial_value(value);
        }
    }

    /// Returns the column schema.
    pub fn columns(&self) -> &[InputTableColumn] {
        &self.columns
    }

    /// Returns the 2D cell matrix.
    pub fn rows(&self) -> &[Vec<CellState>] {
        &self.rows
    }

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Returns the number of columns.
    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    /// Returns the focused cell as `(row, col)`.
    pub fn focus(&self) -> (usize, usize) {
        (self.focus_row, self.focus_col)
    }

    /// Returns the current table-level validation error, if any.
    ///
    /// This is distinct from per-cell validation errors (which each
    /// cell renders inline). The table-level error only surfaces when
    /// submit is attempted while at least one cell has an active
    /// validation failure.
    pub fn table_validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    /// Returns a reference to the theme.
    pub fn theme(&self) -> &ComponentTheme {
        &self.theme
    }

    /// Returns the captured values as `Vec<Vec<String>>`, one inner
    /// vec per row, one `String` per column.
    pub fn values(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| row.iter().map(CellState::value_string).collect())
            .collect()
    }

    fn is_submit_key(&self, event: &KeyEvent) -> bool {
        event.code == self.submit_code && event.modifiers == self.submit_modifiers
    }
}

impl ValidationState for InputTableState {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl StandaloneState for InputTableState {
    type Value = Vec<Vec<String>>;

    fn value(&self) -> Self::Value {
        self.values()
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }
}

/// Grid widget rendered via [`StatefulWidget`] against an
/// [`InputTableState`]. The widget itself is zero-sized — all state
/// lives on the paired state struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputTable;

impl InputTable {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for InputTable {
    type State = InputTableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 || state.columns.is_empty() {
            return;
        }

        let column_widths = compute_column_widths(area.width, state.columns.len());
        let row_heights: Vec<u16> = state
            .rows
            .iter()
            .map(|row| row.iter().map(CellState::min_height).max().unwrap_or(1).max(1))
            .collect();

        let total_height: u16 = row_heights.iter().copied().sum();
        let error_row = state.validation_error.is_some() && total_height < area.height;
        let reserve_error = if error_row { 1u16 } else { 0u16 };

        let available_rows = area.height.saturating_sub(reserve_error);
        let row_rects = layout_rows(area, &row_heights, available_rows);

        for (row_idx, row_rect) in row_rects.iter().enumerate() {
            let col_rects = layout_columns(*row_rect, &column_widths);
            for (col_idx, cell_rect) in col_rects.iter().enumerate() {
                draw_cell(
                    *cell_rect,
                    buf,
                    &mut state.rows[row_idx][col_idx],
                    row_idx == state.focus_row && col_idx == state.focus_col,
                    &state.theme,
                );
            }
        }

        if let Some(message) = state.validation_error.as_deref()
            && error_row
        {
            let y = area.y + total_height.min(area.height.saturating_sub(1));
            let error_line = Line::from(Span::styled(
                message.to_string(),
                state.theme.error_style,
            ));
            buf.set_line(area.x, y, &error_line, area.width);
        }
    }
}

impl HandleEvent for InputTable {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        if event.code == KeyCode::Esc && event.modifiers == KeyModifiers::NONE {
            return EventOutcome::Cancelled;
        }

        if state.is_submit_key(&event) {
            return submit(state);
        }

        match (event.code, event.modifiers) {
            (KeyCode::Tab, m) if m == KeyModifiers::NONE => {
                move_tab(state, 1);
                EventOutcome::Consumed
            }
            (KeyCode::BackTab, _) => {
                move_tab(state, -1);
                EventOutcome::Consumed
            }
            _ => {
                if let Some(outcome) = try_navigate(state, event) {
                    outcome
                } else {
                    route_to_focus(state, event)
                }
            }
        }
    }
}

fn submit(state: &mut InputTableState) -> EventOutcome {
    // Aggregate validation errors; route focus to the first offender.
    let mut offender: Option<(usize, usize)> = None;
    for (r, row) in state.rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell.validation_error().is_some() {
                offender = Some((r, c));
                break;
            }
        }
        if offender.is_some() {
            break;
        }
    }

    // Also trigger per-cell submit validation by simulating an Enter
    // on each focusable cell that needs it (required/min_selections
    // violations only surface on an explicit submit attempt). We only
    // route this to cells that have not already submitted.
    if offender.is_none() {
        for (r, row) in state.rows.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                if !cell.is_focusable() {
                    continue;
                }
                if let CellState::ChooseOne(choose_state) = cell
                    && choose_state.required()
                    && choose_state.selected_value().is_none()
                {
                    choose_state.set_validation_error("Please make a selection");
                    if offender.is_none() {
                        offender = Some((r, c));
                    }
                }
                if let CellState::ChooseMany(choose_state) = cell {
                    let selected: Vec<_> = choose_state.selected_values();
                    if choose_state.required() && selected.is_empty() {
                        choose_state.set_validation_error("Please make a selection");
                        if offender.is_none() {
                            offender = Some((r, c));
                        }
                    } else if let Some(min) = choose_state.min_selections()
                        && selected.len() < min
                    {
                        choose_state
                            .set_validation_error(format!("Please select at least {min}"));
                        if offender.is_none() {
                            offender = Some((r, c));
                        }
                    }
                }
            }
        }
    }

    if let Some((r, c)) = offender {
        state.focus_row = r;
        state.focus_col = c;
        state.validation_error =
            Some("One or more cells need your attention".into());
        return EventOutcome::Consumed;
    }

    state.validation_error = None;
    EventOutcome::Submitted
}

fn route_to_focus(state: &mut InputTableState, event: KeyEvent) -> EventOutcome {
    let Some(cell) = state
        .rows
        .get_mut(state.focus_row)
        .and_then(|r| r.get_mut(state.focus_col))
    else {
        return EventOutcome::Ignored;
    };
    if !cell.is_focusable() {
        return EventOutcome::Ignored;
    }
    match cell.handle_event(event) {
        // Swallow per-cell submit/cancel — only the table level
        // submit key commits the whole grid.
        EventOutcome::Submitted | EventOutcome::Cancelled | EventOutcome::Consumed => {
            state.validation_error = None;
            EventOutcome::Consumed
        }
        EventOutcome::Ignored => EventOutcome::Ignored,
    }
}

fn try_navigate(state: &mut InputTableState, event: KeyEvent) -> Option<EventOutcome> {
    if event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        return None;
    }
    let inside_text_cell = matches!(
        state.rows.get(state.focus_row).and_then(|r| r.get(state.focus_col)),
        Some(CellState::TextInput(_)) | Some(CellState::TextAreaInput(_))
    );
    match event.code {
        KeyCode::Up => {
            move_focus(state, -1, 0);
            Some(EventOutcome::Consumed)
        }
        KeyCode::Down => {
            move_focus(state, 1, 0);
            Some(EventOutcome::Consumed)
        }
        // Horizontal arrows inside editable text cells are consumed by
        // the cell for cursor movement; elsewhere they navigate.
        KeyCode::Left if !inside_text_cell => {
            move_focus(state, 0, -1);
            Some(EventOutcome::Consumed)
        }
        KeyCode::Right if !inside_text_cell => {
            move_focus(state, 0, 1);
            Some(EventOutcome::Consumed)
        }
        _ => None,
    }
}

fn move_focus(state: &mut InputTableState, drow: i32, dcol: i32) {
    let rows = state.row_count() as i32;
    let cols = state.col_count() as i32;
    if rows == 0 || cols == 0 {
        return;
    }
    let mut r = state.focus_row as i32 + drow;
    let mut c = state.focus_col as i32 + dcol;
    if r < 0 {
        r = rows - 1;
    } else if r >= rows {
        r = 0;
    }
    if c < 0 {
        c = cols - 1;
    } else if c >= cols {
        c = 0;
    }
    let (r_u, c_u) = (r as usize, c as usize);
    if let Some(next) = find_focusable(state, r_u, c_u, drow, dcol) {
        state.focus_row = next.0;
        state.focus_col = next.1;
    }
}

fn find_focusable(
    state: &InputTableState,
    start_row: usize,
    start_col: usize,
    drow: i32,
    dcol: i32,
) -> Option<(usize, usize)> {
    let rows = state.row_count();
    let cols = state.col_count();
    if rows == 0 || cols == 0 {
        return None;
    }
    let step_rows = if drow != 0 { rows } else { 1 };
    let step_cols = if dcol != 0 { cols } else { 1 };
    let limit = step_rows.max(step_cols).max(1);
    let mut r = start_row as i32;
    let mut c = start_col as i32;
    for _ in 0..=limit {
        if r >= 0
            && c >= 0
            && (r as usize) < rows
            && (c as usize) < cols
            && state.rows[r as usize][c as usize].is_focusable()
        {
            return Some((r as usize, c as usize));
        }
        r += drow;
        c += dcol;
        if rows > 0 && rows as i32 > 0 {
            if r < 0 {
                r = rows as i32 - 1;
            } else if r >= rows as i32 {
                r = 0;
            }
        }
        if cols > 0 && cols as i32 > 0 {
            if c < 0 {
                c = cols as i32 - 1;
            } else if c >= cols as i32 {
                c = 0;
            }
        }
    }
    None
}

fn move_tab(state: &mut InputTableState, delta: i32) {
    let rows = state.row_count();
    let cols = state.col_count();
    if rows == 0 || cols == 0 {
        return;
    }
    let total = (rows * cols) as i32;
    let mut flat = (state.focus_row * cols + state.focus_col) as i32;
    for _ in 0..total {
        flat += delta;
        if flat < 0 {
            flat += total;
        } else if flat >= total {
            flat -= total;
        }
        let r = (flat / cols as i32) as usize;
        let c = (flat % cols as i32) as usize;
        if state.rows[r][c].is_focusable() {
            state.focus_row = r;
            state.focus_col = c;
            return;
        }
    }
}

fn first_focusable_cell(rows: &[Vec<CellState>]) -> Option<(usize, usize)> {
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell.is_focusable() {
                return Some((r, c));
            }
        }
    }
    None
}

fn compute_column_widths(total_width: u16, column_count: usize) -> Vec<u16> {
    if column_count == 0 {
        return Vec::new();
    }
    let base = total_width / column_count as u16;
    let remainder = total_width % column_count as u16;
    (0..column_count)
        .map(|i| base + if (i as u16) < remainder { 1 } else { 0 })
        .collect()
}

fn layout_rows(area: Rect, row_heights: &[u16], available: u16) -> Vec<Rect> {
    if row_heights.is_empty() || available == 0 {
        return Vec::new();
    }
    let constraints: Vec<Constraint> =
        row_heights.iter().map(|h| Constraint::Length(*h)).collect();
    let row_area = Rect::new(area.x, area.y, area.width, available);
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(row_area)
        .to_vec()
}

fn layout_columns(row_rect: Rect, column_widths: &[u16]) -> Vec<Rect> {
    if column_widths.is_empty() {
        return Vec::new();
    }
    let constraints: Vec<Constraint> = column_widths
        .iter()
        .map(|w| Constraint::Length(*w))
        .collect();
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(row_rect)
        .to_vec()
}

fn draw_cell(
    area: Rect,
    buf: &mut Buffer,
    cell: &mut CellState,
    focused: bool,
    theme: &ComponentTheme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    if focused {
        paint_focus_background(area, buf, theme);
    }

    match cell {
        CellState::StaticText(text) => {
            let line = Line::from(text.clone());
            buf.set_line(area.x, area.y, &line, area.width);
        }
        CellState::BooleanSwitch(state) => {
            BooleanSwitch.render(area, buf, state);
        }
        CellState::TextInput(state) => {
            TextInput.render(area, buf, state);
        }
        CellState::TextAreaInput(state) => {
            TextAreaInput.render(area, buf, state);
        }
        CellState::ChooseOne(state) => {
            ChooseOne.render(area, buf, state);
        }
        CellState::ChooseMany(state) => {
            ChooseMany.render(area, buf, state);
        }
    }
}

fn paint_focus_background(area: Rect, buf: &mut Buffer, theme: &ComponentTheme) {
    let underline = Style::default()
        .add_modifier(Modifier::UNDERLINED)
        .patch(theme.label_style);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_style(underline);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::choose::{ChoiceInput, ChoiceOption};
    use crate::components::input_table::column::{
        BooleanSwitchConfig, TextAreaInputConfig, TextInputConfig,
    };

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn sample_columns() -> Vec<InputTableColumn> {
        vec![
            InputTableColumn::StaticText("label".into()),
            InputTableColumn::TextInput(TextInputConfig::default()),
            InputTableColumn::BooleanSwitch(BooleanSwitchConfig::default()),
        ]
    }

    #[test]
    fn new_places_focus_on_first_focusable_cell() {
        let state = InputTableState::new(sample_columns(), 2);
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn new_populates_row_count_by_column_schema() {
        let state = InputTableState::new(sample_columns(), 3);
        assert_eq!(state.row_count(), 3);
        assert_eq!(state.col_count(), 3);
        assert_eq!(state.values().len(), 3);
        assert_eq!(state.values()[0].len(), 3);
    }

    #[test]
    fn right_arrow_moves_focus_to_next_focusable_column() {
        let mut state = InputTableState::new(sample_columns(), 1);
        // initial focus is (0,1). Right should move to (0,2).
        let outcome = InputTable.handle_event(&mut state, press(KeyCode::Right));
        // Right inside a TextInput is consumed for cursor; not navigation.
        // We rely on Tab here instead — left/right is blocked for text cells.
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn tab_moves_to_next_focusable_cell_across_rows() {
        let mut state = InputTableState::new(sample_columns(), 2);
        assert_eq!(state.focus(), (0, 1));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 2));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        // Next row, first focusable column (col 1).
        assert_eq!(state.focus(), (1, 1));
    }

    #[test]
    fn tab_wraps_to_first_cell() {
        let mut state = InputTableState::new(sample_columns(), 1);
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 2));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn backtab_moves_to_previous_focusable_cell() {
        let mut state = InputTableState::new(sample_columns(), 2);
        // focus: (0,1) → Tab → (0,2) → BackTab → (0,1)
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        let event = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        InputTable.handle_event(&mut state, event);
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn up_and_down_arrows_change_row() {
        let mut state = InputTableState::new(sample_columns(), 3);
        // Start at (0,1). Down should go to (1,1).
        InputTable.handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.focus(), (1, 1));
        InputTable.handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn up_from_first_row_wraps_to_last_row() {
        let mut state = InputTableState::new(sample_columns(), 3);
        InputTable.handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.focus(), (2, 1));
    }

    #[test]
    fn esc_cancels() {
        let mut state = InputTableState::new(sample_columns(), 1);
        let outcome = InputTable.handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn ctrl_s_submits_when_all_cells_valid() {
        let mut state = InputTableState::new(sample_columns(), 1);
        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn typing_into_focused_text_input_updates_value() {
        let mut state = InputTableState::new(sample_columns(), 1);
        InputTable.handle_event(&mut state, press(KeyCode::Char('h')));
        InputTable.handle_event(&mut state, press(KeyCode::Char('i')));
        assert_eq!(state.values()[0][1], "hi");
    }

    #[test]
    fn space_in_focused_boolean_switch_toggles() {
        let mut state = InputTableState::new(sample_columns(), 1);
        // Move focus from TextInput (col 1) to BooleanSwitch (col 2).
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 2));
        InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.values()[0][2], "true");
    }

    #[test]
    fn submit_with_required_choose_one_unset_sets_focus_to_offender_and_consumes() {
        let input = ChoiceInput::new("c", "p")
            .with_options(vec![ChoiceOption::new("a", "A", "alpha")])
            .required();
        let columns = vec![
            InputTableColumn::TextInput(TextInputConfig::default()),
            InputTableColumn::ChooseOne(input),
        ];
        let mut state = InputTableState::new(columns, 1);
        assert_eq!(state.focus(), (0, 0));
        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.focus(), (0, 1));
        assert!(state.table_validation_error().is_some());
    }

    #[test]
    fn submit_succeeds_after_required_cell_is_fixed() {
        let input = ChoiceInput::new("c", "p")
            .with_options(vec![ChoiceOption::new("a", "A", "alpha")])
            .required();
        let columns = vec![InputTableColumn::ChooseOne(input)];
        let mut state = InputTableState::new(columns, 1);
        InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        // Fix: select the single option.
        InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Submitted);
        assert_eq!(state.values()[0][0], "alpha");
    }

    #[test]
    fn set_cell_initial_replaces_text_input_value() {
        let mut state = InputTableState::new(sample_columns(), 2);
        state.set_cell_initial(1, 1, "hello");
        assert_eq!(state.values()[1][1], "hello");
    }

    #[test]
    fn set_cell_initial_on_static_text_overrides_display() {
        let mut state = InputTableState::new(sample_columns(), 1);
        state.set_cell_initial(0, 0, "custom");
        assert_eq!(state.values()[0][0], "custom");
    }

    #[test]
    fn values_joins_choose_many_with_comma() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
        ]);
        let columns = vec![InputTableColumn::ChooseMany(input)];
        let mut state = InputTableState::new(columns, 1);
        state.set_cell_initial(0, 0, "a,b");
        assert_eq!(state.values()[0][0], "alpha,beta");
    }

    #[test]
    fn render_mixed_cell_table_paints_static_and_body() {
        let columns = vec![
            InputTableColumn::StaticText("ID".into()),
            InputTableColumn::TextInput(TextInputConfig {
                initial: "alpha".into(),
                ..TextInputConfig::default()
            }),
            InputTableColumn::BooleanSwitch(BooleanSwitchConfig::default()),
        ];
        let mut state = InputTableState::new(columns, 1);
        let area = Rect::new(0, 0, 36, 1);
        let mut buf = Buffer::empty(area);
        InputTable.render(area, &mut buf, &mut state);

        let mut row = String::new();
        for x in area.left()..area.right() {
            row.push_str(buf[(x, area.y)].symbol());
        }
        let row = row.trim_end();
        assert!(row.starts_with("ID"), "expected static text, got {row:?}");
        assert!(row.contains("alpha"), "expected text-input value, got {row:?}");
    }

    #[test]
    fn render_supports_text_area_cell_with_preferred_height() {
        let columns = vec![InputTableColumn::TextAreaInput(TextAreaInputConfig {
            preferred_width: 10,
            preferred_height: 2,
            initial: vec!["a".into(), "b".into()],
            scrollbar: false,
        })];
        let mut state = InputTableState::new(columns, 1);
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        InputTable.render(area, &mut buf, &mut state);
        let read_row = |y: u16| -> String {
            let mut s = String::new();
            for x in area.left()..area.right() {
                s.push_str(buf[(x, y)].symbol());
            }
            s.trim_end().to_string()
        };
        assert_eq!(read_row(0), "a");
        assert_eq!(read_row(1), "b");
    }

    #[test]
    fn values_encodes_boolean_as_true_or_false() {
        let mut state = InputTableState::new(sample_columns(), 1);
        // Focus on BooleanSwitch (col 2), toggle on.
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.values()[0][2], "true");
    }

    #[test]
    fn custom_submit_key_replaces_default() {
        let columns = vec![InputTableColumn::TextInput(TextInputConfig::default())];
        let mut state = InputTableState::new(columns, 1)
            .with_submit_key(KeyCode::F(2), KeyModifiers::NONE);
        let outcome =
            InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(outcome, EventOutcome::Submitted);
    }
}
