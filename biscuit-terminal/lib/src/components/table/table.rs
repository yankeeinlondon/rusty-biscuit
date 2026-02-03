use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::layout::Layout,
};

#[derive(Debug, Clone)]
pub enum Currency {
    USD,
    GBP,
    EUR,
}


/// Content for a table cell.
#[derive(Debug, Clone)]
pub enum TableCellContent {
    /// Text (which can include escape characters)
    Text(String),
    // Integer(i64),
    // Float(f64),
    // Currency(Currency, f64)
}

impl<T: Into<String>> From<T> for TableCellContent {
    fn from(value: T) -> Self {
        TableCellContent::Text(value.into())
    }
}

/// Column definition for a table.
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// Header text for the column
    pub header: String,
    /// Minimum width for the column (optional)
    pub min_width: Option<usize>,
    /// Maximum width for the column (optional)
    pub max_width: Option<usize>,
}

impl TableColumn {
    /// Create a new column with a header.
    pub fn new<T: Into<String>>(header: T) -> Self {
        TableColumn {
            header: header.into(),
            min_width: None,
            max_width: None,
        }
    }

    /// Set minimum width for the column.
    pub fn with_min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set maximum width for the column.
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }
}

/// A table component for rendering tabular data.
#[derive(Debug)]
pub struct Table {
    title: Option<String>,
    columns: Vec<TableColumn>,
    data: Vec<Vec<TableCellContent>>,
}

impl Default for Table {
    fn default() -> Self {
        Table {
            title: None,
            columns: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl Table {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the table title.
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the columns.
    pub fn with_columns(mut self, columns: Vec<TableColumn>) -> Self {
        self.columns = columns;
        self
    }

    /// Add a row of data.
    pub fn add_row(&mut self, row: Vec<TableCellContent>) {
        self.data.push(row);
    }

    /// Set all data rows.
    pub fn with_data(mut self, data: Vec<Vec<TableCellContent>>) -> Self {
        self.data = data;
        self
    }

    /// Calculate column widths based on content.
    fn calculate_column_widths(&self) -> Vec<usize> {
        let num_cols = self.columns.len().max(
            self.data.iter().map(|row| row.len()).max().unwrap_or(0),
        );

        let mut widths = vec![0; num_cols];

        // Consider header widths
        for (i, col) in self.columns.iter().enumerate() {
            widths[i] = col.header.len();
            if let Some(min) = col.min_width {
                widths[i] = widths[i].max(min);
            }
        }

        // Consider data widths
        for row in &self.data {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_width = match cell {
                        TableCellContent::Text(s) => s.len(),
                        // &TableCellContent::Currency(c, amt) => format_currency(c, amt).len(),
                        // &TableCellContent::Float(num) => format_float(num).len(),
                        // &TableCellContent::Integer(num) => format_int(num).len(),
                    };
                    widths[i] = widths[i].max(cell_width);
                }
            }
        }

        // Apply max width constraints
        for (i, col) in self.columns.iter().enumerate() {
            if let Some(max) = col.max_width {
                if i < widths.len() {
                    widths[i] = widths[i].min(max);
                }
            }
        }

        widths
    }

    /// Render the table content.
    fn render_content(&self, _term: Option<&Terminal>) -> String {
        let mut result = String::new();
        let widths = self.calculate_column_widths();

        // Render title if present
        if let Some(ref title) = self.title {
            result.push_str(title);
            result.push('\n');
        }

        // Render header row
        if !self.columns.is_empty() {
            let mut header_row = String::from("│ ");
            for (i, col) in self.columns.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(col.header.len());
                header_row.push_str(&format!("{:width$}", col.header, width = width));
                if i < self.columns.len() - 1 {
                    header_row.push_str(" │ ");
                }
            }
            header_row.push_str(" │");
            result.push_str(&header_row);
            result.push('\n');

            // Render separator
            let mut separator = String::from("├─");
            for (i, width) in widths.iter().enumerate() {
                separator.push_str(&"─".repeat(*width));
                if i < widths.len() - 1 {
                    separator.push_str("─┼─");
                }
            }
            separator.push_str("─┤");
            result.push_str(&separator);
            result.push('\n');
        }

        // Render data rows
        for row in &self.data {
            let mut row_str = String::from("│ ");
            for (i, cell) in row.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let content = match cell {
                    TableCellContent::Text(s) => s.clone(),
                };
                row_str.push_str(&format!("{:width$}", content, width = width));
                if i < row.len() - 1 {
                    row_str.push_str(" │ ");
                }
            }
            row_str.push_str(" │");
            result.push_str(&row_str);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }
}

impl Renderable for Table {
    fn render(&self, _layout: Option<&Layout>) -> String {
        self.render_content(None)
    }

    fn fallback_render(&self, term: &Terminal, _layout: Option<&Layout>) -> String {
        self.render_content(Some(term))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Age"),
            ])
            .with_data(vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ]);

        let result = table.render(None);
        assert!(result.contains("Name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_table_with_title() {
        let table = Table::new()
            .with_title("Users")
            .with_columns(vec![TableColumn::new("Name")]);

        let result = table.render(None);
        assert!(result.starts_with("Users\n"));
    }

    #[test]
    fn test_empty_table() {
        let table = Table::new();
        let result = table.render(None);
        assert_eq!(result, "");
    }
}
