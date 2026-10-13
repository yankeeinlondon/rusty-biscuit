//! Function signature, parameter, and return-type extraction helpers carved
//! out of `tree_file.rs`. Visibility and `static` live in the `visibility` and
//! `static_flags` modules, which read any declaration node.

use super::*;
use crate::file::parents::IndexedParent as _;

/// Extracts function signature from a function/method node.
///
/// Always returns a signature, with an empty parameter list when there are
/// none, so a parameterless function renders as `tick()`. `name` is the
/// captured name of the function, which selects the function a C/C++
/// declaration declares. `is_static` is the symbol's shared flag; the
/// signature only carries a copy of it.
pub(crate) fn extract_signature(
    node: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
    is_static: bool,
) -> FunctionSignature {
    FunctionSignature {
        parameters: extract_parameters(node, name, language, source),
        return_type: extract_return_type(node, language, source),
        is_static,
    }
}

/// Extracts parameters from a function node.
///
/// For C/C++ the parameters are those of the function `name` declares (see
/// [`declared_function_named`]), never those of a function type wrapped
/// around it: `int (*f(int a))(long b)` has the parameter `a`. A
/// declaration that declares no function named `name` has none.
pub(crate) fn extract_parameters(
    node: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<ParameterInfo> {
    declared_parameters(node, name, language, source).unwrap_or_default()
}

/// The parameters of the declaration's own parameter list (every list of a
/// curried Scala `def`), or `None` when no parameter list was found: the
/// declaration has none (a JavaScript name bound to a function value), or
/// the language's parameters are not extracted (Perl, Bash, Zsh). `Some` of
/// an empty list is a known empty list. A parameter bound by a pattern is
/// listed under the pattern's text; only C/C++ and Go leave unnamed
/// parameters out (see [`declared_parameter_count`]).
pub(crate) fn declared_parameters(
    node: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<Vec<ParameterInfo>> {
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
        // A Swift function's parameters are children of the declaration,
        // which always has its parentheses.
        ProgrammingLanguage::Swift => return Some(extract_swift_parameters(node, source)),
        ProgrammingLanguage::Lua => {
            return node
                .child_by_field_name("parameters")
                .map(|_| extract_lua_parameters(node, source));
        }
        _ => return None,
    };

    // An arrow function's lone unparenthesized parameter (`x => x`).
    if matches!(language, ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript)
        && let Some(parameter) = node.child_by_field_name("parameter")
    {
        let name = parameter.utf8_text(source.as_bytes()).ok()?;
        return Some(vec![ParameterInfo::new(name)]);
    }

    // A curried Scala `def f(a: A)(b: B)` declares every list's
    // parameters, `implicit`/`using` lists included.
    let lists: Vec<Node<'_>> = if language == ProgrammingLanguage::Scala {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == params_node_kind)
            .collect()
    } else {
        vec![parameter_list_node(node, name, language, params_node_kind)?]
    };
    if lists.is_empty() {
        return None;
    }

    let mut parameters = Vec::new();
    for list in lists {
        let mut cursor = list.walk();
        for child in list.children(&mut cursor) {
            // Go needs special handling: a single parameter_declaration can define
            // multiple parameters (e.g., `a, b int` defines both `a` and `b`)
            if language == ProgrammingLanguage::Go {
                parameters.extend(extract_go_parameters(child, source));
            } else if let Some(param) = extract_single_parameter(child, language, source) {
                parameters.push(param);
            }
        }
    }

    Some(parameters)
}

/// The node holding the parameters of `node`'s own list, of kind
/// `list_kind`: for a Go method the list after the receiver, for C/C++ the
/// list of the function `name` declares.
fn parameter_list_node<'tree>(
    node: Node<'tree>,
    name: Node<'tree>,
    language: ProgrammingLanguage,
    list_kind: &str,
) -> Option<Node<'tree>> {
    // A Go method has two parameter lists, the receiver first.
    if language == ProgrammingLanguage::Go && node.kind() == "method_declaration" {
        find_nth_child_by_kind(node, list_kind, 1) // 0-indexed, so 1 = second
    } else if matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp) {
        declared_function_named(node, name)
            .and_then(|declared| declared.function.child_by_field_name("parameters"))
    } else {
        find_child_by_kind(node, list_kind)
    }
}

/// The number of parameters a C, C++ or Go declaration declares, named or
/// not (`int f(int, char *)` declares two, which the signature cannot list
/// by name); `(void)` declares none. `None` for another language, or when
/// the declaration has no parameter list.
pub(crate) fn declared_parameter_count(
    node: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<usize> {
    if !matches!(
        language,
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp | ProgrammingLanguage::Go
    ) {
        return None;
    }
    let list = parameter_list_node(node, name, language, "parameter_list")?;
    let mut cursor = list.walk();
    let children: Vec<Node<'_>> = list.children(&mut cursor).collect();
    if language == ProgrammingLanguage::Go {
        // `a, b int` declares two; `int` alone declares one.
        return Some(
            children
                .iter()
                .map(|child| match child.kind() {
                    "parameter_declaration" => {
                        let mut names = child.walk();
                        child
                            .children_by_field_name("name", &mut names)
                            .count()
                            .max(1)
                    }
                    "variadic_parameter_declaration" => 1,
                    _ => 0,
                })
                .sum(),
        );
    }
    let declarations: Vec<&Node<'_>> = children
        .iter()
        .filter(|child| {
            matches!(
                child.kind(),
                "parameter_declaration"
                    | "optional_parameter_declaration"
                    | "variadic_parameter_declaration"
                    | "variadic_parameter"
                    | "..."
            )
        })
        .collect();
    let is_void = |child: &Node<'_>| {
        child.kind() == "parameter_declaration"
            && child.utf8_text(source.as_bytes()).is_ok_and(|text| text.trim() == "void")
    };
    Some(match declarations.as_slice() {
        [only] if is_void(only) => 0,
        all => all.len(),
    })
}

/// Extracts a single parameter from a parameter node.
///
/// Note: Go is handled separately in `declared_parameters` using `extract_go_parameters`
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
        // Go is handled specially in declared_parameters
        // Swift is handled specially via extract_swift_parameters
        _ => None,
    }
}

/// A Rust parameter is named by its whole pattern as written (`(a, b)`,
/// `_`, `S { x }`), so a destructuring or wildcard parameter is listed and
/// counted like an identifier. A receiver (`&self`, or a `self: Box<Self>`
/// pattern) is named `self`.
pub(crate) fn extract_rust_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();

    if kind == "self_parameter" {
        return Some(ParameterInfo::new("self"));
    }

    // C-variadic `...` in a foreign function, named by its pattern if any.
    if kind == "variadic_parameter" {
        let name = node
            .child_by_field_name("pattern")
            .and_then(|pattern| pattern.utf8_text(source.as_bytes()).ok())
            .unwrap_or("...");
        return Some(ParameterInfo {
            name: name.to_string(),
            type_annotation: None,
            default_value: None,
            is_variadic: true,
        });
    }

    if kind != "parameter" {
        return None;
    }

    let name = node
        .child_by_field_name("pattern")
        .and_then(|pattern| pattern.utf8_text(source.as_bytes()).ok())?
        .to_string();

    let type_annotation = node
        .child_by_field_name("type")
        .or_else(|| find_rust_type_node(node))
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

/// A JavaScript/TypeScript parameter is named by its binding as written:
/// an identifier, or a whole object/array pattern (`{x}`, `[a, b]`); a rest
/// parameter by what follows `...`. A TypeScript `this` parameter declares
/// the receiver's type, not an argument, and is skipped.
pub(crate) fn extract_js_parameter(
    node: Node<'_>,
    kind: &str,
    source: &str,
) -> Option<ParameterInfo> {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).ok().map(str::to_string);

    let (binding, default_value) = match kind {
        "identifier" | "object_pattern" | "array_pattern" | "rest_pattern" => (node, None),
        "assignment_pattern" => (
            node.child_by_field_name("left")?,
            node.child_by_field_name("right").and_then(text),
        ),
        "required_parameter" | "optional_parameter" => (
            node.child_by_field_name("pattern")?,
            node.child_by_field_name("value").and_then(text),
        ),
        _ => return None,
    };
    if binding.kind() == "this" {
        return None;
    }

    let is_variadic = binding.kind() == "rest_pattern";
    let name = if is_variadic {
        binding.named_child(0).and_then(text)?
    } else {
        text(binding)?
    };

    // The `type` field is the whole `: T` annotation.
    let type_annotation = node
        .child_by_field_name("type")
        .and_then(|annotation| annotation.named_child(0))
        .and_then(text);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic,
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

/// The declared return type (the `return_type` field after `->`); `None`
/// when the function declares none, never an inferred `()`.
pub(crate) fn extract_rust_return_type(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|return_type| return_type.utf8_text(source.as_bytes()).ok())
        .map(str::to_string)
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

/// The declared result of a Go function or method (the `result` field: a
/// type or a parenthesized list); `None` when it declares none. A method's
/// receiver and parameter lists are never its result.
pub(crate) fn extract_go_return_type(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("result")
        .and_then(|result| result.utf8_text(source.as_bytes()).ok())
        .map(str::to_string)
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

/// Extracts a PHP parameter from a `simple_parameter`,
/// `property_promotion_parameter` or `variadic_parameter` node; a default
/// is the grammar's `default_value` field, never a comment beside the `=`.
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

    let default_value = node
        .child_by_field_name("default_value")
        .and_then(|value| value.utf8_text(source.as_bytes()).ok())
        .map(str::to_string);

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

    // A spread parameter names itself through a variable declarator.
    let name = find_child_by_kind(node, "identifier")
        .or_else(|| {
            find_child_by_kind(node, "variable_declarator")
                .and_then(|declarator| declarator.child_by_field_name("name"))
        })
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

/// Extracts one C/C++ parameter from a child of a `parameter_list`.
///
/// A `parameter_declaration`, a C++ `optional_parameter_declaration` (with
/// its default value) and a C++ pack (`Ts... values`, variadic) are named by
/// their own declarator (see [`declarator_name`]); an unnamed one is
/// skipped. A C `variadic_parameter` and the C++ `...` token are the
/// variadic tail, named `...`.
pub(crate) fn extract_c_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    let kind = node.kind();
    if matches!(kind, "variadic_parameter" | "...") {
        return Some(ParameterInfo {
            name: "...".to_string(),
            type_annotation: None,
            default_value: None,
            is_variadic: true,
        });
    }
    if !matches!(
        kind,
        "parameter_declaration" | "optional_parameter_declaration" | "variadic_parameter_declaration"
    ) {
        return None;
    }

    let name = find_c_param_name(node, source)?;
    let type_annotation = find_c_type_node(node)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());
    let default_value = node
        .child_by_field_name("default_value")
        .and_then(|value| value.utf8_text(source.as_bytes()).ok())
        .map(str::to_string);

    Some(ParameterInfo {
        name,
        type_annotation,
        default_value,
        is_variadic: kind == "variadic_parameter_declaration",
    })
}

/// The name a C/C++ parameter declares, read from its own declarator.
pub(crate) fn find_c_param_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("declarator")
        .and_then(declarator_name)
        .filter(|name| name.kind() == "identifier")
        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
        .map(str::to_string)
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
        node.indexed_parent()?
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

/// Extracts a C# parameter from a `parameter` node, or a `params` array
/// from its `params` token.
///
/// A default (`int y = 2`) is the expression after the `=` that follows
/// the name; comments around the `=` neither hide nor supply one, and none
/// is taken for a `params` array's type or name.
pub(crate) fn extract_csharp_parameter(node: Node<'_>, source: &str) -> Option<ParameterInfo> {
    // The grammar spells a `params` array as three children of the list:
    // the `params` token, then its type and its name.
    if node.kind() == "params" {
        let type_node = next_named_syntax_sibling(node)?;
        let name = next_named_syntax_sibling(type_node)
            .filter(|name| name.kind() == "identifier")?
            .utf8_text(source.as_bytes())
            .ok()?;
        return Some(ParameterInfo {
            name: name.to_string(),
            type_annotation: type_node.utf8_text(source.as_bytes()).ok().map(str::to_string),
            default_value: None,
            is_variadic: true,
        });
    }
    if node.kind() != "parameter" {
        return None;
    }

    // Check for params modifier (variadic)
    let is_variadic = find_child_by_kind(node, "parameter_modifier")
        .map(|m| m.utf8_text(source.as_bytes()).ok() == Some("params"))
        .unwrap_or(false);

    // A named type (`W x`) and a bare default (`int y = external`) are
    // identifiers too, so only the fields say which one is the name.
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())?;

    let type_annotation = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // The grammar gives a default no field: it is the expression after the
    // `=` that follows the name. Comments are children here too, so neither
    // step may land on one.
    let default_value = node
        .child_by_field_name("name")
        .and_then(next_syntax_sibling)
        .filter(|equals| equals.kind() == "=")
        .and_then(next_named_syntax_sibling)
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
    // The field also covers a user-defined type (`PublicWidget Make()`),
    // whose `identifier` the child scan below cannot tell from the name.
    if let Some(return_type) = node
        .child_by_field_name("returns")
        .or_else(|| node.child_by_field_name("type"))
    {
        return return_type
            .utf8_text(source.as_bytes())
            .ok()
            .map(str::to_string);
    }

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
    }

    None
}

// =============================================================================
// Swift extraction functions
// =============================================================================

/// Extracts Swift parameters from a `function_declaration`,
/// `init_declaration` or `protocol_function_declaration`.
///
/// Swift has a unique AST structure where parameters are direct children
/// of the declaration rather than in a separate parameters node:
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
///
/// A default (`y: Int = 2`) follows its parameter as a sibling: `=`, then
/// the declaration's `default_value` field.
pub(crate) fn extract_swift_parameters(node: Node<'_>, source: &str) -> Vec<ParameterInfo> {
    let mut parameters: Vec<ParameterInfo> = Vec::new();
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return parameters;
    }
    loop {
        let child = cursor.node();
        if child.kind() == "parameter" {
            if let Some(param) = extract_swift_single_parameter(child, source) {
                parameters.push(param);
            }
        } else if cursor.field_name() == Some("default_value")
            && let Some(last) = parameters.last_mut()
            && last.default_value.is_none()
        {
            last.default_value = child.utf8_text(source.as_bytes()).ok().map(str::to_string);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    parameters
}

/// Extracts a single Swift parameter.
pub(crate) fn extract_swift_single_parameter(
    node: Node<'_>,
    source: &str,
) -> Option<ParameterInfo> {
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

    // The grammar's `parameter` never holds `= value`: a default is a
    // field of the enclosing declaration, read by
    // [`extract_swift_parameters`].
    Some(ParameterInfo {
        name,
        type_annotation,
        default_value: None,
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

/// Extracts a Scala parameter from a `parameter` node; a default is the
/// grammar's `default_value` field, never a comment beside the `=`.
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

    let default_value = node
        .child_by_field_name("default_value")
        .and_then(|value| value.utf8_text(source.as_bytes()).ok())
        .map(str::to_string);

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

/// Extracts Lua parameters: plain names, plus `...` as a variadic parameter.
pub(crate) fn extract_lua_parameters(node: Node<'_>, source: &str) -> Vec<ParameterInfo> {
    let Some(params) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };

    let mut cursor = params.walk();
    params
        .named_children(&mut cursor)
        .filter_map(|child| match child.kind() {
            "identifier" => Some(ParameterInfo {
                name: child.utf8_text(source.as_bytes()).ok()?.to_string(),
                type_annotation: None,
                default_value: None,
                is_variadic: false,
            }),
            "vararg_expression" => Some(ParameterInfo {
                name: "...".to_string(),
                type_annotation: None,
                default_value: None,
                is_variadic: true,
            }),
            _ => None,
        })
        .collect()
}
