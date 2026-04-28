use crate::events::{PROVIDERS_DISPLAY_ORDER, Provider};

use super::{
    Agent, AgentCapabilities, AgentDefinitionFormat, CapabilityStatus, CommandFormat, ConfigFormat,
    agent_for, all_agents, parse_agent_id,
};

#[derive(Debug)]
struct StubAgent {
    id: Provider,
    caps: AgentCapabilities,
}

impl Agent for StubAgent {
    fn id(&self) -> Provider {
        self.id
    }

    fn capabilities(&self) -> &AgentCapabilities {
        &self.caps
    }
}

fn baseline_caps() -> AgentCapabilities {
    agent_for(Provider::Codex).capabilities().clone()
}

#[derive(Debug, PartialEq, Eq)]
struct CapabilitySnapshot {
    id: Provider,
    config_format: Option<ConfigFormat>,
    skill_status: CapabilityStatus,
    skill_user_paths: Vec<String>,
    slash_status: CapabilityStatus,
    slash_format: CommandFormat,
    slash_custom_supported: bool,
    subagent_status: CapabilityStatus,
    subagent_format: AgentDefinitionFormat,
    permission_modes: Vec<String>,
    yolo_equivalent: Option<String>,
    sandbox_modes: Vec<String>,
}

fn capability_snapshot(agent: &dyn Agent) -> CapabilitySnapshot {
    let caps = agent.capabilities();
    CapabilitySnapshot {
        id: agent.id(),
        config_format: caps.config.format,
        skill_status: caps.skills.status,
        skill_user_paths: caps
            .skills
            .paths
            .user_paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        slash_status: caps.slash_commands.status,
        slash_format: caps.slash_commands.custom_format,
        slash_custom_supported: caps.slash_commands.custom_supported,
        subagent_status: caps.subagents.status,
        subagent_format: caps.subagents.definition_format,
        permission_modes: caps
            .runtime
            .permissions
            .modes
            .iter()
            .map(|m| (*m).to_string())
            .collect(),
        yolo_equivalent: caps.runtime.permissions.yolo_equivalent.map(str::to_string),
        sandbox_modes: caps
            .runtime
            .permissions
            .sandbox_modes
            .iter()
            .map(|m| (*m).to_string())
            .collect(),
    }
}

#[test]
fn all_agents_are_registered() {
    let agents = all_agents();
    assert_eq!(agents.len(), PROVIDERS_DISPLAY_ORDER.len());

    let mut ids: Vec<Provider> = agents.iter().map(|agent| agent.id()).collect();
    ids.sort_by_key(|id| id.as_slug());
    ids.dedup();

    assert_eq!(ids.len(), PROVIDERS_DISPLAY_ORDER.len());
}

#[test]
fn parse_agent_id_accepts_common_aliases() {
    assert_eq!(parse_agent_id("claude"), Some(Provider::Claude));
    assert_eq!(parse_agent_id("qwen-code"), Some(Provider::QwenCode));
    assert_eq!(parse_agent_id("open code"), Some(Provider::OpenCode));
    assert_eq!(parse_agent_id("unknown"), None);
}

#[test]
fn all_agents_pass_default_validation() {
    for agent in all_agents() {
        assert!(
            agent.validate().is_empty(),
            "{} reported validation issues: {:?}",
            agent.id().as_slug(),
            agent.validate()
        );
    }
}

#[test]
fn validate_reports_empty_binary() {
    let mut caps = baseline_caps();
    caps.meta.binary = "";

    let agent = StubAgent {
        id: Provider::Codex,
        caps,
    };

    assert!(
        agent
            .validate()
            .iter()
            .any(|issue| issue == "binary is empty")
    );
}

#[test]
fn validate_reports_missing_skill_paths_when_supported() {
    let mut caps = baseline_caps();
    caps.skills.status = CapabilityStatus::Supported;
    caps.skills.paths.user_paths.clear();
    caps.skills.paths.project_paths.clear();

    let agent = StubAgent {
        id: Provider::Codex,
        caps,
    };

    assert!(
        agent
            .validate()
            .iter()
            .any(|issue| issue == "skills marked supported but no discovery paths configured")
    );
}

#[test]
fn validate_reports_unknown_subagent_format_when_supported() {
    let mut caps = baseline_caps();
    caps.subagents.status = CapabilityStatus::Supported;
    caps.subagents.definition_format = AgentDefinitionFormat::Unknown;

    let agent = StubAgent {
        id: Provider::Codex,
        caps,
    };

    assert!(
        agent
            .validate()
            .iter()
            .any(|issue| issue == "subagents supported but definition format unknown")
    );
}

#[test]
fn snapshot_claude_code_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::Claude)),
        CapabilitySnapshot {
            id: Provider::Claude,
            config_format: Some(ConfigFormat::Json),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec!["~/.claude/skills".to_string()],
            slash_status: CapabilityStatus::Supported,
            slash_format: CommandFormat::Markdown,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Supported,
            subagent_format: AgentDefinitionFormat::MarkdownFrontmatter,
            permission_modes: vec![
                "default".to_string(),
                "acceptEdits".to_string(),
                "plan".to_string(),
                "dontAsk".to_string(),
                "bypassPermissions".to_string(),
            ],
            yolo_equivalent: Some("--dangerously-skip-permissions".to_string()),
            sandbox_modes: vec![],
        }
    );
}

#[test]
fn snapshot_codex_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::Codex)),
        CapabilitySnapshot {
            id: Provider::Codex,
            config_format: Some(ConfigFormat::Toml),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec![
                "~/.agents/skills".to_string(),
                "~/.codex/skills".to_string()
            ],
            slash_status: CapabilityStatus::Deprecated,
            slash_format: CommandFormat::Markdown,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Experimental,
            subagent_format: AgentDefinitionFormat::Toml,
            permission_modes: vec![
                "untrusted".to_string(),
                "on-failure".to_string(),
                "on-request".to_string(),
                "never".to_string(),
            ],
            yolo_equivalent: Some("--dangerously-bypass-approvals-and-sandbox".to_string()),
            sandbox_modes: vec![
                "read-only".to_string(),
                "workspace-write".to_string(),
                "danger-full-access".to_string(),
            ],
        }
    );
}

#[test]
fn snapshot_gemini_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::Gemini)),
        CapabilitySnapshot {
            id: Provider::Gemini,
            config_format: Some(ConfigFormat::Json),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec![
                "~/.gemini/skills".to_string(),
                "~/.agents/skills".to_string()
            ],
            slash_status: CapabilityStatus::Supported,
            slash_format: CommandFormat::Toml,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Experimental,
            subagent_format: AgentDefinitionFormat::MarkdownFrontmatter,
            permission_modes: vec![
                "default".to_string(),
                "auto_edit".to_string(),
                "yolo".to_string(),
                "plan".to_string(),
            ],
            yolo_equivalent: Some("--approval-mode yolo".to_string()),
            sandbox_modes: vec![
                "seatbelt (macOS)".to_string(),
                "docker".to_string(),
                "podman".to_string(),
            ],
        }
    );
}

#[test]
fn snapshot_goose_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::Goose)),
        CapabilitySnapshot {
            id: Provider::Goose,
            config_format: Some(ConfigFormat::Yaml),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec![
                "~/.claude/skills".to_string(),
                "~/.config/agents/skills".to_string(),
                "~/.config/goose/skills".to_string(),
            ],
            slash_status: CapabilityStatus::Supported,
            slash_format: CommandFormat::RecipeYaml,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Supported,
            subagent_format: AgentDefinitionFormat::NotApplicable,
            permission_modes: vec![
                "auto".to_string(),
                "smart_approve".to_string(),
                "approve".to_string(),
                "chat".to_string(),
            ],
            yolo_equivalent: Some("auto mode".to_string()),
            sandbox_modes: vec![],
        }
    );
}

#[test]
fn snapshot_kimi_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::KimiCode)),
        CapabilitySnapshot {
            id: Provider::KimiCode,
            config_format: Some(ConfigFormat::Toml),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec![
                "~/.config/agents/skills".to_string(),
                "~/.agents/skills".to_string(),
                "~/.kimi/skills".to_string(),
                "~/.claude/skills".to_string(),
                "~/.codex/skills".to_string(),
            ],
            slash_status: CapabilityStatus::Partial,
            slash_format: CommandFormat::None,
            slash_custom_supported: false,
            subagent_status: CapabilityStatus::Supported,
            subagent_format: AgentDefinitionFormat::Yaml,
            permission_modes: vec!["prompted".to_string(), "yolo".to_string()],
            yolo_equivalent: Some("--yolo".to_string()),
            sandbox_modes: vec![],
        }
    );
}

#[test]
fn snapshot_opencode_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::OpenCode)),
        CapabilitySnapshot {
            id: Provider::OpenCode,
            config_format: Some(ConfigFormat::Jsonc),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec![
                "~/.config/opencode/skills".to_string(),
                "~/.claude/skills".to_string(),
                "~/.agents/skills".to_string(),
            ],
            slash_status: CapabilityStatus::Supported,
            slash_format: CommandFormat::Mixed,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Supported,
            subagent_format: AgentDefinitionFormat::MarkdownFrontmatter,
            permission_modes: vec!["allow".to_string(), "ask".to_string(), "deny".to_string()],
            yolo_equivalent: None,
            sandbox_modes: vec![],
        }
    );
}

#[test]
fn snapshot_qwen_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::QwenCode)),
        CapabilitySnapshot {
            id: Provider::QwenCode,
            config_format: Some(ConfigFormat::Json),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec!["~/.qwen/skills".to_string()],
            slash_status: CapabilityStatus::Supported,
            slash_format: CommandFormat::Mixed,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Supported,
            subagent_format: AgentDefinitionFormat::MarkdownFrontmatter,
            permission_modes: vec![
                "plan".to_string(),
                "default".to_string(),
                "auto-edit".to_string(),
                "yolo".to_string(),
            ],
            yolo_equivalent: Some("--yolo".to_string()),
            sandbox_modes: vec!["--sandbox".to_string()],
        }
    );
}

#[test]
fn snapshot_roo_paths_formats_and_permissions() {
    assert_eq!(
        capability_snapshot(agent_for(Provider::RooCode)),
        CapabilitySnapshot {
            id: Provider::RooCode,
            config_format: Some(ConfigFormat::Mixed),
            skill_status: CapabilityStatus::Supported,
            skill_user_paths: vec![
                "~/.roo/skills".to_string(),
                "~/.roo/skills-<modeSlug>".to_string()
            ],
            slash_status: CapabilityStatus::Supported,
            slash_format: CommandFormat::Markdown,
            slash_custom_supported: true,
            subagent_status: CapabilityStatus::Partial,
            subagent_format: AgentDefinitionFormat::ModesYaml,
            permission_modes: vec![
                "auto-approve (default)".to_string(),
                "manual approval (--require-approval)".to_string(),
            ],
            yolo_equivalent: Some("default auto-approve behavior".to_string()),
            sandbox_modes: vec![],
        }
    );
}
