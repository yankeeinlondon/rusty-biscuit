use std::collections::HashMap;
use std::fmt;

use crate::models::metadata::Datetime;
use super::error::AgentStatusError;

/// Platform types for agentic CLI tools.
///
/// This enum represents the different AI-powered CLI platforms that can be
/// detected and queried for status information.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgenticStatusPlatform {
    Codex,
    ClaudeCode,
}

impl fmt::Display for AgenticStatusPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgenticStatusPlatform::Codex => write!(f, "Codex"),
            AgenticStatusPlatform::ClaudeCode => write!(f, "ClaudeCode"),
        }
    }
}

/// API keys for agentic platforms.
///
/// Allows passing in API keys explicitly rather than relying on environment variables.
#[derive(Debug, Clone)]
pub struct AgenticStatusApiKeys {
    pub codex: Option<String>,
    pub claude_code: Option<String>,
}

/// Window size for rate limit caps.
///
/// Defines the time window over which rate limits are enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapWindowSize {
    Session,
    Hours(u8),
    Daily,
    Weekly,
    Monthly,
}

impl fmt::Display for CapWindowSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapWindowSize::Session => write!(f, "session"),
            CapWindowSize::Hours(n) => write!(f, "{} hour{}", n, if *n == 1 { "" } else { "s" }),
            CapWindowSize::Daily => write!(f, "daily"),
            CapWindowSize::Weekly => write!(f, "weekly"),
            CapWindowSize::Monthly => write!(f, "monthly"),
        }
    }
}

/// A single rate limit cap entry parsed from platform output.
///
/// Each platform may report multiple cap windows (e.g., Claude Code reports
/// session + 5h + daily + weekly; Codex reports 5h + weekly).
#[derive(Debug, Clone)]
pub struct CapEntry {
    /// Display label from the output (e.g., "Current session", "5h limit").
    pub label: String,
    /// Usage as a fraction from 0.0 to 1.0+ (values above 1.0 indicate over-limit).
    pub usage: f32,
    /// The time window this cap covers.
    pub window_size: CapWindowSize,
    /// When this cap resets (raw string from output, or "unknown").
    pub resets_at: Datetime,
}

/// Rate limit information for an agentic platform.
///
/// Contains an ordered list of cap entries as they appear in the platform's
/// status output. Different platforms report different numbers of caps.
#[derive(Debug, Clone)]
pub struct AgenticCapLimit {
    /// Cap entries ordered as they appear in the platform output.
    pub caps: Vec<CapEntry>,
}

impl AgenticCapLimit {
    /// Returns the first (shortest-window) cap entry, if any.
    pub fn short_cap(&self) -> Option<&CapEntry> {
        self.caps.first()
    }

    /// Returns the second cap entry, if any.
    pub fn long_cap(&self) -> Option<&CapEntry> {
        self.caps.get(1)
    }
}


/// The **AgentStatus** struct provides status information about
/// Agentic CLI platforms it has access to.
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub installed: HashMap<AgenticStatusPlatform, bool>,
    pub available: Vec<AgenticStatusPlatform>,
    pub limits: HashMap<String, AgenticCapLimit>,
}

impl Default for AgentStatus {
    fn default() -> Self {
        AgentStatus {
            installed: HashMap::new(),
            available: vec![],
            limits: HashMap::new()
        }
    }
}

impl AgentStatus {
    /// Create a new `AgentStatus` struct and populate the available platforms which
    /// have a valid API Key we can use to check:
    ///
    /// 1. we will use ENV keys, or
    /// 2. you can pass in the keys to the new function
    ///
    /// API keys _passed in_ to the new function will always take precedence over ENV vars.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - PTY spawning fails during platform detection
    /// - Reading from PTY processes fails
    /// - Parsing version or status information fails
    pub async fn new(_keys: Option<AgenticStatusApiKeys>) -> Result<Self, AgentStatusError> {
        // Use sniff to detect installed platforms
        let ai_clients = sniff::programs::InstalledAiClients::new();

        let mut installed = HashMap::new();
        let mut available = Vec::new();

        // Check Claude Code
        let claude_installed = ai_clients.claude();
        installed.insert(AgenticStatusPlatform::ClaudeCode, claude_installed);
        if claude_installed {
            available.push(AgenticStatusPlatform::ClaudeCode);
        }

        // Check Codex
        let codex_installed = ai_clients.codex();
        installed.insert(AgenticStatusPlatform::Codex, codex_installed);
        if codex_installed {
            available.push(AgenticStatusPlatform::Codex);
        }

        Ok(AgentStatus {
            installed,
            available,
            limits: HashMap::new(),
        })
    }


    /// Provides the cap limits for the providers we have access to.
    ///
    /// - provides as a vector of `AgentLimit` by default but
    /// - you can pass in a specific AgenticStatusPlatform to get
    ///   just that platform's
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The requested platform is not installed or has no valid API key
    /// - API requests to query limits fail
    /// - Response parsing fails
    pub async fn limits(&self, platform: Option<AgenticStatusPlatform>) -> Result<HashMap<String, AgenticCapLimit>, AgentStatusError> {
        let platforms: Vec<AgenticStatusPlatform> = match platform {
            Some(p) => {
                // Check if the platform is available
                if !self.available.contains(&p) {
                    return Err(AgentStatusError::PlatformNotInstalled(
                        format!("{} is not installed or not available", p)
                    ));
                }
                vec![p]
            }
            None => self.available.clone(),
        };

        let mut limits = HashMap::new();

        for p in platforms {
            let (program, args) = match p {
                AgenticStatusPlatform::ClaudeCode => ("claude", vec!["/status"]),
                AgenticStatusPlatform::Codex => ("codex", vec!["--status"]),
            };

            match super::pty_runner::run_pty_command(program, &args, None).await {
                Ok(output) => {
                    match super::parsers::parse_status_output(p, &output) {
                        Ok(cap_limit) => {
                            limits.insert(p.to_string(), cap_limit);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse status for {}: {}", p, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to query status for {}: {}", p, e);
                }
            }
        }

        Ok(limits)
    }

    /// reports on all status including caps (though currently we only have
    /// the caps we're reporting on)
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The requested platform is not installed or has no valid API key
    /// - API requests to query status fail
    /// - Response parsing fails
    pub async fn status(&self, platform: Option<AgenticStatusPlatform>) -> Result<AgentStatus, AgentStatusError> {
        let limits = self.limits(platform).await?;

        Ok(AgentStatus {
            installed: self.installed.clone(),
            available: self.available.clone(),
            limits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_agent_status() {
        let status = AgentStatus::default();
        assert!(status.installed.is_empty());
        assert!(status.available.is_empty());
        assert!(status.limits.is_empty());
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(format!("{}", AgenticStatusPlatform::ClaudeCode), "ClaudeCode");
        assert_eq!(format!("{}", AgenticStatusPlatform::Codex), "Codex");
    }

    #[test]
    fn test_cap_window_display() {
        assert_eq!(format!("{}", CapWindowSize::Session), "session");
        assert_eq!(format!("{}", CapWindowSize::Daily), "daily");
        assert_eq!(format!("{}", CapWindowSize::Hours(5)), "5 hours");
        assert_eq!(format!("{}", CapWindowSize::Hours(1)), "1 hour");
    }

    #[test]
    fn test_cap_entry_accessors() {
        let limit = AgenticCapLimit {
            caps: vec![
                CapEntry {
                    label: "5h limit".to_string(),
                    usage: 0.25,
                    window_size: CapWindowSize::Hours(5),
                    resets_at: Datetime("in 3h".to_string()),
                },
                CapEntry {
                    label: "Weekly limit".to_string(),
                    usage: 0.10,
                    window_size: CapWindowSize::Weekly,
                    resets_at: Datetime("Mon".to_string()),
                },
            ],
        };
        assert!(limit.short_cap().is_some());
        assert!((limit.short_cap().unwrap().usage - 0.25).abs() < 0.01);
        assert!(limit.long_cap().is_some());
        assert!((limit.long_cap().unwrap().usage - 0.10).abs() < 0.01);
    }

    #[test]
    fn test_empty_caps() {
        let limit = AgenticCapLimit { caps: vec![] };
        assert!(limit.short_cap().is_none());
        assert!(limit.long_cap().is_none());
    }

    #[tokio::test]
    async fn test_new_detects_platforms() {
        // This test runs real detection - may find no platforms or some
        let result = AgentStatus::new(None).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        // installed should have entries for both platforms regardless
        assert_eq!(status.installed.len(), 2);
    }

    #[tokio::test]
    async fn test_limits_with_no_platforms() {
        let status = AgentStatus::default();
        let limits = status.limits(None).await;
        assert!(limits.is_ok());
        assert!(limits.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_limits_with_uninstalled_platform() {
        let status = AgentStatus::default();
        let result = status.limits(Some(AgenticStatusPlatform::ClaudeCode)).await;
        assert!(result.is_err());
    }
}
