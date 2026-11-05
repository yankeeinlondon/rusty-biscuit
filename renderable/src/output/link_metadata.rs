//! Structured link titles and the query metadata they carry.
//!
//! A Markdown link title that is a JSON5 object is structured data; any
//! other title is a plain string, as in CommonMark:
//!
//! ```md
//! [Plain](./a.md "Shown as a tooltip")
//! [Located](./a.md '{ class: "docs-link", qp: { line: 14, column: 83 } }')
//! ```
//!
//! [`parse_link_title`] is the one interpretation of a title. Prose and
//! Darkmatter both call it, so a title never means two things. It reports,
//! without printing, the two notices a caller turns into warnings:
//!
//! | Title | Result | Notice |
//! |---|---|---|
//! | a JSON5 object | [`LinkTitle::Structured`] | [`LinkTitleNotice::Malformed`] only for an ignored value |
//! | starts with `{` but is not a JSON5 object | [`LinkTitle::Plain`] | [`LinkTitleNotice::Malformed`] |
//! | a leftover `key=value` title | [`LinkTitle::Plain`] | [`LinkTitleNotice::Legacy`] |
//! | anything else | [`LinkTitle::Plain`] | none |
//!
//! `qp` (query parameters) is the one new key. Its entries become HTML query
//! parameters and OSC 8 parameters; [`LinkMetadata`] is the typed form a
//! render-tree link node carries.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

/// The warning code for a title that starts with `{` but is not a valid
/// structured title, or whose structured title had a value that was ignored.
pub const MALFORMED_STRUCTURED_TITLE: &str = "dm.link.malformed_structured_title";

/// The warning code for a leftover `key=value` structured title.
pub const LEGACY_STRUCTURED_TITLE: &str = "dm.link.legacy_structured_title";

/// One query-parameter value: a string, a finite number, or a boolean.
///
/// The original scalar is retained so a Markdown round trip writes `14`, not
/// `"14"`; [`QueryValue::to_transport`] gives the string form HTML query
/// strings and OSC 8 parameters carry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryValue {
    /// A boolean value.
    Bool(bool),
    /// A finite number.
    Number(serde_json::Number),
    /// A string value.
    String(String),
}

impl QueryValue {
    /// The value as query-string / OSC 8 parameter text.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::link_metadata::QueryValue;
    ///
    /// assert_eq!(QueryValue::Number(14.into()).to_transport(), "14");
    /// assert_eq!(QueryValue::Bool(true).to_transport(), "true");
    /// ```
    #[must_use]
    pub fn to_transport(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::String(value) => value.clone(),
        }
    }

    fn write_json5(&self, out: &mut String) {
        match self {
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Number(value) => out.push_str(&value.to_string()),
            Self::String(value) => write_json5_string(value, out),
        }
    }
}

impl From<&str> for QueryValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for QueryValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<u64> for QueryValue {
    fn from(value: u64) -> Self {
        Self::Number(value.into())
    }
}

impl From<bool> for QueryValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

/// Query parameters in authored order, each key once.
///
/// Inserting an existing key replaces its value in place, so the last
/// authored value wins and the first authored position is kept.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QueryParams(Vec<(String, QueryValue)>);

impl QueryParams {
    /// An empty parameter list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `key` to `value`, replacing an existing value in place.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<QueryValue>) {
        let key = key.into();
        let value = value.into();
        match self.0.iter_mut().find(|(existing, _)| *existing == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key, value)),
        }
    }

    /// The value for `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&QueryValue> {
        self.0.iter().find(|(existing, _)| existing == key).map(|(_, value)| value)
    }

    /// The parameters in authored order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &QueryValue)> {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// Whether there are no parameters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<K: Into<String>, V: Into<QueryValue>> FromIterator<(K, V)> for QueryParams {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut params = Self::new();
        for (key, value) in iter {
            params.insert(key, value);
        }
        params
    }
}

/// Typed transport metadata a render-tree link node carries
/// ([`NodeAttrs::link_metadata`](crate::tree::NodeAttrs::link_metadata)).
///
/// Each fold lowers it in its own form: the Markdown fold writes a JSON5
/// `qp` title, the browser fold writes query parameters (and the fragment as
/// `heading=`), and the terminal fold writes OSC 8 parameters.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LinkMetadata {
    /// The link's query parameters (`qp`).
    #[serde(default, skip_serializing_if = "QueryParams::is_empty")]
    pub query: QueryParams,
}

impl LinkMetadata {
    /// Whether the metadata carries nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }
}

/// The properties of a structured (JSON5 object) title.
///
/// The string properties keep the meaning they had under the old
/// `key=value` syntax; their values are raw, and each consumer validates
/// them (a CSS `style`, a `target` keyword) as before. Unknown keys are
/// ignored.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StructuredTitle {
    /// The tooltip text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// CSS classes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// Inline CSS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// The browsing context (`_blank`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// A prompt attached to the link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// `data-*` properties, keyed by their full lowercase name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, String>,
    /// Query parameters (`qp`).
    #[serde(default, skip_serializing_if = "QueryParams::is_empty")]
    pub query: QueryParams,
}

impl StructuredTitle {
    /// Whether no property is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.class.is_none()
            && self.style.is_none()
            && self.target.is_none()
            && self.prompt.is_none()
            && self.data.is_empty()
            && self.query.is_empty()
    }

    /// The transport metadata this title carries.
    #[must_use]
    pub fn link_metadata(&self) -> LinkMetadata {
        LinkMetadata {
            query: self.query.clone(),
        }
    }

    /// The title as a JSON5 object, the form the Markdown outputs write.
    ///
    /// Keys are written in a fixed order (`title`, `class`, `style`,
    /// `target`, `prompt`, `data-*` by name, then `qp`); `qp` keeps its
    /// authored order. [`parse_link_title`] reads the result back to an
    /// equal value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::link_metadata::{StructuredTitle, QueryParams};
    ///
    /// let title = StructuredTitle {
    ///     class: Some("docs-link".into()),
    ///     query: QueryParams::from_iter([("line", 14_u64), ("column", 83)]),
    ///     ..StructuredTitle::default()
    /// };
    /// assert_eq!(
    ///     title.to_json5(),
    ///     r#"{ class: "docs-link", qp: { line: 14, column: 83 } }"#,
    /// );
    /// ```
    #[must_use]
    pub fn to_json5(&self) -> String {
        let mut entries: Vec<(&str, String)> = Vec::new();
        let named = [
            ("title", &self.title),
            ("class", &self.class),
            ("style", &self.style),
            ("target", &self.target),
            ("prompt", &self.prompt),
        ];
        for (key, value) in named {
            if let Some(value) = value {
                let mut text = String::new();
                write_json5_string(value, &mut text);
                entries.push((key, text));
            }
        }
        for (key, value) in &self.data {
            let mut text = String::new();
            write_json5_string(value, &mut text);
            entries.push((key.as_str(), text));
        }
        if !self.query.is_empty() {
            let mut text = String::new();
            write_query_object(&self.query, &mut text);
            entries.push(("qp", text));
        }
        let mut out = String::from("{");
        for (index, (key, value)) in entries.iter().enumerate() {
            out.push_str(if index == 0 { " " } else { ", " });
            write_json5_key(key, &mut out);
            out.push_str(": ");
            out.push_str(value);
        }
        out.push_str(if entries.is_empty() { "}" } else { " }" });
        out
    }

    /// [`Self::to_json5`] as a Markdown link title, delimiters included: a
    /// reader unescapes it back to exactly that JSON5 text.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::link_metadata::StructuredTitle;
    ///
    /// let title = StructuredTitle { target: Some("_blank".into()), ..StructuredTitle::default() };
    /// assert_eq!(title.to_markdown_title(), r#"'{ target: "_blank" }'"#);
    /// ```
    #[must_use]
    pub fn to_markdown_title(&self) -> String {
        format!("'{}'", crate::tree::render::markdown::escape_link_title(&self.to_json5(), '\''))
    }
}

/// The `href` of a link carrying `metadata`: its `#fragment` and `qp` entries
/// move into the query string.
///
/// The destination's own query pairs come first, in order, except a key the
/// metadata sets (the metadata wins). Then come `heading` (the fragment,
/// unless `qp` sets `heading` itself), `line`, `column`, and the remaining
/// `qp` keys sorted by name. The path is never rewritten and the result has
/// no fragment.
///
/// ## Examples
///
/// ```
/// use renderable::output::link_metadata::{query_href, LinkMetadata, QueryParams};
///
/// let metadata = LinkMetadata { query: QueryParams::from_iter([("line", 14_u64)]) };
/// assert_eq!(query_href("./a.md?x=1#setup", &metadata), "./a.md?x=1&heading=setup&line=14");
/// ```
#[must_use]
pub fn query_href(url: &str, metadata: &LinkMetadata) -> String {
    let (rest, fragment) = match url.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (url, None),
    };
    let (path, query) = match rest.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (rest, None),
    };
    let query_params = &metadata.query;
    let heading = fragment
        .filter(|fragment| !fragment.is_empty() && query_params.get("heading").is_none())
        .map(percent_decode);
    let sets =
        |key: &str| query_params.get(key).is_some() || (key == "heading" && heading.is_some());

    let mut kept: Vec<&str> = Vec::new();
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
    {
        let key = url::form_urlencoded::parse(pair.as_bytes())
            .next()
            .map(|(key, _)| key.into_owned())
            .unwrap_or_default();
        if !sets(&key) {
            kept.push(pair);
        }
    }

    let mut added = url::form_urlencoded::Serializer::new(String::new());
    if let Some(heading) = &heading {
        added.append_pair("heading", heading);
    }
    let mut ordered: Vec<(&str, String)> = query_params
        .iter()
        .map(|(key, value)| (key, value.to_transport()))
        .collect();
    let rank = |key: &str| match key {
        "heading" => 0,
        "line" => 1,
        "column" => 2,
        _ => 3,
    };
    // A stable sort, so `heading`, `line`, and `column` need no tiebreak.
    ordered.sort_by_key(|(key, _)| (rank(key), if rank(key) == 3 { *key } else { "" }));
    for (key, value) in &ordered {
        added.append_pair(key, value);
    }
    let added = added.finish();

    let mut href = String::from(path);
    let mut separator = '?';
    for pair in kept
        .iter()
        .copied()
        .chain((!added.is_empty()).then_some(added.as_str()))
    {
        href.push(separator);
        href.push_str(pair);
        separator = '&';
    }
    href
}

/// Decodes `%XX` escapes in a URL fragment; an invalid escape stays as
/// written, and invalid UTF-8 is replaced.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = |at: usize| bytes.get(at).and_then(|b| (*b as char).to_digit(16));
        if bytes[index] == b'%'
            && let (Some(high), Some(low)) = (hex(index + 1), hex(index + 2))
        {
            out.push((high * 16 + low) as u8);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// How a link title was interpreted.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTitle {
    /// A plain string title (a CommonMark tooltip).
    Plain(String),
    /// A JSON5 object title.
    Structured(StructuredTitle),
}

/// A problem found while interpreting a title. It is reported, never
/// printed: the caller decides how to deliver it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTitleNotice {
    /// The title starts with `{` but is not a JSON5 object, or a structured
    /// title had a value that was ignored. `reason` says which.
    Malformed {
        /// What was wrong, in plain words.
        reason: String,
    },
    /// The title uses the removed `key=value` syntax; `md clean` migrates it.
    Legacy,
}

impl LinkTitleNotice {
    /// The warning code for this notice.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => MALFORMED_STRUCTURED_TITLE,
            Self::Legacy => LEGACY_STRUCTURED_TITLE,
        }
    }
}

/// A title's interpretation plus the notice it raised, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLinkTitle {
    /// The interpreted title.
    pub title: LinkTitle,
    /// The notice to report, if any.
    pub notice: Option<LinkTitleNotice>,
}

impl ParsedLinkTitle {
    /// The structured title, when the title was a JSON5 object.
    #[must_use]
    pub fn structured(&self) -> Option<&StructuredTitle> {
        match &self.title {
            LinkTitle::Structured(structured) => Some(structured),
            LinkTitle::Plain(_) => None,
        }
    }

    /// The tooltip text: the plain title, or a structured title's `title`.
    #[must_use]
    pub fn tooltip(&self) -> Option<&str> {
        match &self.title {
            LinkTitle::Plain(text) => Some(text.as_str()),
            LinkTitle::Structured(structured) => structured.title.as_deref(),
        }
    }
}

/// Interprets a link title (the text between the title delimiters, already
/// unescaped).
///
/// ## Examples
///
/// ```
/// use renderable::output::link_metadata::{parse_link_title, LinkTitle, LinkTitleNotice};
///
/// let parsed = parse_link_title("{ target: '_blank', qp: { line: 14 } }");
/// let structured = parsed.structured().unwrap();
/// assert_eq!(structured.target.as_deref(), Some("_blank"));
/// assert_eq!(structured.query.get("line").unwrap().to_transport(), "14");
///
/// let legacy = parse_link_title("class='docs' target='_blank'");
/// assert!(matches!(legacy.title, LinkTitle::Plain(_)));
/// assert_eq!(legacy.notice, Some(LinkTitleNotice::Legacy));
/// ```
#[must_use]
pub fn parse_link_title(raw: &str) -> ParsedLinkTitle {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        return match json_five::from_str::<OrderedJson>(trimmed) {
            Ok(OrderedJson::Object(entries)) => structured_from_entries(entries),
            Ok(_) | Err(_) => ParsedLinkTitle {
                title: LinkTitle::Plain(raw.to_owned()),
                notice: Some(LinkTitleNotice::Malformed {
                    reason: "the title starts with `{` but is not a JSON5 object".to_owned(),
                }),
            },
        };
    }
    let notice = is_legacy_structured_title(raw).then_some(LinkTitleNotice::Legacy);
    ParsedLinkTitle {
        title: LinkTitle::Plain(raw.to_owned()),
        notice,
    }
}

/// Whether a title uses the removed `key=value` structured syntax: an
/// unquoted `=` directly after an identifier-like key.
///
/// It is kept only so a document that was not migrated is warned about and
/// so `md clean` can find titles to rewrite; nothing interprets such a title
/// as attributes.
#[must_use]
pub fn is_legacy_structured_title(content: &str) -> bool {
    let content = content.trim();
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in content.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '"' | '\'') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            }
        } else if ch == '=' && quote.is_none() {
            let key = content[..index]
                .trim()
                .split([' ', ','])
                .next_back()
                .unwrap_or("");
            if !key.is_empty()
                && key
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_'))
            {
                return true;
            }
        }
    }
    false
}

fn structured_from_entries(entries: Vec<(String, OrderedJson)>) -> ParsedLinkTitle {
    let mut structured = StructuredTitle::default();
    let mut ignored: Vec<String> = Vec::new();
    for (key, value) in entries {
        let lowered = key.to_ascii_lowercase();
        if lowered == "qp" {
            match value {
                OrderedJson::Object(params) => {
                    for (name, value) in params {
                        match value.into_query_value() {
                            Some(value) => structured.query.insert(name, value),
                            None => ignored.push(format!("qp.{name}")),
                        }
                    }
                }
                _ => ignored.push("qp".to_owned()),
            }
            continue;
        }
        let slot = match lowered.as_str() {
            "title" => Some(&mut structured.title),
            "class" => Some(&mut structured.class),
            "style" => Some(&mut structured.style),
            "target" => Some(&mut structured.target),
            "prompt" => Some(&mut structured.prompt),
            _ => None,
        };
        let is_data = lowered.starts_with("data-");
        if slot.is_none() && !is_data {
            continue;
        }
        let Some(text) = value.into_property_string() else {
            ignored.push(key);
            continue;
        };
        match slot {
            Some(slot) => *slot = Some(text),
            None => {
                structured.data.insert(lowered, text);
            }
        }
    }
    let notice = (!ignored.is_empty()).then(|| LinkTitleNotice::Malformed {
        reason: format!(
            "ignored {} with an unsupported value: {}",
            if ignored.len() == 1 { "a property" } else { "properties" },
            ignored.join(", ")
        ),
    });
    ParsedLinkTitle {
        title: LinkTitle::Structured(structured),
        notice,
    }
}

fn write_query_object(query: &QueryParams, out: &mut String) {
    out.push('{');
    for (index, (key, value)) in query.iter().enumerate() {
        out.push_str(if index == 0 { " " } else { ", " });
        write_json5_key(key, out);
        out.push_str(": ");
        value.write_json5(out);
    }
    out.push_str(if query.is_empty() { "}" } else { " }" });
}

fn write_json5_key(key: &str, out: &mut String) {
    let mut chars = key.chars();
    let is_identifier = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || matches!(first, '_' | '$'))
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$'));
    if is_identifier {
        out.push_str(key);
    } else {
        write_json5_string(key, out);
    }
}

fn write_json5_string(value: &str, out: &mut String) {
    // JSON string syntax is valid JSON5, and serde_json escapes every control
    // character, so the result never needs a second pass.
    out.push_str(&serde_json::to_string(value).expect("a string always serializes"));
}

/// A JSON5 value whose objects keep their authored key order, independent of
/// whether serde_json's `preserve_order` feature is unified into the build.
enum OrderedJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    NonFinite,
    String(String),
    Array,
    Object(Vec<(String, OrderedJson)>),
}

impl OrderedJson {
    fn into_query_value(self) -> Option<QueryValue> {
        match self {
            Self::Bool(value) => Some(QueryValue::Bool(value)),
            Self::Number(value) => Some(QueryValue::Number(value)),
            Self::String(value) => Some(QueryValue::String(value)),
            Self::Null | Self::NonFinite | Self::Array | Self::Object(_) => None,
        }
    }

    fn into_property_string(self) -> Option<String> {
        self.into_query_value().map(|value| value.to_transport())
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

struct OrderedJsonVisitor;

impl<'de> Visitor<'de> for OrderedJsonVisitor {
    type Value = OrderedJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON5 value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<OrderedJson, D::Error> {
        OrderedJson::deserialize(deserializer)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Number(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<OrderedJson, E> {
        Ok(serde_json::Number::from_f64(value).map_or(OrderedJson::NonFinite, OrderedJson::Number))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<OrderedJson, E> {
        Ok(OrderedJson::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<OrderedJson, E> {
        Ok(OrderedJson::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OrderedJson, A::Error> {
        while seq.next_element::<OrderedJson>()?.is_some() {}
        Ok(OrderedJson::Array)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<OrderedJson, A::Error> {
        let mut entries: Vec<(String, OrderedJson)> = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, OrderedJson>()? {
            // A repeated key keeps its first position and its last value,
            // matching `QueryParams::insert`.
            match entries.iter_mut().find(|(existing, _)| *existing == key) {
                Some(slot) => slot.1 = value,
                None => entries.push((key, value)),
            }
        }
        Ok(OrderedJson::Object(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structured(raw: &str) -> (StructuredTitle, Option<LinkTitleNotice>) {
        let parsed = parse_link_title(raw);
        match parsed.title {
            LinkTitle::Structured(structured) => (structured, parsed.notice),
            LinkTitle::Plain(text) => panic!("expected structured, got plain {text:?}"),
        }
    }

    #[test]
    fn a_plain_title_is_plain_without_notice() {
        let parsed = parse_link_title("Shown as a tooltip");
        assert_eq!(parsed.title, LinkTitle::Plain("Shown as a tooltip".into()));
        assert_eq!(parsed.notice, None);
        assert_eq!(parsed.tooltip(), Some("Shown as a tooltip"));
    }

    #[test]
    fn a_json5_object_is_structured_with_supported_properties() {
        let (title, notice) = structured(
            r#"{ title: 'Tip', class: "docs-link", style: "color: red", target: "_blank", prompt: "explain", "data-section": "help", qp: { line: 14, column: 83 } }"#,
        );
        assert_eq!(notice, None);
        assert_eq!(title.title.as_deref(), Some("Tip"));
        assert_eq!(title.class.as_deref(), Some("docs-link"));
        assert_eq!(title.style.as_deref(), Some("color: red"));
        assert_eq!(title.target.as_deref(), Some("_blank"));
        assert_eq!(title.prompt.as_deref(), Some("explain"));
        assert_eq!(title.data.get("data-section").map(String::as_str), Some("help"));
        let keys: Vec<&str> = title.query.iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["line", "column"], "qp keeps authored order");
    }

    #[test]
    fn a_strict_json_object_is_structured_too() {
        let (title, notice) = structured(r#"{"class": "a", "qp": {"line": 3}}"#);
        assert_eq!(notice, None);
        assert_eq!(title.class.as_deref(), Some("a"));
        assert_eq!(title.query.get("line"), Some(&QueryValue::from(3_u64)));
    }

    #[test]
    fn unknown_keys_are_ignored_silently() {
        let (title, notice) = structured("{ class: 'a', colour: 'red', nested: { x: 1 } }");
        assert_eq!(notice, None);
        assert_eq!(title.class.as_deref(), Some("a"));
        assert!(title.data.is_empty());
    }

    #[test]
    fn qp_scalars_normalize_for_transport_and_keep_their_type() {
        let (title, notice) = structured("{ qp: { s: 'x', n: 1.5, i: -2, b: false } }");
        assert_eq!(notice, None);
        let transported: Vec<(String, String)> = title
            .query
            .iter()
            .map(|(key, value)| (key.to_owned(), value.to_transport()))
            .collect();
        assert_eq!(
            transported,
            [
                ("s".to_owned(), "x".to_owned()),
                ("n".to_owned(), "1.5".to_owned()),
                ("i".to_owned(), "-2".to_owned()),
                ("b".to_owned(), "false".to_owned()),
            ]
        );
        assert_eq!(title.to_json5(), r#"{ qp: { s: "x", n: 1.5, i: -2, b: false } }"#);
    }

    #[test]
    fn unsupported_qp_entries_are_dropped_with_a_malformed_notice() {
        let (title, notice) =
            structured("{ qp: { line: 3, a: null, b: [1], c: { d: 1 }, e: Infinity } }");
        assert_eq!(title.query.len(), 1);
        assert_eq!(title.query.get("line"), Some(&QueryValue::from(3_u64)));
        let Some(LinkTitleNotice::Malformed { reason }) = notice else {
            panic!("expected a malformed notice, got {notice:?}");
        };
        for name in ["qp.a", "qp.b", "qp.c", "qp.e"] {
            assert!(reason.contains(name), "{reason} names {name}");
        }
    }

    #[test]
    fn a_non_object_qp_is_ignored_with_a_malformed_notice() {
        for raw in ["{ qp: 3 }", "{ qp: 'line=3' }", "{ qp: [1] }", "{ qp: null }"] {
            let (title, notice) = structured(raw);
            assert!(title.query.is_empty(), "{raw}");
            assert_eq!(notice.as_ref().map(LinkTitleNotice::code), Some(MALFORMED_STRUCTURED_TITLE), "{raw}");
        }
    }

    #[test]
    fn supported_properties_keep_their_string_interpretation() {
        let (title, notice) = structured("{ title: 42, class: true }");
        assert_eq!(notice, None);
        assert_eq!(title.title.as_deref(), Some("42"));
        assert_eq!(title.class.as_deref(), Some("true"));
        let (title, notice) = structured("{ class: ['a'], title: 'kept' }");
        assert_eq!(title.class, None);
        assert_eq!(title.title.as_deref(), Some("kept"));
        assert!(matches!(notice, Some(LinkTitleNotice::Malformed { .. })));
    }

    #[test]
    fn a_brace_that_does_not_parse_is_plain_with_a_malformed_notice() {
        for raw in ["{ class: ", "{not json}", "{ a: 1 } trailing", "{}}"] {
            let parsed = parse_link_title(raw);
            assert_eq!(parsed.title, LinkTitle::Plain(raw.to_owned()), "{raw}");
            assert_eq!(
                parsed.notice.as_ref().map(LinkTitleNotice::code),
                Some(MALFORMED_STRUCTURED_TITLE),
                "{raw}"
            );
        }
    }

    #[test]
    fn a_non_object_json5_title_is_plain_text() {
        for raw in ["42", "[1, 2]", "\"quoted\"", "true"] {
            let parsed = parse_link_title(raw);
            assert_eq!(parsed.title, LinkTitle::Plain(raw.to_owned()), "{raw}");
            assert_eq!(parsed.notice, None, "{raw}");
        }
    }

    #[test]
    fn a_legacy_key_value_title_is_plain_with_a_legacy_notice() {
        let raw = "class='docs-link' target='_blank' data-section='help'";
        let parsed = parse_link_title(raw);
        assert_eq!(parsed.title, LinkTitle::Plain(raw.to_owned()));
        assert_eq!(parsed.notice, Some(LinkTitleNotice::Legacy));
        assert_eq!(LinkTitleNotice::Legacy.code(), LEGACY_STRUCTURED_TITLE);
    }

    #[test]
    fn an_equals_sign_inside_quotes_or_after_punctuation_is_not_legacy() {
        // The detector is today's `key=value` rule unchanged: only an unquoted
        // `=` after an identifier-like word counts.
        for raw in ["see 'x=y' quoted", "\"a=b\"", "-> = arrow", "no assignment"] {
            assert_eq!(parse_link_title(raw).notice, None, "{raw}");
        }
        assert_eq!(parse_link_title("a = b").notice, Some(LinkTitleNotice::Legacy));
    }

    #[test]
    fn json5_round_trip_is_lossless() {
        let raw = r#"{ title: "Say \"hi\"\n", class: "docs-link", target: "_blank", "data-section": "help", qp: { line: 14, column: 83, "a b": "c:d", ok: true } }"#;
        let (title, _) = structured(raw);
        let written = title.to_json5();
        let (reparsed, notice) = structured(&written);
        assert_eq!(notice, None);
        assert_eq!(reparsed, title);
        assert_eq!(reparsed.to_json5(), written, "a second write is unchanged");
    }

    #[test]
    fn a_repeated_qp_key_keeps_its_first_position_and_last_value() {
        let (title, _) = structured("{ qp: { line: 1, column: 2, line: 3 } }");
        assert_eq!(title.to_json5(), "{ qp: { line: 3, column: 2 } }");
    }

    #[test]
    fn an_empty_object_is_an_empty_structured_title() {
        let (title, notice) = structured("{}");
        assert!(title.is_empty());
        assert_eq!(notice, None);
        assert_eq!(title.to_json5(), "{}");
    }

    #[test]
    fn link_metadata_serializes_its_query_in_order() {
        let metadata = LinkMetadata {
            query: QueryParams::from_iter([("line", 14_u64), ("column", 83)]),
        };
        let json = serde_json::to_string(&metadata).unwrap();
        assert_eq!(json, r#"{"query":[["line",14],["column",83]]}"#);
        let back: LinkMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, metadata);
        assert_eq!(serde_json::to_string(&LinkMetadata::default()).unwrap(), "{}");
    }
}
