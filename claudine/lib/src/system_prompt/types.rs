use std::path::PathBuf;

pub const DEFAULT_NON_INTERACTIVE_SYSTEM_PROMPT: &str = r#"
**IMPORTANT:** this is a non-interactive prompt; do not request permission or ask the caller questions!

## Shell restrictions

Do not run commands that require an interactive terminal or follow-up stdin input.
Avoid REPLs, editors, pagers, prompts, and any command that waits for user input.
Prefer one-shot commands and explicit non-interactive flags.
If a task would require sending more input to a running command, choose a different approach.
"#;

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
    /// Found via non-interactive safety prompt discovery.
    NonInteractiveFile {
        path: PathBuf,
        scope: StandardPromptScope,
    },
    /// Built-in fallback used when no non-interactive file exists.
    BuiltInNonInteractive,
}

impl SystemPromptSource {
    pub(crate) fn into_projected(self) -> Self {
        use crate::system_prompt::context::projected_path;

        match self {
            Self::StandardDiscovered { path, scope } => Self::StandardDiscovered {
                path: projected_path(&path),
                scope,
            },
            Self::ExplicitFile { path, mode } => Self::ExplicitFile {
                path: projected_path(&path),
                mode,
            },
            Self::NonInteractiveFile { path, scope } => Self::NonInteractiveFile {
                path: projected_path(&path),
                scope,
            },
            Self::BuiltInNonInteractive => Self::BuiltInNonInteractive,
        }
    }
}

/// Prepared metadata for the non-interactive safety appendix.
#[derive(Debug, Clone)]
pub struct PreparedNonInteractiveAppendix {
    pub source: SystemPromptSource,
    pub raw_text: String,
    pub composed_markdown: String,
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
    /// Extra safety instructions appended for non-interactive sessions.
    pub non_interactive_appendix: Option<PreparedNonInteractiveAppendix>,
}

/// The outcome of the full resolve -> compose pipeline.
#[derive(Debug, Clone)]
pub enum ResolvedSystemPrompt {
    /// No system prompt file was found or specified.
    None,
    /// A file was found but its composed body is empty — explicit disable.
    Disabled { source: SystemPromptSource },
    /// A system prompt is ready for provider delivery.
    Ready(PreparedSystemPrompt),
}

impl ResolvedSystemPrompt {
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
