//! Darkmatter's entry points to schema-trigger discovery.
//!
//! The trigger grammar, discovery, matching, and payload preparation are the
//! language's ([`simplified_schema::triggers`]); import their types from
//! there. These wrappers only hand the library Darkmatter's reserved shipped
//! namespace ([`embedded::SHIPPED_SOURCES`](super::embedded), so a payload's
//! `Name@darkmatter:<stem>` resolves), exactly as
//! [`DarkmatterSchemas`](super::DarkmatterSchemas) does.

use biscuit_file::FileResolutionContext;
use simplified_schema::SchemaError;
use simplified_schema::embedded::EmbeddedSchemaBundle;
use simplified_schema::shipped::PreparationMode;
use simplified_schema::triggers::{SchemaRootSource, TriggerBundleRecord, TriggerRegistry};

use super::embedded::SHIPPED_SOURCES;

/// [`simplified_schema::triggers::scan`] with Darkmatter's shipped sources.
///
/// ## Errors
///
/// Discovery's infrastructure failures (see the library's `scan`).
pub fn scan(context: &FileResolutionContext) -> Result<TriggerRegistry, SchemaError> {
    simplified_schema::triggers::scan(context, SHIPPED_SOURCES)
}

/// [`simplified_schema::triggers::scan_with_embedded`] with Darkmatter's
/// shipped sources.
///
/// ## Errors
///
/// Every [`scan`] error, and an unusable bundle's shape error.
pub fn scan_with_embedded(
    context: &FileResolutionContext,
    bundles: &[EmbeddedSchemaBundle],
) -> Result<TriggerRegistry, SchemaError> {
    simplified_schema::triggers::scan_with_embedded(context, bundles, SHIPPED_SOURCES)
}

/// [`simplified_schema::triggers::scan_with_source`] with Darkmatter's
/// shipped sources.
///
/// ## Errors
///
/// Every [`scan_with_embedded`] error, and any error `source` returns.
pub fn scan_with_source(
    context: &FileResolutionContext,
    bundles: &[EmbeddedSchemaBundle],
    source: &mut dyn SchemaRootSource,
) -> Result<TriggerRegistry, SchemaError> {
    simplified_schema::triggers::scan_with_source(context, bundles, source, SHIPPED_SOURCES)
}

/// [`simplified_schema::triggers::prepare_trigger_bundle`] with Darkmatter's
/// shipped sources: what a host's shipped artifact records for its bundle.
///
/// ## Errors
///
/// The bundle's shape error, or its first document's failure.
pub fn prepare_trigger_bundle(
    bundle: EmbeddedSchemaBundle,
    mode: PreparationMode<'_>,
) -> Result<TriggerBundleRecord, SchemaError> {
    simplified_schema::triggers::prepare_trigger_bundle(bundle, mode, SHIPPED_SOURCES)
}
