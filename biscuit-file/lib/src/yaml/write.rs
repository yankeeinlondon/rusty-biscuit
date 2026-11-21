//! Laid-out YAML serialization.
//!
//! The backend (`serde_yaml_ng`) serializes the caller's value once, and that
//! text is parsed back into the value both the layout and the final check
//! work from. The layout walks that value's structure, never the text, so
//! text that merely resembles YAML syntax inside a scalar is content. Each
//! scalar keeps the spelling the backend gives it (quoting, block style and
//! chomping); only its indentation changes. The rendered text is parsed and
//! compared with the value before anything is returned.
//!
//! See `biscuit-file/docs/topics/yaml-writing.md` for the layout rules.

use serde::Serialize;
use serde_yaml_ng::Value;

use super::types::YamlError;
use crate::Indentation;

/// YAML's longest implicit (single-line, no `?`) mapping key, in characters.
const MAX_IMPLICIT_KEY_CHARS: usize = 1024;

/// Serializes `value` as YAML with block collections indented `indentation`
/// columns per level.
///
/// YAML forbids tab indentation, so [`IndentStyle::Tabs`](crate::IndentStyle)
/// writes `indentation.width` spaces per level. A caller with no destination
/// passes [`Indentation::default`] (four spaces).
///
/// The result is deterministic for a given value and width, parses to the
/// same YAML value as the backend's own output (tags, non-string keys, and
/// mapping order included), and keeps the backend's scalar spellings except
/// where a block scalar cannot express its content at the requested level.
///
/// # Errors
///
/// [`YamlError::Serialize`] when the backend cannot serialize `value`, and
/// [`YamlError::Layout`] when the laid-out text cannot be produced or does
/// not reparse to the backend's value. Either is returned before the caller
/// has text to write.
pub fn to_yaml_string<T: Serialize + ?Sized>(
    value: &T,
    indentation: Indentation,
) -> Result<String, YamlError> {
    let backend = serde_yaml_ng::to_string(value).map_err(YamlError::Serialize)?;
    let expected: Value = serde_yaml_ng::from_str(&backend)
        .map_err(|error| YamlError::Layout(format!("backend output does not reparse: {error}")))?;
    let tree = node(&expected)?;

    let mut renderer = Renderer {
        width: usize::from(indentation.width.get()),
        out: String::with_capacity(backend.len() + backend.len() / 2),
    };
    renderer.root(&tree);
    let rendered = renderer.out;

    let actual: Value = serde_yaml_ng::from_str(&rendered).map_err(|error| {
        YamlError::Layout(format!("laid-out YAML does not reparse: {error}"))
    })?;
    if !equivalent(&expected, &actual) {
        return Err(YamlError::Layout(
            "laid-out YAML does not parse to the serialized value".to_string(),
        ));
    }
    Ok(rendered)
}

/// One node of the document, with its scalars already spelled.
enum Node<'a> {
    Scalar { props: String, scalar: Scalar<'a> },
    Sequence { props: String, items: Vec<Node<'a>> },
    Mapping { props: String, entries: Vec<(Node<'a>, Node<'a>)> },
}

enum Scalar<'a> {
    /// The backend's single-line spelling, quotes included.
    Token(String),
    /// A string the backend writes as a literal block scalar.
    Block { content: &'a str, chomp: Chomp },
}

/// A literal block scalar's chomping indicator.
#[derive(Clone, Copy)]
enum Chomp {
    /// `-`: no final line break.
    Strip,
    /// No indicator: one final line break.
    Clip,
    /// `+`: every trailing line break.
    Keep,
}

fn node(value: &Value) -> Result<Node<'_>, YamlError> {
    Ok(match value {
        Value::Tagged(tagged) => {
            let tag = tagged.tag.to_string();
            match node(&tagged.value)? {
                Node::Scalar { props, scalar } => Node::Scalar { props: join(&tag, &props), scalar },
                Node::Sequence { props, items } => Node::Sequence { props: join(&tag, &props), items },
                Node::Mapping { props, entries } => {
                    Node::Mapping { props: join(&tag, &props), entries }
                }
            }
        }
        Value::Sequence(items) => Node::Sequence {
            props: String::new(),
            items: items.iter().map(node).collect::<Result<_, _>>()?,
        },
        Value::Mapping(entries) => Node::Mapping {
            props: String::new(),
            entries: entries
                .iter()
                .map(|(key, value)| Ok((node(key)?, node(value)?)))
                .collect::<Result<_, YamlError>>()?,
        },
        scalar => Node::Scalar { props: String::new(), scalar: spell(scalar)? },
    })
}

/// The backend's spelling of an untagged scalar, written on its own.
///
/// The backend picks a scalar's style from its content alone, so the
/// spelling it gives the scalar as a whole document is the one it gives it
/// anywhere in a block collection.
fn spell(scalar: &Value) -> Result<Scalar<'_>, YamlError> {
    let text = serde_yaml_ng::to_string(scalar).map_err(YamlError::Serialize)?;
    if let (Some(header), Value::String(content)) = (text.strip_prefix('|'), scalar) {
        let header = header.split('\n').next().unwrap_or_default();
        let chomp = match header.chars().last() {
            Some('-') => Chomp::Strip,
            Some('+') => Chomp::Keep,
            _ => Chomp::Clip,
        };
        return Ok(Scalar::Block { content, chomp });
    }
    let token = text.strip_suffix('\n').unwrap_or(&text);
    if token.contains('\n') {
        return match scalar {
            Value::String(content) => Ok(Scalar::Token(double_quoted(content))),
            _ => Err(YamlError::Layout(format!(
                "the backend spells a scalar over several lines: {text:?}"
            ))),
        };
    }
    Ok(Scalar::Token(token.to_string()))
}

struct Renderer {
    width: usize,
    out: String,
}

/// Where a node is written, relative to the indicator (`key:`, `-`, `?`,
/// `:`) it follows.
#[derive(Clone, Copy)]
struct Place {
    /// Write a space between the indicator and the node.
    separated: bool,
    /// The parent indentation a block scalar's indicator counts from; `None`
    /// at the document root.
    base: Option<usize>,
    /// The column of a block collection's children.
    children: usize,
    /// An untagged block collection starts on the indicator's line.
    compact: bool,
}

impl Renderer {
    fn root(&mut self, node: &Node<'_>) {
        self.node(
            node,
            Place {
                separated: false,
                base: None,
                children: 0,
                compact: true,
            },
        );
    }

    fn node(&mut self, node: &Node<'_>, place: Place) {
        match node {
            Node::Scalar { props, scalar } => match scalar {
                Scalar::Token(token) => self.inline(place, props, token),
                Scalar::Block { content, chomp } => self.block_scalar(place, props, content, *chomp),
            },
            Node::Sequence { props, items } => {
                if items.is_empty() {
                    self.inline(place, props, "[]");
                } else {
                    let first_inline = self.open_collection(place, props);
                    self.items(items, place.children, first_inline);
                }
            }
            Node::Mapping { props, entries } => {
                if entries.is_empty() {
                    self.inline(place, props, "{}");
                } else {
                    let first_inline = self.open_collection(place, props);
                    self.entries(entries, place.children, first_inline);
                }
            }
        }
    }

    /// Writes what precedes a non-empty block collection's first child and
    /// reports whether that child continues on the current line.
    fn open_collection(&mut self, place: Place, props: &str) -> bool {
        if props.is_empty() && place.compact {
            if place.separated {
                self.out.push(' ');
            }
            return true;
        }
        if !props.is_empty() {
            if place.separated {
                self.out.push(' ');
            }
            self.out.push_str(props);
        }
        self.out.push('\n');
        false
    }

    fn inline(&mut self, place: Place, props: &str, text: &str) {
        let text = join(props, text);
        if !text.is_empty() {
            if place.separated {
                self.out.push(' ');
            }
            self.out.push_str(&text);
        }
        self.out.push('\n');
    }

    fn block_scalar(&mut self, place: Place, props: &str, content: &str, chomp: Chomp) {
        let Some(block) = literal_block(content, chomp, place.base, self.width) else {
            self.inline(place, props, &double_quoted(content));
            return;
        };
        self.inline(place, props, &block.header);
        for line in block.body.split('\n') {
            if !line.is_empty() {
                self.indent(block.column);
                self.out.push_str(line);
            }
            self.out.push('\n');
        }
    }

    fn items(&mut self, items: &[Node<'_>], column: usize, first_inline: bool) {
        for (index, item) in items.iter().enumerate() {
            if index > 0 || !first_inline {
                self.indent(column);
            }
            self.out.push('-');
            self.node(item, Self::compact_child(column));
        }
    }

    fn entries(&mut self, entries: &[(Node<'_>, Node<'_>)], column: usize, first_inline: bool) {
        for (index, (key, value)) in entries.iter().enumerate() {
            if index > 0 || !first_inline {
                self.indent(column);
            }
            if let Some(key) = implicit_key(key) {
                self.out.push_str(&key);
                self.out.push(':');
                self.node(
                    value,
                    Place {
                        separated: true,
                        base: Some(column),
                        children: column + self.width,
                        compact: false,
                    },
                );
            } else {
                self.out.push('?');
                match key {
                    // A multiline key is always spelled double-quoted.
                    Node::Scalar {
                        props,
                        scalar: Scalar::Block { content, .. },
                    } => self.inline(Self::compact_child(column), props, &double_quoted(content)),
                    _ => self.node(key, Self::compact_child(column)),
                }
                self.indent(column);
                self.out.push(':');
                self.node(value, Self::compact_child(column));
            }
        }
    }

    /// A node after a `-`, `?`, or `:` indicator at `column`: compact
    /// collections align their children with the first one, two columns in,
    /// whatever the width.
    fn compact_child(column: usize) -> Place {
        Place {
            separated: true,
            base: Some(column),
            children: column + 2,
            compact: true,
        }
    }

    fn indent(&mut self, column: usize) {
        self.out.extend(std::iter::repeat_n(' ', column));
    }
}

/// The key's single-line spelling, or `None` when it needs the explicit `?`
/// form.
fn implicit_key(key: &Node<'_>) -> Option<String> {
    let Node::Scalar { props, scalar } = key else {
        return None;
    };
    let text = match scalar {
        Scalar::Token(token) => join(props, token),
        Scalar::Block { content, .. } => join(props, &double_quoted(content)),
    };
    (!text.is_empty() && text.chars().count() <= MAX_IMPLICIT_KEY_CHARS).then_some(text)
}

fn join(props: &str, text: &str) -> String {
    match (props.is_empty(), text.is_empty()) {
        (true, _) => text.to_string(),
        (false, true) => props.to_string(),
        (false, false) => format!("{props} {text}"),
    }
}

struct LiteralBlock<'a> {
    /// `|`, an indentation indicator when the first line starts with a
    /// space, and the chomping indicator.
    header: String,
    column: usize,
    /// The content lines, without the line break chomping supplies.
    body: &'a str,
}

/// The literal block spelling of `content` at the requested level, or `None`
/// when a block scalar cannot represent it there.
fn literal_block(
    content: &str,
    chomp: Chomp,
    base: Option<usize>,
    width: usize,
) -> Option<LiteralBlock<'_>> {
    if !content.chars().all(block_safe) {
        return None;
    }
    let body = chomped_body(content, chomp)?;
    // Content of only empty lines would be read back as trailing breaks.
    let first = body.split('\n').find(|line| !line.is_empty())?;
    let column = base.unwrap_or(0) + width;

    let mut header = String::from("|");
    if first.starts_with(' ') {
        // An indicator counts from the parent's indentation and is a single
        // digit; the root has no parent column to count from.
        base?;
        if width > 9 {
            return None;
        }
        header.push_str(&width.to_string());
    }
    match chomp {
        Chomp::Strip => header.push('-'),
        Chomp::Clip => {}
        Chomp::Keep => header.push('+'),
    }
    Some(LiteralBlock { header, column, body })
}

/// The content without the final line break chomping supplies, when `chomp`
/// can reproduce `content`'s trailing line breaks.
fn chomped_body(content: &str, chomp: Chomp) -> Option<&str> {
    match chomp {
        Chomp::Strip => (!content.ends_with('\n')).then_some(content),
        Chomp::Clip => content.strip_suffix('\n').filter(|body| !body.ends_with('\n')),
        Chomp::Keep => content.strip_suffix('\n'),
    }
}

/// A character a literal block scalar can carry as itself.
fn block_safe(character: char) -> bool {
    character == '\n'
        || (printable(character) && !matches!(character, '\r' | '\u{85}' | '\u{2028}' | '\u{2029}' | '\u{FEFF}'))
}

/// YAML 1.2's printable set (`c-printable`).
fn printable(character: char) -> bool {
    matches!(character,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{7E}' | '\u{85}' | '\u{A0}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

/// A YAML double-quoted string using only JSON-compatible escapes.
fn double_quoted(content: &str) -> String {
    let mut out = String::with_capacity(content.len() + 2);
    out.push('"');
    for character in content.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            // Every character above U+FFFF is printable, so `\u` suffices.
            character
                if !printable(character)
                    || matches!(character, '\u{85}' | '\u{2028}' | '\u{2029}' | '\u{FEFF}') =>
            {
                out.push_str(&format!("\\u{:04X}", u32::from(character)));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

/// YAML value equality that also requires the same mapping order, treating
/// NaN as equal to NaN.
fn equivalent(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Number(left), Value::Number(right)) => {
            (left.is_nan() && right.is_nan()) || left == right
        }
        (Value::Sequence(left), Value::Sequence(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left, right)| equivalent(left, right))
        }
        (Value::Mapping(left), Value::Mapping(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|((left_key, left_value), (right_key, right_value))| {
                    equivalent(left_key, right_key) && equivalent(left_value, right_value)
                })
        }
        (Value::Tagged(left), Value::Tagged(right)) => {
            left.tag == right.tag && equivalent(&left.value, &right.value)
        }
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> Value {
        serde_yaml_ng::from_str(text).unwrap()
    }

    #[test]
    fn equivalence_treats_nan_as_nan_and_requires_mapping_order() {
        assert!(equivalent(&number(".nan"), &number(".NaN")));
        assert!(!equivalent(&number(".nan"), &number(".inf")));
        let ab: Value = serde_yaml_ng::from_str("a: 1\nb: 2\n").unwrap();
        let ba: Value = serde_yaml_ng::from_str("b: 2\na: 1\n").unwrap();
        assert_eq!(ab, ba, "the backend's own equality ignores order");
        assert!(!equivalent(&ab, &ba));
    }

    #[test]
    fn equivalence_compares_tags_and_non_string_keys() {
        let tagged: Value = serde_yaml_ng::from_str("!A 1").unwrap();
        let other: Value = serde_yaml_ng::from_str("!B 1").unwrap();
        assert!(!equivalent(&tagged, &other));
        let int_key: Value = serde_yaml_ng::from_str("1: x").unwrap();
        let str_key: Value = serde_yaml_ng::from_str("'1': x").unwrap();
        assert!(!equivalent(&int_key, &str_key));
    }

    #[test]
    fn double_quoting_escapes_what_yaml_cannot_carry_raw() {
        assert_eq!(double_quoted("a\"b\\c\n\t"), r#""a\"b\\c\n\t""#);
        assert_eq!(double_quoted("\u{7F}\u{85}\u{2028}\u{FEFF}\u{1}"), r#""\u007F\u0085\u2028\uFEFF\u0001""#);
        assert_eq!(double_quoted("é😀"), "\"é😀\"");
    }
}
