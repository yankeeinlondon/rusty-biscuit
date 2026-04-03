use crate::permissions::{PolicyCertainty, PolicyEffect};

use super::decision::{
    ProtectEvaluation, ProtectFinding, ProtectFindingSource, ProtectPolicyMode, ProtectSeverity,
};

/// Human-readable explanation of a protect evaluation.
#[derive(Debug, Clone)]
pub struct ProtectExplanation {
    pub summary: String,
    pub findings: Vec<FindingExplanation>,
    pub policy_mode: ProtectPolicyMode,
    pub remediation: Option<ProtectRemediation>,
}

/// Explanation of a single finding within an evaluation.
#[derive(Debug, Clone)]
pub struct FindingExplanation {
    pub intent_description: String,
    pub effect: Option<PolicyEffect>,
    pub certainty: PolicyCertainty,
    pub matched_rules: Vec<String>,
    pub severity: ProtectSeverity,
}

/// Suggested remediation for a protect decision.
#[derive(Debug, Clone)]
pub struct ProtectRemediation {
    pub one_time_args: Option<Vec<String>>,
    pub summary: String,
}

/// Render an evaluation into a human-readable explanation.
pub fn explain_evaluation(eval: &ProtectEvaluation) -> ProtectExplanation {
    let summary = format!(
        "Outcome: {:?} (desired: {:?}, degraded: {})",
        eval.decision.outcome, eval.decision.desired_outcome, eval.decision.degraded,
    );

    let findings = eval.findings.iter().map(explain_finding).collect();

    let remediation = suggest_remediation(eval);

    ProtectExplanation {
        summary,
        findings,
        policy_mode: eval.policy_mode.clone(),
        remediation,
    }
}

fn explain_finding(finding: &ProtectFinding) -> FindingExplanation {
    let intent_description = describe_intent(&finding.intent);

    let matched_rules = finding
        .result
        .matched_rules
        .iter()
        .map(|r| r.description.clone())
        .collect();

    FindingExplanation {
        intent_description,
        effect: finding.result.effect,
        certainty: finding.result.certainty,
        matched_rules,
        severity: finding.severity,
    }
}

fn describe_intent(intent: &super::intent::ProtectIntent) -> String {
    use super::intent::ProtectIntent;
    match intent {
        ProtectIntent::ReadPath(pq) => format!("Read path: {}", pq.path.display()),
        ProtectIntent::WritePath(pq) => format!("Write path: {}", pq.path.display()),
        ProtectIntent::TraversePath(pq) => format!("Traverse path: {}", pq.path.display()),
        ProtectIntent::ExecuteCommand(cq) => format!("Execute command: {}", cq.raw),
        ProtectIntent::AccessDomain(dq) => format!("Access domain: {}", dq.domain),
        ProtectIntent::UseMcpServer { server } => format!("Use MCP server: {server}"),
        ProtectIntent::UseMcpTool { server, tool } => {
            format!("Use MCP tool: {server}/{tool}")
        }
        ProtectIntent::SpawnSubagent { name } => {
            format!("Spawn subagent: {}", name.as_deref().unwrap_or("(unnamed)"))
        }
        ProtectIntent::SwitchMode { target } => {
            format!("Switch mode: {}", target.as_deref().unwrap_or("(unknown)"))
        }
        ProtectIntent::ModifyProviderConfig => "Modify provider config".to_string(),
        ProtectIntent::CompletionOutputScan => "Completion output scan".to_string(),
    }
}

fn suggest_remediation(eval: &ProtectEvaluation) -> Option<ProtectRemediation> {
    if eval.decision.degraded {
        return Some(ProtectRemediation {
            one_time_args: None,
            summary: format!(
                "The desired outcome ({:?}) was degraded to {:?} because the provider lacks the capability to enforce it.",
                eval.decision.desired_outcome, eval.decision.outcome,
            ),
        });
    }

    // Suggest CLI args for ConfiguredFallback mode
    if eval.policy_mode == ProtectPolicyMode::ConfiguredFallback {
        let has_non_trivial = eval.findings.iter().any(|f| {
            f.severity >= ProtectSeverity::Medium
                || matches!(f.source, ProtectFindingSource::RuntimeGuard)
        });

        if has_non_trivial {
            return Some(ProtectRemediation {
                one_time_args: None,
                summary: "Policy was evaluated without CLI context (configured fallback). \
                    Running the provider through a Claudine wrapper would provide more accurate policy resolution."
                    .to_string(),
            });
        }
    }

    None
}
