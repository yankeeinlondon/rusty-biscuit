//! Saving what `claudine config` changed.
//!
//! The editor and the `claudine config set` setters never write a whole
//! configuration. The editor saves the difference between each scope's value
//! as it was opened and as it is now; a setter names its one setting. Both go
//! through [`claudine::config::persist`], which re-reads the file under its
//! lock and keeps every comment and every key it was not asked to change,
//! including `permissions` another program wrote while the editor was open.

use std::path::Path;

use claudine::config::persist::{self, SaveOutcome};
use claudine::config::schema::ConfigScope;
use claudine::dispatch::loader::load_claudine_config;
use claudine::error::Result;
use claudine::provider::Provider;
use serde_json::Value;

use super::app::App;

/// Whether a setter changed the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOutcome {
    /// The setting already held the value; nothing was written.
    Unchanged,
    /// The setting was written.
    Updated,
}

/// Saves the user settings the session changed to the user file at `path`.
///
/// ## Errors
///
/// The save's error; the file is then unchanged.
pub fn save_user(app: &App, path: &Path) -> Result<SaveOutcome> {
    persist::save_changes(path, ConfigScope::User, &app.opening_config, &app.config)
}

/// Saves the repository overrides the session changed. `None` outside a
/// repository. A repository file is created only when there is an override
/// to write, and is never deleted: removing every override leaves `{}` and
/// whatever else the file holds.
///
/// ## Errors
///
/// The save's error; the file is then unchanged.
pub fn save_repo(app: &App) -> Option<Result<SaveOutcome>> {
    let path = app.repo_config_path.as_deref()?;
    let opened = app.opening_repo_config.clone().unwrap_or_default();
    let edited = app.repo_config.clone().unwrap_or_default();
    Some(persist::save_changes(path, ConfigScope::Repository, &opened, &edited))
}

/// Sets (or, with `None`, removes) `preferred_agent` in the user file at
/// `path`, changing nothing else.
///
/// ## Errors
///
/// When the file does not load, or the save fails.
pub fn set_favorite_agent(path: &Path, favorite: Option<Provider>) -> Result<SetOutcome> {
    if load_claudine_config(Some(path), None)?.preferred_agent == favorite {
        return Ok(SetOutcome::Unchanged);
    }
    let edit = match favorite {
        Some(provider) => persist::set_path("preferred_agent", serde_json::to_value(provider)?)?,
        None => persist::remove_path("preferred_agent")?,
    };
    persist::edit_config(path, ConfigScope::User, vec![edit])?;
    Ok(SetOutcome::Updated)
}

/// Sets `prompt_for_missing` in the user file at `path`, changing nothing
/// else.
///
/// ## Errors
///
/// When the file does not load, or the save fails.
pub fn set_prompt_for_missing(path: &Path, value: bool) -> Result<SetOutcome> {
    if load_claudine_config(Some(path), None)?.prompt_for_missing == value {
        return Ok(SetOutcome::Unchanged);
    }
    persist::edit_config(path, ConfigScope::User, vec![persist::set_path("prompt_for_missing", Value::Bool(value))?])?;
    Ok(SetOutcome::Updated)
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
