use std::sync::OnceLock;

use super::claude_code::ClaudeCodeAgent;
use super::codex::CodexAgent;
use super::gemini_cli::GeminiCliAgent;
use super::goose::GooseAgent;
use super::kimi_code::KimiCodeAgent;
use super::model::{Agent, AgentId};
use super::opencode::OpenCodeAgent;
use super::qwen_cli::QwenCliAgent;
use super::roo_code::RooCodeAgent;

static CLAUDE_AGENT: OnceLock<ClaudeCodeAgent> = OnceLock::new();
static CODEX_AGENT: OnceLock<CodexAgent> = OnceLock::new();
static GEMINI_AGENT: OnceLock<GeminiCliAgent> = OnceLock::new();
static GOOSE_AGENT: OnceLock<GooseAgent> = OnceLock::new();
static KIMI_AGENT: OnceLock<KimiCodeAgent> = OnceLock::new();
static OPENCODE_AGENT: OnceLock<OpenCodeAgent> = OnceLock::new();
static QWEN_AGENT: OnceLock<QwenCliAgent> = OnceLock::new();
static ROO_AGENT: OnceLock<RooCodeAgent> = OnceLock::new();

static ALL_AGENTS: OnceLock<Vec<&'static dyn Agent>> = OnceLock::new();

pub fn agent_for(id: AgentId) -> &'static dyn Agent {
    match id {
        AgentId::ClaudeCode => CLAUDE_AGENT.get_or_init(ClaudeCodeAgent::new),
        AgentId::Codex => CODEX_AGENT.get_or_init(CodexAgent::new),
        AgentId::GeminiCli => GEMINI_AGENT.get_or_init(GeminiCliAgent::new),
        AgentId::Goose => GOOSE_AGENT.get_or_init(GooseAgent::new),
        AgentId::KimiCode => KIMI_AGENT.get_or_init(KimiCodeAgent::new),
        AgentId::OpenCode => OPENCODE_AGENT.get_or_init(OpenCodeAgent::new),
        AgentId::QwenCli => QWEN_AGENT.get_or_init(QwenCliAgent::new),
        AgentId::RooCode => ROO_AGENT.get_or_init(RooCodeAgent::new),
    }
}

pub fn all_agents() -> &'static [&'static dyn Agent] {
    ALL_AGENTS
        .get_or_init(|| AgentId::ALL.into_iter().map(agent_for).collect())
        .as_slice()
}

pub fn parse_agent_id(input: &str) -> Option<AgentId> {
    let normalized = input.trim().to_ascii_lowercase().replace(['_', ' '], "-");

    AgentId::ALL.into_iter().find(|id| {
        let key = id.as_str();
        key == normalized || id.aliases().iter().any(|alias| *alias == normalized)
    })
}
