//! Simplified Schema's own artifact: the prepared envelope definitions.
//!
//! The library's shipped sources are its two envelope definitions
//! ([`ENVELOPE_DEFINITIONS`]); their prepared form is each definition's
//! lowered JSON Schema. The checked-in artifact
//! (`simplified-schema/lib/generated/shipped.json`) is loaded once per
//! process on first admission ([`envelopes`]); a refused artifact falls back
//! to the bootstrap that parses and lowers the definitions from their text.
//! Validators are compiled lazily either way.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    AcceptedArtifact, ArtifactDescriptor, FreshLower, LowerArtifact, Loaded, PreparationMode, RecordError, Refusal,
};
use crate::envelope::{ENVELOPE_DEFINITIONS, EnvelopeDefinitions, SCHEMA_DEFINITION_PATH, YAML_SCHEMA_DEFINITION_PATH};

/// The owner name in this library's artifact header.
pub const OWNER: &str = "simplified-schema";

/// This library's preparation revision. Increment it whenever lowering or
/// admission changes what the envelope definitions prepare to, then run
/// `just shipped-schemas`.
pub const PREPARATION_REVISION: u32 = 1;

/// The artifact's repository-relative path.
pub const ARTIFACT_PATH: &str = "simplified-schema/lib/generated/shipped.json";

/// The command that regenerates every shipped artifact, from the repository
/// root.
pub const REGENERATE_COMMAND: &str = "just shipped-schemas";

/// The checked-in artifact compiled into this build.
pub const ARTIFACT: &[u8] = include_bytes!("../../generated/shipped.json");

/// The artifact payload: one record per envelope definition, in
/// [`ENVELOPE_DEFINITIONS`] order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopePayload {
    pub definitions: Vec<EnvelopeDefinitionRecord>,
}

/// One envelope definition's lowered JSON Schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeDefinitionRecord {
    /// The definition's virtual path.
    pub identity: String,
    pub json_schema: Value,
}

/// What this library's artifact must describe.
pub fn descriptor() -> ArtifactDescriptor<'static> {
    ArtifactDescriptor {
        owner: OWNER,
        preparation_revision: PREPARATION_REVISION,
        sources: ENVELOPE_DEFINITIONS,
        dependencies: &[],
        configuration: &[],
    }
}

/// The library's loaded envelope definitions and how they were obtained.
pub struct Registry {
    /// The accepted artifact's header proof, or why it was refused.
    artifact: Result<AcceptedArtifact<()>, Refusal>,
    envelopes: Result<EnvelopeDefinitions, String>,
}

impl Registry {
    /// Loads `bytes` as this library's artifact, preparing the compiled
    /// definitions from text when it is refused.
    pub fn load(bytes: Option<&[u8]>) -> Self {
        match super::load_or_prepare(&descriptor(), bytes, accept_payload, EnvelopeDefinitions::prepare_uncached) {
            Loaded::Accepted(accepted) => {
                let mut envelopes = None;
                let artifact = accepted.map(|definitions| envelopes = Some(definitions));
                Self { artifact: Ok(artifact), envelopes: Ok(envelopes.expect("an accepted payload")) }
            }
            Loaded::Prepared { prepared, refusal } => {
                crate::note_work!(ShippedRuntimePreparation);
                Self { artifact: Err(refusal), envelopes: prepared.map_err(|error| error.to_string()) }
            }
        }
    }

    /// The envelope definitions, or the bootstrap's error.
    pub fn envelopes(&self) -> Result<&EnvelopeDefinitions, &str> {
        self.envelopes.as_ref().map_err(String::as_str)
    }

    /// Why the artifact was refused, or `None` when it was accepted.
    pub fn refusal(&self) -> Option<&Refusal> {
        self.artifact.as_ref().err()
    }

    /// This artifact's state as a dependency of a host's artifact.
    pub fn as_dependency(&self) -> LowerArtifact {
        match &self.artifact {
            Ok(accepted) => accepted.as_dependency(),
            Err(_) => LowerArtifact::Refused { owner: OWNER.to_string() },
        }
    }
}

/// The process registry, loaded from [`ARTIFACT`] on first use.
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| Registry::load(Some(ARTIFACT)))
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Whether [`registry`] has loaded in this process (generation must not).
#[doc(hidden)]
pub fn registry_is_loaded() -> bool {
    REGISTRY.get().is_some()
}

/// The envelope definitions `mode` prepares with.
pub(crate) fn envelopes(mode: PreparationMode<'_>) -> Result<&EnvelopeDefinitions, &str> {
    match mode {
        PreparationMode::Runtime => registry().envelopes(),
        PreparationMode::Uncached(fresh) => Ok(&fresh.envelopes),
    }
}

/// Checks the payload's records and builds the definitions from them.
fn accept_payload(_: &super::Acceptance<'_>, payload: EnvelopePayload) -> Result<EnvelopeDefinitions, RecordError> {
    let expected = [SCHEMA_DEFINITION_PATH, YAML_SCHEMA_DEFINITION_PATH];
    if payload.definitions.len() != expected.len() {
        return Err(RecordError::new(
            "/definitions",
            format!("{} definitions are recorded, {} are compiled in", payload.definitions.len(), expected.len()),
        ));
    }
    let mut lowered = Vec::with_capacity(expected.len());
    for (index, (record, identity)) in payload.definitions.into_iter().zip(expected).enumerate() {
        if record.identity != identity {
            return Err(RecordError::new(
                format!("/definitions/{index}/identity"),
                format!("`{}` is recorded where `{identity}` belongs", record.identity),
            ));
        }
        if !record.json_schema.is_object() {
            return Err(RecordError::new(
                format!("/definitions/{index}/json_schema"),
                "an envelope definition lowers to a JSON Schema object",
            ));
        }
        lowered.push(Arc::new(record.json_schema));
    }
    let yaml_schema = lowered.pop().expect("two definitions");
    let schema = lowered.pop().expect("two definitions");
    Ok(EnvelopeDefinitions::from_lowered(schema, yaml_schema))
}

/// The payload of freshly prepared `envelopes`, with every generation check
/// run.
///
/// ## Errors
///
/// A message when a definition does not satisfy its own contract.
pub(crate) fn prepare_payload(envelopes: &EnvelopeDefinitions) -> Result<EnvelopePayload, String> {
    envelopes.check_contracts().map_err(|error| error.to_string())?;
    let (schema, yaml_schema) = envelopes.json_schemas();
    Ok(EnvelopePayload {
        definitions: vec![
            EnvelopeDefinitionRecord { identity: SCHEMA_DEFINITION_PATH.into(), json_schema: schema.clone() },
            EnvelopeDefinitionRecord { identity: YAML_SCHEMA_DEFINITION_PATH.into(), json_schema: yaml_schema.clone() },
        ],
    })
}

/// Generates this library's artifact from current text, consulting neither
/// the checked-in artifact nor the process registry.
///
/// ## Errors
///
/// A message when preparation or a generation check fails.
pub fn generate() -> Result<Vec<u8>, String> {
    FreshLower::prepare().map(|fresh| fresh.artifact().to_vec())
}

/// Accepts `bytes` as this library's artifact, for generation commands that
/// report where a checked-in or freshly generated artifact is refused.
///
/// ## Errors
///
/// The [`Refusal`] of the first failed check.
pub fn check_artifact(bytes: &[u8]) -> Result<(), Refusal> {
    super::accept(&descriptor(), Some(bytes), accept_payload).map(|_| ())
}
