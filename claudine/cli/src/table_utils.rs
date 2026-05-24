use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::utils::layout::{Length, Margin};

pub(crate) fn base_table(columns: Vec<TableColumn>) -> Table {
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().margin = Margin::x(Length::ch(1));
    table
}
