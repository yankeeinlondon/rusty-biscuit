//! Function signature, parameter, return-type, visibility, and static-modifier
//! extraction helpers carved out of `tree_file.rs`.

use super::*;


/// Extracts function signature from a function/method node.
pub(crate) fn extract_signature(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<FunctionSignature> {
    let parameters = extract_parameters(node, language, source);
    let return_type = extract_return_type(node, language, source);
    let visibility = extract_visibility(node, language, source);
    let is_static = extract_is_static(node, language, source);

    if parameters.is_empty() && return_type.is_none() && visibility.is_none() && !is_static {
        return None;
    }

    Some(FunctionSignature {
        parameters,
        return_type,
        visibility,
        is_static,
    })
}

/// Extracts parameters from a function node.
pub(crate) fn extract_parameters(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<ParameterInfo> {
    let params_node_kind = match language {
        ProgrammingLanguage::Rust => "parameters",
        ProgrammingLanguage::Python | ProgrammingLanguage::Scala => "parameters",
        ProgrammingLanguage::Go | ProgrammingLanguage::C | ProgrammingLanguage::Cpp => {
            "parameter_list"
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => "formal_parameters",
        ProgrammingLanguage::Php => "formal_parameters",
        ProgrammingLanguage::Java => "formal_parameters",
        ProgrammingLanguage::CSharp => "parameter_list",
        ProgrammingLanguage::Swift => return extract_swift_parameters(node, source),
        _ => return Vec::new(),
    };

    // For Go, methods have TWO parameter_list nodes:
    // 1. receiver (g *Greeter)
    // 2. actual parameters (name string)
    // We need to find the SECOND parameter_list for methods.
    //
    // For C/C++, parameters are inside function_declarator. The context node
    // may be either function_definition or function_declarator:
    // - If function_definition: look in function_declarator child
    // - If function_declarator: look directly for parameter_list
    let params_node = if language == ProgrammingLanguage::Go && node.kind() == "method_declaration"
    {
        find_nth_child_by_kind(node, params_node_kind, 1) // 0-indexed, so 1 = second
    } else if matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp) {
        // C/C++: Context may be function_declarator or function_definition
        if node.kind() == "function_declarator" {
            // Already at function_declarator, look for parameter_list directly
            find_child_by_kind(node, params_node_kind)
        } else {
            // At function_definition, look inside function_declarator
            find_child_by_kind(node, "function_declarator")
                .and_then(|fd| find_child_by_kind(fd, params_node_kind))
        }
    } else {
        find_child_by_kind(node, params_node_kind)
    };

    let params_node = match params_node {
        Some(n) => n,
        None => return Vec::new(),
    };

    let mut parameters = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        // Go needs special handling: a single parameter_declaration can define
        // multiple parameters (e.g., `a, b int` defines both `a` and `b`)
        if language == ProgrammingLanguage::Go {
            parameters.extend(extract_go_parameters(child, source));
        } else if let Some(param) = extract_single_parameter(child, language, source) {
            parameters.push(param);
        }
    }

    parameters
}

/// Extracts a single parameter from a parameter node.
///
/// Note: Go is handled separately in `extract_parameters` using `extract_go_parameters`
/// because Go allows multiple identifiers per declaration (e.g., `a, b int`).
/// Swift is handled separately using `extract_swift_parameters`.
pub(crate) fn extract_single_parameter(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<ParameterInfo> {
    let kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => extract_rust_parameter(node, source),
        ProgrammingLanguage::Python => extract_python_parameter(node, source),
        ProgrammingLanguage::Scala => extract_scala_parameter(node, source),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            extract_js_parameter(node, kind, source)
        }
        ProgrammingLanguage::Php => extract_php_parameter(node, source),
        ProgrammingLanguage::Java => extract_java_parameter(node, source),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => extract_c_parameter(node, source),
        ProgrammingLanguage::CSharp => extract_csharp_parameter(node, source),
        // Go is handled specially in extract_parameters
        // Swift is handled specially via extract_swift_parameters
        _ => None,
    }
}

pub(crate) fn extract_rust_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();

    if kind == "self_parameter" {
        return Some(ParameterInfo::new("self"));
    }

    if kind != "parameter" {
        return None;
    }

    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation - could be various type node kinds
    let type_annotation = find_rust_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None,
        is_variadic: false,
    })
}

pub(crate) fn extract_python_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();

    // Check if this typed_parameter contains a splat pattern (variadic)
    let has_list_splat = find_child_by_kind(node, "list_splat_pattern").is_some();
    let has_dict_splat = find_child_by_kind(node, "dictionary_splat_pattern").is_some();
    let is_splat = has_list_splat || has_dict_splat;

    // Handle different parameter types
    let (name_node, type_node, default_node, is_variadic) = match kind {
        "identifier" => (Some(node), None, None, false),
        "typed_parameter" => {
            // typed_parameter can contain a splat pattern: *names: str
            let name = if let Some(splat) = find_child_by_kind(node, "list_splat_pattern") {
                find_child_by_kind(splat, "identifier")
            } else if let Some(splat) = find_child_by_kind(node, "dictionary_splat_pattern") {
                find_child_by_kind(splat, "identifier")
            } else {
                find_child_by_kind(node, "identifier")
            };
            (name, find_child_by_kind(node, "type"), None, is_splat)
        }
        "default_parameter" => (
            find_child_by_kind(node, "identifier"),
            None,
            node.child_by_field_name("value"),
            false,
        ),
        "typed_default_parameter" => (
            find_child_by_kind(node, "identifier"),
            find_child_by_kind(node, "type"),
            node.child_by_field_name("value"),
            false,
        ),
        "list_splat_pattern" => (find_child_by_kind(node, "identifier"), None, None, true),
        "dictionary_splat_pattern" => (find_child_by_kind(node, "identifier"), None, None, true),
        _ => return None,
    };

    let name = name_node
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    let type_annotation = type_node
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let default_value = default_node
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Extracts Go parameters from a parameter_declaration node.
///
/// Go allows multiple identifiers to share a type: `a, b int` creates two parameters.
/// This function returns all parameters from a single declaration.
pub(crate) fn extract_go_parameters(node: Node<'_>, source: &str) -> Vec<ParameterInfo> {
    let kind = node.kind();

    let is_variadic = kind == "variadic_parameter_declaration";
    if kind != "parameter_declaration" && kind != "variadic_parameter_declaration" {
        return Vec::new();
    }

    // Find the type annotation (shared by all identifiers in this declaration)
    let type_annotation = find_go_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Collect all identifiers in this declaration
    let mut cursor = node.walk();
    let mut params = Vec::new();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier"
            && let Ok(name) = child.utf8_text(source.as_bytes())
        {
            params.push(ParameterInfo {
                name: name.to_string(),
                type_annotation: type_annotation.clone(),
                default_value: None,
                is_variadic,
            });
        }
    }

    params
}

/// Finds a Go type node among the children.
pub(crate) fn find_go_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const GO_TYPE_KINDS: &[&str] = &[
        "type_identifier",
        "pointer_type",
        "slice_type",
        "array_type",
        "map_type",
        "channel_type",
        "function_type",
        "interface_type",
        "struct_type",
        "qualified_type",
        "generic_type",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| GO_TYPE_KINDS.contains(&child.kind()))
}

pub(crate) fn extract_js_parameter(node: Node<'_>, kind: &str, source: &str) -> Option<ParameterInfo> {
    // Check if this is a rest/variadic parameter
    let is_rest = kind == "rest_pattern" || find_child_by_kind(node, "rest_pattern").is_some();

    let name = if kind == "rest_pattern" {
        find_child_by_kind(node, "identifier")
    } else if kind == "identifier" {
        Some(node)
    } else if kind == "assignment_pattern" {
        node.child_by_field_name("left")
    } else if kind == "required_parameter" || kind == "optional_parameter" {
        // For required_parameter containing rest_pattern: ...names
        if let Some(rest) = find_child_by_kind(node, "rest_pattern") {
            find_child_by_kind(rest, "identifier")
        } else {
            find_child_by_kind(node, "identifier")
        }
    } else {
        return None;
    };

    let name = name
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // For TypeScript, try to get type annotation
    let type_annotation = find_child_by_kind(node, "type_annotation")
        .and_then(|ta| ta.child(1)) // Skip the colon
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let default_value = if kind == "assignment_pattern" {
        node.child_by_field_name("right")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string())
    } else {
        None
    };

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic: is_rest,
    })
}

/// Extracts return type from a function node.
pub(crate) fn extract_return_type(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    match language {
        ProgrammingLanguage::Rust => extract_rust_return_type(node, source),
        ProgrammingLanguage::Python => extract_python_return_type(node, source),
        ProgrammingLanguage::Go => extract_go_return_type(node, source),
        ProgrammingLanguage::TypeScript => extract_typescript_return_type(node, source),
        ProgrammingLanguage::JavaScript => None, // JavaScript has no type annotations
        ProgrammingLanguage::Php => extract_php_return_type(node, source),
        ProgrammingLanguage::Java => extract_java_return_type(node, source),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => extract_c_return_type(node, source),
        ProgrammingLanguage::CSharp => extract_csharp_return_type(node, source),
        ProgrammingLanguage::Swift => extract_swift_return_type(node, source),
        ProgrammingLanguage::Scala => extract_scala_return_type(node, source),
        _ => None,
    }
}

/// Extracts visibility modifier from a function/method node.
pub(crate) fn extract_visibility(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<Visibility> {
    match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            extract_ts_visibility(node, source)
        }
        ProgrammingLanguage::Java => extract_java_visibility(node, source),
        ProgrammingLanguage::CSharp => extract_csharp_visibility(node, source),
        ProgrammingLanguage::Php => extract_php_visibility(node, source),
        ProgrammingLanguage::Rust => extract_rust_visibility(node, source),
        ProgrammingLanguage::Cpp => extract_cpp_visibility(node, source),
        ProgrammingLanguage::Swift => extract_swift_visibility(node, source),
        // Go, Python, Scala, C don't have visibility keywords (use naming conventions instead)
        _ => None,
    }
}

/// Extracts visibility from TypeScript/JavaScript method_definition.
///
/// TypeScript AST structure has accessibility_modifier as a child:
/// ```text
/// method_definition
///   accessibility_modifier (public/protected/private)
///   property_identifier
///   formal_parameters
///   ...
/// ```
pub(crate) fn extract_ts_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "accessibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            return match text {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
    }
    None
}

/// Extracts visibility from Java method_declaration.
///
/// Java AST structure has modifiers containing visibility:
/// ```text
/// method_declaration
///   modifiers
///     public/protected/private
///   type_identifier
///   identifier
///   ...
/// ```
///
/// In Java, interface members are implicitly public and cannot have explicit
/// visibility modifiers. This function infers `Public` for members declared
/// inside an interface.
pub(crate) fn extract_java_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    if let Some(modifiers) = find_child_by_kind(node, "modifiers") {
        let mut cursor = modifiers.walk();
        for child in modifiers.children(&mut cursor) {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            match text {
                "public" => return Some(Visibility::Public),
                "protected" => return Some(Visibility::Protected),
                "private" => return Some(Visibility::Private),
                _ => continue,
            }
        }
    }

    // Java interface members are implicitly public
    if is_inside_interface(node) {
        return Some(Visibility::Public);
    }

    None
}

/// Extracts visibility from C# method_declaration.
///
/// In C#, interface members are implicitly public and cannot have explicit
/// visibility modifiers. This function infers `Public` for members declared
/// inside an interface.
pub(crate) fn extract_csharp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            match text {
                "public" => return Some(Visibility::Public),
                "protected" => return Some(Visibility::Protected),
                "private" => return Some(Visibility::Private),
                "internal" => return Some(Visibility::Internal),
                _ => continue,
            }
        }
    }

    // C# interface members are implicitly public
    if is_inside_interface(node) {
        return Some(Visibility::Public);
    }

    None
}

/// Extracts visibility from PHP method_declaration.
pub(crate) fn extract_php_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            return match text {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
    }
    None
}

/// Extracts visibility from Rust function_item (pub keyword).
pub(crate) fn extract_rust_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = child.utf8_text(source.as_bytes()).ok()?;
            if text.starts_with("pub") {
                return Some(Visibility::Public);
            }
        }
    }
    None
}

/// Extracts visibility from C++ method declarations.
///
/// C++ visibility is handled via access specifiers in the class (public:, private:, etc.)
/// not as part of the method itself. For now, we check for inline access specifiers.
pub(crate) fn extract_cpp_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    // C++ uses access specifiers at the section level, not per-method
    // Check if there's an access_specifier sibling before this node
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        if sibling.kind() == "access_specifier" {
            let text = sibling.utf8_text(source.as_bytes()).ok()?;
            return match text.trim_end_matches(':') {
                "public" => Some(Visibility::Public),
                "protected" => Some(Visibility::Protected),
                "private" => Some(Visibility::Private),
                _ => None,
            };
        }
        // Stop if we hit another method or declaration
        if sibling.kind() == "function_definition"
            || sibling.kind() == "declaration"
            || sibling.kind() == "field_declaration"
        {
            break;
        }
        prev = sibling.prev_sibling();
    }
    None
}

/// Extracts visibility from Swift function declarations.
pub(crate) fn extract_swift_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                let text = modifier.utf8_text(source.as_bytes()).ok()?;
                match text {
                    "public" => return Some(Visibility::Public),
                    "internal" => return Some(Visibility::Internal),
                    "private" => return Some(Visibility::Private),
                    "fileprivate" => return Some(Visibility::Private),
                    _ => continue,
                }
            }
        }
    }
    None
}

/// Extracts whether a function/method is static.
pub(crate) fn extract_is_static(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> bool {
    match language {
        ProgrammingLanguage::Java | ProgrammingLanguage::CSharp => {
            extract_java_csharp_is_static(node, source)
        }
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            extract_ts_is_static(node)
        }
        ProgrammingLanguage::Php => extract_php_is_static(node),
        ProgrammingLanguage::Python => extract_python_is_static(node, source),
        ProgrammingLanguage::Swift => extract_swift_is_static(node, source),
        ProgrammingLanguage::Scala => extract_scala_is_static(node),
        ProgrammingLanguage::Cpp => extract_cpp_is_static(node, source),
        ProgrammingLanguage::Rust => extract_rust_is_static(node, source),
        // Go, C, and other languages don't have static methods in the same way
        _ => false,
    }
}

/// Checks if a Java or C# method has the `static` modifier.
pub(crate) fn extract_java_csharp_is_static(node: Node<'_>, source: &str) -> bool {
    // Java: modifiers child containing "static"
    if let Some(modifiers) = find_child_by_kind(node, "modifiers") {
        let mut cursor = modifiers.walk();
        for child in modifiers.children(&mut cursor) {
            if let Ok(text) = child.utf8_text(source.as_bytes())
                && text == "static"
            {
                return true;
            }
        }
    }

    // C#: direct modifier child containing "static"
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier"
            && let Ok(text) = child.utf8_text(source.as_bytes())
            && text == "static"
        {
            return true;
        }
    }

    false
}

/// Checks if a TypeScript/JavaScript method has the `static` keyword.
pub(crate) fn extract_ts_is_static(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "static" {
            return true;
        }
    }
    false
}

/// Checks if a PHP method has the `static_modifier`.
pub(crate) fn extract_php_is_static(node: Node<'_>) -> bool {
    find_child_by_kind(node, "static_modifier").is_some()
}

/// Checks if a Python method has @staticmethod or @classmethod decorator.
pub(crate) fn extract_python_is_static(node: Node<'_>, source: &str) -> bool {
    // Look at the decorated_definition parent if exists
    if let Some(parent) = node.parent()
        && parent.kind() == "decorated_definition"
    {
        let mut cursor = parent.walk();
        for child in parent.children(&mut cursor) {
            if child.kind() == "decorator"
                && let Ok(text) = child.utf8_text(source.as_bytes())
                && (text.contains("staticmethod") || text.contains("classmethod"))
            {
                return true;
            }
        }
    }
    false
}

/// Checks if a Swift method has `static` or `class` modifier.
pub(crate) fn extract_swift_is_static(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                if let Ok(text) = modifier.utf8_text(source.as_bytes())
                    && (text == "static" || text == "class")
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Checks if a Scala method is inside an object (companion object).
pub(crate) fn extract_scala_is_static(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "object_definition" {
            return true;
        }
        // Stop if we hit a class definition
        if parent.kind() == "class_definition" {
            return false;
        }
        current = parent.parent();
    }
    false
}

/// Checks if a C++ method has the `static` specifier.
pub(crate) fn extract_cpp_is_static(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier"
            && let Ok(text) = child.utf8_text(source.as_bytes())
            && text == "static"
        {
            return true;
        }
    }
    false
}

/// Checks if a Rust method is an associated function (no self parameter).
pub(crate) fn extract_rust_is_static(node: Node<'_>, source: &str) -> bool {
    // A Rust method is "static" (associated function) if it doesn't have self parameter
    // Only applies to methods inside impl blocks
    if !is_inside_impl_block(node) {
        return false;
    }

    // Look for parameters node
    if let Some(params) = find_child_by_kind(node, "parameters") {
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            // Check for self_parameter
            if child.kind() == "self_parameter" {
                return false;
            }
            // Check for `self` in the first parameter
            if child.kind() == "parameter"
                && let Some(pattern) = find_child_by_kind(child, "identifier")
                && let Ok(text) = pattern.utf8_text(source.as_bytes())
                && text == "self"
            {
                return false;
            }
        }
        // Has parameters but no self -> associated function
        return true;
    }

    // No parameters at all means it's an associated function
    true
}

/// Checks if a Rust function is inside an impl block.
pub(crate) fn is_inside_impl_block(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "impl_item" {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Extracts return type from Rust function_item.
///
/// Rust AST structure has the return type as a direct child after `->`:
/// ```text
/// function_item
///   parameters (...)
///   ->
///   type_identifier  <-- this is the return type
///   block { ... }
/// ```
///
/// If no explicit return type is present, returns `()` (unit type) since
/// Rust functions without a return annotation implicitly return unit.
pub(crate) fn extract_rust_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_arrow = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }

        // Once we find the arrow, the next type-like node is the return type
        if found_arrow {
            let kind = child.kind();
            // Skip the block - that's the function body, not the return type
            if kind == "block" {
                return None;
            }
            // These are all valid return type node kinds in Rust
            if matches!(
                kind,
                "type_identifier"
                    | "primitive_type"
                    | "reference_type"
                    | "generic_type"
                    | "scoped_type_identifier"
                    | "tuple_type"
                    | "array_type"
                    | "pointer_type"
                    | "function_type"
                    | "unit_type"
                    | "never_type"
                    | "bounded_type"
                    | "dynamic_type"
                    | "abstract_type"
            ) {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }

    // No explicit return type means the function returns () (unit type)
    Some("()".to_string())
}

/// Extracts return type from Python function_definition.
///
/// Python AST structure has the return type after `->`:
/// ```text
/// function_definition
///   parameters (...)
///   ->
///   type  <-- this is the return type (contains identifier like "str")
///   :
///   block: ...
/// ```
pub(crate) fn extract_python_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_arrow = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }

        if found_arrow {
            let kind = child.kind();
            // The colon and block come after the type
            if kind == ":" || kind == "block" {
                return None;
            }
            // The type node contains the actual return type
            if kind == "type" {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }

    None
}

/// Extracts return type from Go function_declaration.
pub(crate) fn extract_go_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // Go return type is in result field or simple_type
    node.child_by_field_name("result")
        .or_else(|| {
            // Find the second parameter_list (return types)
            let mut count = 0;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "parameter_list" {
                    count += 1;
                    if count == 2 {
                        return Some(child);
                    }
                }
            }
            None
        })
        .or_else(|| {
            // Look for type identifier after parameters
            let mut found_params = false;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "parameter_list" {
                    found_params = true;
                    continue;
                }
                if found_params && child.kind() != "block" {
                    return Some(child);
                }
            }
            None
        })
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
}

/// Extracts return type from TypeScript function_declaration.
///
/// TypeScript AST uses `type_annotation` for the return type:
/// ```text
/// function_declaration
///   formal_parameters (...)
///   type_annotation
///     :
///     predefined_type  <-- this is the return type
///   statement_block { ... }
/// ```
pub(crate) fn extract_typescript_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // Find type_annotation that is a direct child (not inside parameters)
    let mut cursor = node.walk();
    let mut found_params = false;

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        // Track when we've passed the formal_parameters
        if kind == "formal_parameters" {
            found_params = true;
            continue;
        }

        // Look for type_annotation after parameters but before body
        if found_params && kind == "type_annotation" {
            // The type_annotation contains ": type", we want just the type
            let mut ta_cursor = child.walk();
            for ta_child in child.children(&mut ta_cursor) {
                if ta_child.kind() != ":" {
                    return ta_child
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                }
            }
        }

        // Stop if we've reached the function body
        if kind == "statement_block" {
            break;
        }
    }

    None
}

// =============================================================================
// PHP extraction functions
// =============================================================================

/// Extracts a PHP parameter from a simple_parameter or property_promotion_parameter node.
///
/// PHP AST structure:
/// ```text
/// simple_parameter
///   primitive_type (string/int/etc.) or type_identifier (ClassName)
///   variable_name
///     $
///     name
///   = (optional, for defaults)
///   value (optional)
/// ```
pub(crate) fn extract_php_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();
    if kind != "simple_parameter"
        && kind != "property_promotion_parameter"
        && kind != "variadic_parameter"
    {
        return None;
    }

    let is_variadic = kind == "variadic_parameter";

    // Find the variable name
    let var_name = find_child_by_kind(node, "variable_name")?;
    let name_node = find_child_by_kind(var_name, "name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

    // Find the type annotation (primitive_type, type_identifier, union_type, etc.)
    let type_annotation = find_php_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Find default value
    let default_value = find_default_value_after_equals(node, source);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Finds a PHP type node among the children.
pub(crate) fn find_php_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const PHP_TYPE_KINDS: &[&str] = &[
        "primitive_type",
        "named_type",
        "optional_type",
        "union_type",
        "intersection_type",
        "type_list",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| PHP_TYPE_KINDS.contains(&child.kind()))
}

/// Extracts return type from PHP function_definition or method_declaration.
///
/// PHP AST structure has return type after `:`:
/// ```text
/// function_definition
///   formal_parameters (...)
///   :
///   primitive_type  <-- return type
///   compound_statement { ... }
/// ```
pub(crate) fn extract_php_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_colon = false;

    for child in node.children(&mut cursor) {
        if child.kind() == ":" {
            found_colon = true;
            continue;
        }

        if found_colon {
            let kind = child.kind();
            // Stop at the function body
            if kind == "compound_statement" {
                return None;
            }
            // These are valid return type node kinds in PHP
            if matches!(
                kind,
                "primitive_type"
                    | "named_type"
                    | "optional_type"
                    | "union_type"
                    | "intersection_type"
            ) {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }

    None
}

// =============================================================================
// Java extraction functions
// =============================================================================

/// Extracts a Java parameter from a formal_parameter node.
///
/// Java AST structure:
/// ```text
/// formal_parameter
///   type_identifier (String) or integral_type (int)
///   identifier (name)
/// ```
pub(crate) fn extract_java_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();
    if kind != "formal_parameter" && kind != "spread_parameter" {
        return None;
    }

    let is_variadic = kind == "spread_parameter";

    // Find the identifier (parameter name)
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation
    let type_annotation = find_java_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None, // Java doesn't support default parameter values
        is_variadic,
    })
}

/// Extracts return type from Java method_declaration.
///
/// Java AST structure has return type before method name:
/// ```text
/// method_declaration
///   modifiers (public)
///   type_identifier  <-- return type
///   identifier (method name)
///   formal_parameters (...)
///   block { ... }
/// ```
pub(crate) fn extract_java_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // In Java, the return type comes before the method name
    // We need to find the type node that appears before the identifier
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        // Skip modifiers
        if kind == "modifiers" || kind == "marker_annotation" || kind == "annotation" {
            continue;
        }

        // If we hit the identifier, we've gone too far
        if kind == "identifier" {
            return None;
        }

        // These are valid return type node kinds in Java
        if matches!(
            kind,
            "type_identifier"
                | "integral_type"
                | "floating_point_type"
                | "boolean_type"
                | "void_type"
                | "generic_type"
                | "array_type"
                | "scoped_type_identifier"
        ) {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }

    None
}

// =============================================================================
// C/C++ extraction functions
// =============================================================================

/// Extracts a C/C++ parameter from a parameter_declaration node.
///
/// C/C++ AST structure:
/// ```text
/// parameter_declaration
///   primitive_type (int/char/etc.)
///   identifier or pointer_declarator
/// ```
pub(crate) fn extract_c_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    if node.kind() != "parameter_declaration" && node.kind() != "variadic_parameter" {
        return None;
    }

    let is_variadic = node.kind() == "variadic_parameter";
    if is_variadic {
        return Some(ParameterInfo {
            name: "...".to_string(),
            type_annotation: None,
            default_value: None,
            is_variadic: true,
        });
    }

    // Find the identifier - could be direct child or inside pointer_declarator/reference_declarator
    let name = find_c_param_name(node, source)?;

    // Find the type annotation
    let type_annotation = find_c_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None,
        is_variadic: false,
    })
}

/// Finds the parameter name in a C/C++ parameter_declaration.
pub(crate) fn find_c_param_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        if kind == "identifier" {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }

        // Handle pointer declarator: *name
        if kind == "pointer_declarator"
            && let Some(id) = find_child_by_kind(child, "identifier")
        {
            return id.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }

        // Handle reference declarator: &name
        if kind == "reference_declarator"
            && let Some(id) = find_child_by_kind(child, "identifier")
        {
            return id.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }
    }

    None
}

/// Extracts return type from C/C++ function_definition.
///
/// C/C++ AST structure has return type as first child of function_definition:
/// ```text
/// function_definition
///   primitive_type  <-- return type (or qualified_identifier for std::string)
///   function_declarator  <-- This may be the context node
///     identifier
///     parameter_list (...)
///   compound_statement { ... }
/// ```
///
/// Note: The context node may be `function_declarator`, so we need to look
/// at the parent `function_definition` to find the return type.
pub(crate) fn extract_c_return_type(node: Node<'_>, source: &str) -> Option<String> {
    // If the node is function_declarator, look at the parent
    let function_node = if node.kind() == "function_declarator" {
        node.parent()?
    } else {
        node
    };

    let mut cursor = function_node.walk();

    for child in function_node.children(&mut cursor) {
        let kind = child.kind();

        // Skip the function declarator and body
        if kind == "function_declarator" || kind == "compound_statement" {
            break;
        }

        // These are valid return type node kinds in C/C++
        if matches!(
            kind,
            "primitive_type"
                | "type_identifier"
                | "sized_type_specifier"
                | "qualified_identifier"
                | "template_type"
        ) {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }

    None
}

// =============================================================================
// C# extraction functions
// =============================================================================

/// Extracts a C# parameter from a parameter node.
///
/// C# AST structure:
/// ```text
/// parameter
///   predefined_type (string/int/etc.) or type_identifier
///   identifier
/// ```
pub(crate) fn extract_csharp_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    if node.kind() != "parameter" {
        return None;
    }

    // Check for params modifier (variadic)
    let is_variadic = find_child_by_kind(node, "parameter_modifier")
        .map(|m| m.utf8_text(source.as_bytes()).ok() == Some("params"))
        .unwrap_or(false);

    // Find the identifier (parameter name)
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation
    let type_annotation = find_csharp_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Find default value
    let default_value = find_child_by_kind(node, "equals_value_clause")
        .and_then(|eq| eq.child(1))
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Extracts return type from C# method_declaration.
///
/// C# AST structure has return type before method name:
/// ```text
/// method_declaration
///   modifier (public)
///   predefined_type  <-- return type
///   identifier (method name)
///   parameter_list (...)
///   block { ... }
/// ```
pub(crate) fn extract_csharp_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();

        // Skip modifiers
        if kind == "modifier" {
            continue;
        }

        // If we hit the identifier, we've gone too far
        if kind == "identifier" {
            return None;
        }

        // These are valid return type node kinds in C#
        if matches!(
            kind,
            "predefined_type" | "generic_name" | "array_type" | "nullable_type" | "qualified_name"
        ) {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }

        // Handle identifier as type (e.g., custom class names)
        // Note: we need to check this after checking it's not the method name
    }

    None
}

// =============================================================================
// Swift extraction functions
// =============================================================================

/// Extracts Swift parameters from a function_declaration.
///
/// Swift has a unique AST structure where parameters are direct children
/// of function_declaration rather than in a separate parameters node:
/// ```text
/// function_declaration
///   func
///   simple_identifier
///   (
///   parameter
///     simple_identifier
///     :
///     user_type
///   ,
///   parameter
///     ...
///   )
///   ->
///   user_type
///   function_body { ... }
/// ```
pub(crate) fn extract_swift_parameters(node: Node<'_>, source: &str) -> Vec<ParameterInfo> {
    let mut parameters = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "parameter"
            && let Some(param) = extract_swift_single_parameter(child, source)
        {
            parameters.push(param);
        }
    }

    parameters
}

/// Extracts a single Swift parameter.
pub(crate) fn extract_swift_single_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    // Swift parameters can have external and internal names
    // We want the internal name (second identifier) or the only identifier
    let identifiers: Vec<_> = {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|c| c.kind() == "simple_identifier")
            .collect()
    };

    // Use the last identifier as the parameter name (internal name)
    let name = identifiers
        .last()
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation (user_type, array_type, etc.)
    let type_annotation = find_swift_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Check for variadic (...)
    let is_variadic = {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|c| c.kind() == "...")
    };

    // Find default value
    let default_value = find_default_value_after_equals(node, source);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Finds a Swift type node among the children.
pub(crate) fn find_swift_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const SWIFT_TYPE_KINDS: &[&str] = &[
        "user_type",
        "array_type",
        "dictionary_type",
        "optional_type",
        "tuple_type",
        "function_type",
        "metatype",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| SWIFT_TYPE_KINDS.contains(&child.kind()))
}

/// Extracts return type from Swift function_declaration.
///
/// Swift AST structure has return type after `->`:
/// ```text
/// function_declaration
///   ...parameters...
///   ->
///   user_type  <-- return type
///   function_body { ... }
/// ```
pub(crate) fn extract_swift_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_arrow = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }

        if found_arrow {
            let kind = child.kind();

            // Stop at the function body
            if kind == "function_body" {
                return None;
            }

            // These are valid return type node kinds in Swift
            if matches!(
                kind,
                "user_type"
                    | "array_type"
                    | "dictionary_type"
                    | "optional_type"
                    | "tuple_type"
                    | "function_type"
                    | "metatype"
            ) {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }

    None
}

// =============================================================================
// Scala extraction functions
// =============================================================================

/// Extracts return type from Scala function_definition.
///
/// Scala AST structure has return type after `:`:
/// ```text
/// function_definition
///   def
///   identifier
///   parameters (...)
///   :
///   type_identifier  <-- return type
///   =
///   block { ... }
/// ```
pub(crate) fn extract_scala_return_type(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_params = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "parameters" {
            found_params = true;
            continue;
        }

        if found_params && child.kind() == ":" {
            continue;
        }

        if found_params {
            let kind = child.kind();

            // Stop at equals or block
            if kind == "=" || kind == "block" {
                return None;
            }

            // These are valid return type node kinds in Scala
            if matches!(
                kind,
                "type_identifier"
                    | "generic_type"
                    | "tuple_type"
                    | "function_type"
                    | "compound_type"
            ) {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }

    None
}

/// Extracts a Scala parameter from a parameter node.
///
/// Scala AST structure:
/// ```text
/// parameter
///   identifier (name)
///   :
///   type_identifier (type)
///   = (optional)
///   value (optional, for defaults)
/// ```
pub(crate) fn extract_scala_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    if node.kind() != "parameter" {
        return None;
    }

    // Find the identifier (parameter name) - first identifier child
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    // Find the type annotation (type_identifier, generic_type, etc.)
    let type_annotation = find_scala_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Check for variadic (*) - Scala uses `name: Type*` for varargs
    let is_variadic = {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .any(|c| c.kind() == "repeated_parameter_type")
    };

    // Find default value
    let default_value = find_default_value_after_equals(node, source);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
    })
}

/// Finds a Scala type node among the children.
pub(crate) fn find_scala_type_node(node: Node<'_>) -> Option<Node<'_>> {
    const SCALA_TYPE_KINDS: &[&str] = &[
        "type_identifier",
        "generic_type",
        "tuple_type",
        "function_type",
        "compound_type",
        "infix_type",
        "repeated_parameter_type",
    ];

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| SCALA_TYPE_KINDS.contains(&child.kind()))
}

// =============================================================================
// Helper functions
// =============================================================================

/// Finds a default value after an `=` sign in a parameter.
pub(crate) fn find_default_value_after_equals(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut found_equals = false;

    for child in node.children(&mut cursor) {
        if child.kind() == "=" {
            found_equals = true;
            continue;
        }

        if found_equals {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }

    None
}
