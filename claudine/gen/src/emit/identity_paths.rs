//! Identity and path emitters: the `Provider` expression, the sniff
//! install-detection binding, and the `PathTemplate` list forms (from
//! research string arrays and from the facts `{raw, segments}` records).

use strum::VariantNames;

use super::*;

pub(crate) fn provider_expr(ctx: &mut EmitCtx) -> String {
    ctx.import("crate::provider::identity::Provider");
    format!("Provider::{}", ctx.variant)
}

pub(crate) fn sniff_binding(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let name = expect_str(field, value, "the sniff binding")?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(unmappable(
            field,
            format!("`{name}` is not a plausible AiCli variant identifier"),
        ));
    }
    ctx.import("sniff::programs::AiCli");
    Ok(format!("AiCli::{name}"))
}

/// `&[PathTemplate::Static("..."), ...]` from a catalog-shaped string
/// array (research-fed path fields).
pub(crate) fn path_list_from_strings(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let items = expect_array(field, value, "the path list")?;
    let mut elements = Vec::with_capacity(items.len());
    for item in items {
        let raw = expect_str(field, item, "a path")?;
        ctx.import("crate::provider::path_template::PathTemplate");
        elements.push(format!("PathTemplate::Static({raw:?})"));
    }
    Ok(render_slice(&elements, level))
}

/// `&[PathTemplate::Static("..."), ...]` from the facts / serialized shape
/// (`[{raw, segments}]`). Templated entries (non-empty `segments`) have no
/// current constant and fail loudly rather than guessing a segment
/// emission.
pub(crate) fn path_list_from_records(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the path-template list")?;
    let mut raws = Vec::with_capacity(records.len());
    for record in records {
        let raw = expect_str(field, get(field, record, "raw")?, "the `raw` template")?;
        let segments = expect_array(field, get(field, record, "segments")?, "`segments`")?;
        if !segments.is_empty() {
            return Err(unmappable(
                field,
                format!(
                    "templated path emission (non-empty `segments`) is not implemented — \
                     no current constant uses it (raw: {raw:?})"
                ),
            ));
        }
        raws.push(Value::String(raw.to_string()));
    }
    path_list_from_strings(field, &Value::Array(raws), level, ctx)
}

/// Checks a member against a catalog-types variant list and returns the
/// Rust variant name. An unknown member is the "new variant needed" moment
/// and fails loudly rather than being dropped.
fn variant_of(
    field: &'static str,
    value: &Value,
    what: &str,
    variants: &[&str],
) -> Result<String, GenError> {
    let member = expect_str(field, value, what)?;
    if !variants.contains(&member) {
        return Err(unmappable(
            field,
            format!("`{member}` is not one of {variants:?}"),
        ));
    }
    Ok(pascal(member))
}

/// `Some(&OverlaySelectorSpec { ... })` — or `None` for a provider with no
/// provider-scoped redirection surface.
///
/// `source_root` must be home-relative (`~/...`): an overlay's source root
/// is resolved against the launch baseline's home on every OS, so an
/// absolute or drive-qualified root would not be portable.
pub(crate) fn overlay_selector(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    if value.is_null() {
        return Ok("None".to_string());
    }
    ctx.import("crate::provider::overlay::OverlaySelectorSpec");
    let env_var = expect_str(field, get(field, value, "env_var")?, "`env_var`")?;
    if env_var.is_empty()
        || !env_var
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(unmappable(
            field,
            format!("`{env_var}` is not a plausible environment-variable name"),
        ));
    }

    ctx.import("crate::provider::overlay::OverlaySelectorShape");
    let (member, payload) = enum_shape(field, get(field, value, "shape")?)?;
    let shape = match variant_of(
        field,
        &Value::String(member.clone()),
        "`shape`",
        claudine_catalog_types::OverlaySelectorShape::VARIANTS,
    )?
    .as_str()
    {
        "ParentOfProviderDir" => {
            let child = expect_str(field, get(field, &payload, "child")?, "`shape.child`")?;
            if child.is_empty() || child.contains(['/', '\\']) {
                return Err(unmappable(
                    field,
                    format!("`{child}` is not a single path segment"),
                ));
            }
            format!("OverlaySelectorShape::ParentOfProviderDir {{ child: {child:?} }}")
        }
        unit if payload.is_null() => format!("OverlaySelectorShape::{unit}"),
        unit => {
            return Err(unmappable(
                field,
                format!("`{unit}` carries no data — author it as a bare member string"),
            ));
        }
    };

    let classes = expect_array(field, get(field, value, "relocates")?, "`relocates`")?;
    let mut elements = Vec::with_capacity(classes.len());
    for class in classes {
        ctx.import("crate::provider::overlay::OverlayResourceClass");
        let variant = variant_of(
            field,
            class,
            "a relocated resource class",
            claudine_catalog_types::OverlayResourceClass::VARIANTS,
        )?;
        elements.push(format!("OverlayResourceClass::{variant}"));
    }
    let relocates = render_slice(&elements, level + 1);

    let additive = expect_bool(field, get(field, value, "additive")?, "`additive`")?;

    let source_root = match get(field, value, "source_root")? {
        Value::Null => "None".to_string(),
        raw => {
            let path = expect_str(field, raw, "`source_root`")?;
            if !path.starts_with("~/") {
                return Err(unmappable(
                    field,
                    format!("source_root `{path}` must be home-relative (`~/...`)"),
                ));
            }
            ctx.import("crate::provider::path_template::PathTemplate");
            format!("Some(PathTemplate::Static({path:?}))")
        }
    };

    let inner = indent(level + 1);
    Ok(format!(
        "Some(&OverlaySelectorSpec {{\n\
         {inner}env_var: {env_var:?},\n\
         {inner}shape: {shape},\n\
         {inner}relocates: {relocates},\n\
         {inner}additive: {additive},\n\
         {inner}source_root: {source_root},\n\
         {}}})",
        indent(level)
    ))
}

/// `OverlayCapabilities { ... }` — one verdict per `OverlayReason` member.
pub(crate) fn overlay_capabilities(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::overlay::OverlayCapabilities");
    ctx.import("crate::provider::overlay::OverlayCapability");
    let inner = indent(level + 1);
    let mut out = String::from("OverlayCapabilities {\n");
    for reason in claudine_catalog_types::OverlayReason::VARIANTS {
        let verdict = variant_of(
            field,
            get(field, value, reason)?,
            "a capability verdict",
            claudine_catalog_types::OverlayCapability::VARIANTS,
        )?;
        out.push_str(&format!("{inner}{reason}: OverlayCapability::{verdict},\n"));
    }
    out.push_str(&format!("{}}}", indent(level)));
    Ok(out)
}

pub(crate) fn emission_fragment(
    values: &ResolvedValues<'_>,
    memory_const: &str,
    ctx: &mut EmitCtx,
) -> Result<EmissionFragment, GenError> {
    let mut fragment = EmissionFragment::new();
    fragment.field(0, "provider", provider_expr(ctx));
    fragment.field(1, "display_name", string_literal("display_name", values.get("display_name")?)?);
    fragment.field(2, "slug", string_literal("slug", values.get("slug")?)?);
    fragment.field(3, "short_name", string_literal("short_name", values.get("short_name")?)?);
    fragment.field(4, "binary", string_literal("binary", values.get("binary")?)?);
    fragment.field(5, "agent_offset", string_literal("agent_offset", values.get("agent_offset")?)?);
    fragment.field(6, "cli_aliases", str_slice("cli_aliases", values.get("cli_aliases")?, 1)?);
    fragment.field(7, "docs_url", string_literal("docs_url", values.get("docs_url")?)?);
    fragment.field(
        8,
        "usage_dashboard_url",
        optional_string_literal("usage_dashboard_url", values.get("usage_dashboard_url")?)?,
    );
    fragment.field(9, "sniff_binding", sniff_binding("sniff_binding", values.get("sniff_binding")?, ctx)?);
    fragment.field(10, "supports_skills", bool_literal("supports_skills", values.get("supports_skills")?)?);
    fragment.field(
        18,
        "session_log_paths",
        path_list_from_strings("session_log_paths", values.get("session_log_paths")?, 1, ctx)?,
    );
    fragment.field(
        19,
        "config_paths",
        path_list_from_strings("config_paths", values.get("config_paths")?, 1, ctx)?,
    );
    fragment.field(20, "memory_files", memory_const.to_string());
    let memory_files = path_list_from_records("memory_files", values.get("memory_files")?, 0, ctx)?;
    fragment.supporting_item(format!(
        "/// Memory / instruction files contributing to the system prompt\n\
         /// hierarchy — one list, shared by `memory_files` and\n\
         /// `system_prompt.memory_files`.\n\
         const {memory_const}: &[PathTemplate] = {memory_files};\n"
    ));
    fragment.field(
        47,
        "overlay_selector",
        overlay_selector("overlay_selector", values.get("overlay_selector")?, 1, ctx)?,
    );
    fragment.field(
        48,
        "overlay_capabilities",
        overlay_capabilities("overlay_capabilities", values.get("overlay_capabilities")?, 1, ctx)?,
    );
    Ok(fragment)
}
