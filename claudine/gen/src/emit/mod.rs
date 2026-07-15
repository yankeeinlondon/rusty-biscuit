//! Rust-source emission: catalog-shaped JSON values → `data.rs` text.
//!
//! Every emitter here is the *expression half* of a [`Coercion`]: it turns
//! a catalog-shaped `serde_json::Value` (the shape overrides are authored
//! in) into deterministic Rust source text. The file assembler
//! [`emit_data_file`] joins the per-field expressions into a complete
//! `lib/src/provider/<slug>/data.rs`, including the behavior-wiring lines
//! and the `LazyLock`-backed resource-support builder.
//!
//! The per-field emitters live in domain submodules so each catalog area
//! has a predictable owner: [`identity_paths`] (provider/sniff/path
//! templates), [`execution_prompting`] (stream protocol, output formats,
//! system prompt, YOLO/reasoning, prompt args, policy axes),
//! [`models_offerings`] (billing, cap policies, offerings, resume, catalog
//! source), [`event_policy`] (event mapping, ACP, platform kind, gaps,
//! display policy), and [`linking`] (resource-support builder). The shared
//! literal/import helpers and the [`emit_data_file`] assembler stay here.
//!
//! Formatting is hand-rolled and deterministic; the drift test compares
//! generator output against the committed file, so internal consistency is
//! the only formatting contract (the workspace lint gate is clippy, not
//! rustfmt).

use std::collections::BTreeSet;

use serde_json::Value;

use crate::errors::GenError;

mod event_policy;
mod execution_prompting;
mod identity_paths;
mod linking;
mod models_offerings;

use event_policy::{
    acp, display_policy, event_mapping_table, known_gaps, platform_kind, unmapped_native_events,
};
use execution_prompting::{
    cli_sensitive_axes, entrypoints, output_formats, prompt_arg_conventions, reasoning,
    stream_protocol, system_prompt_spec, yolo,
};
use identity_paths::{path_list_from_records, path_list_from_strings, provider_expr, sniff_binding};
use linking::resource_support_builder;
use models_offerings::{
    billing_models, cap_policies, expected_offerings, model_catalog_source, offering_sources,
    resume_support,
};

/// Fixed slug → `Provider` variant map. Generation can never invent a
/// variant: onboarding step 3 (hand wiring) must add it here AND in the
/// lib's `Provider` enum before `claudine-gen generate <slug>` works.
pub const PROVIDER_VARIANTS: &[(&str, &str)] = &[
    ("claude", "Claude"),
    ("codex", "Codex"),
    ("gemini", "Gemini"),
    ("goose", "Goose"),
    ("kimi", "KimiCode"),
    ("opencode", "OpenCode"),
    ("qwen", "QwenCode"),
    ("kilo", "Kilo"),
    ("pi", "Pi"),
    ("antigravity", "Antigravity"),
];

/// Resolves the `Provider` variant name for a slug, failing loudly for
/// unknown slugs (the "new variant needed" moment).
pub fn provider_variant(slug: &str) -> Result<&'static str, GenError> {
    PROVIDER_VARIANTS
        .iter()
        .find(|(s, _)| *s == slug)
        .map(|(_, v)| *v)
        .ok_or_else(|| GenError::UnmappableValue {
            field: "provider",
            message: format!(
                "no Provider variant is wired for slug `{slug}` — add the enum variant and \
                 the PROVIDER_VARIANTS entry (onboarding step 3) before generating"
            ),
        })
}

/// Per-provider emission context: naming prefix, resolved variant, and the
/// set of `use` paths the emitted expressions need.
pub struct EmitCtx {
    /// Upper-cased const prefix, e.g. `CLAUDE`.
    pub prefix: String,
    /// `Provider` variant name, e.g. `KimiCode`.
    pub variant: &'static str,
    /// Full import paths (e.g. `crate::stream::StreamProtocol`) collected
    /// while emitting expressions.
    pub imports: BTreeSet<String>,
}

impl EmitCtx {
    pub fn new(slug: &str) -> Result<Self, GenError> {
        Ok(Self {
            prefix: slug.to_uppercase(),
            variant: provider_variant(slug)?,
            imports: BTreeSet::new(),
        })
    }

    fn import(&mut self, path: &str) {
        self.imports.insert(path.to_string());
    }
}

pub(crate) fn unmappable(field: &'static str, message: String) -> GenError {
    GenError::UnmappableValue { field, message }
}

/// `snake_case` → `PascalCase` (serde wire form → Rust variant name).
pub(crate) fn pascal(member: &str) -> String {
    member
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

pub(crate) fn expect_str<'v>(
    field: &'static str,
    value: &'v Value,
    what: &str,
) -> Result<&'v str, GenError> {
    value
        .as_str()
        .ok_or_else(|| unmappable(field, format!("expected a string for {what}, got `{value}`")))
}

pub(crate) fn expect_bool(field: &'static str, value: &Value, what: &str) -> Result<bool, GenError> {
    value
        .as_bool()
        .ok_or_else(|| unmappable(field, format!("expected a boolean for {what}, got `{value}`")))
}

pub(crate) fn expect_array<'v>(
    field: &'static str,
    value: &'v Value,
    what: &str,
) -> Result<&'v Vec<Value>, GenError> {
    value
        .as_array()
        .ok_or_else(|| unmappable(field, format!("expected an array for {what}, got `{value}`")))
}

pub(crate) fn get<'v>(field: &'static str, value: &'v Value, key: &str) -> Result<&'v Value, GenError> {
    value
        .get(key)
        .ok_or_else(|| unmappable(field, format!("missing key `{key}` in `{value}`")))
}

/// Externally-tagged enum helper: `"member"` → `(member, Null)`,
/// `{"member": payload}` → `(member, payload)`.
pub(crate) fn enum_shape(field: &'static str, value: &Value) -> Result<(String, Value), GenError> {
    match value {
        Value::String(member) => Ok((member.clone(), Value::Null)),
        Value::Object(map) if map.len() == 1 => {
            let (member, payload) = map.iter().next().expect("len checked");
            Ok((member.clone(), payload.clone()))
        }
        other => Err(unmappable(
            field,
            format!("expected an enum member string or single-key object, got `{other}`"),
        )),
    }
}

pub(crate) fn indent(level: usize) -> String {
    "    ".repeat(level)
}

pub fn string_literal(field: &'static str, value: &Value) -> Result<String, GenError> {
    Ok(format!("{:?}", expect_str(field, value, "the value")?))
}

pub fn bool_literal(field: &'static str, value: &Value) -> Result<String, GenError> {
    Ok(expect_bool(field, value, "the value")?.to_string())
}

pub fn optional_string_literal(field: &'static str, value: &Value) -> Result<String, GenError> {
    match value {
        Value::Null => Ok("None".to_string()),
        Value::String(s) => Ok(format!("Some({s:?})")),
        other => Err(unmappable(
            field,
            format!("expected a string or null, got `{other}`"),
        )),
    }
}

/// `&["a", "b"]`, wrapping to one-per-line when the inline form is long.
pub fn str_slice(field: &'static str, value: &Value, level: usize) -> Result<String, GenError> {
    let items = expect_array(field, value, "the list")?;
    let mut literals = Vec::with_capacity(items.len());
    for item in items {
        literals.push(format!("{:?}", expect_str(field, item, "a list element")?));
    }
    Ok(render_slice(&literals, level))
}

/// Renders `&[...]` from prerendered single-line element expressions.
pub(crate) fn render_slice(elements: &[String], level: usize) -> String {
    if elements.is_empty() {
        return "&[]".to_string();
    }
    let inline = format!("&[{}]", elements.join(", "));
    if indent(level).len() + inline.len() <= 88 && !inline.contains('\n') {
        return inline;
    }
    let inner = indent(level + 1);
    let mut out = String::from("&[\n");
    for element in elements {
        out.push_str(&format!("{inner}{element},\n"));
    }
    out.push_str(&format!("{}]", indent(level)));
    out
}

/// Renders `&[...]` from prerendered element expressions that are struct
/// literals (always one element per line).
pub(crate) fn render_struct_slice(elements: &[String], level: usize) -> String {
    if elements.is_empty() {
        return "&[]".to_string();
    }
    let inner = indent(level + 1);
    let mut out = String::from("&[\n");
    for element in elements {
        out.push_str(&format!("{inner}{element},\n"));
    }
    out.push_str(&format!("{}]", indent(level)));
    out
}

pub(crate) fn number_u32(field: &'static str, value: &Value) -> Result<u32, GenError> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| unmappable(field, format!("expected a u32 number, got `{value}`")))
}

/// One resolved catalog field handed to the assembler: the registry field
/// name plus its catalog-shaped value (post-override).
pub struct FieldValue<'a> {
    pub field: &'static str,
    pub value: &'a Value,
}

/// Assembles the complete generated `data.rs` for `slug` from the resolved
/// catalog values (registry order).
///
/// Special-cased fields:
/// - `provider` drives the `Provider::<Variant>` expression and the
///   `ProviderCapabilities.provider` discriminator (the nested
///   `resource_support.provider` copy in the input is ignored by design —
///   field-source matrix exclusion 2).
/// - `event_mapping` is emitted as the named
///   `pub(in crate::provider) static <PREFIX>_EVENT_MAPPING` because
///   behavior modules reference the table directly.
/// - `memory_files` becomes a shared `<PREFIX>_MEMORY_FILES` const that
///   `system_prompt.memory_files` references when the two lists are equal
///   (they are one list semantically; a divergence is a loud error).
/// - `resource_support` becomes the `LazyLock` + accessor + builder-fn
///   trio; the four behavior trait objects and the two fn-pointer
///   accessors are fixed structural wiring parameterized by the provider
///   module's `behavior.rs`.
pub fn emit_data_file(
    slug: &str,
    display_name: &str,
    values: &[FieldValue<'_>],
) -> Result<String, GenError> {
    let mut ctx = EmitCtx::new(slug)?;
    let lookup = |field: &str| -> Result<&Value, GenError> {
        values
            .iter()
            .find(|fv| fv.field == field)
            .map(|fv| fv.value)
            .ok_or_else(|| GenError::MissingValue {
                field: "data.rs",
                message: format!("assembler received no resolved value for `{field}`"),
            })
    };

    let prefix = ctx.prefix.clone();
    let memory_const = format!("{prefix}_MEMORY_FILES");
    let event_static = format!("{prefix}_EVENT_MAPPING");
    let lazy_static = format!("{prefix}_RESOURCE_SUPPORT");

    // system_prompt.memory_files must equal the top-level memory_files
    // list — the generated shape references ONE list (matrix v1 cleanup).
    let memory_value = lookup("memory_files")?;
    let system_prompt_value = lookup("system_prompt")?;
    if system_prompt_value.get("memory_files") != Some(memory_value) {
        return Err(GenError::UnmappableValue {
            field: "system_prompt",
            message: "system_prompt.memory_files diverges from the top-level memory_files \
                      list — the catalog models one list; reconcile the inputs"
                .to_string(),
        });
    }

    // INFO field lines, ProviderInfo declaration order.
    let mut info_lines: Vec<String> = Vec::new();
    let mut push = |field: &str, expr: String| {
        info_lines.push(format!("    {field}: {expr},"));
    };

    push("provider", provider_expr(&mut ctx));
    push("display_name", string_literal("display_name", lookup("display_name")?)?);
    push("slug", string_literal("slug", lookup("slug")?)?);
    push("short_name", string_literal("short_name", lookup("short_name")?)?);
    push("binary", string_literal("binary", lookup("binary")?)?);
    push("agent_offset", string_literal("agent_offset", lookup("agent_offset")?)?);
    push("cli_aliases", str_slice("cli_aliases", lookup("cli_aliases")?, 1)?);
    push("docs_url", string_literal("docs_url", lookup("docs_url")?)?);
    push(
        "usage_dashboard_url",
        optional_string_literal("usage_dashboard_url", lookup("usage_dashboard_url")?)?,
    );
    push(
        "sniff_binding",
        sniff_binding("sniff_binding", lookup("sniff_binding")?, &mut ctx)?,
    );
    push(
        "supports_skills",
        bool_literal("supports_skills", lookup("supports_skills")?)?,
    );
    push(
        "stream_protocol",
        stream_protocol("stream_protocol", lookup("stream_protocol")?, &mut ctx)?,
    );
    push("event_mapping", format!("&{event_static}"));
    // Behavior wiring: fixed structural boilerplate parameterized by the
    // provider ident; the implementations live in behavior.rs.
    let provider_static = format!("{prefix}_PROVIDER");
    push("behavior", format!("&{provider_static}"));
    push("mcp", format!("&{provider_static}"));
    push("adapter", format!("&{provider_static}"));
    push("configurator", format!("&{provider_static}"));
    push("resource_support_fn", "resource_support".to_string());
    push(
        "session_log_paths",
        path_list_from_strings("session_log_paths", lookup("session_log_paths")?, 1, &mut ctx)?,
    );
    push(
        "config_paths",
        path_list_from_strings("config_paths", lookup("config_paths")?, 1, &mut ctx)?,
    );
    push("memory_files", memory_const.clone());
    push(
        "output_formats",
        output_formats("output_formats", lookup("output_formats")?, 1, &mut ctx)?,
    );
    push(
        "entrypoints",
        entrypoints("entrypoints", lookup("entrypoints")?, 1, &mut ctx)?,
    );
    push(
        "system_prompt",
        system_prompt_spec("system_prompt", system_prompt_value, &memory_const, 1, &mut ctx)?,
    );
    push("yolo", yolo("yolo", lookup("yolo")?, 1, &mut ctx)?);
    push("reasoning", reasoning("reasoning", lookup("reasoning")?, 1, &mut ctx)?);
    push("known_gaps", known_gaps("known_gaps", lookup("known_gaps")?, 1, &mut ctx)?);
    push("acp", acp("acp", lookup("acp")?, 1, &mut ctx)?);
    push(
        "prompt_arg_conventions",
        prompt_arg_conventions(
            "prompt_arg_conventions",
            lookup("prompt_arg_conventions")?,
            1,
            &mut ctx,
        )?,
    );
    push(
        "expected_offerings",
        expected_offerings("expected_offerings", lookup("expected_offerings")?, 1, &mut ctx)?,
    );
    push(
        "offering_sources",
        offering_sources("offering_sources", lookup("offering_sources")?, 1, &mut ctx)?,
    );
    push(
        "model_catalog_source",
        model_catalog_source("model_catalog_source", lookup("model_catalog_source")?, &mut ctx)?,
    );
    push(
        "model_env_vars",
        str_slice("model_env_vars", lookup("model_env_vars")?, 1)?,
    );
    push(
        "cli_sensitive_axes",
        cli_sensitive_axes("cli_sensitive_axes", lookup("cli_sensitive_axes")?, 1, &mut ctx)?,
    );
    push(
        "repo_home_root_files",
        str_slice("repo_home_root_files", lookup("repo_home_root_files")?, 1)?,
    );
    push("resume", resume_support("resume", lookup("resume")?, &mut ctx)?);
    push(
        "model_cli_flag",
        optional_string_literal("model_cli_flag", lookup("model_cli_flag")?)?,
    );
    push(
        "non_interactive_conflicting_flags",
        str_slice(
            "non_interactive_conflicting_flags",
            lookup("non_interactive_conflicting_flags")?,
            1,
        )?,
    );
    push(
        "billing_models",
        billing_models("billing_models", lookup("billing_models")?, 1, &mut ctx)?,
    );
    push(
        "cap_policies",
        cap_policies("cap_policies", lookup("cap_policies")?, 1, &mut ctx)?,
    );
    push(
        "allowed_env_keys",
        str_slice("allowed_env_keys", lookup("allowed_env_keys")?, 1)?,
    );
    push(
        "display_policy",
        display_policy("display_policy", lookup("display_policy")?, 1, &mut ctx)?,
    );
    push(
        "suppress_structured_stderr_on_success",
        bool_literal(
            "suppress_structured_stderr_on_success",
            lookup("suppress_structured_stderr_on_success")?,
        )?,
    );
    push(
        "supports_interactive_inline_closure",
        bool_literal(
            "supports_interactive_inline_closure",
            lookup("supports_interactive_inline_closure")?,
        )?,
    );
    push(
        "model_required_in_non_tty",
        bool_literal("model_required_in_non_tty", lookup("model_required_in_non_tty")?)?,
    );
    push(
        "platform_kind",
        platform_kind("platform_kind", lookup("platform_kind")?, &mut ctx)?,
    );
    push(
        "unmapped_native_events",
        unmapped_native_events(
            "unmapped_native_events",
            lookup("unmapped_native_events")?,
            1,
            &mut ctx,
        )?,
    );

    // Supporting items (emitted after INFO).
    let event_table = event_mapping_table("event_mapping", lookup("event_mapping")?, 0, &mut ctx)?;
    let memory_files =
        path_list_from_records("memory_files", memory_value, 0, &mut ctx)?;
    let builder =
        resource_support_builder("resource_support", lookup("resource_support")?, &mut ctx)?;
    ctx.import("std::sync::LazyLock");
    ctx.import("crate::provider::ProviderInfo");
    ctx.import("crate::linking::capabilities::ProviderCapabilities");
    ctx.import("crate::provider::path_template::PathTemplate");

    // Assemble.
    let mut out = String::new();
    out.push_str(&format!(
        "// GENERATED by claudine-gen — DO NOT EDIT BY HAND.\n\
         //\n\
         // Inputs (field-source matrix, design/catalog-generation.md):\n\
         //   docs/providers.yaml — roster identity\n\
         //   docs/providers/facts/{slug}.yaml — topic-less facts\n\
         //   docs/providers/overrides/{slug}.yaml — human overrides (win over any source)\n\
         //   docs/research/<topic>/{slug}.md — sidecar-validated research frontmatter\n\
         // Regenerate with `cargo run -p claudine-gen -- generate`; drift-check with\n\
         // `cargo run -p claudine-gen -- check` (the same code path as the drift test).\n\
         \n\
         //! Typed static catalog data for the {display_name} provider (generated).\n\n"
    ));
    out.push_str(&render_imports(&ctx.imports, &provider_static));
    out.push('\n');
    out.push_str(&format!(
        "static {lazy_static}: LazyLock<ProviderCapabilities> =\n    LazyLock::new(build_resource_support);\n\n\
         fn resource_support() -> &'static ProviderCapabilities {{\n    &{lazy_static}\n}}\n\n"
    ));
    out.push_str(&format!(
        "pub(in crate::provider) static {prefix}_INFO: ProviderInfo = ProviderInfo {{\n"
    ));
    for line in &info_lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("};\n\n");
    out.push_str(
        "/// Event-mapping table (also referenced directly by behavior modules).\n",
    );
    out.push_str(&format!(
        "pub(in crate::provider) static {event_static}: EventMappingTable = {event_table};\n\n"
    ));
    out.push_str(
        "/// Memory / instruction files contributing to the system prompt\n\
         /// hierarchy — one list, shared by `memory_files` and\n\
         /// `system_prompt.memory_files`.\n",
    );
    out.push_str(&format!(
        "const {memory_const}: &[PathTemplate] = {memory_files};\n\n"
    ));
    out.push_str(&builder);
    Ok(out)
}

/// Renders the `use` block: std, external (sniff), crate, then the fixed
/// `super::` wiring imports — each group blank-line separated, paths
/// grouped per module with sorted names.
fn render_imports(paths: &BTreeSet<String>, provider_static: &str) -> String {
    let mut std_group: Vec<&str> = Vec::new();
    let mut extern_group: Vec<&str> = Vec::new();
    let mut crate_group: Vec<&str> = Vec::new();
    for path in paths {
        if path.starts_with("std::") {
            std_group.push(path);
        } else if path.starts_with("crate::") {
            crate_group.push(path);
        } else {
            extern_group.push(path);
        }
    }

    let mut out = String::new();
    for group in [std_group, extern_group, crate_group] {
        if group.is_empty() {
            continue;
        }
        // Group by parent module, preserving BTreeSet order.
        let mut by_module: Vec<(String, Vec<String>)> = Vec::new();
        for path in group {
            let (module, name) = path.rsplit_once("::").expect("import paths are qualified");
            match by_module.last_mut() {
                Some((last, names)) if *last == module => names.push(name.to_string()),
                _ => by_module.push((module.to_string(), vec![name.to_string()])),
            }
        }
        by_module.sort();
        for (module, mut names) in by_module {
            names.sort();
            names.dedup();
            if names.len() == 1 {
                out.push_str(&format!("use {module}::{};\n", names[0]));
            } else {
                out.push_str(&format!("use {module}::{{{}}};\n", names.join(", ")));
            }
        }
        out.push('\n');
    }
    out.push_str(&format!("use super::behavior::{provider_static};\n"));
    out
}

#[cfg(test)]
mod tests;
