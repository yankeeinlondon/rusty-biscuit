//! Saving Claudine configuration files without rewriting them.
//!
//! Every Claudine writer (`claudine config` and its setters, and setup)
//! changes a configuration file through a list of per-path [`YamlEdit`]s
//! applied by Darkmatter's locked writer
//! ([`darkmatter::config::write::ConfigTransaction`]). Under the target's lock
//! the file is re-read, only the named values change, and the candidate text
//! is judged by the scope's schema and Claudine's semantic checks before it
//! replaces the file. Every comment, blank line, key order, quoting style, and
//! key a writer does not name (another program's `permissions`, for example)
//! is kept, and two cooperating writers changing different keys keep each
//! other's edits. A refused or failed save leaves the file untouched and is
//! returned as an error.
//!
//! A writer that holds a whole typed configuration supplies the value it
//! opened and the value it holds now; [`changed_edits`] turns the difference
//! into edits. A new file starts from the scope's schema projection with the
//! writer's edits applied, laid out with the destination's indentation
//! preference ([`initial_text`] previews it with the default).
//!
//! User-facing guide: `claudine/docs/topics/configuration.md`.

use std::cell::RefCell;
use std::path::Path;

use biscuit_file::Indentation;
use darkmatter::config::write::{
    ConfigTransaction, ConfigWriteError, SourceExpectation, TargetChange, ValidationCandidate, ValidationRefusal,
};
use serde::Serialize;
use serde_json::{Map, Value};
use simplified_schema::yaml_documents::load_yaml_document;

pub use darkmatter::yaml_edit::YamlEdit;
pub use simplified_schema::PropertyPath;

use super::load::{ConfigProblem, ConfigProblemKind, document_from_bytes, project_value};
use super::schema::ConfigScope;
use super::{ClaudineConfig, RepoOverrideConfig};
use crate::error::{ClaudineError, ConfigCause, Result};

/// What a save did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SaveOutcome {
    /// The file did not exist and was created.
    pub created: bool,
    /// The file's bytes were replaced (or created). `false` when every edit
    /// already held, so nothing was written.
    pub written: bool,
}

/// A [`YamlEdit::Set`] of `value` at the dotted `path` (`tts.gender`).
///
/// ## Errors
///
/// [`ClaudineError::ConfigValidationWithCause`] when `path` is not a property
/// path.
pub fn set_path(path: &str, value: Value) -> Result<YamlEdit> {
    Ok(YamlEdit::Set { path: parse_path(path)?, value })
}

/// A [`YamlEdit::Remove`] of the dotted `path`.
///
/// ## Errors
///
/// [`ClaudineError::ConfigValidationWithCause`] when `path` is not a property
/// path.
pub fn remove_path(path: &str) -> Result<YamlEdit> {
    Ok(YamlEdit::Remove { path: parse_path(path)? })
}

fn parse_path(path: &str) -> Result<PropertyPath> {
    PropertyPath::parse(path).map_err(|error| path_error(format!("`{path}` is not a configuration path"), error))
}

/// Applies `edits` to the existing configuration file of `scope` at `path`.
///
/// Edits that already hold in the file are dropped; when none remain nothing
/// is written. In a user file that spells `preferred_agent` by its former
/// name `favorite_agent`, an edit of `preferred_agent` edits that key.
///
/// ## Errors
///
/// - [`ClaudineError::ConfigNotFound`] when nothing is at `path`;
/// - [`ClaudineError::ConfigEditRefused`] when the edited file would be
///   invalid, with every problem located in the candidate text;
/// - [`ClaudineError::ConfigWrite`] when the edit cannot be made in place,
///   the lock is not acquired in time, or the file cannot be replaced.
///
/// In every error case the file is unchanged.
pub fn edit_config(path: &Path, scope: ConfigScope, edits: Vec<YamlEdit>) -> Result<SaveOutcome> {
    let authored = read_authored(path)?.ok_or_else(|| ClaudineError::ConfigNotFound(path.to_path_buf()))?;
    commit_edits(path, scope, authored.as_ref(), edits)
}

/// Saves what changed between `opened` and `edited`, two typed values of the
/// configuration of `scope` at `path`, as per-path edits ([`changed_edits`])
/// applied to the file as it is now.
///
/// Nothing is written when the two serialize alike. A missing file is created
/// from the scope's projection (see [`initial_text`]) only when there is
/// something to save.
///
/// ## Errors
///
/// As [`edit_config`], and [`ClaudineError::Io`] when a missing file's
/// directory cannot be created.
pub fn save_changes<T: Serialize>(path: &Path, scope: ConfigScope, opened: &T, edited: &T) -> Result<SaveOutcome> {
    save(path, scope, &serde_json::to_value(opened)?, &serde_json::to_value(edited)?, false)
}

/// Writes setup values: the difference between `defaults` (the typed
/// projection of an empty file, such as [`ClaudineConfig::default`]) and
/// `initialized`, so setup manages exactly the values it chose.
///
/// A missing file is always created, from the scope's projection (see
/// [`initial_text`]) with those edits;
/// an existing file receives only those edits, so running setup again with
/// the same values writes nothing.
///
/// ## Errors
///
/// As [`save_changes`].
pub fn initialize_config<T: Serialize>(
    path: &Path,
    scope: ConfigScope,
    defaults: &T,
    initialized: &T,
) -> Result<SaveOutcome> {
    save(path, scope, &serde_json::to_value(defaults)?, &serde_json::to_value(initialized)?, true)
}

fn save(path: &Path, scope: ConfigScope, opened: &Value, edited: &Value, create_always: bool) -> Result<SaveOutcome> {
    match read_authored(path)? {
        Some(authored) => {
            if opened == edited {
                return Ok(SaveOutcome::default());
            }
            let edits = changed_edits(authored.as_ref(), opened, edited)?;
            commit_edits(path, scope, authored.as_ref(), edits)
        }
        None => {
            if opened == edited && !create_always {
                return Ok(SaveOutcome::default());
            }
            let initial = initial_value(scope);
            let edits = changed_edits(Some(&initial), opened, edited)?;
            create_config(path, scope, initial, edits)
        }
    }
}

/// The text a new configuration file of `scope` starts from, laid out with
/// the default indentation: the schema's projection of an empty file. A user
/// file lists every default the schema declares; a repository file inherits
/// everything, so it starts as `{}`. A created file holds the same values in
/// its destination's indentation.
#[must_use]
pub fn initial_text(scope: ConfigScope) -> String {
    biscuit_file::to_yaml_string(&initial_value(scope), Indentation::default())
        .expect("the schema projection is a JSON mapping, which always lays out as YAML")
}

fn initial_value(scope: ConfigScope) -> Value {
    project_value(scope, &Value::Object(Map::new())).expect("the shipped schema accepts an empty configuration file")
}

/// The per-path edits that turn `opened` into `edited`, two serialized
/// configurations of one scope, given the file's `authored` document (`None`
/// when there is no file, or it is not readable YAML).
///
/// Mappings are compared key by key, recursively, while the file holds a
/// mapping (or nothing) at that path. A changed sequence, scalar, or change
/// of shape is one [`YamlEdit::Set`] of the whole new value. A key `edited`
/// gained is set; a key it lost is removed when the file writes it, and left
/// alone when it came from a schema default: the file never wrote it, and
/// neither removing it nor writing `null` (which takes the default again)
/// would change the configuration.
///
/// ## Errors
///
/// [`ClaudineError::ConfigValidationWithCause`] when a changed key cannot be
/// spelled as a property path (an empty key).
pub fn changed_edits(authored: Option<&Value>, opened: &Value, edited: &Value) -> Result<Vec<YamlEdit>> {
    let mut edits = Vec::new();
    let mut segments = Vec::new();
    diff(authored, opened, edited, &mut segments, &mut edits)?;
    Ok(edits)
}

fn diff(
    authored: Option<&Value>,
    opened: &Value,
    edited: &Value,
    segments: &mut Vec<String>,
    edits: &mut Vec<YamlEdit>,
) -> Result<()> {
    if opened == edited {
        return Ok(());
    }
    let walkable = matches!(authored, None | Some(Value::Object(_)));
    let (Value::Object(before), Value::Object(after), true) = (opened, edited, walkable) else {
        edits.push(YamlEdit::Set { path: path_of(segments)?, value: edited.clone() });
        return Ok(());
    };
    for (key, after_value) in after {
        let authored_child = authored.and_then(|value| value.get(key));
        segments.push(key.clone());
        match before.get(key) {
            Some(before_value) => diff(authored_child, before_value, after_value, segments, edits)?,
            None => edits.push(YamlEdit::Set { path: path_of(segments)?, value: after_value.clone() }),
        }
        segments.pop();
    }
    for key in before.keys().filter(|key| !after.contains_key(*key)) {
        if authored.and_then(|value| value.get(key)).is_some() {
            segments.push(key.clone());
            edits.push(YamlEdit::Remove { path: path_of(segments)? });
            segments.pop();
        }
    }
    Ok(())
}

fn path_of(segments: &[String]) -> Result<PropertyPath> {
    PropertyPath::from_segments(segments.iter().cloned())
        .map_err(|error| path_error(format!("cannot name the configuration value at {segments:?}"), error))
}

fn path_error(context: String, error: simplified_schema::PropertyPathError) -> ClaudineError {
    ClaudineError::ConfigValidationWithCause { message: context, source: ConfigCause::Path(error) }
}

/// Checks `text` as a whole configuration file of `scope` at `path`: the
/// scope's schema, then the typed read and (for a user file) the semantic
/// checks of [`ClaudineConfig::validate`].
///
/// ## Errors
///
/// [`ClaudineError::ConfigInvalid`] with every problem located.
pub fn validate_config_text(text: &str, path: &Path, scope: ConfigScope) -> Result<()> {
    check_text(text, path, scope).map_err(|problems| ClaudineError::ConfigInvalid { problems })
}

fn check_text(text: &str, path: &Path, scope: ConfigScope) -> std::result::Result<(), Vec<ConfigProblem>> {
    let as_problems = |error: ClaudineError| match error {
        ClaudineError::ConfigInvalid { problems } => problems,
        other => vec![ConfigProblem::whole(Some(path), ConfigProblemKind::InvalidSetting, other.to_string())],
    };
    let document = document_from_bytes(text.as_bytes(), path, scope).map_err(as_problems)?;
    match scope {
        ConfigScope::User => ClaudineConfig::from_effective(document.effective, Some(path)).and_then(|config| config.validate()),
        ConfigScope::Repository => RepoOverrideConfig::from_effective(document.effective, Some(path)).map(|_| ()),
    }
    .map_err(as_problems)
}

/// The file's authored document: `None` when the file is missing, and
/// `Some(None)` when it exists but is not readable YAML (the locked editor
/// then reports where).
fn read_authored(path: &Path) -> Result<Option<Option<Value>>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            Ok(Some(load_yaml_document(&text, path).ok().map(|loaded| loaded.json)))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ClaudineError::Io(error)),
    }
}

/// Drops edits that already hold in `authored`, and points a user file's
/// `preferred_agent` edit at `favorite_agent` when only the former name is
/// written.
fn effective_edits(scope: ConfigScope, authored: Option<&Value>, edits: Vec<YamlEdit>) -> Result<Vec<YamlEdit>> {
    let former_name = scope == ConfigScope::User
        && authored.is_some_and(|document| document.get("favorite_agent").is_some() && document.get("preferred_agent").is_none());
    let mut kept = Vec::with_capacity(edits.len());
    for edit in edits {
        let edit = if former_name { renamed(edit)? } else { edit };
        let holds = match (&edit, authored) {
            (YamlEdit::Set { path, value }, Some(document)) => value_at(document, path.segments()) == Some(value),
            (YamlEdit::Remove { path }, Some(document)) => value_at(document, path.segments()).is_none(),
            (YamlEdit::Remove { .. }, None) => true,
            (YamlEdit::Set { .. }, None) => false,
        };
        if !holds {
            kept.push(edit);
        }
    }
    Ok(kept)
}

fn renamed(edit: YamlEdit) -> Result<YamlEdit> {
    let rename = |path: PropertyPath| -> Result<PropertyPath> {
        match path.segments().split_first() {
            Some((first, rest)) if first == "preferred_agent" => {
                path_of(&std::iter::once("favorite_agent".to_string()).chain(rest.iter().cloned()).collect::<Vec<_>>())
            }
            _ => Ok(path),
        }
    };
    Ok(match edit {
        YamlEdit::Set { path, value } => YamlEdit::Set { path: rename(path)?, value },
        YamlEdit::Remove { path } => YamlEdit::Remove { path: rename(path)? },
    })
}

fn value_at<'a>(document: &'a Value, segments: &[String]) -> Option<&'a Value> {
    segments.iter().try_fold(document, |value, segment| match value {
        Value::Object(map) => map.get(segment),
        Value::Array(items) => segment.parse::<usize>().ok().and_then(|index| items.get(index)),
        _ => None,
    })
}

fn commit_edits(path: &Path, scope: ConfigScope, authored: Option<&Value>, edits: Vec<YamlEdit>) -> Result<SaveOutcome> {
    let edits = effective_edits(scope, authored, edits)?;
    if edits.is_empty() {
        return Ok(SaveOutcome::default());
    }
    let path = absolute_target(path)?;
    commit(&path, scope, TargetChange::edit(&path, SourceExpectation::Current, edits))
}

fn create_config(path: &Path, scope: ConfigScope, initial: Value, edits: Vec<YamlEdit>) -> Result<SaveOutcome> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let edits = effective_edits(scope, Some(&initial), edits)?;
    let path = absolute_target(path)?;
    commit(&path, scope, TargetChange::create_yaml(&path, initial, edits))
}

/// The locked writer takes only absolute targets. With no home, Claudine's
/// configuration path is the literal relative `~/.claudine/config.yaml` (its
/// long-standing no-home fallback), resolved against the working directory as
/// any ordinary file write would resolve it.
fn absolute_target(path: &Path) -> Result<std::path::PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::path::absolute(path)?)
}

/// Commits one target under its lock, judging the candidate with
/// [`check_text`].
fn commit(path: &Path, scope: ConfigScope, change: TargetChange) -> Result<SaveOutcome> {
    let write_error = |source: ConfigWriteError| ClaudineError::ConfigWrite { path: path.to_path_buf(), source: Box::new(source) };
    let mut transaction = ConfigTransaction::new();
    transaction.add(change).map_err(write_error)?;

    let refused: RefCell<Vec<ConfigProblem>> = RefCell::new(Vec::new());
    let validate = |candidate: &ValidationCandidate<'_>| {
        check_text(candidate.candidate, candidate.target, scope).map_err(|problems| {
            let message = super::load::join_problems(&problems);
            *refused.borrow_mut() = problems;
            ValidationRefusal::new(message)
        })
    };
    match transaction.commit(&validate) {
        Ok(outcome) => {
            let target = outcome.targets.first();
            let outcome = SaveOutcome {
                created: target.is_some_and(|target| target.created),
                written: target.is_some_and(|target| target.written),
            };
            tracing::info!(?path, ?scope, written = outcome.written, "Saved Claudine configuration");
            Ok(outcome)
        }
        Err(ConfigWriteError::ValidationFailed { refusal, .. }) => {
            let mut problems = refused.into_inner();
            if problems.is_empty() {
                problems.push(ConfigProblem::whole(Some(path), ConfigProblemKind::InvalidSetting, refusal.message));
            }
            Err(ClaudineError::ConfigEditRefused { path: path.to_path_buf(), problems })
        }
        Err(other) => Err(write_error(other)),
    }
}

#[cfg(test)]
#[path = "persist_tests.rs"]
mod tests;
