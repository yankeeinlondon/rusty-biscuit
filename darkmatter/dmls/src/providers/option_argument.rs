//! Completion inside an object-literal argument whose catalog parameter is an
//! inline object, such as `load_data`'s `options`:
//! `{{ load_data("rows.txt", "string[]", {format: "csv"}) }}`.
//!
//! The keys and values offered come from the catalog parameter's declared
//! shape, never from a list kept here: the declared property names at a key
//! position (minus those already written), the quoted members of an `enum`
//! property, and `true`/`false` for a `boolean` one. Any other value position
//! is left to ordinary expression completion. A parameter typed plain
//! `object` declares no keys and is not claimed.
//!
//! The text is usually unfinished, so the call and object holding the cursor
//! are recovered from the expression lexer's tokens up to the cursor rather
//! than from a parse. Passive: nothing is read or evaluated.

use darkmatter::markdown::compose::expression::{Token, identifier_prefix_start, lex_spanned};
use lsp_types::{CompletionItem, CompletionItemKind};
use simplified_schema::expression::scan::expression_region_at;
use simplified_schema::{Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedType, TypeExpr};

use super::DocumentContext;
use super::dsl::{CompletionCandidate, text_edit_item};
use super::type_definition_argument::{ExpressionText, Site, expression_text};
use crate::overlay::expressions;

/// Option completion at `offset` when it sits in a declared-shape object
/// argument at `site`. `Some` claims the cursor even when nothing matches, so
/// the caller offers no expression candidates at a key position; `None` at a
/// value position the shape does not enumerate, so ordinary completion runs.
pub(crate) fn completion(ctx: &DocumentContext, offset: usize, site: Site) -> Option<Vec<CompletionItem>> {
    let ExpressionText { analyzed, from, cursor, shift } = expression_text(ctx, offset, site)?;
    let text: &str = &analyzed.text;
    let region = expression_region_at(text, cursor, &[]).filter(|region| region.start >= from)?;
    let found = option_cursor(text, region.start + 2, cursor, region.string_start)?;
    let (parameter, shape) = object_parameter(&found.function, found.argument)?;
    let candidates: Vec<(String, String, CompletionItemKind)> = match &found.position {
        Position::Key if found.in_string => return None,
        Position::Key => shape
            .properties
            .keys()
            .filter(|key| !found.keys.contains(key) && key.starts_with(&found.partial))
            .map(|key| (key.clone(), format!("{key}: "), CompletionItemKind::PROPERTY))
            .collect(),
        Position::AfterKey => return Some(Vec::new()),
        Position::Value(key) => match value_kind(shape.properties.get(key)?)? {
            ValueKind::Members(members) => members
                .iter()
                .filter(|member| member.starts_with(&found.partial))
                .map(|member| {
                    let insert = if found.in_string { member.clone() } else { format!("\"{member}\"") };
                    (format!("\"{member}\""), insert, CompletionItemKind::ENUM_MEMBER)
                })
                .collect(),
            ValueKind::Boolean if found.in_string => return None,
            ValueKind::Boolean => ["true", "false"]
                .into_iter()
                .filter(|value| value.starts_with(&found.partial))
                .map(|value| (value.to_string(), value.to_string(), CompletionItemKind::VALUE))
                .collect(),
        },
    };
    let span = analyzed.project(found.partial_start - shift..cursor - shift)?;
    let detail = match &found.position {
        Position::Value(key) => format!("{parameter}.{key}"),
        _ => format!("{parameter} key"),
    };
    let items = candidates
        .into_iter()
        .filter_map(|(label, insert, kind)| {
            let new_text = analyzed.encode(&insert)?;
            let mut item = text_edit_item(
                ctx,
                span.start,
                span.end,
                CompletionCandidate {
                    label,
                    new_text: new_text.clone(),
                    kind,
                    detail: Some(detail.clone()),
                    documentation: None,
                },
            )?;
            item.filter_text = Some(new_text);
            Some(item)
        })
        .collect();
    Some(items)
}

/// Whether option completion claims `offset` at `site`.
pub(crate) fn claims(ctx: &DocumentContext, offset: usize, site: Site) -> bool {
    completion(ctx, offset, site).is_some()
}

/// Where the cursor sits inside an object literal.
#[derive(Debug, PartialEq, Eq)]
enum Position {
    /// Where a key is written: after `{`, after a `,`.
    Key,
    /// After a complete key, before its `:`.
    AfterKey,
    /// The start of the value of `key`, before anything but the partial.
    Value(String),
}

/// The object-literal argument holding the cursor.
#[derive(Debug, PartialEq, Eq)]
struct OptionCursor {
    function: String,
    /// The object's argument index in the call.
    argument: usize,
    /// The keys written before the cursor.
    keys: Vec<String>,
    position: Position,
    /// The text typed at the cursor (inside the quotes when `in_string`).
    partial: String,
    /// The analyzed offset where `partial` starts.
    partial_start: usize,
    in_string: bool,
}

enum Frame {
    Call { name: String, argument: usize },
    Group,
    Object { owner: Option<(String, usize)>, keys: Vec<String>, slot: Slot },
}

enum Slot {
    Key,
    AfterKey(String),
    /// `fresh` until a token follows the `:`.
    Value { key: String, fresh: bool },
}

/// Recovers the object literal holding `cursor` in the expression whose body
/// starts at `body_start` of `text`, from the tokens before the cursor's
/// partial. `string_start` is the opening quote of a string the cursor sits
/// in. `None` unless the innermost open frame is an object that is itself a
/// whole argument of a named call.
fn option_cursor(text: &str, body_start: usize, cursor: usize, string_start: Option<usize>) -> Option<OptionCursor> {
    let (head_end, partial_start) = match string_start {
        Some(quote) => (quote, quote + 1),
        None => {
            let start = body_start + identifier_prefix_start(text.get(body_start..cursor)?);
            (start, start)
        }
    };
    let partial = text.get(partial_start..cursor)?;
    // A dotted path is a member access and an escape is not an enum member.
    if partial.contains(['.', '\\']) {
        return None;
    }
    let tokens = lex_spanned(text.get(body_start..head_end)?).ok()?;
    let mut stack: Vec<Frame> = Vec::new();
    let mut previous: Option<&Token> = None;
    for spanned in &tokens {
        let token = &spanned.value;
        if let Some(Frame::Object { slot: Slot::Value { fresh, .. }, .. }) = stack.last_mut()
            && !matches!(token, Token::Comma | Token::RBrace | Token::Eof)
        {
            *fresh = false;
        }
        match token {
            Token::LParen => stack.push(match previous {
                Some(Token::Variable(name)) => Frame::Call { name: name.clone(), argument: 0 },
                _ => Frame::Group,
            }),
            Token::LBracket => stack.push(Frame::Group),
            Token::LBrace => {
                let owner = match (stack.last(), previous) {
                    (Some(Frame::Call { name, argument }), Some(Token::LParen | Token::Comma)) => {
                        Some((name.clone(), *argument))
                    }
                    _ => None,
                };
                stack.push(Frame::Object { owner, keys: Vec::new(), slot: Slot::Key });
            }
            Token::RParen | Token::RBracket | Token::RBrace => {
                stack.pop();
            }
            Token::Comma => match stack.last_mut() {
                Some(Frame::Call { argument, .. }) => *argument += 1,
                Some(Frame::Object { slot, .. }) => *slot = Slot::Key,
                _ => {}
            },
            Token::Variable(key) | Token::StringLiteral(key) => {
                if let Some(Frame::Object { slot: slot @ Slot::Key, .. }) = stack.last_mut() {
                    *slot = Slot::AfterKey(key.clone());
                }
            }
            Token::Colon => {
                if let Some(Frame::Object { keys, slot, .. }) = stack.last_mut()
                    && let Slot::AfterKey(key) = slot
                {
                    let key = std::mem::take(key);
                    keys.push(key.clone());
                    *slot = Slot::Value { key, fresh: true };
                }
            }
            _ => {}
        }
        previous = Some(token);
    }
    let Some(Frame::Object { owner: Some((function, argument)), keys, slot }) = stack.pop() else {
        return None;
    };
    let position = match slot {
        Slot::Key => Position::Key,
        Slot::AfterKey(_) => Position::AfterKey,
        Slot::Value { key, fresh: true } => Position::Value(key),
        Slot::Value { fresh: false, .. } => return None,
    };
    Some(OptionCursor {
        function,
        argument,
        keys,
        position,
        partial: partial.to_string(),
        partial_start,
        in_string: string_start.is_some(),
    })
}

/// The name and declared shape of parameter `index` of `function`, when a
/// catalog overload types it as an inline object (after dropping `null`).
fn object_parameter(function: &str, index: usize) -> Option<(&'static str, &'static SchemaShape)> {
    expressions::function_overloads(function).iter().find_map(|descriptor| {
        let parameter = descriptor.function_type.parameters.as_slice().get(index).filter(|parameter| !parameter.rest)?;
        atoms(&parameter.def).iter().find_map(|atom| match &atom.ty {
            TypeExpr::InlineObject(shape) if !atom.is_array => Some((parameter.name.as_str(), shape)),
            _ => None,
        })
    })
}

fn atoms(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(arms) => arms,
    }
}

/// The values a declared property enumerates.
enum ValueKind<'d> {
    Members(&'d [String]),
    Boolean,
}

fn value_kind(def: &PropertyDef) -> Option<ValueKind<'_>> {
    atoms(def).iter().filter(|atom| !atom.is_array).find_map(|atom| match atom.ty {
        TypeExpr::Primitive(SimplifiedType::Enum) => atom.constraints.iter().find_map(|constraint| match constraint {
            Constraint::Members(members) => Some(ValueKind::Members(members.as_slice())),
            _ => None,
        }),
        TypeExpr::Primitive(SimplifiedType::Boolean) => Some(ValueKind::Boolean),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cursor state at the end of `expression`, read as a `{{` body.
    fn at_end(expression: &str) -> Option<OptionCursor> {
        let text = format!("{{{{ {expression}");
        let region = expression_region_at(&text, text.len(), &[])?;
        option_cursor(&text, region.start + 2, text.len(), region.string_start)
    }

    #[test]
    fn a_key_position_follows_the_brace_and_each_comma() {
        let open = at_end(r#"load_data("r.csv", "string[]", {"#).unwrap();
        assert_eq!((open.function.as_str(), open.argument, &open.position), ("load_data", 2, &Position::Key));
        assert!(open.keys.is_empty());
        let next = at_end(r#"load_data("r.csv", "string[]", {format: "csv", he"#).unwrap();
        assert_eq!(next.position, Position::Key);
        assert_eq!(next.keys, ["format"]);
        assert_eq!(next.partial, "he");
    }

    #[test]
    fn a_value_position_follows_the_colon_and_a_partial_quoted_value_is_in_string() {
        let value = at_end(r#"load_data("r.csv", "string[]", {format: "#).unwrap();
        assert_eq!(value.position, Position::Value("format".into()));
        assert!(!value.in_string);
        let quoted = at_end(r#"load_data("r.csv", "string[]", {header: true, format: "cs"#).unwrap();
        assert_eq!(quoted.position, Position::Value("format".into()));
        assert_eq!((quoted.partial.as_str(), quoted.in_string), ("cs", true));
        assert_eq!(quoted.keys, ["header", "format"]);
    }

    #[test]
    fn a_value_with_more_than_the_partial_is_not_claimed() {
        assert_eq!(at_end(r#"load_data("r.csv", "string[]", {delimiter: ";" + "#), None);
        assert_eq!(at_end(r#"load_data("r.csv", "string[]", {format: f."#), None);
    }

    #[test]
    fn the_second_argument_and_a_nested_object_are_not_claimed() {
        assert_eq!(at_end(r#"load_data("r.csv", "#), None);
        assert_eq!(at_end(r#"load_data("r.csv", "str"#), None);
        assert_eq!(at_end(r#"load_data("r.csv", "string[]", {format: {"#), None);
        assert_eq!(at_end(r#"load_data("r.csv", "string[]", [{"#), None);
    }

    #[test]
    fn a_callback_inside_the_call_keeps_the_objects_own_call() {
        let nested = at_end(r#"map(load_data("h.yaml", "{ name: string }[]", {"#).unwrap();
        assert_eq!((nested.function.as_str(), nested.argument), ("load_data", 2));
        let in_callback = at_end(r#"map(load_data("h.yaml", "string[]"), h => pr_list({"#).unwrap();
        assert_eq!((in_callback.function.as_str(), in_callback.argument), ("pr_list", 0));
        assert_eq!(at_end(r#"map(load_data("h.yaml", "string[]", {}), h => h"#), None);
    }

    #[test]
    fn only_an_inline_object_parameter_declares_keys() {
        let (name, shape) = object_parameter("load_data", 2).unwrap();
        assert_eq!(name, "options");
        assert_eq!(shape.properties.keys().collect::<Vec<_>>(), ["format", "delimiter", "header"]);
        assert!(object_parameter("LOAD_DATA", 2).is_some());
        assert!(object_parameter("load_data", 1).is_none());
        assert!(object_parameter("pr_list", 0).is_none());
    }
}
