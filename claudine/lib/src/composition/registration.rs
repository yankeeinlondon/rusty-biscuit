//! Publishing Claudine's `darkmatter-host` registration.
//!
//! Darkmatter (`md` and DMLS) learns Claudine's schemas, globals, and
//! lifecycle from a `darkmatter-host` manifest that a `register` entry of
//! `.darkmatter.yaml` names ([`MANIFEST_REFERENCE`]). This module writes that
//! manifest and the files it names from the copies compiled into this
//! binary ([`HostRevision::claudine`]); nothing is ever copied by hand.
//!
//! ## Layout
//!
//! ```text
//! ~/.claudine/
//! ├── darkmatter-host.yaml                  the stable manifest
//! └── darkmatter-host/
//!     ├── 0.1.0-<digest>/schemas/…           one immutable revision
//!     └── 0.1.0-<digest>-2/schemas/…         a republished revision
//! ```
//!
//! A revision directory is named `<version>-<digest>`: the host version and
//! the first [`DIGEST_LENGTH`] hex characters of the BLAKE3 content digest
//! ([`HostRevision::digest`]) over the ordered roles and every file's exact
//! text. Its `schemas/` folder holds every file of the revision: the trigger
//! bundle and its libraries, `globals.yaml`, `globals.lifecycle.json`, the
//! identifying trigger `claudine-document.yaml`, and `claudine-config.yaml`.
//! The configuration schema is there only so the installed revision can be
//! inspected; the manifest never names it. The manifest's member paths are
//! relative to the manifest and name one revision directory
//! (`./darkmatter-host/0.1.0-<digest>/schemas/claudine-document.yaml`).
//!
//! ## Contract
//!
//! - A directory with a final (non-staging) name is never written to after it
//!   appears. A revision is written into a uniquely named staging directory
//!   beside it and renamed into place whole, so a reader sees either no
//!   revision or a complete one.
//! - The stable manifest is replaced atomically and last, only after its
//!   revision directory verified byte for byte. It therefore never names a
//!   partial revision, so a reader can never pair a new trigger with an old
//!   catalog.
//! - An existing revision is reused only when every expected file holds
//!   exactly the expected bytes. One that differs (or misses a file) is left
//!   untouched and the revision is published under the next free suffixed
//!   name (`<version>-<digest>-2`, `-3`, …). The digest is shortened for that
//!   reason: identity is always settled by comparing bytes, never by name.
//! - Old revisions are retained for readers still holding an older manifest;
//!   nothing here removes one.
//!
//! ## The no-op rule
//!
//! [`publish`] runs on every launch, so when the stable manifest already
//! equals the expected text and the revision it names verifies, it writes
//! nothing and returns [`PublishOutcome::Current`]. That check costs one read
//! of the manifest and one of each revision file; disk content is compared,
//! never hashed.
//!
//! ## Failures
//!
//! Every failure is a [`PublishError`] and leaves the previous stable
//! manifest and every previous revision as they were. A failed manifest
//! replacement can leave the new revision directory behind; it is complete,
//! and the next publish reuses it.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::composition::schema::bundle::{CLAUDINE_SCHEMA_FILES, CLAUDINE_TRIGGER_FILES};
use crate::composition::schema::globals::HOST_CATALOG_TEXT;
use crate::config::atomic::atomic_write;

/// The `register` entry that names Claudine's stable manifest.
pub const MANIFEST_REFERENCE: &str = "~/.claudine/darkmatter-host.yaml";

/// The host identifier the manifest and the lifecycle artifact record.
pub const HOST_ID: &str = "claudine";

/// Hex characters of the content digest kept in a revision directory's name.
pub const DIGEST_LENGTH: usize = 16;

/// Claudine's identifying trigger, named by the manifest's `identify`.
pub const IDENTIFY_FILE: &str = "claudine-document.yaml";
/// The host global catalog, named by the manifest's `globals`.
pub const GLOBALS_FILE: &str = "globals.yaml";
/// The lifecycle artifact for [`GLOBALS_FILE`], named by `lifecycle`.
pub const LIFECYCLE_FILE: &str = "globals.lifecycle.json";
/// The configuration schema: published for inspection only, never named by
/// the manifest.
pub const CONFIG_SCHEMA_FILE: &str = "claudine-config.yaml";

const IDENTIFY_TEXT: &str = include_str!("../../../schemas/claudine-document.yaml");

const CLAUDINE_DIR: &str = ".claudine";
const MANIFEST_FILE: &str = "darkmatter-host.yaml";
const REVISIONS_DIR: &str = "darkmatter-host";
const SCHEMAS_DIR: &str = "schemas";
const STAGING_PREFIX: &str = ".staging-";
/// Suffixed names tried before giving up; each skipped name is a revision
/// whose bytes differ, which only damage or tampering produces.
const MAX_REVISION_NAMES: usize = 64;

/// The stable manifest under `home`: `~/.claudine/darkmatter-host.yaml`.
#[must_use]
pub fn manifest_path(home: &Path) -> PathBuf {
    home.join(CLAUDINE_DIR).join(MANIFEST_FILE)
}

/// The folder holding every published revision: `~/.claudine/darkmatter-host`.
#[must_use]
pub fn revisions_dir(home: &Path) -> PathBuf {
    home.join(CLAUDINE_DIR).join(REVISIONS_DIR)
}

/// One publishable host revision: a version, every file by portable
/// filename with its exact text, and Claudine's fixed roles over them.
///
/// The roles are fixed: `identify` is [`IDENTIFY_FILE`], the triggers are
/// `claudine.yaml`, `inline-compose.yaml`, and `sequence.yaml` in that order,
/// `globals` is [`GLOBALS_FILE`], `lifecycle` is [`LIFECYCLE_FILE`], and
/// `functions` is empty (Claudine ships no function catalog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRevision {
    version: String,
    files: BTreeMap<String, Cow<'static, str>>,
    digest: String,
}

impl HostRevision {
    /// The revision compiled into this binary, built once per process.
    #[must_use]
    pub fn claudine() -> &'static Self {
        static REVISION: OnceLock<HostRevision> = OnceLock::new();
        REVISION.get_or_init(|| {
            let mut files: BTreeMap<String, Cow<'static, str>> = CLAUDINE_SCHEMA_FILES
                .iter()
                .map(|(name, text)| ((*name).to_string(), Cow::Borrowed(*text)))
                .collect();
            files.insert(GLOBALS_FILE.to_string(), Cow::Borrowed(HOST_CATALOG_TEXT));
            files.insert(
                LIFECYCLE_FILE.to_string(),
                Cow::Owned(crate::composition::lifecycle::lifecycle_artifact()),
            );
            files.insert(IDENTIFY_FILE.to_string(), Cow::Borrowed(IDENTIFY_TEXT));
            files.insert(
                CONFIG_SCHEMA_FILE.to_string(),
                Cow::Borrowed(crate::config::schema::CONFIG_SCHEMA_SOURCE),
            );
            Self::from_parts(env!("CARGO_PKG_VERSION").to_string(), files)
        })
    }

    fn from_parts(version: String, files: BTreeMap<String, Cow<'static, str>>) -> Self {
        let digest = content_digest(&files);
        Self { version, files, digest }
    }

    /// This revision under another host version; the content is unchanged.
    #[must_use]
    pub fn with_version(self, version: impl Into<String>) -> Self {
        Self::from_parts(version.into(), self.files)
    }

    /// This revision with `name` holding `text`, replacing the file of that
    /// name or adding one. Roles are unchanged.
    #[must_use]
    pub fn with_file(mut self, name: impl Into<String>, text: impl Into<String>) -> Self {
        self.files.insert(name.into(), Cow::Owned(text.into()));
        Self::from_parts(self.version, self.files)
    }

    /// The host release the manifest records as `version`.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Every file, by filename, in filename order.
    pub fn files(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files.iter().map(|(name, text)| (name.as_str(), text.as_ref()))
    }

    /// The full BLAKE3 hex content digest over the ordered roles and the
    /// filename → text map, by `biscuit_hash::blake3_hash_json`. The version
    /// is not part of it.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// The first candidate revision directory name, `<version>-<digest>`.
    #[must_use]
    pub fn directory_name(&self) -> String {
        format!("{}-{}", self.version, &self.digest[..DIGEST_LENGTH])
    }

    /// The candidate directory name at `attempt` (1-based): attempt 1 is
    /// [`Self::directory_name`], later ones carry `-<attempt>`.
    fn candidate_name(&self, attempt: usize) -> String {
        match attempt {
            1 => self.directory_name(),
            n => format!("{}-{n}", self.directory_name()),
        }
    }

    /// The stable manifest's text when it names the revision directory
    /// `directory`: exactly the v1 fields.
    #[must_use]
    pub fn manifest_text(&self, directory: &str) -> String {
        let member = |file: &str| quoted(&format!("./{REVISIONS_DIR}/{directory}/{SCHEMAS_DIR}/{file}"));
        let triggers: String = CLAUDINE_TRIGGER_FILES
            .iter()
            .map(|file| format!("    - {}\n", member(file)))
            .collect();
        format!(
            "# Written by Claudine from its built-in schemas and rewritten when they change; do not edit.\n\
             kind: darkmatter-host\n\
             format_version: 1\n\
             host: {HOST_ID}\n\
             version: {version}\n\
             identify: {identify}\n\
             triggers:\n{triggers}\
             globals: {globals}\n\
             lifecycle: {lifecycle}\n\
             functions: []\n",
            version = quoted(&self.version),
            identify = member(IDENTIFY_FILE),
            globals = member(GLOBALS_FILE),
            lifecycle = member(LIFECYCLE_FILE),
        )
    }

    /// The first name (the version or a filename) that is not one portable
    /// path segment.
    fn unportable_name(&self) -> Option<&str> {
        std::iter::once(self.version.as_str())
            .chain(self.files.keys().map(String::as_str))
            .find(|name| !is_portable_segment(name))
    }
}

/// A JSON double-quoted scalar, which YAML reads as the same string.
fn quoted(text: &str) -> String {
    Value::String(text.to_string()).to_string()
}

fn content_digest(files: &BTreeMap<String, Cow<'static, str>>) -> String {
    // Arrays keep the role order: canonical JSON sorts object keys.
    let roles = json!([
        ["identify", IDENTIFY_FILE],
        ["triggers", CLAUDINE_TRIGGER_FILES],
        ["globals", GLOBALS_FILE],
        ["lifecycle", LIFECYCLE_FILE],
        ["functions", []],
    ]);
    let texts: serde_json::Map<String, Value> = files
        .iter()
        .map(|(name, text)| (name.clone(), Value::String(text.to_string())))
        .collect();
    biscuit_hash::blake3_hash_json(&json!({ "roles": roles, "files": texts }))
}

/// Whether `name` can be one file or directory name on every supported OS.
fn is_portable_segment(name: &str) -> bool {
    const RESERVED: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
        && !name.chars().any(|c| c.is_control() || RESERVED.contains(&c))
}

/// What [`publish`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    /// The stable manifest was replaced to name `revision_dir`, which was
    /// written or reused.
    Published {
        #[allow(missing_docs)]
        manifest: PathBuf,
        #[allow(missing_docs)]
        revision_dir: PathBuf,
    },
    /// The stable manifest already named a verified `revision_dir`; nothing
    /// was written.
    Current {
        #[allow(missing_docs)]
        manifest: PathBuf,
        #[allow(missing_docs)]
        revision_dir: PathBuf,
    },
}

#[allow(missing_docs)]
impl PublishOutcome {
    #[must_use]
    pub fn manifest(&self) -> &Path {
        match self {
            Self::Published { manifest, .. } | Self::Current { manifest, .. } => manifest,
        }
    }

    #[must_use]
    pub fn revision_dir(&self) -> &Path {
        match self {
            Self::Published { revision_dir, .. } | Self::Current { revision_dir, .. } => revision_dir,
        }
    }
}

/// Why [`publish`] stopped. In every case the previous stable manifest and
/// every previous revision are unchanged.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PublishError {
    /// The version or a filename cannot be one path segment on every OS.
    #[error("`{name}` cannot name a file or directory on every platform, so no revision was published")]
    UnportableName {
        #[allow(missing_docs)]
        name: String,
    },
    /// The folder holding the revisions could not be created or read.
    #[error("could not prepare the revision folder `{}`: {source}", .path.display())]
    RevisionsDir {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        source: io::Error,
    },
    /// The staging directory could not be created or written.
    #[error("could not stage a revision in `{}`: {source}", .path.display())]
    Staging {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        source: io::Error,
    },
    /// The staged revision could not be renamed into place.
    #[error("could not move the staged revision to `{}`: {source}", .path.display())]
    Rename {
        /// The final revision directory.
        path: PathBuf,
        #[allow(missing_docs)]
        source: io::Error,
    },
    /// Every candidate name was taken by a revision whose files differ.
    #[error(
        "the first {MAX_REVISION_NAMES} revision names for `{name}` in `{}` all hold different files; remove the damaged ones and run again",
        .dir.display()
    )]
    NoFreeRevisionName {
        /// The revisions folder.
        dir: PathBuf,
        /// The unsuffixed revision name.
        name: String,
    },
    /// The stable manifest could not be replaced.
    #[error("could not replace the manifest `{}`: {source}", .path.display())]
    Manifest {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        source: io::Error,
    },
}

/// Publishes `revision` under `home` and points the stable manifest at it,
/// or does nothing when the manifest already names a verified copy of it.
///
/// `home` is the user's home as the caller captured it; neither the process
/// environment nor the working directory is read. See the module docs for
/// the layout and the no-op rule.
///
/// ## Errors
///
/// Returns a [`PublishError`] when a name is not portable, when the
/// revisions folder, the staging directory, or the rename fails, when no
/// free revision name remains, or when the stable manifest cannot be
/// replaced. None of these changes the previous manifest or revisions.
pub fn publish(home: &Path, revision: &HostRevision) -> Result<PublishOutcome, PublishError> {
    if let Some(name) = revision.unportable_name() {
        return Err(PublishError::UnportableName { name: name.to_string() });
    }
    let manifest = manifest_path(home);
    let revisions = revisions_dir(home);

    if let Some(revision_dir) = current_revision(&manifest, &revisions, revision) {
        tracing::debug!(revision = %revision_dir.display(), "the host registration is current");
        return Ok(PublishOutcome::Current { manifest, revision_dir });
    }

    let directory = place_revision(&revisions, revision)?;
    atomic_write(&manifest, revision.manifest_text(&directory).as_bytes())
        .map_err(|source| PublishError::Manifest { path: manifest.clone(), source })?;
    let revision_dir = revisions.join(directory);
    tracing::debug!(revision = %revision_dir.display(), "published the host registration");
    Ok(PublishOutcome::Published { manifest, revision_dir })
}

/// The revision directory the stable manifest names, when the manifest is
/// exactly the expected text for one of the candidate names and that
/// directory verifies.
fn current_revision(manifest: &Path, revisions: &Path, revision: &HostRevision) -> Option<PathBuf> {
    let text = fs::read_to_string(manifest).ok()?;
    for attempt in 1..=MAX_REVISION_NAMES {
        let name = revision.candidate_name(attempt);
        let dir = revisions.join(&name);
        if text == revision.manifest_text(&name) {
            return (revision_state(&dir, revision) == RevisionState::Verified).then_some(dir);
        }
        // Names are taken in order, so the first absent one ends the search.
        if fs::symlink_metadata(&dir).is_err() {
            return None;
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevisionState {
    /// Nothing exists at the name.
    Absent,
    /// Every expected file holds exactly the expected bytes.
    Verified,
    /// Something exists at the name but is not this revision.
    Differs,
}

fn revision_state(dir: &Path, revision: &HostRevision) -> RevisionState {
    match fs::symlink_metadata(dir) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return RevisionState::Absent,
        Ok(metadata) if metadata.is_dir() => {}
        _ => return RevisionState::Differs,
    }
    let schemas = dir.join(SCHEMAS_DIR);
    let verified = revision
        .files()
        .all(|(name, text)| fs::read(schemas.join(name)).is_ok_and(|bytes| bytes == text.as_bytes()));
    if verified {
        RevisionState::Verified
    } else {
        RevisionState::Differs
    }
}

/// The name of a verified directory holding `revision`, reusing one or
/// staging and renaming a new one.
fn place_revision(revisions: &Path, revision: &HostRevision) -> Result<String, PublishError> {
    fs::create_dir_all(revisions)
        .map_err(|source| PublishError::RevisionsDir { path: revisions.to_path_buf(), source })?;
    for attempt in 1..=MAX_REVISION_NAMES {
        let name = revision.candidate_name(attempt);
        let dir = revisions.join(&name);
        match revision_state(&dir, revision) {
            RevisionState::Verified => return Ok(name),
            RevisionState::Differs => {
                tracing::warn!(revision = %dir.display(), "a published host revision differs from its name; leaving it untouched");
            }
            RevisionState::Absent => {
                if stage_and_rename(revisions, &dir, revision)? {
                    return Ok(name);
                }
                // Another publisher created `dir` first; reuse it if it holds
                // this revision.
                if revision_state(&dir, revision) == RevisionState::Verified {
                    return Ok(name);
                }
            }
        }
    }
    Err(PublishError::NoFreeRevisionName { dir: revisions.to_path_buf(), name: revision.directory_name() })
}

/// Writes `revision` into a fresh staging directory and renames it to
/// `final_dir`. `Ok(false)` when the rename lost to another publisher that
/// created `final_dir` meanwhile; the staging directory is removed either way.
fn stage_and_rename(revisions: &Path, final_dir: &Path, revision: &HostRevision) -> Result<bool, PublishError> {
    let staging = tempfile::Builder::new()
        .prefix(STAGING_PREFIX)
        .tempdir_in(revisions)
        .map_err(|source| PublishError::Staging { path: revisions.to_path_buf(), source })?;
    write_files(staging.path(), revision)
        .map_err(|source| PublishError::Staging { path: staging.path().to_path_buf(), source })?;
    match fs::rename(staging.path(), final_dir) {
        Ok(()) => {
            // The directory now lives at `final_dir`; there is nothing left
            // for the guard to remove.
            let _ = staging.keep();
            Ok(true)
        }
        Err(_) if fs::symlink_metadata(final_dir).is_ok() => Ok(false),
        Err(source) => Err(PublishError::Rename { path: final_dir.to_path_buf(), source }),
    }
}

fn write_files(staging: &Path, revision: &HostRevision) -> io::Result<()> {
    let schemas = staging.join(SCHEMAS_DIR);
    fs::create_dir(&schemas)?;
    for (name, text) in revision.files() {
        let mut file = fs::File::create(schemas.join(name))?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_names_are_portable_segments() {
        for name in ["0.1.0", "0.1.0-alpha.1+build", "claudine.yaml"] {
            assert!(is_portable_segment(name), "{name}");
        }
        for name in ["", ".", "..", "a/b", "a\\b", "a:b", "a*", "trailing.", "trailing ", "tab\t"] {
            assert!(!is_portable_segment(name), "{name:?}");
        }
    }

    #[test]
    fn registration_digest_depends_on_content_not_version() {
        let base = HostRevision::claudine().clone();
        let renamed = base.clone().with_version("9.9.9");
        let edited = base.clone().with_file(GLOBALS_FILE, "kind: schema\n");
        assert_eq!(base.digest(), renamed.digest());
        assert_ne!(base.directory_name(), renamed.directory_name());
        assert_ne!(base.digest(), edited.digest());
    }

    #[test]
    fn registration_manifest_never_names_the_config_schema() {
        let revision = HostRevision::claudine();
        let text = revision.manifest_text(&revision.directory_name());
        assert!(!text.contains(CONFIG_SCHEMA_FILE), "{text}");
        assert!(revision.files().any(|(name, _)| name == CONFIG_SCHEMA_FILE));
    }
}
