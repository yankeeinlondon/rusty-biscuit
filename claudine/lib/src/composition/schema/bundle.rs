//! Claudine's schema bundle, compiled into the binary.
//!
//! The five files under `claudine/schemas/` are the only definition of
//! Claudine's schemas. They are embedded verbatim with `include_str!` and
//! offered to Darkmatter's trigger discovery as the last-resort location after
//! the five document-context schema roots, so an installed `claudine` validates
//! its controls without a checkout. A disk file of the same name in any root
//! always wins; see `claudine/docs/topics/schemas.md`.

use darkmatter::markdown::schemas::triggers::TriggerSource;
use darkmatter::markdown::schemas::{EffectiveSchema, EmbeddedSchemaBundle, SchemaOriginKind};

/// The bundle's name in origins, traces, and errors (`embedded:claudine/…`).
pub const CLAUDINE_SCHEMA_BUNDLE_NAME: &str = "claudine";

/// The five schema files as `(file name, text)` pairs, byte-identical to the
/// repository files.
pub const CLAUDINE_SCHEMA_FILES: &[(&str, &str)] = &[
    ("action.yaml", include_str!("../../../../schemas/action.yaml")),
    ("claudine.yaml", include_str!("../../../../schemas/claudine.yaml")),
    ("inline-compose.yaml", include_str!("../../../../schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("../../../../schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("../../../../schemas/linking.yaml")),
];

/// The trigger files of the bundle. A property whose winning declaration is
/// one of these triggers, embedded or on disk, is a Claudine control.
pub const CLAUDINE_TRIGGER_FILES: &[&str] = &["claudine.yaml", "inline-compose.yaml"];

/// The embedded bundle handed to
/// [`DarkmatterSchemas::with_embedded_trigger_bundle`](darkmatter::markdown::schemas::DarkmatterSchemas::with_embedded_trigger_bundle).
#[must_use]
pub const fn claudine_schema_bundle() -> EmbeddedSchemaBundle {
    EmbeddedSchemaBundle::new(CLAUDINE_SCHEMA_BUNDLE_NAME, CLAUDINE_SCHEMA_FILES)
}

/// Whether `property`'s winning declaration in `effective` is a Claudine
/// trigger declaration.
///
/// The winner is Darkmatter's per-property origin: an authored `$schema` that
/// redeclares the property takes it over, so the author's declaration (and its
/// cleanup policy) applies instead. A disk trigger counts when its file name is
/// one of [`CLAUDINE_TRIGGER_FILES`], since a same-name disk file replaces the
/// embedded one wherever it is found.
#[must_use]
pub fn is_claudine_owned(effective: &EffectiveSchema, property: &str) -> bool {
    let Some(origin) = effective.origins.get(property) else {
        return false;
    };
    if origin.kind != SchemaOriginKind::Trigger {
        return false;
    }
    match origin.trigger_source() {
        Some(TriggerSource::Embedded { bundle, name }) => {
            bundle == CLAUDINE_SCHEMA_BUNDLE_NAME && CLAUDINE_TRIGGER_FILES.contains(&name.as_str())
        }
        Some(source @ TriggerSource::File(_)) => source
            .file_name()
            .is_some_and(|name| CLAUDINE_TRIGGER_FILES.contains(&name)),
        None => false,
    }
}
