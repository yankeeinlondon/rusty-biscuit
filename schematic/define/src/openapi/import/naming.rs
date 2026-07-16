//! Naming utilities for OpenAPI import.
//!
//! This module provides functions for converting OpenAPI identifiers
//! to valid Rust identifiers, including case conversion and deconfliction.

use std::collections::HashSet;

/// Rust reserved keywords that cannot be used as identifiers.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Converts a string to PascalCase.
///
/// Handles various input formats:
/// - snake_case: `list_users` -> `ListUsers`
/// - kebab-case: `list-users` -> `ListUsers`
/// - camelCase: `listUsers` -> `ListUsers`
/// - SCREAMING_SNAKE: `LIST_USERS` -> `ListUsers`
/// - Already PascalCase: `ListUsers` -> `ListUsers`
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::naming::to_pascal_case;
///
/// assert_eq!(to_pascal_case("list_users"), "ListUsers");
/// assert_eq!(to_pascal_case("list-users"), "ListUsers");
/// assert_eq!(to_pascal_case("listUsers"), "ListUsers");
/// assert_eq!(to_pascal_case("LIST_USERS"), "ListUsers");
/// ```
pub fn to_pascal_case(s: &str) -> String {
    // Split by common separators first
    let words: Vec<&str> = s
        .split(['_', '-', ' ', '/'])
        .filter(|w| !w.is_empty())
        .collect();

    let mut result = String::new();

    for word in words {
        // Check if word is all uppercase (SCREAMING case)
        let is_all_uppercase = word.chars().all(|c| !c.is_ascii_lowercase());

        if is_all_uppercase {
            // Convert WORD to Word
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
            }
            for c in chars {
                result.push(c.to_ascii_lowercase());
            }
        } else {
            // Process camelCase or PascalCase
            let chars = word.chars();
            let mut is_first = true;

            for c in chars {
                if is_first {
                    result.push(c.to_ascii_uppercase());
                    is_first = false;
                } else if c.is_ascii_uppercase() {
                    // This is a camelCase boundary - preserve the uppercase
                    result.push(c);
                } else {
                    result.push(c);
                }
            }
        }
    }

    result
}

/// Deconflicts a name by appending a numeric suffix if it already exists.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::naming::deconflict_name;
/// use std::collections::HashSet;
///
/// let mut existing = HashSet::new();
/// existing.insert("User".to_string());
/// existing.insert("User2".to_string());
///
/// let result = deconflict_name("User", &existing);
/// assert_eq!(result, "User3");
/// ```
pub fn deconflict_name(name: &str, existing: &HashSet<String>) -> String {
    if !existing.contains(name) {
        return name.to_string();
    }

    // Try numeric suffixes
    for i in 2..=1000 {
        let candidate = format!("{}{}", name, i);
        if !existing.contains(&candidate) {
            return candidate;
        }
    }

    // Fallback: append random-ish suffix
    format!("{}_{}", name, existing.len())
}

/// Sanitizes a string to be a valid Rust identifier.
///
/// - Removes leading digits
/// - Replaces invalid characters with underscores
/// - Handles reserved keywords by appending `_`
/// - Ensures the result is non-empty
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::naming::sanitize_rust_ident;
///
/// assert_eq!(sanitize_rust_ident("123abc"), "Abc");
/// assert_eq!(sanitize_rust_ident("my-name"), "MyName");
/// assert_eq!(sanitize_rust_ident("type"), "Type_");
/// assert_eq!(sanitize_rust_ident(""), "Unknown");
/// ```
pub fn sanitize_rust_ident(s: &str) -> String {
    if s.is_empty() {
        return "Unknown".to_string();
    }

    // First convert to PascalCase
    let pascal = to_pascal_case(s);

    // Remove leading digits
    let trimmed: String = pascal.chars().skip_while(|c| c.is_ascii_digit()).collect();

    // If empty after trimming, use fallback
    // Also ensure first char is uppercase after removing digits
    let base = if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        let mut chars = trimmed.chars();
        let first = chars.next().unwrap().to_ascii_uppercase();
        let rest: String = chars.collect();
        format!("{}{}", first, rest)
    };

    // Replace any remaining invalid characters
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Handle reserved keywords
    let lower = cleaned.to_lowercase();
    if RUST_KEYWORDS.contains(&lower.as_str()) {
        format!("{}_", cleaned)
    } else {
        cleaned
    }
}

/// Sanitizes a string to be a valid Rust struct-field identifier.
///
/// Converts to `snake_case`, then guarantees a usable identifier by handling
/// the three ways snake-casing alone can fall short: Rust keywords (append
/// `_`, e.g. `type` -> `type_`), a leading digit (prepend `_`, e.g. `2fa` ->
/// `_2fa`), and empty input (fall back to `field`).
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::naming::sanitize_rust_field_ident;
///
/// assert_eq!(sanitize_rust_field_ident("userName"), "user_name");
/// assert_eq!(sanitize_rust_field_ident("type"), "type_");
/// assert_eq!(sanitize_rust_field_ident("self"), "self_");
/// assert_eq!(sanitize_rust_field_ident("2fa"), "_2fa");
/// assert_eq!(sanitize_rust_field_ident(""), "field");
/// ```
pub fn sanitize_rust_field_ident(s: &str) -> String {
    let snake = to_snake_case(s);

    // Replace any residual non-identifier characters (e.g. '@', '.').
    let cleaned: String = snake
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if cleaned.is_empty() {
        return "field".to_string();
    }

    // A leading digit is the one snake_case artifact that is not a valid
    // identifier start; prefix it rather than dropping information.
    let cleaned = if cleaned.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{cleaned}")
    } else {
        cleaned
    };

    if RUST_KEYWORDS.contains(&cleaned.as_str()) {
        format!("{cleaned}_")
    } else {
        cleaned
    }
}

/// Generates a fallback operation ID from method and path.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::naming::operation_id_fallback;
///
/// assert_eq!(operation_id_fallback("GET", "/pets"), "GetPets");
/// assert_eq!(operation_id_fallback("POST", "/users/{id}"), "PostUsersId");
/// assert_eq!(operation_id_fallback("DELETE", "/items/{item_id}/tags"), "DeleteItemsItemIdTags");
/// ```
pub fn operation_id_fallback(method: &str, path: &str) -> String {
    let method_part = to_pascal_case(method);

    // Clean up path: remove braces, convert to PascalCase
    let path_cleaned = path
        .replace(['{', '}'], "")
        .replace('/', " ")
        .trim()
        .to_string();

    let path_part = to_pascal_case(&path_cleaned);

    format!("{}{}", method_part, path_part)
}

/// Converts a string to snake_case for field names.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::naming::to_snake_case;
///
/// assert_eq!(to_snake_case("userName"), "user_name");
/// assert_eq!(to_snake_case("UserName"), "user_name");
/// assert_eq!(to_snake_case("user-name"), "user_name");
/// assert_eq!(to_snake_case("USER_NAME"), "user_name");
/// ```
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_lower = false;

    for (i, c) in s.chars().enumerate() {
        if c == '-' || c == '_' || c == ' ' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            prev_lower = false;
        } else if c.is_ascii_uppercase() {
            // Insert underscore before uppercase if prev was lowercase
            if prev_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            // Handle case where we're in an acronym (e.g., "XMLParser" -> "xml_parser")
            // We've already lowercased consecutive uppercase, but need to handle
            // the transition from uppercase sequence to lowercase
            if i > 0 && result.ends_with(|c: char| c.is_ascii_lowercase()) {
                // Normal lowercase continuation
            }
            result.push(c.to_ascii_lowercase());
            prev_lower = c.is_ascii_lowercase();
        }
    }

    // Clean up multiple consecutive underscores
    let mut cleaned = String::new();
    let mut prev_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_underscore && !cleaned.is_empty() {
                cleaned.push(c);
            }
            prev_underscore = true;
        } else {
            cleaned.push(c);
            prev_underscore = false;
        }
    }

    // Remove trailing underscore
    cleaned.trim_end_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== to_pascal_case Tests ==========

    #[test]
    fn pascal_case_from_snake() {
        assert_eq!(to_pascal_case("list_users"), "ListUsers");
        assert_eq!(to_pascal_case("get_user_by_id"), "GetUserById");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }

    #[test]
    fn pascal_case_from_kebab() {
        assert_eq!(to_pascal_case("list-users"), "ListUsers");
        assert_eq!(to_pascal_case("get-user-by-id"), "GetUserById");
    }

    #[test]
    fn pascal_case_from_camel() {
        assert_eq!(to_pascal_case("listUsers"), "ListUsers");
        assert_eq!(to_pascal_case("getUserById"), "GetUserById");
    }

    #[test]
    fn pascal_case_from_screaming() {
        assert_eq!(to_pascal_case("LIST_USERS"), "ListUsers");
        assert_eq!(to_pascal_case("GET_USER"), "GetUser");
    }

    #[test]
    fn pascal_case_already_pascal() {
        assert_eq!(to_pascal_case("ListUsers"), "ListUsers");
        assert_eq!(to_pascal_case("GetUser"), "GetUser");
    }

    #[test]
    fn pascal_case_empty() {
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn pascal_case_single_word() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("USER"), "User");
    }

    #[test]
    fn pascal_case_with_numbers() {
        assert_eq!(to_pascal_case("user_v2"), "UserV2");
        assert_eq!(to_pascal_case("get123"), "Get123");
    }

    #[test]
    fn pascal_case_with_slashes() {
        assert_eq!(to_pascal_case("pets/list"), "PetsList");
    }

    // ========== deconflict_name Tests ==========

    #[test]
    fn deconflict_no_conflict() {
        let existing = HashSet::new();
        assert_eq!(deconflict_name("User", &existing), "User");
    }

    #[test]
    fn deconflict_single_conflict() {
        let mut existing = HashSet::new();
        existing.insert("User".to_string());

        assert_eq!(deconflict_name("User", &existing), "User2");
    }

    #[test]
    fn deconflict_multiple_conflicts() {
        let mut existing = HashSet::new();
        existing.insert("User".to_string());
        existing.insert("User2".to_string());
        existing.insert("User3".to_string());

        assert_eq!(deconflict_name("User", &existing), "User4");
    }

    #[test]
    fn deconflict_different_name() {
        let mut existing = HashSet::new();
        existing.insert("User".to_string());

        assert_eq!(deconflict_name("Pet", &existing), "Pet");
    }

    // ========== sanitize_rust_ident Tests ==========

    #[test]
    fn sanitize_removes_leading_digits() {
        assert_eq!(sanitize_rust_ident("123abc"), "Abc");
        assert_eq!(sanitize_rust_ident("1user"), "User");
    }

    #[test]
    fn sanitize_handles_hyphens() {
        assert_eq!(sanitize_rust_ident("my-name"), "MyName");
    }

    #[test]
    fn sanitize_handles_reserved_words() {
        assert_eq!(sanitize_rust_ident("type"), "Type_");
        assert_eq!(sanitize_rust_ident("struct"), "Struct_");
        assert_eq!(sanitize_rust_ident("enum"), "Enum_");
    }

    #[test]
    fn sanitize_empty_string() {
        assert_eq!(sanitize_rust_ident(""), "Unknown");
    }

    #[test]
    fn sanitize_only_digits() {
        assert_eq!(sanitize_rust_ident("123"), "Unknown");
    }

    #[test]
    fn sanitize_valid_ident() {
        assert_eq!(sanitize_rust_ident("User"), "User");
        assert_eq!(sanitize_rust_ident("MyType"), "MyType");
    }

    #[test]
    fn sanitize_special_characters() {
        // @ is replaced with _ after PascalCase conversion
        assert_eq!(sanitize_rust_ident("user@email"), "User_email");
        // + is replaced with _ after PascalCase conversion
        assert_eq!(sanitize_rust_ident("a+b"), "A_b");
    }

    // ========== sanitize_rust_field_ident Tests ==========

    #[test]
    fn sanitize_field_snake_cases() {
        assert_eq!(sanitize_rust_field_ident("userName"), "user_name");
        assert_eq!(sanitize_rust_field_ident("User-Name"), "user_name");
    }

    #[test]
    fn sanitize_field_appends_underscore_for_keywords() {
        assert_eq!(sanitize_rust_field_ident("type"), "type_");
        assert_eq!(sanitize_rust_field_ident("self"), "self_");
        assert_eq!(sanitize_rust_field_ident("async"), "async_");
    }

    #[test]
    fn sanitize_field_prefixes_leading_digit() {
        assert_eq!(sanitize_rust_field_ident("2fa"), "_2fa");
    }

    #[test]
    fn sanitize_field_empty_falls_back() {
        assert_eq!(sanitize_rust_field_ident(""), "field");
    }

    // ========== operation_id_fallback Tests ==========

    #[test]
    fn operation_fallback_simple() {
        assert_eq!(operation_id_fallback("GET", "/pets"), "GetPets");
        assert_eq!(operation_id_fallback("POST", "/users"), "PostUsers");
    }

    #[test]
    fn operation_fallback_with_path_param() {
        assert_eq!(operation_id_fallback("GET", "/pets/{id}"), "GetPetsId");
        assert_eq!(
            operation_id_fallback("DELETE", "/users/{user_id}"),
            "DeleteUsersUserId"
        );
    }

    #[test]
    fn operation_fallback_nested_path() {
        assert_eq!(
            operation_id_fallback("GET", "/users/{id}/posts"),
            "GetUsersIdPosts"
        );
    }

    #[test]
    fn operation_fallback_root() {
        assert_eq!(operation_id_fallback("GET", "/"), "Get");
    }

    // ========== to_snake_case Tests ==========

    #[test]
    fn snake_case_from_camel() {
        assert_eq!(to_snake_case("userName"), "user_name");
        assert_eq!(to_snake_case("getUserById"), "get_user_by_id");
    }

    #[test]
    fn snake_case_from_pascal() {
        assert_eq!(to_snake_case("UserName"), "user_name");
        assert_eq!(to_snake_case("GetUser"), "get_user");
    }

    #[test]
    fn snake_case_from_kebab() {
        assert_eq!(to_snake_case("user-name"), "user_name");
    }

    #[test]
    fn snake_case_from_screaming() {
        assert_eq!(to_snake_case("USER_NAME"), "user_name");
    }

    #[test]
    fn snake_case_already_snake() {
        assert_eq!(to_snake_case("user_name"), "user_name");
    }

    #[test]
    fn snake_case_single_word() {
        assert_eq!(to_snake_case("user"), "user");
        assert_eq!(to_snake_case("User"), "user");
    }

    #[test]
    fn snake_case_empty() {
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn snake_case_with_numbers() {
        assert_eq!(to_snake_case("userV2"), "user_v2");
    }
}
