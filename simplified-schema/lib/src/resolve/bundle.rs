//! A bundle of verified schema files as the only sources a resolution reads.
//!
//! A host that loads a registration bundle has already read and checked
//! every member's bytes. [`with`] runs a resolution with those bytes in
//! force on the calling thread, and every schema source the resolver would
//! otherwise select or read (a `$schema` file target, a trigger payload
//! reference, a `Name@file` import, an `$extends` or `...spread` operand, an
//! `example(file)`, also inside a template type's returned definition) is
//! decided against the bundle first:
//!
//! - an explicit relative reference (`./x.yaml`, `../y.yaml`) that resolves,
//!   from the declaring file's directory, to a listed member is served from
//!   the member's verified bytes, never from an editor buffer or the disk;
//! - one that resolves inside the bundle root to an unlisted path is refused
//!   without reading it, and one that resolves above the root is refused;
//! - a bare name, a rooted, home, magic, variable, absolute, or recursive
//!   reference is refused: a copied bundle cannot depend on search roots or
//!   on where it is installed;
//! - a URL stays [`SchemaError::RemoteUnsupported`] and is never fetched;
//! - the host's reserved built-in names (`Name@darkmatter:lifecycle`,
//!   [`ShippedSources`](crate::embedded::ShippedSources)) resolve as always:
//!   they are trusted shipped inputs outside the bundle.
//!
//! An `example(file)` naming a listed member is also refused when that
//! member is a standalone `kind: example` document, which a bundle may not
//! carry; any other member still faces the normal example envelope check.
//!
//! An installed bundle ([`BundleSources::on_disk`]) gives members their
//! local paths under its directory, so diagnostics and watches name real
//! files; an embedded one ([`BundleSources::embedded`]) uses the virtual
//! root `<name>`, which names no host file. Both apply the same path and
//! member rules, and nothing under the root is opened, probed, or
//! canonicalized on the disk while the scope is in force.
//!
//! The scope is independent of the template read gate
//! ([`crate::slots::TemplateReads`]): a returned definition's reads must pass
//! both. It also covers reads a host makes through
//! [`buffers::read`](super::buffers::read), so a host evaluating a slot's
//! own data files does that outside the scope.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileReference, FileReferenceKind, PathIdentity};

use crate::errors::SchemaError;

/// A bundle root and the verified bytes of its members.
///
/// Plain data (`Send + Sync`): a host captures it once and may carry it to
/// whichever thread prepares. Cloning shares the byte table.
#[derive(Clone)]
pub struct BundleSources {
    label: String,
    root: PathBuf,
    /// The root's identities a path is placed against: as given, and for a
    /// disk root its canonical spelling when that differs.
    identities: Vec<PathIdentity>,
    virtual_root: bool,
    members: Arc<BTreeMap<String, Arc<[u8]>>>,
}

/// Why a member table or root cannot form a bundle.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("bundle `{bundle}` is invalid: {message}")]
pub struct InvalidBundle {
    pub bundle: String,
    pub message: String,
}

impl BundleSources {
    /// An installed bundle rooted at the absolute directory `root`, holding
    /// `members` keyed by bundle-relative `/`-separated path
    /// (`schemas/shared.yaml`), the form a verified registration snapshot
    /// keys its bytes by.
    ///
    /// ## Errors
    ///
    /// [`InvalidBundle`] for a relative root or a key that is empty, holds
    /// `\`, or has an empty, `.`, or `..` segment.
    pub fn on_disk(root: &Path, members: BTreeMap<String, Arc<[u8]>>) -> Result<Self, InvalidBundle> {
        let label = biscuit_file::to_portable_string(root);
        if !root.is_absolute() {
            return Err(InvalidBundle { bundle: label, message: "the bundle root must be an absolute directory".into() });
        }
        let mut identities = vec![PathIdentity::new(root)];
        if let Ok(canonical) = biscuit_file::canonicalize_simplified(root) {
            let canonical = PathIdentity::new(&canonical);
            if canonical != identities[0] {
                identities.push(canonical);
            }
        }
        Self::checked(label, root.to_path_buf(), identities, false, members)
    }

    /// An embedded bundle named `name` (one plain path segment), with the
    /// same member keys as [`Self::on_disk`]. Its members' local paths are
    /// virtual, `<name>/schemas/shared.yaml`, and name no host file.
    ///
    /// ## Errors
    ///
    /// [`InvalidBundle`] for a name that is not one plain segment, or a key
    /// [`Self::on_disk`] refuses.
    pub fn embedded(name: &str, members: BTreeMap<String, Arc<[u8]>>) -> Result<Self, InvalidBundle> {
        if !crate::embedded::is_plain_segment(name) || name.contains(['<', '>']) {
            return Err(InvalidBundle { bundle: name.to_string(), message: "the bundle name must be one plain path segment".into() });
        }
        let root = PathBuf::from(format!("<{name}>"));
        let identities = vec![PathIdentity::new(&root)];
        Self::checked(name.to_string(), root, identities, true, members)
    }

    fn checked(
        label: String,
        root: PathBuf,
        identities: Vec<PathIdentity>,
        virtual_root: bool,
        members: BTreeMap<String, Arc<[u8]>>,
    ) -> Result<Self, InvalidBundle> {
        for key in members.keys() {
            let malformed = key.is_empty()
                || key.contains('\\')
                || key.split('/').any(|segment| matches!(segment, "" | "." | ".."));
            if malformed {
                return Err(InvalidBundle {
                    bundle: label,
                    message: format!("member `{key}` is not a bundle-relative path written with `/`"),
                });
            }
        }
        Ok(Self { label, root, identities, virtual_root, members: Arc::new(members) })
    }

    /// The root every member's local path is under: the installed directory,
    /// or the virtual `<name>`.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The local path of the listed member at bundle-relative `member`: the
    /// path a host hands the resolver for a member it resolves directly (a
    /// trigger file, a library). `None` when `member` is not listed.
    #[must_use]
    pub fn path_of(&self, member: &str) -> Option<PathBuf> {
        self.members.contains_key(member).then(|| self.local(member))
    }

    fn local(&self, member: &str) -> PathBuf {
        member.split('/').fold(self.root.clone(), |path, segment| path.join(segment))
    }

    /// The bundle-relative path of `path` when it lies under the root.
    fn place(&self, path: &Path) -> Option<String> {
        let target = PathIdentity::new(path);
        self.identities.iter().find_map(|root| {
            let route = target.relative_from(root).filter(|route| route.parent_hops() == 0)?;
            let names: Option<Vec<&str>> = route.forward().iter().map(|name| name.to_str()).collect();
            names.filter(|names| !names.is_empty()).map(|names| names.join("/"))
        })
    }
}

impl std::fmt::Debug for BundleSources {
    /// Names the root and the members, never their bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BundleSources")
            .field("root", &self.root)
            .field("members", &self.members.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// What one [`with`] scope decided and served.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BundleReads {
    /// Every local location a reference resolved to, listed or not, inside
    /// the root or above it, once each in decision order. For an embedded
    /// bundle these are virtual paths.
    pub attempted: Vec<PathBuf>,
    /// The attempted locations inside the root that no member is listed at.
    /// None of them was read; a host watches them to notice a republish
    /// that adds the member.
    pub missing: Vec<PathBuf>,
    /// The bundle-relative members whose bytes were served, once each in
    /// first-read order.
    pub served: Vec<String>,
}

/// Why a reference made under a bundle scope reaches no usable member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BundleRefusal {
    /// Not an explicit relative reference written with `/` (a bare name, a
    /// rooted, home, magic, variable, absolute, or recursive reference).
    NotRelative,
    /// The reference resolves above the bundle root.
    Escapes,
    /// The reference resolves inside the bundle root to no listed member.
    Unlisted,
    /// An `example(file)` names a listed member that is a standalone
    /// `kind: example` document, which a bundle may not carry.
    ExampleMember,
}

impl std::fmt::Display for BundleRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotRelative => "a bundle file is referenced only as `./…` or `../…` from the file that names it",
            Self::Escapes => "it resolves outside the bundle",
            Self::Unlisted => "it names no listed member of the bundle",
            Self::ExampleMember => "a bundle may not carry a standalone `kind: example` document",
        })
    }
}

/// A refused reference: the bundle, the refusal, and where the reference
/// resolved when it resolved at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleBoundary {
    /// The bundle's root as displayed: the installed directory or the
    /// embedded name.
    pub bundle: String,
    pub refusal: BundleRefusal,
    /// The local location the reference resolved to; `None` for
    /// [`BundleRefusal::NotRelative`].
    pub target: Option<PathBuf>,
}

impl std::fmt::Display for BundleBoundary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.target {
            Some(target) => write!(f, "`{}` in bundle `{}`: {}", target.display(), self.bundle, self.refusal),
            None => write!(f, "bundle `{}`: {}", self.bundle, self.refusal),
        }
    }
}

struct Active {
    sources: BundleSources,
    reads: BundleReads,
    /// Members the resolver rewrote an `example(...)` reference to, so the
    /// rewritten spelling selects them again at lowering.
    rehomed: HashSet<PathBuf>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}

/// Runs `resolve` with `sources` deciding every schema source it selects or
/// reads, restoring the scope in force before, even when `resolve` panics.
/// Returns `resolve`'s result with what the scope decided and served.
pub fn with<T>(sources: &BundleSources, resolve: impl FnOnce() -> T) -> (T, BundleReads) {
    struct Restore(Option<Option<Active>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(previous) = self.0.take() {
                ACTIVE.with_borrow_mut(|active| *active = previous);
            }
        }
    }
    let scope = Active { sources: sources.clone(), reads: BundleReads::default(), rehomed: HashSet::new() };
    let previous = ACTIVE.with_borrow_mut(|active| active.replace(scope));
    let restore = Restore(Some(previous));
    let result = resolve();
    let reads = ACTIVE.with_borrow_mut(|active| active.as_mut().map(|scope| std::mem::take(&mut scope.reads)));
    drop(restore);
    (result, reads.unwrap_or_default())
}

/// The bundle's decision on one schema reference.
pub(super) struct Decision {
    pub(super) outcome: Result<PathBuf, BundleBoundary>,
    /// The location to record as probed: `None` for an embedded bundle,
    /// whose virtual paths must never reach a disk probe.
    pub(super) probed: Option<PathBuf>,
}

/// Decides `reference` (trimmed, local, not a URL) written in a file in
/// `base_dir`; `None` with no scope in force.
pub(super) fn select(reference: &str, file_ref: &FileReference, base_dir: &Path) -> Option<Decision> {
    ACTIVE.with_borrow_mut(|active| {
        let active = active.as_mut()?;
        let sources = &active.sources;
        let refuse = |refusal, target: Option<PathBuf>| BundleBoundary { bundle: sources.label.clone(), refusal, target };
        if active.rehomed.contains(Path::new(reference)) {
            let path = PathBuf::from(reference);
            return Some(Decision { probed: (!sources.virtual_root).then(|| path.clone()), outcome: Ok(path) });
        }
        let class = file_ref.class();
        let explicit = class.kind == FileReferenceKind::ExplicitRelative
            && !class.recursive
            && !reference.contains('\\')
            && file_ref.env_variables().is_empty();
        if !explicit {
            return Some(Decision { outcome: Err(refuse(BundleRefusal::NotRelative, None)), probed: None });
        }
        let joined = base_dir.join(reference);
        let (outcome, location) = match sources.place(&joined) {
            Some(member) => {
                let local = sources.local(&member);
                if sources.members.contains_key(&member) {
                    (Ok(local.clone()), local)
                } else {
                    (Err(refuse(BundleRefusal::Unlisted, Some(local.clone()))), local)
                }
            }
            None => (Err(refuse(BundleRefusal::Escapes, Some(joined.clone()))), joined),
        };
        let virtual_root = sources.virtual_root;
        let reads = &mut active.reads;
        if !reads.attempted.contains(&location) {
            reads.attempted.push(location.clone());
        }
        if matches!(&outcome, Err(boundary) if boundary.refusal == BundleRefusal::Unlisted) && !reads.missing.contains(&location) {
            reads.missing.push(location.clone());
        }
        Some(Decision { outcome, probed: (!virtual_root).then_some(location) })
    })
}

/// `path`'s verified bytes; `None` with no scope in force. A path that is
/// no listed member fails as [`std::io::ErrorKind::PermissionDenied`].
pub(super) fn read(path: &Path) -> Option<std::io::Result<Vec<u8>>> {
    ACTIVE.with_borrow_mut(|active| {
        let active = active.as_mut()?;
        let member = active.sources.place(path).filter(|member| active.sources.members.contains_key(member));
        let Some(member) = member else {
            return Some(Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("`{}` is not a listed member of bundle `{}`", path.display(), active.sources.label),
            )));
        };
        let bytes = active.sources.members[&member].to_vec();
        if !active.reads.served.contains(&member) {
            active.reads.served.push(member);
        }
        Some(Ok(bytes))
    })
}

/// The identity of `path` under the scope's root, which is never looked up
/// on the disk; `None` with no scope or for a path outside the root.
pub(crate) fn identity(path: &Path) -> Option<PathBuf> {
    ACTIVE.with_borrow(|active| {
        let sources = &active.as_ref()?.sources;
        sources.place(path).map(|member| sources.local(&member))
    })
}

/// Records that the resolver rewrote an `example(...)` reference to the
/// listed member at `path`.
pub(crate) fn note_rehomed(path: &Path) {
    ACTIVE.with_borrow_mut(|active| {
        if let Some(active) = active.as_mut()
            && active.sources.place(path).is_some_and(|member| active.sources.members.contains_key(&member))
        {
            active.rehomed.insert(path.to_path_buf());
        }
    });
}

/// Refuses the example file `path` (already read as `bytes`) when a scope
/// is in force and it is a standalone `kind: example` document.
pub(crate) fn check_example(reference: &str, path: &Path, bytes: &[u8]) -> Result<(), SchemaError> {
    let label = ACTIVE.with_borrow(|active| active.as_ref().map(|active| active.sources.label.clone()));
    let Some(bundle) = label else {
        return Ok(());
    };
    let is_example = serde_yaml_ng::from_slice::<serde_yaml_ng::Value>(bytes)
        .ok()
        .is_some_and(|document| document.get("kind").and_then(serde_yaml_ng::Value::as_str) == Some("example"));
    if !is_example {
        return Ok(());
    }
    Err(SchemaError::BundleBoundary {
        reference: reference.to_string(),
        boundary: BundleBoundary { bundle, refusal: BundleRefusal::ExampleMember, target: Some(path.to_path_buf()) },
    })
}
