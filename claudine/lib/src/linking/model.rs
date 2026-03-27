use std::collections::BTreeMap;
use std::fmt;
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

/// Root cause for an `IncompleteLink` classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompleteCause {
    /// Canonical source reference has no definition (neither Source nor PartialSource).
    NoCanonicalDefinition,
    /// Target provider does not support custom resources for this type.
    CustomNotSupported,
    /// Target provider requires frontmatter properties that the canonical source lacks.
    MissingProperties(Vec<String>),
    /// Target provider has no configured resource format.
    NoTargetFormat,
    /// Existing symlink points to a different target than the canonical source.
    WrongSymlinkTarget,
}

impl fmt::Display for IncompleteCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IncompleteCause::NoCanonicalDefinition => {
                write!(f, "no canonical definition available")
            }
            IncompleteCause::CustomNotSupported => {
                write!(f, "custom resources not supported")
            }
            IncompleteCause::MissingProperties(props) => {
                write!(f, "missing required properties: {}", props.join(", "))
            }
            IncompleteCause::NoTargetFormat => {
                write!(f, "no target format configured")
            }
            IncompleteCause::WrongSymlinkTarget => {
                write!(f, "symlink points to wrong target")
            }
        }
    }
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
    IncompleteLink(Provider, ResourceScope, IncompleteCause),
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
            | ResourceReference::IncompleteLink(provider, _, _) => *provider,
        }
    }

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
            ResourceReference::IncompleteLink(
                Provider::OpenCode,
                ResourceScope::Repo,
                IncompleteCause::NoCanonicalDefinition,
            ),
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
            ]
        );
    }

    #[test]
    fn incomplete_cause_display() {
        assert_eq!(
            IncompleteCause::NoCanonicalDefinition.to_string(),
            "no canonical definition available"
        );
        assert_eq!(
            IncompleteCause::CustomNotSupported.to_string(),
            "custom resources not supported"
        );
        assert_eq!(
            IncompleteCause::MissingProperties(vec!["slug".into(), "roleDefinition".into()])
                .to_string(),
            "missing required properties: slug, roleDefinition"
        );
        assert_eq!(
            IncompleteCause::NoTargetFormat.to_string(),
            "no target format configured"
        );
        assert_eq!(
            IncompleteCause::WrongSymlinkTarget.to_string(),
            "symlink points to wrong target"
        );
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
