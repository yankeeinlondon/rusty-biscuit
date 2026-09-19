//! Strategy B: immutable generation directories + one atomically replaced
//! pointer file; the fixed committed paths are a derived mirror.
//!
//! ```text
//! messenger/.research-state/generations/
//!   lock
//!   CURRENT                    snapshot_id of the selected generation
//!   <snapshot_id>/<repo path>  full artifact set + manifest
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::fault::{Faults, Point};
use crate::fsutil::{PublicationLock, RetryPolicy, read_optional, replace_file, sync_dir, write_durable, xxh64_hex};
use crate::model::{MANIFEST_PATH, Snapshot, Verified, parse_manifest};
use crate::{SpikeError, io_err, resolve};

pub const GEN_DIR: &str = "messenger/.research-state/generations";

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub faults: Faults,
    pub retry: RetryPolicy,
}

fn gens(root: &Path) -> PathBuf {
    resolve(root, GEN_DIR)
}

pub fn publish(root: &Path, snapshot: &Snapshot, opts: Options) -> Result<(), SpikeError> {
    let _lock = PublicationLock::try_acquire(&gens(root).join("lock"))?;
    opts.faults.check(Point::BeforeStaging)?;
    let manifest = snapshot.manifest_bytes();
    let id = parse_manifest(&manifest)?.snapshot_id;
    let generation = gens(root).join(&id);
    for (index, (path, bytes)) in snapshot.artifacts.iter().enumerate() {
        write_durable(&resolve(&generation, path), bytes).map_err(io_err("stage"))?;
        if index == 0 {
            opts.faults.check(Point::DuringStaging)?;
        }
    }
    write_durable(&resolve(&generation, MANIFEST_PATH), &manifest).map_err(io_err("stage manifest"))?;
    sync_dir(&generation);
    opts.faults.check(Point::BeforeSelection)?;
    replace_file(&gens(root).join("CURRENT"), id.as_bytes(), "b", opts.retry).map_err(io_err("pointer"))?;
    opts.faults.check(Point::AfterSelection)?;
    mirror(root, &id, opts)?;
    gc(root, &id);
    Ok(())
}

/// Copy the selected generation onto the fixed paths, manifest last.
fn mirror(root: &Path, id: &str, opts: Options) -> Result<(), SpikeError> {
    let generation = gens(root).join(id);
    let verified = read_generation(&generation)?;
    for (index, (path, bytes)) in verified.files.iter().enumerate() {
        let dest = resolve(root, path);
        if read_optional(&dest).map_err(io_err("read"))?.as_deref() != Some(bytes.as_slice()) {
            replace_file(&dest, bytes, "b", opts.retry).map_err(io_err(format!("mirror {path}")))?;
        }
        opts.faults.check(Point::MidReplacement(index + 1))?;
    }
    let manifest = fs::read(resolve(&generation, MANIFEST_PATH)).map_err(io_err("gen manifest"))?;
    replace_file(&resolve(root, MANIFEST_PATH), &manifest, "b", opts.retry).map_err(io_err("mirror manifest"))
}

/// Best effort: on Windows an open file inside an old generation blocks
/// removal; the directory is retried at the next publication.
fn gc(root: &Path, keep: &str) {
    let Ok(read) = fs::read_dir(gens(root)) else { return };
    for item in read.flatten() {
        if item.path().is_dir() && item.file_name() != keep {
            let _ = fs::remove_dir_all(item.path());
        }
    }
}

fn read_generation(generation: &Path) -> Result<Verified, SpikeError> {
    let bytes = fs::read(resolve(generation, MANIFEST_PATH)).map_err(io_err("gen manifest"))?;
    let manifest = parse_manifest(&bytes)?;
    let mut files = std::collections::BTreeMap::new();
    for (path, (hash, _)) in &manifest.artifacts {
        let data = fs::read(resolve(generation, path)).map_err(io_err(format!("gen {path}")))?;
        if &xxh64_hex(&data) != hash {
            return Err(SpikeError::Inconsistent { path: path.clone(), reason: "generation mismatch".into() });
        }
        files.insert(path.clone(), data);
    }
    Ok(Verified { manifest, files })
}

/// B's reader: follows the pointer into local state.
pub fn read_selected(root: &Path) -> Result<Verified, SpikeError> {
    let id = read_optional(&gens(root).join("CURRENT"))
        .map_err(io_err("pointer"))?
        .ok_or(SpikeError::NoSnapshot)?;
    read_generation(&gens(root).join(String::from_utf8_lossy(&id).as_ref()))
}

/// Re-mirror the selected generation onto fixed paths and drop the others.
pub fn recover(root: &Path, opts: Options) -> Result<(), SpikeError> {
    let _lock = PublicationLock::try_acquire(&gens(root).join("lock"))?;
    let Some(id) = read_optional(&gens(root).join("CURRENT")).map_err(io_err("pointer"))? else {
        return Ok(());
    };
    let id = String::from_utf8_lossy(&id).into_owned();
    mirror(root, &id, Options { faults: Faults::none(), ..opts })?;
    gc(root, &id);
    Ok(())
}
