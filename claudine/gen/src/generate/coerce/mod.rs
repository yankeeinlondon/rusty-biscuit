//! Source-side half of every coercion: raw source value → catalog-shaped
//! value (the same shape overrides are authored in).
//!
//! [`coerce_to_catalog_shape`] is a thin exhaustive dispatcher over
//! [`Coercion`]. Scalar/string coercions and the record passthroughs (whose
//! loud shape validation lives in the [`emit`](crate::emit) expression half)
//! stay inline; the record-projecting coercions delegate to domain modules
//! that mirror the emit split: [`identity_paths`] (session-log / config
//! paths), [`execution_prompting`] (env-var / cli-flag selection), and
//! [`models_offerings`] / [`event_policy`] for their catalog areas.

use serde_json::Value;

use crate::errors::GenError;
use crate::generate::CoercionSkip;
use crate::registry::{Coercion, RegistryEntry};

mod event_policy;
mod execution_prompting;
mod identity_paths;
mod models_offerings;

/// Shared shape check: value must be an array of strings.
pub(crate) fn expect_string_array(entry: &RegistryEntry, raw: &Value) -> Result<Value, GenError> {
    let items = raw.as_array().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: format!("expected a string array, got `{raw}`"),
    })?;
    for item in items {
        if !item.is_string() {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected string elements, got `{item}`"),
            });
        }
    }
    Ok(raw.clone())
}

/// Source-side half of a coercion: raw source value → catalog-shaped value.
///
/// Record-shaped facts coercions extract as identity — the facts file
/// already carries the catalog shape, and the expression half in
/// [`emit`](crate::emit) performs the loud shape validation.
pub(crate) fn coerce_to_catalog_shape(
    entry: &RegistryEntry,
    raw: &Value,
    skips: &mut Vec<CoercionSkip>,
) -> Result<Value, GenError> {
    match entry.coercion {
        Coercion::StringLiteral
        | Coercion::ProviderVariantFromSlug
        | Coercion::SniffBindingVariant => match raw {
            Value::String(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string, got `{other}`"),
            }),
        },
        Coercion::OptionalStringLiteral | Coercion::StreamProtocolWire => match raw {
            Value::Null | Value::String(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a string or null, got `{other}`"),
            }),
        },
        Coercion::BoolLiteral => match raw {
            Value::Bool(_) => Ok(raw.clone()),
            other => Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a boolean, got `{other}`"),
            }),
        },
        Coercion::StringSlice => expect_string_array(entry, raw),
        Coercion::SkillSupportToBool => {
            let member = raw.as_str().ok_or_else(|| GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a support enum member, got `{raw}`"),
            })?;
            // Open question 3 ruling: first_class/partial -> true;
            // convention_only/none/unknown -> false.
            match member {
                "first_class" | "partial" => Ok(Value::Bool(true)),
                "convention_only" | "none" | "unknown" => Ok(Value::Bool(false)),
                other => Err(GenError::UnmappableValue {
                    field: entry.field,
                    message: format!("`{other}` is not a known skills `support` member"),
                }),
            }
        }
        Coercion::DynamicListingToModelCatalogSource => {
            models_offerings::dynamic_listing_to_model_catalog_source(entry, raw)
        }
        Coercion::EnvVarSitesToStringSlice => {
            execution_prompting::env_var_sites_to_string_slice(entry, raw, skips)
        }
        Coercion::SurfacesToSessionLogPaths => {
            identity_paths::surfaces_to_session_log_paths(entry, raw, skips)
        }
        Coercion::ConfigPathRecordsToConfigPaths => {
            identity_paths::config_path_records_to_config_paths(entry, raw, skips)
        }
        Coercion::ResumeSupportMember => models_offerings::resume_support_member(entry, raw),
        Coercion::CliFlagSitesToFlag => {
            execution_prompting::cli_flag_sites_to_flag(entry, raw, skips)
        }
        Coercion::FlagListToStringSlice => {
            execution_prompting::flag_list_to_string_slice(entry, raw, skips)
        }
        Coercion::BillingModelList => expect_string_array(entry, raw),
        Coercion::PlatformKindMember => event_policy::platform_kind_member(entry, raw),
        Coercion::LocalRunnersToOfferingSources => {
            models_offerings::local_runners_to_offering_sources(entry, raw)
        }
        Coercion::DisplayPolicyRecord => event_policy::display_policy_record(entry, raw),
        // The mixed-source acp record and the artifact-joined offering
        // records are assembled upstream in `extract_catalog_value` and
        // never reach this single-source coercion table.
        Coercion::AcpRecord => unreachable!("AcpRecord is handled in extract_catalog_value"),
        Coercion::DefaultModelsToExpectedOfferings => {
            unreachable!("DefaultModelsToExpectedOfferings is handled in extract_catalog_value")
        }
        // Facts-shaped records pass through; emit.rs validates loudly.
        Coercion::PathTemplateList
        | Coercion::EventMappingRecords
        | Coercion::ResourceSupportRecord
        | Coercion::OutputFormatRecords
        | Coercion::EntrypointRecords
        | Coercion::SystemPromptSpecRecord
        | Coercion::YoloRecordToYoloSupport
        | Coercion::ReasoningRecord
        | Coercion::KnownGapRecords
        | Coercion::UnmappedNativeEventRecords
        | Coercion::PromptArgRecord
        | Coercion::CapPolicyRecords
        | Coercion::AxesRecord => Ok(raw.clone()),
    }
}
