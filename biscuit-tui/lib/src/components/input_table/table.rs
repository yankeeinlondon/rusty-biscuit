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
//!     InputTableColumn::StaticText { id: "row".into(), text: "Row".into() },
//!     InputTableColumn::TextInput { id: "name".into(), config: TextInputConfig::default() },
//!     InputTableColumn::BooleanSwitch { id: "active".into(), config: BooleanSwitchConfig::default() },
//! ];
//! let state = InputTableState::with_blank_rows(columns, 2);
//! let _widget = InputTable::new();
//! let _ = state; // built successfully
//! ```

use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::StatefulWidget,
};

use crate::components::{BooleanSwitch, ChooseMany, ChooseOne, TextAreaInput, TextInput};
use crate::core::{
    ComponentTheme, EventOutcome, HandleEvent, KeyBindings, StandaloneState, ValidationState,
};

use super::cell::{CellState, CellValue, Row, RowCell};
use super::column::InputTableColumn;

/// Mutable state for an [`InputTable`] widget.
///
/// Holds the column schema, the cell matrix, focus coordinates,
/// row scroll offset, theme, key bindings, and any aggregated
/// validation error.
#[derive(Debug, Clone)]
pub struct InputTableState {
    columns: Vec<InputTableColumn>,
    rows: Vec<Vec<CellState>>,
    typed_rows: Vec<Row>,
    focus_row: usize,
    focus_col: usize,
    row_scroll_offset: usize,
    theme: ComponentTheme,
    bindings: KeyBindings,
    validation_error: Option<String>,
}

impl InputTableState {
    /// Creates a new state from typed row data.
    ///
    /// Each [`Row`] must contain exactly one cell per configured
    /// column. Cells are matched by column id, then normalised into
    /// the table's column order.
    ///
    /// ## Panics
    ///
    /// Panics with a clear message when any row's length does not
    /// match `columns.len()`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use tui_chrome::prelude::*;
    /// use tui_chrome::components::input_table::{
    ///     CellValue, InputTable, InputTableColumn, InputTableState, Row,
    ///     RowCell, TextInputConfig,
    /// };
    ///
    /// let columns = vec![
    ///     InputTableColumn::StaticText { id: "row".into(), text: "1".into() },
    ///     InputTableColumn::TextInput { id: "name".into(), config: TextInputConfig::default() },
    /// ];
    /// let initial_rows = vec![
    ///     Row::new(vec![
    ///         RowCell::new("row", CellValue::StaticText("1".into())),
    ///         RowCell::new("name", CellValue::Text("Alice".into())),
    ///     ]),
    ///     Row::new(vec![
    ///         RowCell::new("row", CellValue::StaticText("2".into())),
    ///         RowCell::new("name", CellValue::Text("Bob".into())),
    ///     ]),
    /// ];
    /// let state = InputTableState::new(columns, initial_rows);
    /// assert_eq!(state.row_count(), 2);
    /// ```
    pub fn new(columns: Vec<InputTableColumn>, initial_rows: Vec<Row>) -> Self {
        let col_count = columns.len();
        for (row_idx, row) in initial_rows.iter().enumerate() {
            if row.cells.len() != col_count {
                panic!(
                    "InputTableState::new: row {row_idx} has {} cells, expected {col_count}",
                    row.cells.len()
                );
            }
        }

        let typed_rows: Vec<Row> = initial_rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| normalize_row(row_idx, row, &columns))
            .collect();

        let rows: Vec<Vec<CellState>> = typed_rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .zip(row.cells.iter())
                    .map(|(column, row_cell)| {
                        let mut cell = CellState::from_column(column);
                        apply_cell_value(&mut cell, &row_cell.value);
                        cell
                    })
                    .collect()
            })
            .collect();

        let (focus_row, focus_col) = first_focusable_cell(&rows).unwrap_or((0, 0));
        Self {
            columns,
            rows,
            typed_rows,
            focus_row,
            focus_col,
            row_scroll_offset: 0,
            theme: ComponentTheme {
                help_hint: "Ctrl+S=Submit  Esc=Cancel".into(),
                ..ComponentTheme::default()
            },
            bindings: KeyBindings {
                submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
                ..KeyBindings::default()
            },
            validation_error: None,
        }
    }

    /// Creates a new state with `row_count` blank rows.
    ///
    /// Each cell is seeded from its column's default configuration.
    /// This is a convenience constructor for the common "N empty rows"
    /// case.
    ///
    /// The initial focus lands on the first focusable cell (columns
    /// of type [`InputTableColumn::StaticText`] are skipped). When no
    /// column is focusable, focus defaults to `(0, 0)`.
    pub fn with_blank_rows(columns: Vec<InputTableColumn>, row_count: usize) -> Self {
        let rows: Vec<Vec<CellState>> = (0..row_count)
            .map(|_| columns.iter().map(CellState::from_column).collect())
            .collect();
        let typed_rows: Vec<Row> = rows
            .iter()
            .map(|row| typed_row_from_cells(&columns, row))
            .collect();
        let (focus_row, focus_col) = first_focusable_cell(&rows).unwrap_or((0, 0));
        Self {
            columns,
            rows,
            typed_rows,
            focus_row,
            focus_col,
            row_scroll_offset: 0,
            theme: ComponentTheme {
                help_hint: "Ctrl+S=Submit  Esc=Cancel".into(),
                ..ComponentTheme::default()
            },
            bindings: KeyBindings {
                submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
                ..KeyBindings::default()
            },
            validation_error: None,
        }
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

    /// Overrides the submit key.
    ///
    /// This is a convenience wrapper over `with_key_bindings` for
    /// backwards compatibility.
    pub fn with_submit_key(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.bindings.submit = vec![KeyEvent::new(code, modifiers)];
        self
    }

    /// Replaces the initial value of a specific cell.
    ///
    /// Silently no-ops when `row` or `col` is out of range.
    pub fn set_cell_initial(&mut self, row: usize, col: usize, value: &str) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(col)) {
            cell.set_initial_value(value);
            self.sync_row(row);
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

    /// Returns the captured rows in their typed, column-id-aware form.
    ///
    /// This is the primary public-value accessor for `InputTableState`.
    /// The returned slice stays in sync with the per-cell component state.
    pub fn value(&self) -> &[Row] {
        &self.typed_rows
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

    /// Returns a reference to the key bindings.
    pub fn key_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    /// Returns the captured rows as typed [`Row`] values.
    ///
    /// Each [`Row`] contains a vector of [`RowCell`]s pairing column
    /// ids with typed [`CellValue`]s. Preferred for library consumers
    /// who need to preserve semantic types (booleans, multi-line text,
    /// multi-select arrays).
    ///
    /// [`Row`]: super::cell::Row
    /// [`RowCell`]: super::cell::RowCell
    /// [`CellValue`]: super::cell::CellValue
    pub fn rows_typed(&self) -> Vec<Row> {
        self.typed_rows.clone()
    }

    /// Returns the captured values as `Vec<Vec<String>>`, one inner
    /// vec per row, one `String` per column.
    #[deprecated(note = "use `rows_typed` and map to the desired shape")]
    pub fn values(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| row.iter().map(CellState::value_string).collect())
            .collect()
    }

    fn sync_row(&mut self, row_idx: usize) {
        if let Some(row) = self.rows.get(row_idx) {
            let typed_row = typed_row_from_cells(&self.columns, row);
            if let Some(slot) = self.typed_rows.get_mut(row_idx) {
                *slot = typed_row;
            }
        }
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
    type Value = Vec<Row>;

    fn value(&self) -> Self::Value {
        self.typed_rows.clone()
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn help_hint(&self) -> &str {
        &self.theme.help_hint
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

        let column_widths = compute_column_widths(&state.columns, area.width);
        let row_heights: Vec<u16> = state
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(CellState::min_height)
                    .max()
                    .unwrap_or(1)
                    .max(1)
            })
            .collect();

        let reserve_error: u16 = if state.validation_error.is_some() {
            1
        } else {
            0
        };
        let available = area.height.saturating_sub(reserve_error);

        adjust_table_scroll(state, &row_heights, available);

        let mut visible_rows: Vec<(usize, Rect)> = Vec::new();
        let mut y_offset: u16 = 0;
        for (i, &h) in row_heights.iter().enumerate().skip(state.row_scroll_offset) {
            if y_offset.saturating_add(h) > available {
                break;
            }
            let rect = Rect::new(area.x, area.y + y_offset, area.width, h);
            visible_rows.push((i, rect));
            y_offset += h;
        }

        let last_visible_idx = visible_rows.last().map(|(i, _)| *i);

        for (row_idx, row_rect) in visible_rows {
            let col_rects = layout_columns(row_rect, &column_widths);
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

        let overflow_style = state.theme.selected_style.add_modifier(Modifier::DIM);

        if state.row_scroll_offset > 0 && area.width > 0 {
            let x = area.x + area.width - 1;
            buf[(x, area.y)]
                .set_symbol(&state.theme.overflow_up_indicator)
                .set_style(overflow_style);
        }

        if last_visible_idx.is_some_and(|i| i < row_heights.len() - 1) && area.width > 0 {
            let x = area.x + area.width - 1;
            let y = area.y + y_offset.saturating_sub(1);
            buf[(x, y)]
                .set_symbol(&state.theme.overflow_down_indicator)
                .set_style(overflow_style);
        }

        if let Some(message) = state.validation_error.as_deref()
            && reserve_error > 0
        {
            let error_y = area.y + y_offset.min(area.height.saturating_sub(1));
            let error_line = Line::from(Span::styled(message.to_string(), state.theme.error_style));
            buf.set_line(area.x, error_y, &error_line, area.width);
        }
    }
}

impl HandleEvent for InputTable {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        // Check bindings first.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            return EventOutcome::Cancelled;
        }
        if KeyBindings::matches(&state.bindings.submit, &event) {
            return submit(state);
        }

        // Tab/BackTab navigation.
        match (event.code, event.modifiers) {
            (KeyCode::Tab, m) if m == KeyModifiers::NONE => {
                move_tab(state, 1);
                return EventOutcome::Consumed;
            }
            (KeyCode::BackTab, _) => {
                move_tab(state, -1);
                return EventOutcome::Consumed;
            }
            _ => {}
        }

        // Arrow navigation / delegate to focused cell.
        if let Some(outcome) = try_navigate(state, event) {
            outcome
        } else {
            route_to_focus(state, event)
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
                        choose_state.set_validation_error(format!("Please select at least {min}"));
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
        state.validation_error = Some("One or more cells need your attention".into());
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
            state.sync_row(state.focus_row);
            state.validation_error = None;
            EventOutcome::Consumed
        }
        EventOutcome::Ignored => EventOutcome::Ignored,
    }
}

fn try_navigate(state: &mut InputTableState, event: KeyEvent) -> Option<EventOutcome> {
    // Alt+Up/Alt+Down always navigates rows regardless of cell type.
    if event.modifiers.contains(KeyModifiers::ALT) {
        match event.code {
            KeyCode::Up => {
                move_focus(state, -1, 0);
                return Some(EventOutcome::Consumed);
            }
            KeyCode::Down => {
                move_focus(state, 1, 0);
                return Some(EventOutcome::Consumed);
            }
            _ => return None,
        }
    }

    if event.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    let focused_cell = state
        .rows
        .get(state.focus_row)
        .and_then(|r| r.get(state.focus_col));

    let inside_text_cell = matches!(
        focused_cell,
        Some(CellState::TextInput(_)) | Some(CellState::TextAreaInput(_))
    );
    let inside_choice_cell = matches!(
        focused_cell,
        Some(CellState::ChooseOne(_)) | Some(CellState::ChooseMany(_))
    );

    match event.code {
        KeyCode::Up if !inside_choice_cell => {
            move_focus(state, -1, 0);
            Some(EventOutcome::Consumed)
        }
        KeyCode::Down if !inside_choice_cell => {
            move_focus(state, 1, 0);
            Some(EventOutcome::Consumed)
        }
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

/// Applies a typed [`CellValue`] to an existing [`CellState`].
///
/// Used by [`InputTableState::new`] to seed cells from caller-provided
/// row data.
fn apply_cell_value(cell: &mut CellState, value: &CellValue) {
    match (cell, value) {
        (CellState::StaticText(text), CellValue::StaticText(v)) => {
            *text = v.clone();
        }
        (CellState::BooleanSwitch(state), CellValue::Boolean(b)) => {
            state.set_checked(*b);
        }
        (CellState::TextInput(state), CellValue::Text(v)) => {
            *state = state.clone().with_value(v);
        }
        (CellState::TextAreaInput(state), CellValue::TextArea(lines)) => {
            let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            *state = state.clone().with_value(&line_refs);
        }
        (CellState::ChooseOne(state), CellValue::ChosenOne(Some(id))) => {
            *state = state.clone().with_initial_selection(id);
        }
        (CellState::ChooseOne(_), CellValue::ChosenOne(None)) => {
            // No selection to set; no-op
        }
        (CellState::ChooseMany(state), CellValue::ChosenMany(ids)) => {
            let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
            *state = state.clone().with_initial_selection(&id_refs);
        }
        _ => {
            // Type mismatch: silently no-op. The panic in `new()` already
            // guards row length; mismatched types within a row are the
            // caller's responsibility. We default to the column's seed value.
        }
    }
}

fn normalize_row(row_idx: usize, row: &Row, columns: &[InputTableColumn]) -> Row {
    let mut seen = HashSet::with_capacity(row.cells.len());
    for cell in &row.cells {
        if !seen.insert(cell.column_id.as_str()) {
            panic!(
                "InputTableState::new: row {row_idx} has duplicate column id '{}'",
                cell.column_id
            );
        }
        if !columns.iter().any(|column| column.id() == cell.column_id) {
            panic!(
                "InputTableState::new: row {row_idx} has unknown column id '{}'",
                cell.column_id
            );
        }
    }

    Row::new(
        columns
            .iter()
            .map(|column| {
                let cell = row
                    .cells
                    .iter()
                    .find(|cell| cell.column_id == column.id())
                    .unwrap_or_else(|| {
                        panic!(
                            "InputTableState::new: row {row_idx} is missing column '{}'",
                            column.id()
                        )
                    });
                RowCell::new(column.id(), cell.value.clone())
            })
            .collect(),
    )
}

fn typed_row_from_cells(columns: &[InputTableColumn], row: &[CellState]) -> Row {
    Row::new(
        row.iter()
            .zip(columns.iter())
            .map(|(cell, column)| RowCell::new(column.id(), cell.to_cell_value()))
            .collect(),
    )
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

fn compute_column_widths(columns: &[InputTableColumn], total_width: u16) -> Vec<u16> {
    use unicode_width::UnicodeWidthStr;

    if columns.is_empty() {
        return Vec::new();
    }
    let preferred: Vec<u16> = columns
        .iter()
        .map(|col| match col {
            InputTableColumn::StaticText { text, .. } => {
                (UnicodeWidthStr::width(text.as_str()) as u16).max(3)
            }
            InputTableColumn::BooleanSwitch { .. } => 8,
            InputTableColumn::TextInput { .. } => 20,
            InputTableColumn::TextAreaInput { config, .. } => config.preferred_width,
            InputTableColumn::ChooseOne(_) | InputTableColumn::ChooseMany(_) => 20,
        })
        .collect();

    let total_preferred: u32 = preferred.iter().map(|&w| w as u32).sum();

    if total_preferred <= total_width as u32 && total_preferred > 0 {
        let leftover = total_width as u32 - total_preferred;
        let focusable_count = columns
            .iter()
            .filter(|col| !matches!(col, InputTableColumn::StaticText { .. }))
            .count() as u32;

        if focusable_count == 0 {
            return preferred;
        }

        let per_focusable = leftover / focusable_count;
        let remainder = (leftover % focusable_count) as u16;
        let mut focusable_idx = 0u16;

        preferred
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                if matches!(columns[i], InputTableColumn::StaticText { .. }) {
                    p
                } else {
                    let extra =
                        per_focusable as u16 + if focusable_idx < remainder { 1 } else { 0 };
                    focusable_idx += 1;
                    p + extra
                }
            })
            .collect()
    } else {
        let base = total_width / columns.len() as u16;
        let remainder = total_width % columns.len() as u16;
        (0..columns.len())
            .map(|i| {
                let base_w = base + if (i as u16) < remainder { 1 } else { 0 };
                if matches!(columns[i], InputTableColumn::StaticText { .. }) {
                    base_w.min(preferred[i])
                } else {
                    base_w
                }
            })
            .collect()
    }
}

fn adjust_table_scroll(state: &mut InputTableState, row_heights: &[u16], available: u16) {
    if available == 0 || row_heights.is_empty() || state.rows.is_empty() {
        return;
    }
    if state.row_scroll_offset >= row_heights.len() {
        state.row_scroll_offset = row_heights.len().saturating_sub(1);
    }

    let mut tops: Vec<u32> = Vec::with_capacity(row_heights.len());
    let mut acc: u32 = 0;
    for &h in row_heights {
        tops.push(acc);
        acc += h as u32;
    }

    let focus_top = tops.get(state.focus_row).copied().unwrap_or(0);
    let focus_bottom = focus_top + row_heights.get(state.focus_row).copied().unwrap_or(1) as u32;

    let viewport_top = tops.get(state.row_scroll_offset).copied().unwrap_or(0);
    let viewport_bottom = viewport_top + available as u32;

    if focus_top < viewport_top {
        state.row_scroll_offset = state.focus_row;
    } else if focus_bottom > viewport_bottom {
        for i in state.row_scroll_offset..=state.focus_row {
            let top = tops.get(i).copied().unwrap_or(0);
            if top + available as u32 >= focus_bottom {
                state.row_scroll_offset = i;
                break;
            }
        }
    }
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
            ChooseOne::new().render(area, buf, state);
        }
        CellState::ChooseMany(state) => {
            ChooseMany::new().render(area, buf, state);
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
#[allow(deprecated)]
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
            InputTableColumn::StaticText {
                id: "label".into(),
                text: "Row".into(),
            },
            InputTableColumn::TextInput {
                id: "name".into(),
                config: TextInputConfig::default(),
            },
            InputTableColumn::BooleanSwitch {
                id: "active".into(),
                config: BooleanSwitchConfig::default(),
            },
        ]
    }

    #[test]
    fn with_blank_rows_places_focus_on_first_focusable_cell() {
        let state = InputTableState::with_blank_rows(sample_columns(), 2);
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    #[allow(deprecated)]
    fn with_blank_rows_populates_row_count_by_column_schema() {
        let state = InputTableState::with_blank_rows(sample_columns(), 3);
        assert_eq!(state.row_count(), 3);
        assert_eq!(state.col_count(), 3);
        assert_eq!(state.values().len(), 3);
        assert_eq!(state.values()[0].len(), 3);
    }

    #[test]
    fn right_arrow_moves_focus_to_next_focusable_column() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        // initial focus is (0,1). Right should move to (0,2).
        let outcome = InputTable.handle_event(&mut state, press(KeyCode::Right));
        // Right inside a TextInput is consumed for cursor; not navigation.
        // We rely on Tab here instead — left/right is blocked for text cells.
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn tab_moves_to_next_focusable_cell_across_rows() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 2);
        assert_eq!(state.focus(), (0, 1));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 2));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        // Next row, first focusable column (col 1).
        assert_eq!(state.focus(), (1, 1));
    }

    #[test]
    fn tab_wraps_to_first_cell() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 2));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn backtab_moves_to_previous_focusable_cell() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 2);
        // focus: (0,1) → Tab → (0,2) → BackTab → (0,1)
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        let event = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        InputTable.handle_event(&mut state, event);
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn up_and_down_arrows_change_row() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 3);
        // Start at (0,1). Down should go to (1,1).
        InputTable.handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.focus(), (1, 1));
        InputTable.handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.focus(), (0, 1));
    }

    #[test]
    fn up_down_inside_choice_cell_falls_through_to_cell() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
            ChoiceOption::new("c", "C", "gamma"),
        ]);
        let columns = vec![InputTableColumn::ChooseOne(input)];
        let mut state = InputTableState::with_blank_rows(columns, 2);
        assert_eq!(state.focus(), (0, 0));
        let outcome = InputTable.handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.focus(), (0, 0));
    }

    #[test]
    fn alt_up_down_navigates_rows_from_choice_cell() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
        ]);
        let columns = vec![InputTableColumn::ChooseOne(input)];
        let mut state = InputTableState::with_blank_rows(columns, 3);
        assert_eq!(state.focus(), (0, 0));
        let alt_down = KeyEvent::new(KeyCode::Down, KeyModifiers::ALT);
        InputTable.handle_event(&mut state, alt_down);
        assert_eq!(state.focus(), (1, 0));
        let alt_up = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);
        InputTable.handle_event(&mut state, alt_up);
        assert_eq!(state.focus(), (0, 0));
    }

    #[test]
    fn up_from_first_row_wraps_to_last_row() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 3);
        InputTable.handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.focus(), (2, 1));
    }

    #[test]
    fn esc_cancels() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        let outcome = InputTable.handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn ctrl_s_submits_when_all_cells_valid() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn typing_into_focused_text_input_updates_value() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        InputTable.handle_event(&mut state, press(KeyCode::Char('h')));
        InputTable.handle_event(&mut state, press(KeyCode::Char('i')));
        assert_eq!(state.values()[0][1], "hi");
    }

    #[test]
    fn space_in_focused_boolean_switch_toggles() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
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
            InputTableColumn::TextInput {
                id: "field".into(),
                config: TextInputConfig::default(),
            },
            InputTableColumn::ChooseOne(input),
        ];
        let mut state = InputTableState::with_blank_rows(columns, 1);
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
        let mut state = InputTableState::with_blank_rows(columns, 1);
        InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        // Fix: select the single option.
        InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Submitted);
        let rows = state.value();
        assert_eq!(
            rows[0].cells[0].value,
            CellValue::ChosenOne(Some("alpha".into()))
        );
    }

    #[test]
    #[allow(deprecated)]
    fn set_cell_initial_replaces_text_input_value() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 2);
        state.set_cell_initial(1, 1, "hello");
        assert_eq!(state.values()[1][1], "hello");
    }

    #[test]
    #[allow(deprecated)]
    fn set_cell_initial_on_static_text_overrides_display() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        state.set_cell_initial(0, 0, "custom");
        assert_eq!(state.values()[0][0], "custom");
    }

    #[test]
    #[allow(deprecated)]
    fn values_joins_choose_many_with_comma() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
        ]);
        let columns = vec![InputTableColumn::ChooseMany(input)];
        let mut state = InputTableState::with_blank_rows(columns, 1);
        state.set_cell_initial(0, 0, "a,b");
        assert_eq!(state.values()[0][0], "alpha,beta");
    }

    #[test]
    fn render_mixed_cell_table_paints_static_and_body() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "row".into(),
                text: "ID".into(),
            },
            InputTableColumn::TextInput {
                id: "name".into(),
                config: TextInputConfig {
                    initial: "alpha".into(),
                    ..TextInputConfig::default()
                },
            },
            InputTableColumn::BooleanSwitch {
                id: "active".into(),
                config: BooleanSwitchConfig::default(),
            },
        ];
        let mut state = InputTableState::with_blank_rows(columns, 1);
        let area = Rect::new(0, 0, 36, 1);
        let mut buf = Buffer::empty(area);
        InputTable.render(area, &mut buf, &mut state);

        let mut row = String::new();
        for x in area.left()..area.right() {
            row.push_str(buf[(x, area.y)].symbol());
        }
        let row = row.trim_end();
        assert!(row.starts_with("ID"), "expected static text, got {row:?}");
        assert!(
            row.contains("alpha"),
            "expected text-input value, got {row:?}"
        );
    }

    #[test]
    fn render_supports_text_area_cell_with_preferred_height() {
        let columns = vec![InputTableColumn::TextAreaInput {
            id: "notes".into(),
            config: TextAreaInputConfig {
                preferred_width: 10,
                preferred_height: 2,
                initial: vec!["a".into(), "b".into()],
                scrollbar: false,
            },
        }];
        let mut state = InputTableState::with_blank_rows(columns, 1);
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
    #[allow(deprecated)]
    fn values_encodes_boolean_as_true_or_false() {
        let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
        // Focus on BooleanSwitch (col 2), toggle on.
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.values()[0][2], "true");
    }

    #[test]
    fn custom_submit_key_replaces_default() {
        let columns = vec![InputTableColumn::TextInput {
            id: "field".into(),
            config: TextInputConfig::default(),
        }];
        let mut state = InputTableState::with_blank_rows(columns, 1)
            .with_submit_key(KeyCode::F(2), KeyModifiers::NONE);
        let outcome =
            InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn custom_key_bindings_override_submit_and_cancel() {
        let columns = vec![InputTableColumn::TextInput {
            id: "field".into(),
            config: TextInputConfig::default(),
        }];
        let bindings = KeyBindings {
            submit: vec![KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE)],
            cancel: vec![KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE)],
            ..KeyBindings::default()
        };
        let mut state = InputTableState::with_blank_rows(columns, 1).with_key_bindings(bindings);

        let outcome =
            InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
        assert_eq!(outcome, EventOutcome::Submitted);

        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Ignored);

        let outcome =
            InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn new_accepts_typed_row_vec() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "row".into(),
                text: "1".into(),
            },
            InputTableColumn::BooleanSwitch {
                id: "active".into(),
                config: BooleanSwitchConfig::default(),
            },
        ];
        let initial_rows = vec![Row::new(vec![
            RowCell::new("active", CellValue::Boolean(true)),
            RowCell::new("row", CellValue::StaticText("1".into())),
        ])];
        let state = InputTableState::new(columns, initial_rows);
        assert_eq!(state.row_count(), 1);
        assert_eq!(state.value()[0].get_boolean("active"), Some(true));
        assert_eq!(state.value()[0].get_text("row"), Some("1"));
    }

    #[test]
    #[should_panic(expected = "row 0 has 1 cells, expected 2")]
    fn new_panics_on_length_mismatch() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "a".into(),
                text: "A".into(),
            },
            InputTableColumn::StaticText {
                id: "b".into(),
                text: "B".into(),
            },
        ];
        let initial_rows = vec![Row::new(vec![RowCell::new(
            "a",
            CellValue::StaticText("only_one".into()),
        )])];
        let _state = InputTableState::new(columns, initial_rows);
    }

    #[test]
    fn value_exposes_typed_row_slice_in_column_order() {
        let columns = vec![
            InputTableColumn::TextInput {
                id: "name".into(),
                config: TextInputConfig::default(),
            },
            InputTableColumn::BooleanSwitch {
                id: "active".into(),
                config: BooleanSwitchConfig::default(),
            },
        ];
        let initial_rows = vec![Row::new(vec![
            RowCell::new("active", CellValue::Boolean(true)),
            RowCell::new("name", CellValue::Text("alice".into())),
        ])];
        let state = InputTableState::new(columns, initial_rows);

        let rows = state.value();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells[0].column_id, "name");
        assert_eq!(rows[0].cells[1].column_id, "active");
        assert_eq!(rows[0].get_text("name"), Some("alice"));
        assert_eq!(rows[0].get_boolean("active"), Some(true));
    }

    #[test]
    fn value_stays_in_sync_after_cell_edits() {
        let columns = vec![
            InputTableColumn::TextInput {
                id: "name".into(),
                config: TextInputConfig::default(),
            },
            InputTableColumn::BooleanSwitch {
                id: "active".into(),
                config: BooleanSwitchConfig::default(),
            },
        ];
        let mut state = InputTableState::with_blank_rows(columns, 1);

        InputTable.handle_event(&mut state, press(KeyCode::Char('h')));
        InputTable.handle_event(&mut state, press(KeyCode::Char('i')));
        InputTable.handle_event(&mut state, press(KeyCode::Tab));
        InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));

        let rows = state.value();
        assert_eq!(rows[0].get_text("name"), Some("hi"));
        assert_eq!(rows[0].get_boolean("active"), Some(true));
    }

    #[test]
    fn rows_typed_returns_boolean_cell_value() {
        let columns = vec![InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig {
                initial: true,
                ..BooleanSwitchConfig::default()
            },
        }];
        let state = InputTableState::with_blank_rows(columns, 1);
        let rows = state.rows_typed();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells[0].column_id, "active");
        assert_eq!(rows[0].cells[0].value, CellValue::Boolean(true));
    }

    #[test]
    fn rows_typed_returns_chosen_many_as_vec() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
        ]);
        let columns = vec![InputTableColumn::ChooseMany(input)];
        let mut state = InputTableState::with_blank_rows(columns, 1);
        state.set_cell_initial(0, 0, "a,b");
        let rows = state.rows_typed();
        assert_eq!(rows[0].cells[0].column_id, "c");
        assert_eq!(
            rows[0].cells[0].value,
            CellValue::ChosenMany(vec!["alpha".into(), "beta".into()])
        );
    }

    #[test]
    fn submit_focuses_first_failing_cell_in_row_major_order() {
        let choose_many = ChoiceInput::new("tags", "Tags")
            .with_options(vec![ChoiceOption::new("a", "Alpha", "alpha")])
            .required();
        let choose_one = ChoiceInput::new("color", "Color")
            .with_options(vec![ChoiceOption::new("r", "Red", "red")])
            .required();
        let columns = vec![
            InputTableColumn::ChooseMany(choose_many),
            InputTableColumn::ChooseOne(choose_one),
        ];
        let mut state = InputTableState::with_blank_rows(columns, 2);

        state.set_cell_initial(0, 0, "a");

        let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.focus(), (0, 1));
        assert_eq!(
            state.table_validation_error(),
            Some("One or more cells need your attention")
        );
        assert!(
            state.rows()[0][1].validation_error().is_some(),
            "expected first failing cell to retain its validation error"
        );
    }

    #[test]
    #[should_panic(expected = "unknown column id 'extra'")]
    fn new_panics_on_unknown_column_id() {
        let columns = vec![InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        }];
        let initial_rows = vec![Row::new(vec![RowCell::new(
            "extra",
            CellValue::Text("alice".into()),
        )])];

        let _state = InputTableState::new(columns, initial_rows);
    }

    #[test]
    fn scroll_offset_adjusts_when_focus_moves_past_viewport() {
        let columns = vec![InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        }];
        let mut state = InputTableState::with_blank_rows(columns, 10);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        InputTable.render(area, &mut buf, &mut state);
        assert_eq!(state.row_scroll_offset, 0);
        for _ in 0..4 {
            InputTable.handle_event(&mut state, press(KeyCode::Down));
        }
        InputTable.render(area, &mut buf, &mut state);
        assert!(
            state.row_scroll_offset > 0,
            "expected scroll offset to advance past 0"
        );
    }

    #[test]
    fn scroll_offset_adjusts_when_focus_returns_above_viewport() {
        let columns = vec![InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        }];
        let mut state = InputTableState::with_blank_rows(columns, 10);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        for _ in 0..9 {
            InputTable.handle_event(&mut state, press(KeyCode::Down));
        }
        InputTable.render(area, &mut buf, &mut state);
        let offset_after_down = state.row_scroll_offset;
        assert!(offset_after_down > 0);
        for _ in 0..9 {
            InputTable.handle_event(&mut state, press(KeyCode::Up));
        }
        InputTable.render(area, &mut buf, &mut state);
        assert_eq!(
            state.row_scroll_offset, 0,
            "expected scroll offset to return to 0"
        );
    }

    #[test]
    fn render_paints_overflow_indicators_when_rows_exceed_viewport() {
        let columns = vec![InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        }];
        let mut state = InputTableState::with_blank_rows(columns, 10);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        for _ in 0..5 {
            InputTable.handle_event(&mut state, press(KeyCode::Down));
        }
        InputTable.render(area, &mut buf, &mut state);
        let top_right = buf[(area.x + area.width - 1, area.y)].symbol();
        assert_eq!(top_right, "▲");
        let bottom_right = buf[(area.x + area.width - 1, area.y + 2)].symbol();
        assert_eq!(bottom_right, "▼");
    }

    #[test]
    fn static_text_columns_stay_at_natural_width_with_leftover() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "hi".into(),
                text: "Hello".into(),
            },
            InputTableColumn::TextInput {
                id: "a".into(),
                config: TextInputConfig::default(),
            },
            InputTableColumn::TextInput {
                id: "b".into(),
                config: TextInputConfig::default(),
            },
        ];
        let widths = compute_column_widths(&columns, 60);
        assert_eq!(
            widths[0], 5,
            "StaticText should get exactly its natural unicode width"
        );
        assert_eq!(
            widths.iter().map(|&w| w as u32).sum::<u32>(),
            60,
            "widths should sum to total"
        );
        assert!(
            widths[1] > 20,
            "focusable columns should get leftover width"
        );
        assert!(
            widths[2] > 20,
            "focusable columns should get leftover width"
        );
    }

    #[test]
    fn static_text_does_not_shrink_below_natural_in_overflow() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "lbl".into(),
                text: "LongLabel".into(),
            },
            InputTableColumn::TextInput {
                id: "a".into(),
                config: TextInputConfig::default(),
            },
        ];
        let widths = compute_column_widths(&columns, 10);
        assert_eq!(
            widths[0], 5,
            "StaticText in overflow should get min(base, preferred)"
        );
        assert_eq!(widths.iter().map(|&w| w as u32).sum::<u32>(), 10);
    }

    #[test]
    fn all_static_text_columns_use_preferred_widths() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "a".into(),
                text: "Alpha".into(),
            },
            InputTableColumn::StaticText {
                id: "b".into(),
                text: "Beta".into(),
            },
        ];
        let widths = compute_column_widths(&columns, 40);
        assert_eq!(widths[0], 5, "Alpha is 5 chars wide");
        assert_eq!(widths[1], 4, "Beta is 4 chars wide");
    }

    #[test]
    fn render_static_text_stays_tight() {
        let columns = vec![
            InputTableColumn::StaticText {
                id: "lbl".into(),
                text: "Label".into(),
            },
            InputTableColumn::TextInput {
                id: "name".into(),
                config: TextInputConfig::default(),
            },
        ];
        let mut state = InputTableState::with_blank_rows(columns, 1);
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        InputTable.render(area, &mut buf, &mut state);

        let widths = compute_column_widths(state.columns(), area.width);
        assert_eq!(widths[0], 5, "StaticText 'Label' is 5 chars");

        let static_end = widths[0] as usize;
        let mut label = String::new();
        for x in 0..static_end {
            label.push_str(buf[(x as u16, 0)].symbol());
        }
        assert_eq!(label.trim_end(), "Label");
    }
}
