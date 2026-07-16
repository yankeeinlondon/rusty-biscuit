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

pub(crate) struct ResolvedValues<'a> {
    values: &'a [FieldValue<'a>],
}

impl<'a> ResolvedValues<'a> {
    fn new(values: &'a [FieldValue<'a>]) -> Self {
        Self { values }
    }

    pub(crate) fn get(&self, field: &'static str) -> Result<&'a Value, GenError> {
        self.values
            .iter()
            .find(|value| value.field == field)
            .map(|value| value.value)
            .ok_or_else(|| GenError::MissingValue {
                field: "data.rs",
                message: format!("assembler received no resolved value for `{field}`"),
            })
    }
}

pub(crate) struct EmittedField {
    order: u8,
    name: &'static str,
    expression: String,
}

pub(crate) struct EmissionFragment {
    fields: Vec<EmittedField>,
    supporting_items: Vec<String>,
}

impl EmissionFragment {
    pub(crate) fn new() -> Self {
        Self {
            fields: Vec::new(),
            supporting_items: Vec::new(),
        }
    }

    pub(crate) fn field(&mut self, order: u8, name: &'static str, expression: String) {
        self.fields.push(EmittedField {
            order,
            name,
            expression,
        });
    }

    pub(crate) fn supporting_item(&mut self, item: String) {
        self.supporting_items.push(item);
    }
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
    let values = ResolvedValues::new(values);
    let prefix = ctx.prefix.clone();
    let memory_const = format!("{prefix}_MEMORY_FILES");
    let event_static = format!("{prefix}_EVENT_MAPPING");
    if values.get("system_prompt")?.get("memory_files") != Some(values.get("memory_files")?) {
        return Err(GenError::UnmappableValue {
            field: "system_prompt",
            message: "system_prompt.memory_files diverges from the top-level memory_files \
                      list — the catalog models one list; reconcile the inputs"
                .to_string(),
        });
    }
    let provider_static = format!("{prefix}_PROVIDER");
    let mut fragments = vec![
        event_policy::emission_fragment(&values, &event_static, &mut ctx)?,
        identity_paths::emission_fragment(&values, &memory_const, &mut ctx)?,
        execution_prompting::emission_fragment(&values, &memory_const, &mut ctx)?,
        models_offerings::emission_fragment(&values, &mut ctx)?,
        linking::emission_fragment(&values, &mut ctx)?,
        core_emission_fragment(&values, &provider_static)?,
    ];
    ctx.import("std::sync::LazyLock");
    ctx.import("crate::provider::ProviderInfo");
    ctx.import("crate::linking::capabilities::ProviderCapabilities");
    ctx.import("crate::provider::path_template::PathTemplate");
    let mut fields = fragments
        .iter_mut()
        .flat_map(|fragment| fragment.fields.drain(..))
        .collect::<Vec<_>>();
    fields.sort_by_key(|field| field.order);
    if fields.len() != 47 || fields.iter().enumerate().any(|(order, field)| order != field.order as usize) {
        return Err(unmappable(
            "data.rs",
            "domain fragments did not emit each ProviderInfo field exactly once".to_string(),
        ));
    }
    let supporting_items = fragments
        .into_iter()
        .flat_map(|fragment| fragment.supporting_items)
        .collect::<Vec<_>>();
    Ok(render_data_file(
        slug,
        display_name,
        &prefix,
        &provider_static,
        &ctx.imports,
        &fields,
        &supporting_items,
    ))
}

fn core_emission_fragment(
    values: &ResolvedValues<'_>,
    provider_static: &str,
) -> Result<EmissionFragment, GenError> {
    let mut fragment = EmissionFragment::new();
    for (order, name, expression) in [
        (13, "behavior", format!("&{provider_static}")),
        (14, "mcp", format!("&{provider_static}")),
        (15, "adapter", format!("&{provider_static}")),
        (16, "configurator", format!("&{provider_static}")),
        (17, "resource_support_fn", "resource_support".to_string()),
    ] {
        fragment.field(order, name, expression);
    }
    fragment.field(40, "allowed_env_keys", str_slice("allowed_env_keys", values.get("allowed_env_keys")?, 1)?);
    fragment.field(
        42,
        "suppress_structured_stderr_on_success",
        bool_literal(
            "suppress_structured_stderr_on_success",
            values.get("suppress_structured_stderr_on_success")?,
        )?,
    );
    fragment.field(
        43,
        "supports_interactive_inline_closure",
        bool_literal(
            "supports_interactive_inline_closure",
            values.get("supports_interactive_inline_closure")?,
        )?,
    );
    fragment.field(
        44,
        "model_required_in_non_tty",
        bool_literal("model_required_in_non_tty", values.get("model_required_in_non_tty")?)?,
    );
    Ok(fragment)
}

fn render_data_file(
    slug: &str,
    display_name: &str,
    prefix: &str,
    provider_static: &str,
    imports: &BTreeSet<String>,
    fields: &[EmittedField],
    supporting_items: &[String],
) -> String {
    let lazy_static = format!("{prefix}_RESOURCE_SUPPORT");
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
    out.push_str(&render_imports(imports, provider_static));
    out.push('\n');
    out.push_str(&format!(
        "static {lazy_static}: LazyLock<ProviderCapabilities> =\n    LazyLock::new(build_resource_support);\n\n\
         fn resource_support() -> &'static ProviderCapabilities {{\n    &{lazy_static}\n}}\n\n"
    ));
    out.push_str(&format!(
        "pub(in crate::provider) static {prefix}_INFO: ProviderInfo = ProviderInfo {{\n"
    ));
    for field in fields {
        out.push_str(&format!("    {}: {},\n", field.name, field.expression));
    }
    out.push_str("};\n\n");
    for (index, item) in supporting_items.iter().enumerate() {
        out.push_str(item);
        if index + 1 < supporting_items.len() {
            out.push('\n');
        }
    }
    out
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
