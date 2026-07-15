//! Rust-source emission: catalog-shaped JSON values → `data.rs` text.
//!
//! Every emitter here is the *expression half* of a [`Coercion`]: it turns
//! a catalog-shaped `serde_json::Value` (the shape overrides are authored
//! in) into deterministic Rust source text. The file assembler
//! [`emit_data_file`] joins the per-field expressions into a complete
//! `lib/src/provider/<slug>/data.rs`, including the behavior-wiring lines
//! and the `LazyLock`-backed resource-support builder.
//!
//! Formatting is hand-rolled and deterministic; the drift test compares
//! generator output against the committed file, so internal consistency is
//! the only formatting contract (the workspace lint gate is clippy, not
//! rustfmt).

use std::collections::BTreeSet;

use claudine_catalog_types::{EventClass, ToolResultSummary};
use serde_json::Value;
use strum::VariantNames;

use crate::errors::GenError;

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

fn unmappable(field: &'static str, message: String) -> GenError {
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

fn expect_str<'v>(field: &'static str, value: &'v Value, what: &str) -> Result<&'v str, GenError> {
    value
        .as_str()
        .ok_or_else(|| unmappable(field, format!("expected a string for {what}, got `{value}`")))
}

fn expect_bool(field: &'static str, value: &Value, what: &str) -> Result<bool, GenError> {
    value
        .as_bool()
        .ok_or_else(|| unmappable(field, format!("expected a boolean for {what}, got `{value}`")))
}

fn expect_array<'v>(
    field: &'static str,
    value: &'v Value,
    what: &str,
) -> Result<&'v Vec<Value>, GenError> {
    value
        .as_array()
        .ok_or_else(|| unmappable(field, format!("expected an array for {what}, got `{value}`")))
}

fn get<'v>(field: &'static str, value: &'v Value, key: &str) -> Result<&'v Value, GenError> {
    value
        .get(key)
        .ok_or_else(|| unmappable(field, format!("missing key `{key}` in `{value}`")))
}

/// Externally-tagged enum helper: `"member"` → `(member, Null)`,
/// `{"member": payload}` → `(member, payload)`.
fn enum_shape(field: &'static str, value: &Value) -> Result<(String, Value), GenError> {
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

// ---------------------------------------------------------------------------
// Scalar / list expressions
// ---------------------------------------------------------------------------

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
fn render_struct_slice(elements: &[String], level: usize) -> String {
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

pub fn provider_expr(ctx: &mut EmitCtx) -> String {
    ctx.import("crate::provider::identity::Provider");
    format!("Provider::{}", ctx.variant)
}

pub fn sniff_binding(
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

pub fn stream_protocol(
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
fn stream_protocol_variant(
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

/// Resume-support member → `ResumeSupport::<Variant>` path expression.
pub fn resume_support(
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
pub fn billing_models(
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
pub fn cap_policies(
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
pub fn expected_offerings(
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
pub fn offering_sources(
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

/// Platform-kind member → `PlatformKind::<Variant>` path expression.
pub fn platform_kind(
    field: &'static str,
    value: &Value,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::platform_kind::PlatformKind");
    let member = expect_str(field, value, "a platform-kind member")?;
    match member {
        known @ ("vendor_platform" | "agent_aggregator") => {
            Ok(format!("PlatformKind::{}", pascal(known)))
        }
        other => Err(unmappable(
            field,
            format!("`{other}` is not a PlatformKind wire form"),
        )),
    }
}

/// `&[PathTemplate::Static("..."), ...]` from a catalog-shaped string
/// array (research-fed path fields).
pub fn path_list_from_strings(
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
pub fn path_list_from_records(
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

// ---------------------------------------------------------------------------
// Event mapping
// ---------------------------------------------------------------------------

/// The body of the `EventMappingTable` static:
/// `EventMappingTable { mappings: &[ ... ] }`.
pub fn event_mapping_table(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::event_mapping::EventMappingTable");
    ctx.import("crate::provider::event_mapping::EventMapping");
    let mappings = expect_array(field, get(field, value, "mappings")?, "`mappings`")?;
    let mut rows = Vec::with_capacity(mappings.len());
    for mapping in mappings {
        rows.push(event_mapping_row(field, mapping, level + 2, ctx)?);
    }
    let inner = indent(level + 1);
    Ok(format!(
        "EventMappingTable {{\n{inner}mappings: {},\n{}}}",
        render_struct_slice(&rows, level + 1),
        indent(level)
    ))
}

fn event_mapping_row(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::events::AgenticEvent");
    let event = pascal(expect_str(field, get(field, value, "event")?, "`event`")?);
    let support = support_level(field, get(field, value, "support_level")?, level + 1, ctx)?;
    let aliases = str_slice(field, get(field, value, "parse_aliases")?, level + 1)?;
    let registration = expect_bool(field, get(field, value, "registration_target")?, "`registration_target`")?;
    let inner = indent(level + 1);
    Ok(format!(
        "EventMapping {{\n\
         {inner}event: AgenticEvent::{event},\n\
         {inner}support_level: {support},\n\
         {inner}parse_aliases: {aliases},\n\
         {inner}registration_target: {registration},\n\
         {}}}",
        indent(level)
    ))
}

fn support_level(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::event_mapping::EventSupportLevel");
    let (member, payload) = enum_shape(field, value)?;
    let inner = indent(level + 1);
    let close = indent(level);
    match member.as_str() {
        "not_supported" => Ok("EventSupportLevel::NotSupported".to_string()),
        "hook" => {
            let name = expect_str(field, get(field, &payload, "native_name")?, "`native_name`")?;
            Ok(format!(
                "EventSupportLevel::Hook {{\n{inner}native_name: {name:?},\n{close}}}"
            ))
        }
        "stream_parse" => {
            let protocol =
                stream_protocol_variant(field, get(field, &payload, "protocol")?, ctx)?;
            let name = expect_str(field, get(field, &payload, "native_name")?, "`native_name`")?;
            Ok(format!(
                "EventSupportLevel::StreamParse {{\n\
                 {inner}protocol: {protocol},\n\
                 {inner}native_name: {name:?},\n\
                 {close}}}"
            ))
        }
        "wire_proxy" => {
            ctx.import("crate::provider::event_mapping::WireProxyMode");
            let mode = pascal(expect_str(field, get(field, &payload, "mode")?, "`mode`")?);
            let name = expect_str(field, get(field, &payload, "native_name")?, "`native_name`")?;
            Ok(format!(
                "EventSupportLevel::WireProxy {{\n\
                 {inner}mode: WireProxyMode::{mode},\n\
                 {inner}native_name: {name:?},\n\
                 {close}}}"
            ))
        }
        "acp" => {
            let event = acp_event(field, get(field, &payload, "event")?, ctx)?;
            let name = expect_str(field, get(field, &payload, "native_name")?, "`native_name`")?;
            Ok(format!(
                "EventSupportLevel::Acp {{\n\
                 {inner}event: {event},\n\
                 {inner}native_name: {name:?},\n\
                 {close}}}"
            ))
        }
        "wrapper" => {
            let name = expect_str(field, get(field, &payload, "native_name")?, "`native_name`")?;
            Ok(format!(
                "EventSupportLevel::Wrapper {{\n{inner}native_name: {name:?},\n{close}}}"
            ))
        }
        other => Err(unmappable(
            field,
            format!("`{other}` is not an EventSupportLevel wire form"),
        )),
    }
}

fn acp_event(field: &'static str, value: &Value, ctx: &mut EmitCtx) -> Result<String, GenError> {
    ctx.import("crate::provider::acp::AcpEvent");
    let (member, payload) = enum_shape(field, value)?;
    match member.as_str() {
        "custom" => {
            let tag = expect_str(field, &payload, "the custom ACP event tag")?;
            Ok(format!("AcpEvent::Custom({tag:?})"))
        }
        known @ ("request_permission" | "approval_request" | "tool_call" | "tool_result"
        | "session_update") => Ok(format!("AcpEvent::{}", pascal(known))),
        other => Err(unmappable(
            field,
            format!("`{other}` is not an AcpEvent wire form"),
        )),
    }
}

// ---------------------------------------------------------------------------
// Output formats / entrypoints
// ---------------------------------------------------------------------------

pub fn output_formats(
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

pub fn entrypoints(
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

// ---------------------------------------------------------------------------
// System prompt / yolo / reasoning / gaps / acp / prompt args / axes
// ---------------------------------------------------------------------------

/// `&SystemPromptSpec { ... }` referencing `memory_files_expr` (either the
/// shared `<PREFIX>_MEMORY_FILES` const or an inline list).
pub fn system_prompt_spec(
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

pub fn yolo(
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

pub fn reasoning(
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

fn number_u32(field: &'static str, value: &Value) -> Result<u32, GenError> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| unmappable(field, format!("expected a u32 number, got `{value}`")))
}

pub fn known_gaps(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the known-gap list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::known_gap::KnownGap");
        ctx.import("crate::provider::known_gap::KnownGapArea");
        let area = pascal(expect_str(field, get(field, record, "area")?, "`area`")?);
        let note = expect_str(field, get(field, record, "note")?, "`note`")?;
        let tracker = optional_string_literal(field, get(field, record, "tracker")?)?;
        let inner = indent(level + 2);
        elements.push(format!(
            "KnownGap {{\n\
             {inner}area: KnownGapArea::{area},\n\
             {inner}note: {note:?},\n\
             {inner}tracker: {tracker},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

pub fn unmapped_native_events(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    let records = expect_array(field, value, "the unmapped-native-event list")?;
    let mut elements = Vec::with_capacity(records.len());
    for record in records {
        ctx.import("crate::provider::unmapped_native_event::UnmappedNativeEvent");
        let native_event =
            expect_str(field, get(field, record, "native_event")?, "`native_event`")?;
        let description = expect_str(field, get(field, record, "description")?, "`description`")?;
        let remediation = expect_str(field, get(field, record, "remediation")?, "`remediation`")?;
        let inner = indent(level + 2);
        elements.push(format!(
            "UnmappedNativeEvent {{\n\
             {inner}native_event: {native_event:?},\n\
             {inner}description: {description:?},\n\
             {inner}remediation: {remediation:?},\n\
             {}}}",
            indent(level + 1)
        ));
    }
    Ok(render_struct_slice(&elements, level))
}

pub fn acp(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::acp::AcpSupport");
    ctx.import("crate::provider::acp::AcpServerMode");
    let mode = pascal(expect_str(field, get(field, value, "server_mode")?, "`server_mode`")?);
    let client = expect_bool(field, get(field, value, "client_supported")?, "`client_supported`")?;
    let events = expect_array(field, get(field, value, "events_via_acp")?, "`events_via_acp`")?;
    let mut elements = Vec::with_capacity(events.len());
    for event in events {
        elements.push(acp_event(field, event, ctx)?);
    }
    let inner = indent(level + 1);
    Ok(format!(
        "AcpSupport {{\n\
         {inner}server_mode: AcpServerMode::{mode},\n\
         {inner}client_supported: {client},\n\
         {inner}events_via_acp: {},\n\
         {}}}",
        render_slice(&elements, level + 1),
        indent(level)
    ))
}

pub fn prompt_arg_conventions(
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

/// `DisplayPolicy { ... }` literal from the facts display-policy record
/// (fixed field list, declaration order). Enum sub-keys are validated
/// against the catalog-types variant names — overrides flow through this
/// expression half only, so the check must live here too.
pub fn display_policy(
    field: &'static str,
    value: &Value,
    level: usize,
    ctx: &mut EmitCtx,
) -> Result<String, GenError> {
    ctx.import("crate::provider::display_policy::DisplayPolicy");
    ctx.import("crate::provider::display_policy::ToolResultSummary");
    let summary = expect_str(
        field,
        get(field, value, "tool_result_summary")?,
        "`tool_result_summary`",
    )?;
    if !ToolResultSummary::VARIANTS.contains(&summary) {
        return Err(unmappable(
            field,
            format!("`{summary}` is not a ToolResultSummary wire form"),
        ));
    }
    let suppression = expect_array(
        field,
        get(field, value, "info_event_suppression")?,
        "`info_event_suppression`",
    )?;
    let mut classes = Vec::with_capacity(suppression.len());
    for class in suppression {
        let member = expect_str(field, class, "an event-class member")?;
        if !EventClass::VARIANTS.contains(&member) {
            return Err(unmappable(
                field,
                format!("`{member}` is not an EventClass wire form"),
            ));
        }
        ctx.import("crate::provider::display_policy::EventClass");
        classes.push(format!("EventClass::{}", pascal(member)));
    }
    let collapse = expect_bool(
        field,
        get(field, value, "collapse_task_progress")?,
        "`collapse_task_progress`",
    )?;
    let rate_limit = expect_bool(
        field,
        get(field, value, "suppress_subscription_rate_limit")?,
        "`suppress_subscription_rate_limit`",
    )?;
    let silent_kinds = str_slice(field, get(field, value, "silent_extension_kinds")?, level + 1)?;
    let stdout_noise = str_slice(field, get(field, value, "stdout_noise_prefixes")?, level + 1)?;
    let stderr_noise = str_slice(field, get(field, value, "stderr_noise_prefixes")?, level + 1)?;
    let inner = indent(level + 1);
    Ok(format!(
        "DisplayPolicy {{\n\
         {inner}tool_result_summary: ToolResultSummary::{},\n\
         {inner}info_event_suppression: {},\n\
         {inner}collapse_task_progress: {collapse},\n\
         {inner}suppress_subscription_rate_limit: {rate_limit},\n\
         {inner}silent_extension_kinds: {silent_kinds},\n\
         {inner}stdout_noise_prefixes: {stdout_noise},\n\
         {inner}stderr_noise_prefixes: {stderr_noise},\n\
         {}}}",
        pascal(summary),
        render_slice(&classes, level + 1),
        indent(level)
    ))
}

pub fn cli_sensitive_axes(
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

// ---------------------------------------------------------------------------
// Resource support (LazyLock builder)
// ---------------------------------------------------------------------------

/// The complete `fn build_resource_support() -> ProviderCapabilities` item.
pub fn resource_support_builder(
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

// ---------------------------------------------------------------------------
// File assembly
// ---------------------------------------------------------------------------

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

/// Catalog shape: a bare member string for the unit variants, or the
/// externally tagged `{"shell_command": {"program": ..., "args": [...]}}`
/// object (mirroring the enum's serde wire form) for the data variant.
pub fn model_catalog_source(
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
