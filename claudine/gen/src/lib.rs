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

pub mod apply;
pub mod catalog;
pub mod emit;
pub mod errors;
pub mod generate;
pub mod inputs;
pub mod registry;
pub mod schema_compat;
pub mod signals;

pub use apply::{ApplyOutcome, DeclinedDrift, Decision, apply_generations, override_snippet};
pub use catalog::{build_catalog, catalog_path, check_catalog};
pub use errors::GenError;
pub use generate::{
    CheckOutcome, CoercionSkip, Generation, PROVIDER_SLUGS, Provenance, ResolvedField,
    check_area, committed_data_path, diff_lines, find_area, generate_all, generate_for_area,
};
pub use registry::{
    Coercion, DeclaredSource, EXCLUDED_SERIALIZED_FIELDS, REGISTRY, RegistryEntry,
    SchemaExpectation, mapping_json,
};
pub use signals::{SIGNAL_SLUGS, build_signals, check_signals, signals_path};
