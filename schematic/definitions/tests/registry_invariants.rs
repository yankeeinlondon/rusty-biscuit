//! Cross-list invariants for the API registry dispatch.
//!
//! Several parallel lists must stay in lock-step: `apis_by_module` (the source
//! of truth for defined APIs), `registry_key_for` (name -> key translation),
//! and `get_registry` (key -> registry dispatch). This test drives every
//! defined API through that chain and asserts the registry is present and
//! complete, endpoint IDs are unique, and the base URL is parseable.

use schematic_definitions::apis_by_module;
use schematic_definitions::registry::{get_registry, registry_key_for};
use std::collections::HashSet;

/// A minimal "parseable URL" check that avoids pulling in a URL crate: require
/// a non-empty scheme and authority separated by `://`.
fn base_url_is_parseable(base_url: &str) -> bool {
    match base_url.split_once("://") {
        Some((scheme, rest)) => !scheme.is_empty() && !rest.is_empty(),
        None => false,
    }
}

#[test]
fn every_api_satisfies_registry_invariants() {
    for (module, apis) in apis_by_module() {
        for api in apis {
            let key = registry_key_for(&api.name);

            let registry = get_registry(&key).unwrap_or_else(|| {
                panic!(
                    "API {:?} (module {module:?}) resolved to key {key:?} but get_registry \
                     returned None",
                    api.name
                )
            });

            registry.validate_completeness(&api).unwrap_or_else(|missing| {
                panic!(
                    "API {:?} registry is missing response types: {missing:?}",
                    api.name
                )
            });

            let mut seen = HashSet::new();
            for endpoint in &api.endpoints {
                assert!(
                    seen.insert(endpoint.id.clone()),
                    "API {:?} has duplicate endpoint id {:?}",
                    api.name,
                    endpoint.id
                );
            }

            assert!(
                base_url_is_parseable(&api.base_url),
                "API {:?} has unparseable base_url {:?}",
                api.name,
                api.base_url
            );
        }
    }
}
