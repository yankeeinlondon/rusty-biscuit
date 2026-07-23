//! Typed generation errors.
//!
//! Every gate in the pipeline fails through a distinct variant so tests can
//! assert the *reason* a generation was refused, not just that it was.

use std::path::PathBuf;

use thiserror::Error;

/// Errors raised while loading inputs or generating catalog data.
#[derive(Debug, Error)]
pub enum GenError {
    #[error("io error at `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse YAML at `{path}`: {message}")]
    Yaml { path: PathBuf, message: String },

    #[error("failed to parse JSON: {message}")]
    Json { message: String },

    #[error("failed to load Markdown at `{path}`: {message}")]
    Markdown { path: PathBuf, message: String },

    #[error("no `{slug}` entry (matched on `slug:`) in roster `{path}`")]
    RosterEntryMissing { slug: String, path: PathBuf },

    #[error(
        "roster entry `{slug}` is flagged `skip_research: true` in `{path}` — the entry \
         keeps its identity in the roster but is excluded from research fan-out and \
         generation; remove the flag to re-activate it"
    )]
    RosterEntrySkipped { slug: String, path: PathBuf },

    #[error("roster entry `{slug}` is missing key `{key}`")]
    RosterKeyMissing { slug: String, key: &'static str },

    #[error("research sidecar `{path}` is missing or lacks a root `$schema:` key")]
    SidecarMissing { path: PathBuf },

    #[error("research sidecar `{path}` failed to parse as a SimplifiedSchema: {message}")]
    SidecarInvalid { path: PathBuf, message: String },

    #[error(
        "schema incompatibility for field `{field}`: sidecar declares `{found}` at \
         `{path}` but the mapping registry expects one of [{expected}]"
    )]
    SchemaIncompatible {
        field: &'static str,
        path: String,
        found: String,
        expected: String,
    },

    #[error(
        "schema incompatibility for field `{field}`: sidecar enum members [{offending}] at \
         `{path}` are not variants of `{rust_enum}` (variants: [{variants}])"
    )]
    EnumNotSubset {
        field: &'static str,
        path: String,
        rust_enum: &'static str,
        offending: String,
        variants: String,
    },

    #[error("sidecar property `{path}` not found while checking field `{field}`")]
    SchemaPathMissing { field: &'static str, path: String },

    #[error(
        "research document `{path}` failed schema validation:\n{problems}"
    )]
    ResearchInvalid { path: PathBuf, problems: String },

    #[error(
        "source collision: field `{field}` is declared `{declared}`-sourced but a value \
         for it arrived from `{offending}` — remove the {offending} entry or move the \
         declaration (overrides are the only sanctioned second source)"
    )]
    SourceCollision {
        field: String,
        declared: String,
        offending: String,
    },

    #[error(
        "facts file carries key `{key}` which maps to no registry field — remove it or \
         add a mapping-registry entry"
    )]
    UnknownFactsKey { key: String },

    #[error(
        "override for `{field}` names a field with no mapping-registry entry — overrides \
         may not introduce keys outside the field-source matrix"
    )]
    UnknownOverrideField { field: String },

    #[error("override for `{field}` is missing its required `reason:` string")]
    OverrideMissingReason { field: String },

    #[error("override for `{field}` is missing its required `value:` key")]
    OverrideMissingValue { field: String },

    #[error(
        "wired provider slug `{slug}` has no active roster entry (missing, or flagged \
         `skip_research: true`) — every slug in `emit::PROVIDER_VARIANTS` must have a live \
         `docs/providers.yaml` entry; restore the roster entry or unwire the variant"
    )]
    WiredSlugUnrostered { slug: String },

    #[error("--scaffold requires a provider slug (e.g. `claudine-gen generate <slug> --scaffold`)")]
    ScaffoldRequiresSlug,

    #[error("missing value for field `{field}`: {message}")]
    MissingValue { field: &'static str, message: String },

    #[error(
        "unmappable value for field `{field}`: {message} — this is the \"new variant \
         needed\" moment; extend the catalog type or pin an override"
    )]
    UnmappableValue { field: &'static str, message: String },

    #[error("could not locate a claudine package area (looked for `docs/providers.yaml` upward from `{from}`); pass --area")]
    AreaNotFound { from: PathBuf },

    #[error(
        "unchained-ai models-catalog artifact missing at `{path}` — the expected-offering \
         join cannot run without it; produce it with `cargo run -p unchained-ai-gen --bin \
         emit-catalog` (or `just artifact` in unchained-ai)"
    )]
    ArtifactMissing { path: PathBuf },

    #[error(
        "unchained-ai models-catalog artifact `{path}` declares schema_version {found}, \
         but this generator consumes schema_version {expected} — update the consumer and \
         the artifact together (breaking artifact changes bump the version by contract)"
    )]
    ArtifactSchemaVersion {
        path: PathBuf,
        found: u64,
        expected: u64,
    },

    #[error("unchained-ai models-catalog artifact `{path}`: {message}")]
    ArtifactInvalid { path: PathBuf, message: String },

    #[error(
        "family key `{key}` (derived from expected-offering catalog_id `{catalog_id}`) is \
         absent from the models-catalog artifact's families index — the identity-key \
         grammar and the family index disagree; regenerate the artifact (`just artifact` \
         in unchained-ai) or fix the producing curation tables"
    )]
    FamilyKeyMissing { key: String, catalog_id: String },

    #[error(
        "provider `{slug}` declares its error vocabulary `{declared}`-sourced but a value \
         also arrived from `{offending}` — delete the {offending} `error_vocabulary` entry \
         (exactly one layer may own it; this is the delete-on-graduate guard)"
    )]
    VocabularyCollision {
        slug: String,
        declared: &'static str,
        offending: &'static str,
    },

    #[error("provider `{slug}` has an invalid `error_vocabulary`: {message}")]
    VocabularyInvalid { slug: String, message: String },

    #[error("provider `{slug}` error-vocabulary generation failed: {message}")]
    VocabularyGenInvalid { slug: String, message: String },

    #[error("signals doc `{path}`: {message}")]
    SignalDocInvalid { path: PathBuf, message: String },

    #[error("signals doc `{path}`, record `{record}`: {message}")]
    SignalRecordInvalid {
        path: PathBuf,
        record: String,
        message: String,
    },
}
