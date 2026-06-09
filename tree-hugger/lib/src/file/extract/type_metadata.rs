//! Type metadata (fields, variants, type parameters) extraction helpers carved
//! out of `tree_file.rs`.

use super::*;

/// Finds a Rust type node among the children of a parameter or other node.
///
/// Rust has many type node kinds: primitive_type, type_identifier, reference_type,
/// generic_type, tuple_type, array_type, function_type, etc.
pub(crate) fn find_rust_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const RUST_TYPE_KINDS: &[&str] = &[
        "primitive_type",
        "type_identifier",
        "reference_type",
        "generic_type",
        "scoped_type_identifier",
        "tuple_type",
        "array_type",
        "slice_type",
        "pointer_type",
        "function_type",
        "unit_type",
        "never_type",
        "bounded_type",
        "dynamic_type",
        "abstract_type",
        "macro_invocation", // For macro-generated types
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| RUST_TYPE_KINDS.contains(&child.kind()))
}

// ============================================================================
// Type metadata extraction
// ============================================================================

/// Extracts metadata from a type definition node (struct, enum, interface, etc.).
pub(crate) fn extract_type_metadata(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<TypeMetadata> {
    let node_kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => extract_rust_type_metadata(node, node_kind, source),
        ProgrammingLanguage::TypeScript => {
            extract_typescript_type_metadata(node, node_kind, source)
        }
        ProgrammingLanguage::Go => extract_go_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Python => extract_python_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Java => extract_java_type_metadata(node, node_kind, source),
        ProgrammingLanguage::C => extract_c_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Cpp => extract_cpp_type_metadata(node, node_kind, source),
        ProgrammingLanguage::CSharp => extract_csharp_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Swift => extract_swift_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Scala => extract_scala_type_metadata(node, node_kind, source),
        ProgrammingLanguage::Php => extract_php_type_metadata(node, node_kind, source),
        _ => None,
    }
}

/// Extracts type metadata from Rust struct_item or enum_item nodes.
pub(crate) fn extract_rust_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    // Try by field name first (the proper tree-sitter way)
    if let Some(type_params) = node.child_by_field_name("type_parameters") {
        metadata.type_parameters = extract_rust_type_parameters(type_params, source);
    } else if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        // Fallback to kind-based search
        metadata.type_parameters = extract_rust_type_parameters(type_params, source);
    }

    match node_kind {
        "struct_item" => {
            // Check for field_declaration_list (normal struct) or ordered_field_declaration_list (tuple struct)
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                metadata.fields = extract_rust_struct_fields(field_list, source);
            } else if let Some(tuple_fields) =
                find_child_by_kind(node, "ordered_field_declaration_list")
            {
                // Tuple struct: struct Point(i32, i32)
                metadata.fields = extract_rust_tuple_struct_fields(tuple_fields, source);
            }
        }
        "enum_item" => {
            if let Some(variant_list) = find_child_by_kind(node, "enum_variant_list") {
                metadata.variants = extract_rust_enum_variants(variant_list, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Rust type_parameters node.
pub(crate) fn extract_rust_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" | "lifetime" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
            // type_parameter is the wrapper node for a single generic type param
            "type_parameter" => {
                // The name field contains the type identifier
                if let Some(name) = child.child_by_field_name("name") {
                    if let Ok(text) = name.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                } else if let Some(ident) = find_child_by_kind(child, "type_identifier")
                    && let Ok(text) = ident.utf8_text(source.as_bytes())
                {
                    params.push(text.to_string());
                }
            }
            "lifetime_parameter" => {
                if let Some(lifetime) = child
                    .child_by_field_name("lifetime")
                    .or_else(|| find_child_by_kind(child, "lifetime"))
                    && let Ok(text) = lifetime.utf8_text(source.as_bytes())
                {
                    params.push(text.to_string());
                }
            }
            "constrained_type_parameter" | "optional_type_parameter" => {
                // Get the type identifier from the constrained parameter
                if let Some(ident) = find_child_by_kind(child, "type_identifier")
                    && let Ok(text) = ident.utf8_text(source.as_bytes())
                {
                    params.push(text.to_string());
                }
            }
            _ => {}
        }
    }

    params
}

/// Extracts fields from a Rust field_declaration_list.
pub(crate) fn extract_rust_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        let name = find_child_by_kind(child, "field_identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let type_annotation = find_rust_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let doc_comment = extract_doc_comment(child, ProgrammingLanguage::Rust, source);

        if let Some(name) = name {
            fields.push(FieldInfo {
                name,
                type_annotation,
                doc_comment,
                visibility: None,
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts fields from a Rust tuple struct (ordered_field_declaration_list).
pub(crate) fn extract_rust_tuple_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();
    let mut index = 0;

    for child in node.children(&mut cursor) {
        // Look for type nodes directly as children
        if RUST_TYPE_KINDS.contains(&child.kind()) {
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());

            fields.push(FieldInfo {
                name: index.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: None,
                is_static: false,
            });
            index += 1;
        }
    }

    fields
}

/// List of Rust type node kinds (used for tuple struct field extraction).
const RUST_TYPE_KINDS: &[&str] = &[
    "primitive_type",
    "type_identifier",
    "reference_type",
    "generic_type",
    "scoped_type_identifier",
    "tuple_type",
    "array_type",
    "slice_type",
    "pointer_type",
    "function_type",
    "unit_type",
    "never_type",
    "bounded_type",
    "dynamic_type",
    "abstract_type",
    "macro_invocation",
];

/// Extracts variants from a Rust enum_variant_list.
pub(crate) fn extract_rust_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_variant" {
            continue;
        }

        let name = find_child_by_kind(child, "identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let doc_comment = extract_doc_comment(child, ProgrammingLanguage::Rust, source);

        if let Some(name) = name {
            let mut variant = VariantInfo::unit(&name);
            variant.doc_comment = doc_comment;

            // Check for tuple variant: Variant(Type1, Type2)
            if let Some(tuple_fields) = find_child_by_kind(child, "ordered_field_declaration_list")
            {
                variant.tuple_fields = extract_rust_variant_tuple_fields(tuple_fields, source);
            }

            // Check for struct variant: Variant { field: Type }
            if let Some(field_list) = find_child_by_kind(child, "field_declaration_list") {
                variant.struct_fields = extract_rust_struct_fields(field_list, source);
            }

            variants.push(variant);
        }
    }

    variants
}

/// Extracts tuple field types from an enum variant.
pub(crate) fn extract_rust_variant_tuple_fields(node: Node<'_>, source: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if RUST_TYPE_KINDS.contains(&child.kind())
            && let Ok(text) = child.utf8_text(source.as_bytes())
        {
            fields.push(text.to_string());
        }
    }

    fields
}

/// Extracts type metadata from TypeScript interface or type_alias_declaration.
pub(crate) fn extract_typescript_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_typescript_type_parameters(type_params, source);
    }

    match node_kind {
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "interface_body")
                .or_else(|| find_child_by_kind(node, "object_type"))
            {
                metadata.fields = extract_typescript_interface_fields(body, source);
            }
        }
        "type_alias_declaration" => {
            // Type aliases can be object types or other types
            if let Some(object_type) = find_child_by_kind(node, "object_type") {
                metadata.fields = extract_typescript_interface_fields(object_type, source);
            }
        }
        "class_declaration" => {
            if let Some(body) = find_child_by_kind(node, "class_body") {
                metadata.fields = extract_typescript_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_body") {
                metadata.variants = extract_typescript_enum_variants(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from TypeScript type_parameters node.
pub(crate) fn extract_typescript_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter"
            && let Some(name) = find_child_by_kind(child, "type_identifier")
            && let Ok(text) = name.utf8_text(source.as_bytes())
        {
            params.push(text.to_string());
        }
    }

    params
}

/// Extracts fields from TypeScript interface_body or object_type.
pub(crate) fn extract_typescript_interface_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "property_signature" && kind != "public_field_definition" {
            continue;
        }

        let name = find_child_by_kind(child, "property_identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let type_annotation = find_child_by_kind(child, "type_annotation")
            .and_then(|ta| ta.child(1)) // Skip the colon
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name) = name {
            fields.push(FieldInfo {
                name,
                type_annotation,
                doc_comment: None,
                visibility: None,
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts fields from TypeScript class_body.
pub(crate) fn extract_typescript_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "public_field_definition" && kind != "property_definition" {
            continue;
        }

        let name = find_child_by_kind(child, "property_identifier")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let type_annotation = find_child_by_kind(child, "type_annotation")
            .and_then(|ta| ta.child(1))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Extract visibility and static modifiers
        let visibility = extract_ts_visibility(child, source);
        let is_static = extract_ts_is_static(child);

        if let Some(name) = name {
            fields.push(FieldInfo {
                name,
                type_annotation,
                doc_comment: None,
                visibility,
                is_static,
            });
        }
    }

    fields
}

/// Extracts variants from TypeScript enum_body.
pub(crate) fn extract_typescript_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_assignment" && child.kind() != "property_identifier" {
            continue;
        }

        let name = if child.kind() == "enum_assignment" {
            find_child_by_kind(child, "property_identifier")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string())
        } else {
            child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
        };

        if let Some(name) = name {
            variants.push(VariantInfo::unit(name));
        }
    }

    variants
}

/// Extracts type metadata from Go type_spec nodes.
pub(crate) fn extract_go_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    if node_kind != "type_spec" && node_kind != "type_declaration" {
        return None;
    }

    // For type_declaration, find the type_spec inside
    let type_spec = if node_kind == "type_declaration" {
        find_child_by_kind(node, "type_spec")?
    } else {
        node
    };

    let mut metadata = TypeMetadata::new();

    // Extract type parameters if present
    if let Some(type_params) = find_child_by_kind(type_spec, "type_parameter_list") {
        metadata.type_parameters = extract_go_type_parameters(type_params, source);
    }

    // Check for struct type
    if let Some(struct_type) = find_child_by_kind(type_spec, "struct_type")
        && let Some(field_list) = find_child_by_kind(struct_type, "field_declaration_list")
    {
        metadata.fields = extract_go_struct_fields(field_list, source);
    }

    // Check for interface type
    if let Some(interface_type) = find_child_by_kind(type_spec, "interface_type") {
        metadata.fields = extract_go_interface_methods(interface_type, source);
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts type parameters from Go type_parameter_list.
pub(crate) fn extract_go_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter_declaration" {
            // Get all identifiers in this declaration
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "identifier"
                    && let Ok(text) = inner.utf8_text(source.as_bytes())
                {
                    params.push(text.to_string());
                }
            }
        }
    }

    params
}

/// Extracts fields from Go struct field_declaration_list.
pub(crate) fn extract_go_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Go allows multiple identifiers per field declaration: `a, b int`
        let type_annotation = find_go_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "field_identifier"
                && let Ok(name) = inner.utf8_text(source.as_bytes())
            {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation: type_annotation.clone(),
                    doc_comment: None,
                    visibility: None,
                    is_static: false,
                });
            }
        }
    }

    fields
}

/// Extracts method signatures from Go interface_type.
pub(crate) fn extract_go_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "method_spec" {
            let name = find_child_by_kind(child, "field_identifier")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());

            // Get the full method signature as the "type"
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());

            if let Some(name) = name {
                fields.push(FieldInfo {
                    name,
                    type_annotation,
                    doc_comment: None,
                    visibility: None,
                    is_static: false,
                });
            }
        }
    }

    fields
}

/// Extracts type metadata from Python class_definition.
pub(crate) fn extract_python_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    if node_kind != "class_definition" {
        return None;
    }

    let mut metadata = TypeMetadata::new();

    // Check for class body
    if let Some(body) = find_child_by_kind(node, "block") {
        metadata.fields = extract_python_class_fields(body, source);
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from Python class body.
pub(crate) fn extract_python_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            // Annotated assignment: name: str = "value"
            "expression_statement" => {
                if let Some(assignment) = find_child_by_kind(child, "assignment") {
                    // Check for type annotation
                    if let Some(type_node) = find_child_by_kind(assignment, "type") {
                        let name = assignment
                            .child(0)
                            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                            .map(|s| s.to_string());

                        let type_annotation = type_node
                            .utf8_text(source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());

                        if let Some(name) = name {
                            fields.push(FieldInfo {
                                name,
                                type_annotation,
                                doc_comment: None,
                                visibility: None,
                                is_static: false,
                            });
                        }
                    }
                }
            }
            // Typed assignment without value: name: str
            "typed_assignment_statement" => {
                let name = find_child_by_kind(child, "identifier")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());

                let type_annotation = find_child_by_kind(child, "type")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());

                if let Some(name) = name {
                    fields.push(FieldInfo {
                        name,
                        type_annotation,
                        doc_comment: None,
                        visibility: None,
                        is_static: false,
                    });
                }
            }
            _ => {}
        }
    }

    fields
}

// ============================================================================
// Java type metadata extraction
// ============================================================================

/// Extracts type metadata from Java class, enum, record, or interface declarations.
pub(crate) fn extract_java_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_java_type_parameters(type_params, source);
    }

    match node_kind {
        "class_declaration" => {
            if let Some(body) = find_child_by_kind(node, "class_body") {
                metadata.fields = extract_java_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_body") {
                metadata.variants = extract_java_enum_variants(body, source);
            }
        }
        "record_declaration" => {
            // Record components are in formal_parameters
            if let Some(params) = find_child_by_kind(node, "formal_parameters") {
                metadata.fields = extract_java_record_components(params, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "interface_body") {
                metadata.fields = extract_java_interface_methods(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Java type_parameters node.
pub(crate) fn extract_java_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter"
            && let Some(ident) = find_child_by_kind(child, "type_identifier")
            && let Ok(text) = ident.utf8_text(source.as_bytes())
        {
            params.push(text.to_string());
        }
    }

    params
}

/// Extracts fields from Java class_body.
pub(crate) fn extract_java_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Get the type
        let type_annotation = find_java_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Extract visibility and static modifier
        let visibility = extract_java_visibility(child, source);
        let is_static = extract_java_csharp_is_static(child, source);

        // Get all variable declarators (handles `int x, y;`)
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "variable_declarator"
                && let Some(name_node) = find_child_by_kind(inner, "identifier")
                && let Ok(name) = name_node.utf8_text(source.as_bytes())
            {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation: type_annotation.clone(),
                    doc_comment: None,
                    visibility,
                    is_static,
                });
            }
        }
    }

    fields
}

/// Finds a type node in a Java declaration.
pub(crate) fn find_java_type_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| JAVA_TYPE_KINDS.contains(&child.kind()))
}

/// List of Java type node kinds.
const JAVA_TYPE_KINDS: &[&str] = &[
    "integral_type",
    "floating_point_type",
    "boolean_type",
    "void_type",
    "type_identifier",
    "scoped_type_identifier",
    "generic_type",
    "array_type",
];

/// Extracts variants from Java enum_body.
pub(crate) fn extract_java_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_constant" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            variants.push(VariantInfo::unit(name));
        }
    }

    variants
}

/// Extracts record components from Java formal_parameters.
pub(crate) fn extract_java_record_components(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "formal_parameter" {
            continue;
        }

        let type_annotation = find_java_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: Some(Visibility::Public), // Record components are implicitly public
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts method signatures from Java interface_body.
pub(crate) fn extract_java_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            // Get the full method signature as the "type"
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: Some(Visibility::Public), // Interface members are implicitly public
                is_static: false,
            });
        }
    }

    fields
}

// ============================================================================
// C type metadata extraction
// ============================================================================

/// Extracts type metadata from C struct or enum declarations.
pub(crate) fn extract_c_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "struct_specifier" => {
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                metadata.fields = extract_c_struct_fields(field_list, source);
            }
        }
        "enum_specifier" => {
            if let Some(enumerator_list) = find_child_by_kind(node, "enumerator_list") {
                metadata.variants = extract_c_enum_variants(enumerator_list, source);
            }
        }
        "type_definition" => {
            // For typedef struct { ... } Name; we look for struct_specifier inside
            if let Some(struct_spec) = find_child_by_kind(node, "struct_specifier")
                && let Some(field_list) = find_child_by_kind(struct_spec, "field_declaration_list")
            {
                metadata.fields = extract_c_struct_fields(field_list, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from C field_declaration_list.
pub(crate) fn extract_c_struct_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Get the type (first type-like child)
        let type_annotation = find_c_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Get field identifiers
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "field_identifier"
                && let Ok(name) = inner.utf8_text(source.as_bytes())
            {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation: type_annotation.clone(),
                    doc_comment: None,
                    visibility: None, // C doesn't have visibility modifiers
                    is_static: false,
                });
            }
        }
    }

    fields
}

/// Finds a type node in a C declaration.
pub(crate) fn find_c_type_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| C_TYPE_KINDS.contains(&child.kind()))
}

/// List of C type node kinds.
const C_TYPE_KINDS: &[&str] = &[
    "primitive_type",
    "type_identifier",
    "sized_type_specifier",
    "struct_specifier",
    "enum_specifier",
    "union_specifier",
    // C++ types
    "qualified_identifier",
    "template_type",
];

/// Extracts variants from C enumerator_list.
pub(crate) fn extract_c_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enumerator" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            variants.push(VariantInfo::unit(name));
        }
    }

    variants
}

// ============================================================================
// C++ type metadata extraction
// ============================================================================

/// Extracts type metadata from C++ class, struct, or enum declarations.
pub(crate) fn extract_cpp_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "class_specifier" | "struct_specifier" => {
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                metadata.fields = extract_cpp_class_fields(field_list, source);
            }
        }
        "enum_specifier" => {
            if let Some(enumerator_list) = find_child_by_kind(node, "enumerator_list") {
                metadata.variants = extract_c_enum_variants(enumerator_list, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from C++ field_declaration_list.
pub(crate) fn extract_cpp_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    // Track current visibility section (C++ uses access specifiers at section level)
    let mut current_visibility: Option<Visibility> = None;

    for child in node.children(&mut cursor) {
        // Update visibility for access specifiers
        if child.kind() == "access_specifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                current_visibility = match text.trim_end_matches(':') {
                    "public" => Some(Visibility::Public),
                    "protected" => Some(Visibility::Protected),
                    "private" => Some(Visibility::Private),
                    _ => None,
                };
            }
            continue;
        }

        if child.kind() != "field_declaration" {
            continue;
        }

        // Skip method declarations (they have function_declarator)
        if find_child_by_kind(child, "function_declarator").is_some() {
            continue;
        }

        // Get the type
        let type_annotation = find_c_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Check for static specifier
        let is_static = extract_cpp_is_static(child, source);

        // Get field identifiers
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "field_identifier"
                && let Ok(name) = inner.utf8_text(source.as_bytes())
            {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation: type_annotation.clone(),
                    doc_comment: None,
                    visibility: current_visibility,
                    is_static,
                });
            }
        }
    }

    fields
}

// ============================================================================
// C# type metadata extraction
// ============================================================================

/// Extracts type metadata from C# class, struct, enum, interface, or record declarations.
pub(crate) fn extract_csharp_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameter_list") {
        metadata.type_parameters = extract_csharp_type_parameters(type_params, source);
    }

    match node_kind {
        "class_declaration" | "struct_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_csharp_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_member_declaration_list") {
                metadata.variants = extract_csharp_enum_variants(body, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_csharp_interface_methods(body, source);
            }
        }
        "record_declaration" => {
            // Record parameters are in parameter_list
            if let Some(params) = find_child_by_kind(node, "parameter_list") {
                metadata.fields = extract_csharp_record_parameters(params, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from C# type_parameter_list.
pub(crate) fn extract_csharp_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter"
            && let Some(ident) = find_child_by_kind(child, "identifier")
            && let Ok(text) = ident.utf8_text(source.as_bytes())
        {
            params.push(text.to_string());
        }
    }

    params
}

/// Extracts fields from C# declaration_list (class/struct body).
pub(crate) fn extract_csharp_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }

        // Get the type from variable_declaration
        let type_annotation = find_child_by_kind(child, "variable_declaration")
            .and_then(|vd| find_csharp_type_node(vd))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Extract visibility and static modifier
        let visibility = extract_csharp_visibility(child, source);
        let is_static = extract_java_csharp_is_static(child, source);

        // Get variable declarators
        if let Some(var_decl) = find_child_by_kind(child, "variable_declaration") {
            let mut inner_cursor = var_decl.walk();
            for inner in var_decl.children(&mut inner_cursor) {
                if inner.kind() == "variable_declarator"
                    && let Some(ident) = find_child_by_kind(inner, "identifier")
                    && let Ok(name) = ident.utf8_text(source.as_bytes())
                {
                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation: type_annotation.clone(),
                        doc_comment: None,
                        visibility,
                        is_static,
                    });
                }
            }
        }
    }

    fields
}

/// Finds a type node in a C# variable_declaration.
pub(crate) fn find_csharp_type_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| CSHARP_TYPE_KINDS.contains(&child.kind()))
}

/// List of C# type node kinds.
const CSHARP_TYPE_KINDS: &[&str] = &[
    "predefined_type",
    "identifier",
    "qualified_name",
    "generic_name",
    "array_type",
    "nullable_type",
    "tuple_type",
];

/// Extracts variants from C# enum_member_declaration_list.
pub(crate) fn extract_csharp_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_member_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            variants.push(VariantInfo::unit(name));
        }
    }

    variants
}

/// Extracts method signatures from C# interface declaration_list.
pub(crate) fn extract_csharp_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            // Get the full method signature as the "type"
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: Some(Visibility::Public), // Interface members are implicitly public
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts parameters from C# record parameter_list.
pub(crate) fn extract_csharp_record_parameters(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "parameter" {
            continue;
        }

        let type_annotation = find_csharp_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: Some(Visibility::Public), // Record parameters are implicitly public
                is_static: false,
            });
        }
    }

    fields
}

// ============================================================================
// Swift type metadata extraction
// ============================================================================

/// Extracts type metadata from Swift class, struct, enum, or protocol declarations.
pub(crate) fn extract_swift_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_swift_type_parameters(type_params, source);
    }

    match node_kind {
        "class_declaration" => {
            // Swift uses class_declaration for struct, class, and enum
            // Check for class_body to determine actual type
            if let Some(body) = find_child_by_kind(node, "class_body") {
                // Check if this is an enum by looking for enum_entry nodes
                let mut cursor = body.walk();
                let has_enum_entries = body
                    .children(&mut cursor)
                    .any(|child| child.kind() == "enum_entry");

                if has_enum_entries {
                    metadata.variants = extract_swift_enum_cases(body, source);
                } else {
                    metadata.fields = extract_swift_class_fields(body, source);
                }
            }
        }
        "protocol_declaration" => {
            if let Some(body) = find_child_by_kind(node, "protocol_body") {
                metadata.fields = extract_swift_protocol_methods(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Swift type_parameters.
pub(crate) fn extract_swift_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter"
            && let Some(ident) = find_child_by_kind(child, "type_identifier")
            && let Ok(text) = ident.utf8_text(source.as_bytes())
        {
            params.push(text.to_string());
        }
    }

    params
}

/// Extracts fields from Swift class_body.
pub(crate) fn extract_swift_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }

        // Get pattern (contains the name)
        if let Some(pattern) = find_child_by_kind(child, "pattern")
            && let Some(ident) = find_child_by_kind(pattern, "simple_identifier")
            && let Ok(name) = ident.utf8_text(source.as_bytes())
        {
            // Get type annotation
            let type_annotation = find_child_by_kind(child, "type_annotation")
                .and_then(|ta| ta.child(1)) // Skip colon
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());

            // Extract visibility and static modifier
            let visibility = extract_swift_visibility(child, source);
            let is_static = extract_swift_is_static(child, source);

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility,
                is_static,
            });
        }
    }

    fields
}

/// Extracts enum cases from Swift class_body (for enums).
pub(crate) fn extract_swift_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        // Try multiple possible node types for Swift enum cases
        let kind = child.kind();
        if kind == "enum_entry" || kind == "enum_case_pattern" {
            if let Some(ident) = find_child_by_kind(child, "simple_identifier")
                && let Ok(name) = ident.utf8_text(source.as_bytes())
            {
                variants.push(VariantInfo::unit(name));
            }
        } else if kind == "switch_entry" {
            // Swift switch/case patterns
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "simple_identifier"
                    && let Ok(name) = inner.utf8_text(source.as_bytes())
                {
                    variants.push(VariantInfo::unit(name));
                }
            }
        }
    }

    variants
}

/// Extracts method requirements from Swift protocol_body.
pub(crate) fn extract_swift_protocol_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "protocol_function_declaration" {
            continue;
        }

        if let Some(ident) = find_child_by_kind(child, "simple_identifier")
            && let Ok(name) = ident.utf8_text(source.as_bytes())
        {
            // Get the full method signature
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: None, // Protocol methods don't have visibility
                is_static: false,
            });
        }
    }

    fields
}

// ============================================================================
// Scala type metadata extraction
// ============================================================================

/// Extracts type metadata from Scala class, trait, object, or enum definitions.
pub(crate) fn extract_scala_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    // Extract generic type parameters
    if let Some(type_params) = find_child_by_kind(node, "type_parameters") {
        metadata.type_parameters = extract_scala_type_parameters(type_params, source);
    }

    match node_kind {
        "class_definition" => {
            // Class parameters are in class_parameters
            if let Some(params) = find_child_by_kind(node, "class_parameters") {
                metadata.fields = extract_scala_class_parameters(params, source);
            }
        }
        "trait_definition" => {
            // Traits can have method declarations in template_body
            if let Some(body) = find_child_by_kind(node, "template_body") {
                metadata.fields = extract_scala_trait_methods(body, source);
            }
        }
        "object_definition" => {
            // Objects can have members in template_body
            if let Some(body) = find_child_by_kind(node, "template_body") {
                metadata.fields = extract_scala_object_members(body, source);
            }
        }
        "enum_definition" => {
            if let Some(body) = find_child_by_kind(node, "enum_body") {
                metadata.variants = extract_scala_enum_cases(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts generic type parameters from Scala type_parameters.
pub(crate) fn extract_scala_type_parameters(node: Node<'_>, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        // Look for identifiers inside type parameters
        if child.kind() == "identifier"
            && let Ok(text) = child.utf8_text(source.as_bytes())
        {
            params.push(text.to_string());
        }
    }

    params
}

/// Extracts class parameters from Scala class_parameters.
pub(crate) fn extract_scala_class_parameters(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "class_parameter" {
            continue;
        }

        if let Some(ident) = find_child_by_kind(child, "identifier")
            && let Ok(name) = ident.utf8_text(source.as_bytes())
        {
            // Get type annotation (after colon)
            let type_annotation = find_child_by_kind(child, "type_identifier")
                .or_else(|| find_child_by_kind(child, "generic_type"))
                .or_else(|| find_child_by_kind(child, "compound_type"))
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: None, // Scala class parameters don't have explicit visibility
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts method declarations from Scala trait template_body.
pub(crate) fn extract_scala_trait_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "function_declaration" && kind != "function_definition" {
            continue;
        }

        if let Some(ident) = find_child_by_kind(child, "identifier")
            && let Ok(name) = ident.utf8_text(source.as_bytes())
        {
            // Get the full method signature
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: None, // Scala trait methods don't have explicit visibility
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts members from Scala object template_body.
pub(crate) fn extract_scala_object_members(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind != "function_definition" && kind != "val_definition" && kind != "var_definition" {
            continue;
        }

        // For function definitions
        if kind == "function_definition"
            && let Some(ident) = find_child_by_kind(child, "identifier")
            && let Ok(name) = ident.utf8_text(source.as_bytes())
        {
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: None,
                is_static: true, // Object members are effectively static
            });
        }
    }

    fields
}

/// Extracts enum cases from Scala enum_body.
pub(crate) fn extract_scala_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "simple_enum_case" && child.kind() != "enum_case_definitions" {
            continue;
        }

        if child.kind() == "simple_enum_case" {
            if let Some(ident) = find_child_by_kind(child, "identifier")
                && let Ok(name) = ident.utf8_text(source.as_bytes())
            {
                variants.push(VariantInfo::unit(name));
            }
        } else {
            // enum_case_definitions can contain multiple cases
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "identifier"
                    && let Ok(name) = inner.utf8_text(source.as_bytes())
                {
                    variants.push(VariantInfo::unit(name));
                }
            }
        }
    }

    variants
}

// ============================================================================
// PHP type metadata extraction
// ============================================================================

/// Extracts type metadata from PHP class, interface, trait, or enum declarations.
pub(crate) fn extract_php_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "class_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_php_class_fields(body, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_php_interface_methods(body, source);
            }
        }
        "trait_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_php_class_fields(body, source);
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_declaration_list") {
                metadata.variants = extract_php_enum_cases(body, source);
            }
        }
        _ => {}
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

/// Extracts fields from PHP class declaration_list.
pub(crate) fn extract_php_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }

        // Get type if present
        let type_annotation = find_child_by_kind(child, "named_type")
            .or_else(|| find_child_by_kind(child, "primitive_type"))
            .or_else(|| find_child_by_kind(child, "optional_type"))
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Extract visibility and static modifier
        let visibility = extract_php_visibility(child, source);
        let is_static = extract_php_is_static(child);

        // Get property elements
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if inner.kind() == "property_element"
                && let Some(var_name) = find_child_by_kind(inner, "variable_name")
                && let Some(name_node) = find_child_by_kind(var_name, "name")
                && let Ok(name) = name_node.utf8_text(source.as_bytes())
            {
                fields.push(FieldInfo {
                    name: name.to_string(),
                    type_annotation: type_annotation.clone(),
                    doc_comment: None,
                    visibility,
                    is_static,
                });
            }
        }
    }

    fields
}

/// Extracts method signatures from PHP interface declaration_list.
pub(crate) fn extract_php_interface_methods(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "method_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "name")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            // Get the full method signature
            let type_annotation = child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: Some(Visibility::Public), // Interface methods are implicitly public
                is_static: false,
            });
        }
    }

    fields
}

/// Extracts enum cases from PHP enum_declaration_list.
pub(crate) fn extract_php_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "enum_case" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "name")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            variants.push(VariantInfo::unit(name));
        }
    }

    variants
}
