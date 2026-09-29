//! Automatic repetition help: its on/off setting, the per-execution
//! opportunity budget, and the message it sends.
//!
//! When the content detector reports an early repetition warning, the owner
//! of the producing execution may send the agent [`HELPER_MESSAGE`] through a
//! non-interrupting route. Each warning is one opportunity whether or not a
//! message can be sent; an execution gets [`OPPORTUNITIES_PER_EXECUTION`].
//! Turning automatic help off removes the messages and the notices that say
//! one could not be sent; repetition detection, its stop limit, and manual
//! `claudine steer` are unaffected.
//!
//! Topic: `claudine/docs/topics/automatic-steering.md`.

use std::ffi::OsStr;

use serde::{Deserialize, Deserializer, Serialize};

use super::identity::OpportunityId;
use crate::error::{ClaudineError, Result};

#[cfg(test)]
mod tests;

/// Runtime override for `steering.automatic.enabled`.
pub const AUTO_STEER_ENV: &str = "CLAUDINE_AUTO_STEER";

/// Warning opportunities one agent execution gets. Unavailable, refused,
/// failed, and unconfirmed attempts each use one; none is refunded.
pub const OPPORTUNITIES_PER_EXECUTION: usize = 3;

/// The message automatic help sends. It describes a suspicion and asks the
/// agent to check its progress; it never asks it to claim success.
pub const HELPER_MESSAGE: &str = "Claudine has detected repeated output that may indicate a loop. \
Please check whether you are making progress toward the user's task. \
If you are repeating the same approach, change your approach or stop and explain what is preventing progress. \
Claudine's existing runaway limits still apply.";

/// The `steering` section of user (`~/.claudine/config.json`) and repo
/// (`<repo>/.claudine/config.json`) configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteeringConfig {
    #[serde(default, skip_serializing_if = "AutomaticSteeringConfig::is_unset")]
    pub automatic: AutomaticSteeringConfig,
}

impl SteeringConfig {
    /// Whether nothing is set, so the section can be omitted.
    pub fn is_unset(&self) -> bool {
        self.automatic.is_unset()
    }
}

/// `steering.automatic`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomaticSteeringConfig {
    /// `None` when the key is absent, so the layer inherits. An explicit
    /// `null` is an error rather than a second spelling of absent.
    #[serde(default, deserialize_with = "present_bool", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl AutomaticSteeringConfig {
    /// Whether `enabled` is absent.
    pub fn is_unset(&self) -> bool {
        self.enabled.is_none()
    }
}

/// Reads a key that is present as a boolean; `null` and other types fail.
fn present_bool<'de, D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Option<bool>, D::Error> {
    bool::deserialize(deserializer).map(Some)
}

/// Parses a [`AUTO_STEER_ENV`] value: trimmed, case-insensitive
/// `true/false`, `1/0`, `yes/no`, or `on/off`.
///
/// ## Errors
///
/// [`ClaudineError::ConfigValidation`] for an empty, non-Unicode, or
/// unrecognized value.
pub fn parse_auto_steer(raw: &OsStr) -> Result<bool> {
    let Some(text) = raw.to_str() else {
        return Err(invalid_env("is not valid Unicode"));
    };
    let value = text.trim().to_ascii_lowercase();
    match value.as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        "" => Err(invalid_env("is empty")),
        _ => Err(invalid_env(&format!("is `{}`", text.trim()))),
    }
}

fn invalid_env(problem: &str) -> ClaudineError {
    ClaudineError::ConfigValidation(format!(
        "`{AUTO_STEER_ENV}` {problem}; use true/false, 1/0, yes/no, or on/off, or unset it to use \
         `steering.automatic.enabled` from configuration"
    ))
}

/// Whether automatic help is on: the environment value, else the repo
/// value, else the user value, else on. Only a value that is actually set
/// takes part, so an absent repo value keeps a user opt-out.
///
/// ## Errors
///
/// A present environment value that [`parse_auto_steer`] rejects.
pub fn resolve_enabled(env: Option<&OsStr>, repo: Option<&SteeringConfig>, user: &SteeringConfig) -> Result<bool> {
    if let Some(raw) = env {
        return parse_auto_steer(raw);
    }
    Ok(repo
        .and_then(|repo| repo.automatic.enabled)
        .or(user.automatic.enabled)
        .unwrap_or(true))
}

/// The warning opportunities left to one execution.
#[derive(Debug, Clone, Default)]
pub struct OpportunityBudget {
    used: usize,
}

impl OpportunityBudget {
    /// Uses one opportunity, or `None` once all are used.
    pub fn claim(&mut self) -> Option<OpportunityId> {
        (self.used < OPPORTUNITIES_PER_EXECUTION).then(|| {
            self.used += 1;
            OpportunityId::random()
        })
    }

    /// Opportunities used so far.
    pub fn used(&self) -> usize {
        self.used
    }
}
