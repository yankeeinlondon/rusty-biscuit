//! Model- and offering-domain coercions: the dynamic-listing boolean
//! projected to a [`ModelCatalogSource`] member, the resume-support member
//! passthrough, and the local-runner offering-source records.

use serde_json::Value;
use strum::VariantNames;

use claudine_catalog_types::ResumeSupport;

use crate::errors::GenError;
use crate::offerings;
use crate::registry::RegistryEntry;

pub(super) fn dynamic_listing_to_model_catalog_source(
    entry: &RegistryEntry,
    raw: &Value,
) -> Result<Value, GenError> {
    match raw {
        Value::Bool(false) => Ok(Value::String("none".into())),
        Value::Bool(true) => Err(GenError::UnmappableValue {
            field: entry.field,
            message: "dynamic listing is available but the boolean cannot select a \
                      catalog mechanism — pin today's value with a field-keyed \
                      override until agent-models grows a typed source key"
                .into(),
        }),
        // A future enum-typed sidecar reports the member directly.
        Value::String(_) => Ok(raw.clone()),
        other => Err(GenError::UnmappableValue {
            field: entry.field,
            message: format!("expected a boolean or enum member, got `{other}`"),
        }),
    }
}

pub(super) fn resume_support_member(entry: &RegistryEntry, raw: &Value) -> Result<Value, GenError> {
    let member = raw.as_str().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: format!("expected a resume support enum member, got `{raw}`"),
    })?;
    if !ResumeSupport::VARIANTS.contains(&member) {
        return Err(GenError::UnmappableValue {
            field: entry.field,
            message: format!("`{member}` is not a ResumeSupport member"),
        });
    }
    Ok(raw.clone())
}

pub(super) fn local_runners_to_offering_sources(
    entry: &RegistryEntry,
    raw: &Value,
) -> Result<Value, GenError> {
    offerings::offering_sources_value(entry, raw)
}
