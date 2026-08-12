//! Route resolution for messaging configuration.
//!
//! This module handles runtime resolution of messaging routes, applying scope
//! precedence rules (repo beats user) and resolving secrets from inline values
//! or environment variables.

use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use super::config::{MessagingRouteConfig, ScopedMessagingSettings};

/// Identifies which configuration scope a resolved route came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingScope {
    /// Route resolved from the user-level (global) configuration.
    User,
    /// Route resolved from the repository-level configuration.
    Repo,
}

/// A fully resolved messaging route ready for use by the send module.
#[derive(Debug, Clone)]
pub struct ResolvedMessagingRoute {
    /// The scope from which this route was resolved.
    pub scope: MessagingScope,
    /// The name of the active route within its scope.
    pub name: String,
    /// The provider-specific route configuration.
    pub config: MessagingRouteConfig,
}

/// Combined user and repo messaging settings, preserving scope boundaries.
///
/// The loader populates each field independently so `resolve_effective_route`
/// can apply the correct precedence rules.
#[derive(Debug, Clone, Default)]
pub struct RuntimeMessagingSettings {
    /// User-level (global) messaging settings, if any.
    pub user: Option<ScopedMessagingSettings>,
    /// Repository-level messaging settings, if any.
    pub repo: Option<ScopedMessagingSettings>,
}

/// A parsed Signal recipient, distinguishing phone numbers from group IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalRecipient {
    /// A phone number (starts with `+`).
    Phone(String),
    /// A group ID (any non-phone string).
    Group(String),
}

/// Resolves the effective messaging route from runtime settings.
///
/// Applies scope precedence: repo active route wins over user active route.
/// Returns `None` when neither scope has an active route configured.
///
/// ## Examples
///
/// ```rust
/// use claudine::messaging::{ScopedMessagingSettings, MessagingRouteConfig};
/// // (see tests for detailed usage)
/// ```
pub fn resolve_effective_route(
    messaging: &RuntimeMessagingSettings,
) -> Option<ResolvedMessagingRoute> {
    // Repo scope takes precedence over user scope
    if let Some(route) = resolve_scope(&messaging.repo, MessagingScope::Repo) {
        debug!(
            scope = "repo",
            route.name, "Resolved messaging route from repo scope"
        );
        return Some(route);
    }

    if let Some(route) = resolve_scope(&messaging.user, MessagingScope::User) {
        debug!(
            scope = "user",
            route.name, "Resolved messaging route from user scope"
        );
        return Some(route);
    }

    debug!("No active messaging route in any scope");
    None
}

/// Resolves an active route from a single optional scope.
fn resolve_scope(
    scope: &Option<ScopedMessagingSettings>,
    scope_kind: MessagingScope,
) -> Option<ResolvedMessagingRoute> {
    let settings = scope.as_ref()?;
    let active_name = settings.active.as_ref()?;
    let config = settings.configs.get(active_name)?;

    Some(ResolvedMessagingRoute {
        scope: scope_kind,
        name: active_name.clone(),
        config: config.clone(),
    })
}

/// Resolves a secret value from an inline string or an environment variable.
///
/// The inline value wins if present and non-empty. Falls back to the named
/// environment variable. Returns an error string naming the variable if
/// neither source provides a value.
///
/// ## Returns
///
/// `Ok(value)` with the resolved secret, or `Err(message)` describing which
/// env var was missing.
pub fn resolve_secret(inline: Option<&str>, env_name: &str) -> std::result::Result<String, String> {
    // Inline value wins when present and non-empty
    if let Some(val) = inline
        && !val.is_empty()
    {
        return Ok(val.to_string());
    }

    // Fall back to environment variable
    match std::env::var(env_name) {
        Ok(val) if !val.is_empty() => Ok(val),
        _ => Err(format!(
            "secret not found: set env var {} or provide an inline value",
            env_name
        )),
    }
}

/// Parses a Signal recipient string into a typed `SignalRecipient`.
///
/// Phone numbers begin with `+`; everything else is treated as a group ID.
pub fn parse_signal_recipient(recipient: &str) -> SignalRecipient {
    if recipient.starts_with('+') {
        SignalRecipient::Phone(recipient.to_string())
    } else {
        SignalRecipient::Group(recipient.to_string())
    }
}

/// Resolves a raw image path string to an absolute `PathBuf`.
///
/// Resolution order:
/// 1. Already absolute — returned as-is.
/// 2. Starts with `~/` — home directory expanded via `dirs::home_dir()`.
/// 3. Relative with a `cwd` — joined to `cwd`.
/// 4. Relative with a `repo_root` — joined to `repo_root`.
/// 5. Final fallback — joined to the process working directory.
pub fn resolve_image_path(raw: &str, cwd: Option<&str>, repo_root: Option<&str>) -> PathBuf {
    let path = Path::new(raw);

    // Absolute paths are returned unchanged
    if path.is_absolute() {
        return path.to_path_buf();
    }

    // Tilde expansion
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
        warn!("Could not determine home directory for tilde expansion");
    }

    // Relative: try cwd first
    if let Some(dir) = cwd {
        return PathBuf::from(dir).join(raw);
    }

    // Relative: try repo_root next
    if let Some(root) = repo_root {
        return PathBuf::from(root).join(raw);
    }

    // Final fallback: process working directory
    if let Ok(process_cwd) = std::env::current_dir() {
        return process_cwd.join(raw);
    }

    // Truly last resort: return as-is
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn discord_route(name: &str) -> (String, MessagingRouteConfig) {
        (
            name.to_string(),
            MessagingRouteConfig::Discord {
                channel_id: "123456789".to_string(),
                bot_token: None,
                bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
            },
        )
    }

    fn slack_route(name: &str) -> (String, MessagingRouteConfig) {
        (
            name.to_string(),
            MessagingRouteConfig::Slack {
                channel_id: "C0123456789".to_string(),
                bot_token: None,
                bot_token_env: "SLACK_BOT_TOKEN".to_string(),
            },
        )
    }

    fn scoped(
        active: Option<&str>,
        routes: Vec<(String, MessagingRouteConfig)>,
    ) -> ScopedMessagingSettings {
        ScopedMessagingSettings {
            active: active.map(str::to_string),
            configs: routes.into_iter().collect::<HashMap<_, _>>(),
        }
    }

    // -------------------------------------------------------------------------
    // resolve_effective_route tests
    // -------------------------------------------------------------------------

    #[test]
    fn repo_active_beats_user_active() {
        let settings = RuntimeMessagingSettings {
            user: Some(scoped(
                Some("slack-route"),
                vec![slack_route("slack-route")],
            )),
            repo: Some(scoped(
                Some("discord-route"),
                vec![discord_route("discord-route")],
            )),
        };

        let resolved = resolve_effective_route(&settings).expect("should resolve");
        assert_eq!(resolved.scope, MessagingScope::Repo);
        assert_eq!(resolved.name, "discord-route");
    }

    #[test]
    fn repo_inactive_falls_back_to_user() {
        let settings = RuntimeMessagingSettings {
            user: Some(scoped(
                Some("slack-route"),
                vec![slack_route("slack-route")],
            )),
            repo: Some(scoped(None, vec![discord_route("discord-route")])),
        };

        let resolved = resolve_effective_route(&settings).expect("should resolve");
        assert_eq!(resolved.scope, MessagingScope::User);
        assert_eq!(resolved.name, "slack-route");
    }

    #[test]
    fn both_inactive_returns_none() {
        let settings = RuntimeMessagingSettings {
            user: Some(scoped(None, vec![slack_route("slack-route")])),
            repo: Some(scoped(None, vec![discord_route("discord-route")])),
        };

        assert!(resolve_effective_route(&settings).is_none());
    }

    #[test]
    fn no_scopes_returns_none() {
        let settings = RuntimeMessagingSettings {
            user: None,
            repo: None,
        };

        assert!(resolve_effective_route(&settings).is_none());
    }

    #[test]
    fn user_only_scope() {
        let settings = RuntimeMessagingSettings {
            user: Some(scoped(
                Some("slack-route"),
                vec![slack_route("slack-route")],
            )),
            repo: None,
        };

        let resolved = resolve_effective_route(&settings).expect("should resolve");
        assert_eq!(resolved.scope, MessagingScope::User);
        assert_eq!(resolved.name, "slack-route");
    }

    // -------------------------------------------------------------------------
    // resolve_secret tests
    // -------------------------------------------------------------------------

    #[test]
    #[serial]
    fn inline_secret_wins_over_env() {
        // Even if the env var is set, the inline value should win
        // SAFETY: single-threaded test environment; no concurrent env access
        unsafe {
            std::env::set_var("TEST_INLINE_WINS_VAR", "from-env");
        }
        let result = resolve_secret(Some("from-inline"), "TEST_INLINE_WINS_VAR");
        // SAFETY: single-threaded test environment; no concurrent env access
        unsafe {
            std::env::remove_var("TEST_INLINE_WINS_VAR");
        }

        assert_eq!(result, Ok("from-inline".to_string()));
    }

    #[test]
    #[serial]
    fn env_fallback_works() {
        let var_name = "TEST_ENV_FALLBACK_SECRET_VAR";
        // SAFETY: single-threaded test environment; no concurrent env access
        unsafe {
            std::env::set_var(var_name, "secret-from-env");
        }

        let result = resolve_secret(None, var_name);

        // SAFETY: single-threaded test environment; no concurrent env access
        unsafe {
            std::env::remove_var(var_name);
        }
        assert_eq!(result, Ok("secret-from-env".to_string()));
    }

    #[test]
    #[serial]
    fn missing_env_returns_error() {
        let var_name = "TEST_MISSING_SECRET_VAR_XYZ_NOT_SET";
        // SAFETY: single-threaded test environment; no concurrent env access
        unsafe {
            std::env::remove_var(var_name);
        }

        let result = resolve_secret(None, var_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(var_name));
    }

    // -------------------------------------------------------------------------
    // parse_signal_recipient tests
    // -------------------------------------------------------------------------

    #[test]
    fn signal_phone_recipient() {
        let result = parse_signal_recipient("+15551234567");
        assert_eq!(result, SignalRecipient::Phone("+15551234567".to_string()));
    }

    #[test]
    fn signal_group_recipient() {
        let result = parse_signal_recipient("group-id-base64");
        assert_eq!(
            result,
            SignalRecipient::Group("group-id-base64".to_string())
        );
    }

    // -------------------------------------------------------------------------
    // resolve_image_path tests
    // -------------------------------------------------------------------------

    #[test]
    fn absolute_path_unchanged() {
        let absolute = std::env::current_dir().unwrap().join("absolute/path.png");
        let result = resolve_image_path(absolute.to_str().unwrap(), None, None);
        assert_eq!(result, absolute);
    }

    #[test]
    fn tilde_path_expands() {
        let result = resolve_image_path("~/images/shot.png", None, None);
        // Should start with the home directory
        let home = dirs::home_dir().expect("home dir must exist for this test");
        assert_eq!(result, home.join("images/shot.png"));
    }

    #[test]
    fn relative_path_uses_cwd() {
        let result = resolve_image_path(
            "artifacts/shot.png",
            Some("/workspace/project"),
            Some("/repo/root"),
        );
        assert_eq!(
            result,
            PathBuf::from("/workspace/project/artifacts/shot.png")
        );
    }

    #[test]
    fn relative_path_falls_back_to_repo_root() {
        let result = resolve_image_path("artifacts/shot.png", None, Some("/repo/root"));
        assert_eq!(result, PathBuf::from("/repo/root/artifacts/shot.png"));
    }
}
