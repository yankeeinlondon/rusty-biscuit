use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::events::Provider;

/// Scope classification for a detected resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceScope {
    /// Resource exists in repository scope.
    Repo,
    /// Resource exists in both scopes, and repo scope masks user scope.
    RepoMasked,
    /// Resource exists in user scope.
    User,
}

/// Status class for a resource reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceStatus {
    /// Resource state is healthy and actionable without intervention.
    Ok,
    /// Resource state is unhealthy but automatically fixable.
    IsFixable,
    /// Resource state requires explicit user attention.
    NeedsUserAttention,
}

/// Canonical definition of a resource instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDefinition {
    /// Resource name.
    pub name: String,
    /// Provider this definition belongs to.
    pub provider: Provider,
    /// Scope where this definition lives.
    pub scope: ResourceScope,
    /// Filesystem location.
    pub filepath: PathBuf,
    /// Parsed frontmatter key/value representation.
    pub frontmatter: BTreeMap<String, String>,
    /// Body content for markdown-like resources.
    pub body: String,
    /// Stable hash for frontmatter content.
    pub fm_hash: u64,
    /// Stable hash for body content.
    pub body_hash: u64,
}

/// Detailed resource reference state used by analyze/list/fix workflows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceReference {
    /// Canonical source with all required properties.
    Source(ResourceDefinition),
    /// Canonical source with unmet required properties.
    PartialSource(ResourceDefinition, Vec<String>),
    /// Standalone resource not linked/derived from canonical source.
    Isolated(ResourceDefinition),
    /// Symlink to canonical source.
    Link(Provider, ResourceScope),
    /// Symlink should exist but is missing.
    LinkMissing(Provider, ResourceScope),
    /// Symlink blocked by incomplete canonical properties.
    IncompleteLink(Provider, ResourceScope),
    /// Derived representation exists and matches canonical source hashes.
    DerivedLink(Provider, ResourceScope),
    /// Derived representation exists but is stale.
    DerivedStale(Provider, ResourceScope),
    /// Derived representation should exist but is missing.
    DerivedMissing(Provider, ResourceScope),
}

impl ResourceReference {
    /// Provider associated with this resource reference.
    pub fn provider(&self) -> Provider {
        match self {
            ResourceReference::Source(definition)
            | ResourceReference::PartialSource(definition, _)
            | ResourceReference::Isolated(definition) => definition.provider,
            ResourceReference::Link(provider, _)
            | ResourceReference::LinkMissing(provider, _)
            | ResourceReference::IncompleteLink(provider, _)
            | ResourceReference::DerivedLink(provider, _)
            | ResourceReference::DerivedStale(provider, _)
            | ResourceReference::DerivedMissing(provider, _) => *provider,
        }
    }

    /// Status classification mapped from reference variant semantics.
    pub fn status(&self) -> ReferenceStatus {
        match self {
            ResourceReference::Source(_)
            | ResourceReference::Isolated(_)
            | ResourceReference::Link(_, _)
            | ResourceReference::DerivedLink(_, _) => ReferenceStatus::Ok,
            ResourceReference::DerivedStale(_, _)
            | ResourceReference::DerivedMissing(_, _)
            | ResourceReference::LinkMissing(_, _) => ReferenceStatus::IsFixable,
            ResourceReference::PartialSource(_, _) | ResourceReference::IncompleteLink(_, _) => {
                ReferenceStatus::NeedsUserAttention
            }
        }
    }
}

/// Cross-provider resource kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// Skill definitions.
    Skill,
    /// Slash command definitions.
    SlashCommand,
    /// Agent/subagent definitions.
    AgentDefinition,
    /// Shared scripts directory resources.
    SharedScripts,
}

/// Unified resource representation used by detector/analyzer layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    /// Resource name.
    pub name: String,
    /// Resource kind.
    pub kind: ResourceType,
    /// Scope classification.
    pub scope: ResourceScope,
    /// Resource provider.
    pub provider: Provider,
    /// Definition state.
    pub definition: ResourceReference,
}

/// Converter function signature used by bespoke conversion entries.
pub type BespokeConverter = fn(String) -> String;

/// Supported conversion strategy classes.
#[derive(Clone, Copy)]
pub enum ResourceFormatConversion {
    /// YAML conversion.
    Yaml,
    /// TOML conversion.
    Toml,
    /// Provider/resource bespoke conversion pair.
    Bespoke((BespokeConverter, BespokeConverter)),
}

/// Validation errors for provider ordering configuration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigurationError {
    /// Provider preference list is missing.
    #[error("provider preference is not configured")]
    MissingProviderPreference,
}

/// User-configured ordering preference for providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderOrdering(pub Vec<Provider>);

impl ProviderOrdering {
    /// Build ordering from configured provider preference list.
    ///
    /// Returns an error when the list is empty.
    pub fn new(preference: &[Provider]) -> Result<Self, ConfigurationError> {
        if preference.is_empty() {
            return Err(ConfigurationError::MissingProviderPreference);
        }
        Ok(Self(preference.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_definition(provider: Provider) -> ResourceDefinition {
        ResourceDefinition {
            name: "sample".to_string(),
            provider,
            scope: ResourceScope::User,
            filepath: PathBuf::from("/tmp/sample.md"),
            frontmatter: BTreeMap::new(),
            body: "body".to_string(),
            fm_hash: 1,
            body_hash: 2,
        }
    }

    #[test]
    fn provider_method_maps_all_variants() {
        let definition = sample_definition(Provider::Claude);
        let refs = vec![
            ResourceReference::Source(definition.clone()),
            ResourceReference::PartialSource(definition.clone(), vec!["name".to_string()]),
            ResourceReference::Isolated(definition),
            ResourceReference::Link(Provider::Codex, ResourceScope::Repo),
            ResourceReference::LinkMissing(Provider::Gemini, ResourceScope::Repo),
            ResourceReference::IncompleteLink(Provider::OpenCode, ResourceScope::Repo),
            ResourceReference::DerivedLink(Provider::QwenCode, ResourceScope::Repo),
            ResourceReference::DerivedStale(Provider::RooCode, ResourceScope::Repo),
            ResourceReference::DerivedMissing(Provider::Goose, ResourceScope::Repo),
        ];

        let providers: Vec<Provider> = refs.iter().map(ResourceReference::provider).collect();
        assert_eq!(
            providers,
            vec![
                Provider::Claude,
                Provider::Claude,
                Provider::Claude,
                Provider::Codex,
                Provider::Gemini,
                Provider::OpenCode,
                Provider::QwenCode,
                Provider::RooCode,
                Provider::Goose,
            ]
        );
    }

    #[test]
    fn status_mapping_covers_all_resource_reference_variants() {
        let definition = sample_definition(Provider::Claude);
        let cases = vec![
            (
                ResourceReference::Source(definition.clone()),
                ReferenceStatus::Ok,
            ),
            (
                ResourceReference::PartialSource(definition.clone(), vec!["name".to_string()]),
                ReferenceStatus::NeedsUserAttention,
            ),
            (ResourceReference::Isolated(definition), ReferenceStatus::Ok),
            (
                ResourceReference::Link(Provider::Codex, ResourceScope::Repo),
                ReferenceStatus::Ok,
            ),
            (
                ResourceReference::LinkMissing(Provider::Codex, ResourceScope::Repo),
                ReferenceStatus::IsFixable,
            ),
            (
                ResourceReference::IncompleteLink(Provider::Codex, ResourceScope::Repo),
                ReferenceStatus::NeedsUserAttention,
            ),
            (
                ResourceReference::DerivedLink(Provider::Codex, ResourceScope::Repo),
                ReferenceStatus::Ok,
            ),
            (
                ResourceReference::DerivedStale(Provider::Codex, ResourceScope::Repo),
                ReferenceStatus::IsFixable,
            ),
            (
                ResourceReference::DerivedMissing(Provider::Codex, ResourceScope::Repo),
                ReferenceStatus::IsFixable,
            ),
        ];

        for (reference, expected) in cases {
            assert_eq!(reference.status(), expected);
        }
    }

    #[test]
    fn provider_ordering_requires_non_empty_preference() {
        let empty = ProviderOrdering::new(&[]);
        assert!(matches!(
            empty,
            Err(ConfigurationError::MissingProviderPreference)
        ));

        let ordered = ProviderOrdering::new(&[Provider::Codex, Provider::Claude]).unwrap();
        assert_eq!(ordered.0, vec![Provider::Codex, Provider::Claude]);
    }
}
