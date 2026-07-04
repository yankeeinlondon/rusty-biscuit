//! Deterministic provider-catalog generator for Claudine.
//!
//! Joins the four inputs of the field-source matrix — roster identity
//! (`docs/providers.yaml`), research frontmatter
//! (`docs/research/<topic>/<slug>.md`, sidecar-validated), topic-less facts
//! (`docs/providers/facts/<slug>.yaml`), and human overrides
//! (`docs/providers/overrides/<slug>.yaml`) — into committed `data.rs`
//! fragments, via the mapping registry in [`registry`].
//!
//! Bootstrap rule: this crate never depends on the `claudine` lib or CLI.
//! Design authority: `claudine/features/2026-07-02-provider-metadata/`
//! (spec.md + design/catalog-generation.md).

pub mod errors;
pub mod facts;
pub mod generate;
pub mod inputs;
pub mod registry;
pub mod schema_compat;

pub use errors::GenError;
pub use facts::{MATRIX_FACTS_FIELDS, scrape_facts};
pub use generate::{
    CheckOutcome, CoercionSkip, Generation, Provenance, ResolvedField, check_area,
    committed_fragment_path, find_area, generate_for_area,
};
pub use registry::{Coercion, DeclaredSource, REGISTRY, RegistryEntry, SchemaExpectation,
    mapping_json};
