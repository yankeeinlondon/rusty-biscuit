use std::sync::OnceLock;

use crate::events::{PROVIDERS_DISPLAY_ORDER, Provider};

use super::claude_code::ClaudeCodeAgent;
use super::codex::CodexAgent;
use super::gemini_cli::GeminiCliAgent;
use super::goose::GooseAgent;
use super::kimi_code::KimiCodeAgent;
use super::model::Agent;
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

pub fn agent_for(provider: Provider) -> &'static dyn Agent {
    match provider {
        Provider::Claude => CLAUDE_AGENT.get_or_init(ClaudeCodeAgent::new),
        Provider::Codex => CODEX_AGENT.get_or_init(CodexAgent::new),
        Provider::Gemini => GEMINI_AGENT.get_or_init(GeminiCliAgent::new),
        Provider::Goose => GOOSE_AGENT.get_or_init(GooseAgent::new),
        Provider::KimiCode => KIMI_AGENT.get_or_init(KimiCodeAgent::new),
        Provider::OpenCode => OPENCODE_AGENT.get_or_init(OpenCodeAgent::new),
        Provider::QwenCode => QWEN_AGENT.get_or_init(QwenCliAgent::new),
        Provider::RooCode => ROO_AGENT.get_or_init(RooCodeAgent::new),
    }
}

pub fn all_agents() -> &'static [&'static dyn Agent] {
    ALL_AGENTS
        .get_or_init(|| PROVIDERS_DISPLAY_ORDER.into_iter().map(agent_for).collect())
        .as_slice()
}

pub fn parse_agent_id(input: &str) -> Option<Provider> {
    Provider::parse_cli_name(input)
}
