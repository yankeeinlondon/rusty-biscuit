use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// Parameter types for WebSocket connection parameters.
///
/// These types map to common JSON/query string value types.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::ParamType;
/// use std::str::FromStr;
///
/// // Display as lowercase
/// assert_eq!(ParamType::String.to_string(), "string");
/// assert_eq!(ParamType::Integer.to_string(), "integer");
///
/// // Parse from lowercase
/// assert_eq!(ParamType::from_str("boolean").unwrap(), ParamType::Boolean);
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumIter, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ParamType {
    /// String parameter
    String,
    /// Integer parameter (signed 64-bit)
    Integer,
    /// Boolean parameter
    Boolean,
    /// Floating-point parameter (64-bit)
    Float,
}

/// A connection parameter for WebSocket endpoints.
///
/// Connection parameters are typically passed as query string parameters
/// when establishing the WebSocket connection.
///
/// ## Examples
///
/// ```
/// use schematic_define::websocket::{ConnectionParam, ParamType};
///
/// let param = ConnectionParam {
///     name: "model_id".to_string(),
///     param_type: ParamType::String,
///     required: false,
///     description: Some("The model to use".to_string()),
/// };
///
/// assert_eq!(param.name, "model_id");
/// assert!(!param.required);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionParam {
    /// Parameter name (used in query string or path).
    pub name: String,
    /// Type of the parameter value.
    pub param_type: ParamType,
    /// Whether the parameter is required for connection.
    pub required: bool,
    /// Human-readable description of the parameter.
    pub description: Option<String>,
}
