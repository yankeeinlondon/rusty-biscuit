//! Strategy A: committed manifest + local journal (roll back before the
//! manifest replacement, roll forward after it).
//!
//! State layout under [`STATE_DIR`] (gitignored, per worktree):
//!
//! ```text
//! lock                       File::try_lock target
//! journal.json               present iff a transaction is pending
//! tx/<txid>/new/<repo path>  staged new bytes (artifacts + manifest)
//! tx/<txid>/old/<repo path>  backups of every touched path that existed
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::fault::{Faults, Point};
use crate::fsutil::{
    PublicationLock, RetryPolicy, read_optional, remove_if_present, replace_file, sync_dir, write_durable,
    xxh64_hex,
};
use crate::model::{MANIFEST_PATH, STATE_DIR, Snapshot, parse_manifest};
use crate::{SpikeError, io_err, resolve};

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub faults: Faults,
    pub retry: RetryPolicy,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PublishReport {
    pub replaced: usize,
    pub unchanged: usize,
    pub removed: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recovery {
    Clean,
    RolledBack,
    RolledForward,
}

#[derive(Debug, Clone)]
struct Entry {
    path: String,
    old: Option<String>,
    new: Option<String>,
}

#[derive(Debug, Clone)]
struct Journal {
    txid: String,
    old_manifest: Option<String>,
    new_manifest: String,
    entries: Vec<Entry>,
}

fn state(root: &Path) -> PathBuf {
    resolve(root, STATE_DIR)
}

fn journal_path(root: &Path) -> PathBuf {
    state(root).join("journal.json")
}

fn tx_dir(root: &Path, txid: &str) -> PathBuf {
    state(root).join("tx").join(txid)
}

fn hash_at(path: &Path) -> Result<Option<String>, SpikeError> {
    Ok(read_optional(path).map_err(io_err(format!("read {}", path.display())))?.map(|b| xxh64_hex(&b)))
}

fn opt(value: &Option<String>) -> Value {
    value.as_ref().map_or(Value::Null, |v| Value::String(v.clone()))
}

impl Journal {
    fn to_bytes(&self) -> Vec<u8> {
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|e| json!({"path": e.path, "old_xxh64": opt(&e.old), "new_xxh64": opt(&e.new)}))
            .collect();
        let value = json!({
            "format": "messenger-research-publication-journal/1",
            "txid": self.txid,
            "state": "prepared",
            "old_manifest_xxh64": opt(&self.old_manifest),
            "new_manifest_xxh64": self.new_manifest,
            "entries": entries,
        });
        let mut out = serde_json::to_vec_pretty(&value).expect("json");
        out.push(b'\n');
        out
    }

    fn parse(bytes: &[u8]) -> Result<Self, SpikeError> {
        let corrupt = |what: &str| SpikeError::Corrupt(format!("journal: {what}"));
        let v: Value = serde_json::from_slice(bytes).map_err(|e| corrupt(&e.to_string()))?;
        if v["state"] != "prepared" {
            return Err(corrupt("unknown state"));
        }
        let s = |x: &Value| x.as_str().map(str::to_owned);
        let mut entries = Vec::new();
        for e in v["entries"].as_array().ok_or_else(|| corrupt("entries"))? {
            entries.push(Entry {
                path: s(&e["path"]).ok_or_else(|| corrupt("path"))?,
                old: s(&e["old_xxh64"]),
                new: s(&e["new_xxh64"]),
            });
        }
        Ok(Self {
            txid: s(&v["txid"]).ok_or_else(|| corrupt("txid"))?,
            old_manifest: s(&v["old_manifest_xxh64"]),
            new_manifest: s(&v["new_manifest_xxh64"]).ok_or_else(|| corrupt("new manifest"))?,
            entries,
        })
    }
}

/// Publish `snapshot` to the fixed paths under `root`.
pub fn publish(root: &Path, snapshot: &Snapshot, opts: Options) -> Result<PublishReport, SpikeError> {
    let _lock = PublicationLock::try_acquire(&state(root).join("lock"))?;
    if journal_path(root).exists() {
        return Err(SpikeError::RecoveryRequired);
    }
    opts.faults.check(Point::BeforeStaging)?;

    let new_manifest = snapshot.manifest_bytes();
    let old_manifest_bytes =
        read_optional(&resolve(root, MANIFEST_PATH)).map_err(io_err("read manifest"))?;
    let old_paths: Vec<String> = match &old_manifest_bytes {
        Some(bytes) => parse_manifest(bytes)?.artifacts.into_keys().collect(),
        None => Vec::new(),
    };
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let txid = format!("{}-{nanos:x}", &xxh64_hex(&new_manifest)[..8]);
    let tx = tx_dir(root, &txid);

    // Stage new bytes.
    for (index, (path, bytes)) in snapshot.artifacts.iter().enumerate() {
        write_durable(&resolve(&tx.join("new"), path), bytes).map_err(io_err("stage"))?;
        if index == 0 {
            opts.faults.check(Point::DuringStaging)?;
        }
    }
    write_durable(&resolve(&tx.join("new"), MANIFEST_PATH), &new_manifest).map_err(io_err("stage manifest"))?;

    // Back up every touched path that exists, and build journal entries.
    let mut paths: Vec<String> = snapshot.artifacts.keys().cloned().collect();
    paths.extend(old_paths.into_iter().filter(|p| !snapshot.artifacts.contains_key(p)));
    paths.sort();
    let mut entries = Vec::new();
    for path in paths.iter().chain(std::iter::once(&MANIFEST_PATH.to_string())) {
        let current = read_optional(&resolve(root, path)).map_err(io_err(format!("read {path}")))?;
        if let Some(bytes) = &current {
            write_durable(&resolve(&tx.join("old"), path), bytes).map_err(io_err("backup"))?;
        }
        if path != MANIFEST_PATH {
            entries.push(Entry {
                path: path.clone(),
                old: current.as_deref().map(xxh64_hex),
                new: snapshot.artifacts.get(path).map(|b| xxh64_hex(b)),
            });
        }
    }
    sync_dir(&tx);
    opts.faults.check(Point::StagedNoJournal)?;

    let journal = Journal {
        txid: txid.clone(),
        old_manifest: old_manifest_bytes.as_deref().map(xxh64_hex),
        new_manifest: xxh64_hex(&new_manifest),
        entries,
    };
    replace_file(&journal_path(root), &journal.to_bytes(), &txid, opts.retry).map_err(io_err("write journal"))?;
    opts.faults.check(Point::BeforeSelection)?;

    let report = roll_forward(root, &journal, opts)?;
    finish(root, &journal, opts)?;
    Ok(report)
}

/// Apply staged bytes to every fixed path, then replace the manifest last.
fn roll_forward(root: &Path, journal: &Journal, opts: Options) -> Result<PublishReport, SpikeError> {
    let tx = tx_dir(root, &journal.txid);
    let mut report = PublishReport::default();
    for (index, entry) in journal.entries.iter().enumerate() {
        let dest = resolve(root, &entry.path);
        match &entry.new {
            Some(new) if hash_at(&dest)?.as_ref() == Some(new) => report.unchanged += 1,
            Some(new) => {
                let bytes = staged(&resolve(&tx.join("new"), &entry.path), new)?;
                replace_file(&dest, &bytes, &journal.txid, opts.retry)
                    .map_err(io_err(format!("replace {}", entry.path)))?;
                report.replaced += 1;
            }
            None => {
                remove_if_present(&dest, opts.retry).map_err(io_err(format!("remove {}", entry.path)))?;
                report.removed += 1;
            }
        }
        opts.faults.check(Point::MidReplacement(index + 1))?;
    }
    opts.faults.check(Point::BeforeManifest)?;
    let manifest = resolve(root, MANIFEST_PATH);
    if hash_at(&manifest)?.as_ref() != Some(&journal.new_manifest) {
        let bytes = staged(&resolve(&tx.join("new"), MANIFEST_PATH), &journal.new_manifest)?;
        replace_file(&manifest, &bytes, &journal.txid, opts.retry).map_err(io_err("replace manifest"))?;
    }
    opts.faults.check(Point::AfterSelection)?;
    Ok(report)
}

/// Restore backups (or remove paths that did not exist) for every entry.
fn roll_back(root: &Path, journal: &Journal, opts: Options) -> Result<(), SpikeError> {
    let tx = tx_dir(root, &journal.txid);
    for (index, entry) in journal.entries.iter().enumerate() {
        let dest = resolve(root, &entry.path);
        match &entry.old {
            Some(old) if hash_at(&dest)?.as_ref() == Some(old) => {}
            Some(old) => {
                let bytes = staged(&resolve(&tx.join("old"), &entry.path), old)?;
                replace_file(&dest, &bytes, &journal.txid, opts.retry)
                    .map_err(io_err(format!("restore {}", entry.path)))?;
            }
            None => remove_if_present(&dest, opts.retry).map_err(io_err("remove new path"))?,
        }
        opts.faults.check(Point::MidReplacement(index + 1))?;
    }
    Ok(())
}

fn staged(path: &Path, expected: &str) -> Result<Vec<u8>, SpikeError> {
    let bytes = read_optional(path)
        .map_err(io_err("read staged"))?
        .ok_or_else(|| SpikeError::Corrupt(format!("missing staged {}", path.display())))?;
    if xxh64_hex(&bytes) != expected {
        return Err(SpikeError::Corrupt(format!("staged bytes mismatch {}", path.display())));
    }
    Ok(bytes)
}

/// Remove the journal (the transaction is then closed), then staging and temps.
fn finish(root: &Path, journal: &Journal, opts: Options) -> Result<(), SpikeError> {
    remove_if_present(&journal_path(root), opts.retry).map_err(io_err("remove journal"))?;
    sync_dir(&state(root));
    opts.faults.check(Point::BeforeCleanup)?;
    cleanup_leftovers(root, journal.entries.iter().map(|e| e.path.as_str()))
}

/// Remove every tx directory and every `*.publish-tmp` sibling next to the
/// given paths and the manifest. Only valid while no journal is pending.
fn cleanup_leftovers<'a>(root: &Path, paths: impl Iterator<Item = &'a str>) -> Result<(), SpikeError> {
    let tx_root = state(root).join("tx");
    if tx_root.exists() {
        fs::remove_dir_all(&tx_root).map_err(io_err("remove tx"))?;
    }
    let mut dirs: Vec<PathBuf> = paths
        .chain(std::iter::once(MANIFEST_PATH))
        .filter_map(|p| resolve(root, p).parent().map(Path::to_path_buf))
        .collect();
    dirs.push(state(root));
    dirs.sort();
    dirs.dedup();
    for dir in dirs {
        let Ok(read) = fs::read_dir(&dir) else { continue };
        for item in read.flatten() {
            if item.file_name().to_string_lossy().ends_with(".publish-tmp") {
                let _ = fs::remove_file(item.path());
            }
        }
    }
    Ok(())
}

/// Startup recovery. Run by any writer (generate/publish) before it reads.
///
/// Decision rule: the committed manifest is the selection point. If it
/// already equals the journal's new manifest, roll forward; if it equals the
/// old manifest (or is absent when there was none), roll back. Anything else
/// is refused as corrupt; `git checkout` of the artifact set is the fallback.
/// Recovery is idempotent (every step is hash-guarded), so a crash during
/// recovery is handled by running it again. `opts.faults` exists for tests.
pub fn recover(root: &Path, opts: Options) -> Result<Recovery, SpikeError> {
    let _lock = PublicationLock::try_acquire(&state(root).join("lock"))?;
    let Some(bytes) = read_optional(&journal_path(root)).map_err(io_err("read journal"))? else {
        let manifest_paths = match read_optional(&resolve(root, MANIFEST_PATH)).map_err(io_err("manifest"))? {
            Some(b) => parse_manifest(&b).map(|m| m.artifacts.into_keys().collect()).unwrap_or_default(),
            None => Vec::<String>::new(),
        };
        cleanup_leftovers(root, manifest_paths.iter().map(String::as_str))?;
        return Ok(Recovery::Clean);
    };
    let journal = Journal::parse(&bytes)?;
    let current = hash_at(&resolve(root, MANIFEST_PATH))?;
    let outcome = if current.as_ref() == Some(&journal.new_manifest) {
        roll_forward(root, &journal, opts)?;
        Recovery::RolledForward
    } else if current == journal.old_manifest {
        roll_back(root, &journal, opts)?;
        Recovery::RolledBack
    } else {
        return Err(SpikeError::Corrupt("manifest matches neither side of the journal".into()));
    };
    finish(root, &journal, opts)?;
    Ok(outcome)
}

/// True while a transaction is pending (for reporting "recovery required").
pub fn pending(root: &Path) -> bool {
    journal_path(root).exists()
}
