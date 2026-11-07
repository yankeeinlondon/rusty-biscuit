//! Unsaved editor texts that schema resolution reads in place of files.
//!
//! An editor holds schema sources the author has not saved: a definition, a
//! file it imports, a `$schema` target, or a file that exists only in the
//! editor. [`with`] runs a resolution with a [`SourceBuffers`] snapshot in
//! force on the calling thread, so every schema source the resolver reads (a
//! `$schema` target, a `Name@file` import, an `example(file)`, and the spans
//! an error re-projects) comes from the buffer when one is open at that path,
//! and a reference selects an open buffer at a probed location that holds no
//! file yet. Without a snapshot in force, resolution reads only the disk.
//!
//! The snapshot is plain data (`Send + Sync`): the host captures it once per
//! preparation and may carry it to whichever thread prepares.
//!
//! [`with_reads_retained`] uses the same snapshot for a second purpose: for
//! the length of one host operation, every file read from the disk is kept as
//! if it were open, so separate resolutions in that operation share one read.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Open texts keyed by [`SourceBuffers::key`]. Cloning shares the map.
#[derive(Clone, Debug, Default)]
pub struct SourceBuffers {
    texts: Arc<HashMap<PathBuf, Arc<str>>>,
}

impl SourceBuffers {
    /// The form every path is compared in: its nearest existing ancestor
    /// canonicalized and the rest kept as written, so a file that exists only
    /// in the editor still matches the spelling resolution probes it under
    /// (`/var/…` and `/private/var/…` alike).
    #[must_use]
    pub fn key(path: &Path) -> PathBuf {
        let mut rest = Vec::new();
        let mut current = path;
        while let (Some(parent), Some(name)) = (current.parent(), current.file_name()) {
            rest.push(name);
            if let Ok(canonical) = biscuit_file::canonicalize_simplified(parent) {
                return rest.iter().rev().fold(canonical, |path, name| path.join(name));
            }
            current = parent;
        }
        path.to_path_buf()
    }

    /// Records `text` as the open buffer at `path`, returning the text it
    /// replaced.
    pub fn insert(&mut self, path: &Path, text: Arc<str>) -> Option<Arc<str>> {
        Arc::make_mut(&mut self.texts).insert(Self::key(path), text)
    }

    /// Forgets the buffer at `path`, returning its text.
    pub fn remove(&mut self, path: &Path) -> Option<Arc<str>> {
        let key = Self::key(path);
        if !self.texts.contains_key(&key) {
            return None;
        }
        Arc::make_mut(&mut self.texts).remove(&key)
    }

    /// The open text at `path`.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<&Arc<str>> {
        if self.texts.is_empty() {
            return None;
        }
        self.texts.get(&Self::key(path))
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// Every open path, in [`Self::key`] form.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.texts.keys().map(PathBuf::as_path)
    }
}

thread_local! {
    static ACTIVE: RefCell<SourceBuffers> = RefCell::new(SourceBuffers::default());
    /// How many [`with_reads_retained`] scopes are running on this thread.
    static RETAINING: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Runs `resolve` with the snapshot in force, also keeping each schema
/// source it reads from the disk as an open buffer for the rest of `resolve`:
/// one host operation that runs several separate resolutions (a catalog's
/// globals, say) reads each file once. The kept texts are dropped when
/// `resolve` returns; nothing persists past the call. Inside a running scope
/// this joins it, so what an inner operation reads stays kept for the outer
/// one.
pub fn with_reads_retained<T>(resolve: impl FnOnce() -> T) -> T {
    if RETAINING.get() > 0 {
        return resolve();
    }
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            RETAINING.set(RETAINING.get() - 1);
        }
    }
    let current = ACTIVE.with_borrow(Clone::clone);
    with(&current, || {
        RETAINING.set(RETAINING.get() + 1);
        let _leave = Leave;
        resolve()
    })
}

/// `path`'s text, read as the resolver reads a schema source: the open
/// buffer in force, else the file's, kept for the rest of a
/// [`with_reads_retained`] scope. For a host reading a schema file it then
/// resolves (a baseline, a catalog) before handing the text on.
///
/// ## Errors
///
/// The read's I/O error; [`std::io::ErrorKind::InvalidData`] when the file
/// is not UTF-8.
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    String::from_utf8(read(path)?).map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// Runs `resolve` with `buffers` in force on this thread, restoring the
/// snapshot that was in force before, even when `resolve` panics.
pub fn with<T>(buffers: &SourceBuffers, resolve: impl FnOnce() -> T) -> T {
    struct Restore(Option<SourceBuffers>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(previous) = self.0.take() {
                ACTIVE.with_borrow_mut(|active| *active = previous);
            }
        }
    }
    let previous = ACTIVE.with_borrow_mut(|active| std::mem::replace(active, buffers.clone()));
    let _restore = Restore(Some(previous));
    resolve()
}

/// The open text at `path` in the snapshot in force on this thread.
#[must_use]
pub fn text(path: &Path) -> Option<Arc<str>> {
    ACTIVE.with_borrow(|active| active.get(path).cloned())
}

/// The first of `probed` with an open buffer in the snapshot in force.
#[cfg_attr(not(feature = "file"), allow(dead_code))]
pub(super) fn first_open(probed: &[PathBuf]) -> Option<PathBuf> {
    ACTIVE.with_borrow(|active| {
        if active.is_empty() {
            return None;
        }
        probed.iter().find(|path| active.get(path).is_some()).cloned()
    })
}

/// `path`'s bytes: the open buffer's text in force, else the file's.
pub(super) fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    if let Some(text) = text(path) {
        return Ok(text.as_bytes().to_vec());
    }
    let bytes = std::fs::read(path)?;
    super::observed::opened(path);
    // Non-UTF-8 bytes fail the read's caller, so there is nothing to keep.
    if RETAINING.get() > 0
        && let Ok(text) = std::str::from_utf8(&bytes)
    {
        ACTIVE.with_borrow_mut(|active| active.insert(path, Arc::from(text)));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_applies_only_inside_with_and_is_restored_after() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("types.yaml");
        std::fs::write(&file, "disk").unwrap();
        let mut buffers = SourceBuffers::default();
        buffers.insert(&file, Arc::from("buffer"));
        buffers.insert(&dir.path().join("new.yaml"), Arc::from("unsaved"));

        assert_eq!(read(&file).unwrap(), b"disk");
        let (inside, found) = with(&buffers, || {
            (read(&file).unwrap(), first_open(&[dir.path().join("absent.yaml"), dir.path().join("new.yaml")]))
        });
        assert_eq!(inside, b"buffer");
        assert_eq!(found, Some(dir.path().join("new.yaml")));
        assert_eq!(read(&file).unwrap(), b"disk");
        assert_eq!(first_open(&[dir.path().join("new.yaml")]), None);
    }

    /// A retaining scope serves a file's first read for the rest of the scope,
    /// joins an enclosing scope, and keeps nothing once it ends.
    #[test]
    fn a_retaining_scope_reads_each_file_once_and_forgets_after() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("types.yaml");
        std::fs::write(&file, "first").unwrap();

        let (outer, inner) = with_reads_retained(|| {
            let outer = read(&file).unwrap();
            std::fs::write(&file, "second").unwrap();
            (outer, with_reads_retained(|| read_to_string(&file).unwrap()))
        });
        assert_eq!((outer.as_slice(), inner.as_str()), (b"first".as_slice(), "first"));
        assert_eq!(read(&file).unwrap(), b"second", "nothing is kept past the scope");
        assert!(text(&file).is_none());
    }

    /// The resolver's reads, through the public entry point: an unsaved
    /// import replaces the file's text, an unsaved target that exists only in
    /// the editor is selected, and outside [`with`] the disk is read again.
    #[cfg(feature = "file")]
    #[test]
    fn resolution_reads_unsaved_imports_and_selects_unsaved_targets_only_inside_with() {
        use crate::embedded::ShippedSources;
        use serde_json::json;

        let dir = tempfile::tempdir().unwrap();
        let context = biscuit_file::FileResolutionContext::new(dir.path());
        let library = dir.path().join("types.yaml");
        std::fs::write(&library, "kind: schema\ntypes:\n  Owner: string\n").unwrap();
        let reference: serde_yaml_ng::Value = serde_yaml_ng::from_str("owner: Owner@./types.yaml").unwrap();
        let target: serde_yaml_ng::Value = serde_yaml_ng::from_str("./contract.yaml").unwrap();
        let accepts = |schema: &serde_json::Value, owner: serde_json::Value| {
            jsonschema::validator_for(schema).unwrap().is_valid(&json!({ "owner": owner }))
        };
        let resolve = |value: &serde_yaml_ng::Value| super::super::resolve_yaml_schema(value, dir.path(), &context, ShippedSources::NONE);

        let mut buffers = SourceBuffers::default();
        buffers.insert(&library, Arc::from("kind: schema\ntypes:\n  Owner: number\n"));
        buffers.insert(&dir.path().join("contract.yaml"), Arc::from("$schema:\n  owner: number\n"));

        let saved = resolve(&reference).unwrap();
        assert!(accepts(&saved.json_schema, json!("ops")));
        let (unsaved, contract) = with(&buffers, || (resolve(&reference).unwrap(), resolve(&target).unwrap()));
        assert!(accepts(&unsaved.json_schema, json!(1)) && !accepts(&unsaved.json_schema, json!("ops")));
        assert!(accepts(&contract.json_schema, json!(1)) && !accepts(&contract.json_schema, json!("ops")));
        assert!(accepts(&resolve(&reference).unwrap().json_schema, json!("ops")));
        assert!(resolve(&target).is_err(), "the unsaved target is unknown outside `with`");
    }
}
