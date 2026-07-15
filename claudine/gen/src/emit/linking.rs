//! Linking-resource emitters: the `LazyLock`-backed `ProviderCapabilities`
//! builder plus its per-resource support entries and the skill-frontmatter
//! capability record.

use super::*;

/// The complete `fn build_resource_support() -> ProviderCapabilities` item.
pub(crate) fn resource_support_builder(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::linking::capabilities::ProviderCapabilities");
    ctx.import("crate::provider::identity::Provider");
    let mut out = String::new();
    out.push_str(
        "/// Builds the resource portability descriptor served through\n\
         /// `resource_support_fn` (heap-backed, so it lives behind the `LazyLock`\n\
         /// rather than in the `ProviderInfo` static).\n",
    );
    out.push_str("fn build_resource_support() -> ProviderCapabilities {\n");
    out.push_str("    ProviderCapabilities {\n");
    out.push_str(&format!("        provider: Provider::{},\n", ctx.variant));
    for resource in ["skills", "commands", "agents", "scripts"] {
        let support = resource_support_entry(field, get(field, value, resource)?, 2, ctx)?;
        out.push_str(&format!("        {resource}: {support},\n"));
    }
    let frontmatter = skill_frontmatter(field, get(field, value, "skill_frontmatter")?, 2, ctx)?;
    out.push_str(&format!("        skill_frontmatter: {frontmatter},\n"));
    out.push_str("    }\n}\n");
    Ok(out)
}

fn resource_support_entry(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::linking::capabilities::ResourceSupport");
    ctx.import("crate::linking::capabilities::SupportLevel");
    let level_name = expect_str(field, get(field, value, "level")?, "`level`")?;
    if !matches!(level_name, "Full" | "CustomFormat" | "Limited" | "None") {
        return Err(unmappable(
            field,
            format!("`{level_name}` is not a SupportLevel wire form"),
        ));
    }
    let format = match get(field, value, "format")? {
        Value::Null => "None".to_string(),
        name => {
            ctx.import("crate::linking::capabilities::ResourceFormat");
            let name = expect_str(field, name, "`format`")?;
            if !matches!(
                name,
                "Markdown" | "Toml" | "Yaml" | "Mcp" | "BuiltinOnly" | "Executable"
            ) {
                return Err(unmappable(
                    field,
                    format!("`{name}` is not a ResourceFormat wire form"),
                ));
            }
            format!("Some(ResourceFormat::{name})")
        }
    };
    let mut path_opt = |value: &Value| -> Result<String, GenError> {
        match value {
            Value::Null => Ok("None".to_string()),
            path => {
                ctx.import("std::path::PathBuf");
                Ok(format!(
                    "Some(PathBuf::from({:?}))",
                    expect_str(field, path, "a resource path")?
                ))
            }
        }
    };
    let repo_path = path_opt(get(field, value, "repo_path")?)?;
    let user_path = path_opt(get(field, value, "user_path")?)?;
    let also = expect_array(field, get(field, value, "also_reads_from")?, "`also_reads_from`")?;
    let also_reads = if also.is_empty() {
        "vec![]".to_string()
    } else {
        ctx.import("std::path::PathBuf");
        let mut elements = Vec::with_capacity(also.len());
        for path in also {
            elements.push(format!(
                "PathBuf::from({:?})",
                expect_str(field, path, "an also-reads path")?
            ));
        }
        format!("vec![{}]", elements.join(", "))
    };
    let notes = optional_string_literal(field, get(field, value, "notes")?)?;
    let properties = match get(field, value, "properties")? {
        Value::Null => "None".to_string(),
        schema => {
            ctx.import("crate::linking::capabilities::ResourcePropertySchema");
            let required = str_slice(field, get(field, schema, "required")?, level + 2)?;
            let optional = str_slice(field, get(field, schema, "optional")?, level + 2)?;
            let source_doc = expect_str(field, get(field, schema, "source_doc")?, "`source_doc`")?;
            let inner = indent(level + 2);
            format!(
                "Some(ResourcePropertySchema::new(\n\
                 {inner}{required},\n\
                 {inner}{optional},\n\
                 {inner}{source_doc:?},\n\
                 {}))",
                indent(level + 1)
            )
        }
    };
    let inner = indent(level + 1);
    Ok(format!(
        "ResourceSupport {{\n\
         {inner}level: SupportLevel::{level_name},\n\
         {inner}format: {format},\n\
         {inner}repo_path: {repo_path},\n\
         {inner}user_path: {user_path},\n\
         {inner}also_reads_from: {also_reads},\n\
         {inner}notes: {notes},\n\
         {inner}properties: {properties},\n\
         {}}}",
        indent(level)
    ))
}

/// `SkillFrontmatter` field names in declaration order.
const SKILL_FRONTMATTER_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed_tools",
    "user_invocable",
    "disable_model_invocation",
];

fn skill_frontmatter(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::linking::capabilities::SkillFrontmatter");
    let inner = indent(level + 1);
    let mut out = String::from("SkillFrontmatter {\n");
    for name in SKILL_FRONTMATTER_FIELDS {
        let flag = expect_bool(field, get(field, value, name)?, name)?;
        out.push_str(&format!("{inner}{name}: {flag},\n"));
    }
    out.push_str(&format!("{}}}", indent(level)));
    Ok(out)
}
