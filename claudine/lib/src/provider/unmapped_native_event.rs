//! Provider-native hook events Claudine's event model cannot represent.
//!
//! Some providers fire hooks at phases the 16-event lifecycle model has no
//! counterpart for (e.g. Gemini `BeforeToolSelection`, OpenCode
//! `tool.definition`). Claudine cannot dispatch these; users who want them
//! must configure them directly in the provider. The catalog carries them
//! so CLI reports can say so instead of silently omitting the phase.

use serde::Serialize;

/// One provider-native hook event with no 16-event mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UnmappedNativeEvent {
    /// Provider-native event name (e.g. "BeforeToolSelection").
    pub native_event: &'static str,
    /// What the native event does — phrased so the missing lifecycle
    /// counterpart is evident.
    pub description: &'static str,
    /// How to use the event natively in the provider, since Claudine
    /// cannot dispatch it.
    pub remediation: &'static str,
}
