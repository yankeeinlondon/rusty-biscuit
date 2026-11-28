//! The typed values of a validated permissions block, and the per-entry
//! checks the schema cannot state: glob and reference grammar, portability,
//! and repository containment.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext, GlobReference};

use crate::markdown::compose::shell_expansion::store::parse_rule_entry;
use crate::markdown::compose::shell_expansion::types::ShellRuleEntry;

use super::super::schema::ConfigScope;

/// A setting's effective value and whether the file wrote it. An absent
/// setting holds its scope's schema default and `authored` is `false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting<T> {
    /// The effective value.
    pub value: T,
    /// Whether the configuration file wrote this setting itself.
    pub authored: bool,
}

/// One `file-read` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadEntry {
    /// The entry as written, `!` included.
    pub authored: String,
    /// Whether it is a `!` exclusion.
    pub exclude: bool,
    /// The glob reference after any `!`.
    pub pattern: String,
}

/// One `shell-commands` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandEntry {
    /// The entry as written, `!` included.
    pub authored: String,
    /// Whether it is a `!` exclusion.
    pub exclude: bool,
    /// The parsed rule.
    pub rule: ShellRuleEntry,
}

/// The `network` setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAccess {
    /// Every URL may be fetched.
    Allow,
    /// A URL may be fetched only once this run approves it: an answer to a
    /// schema slot's prompt, or a per-run `--allow host=HOST`. Where nothing
    /// asks, the read needs approval instead of being refused outright.
    Ask,
    /// No URL may be fetched without a per-run `--allow host=HOST`.
    Deny,
}

/// An inline permissions block with its scope's defaults in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopePermissions {
    /// Files a schema slot may read without asking.
    pub file_read: Setting<Vec<FileReadEntry>>,
    /// Shell commands that run without a prompt.
    pub shell_commands: Setting<Vec<ShellCommandEntry>>,
    /// Whether URLs may be fetched.
    pub network: Setting<NetworkAccess>,
}

/// A `permissions` value that points at another file's block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionsReference {
    /// The reference as written.
    pub reference: String,
    /// The configuration file that wrote it.
    pub declared_in: PathBuf,
    /// The directory the reference resolves from: `declared_in`'s directory.
    pub base_dir: PathBuf,
    /// The reference grammar's classification.
    pub kind: FileReferenceKind,
}

/// One configuration file's `permissions` setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionsSetting {
    /// The block is written in the file, or absent and defaulted.
    Inline(ScopePermissions),
    /// The block lives in another file. Following it is planned; the
    /// reference's syntax is already checked.
    Reference(PermissionsReference),
}

/// Why one entry is invalid; the caller locates it.
pub(super) type EntryProblem = String;

/// Checks one `file-read` entry for `scope`. `context` supplies the
/// declared portable variables, the environment, and the repository root.
pub(super) fn file_read_entry(
    authored: &str,
    scope: ConfigScope,
    context: &FileResolutionContext,
    portable: &BTreeSet<String>,
) -> Result<FileReadEntry, EntryProblem> {
    let (exclude, pattern) = match authored.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, authored),
    };
    if pattern.trim().is_empty() {
        return Err("a `file-read` entry needs a pattern".to_string());
    }
    GlobReference::new([pattern]).map_err(|error| error.to_string())?;
    let reference = FileReference::new(pattern).map_err(|error| format!("`{pattern}` is not a file reference: {error}"))?;
    match reference.class().kind {
        FileReferenceKind::Absolute => {
            return Err(format!("`{pattern}` is an absolute path; permissions must be portable"));
        }
        FileReferenceKind::Magic | FileReferenceKind::Vault | FileReferenceKind::Url => {
            return Err(format!(
                "`{pattern}` names a search or remote root; use a relative, `&`, `^`, `~/`, or `{{{{VAR}}}}` pattern"
            ));
        }
        FileReferenceKind::Home if scope == ConfigScope::Repository => {
            return Err(format!("`{pattern}` reads from the home directory, which a repository cannot grant"));
        }
        _ => {}
    }
    for name in reference.env_variables() {
        if !portable.contains(name) {
            return Err(format!(
                "`{{{{{name}}}}}` is not a declared portable variable (list it in `PORTABLE_ENV_VARIABLES`)"
            ));
        }
    }
    if scope == ConfigScope::Repository {
        if climbs_above_start(reference.payload()) {
            return Err(format!("`{pattern}` uses `..` to leave the repository"));
        }
        if !reference.env_variables().is_empty() {
            let expanded = reference
                .interpolated_payload(context)
                .map_err(|error| format!("`{pattern}` cannot be expanded: {error}"))?;
            let inside = context
                .repository_root()
                .is_some_and(|root| !Path::new(&expanded).is_absolute() || Path::new(&expanded).starts_with(root));
            if !inside {
                return Err(format!("`{pattern}` expands to `{expanded}`, outside the repository"));
            }
        }
    }
    Ok(FileReadEntry { authored: authored.to_string(), exclude, pattern: pattern.to_string() })
}

/// Checks one `shell-commands` entry.
pub(super) fn shell_command_entry(authored: &str) -> Result<ShellCommandEntry, EntryProblem> {
    let (exclude, rule) = match authored.strip_prefix('!') {
        Some(rest) => (true, rest.trim()),
        None => (false, authored.trim()),
    };
    if rule.is_empty() {
        return Err("a `shell-commands` entry needs a rule".to_string());
    }
    let parsed = parse_rule_entry(rule)?;
    if let ShellRuleEntry::Exact(command) | ShellRuleEntry::Prefix(command) = &parsed
        && command.trim().is_empty()
    {
        return Err(format!("`{rule}` names no command"));
    }
    Ok(ShellCommandEntry { authored: authored.to_string(), exclude, rule: parsed })
}

/// Checks a `permissions` reference written in `declared_in`.
pub(super) fn permissions_reference(
    reference: &str,
    declared_in: &Path,
    scope: ConfigScope,
) -> Result<PermissionsReference, EntryProblem> {
    if reference.trim().is_empty() {
        return Err("the `permissions` reference is blank".to_string());
    }
    let parsed =
        FileReference::new(reference).map_err(|error| format!("`{reference}` is not a file reference: {error}"))?;
    let kind = parsed.class().kind;
    if kind == FileReferenceKind::Url {
        return Err(format!("`{reference}` is remote; the permissions block must be a local file"));
    }
    if parsed.class().recursive {
        return Err(format!("`{reference}` must name one file"));
    }
    if scope == ConfigScope::Repository {
        let leaves = matches!(
            kind,
            FileReferenceKind::Absolute | FileReferenceKind::Home | FileReferenceKind::Magic | FileReferenceKind::Vault
        ) || !parsed.env_variables().is_empty()
            || climbs_above_start(parsed.payload());
        if leaves {
            return Err(format!("`{reference}` must name a file inside the repository"));
        }
    }
    let base_dir = declared_in.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok(PermissionsReference { reference: reference.to_string(), declared_in: declared_in.to_path_buf(), base_dir, kind })
}

/// Whether `payload`, read lexically, climbs above the directory it starts
/// in at some point (`../x`, `a/../../x`).
fn climbs_above_start(payload: &str) -> bool {
    let mut depth = 0usize;
    for component in Path::new(&payload.replace('\\', "/")).components() {
        match component {
            Component::ParentDir => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            Component::Normal(_) => depth += 1,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climbing_is_judged_lexically_from_the_start() {
        for (payload, climbs) in
            [("a/*.md", false), ("a/../b/*.md", false), ("../x", true), ("a/../../x", true), ("..\\x", true), ("./x", false)]
        {
            assert_eq!(climbs_above_start(payload), climbs, "{payload}");
        }
    }
}
