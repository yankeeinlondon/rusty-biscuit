//! Pure token-shape predicates for the completion classifier.
//!
//! Each function classifies a single argv token by its lexical shape — flag,
//! value-bearing flag, `name=value` setter, or setter-name partial — with no
//! dependency on engine state. Several mirror shapes defined in
//! [`crate::argv`]; they are duplicated here so the classifier stays
//! self-contained (see the individual notes).

/// Split a `name=value` token into its parts. Returns `None` when the token
/// shape does not match `^[A-Za-z_][A-Za-z0-9_-]*=`.
pub(super) fn split_setter(token: &str) -> Option<(&str, &str)> {
    let eq_pos = token.find('=')?;
    if eq_pos == 0 {
        return None;
    }
    let bytes = token.as_bytes();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return None;
    }
    if !bytes[1..eq_pos]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return None;
    }
    Some((&token[..eq_pos], &token[eq_pos + 1..]))
}

/// `^[A-Za-z_][A-Za-z0-9_-]*$` — the partial-name shape recognized as a
/// setter-name candidate (e.g. `tit`, `prompt_for`, `count-down`). Differs
/// from [`is_setter_shaped`] in that there is no `=` separator: the user
/// has not yet typed past the name.
pub(super) fn is_setter_name_partial(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let bytes = token.as_bytes();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

pub(super) fn is_flag_token(token: &str) -> bool {
    token.starts_with('-') && token != "-" && token != "--"
}

/// Value-bearing flag surface for composition subcommands. Mirrors
/// [`crate::argv::COMPOSITION_FLAGS_WITH_VALUE`] but is duplicated here so
/// the classifier stays self-contained.
pub(super) fn is_value_bearing_flag(token: &str) -> bool {
    matches!(
        token,
        "--provider"
            | "--exclude"
            | "--include"
            | "--model"
            | "-m"
            | "--output"
            | "-o"
            | "--append-system-prompt"
            | "--asp"
            | "--replace-system-prompt"
            | "--rsp"
            | "--timeout"
            | "-t"
            | "--operation"
            | "--op"
            | "--set"
            | "--use"
            | "--fail-fast"
    )
}

/// `^[A-Za-z_][A-Za-z0-9_-]*=` — same shape as
/// [`crate::argv::looks_like_setter`]. Duplicated here so the engine stays
/// self-contained.
pub(super) fn is_setter_shaped(token: &str) -> bool {
    let Some(eq_pos) = token.find('=') else {
        return false;
    };
    if eq_pos == 0 {
        return false;
    }
    let bytes = token.as_bytes();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return false;
    }
    bytes[1..eq_pos]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

pub(super) fn is_global_bool_flag(token: &str) -> bool {
    matches!(
        token,
        "--plain" | "--verbose" | "-v" | "-vv" | "-vvv" | "--help" | "-h"
    )
}
