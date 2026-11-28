//! Detecting a change that may widen a repository's permissions, for a
//! trusted CI check.
//!
//! [`compare_revisions`] reads two commits of one repository as passive
//! object data (no checkout, filter, hook, build script, or candidate code
//! runs) and resolves, in each, every repository policy the trusted code
//! understands: each policy file's block (inline, or the block its reference
//! names in that same commit), with the repository defaults in place. It
//! then compares the resolved blocks, the shipped default schemas, and the
//! checker's own inputs, and reports whether the change may widen.
//!
//! A change is proven not to widen only when every difference is one of:
//!
//! - the same resolved block, wherever it lives (moving an inline block to a
//!   reference, or the reverse, compares the resolved blocks);
//! - a removed positive (`file-read` or `shell-commands`) entry;
//! - an added `!` exclusion;
//! - `network` becoming more restrictive (`allow` → `ask` → `deny`).
//!
//! Every other difference is a [`WideningReason`]: an added positive entry, a
//! removed exclusion, `network` becoming less restrictive (`deny` → `ask` or
//! `allow`, `ask` → `allow`), a shipped default that grows
//! or that the trusted code cannot read, and a change to any checker input.
//! Rewriting a glob or a command template is an added entry (and a removed
//! one): no containment between two globs or two templates is attempted.
//! Entry counts are never compared.
//!
//! A file, Git, YAML, or reference error is a [`WideningError`], never "no
//! change".
//!
//! User-facing guide: `darkmatter/docs/topics/configuration.md`
//! ("The cloned-repository risk").

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use biscuit_file::FileResolutionContext;
use serde::{Deserialize, Serialize};
use simplified_schema::SimplifiedSchema;
use sniff::filesystem::git::{CommittedEntry, CommittedTreeSnapshot, GitRepo};

use super::load::{NetworkAccess, ScopePermissions, committed_layer, typed_block};
use super::permissions::shell_rule_text;
use super::schema::{ConfigScope, prepare_host_config_type};

/// What the checker reads, as listed in the repository's
/// `.github/ci/permission-inputs.json`. Every list must be non-empty: an
/// empty list would silently compare nothing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct WideningInputs {
    /// The repository's policy files, repository-relative.
    pub policy_files: Vec<PolicyFile>,
    /// The shipped schemas that hold each scope's default block.
    pub default_schemas: Vec<DefaultSchema>,
    /// Files whose change could alter what this check concludes: its own
    /// code, its workflow, and the parsers it relies on. A path ending in
    /// `/` names every file below it.
    pub checker_inputs: Vec<String>,
}

/// One policy file to resolve in each revision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyFile {
    /// The repository-relative path, `/`-separated.
    pub path: String,
    /// How the file holds its block.
    pub format: PolicyFormat,
}

/// How a policy file holds its `permissions` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyFormat {
    /// A `.darkmatter.yaml`, judged whole against the repository contract.
    DarkmatterConfig,
    /// Another program's configuration (Claudine's), of which only the
    /// top-level `permissions` value is judged.
    PermissionsKey,
}

/// One shipped schema holding default blocks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultSchema {
    /// The repository-relative path, `/`-separated.
    pub path: String,
    /// How the schema declares its configuration types.
    pub format: SchemaFormat,
}

/// How a shipped schema declares the `user-config` and `repository-config`
/// types whose `permissions` default is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaFormat {
    /// Darkmatter's own configuration schema, standalone.
    DarkmatterConfig,
    /// A host program's schema that imports Darkmatter's types.
    HostConfig,
}

impl WideningInputs {
    /// Parses the inputs file's JSON text.
    ///
    /// ## Errors
    ///
    /// A [`WideningError`] for text that is not exactly one JSON object of
    /// this shape (unknown or duplicate keys included), or for an empty
    /// list or path.
    pub fn from_json(text: &str) -> Result<Self, WideningError> {
        let inputs: Self =
            serde_json::from_str(text).map_err(|error| WideningError::new(format!("the inputs file is invalid: {error}")))?;
        for (name, empty) in [
            ("policy-files", inputs.policy_files.is_empty()),
            ("default-schemas", inputs.default_schemas.is_empty()),
            ("checker-inputs", inputs.checker_inputs.is_empty()),
        ] {
            if empty {
                return Err(WideningError::new(format!("the inputs file's `{name}` is empty, so nothing would be compared")));
            }
        }
        let paths = inputs
            .policy_files
            .iter()
            .map(|file| &file.path)
            .chain(inputs.default_schemas.iter().map(|schema| &schema.path))
            .chain(&inputs.checker_inputs);
        for path in paths {
            let bare = path.strip_suffix('/').unwrap_or(path);
            if bare.is_empty() || bare.split('/').any(|name| name.is_empty() || name == "." || name == "..") || path.contains('\\') {
                return Err(WideningError::new(format!("the inputs file names `{path}`, which is not a repository-relative path")));
            }
        }
        Ok(inputs)
    }
}

/// Why the comparison failed. Never read as "no change".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WideningError {
    message: String,
}

impl WideningError {
    fn new(message: String) -> Self {
        Self { message }
    }

    /// The failure, for a person.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for WideningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WideningError {}

/// What [`compare_revisions`] found. Serialized as the checker's JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WideningReport {
    /// The base commit id.
    pub base: String,
    /// The head commit id.
    pub head: String,
    /// Every input whose committed entry differs between the two commits,
    /// sorted: policy files and the targets either revision followed,
    /// default schemas, and checker inputs.
    pub changed_inputs: Vec<String>,
    /// Each resolved policy, before and after.
    pub policies: Vec<PolicyComparison>,
    /// Every reason the change may widen; empty when it provably does not.
    pub reasons: Vec<WideningReason>,
    /// Whether any reason was found.
    pub potential_widening: bool,
}

/// One policy in both revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyComparison {
    /// The policy file, or the default schema, that holds it.
    pub input: String,
    /// `repository` for a policy file; for a default schema, which default
    /// of which scope (`repository-block-default`, `user-entry-defaults`, …).
    pub scope: &'static str,
    /// As at the base commit.
    pub before: ResolvedPolicy,
    /// As at the head commit.
    pub after: ResolvedPolicy,
}

/// A resolved block, normalized: entries deduplicated and sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedPolicy {
    /// The repository-relative file holding the block; `None` when the
    /// block is the default (no file, or no `permissions` in it), or for a
    /// default schema.
    pub block_file: Option<String>,
    /// `file-read` entries, `!` included.
    #[serde(rename = "file-read")]
    pub file_read: Vec<String>,
    /// `shell-commands` rules in canonical form, `!` included.
    #[serde(rename = "shell-commands")]
    pub shell_commands: Vec<String>,
    /// `allow`, `ask`, or `deny`.
    pub network: &'static str,
}

/// One reason a change may widen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WideningReason {
    /// The policy file, default schema, or checker input it concerns.
    pub input: String,
    #[allow(missing_docs)]
    pub kind: ReasonKind,
    /// What changed, for a person.
    pub detail: String,
}

/// The kinds of [`WideningReason`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReasonKind {
    /// A `file-read` or `shell-commands` entry the base did not have.
    AddedAllow,
    /// A `!` entry the head no longer has.
    RemovedExclusion,
    /// `network` became less restrictive (`deny` → `ask` or `allow`,
    /// `ask` → `allow`).
    NetworkAllowed,
    /// A changed default schema whose defaults the trusted code cannot read.
    DefaultsNotUnderstood,
    /// A file the check itself depends on changed.
    CheckerInputChanged,
}

/// Compares the policies of the commits `base` and `head` (full hex ids) of
/// the repository whose working directory is `repository`.
///
/// ## Errors
///
/// A [`WideningError`] when the repository or either commit cannot be read,
/// or a policy file, its reference, or its target fails to resolve in
/// either commit.
pub fn compare_revisions(
    repository: &Path,
    base: &str,
    head: &str,
    inputs: &WideningInputs,
) -> Result<WideningReport, WideningError> {
    let repo = GitRepo::discover(repository)
        .map_err(|error| WideningError::new(format!("`{}` cannot be opened: {error}", repository.display())))?
        .ok_or_else(|| WideningError::new(format!("`{}` is not inside a Git repository", repository.display())))?;
    let root = repo.repo_root().to_path_buf();
    let pin = |id: &str, side: &str| {
        repo.committed_tree_at(id)
            .map_err(|error| WideningError::new(format!("the {side} commit `{id}` cannot be read: {error}")))
    };
    let before = pin(base, "base")?;
    let after = pin(head, "head")?;
    compare_trees(&root, &before, &after, inputs)
}

/// [`compare_revisions`] over two already pinned trees of the repository
/// whose working directory is `root`.
///
/// ## Errors
///
/// As [`compare_revisions`].
pub fn compare_trees(
    root: &Path,
    before: &CommittedTreeSnapshot,
    after: &CommittedTreeSnapshot,
    inputs: &WideningInputs,
) -> Result<WideningReport, WideningError> {
    let context = FileResolutionContext::from_snapshot(root.to_path_buf(), None, Default::default()).with_repository_root(root);
    let mut watched: BTreeSet<String> = BTreeSet::new();
    let mut policies = Vec::new();
    let mut reasons = Vec::new();

    for file in &inputs.policy_files {
        watched.insert(file.path.clone());
        let mut resolve = |tree: &CommittedTreeSnapshot, side: &str| -> Result<ResolvedPolicy, WideningError> {
            let (policy, followed) = resolve_policy(&context, tree, root, file)
                .map_err(|error| WideningError::new(format!("`{}` at the {side} commit: {error}", file.path)))?;
            watched.extend(followed);
            Ok(policy)
        };
        let before_policy = resolve(before, "base")?;
        let after_policy = resolve(after, "head")?;
        reasons.extend(compare_policies(&file.path, &before_policy, &after_policy));
        policies.push(PolicyComparison { input: file.path.clone(), scope: "repository", before: before_policy, after: after_policy });
    }

    for schema in &inputs.default_schemas {
        watched.insert(schema.path.clone());
        let before_bytes = blob(before, &schema.path)?;
        let after_bytes = blob(after, &schema.path)?;
        if before_bytes == after_bytes {
            continue;
        }
        for scope in [ConfigScope::User, ConfigScope::Repository] {
            for probe in [DefaultProbe::NoBlock, DefaultProbe::EmptyBlock] {
                let label = probe.label(scope);
                let read = |bytes: &Option<Vec<u8>>| default_policy(&context, bytes.as_deref(), schema, scope, probe);
                match (read(&before_bytes), read(&after_bytes)) {
                    (Ok(before_policy), Ok(after_policy)) => {
                        reasons.extend(compare_policies(&schema.path, &before_policy, &after_policy));
                        policies.push(PolicyComparison {
                            input: schema.path.clone(),
                            scope: label,
                            before: before_policy,
                            after: after_policy,
                        });
                    }
                    (before_result, after_result) => {
                        let why = match (before_result, after_result) {
                            (_, Err(why)) => format!("at the head commit, {why}"),
                            (Err(why), _) => format!("at the base commit, {why}"),
                            (Ok(_), Ok(_)) => unreachable!("handled by the arm above"),
                        };
                        reasons.push(WideningReason {
                            input: schema.path.clone(),
                            kind: ReasonKind::DefaultsNotUnderstood,
                            detail: format!("`{label}` cannot be read {why}"),
                        });
                    }
                }
            }
        }
    }

    let mut changed: BTreeSet<String> = BTreeSet::new();
    for path in &watched {
        if entry(before, path)? != entry(after, path)? {
            changed.insert(path.clone());
        }
    }
    let mut listed = None;
    for input in &inputs.checker_inputs {
        for path in changed_under(before, after, input, &mut listed)? {
            reasons.push(WideningReason {
                input: path.clone(),
                kind: ReasonKind::CheckerInputChanged,
                detail: format!("`{path}` is an input of this check; review the change before it becomes trusted"),
            });
            changed.insert(path);
        }
    }

    Ok(WideningReport {
        base: before.commit_id().unwrap_or_default(),
        head: after.commit_id().unwrap_or_default(),
        changed_inputs: changed.into_iter().collect(),
        potential_widening: !reasons.is_empty(),
        policies,
        reasons,
    })
}

/// The reasons `after` may grant more than `before`. Pure: equal blocks,
/// removed allows, added exclusions, and `allow` → `deny` give none.
#[must_use]
pub fn compare_policies(input: &str, before: &ResolvedPolicy, after: &ResolvedPolicy) -> Vec<WideningReason> {
    let mut reasons = Vec::new();
    let reason = |kind, detail| WideningReason { input: input.to_string(), kind, detail };
    for (list, before_entries, after_entries) in [
        ("file-read", &before.file_read, &after.file_read),
        ("shell-commands", &before.shell_commands, &after.shell_commands),
    ] {
        let before_set: BTreeSet<&String> = before_entries.iter().collect();
        let after_set: BTreeSet<&String> = after_entries.iter().collect();
        for added in after_set.difference(&before_set).filter(|entry| !entry.starts_with('!')) {
            reasons.push(reason(ReasonKind::AddedAllow, format!("`{list}` gains `{added}`")));
        }
        for removed in before_set.difference(&after_set).filter(|entry| entry.starts_with('!')) {
            reasons.push(reason(ReasonKind::RemovedExclusion, format!("`{list}` loses the exclusion `{removed}`")));
        }
    }
    if network_rank(after.network) > network_rank(before.network) {
        reasons.push(reason(
            ReasonKind::NetworkAllowed,
            format!("`network` changes from `{}` to `{}`", before.network, after.network),
        ));
    }
    reasons
}

/// One revision's resolved policy for `file`, and every repository-relative
/// path its reference followed.
fn resolve_policy(
    context: &FileResolutionContext,
    tree: &CommittedTreeSnapshot,
    root: &Path,
    file: &PolicyFile,
) -> Result<(ResolvedPolicy, Vec<String>), String> {
    let bytes = blob(tree, &file.path).map_err(|error| error.message)?;
    let path = root.join(&file.path);
    let whole = file.format == PolicyFormat::DarkmatterConfig;
    let (layer, dependencies) = committed_layer(context, tree, root, &path, bytes.as_deref(), whole).map_err(|error| error.to_string())?;
    let relative = |absolute: &Path| -> Option<String> {
        let names: Vec<String> = absolute.strip_prefix(root).ok()?.iter().map(|name| name.to_string_lossy().into_owned()).collect();
        Some(names.join("/"))
    };
    let followed: Vec<String> = dependencies.iter().filter_map(|dependency| relative(&dependency.path)).collect();
    let block_file = if layer.is_referenced() {
        relative(layer.block_file())
    } else if layer.permissions().file_read.authored
        || layer.permissions().shell_commands.authored
        || layer.permissions().network.authored
    {
        Some(file.path.clone())
    } else {
        None
    };
    Ok((normalize(layer.permissions(), block_file), followed))
}

/// Which default a configuration takes, for a default schema: the whole
/// block of a file with no `permissions`, or the per-entry defaults of a
/// block that omits an entry. A schema declares the two separately.
#[derive(Debug, Clone, Copy)]
enum DefaultProbe {
    NoBlock,
    EmptyBlock,
}

impl DefaultProbe {
    const fn label(self, scope: ConfigScope) -> &'static str {
        match (scope, self) {
            (ConfigScope::User, Self::NoBlock) => "user-block-default",
            (ConfigScope::User, Self::EmptyBlock) => "user-entry-defaults",
            (ConfigScope::Repository, Self::NoBlock) => "repository-block-default",
            (ConfigScope::Repository, Self::EmptyBlock) => "repository-entry-defaults",
        }
    }
}

/// The `probe` default of `scope` as the schema text `bytes` declares it;
/// an `Err` names why it cannot be read.
fn default_policy(
    context: &FileResolutionContext,
    bytes: Option<&[u8]>,
    schema: &DefaultSchema,
    scope: ConfigScope,
    probe: DefaultProbe,
) -> Result<ResolvedPolicy, String> {
    let bytes = bytes.ok_or_else(|| "the schema is absent".to_string())?;
    let text = std::str::from_utf8(bytes).map_err(|_| "the schema is not UTF-8".to_string())?;
    let document = match probe {
        DefaultProbe::NoBlock => serde_json::json!({}),
        DefaultProbe::EmptyBlock => serde_json::json!({ "permissions": {} }),
    };
    // Candidate schema text is passive data: parsed, never run.
    let defaults = match schema.format {
        SchemaFormat::DarkmatterConfig => {
            let catalog = SimplifiedSchema::try_from(text).map_err(|error| error.to_string())?;
            let contract = catalog.select(scope.config_type()).map_err(|error| error.to_string())?;
            contract.validate_strict(&document).map_err(|error| format!("{error:?}"))?.defaults
        }
        SchemaFormat::HostConfig => {
            let contract = prepare_host_config_type(text, &schema.path, scope.config_type()).map_err(|error| error.to_string())?;
            contract.validate_strict(&document).map_err(|error| format!("{error:?}"))?.defaults
        }
    };
    let mut block = match probe {
        DefaultProbe::NoBlock => defaults
            .iter()
            .find(|default| default.path.segments() == ["permissions"])
            .map(|default| default.value.clone())
            .ok_or_else(|| "it gives `permissions` no default".to_string())?,
        DefaultProbe::EmptyBlock => serde_json::json!({}),
    };
    if let DefaultProbe::EmptyBlock = probe {
        for default in &defaults {
            if let [first, key] = default.path.segments()
                && first == "permissions"
            {
                block[key.as_str()] = default.value.clone();
            }
        }
        for key in ["file-read", "shell-commands", "network"] {
            if block.get(key).is_none() {
                return Err(format!("it gives `permissions.{key}` no default"));
            }
        }
    }
    let block = typed_block(context, &block, Path::new(&schema.path), scope).map_err(|error| error.to_string())?;
    Ok(normalize(&block, None))
}

fn normalize(block: &ScopePermissions, block_file: Option<String>) -> ResolvedPolicy {
    let bang = |exclude: bool| if exclude { "!" } else { "" };
    let file_read: BTreeSet<String> = block
        .file_read
        .value
        .iter()
        .map(|entry| format!("{}{}", bang(entry.exclude), normal_glob(&entry.pattern)))
        .collect();
    let shell_commands: BTreeSet<String> = block
        .shell_commands
        .value
        .iter()
        .map(|entry| format!("{}{}", bang(entry.exclude), shell_rule_text(&entry.rule)))
        .collect();
    ResolvedPolicy {
        block_file,
        file_read: file_read.into_iter().collect(),
        shell_commands: shell_commands.into_iter().collect(),
        network: match block.network.value {
            NetworkAccess::Allow => "allow",
            NetworkAccess::Ask => "ask",
            NetworkAccess::Deny => "deny",
        },
    }
}

/// How much a resolved `network` setting lets through: `deny` < `ask` <
/// `allow`. A spelling the trusted code does not know ranks highest, so a
/// change to it is never proven narrowing.
fn network_rank(network: &str) -> u8 {
    match network {
        "deny" => 0,
        "ask" => 1,
        "allow" => 2,
        _ => 3,
    }
}

/// Spellings of one glob that are equal by construction: a doubled `/` and
/// an inner `/./` name the same path. A leading `./` is kept, since it is
/// part of the reference grammar.
fn normal_glob(pattern: &str) -> String {
    let mut normal = pattern.to_string();
    loop {
        let next = normal.replace("//", "/").replace("/./", "/");
        if next == normal {
            return normal;
        }
        normal = next;
    }
}

/// The committed bytes of a regular file at `path`; `None` when absent.
fn blob(tree: &CommittedTreeSnapshot, path: &str) -> Result<Option<Vec<u8>>, WideningError> {
    match entry(tree, path)? {
        CommittedEntry::Absent => Ok(None),
        CommittedEntry::Blob { bytes, .. } => Ok(Some(bytes)),
        CommittedEntry::Symlink { .. } => {
            Err(WideningError::new(format!("`{path}` is committed as a symbolic link, which is never followed")))
        }
        _ => Err(WideningError::new(format!("`{path}` is committed as a directory or submodule, not a file"))),
    }
}

fn entry(tree: &CommittedTreeSnapshot, path: &str) -> Result<CommittedEntry, WideningError> {
    tree.read_file(path).map_err(|error| WideningError::new(format!("`{path}` cannot be read: {error}")))
}

/// The paths at or below `input` whose entries differ between the trees.
/// `listed` holds each tree's paths once listed, shared across inputs.
fn changed_under(
    before: &CommittedTreeSnapshot,
    after: &CommittedTreeSnapshot,
    input: &str,
    listed: &mut Option<BTreeSet<String>>,
) -> Result<Vec<String>, WideningError> {
    let Some(prefix) = input.strip_suffix('/').map(|bare| format!("{bare}/")) else {
        return Ok(if entry(before, input)? == entry(after, input)? { Vec::new() } else { vec![input.to_string()] });
    };
    if listed.is_none() {
        let mut paths = BTreeSet::new();
        for tree in [before, after] {
            paths.extend(
                tree.paths().map_err(|error| WideningError::new(format!("the committed paths cannot be listed: {error}")))?,
            );
        }
        *listed = Some(paths);
    }
    let mut changed = Vec::new();
    for path in listed.iter().flatten().filter(|path| path.starts_with(&prefix)) {
        if entry(before, path)? != entry(after, path)? {
            changed.push(path.clone());
        }
    }
    Ok(changed)
}
