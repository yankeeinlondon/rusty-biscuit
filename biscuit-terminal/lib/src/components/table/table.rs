pub enum TableCellContent {
    /// Text (which can include escape characters)
    Text(String),

}

pub enum TableColumn {

}

pub struct Table {
    title: Option<String>,
    columns: Option<Vec<TableColumn>>,
    data: Option<Vec<Vec<TableCell>>>,
}

impl Renderable for Table {
    fn render() -> String {
        todo!()
    }
}
