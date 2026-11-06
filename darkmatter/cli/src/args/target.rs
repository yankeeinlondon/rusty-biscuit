use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;
use crate::args::policy::{self, PolicyCommand, PolicySelector, PolicyValueParser, PolicyVerb};
use crate::args::output::{ALL_TARGETS, SchemaDetectOutput, detect_target_parser, target_parser};
use crate::args::{GraphFormat, complete_compose_args, complete_markdown_files};
use biscuit_terminal::output::OutputTarget;
use std::path::PathBuf;

/// Validation sub-targets.
#[derive(Clone, Debug, Subcommand)]
pub enum ValidateTarget {
    /// Validate all references (links, images, transclusions).
    Refs {
        /// Input file path
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Enable remote URL validation
        #[arg(long)]
        remote: bool,

        /// Enable fragment validation
        #[arg(long)]
        fragments: bool,

        /// Remote validation timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,

        /// Stop on first error
        #[arg(long)]
        fail_fast: bool,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,

        /// Show all references, not just issues
        #[arg(long = "show-all")]
        show_all: bool,

        /// Print transclusion graph as Mermaid or DOT
        #[arg(long, value_enum)]
        graph: Option<GraphFormat>,
    },
}

/// Schema sub-targets.
#[derive(Clone, Debug, Subcommand)]
pub enum SchemaTarget {
    /// Validate Markdown frontmatter or whole YAML documents against a schema.
    ///
    /// Positional arguments are either Markdown or YAML file paths, or
    /// `<prop>=<value>` assignments applied to every Markdown document's
    /// frontmatter before validation. A `.yaml`/`.yml` file is validated as a
    /// whole document by the schema its claimed `kind` or root `$schema`
    /// selects, with no baseline; assignments and `--schema` cannot be used
    /// with one. An argument is treated as an assignment when it
    /// contains `=` and the text before the first `=` is a dot-separated
    /// property path (e.g. `title=Hello`, `user.email=ken@ken.net`).
    /// File paths that contain `=` should be disambiguated with `./` —
    /// e.g. `./weird=name.md`.
    Validate {
        /// Markdown or YAML file paths and `<prop>=<value>` assignments
        #[arg(
            value_name = "FILE_OR_PROP=VALUE",
            required = true,
            num_args = 1..,
            add = ArgValueCompleter::new(complete_compose_args)
        )]
        inputs: Vec<String>,

        /// Custom baseline schema path (YAML SimplifiedSchema or JSON
        /// Schema) replacing the built-in Darkmatter baseline; used even when
        /// `DARKMATTER_NO_BASELINE_SCHEMA` is set
        #[arg(long, value_name = "PATH")]
        schema: Option<PathBuf>,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,

        /// Suppress success lines; only failures print
        #[arg(long)]
        quiet: bool,

        /// Stop a feature from running (repeatable): `baseline-schema` validates without any baseline, ignoring `BASELINE_SCHEMA`; the document's `$schema` and trigger schemas still apply
        #[arg(
            long = "disable",
            value_name = "ID",
            value_parser = PolicyValueParser::new(PolicyCommand::SchemaValidate, PolicyVerb::Disable),
            long_help = policy::long_help(PolicyCommand::SchemaValidate, PolicyVerb::Disable),
            add = ArgValueCompleter::new(|current: &std::ffi::OsStr| policy::complete(PolicyCommand::SchemaValidate, PolicyVerb::Disable, current))
        )]
        disable: Vec<PolicySelector>,
    },

    /// Explain schema-trigger discovery and activation for a document.
    Triggers {
        /// Markdown file to inspect
        #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
        file: PathBuf,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Detect a SimplifiedSchema from one or more markdown documents.
    Detect {
        /// Input markdown files
        #[arg(
            value_name = "FILE",
            required = true,
            num_args = 1..,
            add = ArgValueCompleter::new(complete_markdown_files)
        )]
        files: Vec<PathBuf>,

        /// What to print; `yaml` is the bare schema, and `json-schema` is Draft 2020-12
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = detect_target_parser())]
        output: SchemaDetectOutput,

        /// Merge multiple files: widen disagreeing types and mark
        /// properties required only if present in every input file.
        #[arg(long)]
        merge: bool,
    },

    /// Print the SimplifiedSchema language reference.
    ///
    /// Renders a human-readable report covering schema shapes, the type
    /// vocabulary, constraints, inline object syntax, validation behaviour,
    /// and compose-time coercion. The report is generated from the typed
    /// schema-language descriptor catalog in
    /// `darkmatter::markdown::schemas`; the same catalog is available to
    /// library callers via the `schema_type_descriptors()`,
    /// `schema_constraint_descriptors()`, `schema_shape_descriptors()`,
    /// `inline_object_rule_descriptors()`, `coercion_rule_descriptors()`,
    /// and `validation_behavior_descriptors()` functions.
    ///
    /// The command is documentation-only: it does not parse documents,
    /// capture context, construct an `EffectEngine`, resolve file
    /// references, or perform network access.
    About {
        /// What to print; a data format always carries the whole catalog
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },
}
