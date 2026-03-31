use tracing::debug;

use crate::error::Result;
use crate::events::Provider;
use crate::permissions::query::QueryResult;
use crate::permissions::{
    CliPolicyInput, PolicyCertainty, PolicyContext, PolicyEffect, PolicyEngine, PolicyQuery,
    PolicyWarning,
};

use super::config::{ProtectConfig, ProtectPosture};
use super::decision::{
    ProtectDecision, ProtectEvaluation, ProtectFinding, ProtectFindingSource, ProtectOutcome,
    ProtectPolicyMode, ProtectSeverity,
};
use super::intent::ProtectIntent;
use super::observe::{ProtectObservation, ProtectPayload, RuntimeFacts};
use super::redact::{
    ProtectRedactionPlan, contains_instruction_payload, redact_json_with_policy,
    redact_text_with_policy,
};
use super::request::{ProtectCliContext, ProtectRequest, ProtectSessionContext};

// ---------------------------------------------------------------------------
// Snapshot resolution
// ---------------------------------------------------------------------------

/// Resolve a policy snapshot from the engine, returning queryable results and
/// the policy mode that indicates whether we got effective or configured policy.
pub(crate) fn resolve_snapshot(
    engine: &PolicyEngine,
    session: &ProtectSessionContext,
) -> (Option<SnapshotBox>, ProtectPolicyMode) {
    let provider = session.provider;
    let ctx = &session.policy_context;

    match &session.cli {
        ProtectCliContext::Argv(args) => {
            match engine.effective(provider, ctx, CliPolicyInput::Argv(args)) {
                Ok(snapshot) => {
                    debug!(%provider, "Resolved effective policy snapshot from argv");
                    (Some(SnapshotBox::Effective(snapshot)), ProtectPolicyMode::Effective)
                }
                Err(err) => {
                    debug!(%provider, %err, "Failed to resolve effective snapshot, using configured fallback");
                    try_configured_fallback(engine, provider, ctx)
                }
            }
        }
        ProtectCliContext::Parsed(overrides) => {
            match engine.effective(provider, ctx, CliPolicyInput::Parsed(overrides.as_ref())) {
                Ok(snapshot) => {
                    debug!(%provider, "Resolved effective policy snapshot from parsed overrides");
                    (Some(SnapshotBox::Effective(snapshot)), ProtectPolicyMode::Effective)
                }
                Err(err) => {
                    debug!(%provider, %err, "Failed to resolve effective snapshot, using configured fallback");
                    try_configured_fallback(engine, provider, ctx)
                }
            }
        }
        ProtectCliContext::None => try_configured_fallback(engine, provider, ctx),
    }
}

fn try_configured_fallback(
    engine: &PolicyEngine,
    provider: Provider,
    ctx: &PolicyContext,
) -> (Option<SnapshotBox>, ProtectPolicyMode) {
    match engine.configured(provider, ctx) {
        Ok(snapshot) => {
            debug!(%provider, "Resolved configured policy snapshot (fallback)");
            (Some(SnapshotBox::Configured(snapshot)), ProtectPolicyMode::ConfiguredFallback)
        }
        Err(err) => {
            debug!(%provider, %err, "Failed to resolve any policy snapshot");
            (None, ProtectPolicyMode::ConfiguredFallback)
        }
    }
}

/// Wrapper that abstracts over both snapshot types for querying.
pub(crate) enum SnapshotBox {
    Effective(crate::permissions::EffectivePolicySnapshot),
    Configured(crate::permissions::ConfiguredPolicySnapshot),
}

impl SnapshotBox {
    pub(crate) fn query(&self, query: &PolicyQuery) -> QueryResult {
        match self {
            SnapshotBox::Effective(s) => s.query(query),
            SnapshotBox::Configured(s) => s.query(query),
        }
    }
}

// ---------------------------------------------------------------------------
// Full evaluation pipeline
// ---------------------------------------------------------------------------

/// Run the full policy-backed evaluation pipeline.
///
/// Steps:
/// 1. Resolve posture (base + provider override)
/// 2. Resolve policy snapshot
/// 3. Query each intent against the snapshot
/// 4. Apply runtime guards
/// 5. Select desired outcome from findings
/// 6. Apply YOLO mode overrides
/// 7. Build redaction plan if MCP payload present
/// 8. Assemble ProtectEvaluation
pub(crate) fn evaluate_request(
    engine: &PolicyEngine,
    config: &ProtectConfig,
    request: &ProtectRequest,
) -> Result<ProtectEvaluation> {
    let (snapshot, policy_mode) = resolve_snapshot(engine, &request.session);
    evaluate_with_snapshot(config, request, snapshot, policy_mode)
}

/// Evaluate with a pre-resolved snapshot (used by service-level caching).
pub(crate) fn evaluate_with_snapshot(
    config: &ProtectConfig,
    request: &ProtectRequest,
    snapshot: Option<SnapshotBox>,
    policy_mode: ProtectPolicyMode,
) -> Result<ProtectEvaluation> {
    // Step 1: Resolve posture
    let posture = resolve_posture(config, &request.session);

    // Step 3: Query each intent
    let mut findings = Vec::new();
    let mut warnings: Vec<PolicyWarning> = Vec::new();

    for intent in &request.observation.intents {
        if let Some(pq) = intent.to_policy_query() {
            if let Some(ref snap) = snapshot {
                let result = snap.query(&pq);
                let severity = classify_severity(intent, &result, posture);
                warnings.extend(result.warnings.clone());
                findings.push(ProtectFinding {
                    intent: intent.clone(),
                    result,
                    severity,
                    source: ProtectFindingSource::PolicyQuery,
                });
            } else {
                // No snapshot available — treat as unknown
                findings.push(ProtectFinding {
                    intent: intent.clone(),
                    result: QueryResult::unknown(),
                    severity: classify_unknown_severity(intent, posture),
                    source: ProtectFindingSource::PolicyQuery,
                });
            }
        } else if matches!(intent, ProtectIntent::CompletionOutputScan) {
            // Completion scan: instruction injection, secrets, and command patterns.
            let (result, severity) = completion_scan(&request.observation, config);
            findings.push(ProtectFinding {
                intent: intent.clone(),
                result,
                severity,
                source: ProtectFindingSource::CompletionLoop,
            });
        }
    }

    // Step 4: Apply runtime guards
    apply_runtime_guards(config, &request.observation.runtime, posture, &mut findings);

    // Step 5: Select desired outcome
    let desired = select_desired_outcome(&findings, posture);

    // Step 6: Apply YOLO mode
    let outcome = apply_yolo_mode(desired.clone(), config, &request.session, &findings);

    // Step 7: Build redaction plan
    let redaction = build_redaction_plan(config, request.session.provider, &request.observation);

    // If redaction is present and outcome is Allow or AdvisoryOnly,
    // upgrade to AllowWithRedaction since we are actively modifying content.
    let outcome = if redaction.is_some()
        && matches!(
            outcome,
            ProtectOutcome::Allow | ProtectOutcome::AdvisoryOnly { .. }
        )
    {
        ProtectOutcome::AllowWithRedaction {
            reason: "mcp.redaction-applied".to_string(),
        }
    } else {
        outcome
    };

    let decision = ProtectDecision::new(outcome.clone(), desired, "protect".to_string(), None);

    Ok(ProtectEvaluation {
        decision,
        policy_mode,
        findings,
        redaction,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Step 1: Posture resolution
// ---------------------------------------------------------------------------

fn resolve_posture(config: &ProtectConfig, session: &ProtectSessionContext) -> ProtectPosture {
    if let Some(override_cfg) = config.providers.get(&session.provider) {
        if let Some(posture) = override_cfg.posture {
            return posture;
        }
    }
    config.posture
}

// ---------------------------------------------------------------------------
// Step 3: Severity classification
// ---------------------------------------------------------------------------

fn classify_severity(
    intent: &ProtectIntent,
    result: &QueryResult,
    posture: ProtectPosture,
) -> ProtectSeverity {
    let base = match result.effect {
        Some(PolicyEffect::Deny) => {
            if is_destructive_intent(intent) {
                ProtectSeverity::Critical
            } else {
                ProtectSeverity::High
            }
        }
        Some(PolicyEffect::Ask) => ProtectSeverity::Medium,
        Some(PolicyEffect::Allow) => ProtectSeverity::Info,
        None => ProtectSeverity::Medium,
    };

    // Elevate by one level under Strict posture for uncertain results
    if posture == ProtectPosture::Strict && is_uncertain(&result.certainty) {
        elevate_severity(base)
    } else {
        base
    }
}

fn classify_unknown_severity(intent: &ProtectIntent, posture: ProtectPosture) -> ProtectSeverity {
    let base = if is_write_intent(intent) || is_command_intent(intent) {
        ProtectSeverity::Medium
    } else {
        ProtectSeverity::Info
    };

    if posture == ProtectPosture::Strict {
        elevate_severity(base)
    } else {
        base
    }
}

fn is_destructive_intent(intent: &ProtectIntent) -> bool {
    matches!(
        intent,
        ProtectIntent::ModifyProviderConfig | ProtectIntent::SwitchMode { .. }
    )
}

fn is_write_intent(intent: &ProtectIntent) -> bool {
    matches!(
        intent,
        ProtectIntent::WritePath(_) | ProtectIntent::ModifyProviderConfig
    )
}

fn is_command_intent(intent: &ProtectIntent) -> bool {
    matches!(intent, ProtectIntent::ExecuteCommand(_))
}

fn is_uncertain(certainty: &PolicyCertainty) -> bool {
    matches!(
        certainty,
        PolicyCertainty::BestEffort | PolicyCertainty::Unknown
    )
}

fn elevate_severity(severity: ProtectSeverity) -> ProtectSeverity {
    match severity {
        ProtectSeverity::Info => ProtectSeverity::Medium,
        ProtectSeverity::Medium => ProtectSeverity::High,
        ProtectSeverity::High => ProtectSeverity::Critical,
        ProtectSeverity::Critical => ProtectSeverity::Critical,
    }
}

// ---------------------------------------------------------------------------
// Step 4: Runtime guards
// ---------------------------------------------------------------------------

fn apply_runtime_guards(
    config: &ProtectConfig,
    runtime: &RuntimeFacts,
    posture: ProtectPosture,
    findings: &mut Vec<ProtectFinding>,
) {
    // Root without sandbox
    if config.privilege.deny_when_root_without_sandbox
        && runtime.is_root
        && matches!(runtime.has_sandbox, Some(false))
    {
        findings.push(ProtectFinding {
            intent: ProtectIntent::ModifyProviderConfig, // best-fit sentinel
            result: QueryResult::denied("root-without-sandbox"),
            severity: if posture == ProtectPosture::Strict {
                ProtectSeverity::Critical
            } else {
                ProtectSeverity::High
            },
            source: ProtectFindingSource::RuntimeGuard,
        });
    }

    // Collect additional runtime guard findings from existing policy findings.
    let mut extra_guards = Vec::new();

    // Network write escalation: require ask for commands that reach the network.
    if config.privilege.require_ask_for_network_writes {
        for finding in findings.iter() {
            if finding.source == ProtectFindingSource::PolicyQuery
                && is_network_command(&finding.intent)
                && matches!(finding.result.effect, Some(PolicyEffect::Allow) | None)
            {
                extra_guards.push(ProtectFinding {
                    intent: finding.intent.clone(),
                    result: QueryResult::synthetic_ask("runtime-guard.network-write"),
                    severity: ProtectSeverity::Medium,
                    source: ProtectFindingSource::RuntimeGuard,
                });
                break;
            }
        }
    }

    // Broad filesystem write escalation: require ask for writes to sensitive paths.
    if config.privilege.require_ask_for_broad_fs_writes {
        for finding in findings.iter() {
            if finding.source == ProtectFindingSource::PolicyQuery
                && is_broad_fs_write(&finding.intent)
                && matches!(finding.result.effect, Some(PolicyEffect::Allow) | None)
            {
                extra_guards.push(ProtectFinding {
                    intent: finding.intent.clone(),
                    result: QueryResult::synthetic_ask("runtime-guard.broad-fs-write"),
                    severity: ProtectSeverity::Medium,
                    source: ProtectFindingSource::RuntimeGuard,
                });
                break;
            }
        }
    }

    findings.extend(extra_guards);
}

/// Check if a command intent involves network-touching executables.
fn is_network_command(intent: &ProtectIntent) -> bool {
    if let ProtectIntent::ExecuteCommand(cmd) = intent {
        let exe = cmd.executable.as_deref().unwrap_or("");
        matches!(
            exe,
            "curl" | "wget" | "ssh" | "scp" | "rsync" | "nc" | "ncat"
                | "netcat" | "socat" | "ftp" | "sftp"
        )
    } else {
        false
    }
}

/// Check if a write path targets a broad/sensitive filesystem location.
fn is_broad_fs_write(intent: &ProtectIntent) -> bool {
    if let ProtectIntent::WritePath(pq) = intent {
        let path_str = pq.path.to_string_lossy();
        // System directories, home dotfiles, and package manager paths
        path_str.starts_with("/etc/")
            || path_str.starts_with("/usr/")
            || path_str.starts_with("/var/")
            || path_str.starts_with("/opt/")
            || path_str.starts_with("/System/")
            || (path_str.contains("/.") && !path_str.contains("/.git/"))
            || path_str.contains("/node_modules/")
            || path_str.contains("/target/")
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Step 5: Desired outcome selection (decision matrix)
// ---------------------------------------------------------------------------

/// Maps findings through the decision semantics matrix and selects the
/// highest-precedence outcome.
pub(crate) fn select_desired_outcome(
    findings: &[ProtectFinding],
    posture: ProtectPosture,
) -> ProtectOutcome {
    if findings.is_empty() {
        return ProtectOutcome::Allow;
    }

    let outcomes: Vec<ProtectOutcome> = findings
        .iter()
        .map(|f| finding_to_outcome(f, posture))
        .collect();

    select_precedence(outcomes)
}

/// Implement the decision semantics matrix from the design.
///
/// | Effect | Exact           | BestEffort (Advisory) | BestEffort (Balanced) | BestEffort (Strict)   |
/// |--------|-----------------|-----------------------|-----------------------|-----------------------|
/// | Allow  | Allow           | AdvisoryOnly          | Allow                 | AskThenAllowOrStop    |
/// | Ask    | AskThenAllow    | AdvisoryOnly          | AskThenAllowOrStop    | StopCurrent           |
/// | Deny   | StopCurrent     | AdvisoryOnly          | StopCurrent           | StopCurrent           |
fn finding_to_outcome(finding: &ProtectFinding, posture: ProtectPosture) -> ProtectOutcome {
    let effect = finding.result.effect;
    let certainty = finding.result.certainty;

    // RuntimeGuard findings always map to their severity directly
    if finding.source == ProtectFindingSource::RuntimeGuard {
        return match finding.severity {
            ProtectSeverity::Critical => ProtectOutcome::StopSession {
                reason: "runtime-guard.critical".to_string(),
            },
            ProtectSeverity::High => stop_outcome_for_posture(posture, "runtime-guard"),
            ProtectSeverity::Medium => ProtectOutcome::AskThenAllowOrStop {
                reason: "runtime-guard".to_string(),
            },
            ProtectSeverity::Info => ProtectOutcome::AdvisoryOnly {
                reason: "runtime-guard.info".to_string(),
            },
        };
    }

    // CompletionLoop findings
    if finding.source == ProtectFindingSource::CompletionLoop {
        return match finding.severity {
            ProtectSeverity::Critical => ProtectOutcome::StopSession {
                reason: "completion.loop-detected".to_string(),
            },
            ProtectSeverity::High => ProtectOutcome::StopCurrent {
                reason: "completion.secret-found".to_string(),
            },
            _ => ProtectOutcome::Allow,
        };
    }

    match certainty {
        PolicyCertainty::Exact => match effect {
            Some(PolicyEffect::Allow) => ProtectOutcome::Allow,
            Some(PolicyEffect::Ask) => ProtectOutcome::AskThenAllowOrStop {
                reason: "policy.ask".to_string(),
            },
            Some(PolicyEffect::Deny) => ProtectOutcome::StopCurrent {
                reason: "policy.deny".to_string(),
            },
            None => unknown_outcome(&finding.intent, posture),
        },
        PolicyCertainty::BestEffort => match posture {
            ProtectPosture::Advisory => match effect {
                Some(PolicyEffect::Allow) | Some(PolicyEffect::Ask) | Some(PolicyEffect::Deny) => {
                    ProtectOutcome::AdvisoryOnly {
                        reason: "policy.best-effort".to_string(),
                    }
                }
                None => ProtectOutcome::AdvisoryOnly {
                    reason: "policy.unknown".to_string(),
                },
            },
            ProtectPosture::Balanced => match effect {
                Some(PolicyEffect::Allow) => ProtectOutcome::Allow,
                Some(PolicyEffect::Ask) => ProtectOutcome::AskThenAllowOrStop {
                    reason: "policy.ask.best-effort".to_string(),
                },
                Some(PolicyEffect::Deny) => ProtectOutcome::StopCurrent {
                    reason: "policy.deny.best-effort".to_string(),
                },
                None => unknown_outcome(&finding.intent, posture),
            },
            ProtectPosture::Strict => match effect {
                Some(PolicyEffect::Allow) => ProtectOutcome::AskThenAllowOrStop {
                    reason: "policy.allow.best-effort.strict".to_string(),
                },
                Some(PolicyEffect::Ask) => ProtectOutcome::StopCurrent {
                    reason: "policy.ask.best-effort.strict".to_string(),
                },
                Some(PolicyEffect::Deny) => ProtectOutcome::StopCurrent {
                    reason: "policy.deny.best-effort.strict".to_string(),
                },
                None => ProtectOutcome::StopCurrent {
                    reason: "policy.unknown.strict".to_string(),
                },
            },
        },
        PolicyCertainty::Unknown => unknown_outcome(&finding.intent, posture),
    }
}

/// For None/Unknown certainty under Balanced posture:
/// - Writes, commands, MCP tools, config mutation -> AskThenAllowOrStop
/// - Reads, domain access -> AdvisoryOnly
fn unknown_outcome(intent: &ProtectIntent, posture: ProtectPosture) -> ProtectOutcome {
    match posture {
        ProtectPosture::Advisory => ProtectOutcome::AdvisoryOnly {
            reason: "policy.unknown".to_string(),
        },
        ProtectPosture::Strict => ProtectOutcome::StopCurrent {
            reason: "policy.unknown.strict".to_string(),
        },
        ProtectPosture::Balanced => {
            if is_write_or_dangerous_intent(intent) {
                ProtectOutcome::AskThenAllowOrStop {
                    reason: "policy.unknown.write-or-dangerous".to_string(),
                }
            } else {
                ProtectOutcome::AdvisoryOnly {
                    reason: "policy.unknown.read-like".to_string(),
                }
            }
        }
    }
}

fn is_write_or_dangerous_intent(intent: &ProtectIntent) -> bool {
    matches!(
        intent,
        ProtectIntent::WritePath(_)
            | ProtectIntent::ExecuteCommand(_)
            | ProtectIntent::UseMcpTool { .. }
            | ProtectIntent::ModifyProviderConfig
            | ProtectIntent::SwitchMode { .. }
    )
}

pub(crate) fn select_precedence(outcomes: Vec<ProtectOutcome>) -> ProtectOutcome {
    outcomes
        .into_iter()
        .max_by_key(outcome_priority)
        .unwrap_or(ProtectOutcome::Allow)
}

fn outcome_priority(outcome: &ProtectOutcome) -> u8 {
    match outcome {
        ProtectOutcome::StopSession { .. } => 6,
        ProtectOutcome::StopCurrent { .. } => 5,
        ProtectOutcome::AskThenAllowOrStop { .. } => 4,
        ProtectOutcome::AllowWithRedaction { .. } => 3,
        ProtectOutcome::AdvisoryOnly { .. } => 2,
        ProtectOutcome::Allow => 1,
    }
}

pub(crate) fn stop_outcome_for_posture(posture: ProtectPosture, reason: &str) -> ProtectOutcome {
    match posture {
        ProtectPosture::Advisory => ProtectOutcome::AdvisoryOnly {
            reason: reason.to_string(),
        },
        ProtectPosture::Balanced => ProtectOutcome::StopCurrent {
            reason: reason.to_string(),
        },
        ProtectPosture::Strict => ProtectOutcome::StopSession {
            reason: reason.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Step 6: YOLO mode
// ---------------------------------------------------------------------------

fn apply_yolo_mode(
    desired: ProtectOutcome,
    config: &ProtectConfig,
    session: &ProtectSessionContext,
    findings: &[ProtectFinding],
) -> ProtectOutcome {
    if !session.yolo {
        return desired;
    }

    // Force medium-risk ask to advisory
    if config.yolo.force_advisory_for_medium_risk {
        let max_severity = findings
            .iter()
            .map(|f| f.severity)
            .max()
            .unwrap_or(ProtectSeverity::Info);

        if max_severity <= ProtectSeverity::Medium
            && matches!(desired, ProtectOutcome::AskThenAllowOrStop { .. })
        {
            return ProtectOutcome::AdvisoryOnly {
                reason: "yolo.medium-risk-advisory".to_string(),
            };
        }
    }

    // Block critical actions unless allowed
    if !config.yolo.allow_critical_blocking
        && matches!(
            desired,
            ProtectOutcome::StopCurrent { .. } | ProtectOutcome::StopSession { .. }
        )
    {
        return ProtectOutcome::AdvisoryOnly {
            reason: "yolo.blocking-disabled".to_string(),
        };
    }

    desired
}

// ---------------------------------------------------------------------------
// Step 7: Redaction plan
// ---------------------------------------------------------------------------

fn build_redaction_plan(
    config: &ProtectConfig,
    provider: Provider,
    observation: &ProtectObservation,
) -> Option<ProtectRedactionPlan> {
    let payload = observation.payload.as_ref()?;

    // Resolve provider-effective config
    let effective_mcp = if let Some(override_cfg) = config.providers.get(&provider) {
        if let Some(mcp) = override_cfg.mcp.as_ref() {
            super::config::merge_mcp_policy_pub(&config.mcp, mcp)
        } else {
            config.mcp.clone()
        }
    } else {
        config.mcp.clone()
    };

    match payload {
        ProtectPayload::McpText(text) => {
            // Check for instruction payload blocking
            if effective_mcp.block_instruction_payloads && contains_instruction_payload(text) {
                return Some(ProtectRedactionPlan::BlockPayload {
                    reason: "mcp.instruction-payload-blocked".to_string(),
                });
            }

            let redaction =
                redact_text_with_policy(text, &effective_mcp, &config.rules.secret_patterns)
                    .ok()?;
            if redaction.redacted {
                Some(ProtectRedactionPlan::ReplaceText(redaction))
            } else {
                None
            }
        }
        ProtectPayload::McpJson(value) => {
            let redaction =
                redact_json_with_policy(value, &effective_mcp, &config.rules.secret_patterns)
                    .ok()?;

            if redaction.blocked_instruction_payload {
                return Some(ProtectRedactionPlan::BlockPayload {
                    reason: "mcp.instruction-payload-blocked".to_string(),
                });
            }

            if redaction.redacted {
                Some(ProtectRedactionPlan::ReplaceJson(redaction))
            } else {
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Completion scan helpers
// ---------------------------------------------------------------------------

/// Completion scan checks: instruction injection, secret patterns, and command patterns.
///
/// Returns a tuple of (QueryResult, ProtectSeverity) to avoid duplicating
/// the scan logic between result and severity functions.
fn completion_scan(
    observation: &ProtectObservation,
    config: &ProtectConfig,
) -> (QueryResult, ProtectSeverity) {
    let text = match &observation.payload {
        Some(ProtectPayload::McpText(text)) => Some(text.as_str()),
        Some(ProtectPayload::McpJson(_)) => None,
        None => None,
    };

    // 1. Instruction payload detection (always active) — Critical severity
    if let Some(text) = text {
        if contains_instruction_payload(text) {
            return (
                QueryResult::denied("completion.instruction-injection"),
                ProtectSeverity::Critical,
            );
        }
    }

    // 2. Secret pattern scanning (when enabled) — High severity
    if config.completion.secret_scan {
        if let Some(text) = text {
            for pattern in config
                .rules
                .secret_patterns
                .iter()
                .chain(config.mcp.redact_patterns.iter())
            {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(text) {
                        return (
                            QueryResult::denied("completion.secret-found"),
                            ProtectSeverity::High,
                        );
                    }
                }
            }
        }
    }

    // 3. Command pattern checking (when configured) — High severity
    if !config.completion.check_commands.is_empty() {
        for intent in &observation.intents {
            if let ProtectIntent::ExecuteCommand(cmd) = intent {
                for pattern in &config.completion.check_commands {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if re.is_match(&cmd.raw) {
                            return (
                                QueryResult::denied("completion.suspicious-command"),
                                ProtectSeverity::High,
                            );
                        }
                    }
                }
            }
        }
    }

    (QueryResult::allowed("completion.clean"), ProtectSeverity::Info)
}
