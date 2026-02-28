use serde::{Deserialize, Serialize};

use super::facets::{Attribute, SymbolRef, VisibilityLevel};

/// Symbol kind taxonomy for schema v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKindV2 {
    Function,
    Method,
    Constructor,
    Type,
    Class,
    Interface,
    Enum,
    Trait,
    Module,
    Namespace,
    Field,
    Property,
    Variable,
    Constant,
    Parameter,
    TypeAlias,
    EnumVariant,
    Macro,
    Unknown,
}

/// Receiver classification for function-like symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiverKind {
    Value,
    Ref,
    MutRef,
    This,
    Custom(String),
}

/// Method/function receiver metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverInfo {
    pub name: String,
    pub kind: ReceiverKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
}

/// Parameter information for function-like symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterData {
    pub name: String,
    pub position: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_ast: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    pub is_optional: bool,
    pub is_variadic: bool,
    pub is_keyword_only: bool,
    pub is_positional_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ownership_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attributes: Vec<Attribute>,
}

/// Return information for function-like symbols.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_ast: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yields_type: Option<String>,
    pub never_returns: bool,
}

/// Function/method payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<ReceiverInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<ParameterData>,
    pub return_info: ReturnData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calling_convention: Option<String>,
    pub is_variadic: bool,
    pub arity: u16,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub overload_set: Vec<SymbolRef>,
}

/// Field payload used in type and enum data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<VisibilityLevel>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub is_mutable: bool,
    pub is_optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
}

/// Type/class/interface payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeData {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub members: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<FieldData>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constructors: Vec<SymbolRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub associated_types: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub associated_constants: Vec<String>,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_sealed: bool,
    pub is_open: bool,
    pub is_partial: bool,
}

/// Enum variant payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminant: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tuple_fields: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub struct_fields: Vec<FieldData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
}

/// Enum payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnumData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backing_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub variants: Vec<EnumVariantData>,
}

/// Type alias payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeAliasData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliased_type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliased_type_ast: Option<serde_json::Value>,
    pub is_recursive: bool,
}

/// Module payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleData {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub members: Vec<SymbolRef>,
}

/// Variable payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VariableData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initializer_text: Option<String>,
    pub is_const: bool,
}

/// Parameter symbol payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSymbolData {
    pub name: String,
    pub position: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
}

/// Discriminated payload for symbol-kind-specific data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind_data_type", content = "data")]
pub enum SymbolKindData {
    Function(FunctionData),
    Type(TypeData),
    TypeAlias(TypeAliasData),
    Enum(EnumData),
    Field(FieldData),
    Module(ModuleData),
    Variable(VariableData),
    Parameter(ParameterSymbolData),
    Unknown,
}
