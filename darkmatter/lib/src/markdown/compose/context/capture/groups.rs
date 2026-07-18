use std::collections::HashSet;

use crate::markdown::compose::expression::ExpressionFinder;

use super::{agent, changes, datetime, docs, git, host, languages, repo};

/// Independently captured runtime-context domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ContextGroup {
    DateTime,
    Git,
    Repo,
    FileChanges,
    Languages,
    Documents,
    Os,
    Hardware,
    Gpu,
    Agent,
}

impl ContextGroup {
    pub(crate) fn all() -> [Self; 10] {
        [
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

    pub(crate) fn for_key(key: &str) -> Option<Self> {
        group_for_key(key)
    }
}

fn group_for_key(key: &str) -> Option<ContextGroup> {
    [
        (ContextGroup::DateTime, datetime::KEYS),
        (ContextGroup::Git, git::KEYS),
        (ContextGroup::Repo, repo::KEYS),
        (ContextGroup::FileChanges, changes::KEYS),
        (ContextGroup::Languages, languages::KEYS),
        (ContextGroup::Documents, docs::KEYS),
        (ContextGroup::Os, host::OS_KEYS),
        (ContextGroup::Hardware, host::HARDWARE_KEYS),
        (ContextGroup::Gpu, host::GPU_KEYS),
        (ContextGroup::Agent, agent::KEYS),
    ]
    .into_iter()
    .find_map(|(group, keys)| keys.contains(&key).then_some(group))
    .or_else(|| datetime::ALIASES.contains(&key).then_some(ContextGroup::DateTime))
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

    #[test]
    fn every_owned_key_has_exactly_one_group() {
        let domains = [
            datetime::KEYS, git::KEYS, repo::KEYS, changes::KEYS, languages::KEYS, docs::KEYS,
            host::OS_KEYS, host::HARDWARE_KEYS, host::GPU_KEYS, agent::KEYS,
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

        for descriptor in context_variable_descriptors() {
            assert!(
                group_for_key(descriptor.name).is_some(),
                "descriptor `{}` has no capture group",
                descriptor.name,
            );
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
}
