//! The artifact wire format: strict decoding and deterministic encoding.
//!
//! An artifact is one UTF-8 JSON object with exactly two members, `header`
//! and `payload`, written by [`encode`]: the header pretty-printed, and each
//! member of an object payload compact on its own line, then one final LF.
//! A line-based diff of two artifacts therefore names the header field or
//! payload section that changed, while the bulk of the payload costs no
//! indentation to scan. [`decode_header`], [`decode_laid_out`],
//! [`decode_payload`], and [`decode_section`] are its only readers. Both refuse what JSON permits but an artifact never contains: a
//! repeated object key (anywhere, payload schemas included), content after
//! the document, and empty input. JSON5 comments and trailing commas are
//! not JSON and fail the parser. Every refusal carries the JSON pointer of
//! the offending member, when one is known, and the parser's line and
//! column.

use std::borrow::Cow;
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

/// Encodes `header` and `payload` as an artifact (see the module layout).
/// The same values always encode to the same bytes.
///
/// ## Errors
///
/// The serializer's message when a value is not representable, such as a
/// shipped record holding a file-backed namespace.
pub(crate) fn encode<T: Serialize>(header: &ArtifactHeader, payload: &T) -> Result<Vec<u8>, String> {
    let text = |error: serde_json::Error| error.to_string();
    // The payload passes through `Value` (order-preserving, exact floats) so
    // an object payload can be laid out one member per line.
    let payload = serde_json::to_value(payload).map_err(text)?;
    let mut bytes = b"{\n\"header\": ".to_vec();
    bytes.extend(serde_json::to_vec_pretty(header).map_err(text)?);
    bytes.extend_from_slice(b",\n\"payload\": ");
    match &payload {
        serde_json::Value::Object(members) if !members.is_empty() => {
            bytes.extend_from_slice(b"{");
            for (index, (key, value)) in members.iter().enumerate() {
                bytes.extend_from_slice(if index == 0 { b"\n" } else { b",\n" });
                bytes.extend(serde_json::to_vec(key).map_err(text)?);
                bytes.push(b':');
                bytes.extend(serde_json::to_vec(value).map_err(text)?);
            }
            bytes.extend_from_slice(b"\n}");
        }
        other => bytes.extend(serde_json::to_vec(other).map_err(text)?),
    }
    bytes.extend_from_slice(b"\n}\n");
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

/// The header and payload members of an artifact in the canonical layout
/// ([`encode`]), read without scanning the members' JSON.
pub(crate) struct LaidOut<'a> {
    pub(crate) header: ArtifactHeader,
    /// Each payload member: its key, its JSON text, and the 1-based line and
    /// column where that text starts.
    pub(crate) members: Vec<(String, &'a str, usize, usize)>,
}

/// Reads an artifact whose payload is an object of sections by its layout:
/// the header is decoded, and each payload member is located on its own line
/// but not parsed. A section's JSON is checked when it is decoded
/// ([`decode_section`]). Bytes not in the canonical layout are refused:
/// [`encode`] is the only writer, and the freshness check compares bytes.
pub(crate) fn decode_laid_out(bytes: &[u8]) -> Result<LaidOut<'_>, Refusal> {
    check_text(bytes)?;
    let text = std::str::from_utf8(bytes).expect("checked UTF-8");
    let layout = |detail: &str, offset: usize| {
        let (line, column) = absolute_position(bytes, offset, 1, 1);
        Refusal {
            reason: FallbackReason::MalformedArtifact,
            pointer: None,
            line: Some(line),
            column: Some(column),
            detail: format!("the artifact is not in the canonical layout: {detail}"),
        }
    };
    const OPEN: &str = "{\n\"header\": ";
    const BETWEEN: &str = "\n},\n\"payload\": {\n";
    const CLOSE: &str = "\n}\n}\n";
    if !text.starts_with(OPEN) {
        return Err(layout("it does not open with its header", 0));
    }
    // Pretty-printed JSON closes its outermost object with `}` at column 1,
    // and no string holds a raw line feed, so the first such line ends it.
    let header_end = text[OPEN.len()..]
        .find(BETWEEN)
        .map(|at| OPEN.len() + at + 2)
        .ok_or_else(|| layout("no payload object follows the header", OPEN.len()))?;
    let header_text = &text[OPEN.len()..header_end];
    let mut deserializer = serde_json::Deserializer::from_str(header_text);
    let header: ArtifactHeader = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| prefixed(typed_refusal(error, bytes, OPEN.len()), "/header"))?;
    deserializer.end().map_err(|_| layout("content follows the header", header_end))?;
    let body_start = header_end - 2 + BETWEEN.len();
    if !text.ends_with(CLOSE) || text.len() < body_start + CLOSE.len() {
        return Err(layout("it does not close its payload and document on their own lines", text.len()));
    }
    let body = &text[body_start..text.len() - CLOSE.len()];
    let first_line = 1 + text[..body_start].matches('\n').count();
    let mut members: Vec<(String, &str, usize, usize)> = Vec::new();
    let lines: Vec<&str> = body.split('\n').collect();
    let mut offset = body_start;
    for (index, line) in lines.iter().enumerate() {
        let last = index + 1 == lines.len();
        let member = match (last, line.strip_suffix(',')) {
            (false, Some(member)) => member,
            (true, None) => line,
            (false, None) => return Err(layout("a payload member line does not end with `,`", offset + line.len())),
            (true, Some(_)) => return Err(layout("the last payload member is followed by `,`", offset + line.len() - 1)),
        };
        let mut keys = serde_json::Deserializer::from_str(member).into_iter::<String>();
        let key = match keys.next() {
            Some(Ok(key)) if member.starts_with('"') => key,
            _ => return Err(layout("a payload member does not start with its quoted key", offset)),
        };
        let after = keys.byte_offset();
        let Some(value) = member[after..].strip_prefix(':') else {
            return Err(layout("a payload member key is not followed by `:`", offset + after));
        };
        if members.iter().any(|(existing, ..)| *existing == key) {
            return Err(Refusal {
                pointer: Some(format!("/payload/{}", escape_segment(&key))),
                detail: format!("payload member `{key}` is repeated"),
                ..layout("", offset)
            });
        }
        members.push((key, value, first_line + index, after + 2));
        offset += line.len() + 1;
    }
    Ok(LaidOut { header, members })
}

/// `refusal` with `pointer` before its own pointer.
fn prefixed(mut refusal: Refusal, pointer: &str) -> Refusal {
    refusal.pointer = Some(format!("{pointer}{}", refusal.pointer.as_deref().unwrap_or("")));
    refusal
}

/// Decodes one payload section's text, whose JSON pointer in the artifact
/// is `pointer` and which starts at 1-based `line` and `column` of the
/// artifact: the duplicate-key walk over the section, then the typed decode.
pub(crate) fn decode_section<T: DeserializeOwned>(text: &str, pointer: &str, line: usize, column: usize) -> Result<T, Refusal> {
    let located = |refusal: Refusal| {
        let mut refusal = prefixed(refusal, pointer);
        // A section is one line: a position in it is a column offset.
        if let (Some(1), Some(inner)) = (refusal.line, refusal.column) {
            refusal.line = Some(line);
            refusal.column = Some(column + inner - 1);
        }
        refusal
    };
    check_document(text.as_bytes()).map_err(located)?;
    let mut deserializer = serde_json::Deserializer::from_str(text);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| located(typed_refusal(error, text.as_bytes(), 0)))
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
    check_text(bytes)?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut pointer = Vec::new();
    let checked = UniqueKeys { pointer: &mut pointer }.deserialize(&mut deserializer).and_then(|()| deserializer.end());
    checked.map_err(|error| Refusal {
        reason: FallbackReason::MalformedArtifact,
        pointer: (!pointer.is_empty() || error.classify() == serde_json::error::Category::Data)
            .then(|| pointer.iter().rev().map(|segment: &String| format!("/{segment}")).collect()),
        line: Some(error.line()),
        column: Some(error.column()),
        detail: strip_position(&error.to_string()),
    })
}

/// Refuses empty and non-UTF-8 bytes.
fn check_text(bytes: &[u8]) -> Result<(), Refusal> {
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
    Ok(())
}

/// Walks one JSON value, refusing an object that repeats a key. On a
/// refusal, `pointer` holds the path of the repeated key (or of the value
/// being read), innermost segment first: segments are added only while an
/// error unwinds, so a document that checks allocates no pointer.
struct UniqueKeys<'p> {
    pointer: &'p mut Vec<String>,
}

/// An object key, borrowed from the artifact unless it holds an escape.
struct Key<'de>(Cow<'de, str>);

impl<'de> Deserialize<'de> for Key<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Key<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an object key")
            }
            fn visit_borrowed_str<E>(self, key: &'de str) -> Result<Key<'de>, E> {
                Ok(Key(Cow::Borrowed(key)))
            }
            fn visit_str<E>(self, key: &str) -> Result<Key<'de>, E> {
                Ok(Key(Cow::Owned(key.to_string())))
            }
        }
        deserializer.deserialize_str(KeyVisitor)
    }
}

fn escape_segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
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
            match sequence.next_element_seed(UniqueKeys { pointer: self.pointer }) {
                Ok(Some(())) => index += 1,
                Ok(None) => return Ok(()),
                Err(error) => {
                    self.pointer.push(index.to_string());
                    return Err(error);
                }
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        // Artifact objects are small (record fields, one schema's
        // properties), so a linear scan beats hashing every key.
        let mut seen: Vec<Cow<'de, str>> = Vec::new();
        while let Some(Key(key)) = map.next_key::<Key<'de>>()? {
            if seen.contains(&key) {
                self.pointer.push(escape_segment(&key));
                return Err(serde::de::Error::custom("a key is repeated in one object"));
            }
            if let Err(error) = map.next_value_seed(UniqueKeys { pointer: self.pointer }) {
                self.pointer.push(escape_segment(&key));
                return Err(error);
            }
            seen.push(key);
        }
        Ok(())
    }
}
