//! Decides whether a `#[cfg(...)]` predicate compiles its item only in a test
//! build, for the source guards that must tell shipped code from test code.
//!
//! The predicate is evaluated, not searched for words: it is test-only when
//! every configuration that satisfies it has `test` set or enables one of
//! [`TEST_ONLY_FEATURES`]. `all(...)` qualifies when any child does, `any(...)`
//! only when every child does, and `not(...)` never does. A malformed
//! predicate is an error rather than a verdict. `cfg_attr` does not decide
//! whether an item is compiled, so it is never a gate.

use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Lit, Meta, Token};

/// Features that no default build or `cargo install` enables. Each is declared
/// by `claudine/cli/Cargo.toml` or `claudine/lib/Cargo.toml` and is turned on
/// only by a test recipe.
pub(crate) const TEST_ONLY_FEATURES: &[&str] = &[
    "test-fixtures",
    "terminal-tests",
    "daemon-tests",
    "real-tests",
];

/// Whether the attributes of one item compile it only in a test build.
/// Several `#[cfg]` attributes on one item are ANDed, so one test-only
/// attribute is enough.
pub(crate) fn attrs_are_test_only(attrs: &[Attribute]) -> Result<bool, String> {
    let mut test_only = false;
    for attr in attrs {
        test_only |= cfg_is_test_only(attr)?;
    }
    Ok(test_only)
}

/// `Ok(false)` for anything that is not a `#[cfg(...)]` attribute.
pub(crate) fn cfg_is_test_only(attr: &Attribute) -> Result<bool, String> {
    if !attr.path().is_ident("cfg") {
        return Ok(false);
    }
    let list = attr
        .meta
        .require_list()
        .map_err(|error| format!("`#[cfg]` without a predicate: {error}"))?;
    let predicates = list
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map_err(|error| format!("unparseable cfg predicate `{}`: {error}", list.tokens))?;
    let [predicate] = Vec::from_iter(predicates)
        .try_into()
        .map_err(|_| format!("`#[cfg({})]` must hold exactly one predicate", list.tokens))?;
    requires_test(&predicate)
}

fn requires_test(predicate: &Meta) -> Result<bool, String> {
    match predicate {
        Meta::Path(path) => Ok(ident(path)? == "test"),
        Meta::NameValue(pair) => {
            let key = ident(&pair.path)?;
            let Expr::Lit(literal) = &pair.value else {
                return Err(format!("`{key} = …` needs a string literal"));
            };
            let Lit::Str(value) = &literal.lit else {
                return Err(format!("`{key} = …` needs a string literal"));
            };
            Ok(key == "feature" && TEST_ONLY_FEATURES.contains(&value.value().as_str()))
        }
        Meta::List(list) => {
            let operator = ident(&list.path)?;
            let children = list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_err(|error| format!("unparseable `{operator}(…)`: {error}"))?;
            let verdicts = children
                .iter()
                .map(requires_test)
                .collect::<Result<Vec<_>, _>>()?;
            match operator.as_str() {
                "all" => Ok(verdicts.into_iter().any(|test_only| test_only)),
                "any" => {
                    Ok(!verdicts.is_empty() && verdicts.into_iter().all(|test_only| test_only))
                }
                "not" if verdicts.len() == 1 => Ok(false),
                "not" => Err("`not(…)` takes exactly one predicate".to_string()),
                // Any other operator is not known to require a test build.
                _ => Ok(false),
            }
        }
    }
}

fn ident(path: &syn::Path) -> Result<String, String> {
    path.get_ident()
        .map(ToString::to_string)
        .ok_or_else(|| "a cfg name must be a single identifier".to_string())
}
