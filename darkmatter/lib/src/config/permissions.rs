//! Evaluating the captured permission policy: whether a file read, a shell
//! command, or a URL fetch is pre-approved.
//!
//! A [`PermissionPolicy`] is built once per [`ConfigSnapshot`] from the
//! resolved blocks of up to three layers: the user file, and inside a
//! repository the repository file as committed at `HEAD` and as in the
//! working copy (each after following a `permissions` reference). Every
//! question is answered by one rule ([`PermissionPolicy::verdict`]):
//!
//! - an exclusion (`!` entry) in **any** layer removes pre-approval;
//! - otherwise the operation is pre-approved when the user layer allows it,
//!   or when **both** repository layers allow it (each judged on its own, so
//!   the two need not share entries).
//!
//! `network` is `allow` only when every applicable layer says `allow`.
//! Nothing here prompts, records a current-run approval, or applies the
//! built-in shell blacklist: those stay with the caller, which checks the
//! built-in blacklist first and may let a current-run approval override an
//! exclusion.
//!
//! User-facing guide: `darkmatter/docs/topics/permissions.md`.
//!
//! [`ConfigSnapshot`]: super::load::ConfigSnapshot

pub mod persist;

use std::path::{Component, Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference, PathIdentity, canonicalize_simplified};

use super::load::{ConfigError, ConfigErrorKind, ConfigProblem, ConfigRevision, NetworkAccess, ScopePermissions};
use crate::markdown::compose::shell_expansion::types::{ShellRuleEntry, ShellRuleSet};

/// Whether an operation may run without asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionVerdict {
    /// A layer allows it and no layer excludes it.
    Preapproved,
    /// The operation needs a current-run approval (a prompt, or
    /// `dm.schema.slot_approval_required` where nothing can prompt).
    ApprovalRequired {
        /// The exclusion that removed pre-approval; `None` when no layer
        /// allowed the operation in the first place.
        exclusion: Option<PolicyExclusion>,
    },
}

impl PermissionVerdict {
    /// Whether the operation is pre-approved.
    #[must_use]
    pub const fn is_preapproved(&self) -> bool {
        matches!(self, Self::Preapproved)
    }

    /// Whether an exclusion (rather than a missing allow) required approval.
    #[must_use]
    pub const fn is_excluded(&self) -> bool {
        matches!(self, Self::ApprovalRequired { exclusion: Some(_) })
    }
}

/// The `!` entry that removed pre-approval, for a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyExclusion {
    /// The layer whose block holds the entry.
    pub revision: ConfigRevision,
    /// The file that holds the block (a reference's target when followed).
    pub file: PathBuf,
    /// The entry as written, `!` included.
    pub entry: String,
}

/// One layer's resolved block and where it came from.
#[derive(Debug, Clone)]
pub struct PolicyLayer {
    revision: ConfigRevision,
    config_file: PathBuf,
    block_file: PathBuf,
    permissions: ScopePermissions,
    read_allows: Option<GlobReference>,
    read_exclusions: Vec<(String, GlobReference)>,
    shell_allows: ShellRuleSet,
    shell_exclusions: ShellRuleSet,
}

impl PolicyLayer {
    /// A layer from a block already typed by the loader.
    pub(crate) fn new(
        revision: ConfigRevision,
        config_file: PathBuf,
        block_file: PathBuf,
        permissions: ScopePermissions,
    ) -> Result<Self, ConfigError> {
        let invalid = |message: String| {
            ConfigError::one(
                ConfigErrorKind::InvalidSetting,
                ConfigProblem {
                    file: block_file.clone(),
                    revision,
                    pointer: "/permissions/file-read".to_string(),
                    line: None,
                    column: None,
                    message,
                },
            )
        };
        let positives: Vec<&str> = permissions
            .file_read
            .value
            .iter()
            .filter(|entry| !entry.exclude)
            .map(|entry| entry.pattern.as_str())
            .collect();
        // Each entry was parsed alone by the loader, so a combined parse
        // failing is a defect worth reporting, never a reason to allow less
        // silently.
        let read_allows = if positives.is_empty() {
            None
        } else {
            Some(GlobReference::new(&positives).map_err(|error| invalid(error.to_string()))?)
        };
        let read_exclusions = permissions
            .file_read
            .value
            .iter()
            .filter(|entry| entry.exclude)
            .map(|entry| {
                GlobReference::new([entry.pattern.as_str()])
                    .map(|glob| (entry.authored.clone(), glob))
                    .map_err(|error| invalid(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rules = |exclude: bool| ShellRuleSet {
            entries: permissions
                .shell_commands
                .value
                .iter()
                .filter(|entry| entry.exclude == exclude)
                .map(|entry| entry.rule.clone())
                .collect(),
        };
        Ok(Self {
            revision,
            shell_allows: rules(false),
            shell_exclusions: rules(true),
            config_file,
            block_file,
            permissions,
            read_allows,
            read_exclusions,
        })
    }

    /// Which observation this layer is.
    #[must_use]
    pub const fn revision(&self) -> ConfigRevision {
        self.revision
    }

    /// The `.darkmatter.yaml` that declared the block (inline or by
    /// reference).
    #[must_use]
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    /// The file holding the block: [`Self::config_file`] for an inline block,
    /// the followed reference's target otherwise. For
    /// [`ConfigRevision::RepositoryHead`] it is the working-copy spelling of
    /// the committed file.
    #[must_use]
    pub fn block_file(&self) -> &Path {
        &self.block_file
    }

    /// Whether the block came from a followed reference.
    #[must_use]
    pub fn is_referenced(&self) -> bool {
        self.config_file != self.block_file
    }

    /// The resolved block with its scope's defaults in place.
    #[must_use]
    pub const fn permissions(&self) -> &ScopePermissions {
        &self.permissions
    }

    /// The layer's positive `shell-commands` rules.
    #[must_use]
    pub const fn shell_allows(&self) -> &ShellRuleSet {
        &self.shell_allows
    }

    /// The layer's `!` `shell-commands` rules, without the `!`.
    #[must_use]
    pub const fn shell_exclusions(&self) -> &ShellRuleSet {
        &self.shell_exclusions
    }

    fn exclusion(&self, entry: String) -> PolicyExclusion {
        PolicyExclusion { revision: self.revision, file: self.block_file.clone(), entry }
    }
}

/// The repository's two layers.
#[derive(Debug, Clone)]
pub struct RepositoryLayers {
    /// The repository root the context named.
    pub root: PathBuf,
    /// The block committed at the captured `HEAD` (defaults when absent).
    pub head: PolicyLayer,
    /// The block in the working copy (defaults when absent).
    pub worktree: PolicyLayer,
}

/// The evaluated permissions of one captured configuration. `Send + Sync`;
/// [`ConfigSnapshot::policy`](super::load::ConfigSnapshot::policy) shares it
/// with `Arc`.
#[derive(Debug, Clone)]
pub struct PermissionPolicy {
    user: PolicyLayer,
    repository: Option<RepositoryLayers>,
    /// The canonical repository root, for physical containment.
    canonical_root: Option<PathIdentity>,
    home: Option<PathBuf>,
    context: FileResolutionContext,
    shell: ShellPermissions,
}

impl PermissionPolicy {
    pub(crate) fn new(user: PolicyLayer, repository: Option<RepositoryLayers>, context: &FileResolutionContext) -> Self {
        let canonical_root = repository.as_ref().map(|repository| {
            PathIdentity::new(&canonicalize_simplified(&repository.root).unwrap_or_else(|_| repository.root.clone()))
        });
        let shell = ShellPermissions::new(&user, repository.as_ref());
        Self {
            user,
            repository,
            canonical_root,
            home: context.home_dir().map(Path::to_path_buf),
            context: context.clone(),
            shell,
        }
    }

    /// The user layer.
    #[must_use]
    pub const fn user(&self) -> &PolicyLayer {
        &self.user
    }

    /// The repository layers; `None` outside a repository.
    #[must_use]
    pub const fn repository(&self) -> Option<&RepositoryLayers> {
        self.repository.as_ref()
    }

    /// Every layer: the user's, then `HEAD`'s and the working copy's.
    pub fn layers(&self) -> impl Iterator<Item = &PolicyLayer> {
        std::iter::once(&self.user)
            .chain(self.repository.iter().flat_map(|repository| [&repository.head, &repository.worktree]))
    }

    /// The home directory the configuration was captured with.
    #[must_use]
    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    pub(crate) const fn context(&self) -> &FileResolutionContext {
        &self.context
    }

    /// The one combination rule, for any per-layer predicate.
    ///
    /// `excluded` names the matching `!` entry of a layer, if any; the first
    /// layer (user, `HEAD`, working copy) with one decides the exclusion.
    /// Otherwise the operation is pre-approved when `positive` holds for the
    /// user layer, or for both repository layers.
    pub fn verdict(
        &self,
        positive: impl Fn(&PolicyLayer) -> bool,
        excluded: impl Fn(&PolicyLayer) -> Option<String>,
    ) -> PermissionVerdict {
        if let Some(exclusion) = self.layers().find_map(|layer| excluded(layer).map(|entry| layer.exclusion(entry))) {
            return PermissionVerdict::ApprovalRequired { exclusion: Some(exclusion) };
        }
        let repository = self
            .repository
            .as_ref()
            .is_some_and(|repository| positive(&repository.head) && positive(&repository.worktree));
        if repository || positive(&self.user) {
            PermissionVerdict::Preapproved
        } else {
            PermissionVerdict::ApprovalRequired { exclusion: None }
        }
    }

    /// Whether a schema slot may read `target` without asking.
    ///
    /// `reading` is the context of the document doing the read: relative
    /// entries are judged from its directory, `~/` from its home. `target`
    /// need not exist; a relative `target` is read from `reading`'s `cwd`.
    /// `..` is resolved lexically before matching. A repository layer's allow
    /// also requires the file's physical location (symlinks followed, missing
    /// components read from their nearest existing ancestor) to lie inside
    /// the repository, so an out-of-repository symlink target is never a
    /// repository allow. An exclusion matches either the lexical or the
    /// physical path.
    #[must_use]
    pub fn file_read(&self, target: &Path, reading: &FileResolutionContext) -> PermissionVerdict {
        let lexical = lexically_normal(&if target.is_absolute() { target.to_path_buf() } else { reading.cwd().join(target) });
        let physical = physical_path(&lexical);
        let repository_context = self
            .repository
            .as_ref()
            .map(|repository| reading.clone().with_repository_root(&repository.root));
        let context_of = |layer: &PolicyLayer| match (layer.revision, &repository_context) {
            (ConfigRevision::User, _) | (_, None) => reading,
            (_, Some(context)) => context,
        };
        let inside_repository = self
            .canonical_root
            .as_ref()
            .is_some_and(|root| PathIdentity::new(&physical).starts_with(root));
        self.verdict(
            |layer| {
                let context = context_of(layer);
                let allowed = layer.read_allows.as_ref().is_some_and(|glob| glob.matches(&lexical, context));
                allowed && (layer.revision == ConfigRevision::User || inside_repository)
            },
            |layer| {
                let context = context_of(layer);
                layer
                    .read_exclusions
                    .iter()
                    .find(|(_, glob)| glob.matches(&lexical, context) || glob.matches(&physical, context))
                    .map(|(entry, _)| entry.clone())
            },
        )
    }

    /// The shell-command projection of this policy.
    #[must_use]
    pub const fn shell(&self) -> &ShellPermissions {
        &self.shell
    }

    /// `allow` only when every applicable layer says `allow`. A per-run
    /// `--allow host=HOST` is added by the caller afterward.
    #[must_use]
    pub fn network(&self) -> NetworkAccess {
        if self.layers().all(|layer| layer.permissions.network.value == NetworkAccess::Allow) {
            NetworkAccess::Allow
        } else {
            NetworkAccess::Deny
        }
    }
}

/// Each layer's `shell-commands` rules, for the shell runtime's matching
/// primitives ([`ShellRuleSet::matches_exact`], [`ShellRuleSet::matches_prefix`],
/// [`ShellRuleSet::template_covers`], [`ShellRuleSet::blocks_filled`],
/// [`ShellRuleSet::template_blocks_shape`]).
///
/// ## Examples
///
/// ```
/// # use darkmatter::config::permissions::ShellPermissions;
/// # use darkmatter::markdown::compose::shell_expansion::policy::{check_user_blacklist, check_whitelist, normalize_command};
/// fn decide(shell: &ShellPermissions, executable: &str, args: &[String]) -> bool {
///     let normalized = normalize_command(executable, args);
///     shell
///         .verdict(
///             |allows| check_whitelist(allows, executable, &normalized),
///             |excluded| check_user_blacklist(excluded, executable, args, &normalized),
///         )
///         .is_preapproved()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ShellPermissions {
    user: ShellRuleSet,
    repository: Option<(ShellRuleSet, ShellRuleSet)>,
    layer_exclusions: Vec<(ConfigRevision, PathBuf, ShellRuleSet, Vec<String>)>,
    exclusions: ShellRuleSet,
}

impl ShellPermissions {
    fn new(user: &PolicyLayer, repository: Option<&RepositoryLayers>) -> Self {
        let layers: Vec<&PolicyLayer> = std::iter::once(user)
            .chain(repository.into_iter().flat_map(|repository| [&repository.head, &repository.worktree]))
            .collect();
        let mut exclusions = ShellRuleSet::default();
        for layer in &layers {
            for rule in &layer.shell_exclusions.entries {
                if !exclusions.entries.contains(rule) {
                    exclusions.entries.push(rule.clone());
                }
            }
        }
        Self {
            user: user.shell_allows.clone(),
            repository: repository
                .map(|repository| (repository.head.shell_allows.clone(), repository.worktree.shell_allows.clone())),
            layer_exclusions: layers
                .iter()
                .map(|layer| {
                    let authored = layer
                        .permissions
                        .shell_commands
                        .value
                        .iter()
                        .filter(|entry| entry.exclude)
                        .map(|entry| entry.authored.clone())
                        .collect();
                    (layer.revision, layer.block_file.clone(), layer.shell_exclusions.clone(), authored)
                })
                .collect(),
            exclusions,
        }
    }

    /// Whether `positive` holds for the user's rules, or for both the
    /// `HEAD` and working-copy rules. Exclusions are not consulted; see
    /// [`Self::exclusions`] and [`Self::verdict`].
    pub fn preapproves(&self, positive: impl Fn(&ShellRuleSet) -> bool) -> bool {
        positive(&self.user) || self.repository.as_ref().is_some_and(|(head, worktree)| positive(head) && positive(worktree))
    }

    /// Every layer's `!` rules (without the `!`) in one set: a command any of
    /// them matches is not pre-approved in any layer.
    #[must_use]
    pub const fn exclusions(&self) -> &ShellRuleSet {
        &self.exclusions
    }

    /// [`Self::preapproves`] with exclusions applied: `excluded` is asked of
    /// each layer's `!` rules in turn (user, `HEAD`, working copy), and the
    /// first layer it holds for is reported. A matching exclusion names the
    /// layer's first `!` entry, since rule sets do not say which rule matched.
    pub fn verdict(
        &self,
        positive: impl Fn(&ShellRuleSet) -> bool,
        excluded: impl Fn(&ShellRuleSet) -> bool,
    ) -> PermissionVerdict {
        for (revision, file, rules, authored) in &self.layer_exclusions {
            if !rules.entries.is_empty() && excluded(rules) {
                let entry = matching_authored(rules, authored, &excluded);
                return PermissionVerdict::ApprovalRequired {
                    exclusion: Some(PolicyExclusion { revision: *revision, file: file.clone(), entry }),
                };
            }
        }
        if self.preapproves(positive) {
            PermissionVerdict::Preapproved
        } else {
            PermissionVerdict::ApprovalRequired { exclusion: None }
        }
    }
}

/// The authored text of the first rule of `rules` that `excluded` holds for
/// on its own.
fn matching_authored(rules: &ShellRuleSet, authored: &[String], excluded: &impl Fn(&ShellRuleSet) -> bool) -> String {
    rules
        .entries
        .iter()
        .zip(authored)
        .find(|(rule, _)| excluded(&ShellRuleSet { entries: vec![(*rule).clone()] }))
        .or_else(|| rules.entries.iter().zip(authored).next())
        .map(|(_, text)| text.clone())
        .unwrap_or_default()
}

/// The text of a `shell-commands` rule, without `!`: `exact …`, `prefix …`,
/// or `template <canonical shape>`.
#[must_use]
pub fn shell_rule_text(rule: &ShellRuleEntry) -> String {
    match rule {
        ShellRuleEntry::Exact(command) => format!("exact {command}"),
        ShellRuleEntry::Prefix(executable) => format!("prefix {executable}"),
        ShellRuleEntry::Template(shape) => format!("template {}", shape.canonical_text()),
    }
}

/// `path` with `.` dropped and `..` applied to the name before it, without
/// touching the filesystem. A `..` at the root stays at the root.
pub(crate) fn lexically_normal(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if normal.file_name().is_some() {
                    normal.pop();
                } else if !normal.has_root() {
                    normal.push(component);
                }
            }
            other => normal.push(other),
        }
    }
    normal
}

/// Where `path` physically leads: the canonical spelling of its nearest
/// existing ancestor (symlinks followed) joined with the components below it
/// that do not exist yet.
pub(crate) fn physical_path(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut current = path;
    loop {
        if let Ok(canonical) = canonicalize_simplified(current) {
            return missing.iter().rev().fold(canonical, |path: PathBuf, name| path.join(name));
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_os_string());
                current = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_normalization_applies_parent_steps() {
        assert_eq!(lexically_normal(Path::new("/a/b/../c/./d")), PathBuf::from("/a/c/d"));
        assert_eq!(lexically_normal(Path::new("/a/../../b")), PathBuf::from("/b"));
    }

    #[test]
    fn the_policy_is_shareable_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<PermissionPolicy>();
    }

    #[test]
    fn rule_text_reads_back_as_the_same_rule() {
        use crate::markdown::compose::shell_expansion::store::parse_rule_entry;
        for text in ["exact ps -e", "prefix ls", "template git add {{ string }}"] {
            let rule = parse_rule_entry(text).unwrap();
            assert_eq!(parse_rule_entry(&shell_rule_text(&rule)).unwrap(), rule, "{text}");
        }
    }
}
