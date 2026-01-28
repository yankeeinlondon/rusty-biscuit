pub enum ColumnType {
    String,
    Integer,
    Float,
    Currency(CurrencyOptions),
    Metric(MetricOptions),
    OptString,
    OptInteger,
    OptFloat,

    Unknown
}

pub enum ColumnAggregate {
    None,
    Sum,
    Avg,
    Median,
    Min,
    Max,
    Range,
}


pub struct TableCell {

}

pub struct TableRow {
  title: Option<String>,
}

/// A **TableColumn** is used to define:
///
/// - the columns **title** (optionally)
/// - the _data type_ we expect to be in the column
/// - the _alignment_ for this column
///
pub struct TableColumn {
    title: Option<String>,
    kind: ColumnType,
    aggregate: ColumnAggregate,
    alignment: ColumnAlignment,
}

