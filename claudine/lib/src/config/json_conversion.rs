//! Temporary: converting a legacy `config.json` (JSON5) to `config.yaml`.
//!
//! Remove this module, its tests, and its two callers in
//! `dispatch/loader.rs` a few releases after configuration moved to YAML.
//!
//! [`prepare_scope`] runs before a configuration file of one scope is read,
//! and only for the two canonical locations (`~/.claudine/` and
//! `{repo-root}/.claudine/`); an explicitly supplied file is read as YAML.
//!
//! | Files present | Result |
//! |---|---|
//! | `config.json` only | written to `config.yaml` (same values, with flat actions and routes rewritten keyed), then `config.json` renamed to `config.json.bak`; one notice names both and says comments were not kept |
//! | `config.yaml` only | used |
//! | both | `config.yaml` is used; a warning says `config.json` is ignored |
//! | a pre-2026 per-provider `config.json` | renamed to `.bak` and ignored |
//!
//! The YAML is published through Darkmatter's locked transaction with a
//! migration receipt, so a crash between publishing it and renaming the JSON
//! is finished by the next load ([`complete_cleanup`]). A conversion never
//! overwrites an existing `config.json.bak`: an identical one lets the
//! unchanged source be removed, a different one stops with an error that
//! keeps both. Every failure keeps `config.json` and any `config.yaml`
//! untouched.
//!
//! User-facing guide: `claudine/docs/topics/configuration.md`.

use std::path::{Path, PathBuf};

use biscuit_file::serde_yaml_ng;
use darkmatter::config::load::CONFIG_MIGRATION_CODE;
use darkmatter::config::migration_receipt::{CleanupOutcome, MigrationSource, complete_cleanup, receipt_path};
use darkmatter::config::write::{ConfigTransaction, TargetChange, ValidationRefusal};
use serde_json::Value;
use simplified_schema::yaml_documents::load_yaml_document;

use super::load::{ConfigProblem, line_column};
use super::migration;
use super::schema::ConfigScope;
use crate::error::{ClaudineError, Result};

/// The legacy file name in a configuration directory.
pub const LEGACY_FILE_NAME: &str = "config.json";

/// Where a converted legacy file is moved.
pub const BACKUP_FILE_NAME: &str = "config.json.bak";

/// How much a [`ConfigNotice`] matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    /// Something was done for the user (a conversion).
    Notice,
    /// Something the user should fix (an ignored legacy file).
    Warning,
}

/// A conversion outcome a caller reports once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigNotice {
    /// `dm.config.migration`.
    pub code: &'static str,
    #[allow(missing_docs)]
    pub level: NoticeLevel,
    /// The files concerned, legacy file first.
    pub files: Vec<PathBuf>,
    /// What happened, for a reader.
    pub message: String,
}

/// What [`prepare_scope`] left for the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScopeState {
    /// Read `config.yaml` if it exists.
    Ready,
    /// The legacy file was in the pre-2026 per-provider format: it was
    /// renamed to `.bak` and there is nothing to read.
    LegacyFormatIgnored,
}

/// Converts or reports the legacy file beside the canonical `yaml_path`,
/// first finishing a conversion an earlier process was interrupted in.
///
/// ## Errors
///
/// [`ClaudineError::ConfigConversion`] naming the file concerned when the
/// legacy file cannot be converted or a pending conversion cannot be
/// finished; every file is kept.
pub(crate) fn prepare_scope(yaml_path: &Path, scope: ConfigScope, notices: &mut Vec<ConfigNotice>) -> Result<ScopeState> {
    let Some(dir) = yaml_path.parent().filter(|dir| dir.is_dir()) else {
        return Ok(ScopeState::Ready);
    };
    let json_path = dir.join(LEGACY_FILE_NAME);
    let backup_path = dir.join(BACKUP_FILE_NAME);
    let yaml_exists = std::fs::symlink_metadata(yaml_path).is_ok();

    if yaml_exists && receipt_pending(yaml_path) {
        match complete_cleanup(yaml_path, &[json_path.clone(), backup_path.clone()]) {
            Ok(CleanupOutcome::Completed { deleted, backed_up, .. }) if !deleted.is_empty() || !backed_up.is_empty() => {
                notices.push(converted_notice(&json_path, yaml_path, &backup_path, !backed_up.is_empty()));
            }
            Ok(_) => {}
            Err(error) => {
                return Err(conversion_error(vec![ConfigProblem {
                    file: Some(json_path),
                    pointer: error.pointer.clone(),
                    line: None,
                    column: None,
                    message: format!(
                        "an interrupted conversion to {} cannot be finished: {error}. Remove {} once {} holds your settings",
                        yaml_path.display(),
                        error.receipt.display(),
                        yaml_path.display(),
                    ),
                }]));
            }
        }
    }

    let bytes = match std::fs::read(&json_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(ScopeState::Ready),
        Err(error) => {
            return Err(conversion_error(vec![ConfigProblem::whole(Some(&json_path), format!("cannot be read: {error}"))]));
        }
    };

    if yaml_exists {
        if parse_legacy(&bytes, &json_path).is_ok_and(|value| migration::is_old_format(&value)) {
            migration::backup_old_config(&json_path)?;
            return Ok(ScopeState::Ready);
        }
        notices.push(ConfigNotice {
            code: CONFIG_MIGRATION_CODE,
            level: NoticeLevel::Warning,
            files: vec![json_path.clone(), yaml_path.to_path_buf()],
            message: format!(
                "{} is ignored because {} exists; delete {} once {} holds your settings",
                json_path.display(),
                yaml_path.display(),
                json_path.display(),
                yaml_path.display(),
            ),
        });
        return Ok(ScopeState::Ready);
    }

    let mut value = parse_legacy(&bytes, &json_path).map_err(conversion_error)?;
    if migration::is_old_format(&value) {
        migration::backup_old_config(&json_path)?;
        return Ok(ScopeState::LegacyFormatIgnored);
    }
    // A legacy file spells actions and routes flat (`type: bash`); the YAML
    // file is written, and judged, in the keyed form.
    super::wire::upgrade_flat(&mut value);
    convert(&value, &bytes, &json_path, yaml_path, &backup_path, scope, notices)
}

/// Whether a migration receipt waits beside `yaml_path`.
fn receipt_pending(yaml_path: &Path) -> bool {
    receipt_path(yaml_path).is_ok_and(|receipt| std::fs::symlink_metadata(receipt).is_ok())
}

/// Reads a legacy file: UTF-8 JSON5 without duplicate keys or numbers the
/// numeric model refuses.
fn parse_legacy(bytes: &[u8], path: &Path) -> std::result::Result<Value, Vec<ConfigProblem>> {
    let at = |text: &str, offset: usize, pointer: String, message: String| {
        let (line, column) = line_column(text, offset);
        vec![ConfigProblem { file: Some(path.to_path_buf()), pointer, line: Some(line), column: Some(column), message }]
    };
    let text = std::str::from_utf8(bytes).map_err(|error| {
        at(&String::from_utf8_lossy(bytes), error.valid_up_to(), String::new(), "the file is not valid UTF-8".into())
    })?;
    let parsed = biscuit_file::Json5::from_str(text)
        .map_err(|error| vec![ConfigProblem::whole(Some(path), format!("not valid JSON5: {error}"))])?;
    let scan = biscuit_file::json5_scan(text)
        .map_err(|error| vec![ConfigProblem::whole(Some(path), format!("not valid JSON5: {error}"))])?;
    if let Some(duplicate) = scan.duplicate_key {
        let pointer = format!("{}/{}", duplicate.object_pointer, duplicate.key.replace('~', "~0").replace('/', "~1"));
        return Err(at(text, duplicate.offset, pointer, format!("the key `{}` is written more than once", duplicate.key)));
    }
    simplified_schema::numeric::admit_json5_text_numbers(text).map_err(|error| {
        vec![ConfigProblem {
            file: Some(path.to_path_buf()),
            pointer: error.pointer.clone(),
            line: Some(error.line),
            column: Some(error.column),
            message: error.to_string(),
        }]
    })?;
    Ok(parsed.as_json_value().clone())
}

fn convert(
    value: &Value,
    bytes: &[u8],
    json_path: &Path,
    yaml_path: &Path,
    backup_path: &Path,
    scope: ConfigScope,
    notices: &mut Vec<ConfigNotice>,
) -> Result<ScopeState> {
    super::load::project_value(scope, value).map_err(|error| match error {
        ClaudineError::ConfigInvalid { problems } => conversion_error(
            problems.into_iter().map(|problem| ConfigProblem { file: Some(json_path.to_path_buf()), ..problem }).collect(),
        ),
        other => other,
    })?;
    let yaml = render(value).map_err(|message| conversion_error(vec![ConfigProblem::whole(Some(json_path), message)]))?;

    let source = match std::fs::read(backup_path) {
        Ok(existing) if existing == bytes => MigrationSource::delete(json_path, bytes),
        Ok(_) => {
            return Err(conversion_error(vec![ConfigProblem::whole(
                Some(backup_path),
                format!(
                    "{} cannot be converted because {} already exists with other content; both are kept. Move {} away and run again",
                    json_path.display(),
                    backup_path.display(),
                    backup_path.display(),
                ),
            )]));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => MigrationSource::backup(json_path, bytes, backup_path),
        Err(error) => {
            return Err(conversion_error(vec![ConfigProblem::whole(Some(backup_path), format!("cannot be read: {error}"))]));
        }
    };
    let removed_duplicate = matches!(source.action, darkmatter::config::migration_receipt::MigrationAction::Delete);

    let mut transaction = ConfigTransaction::new();
    let published = match transaction.add(TargetChange::create(yaml_path, yaml.clone(), Vec::new()).with_migration(vec![source])) {
        Ok(_) => transaction.commit(&|candidate| {
            if candidate.candidate == yaml {
                Ok(())
            } else {
                Err(ValidationRefusal::new("the converted text changed while it was being written"))
            }
        }),
        Err(error) => Err(error),
    };
    if let Err(error) = published {
        if std::fs::symlink_metadata(yaml_path).is_ok() {
            // Another process published `config.yaml` first; its conversion
            // finishes the cleanup.
            return Ok(ScopeState::Ready);
        }
        return Err(conversion_error(vec![ConfigProblem::whole(
            Some(json_path),
            format!("{} could not be written; nothing was changed: {error}", yaml_path.display()),
        )]));
    }

    let written = std::fs::read_to_string(yaml_path).map_err(ClaudineError::Io)?;
    let reread = load_yaml_document(&written, yaml_path).ok().map(|loaded| loaded.json);
    if reread.as_ref() != Some(value) {
        return Err(conversion_error(vec![ConfigProblem::whole(
            Some(yaml_path),
            format!("does not read back as {}; {} is kept", json_path.display(), json_path.display()),
        )]));
    }
    complete_cleanup(yaml_path, &[json_path.to_path_buf(), backup_path.to_path_buf()]).map_err(|error| {
        conversion_error(vec![ConfigProblem {
            file: Some(json_path.to_path_buf()),
            pointer: error.pointer.clone(),
            line: None,
            column: None,
            message: format!("{} was written but {} could not be moved: {error}", yaml_path.display(), json_path.display()),
        }])
    })?;
    notices.push(converted_notice(json_path, yaml_path, backup_path, !removed_duplicate));
    Ok(ScopeState::Ready)
}

/// The YAML text of `value`, checked to read back as the same value.
fn render(value: &Value) -> std::result::Result<String, String> {
    let text = serde_yaml_ng::to_string(value).expect("a JSON value always serializes as YAML");
    match load_yaml_document(&text, Path::new(super::schema::CONFIG_FILE_NAME)) {
        Ok(loaded) if &loaded.json == value => Ok(text),
        _ => Err("holds a value that does not survive conversion to YAML".to_string()),
    }
}

fn converted_notice(json_path: &Path, yaml_path: &Path, backup_path: &Path, moved: bool) -> ConfigNotice {
    let kept = if moved {
        format!("{} was renamed to {}", json_path.display(), backup_path.display())
    } else {
        format!("{} was removed because an identical {} already exists", json_path.display(), backup_path.display())
    };
    ConfigNotice {
        code: CONFIG_MIGRATION_CODE,
        level: NoticeLevel::Notice,
        files: vec![json_path.to_path_buf(), yaml_path.to_path_buf()],
        message: format!(
            "converted {} to {}; {kept}. JSON5 comments were not carried over",
            json_path.display(),
            yaml_path.display(),
        ),
    }
}

fn conversion_error(problems: Vec<ConfigProblem>) -> ClaudineError {
    ClaudineError::ConfigConversion { problems }
}
