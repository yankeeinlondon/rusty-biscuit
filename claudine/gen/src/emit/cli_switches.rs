//! Expression half of the `cli_switches` field: a catalog-shaped switch
//! catalog → a `CliSwitchCatalog` literal.
//!
//! Overrides reach this emitter without passing through the coercion, so it
//! rejects any value that is not the catalog shape rather than guessing.

use serde_json::Value;

use super::{
    EmissionFragment, EmitCtx, ResolvedValues, enum_shape, expect_array, expect_bool, expect_str,
    get, indent, number_u32, pascal, render_slice, render_struct_slice, str_slice, unmappable,
};
use crate::errors::GenError;

const FIELD: &str = "cli_switches";
const MODULE: &str = "crate::provider::cli_switch";

pub(super) fn emission_fragment(
    values: &ResolvedValues<'_>,
    ctx: &mut EmitCtx,
) -> Result<EmissionFragment, GenError> {
    let mut fragment = EmissionFragment::new();
    fragment.field(49, FIELD, cli_switch_catalog(values.get(FIELD)?, 1, ctx)?);
    Ok(fragment)
}

fn import(ctx: &mut EmitCtx, name: &str) {
    ctx.import(&format!("{MODULE}::{name}"));
}

fn cli_switch_catalog(value: &Value, level: usize, ctx: &mut EmitCtx) -> Result<String, GenError> {
    import(ctx, "CliSwitchCatalog");
    let (member, payload) = enum_shape(FIELD, value)?;
    match member.as_str() {
        "unknown" => {
            let gap = expect_str(FIELD, get(FIELD, &payload, "gap")?, "`gap`")?;
            Ok(format!(
                "CliSwitchCatalog::Unknown {{\n{}gap: {gap:?},\n{}}}",
                indent(level + 1),
                indent(level)
            ))
        }
        "researched" => {
            let records = expect_array(FIELD, &payload, "the switch list")?;
            let mut elements = Vec::with_capacity(records.len());
            for record in records {
                elements.push(cli_switch(record, level + 1, ctx)?);
            }
            Ok(format!(
                "CliSwitchCatalog::Researched({})",
                render_struct_slice(&elements, level)
            ))
        }
        other => Err(unmappable(FIELD, format!("`{other}` is not a CliSwitchCatalog variant"))),
    }
}

fn cli_switch(record: &Value, level: usize, ctx: &mut EmitCtx) -> Result<String, GenError> {
    import(ctx, "CliSwitch");
    let inner = indent(level + 1);
    let flag = expect_str(FIELD, get(FIELD, record, "flag")?, "`flag`")?;
    let aliases = str_slice(FIELD, get(FIELD, record, "aliases")?, level + 1)?;
    let value = switch_value(get(FIELD, record, "value")?, ctx)?;
    let attachments = attachments(get(FIELD, record, "attachments")?, level + 1, ctx)?;
    let scopes = scopes(get(FIELD, record, "scopes")?, level + 1, ctx)?;
    let description = expect_str(FIELD, get(FIELD, record, "description")?, "`description`")?;
    let gap = match get(FIELD, record, "gap")? {
        Value::Null => "None".to_string(),
        other => format!("Some({:?})", expect_str(FIELD, other, "`gap`")?),
    };
    Ok(format!(
        "CliSwitch {{\n\
         {inner}flag: {flag:?},\n\
         {inner}aliases: {aliases},\n\
         {inner}value: {value},\n\
         {inner}attachments: {attachments},\n\
         {inner}scopes: {scopes},\n\
         {inner}description: {description:?},\n\
         {inner}gap: {gap},\n\
         {}}}",
        indent(level)
    ))
}

fn switch_value(value: &Value, ctx: &mut EmitCtx) -> Result<String, GenError> {
    import(ctx, "SwitchValue");
    let (member, payload) = enum_shape(FIELD, value)?;
    match member.as_str() {
        "none" | "unknown" if payload.is_null() => Ok(format!("SwitchValue::{}", pascal(&member))),
        "string" | "number" => {
            let optional = expect_bool(FIELD, get(FIELD, &payload, "optional")?, "`optional`")?;
            Ok(format!("SwitchValue::{} {{ optional: {optional} }}", pascal(&member)))
        }
        "variadic" => {
            import(ctx, "VariadicMin");
            let min = match get(FIELD, &payload, "min")? {
                Value::String(text) if text == "unknown" => "VariadicMin::Unknown".to_string(),
                number => match number_u32(FIELD, number)? {
                    0 => return Err(unmappable(FIELD, "a variadic minimum is at least 1".into())),
                    min => format!("VariadicMin::AtLeast({min})"),
                },
            };
            Ok(format!("SwitchValue::Variadic {{ min: {min} }}"))
        }
        other => Err(unmappable(FIELD, format!("`{other}` is not a SwitchValue shape"))),
    }
}

fn attachments(value: &Value, level: usize, ctx: &mut EmitCtx) -> Result<String, GenError> {
    let mut elements = Vec::new();
    for item in expect_array(FIELD, value, "the attachment list")? {
        import(ctx, "SwitchAttachment");
        let member = expect_str(FIELD, item, "an attachment form")?;
        if !matches!(member, "space" | "equals" | "short_attached") {
            return Err(unmappable(FIELD, format!("`{member}` is not a SwitchAttachment member")));
        }
        elements.push(format!("SwitchAttachment::{}", pascal(member)));
    }
    Ok(render_slice(&elements, level))
}

fn scopes(value: &Value, level: usize, ctx: &mut EmitCtx) -> Result<String, GenError> {
    let mut elements = Vec::new();
    for item in expect_array(FIELD, value, "the scope list")? {
        import(ctx, "SwitchScope");
        let (member, payload) = enum_shape(FIELD, item)?;
        elements.push(match member.as_str() {
            "global" if payload.is_null() => "SwitchScope::Global".to_string(),
            "command" => format!("SwitchScope::Command({})", str_slice(FIELD, &payload, level)?),
            other => return Err(unmappable(FIELD, format!("`{other}` is not a SwitchScope shape"))),
        });
    }
    Ok(render_slice(&elements, level))
}
