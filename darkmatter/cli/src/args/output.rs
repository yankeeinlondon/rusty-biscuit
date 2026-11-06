//! The `--output` and `--width` arguments, shared by every command.
//!
//! A command names the targets it accepts as one of these lists, so the
//! accepted spellings, the `browser` alias, and `--help`'s value list all come
//! from [`OutputTarget`]. [`COMMAND_LEAVES`] records what each command declares;
//! a test holds it to the clap definitions and to the schema files.

use biscuit_terminal::output::{OutputTarget, WidthRequest, parse_width_request};
use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};

/// Both families.
pub const ALL_TARGETS: &[OutputTarget] = &OutputTarget::ALL;

/// `md compose`: its Markdown default and the data family.
pub const COMPOSE_TARGETS: &[OutputTarget] =
    &[OutputTarget::Markdown, OutputTarget::Json, OutputTarget::Json5, OutputTarget::Yaml, OutputTarget::Toml];

/// What `md schema detect` prints: any target, or JSON Schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaDetectOutput {
    /// One of the shared targets.
    Target(OutputTarget),
    /// JSON Schema Draft 2020-12.
    JsonSchema,
}

impl SchemaDetectOutput {
    /// The shared target that delivers this output; JSON Schema is JSON.
    #[must_use]
    pub fn target(self) -> OutputTarget {
        match self {
            Self::Target(target) => target,
            Self::JsonSchema => OutputTarget::Json,
        }
    }
}

/// What one command leaf declares about its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandLeaf {
    /// The subcommand path below `md`.
    pub path: &'static [&'static str],
    /// The target used when `--output` is absent.
    pub default_target: OutputTarget,
    /// The file under `cli/schemas/output/` defining its data output.
    pub schema: &'static str,
    /// Whether the formatted targets other than its default are accepted.
    pub formatted_family: bool,
    /// A value beyond the two families.
    pub extra_value: Option<&'static str>,
}

const fn leaf(path: &'static [&'static str], default_target: OutputTarget, schema: &'static str) -> CommandLeaf {
    CommandLeaf { path, default_target, schema, formatted_family: true, extra_value: None }
}

/// Every command that runs, with what it declares. `md validate` and
/// `md schema` only hold other commands.
pub const COMMAND_LEAVES: &[CommandLeaf] = &[
    CommandLeaf { path: &["compose"], default_target: OutputTarget::Markdown, schema: "compose.yaml", formatted_family: false, extra_value: None },
    leaf(&["render"], OutputTarget::Terminal, "render.yaml"),
    leaf(&["clean"], OutputTarget::Markdown, "clean.yaml"),
    leaf(&["toc"], OutputTarget::Terminal, "toc.yaml"),
    leaf(&["delta"], OutputTarget::Terminal, "delta.yaml"),
    leaf(&["get"], OutputTarget::Json, "get.yaml"),
    leaf(&["set"], OutputTarget::Markdown, "set.yaml"),
    leaf(&["rm"], OutputTarget::Terminal, "rm.yaml"),
    leaf(&["edit"], OutputTarget::Terminal, "edit.yaml"),
    leaf(&["hash"], OutputTarget::Terminal, "hash.yaml"),
    leaf(&["graph"], OutputTarget::Terminal, "graph.yaml"),
    leaf(&["code-block"], OutputTarget::Terminal, "code-block.yaml"),
    leaf(&["validate", "refs"], OutputTarget::Terminal, "validate-refs.yaml"),
    leaf(&["schema", "validate"], OutputTarget::Terminal, "schema-validate.yaml"),
    leaf(&["schema", "triggers"], OutputTarget::Terminal, "schema-triggers.yaml"),
    CommandLeaf {
        path: &["schema", "detect"],
        default_target: OutputTarget::Terminal,
        schema: "schema-detect.yaml",
        formatted_family: true,
        extra_value: Some("json-schema"),
    },
    leaf(&["schema", "about"], OutputTarget::Terminal, "schema-about.yaml"),
];

/// `md schema detect`'s value parser: every target and `json-schema`.
pub fn detect_target_parser() -> impl TypedValueParser<Value = SchemaDetectOutput> {
    let values = ALL_TARGETS
        .iter()
        .map(|target| {
            let value = PossibleValue::new(target.as_str());
            if *target == OutputTarget::Html { value.alias("browser") } else { value }
        })
        .chain([PossibleValue::new("json-schema")]);
    PossibleValuesParser::new(values).map(|raw: String| match raw.as_str() {
        "json-schema" => SchemaDetectOutput::JsonSchema,
        other => SchemaDetectOutput::Target(other.parse().expect("the parser only admits spellings of a target")),
    })
}

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
