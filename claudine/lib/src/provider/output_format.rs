//! Typed output-format and entrypoint capability data.
//!
//! Phase 5 of the centralized providers refactor introduces typed metadata
//! describing each provider's non-interactive output formats and supported
//! entrypoints. The data lives on each [`ProviderInfo`](super::ProviderInfo)
//! as `&'static [OutputFormatSupport]` and `&'static [EntrypointSpec]`.
//!
//! [`OutputFormat`] mirrors the values understood by the universal
//! `--output` flag and intentionally re-exports the same lowercase
//! representation used by `claudine::composition::OutputFormat` so that
//! downstream code can translate between the two without bespoke logic.

use serde::Serialize;

use crate::provider_id::OutputFormatSelector;

/// Universal output-format identifier shared by composition and wrapper
/// catalog data.
///
/// The discriminants line up with `claudine::composition::OutputFormat`
/// (text/json/stream) and the CLI's wrapper-side `OutputFormat`. The lib
/// crate retains its own copy here so the provider catalog stays free of
/// composition-specific types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Plain text output (the default for most providers).
    Text,
    /// One JSON document for the entire response.
    Json,
    /// Newline-delimited JSON / streaming JSON output.
    Stream,
}

impl OutputFormat {
    /// Returns the canonical lowercase identifier (`"text"`, `"json"`,
    /// `"stream"`) used by the composition layer and the CLI's universal
    /// `--output` flag.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Stream => "stream",
        }
    }
}

/// Metadata describing how a provider exposes a particular [`OutputFormat`]
/// in non-interactive mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OutputFormatSupport {
    /// The universal output format identifier this entry maps from.
    pub format: OutputFormat,
    /// The provider's native name for this format (e.g. `"stream-json"`,
    /// `"jsonl"`, `"schema-json"`).
    pub native_name: &'static str,
    /// The CLI flag that selects this format on the provider's binary,
    /// when one exists. `None` if the format is the default and needs no
    /// flag.
    pub cli_flag: Option<&'static str>,
    /// Whether this format also accepts the prompt over stdin.
    pub stdin_supported: bool,
    /// How the provider CLI selects this output format.
    pub selector: OutputFormatSelector,
    /// Extra argv the provider needs alongside the stream selector for
    /// reliable structured output (e.g. Claude's `--print --verbose`).
    /// Empty for most records. Serialized in the describe/catalog
    /// surfaces like every other catalog field.
    pub companion_flags: &'static [&'static str],
}

/// Whether an entrypoint is available in interactive mode, non-interactive
/// mode, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointMode {
    /// The entrypoint is non-interactive only (e.g. `codex exec`).
    NonInteractive,
    /// The entrypoint is interactive only (a TUI subcommand).
    Interactive,
    /// The entrypoint works in both modes.
    Both,
}

/// Catalog entry for a non-interactive entrypoint exposed by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EntrypointSpec {
    /// Optional subcommand (e.g. `Some("exec")` for Codex).
    /// `None` means the provider's main binary is the entrypoint.
    pub subcommand: Option<&'static str>,
    /// Required flags that must accompany the entrypoint, e.g. `&["--print"]`
    /// for Claude / Kimi non-interactive mode.
    pub required_flags: &'static [&'static str],
    /// Mode in which this entrypoint is valid.
    pub mode: EntrypointMode,
}
