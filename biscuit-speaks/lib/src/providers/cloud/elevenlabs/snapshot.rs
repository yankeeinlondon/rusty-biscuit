//! The local record of which voice ElevenLabs' default selection chose.
//!
//! Selecting a default voice lists the account's voices over the network. A
//! detached playback request must find prepared default speech without that
//! round trip, so each authoritative selection is recorded here: the voice
//! and model, the selection inputs that produced them, and a non-secret
//! account context. A record is trusted for [`TRUST_SECONDS`] from when it
//! was written; anything missing, unreadable, mismatched, or expired is a
//! cache miss.
//!
//! Records live beside the voice-capability cache file, so
//! `BISCUIT_SPEAKS_CACHE` relocates them too. No field holds, or is derived
//! from, an API key.

use std::marker::PhantomData;
use std::path::PathBuf;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::debug;

/// How long a record is trusted after it was written.
pub(crate) const TRUST_SECONDS: u64 = 24 * 60 * 60;

/// Names an optional caller-chosen label that tells accounts apart when two
/// API keys share an environment variable.
const ACCOUNT_LABEL_ENV: &str = "BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT";

/// Everything the default voice selection depends on besides the account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectionInputs {
    /// `male`, `female`, or `any`.
    pub gender: String,
    /// The requested language's code prefix.
    pub language: String,
    /// The effective model: the requested one, else the provider default.
    pub model: String,
    /// The configured voice used when the account lists no voices.
    pub fallback: String,
}

/// One recorded selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub voice: String,
    pub model: String,
    #[serde(deserialize_with = "object_only")]
    pub inputs: SelectionInputs,
    pub account_context: String,
    /// Unix seconds.
    pub written_at: u64,
}

/// A non-secret identifier for the account the API key belongs to.
///
/// The API address, the name of the environment variable the key is read
/// from (in the client's precedence order), and the optional
/// `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` label. Two keys under the same
/// variable name share a context unless the label differs; the 24-hour trust
/// window and refresh-on-prepare bound that. `None` without a key.
pub(crate) fn account_context(base_url: &str) -> Option<String> {
    let key_source = ["ELEVEN_LABS_API_KEY", "ELEVENLABS_API_KEY"]
        .into_iter()
        .find(|name| std::env::var(name).is_ok())?;
    let label = std::env::var(ACCOUNT_LABEL_ENV).unwrap_or_default();
    Some(format!("{base_url};key-source={key_source};label={label}"))
}

/// The record file for one selection and account.
///
/// `None` when there is no capability cache location (no home directory and
/// no `BISCUIT_SPEAKS_CACHE`).
fn snapshot_path(inputs: &SelectionInputs, account_context: &str) -> Option<PathBuf> {
    let cache_file = crate::cache::cache_file_path()?;
    let directory = cache_file.parent()?.to_path_buf();
    let identity = serde_json::to_string(&(inputs, account_context)).ok()?;
    let hash = biscuit_hash::xx_hash(&identity);
    Some(directory.join(format!(".biscuit-speaks-elevenlabs-voice-{hash:016x}.json")))
}

/// Publish `snapshot` atomically, replacing any earlier record for the same
/// selection and account.
pub(crate) fn write(snapshot: &Snapshot) -> Result<(), String> {
    let path = snapshot_path(&snapshot.inputs, &snapshot.account_context)
        .ok_or("no capability cache location")?;
    let json = serde_json::to_vec(snapshot).map_err(|error| error.to_string())?;
    crate::audio_cache::write_atomic(&path, &json).map_err(|error| error.to_string())
}

/// The trusted record for `inputs` and `account_context` at `now`, if any.
pub(crate) fn load_trusted(
    inputs: &SelectionInputs,
    account_context: &str,
    now: u64,
) -> Option<Snapshot> {
    let path = snapshot_path(inputs, account_context)?;
    let bytes = std::fs::read(&path).ok()?;
    let snapshot = match parse(&bytes) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            debug!(path = %path.display(), %error, "ignoring unreadable ElevenLabs voice record");
            return None;
        }
    };
    let matches = snapshot.inputs == *inputs
        && snapshot.account_context == account_context
        && snapshot.model == inputs.model
        && !snapshot.voice.is_empty();
    let fresh = snapshot.written_at <= now && now - snapshot.written_at < TRUST_SECONDS;
    (matches && fresh).then_some(snapshot)
}

/// Decode a record, rejecting any shape [`write`] does not produce.
///
/// Derived structs reject missing, null, mistyped, unknown, and duplicate
/// fields, and `end` rejects trailing content.
fn parse(bytes: &[u8]) -> Result<Snapshot, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let snapshot = object_only(&mut deserializer)?;
    deserializer.end()?;
    Ok(snapshot)
}

/// Deserialize a derived struct from a JSON object only; a derived struct
/// alone also accepts its fields as an array.
fn object_only<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct ObjectOnly<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectOnly<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object")
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_map(ObjectOnly(PhantomData))
}
