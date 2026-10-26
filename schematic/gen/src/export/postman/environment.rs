//! Postman environments: the variables a collection uses, with values.

use super::collection::Built;
use super::types::{PostmanEnvironment, PostmanEnvironmentValue};

/// The environment of a built collection: one value per collection
/// variable, in collection order. Base URLs and GraphQL addresses keep their
/// values; credentials and secret parameters are `secret` and empty, and
/// every other parameter is empty. Nothing is read from the host.
pub(crate) fn environment_of(built: &Built) -> PostmanEnvironment {
    let values = built
        .collection
        .variable
        .iter()
        .map(|variable| {
            let secret = built.collected.secrets.contains(&variable.key);
            PostmanEnvironmentValue {
                key: variable.key.clone(),
                value: if secret { String::new() } else { variable.value.clone().unwrap_or_default() },
                type_field: if secret { "secret" } else { "default" }.to_string(),
                enabled: true,
            }
        })
        .collect();
    PostmanEnvironment {
        name: built.collection.info.name.clone(),
        values,
        scope: "environment".to_string(),
    }
}
