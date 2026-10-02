//! Typed CLI switch metadata for the provider catalog.
//!
//! Projected by `claudine-gen` from the `agent-cli` research topic
//! (`docs/research/agent-cli/_types.yaml`, `cli_switch`). The serde form of
//! each type is the catalog shape: what `catalog.json` carries and what a
//! field-keyed override authors.

use serde::{Serialize, Serializer};
use strum::{IntoStaticStr, VariantNames};

/// A provider's switch inventory, or the reason it has none yet.
///
/// Every compiled provider carries one of the two. `Unknown` is a gap, never
/// an empty inventory: a switch looked up in it is unrecognized, not
/// "takes no value".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CliSwitchCatalog {
    /// Researched switches, sorted by canonical spelling and then by scope.
    Researched(&'static [CliSwitch]),
    /// The inventory could not be established; `gap` says why.
    Unknown { gap: &'static str },
}

/// One provider switch at one set of command paths.
///
/// A switch spelled the same way at two disjoint sets of command paths may
/// appear twice; the generator rejects two records whose spellings meet at a
/// command path where both apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CliSwitch {
    /// Canonical spelling, dashes included.
    pub flag: &'static str,
    /// Every other exact spelling of the same switch.
    pub aliases: &'static [&'static str],
    pub value: SwitchValue,
    /// Forms in which the value may be written; empty for
    /// [`SwitchValue::None`] and [`SwitchValue::Unknown`].
    pub attachments: &'static [SwitchAttachment],
    /// Where the switch is accepted; never empty.
    pub scopes: &'static [SwitchScope],
    pub description: &'static str,
    /// What research could not establish; present exactly when the value
    /// type or the variadic minimum is unknown.
    pub gap: Option<&'static str>,
}

/// How a switch takes its value.
///
/// `VARIANTS` is the research `value_type` vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SwitchValue {
    /// Takes no value.
    None,
    /// Takes one string; `optional` when it may be left out.
    String { optional: bool },
    /// Takes one finite decimal number; `optional` when it may be left out.
    Number { optional: bool },
    /// Takes a list of strings.
    Variadic { min: VariadicMin },
    /// Research could not establish the type. Treated as an unrecognized
    /// switch, never as [`SwitchValue::None`].
    Unknown,
}

/// The fewest values a variadic switch requires.
///
/// Serializes as the bare number, or as the string `unknown`, matching the
/// research form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariadicMin {
    /// At least this many values (never zero).
    AtLeast(u32),
    /// Not established; consumers count it as one for ownership and never
    /// fail a launch over it.
    Unknown,
}

impl Serialize for VariadicMin {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            VariadicMin::AtLeast(min) => serializer.serialize_u32(*min),
            VariadicMin::Unknown => serializer.serialize_str("unknown"),
        }
    }
}

/// A form in which a provider accepts a switch's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SwitchAttachment {
    /// The next argument: `--config x=y`.
    Space,
    /// `--config=x=y`.
    Equals,
    /// Written straight after a two-character switch: `-cx=y`.
    ShortAttached,
}

/// Where on a provider's command line a switch is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchScope {
    /// At every command path.
    Global,
    /// Only at this exact native command path after the executable; an
    /// empty path is the root entrypoint.
    Command(&'static [&'static str]),
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use strum::VariantNames;

    use super::*;

    /// The generator validates research members against these lists.
    #[test]
    fn variant_names_are_the_research_vocabulary() {
        assert_eq!(
            SwitchValue::VARIANTS,
            &["none", "string", "number", "variadic", "unknown"]
        );
        assert_eq!(
            SwitchAttachment::VARIANTS,
            &["space", "equals", "short_attached"]
        );
    }

    /// The serde form is the catalog shape `catalog.json` and overrides use.
    #[test]
    fn serde_form_is_the_catalog_shape() {
        static SWITCHES: &[CliSwitch] = &[CliSwitch {
            flag: "--image",
            aliases: &["-i"],
            value: SwitchValue::Variadic {
                min: VariadicMin::AtLeast(1),
            },
            attachments: &[SwitchAttachment::Space, SwitchAttachment::ShortAttached],
            scopes: &[SwitchScope::Global, SwitchScope::Command(&["exec"])],
            description: "Attach images.",
            gap: None,
        }];
        assert_eq!(
            serde_json::to_value(CliSwitchCatalog::Researched(SWITCHES)).unwrap(),
            json!({"researched": [{
                "flag": "--image",
                "aliases": ["-i"],
                "value": {"variadic": {"min": 1}},
                "attachments": ["space", "short_attached"],
                "scopes": ["global", {"command": ["exec"]}],
                "description": "Attach images.",
                "gap": null,
            }]})
        );
        assert_eq!(
            serde_json::to_value(SwitchValue::Variadic {
                min: VariadicMin::Unknown
            })
            .unwrap(),
            json!({"variadic": {"min": "unknown"}})
        );
        assert_eq!(
            serde_json::to_value(SwitchValue::String { optional: true }).unwrap(),
            json!({"string": {"optional": true}})
        );
        assert_eq!(serde_json::to_value(SwitchValue::None).unwrap(), json!("none"));
        assert_eq!(
            serde_json::to_value(CliSwitchCatalog::Unknown { gap: "not researched" }).unwrap(),
            json!({"unknown": {"gap": "not researched"}})
        );
    }
}
