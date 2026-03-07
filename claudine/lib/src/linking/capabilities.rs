//! Provider capability definitions for cross-linking resources.
//!
//! This module provides the single source of truth for what each agentic CLI provider
//! supports in terms of skills, commands, agents, and scripts. The data is derived from
//! analysis of each provider's documentation and behavior.
//!
//! ## Resource Types
//!
//! - **Skills**: Directory bundles with `SKILL.md` entry point
//! - **Commands**: Slash commands (custom prompts invoked via `/name`)
//! - **Agents**: Subagent/persona definitions for task delegation
//! - **Scripts**: Executable scripts that can be invoked by skills/agents

use crate::events::Provider;
use std::path::PathBuf;

/// Types of linkable resources across providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkableResource {
    /// Skills (SKILL.md directories)
    Skill,
    /// Slash commands (/name prompts)
    Command,
    /// Agents/subagents (task delegation)
    Agent,
    /// Scripts (executables)
    Script,
}

impl LinkableResource {
    /// All resource types.
    pub const ALL: [LinkableResource; 4] = [
        LinkableResource::Skill,
        LinkableResource::Command,
        LinkableResource::Agent,
        LinkableResource::Script,
    ];

    /// Display name for the resource type.
    pub fn name(&self) -> &'static str {
        match self {
            LinkableResource::Skill => "Skills",
            LinkableResource::Command => "Commands",
            LinkableResource::Agent => "Agents",
            LinkableResource::Script => "Scripts",
        }
    }

    /// Short name for table headers.
    pub fn abbrev(&self) -> &'static str {
        match self {
            LinkableResource::Skill => "Skill",
            LinkableResource::Command => "Cmd",
            LinkableResource::Agent => "Agent",
            LinkableResource::Script => "Script",
        }
    }
}

impl std::fmt::Display for LinkableResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// File format used for a resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceFormat {
    /// Markdown files with YAML frontmatter (most common)
    Markdown,
    /// TOML configuration files (Gemini commands)
    Toml,
    /// YAML configuration files (Goose recipes, KimiCode agents)
    Yaml,
    /// MCP-based (Model Context Protocol) tools (Goose commands)
    Mcp,
    /// Built-in only, no custom file support
    BuiltinOnly,
    /// Shell scripts or executables
    Executable,
}

impl ResourceFormat {
    /// Display name for the format.
    pub fn name(&self) -> &'static str {
        match self {
            ResourceFormat::Markdown => "Markdown",
            ResourceFormat::Toml => "TOML",
            ResourceFormat::Yaml => "YAML",
            ResourceFormat::Mcp => "MCP",
            ResourceFormat::BuiltinOnly => "Built-in",
            ResourceFormat::Executable => "Executable",
        }
    }

    /// Short identifier for the format.
    pub fn abbrev(&self) -> &'static str {
        match self {
            ResourceFormat::Markdown => ".md",
            ResourceFormat::Toml => ".toml",
            ResourceFormat::Yaml => ".yaml",
            ResourceFormat::Mcp => "MCP",
            ResourceFormat::BuiltinOnly => "-",
            ResourceFormat::Executable => "exec",
        }
    }
}

impl std::fmt::Display for ResourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Level of support for a resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportLevel {
    /// Full support with custom file creation
    Full,
    /// Supported but uses a different/incompatible format
    CustomFormat,
    /// Limited support (e.g., built-in only, no custom)
    Limited,
    /// Not supported by this provider
    None,
}

impl SupportLevel {
    /// Returns whether this level indicates any form of support.
    pub fn is_supported(&self) -> bool {
        !matches!(self, SupportLevel::None)
    }

    /// Returns whether this level allows custom resource creation.
    pub fn allows_custom(&self) -> bool {
        matches!(self, SupportLevel::Full | SupportLevel::CustomFormat)
    }

    /// Returns the indicator character for this support level.
    pub fn indicator(&self) -> &'static str {
        match self {
            SupportLevel::Full => "\u{2705}",                 // ✅
            SupportLevel::CustomFormat => "\u{2699}\u{fe0f}", // ⚙️
            SupportLevel::Limited => "\u{25cb}",              // ○
            SupportLevel::None => "\u{274c}",                 // ❌
        }
    }

    /// Returns a short description of this support level.
    pub fn description(&self) -> &'static str {
        match self {
            SupportLevel::Full => "Full support",
            SupportLevel::CustomFormat => "Custom format",
            SupportLevel::Limited => "Limited/built-in only",
            SupportLevel::None => "Not supported",
        }
    }
}

/// Compatibility metadata for resource frontmatter/config fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourcePropertySchema {
    /// Properties required by the provider for this resource type.
    pub required: &'static [&'static str],
    /// Properties optionally recognized by the provider.
    pub optional: &'static [&'static str],
    /// Source doc used to derive this schema metadata.
    pub source_doc: &'static str,
}

impl ResourcePropertySchema {
    /// Build a schema descriptor from required/optional fields.
    pub const fn new(
        required: &'static [&'static str],
        optional: &'static [&'static str],
        source_doc: &'static str,
    ) -> Self {
        Self {
            required,
            optional,
            source_doc,
        }
    }
}

/// Support details for a specific resource type.
#[derive(Debug, Clone)]
pub struct ResourceSupport {
    /// Level of support
    pub level: SupportLevel,
    /// File format used (if supported)
    pub format: Option<ResourceFormat>,
    /// Repository-scoped directory path (relative to repo root)
    pub repo_path: Option<PathBuf>,
    /// User-scoped directory path (relative to home)
    pub user_path: Option<PathBuf>,
    /// Additional directories this provider reads from (e.g., OpenCode reads .claude/)
    pub also_reads_from: Vec<PathBuf>,
    /// Notes about this resource support
    pub notes: Option<&'static str>,
    /// Required/optional property metadata from cross-referencing docs.
    pub properties: Option<ResourcePropertySchema>,
}

impl ResourceSupport {
    /// Create a new ResourceSupport for a fully supported resource.
    pub fn full(format: ResourceFormat, repo_path: &str, user_path: &str) -> Self {
        Self {
            level: SupportLevel::Full,
            format: Some(format),
            repo_path: Some(PathBuf::from(repo_path)),
            user_path: Some(PathBuf::from(user_path)),
            also_reads_from: vec![],
            notes: None,
            properties: None,
        }
    }

    /// Create a ResourceSupport with a custom format.
    pub fn custom_format(format: ResourceFormat, repo_path: &str, user_path: &str) -> Self {
        Self {
            level: SupportLevel::CustomFormat,
            format: Some(format),
            repo_path: Some(PathBuf::from(repo_path)),
            user_path: Some(PathBuf::from(user_path)),
            also_reads_from: vec![],
            notes: None,
            properties: None,
        }
    }

    /// Create a ResourceSupport for limited/built-in only support.
    pub fn limited() -> Self {
        Self {
            level: SupportLevel::Limited,
            format: Some(ResourceFormat::BuiltinOnly),
            repo_path: None,
            user_path: None,
            also_reads_from: vec![],
            notes: None,
            properties: None,
        }
    }

    /// Create a ResourceSupport for no support.
    pub fn none() -> Self {
        Self {
            level: SupportLevel::None,
            format: None,
            repo_path: None,
            user_path: None,
            also_reads_from: vec![],
            notes: None,
            properties: None,
        }
    }

    /// Add directories that this provider also reads from.
    pub fn with_also_reads(mut self, paths: Vec<&str>) -> Self {
        self.also_reads_from = paths.into_iter().map(PathBuf::from).collect();
        self
    }

    /// Add a note about this resource support.
    pub fn with_note(mut self, note: &'static str) -> Self {
        self.notes = Some(note);
        self
    }

    /// Add compatibility metadata from cross-reference docs.
    pub fn with_properties(mut self, schema: ResourcePropertySchema) -> Self {
        self.properties = Some(schema);
        self
    }

    /// Required properties for this provider/resource combination.
    pub fn required_properties(&self) -> &'static [&'static str] {
        self.properties.map(|schema| schema.required).unwrap_or(&[])
    }

    /// Optional properties for this provider/resource combination.
    pub fn optional_properties(&self) -> &'static [&'static str] {
        self.properties.map(|schema| schema.optional).unwrap_or(&[])
    }

    /// Source doc where property metadata was derived from.
    pub fn property_source_doc(&self) -> Option<&'static str> {
        self.properties.map(|schema| schema.source_doc)
    }
}

/// Skill frontmatter fields supported by a provider.
#[derive(Debug, Clone, Default)]
pub struct SkillFrontmatter {
    /// `name` field (skill identifier)
    pub name: bool,
    /// `description` field (when to use the skill)
    pub description: bool,
    /// `license` field (SPDX identifier)
    pub license: bool,
    /// `compatibility` field (environment requirements)
    pub compatibility: bool,
    /// `metadata` field (custom key-value pairs)
    pub metadata: bool,
    /// `allowed-tools` field (tool permissions)
    pub allowed_tools: bool,
    /// `user-invocable` field (manual invocation)
    pub user_invocable: bool,
    /// `disable-model-invocation` field (auto invocation)
    pub disable_model_invocation: bool,
}

impl SkillFrontmatter {
    /// Standard frontmatter (name, description)
    pub fn standard() -> Self {
        Self {
            name: true,
            description: true,
            ..Default::default()
        }
    }

    /// Full frontmatter support (Claude-style)
    pub fn full() -> Self {
        Self {
            name: true,
            description: true,
            license: true,
            compatibility: false,
            metadata: true,
            allowed_tools: false,
            user_invocable: true,
            disable_model_invocation: true,
        }
    }

    /// Extended frontmatter (with compatibility and metadata)
    pub fn extended() -> Self {
        Self {
            name: true,
            description: true,
            license: true,
            compatibility: true,
            metadata: true,
            allowed_tools: false,
            user_invocable: false,
            disable_model_invocation: false,
        }
    }

    /// With allowed-tools support
    pub fn with_allowed_tools(mut self) -> Self {
        self.allowed_tools = true;
        self
    }
}

/// Complete capability set for a provider.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    /// The provider these capabilities describe
    pub provider: Provider,
    /// Skill support details
    pub skills: ResourceSupport,
    /// Command support details
    pub commands: ResourceSupport,
    /// Agent/subagent support details
    pub agents: ResourceSupport,
    /// Script support details
    pub scripts: ResourceSupport,
    /// Skill frontmatter fields supported
    pub skill_frontmatter: SkillFrontmatter,
}

impl ProviderCapabilities {
    /// Get support for a specific resource type.
    pub fn support_for(&self, resource: LinkableResource) -> &ResourceSupport {
        match resource {
            LinkableResource::Skill => &self.skills,
            LinkableResource::Command => &self.commands,
            LinkableResource::Agent => &self.agents,
            LinkableResource::Script => &self.scripts,
        }
    }

    /// Get the support level for a resource type.
    pub fn support_level(&self, resource: LinkableResource) -> SupportLevel {
        self.support_for(resource).level
    }

    /// Required property names for the given resource type.
    pub fn required_properties_for(&self, resource: LinkableResource) -> &'static [&'static str] {
        self.support_for(resource).required_properties()
    }

    /// Optional property names for the given resource type.
    pub fn optional_properties_for(&self, resource: LinkableResource) -> &'static [&'static str] {
        self.support_for(resource).optional_properties()
    }
}

/// Get capabilities for a specific provider.
pub fn capabilities_for(provider: Provider) -> ProviderCapabilities {
    match provider {
        Provider::Claude => claude_capabilities(),
        Provider::Codex => codex_capabilities(),
        Provider::Gemini => gemini_capabilities(),
        Provider::Goose => goose_capabilities(),
        Provider::KimiCode => kimicode_capabilities(),
        Provider::OpenCode => opencode_capabilities(),
        Provider::QwenCode => qwencode_capabilities(),
        Provider::RooCode => roocode_capabilities(),
    }
}

/// Get capabilities for all providers.
pub fn all_capabilities() -> Vec<ProviderCapabilities> {
    vec![
        claude_capabilities(),
        codex_capabilities(),
        gemini_capabilities(),
        goose_capabilities(),
        kimicode_capabilities(),
        opencode_capabilities(),
        qwencode_capabilities(),
        roocode_capabilities(),
    ]
}

/// All providers in display order.
pub const ALL_PROVIDERS: [Provider; 8] = [
    Provider::Claude,
    Provider::Codex,
    Provider::Gemini,
    Provider::Goose,
    Provider::KimiCode,
    Provider::OpenCode,
    Provider::QwenCode,
    Provider::RooCode,
];

// ============================================================================
// Provider-specific capability definitions
// ============================================================================

const CLAUDE_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &[
        "name",
        "description",
        "argument-hint",
        "disable-model-invocation",
        "user-invocable",
        "allowed-tools",
        "model",
        "context",
        "agent",
        "hooks",
    ],
    "claudine/docs/cross-referencing/claude-code.md",
);
const CLAUDE_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &[
        "name",
        "description",
        "argument-hint",
        "disable-model-invocation",
        "user-invocable",
        "allowed-tools",
        "model",
        "context",
        "agent",
        "hooks",
    ],
    "claudine/docs/cross-referencing/claude-code.md",
);
const CLAUDE_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[
        "tools",
        "disallowedTools",
        "model",
        "permissionMode",
        "maxTurns",
        "skills",
        "mcpServers",
        "hooks",
        "memory",
    ],
    "claudine/docs/cross-referencing/claude-code.md",
);

const CODEX_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata"],
    "claudine/docs/cross-referencing/codex.md",
);
const CODEX_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "argument-hint"],
    "claudine/docs/cross-referencing/codex.md",
);

const GEMINI_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/gemini-cli.md",
);
const GEMINI_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["prompt"],
    &["description"],
    "claudine/docs/cross-referencing/gemini-cli.md",
);
const GEMINI_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[
        "kind",
        "tools",
        "model",
        "temperature",
        "max_turns",
        "timeout_mins",
    ],
    "claudine/docs/cross-referencing/gemini-cli.md",
);

const GOOSE_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata", "allowed-tools"],
    "claudine/docs/cross-referencing/goose.md",
);
const GOOSE_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["title", "description"],
    &[
        "instructions",
        "prompt",
        "extensions",
        "parameters",
        "sub_recipes",
    ],
    "claudine/docs/cross-referencing/goose.md",
);

const KIMI_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &[
        "name",
        "description",
        "license",
        "compatibility",
        "metadata",
        "type",
    ],
    "claudine/docs/cross-referencing/kimi-code.md",
);
const KIMI_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "system_prompt_path", "tools"],
    &["extend", "system_prompt_args", "exclude_tools", "subagents"],
    "claudine/docs/cross-referencing/kimi-code.md",
);

const OPENCODE_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["license", "compatibility", "metadata"],
    "claudine/docs/cross-referencing/opencode.md",
);
const OPENCODE_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "template", "agent", "model", "subtask"],
    "claudine/docs/cross-referencing/opencode.md",
);
const OPENCODE_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["description"],
    &[
        "mode",
        "model",
        "temperature",
        "top_p",
        "tools",
        "permission",
        "steps",
        "color",
        "hidden",
        "disable",
        "prompt",
    ],
    "claudine/docs/cross-referencing/opencode.md",
);

const QWEN_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/qwen-cli.md",
);
const QWEN_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description"],
    "claudine/docs/cross-referencing/qwen-cli.md",
);
const QWEN_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &["tools", "color"],
    "claudine/docs/cross-referencing/qwen-cli.md",
);

const ROO_SKILL_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["name", "description"],
    &[],
    "claudine/docs/cross-referencing/roo-code.md",
);
const ROO_COMMAND_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &[],
    &["description", "argument-hint", "mode"],
    "claudine/docs/cross-referencing/roo-code.md",
);
const ROO_AGENT_SCHEMA: ResourcePropertySchema = ResourcePropertySchema::new(
    &["slug", "name", "roleDefinition", "groups"],
    &["description", "whenToUse", "customInstructions"],
    "claudine/docs/cross-referencing/roo-code.md",
);

fn claude_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Claude,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".claude/skills", ".claude/skills")
            .with_properties(CLAUDE_SKILL_SCHEMA),
        commands: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".claude/commands",
            ".claude/commands",
        )
        .with_properties(CLAUDE_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".claude/agents", ".claude/agents")
            .with_properties(CLAUDE_AGENT_SCHEMA),
        scripts: ResourceSupport::none().with_note("Scripts are stored within skill directories"),
        skill_frontmatter: SkillFrontmatter::full(),
    }
}

fn codex_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Codex,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".codex/skills", ".codex/skills")
            .with_also_reads(vec![".claude/skills", ".agents/skills"])
            .with_properties(CODEX_SKILL_SCHEMA),
        // Codex custom prompts are deprecated but still supported from user scope.
        // Current Codex docs state prompt files live in the local Codex home and
        // are not shared through the repository, so repo_path is intentionally empty.
        // TODO: remove when Codex fully drops prompt files.
        commands: ResourceSupport::custom_format(
            ResourceFormat::Markdown,
            "",
            ".codex/prompts",
        )
        .with_note("Deprecated custom prompts; user scope only; prefer skills")
        .with_properties(CODEX_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".codex/agents", ".codex/agents"),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".codex/scripts",
            ".codex/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

fn gemini_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Gemini,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".gemini/skills", ".gemini/skills")
            .with_note("Also supports .gemini/modules/ as context modules")
            .with_properties(GEMINI_SKILL_SCHEMA),
        commands: ResourceSupport::custom_format(
            ResourceFormat::Toml,
            ".gemini/commands",
            ".gemini/commands",
        )
        .with_note("Uses TOML format with {{args}} placeholder")
        .with_properties(GEMINI_COMMAND_SCHEMA),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Markdown,
            ".gemini/agents",
            ".gemini/agents",
        )
        .with_note("Sub-agent markdown definitions are experimental")
        .with_properties(GEMINI_AGENT_SCHEMA),
        scripts: ResourceSupport::none().with_note("Scripts stored within skill directories"),
        skill_frontmatter: SkillFrontmatter::standard(),
    }
}

fn goose_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::Goose,
        skills: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".goose/skills",
            ".config/goose/skills",
        )
        .with_also_reads(vec![".claude/skills", ".agents/skills"])
        .with_properties(GOOSE_SKILL_SCHEMA),
        commands: ResourceSupport::custom_format(ResourceFormat::Mcp, "", "")
            .with_note("MCP-based commands, not file-based"),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".goose/recipes",
            ".config/goose/recipes",
        )
        .with_note("Recipe YAML files with specific schema")
        .with_properties(GOOSE_AGENT_SCHEMA),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".goose/scripts",
            ".config/goose/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended().with_allowed_tools(),
    }
}

fn kimicode_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::KimiCode,
        skills: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".kimi/skills",
            ".config/agents/skills",
        )
        .with_also_reads(vec![".claude/skills", ".agents/skills", ".codex/skills"])
        .with_properties(KIMI_SKILL_SCHEMA),
        commands: ResourceSupport::limited().with_note("Built-in slash commands only"),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".kimi/agents",
            ".kimi/agents",
        )
        .with_note("YAML agent files loaded via --agent-file flag")
        .with_properties(KIMI_AGENT_SCHEMA),
        scripts: ResourceSupport::none().with_note("Scripts stored within skill directories"),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

fn opencode_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::OpenCode,
        skills: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".opencode/skills",
            ".config/opencode/skills",
        )
        .with_also_reads(vec![".claude/skills", ".agents/skills"])
        .with_properties(OPENCODE_SKILL_SCHEMA),
        commands: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".opencode/commands",
            ".config/opencode/commands",
        )
        .with_properties(OPENCODE_COMMAND_SCHEMA),
        agents: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".opencode/agents",
            ".config/opencode/agents",
        )
        .with_properties(OPENCODE_AGENT_SCHEMA),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".opencode/scripts",
            ".config/opencode/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::extended(),
    }
}

fn qwencode_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::QwenCode,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".qwen/skills", ".qwen/skills")
            .with_note("Skills support is experimental")
            .with_properties(QWEN_SKILL_SCHEMA),
        commands: ResourceSupport::full(
            ResourceFormat::Markdown,
            ".qwen/commands",
            ".qwen/commands",
        )
        .with_note("Markdown custom commands; TOML remains deprecated fallback")
        .with_properties(QWEN_COMMAND_SCHEMA),
        agents: ResourceSupport::full(ResourceFormat::Markdown, ".qwen/agents", ".qwen/agents")
            .with_properties(QWEN_AGENT_SCHEMA),
        scripts: ResourceSupport::full(
            ResourceFormat::Executable,
            ".qwen/scripts",
            ".qwen/scripts",
        ),
        skill_frontmatter: SkillFrontmatter::standard().with_allowed_tools(),
    }
}

fn roocode_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        provider: Provider::RooCode,
        skills: ResourceSupport::full(ResourceFormat::Markdown, ".roo/skills", ".roo/skills")
            .with_note("Roo skills are compatible with markdown skill bundles")
            .with_properties(ROO_SKILL_SCHEMA),
        commands: ResourceSupport::full(ResourceFormat::Markdown, ".roo/commands", ".roo/commands")
            .with_properties(ROO_COMMAND_SCHEMA),
        agents: ResourceSupport::custom_format(
            ResourceFormat::Yaml,
            ".roomodes",
            ".roo/custom_modes.yaml",
        )
        .with_note("Mode definitions via .roomodes/custom_modes.yaml")
        .with_properties(ROO_AGENT_SCHEMA),
        scripts: ResourceSupport::full(ResourceFormat::Executable, ".roo/scripts", ".roo/scripts"),
        skill_frontmatter: SkillFrontmatter::standard().with_allowed_tools(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_providers_have_capabilities() {
        for provider in ALL_PROVIDERS {
            let caps = capabilities_for(provider);
            assert_eq!(caps.provider, provider);
        }
    }

    #[test]
    fn all_capabilities_returns_all_providers() {
        let caps = all_capabilities();
        assert_eq!(caps.len(), 8);
    }

    #[test]
    fn claude_has_full_skill_support() {
        let caps = capabilities_for(Provider::Claude);
        assert_eq!(caps.skills.level, SupportLevel::Full);
        assert_eq!(caps.skills.format, Some(ResourceFormat::Markdown));
        assert_eq!(caps.skills.repo_path, Some(PathBuf::from(".claude/skills")));
    }

    #[test]
    fn claude_has_full_command_support() {
        let caps = capabilities_for(Provider::Claude);
        assert_eq!(caps.commands.level, SupportLevel::Full);
        assert_eq!(caps.commands.format, Some(ResourceFormat::Markdown));
    }

    #[test]
    fn gemini_commands_use_toml() {
        let caps = capabilities_for(Provider::Gemini);
        assert_eq!(caps.commands.level, SupportLevel::CustomFormat);
        assert_eq!(caps.commands.format, Some(ResourceFormat::Toml));
    }

    #[test]
    fn goose_commands_use_mcp() {
        let caps = capabilities_for(Provider::Goose);
        assert_eq!(caps.commands.level, SupportLevel::CustomFormat);
        assert_eq!(caps.commands.format, Some(ResourceFormat::Mcp));
    }

    #[test]
    fn goose_agents_use_yaml() {
        let caps = capabilities_for(Provider::Goose);
        assert_eq!(caps.agents.level, SupportLevel::CustomFormat);
        assert_eq!(caps.agents.format, Some(ResourceFormat::Yaml));
    }

    #[test]
    fn kimicode_commands_are_limited() {
        let caps = capabilities_for(Provider::KimiCode);
        assert_eq!(caps.commands.level, SupportLevel::Limited);
    }

    #[test]
    fn qwencode_commands_are_markdown() {
        let caps = capabilities_for(Provider::QwenCode);
        assert_eq!(caps.commands.level, SupportLevel::Full);
        assert_eq!(caps.commands.format, Some(ResourceFormat::Markdown));
    }

    #[test]
    fn roocode_commands_are_markdown() {
        let caps = capabilities_for(Provider::RooCode);
        assert_eq!(caps.commands.level, SupportLevel::Full);
        assert_eq!(caps.commands.format, Some(ResourceFormat::Markdown));
    }

    #[test]
    fn opencode_reads_from_claude() {
        let caps = capabilities_for(Provider::OpenCode);
        assert!(
            caps.skills
                .also_reads_from
                .contains(&PathBuf::from(".claude/skills"))
        );
    }

    #[test]
    fn goose_reads_from_multiple_sources() {
        let caps = capabilities_for(Provider::Goose);
        assert!(
            caps.skills
                .also_reads_from
                .contains(&PathBuf::from(".claude/skills"))
        );
        assert!(
            caps.skills
                .also_reads_from
                .contains(&PathBuf::from(".agents/skills"))
        );
    }

    #[test]
    fn support_level_indicators() {
        assert_eq!(SupportLevel::Full.indicator(), "\u{2705}");
        assert_eq!(SupportLevel::CustomFormat.indicator(), "\u{2699}\u{fe0f}");
        assert_eq!(SupportLevel::Limited.indicator(), "\u{25cb}");
        assert_eq!(SupportLevel::None.indicator(), "\u{274c}");
    }

    #[test]
    fn linkable_resource_all_types() {
        assert_eq!(LinkableResource::ALL.len(), 4);
    }

    #[test]
    fn resource_format_display() {
        assert_eq!(ResourceFormat::Markdown.name(), "Markdown");
        assert_eq!(ResourceFormat::Toml.abbrev(), ".toml");
    }

    #[test]
    fn skill_frontmatter_variants() {
        let standard = SkillFrontmatter::standard();
        assert!(standard.name);
        assert!(standard.description);
        assert!(!standard.license);

        let full = SkillFrontmatter::full();
        assert!(full.name);
        assert!(full.description);
        assert!(full.license);
        assert!(full.user_invocable);

        let extended = SkillFrontmatter::extended();
        assert!(extended.compatibility);
        assert!(extended.metadata);
    }

    #[test]
    fn support_for_resource() {
        let caps = capabilities_for(Provider::Claude);
        let skill_support = caps.support_for(LinkableResource::Skill);
        assert_eq!(skill_support.level, SupportLevel::Full);

        let script_support = caps.support_for(LinkableResource::Script);
        assert_eq!(script_support.level, SupportLevel::None);
    }

    #[test]
    fn support_level_predicates() {
        assert!(SupportLevel::Full.is_supported());
        assert!(SupportLevel::CustomFormat.is_supported());
        assert!(SupportLevel::Limited.is_supported());
        assert!(!SupportLevel::None.is_supported());

        assert!(SupportLevel::Full.allows_custom());
        assert!(SupportLevel::CustomFormat.allows_custom());
        assert!(!SupportLevel::Limited.allows_custom());
        assert!(!SupportLevel::None.allows_custom());
    }

    #[test]
    fn capability_metadata_exposes_required_and_optional_fields() {
        let caps = capabilities_for(Provider::OpenCode);
        assert_eq!(
            caps.required_properties_for(LinkableResource::Skill),
            &["name", "description"]
        );
        assert!(
            caps.optional_properties_for(LinkableResource::Skill)
                .contains(&"metadata")
        );
        assert_eq!(
            caps.skills.property_source_doc(),
            Some("claudine/docs/cross-referencing/opencode.md")
        );
    }
}
