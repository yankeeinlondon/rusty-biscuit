//! A registered host's global catalog, prepared from its
//! `kind: namespace-schema` files.
//!
//! Each namespace file declares one global (`namespace: loop`) whose members
//! are its `$schema` object. [`prepare_catalog`] resolves every namespace on
//! its own, from the verified bytes of a bundle only, with the file as the
//! import base and the owner of its private `types`, then assembles the
//! declarations into one [`GlobalCatalog`]. The same pass returns the exact
//! digest map the lifecycle artifact is versioned against
//! ([`namespace_catalog_hash`]): each namespace file and every bundled file
//! its resolution read, by bundle-relative path. The verified disk loader
//! and a host's generator of its embedded bundle both call it, so the hash a
//! host writes is the hash a reader checks.
//!
//! A namespace name must be an expression root identifier that no reserved
//! root, Darkmatter global, or `null` already names, and unique within the
//! bundle; a duplicate is reported on every declaration of it.
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::resolve::bundle::{self, BundleSources};
use simplified_schema::yaml_documents::{load_yaml_document, locating_problem};
use simplified_schema::{PropertyAtom, PropertyDef, TypeExpr, parse_namespace_schema_document};

use crate::config::load::line_column;
use crate::markdown::compose::expression::binding::{NULL_ROOT, check_root};
use crate::markdown::schemas::catalog::{DARKMATTER_GLOBALS, GlobalCatalog, GlobalDeclaration};

/// One namespace file of a prepared catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceMember {
    /// The global it declares.
    pub namespace: String,
    /// Its bundle-relative path (`schemas/loop.yaml`).
    pub path: String,
    /// Where it was read: the installed file, or a virtual path for an
    /// embedded bundle. A global's declaration and members are located here.
    pub location: PathBuf,
    /// 1-based line and column of the `namespace` value, when known.
    pub at: Option<(usize, usize)>,
}

/// A host's global catalog with the inputs it was prepared from.
#[derive(Debug, Clone)]
pub struct PreparedCatalog {
    /// One global per namespace, in bundle-path order.
    pub catalog: Arc<GlobalCatalog>,
    /// The namespace files, in bundle-path order.
    pub namespaces: Vec<NamespaceMember>,
    /// The BLAKE3 digest (lowercase hex) of every file the preparation used,
    /// by bundle-relative path: each namespace file and each bundled file it
    /// imports, directly or transitively. Empty for a host with no
    /// namespaces.
    pub digests: BTreeMap<String, String>,
    /// [`namespace_catalog_hash`] of [`Self::digests`].
    pub hash: String,
}

/// Why a namespace file cannot join the catalog, located in that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceRefusal {
    /// The namespace file's bundle-relative path.
    pub path: String,
    /// Where it was read.
    pub location: PathBuf,
    /// 1-based line and column, when known.
    pub at: Option<(usize, usize)>,
    #[allow(missing_docs)]
    pub message: String,
}

/// The `catalog_hash` a lifecycle artifact records for a catalog prepared
/// from `digests` ([`PreparedCatalog::digests`]): the BLAKE3 digest of the
/// canonical JSON object mapping each bundle-relative path to its file's
/// digest. Independent of where the bundle is installed and of the order
/// files are listed in.
#[must_use]
pub fn namespace_catalog_hash(digests: &BTreeMap<String, String>) -> String {
    let map: serde_json::Map<String, Value> = digests.iter().map(|(path, digest)| (path.clone(), json!(digest))).collect();
    biscuit_hash::blake3_hash_json(&Value::Object(map))
}

/// The lowercase hexadecimal BLAKE3 digest of `bytes`, as a manifest's
/// `digests` records it.
#[must_use]
pub fn member_digest(bytes: &[u8]) -> String {
    use std::fmt::Write;
    biscuit_hash::blake3_hash_bytes(bytes).iter().fold(String::with_capacity(64), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// Prepares the catalog of the namespace files at the bundle-relative
/// `paths`, reading only `sources` (whose member bytes are `members`).
/// `context` is the request's captured context; nothing is read through it.
///
/// ## Errors
///
/// Every refusal found: a file that is not a valid namespace schema or
/// whose payload does not resolve inside the bundle, a reserved name, and
/// each declaration of a duplicated name.
pub fn prepare_catalog(
    sources: &BundleSources,
    members: &BTreeMap<String, Arc<[u8]>>,
    paths: &[&str],
    context: &FileResolutionContext,
) -> Result<PreparedCatalog, Vec<NamespaceRefusal>> {
    let mut sorted: Vec<&str> = paths.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut refusals = Vec::new();
    let mut prepared: Vec<(NamespaceMember, GlobalDeclaration)> = Vec::new();
    let mut digests = BTreeMap::new();
    for path in sorted {
        let location = sources.path_of(path).unwrap_or_else(|| sources.root().join(path));
        let refuse = |at: Option<(usize, usize)>, message: String| NamespaceRefusal {
            path: path.to_string(),
            location: location.clone(),
            at,
            message,
        };
        let Some(bytes) = members.get(path) else {
            refusals.push(refuse(None, format!("`{path}` is not a member of the bundle")));
            continue;
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            refusals.push(refuse(None, "the namespace file is not UTF-8 text".to_string()));
            continue;
        };
        digests.insert(path.to_string(), member_digest(bytes));
        match prepare_one(sources, members, path, &location, text, context, &mut digests) {
            Ok(entry) => prepared.push(entry),
            Err((at, message)) => refusals.push(refuse(at, message)),
        }
    }

    let mut by_name: BTreeMap<&str, Vec<&NamespaceMember>> = BTreeMap::new();
    for (member, _) in &prepared {
        by_name.entry(member.namespace.as_str()).or_default().push(member);
    }
    for (name, declared) in &by_name {
        if declared.len() < 2 {
            continue;
        }
        for member in declared {
            let others: Vec<String> =
                declared.iter().filter(|other| other.path != member.path).map(|other| format!("`{}`", other.path)).collect();
            refusals.push(NamespaceRefusal {
                path: member.path.clone(),
                location: member.location.clone(),
                at: member.at,
                message: format!("namespace `{name}` is also declared by {}", others.join(", ")),
            });
        }
    }
    if !refusals.is_empty() {
        refusals.sort_by(|left, right| left.path.cmp(&right.path).then_with(|| left.at.cmp(&right.at)));
        return Err(refusals);
    }
    let (namespaces, declarations): (Vec<_>, Vec<_>) = prepared.into_iter().unzip();
    let hash = namespace_catalog_hash(&digests);
    Ok(PreparedCatalog { catalog: Arc::new(GlobalCatalog::from_declarations(declarations)), namespaces, digests, hash })
}

/// A refusal of one namespace file: its position, when known, and why.
type Refused = (Option<(usize, usize)>, String);

/// The member and declaration of the namespace file at `path`, recording in
/// `digests` every bundled file its resolution read.
fn prepare_one(
    sources: &BundleSources,
    members: &BTreeMap<String, Arc<[u8]>>,
    path: &str,
    location: &std::path::Path,
    text: &str,
    context: &FileResolutionContext,
    digests: &mut BTreeMap<String, String>,
) -> Result<(NamespaceMember, GlobalDeclaration), Refused> {
    let position = |field: &str| {
        load_yaml_document(text, location).ok().and_then(|loaded| {
            loaded.byte_range(&locating_problem(&[field.to_string()])).map(|range| line_column(text, range.start))
        })
    };
    let document = match parse_namespace_schema_document(text, location) {
        Ok(Some(document)) => document,
        Ok(None) => return Err((None, "the file is not a namespace schema (`kind: namespace-schema`)".to_string())),
        Err(error) => {
            let at = match &error {
                simplified_schema::SchemaError::SchemaDocument { span: Some(span), .. } => Some(line_column(text, span.start)),
                _ => None,
            };
            return Err((at, format!("the namespace schema is invalid: {error}")));
        }
    };
    let at = position("namespace");
    let name = document.namespace.clone();
    if let Err(error) = check_root(&name) {
        return Err((at, format!("namespace `{name}` cannot be declared: {error}")));
    }
    if name == NULL_ROOT {
        return Err((at, "namespace `null` cannot be declared: `null` is the null literal".to_string()));
    }
    if DARKMATTER_GLOBALS.iter().any(|global| global.name == name.as_str()) {
        return Err((at, format!("namespace `{name}` cannot be declared: `{name}` is a Darkmatter global")));
    }
    let (resolved, reads) = bundle::with(sources, || {
        simplified_schema::resolve::resolve_namespace_payload(
            text,
            location,
            context,
            crate::markdown::schemas::embedded::SHIPPED_SOURCES,
        )
    });
    for served in &reads.served {
        if let Some(bytes) = members.get(served) {
            digests.insert(served.clone(), member_digest(bytes));
        }
    }
    let resolved = resolved.map_err(|error| (position("$schema"), format!("the namespace's `$schema` does not resolve: {error}")))?;
    let Some(simplified_schema::SchemaRoot::Single(shape)) = resolved.simplified else {
        return Err((position("$schema"), "the namespace's `$schema` must be one object".to_string()));
    };
    let definition = PropertyDef::Single(PropertyAtom {
        ty: TypeExpr::InlineObject(shape),
        is_array: false,
        constraints: Vec::new(),
        array_constraints: Vec::new(),
        description: document.description.clone(),
    });
    let member = NamespaceMember { namespace: name.clone(), path: path.to_string(), location: location.to_path_buf(), at };
    Ok((member, GlobalDeclaration::new(name, definition, resolved.definitions)))
}
