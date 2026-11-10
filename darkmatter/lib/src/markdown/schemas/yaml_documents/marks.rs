//! Source marks for YAML refusals decided after the text has parsed.
//!
//! A second document, a non-string mapping key, and (for a data file) a tag
//! are refused once the value tree exists, and the tree has no positions. The
//! [`PositionMap`](simplified_schema::PositionMap) cannot fill the gap: it
//! refuses a layout it cannot match to the parsed value. These passes re-read the text with
//! `serde_yaml_ng`'s event deserializer instead, walking it in step with the
//! parsed value, and fail on purpose at the offending node; the deserializer
//! stamps that failure with the node's event mark. It follows aliases itself,
//! so the mark exists whether or not the file has one.
//!
//! Every offset is a byte offset into the text the caller passed.

use std::fmt;

use serde::de::{DeserializeSeed, Deserializer, EnumAccess, IgnoredAny, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde_yaml_ng::Value as YamlValue;

/// What a guided pass stops at, in document order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Offender {
    /// A mapping key that is not a string; the mark is the key's.
    NonStringKey,
    /// A tagged value (not a key); the mark is the tag's.
    Tag,
}

/// The `expecting` text of [`Stop`]. Every default `Visitor` method fails
/// with an "invalid type … expected {expecting}" error, so any node handed to
/// [`Stop`] fails with this text, at that node's mark.
const STOP: &str = "darkmatter: the node being located";

/// The byte offset of the first `offender` in `text`, which parsed as `value`.
pub(super) fn first_offender_start(text: &str, value: &YamlValue, offender: Offender) -> Option<usize> {
    let error = Guide { node: value, offender }.deserialize(serde_yaml_ng::Deserializer::from_str(text)).err()?;
    stop_offset(&error)
}

/// The byte offset where the second document of `text` starts: its `---`
/// marker when one introduces it, else its first node (or the parser's
/// error inside it). `None` for a stream of one document.
///
/// Only blank and comment lines may separate the marker from the node the
/// parser reports; any other text means the nearest `---` above belongs to an
/// earlier document (a second document can also follow a `...` end marker
/// with no `---` of its own), and the node itself is reported.
pub(super) fn second_document_start(text: &str) -> Option<usize> {
    let second = serde_yaml_ng::Deserializer::from_str(text).nth(1)?;
    // `Stop` fails at the document's first node, or the parser fails first
    // at a syntax error inside it; either way the mark is in that document.
    let node = second.deserialize_any(Stop).err()?.location()?.index();
    Some(document_marker_above(text, node).unwrap_or(node))
}

/// The start of the `---` line holding `node` or separated from it only by
/// blank and comment lines.
fn document_marker_above(text: &str, node: usize) -> Option<usize> {
    let node = node.min(text.len());
    let node_line = line_start(text, node);
    let mut start = node_line;
    loop {
        let line = text[start..].split('\n').next().unwrap_or_default();
        if is_directives_end(line) {
            return Some(start);
        }
        if start != node_line && !is_blank_or_comment(line) {
            return None;
        }
        if start == 0 {
            return None;
        }
        start = line_start(text, start - 1);
    }
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |index| index + 1)
}

/// A YAML directives-end marker: `---` at the start of a line, followed by
/// whitespace or the line's end.
fn is_directives_end(line: &str) -> bool {
    line.strip_prefix("---").is_some_and(|rest| rest.chars().next().is_none_or(|c| matches!(c, ' ' | '\t' | '\r')))
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']).trim_end_matches('\r');
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// The mark of a failure [`Stop`] raised; `None` for any other failure,
/// whose position would not be the located node's.
fn stop_offset(error: &serde_yaml_ng::Error) -> Option<usize> {
    if !error.to_string().contains(STOP) {
        return None;
    }
    error.location().map(|location| location.index())
}

/// Fails on whatever node it is given (see [`STOP`]).
struct Stop;

impl<'de> Visitor<'de> for Stop {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(STOP)
    }
}

/// Walks the event stream in step with the parsed value `node`.
struct Guide<'a> {
    node: &'a YamlValue,
    offender: Offender,
}

impl<'de> DeserializeSeed<'de> for Guide<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        let offender = self.offender;
        match self.node {
            YamlValue::Tagged(_) if offender == Offender::Tag => deserializer.deserialize_any(Stop),
            // A tagged node reaches a visitor as an enum: the tag, then the value.
            YamlValue::Tagged(tagged) => deserializer.deserialize_any(TaggedGuide { inner: &tagged.value, offender }),
            YamlValue::Mapping(mapping) => deserializer.deserialize_map(MapGuide { mapping, offender }),
            YamlValue::Sequence(items) => deserializer.deserialize_seq(SeqGuide { items, offender }),
            _ => deserializer.deserialize_ignored_any(IgnoredAny).map(|_| ()),
        }
    }
}

struct KeyGuide<'a> {
    key: &'a YamlValue,
    offender: Offender,
}

impl<'de> DeserializeSeed<'de> for KeyGuide<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        if self.offender == Offender::NonStringKey && !self.key.is_string() {
            return deserializer.deserialize_any(Stop);
        }
        deserializer.deserialize_ignored_any(IgnoredAny).map(|_| ())
    }
}

struct MapGuide<'a> {
    mapping: &'a serde_yaml_ng::Mapping,
    offender: Offender,
}

impl<'de> Visitor<'de> for MapGuide<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a mapping")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let offender = self.offender;
        for (key, value) in self.mapping {
            if map.next_key_seed(KeyGuide { key, offender })?.is_none() {
                return Ok(());
            }
            map.next_value_seed(Guide { node: value, offender })?;
        }
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(())
    }
}

struct SeqGuide<'a> {
    items: &'a [YamlValue],
    offender: Offender,
}

impl<'de> Visitor<'de> for SeqGuide<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        for item in self.items {
            if seq.next_element_seed(Guide { node: item, offender: self.offender })?.is_none() {
                return Ok(());
            }
        }
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(())
    }
}

struct TaggedGuide<'a> {
    inner: &'a YamlValue,
    offender: Offender,
}

impl<'de> Visitor<'de> for TaggedGuide<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a tagged value")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<(), A::Error> {
        let (IgnoredAny, variant) = data.variant()?;
        variant.newtype_variant_seed(Guide { node: self.inner, offender: self.offender })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offender(text: &str, offender: Offender) -> Option<(usize, usize)> {
        let value: YamlValue = serde_yaml_ng::from_str(text).expect("parses");
        first_offender_start(text, &value, offender).and_then(|offset| biscuit_file::source_position(text, offset))
    }

    fn second(text: &str) -> Option<(usize, usize)> {
        second_document_start(text).and_then(|offset| biscuit_file::source_position(text, offset))
    }

    #[test]
    fn a_non_string_key_is_marked_at_the_key_in_document_order() {
        assert_eq!(offender("42: text\n", Offender::NonStringKey), Some((1, 1)));
        assert_eq!(offender("a:\n  b: 1\n  true: x\nc: 3\n", Offender::NonStringKey), Some((3, 3)));
        assert_eq!(offender("a:\n  ? [a, b]\n  : x\n", Offender::NonStringKey), Some((2, 5)));
        assert_eq!(offender("base: &b ok\na: *b\n42: text\n", Offender::NonStringKey), Some((3, 1)));
        assert_eq!(offender("a: 1\n", Offender::NonStringKey), None);
    }

    #[test]
    fn a_tag_is_marked_at_the_tag_even_after_an_alias() {
        assert_eq!(offender("base: &b ok\na: *b\nb: !tag text\n", Offender::Tag), Some((3, 4)));
        assert_eq!(offender("a: [1, {b: !t x}]\n", Offender::Tag), Some((1, 12)));
        assert_eq!(offender("!t {a: 1}\n", Offender::Tag), Some((1, 1)));
        assert_eq!(offender("a: 1\n", Offender::Tag), None);
    }

    #[test]
    fn a_second_document_is_marked_at_its_marker_or_its_first_node() {
        assert_eq!(second("a: ok\n---\na: next\n"), Some((2, 1)));
        assert_eq!(second("---\na: ok\n--- # two\n\n# note\na: next\n"), Some((3, 1)));
        assert_eq!(second("a: ok\n--- text\n"), Some((2, 1)));
        assert_eq!(second("---\na: 1\n...\nb: 2\n"), Some((4, 1)));
        assert_eq!(second("a: ok\r\n---\r\na: next\r\n"), Some((2, 1)));
        assert_eq!(second("a: ok\n"), None);
    }
}
