//! Shell policy adapter for harness runtime commands.
//!
//! Thin adapter that reuses Darkmatter's shell expansion infrastructure
//! for tokenization, blacklist/whitelist checking, and command approval.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::ShellCommandOrigin;
use darkmatter::markdown::compose::shell_expansion::template::{
    ApprovalKey, OfferedChoices, ShellCandidate, TemplateShape,
};
use darkmatter::markdown::compose::shell_expansion::tokenize::{ShellToken, tokenize};
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellExpansionError, ShellExpansionOptions,
    ShellPolicyPaths, ShellRuleSet, check_builtin_blacklist, check_template_blacklist,
    check_user_blacklist, check_whitelist, normalize_command, resolve_policy_paths, store,
};

use tracing::{debug, info_span};

use crate::harness::error::{HarnessError, ShellExecCause};
use crate::harness::model::ApprovedRuntimeCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedApprovalDecision {
    Allowed,
    Denied,
    Blacklisted,
}

/// Options for the shell approval flow.
#[derive(Clone)]
pub struct ShellApprovalOptions {
    /// Root directory for policy file resolution.
    pub policy_root: Option<PathBuf>,
    /// Callback for interactive command approval.
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    /// Session-local cache for repeated approval decisions, keyed by
    /// execution mode and policy root, so a resolved-byte decision never
    /// answers for a template or the reverse.
    pub approval_cache: Arc<Mutex<HashMap<ApprovalKey, CachedApprovalDecision>>>,
    /// When set, a non-TTY unapproved command is reported with dry-run
    /// framing (a `Cannot dry-run: …` message pointing at `--yolo` /
    /// pre-approval) instead of the generic "no approval handler" guidance.
    pub dry_run: bool,
}

impl Default for ShellApprovalOptions {
    fn default() -> Self {
        Self {
            policy_root: None,
            approval_handler: None,
            approval_cache: Arc::new(Mutex::new(HashMap::new())),
            dry_run: false,
        }
    }
}

/// Tokenize a raw command string and validate it against shell policies.
///
/// Single-command contract: chain operators (`&&`, `||`) and redirections
/// (`> /dev/null`, `2>&1`, etc.) are rejected. Chains are split into
/// individual commands upstream by Darkmatter's discovery layer before
/// reaching this validator.
///
/// ## Errors
///
/// Returns `HarnessError::ShellCommandDenied` if the command fails to
/// tokenize, is empty, or contains chain operators or redirections.
pub fn validate_and_approve_command(
    raw: &str,
    options: &ShellApprovalOptions,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let _span = info_span!("harness_shell_validate", command = %raw).entered();

    let tokens = tokenize_words_strict(raw).map_err(|_| HarnessError::ShellCommandDenied {
        command: raw.to_string(),
    })?;

    if tokens.is_empty() {
        return Err(HarnessError::ShellCommandDenied {
            command: raw.to_string(),
        });
    }

    validate_and_approve_command_parts(&tokens, options, None, None)
}

/// Tokenize a raw command string into Word-only parts.
///
/// Rejects chain operators and redirections so single-command callers
/// cannot accidentally validate a chain as if it were one command.
/// Chains are split per-command upstream by Darkmatter's discovery layer.
#[allow(clippy::result_large_err)]
pub(crate) fn tokenize_words_strict(raw: &str) -> Result<Vec<String>, ShellExpansionError> {
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("<harness-shell>"),
        PathBuf::from("<harness-shell>"),
        raw.to_string(),
    );
    let tokens = tokenize(raw, &ctx)?;
    let mut words = Vec::with_capacity(tokens.len());
    for tok in tokens {
        match tok {
            ShellToken::Word(w) => words.push(w),
            _ => {
                return Err(ShellExpansionError::ParseDirective {
                    ctx: Box::new(ctx.clone()),
                    origin: ShellCommandOrigin::Body { line: 0 },
                    message: format!(
                        "chain operators and redirections are not allowed here: {raw}"
                    ),
                });
            }
        }
    }
    Ok(words)
}

/// The shell policy files under `policy_root`.
///
/// Darkmatter reads an explicit policy root directly; the context it is
/// handed is anchored there and never consulted.
fn policy_paths_at(
    policy_root: &Path,
) -> Option<darkmatter::markdown::compose::shell_expansion::ShellPolicyPaths> {
    let shell_opts = ShellExpansionOptions {
        policy_root: Some(policy_root.to_path_buf()),
        ..Default::default()
    };
    let context = darkmatter::markdown::compose::build_resolution_context_with_catalog(
        &darkmatter::markdown::compose::RequestSnapshot::new(policy_root),
        None,
    )
    .ok()?;
    resolve_policy_paths(&shell_opts, &ComposeSource::File(policy_root.join("dummy")), &context).ok()
}

/// Validate an already-tokenized command against shell policies.
///
/// This is used for structured command inputs such as JSON arrays or expanded
/// objects where the caller has already separated the executable from its args.
pub fn validate_and_approve_command_parts(
    parts: &[String],
    options: &ShellApprovalOptions,
    source_file: Option<&Path>,
    source_line: Option<usize>,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let raw = parts.join(" ");
    let _span = info_span!(
        "harness_shell_validate",
        command = %raw,
        ?source_file,
        source_line
    )
    .entered();

    if parts.is_empty() {
        return Err(HarnessError::ShellCommandDenied {
            command: String::new(),
        });
    }

    let executable = &parts[0];
    let args: Vec<String> = parts[1..].to_vec();
    let normalized = normalize_command(executable, &args);
    let approved = || ApprovedRuntimeCommand {
        raw: raw.clone(),
        executable: executable.to_string(),
        args: args.clone(),
    };

    // Check built-in blacklist
    if let Some(reason) = check_builtin_blacklist(executable, &args) {
        debug!(reason = %reason, "shell command blocked by builtin blacklist");
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.clone(),
            reason,
        });
    }

    let policy = SessionPolicy::load(options, &raw)?;
    if check_user_blacklist(&policy.blacklist, executable, &args, &normalized) {
        debug!("shell command blocked by user blacklist");
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.clone(),
            reason: "command matches user blacklist".to_string(),
        });
    }

    // Check whitelist — if whitelisted, approve immediately
    if check_whitelist(&policy.whitelist, executable, &normalized) {
        debug!("shell command approved via whitelist");
        return Ok(approved());
    }

    let key = ApprovalKey::exact(options.policy_root.as_deref(), normalized.clone());
    if let Some(decision) = cached_decision(&options.approval_cache, &key) {
        debug!(?decision, "shell command decided via cache");
        return decision_result(decision, &raw).map(|()| approved());
    }

    // If not whitelisted, invoke approval handler. A decision is recorded in
    // the policy files, so a handler needs a policy root.
    let (Some(handler), Some(policy_paths)) = (&options.approval_handler, &policy.paths) else {
        debug!("no approval handler and not whitelisted, denying command");
        return Err(HarnessError::ShellCommandDenied { command: raw });
    };
    let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
        source: display_source(options, source_file),
        origin: ShellCommandOrigin::Body {
            line: source_line.unwrap_or(0),
        },
        raw_command: raw.clone(),
        executable: executable.to_string(),
        args: args.clone(),
        normalized_exact: normalized.clone(),
        whitelist_path: policy_paths.whitelist.clone(),
        blacklist_path: policy_paths.blacklist.clone(),
        alias_name: None,
        chain_executables: Vec::new(),
        offered: OfferedChoices::resolved_bytes(),
        template: None,
    };
    let Ok(decision) = handler.approve(request) else {
        debug!("handler returned error, denying command");
        return Err(HarnessError::ShellCommandDenied { command: raw });
    };
    // A policy write that fails is reported, and nothing is cached: the
    // decision is neither a persistent approval nor an implicit once.
    let cached = match decision {
        ShellApprovalDecision::AllowExactPersist => {
            debug!("handler approved (AllowExactPersist), persisting to whitelist");
            persist(store::append_whitelist_exact(policy_paths, &normalized), policy_paths)?;
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowCommandPersist => {
            debug!("handler approved (AllowCommandPersist), persisting to whitelist");
            persist(store::append_whitelist_prefix(policy_paths, executable), policy_paths)?;
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowOnce => {
            debug!("handler approved (AllowOnce)");
            CachedApprovalDecision::Allowed
        }
        // The shape choice is never offered for resolved bytes.
        ShellApprovalDecision::Deny | ShellApprovalDecision::AllowTemplatePersist => {
            debug!("handler denied command");
            CachedApprovalDecision::Denied
        }
        ShellApprovalDecision::BlacklistPersist => {
            debug!("handler blacklisted command");
            persist(store::append_blacklist_exact(policy_paths, &normalized), policy_paths)?;
            CachedApprovalDecision::Blacklisted
        }
    };
    cache_approval_decision(&options.approval_cache, key, cached);
    fresh_decision_result(cached, &raw)?;
    debug!("shell command approved");
    Ok(approved())
}

/// Approves a command site by its template shape, for a site that may run
/// only as a template (a placeholder's value is produced later in the run).
///
/// Policy order matches resolved-byte approval: the built-in and user
/// blacklists (by shape, keeping each placeholder's position), then a
/// `template` whitelist rule, then `prefix` rules for every fixed program,
/// then the session cache, then the handler. An exact rule never covers a
/// template site.
///
/// ## Returns
///
/// The approved shape.
///
/// ## Errors
///
/// `ShellCommandBlacklisted`, `ShellCommandDenied` (also when the handler
/// returns a choice the request did not offer), or `ShellPolicyWriteFailed`
/// when a persistent decision cannot be written.
pub fn validate_and_approve_template_site(
    candidate: &ShellCandidate,
    options: &ShellApprovalOptions,
) -> Result<TemplateShape, HarnessError> {
    let command = candidate.display_command();
    let _span = info_span!("harness_shell_validate_template", command = %command).entered();
    let Some(shape) = candidate.shape() else {
        return Err(HarnessError::ShellCommandDenied { command });
    };

    let policy = SessionPolicy::load(options, &command)?;
    if let Some(reason) = check_template_blacklist(&shape, &policy.blacklist) {
        return Err(HarnessError::ShellCommandBlacklisted { command, reason });
    }
    let programs: Vec<&str> = shape.segments.iter().map(|segment| segment.program.as_str()).collect();
    if policy.whitelist.template_covers(&shape)
        || programs.iter().all(|program| policy.whitelist.matches_prefix(program))
    {
        debug!("template site approved via whitelist");
        return Ok(shape);
    }

    let key = ApprovalKey::template(options.policy_root.as_deref(), shape.clone());
    if let Some(decision) = cached_decision(&options.approval_cache, &key) {
        return decision_result(decision, &command).map(|()| shape);
    }

    let (Some(handler), Some(policy_paths)) = (&options.approval_handler, &policy.paths) else {
        return Err(HarnessError::ShellCommandDenied { command });
    };
    let mut distinct = std::collections::HashSet::new();
    let chain_executables: Vec<String> = programs
        .iter()
        .filter(|program| distinct.insert(**program))
        .map(|program| program.to_string())
        .collect();
    let offered = candidate.offered();
    let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
        source: ComposeSource::File(candidate.site.source_file.clone()),
        origin: candidate.site.origin.clone(),
        raw_command: command.clone(),
        executable: programs.first().map(|program| program.to_string()).unwrap_or_default(),
        args: Vec::new(),
        normalized_exact: String::new(),
        whitelist_path: policy_paths.whitelist.clone(),
        blacklist_path: policy_paths.blacklist.clone(),
        alias_name: None,
        chain_executables: if chain_executables.len() > 1 { chain_executables } else { Vec::new() },
        offered,
        template: candidate.approval_view(),
    };
    let Ok(decision) = handler.approve(request) else {
        return Err(HarnessError::ShellCommandDenied { command });
    };
    let cached = match decision {
        ShellApprovalDecision::AllowTemplatePersist if offered.shape => {
            persist(store::append_whitelist_template(policy_paths, &shape), policy_paths)?;
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowCommandPersist if offered.prefix => {
            let mut persisted = std::collections::HashSet::new();
            for program in &programs {
                if persisted.insert(*program) {
                    persist(store::append_whitelist_prefix(policy_paths, program), policy_paths)?;
                }
            }
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowOnce => CachedApprovalDecision::Allowed,
        ShellApprovalDecision::BlacklistPersist => {
            persist(store::append_blacklist_template(policy_paths, &shape), policy_paths)?;
            CachedApprovalDecision::Blacklisted
        }
        // Deny, or a choice the request did not offer (exact bytes do not
        // exist for this site).
        _ => CachedApprovalDecision::Denied,
    };
    cache_approval_decision(&options.approval_cache, key, cached);
    fresh_decision_result(cached, &command).map(|()| shape)
}

/// The shape that approves a resolved-byte site by a `template` whitelist
/// rule, when no `exact` rule covers all of its bytes.
///
/// `None` when the site has no placeholder, an exact rule covers it, or no
/// template rule does; its bytes are then approved as resolved bytes.
///
/// ## Errors
///
/// `ShellCommandBlacklisted` when the shape matches the built-in or a user
/// blacklist.
pub fn template_rule_covers(
    candidate: &ShellCandidate,
    options: &ShellApprovalOptions,
) -> Result<Option<TemplateShape>, HarnessError> {
    let (Some(shape), Some(keys)) = (candidate.shape(), &candidate.resolved) else {
        return Ok(None);
    };
    let command = candidate.display_command();
    let policy = SessionPolicy::load(options, &command)?;
    if keys.iter().all(|key| policy.whitelist.matches_exact(key)) || !policy.whitelist.template_covers(&shape) {
        return Ok(None);
    }
    if let Some(reason) = check_template_blacklist(&shape, &policy.blacklist) {
        return Err(HarnessError::ShellCommandBlacklisted { command, reason });
    }
    Ok(Some(shape))
}

/// How the user approved a resolved-byte site at its prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedSiteApproval {
    /// Exact, prefix, or once: the site runs as its resolved bytes, which
    /// the session cache (or the saved rule) now approves.
    Bytes,
    /// The command shape: the site runs as a template.
    Shape(TemplateShape),
}

/// Asks the handler about a resolved-byte site that has a placeholder,
/// offering every choice the site is eligible for: its exact bytes, its
/// shape, and its programs with any arguments.
///
/// An authored program placeholder withdraws prefix approval even when the
/// site has no valid template shape. Nothing is asked, and `None` returned,
/// when the site has neither a shape nor a program placeholder, when a
/// whitelist rule or a session decision already covers every one of its
/// commands, when a command's bytes are blacklisted, or when there is no
/// handler or policy root. The caller then approves the bytes as resolved
/// bytes, which reports those outcomes.
///
/// ## Errors
///
/// `ShellCommandBlacklisted` when the shape matches a blacklist or the user
/// blacklists the command, `ShellCommandDenied` on a denial or a choice the
/// request did not offer, and `ShellPolicyWriteFailed` when a persistent
/// decision cannot be written (nothing is cached then).
pub fn prompt_resolved_site(
    candidate: &ShellCandidate,
    options: &ShellApprovalOptions,
) -> Result<Option<ResolvedSiteApproval>, HarnessError> {
    let Some(keys) = &candidate.resolved else {
        return Ok(None);
    };
    let shape = candidate.shape();
    if shape.is_none() && candidate.program_words_fixed() {
        return Ok(None);
    }
    let offered = candidate.offered();
    if !offered.exact {
        return Ok(None);
    }
    let commands = match keys.iter().map(|key| tokenize_words_strict(key))
        .collect::<Result<Vec<Vec<String>>, _>>() {
        Ok(commands) => commands,
        // Native-shell adapters retain authored shell text as one approval key.
        // Strict tokenization must not erase the site's program restrictions.
        Err(_) if shape.is_none() || !offered.prefix => keys.iter().map(|key| vec![key.clone()]).collect(),
        Err(_) => return Ok(None),
    };
    let command = candidate.display_command();
    let _span = info_span!("harness_shell_prompt_site", command = %command).entered();
    let policy = SessionPolicy::load(options, &command)?;
    let exact_key = |key: &String| ApprovalKey::exact(options.policy_root.as_deref(), key.clone());
    let mut undecided = false;
    for (key, parts) in keys.iter().zip(&commands) {
        let Some((executable, args)) = parts.split_first() else {
            return Ok(None);
        };
        if check_builtin_blacklist(executable, args).is_some()
            || check_user_blacklist(&policy.blacklist, executable, args, key)
        {
            return Ok(None);
        }
        undecided |= !check_whitelist(&policy.whitelist, executable, key)
            && cached_decision(&options.approval_cache, &exact_key(key)).is_none();
    }
    if !undecided {
        return Ok(None);
    }
    let template_key = shape.as_ref().map(|shape| ApprovalKey::template(options.policy_root.as_deref(), shape.clone()));
    if let Some(shape) = &shape {
        if let Some(reason) = check_template_blacklist(shape, &policy.blacklist) {
            return Err(HarnessError::ShellCommandBlacklisted { command, reason });
        }
        if let Some(decision) = cached_decision(&options.approval_cache, template_key.as_ref().expect("shape key")) {
            return decision_result(decision, &command).map(|()| Some(ResolvedSiteApproval::Shape(shape.clone())));
        }
    }
    let (Some(handler), Some(policy_paths)) = (&options.approval_handler, &policy.paths) else {
        return Ok(None);
    };

    let mut distinct = std::collections::HashSet::new();
    let executables: Vec<&str> = commands
        .iter()
        .filter_map(|parts| parts.first().map(String::as_str))
        .filter(|executable| distinct.insert(*executable))
        .collect();
    let first = &commands[0];
    let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
        source: ComposeSource::File(candidate.site.source_file.clone()),
        origin: candidate.site.origin.clone(),
        raw_command: command.clone(),
        executable: first[0].clone(),
        args: first[1..].to_vec(),
        normalized_exact: keys.join(" && "),
        whitelist_path: policy_paths.whitelist.clone(),
        blacklist_path: policy_paths.blacklist.clone(),
        alias_name: None,
        chain_executables: if executables.len() > 1 {
            executables.iter().map(|executable| executable.to_string()).collect()
        } else {
            Vec::new()
        },
        offered,
        template: candidate.approval_view(),
    };
    let Ok(decision) = handler.approve(request) else {
        return Err(HarnessError::ShellCommandDenied { command });
    };
    let cache_bytes = |decision: CachedApprovalDecision| {
        for key in keys {
            cache_approval_decision(&options.approval_cache, exact_key(key), decision);
        }
    };
    match decision {
        ShellApprovalDecision::AllowExactPersist => {
            for key in keys {
                persist(store::append_whitelist_exact(policy_paths, key), policy_paths)?;
            }
            cache_bytes(CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Bytes))
        }
        ShellApprovalDecision::AllowTemplatePersist if offered.shape => {
            let shape = shape.expect("offered shape");
            persist(store::append_whitelist_template(policy_paths, &shape), policy_paths)?;
            cache_approval_decision(&options.approval_cache, template_key.expect("shape key"), CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Shape(shape)))
        }
        ShellApprovalDecision::AllowCommandPersist if offered.prefix => {
            for executable in &executables {
                persist(store::append_whitelist_prefix(policy_paths, executable), policy_paths)?;
            }
            cache_bytes(CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Bytes))
        }
        ShellApprovalDecision::AllowOnce => {
            cache_bytes(CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Bytes))
        }
        ShellApprovalDecision::BlacklistPersist => {
            for key in keys {
                persist(store::append_blacklist_exact(policy_paths, key), policy_paths)?;
            }
            cache_bytes(CachedApprovalDecision::Blacklisted);
            fresh_decision_result(CachedApprovalDecision::Blacklisted, &command).map(|()| None)
        }
        // Deny, or a choice the request did not offer.
        _ => {
            cache_bytes(CachedApprovalDecision::Denied);
            Err(HarnessError::ShellCommandDenied { command })
        }
    }
}

/// Checks resolved bytes against the built-in and user blacklists only, for
/// a command whose site is approved by its shape.
///
/// ## Errors
///
/// `ShellCommandBlacklisted` on a match.
pub fn check_command_blacklists(parts: &[String], options: &ShellApprovalOptions) -> Result<(), HarnessError> {
    let Some((executable, args)) = parts.split_first() else {
        return Ok(());
    };
    let raw = parts.join(" ");
    if let Some(reason) = check_builtin_blacklist(executable, args) {
        return Err(HarnessError::ShellCommandBlacklisted { command: raw, reason });
    }
    let policy = SessionPolicy::load(options, &raw)?;
    if check_user_blacklist(&policy.blacklist, executable, args, &normalize_command(executable, args)) {
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw,
            reason: "command matches user blacklist".to_string(),
        });
    }
    Ok(())
}

/// The user blacklist under `policy_root`, which a template-mode command's
/// filled arguments are checked against before it starts. Empty without a
/// policy root, or when the file cannot be read; the built-in blacklist still
/// applies.
pub fn user_blacklist(policy_root: Option<&Path>) -> ShellRuleSet {
    let options = ShellApprovalOptions {
        policy_root: policy_root.map(Path::to_path_buf),
        ..ShellApprovalOptions::default()
    };
    SessionPolicy::load(&options, "").map(|policy| policy.blacklist).unwrap_or_default()
}

/// The user policy one approval reads: the rule files under the policy root,
/// or none when there is no root.
struct SessionPolicy {
    paths: Option<ShellPolicyPaths>,
    whitelist: ShellRuleSet,
    blacklist: ShellRuleSet,
}

impl SessionPolicy {
    /// Without a policy root there are no user policy files to consult: the
    /// built-in blacklist, the session cache, and the approval handler decide.
    fn load(options: &ShellApprovalOptions, command: &str) -> Result<Self, HarnessError> {
        let paths = match &options.policy_root {
            Some(policy_root) => Some(policy_paths_at(policy_root).ok_or_else(|| {
                HarnessError::ShellCommandDenied {
                    command: command.to_string(),
                }
            })?),
            None => None,
        };
        let load = |path: Option<&Path>| path.and_then(|path| store::load_ruleset(path).ok()).unwrap_or_default();
        Ok(Self {
            whitelist: load(paths.as_ref().map(|paths| paths.whitelist.as_path())),
            blacklist: load(paths.as_ref().map(|paths| paths.blacklist.as_path())),
            paths,
        })
    }
}

/// Display source for approval prompts — real provenance when available.
fn display_source(options: &ShellApprovalOptions, source_file: Option<&Path>) -> ComposeSource {
    match (source_file, &options.policy_root) {
        (Some(path), _) => ComposeSource::File(path.to_path_buf()),
        (None, Some(root)) => ComposeSource::File(root.join("dummy")),
        (None, None) => ComposeSource::Unknown,
    }
}

fn persist(result: Result<(), ShellExpansionError>, paths: &ShellPolicyPaths) -> Result<(), HarnessError> {
    result.map_err(|source| HarnessError::ShellPolicyWriteFailed {
        path: match &source {
            ShellExpansionError::PolicyIo { path, .. } => path.clone(),
            _ => paths.whitelist.clone(),
        },
        source: Box::new(source),
    })
}

fn cached_decision(
    cache: &Arc<Mutex<HashMap<ApprovalKey, CachedApprovalDecision>>>,
    key: &ApprovalKey,
) -> Option<CachedApprovalDecision> {
    cache.lock().unwrap_or_else(|e| e.into_inner()).get(key).copied()
}

/// [`decision_result`] for a decision the handler just made.
fn fresh_decision_result(decision: CachedApprovalDecision, command: &str) -> Result<(), HarnessError> {
    match decision {
        CachedApprovalDecision::Blacklisted => Err(HarnessError::ShellCommandBlacklisted {
            command: command.to_string(),
            reason: "user blacklisted this command".to_string(),
        }),
        decision => decision_result(decision, command),
    }
}

/// The outcome of a cached decision.
fn decision_result(decision: CachedApprovalDecision, command: &str) -> Result<(), HarnessError> {
    match decision {
        CachedApprovalDecision::Allowed => Ok(()),
        CachedApprovalDecision::Denied => Err(HarnessError::ShellCommandDenied {
            command: command.to_string(),
        }),
        CachedApprovalDecision::Blacklisted => Err(HarnessError::ShellCommandBlacklisted {
            command: command.to_string(),
            reason: "command was previously blacklisted during this session".to_string(),
        }),
    }
}

fn cache_approval_decision(
    cache: &Arc<Mutex<HashMap<ApprovalKey, CachedApprovalDecision>>>,
    key: ApprovalKey,
    decision: CachedApprovalDecision,
) {
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    cache.insert(key, decision);
}

/// Execute an approved command asynchronously and return its exit code and stdout/stderr.
///
/// Uses `tokio::process::Command` so the async runtime can await the child
/// process without busy-waiting or blocking a worker thread.
///
/// Enforces the given timeout: if the command does not complete within the
/// duration, it is killed with SIGKILL and an error is returned.
///
/// Returns `(exit_code, stdout, stderr)`.
pub async fn execute_approved_command(
    command: &ApprovedRuntimeCommand,
    working_dir: Option<&std::path::Path>,
    timeout: std::time::Duration,
) -> Result<(i32, String, String), HarnessError> {
    let _span = info_span!(
        "harness_shell_execute",
        command = %command.raw,
        timeout_secs = timeout.as_secs()
    )
    .entered();

    let start = Instant::now();
    let exe = which::which(&command.executable).map_err(|error| {
        HarnessError::ShellCommandExecutionFailed {
            detail: format!("executable '{}' not found in PATH", command.executable),
            source: ShellExecCause::Which(error),
        }
    })?;

    let mut cmd = crate::child_environment::tokio_command(&exe).map_err(|error| {
        HarnessError::ShellCommandExecutionFailed {
            detail: format!("failed to prepare '{}': {error}", command.executable),
            source: ShellExecCause::Spawn(std::io::Error::other(error)),
        }
    })?;
    cmd.args(&command.args);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|error| HarnessError::ShellCommandExecutionFailed {
        detail: format!("failed to spawn '{}': {error}", command.executable),
        source: ShellExecCause::Spawn(error),
    })?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let result = tokio::time::timeout(timeout, child.wait()).await;

    match result {
        Ok(Ok(status)) => {
            let exit_code = status.code().unwrap_or(-1);
            let duration_ms = start.elapsed().as_millis() as u64;

            let stdout = match stdout_pipe {
                Some(mut pipe) => {
                    use tokio::io::AsyncReadExt;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf).await;
                    String::from_utf8_lossy(&buf).to_string()
                }
                None => String::new(),
            };

            let stderr = match stderr_pipe {
                Some(mut pipe) => {
                    use tokio::io::AsyncReadExt;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf).await;
                    String::from_utf8_lossy(&buf).to_string()
                }
                None => String::new(),
            };

            debug!(
                exit_code,
                duration_ms,
                stdout_len = stdout.len(),
                stderr_len = stderr.len(),
                "shell command completed"
            );
            Ok((exit_code, stdout, stderr))
        }
        Ok(Err(error)) => {
            debug!(error = %error, "shell command wait failed");
            Err(HarnessError::ShellCommandExecutionFailed {
                detail: format!("failed to wait for '{}': {error}", command.executable),
                source: ShellExecCause::Wait(error),
            })
        }
        Err(elapsed) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            debug!(timeout_secs = timeout.as_secs(), "shell command timed out");
            Err(HarnessError::ShellCommandExecutionFailed {
                detail: format!(
                    "command '{}' timed out after {}s",
                    command.raw,
                    timeout.as_secs()
                ),
                source: ShellExecCause::Timeout(elapsed),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::shell_expansion::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CapturingHandler {
        captured: Arc<Mutex<Option<ShellApprovalRequest>>>,
    }

    impl CapturingHandler {
        fn new() -> Self {
            Self {
                captured: Arc::new(Mutex::new(None)),
            }
        }

        fn captured_request(&self) -> ShellApprovalRequest {
            self.captured
                .lock()
                .unwrap()
                .clone()
                .expect("handler was never called")
        }
    }

    impl ShellApprovalHandler for CapturingHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            *self.captured.lock().unwrap() = Some(request);
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    struct CountingApprovalHandler {
        approvals: AtomicUsize,
    }

    impl CountingApprovalHandler {
        fn approvals(&self) -> usize {
            self.approvals.load(Ordering::SeqCst)
        }
    }

    impl ShellApprovalHandler for CountingApprovalHandler {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.approvals.fetch_add(1, Ordering::SeqCst);
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    #[test]
    fn unknown_command_denied_without_approval_handler() {
        // Use an isolated directory with no policy files and no approval handler.
        let dir = tempfile::TempDir::new().unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };
        let result = validate_and_approve_command("echo hello", &options);
        // Without whitelist or approval handler, this should be denied
        assert!(result.is_err());
    }

    #[test]
    fn blacklisted_command_rejected() {
        let options = ShellApprovalOptions::default();
        let result = validate_and_approve_command("rm -rf /", &options);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HarnessError::ShellCommandBlacklisted { .. }
        ));
    }

    #[test]
    fn empty_command_rejected() {
        let options = ShellApprovalOptions::default();
        let result = validate_and_approve_command("", &options);
        assert!(result.is_err());
    }

    #[test]
    fn shell_metacharacters_rejected() {
        let options = ShellApprovalOptions::default();
        let result = validate_and_approve_command("echo hello | cat", &options);
        assert!(result.is_err());
    }

    #[test]
    fn chain_operators_rejected_at_single_command_slot() {
        // Chains are split per-command upstream by Darkmatter discovery; a
        // raw chain reaching this validator means a misuse and must error.
        let options = ShellApprovalOptions::default();
        for raw in [
            "git status && git push",
            "make build || echo failed",
            "ls > /dev/null",
            "cmd 2>&1",
        ] {
            let result = validate_and_approve_command(raw, &options);
            assert!(
                matches!(result, Err(HarnessError::ShellCommandDenied { .. })),
                "expected denial for {raw:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn approval_cache_avoids_prompting_for_same_command_twice() {
        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CountingApprovalHandler {
            approvals: AtomicUsize::new(0),
        });
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let first = validate_and_approve_command("echo hello", &options);
        let second = validate_and_approve_command("echo hello", &options);

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert_eq!(handler.approvals(), 1);
    }

    #[tokio::test]
    async fn execute_echo_command() {
        let cmd = ApprovedRuntimeCommand {
            raw: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        };
        let result = execute_approved_command(&cmd, None, std::time::Duration::from_secs(5)).await;
        assert!(result.is_ok());
        let (exit_code, stdout, _stderr) = result.unwrap();
        assert_eq!(exit_code, 0);
        assert!(stdout.trim() == "hello");
    }

    #[tokio::test]
    async fn execute_failing_command() {
        let cmd = ApprovedRuntimeCommand {
            raw: "false".to_string(),
            executable: "false".to_string(),
            args: vec![],
        };
        let result = execute_approved_command(&cmd, None, std::time::Duration::from_secs(5)).await;
        assert!(result.is_ok());
        let (exit_code, _, _) = result.unwrap();
        assert_ne!(exit_code, 0);
    }

    #[tokio::test]
    async fn approved_command_observes_agent_cwd() {
        let expected = crate::child_environment::initialize_process_launch_directory(
            crate::child_environment::LaunchDirectoryMode::Ordinary,
        )
        .unwrap()
        .to_string_lossy()
        .into_owned();
        let cmd = agent_cwd_command();
        let (exit_code, stdout, _) = execute_approved_command(
            &cmd,
            None,
            std::time::Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(exit_code, 0);
        assert_eq!(stdout.trim(), expected);
    }

    #[cfg(windows)]
    fn agent_cwd_command() -> ApprovedRuntimeCommand {
        ApprovedRuntimeCommand {
            raw: "echo %AGENT_CWD%".to_string(),
            executable: "cmd.exe".to_string(),
            args: vec!["/D".to_string(), "/C".to_string(), "echo %AGENT_CWD%".to_string()],
        }
    }

    #[cfg(not(windows))]
    fn agent_cwd_command() -> ApprovedRuntimeCommand {
        ApprovedRuntimeCommand {
            raw: "printf %s \"$AGENT_CWD\"".to_string(),
            executable: "sh".to_string(),
            args: vec!["-c".to_string(), "printf %s \"$AGENT_CWD\"".to_string()],
        }
    }

    #[test]
    fn provenance_is_passed_through_to_approval_request() {
        use darkmatter::markdown::compose::ComposeSource;

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CapturingHandler::new());
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let source_file = PathBuf::from("/home/user/project/template.md");
        let source_line = 42usize;

        let _ = validate_and_approve_command_parts(
            &["echo".to_string(), "hello".to_string()],
            &options,
            Some(&source_file),
            Some(source_line),
        );

        let captured = handler.captured_request();
        assert_eq!(
            captured.source,
            ComposeSource::File(source_file),
            "request should carry the real source file, not a dummy path"
        );
        assert_eq!(
            captured.origin,
            darkmatter::markdown::compose::ShellCommandOrigin::Body { line: 42 },
            "request should carry the real line number"
        );
    }

    #[test]
    fn frozen_options_deny_new_commands_after_cache_populated() {
        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CountingApprovalHandler {
            approvals: AtomicUsize::new(0),
        });
        let mut options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        // Approve a command while the handler is active — populates the cache.
        let first = validate_and_approve_command("echo hello", &options);
        assert!(first.is_ok(), "should approve with active handler");
        assert_eq!(handler.approvals(), 1);

        // Freeze: strip the handler (simulates CachedHarnessLoopContext::freeze_shell_approvals).
        options.approval_handler = None;

        // Previously-approved command still passes via cache.
        let cached = validate_and_approve_command("echo hello", &options);
        assert!(
            cached.is_ok(),
            "cached command should still pass after freeze"
        );

        // A NEW command that was never approved is denied — no handler to prompt.
        let new_cmd = validate_and_approve_command("curl https://example.com", &options);
        assert!(
            new_cmd.is_err(),
            "new command must be denied after freeze — no handler available"
        );
        assert!(matches!(
            new_cmd.unwrap_err(),
            HarnessError::ShellCommandDenied { .. }
        ));

        // Handler was only called once (for the original "echo hello"), never for "curl".
        assert_eq!(
            handler.approvals(),
            1,
            "handler must not be called after freeze"
        );
    }
}
