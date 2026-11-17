//! Wrapper option ownership (pre-clap).
//!
//! A wrapper subcommand (`claudine claude …`) collects every token from its
//! first positional onward into `passthrough` (clap's `trailing_var_arg`), so
//! clap alone would read a Claudine option only when it precedes the prompt.
//! [`hoist_wrapper_options`] removes that dependence on position: every option
//! the wrapper declares ([`OwnedFlags::for_wrapper`], derived from clap), in
//! space, `=`, attached, or short-cluster form, is moved ahead of the first
//! positional, in the order written. A short cluster or an attached short value
//! (`-yq`, `-ojson`) is split into separate tokens (`-y -q`, `-o json`): the
//! wrapper's lenient clap pass (`ignore_errors`) would otherwise drop it into
//! the passthrough instead of reading it. Every other token keeps its order. Nothing
//! at or after the first literal `--` is read or moved: that tail is the
//! provider's.
//!
//! ```text
//! claudine claude "fix it" -m opus --edit --verbose-provider
//! → claudine claude -m opus --edit "fix it" --verbose-provider
//! ```

use std::ffi::OsString;

use super::partition::Ownership;
use super::{OwnedFlags, WRAPPER_SUBCOMMANDS, as_utf8, find_subcommand, is_dash_dash, looks_like_flag};

/// A wrapper option that needs a value had none before the `--` boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingWrapperOptionValue {
    /// The option as written (`-m`, `--operation`).
    pub(crate) option: String,
}

impl std::fmt::Display for MissingWrapperOptionValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let option = &self.option;
        write!(f, "missing value for `{option}`; pass a value like `{option} <VALUE>` before any `--` separator")
    }
}

impl std::error::Error for MissingWrapperOptionValue {}

/// Move every Claudine-owned option of a wrapper invocation ahead of its first
/// positional, so clap parses it wherever it was written.
///
/// Returns `argv` unchanged for any argv that does not name a wrapper
/// subcommand.
///
/// ## Errors
///
/// [`MissingWrapperOptionValue`] when a value-taking option is the last token
/// before the `--` boundary (or of the argv): moving it alone would make it
/// take the prompt as its value.
pub(crate) fn hoist_wrapper_options(argv: Vec<OsString>) -> Result<Vec<OsString>, MissingWrapperOptionValue> {
    let Some((sub_idx, _)) = find_subcommand(&argv, WRAPPER_SUBCOMMANDS) else {
        return Ok(argv);
    };
    let owned = OwnedFlags::for_wrapper();
    let boundary = argv[sub_idx + 1..]
        .iter()
        .position(is_dash_dash)
        .map_or(argv.len(), |offset| sub_idx + 1 + offset);

    let mut options: Vec<OsString> = Vec::new();
    let mut others: Vec<OsString> = Vec::new();
    let mut i = sub_idx + 1;
    while i < boundary {
        let token = &argv[i];
        let Some(text) = as_utf8(token).filter(|text| looks_like_flag(text)) else {
            others.push(token.clone());
            i += 1;
            continue;
        };
        let consumed = match owned.classify(text) {
            Ownership::SelfContained => 1,
            Ownership::ConsumesNext if i + 1 < boundary => 2,
            Ownership::ConsumesNext => {
                return Err(MissingWrapperOptionValue { option: text.to_string() });
            }
            Ownership::Unowned => {
                others.push(token.clone());
                i += 1;
                continue;
            }
        };
        if text.starts_with("--") {
            options.extend(argv[i..i + consumed].iter().cloned());
        } else {
            options.extend(split_short_cluster(&owned, text).into_iter().map(OsString::from));
            options.extend(argv[i + 1..i + consumed].iter().cloned());
        }
        i += consumed;
    }

    let mut hoisted = Vec::with_capacity(argv.len());
    hoisted.extend(argv[..=sub_idx].iter().cloned());
    hoisted.extend(options);
    hoisted.extend(others);
    hoisted.extend(argv[boundary..].iter().cloned());
    Ok(hoisted)
}

/// `-yq` → `-y -q`; `-ojson` → `-o json`; `-yojson` → `-y -o json`. The
/// token is an owned cluster ([`OwnedFlags::classify`] did not call it
/// unowned), so every short before a value-taking one is boolean.
fn split_short_cluster(owned: &OwnedFlags, token: &str) -> Vec<String> {
    let mut split = Vec::new();
    for (offset, short) in token.char_indices().skip(1) {
        let flag = format!("-{short}");
        let value_taking = owned.consumes_next(&flag);
        split.push(flag);
        if value_taking {
            let value = &token[offset + short.len_utf8()..];
            if !value.is_empty() {
                split.push(value.to_string());
            }
            break;
        }
    }
    split
}

#[cfg(test)]
mod tests;
