//! The artifact wire format: strict decoding and deterministic encoding.
//!
//! An artifact is one UTF-8 JSON object with exactly two members, `header`
//! and `payload`, written by [`encode`] with `serde_json`'s pretty printer
//! and one final LF. [`decode_header`] and [`decode_payload`] are its only
//! readers. Both refuse what JSON permits but an artifact never contains: a
//! repeated object key (anywhere, payload schemas included), content after
//! the document, and empty input. JSON5 comments and trailing commas are
//! not JSON and fail the parser. Every refusal carries the JSON pointer of
//! the offending member, when one is known, and the parser's line and
//! column.

use std::collections::HashSet;
use std::fmt;

use serde::de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;

use super::{ArtifactHeader, FallbackReason, Refusal};

/// The decoded top level of an artifact whose payload is not read yet.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    header: ArtifactHeader,
    #[serde(borrow)]
    payload: &'a RawValue,
}

/// An artifact whose document shape and header decoded; its payload is still
/// the authored bytes.
pub(crate) struct DecodedHeader<'a> {
    pub(crate) header: ArtifactHeader,
    payload: &'a RawValue,
    /// Byte offset of the payload in the artifact, for error coordinates.
    payload_offset: usize,
    bytes: &'a [u8],
}

/// Encodes `header` and `payload` as an artifact: pretty-printed JSON and one
/// final LF. The same values always encode to the same bytes.
///
/// ## Errors
///
/// The serializer's message when a value is not representable, such as a
/// shipped record holding a file-backed namespace.
pub(crate) fn encode<T: Serialize>(header: &ArtifactHeader, payload: &T) -> Result<Vec<u8>, String> {
    #[derive(Serialize)]
    struct EnvelopeOut<'a, T> {
        header: &'a ArtifactHeader,
        payload: &'a T,
    }
    let mut bytes =
        serde_json::to_vec_pretty(&EnvelopeOut { header, payload }).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Checks the document shape of `bytes` and decodes its header.
pub(crate) fn decode_header(bytes: &[u8]) -> Result<DecodedHeader<'_>, Refusal> {
    check_document(bytes)?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope: Envelope<'_> = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| typed_refusal(error, bytes, 0))?;
    let payload_offset = envelope.payload.get().as_ptr() as usize - bytes.as_ptr() as usize;
    Ok(DecodedHeader { header: envelope.header, payload: envelope.payload, payload_offset, bytes })
}

/// Decodes the payload of an artifact whose header was accepted.
pub(crate) fn decode_payload<T: DeserializeOwned>(decoded: &DecodedHeader<'_>) -> Result<T, Refusal> {
    let mut deserializer = serde_json::Deserializer::from_str(decoded.payload.get());
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let mut refusal = typed_refusal(error, decoded.bytes, decoded.payload_offset);
        refusal.pointer = Some(format!("/payload{}", refusal.pointer.as_deref().unwrap_or("")));
        refusal
    })
}

/// A typed-decoding error at `offset` bytes into the artifact, located in
/// the whole artifact.
fn typed_refusal(error: serde_path_to_error::Error<serde_json::Error>, bytes: &[u8], offset: usize) -> Refusal {
    let pointer = pointer_of(error.path());
    let inner = error.into_inner();
    let (line, column) = absolute_position(bytes, offset, inner.line(), inner.column());
    Refusal {
        reason: FallbackReason::MalformedArtifact,
        pointer: Some(pointer),
        line: Some(line),
        column: Some(column),
        detail: strip_position(&inner.to_string()),
    }
}

/// The JSON pointer of a `serde_path_to_error` path.
fn pointer_of(path: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    let mut pointer = String::new();
    for segment in path.iter() {
        pointer.push('/');
        match segment {
            Segment::Seq { index } => pointer.push_str(&index.to_string()),
            Segment::Map { key } => pointer.push_str(&key.replace('~', "~0").replace('/', "~1")),
            Segment::Enum { variant } => pointer.push_str(variant),
            Segment::Unknown => pointer.push('?'),
        }
    }
    pointer
}

/// The 1-based line and column, in the whole artifact, of a position that a
/// parser started `offset` bytes in reported as `line`/`column`.
fn absolute_position(bytes: &[u8], offset: usize, line: usize, column: usize) -> (usize, usize) {
    let before = &bytes[..offset.min(bytes.len())];
    let base_line = 1 + before.iter().filter(|byte| **byte == b'\n').count();
    let base_column = 1 + before.iter().rev().take_while(|byte| **byte != b'\n').count();
    if line <= 1 { (base_line, base_column + column.saturating_sub(1)) } else { (base_line + line - 1, column) }
}

/// `serde_json`'s message without its trailing ` at line L column C`, which
/// the refusal carries separately.
fn strip_position(message: &str) -> String {
    match message.rfind(" at line ") {
        Some(index) => message[..index].to_string(),
        None => message.to_string(),
    }
}

/// Refuses bytes that are not exactly one JSON document without repeated
/// object keys.
fn check_document(bytes: &[u8]) -> Result<(), Refusal> {
    let syntax = |detail: String, line: Option<usize>, column: Option<usize>| Refusal {
        reason: FallbackReason::MalformedArtifact,
        pointer: None,
        line,
        column,
        detail,
    };
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(syntax("the artifact is empty".into(), None, None));
    }
    if let Err(error) = std::str::from_utf8(bytes) {
        let (line, column) = absolute_position(bytes, error.valid_up_to(), 1, 1);
        return Err(syntax(format!("the artifact is not UTF-8: {error}"), Some(line), Some(column)));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut pointer = Vec::new();
    let checked = UniqueKeys { pointer: &mut pointer }.deserialize(&mut deserializer).and_then(|()| deserializer.end());
    checked.map_err(|error| Refusal {
        reason: FallbackReason::MalformedArtifact,
        pointer: (!pointer.is_empty() || error.classify() == serde_json::error::Category::Data)
            .then(|| pointer.iter().map(|segment: &String| format!("/{segment}")).collect()),
        line: Some(error.line()),
        column: Some(error.column()),
        detail: strip_position(&error.to_string()),
    })
}

/// Walks one JSON value, refusing an object that repeats a key. On a
/// refusal, `pointer` holds the path of the repeated key.
struct UniqueKeys<'p> {
    pointer: &'p mut Vec<String>,
}

impl<'de> DeserializeSeed<'de> for UniqueKeys<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for UniqueKeys<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        let mut index = 0usize;
        loop {
            self.pointer.push(index.to_string());
            if sequence.next_element_seed(UniqueKeys { pointer: self.pointer })?.is_none() {
                self.pointer.pop();
                return Ok(());
            }
            self.pointer.pop();
            index += 1;
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            let escaped = key.replace('~', "~0").replace('/', "~1");
            if !seen.insert(key) {
                self.pointer.push(escaped);
                return Err(serde::de::Error::custom("a key is repeated in one object"));
            }
            self.pointer.push(escaped);
            map.next_value_seed(UniqueKeys { pointer: self.pointer })?;
            self.pointer.pop();
        }
        Ok(())
    }
}
