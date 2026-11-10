//! File format utilities for the dockhand ecosystem.
//!
//! This crate provides parsing, conversion, and validation for common file formats,
//! plus a file reference system for lazily resolving file paths:
//!
//! - **TOML**: Parse, convert to JSON/YAML, validate against schema
//! - **YAML**: Parse, convert to JSON/TOML, validate against schema
//! - **JSON5**: Parse, convert to JSON/YAML/TOML
//! - **JSON Lines**: Read one JSON value per line with each record's line and
//!   byte offsets (`json_lines`)
//! - **CSV/TSV**: Strictly read delimited text into a header and rows with
//!   each field's raw byte offset (`delimited`, needs `delimited`)
//! - **Source locations**: Byte offset of the value at a JSON Pointer in JSON,
//!   JSON Lines, TOML, or JSON5 text (`data_locations`), and a byte offset's
//!   1-based line and character column (`source_position`)
//! - **PDF**: Extract text, convert to Markdown, extract table of contents
//! - **FileReference**: Parse compact file descriptors (`@`, `&`, `^`, `vault:`, `%`,
//!   `{{ENV}}`) and resolve them lazily against runtime context
//! - **Frontmatter**: Locate a Markdown document's leading `---` block with byte
//!   spans, without parsing it (`frontmatter`, needs `yaml`)
//! - **Fetch**: Feature-gated HTTP fetch primitive with policy-enforcing host
//!   allowlists, conditional request headers, and structured responses
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `toml` | Yes | TOML parsing and conversion |
//! | `yaml` | Yes | YAML parsing and conversion |
//! | `json5` | Yes | JSON5 parsing and conversion |
//! | `extract` | Yes | PDF text extraction via pdf-extract |
//! | `lopdf` | Yes | PDF TOC extraction via lopdf |
//! | `pdfium` | No | High-fidelity PDF extraction via pdfium-render |
//! | `file-reference-syntax` | Yes (via `file-reference`) | Passive file and glob reference grammar, its types, and `FileResolutionContext` as data; no `gix` or `walkdir` |
//! | `file-reference` | Yes | File reference resolution, glob listing, portable references, and git root discovery |
//! | `url` | Yes (via `file-reference-syntax`) | URL classification for remote references |
//! | `fetch` | No | HTTP fetch primitive with host policy enforcement |
//! | `schema` | No | JSON Schema validation for TOML/YAML |
//! | `delimited` | No | Strict CSV/TSV reader (`csv`, `csv-core`) |
//! | `full` | No | All features enabled |
//!
//! ## Re-exports
//!
//! For convenience, this crate re-exports types from underlying crates,
//! allowing consumers to use `use biscuit_file::serde_yaml::Value` instead of
//! `use serde_yaml::Value`. This facilitates migrating from direct crate
//! dependencies to this unified library.
//!
//! ## Examples
//!
//! ### TOML Conversion
//!
//! ```rust
//! use biscuit_file::Toml;
//!
//! let toml = Toml::from_str(r#"
//! [package]
//! name = "example"
//! version = "1.0"
//! "#)?;
//! let json = toml.as_json()?;
//! let yaml = toml.as_yaml()?;
//! # Ok::<(), biscuit_file::TomlError>(())
//! ```
//!
//! ### YAML Conversion
//!
//! ```rust
//! use biscuit_file::Yaml;
//!
//! let yaml = Yaml::from_str("name: example\nversion: '1.0'")?;
//! let json = yaml.as_json()?;
//! let toml = yaml.as_toml()?;
//! # Ok::<(), biscuit_file::YamlError>(())
//! ```
//!
//! ### PDF Extraction
//!
//! ```rust,no_run
//! use biscuit_file::Pdf;
//!
//! let pdf = Pdf::new("document.pdf")?;
//! let text = pdf.as_text()?;
//! let markdown = pdf.as_markdown(Default::default())?;
//! let toc = pdf.toc()?;
//! # Ok::<(), biscuit_file::PdfError>(())
//! ```
//!
//! ### File Reference Resolution
//!
//! ```rust,no_run
//! use biscuit_file::{FileReference, PathPosition};
//!
//! // Magic reference -- searches captured package, repository, and home scopes
//! let file_ref = FileReference::new("@docs/spec.md")?;
//! if let Some(path) = file_ref.resolve()? {
//!     println!("Found: {}", path.display());
//! }
//!
//! // With custom search paths and vault roots
//! let file_ref = FileReference::new("vault:notes/today.md")?
//!     .add_vault("/path/to/vault")
//!     .add_magic_path("/extra/search/path", PathPosition::Start);
//! # Ok::<(), biscuit_file::FileReferenceError>(())
//! ```

mod detect;
mod error;
mod format;
mod list_format;
mod path_text;
mod span;

// `span_serde` is re-exported alongside the type it adapts: `Range` has no
// serde implementation, so a consumer serializing a `SourceSpan` cannot supply
// one itself. Without this the module was unreachable, which a dependent
// crate's `-D warnings` lint surfaced as dead code — invisible to this crate's
// own lint, whose `--all-targets` build includes the tests that exercise it.
pub use span::{SourceSpan, span_serde};

#[cfg(feature = "toml")]
pub mod toml_impl;

#[cfg(feature = "yaml")]
pub mod yaml;

// The frontmatter envelope reader needs YAML only to recognize a near-miss
// `----` fence that encloses a mapping.
#[cfg(feature = "yaml")]
pub mod frontmatter;

#[cfg(feature = "json5")]
pub mod json5;

pub mod json_lines;

#[cfg(feature = "delimited")]
pub mod delimited;

pub mod data_locations;

pub mod source_position;

#[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
pub mod pdf;

#[cfg(feature = "file-reference-syntax")]
pub mod file_reference;

// Re-export core error type
pub use error::BiscuitFileError;

// Re-export file type detection
pub use detect::{FileType, detect_file_type, detect_file_type_from_bytes, detect_from_extension};

// Re-export data format enum
pub use format::DataFormat;

// Re-export list-shape classification and conversion
pub use list_format::{ListFormat, classify_list};

// Re-export the path→text rendering boundary
pub use path_text::{canonicalize_simplified, to_portable_string, try_portable_string};

// Re-export format-specific types
#[cfg(feature = "toml")]
pub use self::toml_impl::{Toml, TomlError, TomlSource};

#[cfg(feature = "yaml")]
pub use self::yaml::{
    EditAudit, EditRejection, EditSetOutcome, RejectedEdit, Yaml, YamlAnalysis, YamlCertainty,
    YamlDiagnostic, YamlDiagnosticCode, YamlError, YamlLocation, YamlParseFailure,
    YamlParseOutcome, YamlPathSegment, YamlRepair, YamlSource, YamlValueLocation,
    YamlValueProperties, analyze_yaml, apply_edit_set, locate_yaml_key, locate_yaml_value,
};

#[cfg(feature = "yaml")]
#[doc(hidden)]
pub use self::yaml::{analyze_parse_count, reset_analyze_parse_count};

pub use self::json_lines::{JsonLineRecord, JsonLines, JsonLinesError, read_json_lines};

pub use self::data_locations::{locate_json, locate_jsonl};

#[cfg(feature = "toml")]
pub use self::data_locations::locate_toml;

#[cfg(feature = "json5")]
pub use self::data_locations::{json5_parse_error_offset, locate_json5};

pub use self::source_position::source_position;

#[cfg(feature = "delimited")]
pub use self::delimited::{
    DelimitedData, DelimitedError, DelimitedField, DelimitedRecord, read_delimited,
};

#[cfg(feature = "json5")]
pub use self::json5::{
    Json5, Json5DuplicateKey, Json5Error, Json5Number, Json5Scan, Json5Source,
    json5_is_well_formed, json5_scan,
};

#[cfg(any(feature = "extract", feature = "lopdf", feature = "pdfium"))]
pub use self::pdf::{Pdf, PdfConfig, PdfError, PdfMarkdown, PdfToc};

#[cfg(feature = "file-reference-syntax")]
pub use self::file_reference::{
    BaseDirOrigin, ContextAnchor, FileReference, FileReferenceClass, FileReferenceError,
    FileReferenceKind, FileResolutionContext, GlobReference, GlobReferenceError, IntentForms,
    LaunchMagicScope, MagicPathRegistration, MagicPathTier, PackageAreaFallback, PathIdentity,
    PathPosition, PortabilityPreference, RelativeRoute, RepositoryScope, RepositoryScopeCatalog,
    RepositoryScopeCatalogError, ResolutionCandidate, ResolutionFailure, RootProvenance,
    capture_env, home_dir, user_cache_dir, user_cache_dir_from,
};

#[cfg(feature = "file-reference")]
pub use self::file_reference::{
    CandidatePlanOrder, CompletionEntryForm, DetailedOutcome, DetailedResolution, GlobListing,
    MagicSearchRoot, PartialCompletion, PreparedGlobReference, ProbeDisposition, ProbedCandidate,
    Resolved, SkippedEntry, find_git_root,
};

#[cfg(feature = "file-reference")]
pub use self::file_reference::{
    Attempt, AttemptOutcome, ConfigurationProblem, EnvAnchorProblem, FilterProblem, Finding,
    InvalidTarget, NotApplicable, PORTABLE_ENV_VARIABLES, PortablePath, PortablePathError,
    PortableReference, ProbeError, ResolutionProblem, SpellingProblem,
};

#[cfg(feature = "fetch")]
pub use self::file_reference::{
    fetch::{
        Conditional, FetchPolicy, FetchResponse, HostPattern, PolicyClient, fetch, fetch_blocking,
        post, post_blocking,
    },
    error::FetchError,
};

#[cfg(feature = "url")]
pub use url;

// Re-export underlying crate types for convenience
// This allows consumers to use `use biscuit_file::serde_yaml::Value`
// instead of `use serde_yaml::Value`, facilitating migration.

#[cfg(feature = "toml")]
pub use toml as toml_crate;

#[cfg(feature = "yaml")]
pub use serde_yaml_ng;

/// YAML value type for direct manipulation.
#[cfg(feature = "yaml")]
pub use serde_yaml_ng::Value as YamlValue;

/// YAML mapping type for direct manipulation.
#[cfg(feature = "yaml")]
pub use serde_yaml_ng::Mapping as YamlMapping;

/// Error type for YAML parsing.
#[cfg(feature = "yaml")]
pub use serde_yaml_ng::Error as YamlParseError;

// Re-export TOML parsing error type
#[cfg(feature = "toml")]
mod toml_error_reexport {
    pub use toml::de::Error;
}

#[cfg(feature = "toml")]
pub use toml_error_reexport::Error as TomlDeError;
