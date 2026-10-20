//! Cross-list invariants for the shipped API catalog.
//!
//! `apis_by_module` is the source of truth for defined APIs and
//! `registry_key_for` names each one. Every API must carry the schema
//! components its references name, every API of a module group must carry
//! the group's identical, complete components, endpoint IDs must be unique,
//! and the base URL must be parseable.

use schematic_definitions::apis_by_module;
use schematic_definitions::registry::registry_key_for;
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
    let mut keys = HashSet::new();
    for (module, apis) in apis_by_module() {
        for api in &apis {
            let key = registry_key_for(&api.name);
            assert!(keys.insert(key.clone()), "API {:?} (module {module:?}) reuses key {key:?}", api.name);

            api.validate_references().unwrap_or_else(|error| {
                panic!("API {:?} has a reference outside api.schemas: {error}", api.name)
            });
            for endpoint in &api.endpoints {
                for reference in endpoint.json_references() {
                    assert!(
                        api.rust_type(reference).is_some(),
                        "API {:?} endpoint {:?} reference {reference:?} has no registered Rust type",
                        api.name,
                        endpoint.id
                    );
                }
            }

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

        for api in &apis[1..] {
            assert!(
                api.schemas == apis[0].schemas,
                "module {module:?}: API {:?} carries different schemas than {:?}",
                api.name,
                apis[0].name
            );
        }
    }
}
