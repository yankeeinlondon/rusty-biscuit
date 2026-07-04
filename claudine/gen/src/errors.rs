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

    #[error("missing value for field `{field}`: {message}")]
    MissingValue { field: &'static str, message: String },

    #[error(
        "unmappable value for field `{field}`: {message} — this is the \"new variant \
         needed\" moment; extend the catalog type or pin an override"
    )]
    UnmappableValue { field: &'static str, message: String },

    #[error("could not locate a claudine package area (looked for `docs/providers.yaml` upward from `{from}`); pass --area")]
    AreaNotFound { from: PathBuf },
}
