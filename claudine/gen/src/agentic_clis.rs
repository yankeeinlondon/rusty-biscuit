//! Roster-level emitter: the `has_agentic_cli` name table Darkmatter compiles.
//!
//! Darkmatter cannot depend on Claudine, so the accepted `has_agentic_cli`
//! names reach it as a committed Darkmatter source file (spec R13). This stage
//! reads `docs/providers.yaml` directly rather than through the per-provider
//! loader: that loader refuses `skip_research: true` entries, and a skipped
//! entry is still a valid roster identity (R22).
//!
//! The artifact carries each `sniff_binding` as the `sniff::programs::AiCli`
//! variant *name*, not a path expression. `claudine-gen` depends on the
//! Darkmatter library, so a table that stopped compiling would block building
//! the tool that regenerates it (bootstrap rule F1). The binding is validated
//! against `AiCli` here, and Darkmatter's own tests prove every row resolves.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sniff::programs::AiCli;
use strum::IntoEnumIterator;

use crate::errors::GenError;
use crate::generate::{CheckOutcome, diff_lines};
use crate::inputs::read_yaml;

/// One accepted `has_agentic_cli` name and the `AiCli` variant that detects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgenticCliName {
    /// A roster `slug` or one of that entry's `cli_aliases`.
    pub name: String,
    /// The roster slug that owns `name`.
    pub slug: String,
    /// The `sniff::programs::AiCli` variant name from `sniff_binding`.
    pub binding: String,
}

/// The committed artifact path. It lives in the sibling `darkmatter` package
/// area, so it is resolved from the monorepo root above `area`.
pub fn agentic_clis_path(area: &Path) -> PathBuf {
    area.parent()
        .unwrap_or(area)
        .join("darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli_generated.rs")
}

/// Loads every accepted name from the roster, in roster order: each entry's
/// slug first, then its aliases, skipping spellings already listed for the
/// same entry.
///
/// ## Errors
///
/// [`GenError::AgenticCliRosterInvalid`] when an entry lacks a slug or
/// `sniff_binding`, a binding names no `AiCli` variant, a name is not a
/// lowercase `[a-z0-9_-]` word, or two roster entries claim the same name.
pub fn load_agentic_cli_names(area: &Path) -> Result<Vec<AgenticCliName>, GenError> {
    let path = area.join("docs/providers.yaml");
    let invalid = |message: String| GenError::AgenticCliRosterInvalid {
        path: path.clone(),
        message,
    };
    let roster = read_yaml(&path)?;
    let entries = roster
        .get("list")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("expected a top-level `list:` sequence".into()))?;

    let variants: Vec<String> = AiCli::iter().map(|variant| format!("{variant:?}")).collect();
    let mut owners = BTreeMap::<String, String>::new();
    let mut names = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let slug = entry
            .get("slug")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("list[{index}] has no string `slug`")))?;
        let binding = entry
            .get("sniff_binding")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("`{slug}` has no string `sniff_binding`")))?;
        if !variants.iter().any(|variant| variant == binding) {
            return Err(invalid(format!(
                "`{slug}` binds `{binding}`, which is not a sniff `AiCli` variant"
            )));
        }
        let aliases = match entry.get("cli_aliases") {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str()
                        .ok_or_else(|| invalid(format!("`{slug}` has a non-string `cli_aliases` item")))
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(invalid(format!("`{slug}` `cli_aliases` must be a sequence"))),
        };

        for name in std::iter::once(slug).chain(aliases) {
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
            {
                return Err(invalid(format!(
                    "`{slug}` name `{name}` must be a non-empty lowercase [a-z0-9_-] word"
                )));
            }
            match owners.get(name) {
                Some(owner) if owner == slug => continue,
                Some(owner) => {
                    return Err(invalid(format!(
                        "name `{name}` is claimed by both `{owner}` and `{slug}`"
                    )));
                }
                None => {}
            }
            owners.insert(name.to_string(), slug.to_string());
            names.push(AgenticCliName {
                name: name.to_string(),
                slug: slug.to_string(),
                binding: binding.to_string(),
            });
        }
    }
    Ok(names)
}

/// Builds the deterministic artifact text from the roster.
pub fn build_agentic_clis(area: &Path) -> Result<String, GenError> {
    Ok(emit_file(&load_agentic_cli_names(area)?))
}

/// Byte-compares [`build_agentic_clis`] output against the committed file — the
/// code path shared by the CLI `check` subcommand and the drift test.
pub fn check_agentic_clis(area: &Path) -> Result<CheckOutcome, GenError> {
    let generated = build_agentic_clis(area)?;
    let path = agentic_clis_path(area);
    if !path.is_file() {
        return Ok(CheckOutcome::MissingCommitted { path });
    }
    let committed = std::fs::read_to_string(&path).map_err(|source| GenError::Io {
        path: path.clone(),
        source,
    })?;
    if committed == generated {
        return Ok(CheckOutcome::Clean);
    }
    Ok(CheckOutcome::Drift {
        details: diff_lines(&committed, &generated),
    })
}

fn emit_file(names: &[AgenticCliName]) -> String {
    let mut out = String::from(
        "// GENERATED by claudine-gen — DO NOT EDIT BY HAND.\n\
         //\n\
         // Input: claudine/docs/providers.yaml (every roster entry, `skip_research` included)\n\
         // Regenerate with `cargo run -p claudine-gen -- generate`; drift-check with\n\
         // `cargo run -p claudine-gen -- check` (the same code path as the drift test).\n\
         \n\
         //! Generated `has_agentic_cli` names.\n\
         //!\n\
         //! Each row is an accepted name (a roster slug or one of its `cli_aliases`)\n\
         //! and the `sniff::programs::AiCli` variant name that detects it. Variant\n\
         //! names stay strings so a stale table never blocks building `claudine-gen`.\n\
         \n\
         pub(crate) const AGENTIC_CLI_NAMES: &[(&str, &str)] = &[\n",
    );
    for name in names {
        out.push_str(&format!("    ({:?}, {:?}),\n", name.name, name.binding));
    }
    out.push_str("];\n");
    out
}

#[cfg(test)]
mod tests;
