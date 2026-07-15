//! Execution- and prompting-shaped emitters: stream protocol, output
//! formats/entrypoints, the system-prompt delivery spec, YOLO/reasoning
//! flags, prompt-arg conventions, and the CLI-sensitive policy axes.

use super::*;

pub(crate) fn stream_protocol(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    match value {
        Value::Null => Ok("None".to_string()),
        Value::String(_) => Ok(format!("Some({})", stream_protocol_variant(field, value, ctx)?)),
        other => Err(unmappable(
            field,
            format!("expected a stream-protocol string or null, got `{other}`"),
        )),
    }
}

/// Kebab-case wire form → `StreamProtocol::<Variant>` path expression.
pub(crate) fn stream_protocol_variant(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let member = expect_str(field, value, "a stream protocol")?;
    let variant = match member {
        "stream-json" => "StreamJson",
        "ndjson" => "Ndjson",
        "jsonl" => "Jsonl",
        "wire-json-rpc" => "WireJsonRpc",
        "json" => "Json",
        other => {
            return Err(unmappable(
                field,
                format!("`{other}` is not a StreamProtocol wire form"),
            ));
        }
    };
    ctx.import("crate::stream::StreamProtocol");
    Ok(format!("StreamProtocol::{variant}"))
}

pub(crate) fn output_formats(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the output-format list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::output_format::OutputFormatSupport");
        ctx.import("crate::provider::output_format::OutputFormat");
        let format = pascal(expect_str(field, get(field, record, "format")?, "`format`")?);
        let native = expect_str(field, get(field, record, "native_name")?, "`native_name`")?;
        let cli_flag = optional_string_literal(field, get(field, record, "cli_flag")?)?;
        let stdin = expect_bool(field, get(field, record, "stdin_supported")?, "`stdin_supported`")?;
        let selector = output_format_selector(field, get(field, record, "selector")?, level + 2, ctx)?;
        let companions = str_slice(field, get(field, record, "companion_flags")?, level + 2)?;
        let inner = indent(level + 2);
        elements.push(format!(
            "OutputFormatSupport {{\n\
             {inner}format: OutputFormat::{format},\n\
             {inner}native_name: {native:?},\n\
             {inner}cli_flag: {cli_flag},\n\
             {inner}stdin_supported: {stdin},\n\
             {inner}selector: {selector},\n\
             {inner}companion_flags: {companions},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

fn output_format_selector(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::OutputFormatSelector");
    let kind = expect_str(field, get(field, value, "kind")?, "the selector `kind`")?;
    let inner = indent(level + 1);
    let close = indent(level);
    let flag_body = |variant: &str, key: &str| -> Result<String, GenError> {
        let site = expect_str(field, get(field, value, key)?, key)?;
        Ok(format!(
            "OutputFormatSelector::{variant} {{\n{inner}{key}: {site:?},\n{close}}}"
        ))
    };
    match kind {
        "default" => Ok("OutputFormatSelector::Default".to_string()),
        "flag" => flag_body("Flag", "flag"),
        "flag_value" => flag_body("FlagValue", "flag"),
        "positional" => flag_body("Positional", "token"),
        "transport_flag" => flag_body("TransportFlag", "flag"),
        other => Err(unmappable(
            field,
            format!("`{other}` is not an OutputFormatSelector kind"),
        )),
    }
}

pub(crate) fn entrypoints(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the entrypoint list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::output_format::EntrypointSpec");
        ctx.import("crate::provider::output_format::EntrypointMode");
        let subcommand = optional_string_literal(field, get(field, record, "subcommand")?)?;
        let flags = str_slice(field, get(field, record, "required_flags")?, level + 2)?;
        let mode = pascal(expect_str(field, get(field, record, "mode")?, "`mode`")?);
        let inner = indent(level + 2);
        elements.push(format!(
            "EntrypointSpec {{\n\
             {inner}subcommand: {subcommand},\n\
             {inner}required_flags: {flags},\n\
             {inner}mode: EntrypointMode::{mode},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

/// `&SystemPromptSpec { ... }` referencing `memory_files_expr` (either the
/// shared `<PREFIX>_MEMORY_FILES` const or an inline list).
pub(crate) fn system_prompt_spec(
    field: &'static str,
    value: &Value,
    memory_files_expr: &str,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::system_prompt::SystemPromptSpec");
    let append = delivery_by_mode(field, get(field, value, "append")?, level + 1, ctx)?;
    let replace = delivery_by_mode(field, get(field, value, "replace")?, level + 1, ctx)?;
    let inner = indent(level + 1);
    Ok(format!(
        "&SystemPromptSpec {{\n\
         {inner}append: {append},\n\
         {inner}replace: {replace},\n\
         {inner}memory_files: {memory_files_expr},\n\
         {}}}",
        indent(level)
    ))
}

fn delivery_by_mode(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::system_prompt::SystemPromptDeliveryByMode");
    let interactive = delivery(field, get(field, value, "interactive")?, level + 1, ctx)?;
    let non_interactive = delivery(field, get(field, value, "non_interactive")?, level + 1, ctx)?;
    let inner = indent(level + 1);
    Ok(format!(
        "SystemPromptDeliveryByMode {{\n\
         {inner}interactive: {interactive},\n\
         {inner}non_interactive: {non_interactive},\n\
         {}}}",
        indent(level)
    ))
}

fn delivery(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::system_prompt::SystemPromptDelivery");
    let (member, payload) = enum_shape(field, value)?;
    let inner = indent(level + 1);
    let close = indent(level);
    let one = |variant: &str, key: &str, payload: &Value| -> Result<String, GenError> {
        let site = expect_str(field, get(field, payload, key)?, key)?;
        Ok(format!(
            "SystemPromptDelivery::{variant} {{\n{inner}{key}: {site:?},\n{close}}}"
        ))
    };
    let two = |variant: &str, payload: &Value| -> Result<String, GenError> {
        let flag = expect_str(field, get(field, payload, "flag")?, "`flag`")?;
        let key = expect_str(field, get(field, payload, "key")?, "`key`")?;
        Ok(format!(
            "SystemPromptDelivery::{variant} {{\n\
             {inner}flag: {flag:?},\n\
             {inner}key: {key:?},\n\
             {close}}}"
        ))
    };
    match member.as_str() {
        "unsupported" => Ok("SystemPromptDelivery::Unsupported".to_string()),
        "inline_flag" => one("InlineFlag", "flag", &payload),
        "file_flag" => one("FileFlag", "flag", &payload),
        "env_var_file" => one("EnvVarFile", "env_var", &payload),
        "shadow_home_file" => one("ShadowHomeFile", "relative_path", &payload),
        "config_key_inline" => two("ConfigKeyInline", &payload),
        "config_key_file" => two("ConfigKeyFile", &payload),
        "custom" => {
            ctx.import("crate::provider::system_prompt::SystemPromptCustomTag");
            let tag = pascal(expect_str(field, &payload, "the custom delivery tag")?);
            Ok(format!(
                "SystemPromptDelivery::Custom(SystemPromptCustomTag::{tag})"
            ))
        }
        other => Err(unmappable(
            field,
            format!("`{other}` is not a SystemPromptDelivery wire form"),
        )),
    }
}

pub(crate) fn yolo(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::yolo::YoloSupport");
    let (member, payload) = enum_shape(field, value)?;
    let inner = indent(level + 1);
    let close = indent(level);
    match member.as_str() {
        "none" => Ok("YoloSupport::None".to_string()),
        "direct_flag" => {
            let flag = expect_str(field, get(field, &payload, "native_flag")?, "`native_flag`")?;
            Ok(format!(
                "YoloSupport::DirectFlag {{\n{inner}native_flag: {flag:?},\n{close}}}"
            ))
        }
        "direct_flag_with_alias" => {
            let flag = expect_str(field, get(field, &payload, "native_flag")?, "`native_flag`")?;
            let aliases = str_slice(field, get(field, &payload, "aliases")?, level + 1)?;
            Ok(format!(
                "YoloSupport::DirectFlagWithAlias {{\n\
                 {inner}native_flag: {flag:?},\n\
                 {inner}aliases: {aliases},\n\
                 {close}}}"
            ))
        }
        "non_interactive_only" => {
            let flag = expect_str(
                field,
                get(field, &payload, "non_interactive_flag")?,
                "`non_interactive_flag`",
            )?;
            Ok(format!(
                "YoloSupport::NonInteractiveOnly {{\n{inner}non_interactive_flag: {flag:?},\n{close}}}"
            ))
        }
        "env_var" => {
            let env_var = expect_str(field, get(field, &payload, "env_var")?, "`env_var`")?;
            let value = expect_str(field, get(field, &payload, "value")?, "`value`")?;
            Ok(format!(
                "YoloSupport::EnvVar {{\n\
                 {inner}env_var: {env_var:?},\n\
                 {inner}value: {value:?},\n\
                 {close}}}"
            ))
        }
        other => Err(unmappable(
            field,
            format!("`{other}` is not a YoloSupport wire form"),
        )),
    }
}

pub(crate) fn reasoning(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::reasoning::ReasoningSupport");
    let (member, payload) = enum_shape(field, value)?;
    let inner = indent(level + 1);
    let close = indent(level);
    match member.as_str() {
        "not_supported" => Ok("ReasoningSupport::NotSupported".to_string()),
        "not_documented" => Ok("ReasoningSupport::NotDocumented".to_string()),
        "named_levels" => {
            let flag = expect_str(field, get(field, &payload, "flag")?, "`flag`")?;
            let levels = str_slice(field, get(field, &payload, "levels")?, level + 1)?;
            Ok(format!(
                "ReasoningSupport::NamedLevels {{\n\
                 {inner}flag: {flag:?},\n\
                 {inner}levels: {levels},\n\
                 {close}}}"
            ))
        }
        "numeric_budget" => {
            let flag = expect_str(field, get(field, &payload, "flag")?, "`flag`")?;
            let min = number_u32(field, get(field, &payload, "min")?)?;
            let max = number_u32(field, get(field, &payload, "max")?)?;
            let default = match get(field, &payload, "default")? {
                Value::Null => "None".to_string(),
                number => format!("Some({})", number_u32(field, number)?),
            };
            Ok(format!(
                "ReasoningSupport::NumericBudget {{\n\
                 {inner}flag: {flag:?},\n\
                 {inner}min: {min},\n\
                 {inner}max: {max},\n\
                 {inner}default: {default},\n\
                 {close}}}"
            ))
        }
        "binary_toggle" => {
            let flag = expect_str(field, get(field, &payload, "flag")?, "`flag`")?;
            let on = expect_str(field, get(field, &payload, "on")?, "`on`")?;
            let off = expect_str(field, get(field, &payload, "off")?, "`off`")?;
            Ok(format!(
                "ReasoningSupport::BinaryToggle {{\n\
                 {inner}flag: {flag:?},\n\
                 {inner}on: {on:?},\n\
                 {inner}off: {off:?},\n\
                 {close}}}"
            ))
        }
        "provider_specific" => {
            ctx.import("crate::provider::reasoning::ReasoningCustomTag");
            let tag = pascal(expect_str(field, &payload, "the custom reasoning tag")?);
            Ok(format!(
                "ReasoningSupport::ProviderSpecific(ReasoningCustomTag::{tag})"
            ))
        }
        other => Err(unmappable(
            field,
            format!("`{other}` is not a ReasoningSupport wire form"),
        )),
    }
}

pub(crate) fn prompt_arg_conventions(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::prompt_args::PromptArgConventions");
    let prompt_flags = str_slice(field, get(field, value, "prompt_flags")?, level + 1)?;
    let entrypoint = optional_string_literal(field, get(field, value, "entrypoint")?)?;
    let inner = indent(level + 1);
    Ok(format!(
        "PromptArgConventions {{\n\
         {inner}prompt_flags: {prompt_flags},\n\
         {inner}entrypoint: {entrypoint},\n\
         {}}}",
        indent(level)
    ))
}

/// The ten policy axes in `CliSensitiveAxes` declaration order.
const AXES: &[&str] = &[
    "read_path",
    "write_path",
    "traverse_path",
    "execute_command",
    "access_domain",
    "use_mcp_server",
    "use_mcp_tool",
    "spawn_subagent",
    "switch_mode",
    "modify_provider_config",
];

pub(crate) fn cli_sensitive_axes(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::cli_sensitivity::CliSensitiveAxes");
    let inner = indent(level + 1);
    let mut out = String::from("CliSensitiveAxes {\n");
    for axis in AXES {
        let flag = expect_bool(field, get(field, value, axis)?, axis)?;
        out.push_str(&format!("{inner}{axis}: {flag},\n"));
    }
    out.push_str(&format!("{}}}", indent(level)));
    Ok(out)
}
