use std::collections::HashMap;

use crate::models::metadata::Datetime;

pub enum AgenticStatusPlatform {
    Codex,
    ClaudeCode,
}

impl Default for AgenticStatusPlatform {
    fn default() -> Self {
        todo!()
    }
}

pub struct AgenticStatusApiKeys {
    codex: Option<String>,
    claude_code: Option<String>,
}

pub enum CapWindowSize {
    Hours(u8),
    Daily,
    Weekly,
    Monthly
}

pub struct AgenticCapLimit {
    /// Reports on the short term cap, specifically what percentage of this cap has been used.
    pub short_cap_usage: f32,

    /// reports on when the short term cap will be reset
    pub short_cap_until: Datetime,
    /// the duration for which the short term cap is open for before resetting
    pub short_cap_window_size: CapWindowSize,

    /// Reports on the long term cap, specifically what percentage of this cap has been used.
    pub long_cap_usage: f32,
    /// the duration for which the long term cap is open for before resetting
    pub long_cap_window_size: CapWindowSize,
    /// reports on when the short term cap will be reset
    pub long_cap_until: Datetime
}


/// The **AgentStatus** struct provides status information about
/// Agentic CLI platforms it has access to.
pub struct AgentStatus {
    pub installed: HashMap<AgenticStatusPlatform,bool>,
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
    pub async fn new(keys: Option<AgenticStatusApiKeys>) {
        todo!()
    }


    /// Provides the cap limits for the providers we have access to.
    ///
    /// - provides as a vector of `AgentLimit` by default but
    /// - you can pass in a specific AgenticStatusPlatform to get
    ///   just that platform's
    pub async fn limits(platform: Option<AgenticStatusPlatform>) {
        todo!()
    }

    /// reports on all status including caps (though currently we only have
    /// the caps we're reporting on)
    pub async fn status(platform: Option<AgenticStatusPlatform>) {
        todo!()
    }
}
