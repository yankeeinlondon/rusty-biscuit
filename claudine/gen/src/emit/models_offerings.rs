//! Model- and offering-shaped emitters: billing models and cap policies,
//! the expected-offering / offering-source records, resume support, and the
//! model-catalog source (unit variants plus the `shell_command` data form).

use super::*;

/// Resume-support member → `ResumeSupport::<Variant>` path expression.
pub(crate) fn resume_support(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::resume_support::ResumeSupport");
    let member = expect_str(field, value, "a resume-support member")?;
    match member {
        known @ ("first_class" | "partial" | "interactive_only" | "non_interactive_only"
        | "none" | "unknown") => Ok(format!("ResumeSupport::{}", pascal(known))),
        other => Err(unmappable(
            field,
            format!("`{other}` is not a ResumeSupport wire form"),
        )),
    }
}

/// `&[BillingModel::<Variant>, ...]` from a snake_case member list.
pub(crate) fn billing_models(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let members = expect_array(field, value, "the billing-model list")?;
    let mut elements = Vec::with_capacity(members.len());
    for member in members {
        ctx.import("crate::provider::billing_model::BillingModel");
        let name = expect_str(field, member, "a billing-model member")?;
        match name {
            known @ ("subscription" | "per_token" | "prepaid_credits" | "provider_only") => {
                elements.push(format!("BillingModel::{}", pascal(known)));
            }
            other => {
                return Err(unmappable(
                    field,
                    format!("`{other}` is not a BillingModel wire form"),
                ));
            }
        }
    }
    Ok(render_slice(&elements, level))
}

/// `&[CapPolicy { ... }, ...]` from the facts `{model, timeframe_secs}`
/// records (Layer A cap-policy catalog). `model: "all"` → `CapScope::All`;
/// any other token → `CapScope::specific(token)`. `timeframe_secs` becomes a
/// `Quantity` in `Unit::DurationSecs`.
pub(crate) fn cap_policies(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the cap-policy list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::cap_policy::CapPolicy");
        ctx.import("crate::provider::cap_policy::CapScope");
        ctx.import("crate::provider::cap_policy::Quantity");
        ctx.import("crate::provider::cap_policy::Unit");
        let model = expect_str(field, get(field, record, "model")?, "`model`")?;
        let scope = if model == "all" {
            "CapScope::All".to_string()
        } else {
            format!("CapScope::specific({model:?})")
        };
        let secs = number_u32(field, get(field, record, "timeframe_secs")?)?;
        let inner = indent(level + 2);
        elements.push(format!(
            "CapPolicy {{\n\
             {inner}model: {scope},\n\
             {inner}timeframe: Quantity {{ value: {secs}.0, unit: Unit::DurationSecs }},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

/// Offering-class member → the `OfferingClass` variant name.
fn offering_class_variant(field: &'static str, value: &Value) -> Result<String, GenError> {
    let member = expect_str(field, value, "an offering-class member")?;
    match member {
        known @ ("vendor_api" | "plan_endpoint" | "aggregator" | "local_runner") => {
            Ok(pascal(known))
        }
        other => Err(unmappable(
            field,
            format!("`{other}` is not an OfferingClass wire form"),
        )),
    }
}

/// `&[ExpectedOffering { ... }, ...]` from the catalog-shaped
/// expected-offering records.
pub(crate) fn expected_offerings(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the expected-offering list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::offering::ExpectedOffering");
        ctx.import("crate::provider::offering::OfferingClass");
        let id = expect_str(field, get(field, record, "id")?, "`id`")?;
        let alias = optional_string_literal(field, get(field, record, "alias")?)?;
        let is_default = expect_bool(field, get(field, record, "is_default")?, "`is_default`")?;
        let context_window = match get(field, record, "context_window")? {
            Value::Null => "None".to_string(),
            number => format!("Some({})", number_u32(field, number)?),
        };
        let class = offering_class_variant(field, get(field, record, "class")?)?;
        let catalog_id = optional_string_literal(field, get(field, record, "catalog_id")?)?;
        let resolves = match get(field, record, "resolves")? {
            Value::Null => "None".to_string(),
            member => {
                ctx.import("crate::provider::offering::ResolvesVia");
                match expect_str(field, member, "`resolves`")? {
                    "family_latest" => "Some(ResolvesVia::FamilyLatest)".to_string(),
                    other => {
                        return Err(unmappable(
                            field,
                            format!("`{other}` is not a ResolvesVia wire form"),
                        ));
                    }
                }
            }
        };
        let inner = indent(level + 2);
        elements.push(format!(
            "ExpectedOffering {{\n\
             {inner}id: {id:?},\n\
             {inner}alias: {alias},\n\
             {inner}is_default: {is_default},\n\
             {inner}context_window: {context_window},\n\
             {inner}class: OfferingClass::{class},\n\
             {inner}catalog_id: {catalog_id},\n\
             {inner}resolves: {resolves},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

/// `&[OfferingSource { ... }, ...]` from the catalog-shaped
/// offering-source records.
pub(crate) fn offering_sources(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the offering-source list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::offering::OfferingSource");
        ctx.import("crate::provider::offering::OfferingClass");
        let prefix = expect_str(field, get(field, record, "prefix")?, "`prefix`")?;
        let class = offering_class_variant(field, get(field, record, "class")?)?;
        let api_standard = optional_string_literal(field, get(field, record, "api_standard")?)?;
        let integration = match get(field, record, "integration")? {
            Value::Null => "None".to_string(),
            member => {
                ctx.import("crate::provider::offering::LocalRunnerIntegration");
                let member = expect_str(field, member, "`integration`")?;
                match member {
                    known @ ("first_class" | "base_url_override" | "proxy_required"
                    | "unsupported") => {
                        format!("Some(LocalRunnerIntegration::{})", pascal(known))
                    }
                    other => {
                        return Err(unmappable(
                            field,
                            format!("`{other}` is not a LocalRunnerIntegration wire form"),
                        ));
                    }
                }
            }
        };
        let inner = indent(level + 2);
        elements.push(format!(
            "OfferingSource {{\n\
             {inner}prefix: {prefix:?},\n\
             {inner}class: OfferingClass::{class},\n\
             {inner}api_standard: {api_standard},\n\
             {inner}integration: {integration},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

/// Catalog shape: a bare member string for the unit variants, or the
/// externally tagged `{"shell_command": {"program": ..., "args": [...]}}`
/// object (mirroring the enum's serde wire form) for the data variant.
pub(crate) fn model_catalog_source(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::model_catalog_source::ModelCatalogSource");
    if let Some(payload) = value.get("shell_command") {
        let program = payload
            .get("program")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                unmappable(field, "shell_command requires a string `program`".to_string())
            })?;
        let args = payload
            .get("args")
            .ok_or_else(|| {
                unmappable(field, "shell_command requires an `args` string array".to_string())
            })?;
        let args = str_slice(field, args, 2)?;
        let outer = indent(1);
        let inner = indent(2);
        return Ok(format!(
            "ModelCatalogSource::ShellCommand {{\n\
             {inner}program: {program:?},\n\
             {inner}args: {args},\n\
             {outer}}}"
        ));
    }
    let member = expect_str(field, value, "an enum member")?;
    crate::generate::model_catalog_source_variant(field, member)
        .map(|variant| format!("ModelCatalogSource::{variant}"))
}
