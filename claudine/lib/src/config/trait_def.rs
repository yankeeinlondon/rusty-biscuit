use std::path::Path;

use sniff::programs::{AiCli, InstalledAiClients};

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

/// Map a claudine `Provider` to the corresponding sniff `AiCli` variant.
fn provider_to_ai_cli(provider: Provider) -> AiCli {
    match provider {
        Provider::Claude => AiCli::Claude,
        Provider::Codex => AiCli::Codex,
        Provider::Gemini => AiCli::GeminiCli,
        Provider::Goose => AiCli::Goose,
        Provider::KimiCode => AiCli::KimiCli,
        Provider::OpenCode => AiCli::Opencode,
        Provider::QwenCode => AiCli::QwenCli,
        Provider::RooCode => AiCli::Roo,
    }
}

/// Result of registering hooks with a provider.
#[derive(Debug)]
pub enum RegistrationResult {
    /// Hooks registered successfully.
    Registered {
        /// Number of events registered.
        event_count: usize,
    },
    /// Registration skipped.
    Skipped(SkipReason),
}

/// Reason a provider was skipped during registration.
#[derive(Debug)]
pub enum SkipReason {
    /// Provider not detected on this system.
    NotDetected,
    /// Provider uses wrapper-only approach (deferred).
    WrapperOnly {
        /// Guidance message for the user.
        guidance: String,
    },
    /// Already registered with same config.
    AlreadyRegistered,
    /// Provider doesn't support native hooks yet.
    NoHookSupport,
}

/// Trait for provider-specific hook configuration management.
pub trait AgentConfigurator {
    /// Which provider this configurator handles.
    fn provider(&self) -> Provider;

    /// Create a minimal valid configuration file for this provider.
    ///
    /// This is called when the provider's CLI is installed but no config file exists.
    /// The created config should be the minimum needed for the provider to work
    /// and for hooks to be registered.
    ///
    /// Returns `Ok(())` on success, or an error if config creation is not supported
    /// or fails.
    fn create_minimal_config(&self, _config_dir: Option<&Path>) -> Result<()> {
        Err(crate::error::ClaudineError::ConfigCreationNotSupported {
            provider: self.provider().to_string(),
        })
    }

    /// Register Claudine hooks with this provider's config.
    fn register(
        &self,
        config: &HookerConfig,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult>;

    /// Remove Claudine hooks from this provider's config.
    fn deregister(&self, config_dir: Option<&Path>) -> Result<()>;

    /// Check if Claudine hooks are already registered.
    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool>;

    /// Return the list of Claudine-registered event names (snake_case).
    ///
    /// Returns an empty vector if no hooks are registered or the config doesn't exist.
    fn registered_events(&self, config_dir: Option<&Path>) -> Result<Vec<String>>;

    /// Whether this provider supports config-file-based hook registration.
    ///
    /// Returns `true` for providers where claudine can modify a config file
    /// to register hooks (Claude, Gemini, OpenCode, Codex).
    ///
    /// Returns `false` for providers that require wrapper/proxy approaches
    /// (Goose, KimiCode, QwenCode).
    fn supports_config_registration(&self) -> bool {
        true // Default to true; override for wrapper-only providers
    }

    /// Returns the events that this configurator can register via config file.
    ///
    /// For most providers, this filters by the provider's native event support.
    /// For providers like Codex that can only register specific events (e.g.,
    /// only `turn_complete` via the `notify` option), this returns that subset.
    ///
    /// Returns `None` to indicate "use Provider::supports_event() filtering".
    fn registerable_events(&self) -> Option<Vec<crate::events::AgenticEvent>> {
        None // Default: use Provider::supports_event() filtering
    }

    /// Check if this provider's CLI binary is installed on the system.
    ///
    /// Uses sniff to detect whether the CLI is available on PATH.
    fn is_cli_installed(&self) -> bool {
        let clients = InstalledAiClients::new();
        clients.is_installed(provider_to_ai_cli(self.provider()))
    }
}
