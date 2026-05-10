use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use biscuit_hash::xx_hash_bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::{ClipboardFormat, ContentType, ImageSnapshot};

/// Format priority used by [`ClipboardEntry::primary_content_type`],
/// [`ClipboardEntry::preview`], and the service's
/// `/history/:id/content` default-format selection.
///
/// Order: `text > html > rtf > image > files`. This is the single
/// source of truth — see review-1 Code Quality #5.
pub const FORMAT_PRIORITY: [ContentType; 5] = [
    ContentType::Text,
    ContentType::Html,
    ContentType::Rtf,
    ContentType::Image,
    ContentType::Files,
];

/// Stable identifier for a [`ClipboardEntry`].
///
/// `EntryId` is a thin newtype around `String` that preserves the
/// historical wire format (a 16-character hex xxhash, e.g.
/// `"a1b2c3d4e5f60718"`) while giving the type system a fighting
/// chance against entry-id / arbitrary-string mix-ups. The serde
/// representation is transparent: an `EntryId` serializes to and
/// deserializes from a JSON string with no envelope.
///
/// ## Examples
///
/// Round-trip through `Display` / `FromStr`:
///
/// ```
/// use std::str::FromStr;
/// use biscuit_clipboard::EntryId;
///
/// let id = EntryId::from_str("a1b2c3d4e5f60718").unwrap();
/// assert_eq!(id.to_string(), "a1b2c3d4e5f60718");
/// ```
///
/// Construct from a `u64` content hash:
///
/// ```
/// use biscuit_clipboard::EntryId;
///
/// let id = EntryId::from(0xDEADBEEFu64);
/// assert_eq!(id.as_str(), "00000000deadbeef");
/// ```
///
/// ## Notes
///
/// The `"current"` value is a valid `EntryId` used by the live
/// `GET /current` endpoint to mark an entry-shaped response that
/// reflects the OS clipboard rather than a row in history. It is the
/// only non-hex `EntryId` the wire contract recognises.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(String);

impl EntryId {
    /// Wrap a string as an `EntryId` without validation.
    ///
    /// ## Notes
    ///
    /// Use [`FromStr`] if you need rejection of malformed identifiers.
    /// This constructor exists so internal callers (e.g. the daemon
    /// turning a `Path<String>` into an id) can avoid the parse cost
    /// when they already trust the source.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the id and return the inner [`String`].
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors returned by [`EntryId`]'s `FromStr` impl.
#[derive(Debug, thiserror::Error)]
pub enum EntryIdParseError {
    /// The input was the empty string.
    #[error("entry id must not be empty")]
    Empty,
    /// The input was neither a 16-char hex string nor the literal
    /// sentinel `"current"`.
    #[error("entry id '{0}' is not a 16-char hex string or 'current'")]
    Malformed(String),
}

impl FromStr for EntryId {
    type Err = EntryIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(EntryIdParseError::Empty);
        }
        if s == "current" {
            return Ok(Self(s.to_string()));
        }
        if s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(Self(s.to_string()));
        }
        Err(EntryIdParseError::Malformed(s.to_string()))
    }
}

impl AsRef<str> for EntryId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for EntryId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<u64> for EntryId {
    fn from(hash: u64) -> Self {
        Self(format!("{hash:016x}"))
    }
}

impl From<String> for EntryId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EntryId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl PartialEq<str> for EntryId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for EntryId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for EntryId {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

/// One captured snapshot of the OS clipboard, with all available
/// formats and a stable content-addressed [`EntryId`].
///
/// ## Examples
///
/// ```
/// use biscuit_clipboard::{ClipboardEntry, content::ClipboardFormat};
///
/// let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".into())]);
/// assert_eq!(entry.find_text(), Some("hello"));
/// assert_eq!(entry.id.as_str().len(), 16);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardEntry {
    /// Stable content-addressed identifier for this entry.
    pub id: EntryId,
    /// UTC timestamp at the moment the entry was captured.
    pub timestamp: DateTime<Utc>,
    /// xxHash digest of `formats`; matches `id` when serialized as hex.
    pub content_hash: u64,
    /// Every format captured for this clipboard change.
    pub formats: Vec<ClipboardFormat>,
}

impl ClipboardEntry {
    /// Build a new entry from a list of formats; computes the content
    /// hash and id.
    pub fn new(formats: Vec<ClipboardFormat>) -> Self {
        let content_hash = content_hash_of(&formats);
        Self::with_hash(formats, content_hash)
    }

    /// Construct an entry from a precomputed content hash.
    ///
    /// ## Notes
    ///
    /// Used by callers that have already computed
    /// [`content_hash_of`] (e.g. the dedup probe in
    /// [`crate::history::History::insert`]) so the hash is not
    /// recomputed.
    pub fn with_hash(formats: Vec<ClipboardFormat>, content_hash: u64) -> Self {
        let id = EntryId::from(content_hash);
        let timestamp = Utc::now();
        Self {
            id,
            timestamp,
            content_hash,
            formats,
        }
    }

    /// Return the highest-priority [`ContentType`] available on this
    /// entry. Defaults to [`ContentType::Text`] for empty entries.
    pub fn primary_content_type(&self) -> ContentType {
        let available: Vec<ContentType> = self.formats.iter().map(|f| f.content_type()).collect();

        for ct in &FORMAT_PRIORITY {
            if available.contains(ct) {
                return *ct;
            }
        }

        ContentType::Text
    }

    /// A short human-readable preview drawn from the highest-priority
    /// available format.
    pub fn preview(&self) -> String {
        for ct in &FORMAT_PRIORITY {
            if let Some(fmt) = self.formats.iter().find(|f| f.content_type() == *ct) {
                return fmt.preview();
            }
        }

        String::new()
    }

    pub fn total_size_bytes(&self) -> usize {
        self.formats.iter().map(|f| f.size_bytes()).sum()
    }

    pub fn find_format(&self, content_type: ContentType) -> Option<&ClipboardFormat> {
        self.formats
            .iter()
            .find(|f| f.content_type() == content_type)
    }

    pub fn find_text(&self) -> Option<&str> {
        self.formats
            .iter()
            .find(|f| f.content_type() == ContentType::Text)
            .and_then(|f| f.as_text())
    }

    pub fn find_image(&self) -> Option<&crate::content::ImageSnapshot> {
        self.formats
            .iter()
            .find(|f| f.content_type() == ContentType::Image)
            .and_then(|f| f.as_image())
    }
}

/// Compute a stable content hash over a set of clipboard formats.
///
/// This is the single source of truth for clipboard-entry hashing —
/// shared by both [`ClipboardEntry::new`] (which derives the entry
/// id) and the dedup probe in [`crate::history::History::insert`].
///
/// ## Notes
///
/// For [`ImageSnapshot::Spilled`], the file's xxhash is recovered
/// from the filename's hex stem (the spill path is named
/// `{xxhash:016x}.dat`, see [`crate::storage::Storage::spill_if_needed`]).
/// This means a `Spilled` and an `Inline` image with the same byte
/// content hash to the same value, which is the property the dedup
/// probe relies on.
pub fn content_hash_of(formats: &[ClipboardFormat]) -> u64 {
    use std::io::Write;

    let mut hasher = Vec::new();
    for fmt in formats {
        match fmt {
            ClipboardFormat::Text(s) => {
                let _ = hasher.write_all(b"text:");
                let _ = hasher.write_all(s.as_bytes());
            }
            ClipboardFormat::Html(s) => {
                let _ = hasher.write_all(b"html:");
                let _ = hasher.write_all(s.as_bytes());
            }
            ClipboardFormat::Rtf(s) => {
                let _ = hasher.write_all(b"rtf:");
                let _ = hasher.write_all(s.as_bytes());
            }
            ClipboardFormat::Image(img) => {
                let _ = hasher.write_all(b"image:");
                let bytes_hash = image_payload_hash(img);
                let _ = hasher.write_all(&bytes_hash.to_le_bytes());
                let _ = hasher.write_all(&img.width().to_le_bytes());
                let _ = hasher.write_all(&img.height().to_le_bytes());
            }
            ClipboardFormat::Files(paths) => {
                let _ = hasher.write_all(b"files:");
                for p in paths {
                    let _ = hasher.write_all(p.to_string_lossy().as_bytes());
                    let _ = hasher.write_all(b"\0");
                }
            }
        }
    }
    xx_hash_bytes(&hasher)
}

/// Stable identity hash for an image's payload.
///
/// - For inline images, hash the raw bytes via xxhash.
/// - For spilled images, recover the original payload hash from the
///   filename stem (which is a 16-character hex xxhash, written by
///   [`crate::storage::Storage::spill_if_needed`]). If the stem cannot
///   be parsed as hex (unexpected — but we don't want to panic), fall
///   back to a tagged tuple of `(size_bytes, path)` which is at worst
///   no less collision-prone than the old behaviour.
fn image_payload_hash(image: &ImageSnapshot) -> u64 {
    match image {
        ImageSnapshot::Inline { data, .. } => xx_hash_bytes(data),
        ImageSnapshot::Spilled {
            path, size_bytes, ..
        } => {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if stem.len() == 16
                && let Ok(parsed) = u64::from_str_radix(stem, 16)
            {
                return parsed;
            }
            let mut buf = Vec::with_capacity(8 + path.as_os_str().len());
            buf.extend_from_slice(&size_bytes.to_le_bytes());
            buf.extend_from_slice(path.to_string_lossy().as_bytes());
            xx_hash_bytes(&buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ImageSnapshot;
    use std::path::PathBuf;

    #[test]
    fn test_entry_id_is_hex_string() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        assert_eq!(entry.id.as_str().len(), 16);
        assert!(entry.id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_entry_id_deterministic() {
        let entry1 = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        let entry2 = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        assert_eq!(entry1.id, entry2.id);
    }

    #[test]
    fn test_entry_id_different_content() {
        let entry1 = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        let entry2 = ClipboardEntry::new(vec![ClipboardFormat::Text("world".to_string())]);
        assert_ne!(entry1.id, entry2.id);
    }

    #[test]
    fn test_primary_content_type_text() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hi".to_string())]);
        assert_eq!(entry.primary_content_type(), ContentType::Text);
    }

    #[test]
    fn test_primary_content_type_priority() {
        let entry = ClipboardEntry::new(vec![
            ClipboardFormat::Html("<b>hi</b>".to_string()),
            ClipboardFormat::Text("hi".to_string()),
        ]);
        assert_eq!(entry.primary_content_type(), ContentType::Text);
    }

    #[test]
    fn test_preview() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hello world".to_string())]);
        assert_eq!(entry.preview(), "hello world");
    }

    #[test]
    fn test_total_size_bytes() {
        let entry = ClipboardEntry::new(vec![
            ClipboardFormat::Text("hello".to_string()),
            ClipboardFormat::Html("<b>hello</b>".to_string()),
        ]);
        assert_eq!(entry.total_size_bytes(), 5 + 12);
    }

    #[test]
    fn test_find_format() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hi".to_string())]);
        assert!(entry.find_format(ContentType::Text).is_some());
        assert!(entry.find_format(ContentType::Html).is_none());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let entry = ClipboardEntry::new(vec![
            ClipboardFormat::Text("hello".to_string()),
            ClipboardFormat::Files(vec![PathBuf::from("/tmp/test.txt")]),
        ]);
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ClipboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.content_hash, deserialized.content_hash);
    }

    #[test]
    fn test_image_entry_hash_uses_data() {
        let entry1 = ClipboardEntry::new(vec![ClipboardFormat::Image(ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 10,
        })]);
        let entry2 = ClipboardEntry::new(vec![ClipboardFormat::Image(ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 10,
        })]);
        assert_eq!(entry1.id, entry2.id);
    }

    #[test]
    fn test_spilled_image_hash_matches_inline() {
        let bytes = vec![1u8, 2, 3, 4, 5];
        let payload_hash = biscuit_hash::xx_hash_bytes(&bytes);
        let spill_path = PathBuf::from(format!("/tmp/cache/{:016x}.dat", payload_hash));

        let inline = ClipboardFormat::Image(ImageSnapshot::Inline {
            data: bytes.clone(),
            width: 4,
            height: 4,
        });
        let spilled = ClipboardFormat::Image(ImageSnapshot::Spilled {
            path: spill_path,
            width: 4,
            height: 4,
            size_bytes: bytes.len() as u64,
        });

        let inline_hash = content_hash_of(&[inline]);
        let spilled_hash = content_hash_of(&[spilled]);
        assert_eq!(inline_hash, spilled_hash);
    }

    #[test]
    fn test_spilled_images_with_different_content_hash_differently() {
        // Two spilled entries whose payloads differ — they MUST hash to
        // different values. This is the regression fixture for the bug
        // described in review-1 #12.
        let path_a = PathBuf::from("/tmp/cache/0011223344556677.dat");
        let path_b = PathBuf::from("/tmp/cache/8899aabbccddeeff.dat");

        let fmt_a = ClipboardFormat::Image(ImageSnapshot::Spilled {
            path: path_a,
            width: 10,
            height: 10,
            size_bytes: 1024,
        });
        let fmt_b = ClipboardFormat::Image(ImageSnapshot::Spilled {
            path: path_b,
            width: 10,
            height: 10,
            size_bytes: 1024,
        });

        assert_ne!(content_hash_of(&[fmt_a]), content_hash_of(&[fmt_b]));
    }

    #[test]
    fn test_centralized_hasher_round_trip() {
        let formats = vec![
            ClipboardFormat::Text("hello".to_string()),
            ClipboardFormat::Html("<b>hi</b>".to_string()),
        ];
        let probe_hash = content_hash_of(&formats);
        let entry = ClipboardEntry::new(formats);
        assert_eq!(entry.content_hash, probe_hash);
        assert_eq!(entry.id.as_str(), format!("{probe_hash:016x}"));
    }

    #[test]
    fn test_with_hash_uses_supplied_hash() {
        let formats = vec![ClipboardFormat::Text("hi".to_string())];
        let entry = ClipboardEntry::with_hash(formats, 0xDEADBEEF);
        assert_eq!(entry.content_hash, 0xDEADBEEF);
        assert_eq!(entry.id.as_str(), format!("{:016x}", 0xDEADBEEFu64));
    }

    #[test]
    fn test_entry_id_display_round_trip() {
        let id = EntryId::from(0xa1b2c3d4_e5f60718u64);
        let rendered = id.to_string();
        let parsed: EntryId = rendered.parse().expect("hex round-trip");
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_entry_id_serde_is_transparent() {
        let id = EntryId::from(0x1234_5678_9abc_def0u64);
        let json = serde_json::to_string(&id).unwrap();
        // Transparent: serializes to a JSON string, no envelope.
        assert_eq!(json, "\"123456789abcdef0\"");
        let back: EntryId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn test_entry_id_from_str_accepts_current_sentinel() {
        let id: EntryId = "current".parse().unwrap();
        assert_eq!(id.as_str(), "current");
    }

    #[test]
    fn test_entry_id_from_str_rejects_non_hex() {
        assert!("not-hex".parse::<EntryId>().is_err());
        assert!("".parse::<EntryId>().is_err());
        assert!("zzzzzzzzzzzzzzzz".parse::<EntryId>().is_err());
        assert!("a1b2c3d4".parse::<EntryId>().is_err()); // wrong length
    }

    #[test]
    fn test_entry_id_from_u64_pads_to_16_chars() {
        let id = EntryId::from(0u64);
        assert_eq!(id.as_str(), "0000000000000000");
    }

    #[test]
    fn test_timestamp_is_recent() {
        let before = Utc::now();
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("test".to_string())]);
        let after = Utc::now();
        assert!(entry.timestamp >= before);
        assert!(entry.timestamp <= after);
    }
}
