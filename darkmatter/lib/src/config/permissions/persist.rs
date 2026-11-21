//! Saving an "always allow" approval to the user's permissions.
//!
//! [`save_user_approval`] appends one exact entry to the user scope's
//! `file-read` or `shell-commands` list: in `~/.darkmatter.yaml`, or in the
//! file its `permissions` reference names. It edits only that list (every
//! other byte, comment, and key stays), materializes the scope's schema
//! default first when the list was omitted (a written list replaces the
//! default), deduplicates, validates the edited block, and writes through a
//! locked [`ConfigTransaction`].
//!
//! It never saves a directory, site, or network grant, never removes an
//! exclusion, and never edits a repository file. Saving changes no captured
//! snapshot: the caller keeps the current-run approval either way, and a
//! [`NotSaved`] is reported as `dm.config.not_saved`.

use std::fmt;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference, PathIdentity, canonicalize_simplified};
use serde_json::Value;
use simplified_schema::PropertyPath;
use simplified_schema::yaml_documents::load_yaml_document;

use super::{PermissionPolicy, lexically_normal, physical_path, shell_rule_text};
use crate::config::load::{CONFIG_NOT_SAVED_CODE, check_permissions_text};
use crate::config::schema::{ConfigScope, permissions_contract};
use crate::config::write::{ConfigTransaction, ConfigWriteError, SourceExpectation, TargetChange, ValidationRefusal};
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::store::parse_rule_entry;
use crate::markdown::compose::shell_expansion::template::FilledChain;
use crate::markdown::compose::shell_expansion::types::{ShellRuleEntry, ShellRuleSet};
use crate::yaml_edit::YamlEdit;

/// An approval the user asked to keep.
#[derive(Debug, Clone)]
pub enum PersistentApproval {
    /// A `shell-commands` rule (`exact`, `prefix`, or `template`).
    ShellRule {
        /// The rule to save.
        rule: ShellRuleEntry,
        /// The command being approved, when known, so an exclusion that
        /// matches its filled arguments is honored too.
        command: Option<FilledChain>,
    },
    /// One file a schema slot reads, saved as an exact `~/` reference.
    FileRead {
        /// The file; a relative path is read from `reading`'s `cwd`.
        target: PathBuf,
        /// The context of the document doing the read.
        reading: Box<FileResolutionContext>,
    },
}

/// A saved (or already present) approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedApproval {
    /// The file edited: `~/.darkmatter.yaml` or its reference's target.
    pub file: PathBuf,
    /// `file-read` or `shell-commands`.
    pub list: &'static str,
    /// The entry as saved.
    pub entry: String,
    /// `false` when an identical entry was already there and nothing was
    /// written.
    pub added: bool,
    /// Whether the scope's default entries were written first because the
    /// list had been omitted.
    pub materialized_defaults: bool,
}

/// Why an approval was not saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotSavedReason {
    /// The configuration's permissions are invalid, so nothing is edited.
    InvalidPolicy,
    /// A `!` entry still matches the approved item; it is not removed.
    Excluded,
    /// The request has no home directory, so there is no user scope.
    NoHome,
    /// The file has no portable spelling (it is outside the home).
    NotPortable,
    /// The target is a directory: only single files are saved.
    NotAFile,
    /// The block's file changed shape since the configuration was captured
    /// (the block moved, or the reference target is gone).
    BlockMoved,
    /// The edit cannot be made in place, or would leave the block invalid;
    /// the file is untouched.
    InvalidEdit,
    /// Locking, reading, or replacing the file failed, or another writer
    /// changed it meanwhile.
    Write,
}

/// An approval that applies to the current run only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotSaved {
    #[allow(missing_docs)]
    pub reason: NotSavedReason,
    /// The file that would have been edited, when known.
    pub file: Option<PathBuf>,
    /// What happened, for the user.
    pub message: String,
}

impl NotSaved {
    fn new(reason: NotSavedReason, file: Option<&Path>, message: impl Into<String>) -> Self {
        Self { reason, file: file.map(Path::to_path_buf), message: message.into() }
    }

    /// `dm.config.not_saved`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        CONFIG_NOT_SAVED_CODE
    }
}

impl fmt::Display for NotSaved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the approval applies to this run only and was not saved: {}", self.message)
    }
}

impl std::error::Error for NotSaved {}

/// Saves `approval` to the user scope of `policy`'s configuration.
///
/// ## Errors
///
/// A [`NotSaved`] for every case [`NotSavedReason`] lists; the file is then
/// untouched.
pub fn save_user_approval(policy: &PermissionPolicy, approval: &PersistentApproval) -> Result<SavedApproval, NotSaved> {
    let Some(home) = policy.home() else {
        return Err(NotSaved::new(NotSavedReason::NoHome, None, "there is no home directory to hold user settings"));
    };
    let (list, entry) = match approval {
        PersistentApproval::ShellRule { rule, command } => ("shell-commands", shell_entry(policy, rule, command.as_ref())?),
        PersistentApproval::FileRead { target, reading } => ("file-read", file_entry(policy, home, target, reading)?),
    };
    let layer = policy.user();
    let file = layer.block_file();
    let whole_config = !layer.is_referenced();
    let current = match std::fs::read(file) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(NotSaved::new(NotSavedReason::Write, Some(file), format!("cannot read it: {error}"))),
    };
    let defaults = default_list(list).map_err(|message| NotSaved::new(NotSavedReason::InvalidEdit, Some(file), message))?;

    let Some(bytes) = current else {
        if !whole_config {
            return Err(NotSaved::new(
                NotSavedReason::BlockMoved,
                Some(file),
                "the file the `permissions` reference names no longer exists",
            ));
        }
        let mut items = defaults;
        push_unique(&mut items, &entry);
        let change = TargetChange::create_yaml(file, permissions_document(list, items), Vec::new());
        commit(policy, file, whole_config, change)?;
        return Ok(SavedApproval { file: file.to_path_buf(), list, entry, added: true, materialized_defaults: true });
    };

    let text = std::str::from_utf8(&bytes)
        .map_err(|_| NotSaved::new(NotSavedReason::InvalidEdit, Some(file), "it is not UTF-8"))?;
    let document = load_yaml_document(text, file)
        .map_err(|error| NotSaved::new(NotSavedReason::InvalidEdit, Some(file), error.to_string()))?
        .json;
    let block = match document.get("permissions") {
        None if whole_config => None,
        Some(Value::Object(block)) => Some(block),
        Some(Value::String(_)) if whole_config => {
            return Err(NotSaved::new(
                NotSavedReason::BlockMoved,
                Some(file),
                "its `permissions` became a reference after the configuration was read",
            ));
        }
        _ => {
            return Err(NotSaved::new(NotSavedReason::BlockMoved, Some(file), "it no longer holds a `permissions` mapping"));
        }
    };
    let path = |segments: &[&str]| {
        PropertyPath::from_segments(segments.iter().copied())
            .map_err(|error| NotSaved::new(NotSavedReason::InvalidEdit, Some(file), error.to_string()))
    };
    let (edits, added, materialized) = match block.and_then(|block| block.get(list)) {
        Some(Value::Array(items)) => {
            if items.iter().any(|item| item.as_str() == Some(entry.as_str())) {
                (Vec::new(), false, false)
            } else {
                let index = items.len().to_string();
                (vec![YamlEdit::Set { path: path(&["permissions", list, &index])?, value: Value::String(entry.clone()) }], true, false)
            }
        }
        Some(_) => {
            return Err(NotSaved::new(NotSavedReason::InvalidEdit, Some(file), format!("`permissions.{list}` is not a list")));
        }
        None => {
            let mut items = defaults;
            push_unique(&mut items, &entry);
            (vec![YamlEdit::Set { path: path(&["permissions", list])?, value: Value::Array(items) }], true, true)
        }
    };
    if added {
        let change = TargetChange::edit(file, SourceExpectation::Bytes(bytes.clone()), edits);
        commit(policy, file, whole_config, change)?;
    }
    Ok(SavedApproval { file: file.to_path_buf(), list, entry, added, materialized_defaults: materialized })
}

fn commit(policy: &PermissionPolicy, file: &Path, whole_config: bool, change: TargetChange) -> Result<(), NotSaved> {
    let mut transaction = ConfigTransaction::new();
    transaction.add(change).map_err(|error| write_error(file, &error))?;
    let context = policy.context();
    transaction
        .commit(&|candidate| {
            check_permissions_text(candidate.candidate, candidate.target, ConfigScope::User, whole_config, context)
                .map(|_| ())
                .map_err(ValidationRefusal::new)
        })
        .map_err(|error| write_error(file, &error))?;
    Ok(())
}

fn write_error(file: &Path, error: &ConfigWriteError) -> NotSaved {
    let reason = match error {
        ConfigWriteError::UnsupportedEdit { .. }
        | ConfigWriteError::ValidationFailed { .. }
        | ConfigWriteError::Layout { .. } => NotSavedReason::InvalidEdit,
        _ => NotSavedReason::Write,
    };
    NotSaved::new(reason, Some(file), error.to_string())
}

/// The user scope's default for `list`, read from the shipped schema.
fn default_list(list: &str) -> Result<Vec<Value>, String> {
    let empty = Value::Object(serde_json::Map::new());
    let success = permissions_contract(ConfigScope::User)
        .validate_strict(&empty)
        .map_err(|failure| format!("the shipped permissions schema has no usable defaults: {failure:?}"))?;
    success
        .defaults
        .into_iter()
        .find(|default| default.path.segments() == [list])
        .and_then(|default| default.value.as_array().cloned())
        .ok_or_else(|| format!("the shipped permissions schema has no default list for `{list}`"))
}

fn push_unique(items: &mut Vec<Value>, entry: &str) {
    if !items.iter().any(|item| item.as_str() == Some(entry)) {
        items.push(Value::String(entry.to_string()));
    }
}

/// A new user configuration holding only `permissions.<list>`, which the
/// transaction lays out with the file's indentation preference.
fn permissions_document(list: &str, items: Vec<Value>) -> Value {
    let mut block = serde_json::Map::new();
    block.insert(list.to_string(), Value::Array(items));
    let mut document = serde_json::Map::new();
    document.insert("permissions".to_string(), Value::Object(block));
    Value::Object(document)
}

/// The rule's text, unless a `!` rule of any layer still matches it.
fn shell_entry(policy: &PermissionPolicy, rule: &ShellRuleEntry, command: Option<&FilledChain>) -> Result<String, NotSaved> {
    let entry = shell_rule_text(rule);
    if parse_rule_entry(&entry).as_ref() != Ok(rule) {
        return Err(NotSaved::new(NotSavedReason::InvalidEdit, None, format!("`{entry}` would not read back as the approved rule")));
    }
    let verdict = policy.shell().verdict(|_| true, |excluded| excludes_rule(excluded, rule, command));
    if let super::PermissionVerdict::ApprovalRequired { exclusion: Some(exclusion) } = verdict {
        return Err(NotSaved::new(
            NotSavedReason::Excluded,
            Some(&exclusion.file),
            format!("`{}` still excludes `{entry}`; the exclusion was left in place", exclusion.entry),
        ));
    }
    Ok(entry)
}

/// Whether `excluded` matches the rule being approved or its command.
fn excludes_rule(excluded: &ShellRuleSet, rule: &ShellRuleEntry, command: Option<&FilledChain>) -> bool {
    let by_rule = match rule {
        ShellRuleEntry::Exact(normalized) => {
            excluded.matches_exact(normalized)
                || normalized.split_whitespace().next().is_some_and(|program| excluded.matches_prefix(program))
        }
        ShellRuleEntry::Prefix(executable) => excluded.matches_prefix(executable),
        ShellRuleEntry::Template(shape) => {
            shape.segments.iter().any(|segment| {
                let literal: Option<Vec<String>> =
                    segment.args.iter().map(|arg| arg.as_literal().map(str::to_string)).collect();
                excluded.matches_prefix(&segment.program)
                    || literal.is_some_and(|args| excluded.matches_exact(&normalize_command(&segment.program, &args)))
            }) || excluded.template_blocks_shape(shape)
        }
    };
    by_rule || command.is_some_and(|command| excluded.blocks_filled(command))
}

/// The exact, portable `file-read` entry for `target`.
fn file_entry(
    policy: &PermissionPolicy,
    home: &Path,
    target: &Path,
    reading: &FileResolutionContext,
) -> Result<String, NotSaved> {
    let lexical = lexically_normal(&if target.is_absolute() { target.to_path_buf() } else { reading.cwd().join(target) });
    if physical_path(&lexical).is_dir() {
        return Err(NotSaved::new(
            NotSavedReason::NotAFile,
            None,
            format!("`{}` is a directory; only single files are saved", lexical.display()),
        ));
    }
    if let super::PermissionVerdict::ApprovalRequired { exclusion: Some(exclusion) } = policy.file_read(&lexical, reading) {
        return Err(NotSaved::new(
            NotSavedReason::Excluded,
            Some(&exclusion.file),
            format!("`{}` still excludes `{}`; the exclusion was left in place", exclusion.entry, lexical.display()),
        ));
    }
    let under = |path: &Path, home: &Path| -> Option<Vec<String>> {
        let names = PathIdentity::new(path).strip_prefix(&PathIdentity::new(home))?.to_vec();
        names.iter().map(|name| name.to_str().map(GlobReference::escape)).collect::<Option<Vec<_>>>().filter(|names| !names.is_empty())
    };
    let canonical_home = canonicalize_simplified(home).ok();
    let names = under(&lexical, home)
        .or_else(|| canonical_home.as_deref().and_then(|canonical| under(&physical_path(&lexical), canonical)));
    match names {
        Some(names) => Ok(format!("~/{}", names.join("/"))),
        None => Err(NotSaved::new(
            NotSavedReason::NotPortable,
            None,
            format!("`{}` is outside the home directory, and absolute paths are not portable", lexical.display()),
        )),
    }
}
