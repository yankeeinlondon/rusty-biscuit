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

pub mod agent_errors_check;
pub mod apply;
pub mod artifact;
pub mod catalog;
pub mod emit;
pub mod errors;
pub mod families;
pub mod generate;
pub mod inputs;
pub mod offerings;
pub mod registry;
pub mod report;
pub mod scaffold;
pub mod schema_compat;
pub mod signals;
pub mod vocabulary;

pub use agent_errors_check::{
    Branch, Check, Finding, FindingsReport, GateErrorScope, GateStatus, ResearchVocabulary,
    check_provider as check_agent_errors, default_findings_path, evaluate as evaluate_agent_errors,
};
pub use apply::{ApplyOutcome, DeclinedDrift, Decision, apply_generations, override_snippet};
pub use artifact::{ArtifactIndex, artifact_path};
pub use catalog::{build_catalog, catalog_path, check_catalog};
pub use errors::GenError;
pub use families::{build_families, check_families, compiled_family_keys, families_path};
pub use offerings::OfferingJoinReport;
pub use generate::{
    CheckOutcome, CoercionSkip, Generation, Provenance, ResolvedField, RosterCrossCheck,
    check_area, committed_data_path, cross_validate_roster, diff_lines, find_area, generate_all,
    generate_for_area, provider_slugs,
};
pub use registry::{
    Coercion, DeclaredSource, EXCLUDED_SERIALIZED_FIELDS, REGISTRY, RegistryEntry,
    SchemaExpectation, mapping_json,
};
pub use scaffold::{scaffold_behavior, scaffold_facts, scaffold_mod};
pub use vocabulary::{
    CodeBucket, DECLARED_SOURCE, ErrorVocabulary, FACTS_KEY, KeywordBucket, RESEARCH_TOPIC,
    VocabularySource, build_vocabulary, check_vocabulary, load_error_vocabulary, vocabulary_path,
};
pub use signals::{SIGNAL_SLUGS, build_signals, check_signals, signals_path};
