//! The `--output` and `--width` arguments, shared by every command that
//! declares them.
//!
//! A command names the targets it accepts as one of these lists, so the
//! accepted spellings, the `browser` alias, and `--help`'s value list all come
//! from [`OutputTarget`].

use biscuit_terminal::output::{OutputTarget, WidthRequest, parse_width_request};
use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};

/// Both families.
pub const ALL_TARGETS: &[OutputTarget] = &OutputTarget::ALL;

/// `md compose`: its Markdown default and the data family.
pub const COMPOSE_TARGETS: &[OutputTarget] =
    &[OutputTarget::Markdown, OutputTarget::Json, OutputTarget::Json5, OutputTarget::Yaml, OutputTarget::Toml];

/// A value parser accepting exactly `targets`.
pub fn target_parser(targets: &'static [OutputTarget]) -> impl TypedValueParser<Value = OutputTarget> {
    let values = targets.iter().map(|target| {
        let value = PossibleValue::new(target.as_str());
        if *target == OutputTarget::Html { value.alias("browser") } else { value }
    });
    PossibleValuesParser::new(values).map(|raw: String| {
        raw.parse::<OutputTarget>().expect("the parser only admits spellings of a target")
    })
}

/// Parses a `--width` value; a bad value is a usage error.
pub fn parse_width(raw: &str) -> Result<WidthRequest, String> {
    parse_width_request(raw).map_err(|error| error.to_string())
}
