use super::*;
use super::execution_prompting::{stream_protocol, yolo};
use super::identity_paths::path_list_from_records;
use serde_json::json;

#[test]
fn pascal_converts_wire_forms() {
    assert_eq!(pascal("session_start"), "SessionStart");
    assert_eq!(pascal("human_in_the_loop"), "HumanInTheLoop");
    assert_eq!(pascal("json_rpc"), "JsonRpc");
    assert_eq!(pascal("not_supported"), "NotSupported");
}

#[test]
fn provider_variant_rejects_unknown_slug() {
    assert_eq!(provider_variant("kimi").unwrap(), "KimiCode");
    assert!(provider_variant("nonesuch").is_err());
}

#[test]
fn yolo_emits_all_wire_forms() {
    let mut ctx = EmitCtx::new("claude").unwrap();
    assert_eq!(
        yolo("yolo", &json!("none"), 1, &mut ctx).unwrap(),
        "YoloSupport::None"
    );
    let direct = yolo(
        "yolo",
        &json!({"direct_flag": {"native_flag": "--yolo"}}),
        1,
        &mut ctx,
    )
    .unwrap();
    assert!(direct.contains("native_flag: \"--yolo\""));
    assert!(yolo("yolo", &json!({"bogus": {}}), 1, &mut ctx).is_err());
}

#[test]
fn path_records_reject_templated_segments() {
    let mut ctx = EmitCtx::new("claude").unwrap();
    let err = path_list_from_records(
        "memory_files",
        &json!([{"raw": "~/{x}", "segments": ["home_dir"]}]),
        0,
        &mut ctx,
    )
    .unwrap_err();
    assert!(matches!(err, GenError::UnmappableValue { .. }));
}

#[test]
fn stream_protocol_maps_kebab_wire_forms() {
    let mut ctx = EmitCtx::new("claude").unwrap();
    assert_eq!(
        stream_protocol("stream_protocol", &json!("wire-json-rpc"), &mut ctx).unwrap(),
        "Some(StreamProtocol::WireJsonRpc)"
    );
    assert_eq!(
        stream_protocol("stream_protocol", &json!(null), &mut ctx).unwrap(),
        "None"
    );
    assert!(stream_protocol("stream_protocol", &json!("sse"), &mut ctx).is_err());
}
