use std::collections::HashSet;

use crate::markdown::compose::expression::ExpressionFinder;

use super::{agent, changes, datetime, docs, git, host, invocation, languages, repo};

/// Independently captured runtime-context domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextGroup {
    /// Invocation-owned values that require no host or repository discovery.
    Invocation,
    /// Clock, calendar, and timezone values.
    DateTime,
    /// Branch, worktree, and merge-conflict values.
    Git,
    /// Repository and package-topology values.
    Repo,
    /// Working-tree and package change values.
    FileChanges,
    /// Programming-language and package-manager values.
    Languages,
    /// Markdown-document and matching-skill values.
    Documents,
    /// Operating-system values.
    Os,
    /// CPU and memory values.
    Hardware,
    /// GPU values.
    Gpu,
    /// Agent and model values derived from the captured environment.
    Agent,
}

impl ContextGroup {
    pub(crate) fn all() -> [Self; 11] {
        [
            Self::Invocation,
            Self::DateTime,
            Self::Git,
            Self::Repo,
            Self::FileChanges,
            Self::Languages,
            Self::Documents,
            Self::Os,
            Self::Hardware,
            Self::Gpu,
            Self::Agent,
        ]
    }

    /// Stable identifier persisted in compose-cache manifests.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Invocation => "invocation",
            Self::DateTime => "datetime",
            Self::Git => "git",
            Self::Repo => "repo",
            Self::FileChanges => "file_changes",
            Self::Languages => "languages",
            Self::Documents => "documents",
            Self::Os => "os",
            Self::Hardware => "hardware",
            Self::Gpu => "gpu",
            Self::Agent => "agent",
        }
    }

    /// The group persisted as [`name`](Self::name).
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|group| group.name() == name)
    }

    pub(crate) fn for_key(key: &str) -> Option<Self> {
        group_for_key(key)
    }

    /// Every `ctx.*` key a capture of this group projects, date/time aliases
    /// included. A captured group whose values omit one of these keys is a
    /// malformed snapshot (`ContextProjectionInvariant`).
    pub(crate) fn projected_keys(self) -> impl Iterator<Item = &'static str> {
        let (keys, aliases): (&'static [&'static str], &'static [&'static str]) = match self {
            Self::Invocation => (invocation::KEYS, &[]),
            Self::DateTime => (datetime::KEYS, datetime::ALIASES),
            Self::Git => (git::KEYS, &[]),
            Self::Repo => (repo::KEYS, &[]),
            Self::FileChanges => (changes::KEYS, &[]),
            Self::Languages => (languages::KEYS, &[]),
            Self::Documents => (docs::KEYS, &[]),
            Self::Os => (host::OS_KEYS, &[]),
            Self::Hardware => (host::HARDWARE_KEYS, &[]),
            Self::Gpu => (host::GPU_KEYS, &[]),
            Self::Agent => (agent::KEYS, &[]),
        };
        keys.iter().chain(aliases).copied()
    }
}

/// Runtime-context groups required to compose one document or content fragment.
///
/// The set describes capture requirements only; it does not expose how a group
/// is populated. Date/time is always included because every existing
/// demand-driven capture entry point provides those zero-discovery values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextRequirements {
    groups: HashSet<ContextGroup>,
}

impl ContextRequirements {
    /// Scans content for active `ctx.*` references.
    pub fn for_content(content: &str) -> Self {
        let mut groups = scan_needed_groups(content);
        groups.insert(ContextGroup::DateTime);
        Self { groups }
    }

    /// Scans both authored frontmatter values and the document body.
    pub fn for_document(document: &crate::markdown::Markdown) -> Self {
        let frontmatter =
            serde_json::to_string(document.frontmatter().as_map()).unwrap_or_default();
        Self::for_content(&format!("{frontmatter}\n{}", document.content()))
    }

    /// Requires every public runtime-context group.
    pub fn all() -> Self {
        Self {
            groups: ContextGroup::all().into_iter().collect(),
        }
    }

    /// Whether this capture requires `group`.
    pub fn contains(&self, group: ContextGroup) -> bool {
        self.groups.contains(&group)
    }

    /// This requirement set plus `group`.
    pub(crate) fn with(mut self, group: ContextGroup) -> Self {
        self.groups.insert(group);
        self
    }

    /// Every group required by this set or by `other`.
    pub(crate) fn union(&self, other: &Self) -> Self {
        Self {
            groups: self.groups.union(&other.groups).copied().collect(),
        }
    }

    /// Iterates the required groups in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = ContextGroup> + '_ {
        self.groups.iter().copied()
    }

    pub(crate) fn from_groups(groups: impl IntoIterator<Item = ContextGroup>) -> Self {
        Self {
            groups: groups.into_iter().collect(),
        }
    }
}

fn group_for_key(key: &str) -> Option<ContextGroup> {
    ContextGroup::all()
        .into_iter()
        .find(|group| group.projected_keys().any(|owned| owned == key))
}

/// Finds the runtime-context domains referenced by `ctx.KEY` expressions.
pub(crate) fn scan_needed_groups(content: &str) -> HashSet<ContextGroup> {
    let mut groups = HashSet::new();
    let literal_spans: Vec<_> = ExpressionFinder::new(content)
        .scan()
        .literals
        .into_iter()
        .map(|literal| literal.start..literal.end)
        .collect();
    let mut pos = 0;

    while let Some(offset) = content[pos..].find("ctx.") {
        let start = pos + offset + 4;
        let key_end = content[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|offset| start + offset)
            .unwrap_or(content.len());
        // Skip `ctx.KEY` matches that fall inside an interpolation literal
        // (`{{{ ... }}}`), whose content is inert on every scanning surface.
        let key_start = pos + offset;
        if !literal_spans.iter().any(|span| span.start <= key_start && key_end <= span.end)
            && let Some(group) = ContextGroup::for_key(&content[start..key_end])
        {
            groups.insert(group);
        }
        pos = key_end;
    }

    groups
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Persisted compose-cache manifests store group names.
    #[test]
    fn every_group_name_is_unique_and_round_trips() {
        let names: HashSet<_> = ContextGroup::all().into_iter().map(ContextGroup::name).collect();
        assert_eq!(names.len(), ContextGroup::all().len());
        for group in ContextGroup::all() {
            assert_eq!(ContextGroup::from_name(group.name()), Some(group));
        }
        assert_eq!(ContextGroup::from_name("Repo"), None);
    }

    #[test]
    fn every_owned_key_has_exactly_one_group() {
        let domains = [
            invocation::KEYS, datetime::KEYS, git::KEYS, repo::KEYS, changes::KEYS,
            languages::KEYS, docs::KEYS, host::OS_KEYS, host::HARDWARE_KEYS, host::GPU_KEYS,
            agent::KEYS,
        ];
        let mut seen = HashSet::new();
        for keys in domains {
            for key in keys {
                assert!(seen.insert(*key), "key `{key}` has multiple owners");
                assert!(group_for_key(key).is_some());
            }
        }
    }

    #[test]
    fn aliases_are_allowlisted_and_unknown_keys_have_no_group() {
        for alias in datetime::ALIASES {
            assert_eq!(group_for_key(alias), Some(ContextGroup::DateTime));
        }
        assert_eq!(group_for_key("user_defined"), None);
    }

    #[test]
    fn every_generated_descriptor_maps_to_one_group_or_explicit_alias() {
        use crate::markdown::compose::context::catalog::context_variable_descriptors;

        use crate::markdown::compose::context::catalog::PENDING_CAPTURE_KEYS;

        for descriptor in context_variable_descriptors() {
            if PENDING_CAPTURE_KEYS.contains(&descriptor.name) {
                assert_eq!(
                    group_for_key(descriptor.name),
                    None,
                    "`{}` has a capture group now; remove it from PENDING_CAPTURE_KEYS",
                    descriptor.name,
                );
                continue;
            }
            assert!(
                group_for_key(descriptor.name).is_some(),
                "descriptor `{}` has no capture group",
                descriptor.name,
            );
        }
    }

    /// The checked lookup trusts `projected_keys` as the capture contract, so it
    /// must name exactly the catalog: no projected key the catalog omits.
    #[test]
    fn every_projected_key_is_a_generated_descriptor() {
        use crate::markdown::compose::context::catalog::context_variable_descriptors;

        let descriptors: HashSet<&str> =
            context_variable_descriptors().iter().map(|descriptor| descriptor.name).collect();
        for group in ContextGroup::all() {
            for key in group.projected_keys() {
                assert!(descriptors.contains(key), "{group:?} projects uncataloged key `{key}`");
            }
        }
    }

    #[test]
    fn literal_masks_ctx_key() {
        let groups = scan_needed_groups("The CPU count is {{{ ctx.cpu_cores }}}.");
        assert!(!groups.contains(&ContextGroup::Hardware));
    }

    #[test]
    fn ctx_key_outside_literal_still_triggers_group() {
        let groups = scan_needed_groups("CPU count: {{ ctx.cpu_cores }} and literal {{{ ctx.os }}}.");
        assert!(groups.contains(&ContextGroup::Hardware));
        // `ctx.os` inside a literal must not trigger the OS group.
        assert!(!groups.contains(&ContextGroup::Os));
    }

    #[test]
    fn public_requirements_scan_frontmatter_body_aliases_and_literals() {
        let document: crate::markdown::Markdown = r#"---
branch: '{{ ctx.branch }}'
literal: '{{{ ctx.gpu }}}'
---
Today is {{ ctx.utc }} on {{ ctx.os }}.
"#
        .into();

        let requirements = ContextRequirements::for_document(&document);

        assert!(requirements.contains(ContextGroup::DateTime));
        assert!(requirements.contains(ContextGroup::Git));
        assert!(requirements.contains(ContextGroup::Os));
        assert!(!requirements.contains(ContextGroup::Gpu));
    }

    #[test]
    fn public_requirements_scan_every_context_group() {
        let requirements = ContextRequirements::for_content(
            "{{ ctx.cwd }} {{ ctx.utc }} {{ ctx.branch }} {{ ctx.repo_root }} \
             {{ ctx.dirty_files }} {{ ctx.programming_languages_in_repo }} \
             {{ ctx.docs_readme }} {{ ctx.os }} {{ ctx.cpu_cores }} \
             {{ ctx.gpu }} {{ ctx.agent }}",
        );

        for group in ContextGroup::all() {
            assert!(requirements.contains(group), "missing group: {group:?}");
        }
    }

    #[test]
    fn cwd_is_owned_only_by_the_invocation_group() {
        assert_eq!(group_for_key("cwd"), Some(ContextGroup::Invocation));
        let requirements = ContextRequirements::for_content("{{ ctx.cwd }}");
        assert!(requirements.contains(ContextGroup::Invocation));
        assert!(!requirements.contains(ContextGroup::Repo));
    }

    #[test]
    fn every_group_has_at_least_one_owned_key() {
        for group in ContextGroup::all() {
            assert!(
                ["cwd", "now", "branch", "repo_root", "dirty_files",
                    "programming_languages_in_repo", "docs_readme", "os", "cpu_cores",
                    "gpu", "agent"]
                    .into_iter()
                    .any(|key| group_for_key(key) == Some(group)),
                "capture group {group:?} has no key registry entry",
            );
        }
    }
}
