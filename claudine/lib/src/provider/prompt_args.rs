//! Typed argv conventions for non-interactive prompt extraction.
//!
//! Phase 6 of the centralized providers refactor promotes
//! `PromptArgConventions` from the CLI's wrapper profile module into the
//! provider catalog so the per-provider data lives alongside other static
//! capability metadata. The CLI's `WrapperProfile::prompt_arg_conventions`
//! default reads from `provider_info(self.provider()).prompt_arg_conventions`,
//! eliminating per-provider overrides for ordinary cases.

/// Describes how a provider's native CLI represents a prompt on argv.
///
/// Used by the CLI's `extract_prompt_source_from_passthrough` helper to
/// find a prompt in raw passthrough arguments without embedding
/// per-provider logic in a central match.
#[derive(Debug, Clone, Copy)]
pub struct PromptArgConventions {
    /// Value-taking flags that carry the prompt string when present,
    /// e.g. `&["-p", "--prompt"]` for Gemini, `&["-t", "--text"]` for
    /// Goose. Empty for providers that accept only a positional prompt.
    pub prompt_flags: &'static [&'static str],
    /// An optional entrypoint subcommand that must be skipped when
    /// scanning for a positional prompt, e.g. `Some("exec")` for Codex
    /// or `Some("run")` for OpenCode / Goose. `None` for providers that
    /// have no subcommand entrypoint.
    pub entrypoint: Option<&'static str>,
    /// Additional value-taking flags whose values must not be mistaken
    /// for a positional prompt, e.g. `&["-m", "--model", "--output-format"]`.
    pub value_taking_flags: &'static [&'static str],
}

impl PromptArgConventions {
    /// Conventions for a provider that accepts only a positional prompt
    /// after an entrypoint subcommand (e.g. Codex `exec`, OpenCode `run`).
    pub const fn positional_after(entrypoint: &'static str) -> Self {
        Self {
            prompt_flags: &[],
            entrypoint: Some(entrypoint),
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }

    /// Default "no entrypoint, no prompt-carrying flag" conventions used
    /// by providers that accept only a bare positional prompt.
    pub const fn positional_only() -> Self {
        Self {
            prompt_flags: &[],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
}

/// Value-taking flags recognized by the prompt extractor across every
/// wrapped provider. This is intentionally the UNION of every provider's
/// value-taking flags, not a per-provider list — the extractor's job is
/// to avoid mistaking a flag's value for a positional prompt, and
/// over-skipping an unknown flag's value is harmless.
pub const COMMON_VALUE_TAKING_FLAGS: &[&str] = &[
    "-m",
    "--model",
    "-o",
    "--output",
    "--output-format",
    "--output-last-message",
    "--approval-mode",
    "--config",
    "-c",
    "--profile",
    "--system-prompt",
    "--sandbox-image",
    "--auth-type",
    "--format",
];
