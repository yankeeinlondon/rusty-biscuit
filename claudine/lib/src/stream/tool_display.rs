//! `ToolCallDisplay` — protocol-level model for rendering a tool invocation
//! (request or response) in a single, provider-agnostic way.

use serde::{Deserialize, Serialize};

/// Direction of a tool event from the assistant's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDirection {
    Outgoing,
    Incoming,
}

/// Outcome of an incoming tool event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    Pending,
}

/// Display-ready tool event. Per spec: status wins over summary on incoming
/// events; the formatter NEVER writes a glyph literally — it populates a
/// biscuit-terminal `Status::ToolUse` instead.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallDisplay {
    pub direction: ToolDirection,
    pub display_name: String,
    pub summary: Option<String>,
    pub status: Option<ToolStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ToolDirection::Outgoing).unwrap(),
            "\"outgoing\""
        );
        assert_eq!(
            serde_json::to_string(&ToolStatus::Success).unwrap(),
            "\"success\""
        );
    }

    #[test]
    fn struct_round_trips_via_clone_and_eq() {
        let display = ToolCallDisplay {
            direction: ToolDirection::Incoming,
            display_name: "Firecrawl Search".into(),
            summary: Some("NFL draft 2026 date".into()),
            status: Some(ToolStatus::Success),
        };
        let cloned = display.clone();
        assert_eq!(display, cloned);
    }
}
