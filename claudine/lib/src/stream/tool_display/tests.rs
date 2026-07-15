use super::*;

#[test]
fn enum_serializes_snake_case() {
    assert_eq!(
        serde_json::to_string(&ToolDirection::Outgoing).unwrap(),
        "\"outgoing\""
    );
    assert_eq!(
        serde_json::to_string(&ToolStatus::Success).unwrap(),
        "\"success\""
    );
}

#[test]
fn struct_round_trips_via_clone_and_eq() {
    let display = ToolCallDisplay {
        direction: ToolDirection::Incoming,
        raw_name: "firecrawl_firecrawl_search".into(),
        display_name: "Firecrawl Search".into(),
        summary: Some("NFL draft 2026 date".into()),
        status: Some(ToolStatus::Success),
        error_detail: None,
    };
    let cloned = display.clone();
    assert_eq!(display, cloned);
}
