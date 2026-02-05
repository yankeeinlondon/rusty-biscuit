use std::path::Path;

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

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
}

/// Trait for provider-specific hook configuration management.
pub trait AgentConfigurator {
    /// Which provider this configurator handles.
    fn provider(&self) -> Provider;

    /// Register Claudine hooks with this provider's config.
    fn register(&self, config: &HookerConfig, config_dir: Option<&Path>) -> Result<RegistrationResult>;

    /// Remove Claudine hooks from this provider's config.
    fn deregister(&self, config_dir: Option<&Path>) -> Result<()>;

    /// Check if Claudine hooks are already registered.
    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool>;

    /// Return the list of Claudine-registered event names (snake_case).
    ///
    /// Returns an empty vector if no hooks are registered or the config doesn't exist.
    fn registered_events(&self, config_dir: Option<&Path>) -> Result<Vec<String>>;
}
