use std::collections::HashSet;

use crate::markdown::compose::expression::ExpressionFinder;

use super::{
    agent, changes, datetime, docs, document, git, host, invocation, languages, network, repo,
};

/// Independently captured runtime-context domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextGroup {
    /// Invocation-owned values that require no host or repository discovery.
    Invocation,
    /// Clock, calendar, and timezone values.
    DateTime,
    /// Branch, worktree, and merge-conflict values.
    Git,
    /// Recent-commit history. Separate from [`Git`](Self::Git) because it
    /// walks commits and their file changes, which ordinary Git facts such as
    /// `ctx.branch` must never pay for.
    GitHistory,
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
    /// Root-document identity projected from the request's retained root.
    Document,
    /// Tailnet membership and default-gateway values.
    Network,
}

impl ContextGroup {
    pub(crate) fn all() -> [Self; 14] {
        [
            Self::Invocation,
            Self::DateTime,
            Self::Git,
            Self::GitHistory,
            Self::Repo,
            Self::FileChanges,
            Self::Languages,
            Self::Documents,
            Self::Os,
            Self::Hardware,
            Self::Gpu,
            Self::Agent,
            Self::Document,
            Self::Network,
        ]
    }

    /// The group that projects the `ctx.<key>` variable `key` names.
    ///
    /// Public because an embedder's [`CurrentProvider`] is handed a bare key
    /// and has to decide which observation answers it.
    ///
    /// ## Returns
    ///
    /// `None` when no cataloged group projects `key`.
    ///
    /// [`CurrentProvider`]: crate::markdown::compose::CurrentProvider
    pub fn for_key(key: &str) -> Option<Self> {
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
            Self::GitHistory => (git::HISTORY_KEYS, &[]),
            Self::Repo => (repo::KEYS, &[]),
            Self::FileChanges => (changes::KEYS, &[]),
            Self::Languages => (languages::KEYS, &[]),
            Self::Documents => (docs::KEYS, &[]),
            Self::Os => (host::OS_KEYS, &[]),
            Self::Hardware => (host::HARDWARE_KEYS, &[]),
            Self::Gpu => (host::GPU_KEYS, &[]),
            Self::Agent => (agent::KEYS, &[]),
            Self::Document => (document::KEYS, &[]),
            Self::Network => (network::KEYS, &[]),
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

    /// A requirement set holding exactly `groups`.
    ///
    /// Unlike [`for_content`](Self::for_content) this adds nothing implicitly,
    /// which is what a single-group refresh needs: an embedder's
    /// [`CurrentProvider`] observes one group and must not pay for a second.
    ///
    /// [`CurrentProvider`]: crate::markdown::compose::CurrentProvider
    pub fn from_groups(groups: impl IntoIterator<Item = ContextGroup>) -> Self {
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

/// Functions that read a group's retained observations instead of a `ctx.*`
/// value, so a call demands the group's capture.
///
/// `recent_commits(count)` is absent: it performs its own Git I/O at call time
/// from the request's file-resolution repository root. The shell probes are
/// absent too: they launch the login shell named by the request environment.
const FUNCTION_GROUPS: &[(&str, ContextGroup)] = &[
    ("package", ContextGroup::Repo),
    ("package_area", ContextGroup::Repo),
    ("ipv4", ContextGroup::Network),
    ("ipv6", ContextGroup::Network),
    ("has_agentic_cli", ContextGroup::Agent),
];

/// Finds the runtime-context domains referenced by `ctx.KEY` expressions and
/// by calls to the [`FUNCTION_GROUPS`] functions.
pub(crate) fn scan_needed_groups(content: &str) -> HashSet<ContextGroup> {
    let mut groups = HashSet::new();
    let literal_spans = scan_literal_spans(content);
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
            && is_root_position(content, key_start)
            && let Some(group) = ContextGroup::for_key(&content[start..key_end])
        {
            groups.insert(group);
        }
        pos = key_end;
    }

    scan_function_groups(content, &literal_spans, &mut groups);
    groups
}

/// The inert `{{{ … }}}` literal spans of `content`, whose contents no
/// scanning surface treats as a reference.
pub(crate) fn scan_literal_spans(content: &str) -> Vec<std::ops::Range<usize>> {
    ExpressionFinder::new(content)
        .scan()
        .literals
        .into_iter()
        .map(|literal| literal.start..literal.end)
        .collect()
}

/// Whether the `ctx` at `start` is a root and not a trailing path segment.
///
/// `current.ctx.x` and `a.ctx.os` name no eager requirement: only a reference
/// whose *root* is `ctx` demands a capture (decision D4).
fn is_root_position(content: &str, start: usize) -> bool {
    let Some(previous) = content[..start].chars().next_back() else {
        return true;
    };
    !(previous.is_alphanumeric() || previous == '_' || previous == '.')
}

/// Adds the group of every `name(` call to a [`FUNCTION_GROUPS`] function.
///
/// A name preceded by an identifier character or `.` is part of a longer
/// identifier or a property path, not a call.
fn scan_function_groups(
    content: &str,
    literal_spans: &[std::ops::Range<usize>],
    groups: &mut HashSet<ContextGroup>,
) {
    let bytes = content.as_bytes();
    let is_ident = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let mut pos = 0;
    while pos < bytes.len() {
        if !is_ident(bytes[pos]) {
            pos += 1;
            continue;
        }
        let start = pos;
        while pos < bytes.len() && is_ident(bytes[pos]) {
            pos += 1;
        }
        if start > 0 && (is_ident(bytes[start - 1]) || bytes[start - 1] == b'.') {
            continue;
        }
        let Some((_, group)) = FUNCTION_GROUPS.iter().find(|(name, _)| *name == &content[start..pos])
        else {
            continue;
        };
        let called = content[pos..].trim_start().starts_with('(');
        if called && !literal_spans.iter().any(|span| span.start <= start && pos <= span.end) {
            groups.insert(*group);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// `all()` must list every variant exactly once; the exhaustive match makes
    /// a new variant a compile error here until it is added to `all()`.
    #[test]
    fn all_lists_every_variant_once() {
        let all = ContextGroup::all();
        let unique: HashSet<_> = all.into_iter().collect();
        assert_eq!(unique.len(), all.len());
        for group in all {
            match group {
                ContextGroup::Invocation
                | ContextGroup::DateTime
                | ContextGroup::Git
                | ContextGroup::GitHistory
                | ContextGroup::Repo
                | ContextGroup::FileChanges
                | ContextGroup::Languages
                | ContextGroup::Documents
                | ContextGroup::Os
                | ContextGroup::Hardware
                | ContextGroup::Gpu
                | ContextGroup::Agent
                | ContextGroup::Document
                | ContextGroup::Network => {}
            }
        }
        for group in [ContextGroup::GitHistory, ContextGroup::Document, ContextGroup::Network] {
            assert!(all.contains(&group), "{group:?} missing from all()");
        }
    }

    #[test]
    fn every_owned_key_has_exactly_one_group() {
        let domains = [
            invocation::KEYS, datetime::KEYS, git::KEYS, git::HISTORY_KEYS, repo::KEYS,
            changes::KEYS, languages::KEYS, docs::KEYS, host::OS_KEYS, host::HARDWARE_KEYS,
            host::GPU_KEYS, agent::KEYS, document::KEYS, network::KEYS,
        ];
        let mut seen = HashSet::new();
        for keys in domains {
            for key in keys {
                assert!(seen.insert(*key), "key `{key}` has multiple owners");
                assert!(group_for_key(key).is_some());
            }
        }
        let mut projected = HashSet::new();
        for group in ContextGroup::all() {
            for key in group.projected_keys() {
                assert!(projected.insert(key), "`{key}` is projected by more than one group");
            }
        }
    }

    #[test]
    fn new_groups_own_their_spec_keys() {
        for (keys, group) in [
            (&["self", "last_updated", "hash", "id", "sid"][..], ContextGroup::Document),
            (&["recent_commits"][..], ContextGroup::GitHistory),
            (&["tailnet", "gateway", "gateway_v6"][..], ContextGroup::Network),
            (&["hostname"][..], ContextGroup::Os),
        ] {
            for key in keys {
                assert_eq!(group_for_key(key), Some(group), "owner of `{key}`");
            }
        }
    }

    /// AC31: ordinary Git facts never demand the recent-history walk, and a
    /// recent-history reference demands only that group beyond date/time.
    #[test]
    fn recent_history_demand_is_separate_from_ordinary_git_facts() {
        for key in git::KEYS {
            let requirements = ContextRequirements::for_content(&format!("{{{{ ctx.{key} }}}}"));
            assert!(requirements.contains(ContextGroup::Git), "`{key}`");
            assert!(!requirements.contains(ContextGroup::GitHistory), "`{key}` demanded history");
        }

        let requirements = ContextRequirements::for_content("{{ ctx.recent_commits }}");
        assert_eq!(
            requirements,
            ContextRequirements::from_groups([ContextGroup::DateTime, ContextGroup::GitHistory]),
        );
    }

    /// Parameterized lookups read retained observations, so a call demands its
    /// group; a longer identifier, a property path, a bare name, or an inert
    /// literal does not.
    #[test]
    fn observation_function_calls_demand_their_group() {
        for (content, group) in [
            ("{{ package(\"a/b\") }}", Some(ContextGroup::Repo)),
            ("{{ package_area (x) }}", Some(ContextGroup::Repo)),
            ("{{ ipv4() }}", Some(ContextGroup::Network)),
            ("{{ len(ipv6(\"fd00::/8\")) }}", Some(ContextGroup::Network)),
            ("{{ my_package(\"a\") }}", None),
            ("{{ x.package(\"a\") }}", None),
            ("{{ packages }} the package (npm)", Some(ContextGroup::Repo)),
            ("{{{ ipv4() }}}", None),
            ("{{ has_agentic_cli(\"kimi\") }}", Some(ContextGroup::Agent)),
            ("{{ can_execute(\"ls\") }}", None),
            ("{{ recent_commits(3) }}", None),
            ("ipv4", None),
        ] {
            let expected = ContextRequirements::from_groups(
                [ContextGroup::DateTime].into_iter().chain(group),
            );
            assert_eq!(ContextRequirements::for_content(content), expected, "{content}");
        }
    }

    #[test]
    fn document_and_network_keys_demand_only_their_group() {
        for (content, group) in [
            ("{{ ctx.self }} {{ ctx.id }}", ContextGroup::Document),
            ("{{ ctx.gateway_v6 }}", ContextGroup::Network),
        ] {
            assert_eq!(
                ContextRequirements::for_content(content),
                ContextRequirements::from_groups([ContextGroup::DateTime, group]),
                "{content}",
            );
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

        for descriptor in context_variable_descriptors() {
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
             {{ ctx.gpu }} {{ ctx.agent }} {{ ctx.recent_commits }} {{ ctx.hash }} \
             {{ ctx.tailnet }}",
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
                    "gpu", "agent", "recent_commits", "self", "tailnet"]
                    .into_iter()
                    .any(|key| group_for_key(key) == Some(group)),
                "capture group {group:?} has no key registry entry",
            );
        }
    }
}
