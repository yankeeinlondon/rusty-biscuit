//! Typed [`CliSwitchCatalog`] descriptor for [`ProviderInfo`], and the one
//! lookup that reads it.
//!
//! The types live in the `claudine-catalog-types` leaf crate so the catalog
//! generator can introspect them without linking this library
//! (provider-metadata design F1); this re-export gives the generated
//! `data.rs` one import path for the whole switch-metadata shape.
//!
//! Lookups are keyed by provider and by the native command path the launch
//! uses (`["exec"]` for a non-interactive Codex run, `[]` for the root
//! entrypoint). A record applies there when one of its scopes is global or
//! names that exact path. Token ownership and the forwarding messages both
//! read the catalog through this module, so they cannot disagree about what a
//! switch is.
//!
//! [`ProviderInfo`]: super::ProviderInfo
//! [`CliSwitchCatalog`]: claudine_catalog_types::CliSwitchCatalog

pub use claudine_catalog_types::{
    CliSwitch, CliSwitchCatalog, SwitchAttachment, SwitchScope, SwitchValue, VariadicMin,
};

use super::{Provider, provider_info};

/// What the compiled catalog establishes about one spelling at one command
/// path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchLookup {
    /// A researched record applies. Its value may still be
    /// [`SwitchValue::Unknown`], which ownership treats as unrecognized.
    Known(&'static CliSwitch),
    /// The provider's inventory is researched, and no record with this
    /// spelling applies at the path.
    NotInCatalog,
    /// The provider's inventory is a gap; nothing is established for any
    /// switch.
    CatalogGap {
        /// Why the inventory could not be established.
        gap: &'static str,
    },
}

impl SwitchLookup {
    /// How the switch takes its value, with anything not established read
    /// as [`SwitchValue::Unknown`] (never [`SwitchValue::None`]).
    pub fn value(&self) -> SwitchValue {
        match self {
            SwitchLookup::Known(switch) => switch.value,
            SwitchLookup::NotInCatalog | SwitchLookup::CatalogGap { .. } => SwitchValue::Unknown,
        }
    }
}

/// Looks up `spelling`, an exact switch spelling such as `-c` or `--config`,
/// for `provider` at the native command `path`.
pub fn lookup_switch(provider: Provider, path: &[&str], spelling: &str) -> SwitchLookup {
    lookup_in(provider_info(provider).cli_switches, path, spelling)
}

fn lookup_in(catalog: CliSwitchCatalog, path: &[&str], spelling: &str) -> SwitchLookup {
    match catalog {
        CliSwitchCatalog::Unknown { gap } => SwitchLookup::CatalogGap { gap },
        CliSwitchCatalog::Researched(switches) => switches
            .iter()
            .find(|switch| switch.applies_at(path) && switch.spellings().any(|s| s == spelling))
            .map_or(SwitchLookup::NotInCatalog, SwitchLookup::Known),
    }
}

/// A command-line token matched to a researched switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchToken<'t> {
    pub switch: &'static CliSwitch,
    /// The spelling the token used, without any attached value.
    pub spelling: &'static str,
    /// The value written into the token itself, and the form it was written
    /// in; `None` when the token is the bare spelling.
    pub attached: Option<(SwitchAttachment, &'t str)>,
}

/// Matches `token` to a researched switch for `provider` at `path`.
///
/// An exact spelling matches first. Otherwise `--name=value` matches a
/// switch that accepts [`SwitchAttachment::Equals`], and `-xvalue` (a
/// single-dash, one-character spelling with text after it) one that accepts
/// [`SwitchAttachment::ShortAttached`]. A form the research did not establish
/// matches nothing, so `-cvalue` is not split for a switch that only takes a
/// separate value.
pub fn match_switch_token<'t>(provider: Provider, path: &[&str], token: &'t str) -> Option<SwitchToken<'t>> {
    match_token_in(provider_info(provider).cli_switches, path, token)
}

fn match_token_in<'t>(catalog: CliSwitchCatalog, path: &[&str], token: &'t str) -> Option<SwitchToken<'t>> {
    let known = |spelling: &str| match lookup_in(catalog, path, spelling) {
        SwitchLookup::Known(switch) => Some(switch),
        SwitchLookup::NotInCatalog | SwitchLookup::CatalogGap { .. } => None,
    };
    let spelled = |switch: &'static CliSwitch, spelling: &str| {
        switch
            .spellings()
            .find(|s| *s == spelling)
            .expect("lookup matched this spelling")
    };
    if let Some(switch) = known(token) {
        return Some(SwitchToken {
            switch,
            spelling: spelled(switch, token),
            attached: None,
        });
    }
    if let Some((name, value)) = token.split_once('=')
        && name.starts_with('-')
        && let Some(switch) = known(name).filter(|s| s.accepts(SwitchAttachment::Equals))
    {
        return Some(SwitchToken {
            switch,
            spelling: spelled(switch, name),
            attached: Some((SwitchAttachment::Equals, value)),
        });
    }
    let mut chars = token.char_indices();
    if let (Some((_, '-')), Some((_, short)), Some((rest, _))) = (chars.next(), chars.next(), chars.next())
        && short != '-'
        && let Some(switch) = known(&token[..rest]).filter(|s| s.accepts(SwitchAttachment::ShortAttached))
    {
        return Some(SwitchToken {
            switch,
            spelling: spelled(switch, &token[..rest]),
            attached: Some((SwitchAttachment::ShortAttached, &token[rest..])),
        });
    }
    None
}

/// One spelling looked up for every provider a command might launch, each at
/// its own command path, keeping every provider's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSwitch {
    arms: Vec<(Provider, SwitchLookup)>,
}

impl CandidateSwitch {
    /// Each candidate provider's answer, in the order the candidates were
    /// given.
    pub fn arms(&self) -> &[(Provider, SwitchLookup)] {
        &self.arms
    }

    /// The value every candidate agrees on, or `None` when two disagree. An
    /// unestablished answer counts as [`SwitchValue::Unknown`], so a switch
    /// known to one candidate and unknown to another disagrees.
    pub fn agreed_value(&self) -> Option<SwitchValue> {
        let mut values = self.arms.iter().map(|(_, lookup)| lookup.value());
        let first = values.next()?;
        values.all(|value| value == first).then_some(first)
    }
}

/// Looks up `spelling` for each `(provider, path)` candidate.
pub fn lookup_candidates<'p>(
    candidates: impl IntoIterator<Item = (Provider, &'p [&'p str])>,
    spelling: &str,
) -> CandidateSwitch {
    CandidateSwitch {
        arms: candidates
            .into_iter()
            .map(|(provider, path)| (provider, lookup_switch(provider, path, spelling)))
            .collect(),
    }
}

#[cfg(test)]
mod tests;
