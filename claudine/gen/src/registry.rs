//! The mapping registry: catalog field → declared source → coercion.
//!
//! Generator v1 (Phase B item 3): one entry per serialized `ProviderInfo`
//! field, in serialization order (which follows the struct's declaration
//! order). The field-source matrix
//! (`features/2026-07-02-provider-metadata/field-source-matrix.md`) is the
//! ruled contract behind every source declaration. Table-A fields (no
//! current constant) are deferred to Phase D; `structured_stream_flag` is
//! retired outright (OQ7b).
//!
//! Serialized fields with NO entry here must appear in
//! [`EXCLUDED_SERIALIZED_FIELDS`] with a justification — the
//! registry-covers-all-fields guard enforces exactly-one-of.

use claudine_catalog_types::{ModelCatalogSource, ResumeSupport};
use serde_json::{Value, json};
use strum::VariantNames;

/// Which input file owns a field's value (the field-source matrix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredSource {
    /// Identity fact from `docs/providers.yaml`.
    Roster { key: &'static str },
    /// Topic-less fact from `docs/providers/facts/<slug>.yaml`.
    Facts { key: &'static str },
    /// Research frontmatter from `docs/research/<topic>/<slug>.md`.
    Research {
        topic: &'static str,
        /// Dot-separated path into the frontmatter / sidecar shape.
        path: &'static str,
    },
}

impl DeclaredSource {
    /// Short source-kind label used in collision errors and `--mapping`.
    pub fn kind(&self) -> &'static str {
        match self {
            DeclaredSource::Roster { .. } => "roster",
            DeclaredSource::Facts { .. } => "facts",
            DeclaredSource::Research { .. } => "research",
        }
    }
}

/// Acceptable sidecar/value shapes for a field, checked BEFORE any value
/// mapping. A field lists alternatives (OR); a shape that matches an
/// alternative's kind but violates its constraint (enum subset) is a loud
/// error, never a silent fall-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaExpectation {
    /// A plain string scalar.
    String,
    /// A boolean (sidecar `boolean` or `boolish`).
    Boolean,
    /// An array of plain strings.
    StringArray,
    /// An `enum(...)` feeding a Rust enum (or a coercion's fixed
    /// vocabulary): the sidecar members must be a subset of the declared
    /// member names.
    EnumSubsetOf {
        rust_enum: &'static str,
        variants: &'static [&'static str],
    },
    /// A single inline-object record carrying at least these fields.
    Record {
        required_fields: &'static [&'static str],
    },
    /// An array of inline-object records carrying at least these fields.
    RecordArray {
        required_fields: &'static [&'static str],
    },
}

impl SchemaExpectation {
    /// Label used in error messages and `--mapping` output.
    pub fn label(&self) -> String {
        match self {
            SchemaExpectation::String => "string".into(),
            SchemaExpectation::Boolean => "boolean".into(),
            SchemaExpectation::StringArray => "string[]".into(),
            SchemaExpectation::EnumSubsetOf { rust_enum, .. } => {
                format!("enum(subset of {rust_enum})")
            }
            SchemaExpectation::Record { required_fields } => {
                format!("record(requires {})", required_fields.join(", "))
            }
            SchemaExpectation::RecordArray { required_fields } => {
                format!("record_array(requires {})", required_fields.join(", "))
            }
        }
    }
}

/// How a source value becomes a Rust field expression.
///
/// Each coercion has two halves: a source-side *extraction* that produces
/// a catalog-shaped intermediate JSON value
/// (`generate::coerce_to_catalog_shape`), and an *expression* half that
/// turns a catalog-shaped value into Rust text (`emit`). Overrides supply
/// catalog-shaped values directly, so they flow through the expression
/// half only — override and source values are compared in the same shape
/// for the staleness lint. Coercions that can drop input records must
/// collect loud skips (Checkpoint A ruling 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coercion {
    /// String scalar → `&'static str` literal.
    StringLiteral,
    /// String-or-absent → `Option<&'static str>` literal.
    OptionalStringLiteral,
    /// Boolean → `bool` literal.
    BoolLiteral,
    /// String array → `&'static [&'static str]` (shared list emitter).
    StringSlice,
    /// Roster slug → `Provider::<Variant>` path expression via the fixed
    /// `emit::PROVIDER_VARIANTS` map. Generation references a variant, it
    /// can never create one (onboarding step 3 hand wiring).
    ProviderVariantFromSlug,
    /// Roster binding name → `AiCli::<Variant>` path expression.
    SniffBindingVariant,
    /// skills topic `support` enum → bool: `first_class`/`partial` → true,
    /// `convention_only`/`none`/`unknown` → false (Open question 3 ruling).
    SkillSupportToBool,
    /// Kebab-case protocol wire form (or null) →
    /// `Option<StreamProtocol>` path expression.
    StreamProtocolWire,
    /// agent-models `default_models[]` records → the deduplicated,
    /// lexically sorted list of `id` strings.
    DefaultModelsToStaticModels,
    /// agent-models `dynamic_listing.available` → [`ModelCatalogSource`]
    /// expression. `false` maps to `static`; `true` cannot select a
    /// mechanism (the boolean carries no program/args information) and
    /// demands a field-keyed override pinning today's value — a bare
    /// member string for the unit variants, or the externally tagged
    /// `{shell_command: {program, args}}` object.
    DynamicListingToModelCatalogSource,
    /// agent-models `model_selection[]` records → the list of bare env-var
    /// identifiers (`method == "env_var"` and `site` is a single
    /// `[A-Z][A-Z0-9_]*` token; compound "A / B" sites are skipped
    /// loudly), document order, deduplicated.
    EnvVarSitesToStringSlice,
    /// agent-logging `surfaces[]` records → `path_macos` of every
    /// `role == session_transcript` record, document order. Annotated
    /// pseudo-paths (containing " (") are skipped loudly.
    SurfacesToSessionLogPaths,
    /// agent-cli `config_paths[]` records → `path` of every macOS
    /// user/repo-scoped record, document order. Non-bare paths (prose
    /// annotations, `<...>` placeholders, env-var references) are skipped
    /// loudly.
    ConfigPathRecordsToConfigPaths,
    /// Facts `[{raw, segments}]` records → `&[PathTemplate]` expression
    /// (`Static` only; templated segments have no current constant).
    PathTemplateList,
    /// Facts event-mapping table → the named `EventMappingTable` static.
    EventMappingRecords,
    /// Facts resource-support record → the `LazyLock`-backed
    /// `ProviderCapabilities` builder.
    ResourceSupportRecord,
    /// Facts output-format records → `&[OutputFormatSupport]`.
    OutputFormatRecords,
    /// Facts entrypoint records → `&[EntrypointSpec]`.
    EntrypointRecords,
    /// Facts system-prompt record → `&'static SystemPromptSpec`
    /// (referencing the shared memory-files const).
    SystemPromptSpecRecord,
    /// Facts YOLO record → [`YoloSupport`] expression.
    YoloRecordToYoloSupport,
    /// Facts reasoning record → `ReasoningSupport` expression.
    ReasoningRecord,
    /// Facts known-gap records → `&[KnownGap]`.
    KnownGapRecords,
    /// Facts ACP record → `AcpSupport` expression.
    AcpRecord,
    /// Facts prompt-arg record → `PromptArgConventions` expression
    /// (post-OQ7a shape: `prompt_flags` + `entrypoint`).
    PromptArgRecord,
    /// Facts axes record → `CliSensitiveAxes` expression.
    AxesRecord,
    /// resume topic `support` enum member → `ResumeSupport` expression.
    ResumeSupportMember,
    /// agent-models `model_selection[]` records → the FIRST
    /// `method == cli_flag` record whose `site` is a single bare flag
    /// token (starts with `-`, no whitespace), or null when none exists.
    /// Non-bare cli_flag sites are skipped loudly.
    CliFlagSitesToFlag,
    /// String array → the entries that are single bare flag tokens;
    /// annotated / prose / env-var entries are skipped loudly.
    FlagListToStringSlice,
    /// Facts snake_case member list → `&[BillingModel]`.
    BillingModelList,
}

impl Coercion {
    /// snake_case label for `--mapping` output.
    pub fn label(&self) -> &'static str {
        match self {
            Coercion::StringLiteral => "string_literal",
            Coercion::OptionalStringLiteral => "optional_string_literal",
            Coercion::BoolLiteral => "bool_literal",
            Coercion::StringSlice => "string_slice",
            Coercion::ProviderVariantFromSlug => "provider_variant_from_slug",
            Coercion::SniffBindingVariant => "sniff_binding_variant",
            Coercion::SkillSupportToBool => "skill_support_to_bool",
            Coercion::StreamProtocolWire => "stream_protocol_wire",
            Coercion::DefaultModelsToStaticModels => "default_models_to_static_models",
            Coercion::DynamicListingToModelCatalogSource => {
                "dynamic_listing_to_model_catalog_source"
            }
            Coercion::EnvVarSitesToStringSlice => "env_var_sites_to_string_slice",
            Coercion::SurfacesToSessionLogPaths => "surfaces_to_session_log_paths",
            Coercion::ConfigPathRecordsToConfigPaths => "config_path_records_to_config_paths",
            Coercion::PathTemplateList => "path_template_list",
            Coercion::EventMappingRecords => "event_mapping_records",
            Coercion::ResourceSupportRecord => "resource_support_record",
            Coercion::OutputFormatRecords => "output_format_records",
            Coercion::EntrypointRecords => "entrypoint_records",
            Coercion::SystemPromptSpecRecord => "system_prompt_spec_record",
            Coercion::YoloRecordToYoloSupport => "yolo_record_to_yolo_support",
            Coercion::ReasoningRecord => "reasoning_record",
            Coercion::KnownGapRecords => "known_gap_records",
            Coercion::AcpRecord => "acp_record",
            Coercion::PromptArgRecord => "prompt_arg_record",
            Coercion::AxesRecord => "axes_record",
            Coercion::ResumeSupportMember => "resume_support_member",
            Coercion::CliFlagSitesToFlag => "cli_flag_sites_to_flag",
            Coercion::FlagListToStringSlice => "flag_list_to_string_slice",
            Coercion::BillingModelList => "billing_model_list",
        }
    }
}

/// One mapping-registry row: a `ProviderInfo` field and where its value
/// comes from.
#[derive(Debug, Clone, Copy)]
pub struct RegistryEntry {
    /// `ProviderInfo` field name (also the key overrides and facts use).
    pub field: &'static str,
    pub source: DeclaredSource,
    pub expected: &'static [SchemaExpectation],
    pub coercion: Coercion,
    /// Optionality marker (Open question 6): a missing source key (or
    /// explicit null) resolves to JSON null instead of erroring.
    pub optional: bool,
    /// One-line field description (feeds `--mapping` and generated docs).
    pub description: &'static str,
}

/// Convenience: the common non-optional entry shape.
const fn entry(
    field: &'static str,
    source: DeclaredSource,
    expected: &'static [SchemaExpectation],
    coercion: Coercion,
    description: &'static str,
) -> RegistryEntry {
    RegistryEntry {
        field,
        source,
        expected,
        coercion,
        optional: false,
        description,
    }
}

/// The skills-topic `support` vocabulary consumed by
/// [`Coercion::SkillSupportToBool`]. A sidecar member outside this list
/// fails the schema↔catalog gate before any research value is read.
pub const SKILL_SUPPORT_MEMBERS: &[&str] =
    &["first_class", "partial", "convention_only", "none", "unknown"];

/// The generator-v1 mapping registry, in `ProviderInfo` serialization
/// order (10 roster + 9 research + 22 facts = 41 fields).
pub const REGISTRY: &[RegistryEntry] = &[
    entry(
        "provider",
        DeclaredSource::Roster { key: "slug" },
        &[SchemaExpectation::String],
        Coercion::ProviderVariantFromSlug,
        "Canonical Provider enum variant, resolved from the roster slug",
    ),
    entry(
        "display_name",
        DeclaredSource::Roster {
            key: "display_name",
        },
        &[SchemaExpectation::String],
        Coercion::StringLiteral,
        "Friendly display name (distinct from the roster `name` heading)",
    ),
    entry(
        "slug",
        DeclaredSource::Roster { key: "slug" },
        &[SchemaExpectation::String],
        Coercion::StringLiteral,
        "Canonical snake-case slug for file paths and JSON keys",
    ),
    entry(
        "short_name",
        DeclaredSource::Roster { key: "short_name" },
        &[SchemaExpectation::String],
        Coercion::StringLiteral,
        "Short display name used in event logs and CLI output",
    ),
    entry(
        "binary",
        DeclaredSource::Roster { key: "binary" },
        &[SchemaExpectation::String],
        Coercion::StringLiteral,
        "Provider binary name on $PATH",
    ),
    entry(
        "agent_offset",
        DeclaredSource::Roster { key: "repo_dir" },
        &[SchemaExpectation::String],
        Coercion::StringLiteral,
        "Agent offset directory used for shadow-HOME isolation",
    ),
    entry(
        "cli_aliases",
        DeclaredSource::Roster {
            key: "cli_aliases",
        },
        &[SchemaExpectation::StringArray],
        Coercion::StringSlice,
        "CLI alias forms accepted on the command line",
    ),
    entry(
        "docs_url",
        DeclaredSource::Roster { key: "docs_url" },
        &[SchemaExpectation::String],
        Coercion::StringLiteral,
        "Provider documentation homepage (roster-owned; topic URLs are verification inputs)",
    ),
    RegistryEntry {
        field: "usage_dashboard_url",
        source: DeclaredSource::Roster {
            key: "usage_dashboard_url",
        },
        expected: &[SchemaExpectation::String],
        coercion: Coercion::OptionalStringLiteral,
        optional: true,
        description: "Usage / billing dashboard URL when one exists",
    },
    entry(
        "sniff_binding",
        DeclaredSource::Roster {
            key: "sniff_binding",
        },
        &[SchemaExpectation::String],
        Coercion::SniffBindingVariant,
        "sniff AiCli variant used for install detection",
    ),
    entry(
        "supports_skills",
        DeclaredSource::Research {
            topic: "skills",
            path: "support",
        },
        &[SchemaExpectation::EnumSubsetOf {
            rust_enum: "skill-support vocabulary (SkillSupportToBool)",
            variants: SKILL_SUPPORT_MEMBERS,
        }],
        Coercion::SkillSupportToBool,
        "Whether the provider supports skill discovery (graduated facts -> research at v1)",
    ),
    RegistryEntry {
        field: "stream_protocol",
        source: DeclaredSource::Facts {
            key: "stream_protocol",
        },
        expected: &[SchemaExpectation::String],
        coercion: Coercion::StreamProtocolWire,
        optional: true,
        description: "Structured stream format used in non-interactive mode, when present",
    },
    entry(
        "event_mapping",
        DeclaredSource::Facts {
            key: "event_mapping",
        },
        &[SchemaExpectation::Record {
            required_fields: &["mappings"],
        }],
        Coercion::EventMappingRecords,
        "Canonical event -> support-level/native-name/alias/registration table",
    ),
    entry(
        "resource_support",
        DeclaredSource::Facts {
            key: "resource_support",
        },
        &[SchemaExpectation::Record {
            required_fields: &["skills", "commands", "agents", "scripts", "skill_frontmatter"],
        }],
        Coercion::ResourceSupportRecord,
        "Resource portability descriptor (skills/commands/agents/scripts specs)",
    ),
    entry(
        "session_log_paths",
        DeclaredSource::Research {
            topic: "agent-logging",
            path: "surfaces",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["role", "path_macos", "format"],
        }],
        Coercion::SurfacesToSessionLogPaths,
        "Per-session transcript templates ({placeholder} grammar, from logging surfaces)",
    ),
    entry(
        "session_locations",
        DeclaredSource::Facts {
            key: "session_locations",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["raw", "segments"],
        }],
        Coercion::PathTemplateList,
        "Ancillary session-state directory templates",
    ),
    entry(
        "config_paths",
        DeclaredSource::Research {
            topic: "agent-cli",
            path: "config_paths",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["os", "scope", "path"],
        }],
        Coercion::ConfigPathRecordsToConfigPaths,
        "User / repo config file templates (macOS projection of the agent-cli inventory)",
    ),
    entry(
        "memory_files",
        DeclaredSource::Facts {
            key: "memory_files",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["raw", "segments"],
        }],
        Coercion::PathTemplateList,
        "Memory / instruction files contributing to the system prompt hierarchy",
    ),
    entry(
        "output_formats",
        DeclaredSource::Facts {
            key: "output_formats",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["format", "native_name", "selector"],
        }],
        Coercion::OutputFormatRecords,
        "Output formats supported in non-interactive mode",
    ),
    entry(
        "entrypoints",
        DeclaredSource::Facts {
            key: "entrypoints",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["mode", "required_flags"],
        }],
        Coercion::EntrypointRecords,
        "Available non-interactive (and selected interactive) entrypoints",
    ),
    entry(
        "system_prompt",
        DeclaredSource::Facts {
            key: "system_prompt",
        },
        &[SchemaExpectation::Record {
            required_fields: &["append", "replace", "memory_files"],
        }],
        Coercion::SystemPromptSpecRecord,
        "Typed system-prompt delivery descriptor (append/replace x interactive/non-interactive)",
    ),
    entry(
        "yolo",
        DeclaredSource::Facts { key: "yolo" },
        &[SchemaExpectation::String, SchemaExpectation::Record { required_fields: &[] }],
        Coercion::YoloRecordToYoloSupport,
        "Typed YOLO / auto-approve descriptor",
    ),
    entry(
        "reasoning",
        DeclaredSource::Facts { key: "reasoning" },
        &[SchemaExpectation::String, SchemaExpectation::Record { required_fields: &[] }],
        Coercion::ReasoningRecord,
        "Typed reasoning / extended-thinking descriptor",
    ),
    entry(
        "known_gaps",
        DeclaredSource::Facts { key: "known_gaps" },
        &[SchemaExpectation::RecordArray {
            required_fields: &["area", "note"],
        }],
        Coercion::KnownGapRecords,
        "Known gaps in provider capability data, classified by area (human-curated)",
    ),
    entry(
        "acp",
        DeclaredSource::Facts { key: "acp" },
        &[SchemaExpectation::Record {
            required_fields: &["server_mode", "client_supported", "events_via_acp"],
        }],
        Coercion::AcpRecord,
        "Typed ACP capability descriptor",
    ),
    entry(
        "prompt_arg_conventions",
        DeclaredSource::Facts {
            key: "prompt_arg_conventions",
        },
        &[SchemaExpectation::Record {
            required_fields: &["prompt_flags"],
        }],
        Coercion::PromptArgRecord,
        "Native-CLI prompt argv conventions (prompt flags + entrypoint; OQ7a shape)",
    ),
    entry(
        "static_models",
        DeclaredSource::Research {
            topic: "agent-models",
            path: "default_models",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["id"],
        }],
        Coercion::DefaultModelsToStaticModels,
        "Compiled-in model id list (deduplicated, lexically sorted research ids)",
    ),
    entry(
        "model_catalog_source",
        DeclaredSource::Research {
            topic: "agent-models",
            path: "dynamic_listing.available",
        },
        // Boolean today; the enum arm admits a future sidecar that reports
        // the catalog source directly — its members must then be a subset
        // of the Rust variants (the schema<->catalog compatibility gate).
        // `shell_command` carries data, so its catalog/override shape is
        // the externally tagged object, not a bare member string.
        &[
            SchemaExpectation::Boolean,
            SchemaExpectation::EnumSubsetOf {
                rust_enum: "ModelCatalogSource",
                variants: ModelCatalogSource::VARIANTS,
            },
        ],
        Coercion::DynamicListingToModelCatalogSource,
        "Source of the provider's model catalog",
    ),
    entry(
        "model_env_vars",
        DeclaredSource::Research {
            topic: "agent-models",
            path: "model_selection",
        },
        &[SchemaExpectation::RecordArray {
            required_fields: &["method", "site"],
        }],
        Coercion::EnvVarSitesToStringSlice,
        "Env vars consulted (in order) for the wrapper MODEL selection chain",
    ),
    entry(
        "cli_sensitive_axes",
        DeclaredSource::Facts {
            key: "cli_sensitive_axes",
        },
        &[SchemaExpectation::Record {
            required_fields: &["read_path", "write_path", "execute_command"],
        }],
        Coercion::AxesRecord,
        "Permission-policy axes a provider's CLI flags can override at runtime",
    ),
    entry(
        "repo_home_root_files",
        DeclaredSource::Facts {
            key: "repo_home_root_files",
        },
        &[SchemaExpectation::StringArray],
        Coercion::StringSlice,
        "Root-level repo-home files preserved during shadow-HOME isolation",
    ),
    entry(
        "resume",
        DeclaredSource::Research {
            topic: "resume",
            path: "support",
        },
        &[SchemaExpectation::EnumSubsetOf {
            rust_enum: "ResumeSupport",
            variants: ResumeSupport::VARIANTS,
        }],
        Coercion::ResumeSupportMember,
        "Overall resume support level (level only; argv mechanics stay in wrapper profiles)",
    ),
    RegistryEntry {
        field: "model_cli_flag",
        source: DeclaredSource::Research {
            topic: "agent-models",
            path: "model_selection",
        },
        expected: &[SchemaExpectation::RecordArray {
            required_fields: &["method", "site"],
        }],
        coercion: Coercion::CliFlagSitesToFlag,
        optional: true,
        description: "Bare CLI flag selecting the model at launch, when one exists",
    },
    entry(
        "non_interactive_conflicting_flags",
        DeclaredSource::Research {
            topic: "non-interactive-sessions",
            path: "claudine_strategy.conflicting_flags",
        },
        &[SchemaExpectation::StringArray],
        Coercion::FlagListToStringSlice,
        "Flags conflicting with Claudine's non-interactive wrapping strategy (bare tokens only)",
    ),
    entry(
        "billing_models",
        DeclaredSource::Facts {
            key: "billing_models",
        },
        &[SchemaExpectation::StringArray],
        Coercion::BillingModelList,
        "Billing models the provider offers",
    ),
    entry(
        "allowed_env_keys",
        DeclaredSource::Facts {
            key: "allowed_env_keys",
        },
        &[SchemaExpectation::StringArray],
        Coercion::StringSlice,
        "Hand-ruled allowlist of provider env keys bypassing the sensitive-key sanitizer",
    ),
    entry(
        "stdout_noise_prefixes",
        DeclaredSource::Facts {
            key: "stdout_noise_prefixes",
        },
        &[SchemaExpectation::StringArray],
        Coercion::StringSlice,
        "Curated stdout line prefixes suppressed as provider noise (non-interactive mode)",
    ),
    entry(
        "stderr_noise_prefixes",
        DeclaredSource::Facts {
            key: "stderr_noise_prefixes",
        },
        &[SchemaExpectation::StringArray],
        Coercion::StringSlice,
        "Curated stderr line prefixes suppressed as provider noise (all modes)",
    ),
    entry(
        "suppress_structured_stderr_on_success",
        DeclaredSource::Facts {
            key: "suppress_structured_stderr_on_success",
        },
        &[SchemaExpectation::Boolean],
        Coercion::BoolLiteral,
        "Whether structured runs buffer filtered stderr and surface it only on error",
    ),
    entry(
        "supports_interactive_inline_closure",
        DeclaredSource::Facts {
            key: "supports_interactive_inline_closure",
        },
        &[SchemaExpectation::Boolean],
        Coercion::BoolLiteral,
        "Whether a final assistant body is recoverable after an interactive session",
    ),
    entry(
        "model_required_in_non_tty",
        DeclaredSource::Facts {
            key: "model_required_in_non_tty",
        },
        &[SchemaExpectation::Boolean],
        Coercion::BoolLiteral,
        "Whether the provider hard-requires an explicit model in non-TTY sessions",
    ),
];

/// Serialized `--describe` fields deliberately NOT in the registry, with
/// the matrix's justification. The registry-covers-all-fields guard
/// asserts every serialized field is registry-covered XOR listed here.
pub const EXCLUDED_SERIALIZED_FIELDS: &[(&str, &str)] = &[
    // The behavior half (trait objects, fn-pointer accessors) is absent
    // from the serialized payload by construction and permanently owned by
    // behavior.rs — matrix exclusion 1. `resource_support.provider` (the
    // nested duplicate of the top-level discriminator) is not a top-level
    // field; the record emitter reproduces it from the `provider` row —
    // matrix exclusion 2. Neither appears as a serialized top-level key,
    // so this list is currently empty of live entries and exists to hold
    // future justified exclusions.
];

/// Looks up a registry entry by field name.
pub fn entry_for(field: &str) -> Option<&'static RegistryEntry> {
    REGISTRY.iter().find(|entry| entry.field == field)
}

/// Serializes the registry as the `--mapping` JSON document.
pub fn mapping_json() -> Value {
    let fields: Vec<Value> = REGISTRY
        .iter()
        .map(|entry| {
            let source = match entry.source {
                DeclaredSource::Roster { key } => json!({ "kind": "roster", "key": key }),
                DeclaredSource::Facts { key } => json!({ "kind": "facts", "key": key }),
                DeclaredSource::Research { topic, path } => {
                    json!({ "kind": "research", "topic": topic, "path": path })
                }
            };
            json!({
                "field": entry.field,
                "source": source,
                "expected": entry
                    .expected
                    .iter()
                    .map(SchemaExpectation::label)
                    .collect::<Vec<_>>(),
                "coercion": entry.coercion.label(),
                "optional": entry.optional,
                "description": entry.description,
            })
        })
        .collect();
    json!({
        "provider_scope": "all",
        "fields": fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_fields_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for entry in REGISTRY {
            assert!(seen.insert(entry.field), "duplicate field {}", entry.field);
        }
    }

    #[test]
    fn registry_matches_matrix_source_counts() {
        let count = |kind: &str| {
            REGISTRY
                .iter()
                .filter(|entry| entry.source.kind() == kind)
                .count()
        };
        assert_eq!(count("roster"), 10, "roster rows");
        assert_eq!(count("research"), 9, "research rows");
        assert_eq!(count("facts"), 22, "facts rows");
        assert_eq!(REGISTRY.len(), 41, "total serialized fields");
    }

    #[test]
    fn mapping_json_covers_every_entry() {
        let value = mapping_json();
        assert_eq!(value["fields"].as_array().unwrap().len(), REGISTRY.len());
        assert_eq!(value["provider_scope"], "all");
    }

    /// `supports_skills` graduated facts → research (Open question 3): the
    /// registry must never declare it facts again without a matrix ruling.
    #[test]
    fn supports_skills_is_research_declared() {
        let entry = entry_for("supports_skills").expect("registered");
        assert!(matches!(
            entry.source,
            DeclaredSource::Research {
                topic: "skills",
                path: "support"
            }
        ));
    }
}
