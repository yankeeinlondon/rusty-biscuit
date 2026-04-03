use std::path::PathBuf;

/// Whether a system prompt should append to or replace the provider's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPromptMode {
    Append,
    Replace,
}

/// Parsed CLI switch state before resolution.
#[derive(Debug, Clone, Default)]
pub struct SystemPromptArgs {
    pub append_file: Option<String>,
    pub replace_file: Option<String>,
}

/// The scope from which a standard `system-prompt.md` was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardPromptScope {
    Package,
    PackageArea,
    Repo,
    User,
    CurrentDirectory,
}

/// Where the effective system prompt came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemPromptSource {
    /// Found via automatic `system-prompt.md` discovery.
    StandardDiscovered {
        path: PathBuf,
        scope: StandardPromptScope,
    },
    /// Provided via an explicit CLI switch.
    ExplicitFile {
        path: PathBuf,
        mode: SystemPromptMode,
    },
}

/// A system prompt that has been resolved, composed, and is ready for
/// provider-specific delivery.
#[derive(Debug, Clone)]
pub struct PreparedSystemPrompt {
    pub mode: SystemPromptMode,
    pub source: SystemPromptSource,
    /// The raw file text before Darkmatter composition.
    pub raw_text: String,
    /// The composed Markdown body (after Darkmatter pipeline).
    pub composed_markdown: String,
}

/// The outcome of the full resolve -> compose pipeline.
#[derive(Debug, Clone)]
pub enum EffectiveSystemPrompt {
    /// No system prompt file was found or specified.
    None,
    /// A file was found but its composed body is empty — explicit disable.
    Disabled { source: SystemPromptSource },
    /// A system prompt is ready for provider delivery.
    Ready(PreparedSystemPrompt),
}

impl EffectiveSystemPrompt {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled { .. })
    }

    pub fn prepared(&self) -> Option<&PreparedSystemPrompt> {
        match self {
            Self::Ready(p) => Some(p),
            _ => Option::None,
        }
    }
}
