//! Following a `permissions` file reference to the block it names, for each
//! layer of the policy.
//!
//! - The user file's reference is resolved on disk.
//! - The repository working copy's reference is resolved on disk, and the
//!   target's canonical location (symlinks followed) must lie inside the
//!   repository.
//! - The repository `HEAD`'s reference is resolved against the pinned
//!   committed tree only: its candidates come from
//!   [`FileReference::candidate_plan`] (no disk probing), committed symbolic
//!   links are followed within the tree (cycles and more than
//!   [`MAX_COMMITTED_LINKS`] links refused), and a path leaving the
//!   repository is refused.
//!
//! The target's top-level `permissions` must be a mapping; it is validated
//! against the referring scope's contract and typed with the same entry
//! checks as an inline block. Every failure names both files and is never
//! replaced by a default or an empty policy.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext, PathIdentity, canonicalize_simplified};
use serde_json::Value;
use simplified_schema::SchemaError;
use simplified_schema::yaml_documents::load_yaml_document;
use sniff::filesystem::git::{CommittedEntry, CommittedTreeSnapshot};

use super::super::permissions::{PolicyLayer, lexically_normal};
use super::super::schema::{ConfigScope, permissions_contract};
use super::{
    ConfigDependency, ConfigError, ConfigErrorKind, ConfigFile, ConfigProblem, ConfigRevision, Observe,
    PermissionsReference, PermissionsSetting, SettingContext, SourcePosition, apply_defaults, byte_range,
    failure_problems, line_column, problem_at,
};
use crate::config::migration_receipt::content_hash;

/// The direct correction of a target whose document lacks a usable block.
const WRITE_MAPPING: &str = "write a `permissions` mapping in it";

/// The most committed symbolic links one `HEAD` reference may pass through,
/// as for path resolution on most systems.
pub const MAX_COMMITTED_LINKS: usize = 40;

/// Where a layer's repository is, for a repository layer.
#[derive(Clone, Copy)]
pub(super) enum Where<'a> {
    User,
    Worktree { root: &'a Path },
    Head { root: &'a Path, tree: &'a CommittedTreeSnapshot },
}

impl Where<'_> {
    const fn scope(self) -> ConfigScope {
        match self {
            Self::User => ConfigScope::User,
            Self::Worktree { .. } | Self::Head { .. } => ConfigScope::Repository,
        }
    }
}

impl Observe<'_> {
    /// The policy layer of `file`: its inline block, or the block its
    /// reference names. Records every followed target in `dependencies`.
    pub(super) fn layer(
        &self,
        file: &ConfigFile,
        place: Where<'_>,
        dependencies: &mut Vec<ConfigDependency>,
    ) -> Result<PolicyLayer, ConfigError> {
        let setting = file.permissions.as_ref().map_err(Clone::clone)?;
        match setting {
            PermissionsSetting::Inline(block) => {
                PolicyLayer::new(file.revision, file.path.clone(), file.path.clone(), block.clone())
            }
            PermissionsSetting::Reference(reference) => {
                let source = Source { reference, revision: file.revision, at: file.permissions_at.clone() };
                let (target, bytes) = self.follow(&source, place)?;
                dependencies.push(ConfigDependency {
                    revision: file.revision,
                    path: target.clone(),
                    content_hash: bytes.as_deref().map(content_hash),
                    referenced_from: Some(file.path.clone()),
                });
                let Some(bytes) = bytes else {
                    return Err(source.error(&target, &format!("which does not exist; {}", source.remedy("create it"))));
                };
                let block = self.target_block(&source, &target, &bytes, place.scope())?;
                PolicyLayer::new(file.revision, file.path.clone(), target, block)
            }
        }
    }

    /// The target path and its bytes (`None` when absent).
    fn follow(&self, source: &Source<'_>, place: Where<'_>) -> Result<(PathBuf, Option<Vec<u8>>), ConfigError> {
        let reference = FileReference::new(&source.reference.reference)
            .map_err(|error| source.error(Path::new(&source.reference.reference), &format!("which is not a file reference: {error}")))?;
        let declared_in = &source.reference.declared_in;
        let mut context = self.context.for_trusted_external_source(declared_in);
        context = match place {
            Where::User => context.allow_external_relative(),
            Where::Worktree { root } | Where::Head { root, .. } => context.with_repository_root(root),
        };
        let candidates = reference
            .candidate_plan(&context)
            .map_err(|error| source.error(Path::new(&source.reference.reference), &format!("which cannot be resolved: {error}")))?;
        let candidates: Vec<PathBuf> =
            candidates.iter().map(|candidate| lexically_normal(&absolute(candidate.path(), &context))).collect();
        let Some(first) = candidates.first().cloned() else {
            return Err(source.error(Path::new(&source.reference.reference), "which names no file"));
        };
        match place {
            Where::User => {
                for candidate in &candidates {
                    if let Some(bytes) = read_disk(candidate, source)? {
                        return Ok((candidate.clone(), Some(bytes)));
                    }
                }
                Ok((first, None))
            }
            Where::Worktree { root } => {
                let root_identity = PathIdentity::new(&canonicalize_simplified(root).unwrap_or_else(|_| root.to_path_buf()));
                for candidate in &candidates {
                    inside(candidate, root, source)?;
                    let Some(bytes) = read_disk(candidate, source)? else { continue };
                    let canonical = canonicalize_simplified(candidate)
                        .map_err(|error| source.error(candidate, &format!("which cannot be resolved: {error}")))?;
                    if !PathIdentity::new(&canonical).starts_with(&root_identity) {
                        return Err(source.error(
                            candidate,
                            &format!("which leads to `{}`, outside the repository", canonical.display()),
                        ));
                    }
                    return Ok((candidate.clone(), Some(bytes)));
                }
                Ok((first, None))
            }
            Where::Head { root, tree } => {
                for candidate in &candidates {
                    let relative = inside(candidate, root, source)?;
                    match read_committed_following(tree, &relative) {
                        Ok(Some((path, bytes))) => return Ok((root.join(path), Some(bytes))),
                        Ok(None) => {}
                        Err(message) => return Err(source.error(candidate, &message)),
                    }
                }
                Ok((first, None))
            }
        }
    }

    /// The typed block under the target's `permissions` key.
    fn target_block(
        &self,
        source: &Source<'_>,
        target: &Path,
        bytes: &[u8],
        scope: ConfigScope,
    ) -> Result<super::ScopePermissions, ConfigError> {
        let text = std::str::from_utf8(bytes).map_err(|_| source.error(target, "which is not valid UTF-8"))?;
        let loaded = load_yaml_document(text, target).map_err(|error| {
            let message = match error {
                SchemaError::SchemaDocument { message, .. } => message,
                other => other.to_string(),
            };
            source.error(target, &format!("which is not valid YAML ({message}); {}", source.remedy("fix it")))
        })?;
        let block = match loaded.json.get("permissions") {
            Some(block @ Value::Object(_)) => block.clone(),
            Some(Value::String(next)) => {
                return Err(source.error(
                    target,
                    &format!(
                        "whose `permissions` is another reference (`{next}`); references do not chain, so {}",
                        source.remedy(&format!("write the block in `{}`", target.display()))
                    ),
                ));
            }
            None if !loaded.json.is_object() => {
                return Err(source.error(
                    target,
                    &format!("which is not a YAML mapping with a `permissions` mapping; {}", source.remedy(WRITE_MAPPING)),
                ));
            }
            Some(_) => {
                return Err(source.error(target, &format!("whose `permissions` is not a mapping; {}", source.remedy(WRITE_MAPPING))));
            }
            None => {
                return Err(source.error(target, &format!("which has no `permissions` mapping; {}", source.remedy(WRITE_MAPPING))));
            }
        };
        let locate = |pointer: &[String]| byte_range(&loaded, pointer).map(|range| line_column(text, range.start));
        let context = SettingContext { path: target, revision: source.revision, scope, locate: &locate };
        let mut effective = block.clone();
        match permissions_contract(scope).validate_strict(&block) {
            // The block's defaults are relative to the block itself.
            Ok(success) => apply_defaults(&mut effective, &success.defaults),
            Err(failure) => {
                let problems = failure_problems(&failure)
                    .into_iter()
                    .map(|(mut segments, message)| {
                        segments.insert(0, "permissions".to_string());
                        context.problem(&segments, message)
                    })
                    .collect();
                return Err(source.invalid_target(target, problems));
            }
        }
        match self.permissions(&effective, Some(&block), &context) {
            Ok(PermissionsSetting::Inline(block)) => Ok(block),
            Ok(PermissionsSetting::Reference(_)) => Err(source.error(target, "whose `permissions` is not a mapping")),
            Err(error) => Err(source.invalid_target(target, error.problems().to_vec())),
        }
    }
}

/// A followed reference's target and its bytes (`None` when absent).
pub(crate) type FollowedTarget = (PathBuf, Option<Vec<u8>>);

/// The file `file`'s `permissions` reference names on disk, and its bytes
/// (`None` when nothing is there), following it as the policy does for the
/// user file (`scope` user) or the repository working copy: what setup
/// ([`crate::config::init`]) reads to write a referenced block back inline.
/// `None` when `file`'s `permissions` is not a valid reference.
///
/// ## Errors
///
/// The reference error the policy would report, naming both files.
pub(crate) fn follow_on_disk(
    context: &FileResolutionContext,
    file: &ConfigFile,
    scope: ConfigScope,
) -> Option<Result<FollowedTarget, ConfigError>> {
    let Ok(PermissionsSetting::Reference(reference)) = &file.permissions else {
        return None;
    };
    let portable = context.portable_variables();
    let observe = Observe { context, portable: &portable };
    let source = Source { reference, revision: file.revision, at: file.permissions_at.clone() };
    let place = match (scope, context.repository_root()) {
        (ConfigScope::User, _) => Where::User,
        (ConfigScope::Repository, Some(root)) => Where::Worktree { root },
        (ConfigScope::Repository, None) => {
            return Some(Err(source.error(Path::new(&reference.reference), "which needs a repository to resolve in")));
        }
    };
    Some(observe.follow(&source, place))
}

/// The referring side of a reference, for errors.
struct Source<'a> {
    reference: &'a PermissionsReference,
    revision: ConfigRevision,
    at: Option<SourcePosition>,
}

impl Source<'_> {
    /// How to repair an unusable target, after `fix` (the direct correction
    /// of the target), in the declaring file's own scope. Only the user scope
    /// names `md init --inline`: that command writes the user file, so for a
    /// repository reference it would succeed and leave the error in place.
    /// A reference declared at `HEAD` is read from the commit, so its
    /// correction must be committed too.
    fn remedy(&self, fix: &str) -> String {
        let declared_in = self.reference.declared_in.display();
        match self.revision {
            ConfigRevision::User => {
                format!("{fix}, point `permissions` at another file, or run `md init --inline` to write the block back inline")
            }
            ConfigRevision::RepositoryWorktree => {
                format!("{fix}, or point `permissions` in `{declared_in}` at another file or write the block there inline")
            }
            ConfigRevision::RepositoryHead => format!(
                "{fix}, or point `permissions` in `{declared_in}` at another file or write the block there inline; commit the correction too, because this reference is declared in the commit at HEAD and a change only in the working copy leaves it broken"
            ),
        }
    }

    fn problem(&self, message: String) -> ConfigProblem {
        problem_at(&self.reference.declared_in, self.revision, "/permissions", self.at.clone(), message)
    }

    /// An error located at the reference, naming its target.
    fn error(&self, target: &Path, what: &str) -> ConfigError {
        ConfigError::one(
            ConfigErrorKind::InvalidReference,
            self.problem(format!(
                "the `permissions` reference `{}` names `{}`, {what}",
                self.reference.reference,
                target.display()
            )),
        )
    }

    /// The target's own problems, followed by the reference that led there.
    fn invalid_target(&self, target: &Path, mut problems: Vec<ConfigProblem>) -> ConfigError {
        problems.push(self.problem(format!(
            "the `permissions` reference `{}` names `{}`, whose block is invalid",
            self.reference.reference,
            target.display()
        )));
        ConfigError::new(ConfigErrorKind::InvalidReference, problems)
    }
}

fn absolute(path: &Path, context: &FileResolutionContext) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { context.cwd().join(path) }
}

/// The bytes of the file at `path`, `None` when nothing (or no file) is
/// there.
fn read_disk(path: &Path, source: &Source<'_>) -> Result<Option<Vec<u8>>, ConfigError> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(source.error(path, &format!("which cannot be read: {error}"))),
    }
    std::fs::read(path).map(Some).map_err(|error| source.error(path, &format!("which cannot be read: {error}")))
}

/// The `/`-separated repository-relative spelling of `candidate`, or an
/// error when it lies outside `root`.
fn inside(candidate: &Path, root: &Path, source: &Source<'_>) -> Result<String, ConfigError> {
    let names = PathIdentity::new(candidate)
        .strip_prefix(&PathIdentity::new(root))
        .map(<[_]>::to_vec)
        .ok_or_else(|| source.error(candidate, "which lies outside the repository"))?;
    let names: Option<Vec<&str>> = names.iter().map(|name| name.to_str()).collect();
    match names {
        Some(names) if !names.is_empty() => Ok(names.join("/")),
        Some(_) => Err(source.error(candidate, "which is the repository root, not a file")),
        None => Err(source.error(candidate, "which has no UTF-8 spelling")),
    }
}

/// The committed bytes at the repository-relative `path`, following
/// committed symbolic links within the tree, with the final path; `None`
/// when nothing is committed there.
///
/// ## Errors
///
/// Why the path cannot be read: a link leaving the repository, a cycle or
/// more than [`MAX_COMMITTED_LINKS`] links, a directory or submodule, or a
/// Git read failure.
pub(crate) fn read_committed_following(
    tree: &CommittedTreeSnapshot,
    path: &str,
) -> Result<Option<(String, Vec<u8>)>, String> {
    let mut names: Vec<String> = path.split('/').map(str::to_string).collect();
    let mut links = 0usize;
    let mut seen = BTreeSet::new();
    'restart: loop {
        for depth in 0..names.len() {
            let prefix = names[..=depth].join("/");
            let entry = tree.read_file(&prefix).map_err(|error| format!("which cannot be read at HEAD: {error}"))?;
            let last = depth + 1 == names.len();
            match entry {
                CommittedEntry::Absent => return Ok(None),
                CommittedEntry::Directory if !last => {}
                CommittedEntry::Blob { bytes, .. } if last => return Ok(Some((prefix, bytes))),
                CommittedEntry::Blob { .. } => return Ok(None),
                CommittedEntry::Directory => return Err("which is a committed directory, not a file".to_string()),
                CommittedEntry::Submodule { .. } => return Err("which is inside a committed submodule".to_string()),
                CommittedEntry::Symlink { target } => {
                    links += 1;
                    if links > MAX_COMMITTED_LINKS {
                        return Err(format!("which passes through more than {MAX_COMMITTED_LINKS} committed symbolic links"));
                    }
                    let target = String::from_utf8(target)
                        .map_err(|_| format!("whose committed symbolic link `{prefix}` is not UTF-8"))?;
                    let mut next = names[..depth].to_vec();
                    if target.starts_with('/') || target.starts_with('\\') || target.get(1..2) == Some(":") {
                        return Err(format!("whose committed symbolic link `{prefix}` leads outside the repository (`{target}`)"));
                    }
                    for name in target.split(['/', '\\']) {
                        match name {
                            "" | "." => {}
                            ".." => {
                                if next.pop().is_none() {
                                    return Err(format!(
                                        "whose committed symbolic link `{prefix}` leads outside the repository (`{target}`)"
                                    ));
                                }
                            }
                            other => next.push(other.to_string()),
                        }
                    }
                    next.extend(names[depth + 1..].iter().cloned());
                    if next.is_empty() {
                        return Err(format!("whose committed symbolic link `{prefix}` leads to the repository root"));
                    }
                    if !seen.insert(next.join("/")) {
                        return Err(format!("whose committed symbolic links form a cycle at `{prefix}`"));
                    }
                    names = next;
                    continue 'restart;
                }
                _ => return Err("which is committed as an entry this version cannot read".to_string()),
            }
        }
        return Ok(None);
    }
}
