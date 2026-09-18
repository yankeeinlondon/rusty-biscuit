//! Snapshot model, the committed publication manifest, and the verified reader.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::fsutil::{read_optional, xxh64_hex};
use crate::{SpikeError, io_err, resolve};

/// Committed manifest: the single selection point for strategy A.
pub const MANIFEST_PATH: &str = "messenger/docs/research/publication.json";
/// Gitignored per-worktree state root.
pub const STATE_DIR: &str = "messenger/.research-state/publication";
pub const FORMAT: &str = "messenger-research-publication/1";
pub const SUPPORTED_SCHEMA: &str = "1";

/// The artifact set a publication replaces, as portable repo-relative paths.
pub const FIXED_ARTIFACTS: &[&str] = &[
    "messenger/docs/research/platforms/discord.md",
    "messenger/docs/research/platforms/slack.md",
    "messenger/docs/research/platforms/telegram.md",
    "messenger/docs/research/platforms/whatsapp.md",
    "messenger/docs/research/platforms/signal.md",
    "messenger/docs/research/platforms/catalog.json",
    "messenger/docs/research/summary/platforms.md",
    "messenger/docs/research/CHANGELOG.md",
    ".claude/skills/messenger/platform-metadata.md",
];

/// A complete snapshot to publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub schema_version: String,
    pub schema_xxh64: String,
    /// Input path -> xxh64 (roster, schema, overrides, prompt...).
    pub inputs: BTreeMap<String, String>,
    /// Artifact path -> bytes.
    pub artifacts: BTreeMap<String, Vec<u8>>,
}

/// Parsed committed manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema_version: String,
    pub schema_xxh64: String,
    pub inputs: BTreeMap<String, String>,
    pub artifacts: BTreeMap<String, (String, u64)>,
    pub snapshot_id: String,
}

impl Snapshot {
    /// Deterministic manifest bytes: sorted keys, LF, trailing newline, no
    /// timestamps or host paths. Identical snapshots produce identical bytes.
    pub fn manifest_bytes(&self) -> Vec<u8> {
        let artifacts: Vec<Value> = self
            .artifacts
            .iter()
            .map(|(path, bytes)| json!({"path": path, "xxh64": xxh64_hex(bytes), "len": bytes.len()}))
            .collect();
        let inputs: Vec<Value> =
            self.inputs.iter().map(|(path, hash)| json!({"path": path, "xxh64": hash})).collect();
        let body = json!({
            "format": FORMAT,
            "schema": {"version": self.schema_version, "xxh64": self.schema_xxh64},
            "inputs": inputs,
            "artifacts": artifacts,
        });
        let snapshot_id = xxh64_hex(&serde_json::to_vec(&body).expect("json"));
        let mut value = body;
        value["snapshot_id"] = Value::String(snapshot_id);
        let mut out = serde_json::to_vec_pretty(&value).expect("json");
        out.push(b'\n');
        out
    }
}

pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest, SpikeError> {
    let corrupt = |what: &str| SpikeError::Corrupt(format!("manifest: {what}"));
    let value: Value = serde_json::from_slice(bytes).map_err(|e| corrupt(&e.to_string()))?;
    if value["format"] != FORMAT {
        return Err(corrupt("unknown format"));
    }
    let text = |v: &Value| v.as_str().map(str::to_owned).ok_or_else(|| corrupt("expected string"));
    let mut artifacts = BTreeMap::new();
    for entry in value["artifacts"].as_array().ok_or_else(|| corrupt("artifacts"))? {
        let len = entry["len"].as_u64().ok_or_else(|| corrupt("len"))?;
        artifacts.insert(text(&entry["path"])?, (text(&entry["xxh64"])?, len));
    }
    let mut inputs = BTreeMap::new();
    for entry in value["inputs"].as_array().ok_or_else(|| corrupt("inputs"))? {
        inputs.insert(text(&entry["path"])?, text(&entry["xxh64"])?);
    }
    Ok(Manifest {
        schema_version: text(&value["schema"]["version"])?,
        schema_xxh64: text(&value["schema"]["xxh64"])?,
        inputs,
        artifacts,
        snapshot_id: text(&value["snapshot_id"])?,
    })
}

/// A snapshot whose every artifact matched the committed manifest.
#[derive(Debug)]
pub struct Verified {
    pub manifest: Manifest,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// The only read path generation and reporting may use.
///
/// Reads the manifest first, then every artifact into memory, and accepts
/// only if each artifact's bytes hash to the manifest's value. Bytes held by
/// the caller therefore always equal exactly one published snapshot, even if
/// a publication runs concurrently (it then fails verification and the
/// caller may retry). A pending journal alone is not a refusal: if the
/// bytes verify, they are a complete snapshot.
pub fn read_verified(root: &Path) -> Result<Verified, SpikeError> {
    let manifest_bytes = read_optional(&resolve(root, MANIFEST_PATH))
        .map_err(io_err("read manifest"))?
        .ok_or(SpikeError::NoSnapshot)?;
    let manifest = parse_manifest(&manifest_bytes)?;
    if manifest.schema_version != SUPPORTED_SCHEMA {
        return Err(SpikeError::Inconsistent {
            path: MANIFEST_PATH.into(),
            reason: format!("unsupported schema {}", manifest.schema_version),
        });
    }
    let mut files = BTreeMap::new();
    for (path, (hash, len)) in &manifest.artifacts {
        let bytes = read_optional(&resolve(root, path))
            .map_err(io_err(format!("read {path}")))?
            .ok_or_else(|| SpikeError::Inconsistent { path: path.clone(), reason: "missing".into() })?;
        if bytes.len() as u64 != *len || &xxh64_hex(&bytes) != hash {
            return Err(SpikeError::Inconsistent { path: path.clone(), reason: "hash mismatch".into() });
        }
        files.insert(path.clone(), bytes);
    }
    Ok(Verified { manifest, files })
}

/// Deterministic fixture snapshot. `label` varies every artifact except
/// `signal.md`, which is reused unchanged (a failed-platform carry-over).
/// `extra_review` adds a review artifact absent from earlier snapshots.
pub fn fixture(label: &str, extra_review: bool) -> Snapshot {
    let mut artifacts = BTreeMap::new();
    for path in FIXED_ARTIFACTS {
        let body = if path.ends_with("signal.md") {
            "---\nplatform: signal\nobserved: 2026-08-01\n---\n\nCarried-over accepted research.\n".to_string()
        } else if path.ends_with(".json") {
            format!("{{\n  \"snapshot\": \"{label}\",\n  \"platforms\": 5\n}}\n")
        } else {
            format!("---\nsnapshot: {label}\n---\n\n# {path}\n\nBody for {label}.\n")
        };
        artifacts.insert((*path).to_string(), body.into_bytes());
    }
    if extra_review {
        artifacts.insert(
            format!("messenger/docs/research/reviews/{label}.json"),
            format!("{{\"review\": \"{label}\"}}\n").into_bytes(),
        );
    }
    let schema = format!("schema for {}", SUPPORTED_SCHEMA);
    let mut inputs = BTreeMap::new();
    inputs.insert("messenger/docs/platforms.yaml".into(), xxh64_hex(format!("roster {label}").as_bytes()));
    inputs.insert("messenger/docs/research/platforms/_schema.yaml".into(), xxh64_hex(schema.as_bytes()));
    Snapshot {
        schema_version: SUPPORTED_SCHEMA.into(),
        schema_xxh64: xxh64_hex(schema.as_bytes()),
        inputs,
        artifacts,
    }
}
