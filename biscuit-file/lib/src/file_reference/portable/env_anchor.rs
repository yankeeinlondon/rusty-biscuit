//! Portable environment variables: which names are declared, and whether a
//! declared variable's value can anchor a target.

use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::diagnostics::EnvAnchorProblem;
use super::path_identity::PathIdentity;
use crate::file_reference::parse;

/// The environment variable listing portable variable names, comma-separated
/// (`CONFIG_DIR, OBSIDIAN_VAULT`).
///
/// It is read from the same environment as the values: the context's captured
/// environment, or the process environment captured once when no context is
/// given.
pub const PORTABLE_ENV_VARIABLES: &str = "PORTABLE_ENV_VARIABLES";

/// The declared portable names, deduplicated and in name order, plus every
/// invalid entry that was skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PortableNames {
    pub names: BTreeSet<String>,
    pub invalid: Vec<String>,
}

impl PortableNames {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

/// The union of [`PORTABLE_ENV_VARIABLES`] in `env` and the builder `extra`
/// names.
///
/// List entries are trimmed and empty entries ignored, so an empty or absent
/// list declares nothing. Builder names are taken exactly as given. A name
/// outside `[A-Z0-9_]+` is recorded once in `invalid` and never made portable.
pub(crate) fn portable_names(env: &HashMap<String, String>, extra: &[String]) -> PortableNames {
    let listed = env
        .get(PORTABLE_ENV_VARIABLES)
        .into_iter()
        .flat_map(|list| list.split(','))
        .map(str::trim)
        .filter(|entry| !entry.is_empty());
    let mut declared = PortableNames::default();
    for name in listed.chain(extra.iter().map(String::as_str)) {
        if is_variable_name(name) {
            declared.names.insert(name.to_string());
        } else if !declared.invalid.iter().any(|invalid| invalid == name) {
            declared.invalid.push(name.to_string());
        }
    }
    declared
}

/// The `{{VAR}}` name grammar.
fn is_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The anchor directory a variable's value names, when it is an absolute
/// path on this host.
pub(crate) fn anchor_dir(value: Option<&str>) -> Result<PathBuf, EnvAnchorProblem> {
    let value = value.ok_or(EnvAnchorProblem::Unset)?;
    if Path::new(value).is_absolute() {
        Ok(PathBuf::from(value))
    } else if parse::is_absolute_reference(value) {
        Err(EnvAnchorProblem::ForeignAbsolute {
            value: value.to_string(),
        })
    } else {
        Err(EnvAnchorProblem::NotAbsolute {
            value: value.to_string(),
        })
    }
}

/// A variable whose value is an absolute whole-component prefix of the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EligibleAnchor {
    pub name: String,
    /// The target's names below the anchor.
    pub rest: Vec<OsString>,
}

/// Every declared variable, judged against one target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AnchorEvaluation {
    /// Ineligible variables, in name order.
    pub rejected: Vec<(String, EnvAnchorProblem)>,
    /// Eligible variables, deepest anchor first; equal depths keep name order.
    pub eligible: Vec<EligibleAnchor>,
}

pub(crate) fn evaluate_anchors(
    names: &PortableNames,
    env: &HashMap<String, String>,
    target: &PathIdentity,
) -> AnchorEvaluation {
    let mut evaluation = AnchorEvaluation::default();
    let mut eligible = Vec::new();
    for name in &names.names {
        let anchor = anchor_dir(env.get(name).map(String::as_str)).and_then(|dir| {
            let identity = PathIdentity::new(&dir);
            match target.strip_prefix(&identity) {
                Some(rest) => Ok((identity.components().len(), rest.to_vec())),
                None => Err(EnvAnchorProblem::NotAPrefix { value: dir }),
            }
        });
        match anchor {
            Ok((depth, rest)) => eligible.push((
                depth,
                EligibleAnchor {
                    name: name.clone(),
                    rest,
                },
            )),
            Err(problem) => evaluation.rejected.push((name.clone(), problem)),
        }
    }
    // Stable, so names already in order break depth ties.
    eligible.sort_by(|(left, _), (right, _)| right.cmp(left));
    evaluation.eligible = eligible.into_iter().map(|(_, anchor)| anchor).collect();
    evaluation
}

#[cfg(test)]
mod tests;
