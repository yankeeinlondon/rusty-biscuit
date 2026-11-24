//! Which schema a whole YAML document is validated against.
//!
//! ```mermaid
//! flowchart TD
//!     Y[YAML document] --> M{root is a mapping?}
//!     M -- no --> N[not validated]
//!     M -- yes --> K{top-level kind claimed by a<br/>shipped or local yaml-schema?}
//!     K -- yes --> V1["the claiming definition only<br/>(a root $schema is an ordinary property)"]
//!     K -- no --> U{registry establishes<br/>the kind unclaimed?}
//!     U -- no --> E[KindSelection error]
//!     U -- yes --> G{claimed by a<br/>registered definition?}
//!     G -- one --> V1
//!     G -- "several different" --> E
//!     G -- no --> S{root $schema?}
//!     S -- yes --> V2["that schema; $schema is stripped<br/>from the validation copy"]
//!     S -- no --> N
//! ```
//!
//! `kind` is read as authored, before any conversion: a missing, null, or
//! non-string `kind` claims nothing. A kind whose claim is ambiguous or
//! failed, or that the registry cannot rule out because a definition or root
//! failed, is a [`SchemaError::KindSelection`], never a fall-through to the
//! registered tier, the `$schema` route, or "no schema". A document with
//! neither a string `kind` nor a `$schema` is never affected by a registry
//! failure.

use std::sync::Arc;

use serde_yaml_ng::Value as YamlValue;
use crate::{KindSelectionCause, SchemaError};

use super::registry::{Claim, KindDefinition, KindRegistry, RegisteredKinds};
use crate::slots::{SLOT_NOT_ALLOWED_IN_KIND_CODE, slots_not_allowed_message};

/// The route a document selected.
pub enum Selection<'a> {
    /// Not validated.
    NoSchema,
    /// Its `kind` is claimed by `definition`; a registered host's definition
    /// has a [`DefinitionSource::Registered`](super::DefinitionSource::Registered)
    /// source naming the host.
    Kind { kind: String, definition: Arc<KindDefinition> },
    /// No claimed `kind`; its root `$schema` value is the contract.
    RootSchema(&'a YamlValue),
}

/// Selects `document`'s route in `registry`.
///
/// ## Errors
///
/// [`SchemaError::KindSelection`] for a string `kind` that is claimed by more
/// than one definition, whose definition failed, or that cannot be ruled out
/// as unclaimed (a definition failed before revealing its claim, or a schema
/// root could not be read).
pub fn select<'a>(registry: &KindRegistry, document: &'a YamlValue) -> Result<Selection<'a>, SchemaError> {
    select_with_registered(registry, None, document)
}

/// [`select`], then, for a string `kind` that `registry` establishes is
/// unclaimed, `registered`'s claim on it.
///
/// ## Errors
///
/// Every [`select`] error, which `registered` never shadows, and
/// [`SchemaError::KindSelection`] with [`KindSelectionCause::Duplicate`] for
/// an unclaimed kind that two or more different registered definitions claim.
pub fn select_with_registered<'a>(
    registry: &KindRegistry,
    registered: Option<&RegisteredKinds>,
    document: &'a YamlValue,
) -> Result<Selection<'a>, SchemaError> {
    let Some(map) = document.as_mapping() else {
        return Ok(Selection::NoSchema);
    };
    if let Some(kind) = map.get("kind").and_then(YamlValue::as_str) {
        let refused = |cause| SchemaError::KindSelection { kind: kind.to_string(), cause };
        if let Some(failure) = registry.root_failures().first() {
            return Err(refused(KindSelectionCause::RootUnreadable {
                root: failure.file.clone(),
                defect: failure.problem.to_string(),
            }));
        }
        match registry.claim(kind) {
            Some(Claim::Healthy(definition)) => {
                return Ok(Selection::Kind { kind: kind.to_string(), definition: Arc::clone(definition) });
            }
            Some(Claim::Duplicate(files)) => return Err(refused(KindSelectionCause::Duplicate { files: files.clone() })),
            Some(Claim::Reserved { file, defect }) => {
                let (defect, code) = described(defect);
                return Err(refused(KindSelectionCause::Reserved { file: file.clone(), defect, code }));
            }
            None => {
                if let Some(failure) = registry.incomplete().first() {
                    let (defect, code) = described(&failure.problem);
                    return Err(refused(KindSelectionCause::Unavailable { file: failure.file.clone(), defect, code }));
                }
                match registered.and_then(|registered| registered.claim(kind)) {
                    Some(Claim::Healthy(definition)) => {
                        return Ok(Selection::Kind { kind: kind.to_string(), definition: Arc::clone(definition) });
                    }
                    Some(Claim::Duplicate(files)) => {
                        return Err(refused(KindSelectionCause::Duplicate { files: files.clone() }));
                    }
                    // `RegisteredKinds` holds only prepared definitions.
                    Some(Claim::Reserved { .. }) | None => {}
                }
            }
        }
    }
    Ok(map.get("$schema").map_or(Selection::NoSchema, Selection::RootSchema))
}

/// A failed definition's defect as a selection reports it, and its code: a
/// definition holding schema slots is refused with
/// [`SLOT_NOT_ALLOWED_IN_KIND_CODE`]. A kind definition decides which
/// documents it judges before anything could resolve a slot, so it never
/// holds one; a `kind: schema` document named by `$schema` may.
fn described(defect: &SchemaError) -> (String, Option<&'static str>) {
    match defect {
        SchemaError::UnresolvedSlots(unresolved) => {
            (slots_not_allowed_message("a kind definition", unresolved), Some(SLOT_NOT_ALLOWED_IN_KIND_CODE))
        }
        other => (other.to_string(), None),
    }
}
