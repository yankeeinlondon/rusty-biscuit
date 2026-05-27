//! Node identity for the remote-signal mesh.
//!
//! Each daemon owns a long-lived [`NodeIdentity`] backed by an
//! ed25519 keypair. The public component is used to derive a stable
//! `node_id` (hex-encoded compressed public key) that other peers see;
//! the private component signs outgoing payloads.
//!
//! Phase 3 persists the secret seed to disk in a single binary file
//! locked to owner-only permissions on Unix. Higher-level pairing and
//! mesh operations (Phase 4+) will sit on top of this module without
//! needing to know how the key material is stored.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ed25519_dalek::{
    SECRET_KEY_LENGTH, Signature, Signer, SigningKey, Verifier, VerifyingKey,
};
use rand_core::{OsRng, RngCore};

/// Length, in bytes, of an ed25519 public key.
pub const PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;

/// Length, in bytes, of an ed25519 signature.
pub const SIGNATURE_LENGTH: usize = ed25519_dalek::SIGNATURE_LENGTH;

/// Errors that can surface while loading, generating, or persisting a
/// [`NodeIdentity`].
#[derive(Debug, thiserror::Error)]
pub enum NodeIdentityError {
    /// The on-disk seed file existed but did not contain exactly
    /// [`SECRET_KEY_LENGTH`] bytes.
    #[error(
        "node identity file {path} has an invalid length: expected {expected} bytes, got {actual}"
    )]
    InvalidSeedLength {
        path: PathBuf,
        expected: usize,
        actual: usize,
    },

    /// A filesystem I/O operation failed while reading or writing the
    /// seed file.
    #[error("node identity I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Long-lived signing identity for a remote-signal node.
///
/// The struct owns the raw ed25519 secret seed and a pre-built
/// [`SigningKey`]. The public key is exposed both as a 32-byte array
/// and as a hex-encoded `node_id` string suitable for logs, RPC, and
/// peer-discovery payloads.
#[derive(Clone)]
pub struct NodeIdentity {
    signing_key: SigningKey,
}

impl NodeIdentity {
    /// Generate a fresh identity from the operating-system RNG.
    #[must_use]
    pub fn generate() -> Self {
        let mut seed = [0u8; SECRET_KEY_LENGTH];
        OsRng.fill_bytes(&mut seed);
        Self::from_seed(seed)
    }

    /// Reconstruct an identity from a raw secret seed.
    #[must_use]
    pub fn from_seed(seed: [u8; SECRET_KEY_LENGTH]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        Self { signing_key }
    }

    /// Raw secret seed bytes. Treat this as sensitive — callers should
    /// avoid copying it outside of the persistence path.
    #[must_use]
    pub fn secret_seed(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.signing_key.to_bytes()
    }

    #[must_use]
    pub fn public_key_bytes(&self) -> [u8; PUBLIC_KEY_LENGTH] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Verifying key for this identity (used by the verification path).
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Stable identifier derived from the public key: lowercase hex of
    /// the 32-byte compressed verifying key. Suitable for log lines,
    /// `owner_node_id` fields, and peer-discovery payloads.
    #[must_use]
    pub fn node_id(&self) -> String {
        encode_hex(&self.public_key_bytes())
    }

    /// Sign `payload` with the secret key. The returned 64-byte
    /// signature can be verified with [`VerifyingKey::verify`].
    #[must_use]
    pub fn sign(&self, payload: &[u8]) -> [u8; SIGNATURE_LENGTH] {
        let signature: Signature = self.signing_key.sign(payload);
        signature.to_bytes()
    }

    /// Load an identity from `path`, generating and persisting a fresh
    /// one if the file does not yet exist. The seed file is written
    /// with owner-only permissions (`0o600`) on Unix.
    pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Self, NodeIdentityError> {
        let path = path.as_ref();
        match fs::read(path) {
            Ok(bytes) => {
                if bytes.len() != SECRET_KEY_LENGTH {
                    return Err(NodeIdentityError::InvalidSeedLength {
                        path: path.to_path_buf(),
                        expected: SECRET_KEY_LENGTH,
                        actual: bytes.len(),
                    });
                }
                let mut seed = [0u8; SECRET_KEY_LENGTH];
                seed.copy_from_slice(&bytes);
                Ok(Self::from_seed(seed))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let identity = Self::generate();
                identity.persist(path)?;
                Ok(identity)
            }
            Err(source) => Err(NodeIdentityError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Persist the secret seed to `path`, creating parent directories
    /// as needed and applying owner-only permissions on Unix.
    pub fn persist(&self, path: impl AsRef<Path>) -> Result<(), NodeIdentityError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|source| NodeIdentityError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let seed = self.secret_seed();
        fs::write(path, seed).map_err(|source| NodeIdentityError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        set_owner_only_permissions(path)?;
        Ok(())
    }
}

impl fmt::Debug for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never reveal the secret seed via Debug.
        f.debug_struct("NodeIdentity")
            .field("node_id", &self.node_id())
            .finish_non_exhaustive()
    }
}

/// Verify `signature` over `payload` against the given 32-byte public
/// key. Returns `false` if the public key is malformed or the
/// signature does not validate.
#[must_use]
pub fn verify_signature(
    public_key: &[u8; PUBLIC_KEY_LENGTH],
    payload: &[u8],
    signature: &[u8; SIGNATURE_LENGTH],
) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let signature = Signature::from_bytes(signature);
    verifying_key.verify(payload, &signature).is_ok()
}

/// Encode a byte slice as lowercase hexadecimal.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<(), NodeIdentityError> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|source| NodeIdentityError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<(), NodeIdentityError> {
    // On non-Unix targets the daemon currently relies on platform
    // ACLs / the user's profile directory to gate access. The seed
    // file itself does not get a permission downgrade.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generated_identity_has_stable_node_id() {
        let identity = NodeIdentity::generate();
        let id = identity.node_id();
        assert_eq!(id.len(), PUBLIC_KEY_LENGTH * 2);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn from_seed_reproduces_public_key() {
        let original = NodeIdentity::generate();
        let seed = original.secret_seed();
        let reloaded = NodeIdentity::from_seed(seed);
        assert_eq!(original.public_key_bytes(), reloaded.public_key_bytes());
        assert_eq!(original.node_id(), reloaded.node_id());
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let identity = NodeIdentity::generate();
        let payload = b"hello-remote-signal";
        let signature = identity.sign(payload);
        assert!(verify_signature(
            &identity.public_key_bytes(),
            payload,
            &signature,
        ));
    }

    #[test]
    fn verify_rejects_tampered_payload() {
        let identity = NodeIdentity::generate();
        let signature = identity.sign(b"original");
        assert!(!verify_signature(
            &identity.public_key_bytes(),
            b"tampered",
            &signature,
        ));
    }

    #[test]
    fn load_or_generate_creates_seed_file() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("node.key");
        let first = NodeIdentity::load_or_generate(&path).expect("generate");
        assert!(path.exists());
        let second = NodeIdentity::load_or_generate(&path).expect("reload");
        assert_eq!(first.public_key_bytes(), second.public_key_bytes());
    }

    #[cfg(unix)]
    #[test]
    fn persisted_seed_has_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("node.key");
        let _ = NodeIdentity::load_or_generate(&path).expect("generate");
        let metadata = fs::metadata(&path).expect("metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0o600, got {mode:o}");
    }

    #[test]
    fn invalid_seed_file_is_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("node.key");
        fs::write(&path, b"too-short").expect("write");
        let err = NodeIdentity::load_or_generate(&path).expect_err("should fail");
        assert!(matches!(err, NodeIdentityError::InvalidSeedLength { .. }));
    }
}
