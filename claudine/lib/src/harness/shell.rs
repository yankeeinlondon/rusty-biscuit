//! Shell policy adapter for harness runtime commands.
//!
//! Thin adapter that reuses Darkmatter's shell expansion infrastructure for
//! tokenization, the built-in blacklist, rule matching, and command approval.
//! Every decision follows `md`'s order: the built-in blacklist refuses first
//! (before yolo, cached approvals, and pre-approvals); then the captured
//! `permissions.shell-commands` pre-approve a command when every segment is
//! allowed and no `!` entry matches; otherwise this run's cached decision or
//! the approval handler decides. An invalid `permissions` block fails every
//! command that reaches that decision.

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
    ShellApprovalDecision, ShellApprovalHandler, ShellExpansionError, ShellRuleEntry, ShellRuleSet,
    check_builtin_blacklist, check_template_blacklist, check_template_exclusion, check_user_blacklist,
    check_whitelist, normalize_command,
};

use tracing::{debug, info_span};

use crate::harness::error::{HarnessError, ShellExecCause};
use crate::harness::model::ApprovedRuntimeCommand;

mod policy;

pub use policy::CapturedPermissions;
use policy::Decider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedApprovalDecision {
    Allowed,
    Denied,
}

/// Options for the shell approval flow.
#[derive(Clone)]
pub struct ShellApprovalOptions {
    /// The captured `permissions` of the source document's configuration.
    pub permissions: CapturedPermissions,
    /// Callback for interactive command approval.
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    /// Session-local cache for repeated approval decisions, keyed by
    /// execution mode, so a resolved-byte decision never answers for a
    /// template or the reverse.
    pub approval_cache: Arc<Mutex<HashMap<ApprovalKey, CachedApprovalDecision>>>,
    /// The rules this run's "always allow" choices added. They apply to the
    /// rest of the run whether or not they could be saved, because the
    /// captured policy is never reloaded mid-run. Shared by every clone.
    pub run_rules: Arc<Mutex<ShellRuleSet>>,
    /// When set, a non-TTY unapproved command is reported with dry-run
    /// framing (a `Cannot dry-run: …` message pointing at `--yolo` /
    /// pre-approval) instead of the generic "no approval handler" guidance.
    pub dry_run: bool,
    /// Closed invocation membership: later discovery may only reuse these approvals.
    pub closed_approvals: Option<darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals>,
}

impl Default for ShellApprovalOptions {
    fn default() -> Self {
        Self {
            permissions: CapturedPermissions::default(),
            approval_handler: None,
            approval_cache: Arc::new(Mutex::new(HashMap::new())),
            run_rules: Arc::new(Mutex::new(ShellRuleSet::default())),
            dry_run: false,
            closed_approvals: None,
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

/// Validate an already-tokenized command against shell policies.
///
/// This is used for structured command inputs such as JSON arrays or expanded
/// objects where the caller has already separated the executable from its args.
///
/// ## Errors
///
/// `ShellCommandBlacklisted` for a built-in refusal, `ShellPermissionsInvalid`
/// when the captured `permissions` are invalid, and `ShellCommandDenied` when
/// nothing approves the command.
pub fn validate_and_approve_command_parts(
    parts: &[String],
    options: &ShellApprovalOptions,
    source_file: Option<&Path>,
    source_line: Option<usize>,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let origin = ShellCommandOrigin::Body { line: source_line.unwrap_or(0) };
    validate_and_approve_command_at(parts, options, source_file, origin)
}

/// [`validate_and_approve_command_parts`] for a command whose approval prompt
/// shows `origin`, such as a frontmatter property rather than a body line.
pub(crate) fn validate_and_approve_command_at(
    parts: &[String],
    options: &ShellApprovalOptions,
    source_file: Option<&Path>,
    origin: ShellCommandOrigin,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    // The display form is the exact key, so a quoted or empty argument reads
    // as the bytes it approves (`git commit -m ""`), not as a bare join.
    let raw = match parts.split_first() {
        Some((executable, args)) => normalize_command(executable, args),
        None => String::new(),
    };
    let _span = info_span!(
        "harness_shell_validate",
        command = %raw,
        ?source_file,
        %origin
    )
    .entered();

    if parts.is_empty() {
        return Err(HarnessError::ShellCommandDenied {
            command: String::new(),
        });
    }

    let executable = &parts[0];
    let args: Vec<String> = parts[1..].to_vec();
    let normalized = raw.clone();
    let approved = || ApprovedRuntimeCommand {
        raw: raw.clone(),
        executable: executable.to_string(),
        args: args.clone(),
    };

    if let Some(reason) = check_builtin_blacklist(executable, &args) {
        debug!(reason = %reason, "shell command blocked by builtin blacklist");
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.clone(),
            reason,
        });
    }

    let decider = Decider::new(options)?;
    let exclusion = decider.exclusion(|rules| check_user_blacklist(rules, executable, &args, &normalized));
    if exclusion.is_none() && decider.allows(|rules| check_whitelist(rules, executable, &normalized)) {
        debug!("shell command pre-approved by permissions");
        return Ok(approved());
    }

    let key = ApprovalKey::exact(normalized.clone());
    if let Some(decision) = cached_decision(&options.approval_cache, &key) {
        debug!(?decision, "shell command decided via cache");
        return decision_result(decision, &raw).map(|()| approved());
    }

    let Some(handler) = &options.approval_handler else {
        debug!("no approval handler and not pre-approved, denying command");
        return Err(HarnessError::ShellCommandDenied { command: raw });
    };
    let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
        source: display_source(source_file),
        origin,
        raw_command: raw.clone(),
        executable: executable.to_string(),
        args: args.clone(),
        normalized_exact: normalized.clone(),
        save_target: decider.save_target(),
        exclusion,
        alias_name: None,
        chain_executables: Vec::new(),
        offered: OfferedChoices::resolved_bytes(),
        template: None,
    };
    let Ok(decision) = handler.approve(request) else {
        debug!("handler returned error, denying command");
        return Err(HarnessError::ShellCommandDenied { command: raw });
    };
    let cached = match decision {
        ShellApprovalDecision::AllowExactPersist => {
            decider.save(ShellRuleEntry::Exact(normalized.clone()), None);
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowCommandPersist => {
            decider.save(ShellRuleEntry::Prefix(executable.to_string()), None);
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowOnce => CachedApprovalDecision::Allowed,
        // The shape choice is never offered for resolved bytes.
        ShellApprovalDecision::Deny | ShellApprovalDecision::AllowTemplatePersist => {
            debug!("handler denied command");
            CachedApprovalDecision::Denied
        }
    };
    cache_approval_decision(&options.approval_cache, key, cached);
    decision_result(cached, &raw)?;
    debug!("shell command approved");
    Ok(approved())
}

/// Approves a command site by its template shape, for a site that may run
/// only as a template (a placeholder's value is produced later in the run).
///
/// Policy order matches resolved-byte approval: the built-in blacklist (by
/// shape, keeping each placeholder's position), then the captured
/// permissions (a `template` rule, or `prefix` rules for every fixed program,
/// unless a `!` entry matches the shape), then the session cache, then the
/// handler. An exact rule never covers a template site.
///
/// ## Returns
///
/// The approved shape.
///
/// ## Errors
///
/// `ShellCommandBlacklisted`, `ShellPermissionsInvalid`, or
/// `ShellCommandDenied` (also when the handler returns a choice the request
/// did not offer).
pub fn validate_and_approve_template_site(
    candidate: &ShellCandidate,
    options: &ShellApprovalOptions,
) -> Result<TemplateShape, HarnessError> {
    let command = candidate.display_command();
    let _span = info_span!("harness_shell_validate_template", command = %command).entered();
    let Some(shape) = candidate.shape() else {
        return Err(HarnessError::ShellCommandDenied { command });
    };

    if let Some(reason) = check_template_blacklist(&shape) {
        return Err(HarnessError::ShellCommandBlacklisted { command, reason });
    }
    let decider = Decider::new(options)?;
    let exclusion = decider.exclusion(|rules| check_template_exclusion(&shape, rules));
    let programs: Vec<&str> = shape.segments.iter().map(|segment| segment.program.as_str()).collect();
    if exclusion.is_none()
        && (decider.allows(|rules| rules.template_covers(&shape))
            || programs.iter().all(|program| decider.allows(|rules| rules.matches_prefix(program))))
    {
        debug!("template site pre-approved by permissions");
        return Ok(shape);
    }

    let key = ApprovalKey::template(shape.clone());
    if let Some(decision) = cached_decision(&options.approval_cache, &key) {
        return decision_result(decision, &command).map(|()| shape);
    }

    let Some(handler) = &options.approval_handler else {
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
        save_target: decider.save_target(),
        exclusion,
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
            decider.save(ShellRuleEntry::Template(shape.clone()), None);
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowCommandPersist if offered.prefix => {
            let mut persisted = std::collections::HashSet::new();
            for program in &programs {
                if persisted.insert(*program) {
                    decider.save(ShellRuleEntry::Prefix(program.to_string()), None);
                }
            }
            CachedApprovalDecision::Allowed
        }
        ShellApprovalDecision::AllowOnce => CachedApprovalDecision::Allowed,
        // Deny, or a choice the request did not offer (exact bytes do not
        // exist for this site).
        _ => CachedApprovalDecision::Denied,
    };
    cache_approval_decision(&options.approval_cache, key, cached);
    decision_result(cached, &command).map(|()| shape)
}

/// The `!` entry matching a resolved-byte site: its shape (holes matching
/// anything), or any of its resolved commands.
fn site_exclusion(
    decider: &Decider<'_>,
    shape: Option<&TemplateShape>,
    keys: &[String],
    commands: &[Vec<String>],
) -> Option<darkmatter::config::permissions::PolicyExclusion> {
    decider.exclusion(|rules| {
        shape.is_some_and(|shape| check_template_exclusion(shape, rules))
            || keys.iter().zip(commands).any(|(key, parts)| match parts.split_first() {
                Some((executable, args)) => check_user_blacklist(rules, executable, args, key),
                None => false,
            })
            || keys.iter().any(|key| rules.matches_exact(key))
    })
}

/// Each resolved key of `candidate` as its words; a key that does not
/// tokenize stays one word, so its program cannot be read from it.
fn resolved_commands(keys: &[String]) -> Vec<Vec<String>> {
    keys.iter()
        .map(|key| tokenize_words_strict(key).unwrap_or_else(|_| vec![key.clone()]))
        .collect()
}

/// The shape that approves a resolved-byte site by a `template` rule, when no
/// `exact` rule covers all of its bytes.
///
/// `None` when the site has no placeholder, an exact rule covers it, no
/// template rule does, or a `!` entry matches the site; its bytes are then
/// approved as resolved bytes.
///
/// ## Errors
///
/// `ShellCommandBlacklisted` when the shape matches the built-in blacklist,
/// `ShellPermissionsInvalid` when the captured `permissions` are invalid.
pub fn template_rule_covers(
    candidate: &ShellCandidate,
    options: &ShellApprovalOptions,
) -> Result<Option<TemplateShape>, HarnessError> {
    let (Some(shape), Some(keys)) = (candidate.shape(), &candidate.resolved) else {
        return Ok(None);
    };
    let command = candidate.display_command();
    let decider = Decider::new(options)?;
    if keys.iter().all(|key| decider.allows(|rules| rules.matches_exact(key)))
        || !decider.allows(|rules| rules.template_covers(&shape))
        || site_exclusion(&decider, Some(&shape), keys, &resolved_commands(keys)).is_some()
    {
        return Ok(None);
    }
    if let Some(reason) = check_template_blacklist(&shape) {
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
/// when the site has neither a shape nor a program placeholder, when the
/// captured permissions or a session decision already cover every one of its
/// commands, when a command's bytes are blacklisted, or when there is no
/// handler. The caller then approves the bytes as resolved bytes, which
/// reports those outcomes.
///
/// ## Errors
///
/// `ShellCommandBlacklisted` when the shape matches the built-in blacklist,
/// `ShellPermissionsInvalid` when the captured `permissions` are invalid, and
/// `ShellCommandDenied` on a denial or a choice the request did not offer.
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
    for parts in &commands {
        let Some((executable, args)) = parts.split_first() else {
            return Ok(None);
        };
        if check_builtin_blacklist(executable, args).is_some() {
            return Ok(None);
        }
    }
    let decider = Decider::new(options)?;
    let exclusion = site_exclusion(&decider, shape.as_ref(), keys, &commands);
    let exact_key = |key: &String| ApprovalKey::exact(key.clone());
    let undecided = keys.iter().zip(&commands).any(|(key, parts)| {
        let preapproved = exclusion.is_none() && decider.allows(|rules| check_whitelist(rules, &parts[0], key));
        !preapproved && cached_decision(&options.approval_cache, &exact_key(key)).is_none()
    });
    if !undecided {
        return Ok(None);
    }
    let template_key = shape.as_ref().map(|shape| ApprovalKey::template(shape.clone()));
    if let Some(shape) = &shape {
        if let Some(reason) = check_template_blacklist(shape) {
            return Err(HarnessError::ShellCommandBlacklisted { command, reason });
        }
        if let Some(decision) = cached_decision(&options.approval_cache, template_key.as_ref().expect("shape key")) {
            return decision_result(decision, &command).map(|()| Some(ResolvedSiteApproval::Shape(shape.clone())));
        }
    }
    let Some(handler) = &options.approval_handler else {
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
        save_target: decider.save_target(),
        exclusion,
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
                decider.save(ShellRuleEntry::Exact(key.clone()), None);
            }
            cache_bytes(CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Bytes))
        }
        ShellApprovalDecision::AllowTemplatePersist if offered.shape => {
            let shape = shape.expect("offered shape");
            decider.save(ShellRuleEntry::Template(shape.clone()), None);
            cache_approval_decision(&options.approval_cache, template_key.expect("shape key"), CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Shape(shape)))
        }
        ShellApprovalDecision::AllowCommandPersist if offered.prefix => {
            for executable in &executables {
                decider.save(ShellRuleEntry::Prefix(executable.to_string()), None);
            }
            cache_bytes(CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Bytes))
        }
        ShellApprovalDecision::AllowOnce => {
            cache_bytes(CachedApprovalDecision::Allowed);
            Ok(Some(ResolvedSiteApproval::Bytes))
        }
        // Deny, or a choice the request did not offer.
        _ => {
            cache_bytes(CachedApprovalDecision::Denied);
            Err(HarnessError::ShellCommandDenied { command })
        }
    }
}

/// Checks resolved bytes against the built-in blacklist only, for a command
/// whose site is approved by its shape (a `!` entry matching those bytes
/// already withheld rule approval of the shape).
///
/// ## Errors
///
/// `ShellCommandBlacklisted` on a match.
pub fn check_command_builtin_blacklist(parts: &[String]) -> Result<(), HarnessError> {
    let Some((executable, args)) = parts.split_first() else {
        return Ok(());
    };
    match check_builtin_blacklist(executable, args) {
        Some(reason) => Err(HarnessError::ShellCommandBlacklisted { command: parts.join(" "), reason }),
        None => Ok(()),
    }
}

/// Display source for approval prompts — real provenance when available.
fn display_source(source_file: Option<&Path>) -> ComposeSource {
    match source_file {
        Some(path) => ComposeSource::File(path.to_path_buf()),
        None => ComposeSource::Unknown,
    }
}

fn cached_decision(
    cache: &Arc<Mutex<HashMap<ApprovalKey, CachedApprovalDecision>>>,
    key: &ApprovalKey,
) -> Option<CachedApprovalDecision> {
    cache.lock().unwrap_or_else(|e| e.into_inner()).get(key).copied()
}

/// The outcome of a cached decision.
fn decision_result(decision: CachedApprovalDecision, command: &str) -> Result<(), HarnessError> {
    match decision {
        CachedApprovalDecision::Allowed => Ok(()),
        CachedApprovalDecision::Denied => Err(HarnessError::ShellCommandDenied {
            command: command.to_string(),
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
mod tests;

/// Fixtures that capture `permissions` from a temporary home, so a test
/// never reads or writes the process's home.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;
    use std::sync::Arc;

    use super::CapturedPermissions;

    /// The configuration of a user whose home is `home`, with `rules` as the
    /// `permissions.shell-commands` of `~/.darkmatter.yaml` (no file is
    /// written when there are none), and no repository.
    pub(crate) fn user_config(home: &Path, rules: &[&str]) -> Arc<darkmatter::config::load::ConfigSnapshot> {
        std::fs::create_dir_all(home).unwrap();
        if !rules.is_empty() {
            let mut text = String::from("permissions:\n  shell-commands:\n");
            for rule in rules {
                text.push_str(&format!("    - {}\n", serde_json::to_string(rule).unwrap()));
            }
            std::fs::write(home.join(darkmatter::config::schema::CONFIG_FILE_NAME), text).unwrap();
        }
        let context = biscuit_file::FileResolutionContext::from_snapshot(home, Some(home.to_path_buf()), Default::default());
        Arc::new(
            darkmatter::config::load::ConfigSnapshot::load(&context, None, darkmatter::config::load::ConfigLoadMode::Passive)
                .unwrap(),
        )
    }

    /// Writes `lines` (one `shell-commands` rule per line) as the
    /// `permissions.shell-commands` of `home/.darkmatter.yaml`.
    pub(crate) fn write_shell_rules(home: &Path, lines: &str) {
        let rules: Vec<&str> = lines.lines().filter(|line| !line.trim().is_empty()).collect();
        user_config(home, &rules);
    }

    /// [`user_config`]'s evaluated permissions.
    pub(crate) fn user_permissions(home: &Path, rules: &[&str]) -> CapturedPermissions {
        CapturedPermissions::of_snapshot(&user_config(home, rules))
    }

    /// The `permissions.shell-commands` entries saved under `home`, as written.
    pub(crate) fn saved_rules(home: &Path) -> Vec<String> {
        let snapshot = user_config(home, &[]);
        let policy = snapshot.policy().unwrap();
        policy.user().permissions().shell_commands.value.iter().map(|entry| entry.authored.clone()).collect()
    }
}
