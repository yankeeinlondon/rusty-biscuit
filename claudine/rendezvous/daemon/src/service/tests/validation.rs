//! RPC input error mapping: empty session id, non-object `details_json`,
//! and an unspecified event kind are each rejected before any register
//! mutation.

use super::*;

#[tokio::test]
async fn session_event_validation_rejects_bad_input() {
    let h = harness();
    let empty_id = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: String::new(),
            kind: rendezvous_core::SessionEventKind::Started as i32,
            details_json: String::new(),
            status: None,
        }))
        .await;
    assert!(empty_id.is_err());

    let bad_details = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "sess-2".into(),
            kind: rendezvous_core::SessionEventKind::Started as i32,
            details_json: "[1,2,3]".into(),
            status: None,
        }))
        .await;
    assert!(bad_details.is_err(), "non-object details must be rejected");

    let unspecified = h
        .service
        .report_session_event(Request::new(rendezvous_core::ReportSessionEventRequest {
            session_id: "sess-3".into(),
            kind: rendezvous_core::SessionEventKind::Unspecified as i32,
            details_json: String::new(),
            status: None,
        }))
        .await;
    assert!(unspecified.is_err());
}
