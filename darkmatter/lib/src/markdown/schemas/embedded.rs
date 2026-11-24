//! Schema documents compiled into the binary, addressed by repository-relative
//! virtual paths.
//!
//! Darkmatter's global catalog imports its partials, and its `fm` partial
//! imports Content Policy's editor schema, with ordinary relative references.
//! Resolving them from a table of embedded texts lets an installed binary
//! prepare the catalog and its baseline without a checkout. This is
//! a source for shipped schemas only: a user `$schema` reaches it solely
//! through the reserved namespace (`SHIPPED_NAMESPACE`,
//! `Name@darkmatter:<stem>`), and an embedded import never falls back to the
//! disk or the network.
//!
//! A host can also supply an
//! [`EmbeddedSchemaBundle`](simplified_schema::embedded::EmbeddedSchemaBundle)
//! of trigger schemas and their libraries. Its documents use the same lexical
//! resolution, keyed by `{bundle}/{file name}` and confined to that bundle;
//! the reserved namespace is the one way out of it.

pub(crate) use simplified_schema::embedded::{EmbeddedDocument, find};
#[cfg(test)]
use simplified_schema::embedded::resolve_path;
use simplified_schema::embedded::ShippedSources;

/// Virtual path of Darkmatter's configuration schema, served as
/// `Name@darkmatter:config` ([`SHIPPED_ALIASES`]).
pub(crate) const CONFIG_SCHEMA_PATH: &str = "darkmatter/schemas/darkmatter-config.yaml";

/// Virtual path of Darkmatter's host registration manifest schema, served as
/// `Name@darkmatter:host` ([`SHIPPED_ALIASES`]).
pub(crate) const HOST_MANIFEST_SCHEMA_PATH: &str = "darkmatter/schemas/darkmatter-host.yaml";

/// Virtual path of Content Policy's editor schema.
pub(crate) const CONTENT_POLICY_PATH: &str = "content-policy/schemas/content-policy.yaml";

/// The shipped documents: Darkmatter's global catalog, its partials
/// (including the neutral lifecycle action definitions, the `md` profile's
/// stacks, and the expression-function catalog, whose `recent-commits` helper
/// type the context partial imports), the configuration schema, the host
/// registration manifest schema, and the Content Policy editor schema the
/// `fm` partial imports.
/// The runtime baseline is the catalog's resolved `fm` definition.
pub(crate) const SHIPPED: &[EmbeddedDocument<'static>] = &[
    EmbeddedDocument {
        path: super::catalog::DARKMATTER_CATALOG_PATH,
        text: include_str!("../../../../schemas/darkmatter.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/fm.yaml",
        text: include_str!("../../../../schemas/partials/fm.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/style.yaml",
        text: include_str!("../../../../schemas/partials/style.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/context-variables.yaml",
        text: include_str!("../../../../schemas/partials/context-variables.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/error.yaml",
        text: include_str!("../../../../schemas/partials/error.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/timing.yaml",
        text: include_str!("../../../../schemas/partials/timing.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/lifecycle.yaml",
        text: include_str!("../../../../schemas/partials/lifecycle.yaml"),
    },
    EmbeddedDocument {
        path: "darkmatter/schemas/partials/md-lifecycle.yaml",
        text: include_str!("../../../../schemas/partials/md-lifecycle.yaml"),
    },
    EmbeddedDocument {
        path: CONFIG_SCHEMA_PATH,
        text: include_str!("../../../../schemas/darkmatter-config.yaml"),
    },
    EmbeddedDocument {
        path: HOST_MANIFEST_SCHEMA_PATH,
        text: include_str!("../../../../schemas/darkmatter-host.yaml"),
    },
    EmbeddedDocument {
        path: CONTENT_POLICY_PATH,
        text: content_policy::EDITOR_SCHEMA,
    },
];

/// Stems of the reserved namespace that name a shipped document outside the
/// partials folder: `permissions@darkmatter:config` names `permissions` in
/// the configuration schema, `darkmatter-host@darkmatter:host` the manifest
/// envelope. Every other stem names its partial.
pub(crate) const SHIPPED_ALIASES: &[(&str, &str)] =
    &[("config", CONFIG_SCHEMA_PATH), ("host", HOST_MANIFEST_SCHEMA_PATH)];

/// The reserved namespace of a named-type reference to one of Darkmatter's
/// shipped partials: `Name@darkmatter:<stem>` names the type `Name` in the
/// [`SHIPPED`] document `darkmatter/schemas/partials/<stem>.yaml` (or the
/// document a [`SHIPPED_ALIASES`] stem names), whoever
/// writes it and wherever it is written. It names no host file, so the disk
/// is never consulted, and references inside the partial resolve among the
/// shipped documents.
pub(crate) const SHIPPED_NAMESPACE: &str = "darkmatter:";

/// Darkmatter's reserved namespace over [`SHIPPED`], the source set every
/// Darkmatter resolution hands the library resolver. A partial's named types
/// come from the accepted shipped artifact ([`super::shipped::NAMED_TYPES`])
/// and are parsed from [`SHIPPED`] when that artifact was refused.
pub(crate) const SHIPPED_SOURCES: ShippedSources =
    ShippedSources::new(SHIPPED_NAMESPACE, "darkmatter/schemas/partials", SHIPPED)
        .with_aliases(SHIPPED_ALIASES)
        .with_prepared(&super::shipped::NAMED_TYPES);

/// The `<stem>` of a reference written in the reserved form
/// `darkmatter:<stem>`, or `None` for any other reference.
#[cfg(test)]
fn reserved_stem(reference: &str) -> Option<&str> {
    SHIPPED_SOURCES.reserved_stem(reference)
}

/// The shipped partial `darkmatter:<stem>` names, or `None` when [`SHIPPED`]
/// has no partial of that stem.
#[cfg(test)]
fn shipped_partial(stem: &str) -> Option<EmbeddedDocument<'static>> {
    SHIPPED_SOURCES.partial(stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fm_partial_import_climbs_three_directories_to_the_policy_schema() {
        assert_eq!(
            resolve_path("darkmatter/schemas/partials/fm.yaml", "../../../content-policy/schemas/content-policy.yaml"),
            Ok(CONTENT_POLICY_PATH.to_string())
        );
    }

    #[test]
    fn reserved_references_name_shipped_partials_only() {
        assert_eq!(reserved_stem(" darkmatter:lifecycle "), Some("lifecycle"));
        assert_eq!(reserved_stem("./lifecycle.yaml"), None);
        assert_eq!(reserved_stem("this"), None);
        for stem in ["lifecycle", "md-lifecycle", "fm", "timing"] {
            let document = shipped_partial(stem).unwrap_or_else(|| panic!("`{stem}` is shipped"));
            assert_eq!(document.path, format!("darkmatter/schemas/partials/{stem}.yaml"));
        }
        assert_eq!(shipped_partial("config").map(|document| document.path), Some(CONFIG_SCHEMA_PATH));
        assert_eq!(shipped_partial("host").map(|document| document.path), Some(HOST_MANIFEST_SCHEMA_PATH));
        for stem in ["", "bogus", "darkmatter-config", "darkmatter-host", "lifecycle.yaml", "../darkmatter", "partials/fm", ".", "..", "darkmatter"] {
            assert!(shipped_partial(stem).is_none(), "`{stem}` names no shipped partial");
        }
    }

    #[test]
    fn the_shipped_table_registers_exactly_the_catalog_its_partials_and_policy_schema() {
        let paths: Vec<_> = SHIPPED.iter().map(|document| document.path).collect();
        assert_eq!(
            paths,
            [
                super::super::catalog::DARKMATTER_CATALOG_PATH,
                "darkmatter/schemas/partials/fm.yaml",
                "darkmatter/schemas/partials/style.yaml",
                "darkmatter/schemas/partials/context-variables.yaml",
                "darkmatter/schemas/partials/error.yaml",
                "darkmatter/schemas/partials/timing.yaml",
                "darkmatter/schemas/partials/lifecycle.yaml",
                "darkmatter/schemas/partials/md-lifecycle.yaml",
                CONFIG_SCHEMA_PATH,
                HOST_MANIFEST_SCHEMA_PATH,
                CONTENT_POLICY_PATH,
            ]
        );
        assert_eq!(find(SHIPPED, CONTENT_POLICY_PATH).map(|d| d.text), Some(content_policy::EDITOR_SCHEMA));
        assert!(find(SHIPPED, "darkmatter/docs/schemas/darkmatter.yaml").is_none());
    }
}
