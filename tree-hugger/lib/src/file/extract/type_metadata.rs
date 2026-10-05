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
        // The JavaScript grammar's class nodes are a subset of TypeScript's.
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
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

/// Extracts type metadata from Rust struct_item, union_item, enum_item or trait_item nodes.
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
        "struct_item" | "union_item" => {
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
        // A trait's associated constants; its methods are standalone.
        "trait_item" => {
            if let Some(body) = node.child_by_field_name("body") {
                metadata.fields = extract_rust_trait_constants(body, source);
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

/// Extracts the associated constants of a Rust trait's declaration_list.
fn extract_rust_trait_constants(body: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut cursor = body.walk();
    body.named_children(&mut cursor)
        .filter(|item| item.kind() == "const_item")
        .filter_map(|item| {
            let name = item
                .child_by_field_name("name")?
                .utf8_text(source.as_bytes())
                .ok()?;
            Some(FieldInfo {
                name: name.to_string(),
                type_annotation: item
                    .child_by_field_name("type")
                    .and_then(|type_node| type_node.utf8_text(source.as_bytes()).ok())
                    .map(str::to_string),
                doc_comment: extract_doc_comment(item, ProgrammingLanguage::Rust, source),
                visibility: None,
                is_static: true,
            })
        })
        .collect()
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

/// The explicit value of an enum member as written (`2` in `B = 2`): the
/// `value` field of a Rust `enum_variant`, a C/C++ `enumerator`, a C#
/// `enum_member_declaration`, a TypeScript `enum_assignment` or a PHP
/// `enum_case`.
pub(crate) fn variant_discriminant(member: Node<'_>, source: &str) -> Option<String> {
    member
        .child_by_field_name("value")?
        .utf8_text(source.as_bytes())
        .ok()
        .map(str::to_string)
}

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
            variant.discriminant = variant_discriminant(child, source);

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

/// Extracts type metadata from a TypeScript or JavaScript class (also
/// abstract), interface, object type alias or enum.
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
        "class_declaration" | "abstract_class_declaration" => {
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

        let name = ecma_member_name(child, source);

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
        // `field_definition` is the JavaScript grammar's class field.
        if !matches!(
            kind,
            "public_field_definition" | "property_definition" | "field_definition"
        ) {
            continue;
        }

        let name = ecma_member_name(child, source);

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

/// The declared name of a JS/TS class field or property signature, as the
/// locals query captures it: an identifier, a `#private` name, the contents
/// of a quoted name (`"label": string` is `label`) or a numeric name. A
/// computed name (`[key]`) declares nothing nameable.
fn ecma_member_name(member: Node<'_>, source: &str) -> Option<String> {
    let name = member
        .child_by_field_name("name")
        .or_else(|| member.child_by_field_name("property"))?;
    let name = match name.kind() {
        "property_identifier" | "private_property_identifier" | "number" => name,
        "string" => find_child_by_kind(name, "string_fragment")?,
        _ => return None,
    };
    name.utf8_text(source.as_bytes()).ok().map(str::to_string)
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
            let mut variant = VariantInfo::unit(name);
            variant.discriminant = variant_discriminant(child, source);
            variants.push(variant);
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
                    visibility: go_identifier_visibility(name),
                    is_static: false,
                });
            }
        }
    }

    fields
}

/// Lists a Go `interface_type`'s `method_spec` children as field entries.
/// tree-sitter-go 0.25 names an interface method `method_elem`, so with the
/// pinned grammar this returns nothing and a Go interface has no list.
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

/// Extracts fields from Python class body: every target of a class-body
/// assignment, through chains (`first = second = 0`) and one level of
/// tuple or list pattern (`a, b = …`, `(c, d) = …`, `[e, f] = …`), as the
/// locals query reports them.
pub(crate) fn extract_python_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "expression_statement" {
            continue;
        }
        // `x: T`, `x: T = v`, `x: ClassVar[T]` and `x = v`; the standalone
        // field symbols come from the same class-body assignments.
        let Some(statement) = find_child_by_kind(child, "assignment") else {
            continue;
        };
        let is_static = is_python_class_attribute(statement, source);
        let mut assignment = Some(statement);
        while let Some(current) = assignment {
            let type_annotation = current
                .child_by_field_name("type")
                .and_then(|type_node| type_node.utf8_text(source.as_bytes()).ok())
                .map(str::to_string);
            for target in python_assignment_targets(current) {
                if let Ok(name) = target.utf8_text(source.as_bytes()) {
                    fields.push(FieldInfo {
                        name: name.to_string(),
                        type_annotation: type_annotation.clone(),
                        doc_comment: None,
                        visibility: None,
                        is_static,
                    });
                }
            }
            assignment = current
                .child_by_field_name("right")
                .filter(|right| right.kind() == "assignment");
        }
    }

    fields
}

/// The names an assignment's left side binds: a name, or the names directly
/// in a tuple or list pattern.
fn python_assignment_targets(assignment: Node<'_>) -> Vec<Node<'_>> {
    let Some(left) = assignment.child_by_field_name("left") else {
        return Vec::new();
    };
    match left.kind() {
        "identifier" => vec![left],
        "pattern_list" | "tuple_pattern" | "list_pattern" => {
            let mut cursor = left.walk();
            left.named_children(&mut cursor)
                .filter(|item| item.kind() == "identifier")
                .collect()
        }
        _ => Vec::new(),
    }
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
        // Record components, then the fields declared in the body.
        "record_declaration" => {
            if let Some(params) = find_child_by_kind(node, "formal_parameters") {
                metadata.fields = extract_java_record_components(params, source);
            }
            if let Some(body) = find_child_by_kind(node, "class_body") {
                metadata.fields.extend(extract_java_class_fields(body, source));
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "interface_body") {
                metadata.fields = extract_java_interface_constants(body, source);
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

/// Extracts the constants of a Java interface_body. Its methods are reported as
/// standalone methods, not as fields.
pub(crate) fn extract_java_interface_constants(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "constant_declaration" {
            continue;
        }

        let type_annotation = find_java_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

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
                    // Implicitly `public static final`.
                    visibility: Some(Visibility::Public),
                    is_static: true,
                });
            }
        }
    }

    fields
}

// ============================================================================
// C type metadata extraction
// ============================================================================

/// Extracts type metadata from C struct, union, enum or typedef declarations.
pub(crate) fn extract_c_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "struct_specifier" | "union_specifier" => {
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
            // `typedef struct { ... } Name;` and `typedef union { ... } Name;`
            if let Some(field_list) = node
                .child_by_field_name("type")
                .filter(|specifier| matches!(specifier.kind(), "struct_specifier" | "union_specifier"))
                .and_then(|specifier| find_child_by_kind(specifier, "field_declaration_list"))
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

    for child in flattened_members(node) {
        if child.kind() != "field_declaration" {
            continue;
        }
        if let Some(promoted) = anonymous_member_list(child) {
            fields.extend(extract_c_struct_fields(promoted, source));
            continue;
        }

        // Get the type (first type-like child)
        let type_annotation = find_c_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let mut inner_cursor = child.walk();
        for inner in child.children_by_field_name("declarator", &mut inner_cursor) {
            if let Some(name_node) = c_field_name(inner)
                && let Ok(name) = name_node.utf8_text(source.as_bytes())
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

/// The field name a C/C++ field declarator introduces, under any chain of
/// pointer, array, reference, parenthesized, attributed and initializer
/// declarators (`char **name`, `int values [[maybe_unused]] [3]`,
/// `int (*callback)(int)`; a C++ variable template member parses as a plain
/// `declaration`). A declarator that declares a function (see
/// [`declared_function`]) is a method, not a field.
fn c_field_name(declarator: Node<'_>) -> Option<Node<'_>> {
    if declared_function(declarator).is_some() {
        return None;
    }
    declarator_name(declarator).filter(|name| matches!(name.kind(), "field_identifier" | "identifier"))
}

/// The member list of an anonymous `struct`/`union` member
/// (`union { int a; float b; };`), whose members belong to the enclosing
/// type. A tagless member with a declarator (`struct { … } named;`) is an
/// ordinary field.
fn anonymous_member_list(field_declaration: Node<'_>) -> Option<Node<'_>> {
    let specifier = field_declaration.child_by_field_name("type")?;
    anonymous_member_declaration(specifier)?;
    specifier.child_by_field_name("body")
}

/// The members of a type body (C/C++ `field_declaration_list` and
/// `enumerator_list`, C# `declaration_list` and
/// `enum_member_declaration_list`) in source order, with transparent wrappers
/// (template headers, preprocessor conditionals, nested or not) replaced by
/// their contents, so a wrapped member is listed like any other.
fn flattened_members(list: Node<'_>) -> Vec<Node<'_>> {
    let mut members = Vec::new();
    let mut cursor = list.walk();
    for child in list.named_children(&mut cursor) {
        if is_transparent_wrapper_kind(child.kind()) {
            members.extend(flattened_members(child));
        } else {
            members.push(child);
        }
    }
    members
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

    for child in flattened_members(node) {
        if child.kind() != "enumerator" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            let mut variant = VariantInfo::unit(name);
            variant.discriminant = variant_discriminant(child, source);
            variants.push(variant);
        }
    }

    variants
}

// ============================================================================
// C++ type metadata extraction
// ============================================================================

/// Extracts type metadata from C++ class, struct, union, or enum declarations.
pub(crate) fn extract_cpp_type_metadata(
    node: Node<'_>,
    node_kind: &str,
    source: &str,
) -> Option<TypeMetadata> {
    let mut metadata = TypeMetadata::new();

    match node_kind {
        "class_specifier" | "struct_specifier" | "union_specifier" => {
            if let Some(field_list) = find_child_by_kind(node, "field_declaration_list") {
                // `class` members default to private; `struct` and `union`
                // members to public.
                let default_visibility = if node_kind == "class_specifier" {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
                metadata.fields =
                    extract_cpp_class_fields(field_list, source, Some(default_visibility));
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

/// Extracts fields from C++ field_declaration_list; `visibility` is the
/// access in effect before the first access specifier.
pub(crate) fn extract_cpp_class_fields(
    node: Node<'_>,
    source: &str,
    visibility: Option<Visibility>,
) -> Vec<FieldInfo> {
    let mut fields = Vec::new();

    // Track current visibility section (C++ uses access specifiers at section level)
    let mut current_visibility = visibility;

    for child in flattened_members(node) {
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

        // A variable template member is a `declaration` under its template header.
        if !matches!(child.kind(), "field_declaration" | "declaration") {
            continue;
        }
        // An anonymous union's members are the enclosing type's, in its
        // current access section.
        if let Some(promoted) = anonymous_member_list(child) {
            fields.extend(extract_cpp_class_fields(promoted, source, current_visibility));
            continue;
        }

        // Get the type
        let type_annotation = find_c_type_node(child)
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        // Check for static specifier
        let is_static = extract_cpp_is_static(child, source);

        let mut inner_cursor = child.walk();
        for inner in child.children_by_field_name("declarator", &mut inner_cursor) {
            if let Some(name_node) = c_field_name(inner)
                && let Ok(name) = name_node.utf8_text(source.as_bytes())
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
        // Positional parameters, then the members declared in the body.
        "record_declaration" => {
            if let Some(params) = find_child_by_kind(node, "parameter_list") {
                metadata.fields = extract_csharp_record_parameters(params, source);
            }
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields.extend(extract_csharp_class_fields(body, source));
            }
        }
        "enum_declaration" => {
            if let Some(body) = find_child_by_kind(node, "enum_member_declaration_list") {
                metadata.variants = extract_csharp_enum_variants(body, source);
            }
        }
        "interface_declaration" => {
            if let Some(body) = find_child_by_kind(node, "declaration_list") {
                metadata.fields = extract_csharp_interface_properties(body, source);
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

/// Extracts the fields and properties of a C# declaration_list (class,
/// struct or record body), including members inside preprocessor
/// conditionals.
pub(crate) fn extract_csharp_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();

    for child in flattened_members(node) {
        if child.kind() == "property_declaration" {
            fields.extend(csharp_property(child, source, None));
            continue;
        }
        // `event Action Changed;` is a field of delegate type.
        if !matches!(child.kind(), "field_declaration" | "event_field_declaration") {
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

/// Extracts variants from C# enum_member_declaration_list, including members
/// inside preprocessor conditionals.
pub(crate) fn extract_csharp_enum_variants(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let mut variants = Vec::new();

    for child in flattened_members(node) {
        if child.kind() != "enum_member_declaration" {
            continue;
        }

        if let Some(name_node) = find_child_by_kind(child, "identifier")
            && let Ok(name) = name_node.utf8_text(source.as_bytes())
        {
            let mut variant = VariantInfo::unit(name);
            variant.discriminant = variant_discriminant(child, source);
            variants.push(variant);
        }
    }

    variants
}

/// Extracts the properties of a C# interface's declaration_list, including
/// properties inside preprocessor conditionals. Its methods are reported as
/// standalone methods, not as fields.
pub(crate) fn extract_csharp_interface_properties(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();

    for child in flattened_members(node) {
        if child.kind() != "property_declaration" {
            continue;
        }
        // Interface members are implicitly public.
        fields.extend(csharp_property(child, source, Some(Visibility::Public)));
    }

    fields
}

/// A C# `property_declaration` as a field entry; `default_visibility`
/// applies when no access modifier is written. An accessor-level modifier
/// (`{ get; private set; }`) does not change the property's access.
fn csharp_property(
    property: Node<'_>,
    source: &str,
    default_visibility: Option<Visibility>,
) -> Option<FieldInfo> {
    let name = property
        .child_by_field_name("name")?
        .utf8_text(source.as_bytes())
        .ok()?;
    Some(FieldInfo {
        name: name.to_string(),
        type_annotation: property
            .child_by_field_name("type")
            .and_then(|type_node| type_node.utf8_text(source.as_bytes()).ok())
            .map(str::to_string),
        doc_comment: None,
        visibility: extract_csharp_visibility(property, source).or(default_visibility),
        is_static: extract_java_csharp_is_static(property, source),
    })
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
                metadata.fields = extract_swift_protocol_properties(body, source);
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

/// Extracts fields from Swift class_body: every binding of a property
/// declaration (`var first = 1, second = 2`), each with its own annotation.
pub(crate) fn extract_swift_class_fields(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }

        let visibility = extract_swift_visibility(child, source);
        let is_static = extract_swift_is_static(child, source);

        // Each `name:` pattern is followed by its own optional annotation.
        let mut pending: Option<String> = None;
        let mut type_annotation = None;
        let mut inner_cursor = child.walk();
        for (index, inner) in child.children(&mut inner_cursor).enumerate() {
            if child.field_name_for_child(index as u32) == Some("name") {
                if let Some(name) = pending.take() {
                    fields.push(swift_field(name, type_annotation.take(), visibility, is_static));
                }
                pending = inner
                    .child_by_field_name("bound_identifier")
                    .and_then(|ident| ident.utf8_text(source.as_bytes()).ok())
                    .map(str::to_string);
                type_annotation = None;
            } else if inner.kind() == "type_annotation" {
                type_annotation = inner
                    .child(1) // Skip colon
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(str::to_string);
            }
        }
        if let Some(name) = pending {
            fields.push(swift_field(name, type_annotation, visibility, is_static));
        }
    }

    fields
}

fn swift_field(
    name: String,
    type_annotation: Option<String>,
    visibility: Option<Visibility>,
    is_static: bool,
) -> FieldInfo {
    FieldInfo {
        name,
        type_annotation,
        doc_comment: None,
        visibility,
        is_static,
    }
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

/// Extracts the property requirements of a Swift protocol_body. Its function
/// requirements are reported as standalone methods, not as fields.
pub(crate) fn extract_swift_protocol_properties(node: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "protocol_property_declaration" {
            continue;
        }

        if let Some(ident) = child
            .child_by_field_name("name")
            .and_then(|pattern| pattern.child_by_field_name("bound_identifier"))
            && let Ok(name) = ident.utf8_text(source.as_bytes())
        {
            let type_annotation = find_child_by_kind(child, "type_annotation")
                .and_then(|annotation| annotation.child_by_field_name("name"))
                .and_then(|type_node| type_node.utf8_text(source.as_bytes()).ok())
                .map(str::to_string);

            fields.push(FieldInfo {
                name: name.to_string(),
                type_annotation,
                doc_comment: None,
                visibility: extract_swift_visibility(child, source),
                is_static: extract_swift_is_static(child, source),
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
        "class_definition" | "trait_definition" | "object_definition" => {
            // Class parameters are in class_parameters
            if let Some(params) = find_child_by_kind(node, "class_parameters") {
                metadata.fields = extract_scala_class_parameters(params, source);
            }
            if let Some(body) = node.child_by_field_name("body") {
                metadata.fields.extend(extract_scala_body_fields(body, source));
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

/// The `val`/`var` definitions and declarations directly in a Scala
/// template body, one entry per name, each with the declaration's written
/// type: a grouped `val a, b: Int` lists `a` and `b`, both typed `Int`. A
/// destructuring pattern (`val (a, b) = …`) declares no listed field.
fn extract_scala_body_fields(body: Node<'_>, source: &str) -> Vec<FieldInfo> {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).ok().map(str::to_string);
    let mut fields = Vec::new();
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        let names: Vec<Node<'_>> = match member.kind() {
            "val_definition" | "var_definition" => match member.child_by_field_name("pattern") {
                Some(pattern) if pattern.kind() == "identifier" => vec![pattern],
                Some(pattern) if pattern.kind() == "identifiers" => {
                    let mut names = pattern.walk();
                    pattern
                        .named_children(&mut names)
                        .filter(|name| name.kind() == "identifier")
                        .collect()
                }
                _ => Vec::new(),
            },
            "val_declaration" | "var_declaration" => {
                let mut names = member.walk();
                member
                    .children_by_field_name("name", &mut names)
                    .filter(|name| name.kind() == "identifier")
                    .collect()
            }
            _ => Vec::new(),
        };
        let type_annotation = member.child_by_field_name("type").and_then(text);
        fields.extend(names.into_iter().filter_map(|name| {
            Some(FieldInfo {
                name: text(name)?,
                type_annotation: type_annotation.clone(),
                doc_comment: None,
                visibility: None,
                is_static: false,
            })
        }));
    }
    fields
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

/// The cases of a Scala `enum_body`, in source order, one entry per name: a
/// simple case (`case Red, Green`) is a unit variant; a full case
/// (`case Value(x: Int)`) lists its class parameters as named payload fields
/// with their declared types as written. Every parameter clause is flattened
/// in order (`case V(x: Int)(using o: Ord)` lists `x` then `o`); a default
/// (`x: Int = 1`) is part of the parameter, never a discriminant, since a
/// Scala case has no explicit value.
pub(crate) fn extract_scala_enum_cases(node: Node<'_>, source: &str) -> Vec<VariantInfo> {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).ok().map(str::to_string);
    let mut variants = Vec::new();
    let mut definitions = node.walk();
    for definition in node.named_children(&mut definitions) {
        if definition.kind() != "enum_case_definitions" {
            continue;
        }
        let mut cases = definition.walk();
        for case in definition.named_children(&mut cases) {
            let Some(name) = case.child_by_field_name("name").and_then(text) else {
                continue;
            };
            match case.kind() {
                "simple_enum_case" => variants.push(VariantInfo::unit(name)),
                "full_enum_case" => {
                    let mut clauses = case.walk();
                    let fields = case
                        .children_by_field_name("class_parameters", &mut clauses)
                        .flat_map(|clause| {
                            let mut parameters = clause.walk();
                            clause
                                .named_children(&mut parameters)
                                .filter(|parameter| parameter.kind() == "class_parameter")
                                .collect::<Vec<_>>()
                        })
                        .filter_map(|parameter| {
                            let mut field = FieldInfo::new(parameter.child_by_field_name("name").and_then(text)?);
                            field.type_annotation = parameter.child_by_field_name("type").and_then(text);
                            Some(field)
                        })
                        .collect();
                    variants.push(VariantInfo::with_fields(name, fields));
                }
                _ => {}
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
            let mut variant = VariantInfo::unit(name);
            variant.discriminant = variant_discriminant(child, source);
            variants.push(variant);
        }
    }

    variants
}
