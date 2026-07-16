//! Event- and support-policy emitters: the event-mapping table and its
//! support levels, ACP support, platform kind, known gaps / unmapped native
//! events, and the render `DisplayPolicy`.

use claudine_catalog_types::{EventClass, ToolResultSummary};
use strum::VariantNames;

use super::execution_prompting::stream_protocol_variant;
use super::*;

/// The body of the `EventMappingTable` static:
/// `EventMappingTable { mappings: &[ ... ] }`.
pub(crate) fn event_mapping_table(
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

pub(crate) fn platform_kind(
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

pub(crate) fn known_gaps(
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

pub(crate) fn unmapped_native_events(
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

pub(crate) fn acp(
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

/// `DisplayPolicy { ... }` literal from the facts display-policy record
/// (fixed field list, declaration order). Enum sub-keys are validated
/// against the catalog-types variant names — overrides flow through this
/// expression half only, so the check must live here too.
pub(crate) fn display_policy(
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

pub(crate) fn emission_fragment(
    values: &ResolvedValues<'_>,
    event_static: &str,
    ctx: &mut EmitCtx,
) -> Result<EmissionFragment, GenError> {
    let mut fragment = EmissionFragment::new();
    fragment.field(12, "event_mapping", format!("&{event_static}"));
    fragment.field(26, "known_gaps", known_gaps("known_gaps", values.get("known_gaps")?, 1, ctx)?);
    fragment.field(27, "acp", acp("acp", values.get("acp")?, 1, ctx)?);
    fragment.field(
        41,
        "display_policy",
        display_policy("display_policy", values.get("display_policy")?, 1, ctx)?,
    );
    fragment.field(45, "platform_kind", platform_kind("platform_kind", values.get("platform_kind")?, ctx)?);
    fragment.field(
        46,
        "unmapped_native_events",
        unmapped_native_events(
            "unmapped_native_events",
            values.get("unmapped_native_events")?,
            1,
            ctx,
        )?,
    );
    let table = event_mapping_table("event_mapping", values.get("event_mapping")?, 0, ctx)?;
    fragment.supporting_item(format!(
        "/// Event-mapping table (also referenced directly by behavior modules).\n\
         pub(in crate::provider) static {event_static}: EventMappingTable = {table};\n"
    ));
    Ok(fragment)
}
