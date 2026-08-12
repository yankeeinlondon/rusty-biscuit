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

use serde::Serialize;

use crate::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use std::path::PathBuf;

/// Types of linkable resources across providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Default, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
///
/// Phase 2 of the centralized providers refactor moved the per-provider
/// data construction into [`crate::provider`]. This function is now a thin
/// forwarding facade that clones the cached value held by the central
/// catalog.
pub fn capabilities_for(provider: Provider) -> ProviderCapabilities {
    crate::provider::provider_info(provider)
        .resource_support()
        .clone()
}

/// All providers in display order.
pub const ALL_PROVIDERS: [Provider; crate::provider::PROVIDER_COUNT] = PROVIDERS_DISPLAY_ORDER;

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
