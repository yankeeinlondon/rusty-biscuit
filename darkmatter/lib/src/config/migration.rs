//! The temporary migration of legacy policy files into `.darkmatter.yaml`.
//!
//! Before the configuration file existed, three line-based files held the
//! policies `permissions` now holds:
//!
//! | Legacy file | Scope | Becomes |
//! | --- | --- | --- |
//! | `.darkmatter-shell-whitelist` | home, repository root | `permissions.shell-commands` entries |
//! | `.darkmatter-shell-blacklist` | home, repository root | `!` entries in `permissions.shell-commands` |
//! | `~/.config/darkmatter/schema-read-policy` | home | `permissions.file-read` entries |
//!
//! [`migrate`] runs at the start of a [`ConfigLoadMode::MigrateLegacy`] load.
//! A legacy file is converted only into a property its scope's
//! configuration does not write; an authored property, even `[]`, is never
//! overwritten and its legacy file is left in place with a notice. The new
//! entries are saved through [`ConfigTransaction`] with a migration receipt,
//! re-read, and only then are the legacy files deleted, through
//! [`complete_cleanup`]; a later load finishes a cleanup a crash interrupted.
//! A [`ConfigLoadMode::Passive`] load never writes or deletes anything;
//! [`passive_notices`] tells its caller what a writing CLI would migrate.
//!
//! Files in a document's own directory (Claudine's old policy fallback) are
//! never read here: only the exact paths above are.
//!
//! This module, its notices, and [`ConfigLoadMode::MigrateLegacy`]'s
//! conversion are removed after a few releases.
//!
//! User-facing guide: `darkmatter/docs/topics/configuration.md`.
//!
//! [`ConfigLoadMode::MigrateLegacy`]: super::load::ConfigLoadMode::MigrateLegacy
//! [`ConfigLoadMode::Passive`]: super::load::ConfigLoadMode::Passive

use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference, canonicalize_simplified};
use serde_json::{Map, Value};
use simplified_schema::PropertyPath;
use simplified_schema::yaml_documents::load_yaml_document;

use super::load::{
    CONFIG_MIGRATION_CODE, ConfigDiagnostic, ConfigError, ConfigErrorKind, ConfigProblem, ConfigRevision,
    ConfigSnapshot,
};
use super::migration_receipt::{
    CleanupOutcome, MigrationErrorKind, MigrationSource, RECEIPT_SUFFIX, complete_cleanup, receipt_path,
};
use super::schema::{CONFIG_FILE_NAME, ConfigScope, contract};
use super::write::{ConfigTransaction, SourceExpectation, TargetChange, ValidationRefusal};
use crate::markdown::compose::shell_expansion::store::parse_rule_entry;
use crate::markdown::schemas::slots::grants::{GRANT_STORE_RELATIVE_PATH, GrantTarget};
use crate::yaml_edit::YamlEdit;

/// The legacy shell allow list's file name, in the home and at a repository
/// root.
pub const LEGACY_WHITELIST_FILE_NAME: &str = ".darkmatter-shell-whitelist";

/// The legacy user-editable shell deny list's file name, in the home and at a
/// repository root.
pub const LEGACY_BLACKLIST_FILE_NAME: &str = ".darkmatter-shell-blacklist";

const SHELL_COMMANDS: &str = "shell-commands";
const FILE_READ: &str = "file-read";

/// One scope's configuration file and the legacy files that feed it.
struct LegacyScope {
    scope: ConfigScope,
    revision: ConfigRevision,
    target: PathBuf,
    whitelist: PathBuf,
    blacklist: PathBuf,
    /// The schema-slot grant store; user scope only.
    grant_store: Option<PathBuf>,
    /// The home a grant store's paths are made portable against.
    home: Option<PathBuf>,
}

impl LegacyScope {
    fn user(home: &Path) -> Self {
        let mut grant_store = home.to_path_buf();
        grant_store.extend(GRANT_STORE_RELATIVE_PATH);
        Self {
            scope: ConfigScope::User,
            revision: ConfigRevision::User,
            target: home.join(CONFIG_FILE_NAME),
            whitelist: home.join(LEGACY_WHITELIST_FILE_NAME),
            blacklist: home.join(LEGACY_BLACKLIST_FILE_NAME),
            grant_store: Some(grant_store),
            home: Some(home.to_path_buf()),
        }
    }

    fn repository(root: &Path) -> Self {
        Self {
            scope: ConfigScope::Repository,
            revision: ConfigRevision::RepositoryWorktree,
            target: root.join(CONFIG_FILE_NAME),
            whitelist: root.join(LEGACY_WHITELIST_FILE_NAME),
            blacklist: root.join(LEGACY_BLACKLIST_FILE_NAME),
            grant_store: None,
            home: None,
        }
    }

    /// The scopes `context` names: the user scope when it has an absolute
    /// home, the repository scope when it names a repository.
    fn of(context: &FileResolutionContext) -> Vec<Self> {
        let user = context.home_dir().filter(|home| home.is_absolute()).map(Self::user);
        let repository = context.repository_root().filter(|root| root.is_absolute()).map(Self::repository);
        user.into_iter().chain(repository).collect()
    }

    /// Every legacy file this scope may clean up: the only paths a receipt
    /// is allowed to name.
    fn known(&self) -> Vec<PathBuf> {
        [Some(&self.whitelist), Some(&self.blacklist), self.grant_store.as_ref()]
            .into_iter()
            .flatten()
            .cloned()
            .collect()
    }

    fn notice(&self, file: &Path, message: String) -> ConfigDiagnostic {
        ConfigDiagnostic { code: CONFIG_MIGRATION_CODE, file: file.to_path_buf(), message, problems: Vec::new() }
    }

    fn line_notice(&self, file: &Path, line: usize, message: String) -> ConfigDiagnostic {
        let problem = self.line_problem(file, line, message.clone());
        ConfigDiagnostic { code: CONFIG_MIGRATION_CODE, file: file.to_path_buf(), message, problems: vec![problem] }
    }

    fn line_problem(&self, file: &Path, line: usize, message: String) -> ConfigProblem {
        ConfigProblem {
            file: file.to_path_buf(),
            revision: self.revision,
            pointer: String::new(),
            line: Some(line),
            column: Some(1),
            message,
            reference_target: None,
        }
    }

    fn error(&self, file: &Path, message: String) -> ConfigError {
        ConfigError::unlocated(ConfigErrorKind::Migration, file, self.revision, message)
    }
}

/// A legacy file that exists, with the bytes the migration read.
struct Source {
    path: PathBuf,
    bytes: Vec<u8>,
}

/// Converts every legacy policy file of the scopes `context` names, then
/// cleans them up; returns the `dm.config.migration` notices to report.
///
/// ## Errors
///
/// A [`ConfigErrorKind::Migration`] error, with nothing written, when a
/// legacy file cannot be read or has a line its grammar rejects (located at
/// that line), or when a pending migration receipt is unreadable, malformed,
/// or names a file this scope does not own.
pub(crate) fn migrate(context: &FileResolutionContext) -> Result<Vec<ConfigDiagnostic>, ConfigError> {
    let mut notices = Vec::new();
    for scope in LegacyScope::of(context) {
        migrate_scope(&scope, &mut notices)?;
    }
    Ok(notices)
}

/// [`migrate`] for the repository scope `context` names alone: what a
/// writing CLI runs when it projects its once-loaded user configuration onto
/// a source's repository ([`ConfigSnapshot::with_repository_projection`]).
///
/// ## Errors
///
/// As [`migrate`].
pub(crate) fn migrate_repository(context: &FileResolutionContext) -> Result<Vec<ConfigDiagnostic>, ConfigError> {
    let mut notices = Vec::new();
    for scope in LegacyScope::of(context).into_iter().filter(|scope| scope.scope == ConfigScope::Repository) {
        migrate_scope(&scope, &mut notices)?;
    }
    Ok(notices)
}

/// What a passive load reports instead of migrating: one notice per legacy
/// file a writing CLI would convert (its property is missing from the
/// configuration `snapshot` loaded).
pub(crate) fn passive_notices(context: &FileResolutionContext, snapshot: &ConfigSnapshot) -> Vec<ConfigDiagnostic> {
    let mut notices = Vec::new();
    for scope in LegacyScope::of(context) {
        let authored = match scope.scope {
            ConfigScope::User => snapshot.user().authored(),
            ConfigScope::Repository => {
                snapshot.repository().and_then(|repository| repository.worktree.authored())
            }
        };
        let block = match authored.and_then(|document| document.get("permissions")) {
            None => None,
            Some(Value::Object(block)) => Some(block),
            // A reference or an invalid value is never migrated into.
            Some(_) => continue,
        };
        let missing = |property: &str| block.is_none_or(|block| !block.contains_key(property));
        let candidates = [
            (Some(&scope.whitelist), SHELL_COMMANDS),
            (Some(&scope.blacklist), SHELL_COMMANDS),
            (scope.grant_store.as_ref(), FILE_READ),
        ];
        for (path, property) in candidates {
            if let Some(path) = path
                && missing(property)
                && std::fs::symlink_metadata(path).is_ok()
            {
                notices.push(scope.notice(
                    path,
                    format!(
                        "`{}` is a legacy policy file and is not in use; the next `md` or Claudine run migrates it into `permissions.{property}` of `{}`",
                        path.display(),
                        scope.target.display()
                    ),
                ));
            }
        }
    }
    notices
}

fn migrate_scope(scope: &LegacyScope, notices: &mut Vec<ConfigDiagnostic>) -> Result<(), ConfigError> {
    let known = scope.known();
    // The receipt sits beside the resolved target (a symlinked
    // configuration's real file).
    let receipt = receipt_path(&scope.target).unwrap_or_else(|_| receipt_file(&scope.target));
    let mut stale_receipt = false;
    if std::fs::symlink_metadata(&scope.target).is_ok() && std::fs::symlink_metadata(&receipt).is_ok() {
        match complete_cleanup(&scope.target, &known) {
            Ok(CleanupOutcome::Completed { deleted, .. }) => {
                for path in deleted {
                    notices.push(scope.notice(
                        &path,
                        format!(
                            "finished an interrupted migration: `{}` was already migrated into `{}` and is now deleted",
                            path.display(),
                            scope.target.display()
                        ),
                    ));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind == MigrationErrorKind::SourceChanged => {
                notices.push(scope.notice(
                    &receipt,
                    format!(
                        "an earlier migration into `{}` is not cleaned up: {error}. Copy any new rules into `{}` by hand, then delete the legacy file",
                        scope.target.display(),
                        scope.target.display()
                    ),
                ));
                return Ok(());
            }
            // The receipt's target was never published (a failed save) or
            // has changed since: retry below; a new save replaces it.
            Err(error) if error.kind == MigrationErrorKind::Unpublished => stale_receipt = true,
            Err(error) => {
                return Err(scope.error(
                    &receipt,
                    format!(
                        "a pending migration into `{}` cannot be finished: {error}. Delete `{}` once `{}` holds your settings",
                        scope.target.display(),
                        receipt.display(),
                        scope.target.display()
                    ),
                ));
            }
        }
    }

    let whitelist = read_source(scope, &scope.whitelist)?;
    let blacklist = read_source(scope, &scope.blacklist)?;
    let grant_store = match &scope.grant_store {
        Some(path) => read_source(scope, path)?,
        None => None,
    };
    if whitelist.is_none() && blacklist.is_none() && grant_store.is_none() {
        if stale_receipt {
            notices.push(stale_receipt_notice(scope, &receipt));
        }
        return Ok(());
    }

    let target_bytes = match std::fs::read(&scope.target) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(scope.error(&scope.target, format!("cannot be read: {error}"))),
    };
    let document = match &target_bytes {
        None => Map::new(),
        // A configuration that does not load is the load's error to report;
        // nothing is migrated into it.
        Some(bytes) => match parse_document(bytes, &scope.target) {
            Some(document) => document,
            None => return Ok(()),
        },
    };
    let block = match document.get("permissions") {
        None => None,
        Some(Value::Object(block)) => Some(block),
        Some(Value::String(reference)) => {
            for source in [&whitelist, &blacklist, &grant_store].into_iter().flatten() {
                notices.push(scope.notice(
                    &source.path,
                    format!(
                        "`{}` is not migrated because `permissions` in `{}` refers to `{reference}`; copy its rules there by hand, then delete it",
                        source.path.display(),
                        scope.target.display()
                    ),
                ));
            }
            return Ok(());
        }
        Some(_) => return Ok(()),
    };
    let authored = |property: &str| block.is_some_and(|block| block.contains_key(property));

    let mut edits = Vec::new();
    let mut sources = Vec::new();
    let mut migrated: Vec<(PathBuf, &str)> = Vec::new();
    let mut line_notices = Vec::new();
    let mut planned = Map::new();

    let shell_sources: Vec<&Source> = [&whitelist, &blacklist].into_iter().flatten().collect();
    if !shell_sources.is_empty() {
        if authored(SHELL_COMMANDS) {
            for source in &shell_sources {
                notices.push(ignored_notice(scope, source, SHELL_COMMANDS));
            }
        } else {
            let mut entries = Vec::new();
            let mut problems = Vec::new();
            if let Some(source) = &whitelist {
                entries.extend(shell_entries(scope, source, "", &mut problems, &mut Vec::new()));
            }
            if let Some(source) = &blacklist {
                entries.extend(shell_entries(scope, source, "!", &mut problems, &mut line_notices));
            }
            if !problems.is_empty() {
                return Err(ConfigError::new(ConfigErrorKind::Migration, problems));
            }
            let value = Value::Array(entries.into_iter().map(Value::String).collect());
            edits.push(set(SHELL_COMMANDS, value.clone()));
            planned.insert(SHELL_COMMANDS.to_string(), value);
            for source in shell_sources {
                sources.push(MigrationSource::delete(&source.path, &source.bytes));
                migrated.push((source.path.clone(), SHELL_COMMANDS));
            }
        }
    }

    if let (Some(source), Some(home)) = (&grant_store, &scope.home) {
        if authored(FILE_READ) {
            notices.push(ignored_notice(scope, source, FILE_READ));
        } else {
            let mut entries = schema_default_file_read(scope.scope);
            let converted = grant_entries(scope, source, home, &mut line_notices)?;
            for entry in converted {
                if !entries.contains(&entry) {
                    entries.push(entry);
                }
            }
            let value = Value::Array(entries.into_iter().map(Value::String).collect());
            edits.push(set(FILE_READ, value.clone()));
            planned.insert(FILE_READ.to_string(), value);
            sources.push(MigrationSource::delete(&source.path, &source.bytes));
            migrated.push((source.path.clone(), FILE_READ));
        }
    }

    if edits.is_empty() {
        if stale_receipt {
            notices.push(stale_receipt_notice(scope, &receipt));
        }
        return Ok(());
    }

    let change = match target_bytes {
        Some(bytes) => TargetChange::edit(&scope.target, SourceExpectation::Bytes(bytes), edits),
        None => TargetChange::create(&scope.target, "{}\n", edits),
    }
    .with_migration(sources);
    let config_scope = scope.scope;
    let mut transaction = ConfigTransaction::new();
    let saved = transaction.add(change).map(|_| ()).and_then(|()| {
        transaction.commit(&|candidate| {
            let value = load_yaml_document(candidate.candidate, candidate.target)
                .map_err(|error| ValidationRefusal::new(error.to_string()))?
                .json;
            contract(config_scope)
                .validate_strict(&value)
                .map(|_| ())
                .map_err(|failure| ValidationRefusal::new(failure_text(&failure)))
        })
    });
    if let Err(error) = saved {
        notices.push(scope.notice(
            &scope.target,
            format!("the legacy policy files could not be migrated, so they are kept and the migration runs again next time: {error}"),
        ));
        return Ok(());
    }

    // Read the saved file back before any source is deleted.
    let reread = std::fs::read(&scope.target).ok().and_then(|bytes| parse_document(&bytes, &scope.target));
    let saved_block = reread.as_ref().and_then(|document| document.get("permissions")).and_then(Value::as_object);
    if planned.iter().any(|(property, value)| saved_block.and_then(|block| block.get(property)) != Some(value)) {
        notices.push(scope.notice(
            &scope.target,
            format!(
                "`{}` does not read back with the migrated entries, so the legacy policy files are kept",
                scope.target.display()
            ),
        ));
        return Ok(());
    }

    let deleted = match complete_cleanup(&scope.target, &known) {
        Ok(CleanupOutcome::Completed { deleted, .. }) => deleted,
        Ok(_) => Vec::new(),
        Err(error) => {
            notices.push(scope.notice(
                &receipt,
                format!("the migrated legacy policy files were kept: {error}"),
            ));
            Vec::new()
        }
    };
    let repository_note = if scope.scope == ConfigScope::Repository {
        "; repository entries take effect once the file is committed"
    } else {
        ""
    };
    for (path, property) in migrated {
        let outcome = if deleted.contains(&path) { "and deleted it" } else { "and kept it" };
        notices.push(scope.notice(
            &path,
            format!(
                "migrated `{}` into `permissions.{property}` of `{}` {outcome}{repository_note}",
                path.display(),
                scope.target.display()
            ),
        ));
    }
    notices.extend(line_notices);
    Ok(())
}

fn receipt_file(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(RECEIPT_SUFFIX);
    target.with_file_name(name)
}

fn stale_receipt_notice(scope: &LegacyScope, receipt: &Path) -> ConfigDiagnostic {
    scope.notice(
        receipt,
        format!(
            "`{}` records a migration `{}` no longer holds; delete it once `{}` holds your settings",
            receipt.display(),
            scope.target.display(),
            scope.target.display()
        ),
    )
}

fn ignored_notice(scope: &LegacyScope, source: &Source, property: &str) -> ConfigDiagnostic {
    scope.notice(
        &source.path,
        format!(
            "`{}` is ignored because `{}` already sets `permissions.{property}`; delete it once its rules are there",
            source.path.display(),
            scope.target.display()
        ),
    )
}

fn read_source(scope: &LegacyScope, path: &Path) -> Result<Option<Source>, ConfigError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(Source { path: path.to_path_buf(), bytes })),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(scope.error(path, format!("the legacy policy file cannot be read: {error}"))),
    }
}

/// The configuration's root mapping, or `None` when it does not load (the
/// load reports why).
fn parse_document(bytes: &[u8], path: &Path) -> Option<Map<String, Value>> {
    let text = std::str::from_utf8(bytes).ok()?;
    match load_yaml_document(text, path).ok()?.json {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn failure_text(failure: &simplified_schema::ValidationFailure) -> String {
    use simplified_schema::ValidationFailure;
    match failure {
        ValidationFailure::InvalidData { problems } | ValidationFailure::UnavailableCheck { problems } => problems
            .iter()
            .map(|problem| format!("`/{}`: {}", problem.instance_path.segments().join("/"), problem.message))
            .collect::<Vec<_>>()
            .join("; "),
        ValidationFailure::Unprepared { error } => error.to_string(),
        other => format!("{other:?}"),
    }
}

fn set(property: &str, value: Value) -> YamlEdit {
    let path = PropertyPath::parse(&format!("permissions.{property}")).expect("a fixed property path parses");
    YamlEdit::Set { path, value }
}

/// The lines of a legacy text file, numbered from 1, without blank lines and
/// `#` comments; `Err` holds the problem for a file that is not UTF-8.
fn rule_lines<'a>(
    scope: &LegacyScope,
    source: &'a Source,
) -> Result<Vec<(usize, &'a str)>, Box<ConfigProblem>> {
    let bytes = source.bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&source.bytes);
    let text = std::str::from_utf8(bytes).map_err(|error| {
        let line = bytes[..error.valid_up_to()].iter().filter(|byte| **byte == b'\n').count() + 1;
        Box::new(scope.line_problem(&source.path, line, "the legacy policy file is not valid UTF-8".to_string()))
    })?;
    Ok(text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .collect())
}

/// A whitelist's or blacklist's rules as `shell-commands` entries (each
/// prefixed by `marker`), once each. A rule the grammar rejects is added to
/// `problems`; each blacklist rule adds a notice that it now prompts.
fn shell_entries(
    scope: &LegacyScope,
    source: &Source,
    marker: &str,
    problems: &mut Vec<ConfigProblem>,
    notices: &mut Vec<ConfigDiagnostic>,
) -> Vec<String> {
    let lines = match rule_lines(scope, source) {
        Ok(lines) => lines,
        Err(problem) => {
            problems.push(*problem);
            return Vec::new();
        }
    };
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    for (line, rule) in lines {
        match parse_rule_entry(rule) {
            Ok(parsed) => {
                if seen.insert(parsed) {
                    let entry = format!("{marker}{rule}");
                    if !marker.is_empty() {
                        notices.push(scope.line_notice(
                            &source.path,
                            line,
                            format!(
                                "the blacklist rule `{rule}` is migrated as `{entry}`: a matching command now prompts instead of being refused"
                            ),
                        ));
                    }
                    entries.push(entry);
                }
            }
            Err(reason) => problems.push(scope.line_problem(
                &source.path,
                line,
                format!("`{rule}` is not a shell rule ({reason}); nothing was migrated"),
            )),
        }
    }
    entries
}

/// The grant store's `file`/`dir` lines under `home` as `file-read` entries;
/// every other line is dropped with a notice.
fn grant_entries(
    scope: &LegacyScope,
    source: &Source,
    home: &Path,
    notices: &mut Vec<ConfigDiagnostic>,
) -> Result<Vec<String>, ConfigError> {
    let lines = rule_lines(scope, source).map_err(|problem| ConfigError::new(ConfigErrorKind::Migration, vec![*problem]))?;
    let mut problems = Vec::new();
    let mut converted = Vec::new();
    let mut dropped = Vec::new();
    for (line, text) in lines {
        let (verb, rest) = split_word(text);
        let exclude = match verb {
            "allow" => false,
            "deny" => true,
            other => {
                problems.push(scope.line_problem(
                    &source.path,
                    line,
                    format!("unknown verb `{other}`; expected `allow` or `deny`; nothing was migrated"),
                ));
                continue;
            }
        };
        let (kind, argument) = split_word(rest);
        if kind.is_empty() || argument.is_empty() {
            problems.push(scope.line_problem(
                &source.path,
                line,
                format!("`{text}` needs a kind and a target; nothing was migrated"),
            ));
            continue;
        }
        let target = match GrantTarget::parse(kind, argument) {
            Ok(target) => target,
            Err(reason) => {
                problems.push(scope.line_problem(&source.path, line, format!("{reason}; nothing was migrated")));
                continue;
            }
        };
        let entry = match &target {
            GrantTarget::File(path) => home_pattern(path, home).filter(|relative| !relative.is_empty()),
            GrantTarget::Dir(path) => home_pattern(path, home).map(|relative| {
                if relative.is_empty() { "**".to_string() } else { format!("{relative}/**") }
            }),
            GrantTarget::Url(_) | GrantTarget::UrlAnyQuery(_) | GrantTarget::Site { .. } => {
                dropped.push(scope.line_notice(
                    &source.path,
                    line,
                    format!(
                        "`{text}` is dropped: URL reads are governed by `permissions.network`, which stays `deny`"
                    ),
                ));
                continue;
            }
        };
        match entry {
            Some(relative) => {
                let pattern = format!("{}~/{relative}", if exclude { "!" } else { "" });
                if GlobReference::new([pattern.trim_start_matches('!')]).is_ok() {
                    if !converted.contains(&pattern) {
                        converted.push(pattern);
                    }
                } else {
                    dropped.push(scope.line_notice(
                        &source.path,
                        line,
                        format!("`{text}` is dropped: it has no portable `file-read` pattern"),
                    ));
                }
            }
            None => dropped.push(scope.line_notice(
                &source.path,
                line,
                format!("`{text}` is dropped: only paths under the home directory can be written portably"),
            )),
        }
    }
    if !problems.is_empty() {
        return Err(ConfigError::new(ConfigErrorKind::Migration, problems));
    }
    notices.extend(dropped);
    Ok(converted)
}

/// The first whitespace-separated word and the trimmed rest.
fn split_word(text: &str) -> (&str, &str) {
    match text.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim()),
        None => (text, ""),
    }
}

/// `path` below `home` as escaped, `/`-separated glob text (empty for the
/// home itself); `None` outside the home or for a name that is not UTF-8.
fn home_pattern(path: &Path, home: &Path) -> Option<String> {
    let canonical_home = canonicalize_simplified(home).ok();
    let relative = canonical_home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
        .or_else(|| path.strip_prefix(home).ok())?;
    let mut names = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => names.push(GlobReference::escape(name.to_str()?)),
            _ => return None,
        }
    }
    Some(names.join("/"))
}

/// The scope's `file-read` default, as the shipped schema declares it.
fn schema_default_file_read(scope: ConfigScope) -> Vec<String> {
    let document = serde_json::json!({ "permissions": {} });
    let success = contract(scope).validate_strict(&document).expect("an empty permissions block is valid");
    success
        .defaults
        .iter()
        .find(|default| default.path.segments() == ["permissions", FILE_READ])
        .and_then(|default| default.value.as_array())
        .expect("the shipped schema declares a `file-read` default for every scope")
        .iter()
        .filter_map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_read_default_comes_from_the_schema_for_each_scope() {
        let user = schema_default_file_read(ConfigScope::User);
        let repository = schema_default_file_read(ConfigScope::Repository);
        assert_eq!(user.len(), 10);
        assert!(user.iter().all(|entry| entry.starts_with("./*.")), "{user:?}");
        assert!(repository.iter().all(|entry| entry.starts_with("&**/*.")), "{repository:?}");
    }

    #[test]
    fn home_patterns_escape_glob_metacharacters_and_refuse_outside_paths() {
        let dir = tempfile::tempdir().unwrap();
        let home = canonicalize_simplified(dir.path()).unwrap();
        assert_eq!(home_pattern(&home.join("x").join("a[1].yaml"), &home).as_deref(), Some("x/a[[]1[]].yaml"));
        assert_eq!(home_pattern(&home, &home).as_deref(), Some(""));
        assert_eq!(home_pattern(&home.parent().unwrap().join("elsewhere"), &home), None);
    }

    #[test]
    fn a_receipt_file_is_the_targets_sibling() {
        let target = Path::new("/home/me/.darkmatter.yaml");
        assert_eq!(receipt_file(target), Path::new("/home/me/.darkmatter.yaml.darkmatter-migration.json"));
    }
}
