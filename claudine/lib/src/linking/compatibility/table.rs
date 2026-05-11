/// Frontmatter properties that are non-portable across providers. Resources
/// containing any of these should not be symlinked into non-Claude provider
/// directories.
///
/// - `model`: shared property name but values are provider-specific (e.g.,
///   `sonnet` vs `gemini-2.5-pro`). Present in Claude, Gemini, and OpenCode
///   agent schemas but with incompatible value semantics.
/// - `tools`: shared property name but Claude uses a comma-separated string
///   while other providers (OpenCode, KimiCode) expect structured records.
///   Value format mismatch causes parse errors.
/// - `skills`: Claude-only. No other provider has a skill auto-loading
///   mechanism. Unknown key — likely ignored but not guaranteed.
///
/// Note: `allowed-tools` / `allowed_tools` is intentionally excluded. It is
/// Claude-only (used in skills/commands) and other providers simply ignore
/// unrecognized frontmatter keys.
pub(crate) const NON_PORTABLE_PROPERTIES: &[&str] = &["model", "tools", "skills"];
