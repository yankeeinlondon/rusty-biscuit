//! Symbol-kind classification, container detection, node spans, and export
//! detection helpers carved out of `tree_file.rs`.

use super::*;


pub(crate) fn is_block_like_dead_code_container(kind: &str) -> bool {
    kind.contains("block")
        || kind.contains("body")
        || kind.contains("compound")
        || kind == "statement_list"
        || kind == "source_file"
}

pub(crate) fn symbol_kind_priority(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Method => 100,
        SymbolKind::Function => 90,
        SymbolKind::Constant => 85,
        SymbolKind::Field => 80,
        SymbolKind::Parameter => 75,
        SymbolKind::Class
        | SymbolKind::Interface
        | SymbolKind::Enum
        | SymbolKind::Trait
        | SymbolKind::Type
        | SymbolKind::Module
        | SymbolKind::Namespace => 70,
        SymbolKind::Macro => 65,
        SymbolKind::Variable => 50,
        SymbolKind::Unknown => 0,
    }
}

pub(crate) fn refine_symbol_kind(mut kind: SymbolKind, node: Node<'_>) -> SymbolKind {
    if kind != SymbolKind::Variable {
        return kind;
    }

    let mut current = Some(node);
    while let Some(cursor) = current {
        match cursor.kind() {
            "self_parameter"
            | "parameter"
            | "parameter_declaration"
            | "typed_parameter"
            | "typed_default_parameter"
            | "default_parameter"
            | "required_parameter"
            | "optional_parameter"
            | "formal_parameter"
            | "formal_parameters"
            | "parameters"
            | "parameter_list" => {
                kind = SymbolKind::Parameter;
                break;
            }
            "field_declaration"
            | "property_declaration"
            | "field_definition"
            | "property_initializer" => {
                kind = SymbolKind::Field;
                break;
            }
            "const_item" | "const_declaration" | "constant" => {
                kind = SymbolKind::Constant;
                break;
            }
            _ => {}
        }

        current = cursor.parent();
    }

    kind
}

pub(crate) fn find_symbol_container(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<(String, SymbolKind)> {
    let mut current = node.parent();
    while let Some(parent) = current {
        match language {
            ProgrammingLanguage::Rust => {
                if parent.kind() == "impl_item" {
                    let trait_name = parent
                        .child_by_field_name("trait")
                        .and_then(|value| value.utf8_text(source.as_bytes()).ok())
                        .map(str::to_string);
                    let type_name = parent
                        .child_by_field_name("type")
                        .and_then(|value| value.utf8_text(source.as_bytes()).ok())
                        .map(str::to_string);

                    let container = match (trait_name, type_name) {
                        (Some(trait_name), Some(type_name)) => {
                            format!("impl {trait_name} for {type_name}")
                        }
                        (None, Some(type_name)) => format!("impl {type_name}"),
                        _ => "impl".to_string(),
                    };

                    return Some((container, SymbolKind::Type));
                }

                if let Some(container) = named_container(
                    parent,
                    source,
                    &[
                        ("trait_item", SymbolKind::Trait),
                        ("struct_item", SymbolKind::Type),
                        ("enum_item", SymbolKind::Enum),
                        ("mod_item", SymbolKind::Module),
                    ],
                ) {
                    return Some(container);
                }
            }
            _ => {
                if let Some(container) = named_container(
                    parent,
                    source,
                    &[
                        ("class_declaration", SymbolKind::Class),
                        ("class_definition", SymbolKind::Class),
                        ("struct_declaration", SymbolKind::Type),
                        ("struct_specifier", SymbolKind::Type),
                        ("enum_declaration", SymbolKind::Enum),
                        ("enum_specifier", SymbolKind::Enum),
                        ("interface_declaration", SymbolKind::Interface),
                        ("protocol_declaration", SymbolKind::Interface),
                        ("record_declaration", SymbolKind::Type),
                        ("namespace_declaration", SymbolKind::Namespace),
                        ("namespace_definition", SymbolKind::Namespace),
                        ("module", SymbolKind::Module),
                        ("type_declaration", SymbolKind::Type),
                    ],
                ) {
                    return Some(container);
                }
            }
        }

        current = parent.parent();
    }

    None
}

pub(crate) fn named_container(
    node: Node<'_>,
    source: &str,
    kind_map: &[(&str, SymbolKind)],
) -> Option<(String, SymbolKind)> {
    let container_kind = kind_map
        .iter()
        .find_map(|(kind_name, mapped_kind)| (*kind_name == node.kind()).then_some(*mapped_kind))?;

    // Common field names used by tree-sitter grammars for declaration identifiers.
    let name = ["name", "declarator", "identifier", "type", "path"]
        .iter()
        .find_map(|field| {
            node.child_by_field_name(field)
                .and_then(|value| value.utf8_text(source.as_bytes()).ok())
                .map(str::to_string)
        })
        .or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find_map(|child| match child.kind() {
                    "identifier" | "type_identifier" | "field_identifier" => {
                        child.utf8_text(source.as_bytes()).ok().map(str::to_string)
                    }
                    _ => None,
                })
        })?;

    Some((name, container_kind))
}

pub(crate) fn is_definition_like_reference(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };

    if is_named_definition_node(parent, node) {
        return true;
    }

    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => matches!(
            parent.kind(),
            "property_signature"
                | "method_signature"
                | "required_parameter"
                | "optional_parameter"
                | "rest_pattern"
                | "shorthand_property_identifier_pattern"
                | "pair_pattern"
        ),
        ProgrammingLanguage::Rust => matches!(
            parent.kind(),
            "parameter" | "self_parameter" | "field_declaration" | "use_as_clause"
        ),
        _ => false,
    }
}

pub(crate) fn is_named_definition_node(parent: Node<'_>, node: Node<'_>) -> bool {
    let parent_kind = parent.kind();
    if !matches!(
        parent_kind,
        "function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "method_definition"
            | "property_signature"
            | "method_signature"
            | "public_field_definition"
            | "variable_declarator"
            | "import_specifier"
            | "namespace_import"
            | "required_parameter"
            | "optional_parameter"
            | "formal_parameter"
            | "function_item"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "mod_item"
            | "const_item"
            | "static_item"
            | "type_item"
            | "field_declaration"
            | "macro_definition"
    ) {
        return false;
    }

    ["name", "alias", "declarator", "value", "parameter", "path"]
        .iter()
        .filter_map(|field| parent.child_by_field_name(field))
        .any(|candidate| candidate.id() == node.id())
}

pub(crate) fn symbol_kind_from_capture(capture_name: &str) -> Option<SymbolKind> {
    let suffix = if let Some(rest) = capture_name.strip_prefix("local.definition.") {
        rest
    } else if capture_name == "local.definition" {
        return Some(SymbolKind::Variable);
    } else {
        return None;
    };

    // Skip context captures (e.g., function.context, type.context)
    // These are used for extracting signatures and doc comments in Phase 3
    if suffix.ends_with(".context") {
        return None;
    }

    match suffix {
        "function" => Some(SymbolKind::Function),
        "method" => Some(SymbolKind::Method),
        "type" => Some(SymbolKind::Type),
        "class" => Some(SymbolKind::Class),
        "interface" => Some(SymbolKind::Interface),
        "enum" => Some(SymbolKind::Enum),
        "trait" => Some(SymbolKind::Trait),
        "namespace" => Some(SymbolKind::Namespace),
        "module" => Some(SymbolKind::Module),
        "var" => Some(SymbolKind::Variable),
        "parameter" => Some(SymbolKind::Parameter),
        "field" => Some(SymbolKind::Field),
        "macro" => Some(SymbolKind::Macro),
        "const" | "constant" => Some(SymbolKind::Constant),
        "import" | "associated" => None,
        _ => Some(SymbolKind::Unknown),
    }
}

pub(crate) fn range_for_node(node: Node<'_>) -> CodeRange {
    let start = node.start_position();
    let end = node.end_position();

    CodeRange {
        start_line: start.row.saturating_add(1),
        start_column: start.column.saturating_add(1),
        end_line: end.row.saturating_add(1),
        end_column: end.column.saturating_add(1),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    }
}

pub(crate) fn text_span_from_code_range(range: &CodeRange) -> TextSpan {
    TextSpan {
        start: TextPoint {
            line: range.start_line as u32,
            column: range.start_column as u32,
        },
        end: TextPoint {
            line: range.end_line as u32,
            column: range.end_column as u32,
        },
        start_byte: range.start_byte as u32,
        end_byte: range.end_byte as u32,
    }
}

pub(crate) fn text_span_for_node(node: Node<'_>) -> TextSpan {
    text_span_from_code_range(&range_for_node(node))
}

pub(crate) fn body_span_for_declaration(node: Node<'_>) -> Option<TextSpan> {
    node.child_by_field_name("body")
        .or_else(|| find_child_by_kind(node, "class_body"))
        .or_else(|| find_child_by_kind(node, "enum_body"))
        .or_else(|| find_child_by_kind(node, "interface_body"))
        .or_else(|| find_child_by_kind(node, "object_type"))
        .map(text_span_for_node)
}

pub(crate) fn symbol_kind_v2_for_node(record: &SymbolRecord, node: Node<'_>, source: &str) -> SymbolKindV2 {
    match node.kind() {
        "type_alias_declaration" => SymbolKindV2::TypeAlias,
        "method_definition" => {
            let is_constructor = node
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(source.as_bytes()).ok())
                .is_some_and(|name| name == "constructor");
            if is_constructor {
                SymbolKindV2::Constructor
            } else {
                SymbolKindV2::Method
            }
        }
        "public_field_definition" | "property_definition" | "property_signature" => {
            SymbolKindV2::Property
        }
        _ => record.kind,
    }
}

pub(crate) fn extract_typescript_type_alias_target(node: Node<'_>, source: &str) -> Option<String> {
    if node.kind() != "type_alias_declaration" {
        return None;
    }

    node.child_by_field_name("value")
        .or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "object_type" || child.kind().ends_with("type"))
        })
        .and_then(|target| target.utf8_text(source.as_bytes()).ok())
        .map(str::to_string)
}

pub(crate) fn is_exported_definition(symbol: &SymbolInfo, node: Node<'_>, root: Node<'_>) -> bool {
    if matches!(
        symbol.kind,
        SymbolKind::Parameter | SymbolKind::Field | SymbolKind::Method
    ) {
        return false;
    }

    if symbol.language == ProgrammingLanguage::Rust {
        return is_rust_public_definition(node);
    }

    if !is_top_level_definition(node, root) {
        return false;
    }

    let parent = node.parent();
    parent.is_some_and(|parent| is_export_node(parent.kind()))
}

pub(crate) fn is_rust_public_definition(node: Node<'_>) -> bool {
    let mut current = Some(node);

    while let Some(cursor_node) = current {
        let mut cursor = cursor_node.walk();
        for child in cursor_node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                return true;
            }
        }

        if matches!(
            cursor_node.kind(),
            "function_item"
                | "struct_item"
                | "enum_item"
                | "trait_item"
                | "mod_item"
                | "const_item"
                | "static_item"
                | "type_item"
                | "macro_definition"
                | "impl_item"
        ) {
            return false;
        }

        current = cursor_node.parent();
    }

    false
}

pub(crate) fn is_export_node(kind: &str) -> bool {
    matches!(
        kind,
        "export_statement"
            | "export_declaration"
            | "export_default_declaration"
            | "public_declaration"
    )
}

pub(crate) fn is_top_level_definition(node: Node<'_>, root: Node<'_>) -> bool {
    let mut current = Some(node);
    while let Some(cursor) = current {
        let Some(parent) = cursor.parent() else {
            break;
        };

        if parent == root {
            return true;
        }

        if matches!(
            parent.kind(),
            "class_body"
                | "interface_body"
                | "object_type"
                | "statement_block"
                | "field_declaration_list"
                | "enum_body"
        ) {
            return false;
        }

        current = Some(parent);
    }

    false
}
