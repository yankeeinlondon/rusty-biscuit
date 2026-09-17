//! Root-document identity: `ctx.self`, `ctx.last_updated`, `ctx.hash`,
//! `ctx.id`, and `ctx.sid` (feature `2026-09-09-more-context`, R1–R5, D1, D2).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use serde_json::{Map, Value};

use super::snapshot::ContextCapture;
use crate::markdown::compose::ComposeSource;
use crate::markdown::hash::{MdHashKind, MdHashOptions};
use crate::markdown::Markdown;

pub(super) const KEYS: &[&str] = &["self", "last_updated", "hash", "id", "sid"];

/// Diagnostic area recorded when the execution nonce could not be drawn.
/// A read of `ctx.id` / `ctx.sid` in a context carrying it fails with
/// `ExpressionError::ExecutionNonceUnavailable`.
pub(crate) const NONCE_AREA: &str = "document.nonce";

/// Version tag prefixed to the encoded identity tuple (D1). Changing the
/// encoding requires a new tag.
const IDENTITY_VERSION: &[u8] = b"darkmatter.ctx.id/v1";

/// The root document of a compose request, retained by its context.
///
/// Built once from the document as loaded. Every source composed in the
/// request projects these values, so a transcluded fragment reports its root.
#[derive(Debug)]
pub(crate) struct RootDocument {
    text: Arc<str>,
    origin: RootOrigin,
}

#[derive(Debug)]
enum RootOrigin {
    File {
        canonical_path: Option<PathBuf>,
        modified: Option<SystemTime>,
    },
    /// URL and in-memory roots have no local path, modification time, or
    /// on-disk hash.
    Detached,
}

impl RootDocument {
    /// Captures `document`'s identity from the text it was parsed from.
    ///
    /// A file loaded through `TryFrom<&Path>` keeps the canonical path and
    /// modification time read with its text. A constructed document whose
    /// source names a file is canonicalized and stat-ed here; its content is
    /// never re-read.
    pub(crate) fn from_markdown(document: &Markdown) -> Arc<Self> {
        let loaded = document.loaded_source();
        let text = loaded
            .map(|loaded| loaded.text.clone())
            .unwrap_or_else(|| document.reconstruct_source().0.into());
        let origin = match document.source() {
            Some(ComposeSource::File(path)) => match loaded.and_then(|l| l.canonical_path.clone()) {
                Some(canonical_path) => RootOrigin::File {
                    canonical_path: Some(canonical_path),
                    modified: loaded.and_then(|loaded| loaded.modified),
                },
                None => {
                    let canonical_path = biscuit_file::canonicalize_simplified(path).ok();
                    let modified = canonical_path
                        .as_ref()
                        .and_then(|path| std::fs::metadata(path).ok())
                        .and_then(|metadata| metadata.modified().ok());
                    RootOrigin::File { canonical_path, modified }
                }
            },
            Some(ComposeSource::Url(_) | ComposeSource::Unknown) | None => RootOrigin::Detached,
        };
        Arc::new(Self { text, origin })
    }

    /// `md hash`'s default `{fm}-{body}` value for the loaded text.
    fn hash(&self) -> Option<String> {
        let document = Markdown::try_from_content(self.text.to_string()).ok()?;
        document
            .compute_hash(MdHashKind::Simple, &MdHashOptions::default())
            .flat_string()
    }
}

/// Everything besides the root that the identity tuple hashes.
pub(super) struct IdentityInputs<'a> {
    pub(super) timestamp_ms: i64,
    pub(super) hostname: &'a str,
    pub(super) repository: &'a str,
}

pub(super) fn populate_document(
    cap: &mut ContextCapture,
    root: Option<&RootDocument>,
    timestamp_ms: i64,
    values: &mut Map<String, Value>,
) {
    populate_document_with_nonce(cap, root, timestamp_ms, draw_nonce(), values);
}

fn populate_document_with_nonce(
    cap: &mut ContextCapture,
    root: Option<&RootDocument>,
    timestamp_ms: i64,
    nonce: Result<[u8; 16], String>,
    values: &mut Map<String, Value>,
) {
    let (self_path, last_updated, hash) = match root.map(|root| (root, &root.origin)) {
        Some((root, RootOrigin::File { canonical_path, modified })) => (
            canonical_path
                .as_ref()
                .map(|path| Value::String(path.display().to_string())),
            modified.map(format_modified),
            root.hash().map(Value::String),
        ),
        Some((_, RootOrigin::Detached)) | None => (None, None, None),
    };
    values.insert("self".into(), self_path.unwrap_or(Value::Null));
    values.insert("last_updated".into(), last_updated.unwrap_or(Value::Null));
    values.insert("hash".into(), hash.unwrap_or(Value::Null));

    match nonce {
        Ok(nonce) => {
            let inputs = IdentityInputs {
                timestamp_ms,
                hostname: cap.identity_hostname.as_deref().unwrap_or_default(),
                repository: cap.repo_name.as_deref().unwrap_or_default(),
            };
            let source = root.map_or("", |root| &root.text);
            let (id, sid) = identity_digests(&encode_identity(source, &inputs, &nonce));
            values.insert("id".into(), Value::String(id));
            values.insert("sid".into(), Value::String(sid));
        }
        // No fallback nonce: an id that is not unique per execution must not
        // be projected at all (Q1).
        Err(detail) => cap.diagnostics.push(
            super::ContextMergeDiagnostic::PartialRuntimeCapture { area: NONCE_AREA, detail },
        ),
    }
}

fn draw_nonce() -> Result<[u8; 16], String> {
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce)
        .map(|()| nonce)
        .map_err(|error| format!("the operating system could not supply entropy: {error}"))
}

fn format_modified(modified: SystemTime) -> Value {
    let local: chrono::DateTime<chrono::Local> = modified.into();
    Value::String(local.format("%Y-%m-%dT%H:%M:%S%:z").to_string())
}

/// The D1 encoding: the version tag, then each field as a little-endian
/// `u64` byte length followed by its bytes.
fn encode_identity(source: &str, inputs: &IdentityInputs<'_>, nonce: &[u8; 16]) -> Vec<u8> {
    let timestamp = inputs.timestamp_ms.to_le_bytes();
    let fields: [&[u8]; 5] = [
        source.as_bytes(),
        &timestamp,
        inputs.hostname.as_bytes(),
        inputs.repository.as_bytes(),
        nonce,
    ];
    let mut encoded = IDENTITY_VERSION.to_vec();
    for field in fields {
        encoded.extend_from_slice(&(field.len() as u64).to_le_bytes());
        encoded.extend_from_slice(field);
    }
    encoded
}

/// `(ctx.id, ctx.sid)`: 16-digit lowercase xxHash and full lowercase BLAKE3 hex.
fn identity_digests(encoded: &[u8]) -> (String, String) {
    let id = format!("{:016x}", biscuit_hash::xx_hash_bytes(encoded));
    let sid = biscuit_hash::blake3_hash_bytes(encoded)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    (id, sid)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const NONCE: [u8; 16] = *b"0123456789abcdef";

    fn inputs() -> IdentityInputs<'static> {
        IdentityInputs { timestamp_ms: 1_700_000_000_123, hostname: "host", repository: "repo" }
    }

    fn capture(hostname: Option<&str>, repository: Option<&str>) -> ContextCapture {
        let mut cap = ContextCapture::for_test_base(PathBuf::from("/tmp"), None);
        cap.identity_hostname = hostname.map(str::to_string);
        cap.repo_name = repository.map(str::to_string);
        cap
    }

    #[test]
    fn encoding_is_versioned_and_length_prefixed() {
        let encoded = encode_identity("ab", &inputs(), &NONCE);

        let mut expected = b"darkmatter.ctx.id/v1".to_vec();
        for field in [
            b"ab".as_slice(),
            &1_700_000_000_123_i64.to_le_bytes(),
            b"host",
            b"repo",
            &NONCE,
        ] {
            expected.extend_from_slice(&(field.len() as u64).to_le_bytes());
            expected.extend_from_slice(field);
        }
        assert_eq!(encoded, expected);
    }

    /// Frozen vectors: a change to the encoding or either digest fails here.
    #[test]
    fn frozen_identity_vectors() {
        let (id, sid) = identity_digests(&encode_identity("---\ntitle: x\n---\nbody\n", &inputs(), &NONCE));
        assert_eq!(id.len(), 16);
        assert_eq!(sid.len(), 64);
        assert!(id.chars().chain(sid.chars()).all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(id, FROZEN_ID, "ctx.id vector changed");
        assert_eq!(sid, FROZEN_SID, "ctx.sid vector changed");
        assert_ne!(id, sid[..16], "ctx.sid is not a re-spelling of ctx.id");
    }

    const FROZEN_ID: &str = "2f5a8fd2d6f97fa3";
    const FROZEN_SID: &str = "f9a05c57ade35083846458b2e484192004a51d67d83d006801c1fce48e4a133d";

    /// Length prefixes keep adjacent fields unambiguous.
    #[test]
    fn shifting_bytes_between_fields_changes_the_identity() {
        let left = IdentityInputs { hostname: "ab", repository: "c", ..inputs() };
        let right = IdentityInputs { hostname: "a", repository: "bc", ..inputs() };
        assert_ne!(
            identity_digests(&encode_identity("", &left, &NONCE)),
            identity_digests(&encode_identity("", &right, &NONCE)),
        );
    }

    /// AC4 / AC36: identical source, frozen clock, hostname, and repository
    /// still differ across executions because each draws its own nonce.
    #[test]
    fn identical_inputs_and_frozen_clock_differ_across_executions() {
        let root = RootDocument::from_markdown(&Markdown::from("same source\n"));
        let run = || {
            let mut cap = capture(Some("host"), Some("repo"));
            let mut values = Map::new();
            populate_document(&mut cap, Some(&root), 1_700_000_000_123, &mut values);
            assert!(cap.diagnostics.is_empty());
            (values["id"].clone(), values["sid"].clone())
        };
        let (first_id, first_sid) = run();
        let (second_id, second_sid) = run();
        assert_ne!(first_id, second_id);
        assert_ne!(first_sid, second_sid);
        assert_ne!(first_id, first_sid);
    }

    #[test]
    fn injected_nonce_reproduces_the_same_identity() {
        let root = RootDocument::from_markdown(&Markdown::from("same source\n"));
        let run = || {
            let mut cap = capture(Some("host"), Some("repo"));
            let mut values = Map::new();
            populate_document_with_nonce(&mut cap, Some(&root), 5, Ok(NONCE), &mut values);
            values
        };
        assert_eq!(run()["id"], run()["id"]);
        assert_eq!(run()["sid"], run()["sid"]);
    }

    /// An entropy failure projects neither id nor sid and records the typed
    /// area the checked lookup turns into a compose error.
    #[test]
    fn entropy_failure_projects_no_identity() {
        let mut cap = capture(Some("host"), None);
        let mut values = Map::new();
        populate_document_with_nonce(&mut cap, None, 5, Err("no entropy".into()), &mut values);

        assert!(!values.contains_key("id"));
        assert!(!values.contains_key("sid"));
        assert_eq!(
            cap.diagnostics,
            vec![super::super::ContextMergeDiagnostic::PartialRuntimeCapture {
                area: NONCE_AREA,
                detail: "no entropy".into(),
            }],
        );
    }

    /// R4: outside a repository and without a hostname the id is still
    /// generated, from `""` inputs.
    #[test]
    fn missing_hostname_and_repository_hash_as_empty_strings() {
        let mut cap = capture(None, None);
        let mut values = Map::new();
        populate_document_with_nonce(&mut cap, None, 7, Ok(NONCE), &mut values);

        let empty = IdentityInputs { timestamp_ms: 7, hostname: "", repository: "" };
        let (id, sid) = identity_digests(&encode_identity("", &empty, &NONCE));
        assert_eq!(values["id"], Value::String(id));
        assert_eq!(values["sid"], Value::String(sid));
    }

    #[test]
    fn in_memory_and_url_roots_have_no_path_mtime_or_hash() {
        let url = "https://example.com/doc.md".parse().expect("valid url");
        for document in [
            Markdown::from("---\ntitle: x\n---\nbody\n"),
            Markdown::from("body\n").with_source(ComposeSource::Url(url)),
            Markdown::from("body\n").with_source(ComposeSource::Unknown),
        ] {
            let root = RootDocument::from_markdown(&document);
            let mut cap = capture(Some("host"), None);
            let mut values = Map::new();
            populate_document_with_nonce(&mut cap, Some(&root), 7, Ok(NONCE), &mut values);
            assert_eq!(values["self"], Value::Null);
            assert_eq!(values["last_updated"], Value::Null);
            assert_eq!(values["hash"], Value::Null);
            assert!(values["id"].is_string());
            assert!(values["sid"].is_string());
        }
    }

    /// D2 / AC30: the identity describes the text as loaded. Mutating the
    /// document or rewriting the file afterwards changes neither the hash nor
    /// the source hashed into the id.
    #[test]
    fn loaded_file_identity_survives_mutation_and_rewrite() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("root.md");
        std::fs::write(&path, "---\ntitle: original\n---\noriginal body\n").expect("write root");
        let mut document = Markdown::try_from(path.as_path()).expect("load root");
        let expected_hash = document
            .compute_hash(MdHashKind::Simple, &MdHashOptions::default())
            .flat_string();

        document.content_mut().push_str("mutated\n");
        std::fs::write(&path, "rewritten\n").expect("rewrite root");
        let root = RootDocument::from_markdown(&document);

        let mut cap = capture(None, None);
        let mut values = Map::new();
        populate_document_with_nonce(&mut cap, Some(&root), 7, Ok(NONCE), &mut values);

        assert_eq!(values["hash"], Value::String(expected_hash.expect("simple hash")));
        let empty = IdentityInputs { timestamp_ms: 7, hostname: "", repository: "" };
        let (id, _) = identity_digests(&encode_identity(
            "---\ntitle: original\n---\noriginal body\n",
            &empty,
            &NONCE,
        ));
        assert_eq!(values["id"], Value::String(id));

        let self_path = values["self"].as_str().expect("ctx.self for a file root");
        assert!(Path::new(self_path).is_absolute());
        assert_eq!(
            PathBuf::from(self_path),
            biscuit_file::canonicalize_simplified(&path).expect("canonical root"),
        );
        assert!(values["last_updated"].is_string());
    }
}
