use std::cell::Cell;
use std::path::Path;

use serde_json::Value;
use tracing_test::traced_test;

use super::*;
use crate::steering::contract::{CancellationOutcome, InterruptionConsent, SteeringMessage};
use crate::steering::identity::ProcessStartIdentity;

const ORIGINAL: &str = "Stop looping ✓.\nUse password=hunter22 and GITHUB_TOKEN=ghp_0123456789abcdefghij0123;\nask ken@example.com, see /Users/ken/project/notes.md";
const MASKED: &str = "Stop looping ✓.\nUse password=**** and GITHUB_TOKEN=****;\nask ken@example.com, see /Users/ken/project/notes.md";

fn managed_request(message: &str) -> SteeringRequest {
    let target = SteeringTargetId::Managed { execution: ExecutionId::from_u128(7) };
    SteeringRequest {
        id: RequestId::from_u128(42),
        target: target.clone(),
        origin: SteeringOrigin::Manual,
        operation: OperationIntent::InterruptThenSubmit,
        message: SteeringMessage::new(message).unwrap(),
        consent: Some(InterruptionConsent { target, operation: OperationIntent::InterruptThenSubmit }),
    }
}

fn pi_context() -> AuditContext {
    AuditContext {
        provider: Some(Provider::Pi),
        conversation: Some("conv-1".into()),
        generation: Some(ConversationGeneration(3)),
        profile: Some("rpc".into()),
        mechanism: Some("abort-submit"),
        opportunity: None,
    }
}

/// Every record in `dir`, in file then line order.
fn read_records(dir: &Path) -> Vec<Value> {
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).collect();
    files.sort();
    files
        .iter()
        .flat_map(|file| {
            std::fs::read_to_string(file)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect::<Vec<Value>>()
        })
        .collect()
}

fn raw_log_text(dir: &Path) -> String {
    std::fs::read_dir(dir).unwrap().map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap()).collect()
}

#[test]
fn records_carry_masked_text_identities_and_separate_interruption_outcomes() {
    let dir = tempfile::tempdir().unwrap();
    let log = SteeringAuditLog::at(dir.path());
    let request = managed_request(ORIGINAL);

    let sent = audited_send(&log, &request, &pi_context(), |delivered| {
        // Delivery receives the original bytes, not the masked copy.
        assert_eq!(delivered.message.as_str(), ORIGINAL);
        DeliveryReport {
            result: SteeringResult::interrupted(
                delivered,
                "abort-submit",
                InterruptionOutcome { cancellation: CancellationOutcome::Established, replacement: Some(SendOutcome::Unknown) },
            ),
            error: Some("replacement rejected: 'hunter22' is not a command".into()),
            provider_echo: Some(format!("{{\"type\":\"steer\",\"text\":{:?}}}", ORIGINAL)),
        }
    });

    assert!(sent.audit_failures.is_empty(), "{:?}", sent.audit_failures);
    assert_eq!(sent.result.outcome, SendOutcome::PartialInterruption);
    assert_eq!(sent.error.as_ref().unwrap().as_str(), "replacement rejected: '****' is not a command");
    assert!(!sent.provider_echo.as_ref().unwrap().as_str().contains("hunter22"));
    assert!(!format!("{sent:?}").contains("hunter22"), "Debug leaks the original");

    let records = read_records(dir.path());
    assert_eq!(records.len(), 2);
    let (req, res) = (&records[0], &records[1]);
    for record in &records {
        assert_eq!(record["record"], "steering");
        assert_eq!(record["schema"], 1);
        assert_eq!(record["request_id"], "00000000-0000-0000-0000-00000000002a");
        assert!(record["timestamp"].is_string());
    }
    assert_eq!(req["kind"], "request");
    assert_eq!(req["message"], MASKED);
    assert_eq!(req["message_bytes"], ORIGINAL.len());
    assert_eq!(req["target"], "managed:00000000-0000-0000-0000-000000000007");
    assert_eq!(req["execution"], "00000000-0000-0000-0000-000000000007");
    assert_eq!(req["origin"], "manual");
    assert_eq!(req["operation"], "interrupt_then_submit");
    assert_eq!(req["provider"], "pi");
    assert_eq!(req["conversation"], "conv-1");
    assert_eq!(req["generation"], 3);
    assert_eq!(req["profile"], "rpc");
    assert_eq!(req["mechanism"], "abort-submit");
    assert_eq!(req["opportunity"], Value::Null);
    assert_eq!(req["consent"]["target"], req["target"]);
    assert_eq!(req["consent"]["operation"], "interrupt_then_submit");
    assert_eq!(req["may_interrupt"], true);

    assert_eq!(res["kind"], "result");
    assert_eq!(res["mechanism"], "abort-submit");
    assert_eq!(res["outcome"], "partial_interruption");
    assert_eq!(res["receipt"], "unknown");
    assert_eq!(res["interruption"]["cancellation"], "established");
    assert_eq!(res["interruption"]["replacement"], "unknown");
    assert_eq!(res["error"], "replacement rejected: '****' is not a command");
    assert!(res.get("message").is_none(), "result records do not repeat the message");

    let raw = raw_log_text(dir.path());
    assert!(!raw.contains("hunter22") && !raw.contains("ghp_"), "{raw}");
}

#[test]
fn automatic_native_request_records_its_opportunity_and_no_execution() {
    let dir = tempfile::tempdir().unwrap();
    let log = SteeringAuditLog::at(dir.path());
    let request = SteeringRequest {
        id: RequestId::from_u128(1),
        target: SteeringTargetId::Native {
            provider: Provider::Codex,
            process: ProcessStartIdentity::new(99, "1700").unwrap(),
            conversation: "thread".into(),
        },
        origin: SteeringOrigin::Automatic,
        operation: OperationIntent::SteerActiveTurn,
        message: SteeringMessage::new("You appear to be repeating yourself.").unwrap(),
        consent: None,
    };
    let context = AuditContext { opportunity: Some(OpportunityId::from_u128(5)), ..AuditContext::default() };
    audited_send(&log, &request, &context, |delivered| {
        DeliveryReport::new(SteeringResult::submitted(delivered, Some("rpc-steer"), SendOutcome::Accepted))
    });

    let records = read_records(dir.path());
    assert_eq!(records[0]["execution"], Value::Null);
    assert_eq!(records[0]["opportunity"], "00000000-0000-0000-0000-000000000005");
    assert_eq!(records[0]["may_interrupt"], false);
    assert_eq!(records[0]["message"], "You appear to be repeating yourself.");
    assert_eq!(records[1]["receipt"], "accepted");
    assert_eq!(records[1]["interruption"], Value::Null);
}

#[traced_test]
#[test]
fn audit_write_failure_never_replays_or_changes_the_send_result() {
    let dir = tempfile::tempdir().unwrap();
    // A regular file where the log directory should be.
    let blocked = dir.path().join("not-a-directory");
    std::fs::write(&blocked, "").unwrap();

    for log in [SteeringAuditLog::at(&blocked), SteeringAuditLog { dir: None }] {
        let deliveries = Cell::new(0);
        let request = managed_request(ORIGINAL);
        let sent = audited_send(&log, &request, &pi_context(), |delivered| {
            deliveries.set(deliveries.get() + 1);
            assert_eq!(delivered.message.as_str(), ORIGINAL);
            DeliveryReport::new(SteeringResult::submitted(delivered, Some("rpc-steer"), SendOutcome::Accepted))
        });

        assert_eq!(deliveries.get(), 1, "delivery runs exactly once");
        assert_eq!(sent.result, SteeringResult::submitted(&request, Some("rpc-steer"), SendOutcome::Accepted));
        assert_eq!(sent.audit_failures.len(), 2);
        let late = DeliveryReport::new(SteeringResult::submitted(&request, None, SendOutcome::Delivered));
        assert!(sent.late.record(&log, &late).is_err());
    }
    assert_eq!(std::fs::read_to_string(&blocked).unwrap(), "");
    assert!(logs_contain("steering audit record not written"));
    assert!(!logs_contain("hunter22"));
    assert!(!logs_contain("password"));
}

#[test]
fn late_results_are_correlated_append_only_updates() {
    let dir = tempfile::tempdir().unwrap();
    let log = SteeringAuditLog::at(dir.path());
    let request = managed_request(ORIGINAL);
    let sent = audited_send(&log, &request, &pi_context(), |delivered| {
        DeliveryReport::new(SteeringResult::submitted(delivered, Some("rpc-steer"), SendOutcome::Unknown))
    });
    assert_eq!(sent.late.request_id(), request.id);

    let mut late = DeliveryReport::new(SteeringResult::submitted(&request, Some("rpc-steer"), SendOutcome::Delivered));
    late.error = Some("echo: password=hunter22".into());
    sent.late.record(&log, &late).unwrap();

    let other = managed_request(ORIGINAL);
    let other = SteeringRequest { id: RequestId::from_u128(43), ..other };
    let foreign = DeliveryReport::new(SteeringResult::submitted(&other, None, SendOutcome::Delivered));
    assert_eq!(sent.late.record(&log, &foreign), Err(AuditFailure::Uncorrelated));

    let records = read_records(dir.path());
    let kinds: Vec<&str> = records.iter().map(|record| record["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["request", "result", "late_result"], "one request, no new send");
    assert_eq!(records[1]["outcome"], "unknown");
    assert_eq!(records[2]["request_id"], records[0]["request_id"]);
    assert_eq!(records[2]["outcome"], "delivered");
    assert_eq!(records[2]["receipt"], "delivered");
    assert_eq!(records[2]["error"], "echo: password=****");
    assert!(records[2].get("message").is_none());
}

/// Read, write again, and read again: the persisted masked text is stable.
#[test]
fn persisted_message_round_trips_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let log = SteeringAuditLog::at(dir.path());
    let deliver = |delivered: &SteeringRequest| {
        DeliveryReport::new(SteeringResult::submitted(delivered, None, SendOutcome::Queued))
    };

    audited_send(&log, &managed_request(ORIGINAL), &pi_context(), deliver);
    let first = read_records(dir.path())[0]["message"].as_str().unwrap().to_string();
    assert_eq!(first, MASKED);

    audited_send(&log, &managed_request(&first), &pi_context(), deliver);
    let records = read_records(dir.path());
    assert_eq!(records[2]["kind"], "request");
    assert_eq!(records[2]["message"], first);
    assert_eq!(records[2]["message_bytes"], MASKED.len());
}
