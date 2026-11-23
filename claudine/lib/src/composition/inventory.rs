//! Claudine's registration inventory: the files of `claudine/registration/`,
//! embedded at build time.
//!
//! `build.rs` walks the folder with [`walk`] and refuses the build when it is
//! not a valid inventory (an unexpected or nonportable file, a symlink or
//! junction, a case collision, non-UTF-8 text, or an ill-formed member
//! envelope). Otherwise it generates the include table behind [`members`],
//! one entry per file in sorted relative-path order. The in-process schemas
//! ([`super::schema`]), the shipped artifact's generator, and the publisher
//! ([`super::registration`]) all read this one table, so the installed
//! binary, the generated artifacts, and the published bytes agree.
//!
//! A member has two identities: its path inside the inventory
//! ([`InventoryMember::path`], `seq.yaml`) and its bundle path
//! ([`InventoryMember::bundle_path`], `schemas/seq.yaml`), where a published
//! `darkmatter-host` manifest lists it and where the embedded bundle
//! ([`bundle_sources`]) serves it. Neither names a file on the host.
//!
//! Folder guide: `claudine/docs/topics/schemas.md`.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use simplified_schema::resolve::bundle::BundleSources;

pub mod walk;

pub use walk::MemberKind;

/// The inventory's repository-relative folder.
pub const SOURCE_DIR: &str = "claudine/registration";

/// The embedded bundle's name: its virtual root is `<claudine>`.
pub const BUNDLE_NAME: &str = "claudine";

/// The folder a member's bundle path is under ([`InventoryMember::bundle_path`]).
pub const MEMBER_FOLDER: &str = "schemas";

/// The lifecycle artifact's bundle path, beside [`MEMBER_FOLDER`].
pub const LIFECYCLE_BUNDLE_PATH: &str = "globals.lifecycle.json";

/// The committed lifecycle artifact compiled into this build, for listing
/// and publication. Its repository path is
/// [`LIFECYCLE_ARTIFACT_PATH`](super::lifecycle::LIFECYCLE_ARTIFACT_PATH);
/// the root `just shipped-schemas` regenerates it.
pub const LIFECYCLE_ARTIFACT: &[u8] = include_bytes!("../../generated/globals.lifecycle.json");

/// One embedded file of `claudine/registration/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryMember {
    /// Its path relative to `claudine/registration/`, written with `/`.
    pub path: &'static str,
    /// Its exact text.
    pub text: &'static str,
    /// What it is, by its own `kind`.
    pub kind: MemberKind,
    /// Its display `name`, when it declares one.
    pub name: Option<&'static str>,
}

impl InventoryMember {
    /// Its path inside a published or embedded bundle:
    /// `schemas/<path>`.
    #[must_use]
    pub fn bundle_path(&self) -> String {
        format!("{MEMBER_FOLDER}/{}", self.path)
    }

    /// Its exact bytes.
    #[must_use]
    pub const fn bytes(&self) -> &'static [u8] {
        self.text.as_bytes()
    }
}

const MEMBERS: &[InventoryMember] = include!(concat!(env!("OUT_DIR"), "/registration_inventory.rs"));

/// Every member, sorted by [`InventoryMember::path`] (byte order).
#[must_use]
pub const fn members() -> &'static [InventoryMember] {
    MEMBERS
}

/// The member at inventory path `path` (`seq.yaml`).
#[must_use]
pub fn member(path: &str) -> Option<&'static InventoryMember> {
    MEMBERS.iter().find(|member| member.path == path)
}

/// The members of `kind`, in path order.
pub fn members_of(kind: MemberKind) -> impl Iterator<Item = &'static InventoryMember> {
    MEMBERS.iter().filter(move |member| member.kind == kind)
}

/// Every member's bytes by bundle path, as a bundle scope serves them.
#[must_use]
pub fn bundle_bytes() -> &'static BTreeMap<String, Arc<[u8]>> {
    static BYTES: OnceLock<BTreeMap<String, Arc<[u8]>>> = OnceLock::new();
    BYTES.get_or_init(|| MEMBERS.iter().map(|member| (member.bundle_path(), Arc::from(member.bytes()))).collect())
}

/// The inventory as an embedded bundle under the virtual root `<claudine>`:
/// a resolution inside
/// [`simplified_schema::resolve::bundle::with`] reads these bytes only.
///
/// ## Panics
///
/// Never for a built inventory: [`walk`] admitted every path the bundle
/// would refuse.
#[must_use]
pub fn bundle_sources() -> &'static BundleSources {
    static SOURCES: OnceLock<BundleSources> = OnceLock::new();
    SOURCES.get_or_init(|| {
        BundleSources::embedded(BUNDLE_NAME, bundle_bytes().clone()).expect("the walked inventory forms a bundle")
    })
}

/// The context every preparation inside [`bundle_sources`] resolves in: a
/// detached snapshot at the virtual root, with no home and no environment.
/// The bundle scope serves every read, so nothing is read through it and it
/// captures no process state.
#[must_use]
pub fn bundle_context() -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::from_snapshot(bundle_sources().root(), None, std::collections::HashMap::new())
}

#[cfg(test)]
mod tests;
