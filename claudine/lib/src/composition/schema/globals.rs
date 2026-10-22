//! Claudine's host catalog: the shapes of `seq`, `loop`, and `group`.
//!
//! `claudine/schemas/globals.yaml` is the one declaration of these globals'
//! shapes, in Darkmatter's global-catalog format. It is embedded verbatim and
//! loaded through Darkmatter's strict catalog reader, so an installed
//! `claudine` needs no schema file on disk. The binding catalog
//! ([`crate::composition::lifecycle::bindings`]) declares exactly these names
//! in every lifecycle view, and the event registry registers the late ones;
//! which events may read each is the binding catalog's policy, not this file's.
//!
//! The file is never a document schema or a trigger: Darkmatter's standalone
//! loader refuses the catalog envelope, and DMLS reads it only as a host
//! catalog for hover and completion.

use std::sync::OnceLock;

use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalDeclaration, GlobalRequirement};

/// The catalog's repository-relative path, its identity in errors.
pub const HOST_CATALOG_PATH: &str = "claudine/schemas/globals.yaml";

/// The catalog text, byte-identical to the repository file.
pub const HOST_CATALOG_TEXT: &str = include_str!("../../../../schemas/globals.yaml");

/// The globals the catalog must declare, in order.
pub const CLAUDINE_GLOBALS: &[GlobalRequirement] = &[
    GlobalRequirement::shape("seq"),
    GlobalRequirement::shape("loop"),
    GlobalRequirement::shape("group"),
];

/// Claudine's host catalog, loaded once per process.
///
/// ## Panics
///
/// When the embedded catalog does not load: a build defect a test catches.
pub fn claudine_globals() -> &'static GlobalCatalog {
    static CATALOG: OnceLock<GlobalCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        GlobalCatalog::from_documents(HOST_CATALOG_PATH, &[(HOST_CATALOG_PATH, HOST_CATALOG_TEXT)], CLAUDINE_GLOBALS)
            .unwrap_or_else(|error| panic!("Claudine's host catalog must load: {error}"))
    })
}

/// The declaration of host global `name`, as a `'static` name for the
/// binding catalog and the registry.
///
/// ## Panics
///
/// When the catalog does not declare `name`: the binding catalog and the
/// catalog file disagree, which a test catches.
pub fn host_global(name: &str) -> &'static GlobalDeclaration {
    claudine_globals()
        .global(name)
        .unwrap_or_else(|| panic!("Claudine's host catalog declares `{name}`"))
}
