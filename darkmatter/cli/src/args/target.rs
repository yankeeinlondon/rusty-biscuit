use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;
use crate::args::{
    GraphFormat, SchemaDetectFormat, SchemaValidateFormat, ValidateOutputFormat,
    complete_compose_args, complete_markdown_files,
};
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

        /// Output format
        #[arg(long, value_enum, default_value_t = ValidateOutputFormat::Text)]
        format: ValidateOutputFormat,

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
    /// Validate markdown frontmatter against a schema.
    ///
    /// Positional arguments are either Markdown file paths or
    /// `<prop>=<value>` assignments applied to every document's frontmatter
    /// before validation. An argument is treated as an assignment when it
    /// contains `=` and the text before the first `=` is a dot-separated
    /// property path (e.g. `title=Hello`, `user.email=ken@ken.net`).
    /// File paths that contain `=` should be disambiguated with `./` —
    /// e.g. `./weird=name.md`.
    Validate {
        /// Markdown file paths and `<prop>=<value>` assignments
        #[arg(
            value_name = "FILE_OR_PROP=VALUE",
            required = true,
            num_args = 1..,
            add = ArgValueCompleter::new(complete_compose_args)
        )]
        inputs: Vec<String>,

        /// Baseline schema path (YAML SimplifiedSchema or JSON Schema)
        #[arg(long, value_name = "PATH")]
        schema: Option<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value_t = SchemaValidateFormat::Pretty)]
        format: SchemaValidateFormat,

        /// Suppress success lines; only failures print
        #[arg(long)]
        quiet: bool,

        /// Disable trigger discovery and bare-name schema-root lookup
        #[arg(long)]
        no_trigger_schemas: bool,
    },

    /// Explain trigger-schema discovery and activation for a document.
    Triggers {
        /// Markdown file to inspect
        #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
        file: PathBuf,
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

        /// Output format
        #[arg(long, value_enum, default_value_t = SchemaDetectFormat::Yaml)]
        format: SchemaDetectFormat,

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
    About,
}
