use super::{ContextGroup, agent, changes, datetime, docs, host, languages, repo};

pub(super) fn group_for_key(key: &str) -> Option<ContextGroup> {
    [
        (ContextGroup::DateTime, datetime::KEYS),
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_owned_key_has_exactly_one_group() {
        let domains = [
            datetime::KEYS, repo::KEYS, changes::KEYS, languages::KEYS, docs::KEYS,
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
}
