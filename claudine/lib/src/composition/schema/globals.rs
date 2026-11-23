//! Claudine's host catalog: the shapes of `seq`, `loop`, and `group`.
//!
//! Each global is declared by one `kind: namespace-schema` file of the
//! registration inventory (`claudine/registration/{group,loop,seq}.yaml`,
//! [`crate::composition::inventory`]). The catalog is prepared from those
//! members by Darkmatter's namespace preparation
//! ([`darkmatter::config::registration::namespaces::prepare_catalog`]), the
//! same pass a reader of the published bundle runs, so the embedded and the
//! installed catalogs agree, and so does the lifecycle artifact's
//! `catalog_hash`. The prepared form ships in Claudine's artifact
//! ([`super::shipped`]); when that is refused, the members are prepared once
//! per process instead. The binding catalog
//! ([`crate::composition::lifecycle::bindings`]) declares exactly these names
//! in every lifecycle view, and the event registry registers the late ones;
//! which events may read each is the binding catalog's policy, not the
//! namespace files'.

use std::sync::OnceLock;

use darkmatter::config::registration::namespaces::{PreparedCatalog, prepare_catalog};
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalDeclaration, GlobalRequirement};

use crate::composition::inventory::{self, MemberKind};

/// The catalog's identity in the shipped record and in errors: the
/// inventory it is prepared from.
pub const HOST_CATALOG_IDENTITY: &str = inventory::SOURCE_DIR;

/// The globals the catalog must declare, in the bundle-path order of their
/// namespace files.
pub const CLAUDINE_GLOBALS: &[GlobalRequirement] = &[
    GlobalRequirement::shape("group"),
    GlobalRequirement::shape("loop"),
    GlobalRequirement::shape("seq"),
];

/// Prepares the catalog of the inventory's namespace members, reading only
/// the embedded bytes ([`inventory::bundle_sources`]).
///
/// ## Errors
///
/// Every namespace refusal, one per line.
pub fn prepare_namespace_catalog() -> Result<PreparedCatalog, String> {
    let paths: Vec<String> = inventory::members_of(MemberKind::Namespace).map(|member| member.bundle_path()).collect();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    prepare_catalog(inventory::bundle_sources(), inventory::bundle_bytes(), &paths, &inventory::bundle_context()).map_err(|refusals| {
        refusals
            .iter()
            .map(|refusal| format!("`{}`: {}", refusal.path, refusal.message))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Claudine's host catalog: the shipped artifact's accepted entry, or, when
/// that was refused, the catalog prepared once per process from the
/// namespace members.
///
/// ## Panics
///
/// When the artifact's entry was refused and the namespace members do not
/// prepare: a build defect a test catches.
pub fn claudine_globals() -> &'static GlobalCatalog {
    super::shipped::registry().globals()
}

/// [`prepare_namespace_catalog`], once per process: the fallback when the
/// shipped artifact's entry was refused.
pub(crate) fn globals_from_sources() -> &'static GlobalCatalog {
    static CATALOG: OnceLock<GlobalCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        darkmatter::note_work!(ShippedRuntimePreparation);
        let prepared = prepare_namespace_catalog().unwrap_or_else(|error| panic!("Claudine's namespace catalog must prepare: {error}"));
        (*prepared.catalog).clone()
    })
}

/// The declaration of host global `name`, as a `'static` name for the
/// binding catalog and the registry.
///
/// ## Panics
///
/// When the catalog does not declare `name`: the binding catalog and the
/// namespace files disagree, which a test catches.
pub fn host_global(name: &str) -> &'static GlobalDeclaration {
    claudine_globals()
        .global(name)
        .unwrap_or_else(|| panic!("Claudine's host catalog declares `{name}`"))
}
