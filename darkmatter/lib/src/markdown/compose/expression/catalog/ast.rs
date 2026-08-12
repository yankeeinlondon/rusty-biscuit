//! Owned AST for authored expression-function catalogs.

use super::DataType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpressionFunctionCatalog {
    pub functions: Vec<CatalogFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogFunction {
    pub name: String,
    pub category: String,
    pub order: usize,
    pub description: String,
    pub overloads: Vec<CatalogOverload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogOverload {
    pub parameters: Vec<CatalogParam>,
    pub returns: CatalogReturn,
    pub example: CatalogExample,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogParam {
    pub name: String,
    pub ty: DataType,
    pub array: bool,
    pub optional: bool,
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogReturn {
    pub value: CatalogReturnValue,
    pub array: bool,
    pub fallible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CatalogReturnValue {
    Data(DataType),
    Enum(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogExample {
    pub expression: String,
    pub result: String,
    pub verification: CatalogVerification,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CatalogVerification {
    Executable,
    DisplayOnly(String),
}
