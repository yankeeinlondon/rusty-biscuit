//! Identity and path emitters: the `Provider` expression, the sniff
//! install-detection binding, and the `PathTemplate` list forms (from
//! research string arrays and from the facts `{raw, segments}` records).

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
