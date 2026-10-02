//! Pure token-shape predicates for the completion classifier.
//!
//! Each function classifies a single argv token by its lexical shape — flag,
//! `name=value` setter, or setter-name partial — with no dependency on engine
//! state. Which Claudine options take a value comes from the clap definitions
//! ([`crate::argv::OwnedFlags`]), never from a list here. The setter shapes
//! read the one setter grammar, [`claudine::composition::setter_key`].

use claudine::composition::{is_setter_name, setter_key};

/// Split a `name=value` token into its parts at the first `=`. Returns `None`
/// when the token is not setter-shaped.
pub(super) fn split_setter(token: &str) -> Option<(&str, &str)> {
    let key = setter_key(token)?;
    Some((key, &token[key.len() + 1..]))
}

/// The partial-name shape recognized as a setter-name candidate (e.g. `tit`,
/// `prompt_for`, `count-down`). Differs from [`is_setter_shaped`] in that
/// there is no `=` separator: the user has not yet typed past the name.
pub(super) fn is_setter_name_partial(token: &str) -> bool {
    is_setter_name(token)
}

pub(super) fn is_flag_token(token: &str) -> bool {
    token.starts_with('-') && token != "-" && token != "--"
}

/// A `name=value` setter token (the shape [`crate::argv::looks_like_setter`]
/// also tests).
pub(super) fn is_setter_shaped(token: &str) -> bool {
    setter_key(token).is_some()
}

pub(super) fn is_global_bool_flag(token: &str) -> bool {
    matches!(
        token,
        "--plain" | "--verbose" | "-v" | "-vv" | "-vvv" | "--help" | "-h"
    )
}
